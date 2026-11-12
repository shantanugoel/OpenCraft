//! The engine thread: the loop around the engine logic.
//!
//! The engine has one thread. It owns the device, the profile, and the
//! images, and it does all device I/O (architecture rule 5). A shell holds an
//! [`EngineHandle`]: it sends commands through the handle, and it gets the
//! events in the function that it gave to [`spawn`]. The reasons are in
//! `docs/decisions/0011-engine-thread-model.md`.

use crate::action::{Handler, HandlerPort, Message};
use crate::engine::{Command, Engine, Event};
use crate::profiles::ProfilesDir;
use opencraft_core::device::DESCRIPTORS;
use opencraft_core::{Clock, Connector, HidConnector, Link};
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// The longest time that a command waits before the engine thread obeys it.
/// The engine thread waits for input from the device for this time, then it
/// looks for commands. Input from the device does not wait.
pub const POLL_INTERVAL: Duration = Duration::from_millis(20);

// A sender that stays busy must not stop device polls or the other queue.
const MESSAGES_PER_POLL: usize = 64;

/// What the engine needs to start.
#[derive(Debug, Clone)]
pub struct Config {
    /// The profiles directory: each profile is a directory in it. See
    /// [`crate::store::default_profiles_dir`]. A test or an example gives a
    /// temporary directory.
    pub profiles_dir: PathBuf,
}

/// The connection of a shell to the engine thread. When the handle goes
/// away, the engine writes the profile and stops.
pub struct EngineHandle {
    commands: Sender<Command>,
    thread: Option<JoinHandle<()>>,
}

impl EngineHandle {
    /// Send a command to the engine. The call does not block. A command to
    /// an engine that stopped does nothing.
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    /// Stop the engine, and wait until the engine thread wrote the profile
    /// and ended.
    pub fn stop(mut self) {
        self.stop_and_join();
    }

    fn stop_and_join(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.send(Command::Stop);
            let _ = thread.join();
        }
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

/// Start the engine on its own thread, with the real device.
///
/// The engine thread calls `sink` for each event. `sink` must return
/// quickly: it puts the event in a queue of the shell and wakes the user
/// interface thread. The first events are the layout, the kind descriptions,
/// the state of each surface, the profiles, the pages, and the state of each
/// binding. See [`Event`].
pub fn spawn(
    config: Config,
    handlers: Vec<Box<dyn Handler>>,
    sink: impl FnMut(Event) + Send + 'static,
) -> io::Result<EngineHandle> {
    let (ports, messages) = start_handlers(handlers)?;
    let (commands, receiver) = mpsc::channel();
    let thread = thread::Builder::new()
        .name("opencraft-engine".to_string())
        .spawn(move || {
            let mut engine = Engine::with_handlers(
                Link::new(HidConnector),
                ProfilesDir::new(config.profiles_dir),
                // The layout until a device connects.
                &DESCRIPTORS[0],
                ports,
                sink,
            );
            run_with_handlers(&mut engine, &receiver, &messages);
        })?;
    Ok(EngineHandle {
        commands,
        thread: Some(thread),
    })
}

/// The loop of the engine thread: obey commands, then wait
/// for the device for [`POLL_INTERVAL`]. The loop ends at the stop command,
/// or when each sender of `commands` is gone. Then the engine writes the
/// profile if it changed.
pub fn run<C: Connector, S: FnMut(Event), K: Clock>(
    engine: &mut Engine<C, S, K>,
    commands: &Receiver<Command>,
) {
    let (_sender, messages) = mpsc::channel();
    run_with_handlers(engine, commands, &messages);
}

/// Connect each handler to an unbounded request queue and the shared message queue.
/// Descriptions are read here, before the engine thread starts.
/// Handler threads stop when their request sender goes away. The engine never joins them.
pub fn start_handlers(
    handlers: Vec<Box<dyn Handler>>,
) -> io::Result<(Vec<HandlerPort>, Receiver<Message>)> {
    let (sender, messages) = mpsc::channel();
    let mut ports = Vec::new();
    let mut kinds = std::collections::BTreeSet::new();
    let mut prepared = Vec::new();
    for handler in handlers {
        let descriptions = handler.kinds();
        for description in &descriptions {
            if !kinds.insert(description.kind.clone()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "two handlers have the same kind",
                ));
            }
        }
        prepared.push((handler, descriptions));
    }
    for (mut handler, descriptions) in prepared {
        let (requests, receiver) = mpsc::channel();
        let sender = sender.clone();
        thread::Builder::new()
            .name("opencraft-handler".into())
            .spawn(move || {
                handler.start(sender);
                while let Ok(request) = receiver.recv() {
                    handler.handle(request);
                }
            })?;
        ports.push(HandlerPort {
            kinds: descriptions,
            sender: requests,
        });
    }
    Ok((ports, messages))
}

/// Read commands and handler messages before each device poll.
pub fn run_with_handlers<C: Connector, S: FnMut(Event), K: Clock>(
    engine: &mut Engine<C, S, K>,
    commands: &Receiver<Command>,
    messages: &Receiver<Message>,
) {
    'run: loop {
        for _ in 0..MESSAGES_PER_POLL {
            match commands.try_recv() {
                Ok(command) => {
                    if !engine.command(command) {
                        break 'run;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break 'run,
            }
        }
        for message in messages.try_iter().take(MESSAGES_PER_POLL) {
            engine.handler_message(message);
        }
        engine.poll(POLL_INTERVAL);
    }
    engine.finish();
}

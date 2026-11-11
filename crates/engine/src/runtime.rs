//! The engine thread: the loop around the engine logic.
//!
//! The engine has one thread. It owns the device, the profile, and the
//! images, and it does all device I/O (architecture rule 5). A shell holds an
//! [`EngineHandle`]: it sends commands through the handle, and it gets the
//! events in the function that it gave to [`spawn`]. The reasons are in
//! `docs/decisions/0011-engine-thread-model.md`.

use crate::engine::{Command, Engine, Event};
use crate::store::ProfileStore;
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

/// What the engine needs to start.
#[derive(Debug, Clone)]
pub struct Config {
    /// The profile directory. See [`crate::store::default_profile_dir`].
    pub profile_dir: PathBuf,
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
/// interface thread. The first events are the layout and the state of each
/// surface.
pub fn spawn(config: Config, sink: impl FnMut(Event) + Send + 'static) -> io::Result<EngineHandle> {
    let (commands, receiver) = mpsc::channel();
    let thread = thread::Builder::new()
        .name("opencraft-engine".to_string())
        .spawn(move || {
            let mut engine = Engine::new(
                Link::new(HidConnector),
                ProfileStore::new(config.profile_dir),
                // The layout until a device connects.
                &DESCRIPTORS[0],
                sink,
            );
            run(&mut engine, &receiver);
        })?;
    Ok(EngineHandle {
        commands,
        thread: Some(thread),
    })
}

/// The loop of the engine thread: obey each command that waits, then wait
/// for the device for [`POLL_INTERVAL`]. The loop ends at the stop command,
/// or when each sender of `commands` is gone. Then the engine writes the
/// profile if it changed.
pub fn run<C: Connector, S: FnMut(Event), K: Clock>(
    engine: &mut Engine<C, S, K>,
    commands: &Receiver<Command>,
) {
    'run: loop {
        loop {
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
        engine.poll(POLL_INTERVAL);
    }
    engine.finish();
}

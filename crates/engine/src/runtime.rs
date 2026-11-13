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
use opencraft_core::{Clock, Connector, Device, HidConnector, HidTransport, Link};
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::Mutex;
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
    /// True: the engine does not look for a device, and the shell shows
    /// "not connected". This is for a check of the shell with no device
    /// while a device is connected: the engine does not open that device,
    /// so a different process can operate it.
    pub no_device: bool,
}

/// The connector of [`spawn`]: the real connector, or no device for
/// [`Config::no_device`].
struct DeviceConnector {
    no_device: bool,
}

impl Connector for DeviceConnector {
    type Transport = HidTransport;

    fn connect(&mut self) -> opencraft_core::Result<Option<Device<HidTransport>>> {
        if self.no_device {
            Ok(None)
        } else {
            HidConnector.connect()
        }
    }
}

/// The longest time that [`EngineHandle::stop`] waits for the engine thread.
///
/// The engine thread writes the profile and the device settings, sends 2
/// reports to the device, and closes it. That is much less than 1 second. A
/// device write has no time limit in the transport, so a write that blocks
/// could stop the quit of the shell with no limit. After this time, the
/// handle does not wait more. The files are written before the device
/// writes.
pub const STOP_TIMEOUT: Duration = Duration::from_secs(3);

/// The connection of a shell to the engine thread. When the handle goes
/// away, the engine stops as for [`EngineHandle::stop`].
pub struct EngineHandle {
    commands: Sender<Command>,
    thread: Option<JoinHandle<()>>,
    /// The engine thread holds the sender. The receiver sees the end of the
    /// thread when the sender goes away, also after a panic. The mutex keeps
    /// the handle `Sync`, so that a shell can share it between threads.
    ended: Mutex<Receiver<()>>,
    stop_timeout: Duration,
}

impl EngineHandle {
    /// Send a command to the engine. The call does not block. A command to
    /// an engine that stopped does nothing.
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    /// Stop the engine, and wait until the engine thread wrote the profile and
    /// the device settings, made the device dark, and ended. See
    /// [`Engine::finish`].
    ///
    /// The call waits for [`STOP_TIMEOUT`] at most. If the engine thread did
    /// not end in that time, the call returns, and the thread continues
    /// until the process exits.
    pub fn stop(mut self) {
        self.stop_and_join();
    }

    fn stop_and_join(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.send(Command::Stop);
        let ended = match self.ended.get_mut() {
            Ok(ended) => ended,
            Err(poisoned) => poisoned.into_inner(),
        };
        match ended.recv_timeout(self.stop_timeout) {
            Err(RecvTimeoutError::Timeout) => eprintln!(
                "opencraft-engine: the engine did not stop in {:.1} s; OpenCraft does not wait more",
                self.stop_timeout.as_secs_f64()
            ),
            // The thread ended, or it is about to end.
            Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                let _ = thread.join();
            }
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
    start_thread(STOP_TIMEOUT, move |commands| {
        let mut engine = Engine::with_handlers(
            Link::new(DeviceConnector {
                no_device: config.no_device,
            }),
            ProfilesDir::new(config.profiles_dir),
            // The layout until a device connects. With more than one model,
            // this is not always the model of the device (issue #156).
            &DESCRIPTORS[0],
            ports,
            sink,
        );
        run_with_handlers(&mut engine, commands, &messages);
    })
}

/// Start the engine thread with the body `body`, and give the handle.
fn start_thread(
    stop_timeout: Duration,
    body: impl FnOnce(&Receiver<Command>) + Send + 'static,
) -> io::Result<EngineHandle> {
    let (commands, receiver) = mpsc::channel();
    let (end, ended) = mpsc::channel::<()>();
    let thread = thread::Builder::new()
        .name("opencraft-engine".to_string())
        .spawn(move || {
            // Dropped at the end of the thread, also at a panic.
            let _end = end;
            body(&receiver);
        })?;
    Ok(EngineHandle {
        commands,
        thread: Some(thread),
        ended: Mutex::new(ended),
        stop_timeout,
    })
}

/// The loop of the engine thread: obey commands, then wait
/// for the device for [`POLL_INTERVAL`]. The loop ends at the stop command,
/// or when each sender of `commands` is gone. Then the engine stops with
/// [`Engine::finish`]: it writes the profile and the device settings if they
/// changed, and makes the device dark.
///
/// A panic in the engine thread ends the thread with no stop sequence. A
/// panic can occur in the middle of an image transfer, and then the device
/// would read the stop sequence as image data. Thus a panic is the same as a
/// crash for the device (decision 0022): the images stay, and the device
/// shows its logo after 20 to 35 seconds.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn sending_a_remove_command_does_not_wait_for_the_engine() {
        let (release, blocked) = mpsc::channel();
        let handle = start_thread(Duration::from_secs(10), move |commands| {
            blocked.recv().unwrap();
            assert!(matches!(commands.recv(), Ok(Command::RemoveProfile { .. })));
            assert!(matches!(commands.recv(), Ok(Command::Stop)));
        })
        .unwrap();
        let start = Instant::now();
        handle.send(Command::RemoveProfile {
            profile: "checks".into(),
        });
        assert!(start.elapsed() < Duration::from_secs(1));
        release.send(()).unwrap();
        handle.stop();
    }

    #[test]
    fn stop_waits_for_the_engine_thread() {
        let (done, finished) = mpsc::channel();
        let handle = start_thread(Duration::from_secs(10), move |commands| {
            while !matches!(commands.recv(), Ok(Command::Stop) | Err(_)) {}
            thread::sleep(Duration::from_millis(50));
            done.send(()).unwrap();
        })
        .unwrap();
        handle.stop();
        assert!(finished.try_recv().is_ok(), "stop returned before the end");
    }

    #[test]
    fn stop_does_not_wait_longer_than_the_limit_for_a_thread_that_blocks() {
        // The body does not look at the commands: it is like a device write
        // that blocks.
        let handle = start_thread(Duration::from_millis(100), |_| {
            thread::sleep(Duration::from_secs(5));
        })
        .unwrap();
        let start = Instant::now();
        handle.stop();
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "{:?}",
            start.elapsed()
        );
    }

    #[test]
    fn stop_returns_at_once_after_a_panic_in_the_engine_thread() {
        let handle = start_thread(Duration::from_secs(10), |_| panic!("test panic")).unwrap();
        thread::sleep(Duration::from_millis(50));
        let start = Instant::now();
        drop(handle);
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "{:?}",
            start.elapsed()
        );
    }
}

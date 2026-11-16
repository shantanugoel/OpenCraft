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
use opencraft_core::device::{Descriptor, DESCRIPTORS};
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

/// A command in the engine queue, with an optional private reply channel.
struct PendingCommand {
    command: Command,
    reply: Option<Sender<std::result::Result<(), String>>>,
}

impl From<Command> for PendingCommand {
    fn from(command: Command) -> Self {
        Self {
            command,
            reply: None,
        }
    }
}

/// A command sender that does not own the engine thread. Each shell can
/// keep a clone. Dropping a sender does not stop the engine.
#[derive(Clone)]
pub struct EngineSender {
    commands: Sender<PendingCommand>,
}

impl EngineSender {
    /// Send a command with no private reply. The call does not block. A
    /// command to an engine that stopped does nothing.
    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command.into());
    }

    /// Send a command and return its private result channel without waiting.
    /// The receiver gets exactly one success or the first problem text after
    /// the engine obeys the whole command, including a group or a profile
    /// guard. Other senders cannot receive this result. Problems still go
    /// to the shell through [`Event::Problem`].
    ///
    /// Success does not wait for delayed file writes or handler responses:
    /// see [`Engine::command_with_result`]. If the engine stops before it
    /// obeys the command, the channel disconnects without a result. A caller
    /// can use `recv_timeout` to limit its wait. Dropping the receiver does
    /// not cancel the command or block the engine.
    pub fn send_with_result(&self, command: Command) -> Receiver<std::result::Result<(), String>> {
        let (reply, result) = mpsc::channel();
        let _ = self.commands.send(PendingCommand {
            command,
            reply: Some(reply),
        });
        result
    }
}

/// The connection of a shell to the engine thread. When the handle goes
/// away, the engine stops as for [`EngineHandle::stop`], even if command
/// senders from [`EngineHandle::sender`] still exist.
pub struct EngineHandle {
    sender: EngineSender,
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
        self.sender.send(command);
    }

    /// Give a command sender to a second shell or thread. It can outlive
    /// this handle, but it cannot stop the engine by being dropped.
    pub fn sender(&self) -> EngineSender {
        self.sender.clone()
    }

    /// Send a command with one private result. See
    /// [`EngineSender::send_with_result`].
    pub fn send_with_result(&self, command: Command) -> Receiver<std::result::Result<(), String>> {
        self.sender.send_with_result(command)
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
        let profiles = ProfilesDir::new(config.profiles_dir);
        let start = start_descriptor(DESCRIPTORS, &profiles);
        let mut engine = Engine::with_handlers(
            Link::new(DeviceConnector {
                no_device: config.no_device,
            }),
            profiles,
            start,
            ports,
            sink,
        );
        run_loop(&mut engine, || commands.try_recv(), &messages);
    })
}

/// The descriptor for the layout, the profiles, and the device settings until
/// a device connects (decision 0039): the model of the last device that
/// connected, as `state.toml` in `profiles` records it. If the file names no
/// model, or a model that is not in `table`, this is the first descriptor of
/// `table`.
///
/// `table` must not be empty.
pub fn start_descriptor(
    table: &'static [Descriptor],
    profiles: &ProfilesDir,
) -> &'static Descriptor {
    profiles
        .last_model()
        .and_then(|model| table.iter().find(|d| d.model == model))
        .unwrap_or(&table[0])
}

/// Start the engine thread with the body `body`, and give the handle.
fn start_thread(
    stop_timeout: Duration,
    body: impl FnOnce(&Receiver<PendingCommand>) + Send + 'static,
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
        sender: EngineSender { commands },
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
    run_loop(engine, || commands.try_recv().map(Into::into), messages);
}

fn run_loop<C: Connector, S: FnMut(Event), K: Clock>(
    engine: &mut Engine<C, S, K>,
    mut next: impl FnMut() -> std::result::Result<PendingCommand, TryRecvError>,
    messages: &Receiver<Message>,
) {
    'run: loop {
        for _ in 0..MESSAGES_PER_POLL {
            match next() {
                Ok(PendingCommand { command, reply }) => {
                    let go_on = if let Some(reply) = reply {
                        let (go_on, result) = engine.command_with_result(command);
                        // An abandoned request must not block the engine.
                        let _ = reply.send(result);
                        go_on
                    } else {
                        engine.command(command)
                    };
                    if !go_on {
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

    struct NoDevice;

    impl Connector for NoDevice {
        type Transport = opencraft_core::FakeTransport;

        fn connect(&mut self) -> opencraft_core::Result<Option<Device<Self::Transport>>> {
            Ok(None)
        }
    }

    fn test_engine(sink: impl FnMut(Event) + Send + 'static) -> EngineHandle {
        start_thread(Duration::from_secs(10), move |commands| {
            let dir = tempfile::tempdir().unwrap();
            let mut engine = Engine::new(
                Link::new(NoDevice),
                ProfilesDir::new(dir.path().to_path_buf()),
                &DESCRIPTORS[0],
                sink,
            );
            let (_sender, messages) = mpsc::channel();
            run_loop(&mut engine, || commands.try_recv(), &messages);
        })
        .unwrap()
    }

    #[test]
    fn a_private_reply_waits_for_the_command_events() {
        let (changed, change) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let handle = test_engine(move |event| {
            if matches!(event, Event::Surface(ref state) if state.content.label.as_deref() == Some("Ready"))
            {
                changed.send(()).unwrap();
                blocked.recv().unwrap();
            }
        });
        let reply = handle.send_with_result(Command::SetLabel {
            surface: crate::SurfaceId::new(opencraft_core::SurfaceKind::KeyFace, 3),
            label: Some("Ready".into()),
        });
        change.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(reply.try_recv(), Err(TryRecvError::Empty));
        release.send(()).unwrap();
        assert_eq!(reply.recv_timeout(Duration::from_secs(10)).unwrap(), Ok(()));
        assert_eq!(
            reply.recv_timeout(Duration::from_secs(10)),
            Err(RecvTimeoutError::Disconnected)
        );
        handle.stop();
    }

    #[test]
    fn two_concurrent_senders_get_only_their_own_results() {
        let (events, event) = mpsc::channel();
        let handle = test_engine(move |e| events.send(e).unwrap());
        let first = handle.sender();
        let second = handle.sender();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let start = barrier.clone();
        let success = thread::spawn(move || {
            start.wait();
            first.send_with_result(Command::InProfile {
                profile: "default".into(),
                command: Box::new(Command::Group {
                    name: "Add page".into(),
                    commands: vec![Command::AddPage { copy_of: None }],
                }),
            })
        });
        let failure = thread::spawn(move || {
            barrier.wait();
            second.send_with_result(Command::SelectPage { page: usize::MAX })
        });
        let success = success.join().unwrap();
        let failure = failure.join().unwrap();
        assert_eq!(
            success.recv_timeout(Duration::from_secs(10)).unwrap(),
            Ok(())
        );
        let problem = failure
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap_err();
        assert_eq!(
            success.recv_timeout(Duration::from_secs(10)),
            Err(RecvTimeoutError::Disconnected)
        );
        assert_eq!(
            failure.recv_timeout(Duration::from_secs(10)),
            Err(RecvTimeoutError::Disconnected)
        );
        handle.stop();
        assert_eq!(
            event
                .try_iter()
                .filter_map(|e| match e {
                    Event::Problem(text) => Some(text),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec![problem]
        );
    }

    #[test]
    fn an_abandoned_reply_does_not_block_the_next_command() {
        let handle = test_engine(|_| {});
        drop(handle.send_with_result(Command::ReadCounters));
        let reply = handle.send_with_result(Command::ReadCounters);
        assert_eq!(reply.recv_timeout(Duration::from_secs(10)).unwrap(), Ok(()));
        handle.stop();
    }

    #[test]
    fn a_sender_does_not_keep_a_stopped_engine_alive() {
        let handle = test_engine(|_| {});
        let sender = handle.sender();
        handle.stop();
        assert_eq!(
            sender
                .send_with_result(Command::ReadCounters)
                .recv_timeout(Duration::from_secs(10)),
            Err(RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn a_queued_reply_disconnects_if_the_engine_does_not_obey_it() {
        let (release, blocked) = mpsc::channel();
        let handle = start_thread(Duration::from_secs(10), move |_| {
            blocked.recv().unwrap();
        })
        .unwrap();
        let reply = handle.send_with_result(Command::ReadCounters);
        assert_eq!(reply.try_recv(), Err(TryRecvError::Empty));
        release.send(()).unwrap();
        assert_eq!(
            reply.recv_timeout(Duration::from_secs(10)),
            Err(RecvTimeoutError::Disconnected)
        );
        handle.stop();
    }

    #[test]
    fn sending_a_remove_command_does_not_wait_for_the_engine() {
        let (release, blocked) = mpsc::channel();
        let handle = start_thread(Duration::from_secs(10), move |commands| {
            blocked.recv().unwrap();
            assert!(matches!(
                commands.recv().map(|pending| pending.command),
                Ok(Command::RemoveProfile { .. })
            ));
            assert!(matches!(
                commands.recv().map(|pending| pending.command),
                Ok(Command::Stop)
            ));
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
            while !matches!(
                commands.recv().map(|pending| pending.command),
                Ok(Command::Stop) | Err(_)
            ) {}
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

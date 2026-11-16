//! The optional, shell-owned control socket (decision 0040).
//!
//! The read cache is a [`Feed`]. The engine thread calls [`Feed::observe`]
//! for each event, before it gives the event to the window, and the worker
//! reads the cache under the same lock. The engine thread also calls the
//! reply of a change request, after the events of the request. Thus a
//! client that gets the response of a change sees the change in the next
//! read, and the user interface thread is not in the path.
//!
//! When enabled, one worker owns all connections. It uses nonblocking
//! sockets, bounded client buffers and a wake pipe, with no periodic wake
//! while idle. No thread exists when off.

mod changes;
pub mod protocol;
#[cfg(unix)]
mod server;
mod state;
#[cfg(test)]
mod tests;

use opencraft_engine::{Command, Event};
use std::{
    io,
    path::Path,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

/// The function that gets the result of one command. The engine thread calls
/// it.
pub type Reply = Box<dyn FnOnce(Result<(), String>) + Send>;

/// How the control module sends a command to the engine. With a reply, the
/// reply gets the result of the command.
pub type Port = Arc<dyn Fn(Command, Option<Reply>) + Send + Sync>;

/// The port of an engine.
pub fn port(engine: opencraft_engine::EngineSender) -> Port {
    Arc::new(move |command, reply| match reply {
        Some(reply) => engine.send_with_reply(command, reply),
        None => engine.send(command),
    })
}

#[derive(Default)]
struct FeedState {
    state: protocol::State,
    #[cfg(unix)]
    waker: Option<server::Waker>,
}

/// The read cache of the control socket. A clone shares the cache. The cache
/// follows the events of the engine also while control is off.
#[derive(Clone, Default)]
pub struct Feed(Arc<Mutex<FeedState>>);

impl Feed {
    fn lock(&self) -> MutexGuard<'_, FeedState> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Update the cache. Call this on the engine thread for each event, or
    /// in a test for each event of a fake engine. It does not block for
    /// more than the time of one update.
    pub fn observe(&self, event: &Event) {
        let mut feed = self.lock();
        feed.state.apply(event);
        #[cfg(unix)]
        if let Some(waker) = &feed.waker {
            // Only the answers that a worker waits for need a wake.
            if matches!(event, Event::Choices { .. } | Event::ProfileRead { .. }) {
                waker.post(server::Message::Event(event.clone()));
            }
        }
    }

    /// Read the cache.
    fn read<T>(&self, read: impl FnOnce(&protocol::State) -> T) -> T {
        read(&self.lock().state)
    }
}

pub struct Control {
    feed: Feed,
    port: Option<Port>,
    #[cfg(unix)]
    server: Option<server::Server>,
}

impl Control {
    pub fn new() -> Self {
        Self::with(Feed::default(), None)
    }

    pub fn with(feed: Feed, port: Option<Port>) -> Self {
        Self {
            feed,
            port,
            #[cfg(unix)]
            server: None,
        }
    }

    /// Update the cache with an event. The shell does not call this. A test
    /// does.
    #[cfg(test)]
    pub fn observe(&self, event: &Event) {
        self.feed.observe(event);
    }

    /// Takes effect before returning, also for open connections. Called by
    /// the shell setting setter; the window switch is a separate task. If the
    /// worker ended with an error, the call with `true` starts a new one.
    pub fn set_enabled(&mut self, enabled: bool, config_dir: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            // A worker that ended by an error or a panic left no socket file.
            // Make a new server for the setting, and tell the shell if that
            // fails.
            if self.server.as_ref().is_some_and(|s| !s.is_running()) {
                self.server = None;
            }
            if !enabled {
                self.server = None;
            } else if self.server.is_none() {
                self.server = Some(server::Server::start(
                    config_dir,
                    self.feed.clone(),
                    self.port.clone(),
                )?);
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = config_dir;
            if enabled {
                Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Control requires a Unix socket.",
                ))
            } else {
                Ok(())
            }
        }
    }
}

/// Only the process that holds the application lock may call this. Never
/// remove a non-socket file, including a symlink, at the socket path.
#[cfg(unix)]
pub fn remove_stale(config_dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::FileTypeExt;
    let path = config_dir.join(protocol::FILE_NAME);
    match std::fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_socket() => std::fs::remove_file(path),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "The control socket path is not a socket.",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

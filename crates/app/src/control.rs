//! The optional, shell-owned control socket (decision 0040).
//!
//! The shell keeps a read cache even while control is off. When enabled,
//! one worker owns all connections and a copy of that cache. Events go to
//! an unbounded queue; the shell never waits for a client or does socket
//! I/O. The worker uses nonblocking sockets, bounded client buffers and a
//! wake pipe, with no periodic wake while idle. No thread exists when off.

pub mod protocol;
#[cfg(unix)]
mod server;
mod state;
#[cfg(test)]
mod tests;

use opencraft_engine::{Command, Event};
use std::{io, path::Path};

pub struct Control {
    state: protocol::State,
    commands: async_channel::Receiver<Command>,
    #[cfg(unix)]
    sender: async_channel::Sender<Command>,
    #[cfg(unix)]
    server: Option<server::Server>,
}

impl Control {
    pub fn new() -> Self {
        let (sender, commands) = async_channel::unbounded();
        #[cfg(not(unix))]
        drop(sender);
        Self {
            state: protocol::State::default(),
            commands,
            #[cfg(unix)]
            sender,
            #[cfg(unix)]
            server: None,
        }
    }

    /// The shell forwards these commands through its EngineHandle sender.
    /// This also gives #403 one place to add completed change requests.
    pub fn commands(&self) -> async_channel::Receiver<Command> {
        self.commands.clone()
    }

    pub fn observe(&mut self, event: &Event) {
        self.state.apply(event);
        #[cfg(unix)]
        if let Some(server) = &self.server {
            server.observe(event.clone());
        }
    }

    /// Takes effect before returning, also for open connections. Called by
    /// the shell setting setter; the window switch is a separate task.
    pub fn set_enabled(&mut self, enabled: bool, config_dir: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            if !enabled {
                self.server = None;
            } else if self.server.is_none() {
                self.server = Some(server::Server::start(
                    config_dir,
                    self.state.clone(),
                    self.sender.clone(),
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

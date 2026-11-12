//! SIGTERM, SIGINT, and SIGHUP start the quit path (decisions 0020 and 0022).
//!
//! The signal handler only writes one byte to a pipe. One thread reads the
//! pipe and sends the quit command to the shell. The application does not
//! block the signals: a child process gets the blocked mask, and then does not
//! stop at SIGTERM. See `docs/kb/gpui.md`, "Quit".

use crate::shell::ShellCommand;
use std::{
    io,
    sync::atomic::{AtomicI32, Ordering},
};

/// The signals that start the quit path.
pub const SIGNALS: [libc::c_int; 3] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP];

/// The end of the pipe that the handler writes to.
static PIPE: AtomicI32 = AtomicI32::new(-1);

extern "C" fn on_signal(signal: libc::c_int) {
    // Only work that is safe in a signal handler: one write.
    let byte = signal as u8;
    // SAFETY: `write` is async-signal-safe. The descriptor stays open until
    // the process stops.
    unsafe {
        libc::write(PIPE.load(Ordering::Relaxed), (&byte as *const u8).cast(), 1);
    }
}

/// Set the handlers for [`SIGNALS`], and start the thread that sends
/// `ShellCommand::Quit` on `commands` for each signal.
pub fn forward(commands: async_channel::Sender<ShellCommand>) -> io::Result<()> {
    forward_signals(&SIGNALS, commands)
}

/// Call this function one time only in a process.
fn forward_signals(
    signals: &[libc::c_int],
    commands: async_channel::Sender<ShellCommand>,
) -> io::Result<()> {
    let mut fds = [0; 2];
    // SAFETY: `pipe` writes two descriptors into the array.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let [read, write] = fds;
    for fd in fds {
        // A child process must not get the pipe.
        // SAFETY: `fd` is a descriptor that `pipe` made.
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    // The handler must not wait if the pipe is full.
    // SAFETY: as above.
    unsafe {
        let flags = libc::fcntl(write, libc::F_GETFL);
        libc::fcntl(write, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    PIPE.store(write, Ordering::Relaxed);
    std::thread::Builder::new()
        .name("opencraft-signals".into())
        .spawn(move || loop {
            let mut byte = 0u8;
            // SAFETY: the buffer is one byte, and `read` is a descriptor
            // that `pipe` made.
            let count = unsafe { libc::read(read, (&mut byte as *mut u8).cast(), 1) };
            if count == 1 {
                eprintln!("OpenCraft got signal {byte}, and quits.");
                if commands.send_blocking(ShellCommand::Quit).is_err() {
                    return;
                }
            } else if count == 0 || io::Error::last_os_error().kind() != io::ErrorKind::Interrupted
            {
                return;
            }
        })?;
    for &signal in signals {
        // SAFETY: the handler does only async-signal-safe work.
        if unsafe {
            libc::signal(
                signal,
                on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t,
            )
        } == libc::SIG_ERR
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn a_signal_sends_one_quit_command() {
        // SIGUSR1 keeps the usual result of SIGTERM and SIGINT for the
        // test program.
        let (sender, receiver) = async_channel::unbounded();
        forward_signals(&[libc::SIGUSR1], sender).unwrap();
        // SAFETY: the handler for SIGUSR1 is set.
        unsafe { libc::raise(libc::SIGUSR1) };
        let start = Instant::now();
        loop {
            match receiver.try_recv() {
                Ok(command) => break assert_eq!(command, ShellCommand::Quit),
                Err(_) if start.elapsed() < Duration::from_secs(5) => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("No quit command: {error}"),
            }
        }
        assert!(receiver.try_recv().is_err());
    }
}

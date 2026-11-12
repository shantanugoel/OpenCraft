//! One OpenCraft process for each user (decision 0019).
//!
//! The application locks the file `opencraft.lock` before the engine starts.
//! The operating system removes the lock when the process stops, also after a
//! crash.

use std::{fs::File, io, os::fd::AsRawFd, path::Path};

pub const FILE_NAME: &str = "opencraft.lock";

/// Lock the file `opencraft.lock` in `dir`. Keep the file that this function
/// gives until the process stops. `Ok(None)` means that a different process
/// holds the lock.
pub fn acquire(dir: &Path) -> io::Result<Option<File>> {
    std::fs::create_dir_all(dir)?;
    // The standard library opens the file with `O_CLOEXEC`. Thus a child
    // process, for example of the shell command action, does not keep the
    // lock after OpenCraft stops.
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(FILE_NAME))?;
    // SAFETY: `flock` reads only the file descriptor, which `file` owns.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(Some(file));
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock {
        Ok(None)
    } else {
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_lock_fails_until_the_first_file_closes() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("OpenCraft");
        let first = acquire(&config).unwrap();
        assert!(first.is_some());
        assert!(config.join(FILE_NAME).exists());
        // A lock from `flock` is for one open file, thus a second open in the
        // same process is the same as a second process.
        assert!(acquire(&config).unwrap().is_none());
        drop(first);
        assert!(acquire(&config).unwrap().is_some());
    }
}

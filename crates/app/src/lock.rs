//! One OpenCraft process for each user (decisions 0019 and 0028).
//!
//! The application locks the file `opencraft.lock` before the engine starts.
//! The operating system removes the lock when the process stops, also after a
//! crash. The process that has the lock writes its process number into the
//! file. A second start reads the number, and asks that process to open its
//! window.

use std::{
    fs::File,
    io::{self, Write},
    os::fd::AsRawFd,
    path::Path,
};

pub const FILE_NAME: &str = "opencraft.lock";

/// The name of the executable of OpenCraft, in the bundle and in a build.
const PROCESS_NAME: &str = "opencraft";

/// Lock the file `opencraft.lock` in `dir`, and write the number of this
/// process into it. Keep the file that this function gives until the
/// process stops. `Ok(None)` means that a different process holds the lock.
pub fn acquire(dir: &Path) -> io::Result<Option<File>> {
    std::fs::create_dir_all(dir)?;
    // The standard library opens the file with `O_CLOEXEC`. Thus a child
    // process, for example of the shell command action, does not keep the
    // lock after OpenCraft stops.
    let mut file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join(FILE_NAME))?;
    // SAFETY: `flock` reads only the file descriptor, which `file` owns.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        // Without the number, a second start only writes its message. Thus
        // an error here does not stop the start.
        let _ = file
            .set_len(0)
            .and_then(|()| writeln!(file, "{}", std::process::id()));
        return Ok(Some(file));
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock {
        Ok(None)
    } else {
        Err(error)
    }
}

/// The number of the OpenCraft process that holds the lock in `dir`, from
/// the file. `None` if the file has no number, or if that process is not
/// OpenCraft, for example a number from an earlier start.
pub fn holder(dir: &Path) -> Option<libc::pid_t> {
    let text = std::fs::read_to_string(dir.join(FILE_NAME)).ok()?;
    let pid = text.trim().parse::<libc::pid_t>().ok().filter(|p| *p > 0)?;
    (pid != std::process::id() as libc::pid_t && process_name(pid)? == PROCESS_NAME).then_some(pid)
}

/// The name of the process `pid`, as the operating system gives it.
#[cfg(target_os = "macos")]
fn process_name(pid: libc::pid_t) -> Option<String> {
    let mut buffer = [0u8; 256];
    // SAFETY: the call writes at most `buffer.len()` bytes into `buffer`.
    let length = unsafe { libc::proc_name(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    let length = usize::try_from(length).ok().filter(|l| *l > 0)?;
    String::from_utf8(buffer[..length].to_vec()).ok()
}

/// The name of the process `pid`, as the operating system gives it.
#[cfg(not(target_os = "macos"))]
fn process_name(pid: libc::pid_t) -> Option<String> {
    let name = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    Some(name.trim_end().to_string())
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

    #[test]
    fn the_lock_file_holds_the_number_of_the_process() {
        let dir = tempfile::tempdir().unwrap();
        // A longer number from an earlier start must not stay in the file.
        std::fs::write(dir.path().join(FILE_NAME), "999999999999\n").unwrap();
        let _lock = acquire(dir.path()).unwrap().unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join(FILE_NAME)).unwrap(),
            format!("{}\n", std::process::id())
        );
    }

    #[test]
    fn only_a_different_opencraft_process_is_a_holder() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE_NAME);
        assert_eq!(holder(dir.path()), None);
        for text in ["", "not a number", "0", "-5"] {
            std::fs::write(&file, text).unwrap();
            assert_eq!(holder(dir.path()), None, "{text:?}");
        }
        // This test process is not a different process, and its name is not
        // "opencraft".
        std::fs::write(&file, format!("{}\n", std::process::id())).unwrap();
        assert_eq!(holder(dir.path()), None);
        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .unwrap();
        std::fs::write(&file, format!("{}\n", child.id())).unwrap();
        assert_eq!(
            process_name(child.id() as libc::pid_t).as_deref(),
            Some("sleep")
        );
        assert_eq!(holder(dir.path()), None);
        let _ = child.kill();
        let _ = child.wait();
    }
}

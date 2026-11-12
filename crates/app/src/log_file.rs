//! The messages of the application go to a file when the process has no
//! terminal (decision 0024).
//!
//! A start through launchd (the start at login) or through the Finder gives
//! the null device as the standard error. Then a person cannot read the
//! messages of the shell and of the engine. In that case the application
//! connects the standard error to the file `opencraft.log` in the directory
//! of the shell.

use std::{
    fs::File,
    io,
    os::fd::AsRawFd,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const FILE_NAME: &str = "opencraft.log";
/// The name of the file of the last start that made the file too large.
pub const OLD_FILE_NAME: &str = "opencraft.log.1";
/// A file that is larger than this at the start becomes the old file.
pub const MAX_SIZE: u64 = 1024 * 1024;

/// Open the log file in `dir` to add lines at its end. If the file is larger
/// than `max_size`, it first becomes the old file, and replaces the old file
/// of an earlier start.
pub fn open(dir: &Path, max_size: u64) -> io::Result<File> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(FILE_NAME);
    if std::fs::metadata(&path).is_ok_and(|data| data.len() > max_size) {
        std::fs::rename(&path, dir.join(OLD_FILE_NAME))?;
    }
    File::options().create(true).append(true).open(path)
}

/// True if the open file `fd` is the null device.
fn is_null_device(fd: libc::c_int) -> bool {
    // SAFETY: `stat` is plain data, and each call writes only into its
    // own structure. The path is a C string with a null byte at its end.
    unsafe {
        let mut of_fd: libc::stat = std::mem::zeroed();
        let mut of_null: libc::stat = std::mem::zeroed();
        libc::fstat(fd, &mut of_fd) == 0
            && libc::stat(c"/dev/null".as_ptr(), &mut of_null) == 0
            && (of_fd.st_mode & libc::S_IFMT) == libc::S_IFCHR
            && of_fd.st_rdev == of_null.st_rdev
    }
}

/// If the standard error is the null device, connect it to the log file in
/// `dir`. The result is true if the messages now go to the file. A terminal,
/// a pipe, and a file that the caller gave stay as they are.
pub fn use_if_no_output(dir: &Path) -> io::Result<bool> {
    if !is_null_device(libc::STDERR_FILENO) {
        return Ok(false);
    }
    let file = open(dir, MAX_SIZE)?;
    // SAFETY: `dup2` reads the two descriptors. The standard error keeps the
    // open file after `file` closes its own descriptor.
    if unsafe { libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(true)
}

/// The time of the system clock when the operating system started this
/// process. The time before `main` is a part of the start time
/// (`docs/kb/footprint.md`).
#[cfg(target_os = "macos")]
pub fn process_start() -> Option<Duration> {
    // SAFETY: `proc_pidinfo` writes at most `size` bytes into `info`, which
    // is plain data.
    unsafe {
        let mut info: libc::proc_bsdinfo = std::mem::zeroed();
        let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
        let written = libc::proc_pidinfo(
            libc::getpid(),
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        );
        (written == size).then(|| {
            Duration::from_secs(info.pbi_start_tvsec) + Duration::from_micros(info.pbi_start_tvusec)
        })
    }
}

#[cfg(not(target_os = "macos"))]
pub fn process_start() -> Option<Duration> {
    None
}

/// Write the first line of a start: the time of the system clock, and the
/// time from the start of the process to this call, if it is known.
pub fn write_start_line() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    match process_start().and_then(|start| now.checked_sub(start)) {
        Some(before) => eprintln!(
            "opencraft-shell {:.3} start: main began {:.0} ms after the start of the process",
            now.as_secs_f64(),
            before.as_secs_f64() * 1000.0
        ),
        None => eprintln!("opencraft-shell {:.3} start", now.as_secs_f64()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn open_adds_lines_and_keeps_one_old_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("OpenCraft");
        writeln!(open(&config, 10).unwrap(), "first").unwrap();
        writeln!(open(&config, 10).unwrap(), "second").unwrap();
        let path = config.join(FILE_NAME);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "first\nsecond\n");
        assert!(!config.join(OLD_FILE_NAME).exists());

        // 13 bytes are more than the limit of 10.
        writeln!(open(&config, 10).unwrap(), "third").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "third\n");
        assert_eq!(
            std::fs::read_to_string(config.join(OLD_FILE_NAME)).unwrap(),
            "first\nsecond\n"
        );

        // The next old file replaces the first one.
        writeln!(open(&config, 1).unwrap(), "fourth").unwrap();
        assert_eq!(
            std::fs::read_to_string(config.join(OLD_FILE_NAME)).unwrap(),
            "third\n"
        );
    }

    #[test]
    fn only_the_null_device_is_the_null_device() {
        let null = File::options().write(true).open("/dev/null").unwrap();
        assert!(is_null_device(null.as_raw_fd()));
        let dir = tempfile::tempdir().unwrap();
        let file = File::create(dir.path().join("a")).unwrap();
        assert!(!is_null_device(file.as_raw_fd()));
        assert!(!is_null_device(-1));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_process_start_is_before_now_and_not_long_ago() {
        let start = process_start().unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        assert!(start <= now);
        assert!(now - start < Duration::from_secs(3600));
    }
}

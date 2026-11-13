//! Local support text. Free-form log details are not safe to publish: they
//! can contain paths, device identities or text from an action. Keep only
//! the fixed lifecycle vocabulary and numeric measurements. Fail closed
//! when a new log message is added. No device serial number is read.

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

const MAX_BYTES: u64 = 16 * 1024;
const MAX_LINES: usize = 100;
const REMOVED: &str = "[Log details removed for privacy]";

/// Read a bounded tail, not an unbounded file. A partial first line is
/// discarded so that it cannot be mistaken for a complete safe message.
fn tail(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let start = length.saturating_sub(MAX_BYTES);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES).read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let text = if start > 0 {
        text.split_once('\n').map_or("", |(_, rest)| rest)
    } else {
        &text
    };
    let lines: Vec<_> = text.lines().rev().take(MAX_LINES).collect();
    Ok(lines.into_iter().rev().collect::<Vec<_>>().join("\n"))
}

/// Fixed engine messages only. A serial number, user name or host name in
/// arbitrary free text must not pass merely because a pattern missed it.
fn safe_message(message: &str) -> bool {
    if matches!(
        message,
        "connected" | "disconnected" | "stop: no device is connected, sent nothing"
    ) {
        return true;
    }
    for (prefix, suffix) in [
        ("full draw done: ", " surfaces"),
        ("wake detected: the computer slept ", " s"),
    ] {
        if let Some(number) = message
            .strip_prefix(prefix)
            .and_then(|s| s.strip_suffix(suffix))
        {
            return !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit() || b == b'.');
        }
    }
    false
}

/// Keep the order and the count of the last log lines. Home paths become
/// `~`; all other free-form details are omitted, not guessed at. Thus an
/// unlabelled serial number is removed too, with no identity read from USB.
fn private_log(log: &str) -> String {
    log.lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix("opencraft-engine ") {
                if let Some((time, message)) = rest.split_once(' ') {
                    if !time.is_empty()
                        && time.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                        && safe_message(message)
                    {
                        return line.to_owned();
                    }
                }
            }
            if line.contains("/Users/") || line.contains("/home/") {
                "[Log details removed for privacy; home: ~]".into()
            } else {
                REMOVED.into()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build on a worker thread. Only numeric macOS product version data is
/// requested; no system command returns the computer name or a serial.
pub fn report(path: &Path, model: &str, firmware: Option<&str>) -> String {
    let os = macos_version();
    let log = tail(path)
        .map(|text| private_log(&text))
        .unwrap_or_else(|_| "Log not available".into());
    format!("OpenCraft {}\nDevice: {model}\nFirmware: {}\nmacOS: {os}\nArchitecture: {}\n\nLog tail (up to 100 lines; free-form details removed):\n{log}\n",
        env!("CARGO_PKG_VERSION"), firmware.unwrap_or("Not available"), std::env::consts::ARCH)
}

#[cfg(target_os = "macos")]
fn macos_version() -> String {
    std::process::Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
        .unwrap_or_else(|| "Not available".into())
}

#[cfg(not(target_os = "macos"))]
fn macos_version() -> String {
    "Not available".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_text_cannot_disclose_personal_data_even_without_a_label() {
        // All identities here are fictitious test data.
        let log = "opencraft-engine 123.456 connected\nserial=FAKE-SERIAL-123\nFAKE-UNLABELLED-456\nfailed /Users/sample-user/Library/a\nfailed /home/sample-user/a\nuser sample-user on sample-host.local\nopencraft-engine 124.0 full draw done: 14 surfaces\nopencraft-engine 125.0 connected FAKE-SERIAL-123";
        let clean = private_log(log);
        for value in [
            "FAKE-SERIAL-123",
            "FAKE-UNLABELLED-456",
            "/Users/",
            "/home/",
            "sample-user",
            "sample-host",
        ] {
            assert!(!clean.contains(value));
        }
        assert!(clean.contains("home: ~"));
        assert!(clean.contains("full draw done: 14 surfaces"));
        assert_eq!(clean.lines().count(), log.lines().count());
    }

    #[test]
    fn the_tail_is_bounded_and_missing_logs_still_give_versions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log");
        let text = report(&path, "Test device", Some("V2.3"));
        for value in [
            "OpenCraft",
            "Test device",
            "V2.3",
            "macOS:",
            "Architecture:",
            "Log not available",
        ] {
            assert!(text.contains(value));
        }
        std::fs::write(&path, "opencraft-engine 123 connected\n".repeat(2000)).unwrap();
        let text = tail(&path).unwrap();
        assert_eq!(text.lines().count(), MAX_LINES);
        assert!(text.len() <= MAX_BYTES as usize);
    }
}

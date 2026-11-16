//! The real executable in the mode `ctl`: it needs no OpenCraft to give its
//! status, and it does nothing that OpenCraft does at its start.
#![cfg(unix)]

use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn opencraft(profiles: &std::path::Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opencraft"))
        .args(arguments)
        .env("OPENCRAFT_PROFILES_DIR", profiles)
        // A start of OpenCraft would find no device. The client must not look
        // for one, also without this variable.
        .env_remove("OPENCRAFT_NO_DEVICE")
        .output()
        .unwrap()
}

#[test]
fn with_no_opencraft_the_client_gives_status_3_and_touches_nothing() {
    let root = tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap();
    let profiles = root.path().join("profiles");
    let output = opencraft(&profiles, &["ctl", "state"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    // The messages go to the terminal (here a pipe), not to the log file.
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(text.contains("does not run"), "{text}");
    // No lock file, no profiles directory, no log file, and no socket: the
    // process started no engine and did not look for a device.
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn the_status_for_arguments_and_the_help() {
    let root = tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap();
    let profiles = root.path().join("profiles");
    let help = opencraft(&profiles, &["ctl", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("profile list"));
    let bad = opencraft(&profiles, &["ctl", "profile", "add"]);
    assert_eq!(bad.status.code(), Some(2));
    assert!(bad.stdout.is_empty());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn the_executable_contains_the_skill_of_its_own_version() {
    let root = tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap();
    let profiles = root.path().join("profiles");
    let output = opencraft(&profiles, &["ctl", "skill"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("---\nname: opencraft\n"), "{text}");
    // The skill needs no OpenCraft and makes no file.
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    let install = root.path().join("skills");
    let output = opencraft(
        &profiles,
        &["ctl", "skill", "--install", install.to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        std::fs::read_to_string(install.join("opencraft").join("SKILL.md")).unwrap(),
        text
    );
}

#[test]
fn a_text_argument_does_not_start_the_application() {
    let root = tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap();
    let profiles = root.path().join("profiles");
    for arguments in [vec!["--help"], vec!["-h"], vec!["help"]] {
        let output = opencraft(&profiles, &arguments);
        assert_eq!(output.status.code(), Some(0), "{arguments:?}");
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(
            text.contains(env!("CARGO_PKG_VERSION")),
            "{arguments:?}: {text}"
        );
        assert!(text.contains("opencraft ctl"), "{arguments:?}: {text}");
    }
    let output = opencraft(&profiles, &["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("opencraft {}", env!("CARGO_PKG_VERSION"))
    );
    // A first word that is not a command is a mistake, and not a start: a
    // start would take the lock, open the device, and keep the terminal.
    for arguments in [vec!["clt", "state"], vec!["state"]] {
        let output = opencraft(&profiles, &arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(output.stdout.is_empty());
        let text = String::from_utf8(output.stderr).unwrap();
        assert!(text.contains("does not know the command"), "{text}");
        assert!(text.contains("opencraft ctl --help"), "{text}");
    }
    // None of these starts made the profiles directory, the lock file, or a
    // log file.
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn status_4_names_a_running_opencraft_by_its_lock_file() {
    let root = tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap();
    let profiles = root.path().join("profiles");
    let config = root.path();
    // A process with the name of the application, which holds no lock and
    // serves no socket. The client reads only the lock file: this test shows
    // that the real path of the client gives status 4, and not status 3.
    let name = config.join("opencraft");
    std::fs::copy("/bin/sleep", &name).unwrap();
    // macOS kills a copy of a signed program that has no valid signature.
    #[cfg(target_os = "macos")]
    assert!(Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(&name)
        .status()
        .unwrap()
        .success());
    let mut child = Command::new(&name).arg("20").spawn().unwrap();
    std::fs::write(config.join("opencraft.lock"), format!("{}\n", child.id())).unwrap();
    let output = opencraft(&profiles, &["ctl", "state"]);
    let status = output.status.code();
    let text = String::from_utf8(output.stderr).unwrap();
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(status, Some(4), "{text}");
    assert!(text.contains("Allow control by other programs"), "{text}");
}

// Keep the imports of the test above honest on a system with no `/bin/sleep`.
#[allow(dead_code)]
fn _unused(_: Duration, _: Instant) {}

//! The real executable in the mode `ctl`: it needs no OpenCraft to give its
//! status, and it does nothing that OpenCraft does at its start.
#![cfg(unix)]

use std::process::{Command, Output};

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

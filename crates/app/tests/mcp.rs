//! The real executable in the mode `mcp`: it needs no OpenCraft, and it does
//! nothing that OpenCraft does at its start.
#![cfg(unix)]

use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

fn opencraft(profiles: &std::path::Path, arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_opencraft"))
        .args(arguments)
        .env("OPENCRAFT_PROFILES_DIR", profiles)
        // A start of OpenCraft would find no device. The server must not look
        // for one, also without this variable.
        .env_remove("OPENCRAFT_NO_DEVICE")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn with_no_opencraft_the_server_gives_a_tool_error_and_touches_nothing() {
    let root = tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap();
    let profiles = root.path().join("profiles");
    let mut child = Command::new(env!("CARGO_BIN_EXE_opencraft"))
        .args(["mcp"])
        .env("OPENCRAFT_PROFILES_DIR", &profiles)
        .env_remove("OPENCRAFT_NO_DEVICE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let input = child.stdin.as_mut().unwrap();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2025-11-25", "capabilities": {}}
        });
        writeln!(input, "{request}").unwrap();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {"name": "opencraft_read", "arguments": {"command": "state"}}
        });
        writeln!(input, "{request}").unwrap();
    }
    // The end of the input stops the server.
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let lines: Vec<Value> = BufReader::new(output.stdout.as_slice())
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[0]["result"]["protocolVersion"], "2025-11-25");
    let result = &lines[1]["result"];
    assert_eq!(result["isError"], true);
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("OpenCraft does not run"), "{text}");
    // The messages for a person go to the standard error, and not to the log
    // file: the server started no engine.
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // No lock file, no profiles directory, no log file, and no socket.
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn the_help_and_the_config_need_no_opencraft_and_make_no_file() {
    let root = tempfile::Builder::new()
        .prefix("oc")
        .tempdir_in("/tmp")
        .unwrap();
    let profiles = root.path().join("profiles");
    let help = opencraft(&profiles, &["mcp", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("opencraft_read"), "{text}");
    let config = opencraft(&profiles, &["mcp", "--config", "claude"]);
    assert_eq!(config.status.code(), Some(0));
    let text = String::from_utf8(config.stdout).unwrap();
    assert!(
        text.starts_with("claude mcp add --scope user opencraft -- "),
        "{text}"
    );
    assert!(text.ends_with(" mcp\n"), "{text}");
    // The short help of the executable names the server too.
    let help = opencraft(&profiles, &["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(String::from_utf8(help.stdout)
        .unwrap()
        .contains("opencraft mcp"));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

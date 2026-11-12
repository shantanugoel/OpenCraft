//! The start at login, with one LaunchAgent file (decision 0021).
//!
//! This module is for macOS only. The shell writes the file to turn the
//! option on and removes it to turn the option off. It does not call
//! `launchctl`: launchd reads the file at the next login.

use std::{
    io,
    path::{Path, PathBuf},
};

pub const FILE_NAME: &str = "opencraft.login.plist";

/// The label of the LaunchAgent.
pub const LABEL: &str = "opencraft.login";

/// The argument that the LaunchAgent gives to the executable.
pub const ARGUMENT: &str = "--login";

/// The LaunchAgents directory of the user: `~/Library/LaunchAgents`. `None`
/// if the environment has no home directory.
pub fn agents_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|home| !home.is_empty())?;
    Some(PathBuf::from(home).join("Library").join("LaunchAgents"))
}

/// The content of the file, for `executable`. Only `&`, `<`, and `>` change
/// to XML entities.
pub fn content(executable: &str) -> String {
    let executable = executable
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{executable}</string>
        <string>{ARGUMENT}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>LimitLoadToSessionType</key>
    <string>Aqua</string>
</dict>
</plist>
"#
    )
}

/// True if the option is on: the file exists in `dir`.
pub fn is_on(dir: &Path) -> bool {
    dir.join(FILE_NAME).is_file()
}

/// Turn the option on: write the file for `executable` in `dir`. The
/// function makes `dir` if it does not exist. It writes a temporary file and
/// then renames it, so that launchd never reads a part of the file.
pub fn turn_on(dir: &Path, executable: &Path) -> io::Result<()> {
    let executable = executable.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "the path of the executable is not UTF-8",
        )
    })?;
    std::fs::create_dir_all(dir)?;
    let temporary = dir.join(format!("{FILE_NAME}.tmp"));
    std::fs::write(&temporary, content(executable))?;
    std::fs::rename(&temporary, dir.join(FILE_NAME)).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })
}

/// Turn the option off: remove the file from `dir`. A missing file is not an
/// error.
pub fn turn_off(dir: &Path) -> io::Result<()> {
    match std::fs::remove_file(dir.join(FILE_NAME)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The content of decision 0021, with `EXECUTABLE` in place of the path.
    const DECISION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>opencraft.login</string>
    <key>ProgramArguments</key>
    <array>
        <string>EXECUTABLE</string>
        <string>--login</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
    <key>LimitLoadToSessionType</key>
    <string>Aqua</string>
</dict>
</plist>
"#;

    #[test]
    fn the_content_is_the_content_of_decision_0021() {
        assert_eq!(content("EXECUTABLE"), DECISION);
        let path = "/Applications/Tom & Jerry/Ä <b>/opencraft";
        assert_eq!(
            content(path),
            DECISION.replace(
                "EXECUTABLE",
                "/Applications/Tom &amp; Jerry/Ä &lt;b&gt;/opencraft"
            )
        );
    }

    #[test]
    fn on_and_off_write_and_remove_the_file() {
        let dir = tempfile::tempdir().unwrap();
        // The function makes a directory that does not exist.
        let agents = dir.path().join("Library").join("LaunchAgents");
        assert!(!is_on(&agents));
        let executable = dir.path().join("Tom & Jerry é").join("opencraft");
        turn_on(&agents, &executable).unwrap();
        assert!(is_on(&agents));
        assert_eq!(
            std::fs::read_to_string(agents.join(FILE_NAME)).unwrap(),
            content(executable.to_str().unwrap())
        );
        assert!(std::fs::read_to_string(agents.join(FILE_NAME))
            .unwrap()
            .contains("Tom &amp; Jerry é/opencraft</string>"));
        assert!(!agents.join("opencraft.login.plist.tmp").exists());
        // A second "on" replaces the file.
        turn_on(&agents, Path::new("/other/opencraft")).unwrap();
        assert!(std::fs::read_to_string(agents.join(FILE_NAME))
            .unwrap()
            .contains("<string>/other/opencraft</string>"));
        turn_off(&agents).unwrap();
        assert!(!is_on(&agents));
        // A second "off" is not an error.
        turn_off(&agents).unwrap();
        assert_eq!(std::fs::read_dir(&agents).unwrap().count(), 0);
    }

    #[test]
    fn a_directory_that_cannot_be_made_gives_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file");
        std::fs::write(&file, "").unwrap();
        assert!(turn_on(&file.join("LaunchAgents"), Path::new("/a/opencraft")).is_err());
        assert!(!is_on(&file.join("LaunchAgents")));
    }

    #[test]
    fn the_agents_directory_is_in_the_library_of_the_user() {
        if let Some(dir) = agents_dir() {
            assert!(dir.ends_with("Library/LaunchAgents"));
        }
    }
}

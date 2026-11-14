//! The ready profiles for Mac applications (issue #340).
//!
//! Each ready profile is a profile file of schema version 5 in
//! `crates/app/assets/ready`. The executable contains the files
//! (`include_str!`), as the starter profiles do (decision 0025). The shell
//! reads a file with the function of the engine, and sends the profile with
//! `Command::AddStarterProfile`. The engine writes the profile (rule 9) and
//! keeps a link only when no profile of the model already has it (decision
//! 0035, section 5).
//!
//! A ready profile uses built-in icons, has no image file, and has no path
//! of one computer. The test `each_ready_profile_loads_and_checks_its_link`
//! checks the link, the icons, and the shortcuts.

use opencraft_engine::{profile_from_text, BackupReason, Profile};

/// One ready profile: the text of its row, its link, and its profile file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadyProfile {
    /// The name of the profile, for example "Safari".
    pub name: &'static str,
    /// The item in the Profiles menu. Title case, as a menu item.
    pub menu_title: &'static str,
    /// The text under the name in the first start list.
    pub description: &'static str,
    /// The bundle identifier of the one link.
    pub bundle_id: &'static str,
    /// The application name stored with the link.
    pub application_name: &'static str,
    /// The text of the profile file.
    text: &'static str,
}

/// The ready profiles, in the order of the menu and of the first start list.
pub const READY: [ReadyProfile; 6] = [
    ReadyProfile {
        name: "Safari",
        menu_title: "Safari",
        description: "Tabs, find, and page zoom",
        bundle_id: "com.apple.Safari",
        application_name: "Safari",
        text: include_str!("../assets/ready/safari.toml"),
    },
    ReadyProfile {
        name: "Finder",
        menu_title: "Finder",
        description: "Files, views, and the sound volume",
        bundle_id: "com.apple.finder",
        application_name: "Finder",
        text: include_str!("../assets/ready/finder.toml"),
    },
    ReadyProfile {
        name: "Zoom",
        menu_title: "Zoom",
        description: "Mute, video, share, and close",
        bundle_id: "us.zoom.xos",
        application_name: "zoom.us",
        text: include_str!("../assets/ready/zoom.toml"),
    },
    ReadyProfile {
        name: "Keynote",
        menu_title: "Keynote",
        description: "Play, slides, and the arrow keys",
        bundle_id: "com.apple.iWork.Keynote",
        application_name: "Keynote",
        text: include_str!("../assets/ready/keynote.toml"),
    },
    ReadyProfile {
        name: "Xcode",
        menu_title: "Xcode",
        description: "Run, test, stop, and the navigators",
        bundle_id: "com.apple.dt.Xcode",
        application_name: "Xcode",
        text: include_str!("../assets/ready/xcode.toml"),
    },
    ReadyProfile {
        name: "Final Cut Pro",
        menu_title: "Final Cut Pro",
        description: "Play, edit, and clip volume",
        bundle_id: "com.apple.FinalCut",
        application_name: "Final Cut Pro",
        text: include_str!("../assets/ready/final-cut-pro.toml"),
    },
];

impl ReadyProfile {
    /// The profile of this ready profile.
    pub fn profile(&self) -> Result<Profile, BackupReason> {
        profile_from_text(self.text)
    }
}

/// The title of the button that opens the list, and of the menu.
pub const LIST_TITLE: &str = "Ready profiles";

/// The title of the submenu in the Profiles menu. Title case.
pub const MENU_TITLE: &str = "Ready Profiles";

/// The button text for `count` selected ready profiles.
pub fn button_text(count: usize) -> String {
    match count {
        0 => format!("{LIST_TITLE}…"),
        1 => "1 ready profile…".to_string(),
        count => format!("{count} ready profiles…"),
    }
}

#[cfg(test)]
mod tests;

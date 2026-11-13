//! The starter profiles of the first start (decision 0029, section 18).
//!
//! Each starter profile is a profile file of the current schema version in
//! `crates/app/assets/starters`. The executable contains the files
//! (`include_str!`), thus `cargo run`, the development bundle, and the
//! release have the same starter profiles, and the bundle needs no more
//! files (decision 0025). The shell reads a file with the function of the
//! engine, and sends the profile to the engine with
//! `Command::AddStarterProfile`: the engine writes the profile (rule 9).
//!
//! A starter profile uses only standard macOS applications and system
//! actions, has no image file, and has no path of one computer. The test
//! `each_starter_profile_loads_and_is_valid` checks this. Each key face and
//! each bar panel of the three profiles with keys has a title and an icon of
//! the built-in set (issue #239, decision 0033), and the test checks each
//! icon name.

use opencraft_engine::{profile_from_text, Action, BackupReason, Profile};

/// One starter profile: the text of its card, and its profile file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Starter {
    /// The name on the card, for example "Everyday Mac".
    pub name: &'static str,
    /// The text below the name on the card.
    pub description: &'static str,
    /// The text of the profile file.
    text: &'static str,
}

/// The starter profiles, in the order of the cards. The first one is
/// selected when the window opens.
pub const STARTERS: [Starter; 4] = [
    Starter {
        name: "Everyday Mac",
        description: "Apps, web pages, sleep, page change",
        text: include_str!("../assets/starters/everyday-mac.toml"),
    },
    Starter {
        name: "Media and sound",
        description: "Play, next track, volume on the knobs",
        text: include_str!("../assets/starters/media-and-sound.toml"),
    },
    Starter {
        name: "Video calls",
        description: "Mute toggle, camera, leave the call",
        text: include_str!("../assets/starters/video-calls.toml"),
    },
    Starter {
        name: "Empty",
        description: "One empty page",
        text: include_str!("../assets/starters/empty.toml"),
    },
];

impl Starter {
    /// The profile of this starter profile.
    pub fn profile(&self) -> Result<Profile, BackupReason> {
        profile_from_text(self.text)
    }
}

/// Each action of `profile`, also each action that a different action
/// contains, for example the actions of a toggle or of a sequence. The
/// function reads only the form of decision 0017, and compares no kind
/// (rule 8).
pub fn actions(profile: &Profile) -> Vec<Action> {
    let mut all = Vec::new();
    for page in profile.pages() {
        for (_, action) in page.bindings() {
            add_with_contained(action, &mut all);
        }
    }
    all
}

/// `action`, and each action in its settings, into `all`.
pub fn add_with_contained(action: &Action, all: &mut Vec<Action>) {
    all.push(action.clone());
    for value in action.settings.values() {
        if let Some(contained) = Action::from_value(value) {
            add_with_contained(&contained, all);
        } else if let Some(list) = Action::list_from_value(value) {
            for contained in &list {
                add_with_contained(contained, all);
            }
        }
    }
}

#[cfg(test)]
mod tests;

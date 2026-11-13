//! The switch by the front application (decision 0035).

use super::*;
use crate::front::{same_bundle, valid_bundle_id, MAX_LINKS};

impl<C: Connector, S: FnMut(Event), K: Clock> Engine<C, S, K> {
    /// A manual selection of the active profile. While OpenCraft is in
    /// front, the next front application brings its profile back.
    pub(super) fn note_manual_selection(&mut self) {
        self.cause = ActiveCause::Selected;
        if self.shell_in_front {
            self.chosen_in_window = true;
        }
    }

    /// [`Command::FrontApplication`].
    pub(super) fn front_application(&mut self, front: FrontApplication) {
        if matches!(front, FrontApplication::Shell) {
            self.shell_in_front = true;
            return;
        }
        // A lock can bring loginwindow to the front. That must not change
        // the profile. The same is true while the computer sleeps.
        if !self.follow_front || self.power.locked || self.power.sleeping {
            self.shell_in_front = false;
            return;
        }
        self.assign_clashes();
        let linked = match &front {
            FrontApplication::Bundle(bundle_id) => self
                .linked_profile(bundle_id)
                .map(|(profile, bundle)| (profile.to_string(), bundle.to_string())),
            FrontApplication::NoBundle | FrontApplication::Shell => None,
        };
        let (target, cause) = match linked {
            Some((profile, bundle_id)) => (profile, ActiveCause::Linked { bundle_id }),
            None => (self.default_profile_id(), ActiveCause::Default),
        };
        let select = self.chosen_in_window || self.last_target.as_deref() != Some(target.as_str());
        if select {
            if target == self.profile_id {
                if self.cause != cause {
                    self.cause = cause;
                    self.send_profiles();
                }
            } else if self.is_profile(&target) && self.save_before_leaving() {
                self.cause = cause.clone();
                self.change_profile(target.clone(), None);
            } else {
                // The profile did not change. A later notice can try again.
                self.shell_in_front = false;
                return;
            }
        }
        self.last_target = Some(target);
        self.shell_in_front = false;
        self.chosen_in_window = false;
    }

    /// [`Command::LinkApplication`].
    pub(super) fn link_application(&mut self, profile: String, link: ApplicationLink) {
        if !self.is_profile(&profile) {
            return;
        }
        let name = link.name.trim().to_string();
        if !valid_bundle_id(&link.bundle_id) || name.is_empty() {
            self.problem(
                "OpenCraft did not add the link, because the bundle identifier or the name \
                 is not correct."
                    .to_string(),
            );
            return;
        }
        let link = ApplicationLink {
            bundle_id: link.bundle_id,
            name,
        };
        if self
            .profile_links(&profile)
            .iter()
            .any(|each| same_bundle(&each.bundle_id, &link.bundle_id))
        {
            return;
        }
        if self.profile_links(&profile).len() >= MAX_LINKS {
            self.problem(
                "OpenCraft did not add the link, because a profile can have 32 links.".to_string(),
            );
            return;
        }
        let owners: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| {
                entry.id != profile
                    && entry
                        .applications
                        .iter()
                        .any(|each| same_bundle(&each.bundle_id, &link.bundle_id))
            })
            .map(|entry| entry.id.clone())
            .collect();
        let mut links = self.profile_links(&profile);
        links.push(link.clone());
        if !self.write_links(&profile, links) {
            return;
        }
        for owner in owners {
            let mut links = self.profile_links(&owner);
            links.retain(|each| !same_bundle(&each.bundle_id, &link.bundle_id));
            let _ = self.write_links(&owner, links);
        }
        self.send_profiles();
    }

    /// [`Command::UnlinkApplication`].
    pub(super) fn unlink_application(&mut self, profile: String, bundle_id: String) {
        if !self.is_profile(&profile) {
            return;
        }
        let mut links = self.profile_links(&profile);
        let before = links.len();
        links.retain(|link| !same_bundle(&link.bundle_id, &bundle_id));
        if links.len() == before {
            return;
        }
        if self.write_links(&profile, links) {
            self.send_profiles();
        }
    }

    /// [`Command::SetDefaultProfile`].
    pub(super) fn set_default_profile(&mut self, profile: String) {
        if !self.entries.iter().any(|entry| entry.id == profile) {
            self.problem(format!(
                "OpenCraft did not change the default profile, because no profile has the \
                 identity '{profile}'."
            ));
            return;
        }
        if self.default_mark.as_deref() == Some(profile.as_str()) {
            return;
        }
        self.default_mark = Some(profile.clone());
        if let Err(error) = self.profiles.set_default(self.descriptor.model, &profile) {
            self.problem(format!(
                "OpenCraft could not record the default profile: {error}"
            ));
        }
        self.send_profiles();
    }

    /// [`Command::SetFollowFrontApplication`].
    pub(super) fn set_follow_front(&mut self, follow: bool) {
        if self.follow_front == follow {
            return;
        }
        self.follow_front = follow;
        if follow {
            self.last_target = None;
        }
        if let Err(error) = self.profiles.set_follow(follow) {
            self.problem(format!("OpenCraft could not record the setting: {error}"));
        }
        self.send_follow();
    }

    /// Remove each link of `profile` that a profile of this model already
    /// has, and tell the shell. `profile` is not in the list yet.
    pub(super) fn drop_clashing_links(&mut self, profile: &mut Profile) {
        self.assign_clashes();
        let mut keep = Vec::new();
        for link in profile.applications.drain(..) {
            let owner = self
                .linked_profile(&link.bundle_id)
                .map(|(id, _)| id.to_string());
            if let Some(id) = owner {
                let name = self
                    .entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .map(|entry| entry.name.clone())
                    .unwrap_or(id);
                self.problem(format!("\"{}\" stays linked to \"{name}\".", link.name));
            } else {
                keep.push(link);
            }
        }
        profile.applications = keep;
    }

    /// The bundle identifiers that do not apply, from the order of the
    /// identities. The first profile that links an application keeps it.
    pub(super) fn assign_clashes(&mut self) {
        let mut seen: Vec<(String, String)> = Vec::new();
        for entry in &mut self.entries {
            let mut clashes = Vec::new();
            for link in &entry.applications {
                let key = link.bundle_id.to_ascii_lowercase();
                let earlier = seen
                    .iter()
                    .any(|(seen_key, owner)| seen_key == &key && owner != &entry.id);
                if earlier {
                    clashes.push(link.bundle_id.clone());
                } else if !seen.iter().any(|(seen_key, _)| seen_key == &key) {
                    seen.push((key, entry.id.clone()));
                }
            }
            entry.clashes = clashes;
        }
    }

    /// The identity of the default profile (decision 0035, section 6).
    pub(super) fn default_profile_id(&self) -> String {
        if let Some(id) = &self.default_mark {
            if self.entries.iter().any(|entry| &entry.id == id) {
                return id.clone();
            }
        }
        if let Some(entry) = self
            .entries
            .iter()
            .find(|entry| !self.has_applicable_link(entry))
        {
            return entry.id.clone();
        }
        self.entries
            .first()
            .map(|entry| entry.id.clone())
            .unwrap_or_else(|| self.profile_id.clone())
    }

    /// The profile that links `bundle_id`, and the bundle identifier as that
    /// profile stores it. A link in `clashes` does not apply.
    fn linked_profile(&self, bundle_id: &str) -> Option<(&str, &str)> {
        for entry in &self.entries {
            for link in &entry.applications {
                if same_bundle(&link.bundle_id, bundle_id)
                    && !entry
                        .clashes
                        .iter()
                        .any(|clash| same_bundle(clash, bundle_id))
                {
                    return Some((entry.id.as_str(), link.bundle_id.as_str()));
                }
            }
        }
        None
    }

    fn has_applicable_link(&self, entry: &ProfileEntry) -> bool {
        entry.applications.iter().any(|link| {
            !entry
                .clashes
                .iter()
                .any(|clash| same_bundle(clash, &link.bundle_id))
        })
    }

    fn profile_links(&self, id: &str) -> Vec<ApplicationLink> {
        if id == self.profile_id {
            self.profile.applications.clone()
        } else {
            self.entries
                .iter()
                .find(|entry| entry.id == id)
                .map(|entry| entry.applications.clone())
                .unwrap_or_default()
        }
    }

    /// Put `applications` on the profile `id`. An inactive profile is
    /// written at once. The active profile is written after the usual delay.
    fn write_links(&mut self, id: &str, applications: Vec<ApplicationLink>) -> bool {
        if id == self.profile_id {
            if self.profile.applications == applications {
                return false;
            }
            self.profile.applications = applications;
            self.changed_at = Some(self.now());
            self.update_entry();
            return true;
        }
        let store = self.profiles.store(id);
        match store.read().and_then(|mut profile| {
            profile.applications = applications;
            store.save(&profile)?;
            Ok(profile)
        }) {
            Ok(profile) => {
                if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
                    *entry = ProfileEntry::of(id, &profile);
                }
                true
            }
            Err(error) => {
                self.problem(format!("OpenCraft could not change the links: {error}"));
                false
            }
        }
    }
}

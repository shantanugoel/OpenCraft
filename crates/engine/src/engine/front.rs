//! The switch by the front application (decision 0035).

use super::*;
use crate::front::valid_bundle_id;
use crate::profile_list::LinkError;

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
        let owner = match &front {
            FrontApplication::Bundle(bundle_id) => self.list.owner_of(bundle_id),
            FrontApplication::NoBundle | FrontApplication::Shell => None,
        };
        let (target, cause) = match owner {
            Some(owner) => (
                owner.profile,
                ActiveCause::Linked {
                    bundle_id: owner.bundle_id,
                },
            ),
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
                // The clock starts after the save of the old profile, and
                // stops when the change of the profile returns: after the
                // device writes, the events, and the write of `state.toml`.
                // Only a measurement writes the line (`switch_trace`).
                self.last_written = 0;
                let started = std::time::Instant::now();
                self.change_profile(target.clone(), None);
                if self.switch_trace
                    && self.link.is_connected()
                    && self.power.state == crate::power::DisplayState::Awake
                {
                    super::log(&format!(
                        "switch-trace front application: the profile is on the device after \
                         {} ms, {} images",
                        started.elapsed().as_millis(),
                        self.last_written
                    ));
                }
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
        let change = match self.list.link(&profile, &link) {
            Ok(change) => change,
            Err(LinkError::TooMany) => {
                self.problem(
                    "OpenCraft did not add the link, because a profile can have 32 links."
                        .to_string(),
                );
                return;
            }
        };
        if change.is_empty() {
            return;
        }
        if let Some(add) = change.add {
            if !self.write_links(&add.profile, add.links) {
                return;
            }
        }
        for release in change.release {
            let _ = self.write_links(&release.profile, release.links);
        }
        self.send_profiles();
    }

    /// [`Command::UnlinkApplication`].
    pub(super) fn unlink_application(&mut self, profile: String, bundle_id: String) {
        if !self.is_profile(&profile) {
            return;
        }
        if let Some(write) = self.list.unlink(&profile, &bundle_id) {
            if self.write_links(&write.profile, write.links) {
                self.send_profiles();
            }
        }
    }

    /// [`Command::SetDefaultProfile`].
    pub(super) fn set_default_profile(&mut self, profile: String) {
        if !self.list.contains(&profile) {
            self.problem(format!(
                "OpenCraft did not change the default profile, because no profile has the \
                 identity “{profile}”."
            ));
            return;
        }
        if !self.list.set_default_mark(&profile) {
            return;
        }
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
        let mut keep = Vec::new();
        for link in profile.applications.drain(..) {
            if let Some(owner) = self.list.owner_of(&link.bundle_id) {
                let id = owner.profile;
                let name = self
                    .list
                    .get(&id)
                    .map(|entry| entry.name.clone())
                    .unwrap_or(id);
                self.problem(format!("“{}” stays linked to “{name}”.", link.name));
            } else {
                keep.push(link);
            }
        }
        profile.applications = keep;
    }

    /// The identity of the default profile (decision 0035, section 6).
    pub(super) fn default_profile_id(&self) -> String {
        self.list
            .default_id()
            .map(str::to_string)
            .unwrap_or_else(|| self.profile_id.clone())
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
                if self.list.contains(id) {
                    self.list.upsert(ProfileEntry::of(id, &profile));
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

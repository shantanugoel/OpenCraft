//! The profile list: the entries of the profiles of one device model, and the
//! default mark (decision 0035).
//!
//! This module is pure. It reads and writes no file. It holds the rules for
//! the links of the profiles to applications:
//!
//! - **Clash order.** The entries are in the order of their identities. If
//!   more than one profile links the same application, the first profile
//!   keeps the link, and the link of each later profile does not apply.
//! - **Match of bundle identifiers.** A bundle identifier matches with no
//!   difference between small and capital letters ([`same_bundle`]).
//! - **Owner of a link.** The owner of a link is the profile whose link of
//!   that application applies.
//! - **Default profile.** The default mark if it names a profile in the list.
//!   Else the first profile with no link that applies. Else the first
//!   profile.
//!
//! The list gives the result of these rules in each [`ProfileEntry`]: the
//! links that apply (`applied`), and the owner of each link
//! (`link_owners`). The list computes them again after each change of an
//! entry. Thus a reader does not find the rules again.
//!
//! A change of a link gives a [`LinkChange`]: the profiles whose files the
//! engine must write. The engine writes the files, and then puts the new
//! entries in the list.

use crate::front::{same_bundle, MAX_LINKS};
use crate::profile::ApplicationLink;
use crate::profiles::{LinkOwner, ProfileEntry};

/// The new links of one profile. The engine writes them to the file of the
/// profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkWrite {
    /// The identity of the profile.
    pub profile: String,
    /// All links of the profile after the change, in their order.
    pub links: Vec<ApplicationLink>,
}

/// The files that a new link needs the engine to write.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkChange {
    /// The profile that gets the link, with all its links. `None` if the
    /// profile has the link already.
    pub add: Option<LinkWrite>,
    /// The other profiles that have the same application. They lose the
    /// link, so that the application has one owner.
    pub release: Vec<LinkWrite>,
}

impl LinkChange {
    /// True if no file changes.
    pub fn is_empty(&self) -> bool {
        self.add.is_none() && self.release.is_empty()
    }
}

/// Why the list does not make a change of a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkError {
    /// The profile has [`MAX_LINKS`] links already.
    TooMany,
}

/// The profiles of one device model, in the order of their identities, and
/// the default mark.
#[derive(Debug, Clone, Default)]
pub struct ProfileList {
    entries: Vec<ProfileEntry>,
    default_mark: Option<String>,
}

impl ProfileList {
    /// A list of `entries` and the default mark `default_mark`. The list
    /// puts the entries in the order of their identities.
    pub fn new(mut entries: Vec<ProfileEntry>, default_mark: Option<String>) -> Self {
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        let mut list = Self {
            entries,
            default_mark,
        };
        list.refresh();
        list
    }

    /// The entries, in the order of the identities.
    pub fn entries(&self) -> &[ProfileEntry] {
        &self.entries
    }

    /// The number of profiles.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if the list has no profile.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry with the identity `id`.
    pub fn get(&self, id: &str) -> Option<&ProfileEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// True if the list has a profile with the identity `id`.
    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// The position of the profile `id` in the list.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.entries.iter().position(|entry| entry.id == id)
    }

    /// Put `entry` in the list. It replaces the entry with the same
    /// identity, or goes to its place in the order of the identities.
    pub fn upsert(&mut self, entry: ProfileEntry) {
        match self.entries.binary_search_by(|e| e.id.cmp(&entry.id)) {
            Ok(at) => self.entries[at] = entry,
            Err(at) => self.entries.insert(at, entry),
        }
        self.refresh();
    }

    /// Remove the profile `id`. If it is the default profile, the list drops
    /// the default mark. Return the position of the profile and whether it
    /// was the default profile, or `None` if the list does not have it.
    pub fn remove(&mut self, id: &str) -> Option<Removal> {
        let at = self.position(id)?;
        self.entries.remove(at);
        let was_default = self.default_mark.as_deref() == Some(id);
        if was_default {
            self.default_mark = None;
        }
        self.refresh();
        Some(Removal { at, was_default })
    }

    /// Give the profile `id` the name `name`. Return true if the name
    /// changed.
    pub fn rename(&mut self, id: &str, name: &str) -> bool {
        match self.entries.iter_mut().find(|entry| entry.id == id) {
            Some(entry) if entry.name != name => {
                entry.name = name.to_string();
                true
            }
            _ => false,
        }
    }

    /// The entries with no entry for the profile `id`.
    pub fn entries_without(&self, id: &str) -> Vec<ProfileEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.id != id)
            .cloned()
            .collect()
    }

    /// Set the default mark to the profile `id`. Return false, and change
    /// nothing, if the list has no such profile or the mark is `id` already.
    pub fn set_default_mark(&mut self, id: &str) -> bool {
        if !self.contains(id) || self.default_mark.as_deref() == Some(id) {
            return false;
        }
        self.default_mark = Some(id.to_string());
        true
    }

    /// Make the default profile of now the default mark, if the list has no
    /// mark. Return the identity if the mark is new, else `None`. A profile
    /// that the user adds must not take the place of a default profile that
    /// the list only derives (the first profile in the order of the
    /// identities).
    pub fn pin_default(&mut self) -> Option<String> {
        if self
            .default_mark
            .as_deref()
            .is_some_and(|m| self.contains(m))
        {
            return None;
        }
        // Only a default profile of step 2 of decision 0035, section 6, is a
        // choice of the list: a profile with no link that applies. A default
        // profile of step 3 (a link applies on each profile) is not a choice:
        // the profile that the user adds has no link, and step 2 then finds
        // it. A mark here would give the new profile the wrong default.
        if self.entries.iter().all(|entry| !entry.applied.is_empty()) {
            return None;
        }
        let id = self.default_id()?.to_string();
        self.default_mark = Some(id.clone());
        Some(id)
    }

    /// The identity of the default profile: the default mark if the list has
    /// that profile, else the first profile with no link that applies, else
    /// the first profile. `None` if the list is empty.
    pub fn default_id(&self) -> Option<&str> {
        if let Some(entry) = self.default_mark.as_deref().and_then(|mark| self.get(mark)) {
            return Some(&entry.id);
        }
        self.entries
            .iter()
            .find(|entry| entry.applied.is_empty())
            .or_else(|| self.entries.first())
            .map(|entry| entry.id.as_str())
    }

    /// The owner of the link to `bundle_id`: the profile whose link of this
    /// application applies. `None` if no profile links it.
    pub fn owner_of(&self, bundle_id: &str) -> Option<LinkOwner> {
        self.entries.iter().find_map(|entry| {
            entry
                .applied
                .iter()
                .find(|link| same_bundle(&link.bundle_id, bundle_id))
                .map(|link| LinkOwner {
                    bundle_id: link.bundle_id.clone(),
                    profile: entry.id.clone(),
                })
        })
    }

    /// The change of the files for the new link `link` of the profile
    /// `profile`.
    ///
    /// - The profile has the link, and it applies: no change.
    /// - The profile has the link, and it does not apply: each other profile
    ///   that has the application loses the link. The profile gets the
    ///   ownership.
    /// - The profile does not have the link: the profile gets it, and each
    ///   other profile that has the application loses it. If the profile
    ///   has the maximum count of links, the result is
    ///   [`LinkError::TooMany`].
    ///
    /// A profile that is not in the list gives no change.
    pub fn link(&self, profile: &str, link: &ApplicationLink) -> Result<LinkChange, LinkError> {
        let Some(entry) = self.get(profile) else {
            return Ok(LinkChange::default());
        };
        let has = entry
            .applications
            .iter()
            .any(|each| same_bundle(&each.bundle_id, &link.bundle_id));
        let applies = entry
            .applied
            .iter()
            .any(|each| same_bundle(&each.bundle_id, &link.bundle_id));
        if has && applies {
            return Ok(LinkChange::default());
        }
        if !has && entry.applications.len() >= MAX_LINKS {
            return Err(LinkError::TooMany);
        }
        let add = (!has).then(|| {
            let mut links = entry.applications.clone();
            links.push(link.clone());
            LinkWrite {
                profile: profile.to_string(),
                links,
            }
        });
        let release = self
            .entries
            .iter()
            .filter(|other| {
                other.id != profile
                    && other
                        .applications
                        .iter()
                        .any(|each| same_bundle(&each.bundle_id, &link.bundle_id))
            })
            .map(|other| LinkWrite {
                profile: other.id.clone(),
                links: other
                    .applications
                    .iter()
                    .filter(|each| !same_bundle(&each.bundle_id, &link.bundle_id))
                    .cloned()
                    .collect(),
            })
            .collect();
        Ok(LinkChange { add, release })
    }

    /// The change of the file for the removal of the link to `bundle_id`
    /// from the profile `profile`. `None` if the profile does not have the
    /// link.
    pub fn unlink(&self, profile: &str, bundle_id: &str) -> Option<LinkWrite> {
        let entry = self.get(profile)?;
        let links: Vec<ApplicationLink> = entry
            .applications
            .iter()
            .filter(|link| !same_bundle(&link.bundle_id, bundle_id))
            .cloned()
            .collect();
        (links.len() != entry.applications.len()).then(|| LinkWrite {
            profile: profile.to_string(),
            links,
        })
    }

    /// Compute `applied` and `link_owners` of each entry again.
    fn refresh(&mut self) {
        // The first owner of each application, from the order of the entries.
        let mut owners: Vec<(String, String)> = Vec::new();
        for entry in &mut self.entries {
            let mut applied = Vec::new();
            let mut link_owners = Vec::new();
            for link in &entry.applications {
                let owner = match owners
                    .iter()
                    .find(|(bundle_id, _)| same_bundle(bundle_id, &link.bundle_id))
                {
                    Some((_, owner)) => owner.clone(),
                    None => {
                        owners.push((link.bundle_id.clone(), entry.id.clone()));
                        entry.id.clone()
                    }
                };
                if owner == entry.id {
                    applied.push(link.clone());
                }
                link_owners.push(LinkOwner {
                    bundle_id: link.bundle_id.clone(),
                    profile: owner,
                });
            }
            entry.applied = applied;
            entry.link_owners = link_owners;
        }
    }
}

/// The result of [`ProfileList::remove`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Removal {
    /// The position that the profile had in the list.
    pub at: usize,
    /// True if the profile was the default profile by the default mark.
    pub was_default: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(bundle_id: &str, name: &str) -> ApplicationLink {
        ApplicationLink {
            bundle_id: bundle_id.into(),
            name: name.into(),
        }
    }

    fn entry(id: &str, links: &[&str]) -> ProfileEntry {
        ProfileEntry {
            id: id.into(),
            name: id.to_uppercase(),
            applications: links.iter().map(|bundle| link(bundle, bundle)).collect(),
            ..Default::default()
        }
    }

    fn list(entries: &[(&str, &[&str])], mark: Option<&str>) -> ProfileList {
        ProfileList::new(
            entries.iter().map(|(id, links)| entry(id, links)).collect(),
            mark.map(str::to_string),
        )
    }

    fn bundles(links: &[ApplicationLink]) -> Vec<&str> {
        links.iter().map(|link| link.bundle_id.as_str()).collect()
    }

    #[test]
    fn the_first_profile_in_the_order_of_the_identities_keeps_a_link() {
        // The entries come in any order: the list sorts them.
        let list = list(
            &[
                ("work", &["com.example.x", "com.example.y"]),
                ("calls", &["com.example.x"]),
                ("home", &["com.example.y", "com.example.z"]),
            ],
            None,
        );
        let ids: Vec<&str> = list.entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["calls", "home", "work"]);
        let [calls, home, work] = list.entries() else {
            unreachable!()
        };
        assert_eq!(bundles(&calls.applied), ["com.example.x"]);
        assert_eq!(bundles(&home.applied), ["com.example.y", "com.example.z"]);
        assert!(work.applied.is_empty());
        let owners: Vec<(&str, &str)> = work
            .link_owners
            .iter()
            .map(|owner| (owner.bundle_id.as_str(), owner.profile.as_str()))
            .collect();
        assert_eq!(
            owners,
            [("com.example.x", "calls"), ("com.example.y", "home")]
        );
    }

    #[test]
    fn a_bundle_identifier_matches_with_no_difference_between_small_and_capital_letters() {
        let list = list(&[("a", &["US.ZOOM.XOS"]), ("b", &["us.zoom.xos"])], None);
        let [a, b] = list.entries() else {
            unreachable!()
        };
        assert_eq!(a.applied.len(), 1);
        assert!(b.applied.is_empty());
        assert_eq!(b.link_owners[0].profile, "a");
        // The owner has the case that its profile stores.
        let owner = list.owner_of("Us.Zoom.Xos").unwrap();
        assert_eq!(owner.profile, "a");
        assert_eq!(owner.bundle_id, "US.ZOOM.XOS");
    }

    #[test]
    fn a_repeated_link_inside_one_profile_is_not_a_clash() {
        let list = list(&[("a", &["com.example.x", "COM.EXAMPLE.X"])], None);
        assert_eq!(list.entries()[0].applied.len(), 2);
    }

    #[test]
    fn the_owner_of_a_link_is_the_profile_whose_link_applies() {
        let list = list(
            &[
                ("a", &["com.example.x"]),
                ("b", &["com.example.x", "com.example.y"]),
            ],
            None,
        );
        assert_eq!(list.owner_of("com.example.x").unwrap().profile, "a");
        assert_eq!(list.owner_of("com.example.y").unwrap().profile, "b");
        assert_eq!(list.owner_of("com.example.none"), None);
        let empty = ProfileList::default();
        assert_eq!(empty.owner_of("com.example.x"), None);
    }

    #[test]
    fn the_default_profile_is_the_mark_then_a_profile_with_no_link_then_the_first() {
        let entries: &[(&str, &[&str])] = &[
            ("a", &["com.example.x"]),
            ("b", &["com.example.x"]),
            ("c", &[]),
        ];
        // The mark wins, also over a profile with no link.
        assert_eq!(list(entries, Some("c")).default_id(), Some("c"));
        assert_eq!(list(entries, Some("a")).default_id(), Some("a"));
        // A mark for a profile that is not in the list does not count.
        assert_eq!(list(entries, Some("gone")).default_id(), Some("b"));
        // No mark: the first profile with no link that applies. The link of
        // "b" does not apply, because "a" has the same link. Thus "b" is that
        // profile, and "c" is not.
        assert_eq!(list(entries, None).default_id(), Some("b"));
        let all_linked: &[(&str, &[&str])] =
            &[("a", &["com.example.x"]), ("b", &["com.example.y"])];
        assert_eq!(list(all_linked, None).default_id(), Some("a"));
        let clash_only: &[(&str, &[&str])] =
            &[("a", &["com.example.x"]), ("b", &["com.example.x"])];
        assert_eq!(list(clash_only, None).default_id(), Some("b"));
        assert_eq!(ProfileList::default().default_id(), None);
    }

    #[test]
    fn the_default_mark_names_a_profile_in_the_list_and_goes_with_it() {
        let mut list = list(&[("a", &[]), ("b", &[])], None);
        assert!(!list.set_default_mark("gone"));
        assert!(list.set_default_mark("b"));
        assert!(!list.set_default_mark("b"), "no change");
        assert_eq!(list.default_id(), Some("b"));
        assert_eq!(
            list.remove("b"),
            Some(Removal {
                at: 1,
                was_default: true
            })
        );
        assert_eq!(list.default_id(), Some("a"));
        assert_eq!(list.remove("b"), None);
        assert_eq!(
            list.remove("a"),
            Some(Removal {
                at: 0,
                was_default: false
            })
        );
        assert!(list.is_empty());
    }

    #[test]
    fn a_change_of_an_entry_gives_new_clashes() {
        let mut list = list(
            &[("a", &["com.example.x"]), ("b", &["com.example.x"])],
            None,
        );
        assert!(list.get("b").unwrap().applied.is_empty());
        list.upsert(entry("a", &[]));
        assert_eq!(list.get("b").unwrap().applied.len(), 1);
        assert_eq!(list.owner_of("com.example.x").unwrap().profile, "b");
        // A new entry goes to its place in the order of the identities.
        list.upsert(entry("aa", &["com.example.x"]));
        let ids: Vec<&str> = list.entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["a", "aa", "b"]);
        assert_eq!(list.owner_of("com.example.x").unwrap().profile, "aa");
        list.remove("aa");
        assert_eq!(list.owner_of("com.example.x").unwrap().profile, "b");
    }

    #[test]
    fn a_rename_changes_the_name_only() {
        let mut list = list(&[("a", &["com.example.x"])], None);
        assert!(list.rename("a", "Other"));
        assert!(!list.rename("a", "Other"));
        assert!(!list.rename("gone", "Other"));
        assert_eq!(list.get("a").unwrap().name, "Other");
        assert_eq!(list.entries_without("a").len(), 0);
    }

    #[test]
    fn a_new_link_goes_to_the_profile_and_leaves_each_other_profile() {
        let list = list(
            &[
                ("a", &["com.example.x", "com.example.y"]),
                ("b", &["com.example.x"]),
                ("c", &[]),
            ],
            None,
        );
        let change = list.link("c", &link("COM.example.X", "X")).unwrap();
        let add = change.add.unwrap();
        assert_eq!(add.profile, "c");
        assert_eq!(bundles(&add.links), ["COM.example.X"]);
        let release: Vec<(&str, Vec<&str>)> = change
            .release
            .iter()
            .map(|write| (write.profile.as_str(), bundles(&write.links)))
            .collect();
        assert_eq!(
            release,
            [("a", vec!["com.example.y"]), ("b", Vec::<&str>::new())]
        );
    }

    #[test]
    fn a_link_that_applies_gives_no_change() {
        let list = list(
            &[("a", &["com.example.x"]), ("b", &["com.example.x"])],
            None,
        );
        let change = list.link("a", &link("com.example.x", "X")).unwrap();
        assert!(change.is_empty());
        // A profile that is not in the list gives no change.
        assert!(list
            .link("gone", &link("com.example.x", "X"))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_link_that_does_not_apply_moves_to_the_profile_that_has_it() {
        let list = list(
            &[("a", &["com.example.x"]), ("b", &["com.example.x"])],
            None,
        );
        let change = list.link("b", &link("com.example.x", "X")).unwrap();
        assert_eq!(change.add, None, "b has the link in its file already");
        assert_eq!(change.release.len(), 1);
        assert_eq!(change.release[0].profile, "a");
        assert!(change.release[0].links.is_empty());
    }

    #[test]
    fn a_profile_can_have_a_maximum_count_of_links() {
        let names: Vec<String> = (0..MAX_LINKS)
            .map(|n| format!("com.example.n{n}"))
            .collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let list = list(&[("a", &refs), ("b", &[])], None);
        assert_eq!(
            list.link("a", &link("com.example.new", "New")),
            Err(LinkError::TooMany)
        );
        assert!(list.link("b", &link("com.example.new", "New")).is_ok());
        // A link that the profile has gives no error.
        assert!(list
            .link("a", &link("com.example.n3", "N"))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn an_unlink_gives_the_links_that_stay() {
        let list = list(&[("a", &["com.example.x", "com.example.y"])], None);
        let write = list.unlink("a", "COM.EXAMPLE.X").unwrap();
        assert_eq!(write.profile, "a");
        assert_eq!(bundles(&write.links), ["com.example.y"]);
        assert_eq!(list.unlink("a", "com.example.none"), None);
        assert_eq!(list.unlink("gone", "com.example.x"), None);
    }
}

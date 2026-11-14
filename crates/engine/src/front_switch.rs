//! The state machine of the automatic switch (decision 0035, sections 5 to 8).
//!
//! This module is pure. It reads and writes no file and no device. It holds
//! four values: whether the shell is in front, the last target, the choice in
//! the window, and the setting "Change the profile with the front
//! application". A front application notice and the situation of the engine
//! go in ([`FrontSwitch::notice`]). A [`Decision`] comes out. The engine
//! carries out the decision.
//!
//! The target comes from the profile list ([`ProfileList`]): the owner of the
//! link to the front application, else the default profile.

use crate::front::{ActiveCause, FrontApplication};
use crate::profile_list::ProfileList;

/// What the engine does after a front application notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Change nothing.
    Nothing,
    /// The active profile stays. Only the cause of the active profile
    /// changes. The engine sends the profile list again.
    SetCause(ActiveCause),
    /// Select this profile, with this cause. If the engine cannot leave the
    /// active profile, it calls [`FrontSwitch::select_failed`].
    Select { profile: String, cause: ActiveCause },
}

/// The facts that the machine needs for one notice.
#[derive(Debug, Clone, Copy)]
pub struct Situation<'a> {
    /// The profiles of the model of the device.
    pub list: &'a ProfileList,
    /// The identity of the active profile.
    pub active: &'a str,
    /// Why the active profile is active.
    pub cause: &'a ActiveCause,
    /// True if the screen is locked.
    pub locked: bool,
    /// True if the computer sleeps.
    pub sleeping: bool,
}

/// The state of the automatic switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontSwitch {
    /// The setting of section 7. A missing key in `state.toml` is true.
    follow: bool,
    /// True if the last notice was the shell itself.
    shell_in_front: bool,
    /// The profile that the rule gave for the last front application that is
    /// not OpenCraft. Empty at the start, and after the setting goes on.
    last_target: Option<String>,
    /// True after a manual selection while OpenCraft is in front, until the
    /// next front application that is not OpenCraft.
    chosen_in_window: bool,
    /// The values before the last decision to select: see
    /// [`Self::select_failed`].
    before_select: Option<(Option<String>, bool)>,
}

impl FrontSwitch {
    /// A machine at the start, with the setting `follow`.
    pub fn new(follow: bool) -> Self {
        Self {
            follow,
            shell_in_front: false,
            last_target: None,
            chosen_in_window: false,
            before_select: None,
        }
    }

    /// The setting "Change the profile with the front application".
    pub fn follow(&self) -> bool {
        self.follow
    }

    /// Change the setting. Return true if it changed. After the setting goes
    /// on, the next notice applies the rule with no last target.
    pub fn set_follow(&mut self, follow: bool) -> bool {
        if self.follow == follow {
            return false;
        }
        self.follow = follow;
        if follow {
            self.last_target = None;
        }
        true
    }

    /// Start again, with the setting `follow`.
    pub fn restart(&mut self, follow: bool) {
        *self = Self::new(follow);
    }

    /// A manual selection of the active profile. While OpenCraft is in
    /// front, the next front application brings its profile back.
    pub fn manual_selection(&mut self) {
        if self.shell_in_front {
            self.chosen_in_window = true;
        }
    }

    /// The engine could not leave the active profile for the profile of the
    /// last [`Decision::Select`]. The machine goes back to the values before
    /// that decision, so that a later notice can try again.
    pub fn select_failed(&mut self) {
        if let Some((last_target, chosen_in_window)) = self.before_select.take() {
            self.last_target = last_target;
            self.chosen_in_window = chosen_in_window;
        }
    }

    /// The rule of decision 0035, section 8, for a notice of `front`.
    pub fn notice(&mut self, front: &FrontApplication, situation: Situation<'_>) -> Decision {
        self.before_select = None;
        // Rule 1: the shell itself. The device keeps its profile.
        if matches!(front, FrontApplication::Shell) {
            self.shell_in_front = true;
            return Decision::Nothing;
        }
        // Rule 2: a lock can bring loginwindow to the front. That must not
        // change the profile. The same is true while the computer sleeps.
        if !self.follow || situation.locked || situation.sleeping {
            self.shell_in_front = false;
            return Decision::Nothing;
        }
        // Rule 3: the target.
        let owner = match front {
            FrontApplication::Bundle(bundle_id) => situation.list.owner_of(bundle_id),
            FrontApplication::NoBundle | FrontApplication::Shell => None,
        };
        let (target, cause) = match owner {
            Some(owner) => (
                owner.profile,
                ActiveCause::Linked {
                    bundle_id: owner.bundle_id,
                },
            ),
            None => (
                situation
                    .list
                    .default_id()
                    .unwrap_or(situation.active)
                    .to_string(),
                ActiveCause::Default,
            ),
        };
        // Rule 4: select if the user chose in the window, or the target is new.
        let select = self.chosen_in_window || self.last_target.as_deref() != Some(target.as_str());
        let decision = if !select {
            Decision::Nothing
        } else if target == situation.active {
            if *situation.cause == cause {
                Decision::Nothing
            } else {
                Decision::SetCause(cause)
            }
        } else {
            self.before_select = Some((self.last_target.clone(), self.chosen_in_window));
            Decision::Select {
                profile: target.clone(),
                cause,
            }
        };
        // Rule 5.
        self.last_target = Some(target);
        self.shell_in_front = false;
        self.chosen_in_window = false;
        decision
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ApplicationLink;
    use crate::profiles::ProfileEntry;

    const ZOOM: &str = "us.zoom.xos";
    const SAFARI: &str = "com.apple.Safari";
    const TERMINAL: &str = "com.apple.Terminal";

    fn entry(id: &str, links: &[&str]) -> ProfileEntry {
        ProfileEntry {
            id: id.into(),
            name: id.to_uppercase(),
            applications: links
                .iter()
                .map(|bundle| ApplicationLink {
                    bundle_id: (*bundle).into(),
                    name: (*bundle).into(),
                })
                .collect(),
            ..Default::default()
        }
    }

    /// `calls` links Zoom. `music` and `home` have no link. `home` is the
    /// default profile. The order of the identities is calls, home, music.
    fn list() -> ProfileList {
        ProfileList::new(
            vec![
                entry("calls", &[ZOOM]),
                entry("home", &[]),
                entry("music", &[]),
            ],
            Some("home".into()),
        )
    }

    fn bundle(id: &str) -> FrontApplication {
        FrontApplication::Bundle(id.into())
    }

    fn linked(bundle_id: &str) -> ActiveCause {
        ActiveCause::Linked {
            bundle_id: bundle_id.into(),
        }
    }

    fn select(profile: &str, cause: ActiveCause) -> Decision {
        Decision::Select {
            profile: profile.into(),
            cause,
        }
    }

    /// One step of a table test.
    enum Step {
        /// A notice of the front application. The decision is expected.
        Front(FrontApplication, Decision),
        /// The same, with the screen locked.
        FrontLocked(FrontApplication, Decision),
        /// The same, with the computer asleep.
        FrontAsleep(FrontApplication, Decision),
        /// A manual selection of this profile.
        Manual(&'static str),
        /// The setting.
        Follow(bool),
    }
    use Step::*;

    /// A model of the engine: the active profile and its cause. It carries
    /// out each decision.
    struct Run {
        machine: FrontSwitch,
        list: ProfileList,
        active: String,
        cause: ActiveCause,
    }

    impl Run {
        fn new(active: &str) -> Self {
            Self {
                machine: FrontSwitch::new(true),
                list: list(),
                active: active.into(),
                cause: ActiveCause::Selected,
            }
        }

        fn notice(&mut self, front: &FrontApplication, locked: bool, sleeping: bool) -> Decision {
            let decision = self.machine.notice(
                front,
                Situation {
                    list: &self.list,
                    active: &self.active,
                    cause: &self.cause,
                    locked,
                    sleeping,
                },
            );
            match &decision {
                Decision::Nothing => {}
                Decision::SetCause(cause) => self.cause = cause.clone(),
                Decision::Select { profile, cause } => {
                    self.active = profile.clone();
                    self.cause = cause.clone();
                }
            }
            decision
        }

        fn manual(&mut self, profile: &str) {
            self.active = profile.into();
            self.cause = ActiveCause::Selected;
            self.machine.manual_selection();
        }
    }

    fn run(start: &str, steps: Vec<(&str, Step)>) {
        let mut run = Run::new(start);
        for (what, step) in steps {
            match step {
                Front(front, expected) => {
                    assert_eq!(run.notice(&front, false, false), expected, "{what}");
                }
                FrontLocked(front, expected) => {
                    assert_eq!(run.notice(&front, true, false), expected, "{what}");
                }
                FrontAsleep(front, expected) => {
                    assert_eq!(run.notice(&front, false, true), expected, "{what}");
                }
                Manual(profile) => run.manual(profile),
                Follow(follow) => {
                    run.machine.set_follow(follow);
                }
            }
        }
    }

    #[test]
    fn section_8_rule_3_the_target_is_the_link_owner_or_the_default_profile() {
        run(
            "music",
            vec![
                (
                    "a linked application selects its profile",
                    Front(bundle(ZOOM), select("calls", linked(ZOOM))),
                ),
                (
                    "a small and a capital letter make no difference",
                    Front(bundle("US.ZOOM.XOS"), Decision::Nothing),
                ),
                (
                    "an application with no link selects the default profile",
                    Front(bundle(SAFARI), select("home", ActiveCause::Default)),
                ),
                (
                    "an application with no bundle has no link",
                    Front(FrontApplication::NoBundle, Decision::Nothing),
                ),
                (
                    "zoom again",
                    Front(bundle(ZOOM), select("calls", linked(ZOOM))),
                ),
                (
                    "no bundle selects the default profile",
                    Front(
                        FrontApplication::NoBundle,
                        select("home", ActiveCause::Default),
                    ),
                ),
            ],
        );
    }

    #[test]
    fn section_8_rule_4_a_new_target_only_changes_the_profile() {
        run(
            "calls",
            vec![
                (
                    "the first notice: the target is the active profile, only the cause changes",
                    Front(bundle(ZOOM), Decision::SetCause(linked(ZOOM))),
                ),
                (
                    "the same target: nothing",
                    Front(bundle(ZOOM), Decision::Nothing),
                ),
                (
                    "safari",
                    Front(bundle(SAFARI), select("home", ActiveCause::Default)),
                ),
                (
                    "terminal has the same target as safari",
                    Front(bundle(TERMINAL), Decision::Nothing),
                ),
            ],
        );
    }

    #[test]
    fn section_8_a_manual_selection_while_another_application_is_in_front_stays() {
        run(
            "home",
            vec![
                (
                    "safari: the default profile is active already",
                    Front(bundle(SAFARI), Decision::SetCause(ActiveCause::Default)),
                ),
                ("a key selects music", Manual("music")),
                (
                    "terminal has the same target as safari: music stays",
                    Front(bundle(TERMINAL), Decision::Nothing),
                ),
                (
                    "zoom has a different target",
                    Front(bundle(ZOOM), select("calls", linked(ZOOM))),
                ),
            ],
        );
    }

    #[test]
    fn section_8_rule_1_and_4_a_selection_in_the_window_does_not_stay() {
        run(
            "calls",
            vec![
                (
                    "zoom",
                    Front(bundle(ZOOM), Decision::SetCause(linked(ZOOM))),
                ),
                (
                    "opencraft: rule 1, nothing",
                    Front(FrontApplication::Shell, Decision::Nothing),
                ),
                ("the user selects work in the window", Manual("music")),
                (
                    "back to zoom: the selection was in the window",
                    Front(bundle(ZOOM), select("calls", linked(ZOOM))),
                ),
                (
                    "another trip to opencraft and back, with no change",
                    Front(FrontApplication::Shell, Decision::Nothing),
                ),
                (
                    "the active profile is the target: no change",
                    Front(bundle(ZOOM), Decision::Nothing),
                ),
            ],
        );
    }

    #[test]
    fn section_8_the_edit_of_the_active_profile_keeps_it() {
        // The user edits "calls" in the window, and selects it again.
        run(
            "calls",
            vec![
                (
                    "zoom",
                    Front(bundle(ZOOM), Decision::SetCause(linked(ZOOM))),
                ),
                (
                    "opencraft",
                    Front(FrontApplication::Shell, Decision::Nothing),
                ),
                ("the user selects the active profile", Manual("calls")),
                (
                    "back to zoom: no load, the cause comes back",
                    Front(bundle(ZOOM), Decision::SetCause(linked(ZOOM))),
                ),
            ],
        );
    }

    #[test]
    fn section_8_a_manual_selection_with_opencraft_not_in_front_is_not_a_choice_in_the_window() {
        run(
            "calls",
            vec![
                (
                    "safari",
                    Front(bundle(SAFARI), select("home", ActiveCause::Default)),
                ),
                ("a key", Manual("music")),
                (
                    "the same target: no change",
                    Front(bundle(TERMINAL), Decision::Nothing),
                ),
                (
                    "opencraft",
                    Front(FrontApplication::Shell, Decision::Nothing),
                ),
                ("a click in the window", Manual("calls")),
                (
                    "safari: the choice in the window ends",
                    Front(bundle(SAFARI), select("home", ActiveCause::Default)),
                ),
                (
                    "terminal: the flag is clear, the target is the same",
                    Front(bundle(TERMINAL), Decision::Nothing),
                ),
            ],
        );
    }

    #[test]
    fn section_8_rule_2_the_setting_off_changes_nothing() {
        run(
            "music",
            vec![
                ("off", Follow(false)),
                ("zoom", Front(bundle(ZOOM), Decision::Nothing)),
                ("safari", Front(bundle(SAFARI), Decision::Nothing)),
                ("on", Follow(true)),
                (
                    "the next notice applies the rule: there is no last target",
                    Front(bundle(ZOOM), select("calls", linked(ZOOM))),
                ),
            ],
        );
    }

    #[test]
    fn section_7_the_setting_on_again_forgets_the_last_target() {
        run(
            "calls",
            vec![
                (
                    "zoom",
                    Front(bundle(ZOOM), Decision::SetCause(linked(ZOOM))),
                ),
                ("a key", Manual("music")),
                ("off", Follow(false)),
                ("on", Follow(true)),
                (
                    "zoom: the last target is empty, so the rule selects",
                    Front(bundle(ZOOM), select("calls", linked(ZOOM))),
                ),
            ],
        );
    }

    #[test]
    fn section_8_rule_2_a_lock_and_a_sleep_change_nothing() {
        run(
            "calls",
            vec![
                (
                    "zoom",
                    Front(bundle(ZOOM), Decision::SetCause(linked(ZOOM))),
                ),
                (
                    "loginwindow comes to the front at the lock",
                    FrontLocked(bundle("com.apple.loginwindow"), Decision::Nothing),
                ),
                (
                    "a notice while the computer sleeps",
                    FrontAsleep(bundle(SAFARI), Decision::Nothing),
                ),
                (
                    "after the unlock, the first notice applies the rule",
                    Front(bundle(SAFARI), select("home", ActiveCause::Default)),
                ),
            ],
        );
    }

    #[test]
    fn section_8_rule_2_a_lock_does_not_keep_the_window_choice() {
        // The notice of rule 2 sets the shell flag to false, and changes no
        // other value.
        run(
            "calls",
            vec![
                (
                    "zoom",
                    Front(bundle(ZOOM), Decision::SetCause(linked(ZOOM))),
                ),
                (
                    "opencraft",
                    Front(FrontApplication::Shell, Decision::Nothing),
                ),
                ("a click in the window", Manual("music")),
                (
                    "a locked notice",
                    FrontLocked(bundle("com.apple.loginwindow"), Decision::Nothing),
                ),
                // The shell flag is false now, so this selection is a key
                // selection. The choice in the window stays true.
                (
                    "zoom: the choice in the window is still true",
                    Front(bundle(ZOOM), select("calls", linked(ZOOM))),
                ),
            ],
        );
    }

    #[test]
    fn section_8_no_default_mark_gives_the_first_profile_with_no_link() {
        let mut run = Run::new("calls");
        run.list = ProfileList::new(
            vec![entry("a", &[ZOOM]), entry("b", &[]), entry("c", &[])],
            None,
        );
        assert_eq!(
            run.notice(&bundle(SAFARI), false, false),
            select("b", ActiveCause::Default)
        );
    }

    #[test]
    fn section_5_a_clashing_link_goes_to_the_first_profile() {
        let mut run = Run::new("music");
        run.list = ProfileList::new(vec![entry("music", &[ZOOM]), entry("calls", &[ZOOM])], None);
        assert_eq!(
            run.notice(&bundle(ZOOM), false, false),
            select("calls", linked(ZOOM))
        );
    }

    #[test]
    fn section_8_an_empty_list_changes_only_the_cause() {
        let mut run = Run::new("only");
        run.list = ProfileList::default();
        assert_eq!(
            run.notice(&bundle(SAFARI), false, false),
            Decision::SetCause(ActiveCause::Default)
        );
    }

    #[test]
    fn a_failed_selection_lets_the_next_notice_try_again() {
        let mut run = Run::new("calls");
        let situation = |run: &Run| {
            let list = run.list.clone();
            (list, run.active.clone(), run.cause.clone())
        };
        let (list, active, cause) = situation(&run);
        let decision = run.machine.notice(
            &bundle(SAFARI),
            Situation {
                list: &list,
                active: &active,
                cause: &cause,
                locked: false,
                sleeping: false,
            },
        );
        assert_eq!(decision, select("home", ActiveCause::Default));
        run.machine.select_failed();
        let decision = run.machine.notice(
            &bundle(SAFARI),
            Situation {
                list: &list,
                active: &active,
                cause: &cause,
                locked: false,
                sleeping: false,
            },
        );
        assert_eq!(decision, select("home", ActiveCause::Default));
    }

    #[test]
    fn a_restart_clears_the_state_and_takes_the_setting() {
        let mut machine = FrontSwitch::new(true);
        machine.set_follow(false);
        machine.restart(true);
        assert!(machine.follow());
        assert_eq!(machine, FrontSwitch::new(true));
    }
}

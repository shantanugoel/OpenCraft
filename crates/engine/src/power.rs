//! Platform-neutral device power policy (decision 0034).

use std::time::Instant;

/// A system notice, translated by the shell. No platform objects cross the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerNotice {
    WillSleep,
    DidWake,
    DisplaysSleep,
    DisplaysWake,
    Locked,
    Unlocked,
}

/// The requested light state, independent of the stored device settings.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DisplayState {
    #[default]
    Awake,
    ScreenOff,
    DeviceSleep,
}

#[derive(Default)]
pub(crate) struct Power {
    pub state: DisplayState,
    pub locked: bool,
    pub sleeping: bool,
    pub displays_asleep: bool,
    pub ready_after: Option<Instant>,
    pub consumed_key: Option<crate::ControlId>,
    pub down_key: Option<crate::ControlId>,
    pub consumed_touch: bool,
    pub idle_deadline: Option<Instant>,
}

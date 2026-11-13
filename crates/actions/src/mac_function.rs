//! Mac system functions. The platform call is behind the System interface.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacFunction {
    MissionControl,
    Launchpad,
    Screenshot,
    ScreenSaver,
    Sleep,
    Lock,
    ShowDesktop,
}

impl MacFunction {
    pub const ALL: [Self; 7] = [
        Self::MissionControl,
        Self::Launchpad,
        Self::Screenshot,
        Self::ScreenSaver,
        Self::Sleep,
        Self::Lock,
        Self::ShowDesktop,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::MissionControl => "mission-control",
            Self::Launchpad => "launchpad",
            Self::Screenshot => "screenshot",
            Self::ScreenSaver => "screen-saver",
            Self::Sleep => "sleep",
            Self::Lock => "lock",
            Self::ShowDesktop => "show-desktop",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|function| function.name() == name)
    }
}

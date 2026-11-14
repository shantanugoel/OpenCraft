//! The action "Open a URL".
//!
//! No browser, or an empty browser, is the default application for the URL.

use crate::descriptions::{field, GROUP_SYSTEM};
use crate::kinds::{nonblank, optional_text, Kind, Work};
use opencraft_engine::{InputType, KindDescription, PathKind, Settings};

/// The kind name.
pub(crate) const ID: &str = "opencraft.open-url";

const NAME: &str = "Open a URL";

/// The registry value for this kind.
pub(crate) static KIND: OpenUrl = OpenUrl;

pub(crate) struct OpenUrl;

impl Kind for OpenUrl {
    fn id(&self) -> &'static str {
        ID
    }

    fn description(&self) -> KindDescription {
        description()
    }

    fn invoke(&self, settings: &Settings, work: &Work<'_>) -> Result<(), String> {
        perform(&read(settings)?, work)
    }
}

/// The description of this kind (decision 0014).
pub(crate) fn description() -> KindDescription {
    KindDescription {
        kind: ID.into(),
        name: NAME.into(),
        group: GROUP_SYSTEM.into(),
        icon: Some("globe".into()),
        fields: vec![
            field("url", "URL", InputType::Text),
            // Optional. No browser is the default browser.
            field(
                "application",
                "Browser",
                InputType::Path {
                    kind: PathKind::File,
                },
            ),
        ],
        ..KindDescription::default()
    }
}

/// The URL to open, after a successful read of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Url {
    pub(crate) url: String,
    /// A name or a path. `None` is the default browser.
    pub(crate) browser: Option<String>,
}

/// Read the settings. The URL keeps its spaces. The browser is trimmed.
/// The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<Url, String> {
    Ok(Url {
        url: nonblank(ID, settings, "url")?.to_string(),
        browser: optional_text(ID, settings, "application")?,
    })
}

/// Open the URL in `browser`, or in the default browser.
pub(crate) fn perform(url: &Url, work: &Work<'_>) -> Result<(), String> {
    work.system
        .open_url(&url.url, url.browser.as_deref())
        .map_err(|e| format!("Cannot open the URL: {e}."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptions::setting_problem;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::{assert_system_kind, values, Work};
    use opencraft_engine::{Settings, Value};

    fn run(url: &Url, system: &System) -> Result<(), String> {
        let sender = None;
        perform(url, &Work::new(system, &sender))
    }

    #[test]
    fn the_description_has_a_url_and_an_optional_browser() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_system_kind(&description, NAME, "globe");
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(description.fields.len(), 2);
        assert_eq!(description.fields[0].key, "url");
        assert_eq!(description.fields[0].label, "URL");
        assert_eq!(description.fields[0].input, InputType::Text);
        assert_eq!(description.fields[0].default, None);
        assert_eq!(description.fields[1].key, "application");
        assert_eq!(description.fields[1].label, "Browser");
        assert!(matches!(
            description.fields[1].input,
            InputType::Path {
                kind: PathKind::File,
            }
        ));
        assert_eq!(description.fields[1].default, None);
    }

    #[test]
    fn read_uses_the_default_browser_when_the_browser_is_missing_or_empty() {
        let url = || values(&[("url", Value::Text("https://example.test".into()))]);
        assert_eq!(
            read(&url()).unwrap(),
            Url {
                url: "https://example.test".into(),
                browser: None,
            }
        );
        let mut empty = url();
        empty.insert("application".into(), Value::Text(String::new()));
        assert_eq!(read(&empty).unwrap().browser, None);
        let mut blank = url();
        blank.insert("application".into(), Value::Text("  ".into()));
        assert_eq!(read(&blank).unwrap().browser, None);
        assert_eq!(
            read(&values(&[
                ("url", Value::Text("https://example.test/a b".into())),
                (
                    "application",
                    Value::Text(" /Applications/Test.app ".into())
                ),
            ]))
            .unwrap(),
            Url {
                url: "https://example.test/a b".into(),
                browser: Some("/Applications/Test.app".into()),
            }
        );
    }

    #[test]
    fn read_rejects_a_blank_url_and_a_browser_that_is_not_text() {
        assert_eq!(
            read(&Settings::new()).unwrap_err(),
            setting_problem(ID, "url")
        );
        assert_eq!(
            read(&values(&[("url", Value::Text("\n".into()))])).unwrap_err(),
            setting_problem(ID, "url")
        );
        assert_eq!(
            read(&values(&[
                ("url", Value::Text("https://example.test".into())),
                ("application", Value::Integer(1)),
            ]))
            .unwrap_err(),
            setting_problem(ID, "application")
        );
    }

    #[test]
    fn a_bad_read_does_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(&Settings::new(), &Work::new(&system, &sender))
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "url"));
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_opens_the_url_and_reports_a_system_error() {
        let system = System::default();
        run(
            &Url {
                url: "https://example.test".into(),
                browser: None,
            },
            &system,
        )
        .unwrap();
        run(
            &Url {
                url: "https://example.test".into(),
                browser: Some("Safari".into()),
            },
            &system,
        )
        .unwrap();
        assert_eq!(
            system.calls(),
            [
                Call::Url("https://example.test".into(), None),
                Call::Url("https://example.test".into(), Some("Safari".into())),
            ]
        );
        system.fail(Fault::Code("url"));
        assert_eq!(
            run(
                &Url {
                    url: "https://example.test".into(),
                    browser: Some("Safari".into()),
                },
                &system,
            )
            .unwrap_err(),
            "Cannot open the URL: the system refused the operation (code 42)."
        );
    }
}

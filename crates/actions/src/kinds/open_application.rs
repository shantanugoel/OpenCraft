//! The action "Open an application".
//!
//! The target is an application, a file, or a folder. Arguments apply only
//! to an application. The functions that classify a target and split
//! arguments are pure, so a test of them needs no system.

use crate::descriptions::{field, setting_problem, GROUP_SYSTEM};
use crate::kinds::{nonblank, Kind, Work};
use opencraft_engine::{InputType, KindDescription, PathKind, Settings, Value};

/// The kind name.
pub(crate) const ID: &str = "opencraft.open-application";

const NAME: &str = "Open an application";

/// The registry value for this kind.
pub(crate) static KIND: OpenApplication = OpenApplication;

pub(crate) struct OpenApplication;

impl Kind for OpenApplication {
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
        icon: Some("app-window".into()),
        fields: vec![
            // The key keeps its name, so that an old profile loads.
            // Kind Application asks the shell for the list of installed
            // applications. The stored text is still a name or a path.
            field(
                "application",
                "Application, file or folder",
                InputType::Path {
                    kind: PathKind::Application,
                },
            ),
            field("arguments", "Arguments", InputType::Text),
        ],
        ..KindDescription::default()
    }
}

/// True if `target` names an application: a name with no `/` (for example
/// `Safari`), or a path of a bundle that ends with `.app`. Each other target
/// is a file or a folder.
fn is_application(target: &str) -> bool {
    let target = target.trim().trim_end_matches('/');
    !target.contains('/') || target.ends_with(".app")
}

/// Split `text` into arguments. A space, a tab, or a line end separates two
/// arguments. A pair of `"` or of `'` joins the characters in it into one
/// argument, and a `\` before a `"`, a `'`, a `\`, or a space gives that
/// character. `None` means that a quote has no end.
fn split_arguments(text: &str) -> Option<Vec<String>> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    // An empty pair of quotes is an argument too.
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut characters = text.chars();
    while let Some(c) = characters.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => {
                quote = Some(c);
                started = true;
            }
            (_, '\\') => match characters.clone().next() {
                Some(next @ ('"' | '\'' | '\\' | ' ')) => {
                    current.push(next);
                    started = true;
                    characters.next();
                }
                _ => current.push('\\'),
            },
            (None, c) if c.is_whitespace() => {
                if started || !current.is_empty() {
                    arguments.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            (_, c) => current.push(c),
        }
    }
    if quote.is_some() {
        return None;
    }
    if started || !current.is_empty() {
        arguments.push(current);
    }
    Some(arguments)
}

/// The target of the action, after a successful read of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Target {
    Application {
        path: String,
        arguments: Vec<String>,
    },
    File {
        path: String,
    },
}

/// Read the settings. The error is the text for the user.
pub(crate) fn read(settings: &Settings) -> Result<Target, String> {
    let raw = nonblank(ID, settings, "application")?;
    let arguments = match settings.get("arguments") {
        None => Vec::new(),
        Some(Value::Text(text)) => split_arguments(text)
            .ok_or_else(|| format!("{NAME}: close each quote in the field \"Arguments\"."))?,
        Some(_) => return Err(setting_problem(ID, "arguments")),
    };
    // `trim` removes spaces. A trailing slash on a bundle path stays.
    let path = raw.trim().to_string();
    if is_application(raw) {
        Ok(Target::Application { path, arguments })
    } else if arguments.is_empty() {
        Ok(Target::File { path })
    } else {
        Err(format!(
            "{NAME}: arguments work only with an application. \
             Clear the field \"Arguments\", or select an application."
        ))
    }
}

/// Open the target. The system brings a running application to the front.
pub(crate) fn perform(target: &Target, work: &Work<'_>) -> Result<(), String> {
    match target {
        Target::Application { path, arguments } => work
            .system
            .open_application(path, arguments)
            .map_err(|e| format!("Cannot open the application: {e}.")),
        Target::File { path } => work
            .system
            .open_file(path)
            .map_err(|e| format!("Cannot open the file or folder: {e}.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::recording::{Call, Fault, System};
    use crate::kinds::{assert_system_kind, values, Work};
    use opencraft_engine::Settings;

    fn words(text: &str) -> Vec<String> {
        split_arguments(text).expect("closed quotes")
    }

    fn run(target: &Target, system: &System) -> Result<(), String> {
        let sender = None;
        perform(target, &Work::new(system, &sender))
    }

    #[test]
    fn the_description_is_the_system_form_with_an_optional_arguments_field() {
        let description = description();
        assert_eq!(description.kind, ID);
        assert_system_kind(&description, NAME, "app-window");
        assert_eq!(crate::descriptions::description(ID), description);
        assert_eq!(description.fields.len(), 2);
        assert_eq!(description.fields[0].key, "application");
        assert_eq!(description.fields[0].label, "Application, file or folder");
        assert_eq!(
            description.fields[0].input,
            InputType::Path {
                kind: PathKind::Application,
            }
        );
        assert_eq!(description.fields[0].default, None);
        assert_eq!(description.fields[1].key, "arguments");
        assert_eq!(description.fields[1].label, "Arguments");
        assert_eq!(description.fields[1].input, InputType::Text);
        assert_eq!(description.fields[1].default, None);
    }

    #[test]
    fn a_name_or_a_bundle_path_is_an_application_and_other_paths_are_not() {
        for target in [
            "Safari",
            "Visual Studio Code",
            "/Applications/Test.app",
            "/Applications/Test.app/",
            "~/Applications/Test.app",
            " Test ",
        ] {
            assert!(is_application(target), "{target}");
        }
        for target in [
            "/Users/Test/notes.txt",
            "~/Documents",
            "/Applications",
            "/tmp/a.app/data",
            "relative/folder",
        ] {
            assert!(!is_application(target), "{target}");
        }
    }

    #[test]
    fn arguments_split_at_spaces_and_quotes_group_words() {
        assert_eq!(words(""), Vec::<String>::new());
        assert_eq!(words("  \t\n"), Vec::<String>::new());
        assert_eq!(words("-a  --b=c\td"), ["-a", "--b=c", "d"]);
        assert_eq!(words("\"a b\" 'c d' e"), ["a b", "c d", "e"]);
        assert_eq!(words("x\"a b\"y"), ["xa by"]);
        assert_eq!(words("\"\" ''"), ["", ""]);
        assert_eq!(words("\"it's\" 'say \"hi\"'"), ["it's", "say \"hi\""]);
        assert_eq!(words("a\\ b \\\"c\\\" \\\\"), ["a b", "\"c\"", "\\"]);
        assert_eq!(words("C:\\path"), ["C:\\path"]);
        assert_eq!(split_arguments("\"open"), None);
        assert_eq!(split_arguments("a 'b"), None);
    }

    #[test]
    fn read_accepts_an_old_profile_and_splits_arguments() {
        let application = |target: &str, arguments: Option<&str>| {
            let mut pairs = vec![("application", Value::Text(target.into()))];
            if let Some(arguments) = arguments {
                pairs.push(("arguments", Value::Text(arguments.into())));
            }
            read(&values(&pairs)).unwrap()
        };
        assert_eq!(
            application("Safari", None),
            Target::Application {
                path: "Safari".into(),
                arguments: vec![],
            }
        );
        assert_eq!(
            application(" Test ", Some("")),
            Target::Application {
                path: "Test".into(),
                arguments: vec![],
            }
        );
        assert_eq!(
            application("Test", Some("  ")),
            Target::Application {
                path: "Test".into(),
                arguments: vec![],
            }
        );
        assert_eq!(
            application(
                "/Applications/Test.app/",
                Some("--new-window \"two words\"")
            ),
            Target::Application {
                path: "/Applications/Test.app/".into(),
                arguments: vec!["--new-window".into(), "two words".into()],
            }
        );
        assert_eq!(
            application("  /Applications  ", None),
            Target::File {
                path: "/Applications".into(),
            }
        );
        // A key that this kind does not use does not change the read.
        let mut settings = values(&[("application", Value::Text("Safari".into()))]);
        settings.insert("extra".into(), Value::Boolean(true));
        assert!(matches!(
            read(&settings).unwrap(),
            Target::Application { .. }
        ));
    }

    #[test]
    fn read_rejects_a_bad_target_a_bad_argument_list_and_arguments_of_a_file() {
        assert_eq!(
            read(&Settings::new()).unwrap_err(),
            setting_problem(ID, "application")
        );
        assert_eq!(
            read(&values(&[("application", Value::Text("  ".into()))])).unwrap_err(),
            setting_problem(ID, "application")
        );
        assert_eq!(
            read(&values(&[
                ("application", Value::Text("Test".into())),
                ("arguments", Value::Integer(5)),
            ]))
            .unwrap_err(),
            setting_problem(ID, "arguments")
        );
        assert_eq!(
            read(&values(&[
                ("application", Value::Text("Test".into())),
                ("arguments", Value::Text("\"open".into())),
            ]))
            .unwrap_err(),
            "Open an application: close each quote in the field \"Arguments\"."
        );
        assert_eq!(
            read(&values(&[
                ("application", Value::Text("/tmp/notes.txt".into())),
                ("arguments", Value::Text("-x".into())),
            ]))
            .unwrap_err(),
            "Open an application: arguments work only with an application. \
             Clear the field \"Arguments\", or select an application."
        );
    }

    #[test]
    fn a_bad_read_does_no_work() {
        let system = System::default();
        let sender = None;
        let error = KIND
            .invoke(&Settings::new(), &Work::new(&system, &sender))
            .unwrap_err();
        assert_eq!(error, setting_problem(ID, "application"));
        assert!(system.calls().is_empty());
    }

    #[test]
    fn perform_opens_an_application_or_a_file_and_reports_a_system_error() {
        let system = System::default();
        run(
            &Target::Application {
                path: "Test".into(),
                arguments: vec!["-x".into()],
            },
            &system,
        )
        .unwrap();
        run(
            &Target::File {
                path: "/tmp/notes.txt".into(),
            },
            &system,
        )
        .unwrap();
        assert_eq!(
            system.calls(),
            [
                Call::Application("Test".into(), vec!["-x".into()]),
                Call::File("/tmp/notes.txt".into()),
            ]
        );
        system.fail(Fault::Code("application"));
        assert_eq!(
            run(
                &Target::Application {
                    path: "Test".into(),
                    arguments: vec![],
                },
                &system,
            )
            .unwrap_err(),
            "Cannot open the application: the system refused the operation (code 42)."
        );
        system.fail(Fault::Code("file"));
        assert_eq!(
            run(
                &Target::File {
                    path: "/tmp/notes.txt".into(),
                },
                &system,
            )
            .unwrap_err(),
            "Cannot open the file or folder: the system refused the operation (code 42)."
        );
    }
}

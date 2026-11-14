//! The action "Open an application".
//!
//! The target is an application, a file, or a folder. Arguments apply only
//! to an application. The action stores the bundle identifier of an
//! application, and the path as a fallback. One registered copy opens at
//! its current path. Two copies of one bundle identifier are told apart by
//! the stored path (decision 0037). The functions that classify a target,
//! choose a copy, and split arguments are pure, so a test of them needs no
//! system.

use crate::descriptions::{field, setting_problem, GROUP_SYSTEM};
use crate::kinds::{nonblank, Kind, Work};
use opencraft_engine::{InputType, KindDescription, PathKind, Settings, Value};

/// The kind name.
pub(crate) const ID: &str = "opencraft.open-application";

const NAME: &str = "Open an application";

/// The setting that stores the bundle identifier.
const BUNDLE_ID: &str = "bundle_id";

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
            // The path stays in this field. The shell stores the bundle
            // identifier in `bundle_id` when the application has one.
            field(
                "application",
                "Application, file or folder",
                InputType::Path {
                    kind: PathKind::Application {
                        bundle_id: BUNDLE_ID.into(),
                    },
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
        /// A bundle identifier, when the setting has one.
        bundle_id: Option<String>,
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
        Ok(Target::Application {
            bundle_id: bundle_id(settings),
            path,
            arguments,
        })
    } else if arguments.is_empty() {
        Ok(Target::File { path })
    } else {
        Err(format!(
            "{NAME}: arguments work only with an application. \
             Clear the field \"Arguments\", or select an application."
        ))
    }
}

/// The bundle identifier in the settings, when the text is one. Any other
/// value is absent: the path is then the only target.
fn bundle_id(settings: &Settings) -> Option<String> {
    match settings.get(BUNDLE_ID) {
        Some(Value::Text(text)) => {
            let text = text.trim();
            opencraft_engine::front::valid_bundle_id(text).then(|| text.to_string())
        }
        _ => None,
    }
}

/// The path to open. One registered copy uses that copy, so a move of the
/// application still opens it. Several copies use the stored path to tell
/// them apart. No copy uses the stored path or name.
pub(crate) fn choose_copy(stored: &str, copies: &[String]) -> String {
    choose_with(stored, copies, std::env::var("HOME").ok().as_deref())
}

fn choose_with(stored: &str, copies: &[String], home: Option<&str>) -> String {
    let copies: Vec<&str> = copies
        .iter()
        .map(String::as_str)
        .filter(|path| !path.trim().is_empty())
        .collect();
    match copies.as_slice() {
        [] => stored.to_string(),
        [only] => (*only).to_string(),
        many => many
            .iter()
            .copied()
            .find(|copy| same_path(stored, copy, home))
            .unwrap_or(stored)
            .to_string(),
    }
}

/// True when `stored` and `copy` are the same path. A leading `~/` is the
/// home folder. A trailing slash does not count, and the case of ASCII
/// letters does not count.
fn same_path(stored: &str, copy: &str, home: Option<&str>) -> bool {
    let norm = |path: &str| {
        expand_prefix(path.trim(), home)
            .trim_end_matches('/')
            .to_ascii_lowercase()
    };
    let stored = norm(stored);
    !stored.is_empty() && stored == norm(copy)
}

/// `path` with a leading `~/` replaced by `home`. A name, and a path that
/// starts with `/`, do not change. This matches `expand_home` in the macOS
/// system code.
fn expand_prefix(path: &str, home: Option<&str>) -> String {
    let Some(rest) = path.strip_prefix("~/") else {
        return path.to_string();
    };
    match home.map(|home| home.trim_end_matches('/')) {
        Some(home) if !home.is_empty() => format!("{home}/{rest}"),
        _ => path.to_string(),
    }
}

/// Open the target. The system brings a running application to the front.
pub(crate) fn perform(target: &Target, work: &Work<'_>) -> Result<(), String> {
    match target {
        Target::Application {
            bundle_id,
            path,
            arguments,
        } => {
            let copies = match bundle_id {
                Some(id) => work.system.application_copies(id),
                None => Vec::new(),
            };
            let path = choose_copy(path, &copies);
            work.system
                .open_application(&path, arguments)
                .map_err(|e| format!("Cannot open the application: {e}."))
        }
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
                kind: PathKind::Application {
                    bundle_id: BUNDLE_ID.into(),
                },
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
    fn read_accepts_a_name_or_a_path_and_splits_arguments() {
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
                bundle_id: None,
                path: "Safari".into(),
                arguments: vec![],
            }
        );
        assert_eq!(
            application(" Test ", Some("")),
            Target::Application {
                bundle_id: None,
                path: "Test".into(),
                arguments: vec![],
            }
        );
        assert_eq!(
            application("Test", Some("  ")),
            Target::Application {
                bundle_id: None,
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
                bundle_id: None,
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
                bundle_id: None,
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
                    bundle_id: None,
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

    #[test]
    fn read_keeps_a_bundle_identifier_and_ignores_a_value_that_is_not_one() {
        let with_id = values(&[
            (
                "application",
                Value::Text(" /Applications/Demo.app ".into()),
            ),
            ("bundle_id", Value::Text(" com.example.demo ".into())),
        ]);
        assert_eq!(
            read(&with_id).unwrap(),
            Target::Application {
                bundle_id: Some("com.example.demo".into()),
                path: "/Applications/Demo.app".into(),
                arguments: vec![],
            }
        );
        // A file has no bundle identifier. The path is the target.
        let file = values(&[
            ("application", Value::Text("/tmp/notes.txt".into())),
            ("bundle_id", Value::Text("com.example.demo".into())),
        ]);
        assert_eq!(
            read(&file).unwrap(),
            Target::File {
                path: "/tmp/notes.txt".into(),
            }
        );
        for ignored in [Value::Text("not an id".into()), Value::Boolean(true)] {
            let settings = values(&[
                ("application", Value::Text("Demo".into())),
                ("bundle_id", ignored),
            ]);
            assert_eq!(
                read(&settings).unwrap(),
                Target::Application {
                    bundle_id: None,
                    path: "Demo".into(),
                    arguments: vec![],
                }
            );
        }
    }

    #[test]
    fn choose_copy_uses_one_registration_or_the_stored_path() {
        let one = vec!["/Applications/Moved.app".into()];
        assert_eq!(
            choose_with("/tmp/old/Demo.app", &one, None),
            "/Applications/Moved.app"
        );
        let two = vec!["/Applications/Demo.app".into(), "/tmp/copy/Demo.app".into()];
        assert_eq!(
            choose_with("/tmp/copy/Demo.app/", &two, None),
            "/tmp/copy/Demo.app"
        );
        assert_eq!(
            choose_with("/APPLICATIONS/DEMO.APP", &two, None),
            "/Applications/Demo.app"
        );
        assert_eq!(
            choose_with("/tmp/missing/Demo.app", &two, None),
            "/tmp/missing/Demo.app"
        );
        assert_eq!(choose_with("Safari", &[], None), "Safari");
        assert_eq!(
            choose_with("~/copy/Demo.app", &two, Some("/tmp")),
            "/tmp/copy/Demo.app"
        );
        assert_eq!(
            expand_prefix("~/Applications/Demo.app", Some("/tmp/copy")),
            "/tmp/copy/Applications/Demo.app"
        );
        assert_eq!(expand_prefix("Safari", Some("/tmp/copy")), "Safari");
        assert_eq!(
            expand_prefix("~/Applications/Demo.app", None),
            "~/Applications/Demo.app"
        );
    }

    #[test]
    fn perform_opens_the_one_registered_copy() {
        let system = System::default();
        system.set_copies(vec!["/Applications/Moved.app".into()]);
        run(
            &Target::Application {
                bundle_id: Some("com.example.demo".into()),
                path: "/tmp/old/Demo.app".into(),
                arguments: vec!["-n".into()],
            },
            &system,
        )
        .unwrap();
        assert_eq!(
            system.calls(),
            [Call::Application(
                "/Applications/Moved.app".into(),
                vec!["-n".into()]
            )]
        );
    }
}

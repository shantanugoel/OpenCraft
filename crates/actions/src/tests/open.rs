use super::*;

fn open_action(target: &str, arguments: Option<Value>) -> opencraft_engine::Action {
    let mut fields = vec![("application", Value::Text(target.into()))];
    if let Some(arguments) = arguments {
        fields.push(("arguments", arguments));
    }
    action("open-application", &fields)
}

#[test]
fn an_application_opens_with_no_arguments_when_the_setting_is_old_or_empty() {
    // An old profile has no "arguments" setting.
    let mut rig = Rig::new("open-application", true);
    rig.invoke(open_action("Test", None));
    rig.invoke(open_action("Test", Some(Value::Text(String::new()))));
    rig.invoke(open_action("Test", Some(Value::Text("  ".into()))));
    rig.invoke(open_action("/Applications/Test.app", None));
    assert_eq!(
        rig.system.calls(),
        [
            Call::Application("Test".into(), vec![]),
            Call::Application("Test".into(), vec![]),
            Call::Application("Test".into(), vec![]),
            Call::Application("/Applications/Test.app".into(), vec![]),
        ]
    );
    assert!(rig.messages.try_recv().is_err());
}

#[test]
fn an_application_gets_the_arguments_as_separate_words() {
    let mut rig = Rig::new("open-application", true);
    rig.invoke(open_action(
        "/Applications/Test.app",
        Some(Value::Text("--new-window \"two words\" -x".into())),
    ));
    assert_eq!(
        rig.system.calls(),
        [Call::Application(
            "/Applications/Test.app".into(),
            vec!["--new-window".into(), "two words".into(), "-x".into()]
        )]
    );
}

#[test]
fn a_file_or_a_folder_opens_through_the_file_call() {
    let mut rig = Rig::new("open-application", true);
    rig.invoke(open_action("/Users/Test/notes.txt", None));
    rig.invoke(open_action("~/Documents", Some(Value::Text("  ".into()))));
    rig.invoke(open_action("  /Applications  ", None));
    assert_eq!(
        rig.system.calls(),
        [
            Call::File("/Users/Test/notes.txt".into()),
            Call::File("~/Documents".into()),
            Call::File("/Applications".into()),
        ]
    );
    assert!(rig.messages.try_recv().is_err());
}

#[test]
fn arguments_for_a_file_and_an_open_quote_are_problems_without_work() {
    let mut rig = Rig::new("open-application", true);
    rig.invoke(open_action(
        "/tmp/notes.txt",
        Some(Value::Text("-x".into())),
    ));
    rig.problem("arguments work only with an application");
    rig.invoke(open_action("Test", Some(Value::Text("\"open".into()))));
    rig.problem("close each quote in the field \"Arguments\"");
    rig.invoke(open_action("Test", Some(Value::Integer(5))));
    rig.problem("Open an application: type a value in the field \"Arguments\".");
    assert!(rig.system.calls().is_empty());
}

#[test]
fn a_system_error_of_an_application_or_a_file_becomes_a_problem() {
    for (target, failure, text) in [
        ("Test", "application", "Cannot open the application"),
        ("/tmp/notes.txt", "file", "Cannot open the file or folder"),
    ] {
        let mut rig = Rig::new("open-application", true);
        rig.system.fail(Some(failure));
        rig.invoke(open_action(target, None));
        rig.problem(text);
    }
}

fn url_action(url: &str, browser: Option<Value>) -> opencraft_engine::Action {
    let mut fields = vec![("url", Value::Text(url.into()))];
    if let Some(browser) = browser {
        fields.push(("application", browser));
    }
    action("open-url", &fields)
}

#[test]
fn a_url_opens_in_the_default_browser_when_the_browser_is_missing_or_empty() {
    // An old profile has no "application" setting.
    let mut rig = Rig::new("open-url", true);
    rig.invoke(url_action("https://example.test", None));
    rig.invoke(url_action(
        "https://example.test",
        Some(Value::Text(String::new())),
    ));
    rig.invoke(url_action(
        "https://example.test",
        Some(Value::Text("  ".into())),
    ));
    assert_eq!(
        rig.system.calls(),
        vec![Call::Url("https://example.test".into(), None); 3]
    );
    assert!(rig.messages.try_recv().is_err());
}

#[test]
fn a_url_opens_in_the_browser_that_the_action_names() {
    let mut rig = Rig::new("open-url", true);
    rig.invoke(url_action(
        "https://example.test",
        Some(Value::Text("Safari".into())),
    ));
    rig.invoke(url_action(
        "https://example.test/a b",
        Some(Value::Text(" /Applications/Test.app ".into())),
    ));
    assert_eq!(
        rig.system.calls(),
        [
            Call::Url("https://example.test".into(), Some("Safari".into())),
            Call::Url(
                "https://example.test/a b".into(),
                Some("/Applications/Test.app".into())
            ),
        ]
    );
}

#[test]
fn a_wrong_browser_setting_or_a_system_error_is_a_problem() {
    let mut rig = Rig::new("open-url", true);
    rig.invoke(url_action("https://example.test", Some(Value::Integer(1))));
    rig.problem("Open a URL: type a value in the field \"Browser\".");
    assert!(rig.system.calls().is_empty());
    rig.system.fail(Some("url"));
    rig.invoke(url_action(
        "https://example.test",
        Some(Value::Text("Safari".into())),
    ));
    rig.problem("Cannot open the URL");
}

fn quit_action(application: &str, force: Option<Value>) -> opencraft_engine::Action {
    let mut fields = vec![("application", Value::Text(application.into()))];
    if let Some(force) = force {
        fields.push(("force", force));
    }
    action("quit-application", &fields)
}

#[test]
fn a_quit_action_asks_the_application_to_quit_and_force_stops_it() {
    let mut rig = Rig::new("quit-application", true);
    rig.invoke(quit_action("Calculator", None));
    rig.invoke(quit_action("Calculator", Some(Value::Boolean(false))));
    rig.invoke(quit_action(
        " /System/Applications/Calculator.app ",
        Some(Value::Boolean(true)),
    ));
    assert_eq!(
        rig.system.calls(),
        [
            Call::Quit("Calculator".into(), false),
            Call::Quit("Calculator".into(), false),
            Call::Quit("/System/Applications/Calculator.app".into(), true),
        ]
    );
    assert!(rig.messages.try_recv().is_err());
}

#[test]
fn a_quit_action_with_wrong_settings_does_no_work() {
    let mut rig = Rig::new("quit-application", true);
    rig.invoke(action("quit-application", &[]));
    rig.problem("Quit an application: type a value in the field \"Application\".");
    rig.invoke(quit_action("  ", None));
    rig.problem("the field \"Application\"");
    rig.invoke(quit_action("Calculator", Some(Value::Text("yes".into()))));
    rig.problem("Quit an application: set the field \"Force quit\" to On or Off.");
    assert!(rig.system.calls().is_empty());
}

#[test]
fn a_quit_error_is_a_problem_and_an_application_that_does_not_run_has_its_own_text() {
    let mut rig = Rig::new("quit-application", true);
    rig.system.fail(Some("quit"));
    rig.invoke(quit_action("Calculator", None));
    rig.problem("Cannot quit the application: the system refused the operation (code 42)");
    // The real system gives `NotRunning`. See the native test of `macos_quit`.
    let text = crate::system::Error::NotRunning.to_string();
    assert_eq!(text, "the application does not run");
}

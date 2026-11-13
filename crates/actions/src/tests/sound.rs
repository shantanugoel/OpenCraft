use super::*;

fn play(values: &[(&str, Value)]) -> opencraft_engine::Action {
    action("play-sound", values)
}

#[test]
fn play_sound_description_has_the_form_group_and_defaults() {
    let rig = Rig::new("play-sound", true);
    let kinds = rig.handler.kinds();
    let names: Vec<_> = kinds.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names, ["Play a sound", "Stop all sound"]);
    let fields: Vec<_> = kinds[0]
        .fields
        .iter()
        .map(|f| (f.key.as_str(), f.label.as_str(), f.default.clone()))
        .collect();
    assert_eq!(
        fields,
        [
            ("file", "File", None),
            ("volume", "Volume", Some(Value::Integer(100))),
            ("device", "Output", Some(Value::Text(String::new()))),
        ]
    );
    assert!(kinds[1].fields.is_empty());
    assert!(kinds.iter().all(|k| k.group == "Media" && k.help.is_none()));
}

#[test]
fn play_sound_sends_file_volume_and_output_to_the_system() {
    let mut rig = Rig::new("play-sound", true);
    rig.invoke(play(&[("file", Value::Text("/sounds/a.wav".into()))]));
    rig.invoke(play(&[
        ("file", Value::Text("/sounds/b.aiff".into())),
        ("volume", Value::Integer(0)),
        ("device", Value::Text(String::new())),
    ]));
    rig.invoke(play(&[
        ("file", Value::Text("/sounds/c.mp3".into())),
        ("volume", Value::Integer(25)),
        ("device", Value::Text("b".into())),
    ]));
    rig.invoke(action("stop-sound", &[]));
    assert_eq!(
        rig.system.calls(),
        [
            Call::PlaySound("/sounds/a.wav".into(), 100, None),
            Call::PlaySound("/sounds/b.aiff".into(), 0, None),
            Call::PlaySound("/sounds/c.mp3".into(), 25, Some("b".into())),
            Call::StopSounds,
        ]
    );
    assert!(rig.messages.try_recv().is_err());
}

#[test]
fn play_sound_bad_settings_and_a_missing_output_play_nothing() {
    let file = ("file", Value::Text("/sounds/a.wav".into()));
    for (values, text) in [
        (vec![], "\"File\""),
        (vec![("file", Value::Text("  ".into()))], "\"File\""),
        (vec![("file", Value::Integer(1))], "\"File\""),
        (
            vec![file.clone(), ("volume", Value::Integer(101))],
            "\"Volume\"",
        ),
        (
            vec![file.clone(), ("volume", Value::Integer(-1))],
            "\"Volume\"",
        ),
        (
            vec![file.clone(), ("volume", Value::Text("5".into()))],
            "\"Volume\"",
        ),
        (
            vec![file.clone(), ("device", Value::Integer(1))],
            "\"Output\"",
        ),
        (
            vec![file.clone(), ("device", Value::Text("gone".into()))],
            "not available",
        ),
    ] {
        let mut rig = Rig::new("play-sound", true);
        rig.invoke(play(&values));
        rig.problem(text);
        assert!(rig.system.calls().is_empty(), "{values:?}");
    }
}

#[test]
fn play_and_stop_errors_are_visible() {
    let mut rig = Rig::new("play-sound", true);
    rig.system.fail(Some("play-sound"));
    rig.invoke(play(&[("file", Value::Text("/sounds/a.wav".into()))]));
    rig.problem("Cannot play the sound file");
    rig.system.fail(Some("stop-sounds"));
    rig.invoke(action("stop-sound", &[]));
    rig.problem("Cannot stop the sound");
    rig.system.fail(Some("outputs"));
    rig.invoke(play(&[
        ("file", Value::Text("/sounds/a.wav".into())),
        ("device", Value::Text("b".into())),
    ]));
    rig.problem("list the sound outputs");
    assert_eq!(
        rig.system.calls(),
        [
            Call::PlaySound("/sounds/a.wav".into(), 100, None),
            Call::StopSounds
        ]
    );
}

#[test]
fn play_sound_output_choices_start_with_the_default_output() {
    let mut rig = Rig::new("play-sound", true);
    rig.handler.handle(Request::Choices {
        id: 7,
        action: play(&[]),
        field: "device".into(),
    });
    let Message::Choices { id, options } = rig.recv() else {
        panic!("choices")
    };
    assert_eq!(id, 7);
    let options: Vec<_> = options
        .iter()
        .map(|o| (o.value.as_str(), o.label.as_str()))
        .collect();
    assert_eq!(
        options,
        [
            ("", "Default output"),
            ("a", "Speakers"),
            ("b", "Headphones")
        ]
    );
    rig.system.fail(Some("outputs"));
    rig.handler.handle(Request::Choices {
        id: 8,
        action: play(&[]),
        field: "device".into(),
    });
    rig.problem("list the sound outputs");
    let Message::Choices { options, .. } = rig.recv() else {
        panic!("choices")
    };
    assert_eq!(options.len(), 1);
    assert!(rig.system.calls().is_empty());
}

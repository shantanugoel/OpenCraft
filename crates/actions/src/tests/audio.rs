use super::*;
use opencraft_engine::{BindingId, FeedbackState};

#[test]
fn output_choices_come_from_the_system_without_changing_it() {
    let mut rig = Rig::new("sound-output", true);
    for field in ["device", "devices"] {
        rig.handler.handle(Request::Choices {
            id: 19,
            action: action("sound-output", &[]),
            field: field.into(),
        });
        let Message::Choices { id, options } = rig.recv() else {
            panic!("choices")
        };
        assert_eq!(id, 19);
        assert_eq!(
            options.iter().map(|o| o.value.as_str()).collect::<Vec<_>>(),
            ["a", "b"]
        );
        assert_eq!(options[1].label, "Headphones");
    }
    assert!(rig.system.calls().is_empty());
    rig.system.fail(Some("outputs"));
    rig.handler.handle(Request::Choices {
        id: 20,
        action: action("sound-output", &[]),
        field: "device".into(),
    });
    rig.problem("list the sound");
    assert_eq!(
        rig.recv(),
        Message::Choices {
            id: 20,
            options: vec![]
        }
    );
}

#[test]
fn output_sets_cycles_in_order_skips_disconnected_and_wraps() {
    let mut rig = Rig::new("sound-output", true);
    rig.invoke(action(
        "sound-output",
        &[("device", Value::Text("b".into()))],
    ));
    feedback(rig.recv(), binding(1), "Headphones", None);
    assert_eq!(rig.system.calls(), [Call::Output("b".into())]);
    let next = action(
        "sound-output",
        &[
            ("operation", Value::Text("next".into())),
            (
                "devices",
                Value::List(["b", "missing", "a"].map(|v| Value::Text(v.into())).into()),
            ),
        ],
    );
    rig.invoke(next.clone());
    feedback(rig.recv(), binding(1), "Speakers", None);
    rig.invoke(next);
    feedback(rig.recv(), binding(1), "Headphones", None);
    let all = action("sound-output", &[("operation", Value::Text("next".into()))]);
    rig.invoke(all);
    feedback(rig.recv(), binding(1), "Speakers", None);
}

#[test]
fn output_bad_settings_and_system_failures_do_not_select_a_device() {
    for values in [
        vec![],
        vec![("device", Value::Text("missing".into()))],
        vec![("device", Value::Integer(1))],
        vec![("operation", Value::Text("bad".into()))],
        vec![("operation", Value::Integer(1))],
        vec![
            ("operation", Value::Text("next".into())),
            ("devices", Value::Boolean(true)),
        ],
        vec![
            ("operation", Value::Text("next".into())),
            ("devices", Value::List(vec![Value::Integer(1)])),
        ],
        vec![
            ("operation", Value::Text("next".into())),
            ("devices", Value::List(vec![Value::Text("missing".into())])),
        ],
    ] {
        let mut rig = Rig::new("sound-output", true);
        rig.invoke(action("sound-output", &values));
        assert!(matches!(rig.recv(), Message::Problem(_)));
        feedback(rig.recv(), binding(1), "Speakers", None);
        assert!(rig.system.calls().is_empty());
    }
    let mut rig = Rig::new("sound-output", true);
    rig.system.fail(Some("set-output"));
    rig.invoke(action(
        "sound-output",
        &[("device", Value::Text("b".into()))],
    ));
    rig.problem("set the sound");
    feedback(rig.recv(), binding(1), "Speakers", None);
}

#[test]
fn output_follows_external_changes_and_stops_listeners_without_idle_reads() {
    let mut rig = Rig::new("sound-output", true);
    rig.handler.handle(Request::Appear {
        binding: binding(1),
        action: action("sound-output", &[]),
    });
    feedback(rig.recv(), binding(1), "Speakers", None);
    let reads = rig.system.output_reads.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(30));
    assert_eq!(rig.system.output_reads.load(Ordering::SeqCst), reads);
    *rig.system.output.lock().unwrap() = "b".into();
    rig.system.output_notifications.lock().unwrap()[0]
        .send(())
        .unwrap();
    feedback(rig.recv(), binding(1), "Headphones", None);
    rig.disappear(1);
    assert!(rig.system.output_notifications.lock().unwrap()[0]
        .send(())
        .is_err());
}

fn appear(rig: &mut Rig, index: u8) {
    rig.handler.handle(Request::Appear {
        binding: binding(index),
        action: action("microphone-mute", &[]),
    });
}
fn feedback(message: Message, id: BindingId, label: &str, state: Option<FeedbackState>) {
    assert_eq!(
        message,
        Message::Feedback {
            binding: id,
            feedback: Some(Feedback {
                label: Some(label.into()),
                state,
                ..Default::default()
            })
        }
    );
}
fn changed(rig: &Rig) {
    rig.system
        .input_notifications
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .send(())
        .unwrap();
}

#[test]
fn microphone_toggles_and_reports_all_active_bindings() {
    let mut rig = Rig::new("microphone-mute", true);
    appear(&mut rig, 1);
    feedback(
        rig.recv(),
        binding(1),
        "Microphone on",
        Some(FeedbackState::Off),
    );
    appear(&mut rig, 2);
    feedback(
        rig.recv(),
        binding(2),
        "Microphone on",
        Some(FeedbackState::Off),
    );
    rig.invoke(action(
        "microphone-mute",
        &[("extra", Value::Text("kept".into()))],
    ));
    // Invoke has binding 1, and both visible bindings change.
    feedback(
        rig.recv(),
        binding(1),
        "Microphone muted",
        Some(FeedbackState::On),
    );
    feedback(
        rig.recv(),
        binding(2),
        "Microphone muted",
        Some(FeedbackState::On),
    );
    assert_eq!(rig.system.calls(), [Call::MicrophoneMuted(true)]);
    rig.invoke(action("microphone-mute", &[]));
    feedback(
        rig.recv(),
        binding(1),
        "Microphone on",
        Some(FeedbackState::Off),
    );
    feedback(
        rig.recv(),
        binding(2),
        "Microphone on",
        Some(FeedbackState::Off),
    );
    assert_eq!(rig.system.asks.load(Ordering::SeqCst), 0);
}

#[test]
fn microphone_follows_events_without_polling_and_stops_after_disappear() {
    let mut rig = Rig::new("microphone-mute", true);
    appear(&mut rig, 1);
    rig.recv();
    let reads = rig.system.microphone_reads.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(35));
    assert_eq!(rig.system.microphone_reads.load(Ordering::SeqCst), reads);
    *rig.system.microphone.lock().unwrap() = Some(true);
    changed(&rig);
    feedback(
        rig.recv(),
        binding(1),
        "Microphone muted",
        Some(FeedbackState::On),
    );
    changed(&rig);
    wait_until(|| rig.system.microphone_reads.load(Ordering::SeqCst) >= reads + 2);
    assert!(rig.messages.try_recv().is_err());
    rig.disappear(1);
    assert!(rig.system.input_notifications.lock().unwrap()[0]
        .send(())
        .is_err());
    appear(&mut rig, 2);
    feedback(
        rig.recv(),
        binding(2),
        "Microphone muted",
        Some(FeedbackState::On),
    );
    let system = rig.system.clone();
    drop(rig);
    assert!(system
        .input_notifications
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .send(())
        .is_err());
}

#[test]
fn microphone_without_mute_is_explicit_and_does_not_use_volume() {
    let mut rig = Rig::new("microphone-mute", true);
    *rig.system.microphone.lock().unwrap() = None;
    appear(&mut rig, 1);
    feedback(rig.recv(), binding(1), "Not supported", None);
    rig.invoke(action("microphone-mute", &[]));
    rig.problem("no writable master mute");
    feedback(rig.recv(), binding(1), "Not supported", None);
    assert!(rig.system.calls().is_empty());
    assert_eq!(rig.system.volume_reads.load(Ordering::SeqCst), 0);
    *rig.system.microphone.lock().unwrap() = Some(false);
    changed(&rig);
    feedback(
        rig.recv(),
        binding(1),
        "Microphone on",
        Some(FeedbackState::Off),
    );
}

#[test]
fn microphone_errors_are_visible_and_reads_recover() {
    let mut rig = Rig::new("microphone-mute", true);
    rig.system.fail(Some("microphone"));
    appear(&mut rig, 1);
    rig.problem("read the microphone");
    feedback(rig.recv(), binding(1), "Not supported", None);
    changed(&rig);
    std::thread::sleep(Duration::from_millis(20));
    assert!(rig.messages.try_recv().is_err());
    rig.system.fail(None);
    changed(&rig);
    feedback(
        rig.recv(),
        binding(1),
        "Microphone on",
        Some(FeedbackState::Off),
    );
    rig.system.fail(Some("set-microphone"));
    rig.invoke(action("microphone-mute", &[]));
    rig.problem("set the microphone");
    feedback(
        rig.recv(),
        binding(1),
        "Microphone on",
        Some(FeedbackState::Off),
    );
}

use super::*;
use opencraft_engine::{BindingId, FeedbackState};

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

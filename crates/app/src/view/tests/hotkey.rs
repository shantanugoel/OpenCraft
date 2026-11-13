//! The hotkey recorder of a gesture card shows the symbols of the hotkey
//! (issue 232), with a fake kind that has one hotkey field.

use super::*;
use gpui_kit::base::test_support::texts;

fn kind() -> KindDescription {
    KindDescription {
        kind: "test.hotkey".into(),
        name: "Hotkey".into(),
        fields: vec![Field {
            key: "hotkey".into(),
            label: "Hotkey".into(),
            input: InputType::Hotkey,
            default: Some(Value::Text(String::new())),
        }],
        ..Default::default()
    }
}

fn binding() -> BindingId {
    BindingId {
        page: 0,
        control: ControlId::new(ControlKind::Key, 2),
        gesture: GestureKind::Press,
    }
}

fn action(hotkey: &str) -> Action {
    Action {
        kind: "test.hotkey".into(),
        settings: Settings::from([("hotkey".to_string(), Value::Text(hotkey.into()))]),
    }
}

fn event(hotkey: &str) -> Event {
    Event::Binding(BindingState {
        binding: binding(),
        action: Some(action(hotkey)),
        available: true,
    })
}

/// The texts that the last frame painted.
fn shown(cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|window, _| {
        texts(window)
            .iter()
            .map(|text| text.text().to_string())
            .collect()
    })
}

fn show(cx: &mut VisualTestContext, text: &str) -> bool {
    shown(cx).iter().any(|shown| shown == text)
}

#[gpui_kit::test]
fn the_recorder_shows_the_symbols_in_the_order_of_macos(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    tx.try_send(Event::Kinds(vec![kind()])).unwrap();
    tx.try_send(event("shift+cmd+m")).unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("hotkey-0").is_some(), "the card is open");
    // The profile has the text of decision 0014. The recorder shows the
    // symbols: Control, Option, Shift, Command, and then the key.
    assert!(show(cx, "⇧ ⌘ M"), "{:?}", shown(cx));
    assert!(!show(cx, "shift+cmd+m"));
    for (value, symbols) in [
        ("cmd+c", "⌘ C"),
        ("cmd+shift+alt+ctrl+return", "⌃ ⌥ ⇧ ⌘ ↩"),
        ("alt+shift+space", "⌥ ⇧ Space"),
        ("cmd+f12", "⌘ F12"),
        ("ctrl+left", "⌃ ←"),
        ("shift+cmd+minus", "⇧ ⌘ -"),
        ("forwarddelete", "⌦"),
    ] {
        tx.try_send(event(value)).unwrap();
        cx.run_until_parked();
        assert!(show(cx, symbols), "{value}: {:?}", shown(cx));
    }

    // A value that the symbols do not know stays as text.
    tx.try_send(event("hyper+a")).unwrap();
    cx.run_until_parked();
    assert!(show(cx, "hyper+a"));
    // An empty value shows the prompt.
    tx.try_send(event("")).unwrap();
    cx.run_until_parked();
    assert!(show(cx, "Record hotkey…"));

    // A click starts the record, and the key sets the value. The command
    // has the text form, and the recorder shows the symbols.
    click(cx, "hotkey-0");
    assert!(show(cx, "Press a key…"));
    cx.simulate_keystrokes("shift-cmd-4");
    cx.run_until_parked();
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: binding(),
            action: Some(action("shift+cmd+4")),
        })
    );
    assert!(show(cx, "⇧ ⌘ 4"), "{:?}", shown(cx));
    assert!(!show(cx, "Press a key…"));
    tx.try_send(event("shift+cmd+4")).unwrap();
    cx.run_until_parked();
    assert!(show(cx, "⇧ ⌘ 4"));
}

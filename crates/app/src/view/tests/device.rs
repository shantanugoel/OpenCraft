//! The device inspector: the selection of the device, the controls, the
//! commands, the throttle of the commands, and the settings that the model
//! has (decisions 0022 and 0029, section 10.7).

use super::super::device::{KNOB_COLOURS, SETTINGS_INTERVAL};
use super::*;
use crate::shell::ShellCommand;
use gpui_kit::component::slider::SliderEvent;
use opencraft_engine::{DeviceLayout, DeviceSettings, KnobLightMode, KnobLights, SettingsSupport};

fn defaults() -> DeviceSettings {
    DeviceSettings {
        brightness: 80,
        haptic: true,
        knob_lights: KnobLights {
            mode: KnobLightMode::Effect,
            brightness: 100,
            colors: vec![Color::WHITE; 2],
        },
    }
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

/// The settings of each settings command, in order.
fn sent_settings(sent: &Commands) -> Vec<DeviceSettings> {
    sent.borrow()
        .iter()
        .filter_map(|c| match c {
            Command::SetDeviceSettings(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// A window with the settings `settings` and the device selected.
fn open(
    cx: &mut TestAppContext,
    settings: DeviceSettings,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, Event::DeviceSettings(settings));
    click(cx, "device-card");
    (view, cx, tx, sent)
}

/// Let the throttle time of the settings commands end.
fn wait_interval(cx: &mut VisualTestContext) {
    cx.executor().advance_clock(SETTINGS_INTERVAL * 2);
    cx.run_until_parked();
}

#[gpui_kit::test]
fn the_inspector_shows_the_engine_settings_and_works_with_no_device(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "device-card");
    assert!(cx.debug_bounds("device-settings").is_some());
    // No settings from the engine yet: the header, and no control.
    assert!(cx.debug_bounds("inspector-header").is_some());
    assert!(cx.debug_bounds("brightness-slider").is_none());
    send(cx, &tx, Event::DeviceSettings(defaults()));
    cx.update(|_, cx| assert!(!view.read(cx).state.connected));
    for id in [
        "brightness-slider",
        "brightness-value",
        "display",
        "feedback",
        "haptic-switch",
        "knob-lights",
        "knob-effect",
        "knob-colours",
        "knob-off",
        "knob-brightness-slider",
        "knob-brightness-value",
        "device-note",
    ] {
        assert!(cx.debug_bounds(id).is_some(), "No element: {id}");
    }
    // The colours show only in the mode "colours".
    assert!(cx.debug_bounds("knob-1-colour").is_none());
    assert!(sent.borrow().is_empty(), "Selecting sends nothing");

    // The toolbar has no button for the device settings.
    assert!(cx.debug_bounds("open-device-settings").is_none());
    cx.update(|_, cx| assert!(view.read(cx).device.open));

    // The editor of a control comes back when the user selects a surface.
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("device-settings").is_none());
    cx.update(|_, cx| assert_eq!(view.read(cx).framed_surface(), Some(key())));
    // While the device is the selection, the selected key has no frame.
    click(cx, "device-card");
    cx.update(|_, cx| {
        assert_eq!(view.read(cx).framed_surface(), None);
        assert_eq!(view.read(cx).framed_control(), None);
    });
    click(cx, "surface-key face 2");
    cx.update(|_, cx| assert_eq!(view.read(cx).framed_surface(), Some(key())));
    assert!(cx.debug_bounds("editor").is_some());
    assert!(sent_settings(&sent).is_empty());
}

#[gpui_kit::test]
fn the_header_shows_the_model_and_the_connection(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = open(cx, defaults());
    assert!(!cx.update(|_, cx| view.read(cx).state.connected));
    let header = cx.debug_bounds("inspector-header").unwrap();
    let state = cx.debug_bounds("device-header-state").unwrap();
    assert!(state.top() >= header.top() && state.bottom() <= header.bottom());
    send(cx, &tx, Event::Connected);
    assert!(cx.update(|_, cx| view.read(cx).state.connected));
    // The inspector keeps the settings and the header after the event.
    assert!(cx.debug_bounds("device-header-state").is_some());
    assert!(cx.debug_bounds("brightness-slider").is_some());
}

#[gpui_kit::test]
fn the_body_of_the_device_selects_the_device(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = window(cx);
    let device = |cx: &mut VisualTestContext| cx.update(|_, cx| view.read(cx).device.open);
    click(cx, "surface-key face 2");
    assert!(!device(cx));
    // A click at the margin of the body, outside the display.
    let body = cx.debug_bounds("device-body").unwrap();
    let point = gpui_kit::point(body.left() + px(6.), body.center().y);
    cx.simulate_mouse_move(point, None, Modifiers::default());
    cx.simulate_click(point, Modifiers::default());
    cx.run_until_parked();
    assert!(device(cx));
    assert!(cx.debug_bounds("device-settings").is_some());
    // The device has the outline of the selection, and no control has it.
    cx.update(|_, cx| assert_eq!(view.read(cx).framed_surface(), None));
    // A key or a knob takes the selection back.
    click(cx, "control-knob 2");
    assert!(!device(cx));
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).framed_control(),
            Some(ControlId::new(ControlKind::Knob, 2))
        )
    });
    // A click on an empty part of the display selects the page, not the device.
    click(cx, "device-card");
    assert!(device(cx));
    let display = cx.debug_bounds("display").unwrap();
    let point = gpui_kit::point(display.left() + px(1.), display.top() + px(1.));
    cx.simulate_mouse_move(point, None, Modifiers::default());
    cx.simulate_click(point, Modifiers::default());
    cx.run_until_parked();
    assert!(!device(cx));
    cx.update(|_, cx| assert!(view.read(cx).state.page_selected));
    assert!(sent.borrow().is_empty());
}

#[gpui_kit::test]
fn the_controls_send_all_settings_with_one_change(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = open(cx, defaults());
    // The switch changes the haptic feedback.
    click(cx, "haptic-switch");
    let mut want = defaults();
    want.haptic = false;
    assert_eq!(sent_settings(&sent), [want.clone()]);
    wait_interval(cx);
    send(cx, &tx, Event::DeviceSettings(want.clone()));
    click(cx, "haptic-switch");
    want.haptic = true;
    assert_eq!(sent_settings(&sent).last(), Some(&want));
    wait_interval(cx);
    send(cx, &tx, Event::DeviceSettings(want.clone()));

    click(cx, "knob-colours");
    want.knob_lights.mode = KnobLightMode::Colors;
    assert_eq!(sent_settings(&sent).last(), Some(&want));
    wait_interval(cx);
    send(cx, &tx, Event::DeviceSettings(want.clone()));

    // A colour sample of the popover sets the colour of its knob.
    click(cx, "knob-2-colour");
    assert!(cx.debug_bounds("knob-colour-popover").is_some());
    click(cx, "knob-2-colour-1");
    want.knob_lights.colors[1] = colour(KNOB_COLOURS[1]);
    assert_eq!(sent_settings(&sent).last(), Some(&want));
    wait_interval(cx);
    send(cx, &tx, Event::DeviceSettings(want.clone()));
    // Escape closes the popover. The colour stays.
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("knob-colour-popover").is_none());

    click(cx, "knob-off");
    want.knob_lights.mode = KnobLightMode::Off;
    assert_eq!(sent_settings(&sent).last(), Some(&want));
    wait_interval(cx);
    // A click on the chosen value sends nothing.
    send(cx, &tx, Event::DeviceSettings(want.clone()));
    let count = sent_settings(&sent).len();
    click(cx, "knob-off");
    assert_eq!(sent_settings(&sent).len(), count);
    // No other command.
    assert_eq!(sent.borrow().len(), count);

    click(cx, "knob-effect");
    want.knob_lights.mode = KnobLightMode::Effect;
    assert_eq!(sent_settings(&sent).last(), Some(&want));
}

#[gpui_kit::test]
fn the_colour_buttons_show_the_knob_and_the_hex_value(cx: &mut TestAppContext) {
    let mut settings = defaults();
    settings.knob_lights.mode = KnobLightMode::Colors;
    settings.knob_lights.colors = vec![Color::rgb(0x14, 0xb8, 0xa6), Color::WHITE];
    let (view, cx, _tx, sent) = open(cx, settings);
    // One button for each knob light, in a grid of four columns.
    let first = cx.debug_bounds("knob-1-colour").unwrap();
    let second = cx.debug_bounds("knob-2-colour").unwrap();
    assert!((first.size.width - second.size.width).abs() < px(1.));
    assert_eq!(first.size.height, second.size.height);
    assert_eq!(first.top(), second.top());
    assert!(second.left() > first.right());
    let grid = cx.debug_bounds("knob-colour-grid").unwrap();
    assert!(second.right() < grid.right() - first.size.width);
    assert!(cx.debug_bounds("knob-1-colour-hex").is_some());
    assert!(cx.debug_bounds("knob-colour-popover").is_none());
    // The popover has the nine samples and the field.
    click(cx, "knob-1-colour");
    for id in [
        "knob-1-colour-0",
        "knob-1-colour-1",
        "knob-1-colour-2",
        "knob-1-colour-3",
        "knob-1-colour-4",
        "knob-1-colour-5",
        "knob-1-colour-6",
        "knob-1-colour-7",
        "knob-1-colour-8",
        "knob-colour-field",
    ] {
        assert!(cx.debug_bounds(id).is_some(), "No element: {id}");
    }
    assert_eq!(KNOB_COLOURS.len(), 9);
    cx.update(|_, cx| {
        let input = view.read(cx).device.colour_input.clone();
        assert_eq!(input.read(cx).value().as_ref(), "#14B8A6");
    });
    // Escape closes the popover with no change.
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("knob-colour-popover").is_none());
    // A second click on the button closes it too.
    click(cx, "knob-2-colour");
    assert!(cx.debug_bounds("knob-colour-popover").is_some());
    click(cx, "knob-2-colour");
    assert!(cx.debug_bounds("knob-colour-popover").is_none());
    assert!(sent_settings(&sent).is_empty());
}

#[gpui_kit::test]
fn the_light_brightness_control_is_off_in_the_mode_off(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = open(cx, defaults());
    let mut settings = defaults();
    settings.knob_lights.mode = KnobLightMode::Off;
    send(cx, &tx, Event::DeviceSettings(settings));
    // A click on the slider changes nothing, and the value stays.
    let bounds = cx.debug_bounds("knob-brightness-slider").unwrap();
    cx.simulate_click(
        gpui_kit::point(bounds.left() + px(1.), bounds.center().y),
        Modifiers::default(),
    );
    cx.run_until_parked();
    wait_interval(cx);
    assert!(sent_settings(&sent).is_empty());
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx)
                .shown_settings()
                .unwrap()
                .knob_lights
                .brightness,
            100
        )
    });
    assert!(cx.debug_bounds("knob-brightness-value").is_some());
}

#[gpui_kit::test]
fn a_moving_control_sends_at_most_one_command_in_each_interval_and_the_last_value(
    cx: &mut TestAppContext,
) {
    let (view, cx, tx, sent) = open(cx, defaults());
    // The user moves the brightness control through 40 values quickly.
    // The slider emits a change for each value, as in a drag.
    let slider = cx.update(|_, cx| view.read(cx).device.brightness.clone().unwrap());
    for value in 30..70u8 {
        slider.update_in(cx, |slider, window, cx| {
            slider.set_value(f32::from(value), window, cx);
            cx.emit(SliderEvent::Change(f32::from(value).into()));
        });
    }
    cx.run_until_parked();
    let values =
        |sent: &Commands| -> Vec<u8> { sent_settings(sent).iter().map(|s| s.brightness).collect() };
    assert_eq!(values(&sent), [30], "the first change goes at once");
    // An older reply of the engine does not move the control back.
    let mut reply = defaults();
    reply.brightness = 30;
    send(cx, &tx, Event::DeviceSettings(reply));
    cx.update(|_, cx| {
        assert_eq!(view.read(cx).shown_settings().unwrap().brightness, 69);
        let slider = view.read(cx).device.brightness.clone().unwrap();
        assert_eq!(slider.read(cx).value().end(), 69.);
    });
    cx.executor().advance_clock(SETTINGS_INTERVAL);
    cx.run_until_parked();
    assert_eq!(values(&sent), [30, 69], "the last value at the end");
    wait_interval(cx);
    assert_eq!(values(&sent), [30, 69], "nothing more");
    let mut reply = defaults();
    reply.brightness = 69;
    send(cx, &tx, Event::DeviceSettings(reply));
    cx.update(|_, cx| assert_eq!(view.read(cx).shown_settings().unwrap().brightness, 69));
}

#[gpui_kit::test]
fn a_click_on_the_brightness_control_sends_its_value(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = open(cx, defaults());
    let bounds = cx.debug_bounds("brightness-slider").unwrap();
    // The left end is the lowest brightness of the model: 15.
    let left = gpui_kit::point(bounds.left() + px(1.), bounds.center().y);
    cx.simulate_mouse_move(left, None, Modifiers::default());
    cx.simulate_click(left, Modifiers::default());
    cx.run_until_parked();
    wait_interval(cx);
    let got = sent_settings(&sent);
    let last = got.last().expect("a settings command");
    assert!(last.brightness <= 20, "{}", last.brightness);
    assert!(last.brightness >= 15, "{}", last.brightness);
}

#[gpui_kit::test]
fn a_colour_field_sends_only_a_complete_colour(cx: &mut TestAppContext) {
    let mut settings = defaults();
    settings.knob_lights.mode = KnobLightMode::Colors;
    let (view, cx, tx, sent) = open(cx, settings.clone());
    click(cx, "knob-1-colour");
    cx.update(|_, cx| {
        let input = view.read(cx).device.colour_input.clone();
        assert_eq!(input.read(cx).value().as_ref(), "#FFFFFF");
    });
    // The field has the focus.
    cx.update(|window, cx| {
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
    });
    for key in ["#", "0", "0", "f", "f", "0"] {
        cx.update(|window, cx| window.dispatch_keystroke(Keystroke::parse(key).unwrap(), cx));
        cx.run_until_parked();
    }
    assert!(sent_settings(&sent).is_empty(), "no command for a part");
    cx.update(|window, cx| window.dispatch_keystroke(Keystroke::parse("0").unwrap(), cx));
    cx.run_until_parked();
    settings.knob_lights.colors[0] = Color::rgb(0, 0xff, 0);
    assert_eq!(sent_settings(&sent), [settings.clone()]);
    // The reply does not change the text that the user typed.
    wait_interval(cx);
    send(cx, &tx, Event::DeviceSettings(settings));
    cx.update(|_, cx| {
        let input = view.read(cx).device.colour_input.clone();
        assert_eq!(input.read(cx).value().as_ref(), "#00ff00");
    });
    // Return closes the popover. The colour stays.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(cx.debug_bounds("knob-colour-popover").is_none());
    assert_eq!(sent_settings(&sent).len(), 1);
}

#[gpui_kit::test]
fn the_inspector_shows_only_the_settings_that_the_model_has(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = open(cx, defaults());
    let Event::Layout(layout) = crate::state::tests::layout() else {
        panic!()
    };
    let plain = DeviceLayout {
        settings: SettingsSupport {
            lowest_brightness: 15,
            knob_lights: 0,
            haptic: false,
        },
        ..(*layout).clone()
    };
    send(cx, &tx, Event::Layout(Arc::new(plain)));
    let mut settings = defaults();
    settings.knob_lights.colors.clear();
    send(cx, &tx, Event::DeviceSettings(settings));
    assert!(cx.debug_bounds("brightness-slider").is_some());
    assert!(cx.debug_bounds("display").is_some());
    for id in [
        "haptic-switch",
        "feedback",
        "knob-lights",
        "knob-effect",
        "knob-brightness-slider",
    ] {
        assert!(cx.debug_bounds(id).is_none(), "{id} shows");
    }
}

#[gpui_kit::test]
fn a_window_that_opens_later_shows_the_settings_of_the_shell(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Dark, None, cx);
    });
    let (events, receiver) = async_channel::unbounded();
    let (commands, command_receiver) = async_channel::unbounded();
    let shell = cx.update(|cx| {
        cx.new(|cx| {
            let mut shell = Shell::new(
                std::path::PathBuf::from("not-written"),
                crate::settings::Settings::default(),
                Rc::new(|_| {}),
            );
            shell.listen(receiver, command_receiver, cx);
            shell
        })
    });
    let mut settings = defaults();
    settings.brightness = 33;
    events.try_send(crate::state::tests::layout()).unwrap();
    events
        .try_send(Event::DeviceSettings(settings.clone()))
        .unwrap();
    cx.run_until_parked();
    assert!(cx.update(|cx| cx.windows().is_empty()));
    assert_eq!(
        cx.update(|cx| shell.read(cx).state.device_settings.clone()),
        Some(settings.clone())
    );
    commands.try_send(ShellCommand::Open).unwrap();
    cx.run_until_parked();
    let view: Entity<DeviceView> = cx
        .update(|cx| {
            let window = cx.windows().into_iter().next()?;
            window
                .update(cx, |root, _, cx| {
                    root.downcast::<gpui_kit::base::Root>()
                        .ok()
                        .and_then(|root| root.read(cx).view().clone().downcast().ok())
                })
                .ok()
                .flatten()
        })
        .expect("a window");
    cx.update(|cx| assert_eq!(view.read(cx).state.device_settings, Some(settings)));
}

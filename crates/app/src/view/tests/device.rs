//! The "Device settings" area: the controls, the commands, the throttle of
//! the commands, and the settings that the model has (decision 0022).

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

/// A window with the settings `settings` and the area open.
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
    click(cx, "open-device-settings");
    (view, cx, tx, sent)
}

/// Let the throttle time of the settings commands end.
fn wait_interval(cx: &mut VisualTestContext) {
    cx.executor().advance_clock(SETTINGS_INTERVAL * 2);
    cx.run_until_parked();
}

#[gpui_kit::test]
fn the_area_shows_the_engine_settings_and_works_with_no_device(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "open-device-settings");
    assert!(cx.debug_bounds("device-settings").is_some());
    // No settings from the engine yet: no control.
    assert!(cx.debug_bounds("brightness-slider").is_none());
    send(cx, &tx, Event::DeviceSettings(defaults()));
    cx.update(|_, cx| assert!(!view.read(cx).state.connected));
    for id in [
        "brightness-slider",
        "brightness-value",
        "haptic-on",
        "haptic-off",
        "knob-effect",
        "knob-colours",
        "knob-off",
        "knob-brightness-slider",
    ] {
        assert!(cx.debug_bounds(id).is_some(), "No element: {id}");
    }
    // The colours show only in the mode "colours".
    assert!(cx.debug_bounds("knob-1-colour-0").is_none());
    assert!(sent.borrow().is_empty(), "Opening the area sends nothing");

    // The surface editor comes back when the user selects a surface.
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("device-settings").is_none());
    assert!(cx.debug_bounds("editor").is_some());
    assert!(sent_settings(&sent).is_empty());
}

#[gpui_kit::test]
fn the_buttons_send_all_settings_with_one_change(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = open(cx, defaults());
    click(cx, "haptic-off");
    let mut want = defaults();
    want.haptic = false;
    assert_eq!(sent_settings(&sent), [want.clone()]);
    wait_interval(cx);
    send(cx, &tx, Event::DeviceSettings(want.clone()));

    click(cx, "knob-colours");
    want.knob_lights.mode = KnobLightMode::Colors;
    assert_eq!(sent_settings(&sent).last(), Some(&want));
    wait_interval(cx);
    send(cx, &tx, Event::DeviceSettings(want.clone()));

    // A colour button sets the colour of its knob and the text of its field.
    click(cx, "knob-2-colour-1");
    want.knob_lights.colors[1] = colour(KNOB_COLOURS[1]);
    assert_eq!(sent_settings(&sent).last(), Some(&want));
    cx.update(|_, cx| {
        let input = view.read(cx).device.colours[1].clone();
        assert_eq!(input.read(cx).value().as_ref(), "#FF0000");
    });
    wait_interval(cx);
    send(cx, &tx, Event::DeviceSettings(want.clone()));

    click(cx, "knob-off");
    want.knob_lights.mode = KnobLightMode::Off;
    assert_eq!(sent_settings(&sent).last(), Some(&want));
    wait_interval(cx);
    // A click on the chosen value sends nothing.
    send(cx, &tx, Event::DeviceSettings(want.clone()));
    let count = sent_settings(&sent).len();
    click(cx, "knob-off");
    click(cx, "haptic-off");
    assert_eq!(sent_settings(&sent).len(), count);
    // No other command.
    assert_eq!(sent.borrow().len(), count);
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
    cx.update(|window, cx| {
        let input = view.read(cx).device.colours[0].clone();
        assert_eq!(input.read(cx).value().as_ref(), "#FFFFFF");
        window.focus(&input.focus_handle(cx), cx);
    });
    cx.run_until_parked();
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
        let input = view.read(cx).device.colours[0].clone();
        assert_eq!(input.read(cx).value().as_ref(), "#00ff00");
    });
}

#[gpui_kit::test]
fn the_area_shows_only_the_settings_that_the_model_has(cx: &mut TestAppContext) {
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
    for id in ["haptic-on", "knob-effect", "knob-brightness-slider"] {
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

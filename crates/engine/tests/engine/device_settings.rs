//! Device settings tests: the commands after each start sequence and at each
//! change, the settings file, and the models with fewer settings. The tests
//! use the fake transport, temporary directories, and no thread
//! (decision 0022).

use super::*;
use opencraft_core::protocol;
use opencraft_engine::device_settings::{settings_path, SETTINGS_FILE};
use opencraft_engine::{KnobLightMode, KnobLights, SettingsSupport};

/// A model with no knob lights and no haptic feedback, and a different
/// lowest brightness.
static PLAIN_MODEL: opencraft_core::Descriptor = opencraft_core::Descriptor {
    name: "Plain test model",
    model: "test-plain",
    settings: SettingsSupport {
        lowest_brightness: 30,
        knob_lights: 0,
        haptic: false,
    },
    ..DESCRIPTORS[0]
};

const GREEN: Color = Color::rgb(0, 255, 0);
const WHITE: Color = Color::WHITE;

fn n4_defaults() -> DeviceSettings {
    DeviceSettings::defaults(DESCRIPTORS[0].settings)
}

/// Settings that are different from the defaults in each value.
fn custom() -> DeviceSettings {
    DeviceSettings {
        brightness: 42,
        screen_off_minutes: 0,
        haptic: false,
        knob_lights: KnobLights {
            mode: KnobLightMode::Colors,
            brightness: 30,
            colors: vec![RED, GREEN, BLUE, WHITE],
        },
    }
}

fn set(rig: &mut Rig, settings: DeviceSettings) {
    assert!(rig.engine.command(Command::SetDeviceSettings(settings)));
}

/// Change the current settings with `change`, and send the command.
fn change(rig: &mut Rig, change: impl FnOnce(&mut DeviceSettings)) {
    let mut settings = rig.engine.device_settings().clone();
    change(&mut settings);
    set(rig, settings);
}

fn settings_events(events: &[Event]) -> Vec<DeviceSettings> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::DeviceSettings(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

fn reports(commands: &[Vec<u8>]) -> Vec<Vec<u8>> {
    commands.iter().cloned().map(full_report).collect()
}

/// The device commands of `settings` after the start sequence, as full
/// reports, in the order of decision 0022: `SETLB`, `LBLIG`, `QUCMD`.
fn after_start(settings: &DeviceSettings) -> Vec<Vec<u8>> {
    let lights = &settings.knob_lights;
    let colors: Vec<[u8; 3]> = lights.colors.iter().map(|c| c.0).collect();
    reports(&[
        protocol::set_led_colors(&colors),
        protocol::set_led_brightness(lights.brightness),
        protocol::set_vibration(settings.haptic),
    ])
}

/// Check the start of a full draw: `DIS`, `LIG` with `brightness`,
/// `CLE 0xFF`, `STP`, then `settings` exactly. Then the removal of the
/// background (the page has no background), the images, and the commit.
fn assert_start(writes: &[Vec<u8>], brightness: u8, settings: &[Vec<u8>]) {
    let start = reports(&[
        protocol::init(),
        protocol::set_brightness(brightness),
        protocol::clear_slot(0xFF),
        protocol::commit(),
    ]);
    assert_eq!(&writes[..4], start.as_slice(), "the start sequence");
    assert_eq!(
        &writes[4..4 + settings.len()],
        settings,
        "the device settings after the start sequence"
    );
    let mut next = &writes[4 + settings.len()];
    if next[..11] == *b"\0CRT\0\0BGCLE" {
        next = &writes[5 + settings.len()];
    }
    assert_eq!(&next[..9], b"\0CRT\0\0BAT", "then the images");
    assert_eq!(writes.last(), Some(&full_report(protocol::commit())));
}

/// A profiles directory with `text` in the settings file.
fn dir_with_settings(text: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(SETTINGS_FILE), text).unwrap();
    dir
}

fn file_text(rig: &Rig) -> Option<String> {
    fs::read_to_string(settings_path(rig.dir.path())).ok()
}

// --- The commands after the start sequence -------------------------------------

#[test]
fn the_defaults_after_the_start_give_the_device_of_the_earlier_phases() {
    let mut rig = Rig::new();
    let device = rig.connect();
    // Brightness 80, the effect (all colours black) at full brightness, and
    // haptic feedback on.
    assert_start(
        &raw_writes(&device),
        80,
        &reports(&[
            protocol::set_led_colors(&[[0, 0, 0]; 4]),
            protocol::set_led_brightness(100),
            protocol::set_vibration(true),
        ]),
    );
    // No file until the user changes a setting.
    rig.poll_for(Duration::from_secs(2));
    assert_eq!(file_text(&rig), None);
}

#[test]
fn each_connection_sends_all_settings_in_the_order_of_the_decision() {
    let mut rig = Rig::new();
    set(&mut rig, custom());
    let first = rig.connect();
    let writes = raw_writes(&first);
    assert_start(&writes, 42, &after_start(&custom()));
    // The exact bytes of the knob light colours and of the haptic command.
    assert_eq!(
        &writes[4][..23],
        b"\0CRT\0\0SETLB\xC8\x1E\x1E\x00\xFF\x00\x14\x28\xDC\xFF\xFF\xFF"
    );
    assert_eq!(&writes[5][..12], b"\0CRT\0\0LBLIG\x1E");
    assert_eq!(&writes[6][..17], b"\0CRT\0\0QUCMD\x1F\x11\x00\xFF\x00\x11");

    // A reconnection sends all settings again: the device keeps no setting
    // after a USB restart (measured).
    first.borrow_mut().disconnect();
    rig.poll_for(Duration::from_secs(1));
    let second = rig.connect();
    assert_start(&raw_writes(&second), 42, &after_start(&custom()));

    // A wake of the computer does a full draw with all settings.
    rig.sleep_computer(Duration::from_secs(70));
    rig.engine.poll(POLL_INTERVAL);
    let writes = raw_writes(&second);
    assert_eq!(writes[0], full_report(protocol::keep_alive()));
    assert_start(&writes[1..], 42, &after_start(&custom()));
}

#[test]
fn a_full_draw_after_a_failed_write_sends_all_settings() {
    let mut rig = Rig::new();
    set(&mut rig, custom());
    let device = rig.connect();
    raw_writes(&device);
    device.borrow_mut().fail_writes("the write failed");
    rig.set_color(KEY_3, BLUE);
    device.borrow_mut().clear_errors();
    rig.poll_for(Duration::from_secs(1));
    assert_start(&raw_writes(&device), 42, &after_start(&custom()));
}

#[test]
fn the_mode_colors_with_all_colours_black_sends_the_commands_of_the_mode_off() {
    let mut rig = Rig::new();
    let mut settings = custom();
    settings.knob_lights.colors = vec![Color::BLACK; 4];
    set(&mut rig, settings.clone());
    let device = rig.connect();
    assert_start(
        &raw_writes(&device),
        42,
        &reports(&[
            protocol::set_led_brightness(0),
            protocol::set_vibration(false),
        ]),
    );
    // The setting keeps the mode and the brightness of the user.
    assert_eq!(rig.engine.device_settings(), &settings);
}

// --- A change ------------------------------------------------------------------

#[test]
fn a_change_sends_only_the_commands_of_that_setting_and_an_event() {
    let mut rig = Rig::new();
    let device = rig.connect();
    raw_writes(&device);
    rig.take_events();
    let mut want = n4_defaults();

    // Brightness: `LIG` only.
    change(&mut rig, |s| s.brightness = 15);
    want.brightness = 15;
    assert_eq!(
        raw_writes(&device),
        reports(&[protocol::set_brightness(15)])
    );
    assert_eq!(rig.take_events(), [Event::DeviceSettings(want.clone())]);

    // Haptic feedback: `QUCMD` only.
    change(&mut rig, |s| s.haptic = false);
    want.haptic = false;
    assert_eq!(
        raw_writes(&device),
        reports(&[protocol::set_vibration(false)])
    );
    assert_eq!(rig.take_events(), [Event::DeviceSettings(want.clone())]);

    // A colour in the mode "effect" changes no command, but the setting.
    change(&mut rig, |s| s.knob_lights.colors[0] = RED);
    want.knob_lights.colors[0] = RED;
    assert!(raw_writes(&device).is_empty());
    assert_eq!(rig.take_events(), [Event::DeviceSettings(want.clone())]);

    // The mode "colors": `SETLB` with the colours, then `LBLIG`.
    change(&mut rig, |s| s.knob_lights.mode = KnobLightMode::Colors);
    want.knob_lights.mode = KnobLightMode::Colors;
    assert_eq!(
        raw_writes(&device),
        reports(&[
            protocol::set_led_colors(&[[200, 30, 30], [255; 3], [255; 3], [255; 3]]),
            protocol::set_led_brightness(100),
        ])
    );

    // The knob light brightness: `SETLB` and `LBLIG`.
    change(&mut rig, |s| s.knob_lights.brightness = 7);
    assert_eq!(
        raw_writes(&device),
        reports(&[
            protocol::set_led_colors(&[[200, 30, 30], [255; 3], [255; 3], [255; 3]]),
            protocol::set_led_brightness(7),
        ])
    );

    // The mode "off": `LBLIG 0` only. Back to "effect": the last brightness.
    change(&mut rig, |s| s.knob_lights.mode = KnobLightMode::Off);
    assert_eq!(
        raw_writes(&device),
        reports(&[protocol::set_led_brightness(0)])
    );
    change(&mut rig, |s| s.knob_lights.mode = KnobLightMode::Effect);
    assert_eq!(
        raw_writes(&device),
        reports(&[
            protocol::set_led_colors(&[[0, 0, 0]; 4]),
            protocol::set_led_brightness(7),
        ])
    );
    rig.take_events();

    // The same settings again: no command and no event.
    let same = rig.engine.device_settings().clone();
    set(&mut rig, same);
    assert!(raw_writes(&device).is_empty());
    assert!(rig.take_events().is_empty());
}

#[test]
fn a_value_out_of_range_in_a_command_goes_into_the_range() {
    let mut rig = Rig::new();
    let device = rig.connect();
    raw_writes(&device);
    rig.take_events();
    let mut settings = n4_defaults();
    settings.brightness = 3;
    settings.knob_lights.brightness = 0;
    settings.knob_lights.colors = vec![RED];
    set(&mut rig, settings);

    let got = settings_events(&rig.take_events());
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].brightness, 15);
    assert_eq!(got[0].knob_lights.brightness, 1);
    assert_eq!(got[0].knob_lights.colors, [RED, WHITE, WHITE, WHITE]);
    assert_eq!(
        raw_writes(&device),
        reports(&[
            protocol::set_brightness(15),
            protocol::set_led_colors(&[[0, 0, 0]; 4]),
            protocol::set_led_brightness(1),
        ])
    );
}

#[test]
fn a_change_with_no_device_is_kept_and_sent_at_the_next_connection() {
    let mut rig = Rig::new();
    rig.take_events();
    let device = rig.plug();
    // The device is present, but the engine did not connect yet.
    set(&mut rig, custom());
    assert!(raw_writes(&device).is_empty());
    assert_eq!(settings_events(&rig.take_events()), [custom()]);
    rig.poll_for(Duration::from_secs(2));
    assert_start(&raw_writes(&device), 42, &after_start(&custom()));
}

#[test]
fn a_failed_write_at_a_change_draws_all_again_with_the_settings() {
    let mut rig = Rig::new();
    let device = rig.connect();
    raw_writes(&device);
    rig.take_events();
    device.borrow_mut().fail_writes("the write failed");
    set(&mut rig, custom());
    assert_eq!(problems(&rig.take_events()).len(), 1);
    device.borrow_mut().clear_errors();
    rig.poll_for(Duration::from_secs(1));
    assert_start(&raw_writes(&device), 42, &after_start(&custom()));
}

// --- A model with fewer settings -------------------------------------------------

#[test]
fn a_model_with_no_knob_lights_and_no_haptic_feedback_gets_no_such_command() {
    let mut rig = Rig::new();
    let device = rig.plug_model(&PLAIN_MODEL);
    rig.poll_for(Duration::from_secs(2));
    assert!(rig.engine.link().is_connected());
    let events = rig.take_events();
    let Some(Event::Layout(layout)) = events
        .iter()
        .find(|e| matches!(e, Event::Layout(l) if l.name == PLAIN_MODEL.name))
    else {
        panic!("no layout of the plain model: {events:?}");
    };
    assert_eq!(layout.settings, PLAIN_MODEL.settings);
    let settings = settings_events(&events);
    let plain = settings.last().unwrap();
    assert!(plain.knob_lights.colors.is_empty());
    assert_start(&raw_writes(&device), 80, &[]);

    // A change of the knob lights or of the haptic feedback sends nothing.
    change(&mut rig, |s| {
        s.haptic = false;
        s.knob_lights.mode = KnobLightMode::Off;
    });
    assert!(raw_writes(&device).is_empty());
    // The lowest brightness of this model is 30.
    change(&mut rig, |s| s.brightness = 20);
    assert_eq!(rig.engine.device_settings().brightness, 30);
    assert_eq!(
        raw_writes(&device),
        reports(&[protocol::set_brightness(30)])
    );
}

#[test]
fn each_model_has_its_own_settings_and_a_change_of_model_writes_the_old_ones() {
    let mut rig = Rig::new();
    set(&mut rig, custom());
    rig.take_events();
    let device = rig.plug_model(&PLAIN_MODEL);
    rig.poll_for(Duration::from_secs(2));
    // The settings of the first model are in the file before the change.
    let text = file_text(&rig).expect("the file is written at the change of the model");
    assert!(text.contains("[n4-pro]"), "{text}");
    assert!(text.contains("brightness = 42"), "{text}");
    let events = rig.take_events();
    assert_eq!(
        settings_events(&events),
        [DeviceSettings::defaults(PLAIN_MODEL.settings)]
    );
    // The event comes before Connected.
    assert_eq!(events.last(), Some(&Event::Connected));

    change(&mut rig, |s| s.brightness = 55);
    rig.poll_for(SAVE_DELAY + Duration::from_millis(100));
    device.borrow_mut().disconnect();
    rig.poll_for(Duration::from_secs(1));
    let device = rig.connect();
    assert_eq!(rig.engine.device_settings(), &custom());
    assert_start(&raw_writes(&device), 42, &after_start(&custom()));
    let text = file_text(&rig).unwrap();
    assert!(
        text.contains("[test-plain]") && text.contains("brightness = 55"),
        "{text}"
    );
}

// --- The settings file ---------------------------------------------------------

#[test]
fn the_file_is_written_after_the_delay_and_one_time_for_many_changes() {
    let mut rig = Rig::new();
    for brightness in 20..60 {
        change(&mut rig, |s| s.brightness = brightness);
        rig.engine.poll(POLL_INTERVAL);
    }
    assert_eq!(file_text(&rig), None, "no write while the changes come");
    rig.poll_for(SAVE_DELAY);
    let text = file_text(&rig).unwrap();
    assert!(text.starts_with("schema_version = 1\n"), "{text}");
    assert!(text.contains("brightness = 59"), "{text}");
    // No profile change: the engine writes no profile file.
    assert_eq!(rig.profile_text(), None);
}

#[test]
fn the_settings_come_back_after_a_restart_of_the_engine() {
    let mut rig = Rig::new();
    set(&mut rig, custom());
    rig.poll_for(SAVE_DELAY);
    let mut rig = rig.restart();
    let events = rig.take_events();
    assert_eq!(events.last(), Some(&Event::DeviceSettings(custom())));
    assert!(problems(&events).is_empty());
    let device = rig.connect();
    assert_start(&raw_writes(&device), 42, &after_start(&custom()));
}

#[test]
fn the_stop_writes_the_settings_before_the_stop_sequence() {
    let mut rig = Rig::new();
    let device = rig.connect();
    raw_writes(&device);
    let at_write = Rc::new(RefCell::new(Vec::new()));
    let path = settings_path(rig.dir.path());
    let seen = Rc::clone(&at_write);
    *rig.world.before_write.borrow_mut() = Some(Box::new(move || {
        seen.borrow_mut().push(fs::read_to_string(&path).ok());
    }));

    // The change comes just before the stop: the delay is not over.
    let (commands, receiver) = mpsc::channel();
    commands.send(Command::SetDeviceSettings(custom())).unwrap();
    commands.send(Command::Stop).unwrap();
    runtime::run(&mut rig.engine, &receiver);

    let writes = raw_writes(&device);
    assert_eq!(&writes[writes.len() - 2..], stop_sequence().as_slice());
    let seen = at_write.borrow();
    let at_dc = seen[seen.len() - 2]
        .as_deref()
        .expect("the file exists at DC");
    assert!(at_dc.contains("brightness = 42"), "{at_dc}");
}

#[test]
fn a_file_with_an_error_gives_the_defaults_a_problem_and_no_write_until_a_change() {
    for text in [
        "this is not TOML [",
        "schema_version = 1\n[n4-pro]\nbrightness = \"high\"\n",
        "schema_version = 7\n[n4-pro]\nbrightness = 50\n",
    ] {
        let mut rig = Rig::in_dir(dir_with_settings(text));
        let events = rig.take_events();
        assert_eq!(problems(&events).len(), 1, "{text}");
        assert!(
            problems(&events)[0].contains(SETTINGS_FILE),
            "{:?}",
            problems(&events)
        );
        assert_eq!(settings_events(&events), [n4_defaults()]);
        let device = rig.connect();
        assert_start(&raw_writes(&device), 80, &after_start_defaults());

        // A change of the profile, and the stop, do not change the file.
        rig.set_color(KEY_3, RED);
        rig.poll_for(Duration::from_secs(2));
        rig.engine.finish();
        assert_eq!(file_text(&rig).as_deref(), Some(text));

        // A change of a setting replaces the file.
        let mut rig = rig.restart();
        rig.take_events();
        change(&mut rig, |s| s.brightness = 33);
        rig.poll_for(SAVE_DELAY);
        let written = file_text(&rig).unwrap();
        assert!(written.contains("brightness = 33"), "{written}");
        let rig = rig.restart();
        assert!(problems(&rig.events.borrow()).is_empty());
    }
}

fn after_start_defaults() -> Vec<Vec<u8>> {
    reports(&[
        protocol::set_led_colors(&[[0, 0, 0]; 4]),
        protocol::set_led_brightness(100),
        protocol::set_vibration(true),
    ])
}

#[test]
fn a_value_out_of_range_in_the_file_goes_into_the_range() {
    let rig = Rig::in_dir(dir_with_settings(
        "schema_version = 1\n[n4-pro]\nbrightness = 2\n[n4-pro.knob_lights]\nbrightness = 400\n\
         mode = \"colors\"\ncolors = [\"#00FF00\"]\n",
    ));
    let settings = rig.engine.device_settings();
    assert_eq!(settings.brightness, 15);
    assert!(settings.haptic, "a missing value gives the default");
    assert_eq!(settings.knob_lights.brightness, 100);
    assert_eq!(settings.knob_lights.colors, [GREEN, WHITE, WHITE, WHITE]);
    assert!(problems(&rig.events.borrow()).is_empty());
}

#[test]
fn a_failed_write_of_the_settings_gives_one_problem_and_a_new_try() {
    let mut rig = Rig::new();
    rig.take_events();
    // A directory in place of the file makes each write fail.
    fs::create_dir_all(settings_path(rig.dir.path())).unwrap();
    change(&mut rig, |s| s.brightness = 33);
    rig.poll_for(SAVE_DELAY * 3);
    let events = rig.take_events();
    assert_eq!(problems(&events).len(), 1, "{events:?}");
    fs::remove_dir(settings_path(rig.dir.path())).unwrap();
    rig.poll_for(SAVE_DELAY * 2);
    assert!(file_text(&rig).unwrap().contains("brightness = 33"));
}

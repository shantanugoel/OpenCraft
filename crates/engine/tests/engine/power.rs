use super::*;
use opencraft_core::protocol;

fn take_writes(device: &Rc<RefCell<FakeTransport>>) -> Vec<Vec<u8>> {
    raw_writes(device)
}
use opencraft_engine::{DisplayState, PowerNotice};

#[test]
fn a_handler_dark_request_consumes_the_whole_wake_gesture() {
    use opencraft_engine::{EngineRequest, Message};
    let mut rig = Rig::new();
    let device = rig.connect();
    take_writes(&device);
    device.borrow_mut().push_input(control_report(3, 1));
    rig.engine.poll(Duration::ZERO);
    rig.take_events();
    rig.engine
        .handler_message(Message::Engine(EngineRequest::DeviceDisplay(
            DisplayState::ScreenOff,
        )));
    assert!(has(&take_writes(&device), protocol::sleep()));
    device.borrow_mut().push_input(control_report(3, 0));
    rig.poll_for(Duration::from_millis(40));
    assert_eq!(rig.engine.display_state(), DisplayState::ScreenOff);
    device.borrow_mut().push_input(control_report(3, 1));
    device.borrow_mut().push_input(control_report(3, 0));
    rig.poll_for(Duration::from_millis(100));
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    assert!(!rig
        .take_events()
        .iter()
        .any(|e| matches!(e, Event::Input(_))));
    assert!(has(&take_writes(&device), protocol::init()));
    device.borrow_mut().push_input(control_report(3, 1));
    rig.poll_for(Duration::from_millis(40));
    assert!(rig
        .take_events()
        .iter()
        .any(|e| matches!(e, Event::Input(_))));
    rig.engine
        .handler_message(Message::Engine(EngineRequest::DeviceDisplay(
            DisplayState::DeviceSleep,
        )));
    assert!(has(&take_writes(&device), protocol::disconnected()));
    rig.engine.command(Command::WakeDevice);
    rig.poll_for(Duration::from_millis(200));
    assert!(has(&take_writes(&device), protocol::init()));
}

#[test]
fn brightness_requests_fit_the_model_publish_settings_and_stay_dark() {
    use opencraft_engine::{BrightnessChange, EngineRequest, Message};
    let mut rig = Rig::new();
    let device = rig.connect();
    take_writes(&device);
    rig.take_events();
    for (request, expected) in [
        (BrightnessChange::Set(40), 40),
        (BrightnessChange::Step(5), 45),
        (BrightnessChange::Step(i16::MIN), 15),
        (BrightnessChange::Step(i16::MAX), 100),
    ] {
        rig.engine
            .handler_message(Message::Engine(EngineRequest::DeviceBrightness(request)));
        assert_eq!(rig.engine.device_settings().brightness, expected);
        assert!(has(
            &take_writes(&device),
            protocol::set_brightness(expected)
        ));
        assert!(rig
            .take_events()
            .iter()
            .any(|e| matches!(e, Event::DeviceSettings(s) if s.brightness == expected)));
    }
    notice(&mut rig, PowerNotice::Locked);
    take_writes(&device);
    rig.engine
        .handler_message(Message::Engine(EngineRequest::DeviceBrightness(
            BrightnessChange::Set(35),
        )));
    assert!(take_writes(&device).is_empty());
    rig.poll_for(SAVE_DELAY);
    assert!(
        fs::read_to_string(opencraft_engine::device_settings::settings_path(
            rig.dir.path()
        ))
        .unwrap()
        .contains("brightness = 35")
    );
    notice(&mut rig, PowerNotice::Unlocked);
    assert!(has(&take_writes(&device), protocol::set_brightness(35)));
}

#[test]
fn the_idle_deadline_uses_device_input_and_wake_restarts_it() {
    let mut rig = Rig::new();
    let device = rig.connect();
    take_writes(&device);
    let mut settings = rig.engine.device_settings().clone();
    assert_eq!(settings.screen_off_minutes, 0);
    settings.screen_off_minutes = 1;
    rig.engine.command(Command::SetDeviceSettings(settings));
    rig.poll_for(Duration::from_secs(40));
    device.borrow_mut().push_input(control_report(3, 1));
    rig.engine.poll(Duration::ZERO);
    rig.poll_for(Duration::from_secs(59));
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    rig.engine.poll(Duration::from_secs(10));
    assert_eq!(rig.engine.display_state(), DisplayState::ScreenOff);
    assert!(has(&take_writes(&device), protocol::sleep()));
    rig.poll_for(Duration::from_secs(65));
    assert!(
        !has(&take_writes(&device), protocol::sleep()),
        "one transition only"
    );
    rig.engine.command(Command::WakeDevice);
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    rig.poll_for(Duration::from_secs(59));
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    rig.poll_for(Duration::from_secs(2));
    assert_eq!(rig.engine.display_state(), DisplayState::ScreenOff);
}

#[test]
fn never_cancels_the_deadline_and_reconnection_does_not_cancel_dark() {
    let mut rig = Rig::new();
    let device = rig.connect();
    let mut settings = rig.engine.device_settings().clone();
    settings.screen_off_minutes = 1;
    rig.engine
        .command(Command::SetDeviceSettings(settings.clone()));
    rig.poll_for(Duration::from_secs(59));
    settings.screen_off_minutes = 0;
    rig.engine
        .command(Command::SetDeviceSettings(settings.clone()));
    take_writes(&device);
    rig.poll_for(Duration::from_secs(180));
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    assert!(!has(&take_writes(&device), protocol::sleep()));
    settings.screen_off_minutes = 1;
    rig.engine.command(Command::SetDeviceSettings(settings));
    rig.poll_for(Duration::from_secs(61));
    device.borrow_mut().disconnect();
    let second = rig.plug();
    rig.poll_for(Duration::from_secs(2));
    assert_eq!(rig.engine.display_state(), DisplayState::ScreenOff);
    let writes = take_writes(&second);
    assert!(!has(&writes, protocol::init()));
    assert!(has(&writes, protocol::sleep()));
}

#[test]
fn a_release_report_with_no_prior_press_can_wake_a_dark_device() {
    use opencraft_engine::{EngineRequest, Message};
    let mut rig = Rig::new();
    let device = rig.connect();
    rig.engine
        .handler_message(Message::Engine(EngineRequest::DeviceDisplay(
            DisplayState::DeviceSleep,
        )));
    device.borrow_mut().push_input(control_report(3, 0));
    rig.poll_for(Duration::from_millis(200));
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
}

fn notice(rig: &mut Rig, notice: PowerNotice) {
    assert!(rig.engine.command(Command::PowerNotice(notice)));
}
fn has(writes: &[Vec<u8>], command: Vec<u8>) -> bool {
    writes.iter().any(|w| w.starts_with(&command))
}

#[test]
fn system_sleep_stops_the_timer_and_wake_draws_all_after_the_dc_pause() {
    let mut rig = Rig::new();
    let device = rig.connect();
    take_writes(&device);
    notice(&mut rig, PowerNotice::WillSleep);
    let writes = take_writes(&device);
    assert!(has(&writes, protocol::disconnected()));
    assert!(has(&writes, protocol::sleep()));
    rig.poll_for(Duration::from_secs(31));
    assert!(take_writes(&device).is_empty());
    notice(&mut rig, PowerNotice::DidWake);
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    assert!(has(&take_writes(&device), protocol::init()));
    notice(&mut rig, PowerNotice::WillSleep);
    take_writes(&device);
    notice(&mut rig, PowerNotice::DidWake);
    assert!(
        take_writes(&device).is_empty(),
        "no start directly after DC"
    );
    rig.poll_for(Duration::from_millis(200));
    assert!(has(&take_writes(&device), protocol::init()));
}

#[test]
fn lock_suppresses_input_and_display_wake_until_unlock() {
    let mut rig = Rig::new();
    let device = rig.connect();
    take_writes(&device);
    rig.take_events();
    notice(&mut rig, PowerNotice::Locked);
    assert!(has(&take_writes(&device), protocol::sleep()));
    for cause in [PowerNotice::DisplaysWake, PowerNotice::DidWake] {
        notice(&mut rig, cause);
        assert!(take_writes(&device).is_empty());
    }
    rig.engine.command(Command::WakeDevice);
    device.borrow_mut().push_input(control_report(3, 1));
    rig.poll_for(Duration::from_secs(11));
    let writes = take_writes(&device);
    assert!(has(&writes, protocol::keep_alive()));
    assert!(!has(&writes, protocol::init()));
    assert!(!rig
        .take_events()
        .iter()
        .any(|e| matches!(e, Event::Input(_))));
    notice(&mut rig, PowerNotice::Unlocked);
    assert!(has(&take_writes(&device), protocol::init()));
}

#[test]
fn dark_reconnection_and_setting_edits_do_not_light_the_device() {
    let mut rig = Rig::new();
    notice(&mut rig, PowerNotice::Locked);
    let device = rig.connect();
    let writes = take_writes(&device);
    assert!(has(&writes, protocol::sleep()));
    assert!(!has(&writes, protocol::init()));
    let mut settings = rig.engine.device_settings().clone();
    settings.brightness = 30;
    rig.engine.command(Command::SetDeviceSettings(settings));
    rig.set_color(SurfaceId::new(SurfaceKind::KeyFace, 1), Color::BLACK);
    assert!(take_writes(&device).is_empty());
    notice(&mut rig, PowerNotice::Unlocked);
    assert!(has(&take_writes(&device), protocol::set_brightness(30)));
}

#[test]
fn a_clock_wake_leaves_the_device_dark_while_the_displays_sleep() {
    let mut rig = Rig::new();
    let device = rig.connect();
    take_writes(&device);
    notice(&mut rig, PowerNotice::DisplaysSleep);
    assert_eq!(rig.engine.display_state(), DisplayState::ScreenOff);
    assert!(has(&take_writes(&device), protocol::sleep()));
    // A maintenance wake of the computer moves the clocks, but the
    // displays stay dark.
    rig.sleep_computer(Duration::from_secs(60));
    rig.poll_for(Duration::from_millis(200));
    assert_eq!(rig.engine.display_state(), DisplayState::ScreenOff);
    assert!(!has(&take_writes(&device), protocol::init()));
    notice(&mut rig, PowerNotice::DisplaysWake);
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    assert!(has(&take_writes(&device), protocol::init()));
}

#[test]
fn a_clock_wake_draws_an_awake_device_again() {
    let mut rig = Rig::new();
    let device = rig.connect();
    take_writes(&device);
    rig.sleep_computer(Duration::from_secs(60));
    rig.poll_for(Duration::from_millis(200));
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    assert!(has(&take_writes(&device), protocol::init()));
}

#[test]
fn a_wake_request_for_an_awake_device_writes_nothing() {
    let mut rig = Rig::new();
    let device = rig.connect();
    take_writes(&device);
    rig.take_events();
    rig.engine.command(Command::WakeDevice);
    rig.engine.poll(Duration::ZERO);
    assert!(take_writes(&device).is_empty());
    assert!(!rig
        .take_events()
        .iter()
        .any(|e| matches!(e, Event::Disconnected)));
}

#[test]
fn a_device_input_wakes_the_device_while_the_displays_sleep() {
    let mut rig = Rig::new();
    let device = rig.connect();
    notice(&mut rig, PowerNotice::DisplaysSleep);
    take_writes(&device);
    device.borrow_mut().push_input(control_report(3, 1));
    device.borrow_mut().push_input(control_report(3, 0));
    rig.poll_for(Duration::from_millis(100));
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    assert!(has(&take_writes(&device), protocol::init()));
    assert!(
        !rig.take_events()
            .iter()
            .any(|e| matches!(e, Event::Input(_))),
        "the wake gesture starts no action"
    );
}

#[test]
fn a_device_that_connects_late_starts_the_idle_time_again() {
    let mut rig = Rig::new();
    let device = rig.connect();
    let mut settings = rig.engine.device_settings().clone();
    settings.screen_off_minutes = 1;
    rig.engine.command(Command::SetDeviceSettings(settings));
    device.borrow_mut().disconnect();
    rig.poll_for(Duration::from_secs(50));
    let second = rig.plug();
    rig.poll_for(Duration::from_secs(30));
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    assert!(has(&take_writes(&second), protocol::init()));
    rig.poll_for(Duration::from_secs(40));
    assert_eq!(rig.engine.display_state(), DisplayState::ScreenOff);
}

use super::*;
use opencraft_core::protocol;

fn take_writes(device: &Rc<RefCell<FakeTransport>>) -> Vec<Vec<u8>> {
    raw_writes(device)
}
use opencraft_engine::{DisplayState, PowerNotice};

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
fn display_sleep_and_clock_wake_use_the_same_policy() {
    let mut rig = Rig::new();
    let device = rig.connect();
    take_writes(&device);
    notice(&mut rig, PowerNotice::DisplaysSleep);
    assert_eq!(rig.engine.display_state(), DisplayState::ScreenOff);
    assert!(has(&take_writes(&device), protocol::sleep()));
    rig.sleep_computer(Duration::from_secs(60));
    rig.poll_for(Duration::from_millis(200));
    assert_eq!(rig.engine.display_state(), DisplayState::Awake);
    assert!(has(&take_writes(&device), protocol::init()));
}

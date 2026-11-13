//! Do one piece of work of [`opencraft_actions::system`], for a test by hand.
//!
//! ```sh
//! cargo run -p opencraft-actions --example system -- volume
//! ```

use opencraft_actions::system::{self, MediaKey};
use std::env;

/// The system reads a key event after the call that sends it. A program that
/// stops at once loses the event. See `docs/kb/macos-permissions.md`.
fn wait() {
    std::thread::sleep(std::time::Duration::from_millis(300));
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["can-send-keys"] => {
            println!("{}", system::can_send_keys());
            Ok(())
        }
        ["ask"] => {
            system::ask_to_send_keys();
            Ok(())
        }
        ["volume"] => system::volume().map(|v| println!("{v:?}")),
        ["set-volume", level] => system::set_volume_level(level.parse().expect("a number")),
        ["mute", state] => system::set_muted(*state == "on"),
        ["media", key] => system::media_key(match *key {
            "play" => MediaKey::PlayPause,
            "next" => MediaKey::Next,
            "previous" => MediaKey::Previous,
            "fast" => MediaKey::FastForward,
            "rewind" => MediaKey::Rewind,
            "up" => MediaKey::VolumeUp,
            "down" => MediaKey::VolumeDown,
            "mute" => MediaKey::Mute,
            _ => panic!("the media key {key} is not known"),
        })
        .map(|()| wait()),
        ["hotkey", text] => system::send_hotkey(&text.parse().expect("a hotkey")).map(|()| wait()),
        ["mac-function", name] => system::mac_function(
            opencraft_actions::mac_function::MacFunction::from_name(name).expect("a Mac function"),
        )
        .map(|()| wait()),
        ["mouse-click", button] => {
            system::mouse_event(opencraft_actions::mouse::MouseEvent::Click {
                button: match *button {
                    "left" => opencraft_actions::mouse::Button::Left,
                    "right" => opencraft_actions::mouse::Button::Right,
                    "middle" => opencraft_actions::mouse::Button::Middle,
                    _ => panic!("unknown mouse button"),
                },
                count: 1,
            })
            .map(|()| wait())
        }
        ["mouse-scroll", x, y] => {
            system::mouse_event(opencraft_actions::mouse::MouseEvent::Scroll {
                x: x.parse().expect("a number"),
                y: y.parse().expect("a number"),
            })
            .map(|()| wait())
        }
        ["type", text] => system::type_text(text).map(|()| wait()),
        ["paste", text] => system::paste_text(text).map(|()| wait()),
        ["app", name] => system::open_application(name, &[]),
        ["file", path] => system::open_file(path),
        ["url", url] => system::open_url(url, None),
        ["url-in", app, url] => system::open_url(url, Some(app)),
        ["shell", command] => system::start_shell_command(command).map(|mut child| {
            println!("{:?}", child.wait());
        }),
        _ => {
            eprintln!(
                "usage: system can-send-keys | ask | volume | set-volume N | mute on|off | \
                 media play|next|previous|up|down|mute | hotkey TEXT | type TEXT | paste TEXT | app NAME | file PATH | \
                 url URL | url-in APP URL | shell COMMAND | mouse-click left|right|middle | mouse-scroll X Y | mac-function NAME"
            );
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

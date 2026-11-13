//! The built-in handlers use the same ports as any other handler.

mod support;

use opencraft_actions::handlers_with_system;
use opencraft_engine::{runtime::start_handlers, Feedback, Message, Request, Status};
use std::sync::Arc;
use std::time::Duration;
use support::*;

#[test]
fn all_built_in_actions_use_only_the_handler_ports() {
    let system = Arc::new(Fake::default());
    system.finish_shell(Exit::Code(9));
    let handlers = handlers_with_system(system.clone());
    let input_kinds = handlers
        .iter()
        .flat_map(|h| h.kinds())
        .filter(|k| k.needs_accessibility)
        .count();
    let (ports, messages) = start_handlers(handlers).unwrap();
    let receive = || messages.recv_timeout(Duration::from_millis(500)).unwrap();
    for _ in 0..input_kinds {
        assert!(matches!(
            receive(),
            Message::Status {
                status: Status::Ready,
                ..
            }
        ));
    }
    let send = |kind: &str, request| {
        let kind = format!("opencraft.{kind}");
        ports
            .iter()
            .find(|p| p.kinds.iter().any(|d| d.kind == kind))
            .unwrap()
            .sender
            .send(request)
            .unwrap();
    };
    for (kind, key, value, expected) in [
        (
            "open-application",
            "application",
            "Test",
            Call::Application("Test".into()),
        ),
        (
            "open-url",
            "url",
            "https://example.test",
            Call::Url("https://example.test".into()),
        ),
        ("hotkey", "hotkey", "cmd+a", Call::Hotkey("cmd+a".into())),
        (
            "mouse",
            "operation",
            "scroll-up",
            Call::Mouse(opencraft_actions::mouse::MouseEvent::Scroll { x: 0, y: 3 }),
        ),
        ("type-text", "text", "Test", Call::Text("Test".into())),
        (
            "media-key",
            "key",
            "next",
            Call::Media(opencraft_actions::system::MediaKey::Next),
        ),
        (
            "shell-command",
            "command",
            "fake command",
            Call::Shell("fake command".into()),
        ),
    ] {
        send(kind, invoke(text_action(kind, key, value)));
        wait_until(|| system.calls().contains(&expected));
    }
    assert!(matches!(receive(), Message::Problem(text) if text.contains("code 9")));
    send(
        "volume",
        Request::Appear {
            binding: binding(1),
            action: action("volume", &[]),
        },
    );
    assert_eq!(
        receive(),
        Message::Feedback {
            binding: binding(1),
            feedback: Some(Feedback {
                label: None,
                level: Some(50),
                state: None,
                other_state: None,
            }),
        }
    );
    send("volume", invoke(action("volume", &[])));
    assert_eq!(
        receive(),
        Message::Feedback {
            binding: binding(1),
            feedback: Some(Feedback {
                label: None,
                level: Some(55),
                state: None,
                other_state: None,
            }),
        }
    );
    assert!(system.calls().contains(&Call::Level(55)));
    // The same port reports invalid settings and resolves a kind status.
    send("open-url", invoke(action("open-url", &[])));
    assert!(matches!(receive(), Message::Problem(text) if text.contains("\"URL\"")));
    system
        .permission
        .store(false, std::sync::atomic::Ordering::SeqCst);
    send("hotkey", invoke(text_action("hotkey", "hotkey", "cmd+a")));
    for _ in 0..input_kinds {
        assert!(matches!(
            receive(),
            Message::Status {
                status: Status::NeedsUser { .. },
                ..
            }
        ));
    }
    assert!(
        matches!(receive(), Message::Problem(text) if text.contains("Accessibility permission"))
    );
    system
        .permission
        .store(true, std::sync::atomic::Ordering::SeqCst);
    send(
        "hotkey",
        Request::Resolve {
            kind: "opencraft.hotkey".into(),
        },
    );
    for _ in 0..input_kinds {
        assert!(matches!(
            receive(),
            Message::Status {
                status: Status::Ready,
                ..
            }
        ));
    }
    assert_eq!(
        system
            .calls()
            .iter()
            .filter(|c| matches!(c, Call::Hotkey(_)))
            .count(),
        1
    );
    // Dropping the ports drops the handlers, including the active volume helper.
    drop(ports);
    assert_eq!(
        messages.recv_timeout(Duration::from_millis(500)),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
    );
}

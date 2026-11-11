//! The OpenCraft shell. Only the engine opens the device and writes files.

mod images;
mod state;
mod view;

use gpui_kit::{
    prelude::*, px, size, Bounds, QuitMode, TitlebarOptions, WindowBounds, WindowOptions,
};
use opencraft_engine::{Command, Config, EngineHandle, Event};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

/// Start the device before the GUI toolkit. The sink never waits for the GUI.
fn start_engine() -> (EngineHandle, async_channel::Receiver<Event>) {
    let (sender, receiver) = async_channel::unbounded();
    let profile_dir = std::env::var_os("OPENCRAFT_PROFILE_DIR")
        .map(PathBuf::from)
        .or_else(opencraft_engine::default_profile_dir)
        .expect("OpenCraft cannot find the profile directory.");
    let trace = start_trace();
    let engine = opencraft_engine::spawn(Config { profile_dir }, move |event| {
        if trace && matches!(event, Event::Connected) {
            trace_now("device ready");
        }
        let _ = sender.try_send(event);
    })
    .expect("OpenCraft cannot start the engine.");
    (engine, receiver)
}

/// True if the environment variable `OPENCRAFT_START_TRACE` is set. Then the
/// application prints the time of each step of its start, for a measurement
/// of the cold start. See `docs/kb/footprint.md`.
fn start_trace() -> bool {
    std::env::var_os("OPENCRAFT_START_TRACE").is_some()
}

/// Print `step` with the time of the system clock, in milliseconds.
fn trace_now(step: &str) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    eprintln!("start-trace {step} {:.3}", now.as_secs_f64() * 1000.0);
}

fn main() {
    if start_trace() {
        trace_now("main");
    }
    let (engine, events) = start_engine();
    // Keep ownership outside the view. Close and quit hooks stop the engine;
    // code after the run loop is a fallback on platforms where it returns.
    let engine = Rc::new(RefCell::new(Some(engine)));
    let commands = engine.clone();
    let shutdown = engine.clone();
    gpui_kit::platform::application()
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(move |cx| {
            gpui_kit::init(cx);
            // macOS terminates inside its native run loop. Do not rely only on
            // code after run(). GPUI awaits this task during graceful quit.
            cx.on_app_quit(move |cx| {
                let engine = shutdown.borrow_mut().take();
                cx.background_executor().spawn(async move {
                    if let Some(engine) = engine {
                        engine.stop();
                    }
                })
            })
            .detach();
            gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);
            let closing = commands.clone();
            let send: Rc<dyn Fn(Command)> = Rc::new(move |command| {
                if let Some(engine) = commands.borrow().as_ref() {
                    engine.send(command);
                }
            });
            let bounds = Bounds::centered(None, size(px(1120.), px(760.)), cx);
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("OpenCraft".into()),
                        ..Default::default()
                    }),
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(900.), px(640.))),
                    ..Default::default()
                },
                move |window, cx| {
                    if start_trace() {
                        window.on_next_frame(|_, _| trace_now("first frame"));
                    }
                    // Finish the engine before closing the last window. This path
                    // has no shutdown time limit and does not block the UI.
                    window.on_window_should_close(cx, move |window, cx| {
                        if let Some(engine) = closing.borrow_mut().take() {
                            window
                                .spawn(cx, async move |cx| {
                                    cx.background_executor()
                                        .spawn(async move {
                                            engine.stop();
                                        })
                                        .await;
                                    let _ = cx.update(|window, cx| {
                                        window.remove_window();
                                        cx.quit();
                                    });
                                })
                                .detach();
                        }
                        false
                    });
                    let view = cx.new(|cx| view::DeviceView::new(events, send, window, cx));
                    cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
                },
            )
            .expect("OpenCraft cannot open the window.");
            cx.activate(true);
        });
    if let Some(engine) = engine.borrow_mut().take() {
        engine.stop();
    };
}

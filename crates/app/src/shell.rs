//! The shell of the process: the state with no window, the shell commands,
//! the window, and the Dock icon (decision 0020).
//!
//! One [`Shell`] entity exists for the process. It gets the engine events
//! also when no window is open. A new window starts from its state, and then
//! gets each event from it. The menu bar item shows the same state.

use crate::{
    menu::MenuState,
    settings::{self, Settings},
    state::ViewState,
    view::DeviceView,
};
use gpui_kit::{
    prelude::*, px, size, ActivationPolicy, App, Bounds, Context, EventEmitter, TitlebarOptions,
    WindowBounds, WindowOptions,
};
use opencraft_engine::{Command, Event};
use std::{path::PathBuf, rc::Rc, time::Duration, time::Instant};

/// The time from the change to the regular activation policy to the
/// activation of the application. See `docs/kb/gpui.md`, "Windows, Dock icon,
/// and activation".
const ACTIVATE_DELAY: Duration = Duration::from_millis(200);

/// A command to the shell, from the menu bar item, from a click on the Dock
/// icon, or from a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand {
    /// Open the window, or bring it to the front.
    Open,
    /// Show the Dock icon if it is hidden, and hide it if it shows.
    ToggleDockIcon,
    /// Start the quit path.
    Quit,
}

pub struct Shell {
    /// The latest engine state. The selection fields stay empty here: each
    /// window has its own selection.
    pub state: ViewState,
    settings: Settings,
    /// The directory of `shell.toml`.
    config_dir: PathBuf,
    /// Sends a command to the engine.
    send: Rc<dyn Fn(Command)>,
    #[cfg(target_os = "macos")]
    menu_bar: Option<crate::menu_bar::MenuBar>,
}

/// The shell sends each engine event to the open window after it applies the
/// event to its own state.
impl EventEmitter<Event> for Shell {}

impl Shell {
    pub fn new(config_dir: PathBuf, settings: Settings, send: Rc<dyn Fn(Command)>) -> Self {
        Self {
            state: ViewState::default(),
            settings,
            config_dir,
            send,
            #[cfg(target_os = "macos")]
            menu_bar: None,
        }
    }

    /// Start the two tasks that receive the engine events and the shell
    /// commands. The tasks stop when the entity goes away.
    pub fn listen(
        &mut self,
        events: async_channel::Receiver<Event>,
        commands: async_channel::Receiver<ShellCommand>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |shell, cx| {
            while let Ok(event) = events.recv().await {
                if shell
                    .update(cx, |shell, cx| shell.receive(event, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        cx.spawn(async move |shell, cx| {
            while let Ok(command) = commands.recv().await {
                if shell
                    .update(cx, |shell, cx| shell.command(command, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Make the menu bar item. Call this on the main thread, in the callback
    /// of `Application::run`.
    #[cfg(target_os = "macos")]
    pub fn add_menu_bar(&mut self, commands: async_channel::Sender<ShellCommand>) {
        self.menu_bar = crate::menu_bar::MenuBar::new(commands);
        if self.menu_bar.is_none() {
            eprintln!("OpenCraft cannot make the menu bar item.");
        }
        self.refresh_menu();
    }

    pub fn receive(&mut self, event: Event, cx: &mut Context<Self>) {
        self.state.apply(event.clone(), Instant::now());
        cx.emit(event);
        self.refresh_menu();
        cx.notify();
    }

    /// Show a problem in the window, also in a window that opens later.
    pub fn problem(&mut self, text: String, cx: &mut Context<Self>) {
        eprintln!("{text}");
        self.receive(Event::Problem(text), cx);
    }

    /// The user closed a problem in the window. A new window does not show it.
    pub fn dismiss_problem(&mut self, text: &str) {
        if let Some(index) = self.state.problems.iter().position(|p| p == text) {
            self.state.problems.remove(index);
        }
    }

    pub fn command(&mut self, command: ShellCommand, cx: &mut Context<Self>) {
        match command {
            ShellCommand::Open => self.open_window(cx),
            ShellCommand::ToggleDockIcon => {
                self.set_show_dock_icon(!self.settings.show_dock_icon, cx)
            }
            ShellCommand::Quit => cx.quit(),
        }
    }

    /// The state for a new window. The short marks of old input do not show.
    pub fn window_state(&self) -> ViewState {
        let mut state = self.state.clone();
        state.highlights.clear();
        state
    }

    // Only the macOS menu bar item and the tests read the menu state.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub fn menu_state(&self) -> MenuState {
        let state = &self.state;
        MenuState {
            connected: state.connected,
            profile: state.profile.as_ref().and_then(|id| {
                state
                    .profiles
                    .iter()
                    .find(|p| &p.id == id)
                    .map(|p| p.name.clone())
            }),
            show_dock_icon: self.settings.show_dock_icon,
        }
    }

    fn refresh_menu(&mut self) {
        #[cfg(target_os = "macos")]
        if self.menu_bar.is_some() {
            let state = self.menu_state();
            if let Some(menu_bar) = &mut self.menu_bar {
                menu_bar.show(&state);
            }
        }
    }

    /// Open the window. If a window is open, bring it and the application to
    /// the front.
    pub fn open_window(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = cx.windows().into_iter().next() {
            let _ = window.update(cx, |_, window, _| window.activate_window());
            cx.activate(true);
            return;
        }
        let shell = cx.entity();
        let state = self.window_state();
        let send = self.send.clone();
        let bounds = Bounds::centered(None, size(px(1120.), px(760.)), cx);
        let opened = cx.open_window(
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
                if crate::start_trace() {
                    window.on_next_frame(|_, _| crate::trace_now("first frame"));
                }
                let view = cx.new(|cx| DeviceView::new(&shell, state, send, window, cx));
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            },
        );
        if let Err(error) = opened {
            eprintln!("OpenCraft cannot open the window: {error}");
            return;
        }
        cx.activate(true);
    }

    fn set_show_dock_icon(&mut self, show: bool, cx: &mut Context<Self>) {
        self.settings.show_dock_icon = show;
        apply_dock_icon(show, cx);
        if let Err(error) = settings::save(&self.config_dir, &self.settings) {
            self.problem(
                format!(
                    "OpenCraft cannot save the Dock icon setting in {}: {error}",
                    self.config_dir.join(settings::FILE_NAME).display()
                ),
                cx,
            );
        }
        self.refresh_menu();
        cx.notify();
    }
}

/// Show or hide the Dock icon at run time. An open window stays open.
fn apply_dock_icon(show: bool, cx: &mut App) {
    if !show {
        cx.set_activation_policy(ActivationPolicy::Accessory);
        return;
    }
    // If the application is active at the change, the menu bar stays with a
    // different application. The Dock application takes the activation
    // first, and the application takes it back later.
    #[cfg(target_os = "macos")]
    let active = crate::menu_bar::activate_dock_if_active();
    #[cfg(not(target_os = "macos"))]
    let active = false;
    cx.set_activation_policy(ActivationPolicy::Regular);
    if active {
        cx.spawn(async move |cx| {
            cx.background_executor().timer(ACTIVATE_DELAY).await;
            cx.update(|cx| cx.activate(true));
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::tests::{key, layout};
    use gpui_kit::{Entity, TestAppContext};
    use opencraft_engine::{ProfileEntry, RgbImage, SurfaceContent, SurfaceState};
    use std::{cell::RefCell, sync::Arc};

    struct Fixture {
        shell: Entity<Shell>,
        events: async_channel::Sender<Event>,
        commands: async_channel::Sender<ShellCommand>,
        sent: Rc<RefCell<Vec<Command>>>,
        dir: tempfile::TempDir,
    }

    fn shell(cx: &mut TestAppContext) -> Fixture {
        cx.update(|cx| {
            gpui_kit::init(cx);
            gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Dark, None, cx);
        });
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().to_path_buf();
        let (events, event_receiver) = async_channel::unbounded();
        let (commands, command_receiver) = async_channel::unbounded();
        let sent: Rc<RefCell<Vec<Command>>> = Default::default();
        let log = sent.clone();
        let shell = cx.update(|cx| {
            cx.new(|cx| {
                let mut shell = Shell::new(
                    config,
                    Settings::default(),
                    Rc::new(move |c| log.borrow_mut().push(c)),
                );
                shell.listen(event_receiver, command_receiver, cx);
                shell
            })
        });
        Fixture {
            shell,
            events,
            commands,
            sent,
            dir,
        }
    }

    fn profiles() -> Event {
        Event::Profiles {
            profiles: vec![ProfileEntry {
                id: "desk".into(),
                name: "Desk".into(),
            }],
            active: "desk".into(),
        }
    }

    fn surface(label: &str) -> Event {
        Event::Surface(SurfaceState {
            id: key(),
            content: SurfaceContent {
                label: Some(label.into()),
                ..Default::default()
            },
            image: Arc::new(RgbImage::new(90, 90)),
        })
    }

    fn view(cx: &mut TestAppContext) -> Option<Entity<DeviceView>> {
        cx.update(|cx| {
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
    }

    #[gpui_kit::test]
    fn the_state_and_the_menu_follow_the_engine_with_no_window(cx: &mut TestAppContext) {
        let f = shell(cx);
        let menu = |cx: &mut TestAppContext| cx.update(|cx| f.shell.read(cx).menu_state());
        assert_eq!(
            menu(cx),
            MenuState {
                connected: false,
                profile: None,
                show_dock_icon: true
            }
        );
        f.events.try_send(layout()).unwrap();
        f.events.try_send(profiles()).unwrap();
        f.events.try_send(Event::Connected).unwrap();
        cx.run_until_parked();
        assert!(cx.update(|cx| cx.windows().is_empty()));
        assert_eq!(
            menu(cx),
            MenuState {
                connected: true,
                profile: Some("Desk".into()),
                show_dock_icon: true
            }
        );
        f.events.try_send(Event::Disconnected).unwrap();
        cx.run_until_parked();
        assert!(!menu(cx).connected);
    }

    #[gpui_kit::test]
    fn a_new_window_shows_the_current_state_and_gets_later_events(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.events.try_send(layout()).unwrap();
        f.events.try_send(surface("Mail")).unwrap();
        f.events.try_send(Event::Connected).unwrap();
        f.events
            .try_send(Event::Problem("Cannot save.".into()))
            .unwrap();
        cx.run_until_parked();
        f.commands.try_send(ShellCommand::Open).unwrap();
        cx.run_until_parked();
        let view = view(cx).expect("The Open command opens a window.");
        cx.update(|cx| {
            let state = &view.read(cx).state;
            assert!(state.connected);
            assert_eq!(
                state.surfaces[&key()].content.label.as_deref(),
                Some("Mail")
            );
            assert_eq!(state.problems, ["Cannot save."]);
        });
        f.events.try_send(surface("Sound")).unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(
                view.read(cx).state.surfaces[&key()]
                    .content
                    .label
                    .as_deref(),
                Some("Sound")
            )
        });
        // A second Open brings the same window to the front.
        f.commands.try_send(ShellCommand::Open).unwrap();
        cx.run_until_parked();
        assert_eq!(cx.update(|cx| cx.windows().len()), 1);
        assert!(f.sent.borrow().is_empty());
    }

    #[gpui_kit::test]
    fn the_engine_state_continues_after_the_window_closes(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.events.try_send(layout()).unwrap();
        f.commands.try_send(ShellCommand::Open).unwrap();
        cx.run_until_parked();
        let window = cx.update(|cx| cx.windows()[0]);
        cx.update(|cx| {
            window
                .update(cx, |_, window, _| window.remove_window())
                .unwrap()
        });
        cx.run_until_parked();
        assert!(cx.update(|cx| cx.windows().is_empty()));
        f.events.try_send(surface("Later")).unwrap();
        f.events.try_send(Event::Connected).unwrap();
        cx.run_until_parked();
        f.commands.try_send(ShellCommand::Open).unwrap();
        cx.run_until_parked();
        let view = view(cx).unwrap();
        cx.update(|cx| {
            let state = &view.read(cx).state;
            assert!(state.connected);
            assert_eq!(
                state.surfaces[&key()].content.label.as_deref(),
                Some("Later")
            );
        });
    }

    #[gpui_kit::test]
    fn the_dock_icon_option_is_kept_in_shell_toml(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.commands.try_send(ShellCommand::ToggleDockIcon).unwrap();
        cx.run_until_parked();
        assert!(!cx.update(|cx| f.shell.read(cx).menu_state().show_dock_icon));
        assert_eq!(
            settings::load(f.dir.path()),
            (
                Settings {
                    show_dock_icon: false
                },
                None
            )
        );
        f.commands.try_send(ShellCommand::ToggleDockIcon).unwrap();
        cx.run_until_parked();
        assert!(cx.update(|cx| f.shell.read(cx).menu_state().show_dock_icon));
        assert_eq!(settings::load(f.dir.path()), (Settings::default(), None));
    }

    #[gpui_kit::test]
    fn a_closed_problem_does_not_show_in_a_new_window(cx: &mut TestAppContext) {
        let f = shell(cx);
        f.events.try_send(Event::Problem("One".into())).unwrap();
        f.events.try_send(Event::Problem("Two".into())).unwrap();
        cx.run_until_parked();
        f.shell.update(cx, |shell, _| shell.dismiss_problem("One"));
        assert_eq!(
            cx.update(|cx| f.shell.read(cx).window_state().problems),
            ["Two"]
        );
    }
}

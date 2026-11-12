//! The page tabs and the profile menu.
//!
//! The tabs and the menu show the pages and the profiles of the engine, and
//! the clicks send commands only. The window shows the active page of the
//! engine: when an action on the device changes the page, the tabs follow.
//! The commands are in decisions 0016 and 0018.

use super::*;
use gpui_kit::{deferred, KeyDownEvent};
use opencraft_engine::ProfileEntry;

/// A change of a page that needs more input from the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PageEdit {
    Rename(usize),
    Remove(usize),
}

/// A change of a profile that needs more input from the user. The text is
/// the identity of the profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ProfileEdit {
    Add,
    Duplicate(String),
    Rename(String),
    Remove(String),
}

/// The state of the page tabs and the profile menu. Only the engine events
/// change the pages and the profiles.
pub(super) struct Lists {
    pub(super) page_edit: Option<PageEdit>,
    pub(super) page_name: Entity<InputState>,
    pub(super) profiles_open: bool,
    pub(super) profile_edit: Option<ProfileEdit>,
    pub(super) profile_name: Entity<InputState>,
    _subscriptions: [Subscription; 2],
}

impl Lists {
    pub(super) fn new(window: &mut Window, cx: &mut Context<DeviceView>) -> Self {
        let page_name = cx.new(|cx| InputState::new(window, cx).placeholder("Page name"));
        let profile_name = cx.new(|cx| InputState::new(window, cx).placeholder("Profile name"));
        let page_enter = cx.subscribe_in(&page_name, window, |view, _, event, window, cx| {
            if matches!(event, TextEvent::PressEnter { .. }) {
                view.save_page_name(window, cx);
            }
        });
        let profile_enter = cx.subscribe_in(&profile_name, window, |view, _, event, window, cx| {
            if matches!(event, TextEvent::PressEnter { .. }) {
                view.save_profile_name(window, cx);
            }
        });
        Self {
            page_edit: None,
            page_name,
            profiles_open: false,
            profile_edit: None,
            profile_name,
            _subscriptions: [page_enter, profile_enter],
        }
    }
}

/// The name of a page for the user. A page with no name has its number.
fn page_title(names: &[String], page: usize) -> String {
    match names.get(page) {
        Some(name) if !name.trim().is_empty() => name.clone(),
        _ => format!("Page {}", page + 1),
    }
}

impl DeviceView {
    /// Apply the part of an engine event that the tabs and the menu use. A
    /// change of a page that the user started stops when the pages change:
    /// its page number can be a different page now.
    pub(super) fn update_lists(&mut self, event: &Event) {
        match event {
            Event::Pages { .. } => self.lists.page_edit = None,
            // The state has the old active profile still. An edit of the
            // active profile stops when a different profile becomes active.
            Event::Profiles { active, .. } if self.state.profile.as_ref() != Some(active) => {
                self.lists.profile_edit = None;
            }
            Event::Profiles { profiles, .. } => {
                let gone = |id: &String| !profiles.iter().any(|p| &p.id == id);
                if match &self.lists.profile_edit {
                    Some(ProfileEdit::Duplicate(id))
                    | Some(ProfileEdit::Rename(id))
                    | Some(ProfileEdit::Remove(id)) => gone(id),
                    _ => false,
                } {
                    self.lists.profile_edit = None;
                }
            }
            _ => {}
        }
    }

    fn active_profile(&self) -> Option<&ProfileEntry> {
        let id = self.state.profile.as_ref()?;
        self.state.profiles.iter().find(|p| &p.id == id)
    }

    fn edit_name(
        &mut self,
        input: Entity<InputState>,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The input is not in the window yet, so an action cannot reach it.
        input.update(cx, |input, cx| {
            input.set_value(text, window, cx);
            input.select_all(window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    fn start_page_edit(&mut self, edit: PageEdit, window: &mut Window, cx: &mut Context<Self>) {
        self.lists.page_edit = Some(edit);
        if let PageEdit::Rename(page) = edit {
            let name = page_title(&self.state.pages, page);
            self.edit_name(self.lists.page_name.clone(), name, window, cx);
        }
        cx.notify();
    }

    fn save_page_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(PageEdit::Rename(page)) = self.lists.page_edit else {
            return;
        };
        let name = self.lists.page_name.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        if self.state.pages.get(page) != Some(&name) {
            (self.send)(Command::RenamePage { page, name });
        }
        self.lists.page_edit = None;
        window.blur(cx);
        cx.notify();
    }

    fn cancel_page_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.lists.page_edit = None;
        window.blur(cx);
        cx.notify();
    }

    fn start_profile_edit(
        &mut self,
        edit: ProfileEdit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = match &edit {
            ProfileEdit::Add => Some(String::new()),
            ProfileEdit::Duplicate(_) => self.active_profile().map(|p| format!("{} copy", p.name)),
            ProfileEdit::Rename(_) => self.active_profile().map(|p| p.name.clone()),
            ProfileEdit::Remove(_) => None,
        };
        self.lists.profile_edit = Some(edit);
        if let Some(name) = name {
            self.edit_name(self.lists.profile_name.clone(), name, window, cx);
        }
        cx.notify();
    }

    fn save_profile_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.lists.profile_name.read(cx).value().trim().to_string();
        if name.is_empty() {
            return;
        }
        match self.lists.profile_edit.take() {
            Some(ProfileEdit::Add) => {
                (self.send)(Command::AddProfile {
                    name,
                    copy_of: None,
                });
                self.lists.profiles_open = false;
            }
            Some(ProfileEdit::Duplicate(id)) => {
                (self.send)(Command::AddProfile {
                    name,
                    copy_of: Some(id),
                });
                self.lists.profiles_open = false;
            }
            Some(ProfileEdit::Rename(id)) => {
                if self
                    .state
                    .profiles
                    .iter()
                    .any(|p| p.id == id && p.name != name)
                {
                    (self.send)(Command::RenameProfile { profile: id, name });
                }
            }
            other => {
                self.lists.profile_edit = other;
                return;
            }
        }
        window.blur(cx);
        cx.notify();
    }

    fn close_profiles(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.lists.profiles_open = false;
        self.lists.profile_edit = None;
        window.blur(cx);
        cx.notify();
    }

    /// The row of page tabs, and the buttons for the active page.
    pub(super) fn page_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let names = &self.state.pages;
        if names.is_empty() {
            return div().into_any_element();
        }
        let active = self.state.page;
        let count = names.len();
        let mut tabs = div()
            .id("page-tabs")
            .debug_selector(|| "page-tabs".into())
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap_1()
            .overflow_x_scroll();
        for page in 0..count {
            if self.lists.page_edit == Some(PageEdit::Rename(page)) {
                tabs = tabs.child(
                    div()
                        .id("page-name")
                        .debug_selector(|| "page-name".into())
                        .flex_shrink_0()
                        .w(px(180.))
                        .text_color(rgb(TEXT))
                        .capture_key_down(cx.listener(Self::escape_page()))
                        .child(Input::new(&self.lists.page_name)),
                );
                continue;
            }
            let selected = page == active;
            tabs = tabs.child(
                button(format!("page-{page}"), page_title(names, page))
                    .max_w(px(200.))
                    .truncate()
                    .when(selected, |d| {
                        d.border_color(rgb(SELECTED)).bg(rgb(0x2c3442))
                    })
                    .on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                        if page != view.state.page {
                            (view.send)(Command::SelectPage { page });
                        } else if event.click_count() >= 2 {
                            view.start_page_edit(PageEdit::Rename(page), window, cx);
                        }
                    })),
            );
        }
        tabs = tabs.child(
            button("page-add", "+ Page")
                .text_color(rgb(MUTED))
                .on_click(
                    cx.listener(|view, _, _, _| (view.send)(Command::AddPage { copy_of: None })),
                ),
        );
        let tools =
            match self.lists.page_edit {
                Some(PageEdit::Rename(_)) => div()
                    .flex()
                    .gap_1()
                    .child(button("page-name-save", "Save").on_click(
                        cx.listener(|view, _, window, cx| view.save_page_name(window, cx)),
                    ))
                    .child(button("page-name-cancel", "Cancel").on_click(
                        cx.listener(|view, _, window, cx| view.cancel_page_edit(window, cx)),
                    )),
                Some(PageEdit::Remove(page)) => {
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().text_sm().child(format!(
                            "Remove “{}” and all its content?",
                            page_title(names, page)
                        )))
                        .child(danger_button("page-remove-confirm", "Remove").on_click(
                            cx.listener(move |view, _, _, cx| {
                                (view.send)(Command::RemovePage { page });
                                view.lists.page_edit = None;
                                cx.notify();
                            }),
                        ))
                        .child(button("page-remove-cancel", "Cancel").on_click(
                            cx.listener(|view, _, window, cx| view.cancel_page_edit(window, cx)),
                        ))
                }
                None => div()
                    .flex()
                    .gap_1()
                    .child(button("page-rename", "Rename").on_click(cx.listener(
                        move |view, _, window, cx| {
                            view.start_page_edit(PageEdit::Rename(active), window, cx)
                        },
                    )))
                    .child(
                        small_button("page-left", "◀", active > 0)
                            .py_2()
                            .tooltip(|window, cx| {
                                Tooltip::new("Move the page to the left").build(window, cx)
                            })
                            .when(active > 0, |d| {
                                d.on_click(cx.listener(move |view, _, _, _| {
                                    (view.send)(Command::MovePage {
                                        page: active,
                                        to: active - 1,
                                    })
                                }))
                            }),
                    )
                    .child(
                        small_button("page-right", "▶", active + 1 < count)
                            .py_2()
                            .tooltip(|window, cx| {
                                Tooltip::new("Move the page to the right").build(window, cx)
                            })
                            .when(active + 1 < count, |d| {
                                d.on_click(cx.listener(move |view, _, _, _| {
                                    (view.send)(Command::MovePage {
                                        page: active,
                                        to: active + 1,
                                    })
                                }))
                            }),
                    )
                    .child(button("page-duplicate", "Duplicate").on_click(cx.listener(
                        move |view, _, _, _| {
                            (view.send)(Command::AddPage {
                                copy_of: Some(active),
                            })
                        },
                    )))
                    .child(
                        small_button("page-remove", "Remove", count > 1)
                            .py_2()
                            .when(count > 1, |d| {
                                d.on_click(cx.listener(move |view, _, window, cx| {
                                    view.start_page_edit(PageEdit::Remove(active), window, cx)
                                }))
                            }),
                    ),
            };
        div()
            .flex()
            .items_center()
            .gap_3()
            .child(tabs)
            .child(div().flex_shrink_0().child(tools))
            .into_any_element()
    }

    /// Escape stops the rename of a page.
    fn escape_page() -> impl Fn(&mut Self, &KeyDownEvent, &mut Window, &mut Context<Self>) + 'static
    {
        |view, event, window, cx| {
            if event.keystroke.key == "escape" {
                cx.stop_propagation();
                view.cancel_page_edit(window, cx);
            }
        }
    }

    /// Escape stops the edit of a profile name.
    fn escape_profile(
    ) -> impl Fn(&mut Self, &KeyDownEvent, &mut Window, &mut Context<Self>) + 'static {
        |view, event, window, cx| {
            if event.keystroke.key == "escape" {
                cx.stop_propagation();
                view.lists.profile_edit = None;
                window.blur(cx);
                cx.notify();
            }
        }
    }

    /// The button that shows the active profile, and the menu that it opens.
    pub(super) fn profile_control(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(active) = self.active_profile() else {
            return div().into_any_element();
        };
        let mut control = div()
            .relative()
            .flex()
            .items_center()
            .gap_2()
            .child(div().text_sm().text_color(rgb(MUTED)).child("Profile"));
        control = control.child(
            button("profile-menu", format!("{} ▾", active.name))
                .max_w(px(220.))
                .truncate()
                .when(self.lists.profiles_open, |d| d.border_color(rgb(SELECTED)))
                .on_click(cx.listener(|view, _, window, cx| {
                    if view.lists.profiles_open {
                        view.close_profiles(window, cx);
                    } else {
                        view.lists.profiles_open = true;
                        cx.notify();
                    }
                })),
        );
        if self.lists.profiles_open {
            control = control.child(deferred(self.profile_menu(active, cx)).with_priority(2));
        }
        control.into_any_element()
    }

    /// A layer over the window while the menu is open. A click on it closes
    /// the menu.
    pub(super) fn profile_backdrop(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.lists.profiles_open.then(|| {
            deferred(
                div()
                    .id("profile-backdrop")
                    .debug_selector(|| "profile-backdrop".into())
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .occlude()
                    .on_click(cx.listener(|view, _, window, cx| view.close_profiles(window, cx))),
            )
            .with_priority(1)
            .into_any_element()
        })
    }

    fn profile_menu(&self, active: &ProfileEntry, cx: &mut Context<Self>) -> AnyElement {
        let id = active.id.clone();
        let only = self.state.profiles.len() < 2;
        let mut menu = div()
            .id("profiles")
            .debug_selector(|| "profiles".into())
            .occlude()
            .absolute()
            .top(px(44.))
            .right_0()
            .w(px(300.))
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(rgb(BORDER))
            .bg(rgb(PANEL))
            .shadow_lg()
            .child(div().text_sm().text_color(rgb(MUTED)).child("Profiles"))
            .child(
                div()
                    .id("profile-list")
                    .max_h(px(220.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(self.state.profiles.iter().enumerate().map(|(index, p)| {
                        let selected = p.id == active.id;
                        let profile = p.id.clone();
                        button(
                            format!("profile-{index}"),
                            format!("{}{}", p.name, if selected { " ✓" } else { "" }),
                        )
                        .truncate()
                        .when(selected, |d| d.border_color(rgb(SELECTED)))
                        .on_click(cx.listener(
                            move |view, _, window, cx| {
                                if view.state.profile.as_ref() != Some(&profile) {
                                    (view.send)(Command::SelectProfile {
                                        profile: profile.clone(),
                                    });
                                }
                                view.close_profiles(window, cx);
                            },
                        ))
                    })),
            )
            .child(div().h(px(1.)).bg(rgb(BORDER)));
        let name_form = |label: String, save: &'static str, cx: &mut Context<Self>| {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_sm().child(label))
                .child(
                    div()
                        .id("profile-name")
                        .debug_selector(|| "profile-name".into())
                        .text_color(rgb(TEXT))
                        .capture_key_down(cx.listener(Self::escape_profile()))
                        .child(Input::new(&self.lists.profile_name)),
                )
                .child(
                    div()
                        .flex()
                        .gap_1()
                        .child(button("profile-name-save", save).on_click(
                            cx.listener(|view, _, window, cx| view.save_profile_name(window, cx)),
                        ))
                        .child(
                            button("profile-edit-cancel", "Cancel").on_click(cx.listener(
                                |view, _, window, cx| {
                                    view.lists.profile_edit = None;
                                    window.blur(cx);
                                    cx.notify();
                                },
                            )),
                        ),
                )
        };
        menu = match &self.lists.profile_edit {
            None => menu.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(button("profile-add", "New").on_click(cx.listener(
                        |view, _, window, cx| view.start_profile_edit(ProfileEdit::Add, window, cx),
                    )))
                    .child({
                        let id = id.clone();
                        button("profile-duplicate", "Duplicate").on_click(cx.listener(
                            move |view, _, window, cx| {
                                view.start_profile_edit(
                                    ProfileEdit::Duplicate(id.clone()),
                                    window,
                                    cx,
                                )
                            },
                        ))
                    })
                    .child({
                        let id = id.clone();
                        button("profile-rename", "Rename").on_click(cx.listener(
                            move |view, _, window, cx| {
                                view.start_profile_edit(ProfileEdit::Rename(id.clone()), window, cx)
                            },
                        ))
                    })
                    .child(small_button("profile-remove", "Remove", !only).py_2().when(
                        !only,
                        |d| {
                            d.on_click(cx.listener(move |view, _, window, cx| {
                                view.start_profile_edit(ProfileEdit::Remove(id.clone()), window, cx)
                            }))
                        },
                    )),
            ),
            Some(ProfileEdit::Add) => {
                menu.child(name_form("Name of the new profile".into(), "Add", cx))
            }
            Some(ProfileEdit::Duplicate(_)) => menu.child(name_form(
                format!("Name of the copy of “{}”", active.name),
                "Add",
                cx,
            )),
            Some(ProfileEdit::Rename(_)) => menu.child(name_form(
                format!("New name for “{}”", active.name),
                "Save",
                cx,
            )),
            Some(ProfileEdit::Remove(remove)) => {
                let remove = remove.clone();
                let name = self
                    .state
                    .profiles
                    .iter()
                    .find(|p| p.id == remove)
                    .map_or_else(|| remove.clone(), |p| p.name.clone());
                menu.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .child(format!("Remove the profile “{name}”?")),
                        )
                        .child(div().text_xs().text_color(rgb(MUTED)).child(
                            "OpenCraft does not delete it. It moves the profile to the \
                             folder “removed” in the profiles folder.",
                        ))
                        .child(
                            div()
                                .flex()
                                .gap_1()
                                .child(danger_button("profile-remove-confirm", "Remove").on_click(
                                    cx.listener(move |view, _, window, cx| {
                                        (view.send)(Command::RemoveProfile {
                                            profile: remove.clone(),
                                        });
                                        view.close_profiles(window, cx);
                                    }),
                                ))
                                .child(button("profile-edit-cancel", "Cancel").on_click(
                                    cx.listener(|view, _, _, cx| {
                                        view.lists.profile_edit = None;
                                        cx.notify();
                                    }),
                                )),
                        ),
                )
            }
        };
        menu.into_any_element()
    }
}

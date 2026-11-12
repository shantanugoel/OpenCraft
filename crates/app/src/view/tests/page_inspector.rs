//! The page inspector (decision 0029, section 10.6): the three ways to
//! select the page, the page commands, and the swipe cards.

use super::pages::{pages, profiles, send, test_action, test_kind};
use super::*;
use gpui_kit::{Bounds, Pixels};

fn bounds(cx: &mut VisualTestContext, id: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(id)
        .unwrap_or_else(|| panic!("No element: {id}"))
}

fn page_selected(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| view.read(cx).state.page_selected)
}

fn name_text(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, cx| {
        view.read(cx)
            .page_inspector
            .name
            .read(cx)
            .value()
            .to_string()
    })
}

fn root_focused(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| view.read(cx).focus.is_focused(window))
}

/// Put the focus in the name field and select its text, so that typed text
/// replaces the name.
fn edit_name(cx: &mut VisualTestContext) {
    click(cx, "page-name");
    cx.update(|window, cx| {
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
    });
    cx.run_until_parked();
}

/// The window with three pages, the second page active, and the page as the
/// selection.
fn page_window(
    cx: &mut TestAppContext,
) -> (
    Entity<DeviceView>,
    &mut VisualTestContext,
    async_channel::Sender<Event>,
    Commands,
) {
    let (view, cx, tx, sent) = window(cx);
    cx.update(|window, _| window.activate_window());
    send(cx, &tx, profiles(&[("work", "Work")], "work"));
    send(cx, &tx, pages(&["Main", "Media", "Extra"], 1));
    click(cx, "page-settings");
    assert!(page_selected(&view, cx));
    sent.borrow_mut().clear();
    (view, cx, tx, sent)
}

#[gpui_kit::test]
fn the_page_is_selected_in_three_ways(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, profiles(&[("work", "Work")], "work"));
    send(cx, &tx, pages(&["Main", "Media", "Extra"], 1));
    // A short window: the swipe cards are below the visible part of the
    // inspector.
    cx.simulate_resize(size(px(1440.), px(400.)));
    cx.run_until_parked();
    assert!(!page_selected(&view, cx));

    // 1. A click on an empty part of the canvas.
    let area = bounds(cx, "device-area");
    cx.simulate_click(
        gpui_kit::point(area.left() + px(4.), area.top() + px(4.)),
        Modifiers::default(),
    );
    cx.run_until_parked();
    assert!(page_selected(&view, cx));
    assert!(cx.debug_bounds("appearance").is_none());
    for id in ["page-picture", "page-name", "page", "touch-bar-gestures"] {
        assert!(cx.debug_bounds(id).is_some(), "{id}");
    }
    assert_eq!(name_text(&view, cx), "Media");
    // The page picture is 72 × 44 at the start of the header.
    let picture = bounds(cx, "page-picture");
    assert_eq!(
        picture.size,
        size(theme::PAGE_PICTURE_WIDTH, theme::PAGE_PICTURE_HEIGHT)
    );
    let header = bounds(cx, "inspector-header");
    assert_eq!(picture.left() - header.left(), theme::SPACE_16);

    // 2. "Page settings" in the page header. It does not scroll.
    click(cx, "surface-key face 2");
    assert!(!page_selected(&view, cx));
    assert!(cx.debug_bounds("page-name").is_none());
    click(cx, "page-settings");
    assert!(page_selected(&view, cx));
    let panel = bounds(cx, "right-panel");
    assert!(bounds(cx, "touch-bar-gestures").bottom() > panel.bottom());

    // 3. The "◀ Touch bar ▶" control. It scrolls the inspector to the swipe
    // cards, also when the page is the selection already.
    click(cx, "control-touch bar 2");
    assert!(page_selected(&view, cx));
    let swipes = bounds(cx, "touch-bar-gestures");
    assert!(
        swipes.bottom() <= panel.bottom() + px(0.5),
        "{swipes:?} {panel:?}"
    );
    assert!(swipes.top() >= panel.top());
    assert!(sent.borrow().is_empty(), "A selection sends no command");
}

#[gpui_kit::test]
fn the_name_field_renames_the_page_with_one_command(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = page_window(cx);

    // Return keeps the name: one command for the full edit, thus one step
    // of the history.
    edit_name(cx);
    cx.simulate_input("Calls");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        sent.borrow().as_slice(),
        &[Command::RenamePage {
            page: 1,
            name: "Calls".into()
        }]
    );
    assert!(root_focused(&view, cx));
    send(cx, &tx, pages(&["Main", "Calls", "Extra"], 1));
    assert_eq!(name_text(&view, cx), "Calls");

    // Escape stops the edit with no change.
    sent.borrow_mut().clear();
    edit_name(cx);
    cx.simulate_input("Other");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(sent.borrow().is_empty());
    assert_eq!(name_text(&view, cx), "Calls");
    assert!(root_focused(&view, cx));

    // An empty name changes nothing.
    edit_name(cx);
    cx.update(|window, cx| {
        window.dispatch_action(Box::new(gpui_kit::component::input::Backspace), cx);
    });
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(sent.borrow().is_empty());
    assert_eq!(name_text(&view, cx), "Calls");

    // A click out of the field keeps the name. A page button ends the edit
    // first, so that the rename comes before its command.
    edit_name(cx);
    cx.simulate_input("Phone");
    click(cx, "page-move-up");
    assert_eq!(
        sent.borrow().as_slice(),
        &[
            Command::RenamePage {
                page: 1,
                name: "Phone".into()
            },
            Command::MovePage { page: 1, to: 0 }
        ]
    );

    // A change of the pages during the edit stops it: the number can be a
    // different page now.
    sent.borrow_mut().clear();
    edit_name(cx);
    cx.simulate_input("Late");
    send(cx, &tx, pages(&["Phone", "Main", "Extra"], 0));
    cx.run_until_parked();
    assert!(sent.borrow().is_empty());
    assert_eq!(name_text(&view, cx), "Phone");
    assert!(root_focused(&view, cx));

    // A page with no name shows its number. That text is no new name.
    send(cx, &tx, pages(&["", "Main", "Extra"], 0));
    assert_eq!(name_text(&view, cx), "Page 1");
    edit_name(cx);
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(sent.borrow().is_empty());
}

#[gpui_kit::test]
fn the_page_buttons_move_duplicate_and_remove_the_page(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = page_window(cx);
    let last = |sent: &Commands| sent.borrow().last().cloned();

    click(cx, "page-move-down");
    assert_eq!(last(&sent), Some(Command::MovePage { page: 1, to: 2 }));
    click(cx, "page-move-up");
    assert_eq!(last(&sent), Some(Command::MovePage { page: 1, to: 0 }));
    click(cx, "page-duplicate");
    assert_eq!(last(&sent), Some(Command::AddPage { copy_of: Some(1) }));

    // The first page cannot move up, the last page cannot move down.
    send(cx, &tx, pages(&["Main", "Media", "Extra"], 0));
    let count = sent.borrow().len();
    click(cx, "page-move-up");
    send(cx, &tx, pages(&["Main", "Media", "Extra"], 2));
    click(cx, "page-move-down");
    assert_eq!(sent.borrow().len(), count);

    // "Remove…" asks first, as in the sidebar. The question names the page.
    click(cx, "page-remove");
    let (message, _) = cx.pending_prompt().expect("a question");
    assert_eq!(message, "Remove the page “Extra”?");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(sent.borrow().len(), count);
    click(cx, "page-remove");
    cx.simulate_prompt_answer("Remove");
    cx.run_until_parked();
    assert_eq!(last(&sent), Some(Command::RemovePage { page: 2 }));

    // The last page cannot go.
    send(cx, &tx, pages(&["Main"], 0));
    let count = sent.borrow().len();
    click(cx, "page-remove");
    assert!(!cx.has_pending_prompt());
    assert_eq!(sent.borrow().len(), count);
}

#[gpui_kit::test]
fn the_swipe_cards_of_the_page_set_the_touch_bar_bindings(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = page_window(cx);
    send(cx, &tx, Event::Kinds(vec![test_kind()]));
    let swipe = |gesture| BindingId {
        page: 1,
        control: ControlId::new(ControlKind::TouchBar, 2),
        gesture,
    };
    // The swipe cards have the wide name column.
    let card = bounds(cx, "gesture-SwipeLeft");
    assert!(card.size.width > px(0.));
    click(cx, "gesture-SwipeRight-kind");
    click(cx, "kind-0");
    assert_eq!(
        sent.borrow().last(),
        Some(&Command::SetBinding {
            binding: swipe(GestureKind::SwipeRight),
            action: Some(test_action("start"))
        })
    );
    // The engine makes the binding: the card opens with its form.
    send(
        cx,
        &tx,
        Event::Binding(BindingState {
            binding: swipe(GestureKind::SwipeRight),
            action: Some(test_action("start")),
            available: true,
        }),
    );
    assert!(page_selected(&view, cx));
    cx.update(|_, cx| {
        assert_eq!(
            view.read(cx).state.binding_id(),
            Some(swipe(GestureKind::SwipeRight))
        )
    });
    assert!(cx.debug_bounds("field-0").is_some());
}

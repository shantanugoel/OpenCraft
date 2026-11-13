//! The icon of a surface (decision 0029, sections 10.2 and 14; issue
//! #190): the row "Icon", and the icon picker with the search, the choice
//! of an icon, the colour, "No icon", and the tab "Recent". The kind tiles
//! show the icon of the kind description.

use super::*;
use crate::view::gesture_card::KindLabel;
use opencraft_engine::SurfaceIcon;

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

/// A surface event with the icon `icon`.
fn with_icon(id: SurfaceId, icon: Option<SurfaceIcon>) -> Event {
    Event::Surface(SurfaceState {
        id,
        content: SurfaceContent {
            icon,
            ..Default::default()
        },
        image: Arc::new(RgbImage::new(90, 90)),
        states: None,
    })
}

/// The icon commands in `sent`.
fn icon_commands(sent: &Commands) -> Vec<(SurfaceId, Option<SurfaceIcon>)> {
    sent.borrow()
        .iter()
        .filter_map(|c| match c {
            Command::SetIcon { surface, icon } => Some((*surface, icon.clone())),
            _ => None,
        })
        .collect()
}

fn icon(name: &str, color: Option<Color>) -> Option<SurfaceIcon> {
    Some(SurfaceIcon {
        name: name.into(),
        color,
    })
}

fn is_open(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| view.read(cx).inspector.icon_picker.open)
}

/// The text of the number of icons in the search field.
fn count(view: &Entity<DeviceView>, cx: &mut VisualTestContext) -> usize {
    cx.update(|_, cx| view.read(cx).inspector.icon_picker.count)
}

/// Replace the text of the focused search field with `text`.
fn search(cx: &mut VisualTestContext, text: &str) {
    cx.update(|window, cx| {
        window.dispatch_action(Box::new(gpui_kit::component::input::SelectAll), cx);
    });
    cx.simulate_input(text);
    cx.run_until_parked();
}

/// Select key face 2 and open the icon picker.
fn open(cx: &mut VisualTestContext) {
    click(cx, "surface-key face 2");
    click(cx, "choose-icon");
    assert!(cx.debug_bounds("icon-picker").is_some(), "the popover");
}

#[gpui_kit::test]
fn the_search_field_has_the_focus_and_filters_the_grid(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = window(cx);
    open(cx);
    assert_eq!(count(&view, cx), opencraft_engine::icon_set().icons().len());
    // With no text, the grid starts with the first group.
    assert!(cx.debug_bounds("icon-cell-accessibility").is_some());

    // The field has the focus: the typed text goes into it.
    cx.simulate_input("globe");
    cx.run_until_parked();
    assert!(cx.debug_bounds("icon-cell-globe").is_some());
    assert!(cx.debug_bounds("icon-cell-accessibility").is_none());
    let found = count(&view, cx);
    assert!(found > 1 && found < 20, "{found}");
    assert!(cx.debug_bounds("icon-empty").is_none());

    // No result.
    cx.simulate_input("zzzz");
    cx.run_until_parked();
    assert_eq!(count(&view, cx), 0);
    assert!(cx.debug_bounds("icon-empty").is_some());

    // Escape closes the popover, and the search starts empty at the next
    // opening.
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!is_open(&view, cx));
    assert!(cx.debug_bounds("icon-picker").is_none());
    click(cx, "choose-icon");
    assert_eq!(count(&view, cx), opencraft_engine::icon_set().icons().len());
}

#[gpui_kit::test]
fn a_click_on_an_icon_sets_it_and_the_popover_stays_open(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    open(cx);
    cx.simulate_input("globe");
    cx.run_until_parked();
    click(cx, "icon-cell-globe");
    assert_eq!(icon_commands(&sent), [(key(), icon("globe", None))]);
    assert!(is_open(&view, cx));
    // The icon is in the recent icons of the shell, and in the tab
    // "Recent".
    let recent = cx.update(|_, cx| view.read(cx).shell.read(cx).recent_icons().to_vec());
    assert_eq!(recent, ["globe"]);
    send(cx, &tx, with_icon(key(), icon("globe", None)));
    click(cx, "icon-tab-recent");
    assert_eq!(count(&view, cx), 1);
    assert!(cx.debug_bounds("icon-cell-globe").is_some());
    click(cx, "icon-tab-icons");
    assert!(count(&view, cx) > 1);

    // A click out of the popover closes it and sends nothing.
    let before = sent.borrow().len();
    click(cx, "canvas");
    assert!(!is_open(&view, cx));
    assert_eq!(sent.borrow().len(), before);
}

#[gpui_kit::test]
fn a_colour_changes_the_icon_or_waits_for_the_next_icon(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    let amber = colour(theme::ICON_COLOURS[2]);
    let green = colour(theme::ICON_COLOURS[3]);
    // With no icon, the colour is for the next icon.
    open(cx);
    click(cx, "icon-colour-2");
    assert!(icon_commands(&sent).is_empty());
    cx.simulate_input("mail");
    cx.run_until_parked();
    click(cx, "icon-cell-mail");
    assert_eq!(icon_commands(&sent), [(key(), icon("mail", Some(amber)))]);

    // With an icon, a colour changes it at once, and a new icon keeps the
    // colour.
    send(cx, &tx, with_icon(key(), icon("mail", Some(amber))));
    click(cx, "icon-colour-3");
    send(cx, &tx, with_icon(key(), icon("mail", Some(green))));
    search(cx, "globe");
    click(cx, "icon-cell-globe");
    assert_eq!(
        icon_commands(&sent)[1..],
        [
            (key(), icon("mail", Some(green))),
            (key(), icon("globe", Some(green)))
        ]
    );
}

#[gpui_kit::test]
fn no_icon_removes_the_icon_and_closes_the_popover(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    send(cx, &tx, with_icon(key(), icon("mail", None)));
    open(cx);
    click(cx, "icon-no-icon");
    assert_eq!(icon_commands(&sent), [(key(), None)]);
    assert!(!is_open(&view, cx));
}

#[gpui_kit::test]
fn a_bar_panel_has_the_icon_row_and_the_picker(cx: &mut TestAppContext) {
    let (_view, cx, _tx, sent) = window(cx);
    click(cx, "surface-bar panel 3");
    assert!(cx.debug_bounds("icon-tile").is_some());
    click(cx, "choose-icon");
    cx.simulate_input("volume-2");
    cx.run_until_parked();
    click(cx, "icon-cell-volume-2");
    assert_eq!(icon_commands(&sent), [(bar(), icon("volume-2", None))]);
}

#[test]
fn a_kind_tile_shows_the_icon_of_the_description() {
    let kind = |icon: Option<&str>| KindDescription {
        kind: "test.web".into(),
        name: "Open a URL".into(),
        group: "System".into(),
        icon: icon.map(Into::into),
        ..Default::default()
    };
    assert_eq!(
        KindLabel::new(&kind(Some("globe"))).icon(),
        Some(&theme::surface_icon_path("globe"))
    );
    // No icon, or an icon that the set does not have: the first letter.
    assert_eq!(KindLabel::new(&kind(None)).icon(), None);
    assert_eq!(KindLabel::new(&kind(Some("no-such-icon"))).icon(), None);
}

#[test]
fn the_asset_source_gives_each_icon_of_the_engine_and_the_window_icons() {
    use gpui_kit::AssetSource;
    let assets = theme::WindowAssets;
    let globe = assets
        .load(&theme::surface_icon_path("globe"))
        .unwrap()
        .expect("the icon");
    assert!(std::str::from_utf8(&globe).unwrap().starts_with("<svg"));
    assert!(assets
        .load(&theme::surface_icon_path("no-such-icon"))
        .unwrap()
        .is_none());
    assert!(assets.load("icons/search.svg").unwrap().is_some());
}

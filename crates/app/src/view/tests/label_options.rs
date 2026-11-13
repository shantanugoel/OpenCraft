//! The label options of the "Appearance" section (decision 0029, section
//! 10.2; issue #189): "Position", and "Text" with the size, the colour, and
//! bold, on a key face and on a bar panel.

use super::*;
use opencraft_engine::{LabelChange, LabelPosition, LabelStyle};

/// The label style commands in `sent`, for the surface `id`.
fn changes(sent: &Commands, id: SurfaceId) -> Vec<LabelChange> {
    sent.borrow()
        .iter()
        .filter_map(|c| match c {
            Command::SetLabelStyle { surface, change } if *surface == id => Some(*change),
            _ => None,
        })
        .collect()
}

/// A surface event with the label "Mail", the style `style`, and an image
/// or no image.
fn styled(id: SurfaceId, style: LabelStyle, image: bool) -> Event {
    Event::Surface(SurfaceState {
        id,
        content: SurfaceContent {
            label: Some("Mail".into()),
            label_style: style,
            image: image.then(|| "images/0123456789abcdef.png".to_string()),
            ..Default::default()
        },
        image: Arc::new(RgbImage::new(90, 90)),
        states: None,
    })
}

fn send(cx: &mut VisualTestContext, tx: &async_channel::Sender<Event>, event: Event) {
    tx.try_send(event).unwrap();
    cx.run_until_parked();
}

/// The selected segment of "Position": the one with no click handler, thus
/// a click on it sends nothing.
fn click_each_position(cx: &mut VisualTestContext) {
    for id in [
        "label-position-top",
        "label-position-middle",
        "label-position-bottom",
        "label-position-none",
    ] {
        click(cx, id);
    }
}

#[gpui_kit::test]
fn each_position_segment_sets_its_position_on_a_key_face_and_a_bar_panel(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    let all = |except: LabelPosition| {
        LabelPosition::ALL
            .into_iter()
            .filter(|p| *p != except)
            .map(|p| LabelChange::Position(Some(p)))
            .collect::<Vec<_>>()
    };
    // A key face with no image: the automatic position is the middle, and
    // a click on the selected segment sends nothing.
    click(cx, "surface-key face 2");
    click_each_position(cx);
    assert_eq!(changes(&sent, key()), all(LabelPosition::Middle));

    // A bar panel with an image: the automatic position is the bottom.
    send(cx, &tx, styled(bar(), LabelStyle::default(), true));
    click(cx, "surface-bar panel 3");
    click_each_position(cx);
    assert_eq!(changes(&sent, bar()), all(LabelPosition::Bottom));

    // A position of the profile is the selected segment.
    let top = LabelStyle {
        position: Some(LabelPosition::Top),
        ..LabelStyle::default()
    };
    send(cx, &tx, styled(bar(), top, true));
    sent.borrow_mut().clear();
    click_each_position(cx);
    assert_eq!(changes(&sent, bar()), all(LabelPosition::Top));
}

#[gpui_kit::test]
fn the_size_menu_sets_each_size_and_shows_the_size_of_the_profile(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("label-size-menu").is_none());
    for size in opencraft_engine::LABEL_SIZES {
        click(cx, "label-size");
        assert!(cx.debug_bounds("label-size-menu").is_some(), "{size}");
        // A test ID must be static.
        let id: &'static str = format!("label-size-{size}").leak();
        click(cx, id);
        assert!(cx.debug_bounds("label-size-menu").is_none(), "{size}");
    }
    let want: Vec<_> = opencraft_engine::LABEL_SIZES
        .into_iter()
        .map(LabelChange::Size)
        .collect();
    assert_eq!(changes(&sent, key()), want);

    // The button shows the size of the profile. Escape closes the menu.
    let small = LabelStyle {
        size: 14,
        ..LabelStyle::default()
    };
    send(cx, &tx, styled(key(), small, false));
    cx.update(|_, cx| {
        let style = view.read(cx).state.surfaces[&key()].content.label_style;
        assert_eq!(style.size, 14);
    });
    click(cx, "label-size");
    // The size of the profile has the check mark.
    assert!(cx.debug_bounds("label-size-14").is_some());
    assert!(cx.debug_bounds("label-size-menu").is_some());
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("label-size-menu").is_none());
    assert_eq!(changes(&sent, key()).len(), want.len());
}

#[gpui_kit::test]
fn the_colour_popover_sets_automatic_a_sample_or_a_hex_value(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    click(cx, "surface-bar panel 3");
    click(cx, "label-colour");
    assert!(cx.debug_bounds("label-colour-popover").is_some());
    // The first two samples are white and black, then the fill colours.
    click(cx, "label-colour-0");
    assert!(cx.debug_bounds("label-colour-popover").is_none());
    click(cx, "label-colour");
    click(cx, "label-colour-1");
    click(cx, "label-colour");
    click(cx, "label-colour-10");
    // The field has the focus. An incomplete value does nothing.
    click(cx, "label-colour");
    cx.simulate_input("#12345");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(cx.debug_bounds("label-colour-popover").is_some());
    cx.simulate_input("6");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(cx.debug_bounds("label-colour-popover").is_none());
    // With a colour in the profile, "Automatic" removes it.
    let red = LabelStyle {
        color: Some(colour(0xb83f4c)),
        ..LabelStyle::default()
    };
    send(cx, &tx, styled(bar(), red, false));
    click(cx, "label-colour");
    click(cx, "label-colour-automatic");
    assert_eq!(
        changes(&sent, bar()),
        [
            LabelChange::Color(Some(colour(0xffffff))),
            LabelChange::Color(Some(colour(0x000000))),
            LabelChange::Color(Some(colour(COLOURS[8]))),
            LabelChange::Color(Some(colour(0x123456))),
            LabelChange::Color(None),
        ]
    );
    // Escape closes the popover with no change.
    click(cx, "label-colour");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("label-colour-popover").is_none());
    assert_eq!(changes(&sent, bar()).len(), 5);
}

#[gpui_kit::test]
fn the_bold_button_switches_the_weight(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    // A label is bold by default: a click makes it regular.
    click(cx, "label-bold");
    let regular = LabelStyle {
        bold: false,
        ..LabelStyle::default()
    };
    send(cx, &tx, styled(key(), regular, false));
    click(cx, "label-bold");
    assert_eq!(
        changes(&sent, key()),
        [LabelChange::Bold(false), LabelChange::Bold(true)]
    );
}

#[gpui_kit::test]
fn the_label_rows_are_as_high_as_a_text_field_and_aligned_with_the_title(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = window(cx);
    for surface in ["surface-key face 2", "surface-bar panel 3"] {
        click(cx, surface);
        let mut bounds = |id: &'static str| {
            cx.debug_bounds(id)
                .unwrap_or_else(|| panic!("No element: {id}"))
        };
        let title = bounds("title-field");
        let track = bounds("label-position");
        let size = bounds("label-size");
        let colour = bounds("label-colour");
        let bold = bounds("label-bold");
        assert_eq!(track.size.height, theme::HEIGHT_CONTROL, "{surface}");
        assert_eq!(track.left(), title.left(), "{surface}");
        assert_eq!(track.right(), title.right(), "{surface}");
        assert_eq!(size.left(), title.left(), "{surface}");
        for b in [size, colour, bold] {
            assert_eq!(b.size.height, theme::HEIGHT_CONTROL, "{surface}");
            assert_eq!(b.center().y, size.center().y, "{surface}");
        }
        assert_eq!(colour.size.width, theme::WIDTH_SQUARE_BUTTON);
        assert_eq!(bold.size.width, theme::WIDTH_SQUARE_BUTTON);
        assert_eq!(colour.left() - size.right(), theme::GAP_ITEMS);
        assert_eq!(bold.left() - colour.right(), theme::GAP_ITEMS);
        // The four segments have one width, and fill the track.
        let segments = [
            "label-position-top",
            "label-position-middle",
            "label-position-bottom",
            "label-position-none",
        ]
        .map(&mut bounds);
        for segment in &segments {
            assert_eq!(segment.size.height, theme::HEIGHT_SEGMENT_INSPECTOR);
            assert!(
                // The layout rounds each edge to a half point.
                (segment.size.width - segments[0].size.width).abs() <= px(1.),
                "{segments:?} {track:?}"
            );
        }
        // The rows are in the order of the record, between "Title" and "Fill".
        let fill = bounds("no-colour");
        assert!(title.bottom() < track.top() && track.bottom() < size.top());
        assert!(size.bottom() < fill.top());
    }
}

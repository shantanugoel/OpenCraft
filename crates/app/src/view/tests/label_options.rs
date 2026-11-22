//! The label options of the "Appearance" section (decision 0029, section
//! 10.2; issue #189): "Position", and "Text" with the size, the colour, and
//! bold, on a key face and on a bar panel.

use super::*;
use opencraft_engine::{Auto, LabelChange, LabelPosition, LabelStyle};

/// The label style commands in `sent`, for the surface `id`.
fn changes(sent: &Commands, id: SurfaceId) -> Vec<LabelChange> {
    sent.borrow()
        .iter()
        .filter_map(|c| match c {
            Command::SetLabelStyle { surface, change } if *surface == id => Some(change.clone()),
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
        image_size: None,
        image_area: None,
        on_image_size: None,
        on_image_area: None,
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
            .map(|p| LabelChange::Position(Some(Auto::Value(p))))
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
        position: Some(Auto::Value(LabelPosition::Top)),
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
    // The menu has each size at the smallest window.
    cx.simulate_resize(size(px(1080.), px(700.)));
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("label-size-menu").is_none());
    for size in opencraft_engine::LABEL_SIZES {
        click(cx, "label-size");
        let menu = cx
            .debug_bounds("label-size-menu")
            .unwrap_or_else(|| panic!("no size menu for {size}"));
        assert!(
            menu.origin.y >= px(0.) && menu.bottom() <= px(700.),
            "the size menu is in the window: {menu:?}"
        );
        // A test ID must be static.
        let id: &'static str = format!("label-size-{size}").leak();
        click(cx, id);
        assert!(cx.debug_bounds("label-size-menu").is_none(), "{size}");
    }
    let want: Vec<_> = opencraft_engine::LABEL_SIZES
        .into_iter()
        .map(|size| LabelChange::Size(Some(size)))
        .collect();
    assert_eq!(changes(&sent, key()), want);

    // The button shows the size of the profile. Escape closes the menu.
    let small = LabelStyle {
        size: Some(14),
        ..LabelStyle::default()
    };
    send(cx, &tx, styled(key(), small, false));
    cx.update(|_, cx| {
        let content = &view.read(cx).state.surfaces[&key()].content;
        let style = content.label_style.resolve(&LabelStyle::default());
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

/// The profiles event with the active profile `default` and the default
/// title style `style`.
fn profile_default(style: LabelStyle) -> Event {
    Event::Profiles {
        profiles: vec![opencraft_engine::ProfileEntry {
            id: "default".into(),
            name: "Default".into(),
            default_label_style: style,
            ..Default::default()
        }],
        active: "default".into(),
        default: "default".into(),
        cause: opencraft_engine::ActiveCause::Selected,
    }
}

#[gpui_kit::test]
fn the_title_shows_whether_it_follows_the_profile_default(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    let default = LabelStyle {
        font: Some(Auto::Value("Menlo".into())),
        position: Some(Auto::Value(LabelPosition::Top)),
        size: Some(30),
        color: Some(Auto::Value(colour(0xb83f4c))),
        bold: Some(false),
        italic: Some(true),
        underline: Some(true),
    };
    send(cx, &tx, profile_default(default.clone()));
    cx.update(|_, cx| {
        assert_eq!(_view.read(cx).state.default_label_style().size, Some(30));
    });
    send(cx, &tx, styled(key(), LabelStyle::default(), false));
    assert!(cx.debug_bounds("title-style-default").is_some());
    assert!(
        cx.debug_bounds("title-style-reset").is_none(),
        "a title with no own style has nothing to reset"
    );

    // An own value shows the reset, and the reset clears each part.
    let own = LabelStyle {
        size: Some(14),
        ..LabelStyle::default()
    };
    send(cx, &tx, styled(key(), own, false));
    assert!(cx.debug_bounds("title-style-reset").is_some());
    click(cx, "title-style-reset");
    let Some(Command::Group { name, commands }) = sent.borrow().last().cloned() else {
        panic!("{:?}", sent.borrow());
    };
    assert_eq!(name, "Reset the Title Style");
    assert_eq!(commands.len(), 7);
    for command in commands {
        let Command::SetLabelStyle { surface, change } = command else {
            panic!("{command:?}");
        };
        assert_eq!(surface, key());
        assert!(matches!(
            change,
            LabelChange::Font(None)
                | LabelChange::Position(None)
                | LabelChange::Size(None)
                | LabelChange::Color(None)
                | LabelChange::Bold(None)
                | LabelChange::Italic(None)
                | LabelChange::Underline(None)
        ));
    }
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
        color: Some(Auto::Value(colour(0xb83f4c))),
        ..LabelStyle::default()
    };
    send(cx, &tx, styled(bar(), red, false));
    click(cx, "label-colour");
    click(cx, "label-colour-automatic");
    assert_eq!(
        changes(&sent, bar()),
        [
            LabelChange::Color(Some(Auto::Value(colour(0xffffff)))),
            LabelChange::Color(Some(Auto::Value(colour(0x000000)))),
            LabelChange::Color(Some(Auto::Value(colour(COLOURS[8])))),
            LabelChange::Color(Some(Auto::Value(colour(0x123456)))),
            // "Automatic" is an own value of the surface, not an inherited
            // one: the colour stops following the profile default
            // (decision 0043, section 2).
            LabelChange::Color(Some(Auto::Auto)),
        ]
    );
    // Escape closes the popover with no change.
    click(cx, "label-colour");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("label-colour-popover").is_none());
    assert_eq!(changes(&sent, bar()).len(), 5);
}

/// The colour popover shows the colour of the profile default when the
/// title follows it, and "Automatic" over that colour is a choice
/// (decision 0043, section 2; review 3, H2).
#[gpui_kit::test]
fn the_colour_popover_shows_the_colour_of_the_profile_default(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    let red = colour(0xb83f4c);
    send(
        cx,
        &tx,
        profile_default(LabelStyle {
            color: Some(Auto::Value(red)),
            ..LabelStyle::default()
        }),
    );
    // The key has no own colour: it shows the colour of the profile default.
    send(cx, &tx, styled(key(), LabelStyle::default(), false));
    click(cx, "label-colour");
    assert!(cx.debug_bounds("label-colour-popover").is_some());
    let text = cx.update(|_, cx| {
        view.read(cx)
            .inspector
            .text_colour_input
            .read(cx)
            .value()
            .to_string()
    });
    assert_eq!(text, "#B83F4C", "the field shows the colour that shows");
    // The sample of the colour that shows is selected, and a click on it
    // makes no undo step. The samples are white and black, then the fills.
    let index = theme::TITLE_COLOURS.len()
        + theme::FILL_COLOURS
            .iter()
            .position(|value| *value == 0xb83f4c)
            .expect("the colour of the profile default is a sample");
    let sample = format!("label-colour-{index}");
    click(cx, Box::leak(sample.into_boxed_str()));
    assert!(
        changes(&sent, key()).is_empty(),
        "a click on the selected sample makes no change"
    );
    assert!(
        cx.debug_bounds("label-colour-popover").is_some(),
        "the popover stays open"
    );
    // "Automatic" stops the follow of the profile default for the colour:
    // one change, thus one step of the history.
    click(cx, "label-colour-automatic");
    assert_eq!(
        changes(&sent, key()),
        [LabelChange::Color(Some(Auto::Auto))]
    );
}

#[gpui_kit::test]
fn the_selected_automatic_text_colour_makes_no_change(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    click(cx, "surface-bar panel 3");
    // The profile has the automatic colour as an own value of the surface.
    let automatic = LabelStyle {
        color: Some(Auto::Auto),
        ..LabelStyle::default()
    };
    send(cx, &tx, styled(bar(), automatic, false));
    click(cx, "label-colour");
    click(cx, "label-colour-automatic");
    assert!(
        changes(&sent, bar()).is_empty(),
        "a click on the selected automatic swatch makes no change"
    );
}

#[gpui_kit::test]
fn the_bold_button_switches_the_weight(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    // A label is bold by default: a click makes it regular.
    click(cx, "label-bold");
    let regular = LabelStyle {
        bold: Some(false),
        ..LabelStyle::default()
    };
    send(cx, &tx, styled(key(), regular, false));
    click(cx, "label-bold");
    assert_eq!(
        changes(&sent, key()),
        [
            LabelChange::Bold(Some(false)),
            LabelChange::Bold(Some(true))
        ]
    );
}

#[gpui_kit::test]
fn the_italic_and_the_underline_buttons_switch_their_style(cx: &mut TestAppContext) {
    let (_view, cx, tx, sent) = window(cx);
    click(cx, "surface-key face 2");
    // A label is upright and has no underline at the start.
    click(cx, "label-italic");
    click(cx, "label-underline");
    assert_eq!(
        changes(&sent, key()),
        [
            LabelChange::Italic(Some(true)),
            LabelChange::Underline(Some(true))
        ]
    );
    // The style of the profile puts the buttons on, and a click removes it.
    sent.borrow_mut().clear();
    send(
        cx,
        &tx,
        styled(
            key(),
            LabelStyle {
                italic: Some(true),
                underline: Some(true),
                ..LabelStyle::default()
            },
            false,
        ),
    );
    click(cx, "label-italic");
    click(cx, "label-underline");
    assert_eq!(
        changes(&sent, key()),
        [
            LabelChange::Italic(Some(false)),
            LabelChange::Underline(Some(false))
        ]
    );
}

#[gpui_kit::test]
fn the_italic_and_the_underline_buttons_fit_at_the_smallest_inspector(cx: &mut TestAppContext) {
    let (_view, cx, _tx, _sent) = window(cx);
    click(cx, "surface-key face 2");
    // A drag of the divider to the right edge makes the inspector as narrow
    // as it can be.
    let start = cx.debug_bounds("inspector-divider").unwrap().center();
    let end = gpui_kit::point(px(theme::WINDOW_SIZE.0 - theme::INSPECTOR_MIN), start.y);
    for step in 0..=8 {
        let t = step as f32 / 8.;
        let at = gpui_kit::point(start.x + (end.x - start.x) * t, start.y);
        if step == 0 {
            cx.simulate_mouse_move(at, None, Modifiers::default());
            cx.simulate_mouse_down(at, gpui_kit::MouseButton::Left, Modifiers::default());
        } else {
            cx.simulate_mouse_move(at, gpui_kit::MouseButton::Left, Modifiers::default());
        }
    }
    cx.simulate_mouse_up(end, gpui_kit::MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    let mut bounds = |id: &'static str| {
        cx.debug_bounds(id)
            .unwrap_or_else(|| panic!("No element: {id}"))
    };
    // The row "Text" keeps each control, and the last one is inside the
    // panel.
    let title = bounds("title-field");
    let underline = bounds("label-underline");
    // The panel is at its minimum: the value column is 214 pixels wide.
    assert!(
        title.size.width <= px(214.),
        "the inspector is not at its minimum: {title:?}"
    );
    assert_eq!(underline.size.width, theme::WIDTH_SQUARE_BUTTON);
    assert!(
        underline.right() <= title.right() + px(0.5),
        "{underline:?} {title:?}"
    );
    for id in ["label-size", "label-colour", "label-bold", "label-italic"] {
        let control = bounds(id);
        assert_eq!(control.center().y, underline.center().y, "{id}");
        assert!(control.left() >= title.left(), "{id}");
    }
    // The buttons are in the order of the row, with the gap of a row.
    let bold = bounds("label-bold");
    let italic = bounds("label-italic");
    let colour = bounds("label-colour");
    assert_eq!(bold.left() - colour.right(), theme::GAP_ITEMS);
    assert_eq!(italic.left() - bold.right(), theme::GAP_ITEMS);
    assert_eq!(underline.left() - italic.right(), theme::GAP_ITEMS);
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
        let italic = bounds("label-italic");
        let underline = bounds("label-underline");
        assert_eq!(track.size.height, theme::HEIGHT_CONTROL, "{surface}");
        assert_eq!(track.left(), title.left(), "{surface}");
        assert_eq!(track.right(), title.right(), "{surface}");
        assert_eq!(size.left(), title.left(), "{surface}");
        for b in [size, colour, bold, italic, underline] {
            assert_eq!(b.size.height, theme::HEIGHT_CONTROL, "{surface}");
            assert_eq!(b.center().y, size.center().y, "{surface}");
        }
        assert_eq!(colour.size.width, theme::WIDTH_SQUARE_BUTTON);
        assert_eq!(bold.size.width, theme::WIDTH_SQUARE_BUTTON);
        assert_eq!(italic.size.width, theme::WIDTH_SQUARE_BUTTON);
        assert_eq!(underline.size.width, theme::WIDTH_SQUARE_BUTTON);
        assert_eq!(colour.left() - size.right(), theme::GAP_ITEMS);
        assert_eq!(bold.left() - colour.right(), theme::GAP_ITEMS);
        assert_eq!(italic.left() - bold.right(), theme::GAP_ITEMS);
        assert_eq!(underline.left() - italic.right(), theme::GAP_ITEMS);
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

/// The fixed font families of a window test: the popover does not need the
/// fonts of the computer.
fn fixed_fonts() -> Vec<opencraft_engine::label::FontFamily> {
    [
        ("Menlo", "Menlo"),
        ("Avenir Next", "Avenir Next"),
        ("Zapfino", "Zapfino"),
    ]
    .into_iter()
    .map(|(name, display)| opencraft_engine::label::FontFamily {
        name: name.into(),
        display: display.into(),
    })
    .collect()
}

/// Give the picker the families of the test.
fn use_fixed_fonts(view: &gpui_kit::Entity<DeviceView>, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.inspector.font_picker.use_fixed_families(fixed_fonts())
        })
    });
}

/// The names of the families of the open popover, without "System font".
fn shown_families(view: &gpui_kit::Entity<DeviceView>, cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, cx| view.read(cx).inspector.font_picker.shown_families())
}

#[gpui_kit::test]
fn the_font_row_sets_a_family_and_the_system_font(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    use_fixed_fonts(&view, cx);
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("font-picker").is_none());
    click(cx, "label-font");
    assert!(cx.debug_bounds("font-picker").is_some(), "the popover");
    // The first row is the system font, and then the families.
    assert_eq!(
        shown_families(&view, cx),
        ["Menlo", "Avenir Next", "Zapfino"]
    );

    // A click on a family sends its stored name.
    click(cx, "font-row-1");
    assert!(cx.debug_bounds("font-picker").is_none());
    assert_eq!(
        changes(&sent, key()),
        [LabelChange::Font(Some(Auto::Value("Menlo".into())))]
    );

    // The profile of the surface puts the check on its family.
    send(
        cx,
        &tx,
        styled(
            key(),
            LabelStyle {
                font: Some(Auto::Value("Menlo".into())),
                ..LabelStyle::default()
            },
            false,
        ),
    );
    click(cx, "label-font");
    // The check mark of row 1 is drawn: the row exists.
    assert!(cx.debug_bounds("font-row-1").is_some());
    // Escape closes the popover with no change.
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("font-picker").is_none());

    // The row "System font" makes the automatic font, not an inherited one.
    sent.borrow_mut().clear();
    click(cx, "label-font");
    click(cx, "font-row-0");
    assert_eq!(changes(&sent, key()), [LabelChange::Font(Some(Auto::Auto))]);
}

#[gpui_kit::test]
fn a_click_on_the_checked_font_row_sends_no_command(cx: &mut TestAppContext) {
    let (view, cx, tx, sent) = window(cx);
    use_fixed_fonts(&view, cx);
    click(cx, "surface-key face 2");
    // The default surface inherits: "System font" has the check mark.
    click(cx, "label-font");
    click(cx, "font-row-0");
    assert!(
        sent.borrow().is_empty(),
        "a click on the checked row sent {:?}",
        sent.borrow()
    );
    assert!(
        cx.debug_bounds("font-picker").is_none(),
        "the popover closes"
    );

    // A family that the surface has is also checked, thus a click on it
    // changes nothing (review 2, M4).
    send(
        cx,
        &tx,
        styled(
            key(),
            LabelStyle {
                font: Some(Auto::Value("Menlo".into())),
                ..LabelStyle::default()
            },
            false,
        ),
    );
    click(cx, "label-font");
    click(cx, "font-row-1");
    assert!(
        sent.borrow().is_empty(),
        "a click on the checked family sent {:?}",
        sent.borrow()
    );
    assert!(
        cx.debug_bounds("font-picker").is_none(),
        "the popover closes"
    );
}

#[gpui_kit::test]
fn the_font_search_finds_a_family_and_the_keys_choose_a_row(cx: &mut TestAppContext) {
    let (view, cx, _tx, sent) = window(cx);
    use_fixed_fonts(&view, cx);
    click(cx, "surface-bar panel 3");
    click(cx, "label-font");
    // Down moves the highlight: the system font, Menlo, Avenir Next.
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(cx.debug_bounds("font-picker").is_none());
    assert_eq!(
        changes(&sent, bar()),
        [LabelChange::Font(Some(Auto::Value("Avenir Next".into())))]
    );

    // The search field has the focus, and finds a part of the name.
    sent.borrow_mut().clear();
    click(cx, "label-font");
    cx.simulate_input("av");
    cx.run_until_parked();
    assert_eq!(shown_families(&view, cx), ["Avenir Next"]);
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        changes(&sent, bar()),
        [LabelChange::Font(Some(Auto::Value("Avenir Next".into())))]
    );

    // A text that matches no family shows the empty state.
    click(cx, "label-font");
    cx.simulate_input("zzzz");
    cx.run_until_parked();
    assert!(cx.debug_bounds("font-empty").is_some());
    assert!(shown_families(&view, cx).is_empty());

    // Escape closes the popover, and the next opening has an empty field.
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds("font-picker").is_none());
    click(cx, "label-font");
    assert_eq!(shown_families(&view, cx).len(), 3);
}

/// Many families, thus the list is longer than the popover.
fn many_fonts(count: usize) -> Vec<opencraft_engine::label::FontFamily> {
    (0..count)
        .map(|index| opencraft_engine::label::FontFamily {
            name: format!("Family {index:02}"),
            display: format!("Family {index:02}"),
        })
        .collect()
}

/// The picker shows the row of the current family when it opens, also when
/// the family is below the last row of the popover (issue 480).
#[gpui_kit::test]
fn the_font_picker_shows_the_row_of_the_current_family(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = window(cx);
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.inspector
                .font_picker
                .use_fixed_families(many_fonts(40))
        })
    });
    click(cx, "surface-key face 2");
    // The family 30 is below the 10 rows that the popover shows.
    send(
        cx,
        &tx,
        styled(
            key(),
            LabelStyle {
                font: Some(Auto::Value("Family 30".into())),
                ..LabelStyle::default()
            },
            false,
        ),
    );
    click(cx, "label-font");
    let popover = cx.debug_bounds("font-picker").expect("the popover");
    // The rows are "System font" and then the families, thus the row 31.
    let row = cx
        .debug_bounds("font-row-31")
        .expect("the row of the current family shows");
    assert!(
        row.top() >= popover.top() && row.bottom() <= popover.bottom(),
        "the row {row:?} is in the popover {popover:?}"
    );
}

#[gpui_kit::test]
fn a_family_that_is_not_installed_shows_the_hint(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = window(cx);
    use_fixed_fonts(&view, cx);
    send(
        cx,
        &tx,
        styled(
            key(),
            LabelStyle {
                font: Some(Auto::Value("Futura X".into())),
                ..LabelStyle::default()
            },
            false,
        ),
    );
    click(cx, "surface-key face 2");
    assert!(cx.debug_bounds("label-font-missing").is_some(), "the hint");
    // An installed family has no hint.
    send(
        cx,
        &tx,
        styled(
            key(),
            LabelStyle {
                font: Some(Auto::Value("Menlo".into())),
                ..LabelStyle::default()
            },
            false,
        ),
    );
    assert!(cx.debug_bounds("label-font-missing").is_none());
    // The system font has no hint.
    send(cx, &tx, styled(key(), LabelStyle::default(), false));
    assert!(cx.debug_bounds("label-font-missing").is_none());
}

/// The first frame of a first open shows the loading state, and the rows
/// come when the background scan of Core Text finishes (review 2, H3).
#[cfg(target_os = "macos")]
#[gpui_kit::test]
fn the_font_picker_shows_loading_before_the_list_arrives(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = window(cx);
    click(cx, "surface-key face 2");
    // Open without the test platform's `run_until_parked`: the first frame
    // is before the background scan finishes.
    cx.update(|window, cx| view.update(cx, |view, cx| view.open_font_picker(window, cx)));
    cx.update(|window, _| window.refresh());
    assert!(
        cx.debug_bounds("font-empty").is_some(),
        "the loading state of the first frame"
    );
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("font-empty").is_none(),
        "the rows are ready"
    );
    assert!(!shown_families(&view, cx).is_empty());
}

/// The hint of a family that is not installed comes with the list, not
/// before it: a named family in the profile is not missing while the first
/// scan runs (review 3, M4).
#[cfg(target_os = "macos")]
#[gpui_kit::test]
fn a_family_is_missing_only_when_the_list_is_there(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = window(cx);
    // The scan of the families runs, and it gives no list in these frames.
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.inspector.font_picker.use_a_scan_that_runs()
        })
    });
    click(cx, "surface-key face 2");
    send(
        cx,
        &tx,
        styled(
            key(),
            LabelStyle {
                font: Some(Auto::Value("Menlo".into())),
                ..LabelStyle::default()
            },
            false,
        ),
    );
    assert!(
        cx.debug_bounds("label-font-missing").is_none(),
        "no list yet: the family is not missing"
    );
    // The scan gives the list, and it has the family.
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            let picker = &view.inspector.font_picker;
            picker.finish_scan(1, fixed_fonts());
        })
    });
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("label-font-missing").is_none(),
        "Menlo is installed"
    );
    // A family that the computer does not have shows the hint.
    tx.try_send(styled(
        key(),
        LabelStyle {
            font: Some(Auto::Value("Futura X".into())),
            ..LabelStyle::default()
        },
        false,
    ))
    .unwrap();
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("label-font-missing").is_some(),
        "a family that is not installed shows the hint"
    );
}

/// A scan that gives the list of now keeps the highlight that the user
/// moved: only the first list puts the highlight on the current family
/// (review 3, M4).
#[cfg(target_os = "macos")]
#[gpui_kit::test]
fn a_refreshed_list_keeps_the_highlight_of_the_font_picker(cx: &mut TestAppContext) {
    let (view, cx, tx, _sent) = window(cx);
    send(
        cx,
        &tx,
        styled(
            key(),
            LabelStyle {
                font: Some(Auto::Value("Menlo".into())),
                ..LabelStyle::default()
            },
            false,
        ),
    );
    click(cx, "surface-key face 2");
    // The first open scans the families.
    click(cx, "label-font");
    cx.run_until_parked();
    // The user moves the highlight of the open picker.
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("down");
    let moved = highlighted_font_row(cx).expect("a highlighted row");
    // A scan gives the list of now while the popover stays open: it keeps
    // the highlight of the user. The open of the picker starts the same
    // scan.
    cx.update(|window, cx| {
        view.update(cx, |view, cx| view.start_font_scan(window, cx));
    });
    cx.run_until_parked();
    assert_eq!(
        highlighted_font_row(cx).as_deref(),
        Some(moved.as_str()),
        "the refresh keeps the highlight of the user"
    );
}

/// The highlighted row of the font picker in a new frame, or `None` when no
/// row is highlighted.
#[cfg(target_os = "macos")]
fn highlighted_font_row(cx: &mut VisualTestContext) -> Option<String> {
    crate::view::controls::accessibility_log::take_selected();
    cx.update(|window, _| window.refresh());
    crate::view::controls::accessibility_log::take_selected()
        .into_iter()
        .find_map(|(row, selected)| (selected && row.starts_with("font-row-")).then_some(row))
}

/// The families of the computer: the engine gives the list, and the picker
/// shows it. The list of another operating system is empty.
#[cfg(target_os = "macos")]
#[gpui_kit::test]
fn the_font_picker_shows_the_families_of_the_computer(cx: &mut TestAppContext) {
    let (view, cx, _tx, _sent) = window(cx);
    click(cx, "surface-key face 2");
    click(cx, "label-font");
    let shown = shown_families(&view, cx);
    assert!(
        shown.iter().any(|name| name == "Menlo"),
        "Menlo is installed"
    );
    assert!(shown.len() > 100, "the system has many families");
    // The search finds a family by a part of its name.
    cx.simulate_input("menlo");
    cx.run_until_parked();
    assert_eq!(shown_families(&view, cx), ["Menlo"]);
}

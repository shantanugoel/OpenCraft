//! Page tests: the commands for pages, the page change, and the request of a
//! handler to select a page. The tests use the fake transport and no thread.

use super::*;
use opencraft_engine::{EngineRequest, Page, PageTarget};

const KIND: &str = "test.pages";

/// The binding of the press of key 3 on `page`.
fn key_3(page: usize) -> BindingId {
    BindingId {
        page,
        ..binding(ControlKind::Key, 3, GestureKind::Press)
    }
}

/// A page with `color` on key face 3, and a binding for the press of key 3.
fn page(name: &str, color: Color) -> Page {
    let mut page = Page::new(name);
    page.update(KEY_3, |c| c.color = Some(color));
    let id = key_3(0);
    page.set_binding(id.control, id.gesture, Some(action(KIND)));
    page
}

/// An engine on a saved profile with `pages`, and the requests of its one
/// handler. The events and the requests of the start are gone.
fn rig_with(pages: Vec<Page>) -> (Rig, mpsc::Receiver<Request>) {
    let dir = TempDir::new().unwrap();
    store_in(dir.path())
        .save(&Profile::new("Pages", pages))
        .unwrap();
    let (port, requests) = port(KIND);
    let mut rig = Rig::with_ports(dir, vec![port]);
    rig.take_events();
    requests.try_iter().count();
    (rig, requests)
}

fn pages(names: &[&str], active: usize) -> Event {
    Event::Pages {
        names: names.iter().map(|n| n.to_string()).collect(),
        active,
        first: 0,
    }
}

fn appear(binding: BindingId) -> Request {
    Request::Appear {
        binding,
        action: action(KIND),
    }
}

fn disappear(binding: BindingId) -> Request {
    Request::Disappear { binding }
}

/// The bindings of the binding states in `events`, in order.
pub(super) fn binding_states(events: &[Event]) -> Vec<BindingId> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Binding(state) => Some(state.binding),
            _ => None,
        })
        .collect()
}

/// The slots of the images in `sent`, in order.
pub(super) fn image_slots(sent: &[Sent]) -> Vec<u8> {
    sent.iter()
        .filter_map(|s| match s {
            Sent::Image { slot, .. } => Some(*slot),
            Sent::Op(_) | Sent::Background { .. } => None,
        })
        .collect()
}

fn command(rig: &mut Rig, command: Command) {
    assert!(rig.engine.command(command));
}

// --- The start and the selection of a page ---------------------------------

#[test]
fn the_engine_starts_on_the_first_page() {
    let dir = TempDir::new().unwrap();
    store_in(dir.path())
        .save(&Profile::new(
            "Pages",
            vec![page("A", RED), page("B", BLUE)],
        ))
        .unwrap();
    let (port, requests) = port(KIND);
    let mut rig = Rig::with_ports(dir, vec![port]);

    assert_eq!(rig.engine.active_page(), 0);
    let events = rig.take_events();
    assert!(events.contains(&pages(&["A", "B"], 0)));
    assert_eq!(binding_states(&events), [key_3(0)]);
    assert_color(&last_state(&events, KEY_3).image, 2, 2, RED);
    assert_eq!(requests.try_iter().collect::<Vec<_>>(), [appear(key_3(0))]);
}

/// One thing that a page change does, as a test sees it.
#[derive(Debug, PartialEq)]
pub(super) enum Step {
    Request(Request),
    Write,
}

#[test]
fn a_page_change_sends_disappear_then_writes_then_sends_appear() {
    let (mut rig, requests) = rig_with(vec![page("A", RED), page("B", BLUE)]);
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    // Put each request and each write in one list, in the order of the
    // engine.
    let requests = Rc::new(requests);
    let steps = Rc::new(RefCell::new(Vec::new()));
    let (queue, list) = (Rc::clone(&requests), Rc::clone(&steps));
    *rig.world.before_write.borrow_mut() = Some(Box::new(move || {
        let mut list = list.borrow_mut();
        list.extend(queue.try_iter().map(Step::Request));
        list.push(Step::Write);
    }));
    command(&mut rig, Command::SelectPage { page: 1 });
    *rig.world.before_write.borrow_mut() = None;
    let mut steps = steps.take();
    steps.extend(requests.try_iter().map(Step::Request));
    steps.dedup();
    assert_eq!(
        steps,
        [
            Step::Request(disappear(key_3(0))),
            Step::Write,
            Step::Request(appear(key_3(1))),
        ]
    );
    assert_eq!(rig.engine.active_page(), 1);

    // Only key face 3 is different. No start sequence.
    let sent = sent(&device);
    assert_eq!(image_slots(&sent), [KEY_3_SLOT]);
    assert_eq!(sent.last(), Some(&op("STP")));
    assert_eq!(sent.len(), 2, "{sent:?}");
    assert_color(&shown_at(&sent, KEY_3_SLOT), 2, 2, BLUE);

    // The shell gets each surface, then the list of pages, then the
    // bindings of the new page.
    let events = rig.take_events();
    assert_eq!(surfaces(&events).len(), 14);
    let at = events.iter().position(|e| *e == pages(&["A", "B"], 1));
    assert_eq!(at, Some(14), "{events:?}");
    assert_eq!(binding_states(&events[15..]), [key_3(1)]);
    assert_eq!(events.len(), 16);
}

#[test]
fn a_page_change_writes_only_the_images_that_are_different() {
    let mut second = page("B", RED);
    second.update(BAR_2, |c| c.color = Some(BLUE));
    let (mut rig, _requests) = rig_with(vec![page("A", RED), second]);
    let device = rig.connect();
    sent(&device);

    for page in [1, 0] {
        command(&mut rig, Command::SelectPage { page });
        let sent = sent(&device);
        assert_eq!(image_slots(&sent), [2], "only bar panel 2: {sent:?}");
        assert_eq!(sent.len(), 2, "one image and the commit");
        let color = if page == 1 { BLUE } else { Color::BLACK };
        assert_color(&shown_at(&sent, 2), 2, 2, color);
    }

    // A copy of the page shows the same images: the engine writes nothing.
    command(&mut rig, Command::AddPage { copy_of: Some(0) });
    assert_eq!(rig.engine.active_page(), 2);
    assert!(sent(&device).is_empty());
}

#[test]
fn a_select_of_the_active_page_does_nothing_and_a_missing_page_is_a_problem() {
    let (mut rig, requests) = rig_with(vec![page("A", RED), page("B", BLUE)]);
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    command(&mut rig, Command::SelectPage { page: 0 });
    assert!(rig.take_events().is_empty());
    command(&mut rig, Command::SelectPage { page: 2 });
    assert_eq!(problems(&rig.take_events()).len(), 1);
    assert!(sent(&device).is_empty());
    assert!(requests.try_recv().is_err());
    assert_eq!(rig.engine.active_page(), 0);
}

#[test]
fn feedback_is_gone_after_a_page_change() {
    let knob = binding(ControlKind::Knob, 2, GestureKind::TurnRight);
    let mut first = page("A", RED);
    first.set_binding(knob.control, knob.gesture, Some(action(KIND)));
    let second = first.clone();
    let (mut rig, _requests) = rig_with(vec![first, second]);
    let device = rig.connect();
    let is_blank = |image: &RgbImage| image.pixels().all(|p| *p == Rgb([0, 0, 0]));
    rig.engine.handler_message(Message::Feedback {
        binding: knob,
        feedback: Some(Feedback {
            label: Some("Volume".into()),
            level: Some(40),
            state: None,
            other_state: None,
        }),
    });
    assert!(!is_blank(&last_state(&rig.take_events(), BAR_2).image));
    sent(&device);

    command(&mut rig, Command::SelectPage { page: 1 });
    assert!(is_blank(&last_state(&rig.take_events(), BAR_2).image));
    assert_eq!(image_slots(&sent(&device)), [2]);

    // Feedback for the old page number does nothing.
    rig.engine.handler_message(Message::Feedback {
        binding: knob,
        feedback: Some(Feedback::default()),
    });
    assert!(rig.take_events().is_empty());

    // The feedback does not come back with the old page.
    command(&mut rig, Command::SelectPage { page: 0 });
    assert!(is_blank(&last_state(&rig.take_events(), BAR_2).image));
    assert!(image_slots(&sent(&device)).is_empty());
}

#[test]
fn a_failed_write_at_a_page_change_draws_all_again() {
    let (mut rig, _requests) = rig_with(vec![page("A", RED), page("B", BLUE)]);
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    device.borrow_mut().fail_writes("the write failed");
    command(&mut rig, Command::SelectPage { page: 1 });
    rig.engine.poll(POLL_INTERVAL);
    device.borrow_mut().clear_errors();
    rig.poll_for(Duration::from_secs(1));

    assert_eq!(problems(&rig.take_events()).len(), 1);
    let sent = sent(&device);
    assert_full_draw(&sent);
    assert_color(&shown_at(&sent, KEY_3_SLOT), 2, 2, BLUE);
}

// --- The request of a handler ------------------------------------------------

fn select(rig: &mut Rig, target: PageTarget) {
    rig.engine
        .handler_message(Message::Engine(EngineRequest::SelectPage(target)));
}

#[test]
fn a_handler_selects_the_next_and_the_previous_page_with_wrap_around() {
    let (mut rig, requests) = rig_with(vec![page("A", RED), page("B", BLUE), page("C", GREEN)]);
    for (target, want) in [
        (PageTarget::Next, 1),
        (PageTarget::Next, 2),
        (PageTarget::Next, 0),
        (PageTarget::Previous, 2),
        (PageTarget::Previous, 1),
        (PageTarget::Number(0), 0),
        (PageTarget::Number(2), 2),
    ] {
        let before = rig.engine.active_page();
        select(&mut rig, target);
        assert_eq!(rig.engine.active_page(), want, "{target:?}");
        assert_eq!(
            requests.try_iter().collect::<Vec<_>>(),
            [disappear(key_3(before)), appear(key_3(want))],
            "{target:?}"
        );
        assert!(rig.take_events().contains(&pages(&["A", "B", "C"], want)));
    }

    // A number that is not a page is a problem, as for the shell.
    select(&mut rig, PageTarget::Number(3));
    assert_eq!(problems(&rig.take_events()).len(), 1);
    assert_eq!(rig.engine.active_page(), 2);
    assert!(requests.try_recv().is_err());
}

#[test]
fn with_one_page_the_next_page_is_the_same_page() {
    let (mut rig, requests) = rig_with(vec![page("A", RED)]);
    select(&mut rig, PageTarget::Next);
    select(&mut rig, PageTarget::Previous);
    assert!(rig.take_events().is_empty());
    assert!(requests.try_recv().is_err());
}

// --- Add, rename, move, and remove ---------------------------------------------

#[test]
fn an_added_page_is_empty_or_a_copy_and_becomes_the_active_page() {
    let (mut rig, requests) = rig_with(vec![page("A", RED)]);
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    command(&mut rig, Command::AddPage { copy_of: None });
    assert_eq!(rig.engine.active_page(), 1);
    let events = rig.take_events();
    assert!(events.contains(&pages(&["A", "Page 2"], 1)));
    assert!(binding_states(&events).is_empty());
    assert!(last_state(&events, KEY_3).content.is_empty());
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(key_3(0))]
    );
    assert_eq!(image_slots(&sent(&device)), [KEY_3_SLOT]);

    command(&mut rig, Command::AddPage { copy_of: Some(0) });
    assert_eq!(rig.engine.active_page(), 2);
    let events = rig.take_events();
    assert!(events.contains(&pages(&["A", "Page 2", "Page 3"], 2)));
    assert_eq!(binding_states(&events), [key_3(2)]);
    assert_eq!(last_state(&events, KEY_3).content.color, Some(RED));
    assert_eq!(requests.try_iter().collect::<Vec<_>>(), [appear(key_3(2))]);

    command(&mut rig, Command::AddPage { copy_of: Some(3) });
    assert_eq!(problems(&rig.take_events()).len(), 1);
    assert_eq!(rig.engine.profile().pages().len(), 3);

    // The delayed save writes the new pages.
    rig.poll_for(Duration::from_secs(1));
    let text = rig.profile_text().unwrap();
    assert_eq!(text.matches("[[pages]]").count(), 3, "{text}");
}

#[test]
fn a_rename_sends_the_list_of_pages_and_the_bindings_again() {
    let (mut rig, requests) = rig_with(vec![page("A", RED), page("B", BLUE)]);
    let device = rig.connect();
    sent(&device);
    rig.take_events();

    command(
        &mut rig,
        Command::RenamePage {
            page: 1,
            name: "Media".into(),
        },
    );
    let events = rig.take_events();
    assert_eq!(events[0], pages(&["A", "Media"], 0));
    assert_eq!(binding_states(&events), [key_3(0)]);
    assert!(matches!(events[2], Event::History { .. }));
    assert_eq!(events.len(), 3);
    assert!(requests.try_recv().is_err());
    assert!(sent(&device).is_empty());

    // The same name, an empty name, and a missing page change nothing.
    let rename = |page: usize, name: &str| Command::RenamePage {
        page,
        name: name.into(),
    };
    command(&mut rig, rename(1, "Media"));
    assert!(rig.take_events().is_empty());
    command(&mut rig, rename(1, "  "));
    command(&mut rig, rename(2, "Other"));
    assert_eq!(problems(&rig.take_events()).len(), 2);

    rig.poll_for(Duration::from_secs(1));
    assert!(rig.profile_text().unwrap().contains("name = \"Media\""));
}

/// An engine on the pages A, B, and C, with C active.
fn three_pages_with_the_last_active() -> (Rig, mpsc::Receiver<Request>, Rc<RefCell<FakeTransport>>)
{
    let (mut rig, requests) = rig_with(vec![page("A", RED), page("B", BLUE), page("C", GREEN)]);
    let device = rig.connect();
    command(&mut rig, Command::SelectPage { page: 2 });
    sent(&device);
    rig.take_events();
    requests.try_iter().count();
    (rig, requests, device)
}

#[test]
fn a_move_that_changes_the_number_of_the_active_page_gives_new_numbers() {
    let (mut rig, requests, device) = three_pages_with_the_last_active();
    let move_page = |rig: &mut Rig, page, to| command(rig, Command::MovePage { page, to });

    // A moves after C: C is now page 1.
    move_page(&mut rig, 0, 2);
    assert_eq!(rig.engine.active_page(), 1);
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(key_3(2)), appear(key_3(1))]
    );
    let events = rig.take_events();
    assert!(events.contains(&pages(&["B", "C", "A"], 1)));
    assert_eq!(binding_states(&events), [key_3(1)]);
    assert!(sent(&device).is_empty(), "the device shows the same page");

    // A moves before C again: C is page 2.
    move_page(&mut rig, 2, 0);
    assert_eq!(rig.engine.active_page(), 2);
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(key_3(1)), appear(key_3(2))]
    );
    assert!(rig.take_events().contains(&pages(&["A", "B", "C"], 2)));

    // A move of two pages before C: C keeps its number.
    move_page(&mut rig, 0, 1);
    assert_eq!(rig.engine.active_page(), 2);
    assert!(requests.try_recv().is_err());
    assert_eq!(
        rig.take_events(),
        [
            pages(&["B", "A", "C"], 2),
            Event::Binding(BindingState {
                binding: key_3(2),
                action: Some(action(KIND)),
                available: true,
            }),
            Event::History {
                undo: Some("Move Page".into()),
                redo: None
            },
        ]
    );

    // A move of the active page itself.
    move_page(&mut rig, 2, 0);
    assert_eq!(rig.engine.active_page(), 0);
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(key_3(2)), appear(key_3(0))]
    );
    assert!(rig.take_events().contains(&pages(&["C", "B", "A"], 0)));
    assert!(sent(&device).is_empty());

    // The same position does nothing. A missing page is a problem.
    move_page(&mut rig, 1, 1);
    assert!(rig.take_events().is_empty());
    move_page(&mut rig, 0, 3);
    assert_eq!(problems(&rig.take_events()).len(), 1);
    assert_eq!(rig.engine.active_page(), 0);
}

#[test]
fn a_removal_before_the_active_page_gives_new_numbers() {
    let (mut rig, requests, device) = three_pages_with_the_last_active();

    // A removal after the active page sends only the list of pages.
    command(&mut rig, Command::SelectPage { page: 1 });
    requests.try_iter().count();
    sent(&device);
    rig.take_events();
    command(&mut rig, Command::RemovePage { page: 2 });
    assert_eq!(rig.engine.active_page(), 1);
    assert!(requests.try_recv().is_err());
    assert_eq!(rig.take_events()[0], pages(&["A", "B"], 1));

    // A removal before the active page: B is now page 0.
    command(&mut rig, Command::RemovePage { page: 0 });
    assert_eq!(rig.engine.active_page(), 0);
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(key_3(1)), appear(key_3(0))]
    );
    let events = rig.take_events();
    assert!(events.contains(&pages(&["B"], 0)));
    assert_eq!(binding_states(&events), [key_3(0)]);
    assert!(sent(&device).is_empty(), "the device shows the same page");

    // A profile keeps one page.
    command(&mut rig, Command::RemovePage { page: 0 });
    command(&mut rig, Command::RemovePage { page: 1 });
    assert_eq!(problems(&rig.take_events()).len(), 2);
    assert_eq!(rig.engine.profile().pages().len(), 1);
}

#[test]
fn a_removal_of_the_active_page_shows_the_next_page_or_the_page_before() {
    let (mut rig, requests, device) = three_pages_with_the_last_active();

    // C is the last page: B before it becomes the active page.
    command(&mut rig, Command::RemovePage { page: 2 });
    assert_eq!(rig.engine.active_page(), 1);
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(key_3(2)), appear(key_3(1))]
    );
    assert!(rig.take_events().contains(&pages(&["A", "B"], 1)));
    let sent_now = sent(&device);
    assert_eq!(image_slots(&sent_now), [KEY_3_SLOT]);
    assert_color(&shown_at(&sent_now, KEY_3_SLOT), 2, 2, BLUE);

    // A is not the last page: B after it becomes the active page, with the
    // same number as A.
    command(&mut rig, Command::SelectPage { page: 0 });
    requests.try_iter().count();
    sent(&device);
    rig.take_events();
    command(&mut rig, Command::RemovePage { page: 0 });
    assert_eq!(rig.engine.active_page(), 0);
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(key_3(0)), appear(key_3(0))]
    );
    let events = rig.take_events();
    assert!(events.contains(&pages(&["B"], 0)));
    assert_eq!(binding_states(&events), [key_3(0)]);
    assert_color(&shown_at(&sent(&device), KEY_3_SLOT), 2, 2, BLUE);
}

// --- The commands for the active page, and the file ------------------------

#[test]
fn surface_and_binding_commands_change_the_active_page() {
    let (mut rig, requests) = rig_with(vec![page("A", RED), page("B", BLUE)]);
    command(&mut rig, Command::SelectPage { page: 1 });
    requests.try_iter().count();
    rig.take_events();

    rig.set_color(KEY_3, GREEN);
    set_binding(&mut rig, key_3(1), None);
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [disappear(key_3(1))]
    );

    let profile = rig.engine.profile();
    assert_eq!(profile.pages()[0].content(KEY_3).color, Some(RED));
    assert_eq!(profile.pages()[1].content(KEY_3).color, Some(GREEN));
    assert!(profile.pages()[0]
        .binding(key_3(0).control, key_3(0).gesture)
        .is_some());
    assert!(profile.pages()[1]
        .binding(key_3(1).control, key_3(1).gesture)
        .is_none());

    // A key press invokes the binding of the active page.
    set_binding(&mut rig, key_3(1), Some(action(KIND)));
    requests.try_iter().count();
    let device = rig.connect();
    device.borrow_mut().push_input(control_report(3, 1));
    rig.engine.poll(POLL_INTERVAL);
    assert_eq!(
        requests.try_iter().collect::<Vec<_>>(),
        [Request::Invoke {
            binding: key_3(1),
            action: action(KIND)
        }]
    );
}

#[test]
fn many_pages_go_to_the_file_and_the_engine_starts_again_on_the_first_page() {
    let mut rig = Rig::new();
    rig.set_color(KEY_3, RED);
    command(&mut rig, Command::AddPage { copy_of: None });
    rig.set_color(KEY_3, BLUE);
    set_binding(&mut rig, key_3(1), Some(action(KIND)));
    command(
        &mut rig,
        Command::RenamePage {
            page: 1,
            name: "Media".into(),
        },
    );
    rig.poll_for(Duration::from_secs(1));

    let mut rig = rig.restart();
    assert_eq!(rig.engine.active_page(), 0);
    let events = rig.take_events();
    assert!(problems(&events).is_empty(), "{events:?}");
    assert!(events.contains(&pages(&["Page 1", "Media"], 0)));
    assert_color(&last_state(&events, KEY_3).image, 2, 2, RED);
    let second = &rig.engine.profile().pages()[1];
    assert_eq!(second.content(KEY_3).color, Some(BLUE));
    assert_eq!(
        second.binding(key_3(1).control, key_3(1).gesture),
        Some(&action(KIND))
    );
}

#[test]
fn a_copy_of_a_page_keeps_its_images_after_the_original_page_is_gone() {
    let mut rig = Rig::new();
    let file = green_file(rig.dir.path());
    command(
        &mut rig,
        Command::SetImage {
            surface: KEY_3,
            path: Some(file),
        },
    );
    command(&mut rig, Command::AddPage { copy_of: Some(0) });
    command(&mut rig, Command::RemovePage { page: 0 });
    rig.poll_for(Duration::from_secs(1));

    // The start removes only the images that no page uses.
    let rig = rig.restart();
    let events = rig.events.borrow();
    assert!(problems(&events).is_empty(), "{events:?}");
    assert_color(&last_state(&events, KEY_3).image, 56, 56, GREEN);
    let images = store_in(rig.dir.path()).dir().join("images");
    assert_eq!(fs::read_dir(&images).unwrap().count(), 1);
}

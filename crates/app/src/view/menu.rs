//! The keys of the menus that the window draws (issue 217).
//!
//! [`controls::popup_menu`](super::controls::popup_menu) wraps its menu in
//! [`MenuKeys`], and [`controls::menu_row`](super::controls::menu_row) makes
//! a [`MenuRow`]. Thus each menu gets the keys with no change at its call
//! site:
//!
//! | Key | Action |
//! |---|---|
//! | Down, Up | Move the highlight to the next or the previous item. At the end, the highlight goes to the other end. |
//! | Home, End | Move the highlight to the first or the last item. |
//! | Return, Space | Select the highlighted item: its click handler runs. |
//! | Tab, Shift-Tab | Nothing: the focus stays on the popup button while its menu is open. |
//! | A letter or a digit | Move the highlight to the first item with a name that starts with the typed text. The text is cleared after one second. |
//!
//! The first item is highlighted when the menu opens. The highlight follows
//! the pointer. A group title and a separator are not items: only a
//! [`MenuRow`] is an item. The menu scrolls to keep the highlighted item
//! visible.
//!
//! Escape closes a menu in the handler of the part that owns the menu (the
//! workspace for the menus of the inspector, the sheet for its menu): these
//! handlers run in the key dispatch of GPUI, and a keystroke interceptor
//! cannot stop them.
//!
//! The keys come from a keystroke interceptor, because the focus is on the
//! workspace and not on the menu. The interceptor runs before the key
//! bindings and the key handlers, and it lives in the state of the
//! element, thus it stops when the menu is not drawn. It does nothing while
//! a text field has the focus, so that a menu with a field (a colour
//! popover) keeps the keys of the field. A menu with no [`MenuRow`] has no
//! keys at all.
//!
//! The menu element is also a focus trap: in a popover with controls (a
//! colour popover), Tab and Shift-Tab move the focus between the controls
//! of the popover only.

use super::{controls::Accessible, theme};
use gpui_kit::{
    component::WindowExt, fill, point, prelude::*, px, AnyElement, App, Bounds, ClickEvent,
    DispatchPhase, Div, Element, ElementId, FocusHandle, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, KeystrokeEvent, LayoutId, MouseMoveEvent, Pixels, Role, ScrollHandle,
    SharedString, Stateful, Subscription, Window,
};
use std::{any::Any, cell::RefCell, panic::Location, rc::Rc, time::Duration, time::Instant};

/// A click handler, as `on_click` takes it.
type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The time after which the typed text of a menu is cleared.
const TYPE_AHEAD: Duration = Duration::from_secs(1);

/// A row of a menu: the item that a key can select. It has the look of the
/// row of decision 0029, section 3.7, and the methods that the call sites
/// use: [`MenuRow::child`], [`MenuRow::on_click`], and `into_any_element`.
pub(super) struct MenuRow {
    frame: Stateful<Div>,
    label: Option<SharedString>,
    on_click: Option<Handler>,
}

impl MenuRow {
    pub(super) fn new(frame: Stateful<Div>) -> Self {
        Self {
            frame,
            label: None,
            on_click: None,
        }
    }

    /// Add a child. The last child that is a text (`&str`, `String`,
    /// `SharedString`, or the [`KindContent`](super::gesture_card::KindContent)
    /// of a kind)
    /// is the name of the item, for the typed text.
    pub(super) fn child<E: IntoElement + 'static>(mut self, child: E) -> Self {
        let any: &dyn Any = &child;
        let text = if let Some(text) = any.downcast_ref::<&'static str>() {
            Some(SharedString::from(*text))
        } else if let Some(text) = any.downcast_ref::<String>() {
            Some(SharedString::from(text.clone()))
        } else if let Some(text) = any.downcast_ref::<SharedString>() {
            Some(text.clone())
        } else {
            any.downcast_ref::<super::gesture_card::KindContent>()
                .map(|content| content.name().clone())
        };
        if text.is_some() {
            self.label = text;
        }
        self.frame = self.frame.child(child);
        self
    }

    /// The name of the item, for a row whose name is not a text child, for
    /// example a text in a `div` that truncates it.
    pub(super) fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// The function that runs at a click, and at Return on the highlighted
    /// item.
    pub(super) fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        let handler: Handler = Rc::new(listener);
        let for_click = handler.clone();
        self.frame = self
            .frame
            .on_click(move |event, window, cx| for_click(event, window, cx));
        self.on_click = Some(handler);
        self
    }
}

impl IntoElement for MenuRow {
    type Element = MenuItem;

    fn into_element(self) -> MenuItem {
        // The name of the item is also its accessible label.
        let frame = match &self.label {
            Some(label) => self.frame.accessible(Role::MenuItem, label.clone()),
            None => self.frame.role(Role::MenuItem),
        };
        MenuItem {
            child: frame.into_any_element(),
            label: self.label,
            on_click: self.on_click,
        }
    }
}

/// The element of a [`MenuRow`]. It tells its menu where it is, paints the
/// highlight under the row, and moves the highlight to the row when the
/// pointer moves over it.
pub(super) struct MenuItem {
    child: AnyElement,
    label: Option<SharedString>,
    on_click: Option<Handler>,
}

impl IntoElement for MenuItem {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for MenuItem {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        self.child.prepaint(window, cx);
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(nav) = CURRENT.with(|current| current.borrow().clone()) {
            let index = {
                let mut state = nav.borrow_mut();
                state.items.push(Item {
                    bounds,
                    label: self.label.clone(),
                    on_click: self.on_click.clone(),
                });
                state.items.len() - 1
            };
            if nav.borrow().highlighted == index {
                window.paint_quad(
                    fill(bounds, theme::MENU_HIGHLIGHT).corner_radii(theme::RADIUS_CONTROL),
                );
            }
            let hitbox = hitbox.clone();
            window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, _| {
                if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
                    let mut state = nav.borrow_mut();
                    if state.highlighted != index {
                        state.highlighted = index;
                        window.refresh();
                    }
                }
            });
        }
        self.child.paint(window, cx);
    }
}

/// One item of the menu in the last frame.
pub(super) struct Item {
    bounds: Bounds<Pixels>,
    label: Option<SharedString>,
    on_click: Option<Handler>,
}

/// The state of an open menu. It lives as long as the menu is drawn.
pub(super) struct Nav {
    /// The items of the last frame, in the order of the menu.
    items: Vec<Item>,
    /// The index of the highlighted item.
    highlighted: usize,
    scroll: ScrollHandle,
    typed: String,
    typed_at: Option<Instant>,
}

pub(super) type NavRef = Rc<RefCell<Nav>>;

thread_local! {
    /// The menu that is in its paint. A [`MenuItem`] reads it.
    static CURRENT: RefCell<Option<NavRef>> = const { RefCell::new(None) };
}

/// The element state of a [`MenuKeys`]. The subscription ends the
/// interceptor of the keys when the state drops.
struct MenuState {
    nav: NavRef,
    /// The focus handle of the focus trap of the menu.
    focus: FocusHandle,
    _keys: Subscription,
}

/// The function that makes the menu element from the scroll handle and the
/// focus handle of the focus trap.
type Builder = Box<dyn FnOnce(&ScrollHandle, &FocusHandle) -> AnyElement>;

/// The wrapper of the menu element: it keeps the state of the keys, makes
/// the scroll handle, and builds the menu with that handle.
pub(super) struct MenuKeys {
    id: ElementId,
    build: Option<Builder>,
    child: Option<AnyElement>,
}

impl MenuKeys {
    /// `build` makes the menu element. It must use the scroll handle for
    /// the scrolling element (`track_scroll`), and the focus handle for its
    /// focus trap.
    pub(super) fn new(
        id: impl Into<ElementId>,
        build: impl FnOnce(&ScrollHandle, &FocusHandle) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            build: Some(Box::new(build)),
            child: None,
        }
    }
}

impl IntoElement for MenuKeys {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for MenuKeys {
    type RequestLayoutState = NavRef;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, NavRef) {
        let id = id.expect("The menu has an element identifier");
        let handle = window.window_handle();
        let (nav, focus) = window.with_element_state(id, |state: Option<MenuState>, _| {
            let state = state.unwrap_or_else(|| {
                let nav: NavRef = Rc::new(RefCell::new(Nav {
                    items: Vec::new(),
                    highlighted: 0,
                    scroll: ScrollHandle::new(),
                    typed: String::new(),
                    typed_at: None,
                }));
                let keys = {
                    let nav = nav.clone();
                    cx.intercept_keystrokes(move |event, window, cx| {
                        if window.window_handle() == handle {
                            on_keystroke(&nav, event, window, cx);
                        }
                    })
                };
                MenuState {
                    nav,
                    focus: cx.focus_handle(),
                    _keys: keys,
                }
            });
            ((state.nav.clone(), state.focus.clone()), state)
        });
        let scroll = nav.borrow().scroll.clone();
        let mut child = (self.build.take().expect("The menu builds one time"))(&scroll, &focus);
        let layout = child.request_layout(window, cx);
        self.child = Some(child);
        (layout, nav)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut NavRef,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(child) = self.child.as_mut() {
            child.prepaint(window, cx);
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        nav: &mut NavRef,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        nav.borrow_mut().items.clear();
        let outer = CURRENT.with(|current| current.replace(Some(nav.clone())));
        if let Some(child) = self.child.as_mut() {
            child.paint(window, cx);
        }
        CURRENT.with(|current| *current.borrow_mut() = outer);
        let mut state = nav.borrow_mut();
        let count = state.items.len();
        if count > 0 && state.highlighted >= count {
            state.highlighted = count - 1;
        }
    }
}

/// The result of a key in a menu.
enum Move {
    To(usize),
    Select,
}

/// A key while a menu is open. The keys that the menu does not use go on to
/// the workspace.
fn on_keystroke(nav: &NavRef, event: &KeystrokeEvent, window: &mut Window, cx: &mut App) {
    let keystroke = &event.keystroke;
    let modifiers = keystroke.modifiers;
    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
        return;
    }
    if nav.borrow().items.is_empty() || window.focused_input(cx).is_some() {
        return;
    }
    let step = {
        let mut state = nav.borrow_mut();
        let count = state.items.len();
        let current = state.highlighted.min(count - 1);
        let typed = keystroke
            .key_char
            .as_deref()
            .filter(|text| text.chars().all(char::is_alphanumeric) && !text.is_empty());
        match (keystroke.key.as_str(), typed) {
            ("down", _) => Move::To((current + 1) % count),
            ("up", _) => Move::To((current + count - 1) % count),
            ("home", _) => Move::To(0),
            ("end", _) => Move::To(count - 1),
            ("enter" | "space", _) => Move::Select,
            ("tab", _) => Move::To(current),
            (_, Some(text)) => {
                let now = Instant::now();
                if state
                    .typed_at
                    .is_none_or(|at| now.duration_since(at) > TYPE_AHEAD)
                {
                    state.typed.clear();
                }
                state.typed.push_str(&text.to_lowercase());
                state.typed_at = Some(now);
                let typed = state.typed.clone();
                match state.items.iter().position(|item| {
                    item.label
                        .as_ref()
                        .is_some_and(|label| label.to_lowercase().starts_with(&typed))
                }) {
                    Some(index) => Move::To(index),
                    None => Move::To(current),
                }
            }
            _ => return,
        }
    };
    cx.stop_propagation();
    match step {
        Move::To(index) => {
            let mut state = nav.borrow_mut();
            state.highlighted = index;
            state.reveal();
            window.refresh();
        }
        Move::Select => {
            let handler = {
                let state = nav.borrow();
                state
                    .items
                    .get(state.highlighted)
                    .and_then(|item| item.on_click.clone())
            };
            // The handlers of the key dispatch run after this function, and
            // they see the menu as it is now. The item is selected after
            // them: the sheet does not close at the Return of a menu.
            if let Some(handler) = handler {
                window.defer(cx, move |window, cx| {
                    handler(&ClickEvent::default(), window, cx)
                });
            }
        }
    }
}

impl Nav {
    /// Scroll the menu so that the highlighted item is visible. The first
    /// item scrolls to the top, so that the title of its group shows, and
    /// the last item scrolls to the bottom.
    fn reveal(&self) {
        let Some(item) = self.items.get(self.highlighted) else {
            return;
        };
        let view = self.scroll.bounds();
        let max = self.scroll.max_offset().y;
        let mut y = self.scroll.offset().y;
        if self.highlighted == 0 {
            y = px(0.);
        } else if self.highlighted + 1 == self.items.len() {
            y = -max;
        } else if item.bounds.top() < view.top() + theme::SPACE_8 {
            y += view.top() + theme::SPACE_8 - item.bounds.top();
        } else if item.bounds.bottom() > view.bottom() - theme::SPACE_8 {
            y -= item.bounds.bottom() - (view.bottom() - theme::SPACE_8);
        }
        self.scroll.set_offset(point(px(0.), y.clamp(-max, px(0.))));
    }
}

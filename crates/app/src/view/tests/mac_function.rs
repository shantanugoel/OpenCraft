//! The actual kind description supplies the system function popup.
use super::*;

#[gpui_kit::test]
fn mac_functions_use_the_shared_choice_form(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = window(cx);
    let kind = opencraft_actions::handlers()
        .iter()
        .flat_map(|handler| handler.kinds())
        .find(|kind| kind.kind == "opencraft.mac-function")
        .unwrap();
    let action = Action {
        kind: kind.kind.clone(),
        settings: kind
            .fields
            .iter()
            .filter_map(|field| {
                field
                    .default
                    .clone()
                    .map(|value| (field.key.clone(), value))
            })
            .collect(),
    };
    tx.try_send(Event::Kinds(vec![kind])).unwrap();
    tx.try_send(Event::Binding(BindingState {
        binding: BindingId {
            page: 0,
            control: ControlId::new(ControlKind::Key, 2),
            gesture: GestureKind::Press,
        },
        action: Some(action),
        available: true,
    }))
    .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    click(cx, "choice-0");
    for id in [
        "choice-0-0",
        "choice-0-1",
        "choice-0-2",
        "choice-0-3",
        "choice-0-4",
        "choice-0-5",
        "choice-0-6",
    ] {
        assert!(cx.debug_bounds(id).is_some());
    }
}

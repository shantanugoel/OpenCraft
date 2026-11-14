//! The mouse kind uses the same description-driven form as other kinds.
use super::*;

#[gpui_kit::test]
fn mouse_form_comes_from_its_description(cx: &mut TestAppContext) {
    let (_view, cx, tx, _sent) = window(cx);
    let kind = opencraft_actions::handlers()
        .iter()
        .flat_map(|handler| handler.kinds())
        .find(|kind| kind.kind == "opencraft.mouse")
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
            folder: None,
        },
        action: Some(action),
        available: true,
    }))
    .unwrap();
    cx.run_until_parked();
    click(cx, "surface-key face 2");
    // Click uses the button. It does not use X, Y, or the scroll step.
    for id in ["choice-0", "choice-1"] {
        assert!(cx.debug_bounds(id).is_some(), "{id}");
    }
    for id in ["field-2", "field-3", "field-4"] {
        assert!(cx.debug_bounds(id).is_none(), "{id} is hidden");
    }
}

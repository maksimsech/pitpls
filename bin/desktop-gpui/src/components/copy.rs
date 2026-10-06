use crate::theme::palette;
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme, Sizable,
        button::{Button, ButtonCustomVariant, ButtonVariants},
    },
    prelude::FluentBuilder,
    *,
};
use std::time::Duration;

/// Copies `value` and then shows "✓ Copied" for a moment. The tooltip says
/// exactly what is copied.
///
/// The kit's `Clipboard` only swaps its icon for a check and has no way to
/// add the "Copied" label or its colour, so this is a kit `Button` with the
/// same behaviour.
#[derive(IntoElement)]
pub struct CopyButton {
    id: ElementId,
    value: SharedString,
}

impl CopyButton {
    pub fn new(id: impl Into<ElementId>, value: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
        }
    }
}

#[derive(Default)]
struct CopyState {
    copied: bool,
    /// Replacing it restarts the "Copied" time after another click.
    _reset: Option<Task<()>>,
}

impl RenderOnce for CopyButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| CopyState::default());
        let copied = state.read(cx).copied;
        let p = palette(cx);
        // Faint at rest and full text colour on hover; "Copied" stays green.
        let variant = ButtonCustomVariant::new(cx)
            .foreground(if copied { p.ok } else { p.text })
            .hover(cx.theme().button_hover)
            .active(cx.theme().button_active);
        let faint = p.faint;
        let tooltip = SharedString::from(format!("Copy {}", self.value));
        let value = self.value;
        Button::new(self.id)
            .custom(variant)
            .small()
            .icon(if copied {
                IconName::Check
            } else {
                IconName::Copy
            })
            .when(copied, |button| button.label("Copied"))
            .when(!copied, |button| button.text_color(faint))
            .tooltip(tooltip.clone())
            .accessibility_label(tooltip)
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                cx.write_to_clipboard(ClipboardItem::new_string(value.to_string()));
                state.update(cx, |state, cx| {
                    state.copied = true;
                    state._reset = Some(cx.spawn(async move |state, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(1600))
                            .await;
                        let _ = state.update(cx, |state, cx| {
                            state.copied = false;
                            cx.notify();
                        });
                    }));
                    cx.notify();
                });
            })
    }
}

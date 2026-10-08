use super::Status;
use crate::theme::palette;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        dialog::{Dialog, DialogAction, DialogClose, DialogFooter},
        kbd::Kbd,
        *,
    },
    *,
};

actions!(record_editor, [SaveRecord]);

/// The key context of a record editor's body and footer, where
/// [`SAVE_KEYS`] saves from any field. Enter saves from a text field, as in
/// every dialog, but opens a focused select or date picker.
const EDITOR_CONTEXT: &str = "RecordEditor";
const SAVE_KEYS: &str = "secondary-enter";

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(SAVE_KEYS, SaveRecord, Some(EDITOR_CONTEXT))]);
}

/// Opens a dialog that `render` rebuilds from the view on every frame.
pub fn open<V: 'static>(
    window: &mut Window,
    cx: &mut Context<V>,
    render: fn(&V, Dialog, &mut Context<V>) -> Dialog,
) {
    let view = cx.entity().downgrade();
    window.open_dialog(cx, move |dialog, _, cx| {
        view.update(cx, |this, cx| render(this, dialog, cx))
            .unwrap_or_else(|_| Dialog::new(cx))
    });
}

/// A form that can't be dismissed while busy. Confirming runs `submit` and
/// keeps the dialog open; the view closes it once the submission succeeds.
pub fn form<V: 'static>(
    this: &V,
    dialog: Dialog,
    submit_label: &'static str,
    status: fn(&V) -> &Status,
    submit: fn(&mut V, &mut Window, &mut Context<V>),
    close: fn(&mut V, &mut Context<V>),
    cx: &mut Context<V>,
) -> Dialog {
    let busy = status(this).busy;
    behaviour(this, dialog, status, submit, close, cx)
        .when(status(this).is_visible(), |dialog| {
            dialog.child(status(this).render())
        })
        .footer(footer(
            None,
            Button::new("dialog-submit").label(submit_label).primary(),
            busy,
        ))
}

/// A record editor: a [`form`] whose `body` shows its own errors, with the
/// keyboard hints in the footer before Cancel and the submit button.
/// [`SAVE_KEYS`] runs `submit` from anywhere in the body or footer.
#[allow(clippy::too_many_arguments)]
pub fn editor<V: 'static>(
    this: &V,
    dialog: Dialog,
    body: impl IntoElement,
    submit_label: &'static str,
    status: fn(&V) -> &Status,
    submit: fn(&mut V, &mut Window, &mut Context<V>),
    close: fn(&mut V, &mut Context<V>),
    cx: &mut Context<V>,
) -> Dialog {
    let busy = status(this).busy;
    let p = *palette(cx);
    let hints = h_flex()
        .flex_1()
        .min_w_0()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(p.faint)
        .child(key_hint("escape", cx))
        .child("cancel")
        .child(key_hint(SAVE_KEYS, cx).ml(px(8.)))
        .child("save");
    behaviour(this, dialog, status, submit, close, cx)
        .child(
            div()
                .key_context(EDITOR_CONTEXT)
                .on_action(save(submit, cx))
                .child(body),
        )
        .footer(
            div()
                .key_context(EDITOR_CONTEXT)
                .on_action(save(submit, cx))
                .child(footer(
                    Some(hints),
                    Button::new("dialog-submit").label(submit_label).primary(),
                    busy,
                )),
        )
}

fn save<V: 'static>(
    submit: fn(&mut V, &mut Window, &mut Context<V>),
    cx: &mut Context<V>,
) -> impl Fn(&SaveRecord, &mut Window, &mut App) + 'static {
    cx.listener(move |this, _: &SaveRecord, window, cx| submit(this, window, cx))
}

/// A keystroke in the kit's `Kbd`, drawn as an outlined 18px key.
fn key_hint(keys: &str, cx: &App) -> Kbd {
    let p = palette(cx);
    Kbd::new(Keystroke::parse(keys).expect("a valid keystroke"))
        .outline()
        .h(px(18.))
        .px(px(5.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .bg(gpui_kit::transparent_black())
        .border_color(p.strong_line)
        .text_color(p.faint)
        .text_size(px(11.))
}

/// What every form dialog shares: no dismissing while busy, Enter submits
/// without closing, and `close` runs when it closes.
fn behaviour<V: 'static>(
    this: &V,
    dialog: Dialog,
    status: fn(&V) -> &Status,
    submit: fn(&mut V, &mut Window, &mut Context<V>),
    close: fn(&mut V, &mut Context<V>),
    cx: &mut Context<V>,
) -> Dialog {
    let busy = status(this).busy;
    let view = cx.entity().downgrade();
    let submit_view = view.clone();
    dialog
        .overlay_closable(!busy)
        .keyboard(!busy)
        .close_button(!busy)
        .on_ok(move |_, window, cx| {
            let _ = submit_view.update(cx, |this, cx| submit(this, window, cx));
            false
        })
        .on_cancel(move |_, _, cx| {
            view.update(cx, |this, _| !status(this).busy)
                .unwrap_or(true)
        })
        .on_close(cx.listener(move |this, _, _, cx| {
            if !status(this).busy {
                close(this, cx);
            }
        }))
}

/// Opens a confirmation for a destructive action. Closing it any way runs
/// `close`, after `confirm` when confirmed.
pub fn confirm<V: 'static>(
    title: &'static str,
    message: impl Into<SharedString>,
    label: &'static str,
    confirm: fn(&mut V, &mut Window, &mut Context<V>),
    close: fn(&mut V, &mut Context<V>),
    window: &mut Window,
    cx: &mut Context<V>,
) {
    let message = message.into();
    let view = cx.entity().downgrade();
    window.open_dialog(cx, move |dialog, _, _| {
        let confirm_view = view.clone();
        let close_view = view.clone();
        dialog
            .title(title)
            .child(message.clone())
            .footer(footer(
                None,
                Button::new("dialog-confirm").label(label).danger(),
                false,
            ))
            .on_ok(move |_, window, cx| {
                confirm_view
                    .update(cx, |this, cx| confirm(this, window, cx))
                    .is_ok()
            })
            .on_close(move |_, _, cx| {
                let _ = close_view.update(cx, close);
            })
    });
}

/// Kit buttons that send Cancel and Confirm to the dialog they're in, sized to
/// their labels rather than to the footer, after the optional `hints`.
fn footer(hints: Option<Div>, confirm: Button, disabled: bool) -> DialogFooter {
    DialogFooter::new()
        .children(hints)
        .child(div().child(DialogClose::new().trigger(|button| {
            button
                .label("Cancel")
                .accessibility_label("Cancel")
                .outline()
                .disabled(disabled)
        })))
        .child(div().child(DialogAction::new().child(confirm.disabled(disabled))))
}

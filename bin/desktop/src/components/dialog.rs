use super::{ButtonText, FOCUS_RING, Status};
use crate::theme::palette;
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

/// Enter opens a focused select or date picker instead of saving, so
/// [`SAVE_KEYS`] saves from any field.
const EDITOR_CONTEXT: &str = "RecordEditor";
const SAVE_KEYS: &str = "secondary-enter";

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(SAVE_KEYS, SaveRecord, Some(EDITOR_CONTEXT))]);
}

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

#[allow(clippy::too_many_arguments)]
pub fn form<V: 'static>(
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
    behaviour(this, dialog, status, submit, close, cx)
        .children(status(this).error_alert())
        // The body clips, so it leaves room for the last field's ring.
        .child(div().pb(FOCUS_RING).child(body))
        .footer(footer(
            None,
            Button::new("dialog-submit")
                .text_label(submit_label)
                .primary(),
            busy,
        ))
}

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
                .pb(FOCUS_RING)
                .child(body),
        )
        .footer(
            div()
                .key_context(EDITOR_CONTEXT)
                .on_action(save(submit, cx))
                .child(footer(
                    Some(hints),
                    Button::new("dialog-submit")
                        .text_label(submit_label)
                        .primary(),
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
            // Stay open: the view closes the dialog once the submission
            // succeeds.
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

/// `close` runs on any close, after `confirm` when confirmed.
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
                Button::new("dialog-confirm").text_label(label).danger(),
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

/// The `div`s size the buttons to their labels instead of the footer.
fn footer(hints: Option<Div>, confirm: Button, disabled: bool) -> DialogFooter {
    DialogFooter::new()
        .children(hints)
        .child(
            div().child(
                DialogClose::new()
                    .trigger(|button| button.text_label("Cancel").outline().disabled(disabled)),
            ),
        )
        .child(div().child(DialogAction::new().child(confirm.disabled(disabled))))
}

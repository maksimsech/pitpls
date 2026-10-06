use super::Status;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        dialog::{Dialog, DialogAction, DialogClose, DialogFooter},
        *,
    },
    *,
};

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
        .when(status(this).is_visible(), |dialog| {
            dialog.child(status(this).render())
        })
        .footer(footer(
            Button::new("dialog-submit").label(submit_label).primary(),
            busy,
        ))
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
/// their labels rather than to the footer.
fn footer(confirm: Button, disabled: bool) -> DialogFooter {
    DialogFooter::new()
        .child(div().child(DialogClose::new().trigger(|button| {
            button
                .label("Cancel")
                .accessibility_label("Cancel")
                .outline()
                .disabled(disabled)
        })))
        .child(div().child(DialogAction::new().child(confirm.disabled(disabled))))
}

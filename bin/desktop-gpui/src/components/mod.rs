use gpui_kit::prelude::FluentBuilder;
pub mod file_picker;
pub mod form;
pub mod records;
pub mod table;

use crate::format::DisplayText;
use gpui_kit::{
    base::SelectableText,
    component::{
        alert::Alert,
        group_box::{GroupBox, GroupBoxVariants},
        menu::{ContextMenuExt, PopupMenuItem},
        tooltip::Tooltip,
        *,
    },
    *,
};

#[derive(Default)]
pub struct Status {
    pub busy: bool,
    pub loading: bool,
    pub loading_visible: bool,
    loading_delay: Option<Task<()>>,
    // A dismissed form error must not make a failed load look like empty data.
    pub ready: bool,
    pub error: Option<SharedString>,
    pub message: Option<SharedString>,
    // The page owns its pending UI callback; replacing/dropping it cancels the
    // stale callback while already-started service writes can safely finish.
    pub task: Option<Task<()>>,
}

impl Status {
    pub fn is_visible(&self) -> bool {
        self.busy || self.error.is_some() || self.message.is_some()
    }

    pub fn begin_load<V: 'static>(
        &mut self,
        window: &Window,
        cx: &Context<V>,
        status: fn(&mut V) -> &mut Status,
    ) {
        self.loading = true;
        self.loading_visible = false;
        self.error = None;
        let timer = cx
            .background_executor()
            .timer(std::time::Duration::from_millis(180));
        self.loading_delay = Some(cx.spawn_in(window, async move |view, cx| {
            timer.await;
            let _ = view.update_in(cx, |view, _, cx| {
                let status = status(view);
                if status.loading {
                    status.loading_visible = true;
                    cx.notify();
                }
            });
        }));
    }

    pub fn loaded<T>(&mut self, result: Result<T, String>) -> Option<T> {
        self.loading = false;
        self.loading_visible = false;
        self.loading_delay = None;
        self.ready = result.is_ok();
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.error = Some(error.into());
                None
            }
        }
    }

    pub fn begin_save(&mut self) {
        self.busy = true;
        self.error = None;
        self.message = None;
    }

    pub fn saved(&mut self, result: Result<String, String>) -> bool {
        self.busy = false;
        match result {
            Ok(message) => {
                self.message = Some(message.into());
                true
            }
            Err(error) => {
                self.error = Some(error.into());
                false
            }
        }
    }

    pub fn render(&self, _: &App) -> Div {
        v_flex()
            .flex_shrink_0()
            .gap_3()
            .when_some(self.message.clone(), |view, message| {
                view.child(Alert::success("operation-success", message))
            })
            .when_some(self.error.clone(), |view, error| {
                view.child(
                    Alert::error("operation-error", error)
                        .title("Unable to complete the operation"),
                )
            })
            .when(self.busy, |view| view.child("Working…"))
    }

    /// An overlay keeps background feedback out of the page's layout flow.
    pub fn refreshing(&self, cx: &App) -> Div {
        div()
            .absolute()
            .top_1()
            .right_5()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .when(self.ready && self.loading_visible, |view| {
                view.child("Refreshing…")
            })
    }
}

pub fn notice(message: impl Into<SharedString>, _: &App) -> Alert {
    Alert::warning("page-notice", message.into())
}

pub fn empty(message: &'static str, cx: &App) -> Div {
    v_flex()
        .gap_3()
        .items_center()
        .justify_center()
        .p_8()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius)
        .child(div().text_color(cx.theme().muted_foreground).child(message))
}

pub fn scroll(content: impl IntoElement) -> AnyElement {
    use gpui_kit::component::scroll::ScrollableElement;
    div()
        .id("page-scroll")
        .flex_1()
        .min_h_0()
        .w_full()
        .overflow_y_scrollbar()
        .child(content)
        .into_any_element()
}

/// Center a 1200 px content column with responsive side gutters.
/// This only sets geometry; pages retain their own scrolling behavior.
pub fn page_content() -> Div {
    v_flex()
        .w_full()
        .max_w(px(1232.))
        .min_w_0()
        .mx_auto()
        .px(px(16.))
        .py(px(24.))
}

pub struct SummaryGroup {
    pub title: &'static str,
    pub values: Vec<(&'static str, DisplayText)>,
}

/// Show a value as text that can be selected and copied. A shortened value
/// also shows its full value on hover and can copy it from the context menu.
pub fn display_text(element: Stateful<Div>, value: &DisplayText) -> AnyElement {
    let element = element.child(SelectableText::new("text", value.text.clone()));
    let Some(full) = value.full.clone() else {
        return element.into_any_element();
    };
    let copied = full.clone();
    element
        .tooltip(move |window, cx| Tooltip::new(full.clone()).build(window, cx))
        .context_menu(move |menu, _, _| {
            let copied = copied.clone();
            menu.item(
                PopupMenuItem::new("Copy full value").on_click(move |_, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(copied.to_string()))
                }),
            )
        })
        .into_any_element()
}

pub fn section_heading(title: &'static str) -> Div {
    div().text_xl().font_semibold().child(title)
}

/// Use the same card geometry as loaded summaries; only the values are pending.
pub fn summary_skeleton(
    title: &'static str,
    labels: &[&'static str],
    visible: bool,
    cx: &App,
) -> Div {
    v_flex()
        .flex_shrink_0()
        .gap_3()
        .child(section_heading(title))
        .child(
            h_flex()
                .flex_wrap()
                .gap_3()
                .children(labels.iter().map(|label| {
                    GroupBox::new()
                        .id(SharedString::from(format!("pending-{title}-{label}")))
                        .outline()
                        .min_w(rems(15.7))
                        .flex_1()
                        .content_style(gpui_kit::StyleRefinement::default().gap_2())
                        .child(
                            div()
                                .text_base()
                                .font_medium()
                                .text_color(cx.theme().muted_foreground)
                                .child(*label),
                        )
                        .child(
                            div().text_lg().child(
                                div()
                                    .h(rems(1.75))
                                    .w(rems(9.))
                                    .rounded(px(4.))
                                    .bg(cx.theme().skeleton)
                                    .opacity(if visible { 1. } else { 0. }),
                            ),
                        )
                })),
        )
}

pub fn summaries(groups: &[SummaryGroup], cx: &App) -> Div {
    v_flex()
        .flex_shrink_0()
        .gap_5()
        .children(groups.iter().map(|group| {
            v_flex().gap_3().child(section_heading(group.title)).child(
                h_flex()
                    .flex_wrap()
                    .gap_3()
                    .children(group.values.iter().map(|(label, value)| {
                        GroupBox::new()
                            .id(SharedString::from(format!(
                                "summary-{}-{label}",
                                group.title
                            )))
                            .outline()
                            .min_w(rems(15.7))
                            .flex_1()
                            .content_style(gpui_kit::StyleRefinement::default().gap_2())
                            .child(
                                div()
                                    .text_base()
                                    .font_medium()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(*label),
                            )
                            .child(display_text(
                                div()
                                    .id("value")
                                    .text_lg()
                                    .font_family(cx.theme().mono_font_family.clone()),
                                value,
                            ))
                    })),
            )
        }))
}

/// Keep confirmations dismissible while routing both buttons through the
/// dialog's existing callbacks (including its close/focus cleanup).
pub fn confirmation_footer(label: &'static str) -> Div {
    use gpui_kit::component::{
        button::*,
        dialog::{Cancel, Confirm},
    };
    h_flex()
        .justify_end()
        .gap_2()
        .child(
            Button::new("cancel-confirmation")
                .label("Cancel")
                .outline()
                .on_click(|_, window, cx| window.dispatch_action(Box::new(Cancel), cx)),
        )
        .child(
            Button::new("confirm-action")
                .label(label)
                .danger()
                .on_click(|_, window, cx| {
                    window.dispatch_action(Box::new(Confirm { secondary: false }), cx)
                }),
        )
}

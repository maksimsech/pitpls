use gpui_kit::prelude::FluentBuilder;
pub mod file_picker;
pub mod form;
pub mod table;

use gpui_kit::{component::*, *};

#[derive(Default)]
pub struct Status {
    pub busy: bool,
    pub loading: bool,
    // A dismissed form error must not make a failed load look like empty data.
    pub ready: bool,
    pub error: Option<SharedString>,
    pub message: Option<SharedString>,
    // The page owns its pending UI callback; replacing/dropping it cancels the
    // stale callback while already-started service writes can safely finish.
    pub task: Option<Task<()>>,
}

impl Status {
    pub fn begin_load(&mut self) {
        self.loading = true;
        self.ready = false;
        self.error = None;
    }

    pub fn loaded<T>(&mut self, result: Result<T, String>) -> Option<T> {
        self.loading = false;
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

    pub fn render(&self, cx: &App) -> Div {
        v_flex()
            .gap_3()
            .when_some(self.message.clone(), |view, message| {
                view.child(notice(message, cx))
            })
            .when_some(self.error.clone(), |view, error| {
                view.child(
                    v_flex()
                        .gap_2()
                        .p_4()
                        .border_1()
                        .border_color(cx.theme().danger)
                        .rounded(cx.theme().radius)
                        .child(
                            div()
                                .font_semibold()
                                .child("Unable to complete the operation"),
                        )
                        .child(error),
                )
            })
            .when(self.busy, |view| view.child("Working…"))
            .when(self.loading, |view| view.child("Loading…"))
    }
}

pub fn notice(message: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .p_3()
        .rounded(cx.theme().radius)
        .bg(cx.theme().muted)
        .child(message.into())
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

pub fn period(year: Option<i32>, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(
            year.map(|year| format!("Reporting year {year}"))
                .unwrap_or_else(|| "All years · Historical information".into()),
        )
}

pub struct SummaryGroup {
    pub title: &'static str,
    pub values: Vec<(&'static str, SharedString)>,
}

pub fn summaries(groups: &[SummaryGroup], cx: &App) -> Div {
    v_flex().gap_5().children(groups.iter().map(|group| {
        v_flex()
            .gap_3()
            .child(div().font_semibold().child(group.title))
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_3()
                    .children(group.values.iter().map(|(label, value)| {
                        v_flex()
                            .gap_2()
                            .min_w(rems(15.7))
                            .flex_1()
                            .p_4()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(*label),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    .child(value.clone()),
                            )
                    })),
            )
    }))
}

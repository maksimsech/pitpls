use gpui_kit::prelude::FluentBuilder;
pub mod file_picker;
pub mod form;
pub mod records;
pub mod table;

use gpui_kit::{
    component::{
        alert::Alert,
        group_box::{GroupBox, GroupBoxVariants},
        *,
    },
    *,
};

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
            .when(self.loading, |view| view.child("Loading…"))
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

pub struct SummaryGroup {
    pub title: &'static str,
    pub values: Vec<(&'static str, SharedString)>,
}

pub fn summaries(groups: &[SummaryGroup], cx: &App) -> Div {
    v_flex()
        .flex_shrink_0()
        .gap_5()
        .children(groups.iter().map(|group| {
            v_flex()
                .gap_3()
                .child(div().font_semibold().child(group.title))
                .child(
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

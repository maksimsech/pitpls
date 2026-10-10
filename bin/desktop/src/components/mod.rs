pub mod copy;
pub mod data;
pub mod dialog;
pub mod file_picker;
pub mod form;
pub mod header;
pub mod nbp;
pub mod notice;
pub mod records;
pub mod value;

use crate::services::Error as ServiceError;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{alert::Alert, spinner::Spinner, *},
    *,
};
use std::time::Duration;

pub const SKELETON_DELAY: Duration = Duration::from_millis(180);

#[derive(Default)]
pub struct Status {
    pub busy: bool,
    pub loading: bool,
    pub loading_visible: bool,
    loading_delay: Option<Task<()>>,
    /// Forms also set and clear `error`, so it can't tell a failed load from
    /// empty data.
    pub ready: bool,
    pub error: Option<SharedString>,
    pub message: Option<SharedString>,
    /// Replacing or dropping this cancels the pending UI callback. Work already
    /// started in the background still finishes.
    pub task: Option<Task<()>>,
}

impl Status {
    pub fn is_visible(&self) -> bool {
        self.error.is_some() || self.message.is_some()
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
        let timer = cx.background_executor().timer(SKELETON_DELAY);
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

    pub fn loaded<T>(&mut self, result: Result<T, ServiceError>) -> Option<T> {
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

    pub fn saved(&mut self, result: Result<String, ServiceError>) -> bool {
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

    pub fn render(&self) -> Div {
        v_flex()
            .flex_shrink_0()
            .gap_3()
            .when_some(self.message.clone(), |view, message| {
                view.child(Alert::success("operation-success", message))
            })
            .children(self.error_alert())
    }

    pub fn error_alert(&self) -> Option<Alert> {
        self.error.clone().map(|error| {
            Alert::error("operation-error", error).title("Unable to complete the operation")
        })
    }

    pub fn refreshing(&self, cx: &App) -> Div {
        div()
            .text_size(px(12.))
            .text_color(crate::theme::palette(cx).faint)
            .child("Refreshing…")
            // Hidden, not empty, so it adds no gap to the header's row.
            .when(!(self.ready && self.loading_visible), |view| view.hidden())
    }
}

/// How far the kit's focus ring reaches outside a control.
pub const FOCUS_RING: Pixels = px(3.);

pub fn measure_width<V: 'static>(
    current: Option<Pixels>,
    width: fn(&mut V) -> &mut Option<Pixels>,
    cx: &Context<V>,
) -> impl IntoElement {
    let view = cx.entity().downgrade();
    canvas(
        move |bounds, _, cx| {
            let measured = bounds.size.width;
            if measured > px(0.) && current != Some(measured) {
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, cx| {
                        *width(view) = Some(measured);
                        cx.notify();
                    });
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
}

pub fn spinner() -> Spinner {
    Spinner::new().with_size(px(14.))
}

/// Text of a default-size button, as the kit drew it before 0.7.1, which
/// moved them to `text_sm` (12.25px).
pub const BUTTON_TEXT: Pixels = px(14.);

pub trait ButtonText {
    /// The kit sizes [`Button::label`] itself, so the text goes on a child at
    /// [`BUTTON_TEXT`].
    fn text_label(self, label: impl Into<SharedString>) -> Self;
}

impl ButtonText for button::Button {
    fn text_label(self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.accessibility_label(label.clone())
            .child(div().text_size(BUTTON_TEXT).child(label))
    }
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

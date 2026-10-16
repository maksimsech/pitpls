#[expect(dead_code, reason = "Portfolio pages adopt this in tasks 5–9.")]
pub mod chart;
pub mod copy;
pub mod data;
pub mod dialog;
pub mod file_picker;
pub mod form;
pub mod header;
pub mod nbp;
pub mod notice;
pub mod records;
pub mod tabs;
pub mod value;
#[expect(dead_code, reason = "Portfolio pages adopt this in tasks 5–9.")]
pub mod year_table;

use crate::services::Error as ServiceError;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{alert::Alert, spinner::Spinner, *},
    *,
};
use rust_i18n::t;
use std::{rc::Rc, time::Duration};

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
    pub error: Option<StatusText>,
    pub message: Option<StatusText>,
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

    pub fn render<V: 'static>(&self, status: fn(&mut V) -> &mut Status, cx: &Context<V>) -> Div {
        v_flex()
            .flex_shrink_0()
            .gap_3()
            .when_some(self.message.as_ref(), |view, message| {
                view.child(
                    Alert::success("operation-success", message.text()).on_close(cx.listener(
                        move |view, _, _, cx| {
                            status(view).message = None;
                            cx.notify();
                        },
                    )),
                )
            })
            .children(self.error_alert().map(|alert| {
                alert.on_close(cx.listener(move |view, _, _, cx| {
                    status(view).error = None;
                    cx.notify();
                }))
            }))
    }

    pub fn error_alert(&self) -> Option<Alert> {
        self.error.as_ref().map(|error| {
            Alert::error("operation-error", error.text()).title(t!("common.operation_failed"))
        })
    }

    pub fn refreshing(&self, cx: &App) -> Div {
        div()
            .text_size(px(12.))
            .text_color(crate::theme::palette(cx).faint)
            .child(t!("common.refreshing"))
            // Hidden, not empty, so it adds no gap to the header's row.
            .when(!(self.ready && self.loading_visible), |view| view.hidden())
    }
}

/// Kept as its source rather than its words, so it follows a change of
/// language while it's shown.
#[derive(Clone)]
pub enum StatusText {
    Plain(SharedString),
    Key(&'static str),
    Error(Rc<ServiceError>),
}

impl StatusText {
    pub fn text(&self) -> SharedString {
        match self {
            Self::Plain(text) => text.clone(),
            Self::Key(key) => t!(*key).into(),
            Self::Error(error) => error.as_ref().into(),
        }
    }
}

impl From<ServiceError> for StatusText {
    fn from(error: ServiceError) -> Self {
        Self::Error(Rc::new(error))
    }
}

impl From<String> for StatusText {
    fn from(text: String) -> Self {
        Self::Plain(text.into())
    }
}

impl From<SharedString> for StatusText {
    fn from(text: SharedString) -> Self {
        Self::Plain(text)
    }
}

/// How far the kit's focus ring reaches outside a control.
pub const FOCUS_RING: Pixels = px(3.);

pub fn measure_width<V: 'static>(
    current: Option<Pixels>,
    width: impl FnOnce(&mut V) -> &mut Option<Pixels> + 'static,
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

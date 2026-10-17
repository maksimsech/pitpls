use crate::{components::tabs::TabItem, format::DisplayText, theme::palette};
use gpui_kit::{
    assets::IconName,
    base::InteractiveElementExt,
    component::{Icon, h_flex, scroll::ScrollableElement},
    *,
};
use std::rc::Rc;

pub type OpenYear = Rc<dyn Fn(i32, &mut Window, &mut App)>;

pub struct PortfolioStat {
    pub label: &'static str,
    pub value: DisplayText,
    pub unit: Option<SharedString>,
    pub caption: Option<SharedString>,
}

pub trait PortfolioView: Default + Send + 'static {
    const LABELS: &'static [&'static str] = &[];

    fn stats(&self) -> Vec<PortfolioStat> {
        Vec::new()
    }

    fn tabs(&self) -> Vec<TabItem<usize>> {
        Vec::new()
    }

    fn caption(&self, _: Option<usize>) -> SharedString {
        SharedString::default()
    }

    fn records_note(&self) -> SharedString {
        SharedString::default()
    }

    fn filtered_count(&self, _: usize, _: &str) -> Option<(usize, usize)> {
        None
    }

    fn render(
        &self,
        tab: usize,
        query: &str,
        scroll: &ScrollHandle,
        width: Pixels,
        on_open_year: OpenYear,
        cx: &App,
    ) -> AnyElement;
}

impl PortfolioView for () {
    fn render(
        &self,
        _: usize,
        _: &str,
        _: &ScrollHandle,
        _: Pixels,
        _: OpenYear,
        _: &App,
    ) -> AnyElement {
        div().into_any_element()
    }
}

pub fn note(text: impl Into<SharedString>, cx: &App) -> Div {
    h_flex()
        .flex_shrink_0()
        .items_start()
        .px(px(20.))
        .py(px(16.))
        .gap(px(8.))
        .text_size(px(12.))
        .text_color(palette(cx).faint)
        .child(Icon::new(IconName::Info).size(px(14.)).flex_shrink_0())
        .child(div().min_w_0().child(text.into()))
}

pub fn scroll_table(rows: Div, width: Pixels, scroll: &ScrollHandle) -> Stateful<Div> {
    div()
        .id("portfolio-table")
        .relative()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .overflow_hidden()
        .child(
            h_flex()
                .id("portfolio-pan")
                .w_full()
                .overflow_x_scroll()
                .lock_scroll_axis()
                .track_scroll(scroll)
                .child(rows.w(width).flex_none()),
        )
        .horizontal_scrollbar(scroll)
}

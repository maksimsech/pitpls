use crate::{
    components::{notice, tabs::TabItem},
    format::DisplayText,
};
use gpui_kit::*;
use std::rc::Rc;

pub type OpenYear = Rc<dyn Fn(i32, &mut Window, &mut App)>;

pub struct PortfolioStat {
    pub label: &'static str,
    pub value: DisplayText,
    pub unit: Option<SharedString>,
    pub caption: Option<SharedString>,
}

pub trait PortfolioView: Send + 'static {
    const LABELS: &'static [&'static str];

    fn stats(&self) -> Vec<PortfolioStat>;
    fn tabs(&self) -> Vec<TabItem<usize>>;
    fn caption(&self, tab: Option<usize>) -> SharedString;
    fn records_note(&self) -> SharedString;

    fn callout(&self) -> Option<SharedString> {
        None
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

pub fn note(text: impl Into<SharedString>, cx: &App) -> Div {
    notice::info(text, cx)
        .flex_shrink_0()
        .px(px(20.))
        .py(px(16.))
}

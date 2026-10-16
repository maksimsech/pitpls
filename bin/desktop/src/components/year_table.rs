use super::value;
use crate::{
    format::DisplayText,
    theme::{palette, tabular_digits},
};
use gpui_kit::{
    assets::IconName,
    base::Button,
    component::{Icon, StyledExt, h_flex, tooltip::Tooltip},
    prelude::FluentBuilder,
    *,
};
use rust_i18n::t;

#[derive(Clone, Copy)]
pub enum ColumnWidth {
    Fixed(Pixels),
    Flexible,
}

pub struct YearTable {
    widths: Vec<ColumnWidth>,
}

impl YearTable {
    pub fn new(widths: Vec<ColumnWidth>) -> Self {
        Self { widths }
    }

    fn cell(&self, index: usize) -> Div {
        div()
            .min_w_0()
            .when_else(
                index == 0,
                |cell| cell.text_left(),
                |cell| cell.text_right(),
            )
            .map(|cell| match self.widths.get(index) {
                Some(ColumnWidth::Fixed(width)) => cell.w(*width).flex_shrink_0(),
                _ => cell.flex_1(),
            })
    }

    fn frame(&self, cx: &App) -> Div {
        h_flex()
            .w_full()
            .gap(px(8.))
            .pl(px(16.))
            .pr(px(8.))
            .border_t_1()
            .border_color(palette(cx).line)
    }

    pub fn header(&self, columns: Vec<SharedString>, cx: &App) -> Div {
        self.frame(cx)
            .h(px(32.))
            .text_size(px(12.))
            .text_color(palette(cx).faint)
            .children(
                columns
                    .into_iter()
                    .enumerate()
                    .map(|(index, label)| self.cell(index).child(label)),
            )
            .child(div().w(px(28.)).flex_shrink_0())
    }

    fn cells(&self, cells: Vec<DisplayText>, cx: &App) -> Vec<Div> {
        cells
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                self.cell(index + 1)
                    .text_color(palette(cx).text)
                    .child(value::text(cell, true, false, cx))
            })
            .collect()
    }

    pub fn year_row(
        &self,
        year: i32,
        cells: Vec<DisplayText>,
        so_far: bool,
        on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        cx: &App,
    ) -> Button {
        let p = *palette(cx);
        let label: SharedString = t!("year.open_tax_year", year = year).into();
        Button::new(("portfolio-year", year as u64))
            .role(Role::Button)
            .accessibility_label(label.clone())
            .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
            .w_full()
            .h(px(40.))
            .text_size(px(13.))
            .font_features(tabular_digits())
            .text_color(transparent_black())
            .hover(move |style| style.bg(p.hover).text_color(p.faint))
            .focus(move |style| style.bg(p.hover).text_color(p.faint))
            .child(
                self.frame(cx)
                    .h_full()
                    .child(
                        self.cell(0)
                            .flex()
                            .items_center()
                            .flex_wrap()
                            .gap_x(px(3.))
                            .font_medium()
                            .text_color(p.text)
                            .child(year.to_string())
                            .when(so_far, |cell| {
                                cell.child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::NORMAL)
                                        .text_color(p.faint)
                                        .child(format!("· {}", t!("year.so_far"))),
                                )
                            }),
                    )
                    .children(self.cells(cells, cx))
                    .child(
                        h_flex()
                            .w(px(28.))
                            .flex_shrink_0()
                            .justify_end()
                            .child(Icon::new(IconName::ChevronRight).size(px(14.))),
                    ),
            )
            .on_click(on_open)
    }

    pub fn total_row(
        &self,
        label: impl Into<SharedString>,
        cells: Vec<DisplayText>,
        cx: &App,
    ) -> Div {
        self.frame(cx)
            .h(px(40.))
            .text_size(px(13.))
            .font_semibold()
            .font_features(tabular_digits())
            .border_color(palette(cx).strong_line)
            .child(self.cell(0).child(label.into()))
            .children(self.cells(cells, cx))
            .child(div().w(px(28.)).flex_shrink_0())
    }
}

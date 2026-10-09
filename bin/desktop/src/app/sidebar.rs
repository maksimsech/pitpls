use super::*;
use crate::{
    components::header,
    theme::{palette, tabular_digits},
};
use gpui_kit::{
    assets::IconName,
    base::Easing,
    component::{button::*, popover::Popover, tag::Tag},
};
use std::time::Duration;

const ITEM_HEIGHT: Pixels = px(30.);
const CONTROL: Pixels = px(28.);
const SWITCHER_WIDTH: Pixels = px(204.);
/// The app's "pls" wordmark. GPUI fills it with the element's text colour;
/// the size keeps the SVG's 775.5 × 532.6 view box.
const WORDMARK: &[u8] = include_bytes!("../../assets/pls-wordmark.svg");
const WORDMARK_WIDTH: Pixels = px(24.75);
const WORDMARK_HEIGHT: Pixels = px(17.);

/// How far the sidebar is expanded: 1 is the labelled sidebar, 0 the icon
/// rail. `size` drives widths and positions, `fade` the labels, counts and
/// group names, and `line` the rail's group dividers, each with the timing
/// of the design mockup.
#[derive(Clone, Copy)]
pub struct Expansion {
    size: f32,
    fade: f32,
    line: f32,
}

impl Expansion {
    pub fn settled(expanded: bool) -> Self {
        let value = if expanded { 1. } else { 0. };
        Self {
            size: value,
            fade: value,
            line: value,
        }
    }

    pub fn animate(expanded: bool, window: &mut Window, cx: &mut App) -> Self {
        let target = if expanded { 1. } else { 0. };
        let mut timed = |id: &'static str, millis, easing| {
            transition(
                id,
                target,
                Transition::new(Duration::from_millis(millis)).easing(easing),
                window,
                cx,
            )
        };
        Self {
            size: timed(
                "sidebar-size",
                240,
                Easing::CubicBezier {
                    x1: 0.2,
                    y1: 0.8,
                    x2: 0.2,
                    y2: 1.,
                },
            ),
            fade: timed("sidebar-fade", 140, Easing::Ease),
            line: timed("sidebar-line", 200, Easing::Ease),
        }
    }

    /// `rail` in the icon rail, `expanded` in the labelled sidebar.
    fn size(&self, rail: f32, expanded: f32) -> Pixels {
        px(rail + (expanded - rail) * self.size)
    }
}

pub fn width(expansion: Expansion) -> Pixels {
    expansion.size(84., 220.)
}

/// The sidebar's side padding. The rail's content is 60px wide as in the
/// mockup, but macOS draws the traffic lights about 60px wide from x = 16,
/// so the rail pads 12px a side (84px) to keep 8px between them and the
/// panel.
fn padding(expansion: Expansion) -> Pixels {
    expansion.size(12., 8.)
}

impl Desktop {
    pub fn sidebar(
        &self,
        expansion: Expansion,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let collapsed = self.preferences.sidebar_collapsed;
        let body_top = expansion.size(84., 52.);
        let toggle_label = if collapsed {
            "Expand sidebar"
        } else {
            "Collapse sidebar"
        };
        let wordmark = (expansion.fade > 0.).then(|| wordmark(expansion, window, cx));
        div()
            .relative()
            .flex_shrink_0()
            .w(width(expansion))
            .h_full()
            .overflow_hidden()
            .text_size(px(13.))
            // The traffic lights and the toggle sit in this area.
            .child(
                header::drag_region("sidebar-drag", window, cx)
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .h(body_top)
                    .children(wordmark),
            )
            .child(
                v_flex()
                    .absolute()
                    .inset_0()
                    .pt(body_top)
                    .px(padding(expansion))
                    .pb(px(10.))
                    .child(self.year_switcher(expansion, cx))
                    .child(self.pages(expansion, cx))
                    .child(self.sidebar_footer(expansion, cx)),
            )
            .child(
                header::no_drag()
                    .absolute()
                    .left(expansion.size(28., 184.))
                    .top(expansion.size(48., 8.))
                    .child(
                        header::icon_button("sidebar-toggle", IconName::PanelLeft, toggle_label)
                            .text_color(palette(cx).muted)
                            .tooltip_placement(if collapsed {
                                Placement::Right
                            } else {
                                Placement::Bottom
                            })
                            .disabled(self.context.is_none())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.preferences.sidebar_collapsed =
                                    !this.preferences.sidebar_collapsed;
                                this.save_preferences(window, cx);
                                cx.notify();
                            })),
                    ),
            )
    }

    /// The year switcher: a large button in the sidebar and a small one in
    /// the rail, crossfading. The one for the current state opens the menu.
    fn year_switcher(&self, expansion: Expansion, cx: &mut Context<Self>) -> Div {
        let p = *palette(cx);
        let collapsed = self.preferences.sidebar_collapsed;
        let label = header::year_label(self.preferences.year);
        // A disabled trigger would still open a popover, so a switcher
        // locked by a page isn't wrapped in one. A year being saved keeps it,
        // since remounting the open menu would move focus out of its field;
        // the menu's rows are disabled meanwhile.
        let enabled = self.context.is_some() && !self.page_locked;
        let large = Button::new("tax-year")
            .w(SWITCHER_WIDTH)
            .h(px(46.))
            .pl(px(8.))
            .pr(px(10.))
            .rounded(px(10.))
            .border_color(p.line)
            .accessibility_label(format!("Tax year {label}, change"))
            .disabled(!enabled)
            .child(
                h_flex()
                    .w_full()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .justify_center()
                            .size(CONTROL)
                            .rounded(px(7.))
                            .bg(p.selected)
                            .text_color(p.muted)
                            .child(Icon::new(IconName::Calendar).size(px(15.))),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .line_height(relative(1.25))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.faint)
                                    .child("Tax year"),
                            )
                            .child(div().font_semibold().child(label.clone())),
                    )
                    .child(
                        Icon::new(IconName::ChevronsUpDown)
                            .size(px(14.))
                            .text_color(p.faint),
                    ),
            );
        let small = Button::new("tax-year-rail")
            .w(px(52.))
            .h(CONTROL)
            .p_0()
            .rounded(px(7.))
            .border_color(p.line)
            .accessibility_label(format!("Tax year {label}, change"))
            // Only in the rail. While the sidebar expands, this button slides
            // under the pointer and fades out; a tooltip it started would
            // stay once it's no longer drawn, since the kit hides tooltips on
            // hover-out.
            .when(collapsed, |button| {
                button
                    .tooltip(format!("Tax year {label}"))
                    .tooltip_placement(Placement::Right)
            })
            .disabled(!enabled)
            .child(div().text_size(px(12.)).font_semibold().child(
                if self.preferences.year.is_some() {
                    label.clone()
                } else {
                    "All".into()
                },
            ));
        let mut trigger = |id: &'static str, button: Button, active: bool| {
            if enabled && active {
                self.year_popover(id, button, cx).into_any_element()
            } else {
                button.into_any_element()
            }
        };
        let large = trigger("year-menu", large, !collapsed);
        let small = trigger("year-menu-rail", small, collapsed);
        div()
            .relative()
            .flex_shrink_0()
            .h(expansion.size(28., 46.))
            .when(expansion.fade > 0., |view| {
                view.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .opacity(expansion.fade)
                        .child(large),
                )
            })
            .when(expansion.fade < 1., |view| {
                view.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(4.))
                        .opacity(1. - expansion.fade)
                        .child(small),
                )
            })
    }

    fn year_popover(&self, id: &'static str, trigger: Button, cx: &mut Context<Self>) -> Popover {
        let desktop = cx.entity().downgrade();
        Popover::new(id)
            .anchor(Anchor::TopLeft)
            .offset(px(6.))
            .open(self.year_menu_open)
            .on_open_change(
                cx.listener(|this, open: &bool, window, cx| this.set_year_menu(*open, window, cx)),
            )
            .trigger(trigger)
            .w(px(268.))
            .p(px(6.))
            .rounded(px(12.))
            .border_color(palette(cx).strong_line)
            .content(move |_, _, cx| {
                desktop
                    .update(cx, |this, cx| this.year_menu(cx).into_any_element())
                    .unwrap_or_else(|_| div().into_any_element())
            })
    }

    fn pages(&self, expansion: Expansion, cx: &mut Context<Self>) -> Stateful<Div> {
        let year = self.preferences.year;
        let years = self
            .years
            .iter()
            .filter(|info| year.is_none_or(|year| info.year == year));
        let (dividends, interests, cryptos) = years.fold((0, 0, 0), |sum, info| {
            (
                sum.0 + info.dividends,
                sum.1 + info.interests,
                sum.2 + info.cryptos,
            )
        });
        let count = |count: u32| Some(count.to_string());
        let p = *palette(cx);
        let planned = Tag::custom(transparent_black(), p.faint, p.strong_line)
            .rounded_full()
            .h(px(20.))
            .px(px(7.))
            .py_0()
            .text_size(px(11.))
            .child("Planned")
            .into_any_element();
        v_flex()
            .id("sidebar-pages")
            .flex_1()
            .min_h_0()
            .mt(expansion.size(8., 12.))
            .gap(px(2.))
            .overflow_y_scroll()
            .child(self.page_item(Page::Home, None, expansion, cx))
            .child(group("Records", expansion, cx))
            .child(self.page_item(Page::Dividends, count(dividends), expansion, cx))
            .child(self.page_item(Page::Interests, count(interests), expansion, cx))
            .child(self.page_item(Page::Crypto, count(cryptos), expansion, cx))
            .child(
                nav_item(
                    "navigate-Stocks",
                    "Stocks",
                    IconName::TrendingUp,
                    Some(planned),
                    expansion,
                )
                .text_color(p.faint)
                .disabled(true)
                .when(self.preferences.sidebar_collapsed, |item| {
                    item.tooltip("Stocks · planned")
                        .tooltip_placement(Placement::Right)
                }),
            )
            .child(group("Data", expansion, cx))
            .child(self.page_item(Page::Imports, None, expansion, cx))
            .child(self.page_item(Page::Rates, None, expansion, cx))
    }

    fn page_item(
        &self,
        page: Page,
        count: Option<String>,
        expansion: Expansion,
        cx: &mut Context<Self>,
    ) -> Button {
        let p = *palette(cx);
        let selected = self.page == page;
        let count = count.map(|count| {
            div()
                .font_weight(FontWeight::NORMAL)
                .text_size(px(12.))
                .text_color(p.faint)
                .font_features(tabular_digits())
                .child(count)
                .into_any_element()
        });
        nav_item(
            SharedString::from(format!("navigate-{}", page.title())),
            page.title(),
            page.icon(),
            count,
            expansion,
        )
        .when(selected, |item| {
            item.selected(true)
                .bg(p.selected)
                .text_color(p.text)
                .font_medium()
        })
        .when(!selected, |item| item.text_color(p.muted))
        .disabled(self.locked() || self.context.is_none())
        .on_click(cx.listener(move |this, _, window, cx| this.navigate(page, window, cx)))
        .when(self.preferences.sidebar_collapsed, |item| {
            item.tooltip(page.title())
                .tooltip_placement(Placement::Right)
        })
    }

    fn sidebar_footer(&self, expansion: Expansion, cx: &mut Context<Self>) -> Div {
        let dark = cx.theme().is_dark();
        let theme_label = if dark {
            "Switch to light theme"
        } else {
            "Switch to dark theme"
        };
        div()
            .relative()
            .flex_shrink_0()
            .h(expansion.size(76., 38.))
            .border_t_1()
            .border_color(palette(cx).line)
            .child(
                div()
                    .absolute()
                    .top(px(8.))
                    .left_0()
                    .w(expansion.size(60., 172.))
                    .child(self.page_item(Page::Settings, None, expansion, cx)),
            )
            .child(
                div()
                    .absolute()
                    .left(expansion.size(16., 176.))
                    .top(expansion.size(44., 9.))
                    .child(
                        header::icon_button(
                            "theme",
                            if dark { IconName::Moon } else { IconName::Sun },
                            theme_label,
                        )
                        .text_color(palette(cx).muted)
                        .tooltip_placement(Placement::Right)
                        .disabled(self.context.is_none())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.set_theme(Some(!cx.theme().is_dark()), window, cx)
                        })),
                    ),
            )
    }
}

/// The wordmark beside the traffic lights, which macOS draws about 60pt wide
/// from x = 16 (none in full screen). It fades out with the labels: the rail
/// has no room for it.
fn wordmark(expansion: Expansion, window: &Window, cx: &App) -> Svg {
    let left = if cfg!(target_os = "macos") && !window.is_fullscreen() {
        px(92.)
    } else {
        px(16.)
    };
    svg()
        .data(WORDMARK)
        .absolute()
        .left(left)
        .top((crate::TITLE_ROW_HEIGHT - WORDMARK_HEIGHT) / 2.)
        .w(WORDMARK_WIDTH)
        .h(WORDMARK_HEIGHT)
        .text_color(palette(cx).muted)
        .opacity(expansion.fade)
}

/// A 30px sidebar row: icon, then the label and `trailing` (a count or a
/// tag), which fade out in the rail while the icon slides to the centre.
fn nav_item(
    id: impl Into<ElementId>,
    label: &'static str,
    icon: IconName,
    trailing: Option<AnyElement>,
    expansion: Expansion,
) -> Button {
    Button::new(id)
        .ghost()
        .w_full()
        .h(ITEM_HEIGHT)
        .flex_shrink_0()
        .pl(expansion.size(22., 10.))
        .pr(px(10.))
        .rounded(px(8.))
        .accessibility_label(label)
        .child(
            h_flex()
                .w_full()
                .gap(px(10.))
                .child(Icon::new(icon).size(px(16.)).flex_shrink_0())
                .when(expansion.fade > 0., |row| {
                    row.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .whitespace_nowrap()
                            .opacity(expansion.fade)
                            .child(label),
                    )
                    .when_some(trailing, |row, trailing| {
                        row.child(
                            div()
                                .flex_shrink_0()
                                .opacity(expansion.fade)
                                .child(trailing),
                        )
                    })
                }),
        )
}

/// A group name over its pages; in the rail, a short divider instead.
fn group(label: &'static str, expansion: Expansion, cx: &App) -> Div {
    let p = *palette(cx);
    div()
        .relative()
        .flex_shrink_0()
        .w_full()
        .h(expansion.size(13., 30.))
        .when(expansion.fade > 0., |view| {
            view.child(
                div()
                    .absolute()
                    .left(px(10.))
                    .bottom(px(6.))
                    .whitespace_nowrap()
                    .text_size(px(11.5))
                    .font_medium()
                    .text_color(p.faint)
                    .opacity(expansion.fade)
                    .child(label),
            )
        })
        .when(expansion.line < 1., |view| {
            view.child(
                div()
                    .absolute()
                    .left(px(16.))
                    .right(px(16.))
                    .top(relative(0.5))
                    .h(px(1.))
                    .bg(p.line)
                    .opacity(1. - expansion.line),
            )
        })
}

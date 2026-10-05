use super::*;
use crate::TOOLBAR_HEIGHT;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{
        button::*,
        menu::{DropdownMenu, PopupMenu, PopupMenuItem},
    },
};

const SIDEBAR_WIDTH: Pixels = px(230.);
const RAIL_WIDTH: Pixels = px(52.);
const NAV_BUTTON_SIZE: Pixels = px(36.);
const NAV_PADDING: Pixels = px(8.);
const NAV_GAP: Pixels = px(4.);

impl Desktop {
    pub fn toolbar(&self, progress: f32, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let locked = self.locked();
        let collapsed = self.preferences.sidebar_collapsed;
        let fullscreen = window.is_fullscreen() || window.is_simple_fullscreen();
        let integrated_titlebar = cfg!(target_os = "macos") && !fullscreen;
        // Expanded navigation and the page title share the same vertical edge.
        // The collapsed header still reserves room for the native window controls.
        let leading_width = if integrated_titlebar {
            px(52.) + (SIDEBAR_WIDTH - px(96.) - px(52.)) * progress
        } else {
            Self::sidebar_width(progress)
        };
        let leading = h_flex()
            .relative()
            .h_full()
            .w(leading_width)
            .flex_shrink_0()
            .px_3()
            // Keep the toolbar divider inset in both sidebar states. When
            // expanded, it lines up with the navigation panel's right edge.
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top((TOOLBAR_HEIGHT - px(20.)) / 2.)
                    .w(px(1.))
                    .h(px(20.))
                    .bg(cx.theme().sidebar_border),
            )
            .child(
                Button::new("sidebar-toggle")
                    .icon(IconName::PanelLeft)
                    .ghost()
                    .accessibility_label(if collapsed {
                        "Expand sidebar"
                    } else {
                        "Collapse sidebar"
                    })
                    .disabled(self.context.is_none())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.preferences.sidebar_collapsed = !this.preferences.sidebar_collapsed;
                        this.save_preferences(window, cx);
                        cx.notify();
                    }))
                    .tooltip(if collapsed {
                        "Expand sidebar"
                    } else {
                        "Collapse sidebar"
                    })
                    .tooltip_placement(Placement::Bottom),
            );
        let title = h_flex()
            .flex_1()
            .min_w_0()
            .gap_2()
            .pl(px(8.))
            .pr(px(24.))
            .when(!self.history.is_empty(), |view| {
                view.child(
                    Button::new("back")
                        .icon(IconName::ArrowLeft)
                        .ghost()
                        .accessibility_label("Back")
                        .disabled(locked)
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.locked() {
                                return;
                            }
                            if let Some(page) = this.history.pop() {
                                this.page = page;
                                this.mount_page(window, cx);
                            }
                        }))
                        .tooltip("Back")
                        .tooltip_placement(Placement::Bottom),
                )
            })
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .truncate()
                    .child(self.page.title()),
            );
        let desktop = cx.entity().downgrade();
        let controls = h_flex()
            .flex_shrink_0()
            .gap_1()
            .pr(px(12.))
            .when(self.page.has_year(), |view| {
                view.child(
                    Button::new("reporting-year")
                        .label(
                            self.preferences
                                .year
                                .map_or_else(|| "All years".to_string(), |year| year.to_string()),
                        )
                        .dropdown_caret(true)
                        .outline()
                        .min_w(px(120.))
                        .accessibility_label("Reporting year")
                        .disabled(locked || self.status.loading || self.context.is_none())
                        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, cx| {
                            if let Some(desktop) = desktop.upgrade() {
                                desktop.update(cx, |this, cx| this.year_menu(menu, cx))
                            } else {
                                menu
                            }
                        }),
                )
            })
            .child(
                Button::new("refresh")
                    .icon(IconName::RefreshCw)
                    .ghost()
                    .accessibility_label("Refresh current page")
                    .disabled(locked || self.status.loading)
                    .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx)))
                    .tooltip("Refresh current page")
                    .tooltip_placement(Placement::Bottom),
            );
        let toolbar = h_flex()
            .size_full()
            .child(leading)
            .child(title)
            .child(controls);

        #[cfg(target_os = "macos")]
        if integrated_titlebar {
            return TitleBar::new()
                .h(TOOLBAR_HEIGHT)
                .pl(px(96.))
                .pr_0()
                .border_b_0()
                .bg(cx.theme().title_bar)
                .child(toolbar)
                .into_any_element();
        }

        // Fullscreen keeps the same single header flush with the viewport top.
        // AppKit owns the temporary system-bar reveal; it gets no permanent
        // space in our layout. Windowed mode restores the traffic-light inset.
        div()
            .h(TOOLBAR_HEIGHT)
            .flex_shrink_0()
            .bg(cx.theme().title_bar)
            .child(toolbar)
            .into_any_element()
    }

    fn year_menu(&self, mut menu: PopupMenu, cx: &mut Context<Self>) -> PopupMenu {
        let mut years = self.years.clone();
        if let Some(year) = self.preferences.year
            && !years.contains(&year)
        {
            years.push(year);
        }
        years.sort_unstable_by(|a, b| b.cmp(a));
        for year in std::iter::once(None).chain(years.into_iter().map(Some)) {
            menu = menu.item(
                PopupMenuItem::new(
                    year.map_or_else(|| "All years".to_string(), |year| year.to_string()),
                )
                .checked(self.preferences.year == year)
                .on_click(cx.listener(move |this, _, window, cx| {
                    if this.locked() || this.preferences.year == year {
                        return;
                    }
                    this.preferences.year = year;
                    this.save_preferences(window, cx);
                    this.mount_page(window, cx);
                    cx.notify();
                })),
            );
        }
        menu.separator()
            .item(PopupMenuItem::new("Manage years…").on_click(cx.listener(
                |this, _, window, cx| {
                    if this.locked() || this.status.loading {
                        return;
                    }
                    // Open after the dropdown closes so dialog focus is preserved.
                    cx.defer_in(window, |this, window, cx| {
                        this.open_year_manager(window, cx)
                    });
                },
            )))
            .scrollable(true)
    }

    fn sidebar_width(progress: f32) -> Pixels {
        RAIL_WIDTH + (SIDEBAR_WIDTH - RAIL_WIDTH) * progress
    }

    fn navigation_button(&self, page: Page, progress: f32, cx: &mut Context<Self>) -> AnyElement {
        let collapsed = self.preferences.sidebar_collapsed;
        let icon = match page {
            Page::Home => IconName::House,
            Page::Imports => IconName::Upload,
            Page::Dividends => IconName::Coins,
            Page::Interests => IconName::Percent,
            Page::Crypto => IconName::Bitcoin,
            Page::Rates => IconName::ChartLine,
            Page::Settings => IconName::Settings,
        };
        Button::new(SharedString::from(format!("navigate-{}", page.title())))
            .ghost()
            .selected(self.page == page)
            .when(self.page == page, |button| {
                button.bg(cx.theme().sidebar_accent).font_medium()
            })
            .w_full()
            .h(NAV_BUTTON_SIZE)
            .px(px(10.))
            .child(
                h_flex()
                    .w_full()
                    .overflow_hidden()
                    .gap(px(10.))
                    .child(div().flex_shrink_0().child(Icon::new(icon).size(px(16.))))
                    .when(progress > 0., |view| {
                        view.child(
                            div()
                                .whitespace_nowrap()
                                .opacity(progress)
                                .child(page.title()),
                        )
                    }),
            )
            .accessibility_label(page.title())
            .disabled(self.locked() || self.context.is_none())
            .on_click(cx.listener(move |this, _, window, cx| this.navigate(page, window, cx)))
            .when(collapsed, |button| {
                button
                    .tooltip(page.title())
                    .tooltip_placement(Placement::Right)
            })
            .into_any_element()
    }

    pub fn navigation_panel(&self, progress: f32, cx: &mut Context<Self>) -> Div {
        let inner_width = Self::sidebar_width(progress) - NAV_PADDING * 2.;
        let footer_step = NAV_BUTTON_SIZE + NAV_GAP;
        let footer_top = NAV_PADDING + px(1.);
        let theme_button = Button::new("theme")
            .icon(if cx.theme().is_dark() {
                IconName::Moon
            } else {
                IconName::Sun
            })
            .ghost()
            .size(NAV_BUTTON_SIZE)
            .accessibility_label("Toggle light and dark theme")
            .tooltip(if cx.theme().is_dark() {
                "Switch to light theme"
            } else {
                "Switch to dark theme"
            })
            .tooltip_placement(Placement::Right)
            .disabled(self.context.is_none())
            .on_click(cx.listener(|this, _, window, cx| {
                this.preferences.dark = Some(!cx.theme().is_dark());
                apply_theme(this.preferences.dark, window, cx);
                this.save_preferences(window, cx);
                cx.notify();
            }));
        v_flex()
            .w(Self::sidebar_width(progress))
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .bg(cx.theme().sidebar.mix_oklab(cx.theme().title_bar, progress))
            .p(NAV_PADDING)
            .child(
                v_flex()
                    .id("sidebar-navigation")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .gap(NAV_GAP)
                    .children(
                        Page::ALL
                            .into_iter()
                            .filter(|page| *page != Page::Settings)
                            .map(|page| self.navigation_button(page, progress, cx)),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_shrink_0()
                    .h(footer_top + NAV_BUTTON_SIZE + footer_step * (1. - progress))
                    .mt_2()
                    .border_t_1()
                    .border_color(cx.theme().sidebar_border)
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .top(footer_top)
                            .min_w_0()
                            .w(inner_width - footer_step * progress)
                            .child(self.navigation_button(Page::Settings, progress, cx)),
                    )
                    .child(
                        div()
                            .absolute()
                            .left((inner_width - NAV_BUTTON_SIZE) * progress)
                            .top(footer_top + footer_step * (1. - progress))
                            .child(theme_button),
                    ),
            )
    }
}

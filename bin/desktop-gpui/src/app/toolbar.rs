use super::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{
        button::*,
        menu::{DropdownMenu, PopupMenu, PopupMenuItem},
    },
};

pub(super) const TOOLBAR_HEIGHT: Pixels = px(64.);
const SIDEBAR_WIDTH: Pixels = px(230.);
const RAIL_WIDTH: Pixels = px(68.);

impl Desktop {
    pub fn toolbar(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let locked = self.locked();
        let collapsed = self.preferences.sidebar_collapsed;
        let fullscreen = window.is_fullscreen() || window.is_simple_fullscreen();
        let integrated_titlebar = cfg!(target_os = "macos") && !fullscreen;
        // Expanded navigation and the page title share the same vertical edge.
        // The collapsed header still reserves room for the native window controls.
        let leading_width = if integrated_titlebar {
            if collapsed {
                px(52.)
            } else {
                SIDEBAR_WIDTH - px(96.)
            }
        } else {
            self.sidebar_width()
        };
        let leading = h_flex()
            .h_full()
            .w(leading_width)
            .flex_shrink_0()
            .px_3()
            .when(!collapsed, |view| {
                view.border_r_1().border_color(cx.theme().sidebar_border)
            })
            .child(
                Button::new("sidebar-toggle")
                    .icon(IconName::PanelLeft)
                    .ghost()
                    .tooltip(if collapsed {
                        "Expand sidebar"
                    } else {
                        "Collapse sidebar"
                    })
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
                    })),
            );
        let title = h_flex()
            .flex_1()
            .min_w_0()
            .gap_2()
            .px(px(24.))
            .when(!self.history.is_empty(), |view| {
                view.child(
                    Button::new("back")
                        .icon(IconName::ArrowLeft)
                        .ghost()
                        .tooltip("Back")
                        .accessibility_label("Back")
                        .disabled(locked)
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.locked() {
                                return;
                            }
                            if let Some(page) = this.history.pop() {
                                this.page = page;
                                this.mount_page(window, cx);
                                this.load_years(window, cx);
                            }
                        })),
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
            .gap_2()
            .pr(px(24.))
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
                    .tooltip("Refresh current page")
                    .accessibility_label("Refresh current page")
                    .disabled(locked || self.status.loading)
                    .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
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
                .border_b_1()
                .border_color(cx.theme().sidebar_border)
                .bg(cx.theme().sidebar)
                .child(toolbar)
                .into_any_element();
        }

        // Fullscreen keeps the same single header flush with the viewport top.
        // AppKit owns the temporary system-bar reveal; it gets no permanent
        // space in our layout. Windowed mode restores the traffic-light inset.
        div()
            .h(TOOLBAR_HEIGHT)
            .flex_shrink_0()
            .border_b_1()
            .border_color(cx.theme().sidebar_border)
            .bg(cx.theme().sidebar)
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

    fn sidebar_width(&self) -> Pixels {
        if self.preferences.sidebar_collapsed {
            RAIL_WIDTH
        } else {
            SIDEBAR_WIDTH
        }
    }

    fn navigation_button(&self, page: Page, cx: &mut Context<Self>) -> Button {
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
            .h(px(40.))
            .px(px(12.))
            .child(
                h_flex()
                    .w_full()
                    .gap(px(12.))
                    .when(collapsed, |view| view.justify_center())
                    .child(Icon::new(icon).size(px(18.)))
                    .when(!collapsed, |view| view.child(page.title())),
            )
            .when(collapsed, |button| button.tooltip(page.title()))
            .accessibility_label(page.title())
            .disabled(self.locked() || self.context.is_none())
            .on_click(cx.listener(move |this, _, window, cx| this.navigate(page, window, cx)))
    }

    pub fn navigation_panel(&self, cx: &mut Context<Self>) -> Div {
        let collapsed = self.preferences.sidebar_collapsed;
        let theme_button = Button::new("theme")
            .icon(if cx.theme().is_dark() {
                IconName::Moon
            } else {
                IconName::Sun
            })
            .ghost()
            .size(px(40.))
            .tooltip(if cx.theme().is_dark() {
                "Switch to light theme"
            } else {
                "Switch to dark theme"
            })
            .accessibility_label("Toggle light and dark theme")
            .disabled(self.context.is_none())
            .on_click(cx.listener(|this, _, window, cx| {
                let dark = !cx.theme().is_dark();
                Theme::change(
                    if dark {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    },
                    Some(window),
                    cx,
                );
                configure_theme(cx);
                this.preferences.dark = Some(dark);
                this.save_preferences(window, cx);
                cx.notify();
            }));
        v_flex()
            .w(self.sidebar_width())
            .h_full()
            .flex_shrink_0()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().sidebar_border)
            .p(px(12.))
            .child(
                v_flex()
                    .id("sidebar-navigation")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .gap(px(6.))
                    .children(
                        Page::ALL
                            .into_iter()
                            .filter(|page| *page != Page::Settings)
                            .map(|page| self.navigation_button(page, cx)),
                    ),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .when(collapsed, |view| view.flex_col())
                    .gap_2()
                    .pt_3()
                    .mt_3()
                    .border_t_1()
                    .border_color(cx.theme().sidebar_border)
                    .child(
                        div()
                            .min_w_0()
                            .when(!collapsed, |view| view.flex_1())
                            .when(collapsed, |view| view.w_full())
                            .child(self.navigation_button(Page::Settings, cx)),
                    )
                    .child(theme_button),
            )
    }
}

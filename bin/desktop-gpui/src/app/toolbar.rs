use super::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{
        button::*,
        sidebar::{SidebarItem, SidebarMenu, SidebarMenuItem},
    },
};

impl Desktop {
    pub fn toolbar(&self, cx: &mut Context<Self>) -> Div {
        let locked = self.locked();
        let navigation = h_flex()
            .p_1()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background.opacity(0.95))
            .gap_2()
            .flex_shrink_0()
            .child(
                Button::new("menu")
                    .icon(IconName::Menu)
                    .outline()
                    .accessibility_label("Toggle navigation menu")
                    .disabled(locked)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.menu_open = !this.menu_open;
                        if this.menu_open {
                            window.focus(&this.navigation_focus, cx);
                        }
                        cx.notify();
                    })),
            )
            .when(!self.history.is_empty(), |bar| {
                bar.child(
                    Button::new("back")
                        .icon(IconName::ArrowLeft)
                        .outline()
                        .accessibility_label("Back")
                        .disabled(locked)
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.locked() {
                                return;
                            }
                            if let Some(page) = this.history.pop() {
                                this.page = page;
                                this.menu_open = false;
                                this.mount_page(window, cx);
                                this.load_years(window, cx);
                            }
                        })),
                )
            })
            .child(
                Button::new("theme")
                    .icon(if cx.theme().is_dark() {
                        IconName::Sun
                    } else {
                        IconName::Moon
                    })
                    .outline()
                    .ghost()
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
                    })),
            )
            .child(
                div()
                    .text_lg()
                    .font_semibold()
                    .px_2()
                    .child(self.page.title()),
            );
        let controls = h_flex()
            .p_1()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background.opacity(0.95))
            .gap_2()
            .flex_shrink_0()
            .when(self.page.has_year(), |bar| {
                bar.child(
                    // Select's outer element fills its parent in GPUI Kit 0.7.
                    // Size the host, not just the inner trigger.
                    div().w(px(145.)).h_8().flex_shrink_0().child(
                        Select::new(&self.year_select)
                            .accessibility_label("Reporting year")
                            .disabled(locked || self.context.is_none()),
                    ),
                )
                .child(
                    Button::new("years")
                        .label("Manage years")
                        .outline()
                        .disabled(locked || self.status.loading || self.context.is_none())
                        .on_click(
                            cx.listener(|this, _, window, cx| this.open_year_manager(window, cx)),
                        ),
                )
            })
            .child(
                Button::new("refresh")
                    .icon(IconName::RefreshCw)
                    .outline()
                    .accessibility_label("Refresh current page")
                    .disabled(locked || self.status.loading)
                    .on_click(cx.listener(|this, _, window, cx| this.reload(window, cx))),
            );
        h_flex()
            .flex_shrink_0()
            .flex_wrap()
            .justify_between()
            .gap_3()
            .p_4()
            .child(navigation)
            .child(controls)
    }

    pub fn navigation_panel(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let menu = SidebarMenu::new().children(Page::ALL.into_iter().map(|page| {
            SidebarMenuItem::new(page.title())
                .icon(match page {
                    Page::Home => IconName::House,
                    Page::Imports => IconName::Upload,
                    Page::Dividends => IconName::Coins,
                    Page::Interests => IconName::Percent,
                    Page::Crypto => IconName::Bitcoin,
                    Page::Rates => IconName::ChartLine,
                    Page::Settings => IconName::Settings,
                })
                .active(self.page == page)
                .disable(self.locked() || self.context.is_none())
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.menu_open = false;
                    this.navigate(page, window, cx);
                    cx.notify();
                }))
        }));
        div()
            .absolute()
            .inset_0()
            .child(
                div()
                    .id("navigation-backdrop")
                    .absolute()
                    .inset_0()
                    .bg(cx.theme().background.opacity(0.4))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.menu_open = false;
                        cx.notify();
                    })),
            )
            .child(
                v_flex()
                    .id("navigation-panel")
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left_0()
                    .w(px(280.))
                    .pt_16()
                    .px_3()
                    .pb_3()
                    .bg(cx.theme().background)
                    .border_r_1()
                    .border_color(cx.theme().border)
                    .overflow_y_scroll()
                    .child(menu.render("navigation", window, cx)),
            )
    }
}

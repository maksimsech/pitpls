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
            .gap_2()
            .flex_shrink_0()
            .child(
                Button::new("menu")
                    .icon(IconName::Menu)
                    .outline()
                    .accessibility_label("Toggle navigation menu")
                    .disabled(locked)
                    .on_click(cx.listener(|this, _, window, cx| this.open_navigation(window, cx))),
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
                                this.manage_years = false;
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
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.manage_years = !this.manage_years;
                            cx.notify();
                        })),
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
            .border_b_1()
            .border_color(cx.theme().border)
            .child(navigation)
            .child(controls)
    }

    fn open_navigation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        let page = cx.entity().downgrade();
        window.open_sheet_at(Placement::Left, cx, move |sheet, window, cx| {
            let Ok(menu) = page.update(cx, |this, cx| {
                SidebarMenu::new().children(Page::ALL.into_iter().map(|page| {
                    SidebarMenuItem::new(page.title())
                        .active(this.page == page)
                        .suffix({
                            let desktop = cx.entity().downgrade();
                            let disabled = this.locked() || this.context.is_none();
                            move |_, _| {
                                Button::new(SharedString::from(format!("open-{}", page.title())))
                                    .icon(IconName::ChevronRight)
                                    .ghost()
                                    .small()
                                    .accessibility_label(format!("Open {}", page.title()))
                                    .disabled(disabled)
                                    .on_click({
                                        let desktop = desktop.clone();
                                        move |_, window, cx| {
                                            cx.stop_propagation();
                                            window.close_sheet(cx);
                                            let _ = desktop.update(cx, |this, cx| {
                                                this.navigate(page, window, cx)
                                            });
                                        }
                                    })
                            }
                        })
                        .disable(this.locked() || this.context.is_none())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            window.close_sheet(cx);
                            this.navigate(page, window, cx);
                        }))
                }))
            }) else {
                return sheet;
            };
            sheet
                .title("pitpls")
                .size(px(280.))
                .child(menu.render("navigation", window, cx))
        });
    }
}

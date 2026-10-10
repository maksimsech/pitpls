mod sidebar;
mod years;

use crate::{
    components::{ButtonText, Status, form, header, notice, spinner},
    config::{self, Config, Preferences},
    navigation::{Page, PageContext, PageEvent, PageEvents},
    pages,
    services::{Services, finish},
    theme::{apply_theme, configure_theme, palette},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    base::{Transition, transition},
    component::{
        button::{Button, ButtonVariants},
        input::{InputEvent, InputState},
        *,
    },
    *,
};
use pitpls_app::use_case::year::YearInfo;
use sidebar::Expansion;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct Desktop {
    config: Arc<Config>,
    runtime: Arc<Runtime>,
    context: Option<PageContext>,
    events: Entity<PageEvents>,
    page: Page,
    active: Option<AnyView>,
    page_locked: bool,
    preferences: Preferences,
    preference_task: Option<Task<()>>,
    status: Status,
    years: Vec<YearInfo>,
    years_task: Option<Task<()>>,
    year_menu_open: bool,
    year_input: Entity<InputState>,
    year_error: Option<SharedString>,
    year_menu_focus: FocusHandle,
    focus: FocusHandle,
    _subscriptions: [Subscription; 4],
}

impl Desktop {
    pub fn new(
        config: Arc<Config>,
        runtime: Arc<Runtime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let events = cx.new(|_| PageEvents);
        let page_subscription = cx.subscribe_in(&events, window, |this, _, event, window, cx| {
            match event {
                PageEvent::LockNavigation(locked) => {
                    if this.page_locked == *locked {
                        return;
                    }
                    this.page_locked = *locked;
                    if *locked {
                        this.year_menu_open = false;
                    }
                }
                PageEvent::Navigate(page) => this.navigate(*page, window, cx),
                PageEvent::YearsChanged => this.load_years(window, cx),
                PageEvent::SelectYear(year) => this.select_year(*year, window, cx),
                PageEvent::SetTheme(dark) => this.set_theme(*dark, window, cx),
                PageEvent::Imported(last_import) => {
                    this.preferences.last_import = Some(last_import.clone());
                    this.save_preferences(window, cx);
                }
            }
            cx.notify();
        });
        let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
            if this.preferences.dark.is_none() {
                Theme::sync_system_appearance(Some(window), cx);
                configure_theme(cx);
            }
        });
        let year_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Add year, e.g. 2023"));
        let input_subscription = cx.subscribe_in(
            &year_input,
            window,
            |this, _, event, window, cx| match event {
                InputEvent::PressEnter { .. } => this.add_year(window, cx),
                InputEvent::Change if this.year_error.is_some() => {
                    this.year_error = None;
                    cx.notify();
                }
                _ => {}
            },
        );
        // When the focused element goes away (its page is replaced, its
        // virtual row scrolls out of view), focus moves to its nearest
        // focusable ancestor, such as the page, or else to the root, so Tab
        // keeps working. An open dialog keeps focus to itself.
        let focus_subscription = cx.on_focus_lost(window, |this, window, cx| {
            match window.focus_lost_restore_target(cx) {
                Some(target) => window.focus(&target, cx),
                None if !window.has_active_dialog(cx) => window.focus(&this.focus, cx),
                None => {}
            }
        });
        let mut view = Self {
            config,
            runtime,
            context: None,
            events,
            page: Page::Home,
            active: None,
            page_locked: false,
            preferences: Preferences::default(),
            preference_task: None,
            status: Status::default(),
            years: vec![],
            years_task: None,
            year_menu_open: false,
            year_input,
            year_error: None,
            year_menu_focus: cx.focus_handle(),
            focus: cx.focus_handle(),
            _subscriptions: [
                page_subscription,
                appearance_subscription,
                input_subscription,
                focus_subscription,
            ],
        };
        // So Tab works before the first click.
        window.focus(&view.focus, cx);
        view.connect(window, cx);
        view
    }

    fn locked(&self) -> bool {
        self.status.busy || self.page_locked
    }

    fn connect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status.begin_save();
        let config = self.config.clone();
        let job = self.runtime.spawn(async move {
            let db = pitpls_db::Database::open(&config.database).await?;
            let (preferences, warning) = match config::read_preferences(&config.preferences).await {
                Ok(value) => (value, None),
                Err(error) => (Preferences::default(), Some(error)),
            };
            Ok::<_, pitpls_db::OpenDatabaseError>((
                Arc::new(pitpls_app::App::new(db)),
                preferences,
                warning,
            ))
        });
        self.status.task = Some(cx.spawn_in(window, async move |this, cx| {
            let result = finish(job).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.status.busy = false;
                match result {
                    Ok((app, preferences, warning)) => {
                        this.context = Some(PageContext::new(
                            Services {
                                app,
                                runtime: this.runtime.clone(),
                            },
                            this.events.clone(),
                        ));
                        this.preferences = preferences;
                        this.status.message = warning.map(Into::into);
                        apply_theme(this.preferences.dark, window, cx);
                        this.mount_page(window, cx);
                        this.load_years(window, cx);
                        if !this.preferences.notice_accepted {
                            this.open_notice(window, cx);
                        }
                    }
                    Err(error) => this.status.error = Some(error.into()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn mount_page(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(context) = self.context.clone() else {
            return;
        };
        self.page_locked = false;
        self.active = Some(pages::open(
            self.page,
            context,
            &self.preferences,
            window,
            cx,
        ));
        cx.notify();
    }

    fn navigate(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        if page != self.page {
            self.page = page;
            self.mount_page(window, cx);
        }
        cx.notify();
    }

    fn open_notice(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let view = view.clone();
            notice::first_launch(dialog, cx).on_ok(move |_, window, cx| {
                view.update(cx, |this, cx| {
                    this.preferences.notice_accepted = true;
                    this.save_preferences(window, cx);
                })
                .is_ok()
            })
        });
    }

    fn set_theme(&mut self, dark: Option<bool>, window: &mut Window, cx: &mut Context<Self>) {
        self.preferences.dark = dark;
        apply_theme(dark, window, cx);
        self.save_preferences(window, cx);
        cx.notify();
    }

    fn save_preferences(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let previous = self.preference_task.take();
        let runtime = self.runtime.clone();
        let path = self.config.preferences.clone();
        let preferences = self.preferences.clone();
        // Chain saves so that quick successive changes are written in order.
        self.preference_task = Some(cx.spawn_in(window, async move |this, cx| {
            if let Some(previous) = previous {
                previous.await;
            }
            let result = finish(runtime.spawn(config::save_preferences(path, preferences))).await;
            if let Err(error) = result {
                let _ = this.update_in(cx, |this, _, cx| {
                    this.status.error = Some(error.into());
                    cx.notify();
                });
            }
        }));
    }

    fn main_panel(&self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let panel = v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .mt(px(6.))
            .mr(px(6.))
            .mb(px(6.))
            .overflow_hidden()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(10.));
        if self.context.is_none() {
            return panel
                .child(header::page("Database connection", None, window, cx))
                .child(
                    v_flex()
                        .px(px(20.))
                        .pb_5()
                        .gap_4()
                        .child(
                            h_flex()
                                .gap_2()
                                .text_color(cx.theme().muted_foreground)
                                .child(div().min_w_0().child(
                                    "Your records will appear when the database connection is \
                                     available.",
                                ))
                                .when(self.status.busy, |line| {
                                    line.child(div().flex_shrink_0().child(spinner()))
                                }),
                        )
                        .when(self.status.is_visible(), |view| {
                            view.child(self.status.render())
                        })
                        .when(!self.status.busy, |view| {
                            view.child(
                                h_flex().child(
                                    Button::new("retry-connection")
                                        .text_label("Retry connection")
                                        .primary()
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.connect(window, cx)
                                        })),
                                ),
                            )
                        }),
                );
        }
        panel
            .when_some(self.active.clone(), |panel, page| {
                panel.child(v_flex().flex_1().min_h_0().min_w_0().child(page))
            })
            .when(self.status.is_visible(), |panel| {
                panel.child(
                    div()
                        .flex_shrink_0()
                        .px(px(20.))
                        .py_3()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child(self.status.render()),
                )
            })
    }
}

impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let expanded = !self.preferences.sidebar_collapsed;
        // Animate only once preferences are restored, so the saved state
        // doesn't animate in.
        let expansion = if self.context.is_some() {
            Expansion::animate(expanded, window, cx)
        } else {
            Expansion::settled(expanded)
        };
        h_flex()
            .id("desktop")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .items_stretch()
            .overflow_hidden()
            .bg(palette(cx).chrome)
            .text_color(cx.theme().foreground)
            .font_family(cx.theme().font_family.clone())
            .text_sm()
            .child(self.sidebar(expansion, window, cx))
            .child(self.main_panel(window, cx))
    }
}

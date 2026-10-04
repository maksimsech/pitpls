mod toolbar;
mod years;

use crate::{
    components::{self, Status, form},
    config::{self, Config, Preferences},
    navigation::{Page, PageContext, PageEvent, PageEvents},
    pages::{self, PageHandle},
    services::{Services, finish},
    theme::configure_theme,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    base::{Transition, transition},
    component::{button::ButtonVariants, input::InputState, *},
    *,
};
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct Desktop {
    config: Arc<Config>,
    runtime: Arc<Runtime>,
    context: Option<PageContext>,
    events: Entity<PageEvents>,
    page: Page,
    active: Option<PageHandle>,
    history: Vec<Page>,
    page_locked: bool,
    preferences: Preferences,
    preference_task: Option<Task<()>>,
    status: Status,
    years: Vec<i32>,
    manage_years: bool,
    navigation_focus: FocusHandle,
    hover_labels: Entity<gpui_kit::base::TooltipOverlay>,
    year_form: Option<Entity<InputState>>,
    delete_year: Option<i32>,
    return_focus: Option<FocusHandle>,
    _subscriptions: Vec<Subscription>,
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
                }
                PageEvent::Navigate(page) => this.navigate(*page, window, cx),
            }
            cx.notify();
        });
        let mut view = Self {
            config,
            runtime,
            context: None,
            events,
            page: Page::Home,
            active: None,
            history: vec![],
            page_locked: false,
            preferences: Preferences::default(),
            preference_task: None,
            status: Status::default(),
            years: vec![],
            manage_years: false,
            navigation_focus: cx.focus_handle(),
            hover_labels: cx.new(|_| gpui_kit::base::TooltipOverlay::new()),
            year_form: None,
            delete_year: None,
            return_focus: None,
            _subscriptions: vec![page_subscription],
        };
        #[cfg(target_os = "macos")]
        {
            crate::window_chrome::sync_fullscreen_title(window);
            view._subscriptions
                .push(cx.observe_window_bounds(window, |_, window, cx| {
                    // Apply after AppKit/GPUI finish updating the native window.
                    cx.defer_in(window, |_, window, _| {
                        crate::window_chrome::sync_fullscreen_title(window);
                    });
                }));
        }
        view.connect(window, cx);
        view
    }

    fn locked(&self) -> bool {
        self.status.busy
            || self.manage_years
            || self.page_locked
            || self.year_form.is_some()
            || self.delete_year.is_some()
    }

    fn connect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.status.begin_save();
        let config = self.config.clone();
        let job = self.runtime.spawn(async move {
            let db = pitpls_db::Database::open(&config.database)
                .await
                .map_err(|e| e.to_string())?;
            let (preferences, warning) = match config::read_preferences(&config.preferences).await {
                Ok(value) => (value, None),
                Err(error) => (Preferences::default(), Some(error)),
            };
            Ok((Arc::new(pitpls_app::App::new(db)), preferences, warning))
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
                        if let Some(dark) = preferences.dark {
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
                        }
                        this.mount_page(window, cx);
                        this.load_years(window, cx);
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
        // Replacing the page drops its tasks and controls. Late results cannot
        // update the entity for a different route or reporting year.
        self.active = Some(pages::open(
            self.page,
            context,
            self.preferences.year,
            window,
            cx,
        ));
        cx.notify();
    }

    fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        if self.context.is_none() {
            self.connect(window, cx);
            return;
        }
        self.load_years(window, cx);
        if let Some(page) = &self.active {
            page.refresh(window, cx);
        }
    }

    fn navigate(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        if self.locked() {
            return;
        }
        if page != self.page {
            self.history.push(self.page);
            self.page = page;
            self.manage_years = false;
            self.mount_page(window, cx);
            self.load_years(window, cx);
        }
        cx.notify();
    }

    fn save_preferences(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let previous = self.preference_task.take();
        let runtime = self.runtime.clone();
        let path = self.config.preferences.clone();
        let preferences = self.preferences;
        // Preserve write order when theme or reporting year changes quickly.
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
}

impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let target = if self.preferences.sidebar_collapsed {
            0.
        } else {
            1.
        };
        // Initialize from restored preferences; only user changes animate.
        // One shared value keeps the header, content edge, and labels in sync.
        let sidebar_progress = if self.context.is_some() {
            let motion = cx.theme().motion_tokens();
            transition(
                "desktop-sidebar-expansion",
                target,
                Transition::new(motion.duration_normal).easing(motion.easing_move.clone()),
                window,
                cx,
            )
        } else {
            target
        };
        // Keep one bounded content region in every state, including startup and
        // connection failures, so page content cannot displace the window chrome.
        let mut content = v_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            // The frame starts to the right of navigation in both sidebar states.
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(12.));
        if self.context.is_none() {
            content = content.child(components::scroll(
                v_flex()
                    .p_5()
                    .gap_4()
                    .child(div().text_lg().font_semibold().child("Database connection"))
                    .child(div().text_color(cx.theme().muted_foreground).child(
                        "Your records will appear when the database connection is available.",
                    ))
                    .child(self.status.render(cx))
                    .when(!self.status.busy, |view| {
                        view.child(
                            h_flex().child(
                                gpui_kit::component::button::Button::new("retry-connection")
                                    .label("Retry connection")
                                    .primary()
                                    .on_click(
                                        cx.listener(|this, _, window, cx| this.connect(window, cx)),
                                    ),
                            ),
                        )
                    }),
            ));
        } else {
            content = content
                .when(self.status.is_visible(), |view| {
                    view.child(
                        div()
                            .flex_shrink_0()
                            .px_5()
                            .pt_4()
                            .child(self.status.render(cx)),
                    )
                })
                .when_some(self.active.as_ref(), |view, page| {
                    view.child(page.view.clone())
                });
        }
        v_flex()
            .id("desktop")
            .track_focus(&self.navigation_focus)
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().title_bar)
            .text_color(cx.theme().foreground)
            .font_family(cx.theme().font_family.clone())
            .text_sm()
            .child(self.toolbar(sidebar_progress, window, cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .mr(px(4.))
                    .mb(px(4.))
                    .overflow_hidden()
                    .items_stretch()
                    .child(self.navigation_panel(sidebar_progress, cx))
                    .child(content),
            )
            .child(self.hover_labels.clone())
    }
}

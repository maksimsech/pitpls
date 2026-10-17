use crate::{config::LastImport, i18n::Language};
use gpui_kit::{App, Context, Entity, EntityId, EventEmitter, assets::IconName};
use rust_i18n::t;
use std::borrow::Cow;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Home,
    Imports,
    Dividends,
    Interests,
    Crypto,
    Stocks,
    Rates,
    Settings,
}

impl Page {
    pub fn title(self) -> Cow<'static, str> {
        match self {
            Self::Home => t!("nav.summary"),
            Self::Imports => t!("nav.imports"),
            Self::Dividends => t!("nav.dividends"),
            Self::Interests => t!("nav.interests"),
            Self::Crypto => t!("nav.crypto"),
            Self::Stocks => t!("nav.stocks"),
            Self::Rates => t!("nav.rates"),
            Self::Settings => t!("nav.settings"),
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Self::Home => IconName::House,
            Self::Imports => IconName::Upload,
            Self::Dividends => IconName::Coins,
            Self::Interests => IconName::Percent,
            Self::Crypto => IconName::Bitcoin,
            Self::Stocks => IconName::TrendingUp,
            Self::Rates => IconName::ChartLine,
            Self::Settings => IconName::Settings,
        }
    }
}

pub enum PageEvent {
    Navigate(Page),
    LockNavigation(bool),
    YearsChanged,
    SelectYear(Option<i32>),
    /// `None` follows the system.
    SetTheme(Option<bool>),
    /// `None` follows the system.
    SetLanguage(Option<Language>),
    Imported(LastImport),
    Loaded(EntityId),
}

pub struct PageEvents;
impl EventEmitter<PageEvent> for PageEvents {}

#[derive(Clone)]
pub struct PageContext {
    pub services: crate::services::Services,
    events: Entity<PageEvents>,
}

impl PageContext {
    pub fn new(services: crate::services::Services, events: Entity<PageEvents>) -> Self {
        Self { services, events }
    }

    pub fn set_locked(&self, locked: bool, cx: &mut App) {
        self.events
            .update(cx, |_, cx| cx.emit(PageEvent::LockNavigation(locked)));
    }

    pub fn navigate(&self, page: Page, cx: &mut App) {
        self.events
            .update(cx, |_, cx| cx.emit(PageEvent::Navigate(page)));
    }

    pub fn years_changed(&self, cx: &mut App) {
        self.events
            .update(cx, |_, cx| cx.emit(PageEvent::YearsChanged));
    }

    pub fn select_year(&self, year: Option<i32>, cx: &mut App) {
        self.events
            .update(cx, |_, cx| cx.emit(PageEvent::SelectYear(year)));
    }

    pub fn set_theme(&self, dark: Option<bool>, cx: &mut App) {
        self.events
            .update(cx, |_, cx| cx.emit(PageEvent::SetTheme(dark)));
    }

    pub fn set_language(&self, language: Option<Language>, cx: &mut App) {
        self.events
            .update(cx, |_, cx| cx.emit(PageEvent::SetLanguage(language)));
    }

    pub fn imported(&self, last_import: LastImport, cx: &mut App) {
        self.events
            .update(cx, |_, cx| cx.emit(PageEvent::Imported(last_import)));
    }

    pub fn loaded<V: 'static>(&self, cx: &mut Context<V>) {
        let page = cx.entity_id();
        self.events
            .update(cx, |_, cx| cx.emit(PageEvent::Loaded(page)));
    }
}

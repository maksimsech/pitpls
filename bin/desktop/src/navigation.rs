use crate::config::LastImport;
use gpui_kit::{App, Context, Entity, EntityId, EventEmitter, assets::IconName};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Home,
    Imports,
    Dividends,
    Interests,
    Crypto,
    Rates,
    Settings,
}

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Self::Home => "Summary",
            Self::Imports => "Imports",
            Self::Dividends => "Dividends",
            Self::Interests => "Interests",
            Self::Crypto => "Crypto",
            Self::Rates => "Rates",
            Self::Settings => "Settings",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            Self::Home => IconName::House,
            Self::Imports => IconName::Upload,
            Self::Dividends => IconName::Coins,
            Self::Interests => IconName::Percent,
            Self::Crypto => IconName::Bitcoin,
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

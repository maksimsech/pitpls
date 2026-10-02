use gpui_kit::{App, Entity, EventEmitter};

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
    pub const ALL: [Self; 7] = [
        Self::Home,
        Self::Imports,
        Self::Dividends,
        Self::Interests,
        Self::Crypto,
        Self::Rates,
        Self::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Imports => "Imports",
            Self::Dividends => "Dividends",
            Self::Interests => "Interests",
            Self::Crypto => "Crypto",
            Self::Rates => "Rates",
            Self::Settings => "Settings",
        }
    }

    pub fn has_year(self) -> bool {
        matches!(
            self,
            Self::Home | Self::Dividends | Self::Interests | Self::Crypto
        )
    }
}

pub enum PageEvent {
    Navigate(Page),
    LockNavigation(bool),
}

/// An explicit window-owned event channel; pages never hold the app shell.
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
}

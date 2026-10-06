mod home;
mod imports;
mod rates;
mod records;
mod settings;

use crate::navigation::{Page, PageContext};
use gpui_kit::*;
use records::{Crypto, Dividends, Interests, RecordsPage};

/// A page reloads its data itself, from its header's Refresh button.
pub trait PageView: Render {
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>);
}

pub struct PageHandle {
    pub view: AnyView,
}

impl PageHandle {
    fn new<V: PageView>(entity: Entity<V>) -> Self {
        Self {
            view: entity.into(),
        }
    }
}

pub fn open(
    page: Page,
    context: PageContext,
    year: Option<i32>,
    window: &mut Window,
    cx: &mut App,
) -> PageHandle {
    match page {
        Page::Home => PageHandle::new(cx.new(|cx| home::HomePage::new(context, year, window, cx))),
        Page::Imports => {
            PageHandle::new(cx.new(|cx| imports::ImportsPage::new(context, window, cx)))
        }
        Page::Dividends => {
            PageHandle::new(cx.new(|cx| RecordsPage::<Dividends>::new(context, year, window, cx)))
        }
        Page::Interests => {
            PageHandle::new(cx.new(|cx| RecordsPage::<Interests>::new(context, year, window, cx)))
        }
        Page::Crypto => {
            PageHandle::new(cx.new(|cx| RecordsPage::<Crypto>::new(context, year, window, cx)))
        }
        Page::Rates => PageHandle::new(cx.new(|cx| rates::RatesPage::new(context, window, cx))),
        Page::Settings => {
            PageHandle::new(cx.new(|cx| settings::SettingsPage::new(context, window, cx)))
        }
    }
}

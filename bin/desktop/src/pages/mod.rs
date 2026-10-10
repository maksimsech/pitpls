mod home;
mod imports;
mod rates;
mod records;
mod settings;

use crate::{
    config::Preferences,
    navigation::{Page, PageContext},
};
use gpui_kit::*;
use records::{Crypto, Dividends, Interests, RecordsPage};

pub fn open(
    page: Page,
    context: PageContext,
    preferences: &Preferences,
    window: &mut Window,
    cx: &mut App,
) -> AnyView {
    let year = preferences.year;
    let last_import = preferences.last_import.clone();
    match page {
        Page::Home => cx
            .new(|cx| home::HomePage::new(context, year, last_import, window, cx))
            .into(),
        Page::Imports => cx
            .new(|cx| imports::ImportsPage::new(context, year, last_import, window, cx))
            .into(),
        Page::Dividends => cx
            .new(|cx| RecordsPage::<Dividends>::new(context, year, window, cx))
            .into(),
        Page::Interests => cx
            .new(|cx| RecordsPage::<Interests>::new(context, year, window, cx))
            .into(),
        Page::Crypto => cx
            .new(|cx| RecordsPage::<Crypto>::new(context, year, window, cx))
            .into(),
        Page::Rates => cx
            .new(|cx| rates::RatesPage::new(context, window, cx))
            .into(),
        Page::Settings => cx
            .new(|cx| settings::SettingsPage::new(context, window, cx))
            .into(),
    }
}

mod home;
mod imports;
mod rates;
mod records;
mod settings;
mod stocks;

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
    width: Option<Pixels>,
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
            .new(|cx| RecordsPage::<Dividends>::new(context, year, width, window, cx))
            .into(),
        Page::Interests => cx
            .new(|cx| RecordsPage::<Interests>::new(context, year, width, window, cx))
            .into(),
        Page::Crypto => cx
            .new(|cx| RecordsPage::<Crypto>::new(context, year, width, window, cx))
            .into(),
        Page::Stocks => cx
            .new(|cx| stocks::StocksPage::new(context, year, width, window, cx))
            .into(),
        Page::Rates => cx
            .new(|cx| rates::RatesPage::new(context, window, cx))
            .into(),
        Page::Settings => cx
            .new(|cx| settings::SettingsPage::new(context, window, cx))
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{
        AnyView, AppContext as _, Context, IntoElement, Render, TestAppContext, Window,
    };
    use pitpls_core::{
        common::Country,
        crypto::{Action, Crypto},
        dividend::Dividend,
        interest::Interest,
    };
    use rust_decimal::dec;

    use super::open;
    use crate::{
        config::Preferences,
        fixtures::{self, REVOLUT, T212, on, usd},
        navigation::Page,
    };

    struct Host(AnyView);

    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.0.clone()
        }
    }

    // Every page, for a tax year and for All years, loads without its error state.
    #[gpui_kit::test]
    fn every_page_loads_and_renders_with_records(cx: &mut TestAppContext) {
        let runtime = fixtures::runtime();
        let app = runtime.block_on(async {
            let app = fixtures::stock_app().await;
            let time = on(2025, 5, 15);
            app.db
                .dividend_repo()
                .insert(&Dividend {
                    id: "dividend".to_owned(),
                    time,
                    ticker: "KO".to_owned(),
                    value: usd(dec!(100)),
                    tax_paid: usd(dec!(15)),
                    country: Country::USA,
                    provider: REVOLUT.to_owned(),
                })
                .await
                .unwrap();
            app.db
                .interest_repo()
                .insert(&Interest {
                    id: "interest".to_owned(),
                    time,
                    value: usd(dec!(10)),
                    provider: T212.to_owned(),
                })
                .await
                .unwrap();
            app.db
                .crypto_repo()
                .insert(&Crypto {
                    id: "crypto".to_owned(),
                    value: usd(dec!(50)),
                    fee: usd(dec!(1)),
                    action: Action::FiatSell,
                    time,
                    provider: "Coinbase".to_owned(),
                })
                .await
                .unwrap();
            std::sync::Arc::new(app)
        });

        for year in [Some(2025), None] {
            for page in [
                Page::Home,
                Page::Imports,
                Page::Dividends,
                Page::Interests,
                Page::Crypto,
                Page::Stocks,
                Page::Rates,
                Page::Settings,
            ] {
                let preferences = Preferences {
                    year,
                    ..Default::default()
                };
                let ui = fixtures::open(cx, runtime.clone(), app.clone(), |context, window, cx| {
                    let view = open(page, context, &preferences, None, window, cx);
                    cx.new(|_| Host(view))
                });
                ui.wait_loaded(1, cx);
                let title = page.title();
                for retry in ["retry", "retry-rates"] {
                    assert!(
                        ui.find(retry, cx).is_none(),
                        "{title} for {year:?} shows its error state"
                    );
                }
            }
        }
    }
}

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate};
use pitpls_core::{
    common::Currency,
    crypto::{self, CalculateSellBuyValuesError, CryptoTaxData},
    dividend::{self, CalculateDividendTaxError, DividendTaxData},
    interest::{self, CalculateInterestTaxError, InterestTaxData},
    portfolio::{
        self, CryptoTotals, CryptoYear, DividendPayer, DividendYear, EarnedYear, InterestMonth,
        InterestProvider, InterestYear, Last12Months, Position, RealizedInstrument, RealizedYear,
    },
    rate::{NbpRateProvider, RateConverterError},
    stock::{self, CalculateStocksError, Replay, Stock},
};
use pitpls_db::RepositoryError;
use rust_decimal::Decimal;
use thiserror::Error;

use super::stock::{Listings, load_listings};
use crate::App;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum Error {
    Repository(#[from] RepositoryError),
    Dividend(#[from] CalculateDividendTaxError),
    Interest(#[from] CalculateInterestTaxError),
    Crypto(#[from] CalculateSellBuyValuesError),
    Stock(#[from] CalculateStocksError),
}

impl Error {
    pub fn rate_error(&self) -> Option<&RateConverterError> {
        match self {
            Self::Dividend(
                CalculateDividendTaxError::DividendConversion(error)
                | CalculateDividendTaxError::PaidTaxConversion(error),
            )
            | Self::Interest(CalculateInterestTaxError::InterestConversion(error))
            | Self::Crypto(
                CalculateSellBuyValuesError::ValueConversion(error)
                | CalculateSellBuyValuesError::FeeConversion(error),
            )
            | Self::Stock(
                CalculateStocksError::PriceConversion(error)
                | CalculateStocksError::FeeConversion(error)
                | CalculateStocksError::TaxPaidConversion(error),
            ) => Some(error),
            Self::Repository(_) => None,
        }
    }
}

pub struct PortfolioSummary {
    pub earned: Vec<EarnedYear>,
    pub realized: Decimal,
    pub dividends: Decimal,
    pub interest: Decimal,
    pub sales: usize,
    pub dividend_payments: usize,
    pub interest_records: usize,
    pub crypto_records: usize,
    pub first_sale_year: Option<i32>,
    pub positions: Vec<Position>,
    pub cost_by_currency: Vec<(Currency, Decimal)>,
    pub holding_providers: usize,
    pub listings: Listings,
    pub crypto: CryptoTotals,
    pub crypto_providers: Vec<String>,
    pub current_year: i32,
}

pub struct StockPortfolio {
    pub records: Vec<Stock>,
    pub replay: Replay,
    pub listings: Listings,
    pub positions: Vec<Position>,
    pub realized_years: Vec<RealizedYear>,
    pub realized_instruments: Vec<RealizedInstrument>,
}

pub struct DividendPortfolio {
    pub data: DividendTaxData,
    pub years: Vec<DividendYear>,
    pub payers: Vec<DividendPayer>,
}

pub struct InterestPortfolio {
    pub data: InterestTaxData,
    pub months: Vec<InterestMonth>,
    pub years: Vec<InterestYear>,
    pub providers: Vec<InterestProvider>,
    pub last_12_months: Last12Months,
}

pub struct CryptoPortfolio {
    pub data: CryptoTaxData,
    pub years: Vec<CryptoYear>,
    pub totals: CryptoTotals,
}

pub async fn load_summary(app: &App, today: NaiveDate) -> Result<PortfolioSummary, Error> {
    let rates = NbpRateProvider::new(app.db.rate_repo().load_all().await?);
    let mut dividends = app.db.dividend_repo().get_by_year(None).await?;
    let mut interests = app.db.interest_repo().get_by_year(None).await?;
    let mut cryptos = app.db.crypto_repo().get_by_year(None).await?;
    let stocks = app.db.stock_repo().get_by_year(None).await?;
    let listings = load_listings(app).await?;
    let rounding = app.db.settings_repo().load_dividend_rounding().await?;

    dividends.sort_unstable_by_key(|record| record.time);
    interests.sort_unstable_by_key(|record| record.time);
    cryptos.sort_unstable_by_key(|record| record.time);

    // Run every calculation before choosing an error: the first kind to fail may miss a later year.
    let results = (
        dividend::calculate(dividends, &rates, rounding),
        interest::calculate(interests, &rates),
        crypto::calculate_sell_buy_values(cryptos, &rates),
        stock::replay(&stocks, &listings.exchanges(), &rates),
    );
    let (dividends, interests, cryptos, replay) = match results {
        (Ok(dividends), Ok(interests), Ok(cryptos), Ok(replay)) => {
            (dividends, interests, cryptos, replay)
        }
        (dividends, interests, cryptos, stocks) => {
            return Err([
                dividends.err().map(Error::from),
                interests.err().map(Error::from),
                cryptos.err().map(Error::from),
                stocks.err().map(Error::from),
            ]
            .into_iter()
            .flatten()
            .min_by_key(|error| {
                error
                    .rate_error()
                    .and_then(RateConverterError::missing_year)
                    .unwrap_or(i32::MAX)
            })
            .expect("at least one calculation failed"));
        }
    };

    let realized_years = portfolio::realized_by_year(&replay.sales);
    let dividend_years = portfolio::dividends_by_year(&dividends.calculated);
    let interest_years = portfolio::interest_by_year(&interests.calculated);
    let crypto_years = portfolio::crypto_by_year(&cryptos.calculated);
    let positions = portfolio::positions(&replay.open_lots);
    let mut cost_by_currency: BTreeMap<Currency, Decimal> = BTreeMap::new();
    let mut holding_providers = BTreeSet::new();
    for position in &positions {
        *cost_by_currency.entry(position.currency).or_default() += position.cost;
        holding_providers.insert(position.provider.as_str());
    }
    let holding_providers = holding_providers.len();
    let mut cost_by_currency: Vec<_> = cost_by_currency.into_iter().collect();
    cost_by_currency.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.as_str().cmp(b.0.as_str())));

    Ok(PortfolioSummary {
        earned: portfolio::earned_by_year(&realized_years, &dividend_years, &interest_years),
        realized: realized_years.iter().map(|year| year.profit).sum(),
        dividends: dividend_years.iter().map(|year| year.gross).sum(),
        interest: interest_years.iter().map(|year| year.total).sum(),
        sales: replay.sales.len(),
        dividend_payments: dividends.calculated.len(),
        interest_records: interests.calculated.len(),
        crypto_records: cryptos.calculated.len(),
        first_sale_year: realized_years.first().map(|year| year.year),
        positions,
        cost_by_currency,
        holding_providers,
        listings,
        crypto: portfolio::crypto_totals(&crypto_years),
        crypto_providers: cryptos
            .calculated
            .iter()
            .map(|record| record.provider.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        current_year: today.year(),
    })
}

pub async fn load_stock_portfolio(app: &App) -> Result<StockPortfolio, Error> {
    let records = app.db.stock_repo().get_by_year(None).await?;
    let listings = load_listings(app).await?;
    let rates = NbpRateProvider::new(app.db.rate_repo().load_all().await?);
    let replay = stock::replay(&records, &listings.exchanges(), &rates)?;
    Ok(StockPortfolio {
        positions: portfolio::positions(&replay.open_lots),
        realized_years: portfolio::realized_by_year(&replay.sales),
        realized_instruments: portfolio::realized_by_instrument(&replay.sales, &replay.open_lots),
        records,
        replay,
        listings,
    })
}

pub async fn load_dividend_portfolio(app: &App) -> Result<DividendPortfolio, Error> {
    let mut dividends = app.db.dividend_repo().get_by_year(None).await?;
    dividends.sort_unstable_by_key(|record| record.time);
    let rates = NbpRateProvider::new(app.db.rate_repo().load_all().await?);
    let rounding = app.db.settings_repo().load_dividend_rounding().await?;
    let data = dividend::calculate(dividends, &rates, rounding)?;
    Ok(DividendPortfolio {
        years: portfolio::dividends_by_year(&data.calculated),
        payers: portfolio::dividends_by_payer(&data.calculated),
        data,
    })
}

pub async fn load_interest_portfolio(
    app: &App,
    today: NaiveDate,
) -> Result<InterestPortfolio, Error> {
    let mut interests = app.db.interest_repo().get_by_year(None).await?;
    interests.sort_unstable_by_key(|record| record.time);
    let rates = NbpRateProvider::new(app.db.rate_repo().load_all().await?);
    let data = interest::calculate(interests, &rates)?;
    Ok(InterestPortfolio {
        months: portfolio::interest_by_month(&data.calculated),
        years: portfolio::interest_by_year(&data.calculated),
        providers: portfolio::interest_by_provider(&data.calculated),
        last_12_months: portfolio::interest_last_12_months(&data.calculated, today),
        data,
    })
}

pub async fn load_crypto_portfolio(app: &App) -> Result<CryptoPortfolio, Error> {
    let mut cryptos = app.db.crypto_repo().get_by_year(None).await?;
    cryptos.sort_unstable_by_key(|record| record.time);
    let rates = NbpRateProvider::new(app.db.rate_repo().load_all().await?);
    let data = crypto::calculate_sell_buy_values(cryptos, &rates)?;
    let years = portfolio::crypto_by_year(&data.calculated);
    Ok(CryptoPortfolio {
        totals: portfolio::crypto_totals(&years),
        data,
        years,
    })
}

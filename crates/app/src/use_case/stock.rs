use std::collections::{BTreeMap, btree_map::Entry};

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use pitpls_core::{
    common::{Amount, Country, Currency, Mic},
    exchange::Exchange,
    rate::NbpRateProvider,
    stock::{
        CalculateStocksError, Instrument, InstrumentExchange, Replay, Stock, StockAction, StockBuy,
        StockPreview, StockSell, StockSplit, StockTaxData, calculate_stocks, market, preview,
        replay, settlement_date,
    },
    time::tax_date,
};
use pitpls_db::{RepositoryError, repository::exchange::ExchangeRepository};
use rust_decimal::Decimal;
use thiserror::Error;

use super::exchange::country_of;
use super::validation::{
    AmountField, Error as ValidationError, TextField, parse_amount, parse_split, parse_time,
    validate_not_empty, validate_not_negative, validate_positive, validate_year,
};
use crate::App;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum Error {
    Validation(#[from] ValidationError),
    Repository(#[from] RepositoryError),
    DuplicateId(String),
    NotFound(String),
    Calculation(#[from] CalculateStocksError),
    NothingToPreview,
}

pub struct StockFields {
    pub time: String,
    pub ticker: String,
    pub action: StockActionFields,
}

pub enum StockActionFields {
    Buy(TradeFields),
    Sell {
        trade: TradeFields,
        tax_paid: Option<(String, Currency)>,
    },
    Split {
        before: String,
        after: String,
    },
}

pub struct TradeFields {
    pub shares: String,
    pub price: String,
    pub price_currency: Currency,
    pub fee: String,
    pub fee_currency: Currency,
    pub provider: String,
    pub settlement_date: Option<NaiveDate>,
    // Saved on the instrument, so every record of the ticker + provider uses it. `None` keeps
    // the one the instrument has.
    pub exchange: Option<Mic>,
}

pub struct CreateStockInput {
    pub id: Option<String>,
    pub fields: StockFields,
}

pub struct UpdateStockInput {
    pub id: String,
    pub fields: StockFields,
}

pub struct SetExchangeInput {
    pub ticker: String,
    pub provider: String,
    pub exchange: Option<Mic>,
}

// An instrument whose exchange is in the exchanges table.
#[derive(Clone)]
pub struct Listing {
    pub ticker: String,
    pub provider: String,
    pub mic: Mic,
    pub name: String,
    pub country: Country,
}

#[derive(Clone)]
pub struct Listings(Vec<Listing>);

pub struct StockData {
    pub tax: StockTaxData,
    pub records: Vec<Stock>,
    pub listings: Listings,
}

pub struct StockHistory {
    pub records: Vec<Stock>,
    pub replay: Replay,
    pub listing: Option<Listing>,
}

impl Listing {
    fn exchange(&self) -> InstrumentExchange {
        InstrumentExchange {
            ticker: self.ticker.clone(),
            provider: self.provider.clone(),
            mic: self.mic,
            country: self.country,
        }
    }
}

impl Listings {
    pub fn get(&self, ticker: &str, provider: &str) -> Option<&Listing> {
        self.0
            .iter()
            .find(|listing| listing.ticker == ticker && listing.provider == provider)
    }

    pub fn name(&self, mic: Mic) -> Option<&str> {
        self.0
            .iter()
            .find(|listing| listing.mic == mic)
            .map(|listing| listing.name.as_str())
    }

    pub fn exchanges(&self) -> Vec<InstrumentExchange> {
        self.0.iter().map(Listing::exchange).collect()
    }
}

fn build_stock(id: String, fields: StockFields) -> Result<(Stock, Option<Mic>), ValidationError> {
    let time = parse_time(&fields.time)?;
    let (action, exchange) = match fields.action {
        StockActionFields::Buy(trade) => {
            let exchange = trade.exchange;
            (StockAction::Buy(build_trade(trade)?), exchange)
        }
        StockActionFields::Sell { trade, tax_paid } => {
            let exchange = trade.exchange;
            let StockBuy {
                number,
                price,
                fee,
                provider,
                settlement_date,
            } = build_trade(trade)?;
            let tax_paid = tax_paid
                .map(|(value, currency)| {
                    parse_amount(&value, AmountField::TaxPaid)
                        .map(|value| Amount { value, currency })
                })
                .transpose()?;
            let sell = StockSell {
                number,
                price,
                fee,
                provider,
                settlement_date,
                tax_paid,
            };
            (StockAction::Sell(sell), exchange)
        }
        StockActionFields::Split { before, after } => {
            let split = StockSplit {
                before: parse_split(&before, AmountField::SplitBefore)?,
                after: parse_split(&after, AmountField::SplitAfter)?,
            };
            (StockAction::Split(split), None)
        }
    };

    let stock = trimmed(Stock {
        id,
        time,
        ticker: fields.ticker,
        action,
    });
    validate(&stock)?;
    Ok((stock, exchange))
}

fn build_trade(fields: TradeFields) -> Result<StockBuy, ValidationError> {
    Ok(StockBuy {
        number: parse_amount(&fields.shares, AmountField::Shares)?,
        price: Amount {
            value: parse_amount(&fields.price, AmountField::Price)?,
            currency: fields.price_currency,
        },
        fee: Amount {
            value: parse_amount(&fields.fee, AmountField::Fee)?,
            currency: fields.fee_currency,
        },
        provider: fields.provider,
        settlement_date: fields.settlement_date,
    })
}

// FIFO matches ticker and provider exactly.
fn trimmed(mut stock: Stock) -> Stock {
    stock.ticker = stock.ticker.trim().to_owned();
    match &mut stock.action {
        StockAction::Buy(StockBuy { provider, .. })
        | StockAction::Sell(StockSell { provider, .. }) => {
            *provider = provider.trim().to_owned();
        }
        StockAction::Split(_) => {}
    }
    stock
}

fn validate(stock: &Stock) -> Result<(), ValidationError> {
    validate_year(tax_date(stock.time).year())?;
    validate_not_empty(&stock.ticker, TextField::Ticker)?;
    match &stock.action {
        StockAction::Buy(buy) => validate_trade(buy.number, &buy.price, &buy.fee, &buy.provider),
        StockAction::Sell(sell) => {
            validate_trade(sell.number, &sell.price, &sell.fee, &sell.provider)?;
            sell.tax_paid.map_or(Ok(()), |tax_paid| {
                validate_not_negative(tax_paid.value, AmountField::TaxPaid)
            })
        }
        StockAction::Split(split) => {
            validate_positive(split.before.into(), AmountField::SplitBefore)?;
            validate_positive(split.after.into(), AmountField::SplitAfter)
        }
    }
}

fn validate_trade(
    shares: Decimal,
    price: &Amount,
    fee: &Amount,
    provider: &str,
) -> Result<(), ValidationError> {
    validate_positive(shares, AmountField::Shares)?;
    validate_not_negative(price.value, AmountField::Price)?;
    validate_not_negative(fee.value, AmountField::Fee)?;
    validate_not_empty(provider, TextField::Provider)
}

async fn save_instrument(
    app: &App,
    stock: &Stock,
    exchange: Option<Mic>,
) -> Result<(), RepositoryError> {
    let Some(provider) = stock.provider() else {
        return Ok(());
    };
    let repo = app.db.instrument_repo();
    match exchange {
        Some(_) => repo.set_exchange(&stock.ticker, provider, exchange).await,
        None => repo.add(&stock.ticker, provider).await,
    }
}

pub async fn create_stock(app: &App, input: CreateStockInput) -> Result<String, Error> {
    let id = input
        .id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let (stock, exchange) = build_stock(id.clone(), input.fields)?;

    app.db.stock_repo().insert(&stock).await.map_err(|error| {
        if error.is_unique_violation() {
            Error::DuplicateId(id.clone())
        } else {
            Error::Repository(error)
        }
    })?;
    save_instrument(app, &stock, exchange).await?;

    Ok(id)
}

pub async fn update_stock(app: &App, input: UpdateStockInput) -> Result<(), Error> {
    if input.id.trim().is_empty() {
        return Err(ValidationError::MissingId.into());
    }

    let id = input.id.clone();
    let (stock, exchange) = build_stock(id.clone(), input.fields)?;

    let rows = app.db.stock_repo().update(&stock).await?;
    if rows == 0 {
        return Err(Error::NotFound(id));
    }
    save_instrument(app, &stock, exchange).await?;
    Ok(())
}

// Instrument rows stay, so a re-import doesn't ask for the exchange again.
pub async fn delete_stocks(app: &App, ids: Vec<String>) -> Result<u64, RepositoryError> {
    app.db.stock_repo().delete_by_ids(&ids).await
}

pub async fn load_stocks(app: &App, year: i32) -> Result<StockData, Error> {
    validate_year(year)?;
    let mut records = app.db.stock_repo().get_through_year(year).await?;
    let listings = load_listings(app).await?;
    let rates = NbpRateProvider::new(app.db.rate_repo().load_all().await?);

    let tax = calculate_stocks(&records, &listings.exchanges(), year, &rates)?;
    records.retain(|record| tax_date(record.time).year() == year);

    Ok(StockData {
        tax,
        records,
        listings,
    })
}

// Up to 31.12 of the year: a year view never shows what happened later.
pub async fn load_history(
    app: &App,
    ticker: &str,
    provider: &str,
    year: i32,
) -> Result<StockHistory, Error> {
    validate_year(year)?;
    let records = app
        .db
        .stock_repo()
        .get_by_ticker_through_year(ticker, Some(provider), year)
        .await?;
    let instrument = app.db.instrument_repo().get(ticker, provider).await?;
    let listings = listings(app, instrument).await?;
    let rates = NbpRateProvider::new(app.db.rate_repo().load_all().await?);

    let replay = replay(&records, &listings.exchanges(), &rates)?;

    Ok(StockHistory {
        records,
        replay,
        listing: listings.0.into_iter().next(),
    })
}

// `exchange` is the one picked in the dialog; `None` uses the instrument's.
pub async fn preview_stock(
    app: &App,
    draft: Stock,
    exchange: Option<Mic>,
) -> Result<StockPreview, Error> {
    let draft = trimmed(draft);
    validate(&draft)?;

    let records = app
        .db
        .stock_repo()
        .get_by_ticker_through_year(&draft.ticker, None, tax_date(draft.time).year())
        .await?;
    let mut instruments = app.db.instrument_repo().list().await?;
    instruments.retain(|instrument| instrument.ticker == draft.ticker);
    if let (Some(mic), Some(provider)) = (exchange, draft.provider()) {
        match instruments
            .iter_mut()
            .find(|instrument| instrument.provider == provider)
        {
            Some(instrument) => instrument.exchange = Some(mic),
            None => instruments.push(Instrument {
                ticker: draft.ticker.clone(),
                provider: provider.to_owned(),
                exchange: Some(mic),
                isin: None,
            }),
        }
    }
    let listings = listings(app, instruments).await?;
    let rates = NbpRateProvider::new(app.db.rate_repo().load_all().await?);

    preview(&records, &draft, &listings.exchanges(), &rates)?.ok_or(Error::NothingToPreview)
}

// `exchange` is the one picked in the dialog; `None` uses the instrument's.
pub async fn settlement_hint(
    app: &App,
    time: DateTime<Utc>,
    ticker: &str,
    provider: &str,
    currency: Currency,
    exchange: Option<Mic>,
) -> Result<NaiveDate, RepositoryError> {
    let exchange = match exchange {
        Some(mic) => Some(mic),
        None => app
            .db
            .instrument_repo()
            .get(ticker.trim(), provider.trim())
            .await?
            .and_then(|instrument| instrument.exchange),
    };
    let country = match exchange {
        Some(mic) => country_of(app, mic).await?,
        None => None,
    };
    Ok(settlement_date(time, market(country, currency)))
}

pub async fn set_exchange(
    app: &App,
    ticker: &str,
    provider: &str,
    exchange: Option<Mic>,
) -> Result<(), RepositoryError> {
    app.db
        .instrument_repo()
        .set_exchange(ticker, provider, exchange)
        .await
}

pub async fn set_exchanges(
    app: &App,
    inputs: Vec<SetExchangeInput>,
) -> Result<(), RepositoryError> {
    app.db
        .instrument_repo()
        .set_exchanges(inputs.iter().map(|input| {
            (
                input.ticker.as_str(),
                input.provider.as_str(),
                input.exchange,
            )
        }))
        .await
}

pub async fn load_listings(app: &App) -> Result<Listings, RepositoryError> {
    let instruments = app.db.instrument_repo().list().await?;
    listings(app, instruments).await
}

// A MIC missing from the exchanges table counts as no exchange.
async fn listings(
    app: &App,
    instruments: impl IntoIterator<Item = Instrument>,
) -> Result<Listings, RepositoryError> {
    let repo = app.db.exchange_repo();
    let mut exchanges: BTreeMap<Mic, Option<Exchange>> = BTreeMap::new();
    let mut listings = Vec::new();
    for instrument in instruments {
        let Some(mic) = instrument.exchange else {
            continue;
        };
        let exchange = match exchanges.entry(mic) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(operating_exchange(&repo, mic).await?),
        };
        if let Some(exchange) = exchange {
            listings.push(Listing {
                ticker: instrument.ticker,
                provider: instrument.provider,
                mic: exchange.mic,
                name: exchange.display_name(),
                country: exchange.country,
            });
        }
    }
    Ok(Listings(listings))
}

// A segment MIC (XNGS) is shown and counted as its operating MIC (XNAS); the country is the same.
async fn operating_exchange(
    repo: &ExchangeRepository,
    mic: Mic,
) -> Result<Option<Exchange>, RepositoryError> {
    let Some(exchange) = repo.get(mic).await? else {
        return Ok(None);
    };
    if exchange.operating_mic == exchange.mic {
        return Ok(Some(exchange));
    }
    Ok(Some(
        repo.get(exchange.operating_mic).await?.unwrap_or(exchange),
    ))
}

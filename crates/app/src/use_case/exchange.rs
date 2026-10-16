use std::collections::{BTreeMap, BTreeSet};

use chrono::{Local, NaiveDate};
use pitpls_core::{
    common::{Country, Mic},
    exchange::{Exchange, ExchangeKind, ExchangeStatus},
};
use pitpls_db::RepositoryError;
use pitpls_mic::{DownloadError, load_exchanges};
use thiserror::Error;

use crate::App;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub enum RefreshError {
    Download(#[from] DownloadError),
    Repository(#[from] RepositoryError),
}

pub async fn refresh_exchanges(app: &App) -> Result<u64, RefreshError> {
    let exchanges = load_exchanges(&app.api_client).await?;
    let rows = app.db.exchange_repo().replace_all(&exchanges).await?;
    app.db
        .settings_repo()
        .save_exchanges_downloaded(Local::now().date_naive())
        .await?;
    Ok(rows)
}

pub async fn exchanges_downloaded(app: &App) -> Result<Option<NaiveDate>, RepositoryError> {
    app.db.settings_repo().load_exchanges_downloaded().await
}

pub async fn list_exchanges(app: &App) -> Result<Vec<Exchange>, RepositoryError> {
    let exchanges = app.db.exchange_repo().list().await?;
    let operating_mics = exchanges
        .iter()
        .map(|exchange| (exchange.mic, exchange.operating_mic))
        .collect::<BTreeMap<_, _>>();
    // An instrument may hold a segment MIC; it counts as used for its operating MIC.
    let used = app
        .db
        .instrument_repo()
        .list()
        .await?
        .into_iter()
        .filter_map(|instrument| instrument.exchange)
        .map(|mic| operating_mics.get(&mic).copied().unwrap_or(mic))
        .collect::<BTreeSet<Mic>>();

    let mut exchanges = exchanges
        .into_iter()
        .filter(|exchange| {
            exchange.kind == ExchangeKind::Operating && exchange.status != ExchangeStatus::Expired
        })
        .collect::<Vec<_>>();
    exchanges.sort_by_cached_key(|exchange| {
        (
            !used.contains(&exchange.mic),
            exchange.display_name().to_lowercase(),
        )
    });
    Ok(exchanges)
}

pub async fn country_of(app: &App, mic: Mic) -> Result<Option<Country>, RepositoryError> {
    let exchange = app.db.exchange_repo().get(mic).await?;
    Ok(exchange.map(|exchange| exchange.country))
}

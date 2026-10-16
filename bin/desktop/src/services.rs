use std::{future::Future, sync::Arc};

use gpui_kit::{Context, Task, Window};
use pitpls_app::{
    App,
    use_case::{
        crypto::Error as CryptoError,
        dividend::Error as DividendError,
        exchange::RefreshError as ExchangeRefreshError,
        import::Error as ImportError,
        interest::Error as InterestError,
        portfolio::Error as PortfolioError,
        rate::{ApiImportError, CsvImportError},
        stock::Error as StockError,
        tax::Error as TaxError,
        year::Error as YearError,
    },
};
use pitpls_core::{
    crypto::CalculateSellBuyValuesError, dividend::CalculateDividendTaxError,
    interest::CalculateInterestTaxError, rate::RateConverterError, stock::CalculateStocksError,
    summary::CalculateTaxSummaryError,
};
use pitpls_db::RepositoryError;
use tokio::runtime::Runtime;

#[derive(Clone)]
pub struct Services {
    pub app: Arc<App>,
    pub runtime: Arc<Runtime>,
}

impl Services {
    pub fn run<
        V: 'static,
        T: Send + 'static,
        F: Future<Output = Result<T, Error>> + Send + 'static,
    >(
        &self,
        window: &Window,
        cx: &Context<V>,
        work: impl FnOnce(Arc<App>) -> F + Send + 'static,
        complete: impl FnOnce(&mut V, Result<T, Error>, &mut Window, &mut Context<V>) + 'static,
    ) -> Task<()> {
        let app = self.app.clone();
        let job = self.runtime.spawn(async move { work(app).await });
        cx.spawn_in(window, async move |view, cx| {
            let result = finish(job).await;
            let _ = view.update_in(cx, |view, window, cx| complete(view, result, window, cx));
        })
    }
}

/// UI boundary for operations sharing status and background-task handling.
/// Application use cases keep their own narrower error types.
#[derive(Debug)]
pub enum Error {
    Crypto(CryptoError),
    Dividend(DividendError),
    Interest(InterestError),
    Stock(StockError),
    Portfolio(PortfolioError),
    ExchangeRefresh(ExchangeRefreshError),
    Import(ImportError),
    RateCsv(CsvImportError),
    RateApi(ApiImportError),
    Tax(TaxError),
    Year(YearError),
    Repository(RepositoryError),
    OpenDatabase(pitpls_db::OpenDatabaseError),
    DeleteDatabase(std::io::Error),
    Background(tokio::task::JoinError),
    ReadPreferences(std::io::Error),
    ParsePreferences(serde_json::Error),
    WritePreferences(std::io::Error),
    SerializePreferences(serde_json::Error),
}

impl Error {
    /// Starting over with an empty database can fix these.
    pub fn broken_database(&self) -> bool {
        use pitpls_db::OpenDatabaseError::{Connect, Migrate};

        matches!(
            self,
            Self::OpenDatabase(Connect(_) | Migrate(_)) | Self::DeleteDatabase(_)
        )
    }

    pub fn missing_rate(&self) -> bool {
        self.rate_error().is_some()
    }

    pub fn missing_rate_year(&self) -> Option<i32> {
        self.rate_error().and_then(RateConverterError::missing_year)
    }

    fn rate_error(&self) -> Option<&RateConverterError> {
        use CalculateDividendTaxError::{DividendConversion, PaidTaxConversion};
        use CalculateInterestTaxError::InterestConversion;
        use CalculateSellBuyValuesError::{FeeConversion, ValueConversion};
        use CalculateStocksError::{PriceConversion, TaxPaidConversion};
        use CalculateTaxSummaryError::{Crypto, Dividend, Interest, Stock};

        match self {
            Self::Crypto(CryptoError::Calculation(
                ValueConversion(error) | FeeConversion(error),
            ))
            | Self::Dividend(DividendError::Calculation(
                DividendConversion(error) | PaidTaxConversion(error),
            ))
            | Self::Interest(InterestError::Calculation(InterestConversion(error)))
            | Self::Stock(StockError::Calculation(
                PriceConversion(error)
                | CalculateStocksError::FeeConversion(error)
                | TaxPaidConversion(error),
            ))
            | Self::Tax(TaxError::Calculation(
                Crypto(ValueConversion(error) | FeeConversion(error))
                | Dividend(DividendConversion(error) | PaidTaxConversion(error))
                | Interest(InterestConversion(error))
                | Stock(
                    PriceConversion(error)
                    | CalculateStocksError::FeeConversion(error)
                    | TaxPaidConversion(error),
                ),
            )) => Some(error),
            Self::Portfolio(error) => error.rate_error(),
            _ => None,
        }
    }
}

impl From<CryptoError> for Error {
    fn from(error: CryptoError) -> Self {
        Self::Crypto(error)
    }
}

impl From<DividendError> for Error {
    fn from(error: DividendError) -> Self {
        Self::Dividend(error)
    }
}

impl From<InterestError> for Error {
    fn from(error: InterestError) -> Self {
        Self::Interest(error)
    }
}

impl From<StockError> for Error {
    fn from(error: StockError) -> Self {
        Self::Stock(error)
    }
}

impl From<PortfolioError> for Error {
    fn from(error: PortfolioError) -> Self {
        Self::Portfolio(error)
    }
}

impl From<ExchangeRefreshError> for Error {
    fn from(error: ExchangeRefreshError) -> Self {
        Self::ExchangeRefresh(error)
    }
}

impl From<ImportError> for Error {
    fn from(error: ImportError) -> Self {
        Self::Import(error)
    }
}

impl From<CsvImportError> for Error {
    fn from(error: CsvImportError) -> Self {
        Self::RateCsv(error)
    }
}

impl From<ApiImportError> for Error {
    fn from(error: ApiImportError) -> Self {
        Self::RateApi(error)
    }
}

impl From<TaxError> for Error {
    fn from(error: TaxError) -> Self {
        Self::Tax(error)
    }
}

impl From<YearError> for Error {
    fn from(error: YearError) -> Self {
        Self::Year(error)
    }
}

impl From<RepositoryError> for Error {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}

impl From<pitpls_db::OpenDatabaseError> for Error {
    fn from(error: pitpls_db::OpenDatabaseError) -> Self {
        Self::OpenDatabase(error)
    }
}

pub async fn finish<T, E: Into<Error>>(
    job: tokio::task::JoinHandle<Result<T, E>>,
) -> Result<T, Error> {
    job.await.map_err(Error::Background)?.map_err(Into::into)
}

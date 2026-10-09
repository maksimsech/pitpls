use std::{future::Future, sync::Arc};

use gpui_kit::{Context, Task, Window};
use pitpls_app::{
    App,
    use_case::{
        crypto::Error as CryptoError,
        dividend::Error as DividendError,
        import::Error as ImportError,
        interest::Error as InterestError,
        rate::{ApiImportError, CsvImportError},
        tax::Error as TaxError,
        year::Error as YearError,
    },
};
use pitpls_core::{
    crypto::CalculateSellBuyValuesError, dividend::CalculateDividendTaxError,
    interest::CalculateInterestTaxError, summary::CalculateTaxSummaryError,
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
    Import(ImportError),
    RateCsv(CsvImportError),
    RateApi(ApiImportError),
    Tax(TaxError),
    Year(YearError),
    Repository(RepositoryError),
    OpenDatabase(pitpls_db::OpenDatabaseError),
    Background(tokio::task::JoinError),
    ReadPreferences(std::io::Error),
    ParsePreferences(serde_json::Error),
    WritePreferences(std::io::Error),
    SerializePreferences(serde_json::Error),
}

impl Error {
    pub fn missing_rate(&self) -> bool {
        use CalculateDividendTaxError::{DividendConversion, PaidTaxConversion};
        use CalculateInterestTaxError::InterestConversion;
        use CalculateSellBuyValuesError::{FeeConversion, ValueConversion};
        use CalculateTaxSummaryError::{Crypto, Dividend, Interest};

        matches!(
            self,
            Self::Crypto(CryptoError::Calculation(
                ValueConversion(_) | FeeConversion(_)
            )) | Self::Dividend(DividendError::Calculation(
                DividendConversion(_) | PaidTaxConversion(_)
            )) | Self::Interest(InterestError::Calculation(InterestConversion(_)))
                | Self::Tax(TaxError::Calculation(
                    Crypto(ValueConversion(_) | FeeConversion(_))
                        | Dividend(DividendConversion(_) | PaidTaxConversion(_))
                        | Interest(InterestConversion(_))
                ))
        )
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

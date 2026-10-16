use std::iter;

use super::{
    CalculateStocksError,
    model::{InstrumentExchange, Replay, Stock, StockAction, StockPreview},
    replay::{replay, sort_key},
};
use crate::rate::NbpRateProvider;

// A stored record with the draft's ID is the one being edited, so the draft replaces it.
pub fn preview(
    records: &[Stock],
    draft: &Stock,
    exchanges: &[InstrumentExchange],
    rates: &NbpRateProvider,
) -> Result<Option<StockPreview>, CalculateStocksError> {
    let draft_key = sort_key(draft);
    let earlier = records.iter().filter(|record| {
        record.ticker == draft.ticker && record.id != draft.id && sort_key(record) < draft_key
    });
    let Replay {
        sales,
        open_lots,
        holdings,
    } = replay(earlier.chain(iter::once(draft)), exchanges, rates)?;
    let mut holdings = holdings
        .into_iter()
        .filter(|holding| holding.record_id == draft.id);

    Ok(match draft.action {
        StockAction::Buy(_) => open_lots
            .into_iter()
            .find(|lot| lot.buy_id == draft.id)
            .zip(holdings.next())
            .map(|(lot, holding)| StockPreview::Buy { lot, holding }),
        StockAction::Sell(_) => sales
            .into_iter()
            .find(|sale| sale.id == draft.id)
            .zip(holdings.next())
            .map(|(sale, holding)| StockPreview::Sell { sale, holding }),
        StockAction::Split(_) => Some(StockPreview::Split {
            lots: open_lots,
            holdings: holdings.collect(),
        }),
    })
}

CREATE TABLE stocks(
    id TEXT PRIMARY KEY,
    time TEXT NOT NULL,
    ticker TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('Buy', 'Sell', 'Split')),
    number TEXT,
    price TEXT,
    price_currency TEXT,
    fee TEXT,
    fee_currency TEXT,
    provider TEXT,
    settlement_date DATE,
    tax_paid TEXT,
    tax_paid_currency TEXT,
    split_before INTEGER,
    split_after INTEGER,
    CHECK (
        (
            action = 'Buy'
                AND number IS NOT NULL
                AND price IS NOT NULL
                AND price_currency IS NOT NULL
                AND fee IS NOT NULL
                AND fee_currency IS NOT NULL
                AND provider IS NOT NULL
                AND tax_paid IS NULL
                AND tax_paid_currency IS NULL
                AND split_before IS NULL
                AND split_after IS NULL
        )
        OR
        (
            action = 'Sell'
                AND number IS NOT NULL
                AND price IS NOT NULL
                AND price_currency IS NOT NULL
                AND fee IS NOT NULL
                AND fee_currency IS NOT NULL
                AND provider IS NOT NULL
                AND (tax_paid IS NULL) = (tax_paid_currency IS NULL)
                AND split_before IS NULL
                AND split_after IS NULL
        )
        OR
        (
            action = 'Split'
                AND split_before IS NOT NULL
                AND split_before > 0
                AND split_after IS NOT NULL
                AND split_after > 0
                AND number IS NULL
                AND price IS NULL
                AND price_currency IS NULL
                AND fee IS NULL
                AND fee_currency IS NULL
                AND provider IS NULL
                AND settlement_date IS NULL
                AND tax_paid IS NULL
                AND tax_paid_currency IS NULL
        )
    )
);

CREATE INDEX stocks_time_idx ON stocks(time);

CREATE TABLE stock_instruments(
    ticker TEXT NOT NULL,
    provider TEXT NOT NULL,
    exchange TEXT,
    isin TEXT,
    PRIMARY KEY (ticker, provider)
);

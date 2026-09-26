CREATE TABLE stocks(
    id TEXT PRIMARY KEY,
    date DATE NOT NULL,
    ticker TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('Buy', 'Sell', 'Split')),
    number TEXT,
    price TEXT,
    price_currency TEXT,
    fee TEXT,
    fee_currency TEXT,
    provider TEXT,
    ratio TEXT,
    CHECK (
        (
            action IN ('Buy', 'Sell')
                AND number IS NOT NULL
                AND price IS NOT NULL
                AND price_currency IS NOT NULL
                AND fee IS NOT NULL
                AND fee_currency IS NOT NULL
                AND provider IS NOT NULL
                AND ratio IS NULL
        )
        OR
        (
            action = 'Split'
                AND ratio IS NOT NULL
                AND number IS NULL
                AND price IS NULL
                AND price_currency IS NULL
                AND fee IS NULL
                AND fee_currency IS NULL
                AND provider IS NULL
        )
    )
);

CREATE INDEX stocks_date_idx ON stocks(date);

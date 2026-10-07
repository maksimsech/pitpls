CREATE TABLE last_import(
    id INTEGER PRIMARY KEY CHECK (id = 1),
    provider TEXT NOT NULL,
    file_name TEXT NOT NULL,
    dividends INTEGER NOT NULL,
    interests INTEGER NOT NULL,
    cryptos INTEGER NOT NULL,
    imported_at DATETIME NOT NULL
);

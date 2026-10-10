# pitpls

`pitpls` is a desktop project for collecting and reviewing financial data.

## Usage Notice

pitpls is for information only. It is not tax, financial or legal advice, and not a guide to filing your return.

Results aren't guaranteed to be complete or correct. You are responsible for your tax return: check every figure yourself, and ask a tax adviser if you're unsure.

The app shows this notice on first launch and keeps a short reminder on the Summary page.

## How to Use

Make sure Rust is available on your machine, then run the app in development
mode:

```sh
just d
```

For a separate database during local testing:

```sh
just d --database /tmp/pitpls/pitpls.db
```

No builds are provided. At the moment, the project is intended to be run in development mode only.

## Project Structure

The repository is split into a small set of focused parts:

- `bin/desktop/` - native GPUI desktop application using the shared Rust services.
- `crates/app/` - application use cases, input validation, and orchestration.
- `crates/core/` - shared financial domain types used across the app.
- `crates/db/` - SQLite schema and repositories.
- `crates/importers/` - importer registry and source-specific parsers.
- `crates/nbr/` - NBP exchange-rate clients and parsers.

The importer crate is organized like this:

- `crates/importers/src/lib.rs` - public crate entrypoint, importer registry, and `import(...)` dispatch.
- `crates/importers/src/model.rs` - public importer metadata and shared import result types.
- `crates/importers/src/impls/` - private parser implementations for each supported source.

## Contributing

Contributions are welcome, especially custom importers.

If you want to add a new importer:

1. Create a new parser module in `crates/importers/src/impls/`.
2. Export the new module from `crates/importers/src/impls/mod.rs`.
3. Extend `ImporterKind` in `crates/importers/src/model.rs`.
4. Register the importer in the `IMPORTERS` list in `crates/importers/src/lib.rs`.
5. Hook the parser into the `import(...)` match in `crates/importers/src/lib.rs`.
6. Keep the importer focused on one source format and one clear parsing flow.

When opening a pull request, keep it small, directed, and limited to one change or importer at a time.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).

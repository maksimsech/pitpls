# pitpls

`pitpls` is a desktop project for collecting and reviewing financial data.

## Usage Notice

This tool is created for informational purposes only.

It is not designed for calculating tax and should not be treated as financial advice, tax advice, or a tax filing guide.

Always verify any data and calculations on your own before using them for real financial or tax decisions.

## How to Use

The project must be installed before use.

1. Install JavaScript dependencies:

```sh
npm --prefix bin/desktop install
```

2. Make sure Rust and the Tauri development prerequisites are available on your machine.

3. Run the project locally in development mode:

```sh
cd bin/desktop
npm run tauri dev
```

You can also use the shortcut from the `justfile`:

```sh
just d
```

No prebuilt downloads are provided. The Tauri app is intended to be run in development mode.

A native SwiftUI macOS frontend is also available in `bin/macos/`. Build and run it
with `just m`, or use `just mb` to build only.

## Project Structure

The repository is split into a small set of focused parts:

- `bin/desktop/` - complete desktop application package, including the React frontend.
- `bin/desktop/src-tauri/` - Tauri binary and thin command adapters exposed to the frontend.
- `bin/desktop/src/` - React pages, components, hooks, and generated bindings.
- `bin/macos/` - native SwiftUI macOS frontend and app bundle build script.
- `crates/macos_bindings/` - typed UniFFI adapters and Swift binding generator.
- `crates/app/` - application use cases, input validation, and orchestration.
- `crates/core/` - shared financial domain types used across the app.
- `crates/db/` - SQLite schema and repositories.
- `crates/importers/` - importer registry and source-specific parsers.
- `crates/nbr/` - NBP exchange-rate clients and parsers.

The importer crate is organized:

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

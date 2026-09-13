# Native macOS app

A SwiftUI frontend for the existing Rust application. Requires macOS 14+, Swift 6
(Xcode or Command Line Tools), Rust, and `just`. 

## Architecture

- `Sources/Pitpls/`: presentation, transient view state, and native file selection.
- `Sources/PitCore/` and `Sources/PitCoreFFI/`: generated Swift and C bindings.
- `../../crates/macos_bindings/`: UniFFI adapter calling the same `pitpls-app` use cases
  that the Tauri commands call.

## Contracts and development

```sh
just et swift       # Regenerate Swift bindings from the Rust library
```

Native builds regenerate bindings automatically. Generated files and build
products are ignored by Git. When an exposed Rust type changes, update its remote
description in `crates/macos_bindings/src/contracts.rs`; Rust compilation checks it
against the source type. Do not edit generated Swift or C files.

After `just et swift`, open `bin/macos/Package.swift` in Xcode to work on the UI.
The package defaults to linking the debug Rust archive. The build script selects
the optimized archive for release builds. No dev server is involved.

## Storage

By default, both frontends use the existing Tauri database at:

```text
~/Library/Application Support/com.mngapp.pitpls/pitpls.db
```

For an isolated library, run the executable with an explicit database path:

```sh
PITPLS_DATABASE_PATH=/tmp/macos_bindings-preview.db bin/macos/.build/Pitpls.app/Contents/MacOS/Pitpls
```

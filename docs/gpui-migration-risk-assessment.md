# Tauri to GPUI migration: risk assessment and findings

Status: discovery complete; implementation not started  
Reviewed: 2026-08-10  
Reference GPUI source: local Zed checkout at commit `492acd6c815cbe8c7366d54e6092341340afa6c7`

## Executive summary

The migration is feasible, but it is a desktop-shell and UI rewrite rather than a framework swap. The tax domain, importers, NBP integration, and most database queries can remain Rust. The React, Router, Tailwind/shadcn, browser storage, Tauri command layer, plugins, and packaging must be replaced or reorganized.

The largest risks are integrating SQLx/Reqwest's Tokio work with GPUI's executor, preserving exact decimal values, reproducing the variable-height virtualized tables, pinning compatible pre-1.0 dependencies, and shipping an accessible cross-platform build.

The application is still in development and has no deployed clients. The GPUI version may therefore start with a clean database, a new storage location, and a squashed initial schema. There is no requirement to migrate installed databases, preserve client data, or provide a Tauri rollback release. Keep the current Tauri UI only as a behavioral reference while the rewrite is in progress.

## Scope reviewed

- The current React UI, its seven routes, dialogs, forms, virtualized tables, selection behavior, themes, and local preferences.
- The 24 Tauri commands and the SQLx repository layer.
- SQLite schema, migrations, repository transactions, and application startup.
- Portable Rust crates: `core`, `importers`, and `nbr`.
- GPUI source and examples in the downloaded Zed repository.
- `gpui-component` controls and dependency layout.
- Async execution, file dialogs, accessibility, fonts, packaging, and platform support.

## Prioritized risk register

| ID | Risk | Severity | Likelihood | Required response |
|---|---|---:|---:|---|
| R1 | Database bootstrap remains coupled to Tauri and its plugins | Medium | High | Move path selection, connection setup, and schema creation into the new storage crate. A clean GPUI database is acceptable. |
| R2 | Repository extraction changes transactions or query behavior | High | Medium | Preserve transaction boundaries and compare results against the current implementation using disposable development data. |
| R3 | SQLx/Reqwest run without the Tokio runtime previously supplied by Tauri | High | High | Own a Tokio runtime explicitly and define one bridge between background services and the GPUI foreground thread. |
| R4 | Duplicate or incompatible GPUI packages enter the dependency graph | High | Medium | Pin one compatible GPUI/gpui-component source set and fail the spike if `cargo tree -d` shows duplicate GPUI packages. |
| R5 | Money passes through `f64` in GPUI controls | Critical | Medium | Keep `rust_decimal::Decimal` and string-based input/parsing end to end; do not use an `f64` number field for financial values. |
| R6 | Variable-height expandable tables regress scrolling, selection, or focus | High | High | Prototype one complete financial table before porting the other two; use a variable-height list or move details to a panel/dialog. |
| R7 | Dropped GPUI tasks cancel work or stale results overwrite newer state | High | High | Store/detach tasks deliberately, use weak entities, and guard each page request with an operation generation ID. |
| R8 | Keyboard and screen-reader behavior regresses | High | High | Treat roles, labels, focus order, actions, and stable accessibility IDs as explicit implementation work and acceptance criteria. |
| R9 | Native file picking loses CSV/PDF filters | Medium | High | Validate extensions after selection and give a clear error, or adopt a platform picker that supports filters. |
| R10 | Packaging, signing, fonts, icons, or Linux/Windows support block the eventual first release | Medium | Medium | Prove a development-host package during migration; defer signing and the full platform matrix until first-release work. |
| R11 | Visual and behavioral parity drifts during a full rewrite | Medium | High | Capture reference screenshots and a route-by-route acceptance checklist before implementation. |
| R12 | Year/date invariants become panics after command-layer validation is removed | High | Medium | Move validation into application services and remove repository `unwrap` calls during extraction. |

## Detailed findings and notes

### R1 — Database bootstrap after Tauri

The current application obtains its database location and migration startup behavior through Tauri and `tauri-plugin-sql`. GPUI does not provide those conventions. The new storage crate must own:

- the platform data-directory choice;
- directory creation and database naming;
- SQLx connection options and pool lifetime;
- schema creation/migration at application startup;
- development reset behavior and useful startup errors.

Because there are no deployed clients, the GPUI application does **not** need to discover, copy, upgrade, or preserve the current Tauri database. It may use a new application-data path and create a new database. If old development data is useful during implementation, import or copy it manually as disposable test data rather than building client migration machinery.

The existing schema can be moved into the new storage crate as a clean initial migration. It may also be edited or squashed before the first release. There is no need to preserve the checksum of the already-applied development migration. Once a real release is distributed, applied migrations must become immutable in the normal SQLx way.

### R2 — Repository and transaction parity

The database risk that remains is behavioral. Extracting repositories from `src-tauri` can change query filters, ordering, joins, transaction scope, or error propagation even when the schema is unchanged.

Preserve the existing transactions around imports, batch deletes, and multi-step writes. Configure a deliberate SQLite busy timeout instead of inheriting the current effective value of zero, and make startup/query errors visible in the GPUI interface.

Use a disposable seeded development database to compare record lists, totals, warnings, settings, and tax calculations between Tauri and GPUI. This is parity verification, not an installed-client migration requirement.

### R3 — GPUI and Tokio execution boundary

GPUI has foreground and background executors, but they are not a Tokio runtime. SQLx pools and Reqwest currently rely on Tokio support that Tauri supplies indirectly. A direct port that starts these operations from GPUI without a Tokio runtime can panic or stall.

Recommended design:

```text
GPUI view/entity
    -> application service request
        -> owned Tokio runtime/handle
            -> SQLx / Reqwest / file I/O
        <- typed result
    <- update weak GPUI entity on foreground executor
```

Create a `ServiceRuntime` before `Application::run`, own its multi-thread Tokio runtime for the process lifetime, and expose narrow application-service operations. Do not scatter ad hoc runtimes through views.

GPUI `Task` values cancel when dropped. Every task must be awaited, stored, or explicitly detached. Use `WeakEntity` in long-running work so completed tasks do not retain closed windows. Reads may be cancellable; after a database mutation starts, dropping the view must not leave application state ambiguous.

React currently discards page state when the selected year changes. GPUI will not do this automatically. Assign a generation ID to page loads so a slow response for an old route/year cannot overwrite newer state.

### R4 — Dependency compatibility and pinning

The local Zed checkout resolves `gpui 0.2.2`. The published `gpui-component 0.5.1` package also declares `gpui ^0.2.2`, making it the simplest initial compatibility candidate.

The `gpui-component` Git main branch is more hazardous: its workspace currently points GPUI-related crates directly at Zed's moving `main`, and recent releases have included breaking changes. Mixing crates.io GPUI, a pinned Zed revision, and an indirectly unpinned Git GPUI can create duplicate, type-incompatible copies of the same crates.

The dependency spike must:

- Compile a minimal window containing input, select, date picker, dialog, table/list, tooltip, notification, and theme controls.
- Select one source identity for all GPUI crates.
- Commit `Cargo.lock` and pin exact versions or one exact Git revision.
- Check `cargo tree -d` and reject duplicate GPUI packages.
- Measure clean build time and release binary size; gpui-component pulls a broad UI dependency set.
- Record the chosen versions and upgrade policy in the repository.

Prefer published `gpui-component 0.5.1` plus GPUI `0.2.2` for the first spike. If a required fix exists only on Git, pin the whole compatible set to exact revisions or maintain a project fork.

### R5 — Exact financial values

The existing application deliberately represents monetary amounts with `rust_decimal::Decimal` and transfers them through Tauri as strings. That contract prevents binary floating-point errors.

`gpui-component`'s ordinary number-field settings parse through `f64`. They are not acceptable for transaction values, tax, fees, exchange rates, or totals. Use a text input with explicit `Decimal::from_str` validation, preserving the original edit string until commit.

Also preserve display behavior in `src/components/currency-value.tsx`: currency symbols, trailing-zero trimming, the current five-decimal compact presentation, truncation marker, and full-value tooltip. Direct Rust values remove the serialization boundary but must not introduce implicit rounding or locale-dependent parsing.

### R6 — Virtualized tables and expandable rows

Rates, interest, crypto, and dividend pages virtualize their rows. The three financial record tables also expand rows to show calculation details, so row heights change at runtime. The current TanStack virtualizer measures these changes.

The available GPUI/gpui-component table examples establish fixed-height virtualization, including a 10,000-row GPUI example. They do not prove parity for this variable-height interaction. Build one vertical slice—recommended: the crypto page—with:

- large data volume;
- expandable calculation details;
- row click and checkbox selection without event conflicts;
- multi-delete and individual edit/delete actions;
- stable vertical and horizontal scroll position;
- keyboard focus and screen-reader state;
- totals and loading/error/empty states.

If the table cannot safely support changing heights, use a variable-height virtual list or show details in an adjacent inspector/dialog. Decide this during the spike, not after all three pages have been ported.

### R7 — State and task lifecycle

GPUI renders and mutates entities on the foreground thread. Entity state changes need the correct context and notification. Re-entrant updates can panic.

Create a small, explicit state model:

- application state: selected year, theme, service handles;
- route state: current page and navigation;
- page entities: query state, selection, dialog/form state, and generation ID;
- durable preferences: selected year and theme in a small settings store rather than browser `localStorage`.

Avoid a single global mutable application object and avoid one task per widget without ownership rules. Define cancellation, retry, and error display behavior for every service request.

### R8 — Accessibility and keyboard operation

Browser semantics from HTML, Radix, and shadcn do not carry into GPUI. GPUI uses AccessKit, but programmatic accessibility remains explicit: roles, labels, actions, focus handling, tab order, and stable element IDs must be supplied.

Important implementation traps found in GPUI source:

- Nodes without a role are not reported to accessibility clients.
- Duplicate global element IDs can cause nodes to be silently dropped in release builds.
- A `text!` invocation repeated through a list needs per-item IDs such as `.with_id(index)`.
- Accessibility actions and keyboard focus are separate pieces of work.

Acceptance checks must include keyboard-only completion of imports and all CRUD forms; focus trapping/restoration for dialogs; labelled validation errors; announced selection/expanded state; and a VoiceOver pass on macOS. Windows screen-reader and Linux keyboard passes belong in the release matrix. This requirement follows the semantic behavior currently inherited from the shadcn/Radix UI.

### R9 — File dialogs and import safety

Current import dialogs constrain users to Trading 212 CSV, Coinbase CSV, Revolut PDF, rate CSV, or the appropriate combinations. GPUI's built-in `PathPromptOptions` supports files/directories/multiple selection and prompt text, but not extension filters. Linux can also fail to open the platform picker.

Validate the selected extension and file signature/content before starting an import, keep a clear recoverable error state, and never infer an importer solely from an unchecked extension. If native filter UX is mandatory, evaluate a separate cross-platform picker during the dependency spike.

The registered Tauri opener and notification plugins have no current application usage. They need no behavioral port today. Notifications can be added later; on macOS, delivery requires an application bundle.

### R10 — Packaging and platform support

GPUI supports macOS, Windows, and Linux, but it does not replace Tauri's bundler. A release tool such as Cargo Packager must recreate:

- bundle identifier and application metadata;
- icons and embedded resources;
- macOS signing/notarization and Metal/font requirements;
- Windows installer/signing and DirectWrite behavior;
- Linux X11/Wayland features, system libraries, and package formats;
- clean-install behavior and database directory creation.

Create a packageable hello-window on the development host during the technical spike so resource and bundle assumptions are tested early. Signing, notarization, installers, and the full target-platform matrix can be deferred until first-release work. The development-only status removes installer-upgrade and client-migration work entirely.

### R11 — Theme, fonts, and visual parity

The current UI uses Tailwind semantic tokens in OKLCH, light/dark/system themes, Lucide icons, and the Geist variable webfont. These are assets and behavior, not reusable React code.

- Map semantic colors to a typed GPUI theme; do not copy raw class names into view code.
- Capture screenshots of every route, dialog, loading/error/empty state, and expanded table row at a defined window size.
- Port Lucide icons as licensed SVG assets or verified gpui-component equivalents.
- The installed Geist package contains WOFF2 assets, while Zed's known embedded-font path loads TTF data. Obtain an official TTF/variable-TTF asset, retain its license notice, and verify all weights on each platform.
- Preserve system-theme changes and persist the user's explicit choice outside browser localStorage.

Pixel identity is less important than preserving information hierarchy, density, validation clarity, and keyboard behavior.

### R12 — Validation formerly enforced at the command boundary

The Tauri command layer currently validates user-shaped input before repository calls. When it disappears, validation must move into application services, not into GPUI widgets alone.

Year-filtered repository code uses `NaiveDate::from_ymd_opt(...).unwrap()` in the crypto, dividend, and interest paths. The current year commands constrain input to `1900..=2100`, but a new call path could bypass that invariant and panic. Make the year a validated domain/application value and replace these unwraps while extracting storage services.

Apply the same rule to country codes, dates, action types, currencies, decimal strings, and import paths. Widgets give early feedback; services remain the authoritative boundary.

## Proposed architecture boundary

```text
crates/core          tax/domain rules and Decimal-based models
crates/importers     broker/import parsing
crates/nbr           NBP client and parsing
crates/storage       SQLite connection, schema bootstrap, repositories, path policy
crates/application   validated use cases, transactions, async runtime boundary
crates/desktop       GPUI entities, views, assets, navigation, packaging entry point
```

Move behavior in slices. First extract `storage` and `application` while Tauri still calls them. Then build GPUI views against the same services. This separates risky data/runtime work from visual work and lets the current UI serve as a parity reference.

## Migration gates

### Gate 0 — Baseline and database policy

- Record reference screenshots and a manual behavior checklist for all routes.
- Decide the GPUI application-data path and database filename.
- Create a clean initial schema migration owned by the storage crate.
- Decide how developers reset or seed disposable local data.

### Gate 1 — Technical spikes

- One pinned GPUI dependency graph with no duplicate GPUI crates.
- Owned Tokio runtime successfully executes one SQLx query and one NBP HTTP request.
- One exact-Decimal edit/display round trip.
- One file-selection/import flow with invalid-extension handling.
- One accessible dialog and form operated entirely by keyboard.
- One packaged hello-window on the development host.

### Gate 2 — Storage/application extraction

- Tauri uses the extracted repositories and application services without behavior changes.
- A clean database is created reliably at the new GPUI path.
- All mutations retain transaction and validation boundaries.
- Startup, shutdown, and connection errors are handled explicitly.

### Gate 3 — Vertical UI slice

- Application shell, navigation, year selection, theme, and one complete financial page.
- Variable-height/detail presentation decision finalized.
- Loading, empty, error, validation, selection, and destructive actions reviewed.
- Keyboard and VoiceOver checks pass.

### Gate 4 — Feature parity

- Dashboard, imports, rates, crypto, dividends, interests, settings, and all dialogs match the checklist.
- Exact totals and formatted values are compared against the Tauri build using the same disposable seeded data set.
- Stale async results, cancellation, closing windows, and offline NBP errors are exercised.

### Gate 5 — Release readiness

- A clean install creates and opens its database without Tauri components.
- macOS, Windows, and Linux packages pass smoke tests.
- Signing/notarization and installer identity are complete.

## Effort impact

The earlier estimate remains reasonable only if the first two gates succeed without redesign. For an automated implementation agent, allow approximately **12–20 focused working hours** for the code migration plus packaging/documentation iteration, typically **2–4 calendar days** because cross-platform builds and user validation add waiting time.

The estimate should be revised upward if any of these occur:

- variable-height rows require a custom table implementation;
- GPUI/gpui-component must be forked;
- signed Windows/Linux release environments are not already available;
- the clean database bootstrap or seeded parity comparison exposes repository differences;
- accessibility requires missing primitives in the selected component version.

Do not treat the estimate as a promise until Gate 1 is complete. That spike is the decision point that converts the largest unknowns into measured work.

## Verification notes and limitations

- Existing frontend build and lint had passed during the initial review.
- Rust tests were not completed in this environment because a cached Cargo dependency could not be unpacked under the filesystem sandbox. This is an unverified baseline, not a reported test failure.
- No new tests were added and no development server was started, following repository instructions.
- The local development database was not modified and individual financial records were not inspected.
- GPUI and gpui-component are pre-1.0 and can change; recheck the pinned versions when implementation begins.

## Sources consulted

Project evidence:

- `src/main.tsx`, `src/pages/`, `src/components/`, `src/index.css`
- `src/components/currency-value.tsx`, `src/components/year-provider.tsx`
- `src-tauri/src/command/`, `src-tauri/src/repository/`, `src-tauri/src/db.rs`
- `src-tauri/tauri.conf.json`, workspace `Cargo.toml` files

Downloaded Zed/GPUI evidence:

- `/Users/maksimsech/Documents/projects/zed/crates/gpui/README.md`
- `/Users/maksimsech/Documents/projects/zed/crates/gpui/src/_accessibility.rs`
- `/Users/maksimsech/Documents/projects/zed/crates/gpui/src/platform.rs`
- `/Users/maksimsech/Documents/projects/zed/crates/gpui/examples/data_table.rs`
- `/Users/maksimsech/Documents/projects/zed/crates/gpui/examples/input.rs`
- `/Users/maksimsech/Documents/projects/zed/crates/gpui/examples/a11y.rs`
- [Zed GPUI crate](https://github.com/zed-industries/zed/tree/main/crates/gpui)

External primary/reference documentation:

- [gpui-component crate documentation](https://docs.rs/crate/gpui-component/latest)
- [gpui-component repository](https://github.com/longbridge/gpui-component)
- [SQLx runtime support](https://docs.rs/sqlx/latest/sqlx/)
- [Tauri path API](https://v2.tauri.app/reference/javascript/api/namespacepath/)
- [Cargo Packager](https://docs.crabnebula.dev/packager/)

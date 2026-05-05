use pitpls_importers::{
    IMPORTERS,
    model::{Importer, InputType, OutputType},
};
use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri_specta::{Builder, collect_commands};

use crate::command::{
    crypto::{create_crypto, delete_cryptos, load_cryptos, update_crypto},
    dividend::{create_dividend, delete_dividends, load_dividends, update_dividend},
    import::run_import,
    interest::{create_interest, delete_interests, load_interests, update_interest},
    rate::{import_api, import_csv, list_rates, reset_rates},
    settings::{load_settings, update_settings},
    tax::load_tax_summary,
    warnings::get_warnings,
    year::{add_year, delete_year, list_years},
};
use crate::db::{DB_URL, migrations};
use crate::state::setup;

mod command;
mod db;
mod repository;
mod state;

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            import_csv,
            import_api,
            reset_rates,
            list_rates,
            load_cryptos,
            delete_cryptos,
            create_crypto,
            update_crypto,
            load_dividends,
            delete_dividends,
            create_dividend,
            update_dividend,
            load_interests,
            delete_interests,
            create_interest,
            update_interest,
            run_import,
            get_warnings,
            load_tax_summary,
            list_years,
            add_year,
            delete_year,
            load_settings,
            update_settings,
        ])
        .typ::<Importer>()
        .typ::<InputType>()
        .typ::<OutputType>()
        .constant("IMPORTERS", IMPORTERS)
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = specta_builder();

    #[cfg(debug_assertions)]
    specta_builder
        .export(
            Typescript::default()
                .bigint(BigIntExportBehavior::Number)
                .header("// @ts-nocheck\n"),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(DB_URL, migrations())
                .build(),
        )
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            specta_builder.mount_events(app);
            let handle = app.handle().clone();
            setup(handle)
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_bindings() {
        specta_builder()
            .export(
                Typescript::default()
                    .bigint(BigIntExportBehavior::Number)
                    .header("// @ts-nocheck\n"),
                "../src/bindings.ts",
            )
            .expect("failed to export typescript bindings");
    }
}

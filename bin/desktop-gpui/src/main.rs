//! Native pitpls application bootstrap.
//!
//! Keep process-wide initialization here. Window state and rendering belong in
//! `main_window`, while application commands and the native menu live in
//! `menus`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod crypto_form;
mod main_window;
mod menus;
mod services;

use gpui::{
    App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size,
};
use gpui_component::Root;

use crate::{assets::Assets, main_window::MainWindow, services::Services};

fn main() {
    let bootstrap = Services::open().map_err(|error| error.to_string());

    Application::new()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);

            let bounds = Bounds::centered(None, size(px(1180.), px(760.)), cx);
            let bootstrap = bootstrap.clone();
            let main_window = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(720.), px(520.))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("pitpls".into()),
                        ..Default::default()
                    }),
                    app_id: Some("com.mngapp.pitpls".to_string()),
                    ..Default::default()
                },
                move |window, cx| {
                    let view = cx.new(|cx| MainWindow::new(bootstrap, window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            );
            if let Err(error) = main_window {
                eprintln!("failed to open pitpls window: {error:#}");
                cx.quit();
                return;
            }

            menus::init(cx);
            cx.activate(true);
        });
}

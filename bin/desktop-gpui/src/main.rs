mod app;
mod components;
mod config;
mod format;
mod navigation;
mod pages;
mod services;
mod theme;

use gpui_kit::*;
use std::sync::Arc;

actions!(desktop_gpui, [Quit]);

fn main() {
    if let Err(error) = run() {
        eprintln!("desktop-gpui: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let Some(config) = config::Config::from_args()? else {
        return Ok(());
    };
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?,
    );
    gpui_kit::application()
        // The component-only bundle omits app icons such as SquarePen and Trash.
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            gpui_kit::component::Theme::sync_system_appearance(None, cx);
            theme::configure_theme(cx);
            cx.bind_keys([
                #[cfg(target_os = "macos")]
                KeyBinding::new("cmd-q", Quit, None),
                #[cfg(not(target_os = "macos"))]
                KeyBinding::new("alt-f4", Quit, None),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::centered(size(px(1180.), px(780.)), cx)),
                    window_min_size: Some(size(px(640.), px(480.))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    window.set_window_title("pitpls");
                    cx.new(|cx| app::Desktop::new(Arc::new(config), runtime, window, cx))
                },
            )
            .expect("Could not open the window");
            cx.activate(true);
        });
    Ok(())
}

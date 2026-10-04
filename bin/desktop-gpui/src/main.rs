mod app;
#[cfg(target_os = "macos")]
mod app_icon;
mod components;
mod config;
mod format;
mod navigation;
mod pages;
mod services;
mod theme;
#[cfg(target_os = "macos")]
mod window_chrome;

use gpui_kit::*;
use std::sync::Arc;

pub(crate) const APP_NAME: &str = "pitpls";

actions!(desktop_gpui, [Quit]);

fn main() {
    if let Err(error) = run() {
        eprintln!("{APP_NAME}: {error}");
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
            #[cfg(target_os = "macos")]
            app_icon::install();
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
                    #[cfg(target_os = "macos")]
                    titlebar: Some(TitlebarOptions {
                        title: None,
                        appears_transparent: true,
                        // Center the native controls in the 64px app toolbar.
                        traffic_light_position: Some(point(px(16.), px(25.))),
                    }),
                    #[cfg(target_os = "macos")]
                    app_owns_titlebar_drag: true,
                    #[cfg(target_os = "macos")]
                    window_min_size: Some(size(px(760.), px(480.))),
                    #[cfg(not(target_os = "macos"))]
                    window_min_size: Some(size(px(640.), px(480.))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    window.set_window_title(APP_NAME);
                    cx.new(|cx| app::Desktop::new(Arc::new(config), runtime, window, cx))
                },
            )
            .expect("Could not open the window");
            cx.activate(true);
        });
    Ok(())
}

mod app;
mod components;
mod config;
mod format;
mod messages;
mod navigation;
mod pages;
mod services;
mod theme;
mod title_row;

use gpui_kit::*;
use std::sync::Arc;

const APP_NAME: &str = "pitpls";

actions!(desktop, [Quit]);
#[cfg(target_os = "macos")]
actions!(
    desktop,
    [
        About,
        Hide,
        HideOthers,
        ShowAll,
        Minimize,
        Zoom,
        ToggleFullScreen,
        CloseWindow
    ]
);

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
        // The default bundle lacks icons such as Trash and Upload.
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            gpui_kit::component::Theme::sync_system_appearance(None, cx);
            theme::configure_theme(cx);
            components::dialog::bind_keys(cx);
            cx.bind_keys([
                #[cfg(target_os = "macos")]
                KeyBinding::new("cmd-q", Quit, None),
                #[cfg(not(target_os = "macos"))]
                KeyBinding::new("alt-f4", Quit, None),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());
            #[cfg(target_os = "macos")]
            install_macos_menus(cx);
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
                        ..Default::default()
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
                    #[cfg(target_os = "macos")]
                    {
                        title_row::measure(window, cx);
                        // The sheet margin follows the measured row.
                        theme::configure_theme(cx);
                    }
                    cx.new(|cx| app::Desktop::new(Arc::new(config), runtime, window, cx))
                },
            )
            .expect("Could not open the window");
            cx.activate(true);
        });
    Ok(())
}

#[cfg(target_os = "macos")]
fn install_macos_menus(cx: &mut App) {
    use gpui_kit::component::input::{Copy, Cut, Paste, Redo, SelectAll, Undo};

    cx.bind_keys([
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("ctrl-cmd-f", ToggleFullScreen, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
    ]);
    // AppKit fills the panel from the bundle's Info.plist.
    cx.on_action(|_: &About, _| {
        let mtm = objc2::MainThreadMarker::new().expect("Actions run on the main thread");
        objc2_app_kit::NSApplication::sharedApplication(mtm).orderFrontStandardAboutPanel(None);
    });
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &Minimize, cx| with_active_window(cx, |window| window.minimize_window()));
    cx.on_action(|_: &Zoom, cx| with_active_window(cx, |window| window.zoom_window()));
    cx.on_action(|_: &ToggleFullScreen, cx| {
        with_active_window(cx, |window| window.toggle_fullscreen())
    });
    cx.on_action(|_: &CloseWindow, cx| with_active_window(cx, |window| window.remove_window()));
    cx.set_menus([
        Menu::new(APP_NAME).items([
            MenuItem::action(format!("About {APP_NAME}"), About),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action(format!("Hide {APP_NAME}"), Hide),
            MenuItem::action("Hide Others", HideOthers),
            MenuItem::action("Show All", ShowAll),
            MenuItem::separator(),
            MenuItem::action(format!("Quit {APP_NAME}"), Quit),
        ]),
        Menu::new("Edit").items([
            MenuItem::os_action("Undo", Undo, OsAction::Undo),
            MenuItem::os_action("Redo", Redo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("Cut", Cut, OsAction::Cut),
            MenuItem::os_action("Copy", Copy, OsAction::Copy),
            MenuItem::os_action("Paste", Paste, OsAction::Paste),
            MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
        ]),
        // AppKit adds full screen and tiling items to the menu named "Window".
        Menu::new("Window").items([
            MenuItem::action("Minimize", Minimize),
            MenuItem::action("Zoom", Zoom),
            MenuItem::separator(),
            MenuItem::action("Close Window", CloseWindow),
        ]),
    ]);
}

/// Window actions are dispatched while the window is being updated, so act on
/// it after the dispatch.
#[cfg(target_os = "macos")]
fn with_active_window(cx: &mut App, action: impl FnOnce(&mut Window) + 'static) {
    let Some(handle) = cx.active_window() else {
        return;
    };
    cx.defer(move |cx| {
        let _ = handle.update(cx, |_, window, _| action(window));
    });
}

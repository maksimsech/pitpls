//! Native application menu and process-wide keyboard commands.
//!
//! Keeping these outside the main window mirrors GPUI's global action model:
//! menu actions remain application-scoped instead of being coupled to a view.

use gpui::{App, KeyBinding, Menu, MenuItem, SystemMenuType, actions};

const APP_NAME: &str = "pitpls";

actions!(pitpls, [Quit]);

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.set_menus(vec![Menu {
        name: APP_NAME.into(),
        items: vec![
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action(format!("Quit {APP_NAME}"), Quit),
        ],
    }]);
}

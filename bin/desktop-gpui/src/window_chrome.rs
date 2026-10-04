use gpui_kit::Window;
use objc2_app_kit::{NSView, NSWindowTitleVisibility};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Show the window's existing `pitpls` title in AppKit's fullscreen bar.
/// GPUI hides it when creating a window with a custom transparent title bar.
pub fn sync_fullscreen_title(window: &Window) {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    // SAFETY: GPUI provides a live NSView for this window, and this function
    // runs on the AppKit main thread. The reference never escapes this call.
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    let Some(native_window) = view.window() else {
        return;
    };
    let visibility = if window.is_fullscreen() {
        NSWindowTitleVisibility::Visible
    } else {
        NSWindowTitleVisibility::Hidden
    };
    if native_window.titleVisibility() != visibility {
        native_window.setTitleVisibility(visibility);
    }
}

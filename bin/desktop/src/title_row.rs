use gpui_kit::*;

/// Fitted to wherever macOS drew the traffic lights: moving them instead makes
/// them jump on leaving full screen.
#[derive(Clone, Copy)]
pub struct TitleRow {
    pub height: Pixels,
    pub lights_end: Pixels,
}

impl Global for TitleRow {}

const MACOS_27: TitleRow = TitleRow {
    height: px(32.),
    lights_end: px(69.),
};

pub fn title_row(cx: &App) -> TitleRow {
    cx.try_global::<TitleRow>().copied().unwrap_or(MACOS_27)
}

#[cfg(target_os = "macos")]
pub fn measure(window: &Window, cx: &mut App) {
    if let Some(row) = read(window) {
        cx.set_global(row);
    }
}

#[cfg(target_os = "macos")]
fn read(window: &Window) -> Option<TitleRow> {
    use objc2_app_kit::{NSView, NSWindowButton};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let RawWindowHandle::AppKit(handle) = HasWindowHandle::window_handle(window).ok()?.as_raw()
    else {
        return None;
    };
    // SAFETY: an AppKit handle points at the window's view, which lives as
    // long as the window, and this runs on the main thread.
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    let native = view.window()?;
    let close = native.standardWindowButton(NSWindowButton::CloseButton)?;
    let zoom = native.standardWindowButton(NSWindowButton::ZoomButton)?;
    // Window coordinates grow upwards from the bottom edge.
    let close = close.convertRect_toView(close.bounds(), None);
    let zoom = zoom.convertRect_toView(zoom.bounds(), None);
    let centre = native.frame().size.height - (close.origin.y + close.size.height / 2.);
    Some(TitleRow {
        height: px(2. * centre as f32),
        lights_end: px((zoom.origin.x + zoom.size.width) as f32),
    })
}

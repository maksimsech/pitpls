use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::NSData;

pub fn install() {
    let mtm = MainThreadMarker::new().expect("The app icon must be set on the main thread");
    // Same icon bundled by the Swift app on feat/macos-native.
    let data = NSData::with_bytes(include_bytes!("../assets/AppIcon.icns"));
    let Some(icon) = NSImage::initWithData(NSImage::alloc(), &data) else {
        eprintln!("{}: could not load the app icon", crate::APP_NAME);
        return;
    };
    // SAFETY: AppKit receives a valid, non-null image on the main thread.
    unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&icon)) };
}

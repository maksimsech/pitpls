use gpui_kit::*;

/// Cancelling the picker completes with `Ok(None)`.
pub fn pick<V: 'static>(
    extension: &'static str,
    window: &Window,
    cx: &Context<V>,
    complete: impl FnOnce(&mut V, Result<Option<String>, String>, &mut Window, &mut Context<V>)
    + 'static,
) -> Task<()> {
    let dialog = rfd::AsyncFileDialog::new()
        .add_filter(extension.to_uppercase(), &[extension])
        .set_parent(window);
    cx.spawn_in(window, async move |view, cx| {
        // Open the panel outside the update that requested it, so AppKit's
        // window callbacks don't re-enter it.
        let result = match dialog.pick_file().await {
            None => Ok(None),
            // Some platforms treat the filter as a hint only.
            Some(file) => {
                let path = file.path();
                if !path
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.eq_ignore_ascii_case(extension))
                {
                    Err(format!("Select a .{extension} file."))
                } else {
                    path.to_str()
                        .map(|path| Some(path.to_owned()))
                        .ok_or_else(|| "The selected path is not valid UTF-8.".into())
                }
            }
        };
        let _ = view.update_in(cx, |view, window, cx| complete(view, result, window, cx));
    })
}

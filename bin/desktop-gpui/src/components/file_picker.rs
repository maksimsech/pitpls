use gpui_kit::*;

/// A retained, window-scoped native picker. Cancellation is not an error.
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
        // Opening the panel from this task keeps AppKit's window callbacks
        // outside the update that requested it.
        let result = match dialog.pick_file().await {
            None => Ok(None),
            // The filter is advisory on some platforms; keep the check.
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

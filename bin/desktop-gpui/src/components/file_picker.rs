use gpui_kit::*;

/// A retained, window-scoped native picker. Cancellation is not an error.
pub fn pick<V: 'static>(
    extension: &'static str,
    window: &Window,
    cx: &Context<V>,
    complete: impl FnOnce(&mut V, Result<Option<String>, String>, &mut Window, &mut Context<V>)
    + 'static,
) -> Task<()> {
    let prompt = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some(format!("Select a {} file", extension.to_uppercase()).into()),
    });
    cx.spawn_in(window, async move |view, cx| {
        let result = prompt
            .await
            .map_err(|e| e.to_string())
            .and_then(|result| result.map_err(|e| e.to_string()))
            .and_then(|paths| {
                let Some(path) = paths.and_then(|paths| paths.into_iter().next()) else {
                    return Ok(None);
                };
                if !path
                    .extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.eq_ignore_ascii_case(extension))
                {
                    return Err(format!("Select a .{extension} file."));
                }
                path.to_str()
                    .map(|path| Some(path.to_owned()))
                    .ok_or_else(|| "The selected path is not valid UTF-8.".into())
            });
        let _ = view.update_in(cx, |view, window, cx| complete(view, result, window, cx));
    })
}

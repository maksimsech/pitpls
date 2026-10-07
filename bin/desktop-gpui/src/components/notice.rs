use super::header;
use crate::theme::palette;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{
        Disableable, Icon, StyledExt,
        button::{Button, ButtonVariants},
        empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle},
        h_flex,
    },
    *,
};

/// A centred notice in the page body, built from the kit's `Empty`.
pub fn notice(
    icon: IconName,
    warning: bool,
    title: String,
    description: impl Into<SharedString>,
    cx: &App,
) -> Empty {
    let p = *palette(cx);
    let media = EmptyMedia::new()
        .size(px(44.))
        .rounded(px(12.))
        .when(warning, |media| {
            media
                .bg(p.warning.opacity(0.08))
                .border_1()
                .border_color(p.warning.opacity(0.3))
                .text_color(p.warning)
        })
        .when(!warning, |media| media.bg(p.selected).text_color(p.muted))
        .child(Icon::new(icon).size(px(20.)));
    Empty::new().pb(px(60.)).header(
        EmptyHeader::new()
            .max_w(px(420.))
            .media(media)
            .title(
                EmptyTitle::new()
                    .text_size(px(15.))
                    .font_semibold()
                    .child(title),
            )
            .description(
                EmptyDescription::new()
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(description.into()),
            ),
    )
}

/// One missing NBP rate fails the calculation for the whole page, as it
/// always has. The error says which rate; importing the year fixes it.
pub fn missing_rate(
    year: Option<i32>,
    error: SharedString,
    disabled: bool,
    on_import: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open_rates: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> Empty {
    let (title, import) = match year {
        Some(year) => (
            format!("Can't calculate {year} yet"),
            SharedString::from(format!("Import {year} from NBP")),
        ),
        None => (
            "Can't calculate all years yet".into(),
            "Import from NBP".into(),
        ),
    };
    notice(IconName::TriangleAlert, true, title, error, cx).content(
        EmptyContent::new().child(
            h_flex()
                .gap(px(8.))
                .mt(px(6.))
                .child(
                    Button::new("missing-import")
                        .h(px(28.))
                        .px(px(11.))
                        .rounded(px(8.))
                        .primary()
                        .accessibility_label(import.clone())
                        .disabled(disabled)
                        .child(div().text_size(px(13.)).font_medium().child(import))
                        .on_click(on_import),
                )
                .child(
                    header::button("missing-rates", None, "Open rates")
                        .disabled(disabled)
                        .on_click(on_open_rates),
                ),
        ),
    )
}

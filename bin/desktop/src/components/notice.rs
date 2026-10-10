use super::{ButtonText, header};
use crate::theme::palette;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{
        Disableable, Icon, StyledExt,
        button::{Button, ButtonVariants},
        dialog::{Dialog, DialogAction, DialogFooter},
        empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle},
        h_flex, v_flex,
    },
    *,
};

pub fn disclaimer(cx: &App) -> Div {
    h_flex()
        .items_start()
        .gap(px(8.))
        .px(px(16.))
        .text_size(px(12.))
        .line_height(relative(1.5))
        .text_color(palette(cx).faint)
        .child(
            Icon::new(IconName::Info)
                .size(px(14.))
                .flex_shrink_0()
                .mt(px(2.)),
        )
        .child(div().flex_1().min_w_0().child(
            "For information only, not tax advice. You're responsible for checking every figure \
             before you file.",
        ))
}

pub fn first_launch(dialog: Dialog, cx: &App) -> Dialog {
    dialog
        .title("Before you start")
        .close_button(false)
        .overlay_closable(false)
        .keyboard(false)
        .child(
            v_flex()
                .gap(px(10.))
                .child(
                    "pitpls is for information only. It is not tax, financial or legal advice, \
                     and not a guide to filing your return.",
                )
                .child(div().text_color(palette(cx).muted).child(
                    "Results aren't guaranteed to be complete or correct. You are responsible \
                     for your tax return: check every figure yourself, and ask a tax adviser if \
                     you're unsure.",
                )),
        )
        .footer(
            DialogFooter::new().child(
                div().child(
                    DialogAction::new().child(
                        Button::new("notice-accept")
                            .text_label("I understand")
                            .primary(),
                    ),
                ),
            ),
        )
}

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
                    header::button("missing-import", None, import)
                        .primary()
                        .disabled(disabled)
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

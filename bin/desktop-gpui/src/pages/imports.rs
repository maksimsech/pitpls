use super::PageView;
use crate::{
    components::{self, Status, file_picker, header},
    navigation::{Page, PageContext},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{
        button::*,
        group_box::{GroupBox, GroupBoxVariants},
        spinner::Spinner,
        *,
    },
    *,
};
use pitpls_app::use_case::import;
use pitpls_importers::{IMPORTERS, InputType, OutputType};
use std::path::Path;

pub struct ImportsPage {
    context: PageContext,
    status: Status,
    active_importer: Option<usize>,
    file_name: Option<SharedString>,
}

impl ImportsPage {
    pub fn new(context: PageContext, _: &mut Window, _: &mut Context<Self>) -> Self {
        Self {
            context,
            status: Status::default(),
            active_importer: None,
            file_name: None,
        }
    }

    fn pick_file(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
        self.active_importer = Some(index);
        self.file_name = None;
        self.status.begin_save();
        self.context.set_locked(true, cx);
        let extension = match IMPORTERS[index].input[0] {
            InputType::Csv => "csv",
            InputType::Pdf => "pdf",
        };
        self.status.task = Some(file_picker::pick(
            extension,
            window,
            cx,
            move |this, result, window, cx| {
                match result {
                    Ok(Some(file)) => {
                        this.file_name = Path::new(&file)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned().into());
                        let kind = IMPORTERS[index].kind;
                        this.status.task = Some(this.context.services.run(
                            window,
                            cx,
                            move |app| async move {
                                let result = import::run_import(&app, kind, file).await?;
                                let counts = IMPORTERS[index]
                                    .output
                                    .iter()
                                    .map(|output| match output {
                                        OutputType::Dividend => count(result.dividends, "dividend"),
                                        OutputType::Crypto => {
                                            count(result.cryptos, "crypto record")
                                        }
                                        OutputType::Interest => {
                                            count(result.interests, "interest record")
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" and ");
                                Ok(format!("Imported {counts}."))
                            },
                            |this, result, _, cx| {
                                this.status.saved(result);
                                // Importers commit each record type separately, so even
                                // a failed import may have added reporting years.
                                this.context.years_changed(cx);
                                this.context.set_locked(false, cx);
                                cx.notify();
                            },
                        ));
                    }
                    Ok(None) => {
                        this.status.busy = false;
                        this.active_importer = None;
                        this.context.set_locked(false, cx);
                    }
                    Err(error) => {
                        this.status.busy = false;
                        this.status.error = Some(error.into());
                        this.context.set_locked(false, cx);
                    }
                }
                cx.notify();
            },
        ));
        cx.notify();
    }
}

impl PageView for ImportsPage {
    fn refresh(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if !self.status.busy {
            self.status.error = None;
            self.status.message = None;
            self.active_importer = None;
            self.file_name = None;
            cx.notify();
        }
    }
}

impl Render for ImportsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .child(header::page(Page::Imports.title(), None, window, cx))
            .child(components::scroll(
                components::page_content().child(self.imports(cx)),
            ))
    }
}

impl ImportsPage {
    fn imports(&self, cx: &mut Context<Self>) -> Div {
        v_flex()
            .gap_3()
            .children(IMPORTERS.iter().enumerate().map(|(index, importer)| {
                let active = self.active_importer == Some(index);
                let importing = active && self.status.busy;
                let outputs = importer
                    .output
                    .iter()
                    .map(|value| match value {
                        OutputType::Dividend => "Dividends",
                        OutputType::Crypto => "Crypto",
                        OutputType::Interest => "Interest",
                    })
                    .collect::<Vec<_>>()
                    .join(" & ");
                let formats = importer
                    .input
                    .iter()
                    .map(|value| match value {
                        InputType::Csv => "CSV",
                        InputType::Pdf => "PDF",
                    })
                    .collect::<Vec<_>>()
                    .join(" / ");
                let file_label = if active {
                    self.file_name.as_deref().unwrap_or(&formats)
                } else {
                    &formats
                };
                GroupBox::new()
                    .id(("importer", index))
                    .outline()
                    .content_style(StyleRefinement::default().p_4().gap_3().bg(if active {
                        cx.theme().group_box
                    } else {
                        cx.theme().background
                    }))
                    .child(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .flex_wrap()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w(px(180.))
                                    .gap_1()
                                    .child(components::section_heading(importer.name))
                                    .child(
                                        div()
                                            .text_base()
                                            .truncate()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("{file_label} · {outputs}")),
                                    ),
                            )
                            .when(importing, |row| {
                                row.child(
                                    h_flex()
                                        .flex_shrink_0()
                                        .h_8()
                                        .gap_2()
                                        .child(Spinner::new().small())
                                        .child(if self.file_name.is_some() {
                                            "Importing…"
                                        } else {
                                            "Choosing file…"
                                        }),
                                )
                            })
                            .when(!importing, |row| {
                                row.child(
                                    Button::new(("import-file", index))
                                        .icon(IconName::Upload)
                                        .label(if active && self.status.error.is_some() {
                                            "Try another file".to_owned()
                                        } else {
                                            format!("Choose {formats}")
                                        })
                                        .outline()
                                        .flex_shrink_0()
                                        .accessibility_label(format!(
                                            "Choose {formats} file for {}",
                                            importer.name,
                                        ))
                                        .disabled(self.status.busy)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.pick_file(index, window, cx)
                                        })),
                                )
                            }),
                    )
                    .when(active, |card| {
                        card.when_some(self.status.message.clone(), |card, message| {
                            card.child(
                                h_flex()
                                    .items_start()
                                    .gap_2()
                                    .pt_3()
                                    .border_t_1()
                                    .border_color(cx.theme().border)
                                    .child(Icon::new(IconName::Check).small())
                                    .child(div().flex_1().min_w_0().child(message)),
                            )
                        })
                        .when_some(
                            self.status.error.clone(),
                            |card, error| {
                                card.child(
                                    v_flex()
                                        .gap_1()
                                        .pt_3()
                                        .border_t_1()
                                        .border_color(cx.theme().border)
                                        .text_color(cx.theme().danger)
                                        .child(div().font_medium().child("Couldn't import file"))
                                        .child(error),
                                )
                            },
                        )
                    })
            }))
    }
}

fn count(count: u64, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

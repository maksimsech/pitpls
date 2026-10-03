use super::PageView;
use crate::{
    components::{self, Status, file_picker},
    navigation::PageContext,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    assets::IconName,
    component::{button::*, list::ListItem, tag::Tag},
};
use gpui_kit::{component::*, *};
use pitpls_app::use_case::import;
use pitpls_importers::{IMPORTERS, ImporterKind, InputType, OutputType};

pub struct ImportsPage {
    context: PageContext,
    status: Status,
}

impl ImportsPage {
    pub fn new(context: PageContext, _: &mut Window, _: &mut Context<Self>) -> Self {
        Self {
            context,
            status: Status::default(),
        }
    }

    fn pick_file(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.status.busy {
            return;
        }
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
                        let kind = match &IMPORTERS[index].kind {
                            ImporterKind::T212 => ImporterKind::T212,
                            ImporterKind::Revolut => ImporterKind::Revolut,
                            ImporterKind::Coinbase => ImporterKind::Coinbase,
                        };
                        this.status.task = Some(this.context.services.run(window, cx, move |app| async move {
                        let result = import::run_import(&app, kind, file).await?;
                        Ok(format!("Imported {} dividends, {} crypto records and {} interest records.", result.dividends, result.cryptos, result.interests))
                    }, |this, result, _, cx| {
                        this.status.saved(result);
                        this.context.set_locked(false, cx);
                        cx.notify();
                    }));
                    }
                    result => {
                        this.status.busy = false;
                        this.status.error = result.err().map(Into::into);
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
            cx.notify();
        }
    }
}

impl Render for ImportsPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        components::scroll(
            v_flex()
                .gap_4()
                .p_5()
                .when(self.status.is_visible(), |view| {
                    view.child(self.status.render(cx))
                })
                .child(self.imports(cx)),
        )
    }
}

impl ImportsPage {
    fn imports(&self, cx: &mut Context<Self>) -> Div {
        v_flex()
            .gap_3()
            .children(IMPORTERS.iter().enumerate().map(|(index, importer)| {
                let outputs = importer
                    .output
                    .iter()
                    .map(|value| match value {
                        OutputType::Dividend => "Dividend",
                        OutputType::Crypto => "Crypto",
                        OutputType::Interest => "Interest",
                    })
                    .collect::<Vec<_>>();
                let formats = importer
                    .input
                    .iter()
                    .map(|value| match value {
                        InputType::Csv => "CSV",
                        InputType::Pdf => "PDF",
                    })
                    .collect::<Vec<_>>()
                    .join(" / ");
                ListItem::new(("importer", index))
                    .accessibility_label(format!("Import {}", importer.name))
                    .disabled(self.status.busy)
                    .text_sm()
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.pick_file(index, window, cx)),
                    )
                    .gap_4()
                    .p_4()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .child(div().font_semibold().child(importer.name))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(formats),
                                    ),
                            )
                            .child(
                                h_flex().gap_2().children(
                                    outputs
                                        .into_iter()
                                        .map(|output| Tag::secondary().outline().child(output)),
                                ),
                            ),
                    )
                    .suffix({
                        let busy = self.status.busy;
                        let page = cx.entity().downgrade();
                        move |_, _| {
                            Button::new(("import-file", index))
                                .icon(IconName::Upload)
                                .ghost()
                                .accessibility_label(format!("Import {}", importer.name))
                                .disabled(busy)
                                .on_click({
                                    let page = page.clone();
                                    move |_, window, cx| {
                                        cx.stop_propagation();
                                        let _ = page.update(cx, |this, cx| {
                                            this.pick_file(index, window, cx)
                                        });
                                    }
                                })
                        }
                    })
            }))
    }
}

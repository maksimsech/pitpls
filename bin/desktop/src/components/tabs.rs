use crate::theme::{palette, tabular_digits};
use gpui_kit::{
    component::{button::*, *},
    prelude::FluentBuilder,
    *,
};
use std::{rc::Rc, sync::Arc};

pub struct TabItem<K> {
    pub key: K,
    pub label: SharedString,
    pub count: usize,
}

pub fn tabs<K: PartialEq + 'static>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    items: Vec<TabItem<K>>,
    selected: K,
    on_select: impl Fn(&K, &mut Window, &mut App) + 'static,
    caption: impl Into<SharedString>,
    cx: &App,
) -> Div {
    let p = *palette(cx);
    let id = Arc::new(id.into());
    let on_select = Rc::new(on_select);
    h_flex()
        .flex_shrink_0()
        .h(px(40.))
        .px(px(20.))
        .pb(px(8.))
        .gap(px(12.))
        .child(
            h_flex()
                .id(id.as_ref().clone())
                .flex_shrink_0()
                .role(Role::TabList)
                .aria_label(label)
                .gap(px(2.))
                .p(px(2.))
                .rounded(px(9.))
                .bg(p.selected)
                .children(items.into_iter().enumerate().map(|(index, item)| {
                    let active = selected == item.key;
                    let on_select = on_select.clone();
                    Button::new(ElementId::NamedChild(id.clone(), index.to_string().into()))
                        .ghost()
                        .role(Role::Tab)
                        .h(px(24.))
                        .px(px(10.))
                        .gap(px(6.))
                        .rounded(px(7.))
                        .accessibility_label(format!("{}, {}", item.label, item.count))
                        .when(active, |tab| tab.bg(cx.theme().tokens.button).shadow_xs())
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_medium()
                                .text_color(if active { p.text } else { p.muted })
                                .child(item.label),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(p.faint)
                                .font_features(tabular_digits())
                                .child(item.count.to_string()),
                        )
                        .on_click(move |_, window, cx| on_select(&item.key, window, cx))
                })),
        )
        .child(div().flex_1())
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(px(12.))
                .text_color(p.faint)
                .child(caption.into()),
        )
}

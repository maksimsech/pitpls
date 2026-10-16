use gpui_kit::{App, Global};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Language {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "pl")]
    Polish,
}

impl Language {
    fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Polish => "pl",
        }
    }

    /// Each in its own language, so it can be found whichever one is shown.
    pub fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Polish => "Polski",
        }
    }

    fn system() -> Self {
        match sys_locale::get_locale() {
            Some(locale) if locale.to_lowercase().starts_with("pl") => Self::Polish,
            _ => Self::English,
        }
    }
}

struct LanguageChoice(Option<Language>);

impl Global for LanguageChoice {}

pub fn language_choice(cx: &App) -> Option<Language> {
    cx.try_global::<LanguageChoice>()
        .and_then(|choice| choice.0)
}

/// `None` follows the system. Views and menus show it once they're redrawn.
pub fn apply_language(language: Option<Language>, cx: &mut App) {
    cx.set_global(LanguageChoice(language));
    gpui_kit::component::set_locale(language.unwrap_or_else(Language::system).code());
}

/// `key.one` or `key.other` in English, and `key.one`, `key.few` or
/// `key.many` in Polish.
pub fn plural_key(key: &str, count: u64) -> String {
    format!("{key}.{}", plural_form(Some(count)))
}

/// A fraction takes Polish `many`: the nouns counted here have the same form
/// for both.
pub fn plural_key_decimal(key: &str, value: Decimal) -> String {
    let whole = value.fract().is_zero().then(|| value.to_u64()).flatten();
    format!("{key}.{}", plural_form(whole))
}

fn plural_form(count: Option<u64>) -> &'static str {
    let polish = &*rust_i18n::locale() == "pl";
    match count {
        Some(1) => "one",
        Some(count)
            if polish && (2..=4).contains(&(count % 10)) && !(12..=14).contains(&(count % 100)) =>
        {
            "few"
        }
        _ if polish => "many",
        _ => "other",
    }
}

pub fn plural(key: &str, count: u64) -> Cow<'static, str> {
    t!(plural_key(key, count), count = count)
}

/// "a, b and c", or "a, b, c and 2 more" past four.
pub fn and_list(items: &[impl AsRef<str>]) -> String {
    const SHOWN: usize = 4;
    let items = items.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    match items.as_slice() {
        [] => String::new(),
        [one] => (*one).to_owned(),
        _ if items.len() > SHOWN => t!(
            "list.and_more",
            items = items[..SHOWN - 1].join(", "),
            count = items.len() - (SHOWN - 1)
        )
        .into_owned(),
        [rest @ .., last] => t!("list.and", items = rest.join(", "), last = last).into_owned(),
    }
}

pub fn or_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => t!("list.or", items = rest.join(", "), last = last).into_owned(),
    }
}

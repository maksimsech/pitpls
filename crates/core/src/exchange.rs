use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::common::{Country, Mic};

// Lower case inside a title-case name; other short words are mostly abbreviations (AG, PLC, MTF).
const LOWER_CASE_WORDS: [&str; 15] = [
    "AND", "OF", "THE", "FOR", "DA", "DAS", "DE", "DEL", "DES", "DI", "DO", "DOS", "DU", "LA", "LE",
];
const ABBREVIATION_MAX_LEN: usize = 3;

#[derive(Clone, Debug)]
pub struct Exchange {
    pub mic: Mic,
    pub operating_mic: Mic,
    pub kind: ExchangeKind,
    pub name: String,
    pub acronym: Option<String>,
    pub country: Country,
    pub city: Option<String>,
    pub status: ExchangeStatus,
    pub last_update: Option<NaiveDate>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExchangeKind {
    #[serde(rename = "OPRT")]
    Operating,
    #[serde(rename = "SGMT")]
    Segment,
}

// `Updated` is a live MIC changed in the latest release.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExchangeStatus {
    #[serde(rename = "ACTIVE")]
    Active,
    #[serde(rename = "UPDATED")]
    Updated,
    #[serde(rename = "EXPIRED")]
    Expired,
}

impl Exchange {
    pub fn display_name(&self) -> String {
        match &self.acronym {
            Some(acronym) => acronym.clone(),
            None => title_case(&self.name),
        }
    }
}

fn title_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut word = String::new();
    let mut first = true;
    for c in name.chars() {
        if c.is_alphanumeric() {
            word.push(c);
            continue;
        }
        push_word(&mut result, &word, &mut first);
        word.clear();
        result.push(c);
    }
    push_word(&mut result, &word, &mut first);
    result
}

fn push_word(result: &mut String, word: &str, first: &mut bool) {
    if word.is_empty() {
        return;
    }
    let lower_case = LOWER_CASE_WORDS.contains(&word);
    let first_word = std::mem::replace(first, false);

    if lower_case && !first_word {
        result.extend(word.chars().flat_map(char::to_lowercase));
    } else if !lower_case
        && (word.chars().count() <= ABBREVIATION_MAX_LEN
            || word.chars().any(|c| c.is_ascii_digit()))
    {
        result.push_str(word);
    } else {
        let mut chars = word.chars();
        result.extend(chars.next());
        result.extend(chars.flat_map(char::to_lowercase));
    }
}

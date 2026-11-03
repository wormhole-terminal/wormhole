//! The window's words in the active language: every id of `words_table` with its text, for the window to show
//! instead of its English (which it keeps only as the source of the id). Changing the language changes the
//! snapshot's words, and the settings global already notifies the window.

use serde_json::{Map, Value};
use wormhole_text::text;

use super::words_table::WORDS;

pub(super) fn snapshot() -> Value {
    Value::Object(
        WORDS
            .iter()
            .map(|(id, key)| ((*id).to_string(), Value::String(text(*key).to_string())))
            .collect::<Map<_, _>>(),
    )
}

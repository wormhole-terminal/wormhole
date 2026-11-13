//! What the Theme Studio's editor asks of core while a value is typed and while a theme is read for contrast: the
//! names and functions an expression offers at the caret (`theme::tokens::completions`) and the fixes for the pairs
//! that fall short (`contrast_issues`). Core proposes, the Studio applies: nothing here changes a file.

use std::path::Path;

use wormhole_core::config::resolve_theme_file_in;
use wormhole_core::theme::tokens::{completions, contrast_issues, read_theme_file};

use crate::error::FfiError;
use crate::studio_files::folded;

/// One name the expression field offers.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct StudioCompletion {
    /// What the list shows: the name, or the function with what it takes.
    pub label: String,
    /// What takes the place of the name being typed.
    pub insert: String,
    pub function: bool,
}

/// What the field offers at the caret, and the bytes of the text the taken one replaces.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct StudioCompletions {
    /// UTF-8 byte offsets into the text typed.
    pub start: u32,
    pub end: u32,
    pub items: Vec<StudioCompletion>,
}

/// The names to offer for `text` typed for `target`, the caret at byte `caret`; none while the caret is not after a
/// name, or once the name is a colour that exists (Return then applies it).
#[uniffi::export]
pub fn studio_completions(text: String, caret: u32, target: String) -> Option<StudioCompletions> {
    let found = completions(&text, usize::try_from(caret).ok()?, &target)?;
    Some(StudioCompletions {
        start: u32::try_from(found.range.start).ok()?,
        end: u32::try_from(found.range.end).ok()?,
        items: found
            .items
            .into_iter()
            .map(|c| StudioCompletion { label: c.label, insert: c.insert, function: c.function })
            .collect(),
    })
}

/// A suggested new value for a token, written as an expression the person applies or not.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct StudioContrastFix {
    pub token: String,
    pub expression: String,
    /// What the button says ("Lighten to 3:1").
    pub label: String,
}

/// A pair that falls short, and what to do about it.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioContrastIssue {
    pub fg: String,
    pub bg: String,
    pub underlay: String,
    pub ratio: f32,
    pub min: f32,
    /// None for the terminal palette's own pair, or when `follows` covers it.
    pub fix: Option<StudioContrastFix>,
    /// The failing token this pair's text follows: that token's fix clears this pair too.
    pub follows: Option<String>,
}

/// The pairs of the theme `text` (the shown appearance of a pair) that fall short, each with core's fix, which wraps
/// what the person typed rather than a resolved colour and follows tokens that merely name a failing one.
#[uniffi::export]
pub fn studio_contrast_issues(
    themes_dir: String,
    text: String,
    shown: Option<String>,
) -> Result<Vec<StudioContrastIssue>, FfiError> {
    let file =
        folded(&read_theme_file(&text).map_err(|reason| FfiError::Theme { reason })?, shown.as_deref());
    let tokens =
        resolve_theme_file_in(Path::new(&themes_dir), &file).map_err(|reason| FfiError::Theme { reason })?;
    Ok(contrast_issues(&tokens, &file.ui)
        .into_iter()
        .map(|issue| StudioContrastIssue {
            fg: issue.check.fg.into(),
            bg: issue.check.bg.into(),
            underlay: issue.check.underlay.into(),
            ratio: issue.check.ratio,
            min: issue.check.min,
            fix: issue.fix.map(|f| StudioContrastFix {
                token: f.token,
                expression: f.expression,
                label: f.label,
            }),
            follows: issue.follows.map(Into::into),
        })
        .collect())
}

//! What the Theme Studio's editor asks of core while a value is typed and while a theme is read for contrast: the
//! names and functions an expression offers at the caret (`theme::tokens::completions`), every contrast pair
//! measured (`contrast_checks`), the fixes for the pairs that fall short (`contrast_issues`) and who draws with a
//! token (`TOKEN_USES`, "Used by"). Core proposes, the Studio applies: nothing here changes a file.

use std::path::Path;

use wormhole_core::config::resolve_theme_file_in;
use wormhole_core::theme::tokens::{Area, TOKEN_USES, completions, contrast_issues, read_theme_file};

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

/// One text-on-background pair of core's `CONTRAST_PAIRS`, measured as the window shows it.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioContrastCheck {
    pub fg: String,
    pub bg: String,
    pub underlay: String,
    pub ratio: f32,
    pub min: f32,
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

/// Every pair measured and the ones that fall short, from one resolve: the list's marks, the Low filter, the
/// editor's Contrast section and the card all read this, so they cannot disagree.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct StudioContrast {
    /// Every pair of `CONTRAST_PAIRS` the theme has the tokens for, in its order.
    pub checks: Vec<StudioContrastCheck>,
    /// The ones that fall short, in the same order, each with core's fix.
    pub issues: Vec<StudioContrastIssue>,
}

/// The pairs of the theme `text` (the shown appearance of a pair), as Increase Contrast shows them when
/// `increase_contrast`: every one measured (`contrast_checks`), and those that fall short with core's fix, which
/// wraps what the person typed rather than a resolved colour and follows tokens that merely name a failing one.
#[uniffi::export]
pub fn studio_contrast(
    themes_dir: String,
    text: String,
    shown: Option<String>,
    increase_contrast: bool,
) -> Result<StudioContrast, FfiError> {
    let file =
        folded(&read_theme_file(&text).map_err(|reason| FfiError::Theme { reason })?, shown.as_deref());
    let mut tokens =
        resolve_theme_file_in(Path::new(&themes_dir), &file).map_err(|reason| FfiError::Theme { reason })?;
    if increase_contrast {
        tokens = tokens.high_contrast();
    }
    let checks = tokens
        .contrast_checks()
        .into_iter()
        .map(|c| StudioContrastCheck {
            fg: c.fg.into(),
            bg: c.bg.into(),
            underlay: c.underlay.into(),
            ratio: c.ratio,
            min: c.min,
        })
        .collect();
    let issues = contrast_issues(&tokens, &file.ui)
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
        .collect();
    Ok(StudioContrast { checks, issues })
}

/// A component of the design and the tokens it draws with (core's `TokenUse`): what "Used by" names, what a metric's
/// "n uses" counts and what the token list's area filter keeps.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct StudioTokenUse {
    pub id: String,
    pub label: String,
    /// The part of the app: `sidebar`, `panel`, `chrome`, `settings`, `ambient`, or `all` for what every part shows.
    pub area: String,
    /// UI tokens and `metrics.*`; `terminal.*` and `syntax.*` stand for every terminal or syntax colour.
    pub tokens: Vec<String>,
}

/// Core's `TOKEN_USES`, in its order.
#[uniffi::export]
pub fn studio_token_uses() -> Vec<StudioTokenUse> {
    TOKEN_USES
        .iter()
        .map(|u| StudioTokenUse {
            id: u.id.into(),
            label: u.label.into(),
            area: match u.area {
                Area::Sidebar => "sidebar",
                Area::Panel => "panel",
                Area::Chrome => "chrome",
                Area::Settings => "settings",
                Area::Ambient => "ambient",
                Area::Basics => "all",
            }
            .into(),
            tokens: u.tokens.iter().map(|t| (*t).to_string()).collect(),
        })
        .collect()
}

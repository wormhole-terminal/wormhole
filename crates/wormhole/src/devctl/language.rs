//! `language`: the interface's language, including the pseudo-language that only devctl can ask for.

use anyhow::{Result, anyhow};
use wormhole_text::Language;

use super::Ctx;

/// `language <tag|system>`: speak that language (`en`, `en-XA`) or the system's again, and redraw the
/// windows; without an argument, the one in use and the ones to choose from.
pub(super) fn language(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let language = match arg.trim() {
        "" => {
            let tags: Vec<&str> = Language::all().map(Language::tag).collect();
            return Ok(format!("language {} ({})\n", wormhole_text::language().tag(), tags.join(", ")));
        }
        "system" => crate::app::language::from_system(),
        tag => Language::from_tag(tag).ok_or_else(|| {
            let tags: Vec<&str> = Language::all().map(Language::tag).collect();
            anyhow!("expected system or one of {}, got {tag:?}", tags.join(", "))
        })?,
    };
    crate::app::language::change(language, ctx.cx);
    Ok(String::new())
}

//! A theme resolved at run time by `wormhole-core`'s own loader, for what the bundled dump can't hold:
//! a user's theme file (`.jsonc`, with `extends`), the person's primary and branch colour, a window
//! opacity. The order is the app's (`settings/derive.rs`): the theme, `with_choices`, then Increase
//! Contrast's variant of that, and translucency on the result. The answer is one JSON object in the
//! dump's shape (`theme_json`). Nothing here writes a file; the themes folder is only read.

use std::path::Path;

use wormhole_core::theme::registry::{UserThemes, builtin_named};
use wormhole_core::theme::tokens::{BranchTint, Color, ColorChoices, ThemeTokens, Translucency};

use crate::theme_json::{appearance, string, terminal, variant};

/// What to resolve. `theme` is what `settings.theme` holds: a name, a slug, or a path.
pub(crate) struct Request<'a> {
    pub(crate) theme: &'a str,
    /// The folder user themes live in (read only); `None` has none.
    pub(crate) themes_dir: Option<&'a Path>,
    /// `#rrggbb` replaces the theme's primary; empty keeps the theme's own.
    pub(crate) primary: &'a str,
    /// `""` the theme's own branch colour, `"primary"`, or `#rrggbb`.
    pub(crate) branch: &'a str,
    pub(crate) increase_contrast: bool,
    pub(crate) opacity: f32,
    pub(crate) blur: bool,
}

pub(crate) fn resolve(req: &Request<'_>) -> Result<String, String> {
    let themes = UserThemes::read(req.themes_dir.unwrap_or(Path::new("")));
    let (id, base) = find(&themes, req.theme)?;
    let choices = ColorChoices {
        primary: if req.primary.trim().is_empty() {
            None
        } else {
            Some(Color::from_hex(req.primary.trim())?)
        },
        branch: match req.branch.trim() {
            "" => None,
            "primary" => Some(BranchTint::Primary),
            hex => Some(BranchTint::Color(Color::from_hex(hex)?.with_alpha(1.0))),
        },
    };
    let chosen = if choices.is_empty() { base } else { base.with_choices(choices) };
    let shown = if req.increase_contrast { chosen.high_contrast() } else { chosen };
    let look = Translucency { opacity: req.opacity, blur: req.blur };
    Ok(format!(
        "{{\"id\": {}, \"name\": {}, \"family\": null, \"appearance\": \"{}\",\n \"terminal\": {},\n \"normal\": {}}}",
        string(&id),
        string(&shown.name),
        appearance(&shown),
        terminal(&shown),
        variant(&shown, Some(look)),
    ))
}

/// The user's themes in a folder, as `[{"id", "name", "path", "appearance"|null, "error"|null}]` (`path`: the file
/// `settings.theme` holds when a theme was imported or saved from the Studio).
pub(crate) fn list(dir: &Path) -> String {
    let themes = UserThemes::read(dir);
    let items = themes.themes.iter().map(|t| match themes.resolve(t) {
        Ok(tokens) => format!(
            "{{\"id\": {}, \"name\": {}, \"path\": {}, \"appearance\": \"{}\", \"error\": null}}",
            string(&t.slug),
            string(&t.name),
            string(&t.path.to_string_lossy()),
            appearance(&tokens)
        ),
        Err(err) => format!(
            "{{\"id\": {}, \"name\": {}, \"path\": {}, \"appearance\": null, \"error\": {}}}",
            string(&t.slug),
            string(&t.name),
            string(&t.path.to_string_lossy()),
            string(&err)
        ),
    });
    format!("[{}]", items.collect::<Vec<_>>().join(", "))
}

/// A user theme first (by name or slug), then a bundled one, then a theme file in the folder by path:
/// the order `settings.theme` is resolved in.
fn find(themes: &UserThemes, value: &str) -> Result<(String, ThemeTokens), String> {
    let user = themes
        .named(value)
        .or_else(|| themes.themes.iter().find(|t| t.slug == value))
        .or_else(|| themes.by_path(value));
    if let Some(theme) = user {
        return themes.resolve(theme).map(|tokens| (theme.slug.clone(), tokens));
    }
    if let Some(tokens) = builtin_named(value) {
        return Ok((crate::theme_dump::id(&tokens.name), tokens.clone()));
    }
    Err(format!("unknown theme {value:?}"))
}

#[cfg(test)]
#[allow(clippy::disallowed_methods, reason = "a test writing its own scratch theme folder")]
mod tests {
    use super::{Request, list, resolve};
    use std::path::Path;

    fn request<'a>(theme: &'a str, dir: Option<&'a Path>) -> Request<'a> {
        Request {
            theme,
            themes_dir: dir,
            primary: "",
            branch: "",
            increase_contrast: false,
            opacity: 1.0,
            blur: false,
        }
    }

    #[test]
    fn a_bundled_theme_resolves_to_its_dumped_tokens() {
        let json = resolve(&request("nord", None)).expect("resolves");
        assert!(json.contains("\"id\": \"nord\""));
        assert!(json.contains("\"translucent\": null"), "opacity 1 is opaque");
    }

    #[test]
    fn an_unknown_theme_is_an_error_not_a_default() {
        assert!(resolve(&request("no such theme", None)).is_err());
    }

    #[test]
    fn a_user_theme_file_with_comments_extends_a_bundled_one() {
        let dir = std::env::temp_dir().join(format!("wh-theme-resolve-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(
            dir.join("Mine.json"),
            "// my theme\n{ \"name\": \"Mine\", \"extends\": \"Nord\", \"ui\": { \"primary\": \"#ff6600\" } }\n",
        )
        .expect("theme file");
        let json = resolve(&request("Mine", Some(&dir))).expect("resolves");
        assert!(json.contains("\"name\": \"Mine\""));
        assert!(json.contains("\"primary\": [1, 0.4, 0, 1]"), "the file's primary is applied: {json}");
        assert!(list(&dir).contains("\"name\": \"Mine\""));
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn choices_and_contrast_follow_the_apps_order() {
        let mut req = request("wormhole.dark", None);
        req.primary = "#ff6600";
        req.branch = "primary";
        req.increase_contrast = true;
        let json = resolve(&req).expect("resolves");
        assert!(json.contains("\"primary\": [1, 0.4, 0, 1]"));
        assert!(json.contains("\"focus_ring_width\": 3"), "the contrast variant of the chosen colours");
    }

    #[test]
    fn a_translucent_window_carries_the_judged_text() {
        let mut req = request("wormhole.dark", None);
        req.opacity = 0.8;
        req.blur = true;
        assert!(resolve(&req).expect("resolves").contains("\"selectedMuted\""));
    }
}

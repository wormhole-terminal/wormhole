//! Slugs: the short names a theme is referred to by (`wormhole.light`, `tokyo_night`,
//! `my_theme`), the same rule as a token's name: a dot is a level, an underscore joins the
//! words of one name.
//!
//! - The grammar is parts of `[a-z0-9_]` joined by dots ([`is_slug`]).
//! - Every built-in theme has one, written in [`BUILTIN_SLUGS`] by hand and checked against
//!   the themes themselves by a test; they are reserved ([`is_reserved`]).
//! - A user theme's slug is the `id` stored in its file. A file from before ids gets one from
//!   its display name ([`derive_id`]) when it is first saved; until then [`assign_ids`] gives
//!   it the same one in memory, and settles clashes the same way every time.

use std::collections::HashSet;

/// What a slug is, for messages.
pub const GRAMMAR: &str = "lowercase letters, digits and `_`, with `.` between levels";

/// The longest slug accepted.
pub const MAX_SLUG_LEN: usize = 96;

/// Every built-in theme: its display name (what `settings.theme` has held until now) and its
/// slug. Wormhole's own themes are families (`wormhole.light` is "Wormhole Light"); a family
/// has no theme of its own, its slug means its first version.
pub const BUILTIN_SLUGS: &[(&str, &str)] = &[
    // The imported palettes, in `theme::builtins` order.
    ("Tokyo Night", "tokyo_night"),
    ("Catppuccin Mocha", "catppuccin_mocha"),
    ("Catppuccin Latte", "catppuccin_latte"),
    ("Dracula", "dracula"),
    ("Nord", "nord"),
    ("Gruvbox Dark", "gruvbox_dark"),
    ("One Dark", "one_dark"),
    ("Solarized Dark", "solarized_dark"),
    ("Rose Pine", "rose_pine"),
    ("Kanagawa Wave", "kanagawa_wave"),
    ("Everforest Dark", "everforest_dark"),
    ("GitHub Dark", "github_dark"),
    ("GitHub Light", "github_light"),
    ("Monokai Pro", "monokai_pro"),
    // Wormhole's own, in `tokens::own` order.
    ("Wormhole Dark", "wormhole.dark"),
    ("Wormhole Light", "wormhole.light"),
    ("Wormhole Electric Blue Dark", "wormhole_electric_blue.dark"),
    ("Wormhole Electric Blue Light", "wormhole_electric_blue.light"),
    ("Wormhole Vivid Blue Dark", "wormhole_vivid_blue.dark"),
    ("Wormhole Vivid Blue Light", "wormhole_vivid_blue.light"),
    ("Wormhole Klein Blue Dark", "wormhole_klein_blue.dark"),
    ("Wormhole Klein Blue Light", "wormhole_klein_blue.light"),
    ("Wormhole Cobalt Dark", "wormhole_cobalt.dark"),
    ("Wormhole Cobalt Light", "wormhole_cobalt.light"),
    ("Wormhole Azure Dark", "wormhole_azure.dark"),
    ("Wormhole Azure Light", "wormhole_azure.light"),
    ("Wormhole Sky Dark", "wormhole_sky.dark"),
    ("Wormhole Sky Light", "wormhole_sky.light"),
    ("Wormhole Ultramarine Dark", "wormhole_ultramarine.dark"),
    ("Wormhole Ultramarine Light", "wormhole_ultramarine.light"),
    ("Wormhole Volt Dark", "wormhole_volt.dark"),
    ("Wormhole Volt Light", "wormhole_volt.light"),
    ("Wormhole Signal Orange Dark", "wormhole_signal_orange.dark"),
    ("Wormhole Signal Orange Light", "wormhole_signal_orange.light"),
    ("Wormhole Hot Pink Dark", "wormhole_hot_pink.dark"),
    ("Wormhole Hot Pink Light", "wormhole_hot_pink.light"),
    ("Wormhole Ultraviolet Dark", "wormhole_ultraviolet.dark"),
    ("Wormhole Ultraviolet Light", "wormhole_ultraviolet.light"),
    ("Wormhole Acid Cyan Dark", "wormhole_acid_cyan.dark"),
    ("Wormhole Acid Cyan Light", "wormhole_acid_cyan.light"),
    ("Wormhole Red Dark", "wormhole_red.dark"),
    ("Wormhole Red Light", "wormhole_red.light"),
    ("Wormhole White Dark", "wormhole_white.dark"),
    ("Wormhole White Light", "wormhole_white.light"),
    ("Nebula", "nebula"),
    ("Solar Flare", "solar_flare"),
    ("Borealis", "borealis"),
    ("Event Horizon", "event_horizon"),
    ("Void", "void"),
    ("Crimson Dark", "crimson.dark"),
    ("Crimson Light", "crimson.light"),
    ("Cyberpunk", "cyberpunk"),
    ("Matrix", "matrix"),
    ("Soltari Light", "soltari.light"),
    ("Soltari Dark", "soltari.dark"),
];

/// One level of a slug: lowercase letters, digits and `_`, not empty.
pub fn is_part(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Whether `s` is a slug: parts joined by single dots, within [`MAX_SLUG_LEN`].
pub fn is_slug(s: &str) -> bool {
    s.len() <= MAX_SLUG_LEN && s.split('.').all(is_part)
}

/// The slug of the built-in theme called `name` (exactly as it is listed).
pub fn builtin_slug(name: &str) -> Option<&'static str> {
    BUILTIN_SLUGS.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// The built-in theme a slug names; a family's slug (`wormhole`) names its first version.
pub fn builtin_name(slug: &str) -> Option<&'static str> {
    let exact = BUILTIN_SLUGS.iter().find(|(_, s)| *s == slug);
    let family = || BUILTIN_SLUGS.iter().find(|(_, s)| s.split_once('.').is_some_and(|(f, _)| f == slug));
    exact.or_else(family).map(|(n, _)| *n)
}

/// Whether a user theme may not take `slug`: it is a built-in's, or the family of some.
pub fn is_reserved(slug: &str) -> bool {
    builtin_name(slug).is_some()
}

/// The slug of version `short` of the theme `id` (`my_theme.dark`).
pub fn version_slug(id: &str, short: &str) -> String {
    format!("{id}.{short}")
}

/// A slug part made from words: lowercase ASCII letters and digits, every other run of
/// characters one `_`, no `_` at either end; `fallback` when nothing is left. Cut to 64.
pub fn part_from(words: &str, fallback: &str) -> String {
    let mut out = String::new();
    for c in words.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let part = out.trim_matches('_');
    let part: String = part.chars().take(64).collect();
    let part = part.trim_end_matches('_');
    if part.is_empty() { fallback.to_string() } else { part.to_string() }
}

/// The id a theme named `name` gets when it has none: `"My Theme"` is `my_theme`.
pub fn derive_id(name: &str) -> String {
    part_from(name, "theme")
}

/// What one theme asks for.
#[derive(Clone, Copy, Debug)]
pub struct IdRequest<'a> {
    /// The `id` the file stores, if it has one.
    pub stored: Option<&'a str>,
    /// Its display name, for a file without an id.
    pub name: &'a str,
}

/// What a theme got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assigned {
    pub id: String,
    /// Set when it is not the id the theme asked for.
    pub clash: Option<Clash>,
}

/// Why a theme did not get the id it asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clash {
    pub wanted: String,
    pub reason: ClashReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClashReason {
    /// A built-in theme has it.
    Reserved,
    /// Another theme of the list (by its index) has it.
    TakenBy(usize),
}

/// Give every theme of `themes` (in a fixed order, such as the sorted file paths) a slug
/// nobody else has and no built-in has. A theme that stores its id is served before one
/// that has none, each group in list order, so the same files always give the same answer
/// and adding a file without an id never takes another's. The loser of a clash gets the
/// wanted slug followed by `_2`, `_3`, ..., and the clash is reported.
pub fn assign_ids(themes: &[IdRequest<'_>]) -> Vec<Assigned> {
    let wanted: Vec<String> = themes
        .iter()
        .map(|t| match t.stored {
            Some(id) if is_slug(id) => id.to_string(),
            _ => derive_id(t.name),
        })
        .collect();
    let stored_first = (0..themes.len()).filter(|&i| themes[i].stored.is_some_and(is_slug));
    let derived = (0..themes.len()).filter(|&i| !themes[i].stored.is_some_and(is_slug));
    let mut owner: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut taken: HashSet<String> = HashSet::new();
    let mut out: Vec<Option<Assigned>> = vec![None; themes.len()];
    for i in stored_first.chain(derived) {
        let want = &wanted[i];
        let reason = if is_reserved(want) {
            Some(ClashReason::Reserved)
        } else {
            owner.get(want).map(|&by| ClashReason::TakenBy(by))
        };
        let id = match reason {
            None => want.clone(),
            Some(_) => (2..)
                .map(|n| format!("{want}_{n}"))
                .find(|c| !is_reserved(c) && !taken.contains(c))
                .unwrap_or_else(|| want.clone()),
        };
        taken.insert(id.clone());
        owner.entry(id.clone()).or_insert(i);
        out[i] = Some(Assigned { id, clash: reason.map(|reason| Clash { wanted: want.clone(), reason }) });
    }
    out.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::tokens::builtins;

    #[test]
    fn the_table_names_every_built_in_theme_and_nothing_else() {
        let mut listed: Vec<&str> = BUILTIN_SLUGS.iter().map(|(n, _)| *n).collect();
        let mut actual: Vec<&str> = builtins().iter().map(|t| t.name.as_str()).collect();
        listed.sort_unstable();
        actual.sort_unstable();
        assert_eq!(listed, actual);
    }

    #[test]
    fn slugs_follow_the_grammar_and_are_unique() {
        let mut seen = HashSet::new();
        for (name, slug) in BUILTIN_SLUGS {
            assert!(is_slug(slug), "{name}: {slug}");
            assert!(seen.insert(*slug), "{slug} twice");
            // The slug is the name in the one rule: words joined by `_`, a version after a dot.
            assert_eq!(slug.replace(['.', '_'], ""), name.replace(' ', "").to_lowercase(), "{name}");
        }
    }

    #[test]
    fn grammar() {
        for ok in ["a", "my_theme", "wormhole.light", "a.b.c", "x1_2.y3", "_"] {
            assert!(is_slug(ok), "{ok}");
        }
        for bad in ["", "A", "my theme", "a..b", ".a", "a.", "a-b", "é", "a/b", &"a".repeat(MAX_SLUG_LEN + 1)]
        {
            assert!(!is_slug(bad), "{bad}");
        }
    }

    #[test]
    fn built_ins_are_found_by_slug_and_family() {
        assert_eq!(builtin_name("catppuccin_latte"), Some("Catppuccin Latte"));
        assert_eq!(builtin_name("wormhole.light"), Some("Wormhole Light"));
        assert_eq!(builtin_name("wormhole"), Some("Wormhole Dark"));
        assert_eq!(builtin_name("soltari"), Some("Soltari Light"));
        assert_eq!(builtin_name("my_theme"), None);
        assert_eq!(builtin_slug("Nord"), Some("nord"));
        assert!(is_reserved("nord") && is_reserved("wormhole") && !is_reserved("wormhole_2"));
    }

    #[test]
    fn ids_come_from_names() {
        assert_eq!(derive_id("My Theme"), "my_theme");
        assert_eq!(derive_id("My-Theme"), "my_theme");
        assert_eq!(derive_id("  Café  Noir!! "), "caf_noir");
        assert_eq!(derive_id("日本語"), "theme");
        assert_eq!(derive_id(&"x".repeat(200)).len(), 64);
        assert!(is_slug(&derive_id("Wormhole.Light")));
    }

    fn ids(themes: &[IdRequest<'_>]) -> Vec<String> {
        assign_ids(themes).into_iter().map(|a| a.id).collect()
    }

    #[test]
    fn clashes_are_settled_the_same_way_every_time() {
        let req = |stored, name| IdRequest { stored, name };
        // "My Theme" and "My-Theme" want the same id: the first in the list keeps it.
        let a = [req(None, "My Theme"), req(None, "My-Theme")];
        let got = assign_ids(&a);
        assert_eq!((got[0].id.as_str(), got[1].id.as_str()), ("my_theme", "my_theme_2"));
        assert_eq!(got[1].clash, Some(Clash { wanted: "my_theme".into(), reason: ClashReason::TakenBy(0) }));
        assert_eq!(got[0].clash, None);
        // A file that stores its id is served before one that has none, wherever they sit.
        let b = [req(None, "My Theme"), req(Some("my_theme"), "Other")];
        assert_eq!(ids(&b), ["my_theme_2", "my_theme"]);
        // A Finder copy has the same stored id as the original: the second loses.
        let c = [req(Some("dusk"), "Dusk"), req(Some("dusk"), "Dusk copy")];
        assert_eq!(ids(&c), ["dusk", "dusk_2"]);
        // A user theme called "Nord" (or whose id is a built-in's) gets another id.
        let d = [req(None, "Nord"), req(Some("wormhole.light"), "x"), req(Some("wormhole"), "y")];
        let got = assign_ids(&d);
        assert_eq!(
            got.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            ["nord_2", "wormhole.light_2", "wormhole_2"]
        );
        assert!(got.iter().all(|a| a.clash.as_ref().is_some_and(|c| c.reason == ClashReason::Reserved)));
        // Suffixes do not collide with a theme that really has that id.
        let e = [req(None, "A"), req(None, "a"), req(Some("a_2"), "z")];
        assert_eq!(ids(&e), ["a", "a_3", "a_2"]);
        // An id that is not a slug is not trusted: the name is used.
        assert_eq!(ids(&[req(Some("Bad Id"), "Good Name")]), ["good_name"]);
    }
}

//! Who uses a token: the components that draw with it (the Theme Studio's "Used by", and
//! the outlines it draws round them in its gallery), the tokens whose rules follow it,
//! and the names an expression refers to.

use std::collections::BTreeSet;

use super::catalog::UI_TOKENS;
use super::expr::parse_expr;
use super::model::Appearance;
use super::rules::default_rules;

/// The part of the app a component belongs to: what the Theme Studio's area filter narrows
/// its gallery and its token list to. A component is in the part whose gallery specimen draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Area {
    Sidebar,
    Panel,
    Chrome,
    Settings,
    Ambient,
    /// The terminal and code, which every part shows: listed under All only.
    Basics,
}

/// A component of the design and the tokens it draws with. `tokens` are UI tokens
/// and `metrics.*`; `terminal.*` and `syntax.*` stand for every terminal or syntax color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenUse {
    /// What the Studio's gallery marks the component's specimen with.
    pub id: &'static str,
    /// The component as "Used by" names it.
    pub label: &'static str,
    pub area: Area,
    pub tokens: &'static [&'static str],
}

const fn u(id: &'static str, label: &'static str, area: Area, tokens: &'static [&'static str]) -> TokenUse {
    TokenUse { id, label, area, tokens }
}

use Area::{Ambient, Basics, Chrome, Panel, Settings, Sidebar};

/// The components and what they draw with, from the style components (through
/// `UiColors`), GPUI Kit's colors (`theme_bridge`), the terminal and the diff view.
pub const TOKEN_USES: &[TokenUse] = &[
    u(
        "sidebar.row",
        "sidebar row",
        Sidebar,
        &[
            "sidebar.background",
            "sidebar.row.hover",
            "sidebar.row.selected",
            "sidebar.row.selected.text",
            "sidebar.row.selected.muted",
            "state.hover",
            "text.primary",
            "text.muted",
            "text.faint",
            "agent.attention.text",
            "agent.done.text",
        ],
    ),
    u(
        "sidebar.card",
        "sidebar card",
        Sidebar,
        &[
            "sidebar.background",
            "card",
            "card.edge",
            "text.primary",
            "text.muted",
            "metrics.border_width",
            "metrics.radius_lg",
        ],
    ),
    u(
        "sidebar.band",
        "group head",
        Sidebar,
        &[
            "sidebar.background",
            "band",
            "band.hover",
            "band.solid",
            "text.primary",
            "text.muted",
            "border",
            "state.highlight",
            "primary",
            "metrics.border_width",
        ],
    ),
    u(
        "folder.color",
        "folder colour",
        Sidebar,
        &[
            // (The eight names also take their lightness and chroma from the terminal palette's
            // six colours; terminal colours are listed under Basics only.)
            "surface.window",
            "surface.panel",
            "band.solid",
            "band.hover",
            "border",
            "text.primary",
            "text.muted",
            "metrics.radius_sm",
        ],
    ),
    u(
        "status.glyph",
        "status glyph",
        Sidebar,
        &[
            "agent.working",
            "agent.done",
            "agent.attention",
            "agent.exited",
            "agent.attention.ring",
            "status.danger",
            "text.faint",
        ],
    ),
    u(
        "status.tile",
        "status tile",
        Sidebar,
        &[
            "tile.background",
            "tile.mark",
            "tile.mark.brand",
            "tile.sheen",
            "tile.edge",
            "agent.claude",
            "agent.gemini",
            "agent.gemini.end",
            "agent.working",
            "agent.attention",
            "agent.done",
            "status.danger",
            "primary",
            "text.muted",
            "sidebar.background",
            "sidebar.row.selected",
        ],
    ),
    u(
        "status.bar",
        "status bar",
        Sidebar,
        &[
            "sidebar.background",
            "border",
            "band.hover",
            "text.muted",
            "agent.attention.text",
            "metrics.radius",
            "metrics.border_width",
        ],
    ),
    u(
        "branch.chip",
        "branch chip",
        Sidebar,
        &[
            "git.branch",
            "git.branch.background",
            "banner.warning.background",
            "status.warning.text",
            "text.muted",
            "metrics.radius_chip",
        ],
    ),
    u(
        "panel.band",
        "section band",
        Panel,
        &[
            "surface.panel",
            "band",
            "band.hover",
            "band.solid",
            "text.primary",
            "text.muted",
            "border",
            "metrics.border_width",
        ],
    ),
    u(
        "panel.item",
        "panel row",
        Panel,
        &[
            "surface.panel",
            "state.hover",
            "state.selected",
            "sidebar.row.selected.text",
            "sidebar.row.selected.muted",
            "text.primary",
            "text.muted",
            "text.faint",
            "checkbox.background",
            "checkbox.checked",
            "checkbox.border",
        ],
    ),
    u(
        "file.row",
        "file row",
        Panel,
        &[
            "state.hover",
            "state.selected",
            "sidebar.row.selected.text",
            "sidebar.row.selected.muted",
            "text.muted",
            "status.success.text",
            "status.warning.text",
            "status.danger.text",
            "diff.added",
            "diff.removed",
        ],
    ),
    u(
        "commit.row",
        "commit row",
        Panel,
        &[
            "state.hover",
            "state.selected",
            "sidebar.row.selected.text",
            "sidebar.row.selected.muted",
            "text.primary",
            "text.muted",
            "primary",
            "git.branch",
        ],
    ),
    u(
        "diff",
        "diff view",
        Panel,
        &[
            "surface.window",
            "text.primary",
            "state.hover",
            "diff.added",
            "diff.added.background",
            "diff.removed",
            "diff.removed.background",
            "diff.hunk.background",
            "diff.hunk.text",
            "diff.line_number",
            "diff.filler",
            "scrollbar.thumb",
            "syntax.*",
        ],
    ),
    u(
        "callout",
        "callout",
        Panel,
        &[
            "banner.info.background",
            "banner.info.border",
            "banner.warning.background",
            "banner.warning.border",
            "banner.danger.background",
            "banner.danger.border",
            "banner.success.background",
            "banner.success.border",
            "status.info",
            "status.warning",
            "status.danger",
            "status.success",
            "text.primary",
            "trust.command",
        ],
    ),
    u(
        "icon.button",
        "icon button",
        Chrome,
        &[
            "band.hover",
            "state.selected",
            "state.selected.inset",
            "sidebar.row.selected.text",
            "text.on_selected",
            "text.on_selected.muted",
            "text.primary",
            "text.muted",
            "metrics.radius",
            "metrics.radius_sm",
        ],
    ),
    u(
        "icon.tab",
        "icon tab",
        Panel,
        &[
            "band.hover",
            "state.active",
            "state.selected",
            "state.selected.inset",
            "sidebar.row.selected.text",
            "text.primary",
            "text.muted",
            "metrics.radius",
            "metrics.radius_chip",
        ],
    ),
    u(
        "title.tab",
        "title tab",
        Chrome,
        &[
            "titlebar.background",
            "tab.active.background",
            "tab.inactive.text",
            "state.hover",
            "text.primary",
            "text.muted",
            "agent.attention.ring",
            "agent.attention.tint",
            "metrics.radius",
        ],
    ),
    u(
        "keycap",
        "key cap",
        Chrome,
        &["surface.elevated", "border.strong", "text.primary", "metrics.radius_chip", "metrics.border_width"],
    ),
    u(
        "badge",
        "count badge",
        Chrome,
        &[
            "border",
            "text.primary",
            "state.selected.inset",
            "sidebar.row.selected.text",
            "metrics.radius_chip",
            "metrics.border_width",
        ],
    ),
    u("tint.chip", "tint chip", Settings, &["chip.tint", "sidebar.row.selected.text", "metrics.radius_chip"]),
    u("neutral.chip", "neutral chip", Chrome, &["band.hover", "text.primary", "metrics.radius_chip"]),
    u(
        "title.extras",
        "title row extras",
        Chrome,
        &[
            "band.hover",
            "text.primary",
            "text.muted",
            "status.warning",
            "status.success.text",
            "git.branch",
            "primary",
            "border",
            "metrics.radius_chip",
        ],
    ),
    u("tooltip", "tooltip", Chrome, &["tooltip.background", "tooltip.text", "shadow", "metrics.radius"]),
    u("focus.ring", "focus ring", Chrome, &["border.focus", "metrics.focus_ring_width"]),
    u(
        "settings.nav",
        "settings nav",
        Settings,
        &[
            "sidebar.background",
            "sidebar.row.selected",
            "sidebar.row.selected.text",
            "state.hover",
            "text.primary",
            "text.muted",
            "text.faint",
            "agent.attention",
        ],
    ),
    u(
        "setting.row",
        "setting row",
        Settings,
        &[
            "surface.window",
            "state.hover",
            "state.highlight",
            "text.primary",
            "text.muted",
            "text.faint",
            "text.link",
            "primary",
            "metrics.radius",
        ],
    ),
    u(
        "switch",
        "switch",
        Settings,
        &["checkbox.background", "checkbox.checked", "border.strong", "switch.knob", "primary"],
    ),
    u(
        "segmented",
        "segmented control",
        Chrome,
        &[
            "chip.selected",
            "chip.border",
            "text.on_selected",
            "text.on_selected.muted",
            "state.hover",
            "text.primary",
            "text.muted",
            "metrics.radius",
            "metrics.radius_chip",
        ],
    ),
    u(
        "stepper",
        "stepper",
        Settings,
        &["input.background", "border.strong", "text.primary", "metrics.radius", "metrics.border_width"],
    ),
    u(
        "recorder",
        "shortcut recorder",
        Settings,
        &[
            "input.background",
            "border.strong",
            "border.focus",
            "status.danger",
            "status.warning",
            "text.muted",
            "text.faint",
            "metrics.radius",
            "metrics.focus_ring_width",
        ],
    ),
    u(
        "button",
        "button",
        Settings,
        &[
            "button.background",
            "button.hover",
            "button.text",
            "state.active",
            "border.strong",
            "text.disabled",
            "metrics.radius",
            "metrics.border_width",
        ],
    ),
    u(
        "button.primary",
        "primary button",
        Settings,
        &["primary", "primary.hover", "text.on_primary", "text.disabled", "metrics.radius"],
    ),
    u(
        "input",
        "text field",
        Settings,
        &[
            "input.background",
            "input.border",
            "input.border.focus",
            "input.placeholder",
            "selection.ui",
            "text.primary",
            "metrics.radius",
            "metrics.border_width",
        ],
    ),
    u(
        "checkbox",
        "checkbox",
        Settings,
        &[
            "checkbox.background",
            "checkbox.checked",
            "checkbox.border",
            "text.on_primary",
            "metrics.radius_sm",
        ],
    ),
    u(
        "menu",
        "menu",
        Settings,
        &[
            "surface.elevated",
            "border",
            "shadow",
            "state.hover",
            "list.selected",
            "text.primary",
            "text.muted",
            "status.danger",
            "metrics.radius_lg",
        ],
    ),
    u(
        "banner",
        "banner",
        Settings,
        &[
            "banner.info.background",
            "banner.info.border",
            "banner.warning.background",
            "banner.warning.border",
            "banner.danger.background",
            "banner.danger.border",
            "banner.success.background",
            "banner.success.border",
            "status.info",
            "status.warning",
            "status.danger",
            "status.success",
            "text.primary",
            "metrics.radius",
            "metrics.border_width",
        ],
    ),
    u(
        "palette.row",
        "palette result",
        Ambient,
        &[
            "surface.elevated",
            "list.selected",
            "list.selected.text",
            "list.selected.muted",
            "state.hover",
            "text.primary",
            "text.muted",
            "text.faint",
        ],
    ),
    u(
        "find",
        "find in scrollback",
        Chrome,
        &[
            "find.match",
            "find.match.current",
            "find.match.current.text",
            "surface.elevated",
            "border.strong",
            "shadow",
            "input.background",
            "border",
            "border.focus",
            "text.primary",
            "text.muted",
            "state.selected",
            "band.hover",
            "status.danger.text",
            "metrics.radius",
            "metrics.radius_lg",
        ],
    ),
    u("readout", "resize readout", Ambient, &["tooltip.background", "tooltip.text", "metrics.radius"]),
    u("resize.handle", "resize handle", Ambient, &["resize.handle.hover", "resize.handle.active"]),
    u(
        "dialog",
        "dialog",
        Settings,
        &["surface.overlay", "surface.elevated", "shadow", "text.primary", "text.muted", "metrics.radius_lg"],
    ),
    u("card", "card", Chrome, &["surface.panel", "border", "metrics.radius_lg", "metrics.border_width"]),
    u("scrollbar", "scrollbar", Settings, &["scrollbar.thumb", "text.faint"]),
    u("terminal", "terminal", Basics, &["surface.window", "state.highlight", "terminal.*"]),
    u("code", "code", Basics, &["surface.window", "syntax.*"]),
];

/// Whether a use's entry `pattern` names `token` (`terminal.*` names every terminal color).
fn names(pattern: &str, token: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => token.starts_with(prefix),
        None => pattern == token,
    }
}

/// The components that draw with `token`, in [`TOKEN_USES`] order.
pub fn uses_of(token: &str) -> Vec<&'static TokenUse> {
    TOKEN_USES.iter().filter(|u| u.tokens.iter().any(|p| names(p, token))).collect()
}

/// Whether a component of `area` draws with `token`: what the Studio's token list keeps while
/// its area filter is on `area`.
pub fn used_in(token: &str, area: Area) -> bool {
    TOKEN_USES.iter().any(|u| u.area == area && u.tokens.iter().any(|p| names(p, token)))
}

/// The UI tokens whose default rule is computed from `token`, directly or through other
/// tokens, in [`UI_TOKENS`] order: what moves when it changes.
pub fn followers(token: &str, appearance: Appearance) -> Vec<&'static str> {
    let rules = default_rules(appearance);
    let follows = |name: &str| {
        let mut seen = BTreeSet::new();
        let mut todo = vec![name];
        while let Some(node) = todo.pop() {
            if !seen.insert(node) {
                continue;
            }
            let Some(rule) = rules.get(node) else { continue };
            let mut refs = Vec::new();
            rule.refs(&mut refs);
            if refs.contains(&token) {
                return true;
            }
            todo.extend(refs);
        }
        false
    };
    UI_TOKENS.iter().map(|t| t.name).filter(|name| *name != token && follows(name)).collect()
}

/// The names `src` (a rule or a typed value) refers to directly, in order and once
/// each; none when it doesn't parse.
pub fn refers_to(src: &str) -> Vec<String> {
    let Ok(expr) = parse_expr(src) else { return Vec::new() };
    let mut refs = Vec::new();
    expr.refs(&mut refs);
    let mut out: Vec<String> = Vec::new();
    for name in refs {
        if !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{Area, TOKEN_USES, followers, refers_to, used_in, uses_of};
    use crate::tokens::{Appearance, METRIC_TOKENS, UI_TOKENS};

    fn is_token(name: &str) -> bool {
        UI_TOKENS.iter().chain(METRIC_TOKENS).any(|t| t.name == name)
    }

    #[test]
    fn every_use_names_real_tokens_once() {
        let mut ids = BTreeSet::new();
        for u in TOKEN_USES {
            assert!(ids.insert(u.id), "{} is listed twice", u.id);
            assert!(!u.label.is_empty() && !u.tokens.is_empty(), "{}", u.id);
            let mut seen = BTreeSet::new();
            for name in u.tokens {
                assert!(seen.insert(name), "{}: {name} twice", u.id);
                assert!(
                    is_token(name) || ["terminal.*", "syntax.*"].contains(name),
                    "{}: {name} is not a token",
                    u.id
                );
            }
        }
    }

    /// "Used by" is never empty: every token the Studio lists draws something.
    #[test]
    fn every_token_has_a_use() {
        let unused: Vec<&str> =
            UI_TOKENS.iter().chain(METRIC_TOKENS).map(|t| t.name).filter(|n| uses_of(n).is_empty()).collect();
        assert!(unused.is_empty(), "no component uses {unused:?}");
        assert!(!uses_of("terminal.ansi.4").is_empty());
        assert!(!uses_of("syntax.keyword").is_empty());
        assert!(uses_of("not.a.token").is_empty());
    }

    #[test]
    fn the_selected_row_tint_is_used_by_the_sidebar_and_the_settings_nav() {
        let ids: Vec<&str> = uses_of("sidebar.row.selected").iter().map(|u| u.id).collect();
        assert!(ids.contains(&"sidebar.row") && ids.contains(&"settings.nav"), "{ids:?}");
        let chip: Vec<&str> = uses_of("metrics.radius_chip").iter().map(|u| u.id).collect();
        assert!(chip.contains(&"keycap") && chip.contains(&"badge"), "{chip:?}");
    }

    /// Every use is in a part of the app (its `area` can't be left out), every part the filter
    /// offers has components, and only the terminal and code are left to All.
    #[test]
    fn every_use_has_an_area() {
        for area in [Area::Sidebar, Area::Panel, Area::Chrome, Area::Settings, Area::Ambient] {
            assert!(TOKEN_USES.iter().any(|u| u.area == area), "nothing in {area:?}");
        }
        let basics: Vec<&str> = TOKEN_USES.iter().filter(|u| u.area == Area::Basics).map(|u| u.id).collect();
        assert_eq!(basics, ["terminal", "code"]);
    }

    #[test]
    fn a_token_is_in_the_areas_whose_components_draw_with_it() {
        assert!(used_in("sidebar.row.selected", Area::Sidebar));
        assert!(used_in("sidebar.row.selected", Area::Settings), "the settings nav");
        assert!(!used_in("sidebar.row.selected", Area::Ambient));
        assert!(used_in("metrics.radius_chip", Area::Sidebar), "the branch chip");
        assert!(!used_in("primary", Area::Ambient));
        assert!(used_in("terminal.ansi.4", Area::Basics) && !used_in("terminal.ansi.4", Area::Sidebar));
        assert!(!used_in("not.a.token", Area::Sidebar));
        // Every interface token is in one of the parts the filter offers: none is under All only.
        let parts = [Area::Sidebar, Area::Panel, Area::Chrome, Area::Settings, Area::Ambient];
        let nowhere: Vec<&str> = UI_TOKENS
            .iter()
            .chain(METRIC_TOKENS)
            .map(|t| t.name)
            .filter(|name| !parts.iter().any(|area| used_in(name, *area)))
            .collect();
        assert!(nowhere.is_empty(), "{nowhere:?}");
    }

    #[test]
    fn followers_are_what_moves_with_a_token() {
        for appearance in [Appearance::Dark, Appearance::Light] {
            let f = followers("primary", appearance);
            for follower in ["border.focus", "agent.working", "sidebar.row.selected", "text.link"] {
                assert!(f.contains(&follower), "{appearance:?}: {follower} in {f:?}");
            }
            assert!(!f.contains(&"primary"));
            assert!(!f.contains(&"surface.window"), "{f:?}");
            // Through another token: the selected row's text follows the row's tint.
            assert!(f.contains(&"sidebar.row.selected.text"));
        }
        assert!(followers("selection.ui", Appearance::Dark).is_empty());
    }

    #[test]
    fn names_an_expression_refers_to() {
        assert_eq!(refers_to("mix(sidebar.background, primary, 14%)"), ["sidebar.background", "primary"]);
        assert_eq!(
            refers_to("readable(primary, surface.window, primary / 50%, 3)"),
            ["primary", "surface.window"]
        );
        assert!(refers_to("#7aa2f7").is_empty());
        assert!(refers_to("mix(a,").is_empty());
    }
}

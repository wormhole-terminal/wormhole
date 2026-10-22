//! Contrast checks: the text/background pairs every theme should keep
//! readable, and which of them a theme falls short on.

use super::theme_tokens::ThemeTokens;
use crate::theme::tokens::color::Color;

/// A text-on-background pair checked by [`ThemeTokens::contrast_warnings`].
/// Translucent backgrounds are composited over `underlay` (itself composited
/// over `surface.window` if translucent).
#[derive(Clone, Copy, Debug)]
pub struct ContrastPair {
    pub fg: &'static str,
    pub bg: &'static str,
    pub underlay: &'static str,
    pub min: f32,
}

const fn pair(fg: &'static str, bg: &'static str, min: f32) -> ContrastPair {
    ContrastPair { fg, bg, underlay: "surface.window", min }
}

const fn on(fg: &'static str, bg: &'static str, underlay: &'static str, min: f32) -> ContrastPair {
    ContrastPair { fg, bg, underlay, min }
}

/// Body text needs 4.5:1 (WCAG AA); secondary/large text, icons, status dots and
/// the outlines of controls need 3:1.
pub const CONTRAST_PAIRS: &[ContrastPair] = &[
    pair("terminal.foreground", "terminal.background", 4.5),
    pair("text.primary", "surface.window", 4.5),
    pair("text.primary", "surface.panel", 4.5),
    pair("text.primary", "surface.elevated", 4.5),
    pair("text.primary", "sidebar.background", 4.5),
    pair("text.primary", "titlebar.background", 4.5),
    pair("text.muted", "surface.window", 4.5),
    pair("text.muted", "surface.panel", 4.5),
    pair("text.muted", "surface.elevated", 4.5),
    pair("text.muted", "sidebar.background", 4.5),
    on("text.muted", "band", "sidebar.background", 4.5),
    pair("text.faint", "surface.window", 3.0),
    on("text.faint", "band", "sidebar.background", 3.0),
    pair("text.link", "surface.window", 4.5),
    pair("text.on_primary", "primary", 4.5),
    // Solid selected fills: a chosen segment, a primary button.
    pair("text.on_selected", "chip.selected", 4.5),
    pair("text.on_selected.muted", "chip.selected", 4.5),
    // Tinted selected fills carry their own text: the name and its second line.
    on("sidebar.row.selected.text", "sidebar.row.selected", "sidebar.background", 4.5),
    on("sidebar.row.selected.muted", "sidebar.row.selected", "sidebar.background", 4.5),
    on("list.selected.text", "list.selected", "surface.elevated", 4.5),
    on("list.selected.muted", "list.selected", "surface.elevated", 4.5),
    pair("button.text", "button.background", 4.5),
    pair("button.text", "button.hover", 4.5),
    pair("input.placeholder", "input.background", 3.0),
    pair("checkbox.border", "surface.panel", 3.0),
    pair("tab.inactive.text", "titlebar.background", 3.0),
    pair("tooltip.text", "tooltip.background", 4.5),
    pair("diff.added", "diff.added.background", 4.5),
    pair("diff.removed", "diff.removed.background", 4.5),
    pair("diff.hunk.text", "diff.hunk.background", 4.5),
    pair("find.match.current.text", "find.match.current", 4.5),
    pair("diff.line_number", "diff.added.background", 3.0),
    pair("diff.line_number", "diff.removed.background", 3.0),
    pair("git.branch", "sidebar.background", 4.5),
    pair("git.branch", "surface.panel", 4.5),
    on("git.branch", "git.branch.background", "band.solid", 4.5),
    pair("primary", "surface.window", 3.0),
    // The focus ring on every surface a focused control or row can sit on.
    pair("border.focus", "surface.window", 3.0),
    pair("border.focus", "surface.panel", 3.0),
    pair("border.focus", "surface.elevated", 3.0),
    pair("border.focus", "sidebar.background", 3.0),
    on("border.focus", "sidebar.row.selected", "sidebar.background", 3.0),
    pair("status.success", "surface.window", 3.0),
    pair("status.warning", "surface.window", 3.0),
    pair("status.danger", "surface.window", 3.0),
    pair("status.info", "surface.window", 3.0),
    // Status colors as text: right-panel letters and counts, on a row and a selected one.
    pair("status.success.text", "surface.panel", 4.5),
    on("status.success.text", "state.selected", "surface.panel", 4.5),
    pair("status.warning.text", "surface.panel", 4.5),
    on("status.warning.text", "state.selected", "surface.panel", 4.5),
    pair("status.danger.text", "surface.panel", 4.5),
    on("status.danger.text", "state.selected", "surface.panel", 4.5),
    // A project's commands: on the trust callout's tint in the panel, and in their own window.
    on("trust.command", "banner.warning.background", "surface.panel", 4.5),
    pair("trust.command", "surface.window", 4.5),
    // The sidebar's and the panel's cards: their fill is faint, but text and marks on them keep
    // what they have on the column (the amber edge of a card that needs you included).
    on("text.primary", "card", "sidebar.background", 4.5),
    on("text.muted", "card", "sidebar.background", 4.5),
    on("text.faint", "card", "sidebar.background", 3.0),
    on("agent.attention", "card", "sidebar.background", 3.0),
    on("agent.attention.text", "card", "sidebar.background", 4.5),
    on("agent.done.text", "card", "sidebar.background", 4.5),
    on("agent.working", "card", "sidebar.background", 3.0),
    pair("agent.working", "sidebar.background", 3.0),
    pair("agent.done", "sidebar.background", 3.0),
    pair("agent.attention", "sidebar.background", 3.0),
    pair("agent.exited", "sidebar.background", 3.0),
    // Agent states as text: sidebar second lines, on a row and the selected one.
    pair("agent.attention.text", "sidebar.background", 4.5),
    on("agent.attention.text", "sidebar.row.selected", "sidebar.background", 4.5),
    pair("agent.done.text", "sidebar.background", 4.5),
    on("agent.done.text", "sidebar.row.selected", "sidebar.background", 4.5),
];

impl ThemeTokens {
    /// `name` composited down to an opaque color over `underlay` (and that
    /// over `surface.window`, and that over black) so contrast is measurable.
    pub(super) fn opaque(&self, name: &str, underlay: &str) -> Option<Color> {
        let window = self.get("surface.window")?.over(Color::BLACK);
        let under = self.get(underlay)?.over(window);
        Some(self.get(name)?.over(under))
    }

    /// Pairs from [`CONTRAST_PAIRS`] below their minimum: (fg, bg, ratio).
    pub fn contrast_warnings(&self) -> Vec<(String, String, f32)> {
        CONTRAST_PAIRS
            .iter()
            .filter_map(|p| {
                let bg = self.opaque(p.bg, p.underlay)?;
                let fg = self.get(p.fg)?.over(bg);
                let ratio = Color::contrast_ratio(fg, bg);
                (ratio < p.min).then(|| (p.fg.to_string(), p.bg.to_string(), ratio))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::theme::tokens::test_support::resolve_str;

    #[test]
    fn contrast_warnings_report_bad_pairs() {
        let t = resolve_str(r##"{"ui": {"text.muted": "#2a2b36", "primary": "#1a1b27"}}"##).unwrap();
        let w = t.contrast_warnings();
        assert!(w.iter().any(|(f, b, r)| f == "text.muted" && b == "surface.window" && *r < 1.5), "{w:?}");
        assert!(w.iter().any(|(f, b, _)| f == "primary" && b == "surface.window"), "{w:?}");
        assert!(!w.iter().any(|(f, _, _)| f == "text.primary"));
    }
}

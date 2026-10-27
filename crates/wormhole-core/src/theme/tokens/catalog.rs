//! The token catalogue: every UI token and metric the Theme Studio lists, in
//! inspector order, with what it is for.

use super::rules::SYNTAX_RULES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenInfo {
    pub name: &'static str,
    pub group: &'static str,
    pub description: &'static str,
}

const fn t(name: &'static str, group: &'static str, description: &'static str) -> TokenInfo {
    TokenInfo { name, group, description }
}

/// Canonical UI tokens, in inspector order. Theme files may only override these.
pub const UI_TOKENS: &[TokenInfo] = &[
    t("surface.window", "Surfaces", "Main window background, behind the terminal"),
    t("surface.panel", "Surfaces", "Side panels and secondary areas"),
    t("surface.elevated", "Surfaces", "Popovers, menus and cards that float above content"),
    t("surface.overlay", "Surfaces", "Scrim behind modal dialogs"),
    t("titlebar.background", "Chrome", "Title bar / tab strip background"),
    t("sidebar.background", "Chrome", "Terminal sidebar background"),
    t("sidebar.row.hover", "Chrome", "Sidebar row under the pointer (overlay)"),
    t(
        "sidebar.row.selected",
        "Chrome",
        "The current terminal's row in the sidebar, and the settings nav (opaque tint)",
    ),
    t("sidebar.row.selected.text", "Chrome", "Name and icon on a selected row; also right-panel rows"),
    t(
        "sidebar.row.selected.muted",
        "Chrome",
        "Second line and metadata on a selected row; still meets 4.5:1",
    ),
    t("card", "Chrome", "A card's fill in the sidebar: a group's, or a terminal's own (overlay)"),
    t("card.edge", "Chrome", "A card's edge, which carries it where its fill is faint (overlay)"),
    t("band", "Chrome", "Folder bands in the sidebar and section bands in the right panel (overlay)"),
    t("band.hover", "Chrome", "A band under the pointer; also the hover of an icon button (overlay)"),
    t("band.solid", "Chrome", "A band over the sidebar, opaque: the fill of a band that sticks to the top"),
    t("text.primary", "Text", "Body text and labels"),
    t("text.muted", "Text", "Secondary text; still meets 4.5:1, on a band too"),
    t("text.faint", "Text", "Hints, placeholders, decorative glyphs (3:1, on a band or a hovered row too)"),
    t("text.disabled", "Text", "Disabled controls (no contrast requirement)"),
    t("text.on_primary", "Text", "Text drawn on a primary fill"),
    t(
        "text.on_selected",
        "Text",
        "Text and icons on a solid selected fill: a chosen segment, a primary button",
    ),
    t("text.on_selected.muted", "Text", "Secondary text on a solid selected fill; still meets 4.5:1"),
    t("text.link", "Text", "Links and inline actions"),
    t("trust.command", "Text", "What a project would run, in the trust notice and its window"),
    t("border", "Borders", "Hairline separators (translucent)"),
    t("border.strong", "Borders", "Control outlines (translucent)"),
    t(
        "divider",
        "Borders",
        "The lines between the window's columns: the sidebar, the terminal and the panel",
    ),
    t("border.focus", "Borders", "Keyboard focus ring (3:1 on every surface)"),
    t("state.hover", "States", "Generic hover overlay"),
    t("state.active", "States", "Pressed overlay"),
    t(
        "state.selected",
        "States",
        "Selected item tint: right-panel rows, icon tabs, pressed toggles (overlay)",
    ),
    t("state.selected.inset", "States", "Badges and buttons inside a selected tint"),
    t("state.highlight", "States", "Tint for drop targets (overlay)"),
    t("list.selected", "Lists", "Selected result in the palette, quick panel and pickers (opaque tint)"),
    t("list.selected.text", "Lists", "Title of the selected result"),
    t("list.selected.muted", "Lists", "Detail text of the selected result; still meets 4.5:1"),
    t("primary", "Primary", "Brand / primary action color"),
    t("primary.hover", "Primary", "Primary under the pointer"),
    t("status.success", "Status", "Success icons"),
    t(
        "status.warning",
        "Status",
        "Warning icons (amber when the palette's yellow is darker than muted text)",
    ),
    t("status.danger", "Status", "Errors and destructive actions"),
    t("status.info", "Status", "Informational icons and text"),
    t("status.success.text", "Status", "Success as text in the right panel: A / U letters, +N"),
    t("status.warning.text", "Status", "Warning as text in the right panel: M letters, banner text"),
    t("status.danger.text", "Status", "Danger as text in the right panel: D letters, −N, git errors"),
    t("banner.info.background", "Banners", "Info banner fill (overlay)"),
    t("banner.info.border", "Banners", "Info banner outline"),
    t("banner.warning.background", "Banners", "Warning banner fill, e.g. the project trust notice (overlay)"),
    t("banner.warning.border", "Banners", "Warning banner outline"),
    t("banner.danger.background", "Banners", "Danger banner fill (overlay)"),
    t("banner.danger.border", "Banners", "Danger banner outline"),
    t("banner.success.background", "Banners", "Success banner fill (overlay)"),
    t("banner.success.border", "Banners", "Success banner outline"),
    t("agent.working", "Agents", "Agent is busy"),
    t("agent.done", "Agents", "Agent finished"),
    t("agent.attention", "Agents", "Agent needs input"),
    t("agent.exited", "Agents", "Agent process exited"),
    t("agent.bell", "Agents", "The bell's orange: the badge of a terminal that rang"),
    t("agent.attention.ring", "Agents", "The halo round a needs-you dot (overlay)"),
    t("agent.attention.tint", "Agents", "Fill of a tab that waits for input (overlay)"),
    t("agent.attention.text", "Agents", "“Needs input” as text: second lines, the status bar"),
    t("agent.done.text", "Agents", "“Finished” as text: second lines"),
    t(
        "tile.background",
        "Agents",
        "The status tile of a terminal, and of every agent without a color of its own (the agent \
         registry holds the brands' colors)",
    ),
    t("tile.mark", "Agents", "The terminal symbol or logo on a black tile"),
    t("tile.mark.brand", "Agents", "The logo or the letters on an agent's own color"),
    t("tile.sheen", "Agents", "The faint light across a status tile's top (overlay)"),
    t("tile.edge", "Agents", "A status tile's hairline edge (overlay)"),
    t("diff.added", "Diff", "Added-line text and gutter marks, +N in rows"),
    t("diff.added.background", "Diff", "Added-line background"),
    t("diff.removed", "Diff", "Removed-line text and gutter marks, −N in rows"),
    t("diff.removed.background", "Diff", "Removed-line background"),
    t("diff.hunk.background", "Diff", "Hunk header background"),
    t("diff.hunk.text", "Diff", "Hunk header text (@@ -41,7 +41,10 @@)"),
    t("diff.line_number", "Diff", "Line numbers beside the code"),
    t("diff.filler", "Diff", "Hatch stripes where one side has no line (translucent)"),
    t("git.branch", "Git", "Branch names and the ⎇ before them"),
    t("git.branch.background", "Git", "Fill of a branch chip (overlay)"),
    t("find.match", "Find", "Every match of a find in the terminal (overlay)"),
    t("find.match.current", "Find", "The current match, and the matches' ticks along the terminal's edge"),
    t("find.match.current.text", "Find", "Text on the current match"),
    t("button.background", "Buttons", "Secondary button fill"),
    t("button.hover", "Buttons", "Secondary button fill under the pointer"),
    t("button.text", "Buttons", "Secondary button label"),
    t("input.background", "Inputs", "Text field fill"),
    t("input.border", "Inputs", "Text field outline"),
    t("input.border.focus", "Inputs", "Focused text field outline"),
    t("input.placeholder", "Inputs", "Placeholder text"),
    t("checkbox.background", "Controls", "Unchecked checkbox / toggle fill"),
    t("checkbox.checked", "Controls", "Checked checkbox / toggle fill"),
    t("checkbox.border", "Controls", "Outline of an unchecked checkbox (3:1)"),
    t("switch.knob", "Controls", "Toggle knob and slider thumb"),
    t("tab.active.background", "Tabs", "Active tab fill"),
    t("tab.inactive.text", "Tabs", "Inactive tab label"),
    t("chip.border", "Tabs", "Chip / pill outline"),
    t("chip.selected", "Tabs", "Active tab or the chosen segment of a segmented control (solid)"),
    t("chip.tint", "Tabs", "Tinted chip: the sidebar's filter chip, a project override (overlay)"),
    t("scrollbar.thumb", "Misc", "Scrollbar thumb (translucent)"),
    t("resize.handle.hover", "Misc", "The line of a resize handle under the pointer"),
    t("resize.handle.active", "Misc", "The line of a resize handle while dragging"),
    t("tooltip.background", "Misc", "Tooltip fill"),
    t("tooltip.text", "Misc", "Tooltip text"),
    t("shadow", "Misc", "Drop shadow color"),
    t("selection.ui", "Misc", "Text selection in UI text fields"),
];

/// The metrics the Theme Studio edits, as `metrics.<key>` nodes, in inspector
/// order. Theme files set them under `"metrics"` (the fonts are chosen in
/// Settings instead, and `spacing` is not drawn from yet).
pub const METRIC_TOKENS: &[TokenInfo] = &[
    t("metrics.radius_sm", "Shape", "Tiny elements: swatches, a small icon button's hover (px)"),
    t("metrics.radius_chip", "Shape", "Tabs, choice chips, badges and key caps (px)"),
    t("metrics.radius", "Shape", "Controls and rows: buttons, inputs, selects, list rows (px)"),
    t("metrics.radius_lg", "Shape", "Cards, menus, popovers and other floating surfaces (px)"),
    t("metrics.border_width", "Shape", "Outlines and hairline rules (px)"),
    t("metrics.divider_width", "Shape", "The lines between the window's columns (px, 0 draws none)"),
    t("metrics.focus_ring_width", "Shape", "Keyboard focus ring (px)"),
];

/// Syntax keys that have built-in defaults. Theme files may add others; a new
/// key without a color falls back to `syntax.variable`.
pub fn syntax_keys() -> impl Iterator<Item = &'static str> {
    SYNTAX_RULES.iter().map(|r| r.0)
}

#[cfg(test)]
mod tests {
    use super::{METRIC_TOKENS, UI_TOKENS};

    /// The Studio starts a new group heading whenever the group changes, so a group
    /// split in two would show up twice.
    #[test]
    fn each_group_is_one_run() {
        let mut seen: Vec<&str> = Vec::new();
        for info in UI_TOKENS.iter().chain(METRIC_TOKENS) {
            if seen.last() != Some(&info.group) {
                assert!(!seen.contains(&info.group), "{} splits the group {}", info.name, info.group);
                seen.push(info.group);
            }
        }
        for group in ["Chrome", "Lists", "Banners", "Shape"] {
            assert!(seen.contains(&group), "{group}");
        }
        let chrome: Vec<_> = UI_TOKENS.iter().filter(|t| t.group == "Chrome").map(|t| t.name).collect();
        for band in ["band", "band.hover", "band.solid"] {
            assert!(chrome.contains(&band), "{band} is not in Chrome");
        }
    }
}

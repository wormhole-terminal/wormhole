// The text/background pairs every theme should keep readable, as `wormhole-core` lists them
// (`theme/tokens/contrast.rs` `CONTRAST_PAIRS`). A translucent background is judged over its `underlay`.
// Copied as data, with a test that compares it to the Rust file, until the FFI hands the list over.

public struct ContrastPair: Sendable, Equatable {
    public let fg: String
    public let bg: String
    public let underlay: String
    public let min: Double

    public init(_ fg: String, _ bg: String, _ min: Double, underlay: String = "surface.window") {
        (self.fg, self.bg, self.underlay, self.min) = (fg, bg, underlay, min)
    }

    /// Body text needs 4.5:1 (WCAG AA); secondary text, icons, status dots and control outlines need 3:1.
    public static let all: [ContrastPair] = [
        .init("terminal.foreground", "terminal.background", 4.5),
        .init("text.primary", "surface.window", 4.5),
        .init("text.primary", "surface.panel", 4.5),
        .init("text.primary", "surface.elevated", 4.5),
        .init("text.primary", "sidebar.background", 4.5),
        .init("text.primary", "titlebar.background", 4.5),
        .init("text.muted", "surface.window", 4.5),
        .init("text.muted", "surface.panel", 4.5),
        .init("text.muted", "surface.elevated", 4.5),
        .init("text.muted", "sidebar.background", 4.5),
        .init("text.muted", "band", 4.5, underlay: "sidebar.background"),
        .init("text.faint", "surface.window", 3.0),
        .init("text.faint", "band", 3.0, underlay: "sidebar.background"),
        .init("text.link", "surface.window", 4.5),
        .init("text.on_primary", "primary", 4.5),
        // Solid selected fills: a chosen segment, a primary button.
        .init("text.on_selected", "chip.selected", 4.5),
        .init("text.on_selected.muted", "chip.selected", 4.5),
        // Tinted selected fills carry their own text: the name and its second line.
        .init("sidebar.row.selected.text", "sidebar.row.selected", 4.5, underlay: "sidebar.background"),
        .init("sidebar.row.selected.muted", "sidebar.row.selected", 4.5, underlay: "sidebar.background"),
        .init("list.selected.text", "list.selected", 4.5, underlay: "surface.elevated"),
        .init("list.selected.muted", "list.selected", 4.5, underlay: "surface.elevated"),
        .init("button.text", "button.background", 4.5),
        .init("button.text", "button.hover", 4.5),
        .init("input.placeholder", "input.background", 3.0),
        .init("checkbox.border", "surface.panel", 3.0),
        .init("tab.inactive.text", "titlebar.background", 3.0),
        .init("tooltip.text", "tooltip.background", 4.5),
        // Status colours as text on the tooltip's fill, which is the opposite of the window's lightness.
        .init("tooltip.success", "tooltip.background", 4.5),
        .init("tooltip.danger", "tooltip.background", 4.5),
        .init("diff.added", "diff.added.background", 4.5),
        .init("diff.removed", "diff.removed.background", 4.5),
        .init("diff.hunk.text", "diff.hunk.background", 4.5),
        .init("find.match.current.text", "find.match.current", 4.5),
        .init("diff.line_number", "diff.added.background", 3.0),
        .init("diff.line_number", "diff.removed.background", 3.0),
        .init("git.branch", "sidebar.background", 4.5),
        .init("git.branch", "surface.panel", 4.5),
        .init("git.tag", "surface.panel", 4.5),
        .init("git.branch", "git.branch.background", 4.5, underlay: "band.solid"),
        .init("primary", "surface.window", 3.0),
        // The focus ring on every surface a focused control or row can sit on.
        .init("border.focus", "surface.window", 3.0),
        .init("border.focus", "surface.panel", 3.0),
        .init("border.focus", "surface.elevated", 3.0),
        .init("border.focus", "sidebar.background", 3.0),
        .init("border.focus", "sidebar.row.selected", 3.0, underlay: "sidebar.background"),
        .init("status.success", "surface.window", 3.0),
        .init("status.warning", "surface.window", 3.0),
        .init("status.danger", "surface.window", 3.0),
        .init("status.info", "surface.window", 3.0),
        // Status colors as text: right-panel letters and counts, on a row and a selected one.
        .init("status.success.text", "surface.panel", 4.5),
        .init("status.success.text", "state.selected", 4.5, underlay: "surface.panel"),
        .init("status.warning.text", "surface.panel", 4.5),
        .init("status.warning.text", "state.selected", 4.5, underlay: "surface.panel"),
        .init("status.danger.text", "surface.panel", 4.5),
        .init("status.danger.text", "state.selected", 4.5, underlay: "surface.panel"),
        // A project's commands: on the trust callout's tint in the panel, and in their own window.
        .init("trust.command", "banner.warning.background", 4.5, underlay: "surface.panel"),
        .init("trust.command", "surface.window", 4.5),
        // The sidebar's and the panel's cards: their fill is faint, but text and marks on them keep
        // what they have on the column (the amber edge of a card that needs input included).
        .init("text.primary", "card", 4.5, underlay: "sidebar.background"),
        .init("text.muted", "card", 4.5, underlay: "sidebar.background"),
        .init("text.faint", "card", 3.0, underlay: "sidebar.background"),
        .init("agent.attention", "card", 3.0, underlay: "sidebar.background"),
        .init("agent.attention.text", "card", 4.5, underlay: "sidebar.background"),
        .init("agent.done.text", "card", 4.5, underlay: "sidebar.background"),
        .init("agent.working", "card", 3.0, underlay: "sidebar.background"),
        .init("agent.working", "sidebar.background", 3.0),
        .init("agent.done", "sidebar.background", 3.0),
        .init("agent.attention", "sidebar.background", 3.0),
        .init("agent.exited", "sidebar.background", 3.0),
        // Agent states as text: sidebar second lines, on a row and the selected one.
        .init("agent.attention.text", "sidebar.background", 4.5),
        .init("agent.attention.text", "sidebar.row.selected", 4.5, underlay: "sidebar.background"),
        .init("agent.done.text", "sidebar.background", 4.5),
        .init("agent.done.text", "sidebar.row.selected", 4.5, underlay: "sidebar.background"),    ]
}

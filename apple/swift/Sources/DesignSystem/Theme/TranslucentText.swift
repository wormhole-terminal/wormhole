// What a window that lets the desktop through draws in place of a few tokens: core judges the text again
// on the worst colours that can show behind it (`ThemeTokens::translucent`, theme.md "Translucent windows").
// The values come from Rust, in the dump for the default look and from the resolver for any other.

public struct TranslucentText: Sendable, Equatable, Decodable {
    public let text: RGBA
    public let muted: RGBA
    public let faint: RGBA
    public let branch: RGBA
    public let selectedText: RGBA
    public let selectedMuted: RGBA
}

/// How the window lets the desktop through (`Settings.opacity` 0.3...1, `blur`). The defaults are the app's.
public struct WindowLook: Sendable, Equatable, Codable {
    public var opacity: Double
    public var blur: Bool

    public init(opacity: Double, blur: Bool) { (self.opacity, self.blur) = (opacity, blur) }
}

extension ThemeTokens {
    /// These tokens as a window at `opacity` draws them, as the GPUI app's `UiColors::new` does: the three
    /// window surfaces at that opacity, and the text tokens core judged again. `nil` text keeps them as they are.
    func windowed(opacity: Double, text seen: TranslucentText?) -> ThemeTokens {
        var ui = self.ui
        for surface in ["surface.window", "sidebar.background", "surface.panel"] {
            if let c = ui[surface] { ui[surface] = RGBA(c.red, c.green, c.blue, c.alpha * opacity) }
        }
        if let seen {
            ui["text.primary"] = seen.text
            ui["text.muted"] = seen.muted
            ui["text.faint"] = seen.faint
            ui["git.branch"] = seen.branch
            ui["sidebar.row.selected.text"] = seen.selectedText
            ui["sidebar.row.selected.muted"] = seen.selectedMuted
        }
        return ThemeTokens(terminal: terminal, metrics: metrics, ui: ui, syntax: syntax, translucent: translucent)
    }
}

// The colours today's views name, as thin readers of the live tokens. They exist so the views written
// before the token system change colour with the theme without being touched; new code reads
// `@Environment(\.tokens)` and names the Rust token (`tokens.sidebarRowSelected`). Each line says which.

import SwiftUI

@MainActor
public enum Palette {
    private static var tokens: ThemeTokens { ThemeStore.shared.tokens }

    public static var sidebarBackground: Color { tokens.sidebarBackground }
    /// `state.hover`: the generic hover overlay.
    public static var hover: Color { tokens.stateHover }
    /// `sidebar.row.selected`: the current row's opaque tint.
    public static var selected: Color { tokens.sidebarRowSelected }
    public static var divider: Color { tokens.divider }
    public static var textPrimary: Color { tokens.textPrimary }
    /// `text.muted`.
    public static var textSecondary: Color { tokens.textMuted }
    public static var textFaint: Color { tokens.textFaint }
    public static var working: Color { tokens.agentWorking }
    public static var attention: Color { tokens.agentAttention }
    public static var done: Color { tokens.agentDone }
    public static var danger: Color { tokens.statusDanger }
    /// `control.background`: a field's or badge's face.
    public static var fill: Color { tokens.controlBackground }
    /// `state.active`: a row or button while held down.
    public static var pressed: Color { tokens.stateActive }
    /// `border.focus`.
    public static var focusRing: Color { tokens.borderFocus }
}

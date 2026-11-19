// The colours and shape the chrome draws from, named as the GPUI app's `UiColors` names them and each read
// from the token `settings/ui_colors.rs` reads for it (the comment says which). Built from the tokens in the
// environment, so a theme change (or Increase Contrast, which picks another set of tokens) re-colours the
// window with no view touched. The surfaces are the window's own and get its opacity (`WindowLook.surface`).

import DesignSystem
import SwiftUI

struct ChromeColours {
    private let tokens: ThemeTokens

    init(tokens: ThemeTokens) { self.tokens = tokens }

    /// `surface.window`: the terminal's column from the title row down (`ui.background`).
    var terminal: Color { tokens.surfaceWindow }
    /// `sidebar.background` (`ui.panel`) and `surface.panel` (`ui.panel_right`).
    var sidebar: Color { tokens.sidebarBackground }
    var panel: Color { tokens.surfacePanel }
    /// The terminal's own surface under its grid (`terminal.background`).
    var terminalSurface: Color { tokens.terminal.background.color }
    var divider: Color { tokens.divider }
    /// `border`: the status bar's rule and separators.
    var rule: Color { tokens.border }
    var text: Color { tokens.textPrimary }
    var muted: Color { tokens.textMuted }
    var faint: Color { tokens.textFaint }
    /// `band.hover`: the hover of an icon button; `state.hover` the generic overlay.
    var bandHover: Color { tokens.bandHover }
    var hover: Color { tokens.stateHover }
    /// `state.selected` and `sidebar.row.selected.text`: the active icon tab and a pressed toggle.
    var selected: Color { tokens.stateSelected }
    var selectedText: Color { tokens.sidebarRowSelectedText }
    var branch: Color { tokens.gitBranch }
    var branchBackground: Color { tokens.gitBranchBackground }
    var attentionText: Color { tokens.agentAttentionText }
    var dangerText: Color { tokens.statusDangerText }
    /// `agent.attention.tint`: the fill of the status bar's button for the terminal that needs you.
    var attentionTint: Color { tokens.agentAttentionTint }
    /// The needs-you button under the pointer: the attention ring at `ATTENTION_HOVER` (30 %, `style/status.rs`).
    var attentionHover: Color { tokens.agentAttentionRing.opacity(0.3) }
    var tooltip: Color { tokens.tooltipBackground }
    var tooltipText: Color { tokens.tooltipText }
    var primary: Color { tokens.primary }
    /// `border.focus` and `metrics.focus_ring_width`: the keyboard's ring.
    var focusRing: Color { tokens.borderFocus }
    var focusRingWidth: CGFloat { tokens.metrics.focusRingWidth }

    /// `metrics.divider_width`, 0 for no line; the corners by role.
    var dividerWidth: CGFloat { tokens.metrics.dividerWidth }
    var borderWidth: CGFloat { tokens.metrics.borderWidth }
    var radius: CGFloat { tokens.metrics.radius }
    var radiusChip: CGFloat { tokens.metrics.radiusChip }
    var radiusLarge: CGFloat { tokens.metrics.radiusLg }
}

extension EnvironmentValues {
    var chrome: ChromeColours { ChromeColours(tokens: tokens) }
}

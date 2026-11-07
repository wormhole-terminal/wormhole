// The colours and shape the chrome draws from, named as the GPUI app's `UiColors` names them. This is the one
// seam between the window and the theme: until the tokens reach it they come from the system's adaptive
// colours, and a themed window replaces `ChromeColours.current` in the environment, nothing else.

import DesignSystem
import SwiftUI

struct ChromeColours {
    /// The window's own surface, behind everything (`surface.window`).
    var window: Color
    /// The sidebar's surface (`ui.panel`) and the right panel's (`ui.panel_right`).
    var sidebar: Color
    var panel: Color
    /// The terminal's column from the title row down (`ui.background`).
    var terminal: Color
    var divider: Color
    var text: Color
    var muted: Color
    var faint: Color
    var hover: Color
    var selected: Color
    var selectedText: Color
    var branch: Color
    var branchBackground: Color
    var attentionText: Color
    var dangerText: Color
    var doneText: Color
    var rule: Color
    /// The terminal's own surface under its grid, and a tooltip with the readout beside a resize handle.
    var terminalSurface: Color
    var tooltip: Color
    var tooltipText: Color
    /// The lines' width (`metrics.divider_width`), and the corners.
    var dividerWidth: CGFloat
    var radius: CGFloat
    var radiusChip: CGFloat
    var radiusLarge: CGFloat

    static let system = ChromeColours(
        window: Color(nsColor: .windowBackgroundColor), sidebar: Palette.sidebarBackground,
        panel: Color(nsColor: .windowBackgroundColor), terminal: Color(nsColor: .windowBackgroundColor),
        divider: Palette.divider, text: Palette.textPrimary, muted: Palette.textSecondary, faint: Palette.textFaint,
        hover: Palette.hover, selected: Palette.selected, selectedText: Palette.textPrimary,
        branch: Palette.textSecondary, branchBackground: Palette.hover, attentionText: Palette.attention,
        dangerText: Palette.danger, doneText: Palette.done, rule: Palette.divider,
        terminalSurface: Color(nsColor: .black), tooltip: Color(nsColor: .labelColor),
        tooltipText: Color(nsColor: .windowBackgroundColor), dividerWidth: 1,
        radius: Radius.row, radiusChip: Radius.chip, radiusLarge: Radius.card)
}

private struct ChromeColoursKey: EnvironmentKey {
    static let defaultValue = ChromeColours.system
}

extension EnvironmentValues {
    var chrome: ChromeColours {
        get { self[ChromeColoursKey.self] }
        set { self[ChromeColoursKey.self] = newValue }
    }
}

// The colours the shortcut recorder draws in: from the theme's tokens, so it follows the theme and Increase
// Contrast like every other control.

import AppKit
import DesignSystem
import SwiftUI

public struct ShortcutRecorderStyle {
    public var fill: NSColor
    public var text: NSColor
    public var muted: NSColor
    public var ring: NSColor
    public var danger: NSColor
    public var radius: CGFloat

    @MainActor public static func themed(_ tokens: ThemeTokens) -> ShortcutRecorderStyle {
        ShortcutRecorderStyle(
            fill: NSColor(tokens.controlBackground), text: NSColor(tokens.textPrimary), muted: NSColor(tokens.textMuted),
            ring: NSColor(tokens.borderFocus), danger: NSColor(tokens.statusDanger), radius: tokens.metrics.radius)
    }
}

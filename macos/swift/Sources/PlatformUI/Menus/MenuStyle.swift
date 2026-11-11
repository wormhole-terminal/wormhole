// What the menus need from the theme and nothing more: the colour of a token. The status glyphs of the status
// item's menu are drawn in the theme's status colours, not the system's (`StatusColors` in the GPUI app).

import AppKit
import DesignSystem
import PlatformModel
import SwiftUI

public struct MenuStyle {
    /// The colour of a theme token by its dotted name (`agent.attention`); nil is the menu's own text colour.
    public var color: (String) -> NSColor?

    public init(color: @escaping (String) -> NSColor?) { self.color = color }

    /// The live theme's colours for the tokens the menus use.
    @MainActor public static func themed(_ store: ThemeStore = .shared) -> MenuStyle {
        MenuStyle { token in
            let tokens = store.tokens
            let color: Color? =
                switch token {
                case "agent.attention": tokens.agentAttention
                case "agent.done": tokens.agentDone
                case "agent.working": tokens.agentWorking
                case "status.danger": tokens.statusDanger
                case "text.muted": tokens.textMuted
                default: nil
                }
            return color.map(NSColor.init)
        }
    }
}

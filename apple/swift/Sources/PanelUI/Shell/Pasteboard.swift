// Copy actions in context menus. The host may replace the destination (tests do), so views read it from the
// environment instead of touching NSPasteboard themselves.

import AppKit
import SwiftUI

typealias PanelCopy = @MainActor (String) -> Void

@MainActor
enum PanelPasteboard {
    static func copy(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}

extension EnvironmentValues {
    @Entry var panelCopy: PanelCopy = { PanelPasteboard.copy($0) }
}

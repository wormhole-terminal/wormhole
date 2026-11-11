// Copy actions in context menus. The host may replace the destination (tests do), so views read it from the
// environment instead of touching NSPasteboard themselves.

import AppKit
import DesignSystem
import PanelModel
import SwiftUI

typealias PanelCopy = @MainActor (String, PanelCopied?) -> Void

@MainActor
enum PanelPasteboard {
    static func copy(_ text: String) {
        Pasteboard.setText(text)
    }
}

extension EnvironmentValues {
    @Entry var panelCopy: PanelCopy = { text, _ in PanelPasteboard.copy(text) }
}

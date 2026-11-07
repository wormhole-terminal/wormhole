// The settings window's content: the nav at the left, a column divider, and the page. A search replaces the page
// with its results. Keyboard: F6 / ⇧F6 move between the areas, Esc clears the search and then closes (`onClose`),
// ⌘W closes (the window does that).

import DesignSystem
import SettingsModel
import AppKit
import SwiftUI

public struct SettingsView: View {
    let model: SettingsModel
    let onClose: () -> Void
    @FocusState private var focus: SettingsFocus?
    @Environment(\.tokens) private var tokens
    private static let f6 = KeyEquivalent(Character(UnicodeScalar(UInt32(NSF6FunctionKey)) ?? " "))

    public init(model: SettingsModel, onClose: @escaping () -> Void = {}) {
        (self.model, self.onClose) = (model, onClose)
    }

    public var body: some View {
        HStack(spacing: 0) {
            SettingsNav(model: model, focus: $focus)
            Rectangle().fill(tokens.divider).frame(width: tokens.metrics.dividerWidth)
            page.focusable().focused($focus, equals: .content).focusEffectDisabled()
        }
        .background(tokens.surfaceWindow)
        .foregroundStyle(tokens.textPrimary)
        .onKeyPress(.escape) { escape() }
        .onKeyPress(keys: [Self.f6], phases: .down) { cycle(backwards: $0.modifiers.contains(.shift)) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(SettingsText.settings)
        .motion(Motion.selection, value: model.selection)
    }

    @ViewBuilder private var page: some View {
        if model.isSearching {
            ResultsPage(model: model)
        } else if model.selection == SettingsModel.changedID {
            ChangedPage(model: model)
        } else if let section = model.schema.section(model.selection) {
            SettingsPage(model: model, section: section)
        }
    }

    private func escape() -> KeyPress.Result {
        if model.isSearching {
            model.query = ""
        } else {
            onClose()
        }
        return .handled
    }

    private func cycle(backwards: Bool) -> KeyPress.Result {
        focus = (focus ?? .content).next(backwards: backwards)
        return .handled
    }
}

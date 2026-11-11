// The settings window's content: the nav at the left, a column divider, and the page. A search replaces the page
// with its results. Keyboard: F6 / ⇧F6 move between the areas, Esc clears the search and then closes (`onClose`),
// ⌘W closes (the window does that).

import DesignSystem
import FontPickerModel
import FontPickerUI
import SettingsModel
import AppKit
import SwiftUI

public struct SettingsView: View {
    @Bindable var model: SettingsModel
    let onClose: () -> Void
    @FocusState private var focus: SettingsFocus?
    @State private var fonts: FontLibrary
    @Environment(\.tokens) private var tokens
    private static let f6 = KeyEquivalent(Character(UnicodeScalar(UInt32(NSF6FunctionKey)) ?? " "))

    public init(model: SettingsModel, onClose: @escaping () -> Void = {}) {
        (self.model, self.onClose) = (model, onClose)
        _fonts = State(initialValue: FontLibrary(rules: model.host.fontRules, system: CoreTextFonts(), recents: FileFontRecents()))
    }

    public var body: some View {
        NavigationSplitView {
            SettingsNav(model: model, focus: $focus)
                .navigationSplitViewColumnWidth(min: 200, ideal: SettingsMetric.navWidth, max: 280)
        } detail: {
            page.focusable().focused($focus, equals: .content).focusEffectDisabled()
                .background(tokens.surfaceWindow)
        }
        // The system's search field, in the sidebar's header; Return opens the first result.
        .searchable(text: $model.query, placement: .sidebar, prompt: SettingsText.searchSettings)
        .searchFocused($focus, equals: .search)
        .onSubmit(of: .search) { if let first = model.hits.first { model.jump(to: first) } }
        // The theme enters the system's controls here: their tint (switch, slider, selection) and, in `themed`, light or dark.
        .tint(tokens.primary)
        .secondaryBackdrop(.settings)
        .foregroundStyle(tokens.textPrimary)
        .environment(\.fontLibrary, fonts)
        .environment(\.textScale, SettingsMetric.textScale(model.value("settings_text_size").string))
        .onKeyPress(.escape) { escape() }
        .onKeyPress(keys: [Self.f6], phases: .down) { cycle(backwards: $0.modifiers.contains(.shift)) }
        .onKeyPress(keys: ["f"], phases: .down) { unlockFlags($0) }
        // Locking the feature flags while their page is open takes the page away with its row.
        .onChange(of: model.navSections.map(\.id)) { _, ids in
            if !ids.contains(model.selection), model.selection != "profile", model.selection != SettingsModel.changedID, let first = ids.first {
                model.show(first)
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(SettingsText.settings)
        .motion(Motion.selection, value: model.selection)
        .sheet(isPresented: $model.showsNewProfile) { NewProfileSheet(model: model) }
        .alert(SettingsText.deleteProfileTitle, isPresented: $model.confirmsProfileDelete) {
            Button(SettingsText.delete, role: .destructive, action: deleteProfile)
            Button(SettingsText.cancel, role: .cancel) {}
        } message: {
            Text(activeProfile?.deleteQuestion ?? "")
        }
    }

    /// The profile Wormhole is in, which Delete would remove (never Default).
    private var activeProfile: ProfileChoice? { model.host.panelRecords().profiles.first(where: \.isActive) }

    private func deleteProfile() {
        if let profile = activeProfile, !profile.isDefault { _ = model.host.removeProfile(profile.id) }
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

    /// ⌃⌥⌘F, in no menu on purpose: unlocks the feature flags in a release (and opens their page), or locks them again.
    private func unlockFlags(_ press: KeyPress) -> KeyPress.Result {
        guard press.modifiers == [.control, .option, .command] else { return .ignored }
        if model.host.toggleFlagsUnlocked() { model.show("featureFlags") }
        return .handled
    }

    private func cycle(backwards: Bool) -> KeyPress.Result {
        focus = (focus ?? .content).next(backwards: backwards)
        return .handled
    }
}

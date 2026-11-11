// The New profile sheet (`sections/profile_sheet.rs`): a name, and what the profile starts as, Wormhole's defaults or a copy
// of another profile's settings. Core refuses a name it cannot use, and the sentence stays under the field.

import DesignSystem
import SettingsModel
import SwiftUI

struct NewProfileSheet: View {
    let model: SettingsModel
    @State private var name = ""
    /// A profile's id to copy, or nil for the defaults.
    @State private var copyOf: String?
    @State private var problem: String?
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            Text(SettingsText.newProfileTitle).textRole(.heading).foregroundStyle(tokens.textPrimary)
            VStack(alignment: .leading, spacing: Space._1) {
                label(SettingsText.profileName)
                TextField(SettingsText.giveAName, text: $name).textFieldStyle(.roundedBorder).focused($focused).onSubmit(create).recordFrame("sheet.name")
            }
            VStack(alignment: .leading, spacing: Space._1) {
                label(SettingsText.startFrom)
                Picker(SettingsText.startFrom, selection: $copyOf) {
                    Text(SettingsText.wormholeDefaults).tag(String?.none)
                    ForEach(model.host.panelRecords().profiles) { Text($0.copyLabel).tag(String?.some($0.id)) }
                }
                .labelsHidden()
            }
            Text(SettingsText.loginsNeverCopied).textRole(.caption).foregroundStyle(tokens.textMuted)
                .fixedSize(horizontal: false, vertical: true)
            if let problem {
                Text(problem).textRole(.caption).foregroundStyle(tokens.statusDangerText)
                    .fixedSize(horizontal: false, vertical: true)
            }
            HStack(spacing: Space._2) {
                Spacer(minLength: 0)
                ActionButton(title: SettingsText.cancel, symbol: nil, frameId: "sheet.cancel") { model.showsNewProfile = false }.keyboardShortcut(.cancelAction)
                ActionButton(title: SettingsText.create, symbol: nil, frameId: "sheet.create", action: create).keyboardShortcut(.defaultAction)
                    .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .padding(Space._4).frame(width: 380).background(tokens.surfaceOverlay)
        .onAppear { focused = true }
    }

    private func label(_ text: String) -> some View {
        Text(text).textRole(.caption, weight: .medium).foregroundStyle(tokens.textMuted)
    }

    private func create() {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        problem = model.host.makeProfile(name: trimmed, copyOf: copyOf)
        if problem == nil { model.showsNewProfile = false }
    }
}

// Terminal → Open folders in (`draw_open_in`): the apps on this Mac that can open a folder, the one the title row's button
// opens checked, and "Choose app…" for one that is not in the list, kept by its bundle identifier. An empty setting
// opens the first app listed, so the select shows that one; an app that is gone is shown as written.

import AppKit
import DesignSystem
import SettingsModel
import SwiftUI
import UniformTypeIdentifiers
import WordsModel

struct OpenInControl: View {
    let model: SettingsModel
    let key: String
    let label: String

    var body: some View {
        let apps = model.host.options(for: .openIn)
        let chosen = model.value(key).string ?? ""
        let listed = chosen.isEmpty || apps.contains { $0.value == .string(chosen) }
        let options = listed ? apps : apps + [SettingOption(value: .string(chosen), label: chosen)]
        HStack(spacing: Space._2) {
            OptionPicker(options: options, selected: chosen.isEmpty ? (apps.first?.value ?? .string("")) : .string(chosen), label: label, width: SettingsMetric.shellWidth) {
                model.set(key, $0)
            }
            LinkButton(title: Words.text("settings-terminal-choose-app", fallback: "Choose app…")) { chooseApp() }
                .recordFrame("open-in.choose")
        }
        .fixedSize(horizontal: true, vertical: false)
    }

    /// An app from anywhere; anything that is not an app bundle is ignored.
    private func chooseApp() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.applicationBundle]
        panel.allowsMultipleSelection = false
        panel.directoryURL = URL(fileURLWithPath: "/Applications")
        panel.prompt = Words.text("settings-profile-choose", fallback: "Choose")
        guard let url = PanelPick.run(panel), let bundle = Bundle(url: url)?.bundleIdentifier else { return }
        model.set(key, .string(bundle))
    }
}

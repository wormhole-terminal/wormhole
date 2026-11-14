// One project: its trust (a badge for where it stands and the one action that fits), what it would run while it is not
// trusted, a Default | On | Off for each feature (only for the person: nothing is written into the project), what is wrong
// with its files, and the two things to do with it.

import AppKit
import DesignSystem
import SettingsModel
import SwiftUI

struct ProjectPageView: View {
    let projects: ProjectsModel
    let page: ProjectPage
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._6) {
            VStack(alignment: .leading, spacing: Space._2) {
                SettingGroup(title: ProjectsText.trust) { trustRow }
                if !page.commands.isEmpty {
                    VStack(alignment: .leading, spacing: Space._1) {
                        ForEach(page.commands, id: \.self) { Text($0).textRole(.caption, design: .monospaced) }
                    }
                    .foregroundStyle(tokens.textPrimary).padding(Space._3).frame(maxWidth: .infinity, alignment: .leading)
                    .background(tokens.settingsGroup, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
                }
                if let error = page.error { Banner(kind: .danger, text: error) }
            }
            SettingGroup(title: ProjectsText.fromThisProject, blurb: ProjectsText.onlyForYou) {
                ForEach(page.features) { feature in
                    SettingRow(label: feature.label, description: feature.description, visibleHint: true, state: RowState(), frameId: "project.feature.\(feature.key)", onReset: {}) {
                        OptionPicker(options: choices(feature), selected: encode(feature.choice), label: feature.label, segmented: true) { value in
                            Task { await projects.setChoice(feature.key, to: decode(value)) }
                        }
                    }
                    .opacity(feature.faint ? 0.5 : 1)
                }
            }
            if !page.problems.isEmpty {
                SettingGroup(title: ProjectsText.problems) {
                    VStack(spacing: Space._2) { ForEach(page.problems, id: \.self) { Banner(kind: .warning, text: $0) } }.padding(.vertical, Space._2)
                }
            }
            HStack(spacing: Space._2) {
                ActionButton(title: ProjectsText.openTerminal, symbol: Icon.openTerminal.rawValue, frameId: "project.open-terminal") { projects.openTerminal() }
                LinkButton(title: ProjectsText.showInFinder(page.dirName)) {
                    // The core makes the URL: the root is a key where the folder's name is not text.
                    Task { if let folder = await projects.projectFolder() { ExternalOpen.open(folder) } }
                }
            }
        }
    }

    private var trustRow: some View {
        HStack(spacing: Space._4) {
            VStack(alignment: .leading, spacing: 0) {
                HStack(spacing: Space._2) {
                    Text(ProjectsText.thisProject).textRole(.body, weight: .medium).foregroundStyle(tokens.textPrimary)
                    StatusBadge(text: page.statusWord, color: color)
                }
                Text(page.detail).textRole(.caption).foregroundStyle(tokens.textMuted)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if page.trust == .trusted {
                ActionButton(title: ProjectsText.revokeTrust, symbol: nil, frameId: "project.revoke-trust") { Task { await projects.revoke(page.root) } }
            } else {
                ActionButton(title: page.approving ? ProjectsText.waiting : ProjectsText.trustTouchID, symbol: Icon.touchID.rawValue, frameId: "project.trust") {
                    Task { await projects.trust() }
                }
                .disabled(page.approving)
            }
        }
        .padding(.vertical, Space._2).frame(minHeight: SettingsMetric.rowMinHeight)
        .accessibilityElement(children: .combine)
    }

    private var color: Color {
        switch page.trust {
        case .trusted: tokens.statusSuccessText
        case .changed: tokens.statusWarningText
        case .untrusted: tokens.textMuted
        }
    }

    /// Default (on or off, as it stands) | On | Off, the choice stored as null, true or false.
    private func choices(_ feature: ProjectFeatureRow) -> [SettingOption] {
        [
            SettingOption(value: .string("default"), label: feature.defaultOn ? ProjectsText.defaultOn : ProjectsText.defaultOff),
            SettingOption(value: .string("on"), label: ProjectsText.on), SettingOption(value: .string("off"), label: ProjectsText.off),
        ]
    }

    private func encode(_ choice: Bool?) -> JSONValue { .string(choice.map { $0 ? "on" : "off" } ?? "default") }
    private func decode(_ value: JSONValue) -> Bool? { value.string.flatMap { $0 == "default" ? nil : $0 == "on" } }
}

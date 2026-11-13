// One schema row on a page: a labelled `SettingRow` with its control, or the block a control brings with it.
// What the row says about itself (changed, found by a search, revealed) comes from the model and the page.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct EntryRowView: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry
    var found: SearchHit?
    var resetAlways = false

    var body: some View {
        switch entry.control.layout {
        case .strip: EmptyView()
        case .block, .bare: BlockView(model: model, entry: entry)
        case .row, .tallRow:
            VStack(alignment: .leading, spacing: 0) {
                row
                if entry.id == "shells" { ShellLines(lines: model.host.shellProfiles()) }
            }
            .id(entry.id)
        }
    }

    private var row: some View {
        SettingRow(
            label: caption?.label ?? entry.label, description: projectNote ?? caption?.text ?? entry.description,
            visibleHint: entry.visibleHint || projectNote != nil || caption != nil,
            state: state,
            tall: entry.control.layout == .tallRow, frameId: entry.id, keys: entry.keys, json: { model.host.copyJSON(keys: entry.keys) },
            chip: RowExtras.chip(entry, model), below: RowExtras.below(entry, model), onReset: { model.reset(entry) }
        ) {
            EntryControl(model: model, entry: entry)
        }
    }

    /// "Set by <project>/.wormhole/config.json while one of its terminals is in front; yours is kept": the project of the
    /// terminal in front sets this row.
    private var projectNote: String? {
        guard let project = model.host.panelRecords().project, project.entries.contains(entry.id) else { return nil }
        return project.note
    }

    /// What a row says under its label, and in place of it where GPUI's `describe` changes the label too (the permission row).
    private var caption: Caption? {
        switch entry.id {
        case "profile-start": return Caption(model.profile.form?.startCaption)
        case "worktree-base": return Caption(model.host.worktreeCaption(base: model.value("worktree_base").string ?? ""))
        case "profile-delete":
            guard model.host.panelRecords().profiles.first(where: \.isActive)?.isDefault == true else { return nil }
            return Caption(Words.text("settings-profile-the-default-profile-stays-it-holds", fallback: "The Default profile stays: it holds the data Wormhole always had."))
        case "notify-permission":
            let said = NotifyText.permission(model.notifyStatus.permission)
            return Caption(said.hint, label: said.title)
        case "notify-test": return Caption(NotifyText.test(model.notifyStatus.test))
        case "notify-needs-you": return Caption(NotifyText.needsYou(timeSensitive: model.notifyStatus.timeSensitive))
        default: return Caption(RowCaptions.text(entry, model))
        }
    }

    private struct Caption {
        let text: String
        var label: String?

        init?(_ text: String?, label: String? = nil) {
            guard let text else { return nil }
            (self.text, self.label) = (text, label)
        }
    }

    private var state: RowState {
        RowState(
            isChanged: model.isChanged(entry), resetAlways: resetAlways,
            configKey: (model.value("show_config_keys").bool ?? false) ? entry.keys.first : nil,
            found: found?.labelRange, foundDescription: found?.descriptionRange,
            isRevealed: model.revealed == entry.id, context: found?.group)
    }
}

/// The shell profiles under the Shells row: a name column, then what each runs (`list_shells`).
private struct ShellLines: View {
    let lines: [ShellLine]
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(lines, id: \.name) { line in
                HStack(spacing: Space._3) {
                    Text(line.name).textRole(.body).lineLimit(1)
                        .frame(width: SettingsMetric.shellNameWidth, alignment: .leading)
                    Text(line.commandLine).textRole(.caption, design: .monospaced)
                        .foregroundStyle(tokens.textMuted).lineLimit(1).truncationMode(.middle)
                }
                .frame(height: Metric.control)
                .accessibilityElement(children: .combine)
            }
        }
    }
}

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
                // The id a search result's reveal scrolls to. On the row, not on this stack: a group lays its rows out as
                // subviews (`SettingGroup`), which takes an id at a subview's root for its own and hides it from `scrollTo`.
                row.id(entry.id)
                if entry.id == "shells" { ShellLines(lines: model.host.shellProfiles()) }
            }
        }
    }

    private var row: some View {
        SettingRow(
            label: caption?.label ?? entry.label, description: projectNote ?? caption?.text ?? entry.description,
            visibleHint: entry.visibleHint || projectNote != nil || caption != nil,
            state: state,
            tall: entry.control.layout == .tallRow, frameId: entry.id, keys: projectNote == nil ? entry.keys : [], json: { model.host.copyJSON(keys: entry.keys) },
            chip: projectChip ?? RowExtras.chip(entry, model), below: RowExtras.below(entry, model), onReset: { model.reset(entry) }
        ) {
            EntryControl(model: model, entry: entry)
        }
    }

    /// "Set by <project>/.wormhole/config.json while one of its terminals is in front; yours is kept": the project of the
    /// terminal in front sets this row.
    private var projectNote: String? { Self.projectNote(entry, model) }

    static func projectNote(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> String? {
        guard let project = model.host.panelRecords().project, project.entries.contains(entry.id) else { return nil }
        return project.note
    }

    /// The row shows the changed dot and the reset arrow: it differs from its default and no project sets it. A row a project
    /// sets shows the project's chip instead, and its arrow would put back a value the project hides (`SettingState::Project`).
    static func isChanged(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> Bool {
        projectNote(entry, model) == nil && model.isChanged(entry)
    }

    /// The project that sets this row, as the tag after its label (the row keeps neither key nor menu then).
    private var projectChip: String? { projectNote == nil ? nil : model.host.panelRecords().project?.name }

    private var caption: Caption? { Self.caption(entry, model) }

    /// What a row says under its label, and in place of it where GPUI's `describe` changes the label too (the permission row).
    /// The control folder's `settings-page` reads the same.
    static func caption(_ entry: SettingsSchema.Entry, _ model: SettingsModel) -> Caption? {
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

    struct Caption {
        let text: String
        var label: String?

        init?(_ text: String?, label: String? = nil) {
            guard let text else { return nil }
            (self.text, self.label) = (text, label)
        }
    }

    private var state: RowState {
        RowState(
            isChanged: Self.isChanged(entry, model), resetAlways: resetAlways,
            configKey: (model.value("show_config_keys").bool ?? false) && projectNote == nil ? entry.keys.first : nil,
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

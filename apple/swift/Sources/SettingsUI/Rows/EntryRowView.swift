// One schema row on a page: a labelled `SettingRow` with its control, or the block a control brings with it.
// What the row says about itself (changed, found by a search, revealed) comes from the model and the page.

import SettingsModel
import SwiftUI

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
            SettingRow(
                label: entry.label, description: projectNote ?? entry.description, visibleHint: entry.visibleHint || projectNote != nil,
                state: state,
                tall: entry.control.layout == .tallRow, onReset: { model.reset(entry) }
            ) {
                EntryControl(model: model, entry: entry)
            }
            .id(entry.id)
        }
    }

    /// "Set by <project>/.wormhole/config.json while one of its terminals is in front; yours is kept": the project of the
    /// terminal in front sets this row.
    private var projectNote: String? {
        guard let project = model.host.panelRecords().project, project.entries.contains(entry.id) else { return nil }
        return project.note
    }

    private var state: RowState {
        RowState(
            isChanged: model.isChanged(entry), resetAlways: resetAlways,
            configKey: (model.value("show_config_keys").bool ?? false) ? entry.keys.first : nil,
            found: found?.labelRange, foundDescription: found?.descriptionRange,
            isRevealed: model.revealed == entry.id, context: found?.group)
    }
}

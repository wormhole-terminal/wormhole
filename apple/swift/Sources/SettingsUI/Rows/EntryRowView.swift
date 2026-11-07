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
        case .block: BlockView(model: model, entry: entry)
        case .row, .tallRow:
            SettingRow(
                label: entry.label, description: entry.description, visibleHint: entry.visibleHint, state: state,
                tall: entry.control.layout == .tallRow, onReset: { model.reset(entry) }
            ) {
                EntryControl(model: model, entry: entry)
            }
            .id(entry.id)
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

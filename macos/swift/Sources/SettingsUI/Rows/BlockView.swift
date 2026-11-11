// A control that stands on its own on the card with no label column: the theme cards and the record lists.

import SettingsModel
import SwiftUI

struct BlockView: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry

    var body: some View {
        switch entry.control {
        case .themeCards: ThemeCards(model: model)
        case .panel(let name): PanelView(model: model, entry: entry, name: name)
        default: EmptyView()
        }
    }
}

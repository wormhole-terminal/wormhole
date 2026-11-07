// The Changes list's view options (`features/git/changes/view_menu.rs`): the files as a folder tree or a flat list, and
// the untracked ones in a card of their own or among the rest.

import PanelModel
import SwiftUI

struct ViewOptionsMenu: View {
    @Binding var layout: ChangeRows

    var body: some View {
        BandMenu(symbol: "slider.horizontal.3", label: panelText("changes-view-options", "View options")) {
            Picker(panelText("changes-view-layout", "Show files as"), selection: $layout.tree) {
                Text(panelText("changes-view-tree", "Tree")).tag(true)
                Text(panelText("changes-view-list", "List")).tag(false)
            }
            .pickerStyle(.inline)
            Toggle(panelText("changes-view-group", "Group untracked files"), isOn: $layout.groupUntracked)
        }
    }
}

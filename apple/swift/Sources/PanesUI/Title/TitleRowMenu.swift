// A right-click anywhere on the row (but on the folder, whose own is the path menu): every part with a check
// while it is shown, a reset, and a way to Settings' list, where they are also moved (`title/parts.rs`).

import PanesModel
import SwiftUI

struct TitleRowMenu: View {
    let row: TitleRow
    let actions: TitleActions

    var body: some View {
        Section(PanesText.showInTitleRow) {
            ForEach(TitlePart.allCases.filter(\.hideable), id: \.self) { part in
                Toggle(PanesText.partName(part), isOn: Binding(get: { row.shows(part) }, set: { actions.setPart(part, $0) }))
            }
        }
        Divider()
        Button(PanesText.resetTitleRow, action: actions.resetRow).disabled(row.isDefault)
        Button(PanesText.customizeTitleRow, action: actions.customizeRow)
    }
}

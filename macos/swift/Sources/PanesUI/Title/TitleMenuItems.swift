// A title row menu's entries as a SwiftUI `.contextMenu` draws them (a right-click on the row, or on its folder). A heading is
// a line of text: a SwiftUI `Section` would add separators of its own above and below.

import PanesModel
import SwiftUI

struct TitleMenuItems: View {
    let entries: [TitleMenuEntry]

    var body: some View {
        ForEach(entries.indices, id: \.self) { index in
            switch entries[index] {
            case .heading(let text): Text(text)
            case .divider: Divider()
            case .item(let title, let enabled, let checked, let icon, let run):
                Group {
                    if let checked {
                        Toggle(title, isOn: Binding(get: { checked }, set: { _ in run() }))
                    } else if let icon {
                        Button(action: run) { Label { Text(title) } icon: { Image(nsImage: TitleMenuEntries.icon(of: icon)) } }
                    } else {
                        Button(title, action: run)
                    }
                }
                .disabled(!enabled)
            }
        }
    }
}

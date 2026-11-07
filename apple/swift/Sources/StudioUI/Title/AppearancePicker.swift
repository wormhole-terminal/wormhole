// Light | Dark for a theme that ships both: which of its versions the Studio edits and shows. The system's
// segmented control, so it speaks and moves like every other (`appearance chips` of the title row).

import DesignSystem
import StudioModel
import SwiftUI

struct AppearancePicker: View {
    @Bindable var document: StudioDocument

    var body: some View {
        if document.file.isPair {
            Picker(StudioWords.appearance, selection: Binding(
                get: { document.shown ?? "" }, set: { document.switchVersion(to: $0) })
            ) {
                ForEach(document.file.versions, id: \.self) { Text($0).tag($0) }
            }
            .pickerStyle(.segmented).controlSize(.small).fixedSize()
        }
    }
}

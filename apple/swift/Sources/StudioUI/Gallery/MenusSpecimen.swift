// The select: a trigger in two widths, and the pop-up button a Settings row uses (`gallery/selects.rs`). The menu
// itself is the system's, drawn when opened.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SwiftUI

struct MenusSpecimen: View {
    @State private var theme = "Tokyo Night"
    @State private var branch = "main"
    @State private var mode = "Match system"
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(alignment: .top, spacing: Space._4) {
            Picker("", selection: $theme) { ForEach(["Tokyo Night", "Nord", "Dracula"], id: \.self) { Text($0) } }
                .labelsHidden().frame(width: 220)
            Picker("", selection: $branch) { ForEach(["main", "ui-unify"], id: \.self) { Text($0) } }
                .labelsHidden().controlSize(.small).frame(width: 110)
            Picker("Mode", selection: $mode) { ForEach(["Match system", "Dark", "Light"], id: \.self) { Text($0) } }
                .frame(width: 220)
        }
        .tint(tokens.primary)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

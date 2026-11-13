// The select: a trigger in two widths, and the pop-up button a Settings row uses (`gallery/selects.rs`). Their menus
// are the system's, drawn when opened (System controls); under them the one opened menu the app draws itself, the rev
// picker's list with its search row.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import PanelUI
import SwiftUI

struct MenusSpecimen: View {
    @State private var theme = "Tokyo Night"
    @State private var branch = "main"
    @State private var mode = "Match system"
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            triggers
            RevListSpecimen()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var triggers: some View {
        HStack(alignment: .top, spacing: Space._4) {
            Picker("", selection: $theme) { ForEach(["Tokyo Night", "Nord", "Dracula"], id: \.self) { Text($0) } }
                .labelsHidden().frame(width: 220).marked("select")
            Picker("", selection: $branch) { ForEach(["main", "ui-unify"], id: \.self) { Text($0) } }
                .labelsHidden().controlSize(.small).frame(width: 110).marked("select")
            Picker("Mode", selection: $mode) { ForEach(["Match system", "Dark", "Light"], id: \.self) { Text($0) } }
                .frame(width: 220).marked("select")
        }
        .tint(tokens.primary)
    }
}

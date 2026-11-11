// The theme's name in the title row: a button that is a menu. It opens another theme (yours, then Wormhole's),
// and renames, duplicates, reverts, browses the versions of, or trashes this one (`studio/theme_menu.rs`).

import DesignSystem
import StudioModel
import SwiftUI

struct ThemeNameMenu: View {
    @Bindable var session: StudioSession
    @Binding var showsVersions: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        let document = session.document
        Menu {
            if !session.userThemeNames.isEmpty {
                Section(StudioWords.yourThemes) { ForEach(session.userThemeNames, id: \.self) { name in openButton(name) } }
            }
            Section(StudioWords.wormhole) { ForEach(session.builtInNames, id: \.self) { name in openButton(name) } }
            Divider()
            Button(StudioWords.rename) { session.renaming = true }
            Button(StudioWords.saveCopy) { session.saveCopy() }
            Button(StudioWords.revert) { document.revertToSaved() }.disabled(!document.isDirty)
            if document.subject.url != nil {
                Button(StudioWords.versions) { showsVersions = true }
                Button(StudioWords.trash, role: .destructive) { Task { await session.trash() } }
            }
        } label: {
            // One Text, so the system's menu label keeps the chevron after the name.
            (Text(document.name).font(TextRole.heading.font.weight(.medium))
                + Text(" ") + Text(Image(systemName: "chevron.down")).font(.system(size: Metric.chevron, weight: .bold)))
                .lineLimit(1).truncationMode(.tail)
                .frame(maxWidth: StudioMetrics.nameMaxWidth)
        }
        .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
        .help(StudioWords.tooltipMenu)
        .accessibilityLabel(document.name)
        .accessibilityHint(StudioWords.tooltipMenu)
    }

    private func openButton(_ name: String) -> some View {
        Button(name) { Task { await session.open(name) } }
    }
}

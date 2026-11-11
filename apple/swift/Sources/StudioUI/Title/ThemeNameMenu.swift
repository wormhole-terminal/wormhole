// The theme's name in the title row: a button that is a menu (`studio/theme_menu.rs`). Your themes (the one edited
// checked), the project's theme, New Theme, New Theme From (yours, then the built-ins by family, Wormhole's own under one
// name), Duplicate, Rename, Move to Trash, Add Light or Dark Appearance, Update from the theme it was copied from,
// Revert to Saved and the saved versions.

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
            // Read when the menu opens: listing the themes reads the folder.
            let list = session.themeList
            if !list.own.isEmpty {
                Section(StudioWords.yourThemes) {
                    ForEach(list.own) { choice in
                        checkable(choice.name, checked: editing(choice)) { Task { await session.open(choice, builtin: false) } }
                    }
                }
            }
            if session.hasProjectTheme {
                Section(StudioWords.menuThisProject) {
                    let checked: Bool = if case .project = document.subject { true } else { false }
                    checkable(StudioWords.menuProjectTheme, checked: checked) { Task { await session.openProject() } }
                }
            }
            Divider()
            Button(StudioWords.menuNew) { Task { await session.openNew() } }
            Menu(StudioWords.menuNewFrom) { newFrom(list) }
            Button(StudioWords.menuDuplicate(document.name)) { session.duplicate() }.keyboardShortcut("s", modifiers: [.command, .shift])
            Button(StudioWords.rename) { session.renaming = true }
            if document.subject.url != nil {
                Button(StudioWords.trash, role: .destructive) { Task { await session.trash() } }
            }
            if let missing = document.missingAppearance {
                Button(missing == .light ? StudioWords.menuAddLight : StudioWords.menuAddDark) { session.addAppearance(missing) }
            }
            updateItem(document)
            Divider()
            Button(StudioWords.revert) { document.revertToSaved() }.disabled(!document.isDirty)
            Menu(StudioWords.menuRevertTo) {
                Button(StudioWords.versions) { showsVersions = true }.disabled(document.subject.url == nil)
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

    private func editing(_ choice: ThemeChoice) -> Bool { session.document.subject.url?.path == choice.value }

    /// The system's own check mark beside the item the draft is.
    @ViewBuilder private func checkable(_ title: String, checked: Bool, action: @escaping () -> Void) -> some View {
        Toggle(title, isOn: Binding(get: { checked }, set: { _ in action() }))
    }

    /// New Theme From: the person's themes, then the built-ins (a family once), each a copy to start from.
    @ViewBuilder private func newFrom(_ list: ThemeList) -> some View {
        ForEach(list.own) { own in Button(own.name) { Task { await session.newFrom(own, builtin: false) } } }
        if !list.own.isEmpty { Divider() }
        let wormhole = list.builtins.filter(\.wormhole)
        if !wormhole.isEmpty { Menu(StudioWords.wormhole) { ForEach(wormhole) { family($0) } } }
        ForEach(list.builtins.filter { !$0.wormhole }) { family($0) }
    }

    /// A built-in family as one item, or a submenu of its appearances.
    @ViewBuilder private func family(_ family: ThemeFamily) -> some View {
        if let only = family.choices.first, family.choices.count == 1 {
            Button(only.name) { Task { await session.newFrom(only, builtin: true) } }
        } else {
            Menu(family.name) {
                ForEach(family.choices) { choice in Button(choice.name) { Task { await session.newFrom(choice, builtin: true) } } }
            }
        }
    }

    /// "Update from X…" while the source changed, and the two states that say nothing is to be done, greyed.
    @ViewBuilder private func updateItem(_ document: StudioDocument) -> some View {
        let update = document.update
        switch update.state {
        case .none: EmptyView()
        case .changed: Button(StudioWords.updateFrom(update.from)) { session.askUpdate() }
        case .current: Button(StudioWords.updateCurrent(update.from)) {}.disabled(true)
        case .missing: Button(StudioWords.updateMissing(update.from)) {}.disabled(true)
        }
    }
}

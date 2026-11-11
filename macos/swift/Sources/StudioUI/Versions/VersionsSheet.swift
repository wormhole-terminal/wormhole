// Versions: the earlier saves of a theme, newest first, the file as it is now on top and the original marked.
// Selecting a row (the arrows or the pointer) previews it in every window; Restore loads it as unsaved edits,
// so it can be undone or saved; Done puts the draft back (`studio/versions.rs`).

import DesignSystem
import StudioModel
import SwiftUI

struct VersionsSheet: View {
    @Bindable var document: StudioDocument
    let dismiss: () -> Void
    @State private var rows: [VersionRow] = []
    @State private var selected: VersionRow.ID?
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            Text(StudioWords.versionsTitle(document.name)).font(TextRole.heading.font)
            if rows.count <= 1 {
                Text(StudioWords.versionsEmpty).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            }
            List(rows, selection: $selected) { row in
                VersionRowView(row: row).tag(row.id)
            }
            .listStyle(.inset).scrollContentBackground(.hidden).frame(minHeight: 220)
            Text(StudioWords.versionsNote).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
            HStack {
                Spacer()
                Button(StudioWords.restore) { restore() }.disabled(current == nil || current?.id == nil)
                Button(StudioWords.done) { document.refresh(); dismiss() }.keyboardShortcut(.defaultAction)
            }
        }
        .padding(Space._4).frame(width: 440).background(tokens.surfaceElevated)
        .onAppear { rows = document.versionRows(); selected = rows.first?.id }
        .onChange(of: selected) { _, _ in if let current { document.preview(current) } }
        .onExitCommand { document.refresh(); dismiss() }
        .accessibilityElement(children: .contain)
    }

    private var current: VersionRow? { rows.first { $0.id == selected } }

    private func restore() {
        if let current { document.restore(current) }
        document.refresh()
        dismiss()
    }
}

private struct VersionRowView: View {
    let row: VersionRow
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 0) {
                Text(title).font(TextRole.body.font)
                Text(row.file == nil ? StudioWords.unreadable : row.summary).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            }
            Spacer(minLength: 0)
        }
        .accessibilityElement(children: .combine)
    }

    private var title: String {
        guard let saved = row.saved else { return StudioWords.versionsCurrent }
        let when = VersionTime.words(for: saved, now: Date())
        return row.pinned ? StudioWords.original(when) : when
    }
}

// The panel's three tabs as specimens: each owns a store over the sample backend, loaded once when it appears,
// because the panel's views read their stores and no longer take plain models.

import PanelModel
import PanelUI
import SwiftUI

struct NotesSpecimen: View {
    @State private var store = NotesStore(model: PanelSamples.notes)
    var body: some View { NotesView(store: store) }
}

struct ChangesSpecimen: View {
    @State private var store = ChangesStore(git: MockGitBackend(), highlighter: LexerHighlighter())
    let opensFirst: Bool

    var body: some View {
        ChangesView(store: store).task {
            store.refresh()
            if opensFirst { store.open(PanelSamples.files.first?.id) }
        }
    }
}

struct HistorySpecimen: View {
    @State private var store = HistoryStore(git: MockGitBackend(), highlighter: LexerHighlighter())
    var body: some View { HistoryView(store: store, now: PanelSamples.now).task { store.refresh() } }
}

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
        // The window's in the app: here the specimen's, so a click on a line shows the editor.
        .commentSheet(store)
    }
}

struct HistorySpecimen: View {
    @State private var store = HistoryStore(git: MockGitBackend(), highlighter: LexerHighlighter())
    var body: some View { HistoryView(store: store, now: PanelSamples.now).task { store.refresh() } }
}

/// The Project tab: the folder's colour, the trust callout over the commands, and the rows of what the project brings,
/// two lines each. The sample project is untrusted, so the callout shows.
struct ProjectSpecimen: View {
    @State private var store = ProjectStore(backend: MockProjectBackend())
    @State private var notes = NotesStore(model: PanelSamples.notes)

    var body: some View {
        ProjectView(store: store, notes: notes).task { store.follow(PanelSamples.project.root) }
    }
}

/// The Files tab: the path button and the tree's folder and file rows, over a small virtual project.
struct FilesSpecimen: View {
    @State private var store = FilesStore(backend: MockFilesBackend(pinned: []))

    var body: some View {
        FilesView(store: store).task { store.follow(PanelSamples.project.root) }
    }
}

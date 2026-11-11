import Foundation
import PanelModel
import Testing
import WindowUI
import WorkspaceModel

@testable import Wormhole

/// What a notes.md merge brings into the terminal on show is in the database before anything else happens: quit
/// right after it and the next launch must not read the merged lines as deleted.
@MainActor
struct NotesMergeSavedTests {
    final class Persistence: NotesPersisting, @unchecked Sendable {
        var saved: [Int: NotesModel] = [:]
        var merged: NotesModel?
        func load(session: Int) -> NotesModel { saved[session] ?? NotesModel() }
        func save(session: Int, model: NotesModel) -> Bool {
            saved[session] = model
            return true
        }
        func sync(session: Int, root: String, model: NotesModel, editing: Bool) async -> NotesModel? { merged }
        func closed(session: Int) {}
        func idsStart(at firstFree: Int) {}
        func follow(profile id: String) {}
    }

    @Test func aMergeIntoTheShownTerminalIsSavedAtOnce() async {
        let hub = WindowHub(store: WorkspaceStore(backend: MockWorkspaceBackend(selected: 1)), terminals: TerminalViews { _ in nil })
        let persistence = Persistence()
        let coordinator = NotesCoordinator(persistence: persistence, hub: hub)
        coordinator.start()
        var merged = hub.panel.notes.model
        merged.notes.append(Note(text: "from notes.md"))
        persistence.merged = merged
        await coordinator.sync()
        #expect(hub.panel.notes.model == merged)
        #expect(persistence.saved[1] == merged, "the merged text is in the database, not only on screen")
    }
}

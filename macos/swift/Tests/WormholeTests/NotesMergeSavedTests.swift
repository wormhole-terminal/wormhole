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

    /// Holds terminal 1's merge in flight until the test lets it land; reports the load of terminal 2 (the switch).
    final class Held: NotesPersisting, @unchecked Sendable {
        var saved: [Int: NotesModel] = [:]
        var merged: NotesModel?
        var entered: AsyncStream<Void>.Continuation?
        var switched: AsyncStream<Void>.Continuation?
        var release: CheckedContinuation<Void, Never>?
        var closedSessions: [Int] = []
        func load(session: Int) -> NotesModel {
            if session == 2, release != nil { switched?.yield() }
            return saved[session] ?? NotesModel()
        }
        func save(session: Int, model: NotesModel) -> Bool {
            saved[session] = model
            return true
        }
        func sync(session: Int, root: String, model: NotesModel, editing: Bool) async -> NotesModel? {
            guard session == 1 else { return nil }
            await withCheckedContinuation { release = $0; entered?.yield() }
            return merged
        }
        func closed(session: Int) { closedSessions.append(session) }
        func idsStart(at firstFree: Int) {}
        func follow(profile id: String) {}
    }

    /// The core has merged and moved its base when the answer comes; the terminal left meanwhile still gets the merge,
    /// stored for IT, and the terminal shown now is not given another's notes.
    @Test func aMergeForATerminalSwitchedAwayDuringItIsStoredForThatTerminal() async {
        let store = WorkspaceStore(backend: MockWorkspaceBackend(selected: 1))
        let hub = WindowHub(store: store, terminals: TerminalViews { _ in nil })
        let persistence = Held()
        let entered = AsyncStream<Void>.makeStream()
        let switched = AsyncStream<Void>.makeStream()
        persistence.entered = entered.continuation
        persistence.switched = switched.continuation
        let coordinator = NotesCoordinator(persistence: persistence, hub: hub)
        coordinator.start()
        var merged = hub.panel.notes.model
        merged.notes.append(Note(text: "from notes.md"))
        persistence.merged = merged
        let running = Task { await coordinator.sync() }
        var inFlight = entered.stream.makeAsyncIterator()
        _ = await inFlight.next()
        store.send(.select(2))
        var moved = switched.stream.makeAsyncIterator()
        _ = await moved.next()
        persistence.release?.resume()
        await running.value
        #expect(persistence.saved[1]?.notes.map(\.text) == ["from notes.md"], "stored for the terminal the merge was for")
        #expect(!hub.panel.notes.model.notes.contains { $0.text == "from notes.md" }, "not applied to the terminal shown now")
        #expect(persistence.saved[2] == nil)
    }
}

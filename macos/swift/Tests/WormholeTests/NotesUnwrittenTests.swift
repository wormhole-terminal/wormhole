import Foundation
import PanelModel
import Testing
import WindowUI
import WorkspaceModel

@testable import Wormhole

/// A save the database did not take: the notes are kept in memory, as the GPUI app keeps every terminal's
/// (`features/project/notes_sync.rs:85` merges from them), read from there and saved again, so a merge with notes.md
/// never runs against a database that lacks what the person wrote.
@MainActor
struct NotesUnwrittenTests {
    /// The database (each terminal's steps, by their words), notes.md and the core's merge bases, which outlast a relaunch.
    /// `sync` is core's three-way merge for steps told apart by their words (`project::plan_sync`).
    final class Store: NotesPersisting, @unchecked Sendable {
        var saved: [Int: [String]] = [:]
        /// How many of a terminal's next saves the database refuses.
        var refused: [Int: Int] = [:]
        var file: [String] = []
        var bases: [Int: [String]] = [:]

        func read(session: Int) -> NotesModel? {
            NotesModel(notes: [], tasks: TaskList((saved[session] ?? []).map { PanelTask(text: $0) }))
        }

        func save(session: Int, model: NotesModel) -> Bool {
            if let left = refused[session], left > 0 {
                refused[session] = left - 1
                return false
            }
            saved[session] = model.tasks.items.map(\.text)
            return true
        }

        func sync(session: Int, root: String, model: NotesModel, editing: Bool) async -> NotesModel? {
            let local = model.tasks.items.map(\.text)
            let merged: [String]
            if let base = bases[session] {
                // The file's lines but the ones the person deleted, then the person's new ones; a line of the base that
                // the file lacks was deleted there.
                let kept = file.filter { local.contains($0) || !base.contains($0) }
                merged = kept + local.filter { !base.contains($0) && !kept.contains($0) }
            } else {
                merged = file + local.filter { !file.contains($0) }
            }
            file = merged
            bases[session] = merged
            guard merged != local else { return nil }
            var answer = model
            answer.tasks = TaskList(merged.map { text in model.tasks.items.first { $0.text == text } ?? PanelTask(text: text) })
            return answer
        }

        func closed(session: Int) { bases[session] = nil }
        func idsStart(at firstFree: Int) {}
        func follow(profile id: String) {}
    }

    /// A window over `persistence`: a launch, or a relaunch on the same database, file and bases.
    private func window(_ persistence: Store) -> (WorkspaceStore, WindowHub, NotesCoordinator) {
        let store = WorkspaceStore(backend: MockWorkspaceBackend(selected: 1))
        let hub = WindowHub(store: store, terminals: TerminalViews { _ in nil })
        let coordinator = NotesCoordinator(persistence: persistence, hub: hub, merging: NotesMerging(), syncEvery: nil)
        coordinator.start()
        return (store, hub, coordinator)
    }

    private static func synced(_ steps: [String], terminal: Int = 1) -> Store {
        let persistence = Store()
        persistence.saved[terminal] = steps
        persistence.file = steps
        persistence.bases[terminal] = steps
        return persistence
    }

    /// The write of an edit fails once; a merge takes the edit into notes.md and the base. The next look saves it again,
    /// so after a relaunch the line is in the database and stays in notes.md (unsaved, it would read as deleted there).
    @Test func anEditWhoseWriteFailedOnceIsThereAfterAMergeAndARelaunch() async {
        let persistence = Self.synced(["build"])
        let (_, hub, coordinator) = window(persistence)
        persistence.refused[1] = 1
        hub.panel.notes.model.tasks.add("call Ann")
        await coordinator.sync()
        #expect(persistence.file.contains("call Ann"), "the merge took the edit into notes.md")
        await coordinator.sync()
        #expect(persistence.saved[1] == ["build", "call Ann"], "saved again at the next look")

        let (_, again, relaunched) = window(persistence)
        await relaunched.sync()
        #expect(persistence.file.contains("call Ann"), "not read as a line the person deleted")
        #expect(again.panel.notes.model.tasks.items.map(\.text) == ["build", "call Ann"])
    }

    /// The same, with the quit right after the merge: the quit writes once more what the database did not take.
    @Test func anEditWhoseWriteFailedIsSavedAgainAtQuit() async {
        let persistence = Self.synced(["build"])
        let (_, hub, coordinator) = window(persistence)
        persistence.refused[1] = 1
        hub.panel.notes.model.tasks.add("call Ann")
        await coordinator.sync()
        coordinator.flush()
        #expect(persistence.saved[1] == ["build", "call Ann"])

        let (_, _, relaunched) = window(persistence)
        await relaunched.sync()
        #expect(persistence.file.contains("call Ann"))
    }

    /// The edit's save fails as the person switches to another terminal: the terminal left holds the edit in memory, is
    /// merged from there (never from the database that lacks it), is saved at the next look and shows it when shown again.
    @Test func anEditNotSavedOnSwitchingAwayIsKeptMergedAndShownAgain() async {
        let persistence = Self.synced(["build"])
        let (store, hub, coordinator) = window(persistence)
        persistence.refused[1] = 1
        hub.panel.notes.model.tasks.add("call Ann")
        store.send(.select(2))
        await coordinator.sync()
        await coordinator.sync()
        #expect(persistence.file.contains("call Ann"), "merged from what the terminal holds, and not taken out again")
        #expect(persistence.saved[1] == ["build", "call Ann"], "saved once the database takes it")
        store.send(.select(1))
        await coordinator.sync()
        #expect(hub.panel.notes.model.tasks.items.map(\.text) == ["build", "call Ann"])
    }

    /// While the database goes on refusing, the terminal shown again shows what it held, not the database's older copy.
    @Test func aTerminalShownAgainShowsWhatTheDatabaseDidNotTake() async {
        let persistence = Self.synced(["build"])
        let (store, hub, coordinator) = window(persistence)
        persistence.refused[1] = 100
        hub.panel.notes.model.tasks.add("call Ann")
        store.send(.select(2))
        await coordinator.sync()
        store.send(.select(1))
        await coordinator.sync()
        #expect(hub.panel.notes.model.tasks.items.map(\.text) == ["build", "call Ann"])
        #expect(persistence.saved[1] == ["build"], "the database still refuses")
        persistence.refused[1] = 0
        await coordinator.sync()
        #expect(persistence.saved[1] == ["build", "call Ann"])
    }
}

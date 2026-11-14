import Foundation
import PanelModel
import Testing
import WindowUI
import WorkspaceModel

@testable import Wormhole

/// A notes.md merge lands on what the terminal holds by then, and what it brings is in the database before anything else
/// happens: nothing of the person's or the file's is lost or there twice, whichever terminal is shown and whatever the
/// person did while the merge ran.
@MainActor
struct NotesMergeSavedTests {
    /// The database as the real store is (`RustNotesPersistence`, `NotesMapping.swift` `init(saved:)`): it keeps what an item
    /// says, never its id, so every read makes each item anew; blank ones are not kept. The merge's answer is built from what
    /// was sent, as `applying` builds it: the items sent keep their ids.
    final class Store: NotesPersisting, @unchecked Sendable {
        var saved: [Int: NotesModel] = [:]
        var unreadable: Set<Int> = []
        /// What the merge answers for a terminal, given what was sent; nil: nothing changed.
        var answer: (Int, NotesModel) -> NotesModel? = { _, _ in nil }
        var sent: [Int: NotesModel] = [:]
        var closedSessions: [Int] = []
        /// The terminal whose merge waits until the test lets it land.
        var held: Int?
        var entered: AsyncStream<Void>.Continuation?
        var release: CheckedContinuation<Void, Never>?

        func read(session: Int) -> NotesModel? {
            guard !unreadable.contains(session) else { return nil }
            guard let model = saved[session] else { return NotesModel() }
            var fresh = NotesModel(
                notes: model.notes.filter { !$0.isBlank }.map { Note(text: $0.text, collapsed: $0.collapsed) },
                tasks: TaskList(model.tasks.items.map { PanelTask(text: $0.text, done: $0.done) }))
            (fresh.stepsCollapsed, fresh.notesCollapsed) = (model.stepsCollapsed, model.notesCollapsed)
            return fresh
        }

        func save(session: Int, model: NotesModel) -> Bool {
            saved[session] = model
            return true
        }

        func sync(session: Int, root: String, model: NotesModel, editing: Bool) async -> NotesModel? {
            sent[session] = model
            if session == held { await withCheckedContinuation { release = $0; entered?.yield() } }
            return answer(session, model)
        }

        func closed(session: Int) { closedSessions.append(session) }
        func idsStart(at firstFree: Int) {}
        func follow(profile id: String) {}
    }

    private static func model(steps: [String] = [], notes: [String] = []) -> NotesModel {
        NotesModel(notes: notes.map { Note(text: $0) }, tasks: TaskList(steps.map { PanelTask(text: $0) }))
    }

    private static func steps(_ model: NotesModel?) -> [String] {
        model?.tasks.items.map { "\($0.done ? "x" : " ") \($0.text)" } ?? []
    }

    private func window(_ persistence: Store, merging: NotesMerging = NotesMerging()) -> (WorkspaceStore, WindowHub, NotesCoordinator) {
        let store = WorkspaceStore(backend: MockWorkspaceBackend(selected: 1))
        let hub = WindowHub(store: store, terminals: TerminalViews { _ in nil })
        let coordinator = NotesCoordinator(persistence: persistence, hub: hub, merging: merging)
        coordinator.start()
        return (store, hub, coordinator)
    }

    /// Runs a sync whose merge for `persistence.held` waits; `meanwhile` happens while it waits, then it lands.
    private func syncHeld(_ coordinator: NotesCoordinator, _ persistence: Store, meanwhile: () -> Void) async {
        let entered = AsyncStream<Void>.makeStream()
        persistence.entered = entered.continuation
        let running = Task { await coordinator.sync() }
        var inFlight = entered.stream.makeAsyncIterator()
        _ = await inFlight.next()
        meanwhile()
        persistence.release?.resume()
        await running.value
    }

    @Test func aMergeIntoTheShownTerminalIsSavedAtOnce() async {
        let persistence = Store()
        let (_, hub, coordinator) = window(persistence)
        persistence.answer = { id, sent in
            guard id == 1 else { return nil }
            var merged = sent
            merged.notes.append(Note(text: "from notes.md"))
            return merged
        }
        await coordinator.sync()
        #expect(hub.panel.notes.model.notes.map(\.text) == ["from notes.md"])
        #expect(persistence.saved[1]?.notes.map(\.text) == ["from notes.md"], "the merged text is in the database, not only on screen")
    }

    /// The core has merged and moved its base when the answer comes; the terminal left meanwhile still gets the merge,
    /// stored for IT, and the terminal shown now is not given another's notes.
    @Test func aMergeForATerminalSwitchedAwayDuringItIsStoredForThatTerminal() async {
        let persistence = Store()
        let (store, hub, coordinator) = window(persistence)
        persistence.held = 1
        persistence.answer = { id, sent in
            guard id == 1 else { return nil }
            var merged = sent
            merged.notes.append(Note(text: "from notes.md"))
            return merged
        }
        await syncHeld(coordinator, persistence) { store.send(.select(2)) }
        #expect(persistence.saved[1]?.notes.map(\.text) == ["from notes.md"], "stored for the terminal the merge was for")
        #expect(!hub.panel.notes.model.notes.contains { $0.text == "from notes.md" }, "not applied to the terminal shown now")
        #expect(persistence.saved[2]?.notes.contains { $0.text == "from notes.md" } != true)
    }

    /// A terminal that is not shown is sent as read from the database and landed on a second read: the two share no id,
    /// and nothing may come back twice.
    @Test func aTerminalNotOnShowIsMergedOnceFromTheDatabase() async {
        let persistence = Store()
        persistence.saved[2] = Self.model(steps: ["build", "ship"], notes: ["Ask Ann"])
        let (_, _, coordinator) = window(persistence)
        persistence.answer = { id, sent in
            guard id == 2 else { return nil }
            var merged = sent
            merged.tasks = TaskList(sent.tasks.items + [PanelTask(text: "from the file")])
            merged.notes.append(Note(text: "A file's note"))
            return merged
        }
        await coordinator.sync()
        #expect(Self.steps(persistence.saved[2]) == ["  build", "  ship", "  from the file"])
        #expect(persistence.saved[2]?.notes.map(\.text) == ["Ask Ann", "A file's note"])
        // The next look finds nothing to do, and nothing changes.
        persistence.answer = { _, _ in nil }
        await coordinator.sync()
        #expect(Self.steps(persistence.saved[2]) == ["  build", "  ship", "  from the file"])
    }

    /// A step ticked and moved while the merge runs stays as the person left it, keeps its row, and the file's own tick and
    /// step come in.
    @Test func aStepTickedAndMovedDuringTheMergeStaysSo() async throws {
        let persistence = Store()
        persistence.saved[1] = Self.model(steps: ["a", "b", "c"])
        let (_, hub, coordinator) = window(persistence)
        let b = try #require(hub.panel.notes.model.tasks.items.first { $0.text == "b" })
        persistence.held = 1
        persistence.answer = { id, sent in
            guard id == 1 else { return nil }
            // The file ticked "a" and added "d".
            var merged = sent
            var items = sent.tasks.items
            items[0].done = true
            merged.tasks = TaskList(items + [PanelTask(text: "d")])
            return merged
        }
        await syncHeld(coordinator, persistence) {
            hub.panel.notes.model.tasks.toggle(b.id)
            hub.panel.notes.model.tasks.move(from: 1, to: 0)
        }
        #expect(Self.steps(hub.panel.notes.model) == ["x b", "x a", "  c", "  d"])
        #expect(hub.panel.notes.model.tasks.items.first?.id == b.id, "the step keeps its row")
        #expect(Self.steps(persistence.saved[1]) == ["x b", "x a", "  c", "  d"])
    }

    /// A step deleted while the merge runs stays deleted, one added stays once, where it was put.
    @Test func aStepDeletedOrAddedDuringTheMergeStaysSo() async throws {
        let persistence = Store()
        persistence.saved[1] = Self.model(steps: ["a", "b", "c"])
        let (_, hub, coordinator) = window(persistence)
        let a = try #require(hub.panel.notes.model.tasks.items.first { $0.text == "a" })
        let c = try #require(hub.panel.notes.model.tasks.items.first { $0.text == "c" })
        persistence.held = 1
        persistence.answer = { id, sent in
            guard id == 1 else { return nil }
            var merged = sent
            merged.tasks = TaskList(sent.tasks.items + [PanelTask(text: "d")])
            return merged
        }
        await syncHeld(coordinator, persistence) {
            hub.panel.notes.model.tasks.delete(c.id)
            hub.panel.notes.model.tasks.add("e", after: a.id)
        }
        #expect(Self.steps(hub.panel.notes.model) == ["  a", "  e", "  b", "  d"])
    }

    /// Two notes that say the same are two: the second rewritten during the merge is rewritten once, the first stays.
    @Test func twoNotesThatSayTheSameStayTwo() async {
        let persistence = Store()
        persistence.saved[1] = Self.model(notes: ["call Ann", "call Ann"])
        let (_, hub, coordinator) = window(persistence)
        persistence.held = 1
        persistence.answer = { id, sent in
            guard id == 1 else { return nil }
            var merged = sent
            merged.notes.append(Note(text: "from the file"))
            return merged
        }
        await syncHeld(coordinator, persistence) { hub.panel.notes.model.notes[1].text = "call Ann and Bo" }
        #expect(hub.panel.notes.model.notes.map(\.text) == ["call Ann", "call Ann and Bo", "from the file"])
        #expect(Set(hub.panel.notes.model.notes.map(\.id)).count == 3, "no two notes share an id")
    }

    /// When what a terminal holds cannot be read the merge is not laid over an empty list (which reads as every line deleted):
    /// nothing is stored, and the base the core moved is forgotten, so the next merge keeps both sides.
    @Test func aTerminalThatCannotBeReadIsNeitherSentNorStoredOver() async {
        let persistence = Store()
        persistence.saved[2] = Self.model(steps: ["build"])
        persistence.saved[12] = Self.model(steps: ["release"])
        let (_, _, coordinator) = window(persistence)
        persistence.unreadable = [12]
        persistence.held = 2
        persistence.answer = { _, sent in
            var merged = sent
            merged.tasks = TaskList(sent.tasks.items + [PanelTask(text: "from the file")])
            return merged
        }
        await syncHeld(coordinator, persistence) { persistence.unreadable.insert(2) }
        #expect(persistence.sent[12] == nil, "an unreadable terminal is not sent as empty")
        #expect(Self.steps(persistence.saved[2]) == ["  build"], "nothing stored over what could not be read")
        #expect(persistence.closedSessions.contains(2), "its base is forgotten")
    }

    /// A merge that landed while the terminal could not be read is laid once it can, before its next merge is sent: the base
    /// was forgotten at once (a crash first loses nothing), and a line the file deleted in that merge does not come back with
    /// the union that follows.
    @Test func aMergeThatCouldNotBeLaidIsLaidOnceTheTerminalReads() async {
        let persistence = Store()
        persistence.saved[2] = Self.model(steps: ["build", "old"])
        let (_, _, coordinator) = window(persistence)
        persistence.held = 2
        // Not the main actor's: the held merge answers after its wait, off it (an isolated closure ends the test process there).
        persistence.answer = { @Sendable id, sent in
            guard id == 2 else { return nil }
            // notes.md deleted "old" and gained "new".
            var merged = sent
            merged.tasks = TaskList(sent.tasks.items.filter { $0.text != "old" } + [PanelTask(text: "new")])
            return merged
        }
        await syncHeld(coordinator, persistence) { persistence.unreadable.insert(2) }
        #expect(Self.steps(persistence.saved[2]) == ["  build", "  old"], "nothing stored over what could not be read")
        #expect(persistence.closedSessions.contains(2), "its base is forgotten at once")
        persistence.held = nil
        persistence.answer = { _, _ in nil }
        persistence.unreadable = []
        // Something the control folder's `task-add` wrote meanwhile is kept; the file's line follows the one it follows there.
        persistence.saved[2]?.tasks.add("added from outside")
        await coordinator.sync()
        #expect(Self.steps(persistence.saved[2]) == ["  build", "  new", "  added from outside"])
        #expect(Self.steps(persistence.sent[2]) == ["  build", "  new", "  added from outside"], "the next merge sends what was laid")
        await coordinator.sync()
        #expect(Self.steps(persistence.saved[2]) == ["  build", "  new", "  added from outside"], "laid once")
    }

    /// The terminal on show could not be read: nothing is saved over it or merged, and once it can be read what was typed
    /// meanwhile comes after what was saved.
    @Test func theShownTerminalThatCannotBeReadComesBackWithWhatWasTyped() async {
        let persistence = Store()
        persistence.saved[1] = Self.model(steps: ["kept"])
        persistence.unreadable = [1]
        let (_, hub, coordinator) = window(persistence)
        #expect(hub.panel.notes.model.tasks.items.isEmpty)
        hub.panel.notes.model.tasks.add("typed")
        await coordinator.sync()
        #expect(persistence.sent[1] == nil, "not merged as if every line were deleted")
        #expect(Self.steps(persistence.saved[1]) == ["  kept"], "not saved over")
        persistence.unreadable = []
        await coordinator.sync()
        #expect(Self.steps(hub.panel.notes.model) == ["  kept", "  typed"])
        #expect(Self.steps(persistence.sent[1]) == ["  kept", "  typed"])
    }

    /// A terminal moved to another window or closed while its merge runs: nothing is stored for it here (the other window
    /// would save what it shows over it), and its base is forgotten, so the next merge keeps both sides.
    @Test func aTerminalThatLeftDuringTheMergeHasItsBaseForgotten() async {
        let persistence = Store()
        persistence.saved[2] = Self.model(steps: ["build"])
        let (store, _, coordinator) = window(persistence)
        persistence.held = 2
        persistence.answer = { id, sent in
            guard id == 2 else { return nil }
            var merged = sent
            merged.tasks = TaskList(sent.tasks.items + [PanelTask(text: "from the file")])
            return merged
        }
        await syncHeld(coordinator, persistence) {
            coordinator.departing([2])
            store.send(.close(2))
        }
        #expect(Self.steps(persistence.saved[2]) == ["  build"])
        #expect(persistence.closedSessions == [2])
    }

    /// A terminal moved into another window while the window it left still merges it: the window it came to does not send it
    /// until that merge has landed, so it never merges what it read of it before against the base that merge just moved
    /// (which would read the file's new lines as deleted by the person and take them out of notes.md).
    @Test func aTerminalMovedInWaitsForTheMergeTheWindowItLeftSent() async {
        let merging = NotesMerging()
        let left = Store()
        left.saved[2] = Self.model(steps: ["build"])
        let (_, _, leaving) = window(left, merging: merging)
        let arrived = Store()
        arrived.saved[2] = Self.model(steps: ["build"])
        let (_, _, receiving) = window(arrived, merging: merging)
        left.held = 2
        let entered = AsyncStream<Void>.makeStream()
        left.entered = entered.continuation
        let running = Task { await leaving.sync() }
        var inFlight = entered.stream.makeAsyncIterator()
        _ = await inFlight.next()
        await receiving.sync()
        #expect(arrived.sent[2] == nil, "not sent while the window it left merges it")
        left.release?.resume()
        await running.value
        await receiving.sync()
        #expect(arrived.sent[2] != nil, "sent once that merge has landed")
    }
}

import PaletteModel
import Testing

@testable import PlatformBridge

/// The palette's actions and the shortcut sheet as core's keymap lists them, over the real FFI.
@Suite struct KeymapListsBridgeTests {
    private let lists = CoreKeymapLists()

    @Test func theActionsAreCoresWithItsWordsOrderKeysAndGroups() {
        let list = lists.actions(ActionFacts())
        #expect(list.rows.count > 60)
        let first = list.rows.first
        #expect((first?.id, first?.label, first?.key?.glyphs, first?.section) == ("newTerminal", "New terminal", "⌘T", .terminal))
        #expect(Set(list.rows.map(\.section)) == [.terminal, .view, .app])
        #expect(list.suggested.first == "newTerminal")
        #expect(list.rows.first { $0.id == "keyboardShortcuts" }?.key?.glyphs == "⌥⌘/")
    }

    @Test func aGreyedOrHiddenActionIsNotOffered() {
        let ids = lists.actions(ActionFacts(disabled: ["showChanges"], hidden: ["showNotes"])).rows.map(\.id)
        #expect(!ids.contains("showChanges") && !ids.contains("showNotes"))
        let flipped = lists.actions(ActionFacts(off: ["toggleSidebar"])).rows.first { $0.id == "toggleSidebar" }
        #expect(flipped?.label == "Show sidebar")
    }

    @Test func theSheetHasCoresSectionsAndItsFilterMarksCharacters() {
        let all = lists.sheet(hidden: [], query: "")
        #expect(all.sections.count == 6 && all.shown == all.total && all.exampleKey == "⌘W")
        let byKey = lists.sheet(hidden: [], query: "⌘W").sections.flatMap(\.rows).map(\.label)
        #expect(byKey.first?.hasPrefix("Close terminal") == true)
        let split = lists.sheet(hidden: [], query: "split").sections.flatMap(\.rows).first { $0.label.hasPrefix("Split") }
        #expect(split?.hit == 0..<5)
        #expect(lists.sheet(hidden: ["newNote"], query: "new note").shown == 0)
        let run = all.sections.flatMap(\.rows).first { $0.label.hasPrefix("Terminals 1") }
        #expect(run?.isRun == true && run?.keys.map(\.glyphs) == ["⌘1", "⌘9"])
    }

    @Test func columnsMinimiseTheTallest() {
        #expect(lists.columns([5, 5, 5], 3) == [0..<1, 1..<2, 2..<3])
        #expect(lists.columns([9, 2, 2, 9], 2) == [0..<2, 2..<4])
        #expect(lists.columns([], 3).isEmpty)
    }
}

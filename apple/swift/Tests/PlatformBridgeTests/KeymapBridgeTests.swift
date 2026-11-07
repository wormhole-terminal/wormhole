import PlatformModel
import Testing
import WormholeBindings

@testable import PlatformBridge

/// Core's table read live, against the dump the build without the Rust core reads: the two are one table.
@Suite struct KeymapBridgeTests {
    private let live = PlatformBridge.makeKeymap()
    private let dumped = Keymap.bundled

    @Test func theLiveTableIsTheDumpedOne() {
        #expect(live.entries == dumped.entries)
        #expect(live.systemKeys.map(\.spec) == dumped.systemKeys.map(\.spec))
        #expect(live.systemKeys.map(\.what) == dumped.systemKeys.map(\.what))
        #expect(live.entries.count == 131)
    }

    @Test func menusFromCoreAreTheMenusFromTheDumpedLayout() {
        for state in Self.states() {
            let fromCore = MenuDump.text(CoreMenus().menus(for: state))
            #expect(fromCore == MenuDump.text(MenuBuilder(keymap: dumped).menus(for: state)))
            // The Swift values lose nothing of what core's own dump says.
            #expect(fromCore == keymapMenusDump(state: state.record))
        }
    }

    @Test func theMenusCarryTheirCommandsAndKeys() throws {
        let bar = CoreMenus().menus(for: Self.states()[0])
        let window = try #require(bar.first { $0.title == "Window" })
        let numbered = window.items.compactMap { node -> MenuItem? in
            if case .item(let item) = node, case .action(let id) = item.command, id.rawValue.hasPrefix("terminal.") { item } else { nil }
        }
        #expect(numbered.map(\.title) == ["sidebar polish", "diff perf", "zsh"])
        #expect(numbered.map(\.checked) == [false, true, false])
        #expect(numbered.first?.key == KeySpec(.command, "1"))
    }

    @Test func aStaleActionNameInTheStateIsDroppedNotAFailure() {
        var state = MenuState()
        state.hidden = ["noSuchAction", "newNote"]
        let text = MenuDump.text(CoreMenus().menus(for: state))
        #expect(!text.contains("New Note"))
    }

    @Test func keysReadBackTheSameInSwiftAndInCore() throws {
        var specs: [KeySpec] = live.entries.flatMap(\.keys)
        let keys = ["a", "t", "]", "}", "space", "enter", "escape", "backspace", "delete", "up", "f6", "f13", "9", "`", "~"]
        for mask in 0..<16 { for key in keys { specs.append(KeySpec(KeyMods(rawValue: mask), key)) } }
        for spec in specs {
            #expect(keySpecGlyphs(key: spec.record) == spec.glyphText, "\(spec)")
            #expect(keySpecText(key: spec.record) == spec.text, "\(spec)")
            #expect(keySpecFromGlyphs(text: spec.glyphText).map(KeySpec.init) == KeySpec(glyphs: spec.glyphText), "\(spec.glyphText)")
            #expect(keySpecSameKey(a: spec.record, b: spec.canonical.record), "\(spec)")
            // Who has the key, and what the recorder says about it.
            #expect(shortcutConflict(key: spec.record, own: nil) == ShortcutRecording.conflict(spec, in: live), "\(spec.glyphText)")
        }
        for text in ["⌘T", "⇧⌘]", "F6", "⌥Space", "⌘", "", "⌘⌘T", "⇧F21", "⌘Hyper", "⌃⌥⇧⌘↩"] {
            #expect(keySpecFromGlyphs(text: text).map(KeySpec.init) == KeySpec(glyphs: text), "\(text)")
        }
    }

    @Test func theGrammarOfConfigReadsTheSameInSwiftAndInCore() {
        for text in ["cmd-shift-r", "ctrl+alt+t", "cmd--", "ctrl++", "super-space", "return", "esc", "cmd-", "hyper-x", "cmd-cmd-x", "", "alt-⌘-t"] {
            let core = try? keySpecParse(text: text)
            let swift = try? KeySpec(parsing: text)
            #expect(core.map(KeySpec.init) == swift, "\(text)")
        }
    }

    @Test func recordingAShortcutDoesTheSameInSwiftAndInCore() {
        for mask in 0..<16 {
            for key in ["escape", "backspace", "delete", "a", "space", "f5", "f21", "r", "enter"] {
                let core: Recorded =
                    switch recordShortcut(mods: UInt8(mask), key: key) {
                    case .cancel: .cancel
                    case .clear: .clear
                    case .needsModifier: .needsModifier
                    case .shortcut(let spec): .shortcut(KeySpec(spec))
                    }
                #expect(core == ShortcutRecording.record(KeyMods(rawValue: mask), key: key), "\(mask) \(key)")
            }
        }
    }

    @Test func conflictsWithTheFieldsOwnActionDoNotCount() {
        let spec = KeySpec(.command, "t")
        #expect(shortcutConflict(key: spec.record, own: "newTerminal") == nil)
        #expect(ShortcutRecording.conflict(spec, own: .newTerminal, in: live) == nil)
        #expect(shortcutConflict(key: spec.record, own: nil) == "⌘T is New Terminal")
    }

    @Test func keysAreRoutedByScope() {
        #expect(keyActionFor(key: KeySpec(.command, "t").record, scope: .terminal) == "newTerminal")
        #expect(keyTerminalAction(key: KeySpec(.command, "c").record) == "copy")
        #expect(keyTerminalYields(key: KeySpec(.command, "t").record))
        for scope in [KeyScope.app, .edit, .terminal, .list, .studio] {
            for spec in live.entries.flatMap(\.keys) {
                #expect(keyActionFor(key: spec.record, scope: scope.record) == live.action(for: spec, scope: scope)?.id.rawValue)
            }
        }
    }

    @Test func actionsAreHiddenByTheirFeatureAndFlag() {
        let hidden = keymapHiddenActions(featuresOff: ["notes"], flagsOff: ["folder_picker"])
        #expect(Set(hidden).isSuperset(of: ["newNote", "insertFolderPath", "showNotes"]))
        #expect(!hidden.contains("newTerminal"))
    }

    /// What a window in front can look like: a debug build with five terminals; a split with an undo; a bare one.
    static func states() -> [MenuState] {
        var front = MenuState()
        front.off = ["toggleStatusBar", "toggleMinimap", "fullScreen", "zoomPane"]
        front.checked = ["showChanges"]
        front.terminals = ["sidebar polish", "diff perf", "zsh"]
        front.currentTerminal = 1
        front.profiles = ["Default", "Work"]
        (front.currentProfile, front.windowProfile) = (0, 0)
        front.recent = ["terminal", "web"]
        front.shells = ["zsh", "fish"]
        front.projectActions = [.init(id: "test", title: "Run tests")]
        front.debug = true
        var split = MenuState()
        split.split = true
        split.undo = "Change “primary”"
        split.persistent = true
        split.hidden = ["showHistory", "newNote"]
        split.disabled = ["revealInFinder", "copyPath"]
        return [front, split, MenuState()]
    }
}

import PlatformModel
import Testing

@Suite struct KeymapTests {
    private let keymap = Keymap.bundled

    private func spec(_ glyphs: String) throws -> KeySpec { try #require(KeySpec(glyphs: glyphs)) }

    @Test func theTableIsCoresTable() {
        // 113 actions plus the nine terminals and nine groups the loops add.
        #expect(keymap.entries.count == 131)
        #expect(keymap.entries.filter { !$0.id.rawValue.contains(".") }.count == 113)
        #expect(Set(keymap.entries.map(\.id)).count == 131, "ids are unique")
        #expect(keymap.layout.count == 8)
        #expect(keymap.systemKeys.count == 19)
        #expect(Set(keymap.entries.compactMap(\.flag)).count == 1, "the folder picker is the one flag")
    }

    @Test func rowsCarryTheirKeysAndFlags() throws {
        #expect(keymap.entry("newTerminal")?.keys.map(\.glyphText) == ["⌘T", "⇧⌘N"])
        #expect(keymap.entry("zoomPane")?.title == "Unzoom Pane" && keymap.entry("zoomPane")?.titleOff == "Zoom Pane")
        #expect(keymap.entry("insertFolderPath")?.flag == "folder_picker")
        #expect(keymap.entry("inspector")?.debugOnly == true)
        #expect(keymap.entry("quitAndEndAll")?.persistentOnly == true)
        #expect(keymap.entry("focusPaneLeft")?.scope == .terminal && keymap.entry("focusPaneLeft")?.displayName == "Select Pane on the Left")
        #expect(keymap.entry("help")?.stage == .system && keymap.entry("print")?.stage == .reserved)
        #expect(keymap.key(.terminal(3))?.glyphText == "⌘3" && keymap.key(.group(3))?.glyphText == "⌃⌥3")
        #expect(keymap.withKey("Hide sidebar", "toggleSidebar") == "Hide sidebar (⌃⌘S)")
    }

    @Test func everyKeyRoundTripsThroughItsGlyphs() throws {
        for entry in keymap.entries {
            for key in entry.keys {
                let again = try #require(KeySpec(glyphs: key.glyphText), "\(entry.id) \(key.glyphText)")
                #expect(again.sameKey(key), "\(entry.id)")
                #expect(try KeySpec(parsing: key.text) == key, "\(entry.id) \(key.text)")
            }
        }
    }

    @Test func noTwoActionsShareAKeyWhereBothApply() {
        for (index, a) in keymap.entries.enumerated() {
            for b in keymap.entries[(index + 1)...] where a.scope.overlaps(b.scope) {
                for key in a.keys { #expect(!b.keys.contains { $0.sameKey(key) }, "\(a.id) and \(b.id) share \(key.glyphText)") }
            }
        }
    }

    @Test func appKeysCarryCommandExceptF6() {
        for entry in keymap.entries where entry.scope == .app && entry.stage == .now {
            for key in entry.keys where !key.mods.contains(.command) {
                #expect(key.isFunctionKey || key.mods == [.control, .option], "\(entry.id) \(key.glyphText)")
            }
        }
    }

    @Test func aUsKeyboardsShiftedSpellingIsTheSameKey() throws {
        #expect(try spec("⌘}").sameKey(spec("⇧⌘]")))
        #expect(try spec("⌘+").sameKey(spec("⇧⌘=")))
        #expect(try !spec("⌘]").sameKey(spec("⇧⌘]")))
        #expect(keymap.action(for: try spec("⇧⌘]"), scope: .app)?.id == "nextTerminal")
    }

    @Test func theTextGrammar() throws {
        #expect(try KeySpec(parsing: " Command+Shift+R ") == KeySpec([.command, .shift], "r"))
        #expect(try KeySpec(parsing: "cmd--") == KeySpec(.command, "-"))
        #expect(try KeySpec(parsing: "cmd-+") == KeySpec(.command, "+"))
        #expect(try KeySpec(parsing: "platform-option-return") == KeySpec([.command, .option], "enter"))
        #expect(try KeySpec(parsing: "ctrl-backtick").key == "`")
        #expect(try KeySpec(parsing: "f12") == KeySpec([], "f12"))
        #expect(throws: KeySpec.ParseError.empty) { try KeySpec(parsing: "") }
        #expect(throws: KeySpec.ParseError.missingKey) { try KeySpec(parsing: "cmd-") }
        #expect(throws: KeySpec.ParseError.unknownModifier("hyper")) { try KeySpec(parsing: "hyper-t") }
        #expect(throws: KeySpec.ParseError.duplicateModifier("command")) { try KeySpec(parsing: "cmd-command-t") }
        #expect(KeySpec(glyphs: "⌘⌘T") == nil && KeySpec(glyphs: "⌘") == nil && KeySpec(glyphs: "⌘nosuchkey") == nil)
        #expect(try spec("⌥Space").glyphText == "⌥Space" && spec("⇧F6").glyphText == "⇧F6")
    }

    @Test func whoHasAKey() throws {
        #expect(keymap.uses(try spec("⌘T"))?.description == "⌘T is New Terminal")
        #expect(keymap.uses(try spec("⌘Space")) == .system("Spotlight"))
        #expect(keymap.uses(try spec("⌃R")) == .shell("a control character"))
        #expect(keymap.uses(try spec("⌥Space")) == .shell("Meta, word jumps and accents"))
        #expect(keymap.uses(try spec("⇧A")) == .shell("typing"))
        #expect(keymap.uses(try spec("⇧⌘Space")) == nil, "the suggested hotkey is free")
        #expect(keymap.uses(try spec("⌥F5")) == nil)
    }

    @Test func recordingAShortcut() throws {
        #expect(ShortcutRecording.record([], key: "escape") == .cancel)
        #expect(ShortcutRecording.record([], key: "backspace") == .clear)
        #expect(ShortcutRecording.record([], key: "a") == .needsModifier)
        #expect(ShortcutRecording.record([], key: "f5") == .shortcut(KeySpec([], "f5")))
        #expect(ShortcutRecording.record(.command, key: "escape") == .shortcut(KeySpec(.command, "escape")))
        let free = KeySpec([.shift, .command], "space")
        #expect(ShortcutRecording.conflict(free, in: keymap) == nil)
        #expect(ShortcutRecording.conflict(try spec("⌘Space"), in: keymap) == "macOS uses ⌘Space for Spotlight")
        #expect(ShortcutRecording.conflict(try spec("⌥Space"), in: keymap) == "The shell uses ⌥Space")
        #expect(ShortcutRecording.conflict(try spec("⌘T"), in: keymap)?.hasPrefix("⌘T is ") == true)
        // The jump key's own action is no conflict for the jump key.
        #expect(ShortcutRecording.conflict(try spec("⌥⌘J"), in: keymap) != nil)
        #expect(ShortcutRecording.conflict(try spec("⌥⌘J"), own: .goToWaiting, in: keymap) == nil)
    }

    @Test func backgroundModeNeedsAYes() {
        for yes in ["1", "true", "TRUE", " yes ", "on"] { #expect(PlatformPolicy.requested(yes), "\(yes)") }
        for no in ["", "0", "false", "no", "off", "2", "background"] { #expect(!PlatformPolicy.requested(no), "\(no)") }
        #expect(!PlatformPolicy.requested(nil))
        #expect(PlatformPolicy.current(environment: [:], arguments: ["Wormhole", "--background"]).background)
        #expect(PlatformPolicy.current(environment: [:], arguments: ["Wormhole"]).touchesSystem)
    }
}

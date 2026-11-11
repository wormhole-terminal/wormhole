import AppKit
import PlatformModel
import Testing

@testable import PlatformUI

/// The native menu bar built from the table: what AppKit would show and answer.
@MainActor @Suite struct AppKitMenuTests {
    private func main() -> NSMenu { AppMenuBuilder().mainMenu(for: MenuBarTests.frontWindow()) }

    private func item(_ menu: String, _ title: String) -> NSMenuItem? {
        main().items.first { $0.title == menu }?.submenu?.items.first { $0.title == title }
    }

    @Test func theBarHasTheDesignedMenusInOrder() {
        #expect(main().items.map(\.title) == ["Wormhole", "Shell", "Edit", "View", "Terminal", "Window", "Help", "Developer"])
    }

    @Test func keysAreBoundWithTheirModifiers() throws {
        let newTerminal = try #require(item("Shell", "New Terminal"))
        #expect(newTerminal.keyEquivalent == "t" && newTerminal.keyEquivalentModifierMask == .command)
        let waiting = try #require(item("Window", "Go to Waiting Terminal"))
        #expect(waiting.keyEquivalent == "j" && waiting.keyEquivalentModifierMask == [.command, .option])
        let next = try #require(item("Window", "Next Terminal"))
        #expect(next.keyEquivalent == String(UnicodeScalar(UInt32(NSDownArrowFunctionKey))!) && next.keyEquivalentModifierMask == [.command, .control])
        let f6 = try #require(item("Terminal", "Focus Next Area"))
        #expect(f6.keyEquivalent == String(UnicodeScalar(UInt32(NSF6FunctionKey))!) && f6.keyEquivalentModifierMask.isEmpty)
        let zoom = try #require(item("View", "Zoom Pane"))
        #expect(zoom.keyEquivalent == "\r" && zoom.keyEquivalentModifierMask == [.command, .shift])
        let bigger = try #require(item("View", "Bigger"))
        #expect(bigger.keyEquivalent == "+")
    }

    @Test func theSystemsActionsAreAnsweredByAppKitThroughTheResponderChain() throws {
        let quit = try #require(item("Wormhole", "Quit Wormhole"))
        #expect(quit.action == #selector(NSApplication.terminate(_:)) && quit.target == nil)
        let copy = try #require(item("Edit", "Copy"))
        #expect(copy.action == #selector(NSText.copy(_:)) && copy.target == nil)
        let minimize = try #require(item("Window", "Minimize"))
        #expect(minimize.action == #selector(NSWindow.performMiniaturize(_:)))
        #expect(item("View", "Enter Full Screen")?.action == #selector(NSWindow.toggleFullScreen(_:)))
        #expect(item("Wormhole", "Hide Wormhole")?.action == #selector(NSApplication.hide(_:)))
    }

    @Test func everythingElseGoesThroughTheOneDispatcher() throws {
        var got: [MenuCommand] = []
        let builder = AppMenuBuilder(dispatcher: MenuDispatcher { got.append($0) })
        let menu = builder.mainMenu(for: MenuBarTests.frontWindow())
        let shell = try #require(menu.items.first { $0.title == "Shell" }?.submenu)
        shell.performActionForItem(at: shell.indexOfItem(withTitle: "New Terminal"))
        let window = try #require(menu.items.first { $0.title == "Window" }?.submenu)
        window.performActionForItem(at: window.indexOfItem(withTitle: "diff perf"))
        let profile = try #require(shell.item(withTitle: "Profile")?.submenu)
        profile.performActionForItem(at: profile.indexOfItem(withTitle: "Work"))
        #expect(got == [.action("newTerminal"), .action(.terminal(2)), .switchProfile(1)])
    }

    @Test func checkedDisabledAndPlaceholderItemsKeepTheirState() throws {
        #expect(item("View", "Changes")?.state == .on)
        #expect(item("View", "History")?.state == .off)
        let recent = try #require(main().items.first { $0.title == "Shell" }?.submenu?.item(withTitle: "Open Recent")?.submenu)
        #expect(recent.items.map(\.title) == ["No Recent Folders"] && recent.items[0].isEnabled == false)
        var state = MenuBarTests.frontWindow()
        state.disabled = ["revealInFinder"]
        let disabled = AppMenuBuilder().mainMenu(for: state).items.first { $0.title == "Terminal" }?.submenu?.item(withTitle: "Reveal in Finder")
        #expect(disabled?.isEnabled == false)
    }

    @Test func theWindowAndHelpMenusAreRegisteredAndServicesIsThere() {
        let builder = AppMenuBuilder()
        var state = MenuBarTests.frontWindow()
        state.debug = false
        builder.update(state)
        #expect(NSApp.mainMenu?.items.count == 7)
        #expect(NSApp.windowsMenu?.title == "Window" && NSApp.helpMenu?.title == "Help")
        #expect(NSApp.servicesMenu != nil)
        builder.update(state)
        #expect(builder.shown == state, "the same state costs nothing")
    }

    @Test func aContextMenuIsTheSameNativeMenu() throws {
        var chosen: [MenuCommand] = []
        let presenter = ContextMenuPresenter(dispatcher: MenuDispatcher { chosen.append($0) })
        var facts = RowMenuFacts(id: 7)
        facts.cwd = "/tmp"
        facts.groups = [.init(id: 2, name: "Work")]
        let menu = presenter.menu(RowMenu.nodes(facts))
        #expect(menu.items.map(\.title).prefix(3) == ["Rename…", "Add terminal to new group", "Add terminal to group"])
        menu.performActionForItem(at: 0)
        let groups = try #require(menu.item(withTitle: "Add terminal to group")?.submenu)
        groups.performActionForItem(at: 0)
        #expect(chosen == [.context(ContextAction("row.rename", subject: 7)), .context(ContextAction("row.intoGroup", subject: 7, argument: "2"))])
    }

    @Test func statusRowsCarryTheirTimeAndSecondLine() throws {
        var snapshot = StatusSnapshot()
        snapshot.terminals = [AmbientTerminal(session: 1, name: "diff perf", state: .needsYou, since: Date().addingTimeInterval(-125), source: .agent, who: "Claude Code", message: "Approve?")]
        let dispatcher = MenuDispatcher()
        let model = StatusMenu.build(snapshot, now: Date())
        let menu = NSMenuFactory(dispatcher: dispatcher, style: MenuStyle { _ in nil }).menu(title: "", model.menu)
        let row = try #require(menu.items.first { $0.title.hasPrefix("diff perf") })
        #expect(row.attributedTitle?.string.hasSuffix("2m") == true)
        #expect(row.subtitle == "Claude Code · Approve?" && row.image != nil)
        #expect(menu.items.first?.isSectionHeader == true)
    }

    @Test func aKeyEquivalentForEveryKeyOfTheTable() {
        for entry in Keymap.bundled.entries { for key in entry.keys { #expect(KeyEquivalent(key) != nil, "\(entry.id) \(key.glyphText)") } }
    }

    @Test func keyCodesGoBothWays() throws {
        let command = try #require(KeyCodes.code(for: KeySpec(.command, "t")))
        #expect(command.code == 17 && command.modifiers == 256)
        #expect(KeyCodes.code(for: KeySpec(.command, "}"))?.code == KeyCodes.code(for: KeySpec([.command, .shift], "]"))?.code)
        #expect(KeyCodes.name(forCode: 49) == "space" && KeyCodes.name(forCode: 126) == "up" && KeyCodes.name(forCode: 97) == "f6")
        #expect(KeyCodes.code(for: KeySpec(.command, "nosuch")) == nil)
    }
}

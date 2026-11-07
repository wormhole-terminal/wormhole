// The terminal's context menu (right-click on its grid): the native menu of `TerminalMenu`'s items for what is under
// the pointer, and what choosing one does. Its items are the keymap's own, so each runs the action of the same id.

import AppKit
import PlatformModel
import PlatformUI
import TerminalMetal
import WorkspaceModel

extension AppController {
    /// The menu for terminal `id`, built as it is asked for so its facts are current.
    func terminalMenu(for id: TerminalID) -> NSMenu? {
        var facts = TerminalMenuFacts()
        facts.canPaste = NSPasteboard.general.string(forType: .string) != nil
        facts.canCopy = true
        facts.splitRight = router.table["splitRight"] != nil
        facts.splitDown = router.table["splitDown"] != nil
        hub.send(.select(id))
        facts.block = hub.contextBlock(in: id)
        return contextMenus.menu(TerminalMenu.nodes(facts))
    }

    /// A context menu's choice. Terminal menu kinds map onto actions; the others (rows, groups, paths) are the
    /// sidebar's and title row's own SwiftUI menus here, so nothing else arrives.
    func runContext(_ action: ContextAction) {
        guard let kind = TerminalMenu.Kind.allCases.first(where: { $0.key == action.kind }) else { return }
        switch kind {
        case .copy: router.run("copy")
        case .paste: NSApp.sendAction(#selector(NSText.paste(_:)), to: nil, from: nil)
        case .selectAll: NSApp.sendAction(#selector(NSText.selectAll(_:)), to: nil, from: nil)
        case .find: router.run("find")
        case .splitRight: router.run("splitRight")
        case .splitDown: router.run("splitDown")
        case .copyCommand: hub.blockAction(.copyCommand, on: action.subject.map(UInt64.init(bitPattern:)))
        case .copyOutput: hub.blockAction(.copyOutput, on: action.subject.map(UInt64.init(bitPattern:)))
        case .putCommandAtPrompt: hub.blockAction(.putAtPrompt, on: action.subject.map(UInt64.init(bitPattern:)))
        case .openLink, .copyLink, .clear:
            // Links come with the engine's marks through the FFI.
            break
        }
    }
}

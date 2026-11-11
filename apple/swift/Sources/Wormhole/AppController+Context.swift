// The terminal's context menu (right-click on its grid): the native menu of `TerminalMenu`'s items for what is under
// the pointer, and what choosing one does. Its items are the keymap's own, so each runs the action of the same id.

import AppKit
import DesignSystem
import PlatformModel
import PlatformUI
import TerminalMetal
import WorkspaceModel

extension AppController {
    /// The menu for terminal `id`, built as it is asked for so its facts are current.
    func terminalMenu(for id: TerminalID, in session: WindowSession? = nil) -> NSMenu? {
        var facts = TerminalMenuFacts()
        facts.canPaste = Pasteboard.text()?.isEmpty == false
        facts.splitRight = router.table["splitRight"] != nil
        facts.splitDown = router.table["splitDown"] != nil
        let holder = session ?? self.session(holding: id) ?? front
        holder.hub.send(.select(id))
        let view = holder.hub.terminals.existing(id)
        facts.canCopy = view?.canCopy ?? false
        facts.link = view?.linkAddressUnderPointer
        holder.hub.foldsOn = settingsModel.host.flags().contains { $0.key == "output_folds" && $0.on }
        facts.block = holder.hub.contextBlock(in: id)
        return contextMenus.menu(TerminalMenu.nodes(facts))
    }

    /// A context menu's choice. Terminal menu kinds map onto actions; the others (rows, groups, paths) are the
    /// sidebar's and title row's own SwiftUI menus here, so nothing else arrives.
    func runContext(_ action: ContextAction) {
        if let row = RowMenu.Kind.allCases.first(where: { $0.key == action.kind }) { return runRow(row, action) }
        if let group = GroupMenu.Kind.allCases.first(where: { $0.key == action.kind }) { return runGroup(group, action) }
        guard let kind = TerminalMenu.Kind.allCases.first(where: { $0.key == action.kind }) else { return }
        switch kind {
        case .copy: selectedView?.copy(nil)
        case .paste: selectedView?.paste(nil)
        // The terminal the menu is over, not the responder chain (which has no key window in a background instance).
        case .selectAll: selectedView?.selectAll(nil)
        case .find: router.run("find")
        case .splitRight: router.run("splitRight")
        case .splitDown: router.run("splitDown")
        case .copyCommand: hub.blockAction(.copyCommand, on: action.subject.map(UInt64.init(bitPattern:)))
        case .copyOutput: hub.blockAction(.copyOutput, on: action.subject.map(UInt64.init(bitPattern:)))
        case .putCommandAtPrompt: hub.blockAction(.putAtPrompt, on: action.subject.map(UInt64.init(bitPattern:)))
        case .openLink: if let address = action.argument { selectedView?.openLink(address: address) }
        case .copyLink:
            guard let address = action.argument else { return }
            selectedView?.copyLink(address: address)
        case .foldOutput: hub.foldOutput(block: action.subject.map(UInt64.init(bitPattern:)))
        case .clear: selectedView?.clearBuffer(nil)
        }
    }
}

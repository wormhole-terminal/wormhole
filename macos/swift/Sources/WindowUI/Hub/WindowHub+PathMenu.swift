// The selected terminal's folder menu for the control folder's `path-menu`: what a right-click on the folder in the title row
// shows (Open in Finder, Copy Path, Insert into Terminal, then each folder up the path), from the facts the row is drawn from.

import AppKit
import PanesUI
import WorkspaceModel

extension WindowHub {
    /// Nil when the selected terminal has no folder.
    public var selectedPathMenu: NSMenu? {
        guard let terminal = state.selectedTerminal, terminal.folder != nil else { return nil }
        // The folder's facts do not depend on the status bar; `TerminalColumn` hands `FolderCrumb` these.
        let facts = TitleFactsMaker.facts(for: terminal, row: titleRow, statusBarOn: false, extras: state.title)
        return TitleKeyMenu.pathMenu(facts: facts, actions: titleActions(for: terminal))
    }
}

// The selected terminal's folder menu for the control folder's `path-menu`: what a right-click on the folder in the title row
// offers (Open in Finder, Copy Path, Insert into Terminal, then each folder up the path), as one native menu.

import AppKit
import PanesUI

extension WindowHub {
    /// Nil when the selected terminal has no folder.
    public var selectedPathMenu: NSMenu? {
        guard let terminal = state.selectedTerminal, let folder = terminal.folder else { return nil }
        return TitleKeyMenu.pathMenu(folder: (folder as NSString).expandingTildeInPath, shown: folder, actions: titleActions(for: terminal))
    }
}

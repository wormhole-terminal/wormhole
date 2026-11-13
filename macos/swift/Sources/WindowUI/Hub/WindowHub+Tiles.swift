// A terminal's tile away from the list, for the title row and the pane headers: the row's own (the agent's logo, the
// status light and badge), as the tab bar draws it (`tab_bar.rs` `tile_for`, `terminal_tile` in the title row and pane header).

import PanesUI
import SidebarUI
import SwiftUI
import WorkspaceModel

extension WindowHub {
    /// The tile of terminal `id` at `size`; draws nothing for a terminal the window does not list.
    func tileMaker(appearance: SidebarAppearance) -> (TerminalID, CGFloat) -> TerminalStatusTile? {
        { [unowned self] id, size in
            guard let terminal = state.all.first(where: { $0.id == id }) else { return nil }
            return TerminalStatusTile(terminal: terminal, size: size, appearance: appearance)
        }
    }

    /// The pane verbs with the row's own tile in each header.
    func paneActions(appearance: SidebarAppearance) -> PaneActions {
        var actions = paneActions
        actions.tile = tileMaker(appearance: appearance)
        return actions
    }

    /// The title row's verbs with the row's own tile before the terminal's name.
    func titleActions(for terminal: TerminalSummary, appearance: SidebarAppearance) -> TitleActions {
        var title = titleActions(for: terminal)
        title.tile = tileMaker(appearance: appearance)
        return title
    }
}

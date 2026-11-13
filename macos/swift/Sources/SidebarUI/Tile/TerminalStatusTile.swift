// A terminal's tile away from the list: a tab's and a switcher card's are the row's own tile (`tab_bar.rs` `tile_for`,
// `switcher.rs`), with the agent's logo and the status light and badge, at the size the place asks for.

import DesignSystem
import SwiftUI
import WorkspaceModel

public struct TerminalStatusTile: View {
    let terminal: TerminalSummary
    let size: CGFloat
    let appearance: SidebarAppearance
    @Environment(\.tokens) private var tokens

    public init(terminal: TerminalSummary, size: CGFloat, appearance: SidebarAppearance) {
        (self.terminal, self.size, self.appearance) = (terminal, size, appearance)
    }

    public var body: some View {
        StatusTile(
            // With Status dot off every tile is the quiet one, as in the list.
            state: appearance.showsStatus ? TileState(terminal) : .shell, agent: terminal.agent, size: size,
            phase: StatusTile.phase(of: terminal.id), surface: tokens.surfacePanel)
            .environment(\.sidebarLook, appearance)
    }

    /// The terminals as a card lists their tiles: the loudest first (`TileState` is in the core's `Urgency` order), the
    /// list's order among equals.
    public static func loudestFirst(_ terminals: [TerminalSummary]) -> [TerminalSummary] {
        terminals.enumerated()
            .sorted { a, b in
                let (left, right) = (TileState(a.element).rawValue, TileState(b.element).rawValue)
                return left == right ? a.offset < b.offset : left < right
            }
            .map(\.element)
    }
}

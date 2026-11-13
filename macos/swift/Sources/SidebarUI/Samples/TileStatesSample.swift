// The list's own tile in every state, for the Theme Studio's gallery (`studio/gallery/tiles.rs` `tile_states`): a plain
// terminal's states, an agent's, and the agent's again on a selected row, where the badge is cut out in the row's colour.
// Under them one tile whose status changes when asked, played by the status motion in use (`StatusTile`'s own change and
// the flare): the Swift tile cannot be held part-way through a change, so the change is played, not caught mid-way.

import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

/// The sidebar's status tile in every state and a status change to play, for the Studio's gallery.
public struct TileStatesSpecimen: View {
    @State private var asking = false
    @State private var changes = 0
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarMetrics) private var metrics

    /// What a plain terminal's tile can say, and an agent's (`TileState`).
    private static let plain: [TileState] = [.shell, .running, .bell, .unread, .failed]
    private static let agent: [TileState] = [.shell, .working, .needsYou, .done, .failed]

    /// A specimen takes nothing: its sample is its own.
    public init() {}

    /// The sample, drawn by the app's own views.
    public var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            line(Self.plain, agent: nil, surface: tokens.surfacePanel)
            line(Self.agent, agent: .claude, surface: tokens.surfacePanel)
            line(Self.agent, agent: .codex, surface: tokens.sidebarRowSelected)
                .padding(Space._1)
                .background(tokens.sidebarRowSelected, in: RoundedRectangle(cornerRadius: metrics.rowRadius))
            change
        }
    }

    private func line(_ states: [TileState], agent: AgentID?, surface: Color) -> some View {
        HStack(spacing: Space._4) {
            ForEach(states, id: \.self) { StatusTile(state: $0, agent: agent, size: metrics.tile, phase: 0, surface: surface) }
        }
        .allowsHitTesting(false)
    }

    /// Working to needs input and back, each press one change.
    private var change: some View {
        let (from, to): (TileState, TileState) = asking ? (.working, .needsYou) : (.needsYou, .working)
        return HStack(spacing: Space._4) {
            StatusTile(state: to, agent: .claude, size: metrics.tile, phase: 0, surface: tokens.surfacePanel)
                .statusFlare(changes == 0 ? nil : StatusShift(count: changes, from: from, to: to), tile: metrics.tile)
            Button(Words.text("studio-gallery-play-status", fallback: "Play a status change")) {
                asking.toggle()
                changes += 1
            }
        }
    }
}

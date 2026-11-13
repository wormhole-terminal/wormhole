// The list's own tile in every state, for the Theme Studio's gallery (`studio/gallery/tiles.rs` `tile_states`): a plain
// terminal's states, an agent's, and the agent's again on a selected row, where the badge is cut out in the row's colour.
// Under them one tile whose status changes when asked, played by the status motion in use (`StatusTile`'s own change and
// the flare): a flare and a nudge are also held a third of the way through (the core's pose table at that time); the other variants play.

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
            caught
        }
    }

    /// The two motions that add something on top of a tile, each held a third of the way through working to needs input
    /// (`status_changes`): the ring on its way out, the row's nudge. The others (a cross-fade, a pop, a wash, a roll) are
    /// the system's transitions, which cannot be held part-way; they play above.
    private var caught: some View {
        HStack(spacing: Space._6) {
            ForEach([StatusMotion.flare, .nudge], id: \.self) { motion in
                VStack(spacing: Space._2) {
                    StatusTile(state: .needsYou, agent: .claude, size: metrics.tile, phase: 0, surface: tokens.surfacePanel)
                        .statusCaught(
                            StatusShift(count: 1, from: .working, to: .needsYou), variant: motion,
                            tile: motion == .flare ? metrics.tile : nil, at: 1.0 / 3.0)
                    Text(motion.coreName).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                }
            }
        }
        .padding(Space._3)
        .allowsHitTesting(false)
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

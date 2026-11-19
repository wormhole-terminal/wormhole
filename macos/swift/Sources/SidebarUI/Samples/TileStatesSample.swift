// The list's own tile in every state, for the Theme Studio's gallery (`studio/gallery/tiles.rs` `tile_states`): a plain
// terminal's states, an agent's, and the agent's again on a selected row, where the badge is cut out in the row's colour.
// Under them one tile whose status changes when asked, played by the status motion in use (`StatusTile`'s own change and
// the flare), and every variant but Classic (a cut) held a third of the way from working to needs input (`status_changes`):
// each drawn from the core's pose table at that time (`pose::status`), part by part as the row has them: the old light and
// badge on their way out, the new ones on their way in, the card's wash, the ring of a flare, a nudge, the line's roll.

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

    /// The change from working to needs input a third of the way through each variant (Classic is a cut: nothing to catch).
    /// A flare and a nudge add something to the tile already changed; the others are its parts and its card as the pose has them.
    private var caught: some View {
        let shift = StatusShift(count: 1, from: .working, to: .needsYou)
        return HStack(alignment: .top, spacing: Space._6) {
            ForEach([StatusMotion.crossfade, .badgePop, .wash, .lineRoll, .flare, .nudge], id: \.self) { motion in
                VStack(spacing: Space._2) {
                    switch motion {
                    case .flare, .nudge:
                        StatusTile(state: .needsYou, agent: .claude, size: metrics.tile, phase: 0, surface: tokens.surfacePanel)
                            .statusCaught(shift, variant: motion, tile: motion == .flare ? metrics.tile : nil, at: Self.third)
                            .padding(Space._2)
                    default:
                        posed(Self.pose(motion, shift))
                    }
                    if motion == .lineRoll { rolled(Self.pose(motion, shift).roll) }
                    Text(motion.coreName).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                }
            }
        }
        .padding(Space._3)
        .allowsHitTesting(false)
    }

    /// Where every specimen of a change is held: a third of the way (`ShiftClock::At(1.0 / 3.0)`).
    private static let third = 1.0 / 3.0
    /// How strong a wash is at its fullest (`WASH_TINT`, as the row's `StatusWash`).
    private static let washTint = 0.16

    private static func pose(_ motion: StatusMotion, _ shift: StatusShift) -> StatusPose {
        RowMotion.tables?.statusPose(motion.coreName, shift.from.rawValue, shift.to.rawValue, third) ?? .rest
    }

    /// The tile caught in `pose`: the working light as far as it still shows, the needs-input edge and badge as far as they have
    /// come (the badge at its size of the moment), and the card behind washed in the attention colour as strong as the pose says.
    private func posed(_ pose: StatusPose) -> some View {
        let size = metrics.tile
        let surface = tokens.surfacePanel
        return StatusTile(state: .shell, agent: .claude, size: size, phase: 0, surface: surface)
            .overlay { TileLight(state: .working, size: size, phase: 0).opacity(pose.leaving) }
            .overlay { TileLight(state: .needsYou, size: size, phase: 0).opacity(pose.ring) }
            .overlay(alignment: .bottomTrailing) {
                TileBadge(state: .needsYou, tile: size, surface: surface).scaleEffect(CGFloat(pose.badgeScale)).opacity(pose.badgeOpacity)
            }
            .padding(Space._2)
            .background(tokens.agentAttention.opacity(Self.washTint * pose.wash), in: RoundedRectangle(cornerRadius: metrics.rowRadius))
    }

    /// The second line `roll` of the way from the old words to the new: the old ones going up out of the line, the new ones
    /// coming in from below (`SecondLineView`'s roll).
    private func rolled(_ roll: Double) -> some View {
        let height = RowGeometry.metaLine
        return ZStack(alignment: .leading) {
            Text(Words.text("settings-menu-bar-preview-working", fallback: "Working")).foregroundStyle(tokens.textMuted)
                .offset(y: -height * CGFloat(roll))
            Text(Words.text("settings-menu-bar-preview-needs-input", fallback: "Needs input")).foregroundStyle(tokens.agentAttentionText)
                .offset(y: height * CGFloat(1 - roll))
        }
        .font(TextRole.caption.font)
        .frame(height: height)
        .clipped()
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

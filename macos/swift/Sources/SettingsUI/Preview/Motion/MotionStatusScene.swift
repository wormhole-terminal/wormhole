// A terminal's status changing (`motion_preview/status.rs`): one row above a quiet one goes from idle to working, to
// asking for input, to done and back to idle, as the chosen `motion.status` plays each change. The row is drawn from the
// pose the core gives (`pose::status`) at the time the loop says: the old badge leaving and the new one coming, the flare's
// ring, the card's wash, the row's nudge and the second line's roll. Classic is a cut.

import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

struct MotionStatusScene: View {
    let tokens: ThemeTokens
    let variant: String
    let t: Double

    /// A status the row goes through: its place in the core's urgency order (needs you 0, working 2, done 6, idle 7), its
    /// second line and its colour (none while idle).
    private struct Status {
        let urgency: Int
        let detail: String
        let color: KeyPath<ThemeTokens, Color>?
    }

    private static var states: [Status] {
        [
            Status(urgency: 7, detail: MotionSample.idleDetail, color: nil),
            Status(urgency: 2, detail: "\(Words.text("settings-menu-bar-preview-working", fallback: "Working")) · 4m", color: \.agentWorking),
            Status(urgency: 0, detail: "\(Words.text("settings-menu-bar-preview-needs-input", fallback: "Needs input")) · 2m", color: \.agentAttention),
            Status(urgency: 6, detail: "\(Words.text("settings-menu-bar-preview-finished", fallback: "Finished")) · 1m", color: \.agentDone),
        ]
    }
    /// When each change begins, as shares of the loop: to working, to asking, to done, and back to idle (`BEGINS`).
    private static let begins = [0.1, 0.35, 0.6, 0.85]
    /// How strong a wash is at its fullest (`WASH_TINT`, as the sidebar's).
    private static let washTint = 0.16

    var body: some View {
        let height = MotionRow<AgentTile>.regularHeight
        let states = Self.states
        let playing = change(states)
        let pose = playing.pose
        let now = states[playing.now]
        let was = playing.from.map { states[$0] }
        MotionStage(tokens: tokens) {
            MotionRow(
                tokens: tokens, name: MotionSample.changing, detail: now.detail, detailColor: textColor(now),
                rolling: was.map { (from: $0.detail, color: textColor($0), roll: pose.roll) }
            ) { tile(pose, now: now, was: was) }
            .background {
                if let color = now.color, pose.wash > 0 {
                    RoundedRectangle(cornerRadius: Radius.row).fill(tokens[keyPath: color].opacity(Self.washTint * pose.wash))
                }
            }
            .offset(x: CGFloat(pose.dx))
            .spot(top: 0, height: height)
            MotionRow(tokens: tokens, name: MotionSample.running, detail: MotionSample.runningDetail) {
                AgentTile(tokens: tokens, size: MotionRow<AgentTile>.regularTile)
            }
            .spot(top: height, height: height)
        }
    }

    /// A second line asking for input is in the attention colour; any other is quiet.
    private func textColor(_ status: Status) -> Color? { status.urgency == 0 ? tokens.agentAttentionText : nil }

    /// The tile: the old status's dot as far as it still shows, the new one's as far as it has come, and the flare's ring.
    private func tile(_ pose: StatusPose, now: Status, was: Status?) -> some View {
        let size = MotionRow<AgentTile>.regularTile
        return AgentTile(tokens: tokens, size: size)
            .overlay(alignment: .bottomTrailing) {
                if let color = was?.color { AgentTile.dot(tokens[keyPath: color], tile: size).opacity(pose.leaving) }
            }
            .overlay(alignment: .bottomTrailing) {
                if let color = now.color { AgentTile.dot(tokens[keyPath: color], tile: size).scaleEffect(CGFloat(pose.badgeScale)).opacity(pose.badgeOpacity) }
            }
            .overlay {
                // The ring leaves the tile's edge: as thick as it reaches far, and fading as it goes.
                if pose.flareReach > 0, pose.flareStrength > 0 {
                    let reach = CGFloat(pose.flareReach)
                    RoundedRectangle(cornerRadius: size * AgentTile.corner + reach)
                        .strokeBorder(tokens.agentAttentionRing.opacity(pose.flareStrength), lineWidth: reach)
                        .frame(width: size + 2 * reach, height: size + 2 * reach)
                }
            }
    }

    /// The status the row is in at `t` and, while the change into it plays, what it came from and how the change stands.
    private func change(_ states: [Status]) -> (now: Int, from: Int?, pose: StatusPose) {
        let changes = Self.begins.filter { t >= $0 }.count
        let now = changes % states.count
        guard changes > 0, let tables = RowMotion.tables else { return (now, nil, .rest) }
        let from = (now + states.count - 1) % states.count
        let total = tables.statusSeconds(variant, states[from].urgency, states[now].urgency)
        let progress = MotionLoop.progress(MotionLoop.since(t, Self.begins[changes - 1]), of: total)
        guard progress < 1 else { return (now, nil, .rest) }
        return (now, from, tables.statusPose(variant, states[from].urgency, states[now].urgency, progress))
    }
}

// A group folding and opening (`motion_preview/fold.rs`): the group's head, its two terminals and a terminal under the
// group, drawn from the poses of the chosen `motion.fold` (core's `pose::fold`). The stage is as tall as the group open:
// folded, the rows' room is left empty for as long as it lasts, which is the point of the scene.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct MotionFoldScene: View {
    let tokens: ThemeTokens
    let variant: String
    let t: Double

    /// When the group folds and when it opens, as shares of the loop (`FOLDS`, `OPENS`), and its rows.
    private static let folds = 0.15
    private static let opens = 0.6
    private static let rows = 2

    private struct Placed {
        var rows = [RowPose.rest, .rest]
        /// How much of the rows' room is open to what is under them.
        var open: CGFloat = 1
        /// How far the folded head's summary has faded in; nil while the group is open.
        var summary: Double?
    }

    var body: some View {
        let height = MotionRow<AgentTile>.denseHeight
        let placed = placed
        let second = height * (1 + CGFloat(placed.rows[0].clip))
        MotionStage(tokens: tokens) {
            head(summary: placed.summary).spot(top: 0, height: height)
            // Each row follows the one above it as far as that is open; the terminal under the group follows them all.
            MotionRow(tokens: tokens, name: MotionSample.working, badge: tokens.agentWorking).spot(top: height, height: height, pose: placed.rows[0])
            MotionRow(tokens: tokens, name: MotionSample.asking, badge: tokens.agentAttention).spot(top: second, height: height, pose: placed.rows[1])
            MotionRow(tokens: tokens, name: MotionSample.idle).spot(top: height * (1 + CGFloat(Self.rows) * placed.open), height: height)
        }
    }

    /// The head: its chevron open until the fold starts and again once it opens; folded it also says what the group holds,
    /// which some folds wait for the rows to be gone to show.
    private func head(summary: Double?) -> some View {
        HStack(spacing: Space._1) {
            Image(systemName: summary == nil ? "chevron.down" : "chevron.right").font(.system(size: 9, weight: .semibold)).foregroundStyle(tokens.textMuted)
            Text(MotionSample.group).font(.system(size: 12, weight: .semibold)).foregroundStyle(tokens.textPrimary)
            Spacer(minLength: 0)
            if let summary { Text(String(Self.rows)).font(.system(size: 11)).foregroundStyle(tokens.textMuted).opacity(summary) }
        }
        .padding(.horizontal, Space._2)
        .frame(height: MotionRow<AgentTile>.denseHeight)
    }

    private var placed: Placed {
        guard let tables = RowMotion.tables, t >= Self.folds else { return Placed() }
        let entering = t >= Self.opens
        let elapsed = MotionLoop.since(t, entering ? Self.opens : Self.folds)
        let progress = MotionLoop.progress(elapsed, of: tables.foldPlan(variant, entering, Self.rows).seconds)
        return Placed(
            rows: (0..<Self.rows).map { tables.foldPose(variant, entering, progress, $0, Self.rows) },
            open: CGFloat(tables.foldRoom(variant, entering, Self.rows, elapsed)),
            summary: entering ? nil : tables.foldSummary(variant, Self.rows, elapsed))
    }
}

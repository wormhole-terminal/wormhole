// A terminal opening and closing (`motion_preview/insert.rs`): two terminals, a third coming in at the top and going
// again, drawn from the poses of the chosen `motion.insert` (core's `pose::insert`). The rows under the new one make room
// as the motion says: they follow its height, or slide on the motion's spring.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct MotionInsertScene: View {
    let tokens: ThemeTokens
    let variant: String
    let t: Double

    /// When the new terminal opens and when it closes, as shares of the loop (`OPENS`, `CLOSES`).
    private static let opens = 0.15
    private static let closes = 0.6

    var body: some View {
        let height = MotionRow<AgentTile>.denseHeight
        let (pose, room) = place
        MotionStage(tokens: tokens) {
            // The new row first: the rows under it cover it as they slide, as they do in the list.
            MotionRow(tokens: tokens, name: MotionSample.idle).spot(top: 0, height: height, pose: pose)
            MotionRow(tokens: tokens, name: MotionSample.working, badge: tokens.agentWorking).spot(top: height * room, height: height)
            MotionRow(tokens: tokens, name: MotionSample.asking, badge: tokens.agentAttention).spot(top: height * (1 + room), height: height)
        }
    }

    /// The new terminal at `t`, and how much of its place is open to the rows under it. Without the core's tables the picture
    /// is the one before anything moves.
    private var place: (pose: RowPose, room: CGFloat) {
        guard let tables = RowMotion.tables else { return (RowPose(opacity: 0, dx: 0, dy: 0, clip: 0), 0) }
        let entering = t < Self.closes
        let elapsed = MotionLoop.since(t, entering ? Self.opens : Self.closes)
        let pose = tables.insertPose(variant, entering, MotionLoop.progress(elapsed, of: tables.insertPlan(variant, entering).seconds))
        return (pose, CGFloat(tables.insertRoom(variant, entering, elapsed)))
    }
}

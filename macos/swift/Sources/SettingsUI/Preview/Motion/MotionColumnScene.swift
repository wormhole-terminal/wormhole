// The sidebar hiding and showing (`motion_preview/column.rs`): a miniature window with the sidebar's rows and a terminal
// beside them, drawn from the pose of the chosen `motion.column` (core's `pose::column`). The column slides over the
// terminal and the terminal moves with its edge; it is moved, never squeezed. Content after fades the rows out before the
// column goes and in after it has begun to come, over a surface that stays.

import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

struct MotionColumnScene: View {
    let tokens: ThemeTokens
    let variant: String
    let t: Double

    /// When the column hides and when it shows again, as shares of the loop (`HIDES`, `SHOWS`).
    private static let hides = 0.15
    private static let shows = 0.55
    /// The share of the miniature window the sidebar takes, and its width at most (`SIDEBAR_SHARE`, `SIDEBAR_W`).
    private static let sidebarShare: CGFloat = 0.5
    private static let sidebarMax: CGFloat = 200

    var body: some View {
        let pose = pose
        GeometryReader { proxy in
            let width = min(proxy.size.width * Self.sidebarShare, Self.sidebarMax)
            ZStack(alignment: .topLeading) {
                Text(Words.text("settings-motion-column-terminal", fallback: "Terminal")).textRole(.caption).foregroundStyle(tokens.textMuted)
                    .padding(Space._3).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                    .background(tokens.terminal.background.color)
                    .offset(x: width * CGFloat(pose.shown))
                VStack(spacing: 0) {
                    MotionRow(tokens: tokens, name: MotionSample.working, badge: tokens.agentWorking)
                    MotionRow(tokens: tokens, name: MotionSample.asking, badge: tokens.agentAttention)
                }
                // What is in the column fades; the surface under it stays.
                .opacity(pose.content)
                .frame(width: width).frame(maxHeight: .infinity, alignment: .top)
                .background(tokens.sidebarBackground)
                .offset(x: -width * CGFloat(1 - pose.shown))
            }
            .clipped()
        }
    }

    /// The column at `t`: in until it hides, going until it is out, out until it shows, coming until it is in.
    private var pose: (shown: Double, content: Double) {
        guard let tables = RowMotion.tables else { return (1, 1) }
        return t < Self.shows
            ? tables.columnPose(variant, false, MotionLoop.since(t, Self.hides)) : tables.columnPose(variant, true, MotionLoop.since(t, Self.shows))
    }
}

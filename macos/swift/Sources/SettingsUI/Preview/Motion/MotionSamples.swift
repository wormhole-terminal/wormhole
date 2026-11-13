// What the Motion scenes are made of: the sidebar's row in miniature (the Terminals list scene's), the surface a scene
// stands on, and the spot a part is put on (`motion_preview/mod.rs` `Spot`).

import DesignSystem
import SwiftUI
import WorkspaceModel

/// A terminal's row at the list's smallest density, or with a second line (`detail`) at its Regular one.
struct MotionRow<Tile: View>: View {
    let tokens: ThemeTokens
    let name: String
    var detail: String?
    var detailColor: Color?
    /// The second line it had before, rolling up and away as the new one comes (`roll`: 0 the old words, 1 the new).
    var rolling: (from: String, color: Color?, roll: Double)?
    @ViewBuilder let tile: Tile

    /// The rows' heights at Dense and at Regular, and their tiles, as the Terminals list scene draws them.
    static var denseHeight: CGFloat { 26 }
    static var regularHeight: CGFloat { Metric.rowTwoLine + 4 }
    static var denseTile: CGFloat { Metric.tileSmall }
    static var regularTile: CGFloat { Metric.tile }

    var body: some View {
        HStack(spacing: Space._2) {
            tile
            VStack(alignment: .leading, spacing: 0) {
                Text(name).font(.system(size: 12)).lineLimit(1).foregroundStyle(tokens.textPrimary)
                if let detail {
                    let roll = rolling?.roll ?? 1
                    ZStack(alignment: .topLeading) {
                        if let rolling {
                            line(rolling.from, rolling.color).visualEffect { content, proxy in content.offset(y: -roll * proxy.size.height).opacity(1 - roll) }
                        }
                        line(detail, detailColor).visualEffect { content, proxy in content.offset(y: (1 - roll) * proxy.size.height).opacity(roll) }
                    }
                    .clipped()
                }
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, Space._2)
        .frame(height: detail == nil ? Self.denseHeight : Self.regularHeight)
    }

    private func line(_ words: String, _ color: Color?) -> some View {
        Text(words).font(.system(size: 11)).lineLimit(1).foregroundStyle(color ?? tokens.textMuted)
    }
}

extension MotionRow where Tile == AgentTile {
    /// A dense row with the plain tile and the dot of its state.
    init(tokens: ThemeTokens, name: String, badge: Color? = nil) {
        self.init(tokens: tokens, name: name) { AgentTile(tokens: tokens, size: Self.denseTile, badge: badge) }
    }
}

/// The made-up terminals of the scenes (`samples.rs`).
enum MotionSample {
    static let working = "Sidebar cards"
    static let asking = "Fix pile fan-out"
    static let idle = "scratch"
    static let running = "cargo test"
    static let runningDetail = "cargo test --workspace"
    static let changing = "Rate limits"
    static let group = "api"
    static let idleDetail = "~"
}

/// The surface a scene stands on: the sidebar's, as tall and wide as its cell, with what does not fit cut away.
struct MotionStage<Parts: View>: View {
    let tokens: ThemeTokens
    @ViewBuilder let parts: Parts

    var body: some View {
        ZStack(alignment: .topLeading) { parts }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .background(tokens.sidebarBackground)
            .clipped()
    }
}

extension View {
    /// Puts a part of a scene `top` points down its stage, `height` tall, as `pose` says: moved, faded and cut from the bottom
    /// (cut, not squeezed: the part keeps the height it has whole).
    func spot(top: CGFloat, height: CGFloat, pose: RowPose = .rest) -> some View {
        frame(height: height, alignment: .top)
            .frame(height: max(0, height * CGFloat(pose.clip)), alignment: .top)
            .clipped()
            .opacity(pose.opacity)
            .offset(x: CGFloat(pose.dx), y: top + CGFloat(pose.dy))
    }
}

// A terminal's status tile in a scene (`TILE`, `TILE_MINI`): the agent's mark on the tile colour, with the
// small status dot of its state at the corner when it has one.

import DesignSystem
import SwiftUI

struct AgentTile: View {
    let tokens: ThemeTokens
    let size: CGFloat
    var badge: Color?

    /// The tile's corner as a share of its size.
    static let corner: CGFloat = 0.28

    var body: some View {
        RoundedRectangle(cornerRadius: size * Self.corner).fill(tokens.tileBackground)
            .frame(width: size, height: size)
            .overlay(Image(systemName: "asterisk").font(.system(size: size * 0.5, weight: .bold)).foregroundStyle(tokens.tileMark))
            .overlay(alignment: .bottomTrailing) {
                if let badge { Self.dot(badge, tile: size) }
            }
    }

    /// The status dot at the corner of a tile `size` wide, for a scene that draws it by itself (fading, growing).
    static func dot(_ color: Color, tile size: CGFloat) -> some View {
        Circle().fill(color).frame(width: size * 0.34, height: size * 0.34).offset(x: 2, y: 2)
    }
}

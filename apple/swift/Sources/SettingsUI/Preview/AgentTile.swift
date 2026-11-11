// A terminal's status tile in a scene (`TILE`, `TILE_MINI`): the agent's mark on the tile colour, with the
// small status dot of its state at the corner when it has one.

import DesignSystem
import SwiftUI

struct AgentTile: View {
    let tokens: ThemeTokens
    let size: CGFloat
    var badge: Color?

    var body: some View {
        RoundedRectangle(cornerRadius: size * 0.28).fill(tokens.tileBackground)
            .frame(width: size, height: size)
            .overlay(Image(systemName: "asterisk").font(.system(size: size * 0.5, weight: .bold)).foregroundStyle(tokens.tileMark))
            .overlay(alignment: .bottomTrailing) {
                if let badge { Circle().fill(badge).frame(width: size * 0.34, height: size * 0.34).offset(x: 2, y: 2) }
            }
    }
}

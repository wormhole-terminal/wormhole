// The badge in a tile's bottom-right corner for the states the edge doesn't tell: needs input (!), finished (✓),
// failed (×), the bell and unseen output (a plain dot). It is cut out of the tile by a rim in the surface's colour, and it
// grows in with a spring when the state arrives (`style/tile/badge.rs`).

import DesignSystem
import SwiftUI

struct TileBadge: View {
    let state: TileState
    let tile: CGFloat
    /// What lies under the tile, which the rim cuts the badge out in.
    let surface: Color
    @Environment(\.tokens) private var tokens

    var body: some View {
        if let fill {
            let whole = TileGeometry.badgeSize(tile)
            // A dot for new output is smaller than the other badges, with nothing inside.
            let size = state == .unread ? whole - TileGeometry.rim * 2 : whole
            Circle().fill(fill)
                .overlay { markView(whole - TileGeometry.rim * 2 - 2) }
                .overlay { Circle().strokeBorder(surface, lineWidth: TileGeometry.rim) }
                .frame(width: size, height: size)
                // It hangs `hang` past the corner when whole, and about the same middle when smaller.
                .offset(x: TileGeometry.hang - (whole - size) / 2, y: TileGeometry.hang - (whole - size) / 2)
        }
    }

    private var fill: Color? {
        switch state {
        case .needsYou: tokens.agentAttention
        case .done: tokens.agentDone
        case .failed: tokens.statusDanger
        case .bell: tokens.agentBell
        case .unread: tokens.primary
        case .working, .running, .shell: nil
        }
    }

    @ViewBuilder private func markView(_ size: CGFloat) -> some View {
        switch state {
        case .needsYou: NeedsYouMark(size: size).foregroundStyle(surface)
        case .done: GlyphView(glyph: .check, size: size).foregroundStyle(surface)
        case .failed: GlyphView(glyph: .failed, size: size).foregroundStyle(surface)
        case .bell: GlyphView(glyph: .bell, size: size).foregroundStyle(surface)
        default: EmptyView()
        }
    }
}

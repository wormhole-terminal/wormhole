// A terminal's tile: the terminal symbol in a rounded square drawn like an app's icon, black unless the theme says
// otherwise, with a faint sheen across its top and a hairline edge; its state round it (a light, an amber edge)
// and in its corner (a badge). A port of `style::status_tile` (`style/tile/`). The model names no agent, so the
// face is always the terminal; an agent's own colour and logo join when the model carries one.

import DesignSystem
import SwiftUI

struct StatusTile: View {
    let state: TileState
    let size: CGFloat
    let phase: Double
    /// What lies under the tile: the selected row's tint, or the sidebar's own surface.
    let surface: Color
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: size * TileGeometry.corner)
        shape.fill(tokens.tileBackground)
            .overlay {
                shape.fill(LinearGradient(
                    stops: [.init(color: tokens.tileSheen, location: 0), .init(color: tokens.tileSheen.opacity(0), location: TileGeometry.sheenReach)],
                    startPoint: .top, endPoint: .bottom))
            }
            .overlay { shape.strokeBorder(tokens.tileEdge, lineWidth: tokens.metrics.borderWidth) }
            .overlay {
                GlyphView(glyph: .terminal, size: size * TileGeometry.face).foregroundStyle(tokens.tileMark)
            }
            .overlay { TileLight(state: state, size: size, phase: phase) }
            .overlay(alignment: .bottomTrailing) {
                TileBadge(state: state, tile: size, surface: surface)
                    .id(state)
                    .transition(.scale(scale: 0.01).combined(with: .opacity))
            }
            .frame(width: size, height: size)
            .animation(SidebarMotion.badge, value: state)
            .accessibilityHidden(true)
    }

    /// Where in its turn a terminal's light is: its own, from its id, the same at every size.
    static func phase(of id: Int) -> Double { Double((id &* 2_654_435_761) & 0x7fff_ffff % 1000) / 1000 }
}

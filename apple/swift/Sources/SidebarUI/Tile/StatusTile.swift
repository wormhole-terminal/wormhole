// A terminal's tile: what it is (an agent's logo or letters on the agent's own colour, else the terminal symbol on
// black) in a rounded square drawn like an app's icon, with a faint sheen across its top and a hairline edge; its state
// round it (a light, an amber edge) and in its corner (a badge). A port of `style::status_tile` (`style/tile/`).

import DesignSystem
import SwiftUI
import WorkspaceModel

struct StatusTile: View {
    let state: TileState
    /// The agent in the terminal; nil is a plain terminal.
    let agent: AgentID?
    let size: CGFloat
    let phase: Double
    /// What lies under the tile: the selected row's tint, or the sidebar's own surface.
    let surface: Color
    @Environment(\.tokens) private var tokens
    @Environment(\.sidebarLook) private var look
    @Environment(\.reduceMotion) private var reduceMotion

    /// The monogram's letters as a share of the tile (`MONOGRAM_SHARE`).
    private static let monogram: CGFloat = 0.36

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: size * TileGeometry.corner)
        (agent?.tilePaint ?? .black).shape(shape, black: tokens.tileBackground)
            .overlay {
                shape.fill(LinearGradient(
                    stops: [.init(color: tokens.tileSheen, location: 0), .init(color: tokens.tileSheen.opacity(0), location: TileGeometry.sheenReach)],
                    startPoint: .top, endPoint: .bottom))
            }
            .overlay { shape.strokeBorder(tokens.tileEdge, lineWidth: tokens.metrics.borderWidth) }
            .overlay { face }
            .overlay { TileLight(state: state, size: size, phase: phase).id(state).transition(.opacity) }
            .overlay(alignment: .bottomTrailing) {
                TileBadge(state: state, tile: size, surface: surface)
                    .id(state)
                    .transition(grows ? .scale(scale: 0.01).combined(with: .opacity) : .opacity)
            }
            .frame(width: size, height: size)
            .animation(animation, value: state)
            .accessibilityHidden(true)
    }

    /// The badge grows in on a spring under "Badge pops"; every other way, and Reduce Motion, it fades (`StatusMotion`).
    private var grows: Bool { look.statusMotion == .badgePop && !reduceMotion }

    private var animation: Animation? {
        switch look.statusMotion {
        case .classic: nil
        case .badgePop where !reduceMotion: SidebarMotion.badge
        default: SidebarMotion.crossfade
        }
    }

    @ViewBuilder private var face: some View {
        let inside = size * TileGeometry.face
        if let agent {
            if AgentLogo.key(agent) != nil {
                AgentLogo(agent: agent, size: inside).foregroundStyle(tokens.tileMarkBrand)
            } else {
                Text(agent.monogram).font(.system(size: size * Self.monogram, weight: .bold)).foregroundStyle(tokens.tileMarkBrand)
            }
        } else {
            GlyphView(glyph: .terminal, size: inside).foregroundStyle(tokens.tileMark)
        }
    }

    /// Where in its turn a terminal's light is: its own, from its id, the same at every size.
    static func phase(of id: Int) -> Double { Double((id &* 2_654_435_761) & 0x7fff_ffff % 1000) / 1000 }
}

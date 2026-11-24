// The light that circles a working tile's edge and the amber edge that breathes while a terminal waits for you
// (`style/tile/orbit.rs`, `edge.rs`). Both sit just outside the tile and run off the clock, shifted by the
// terminal's own phase so a list of tiles is not in step; Core Animation plays them (`TileLightLayers`). Still, under
// Reduce Motion: the light is the whole ring at full strength and the edge holds.

import DesignSystem
import SwiftUI

struct TileLight: View {
    let state: TileState
    let size: CGFloat
    /// A share of a turn, the terminal's own, so its tiles of every size turn together.
    let phase: Double
    @Environment(\.tokens) private var tokens
    @Environment(\.reduceMotion) private var reduceMotion

    private var outer: CGFloat { size + 2 * TileGeometry.ringOut }
    private var round: CGFloat { size * TileGeometry.corner + TileGeometry.ringOut }

    var body: some View {
        switch state {
        case .working: orbit(tokens.agentWorking)
        case .needsYou: breathing(tokens.agentAttention)
        default: EmptyView()
        }
    }

    @ViewBuilder private func orbit(_ color: Color) -> some View {
        let stroke = TileGeometry.line(size)
        Group {
            if reduceMotion {
                // The whole ring at full strength.
                Path(TileLightLayers.ring(in: CGRect(x: 0, y: 0, width: outer, height: outer), round: round, stroke: stroke))
                    .stroke(color, style: StrokeStyle(lineWidth: stroke, lineJoin: .round))
            } else {
                LayerDrawing(key: Light(color: color, round: round, stroke: stroke, phase: phase)) {
                    TileLightLayers.orbit(in: $0, round: round, stroke: stroke, color: color, phase: phase)
                }
            }
        }
        .frame(width: outer, height: outer)
        .frame(width: size, height: size)
        .allowsHitTesting(false)
    }

    @ViewBuilder private func breathing(_ color: Color) -> some View {
        let stroke = TileGeometry.line(size)
        Group {
            if reduceMotion {
                RoundedRectangle(cornerRadius: round).strokeBorder(color, lineWidth: stroke)
            } else {
                LayerDrawing(key: Light(color: color, round: round, stroke: stroke, phase: phase)) {
                    TileLightLayers.breathing(in: $0, round: round, stroke: stroke, color: color, phase: phase)
                }
            }
        }
        .frame(width: outer, height: outer)
        .frame(width: size, height: size)
        .allowsHitTesting(false)
    }

    /// What a light's layers are made from.
    private struct Light: Hashable {
        let color: Color
        let round: CGFloat
        let stroke: CGFloat
        let phase: Double
    }
}

// The light that circles a working tile's edge and the amber edge that breathes while a terminal waits for you
// (`style/tile/orbit.rs`, `edge.rs`). Both sit just outside the tile and run off the clock, shifted by the
// terminal's own phase so a list of tiles is not in step. Still, under Reduce Motion: the light is the whole ring
// at full strength and the edge holds.

import DesignSystem
import SwiftUI

struct TileLight: View {
    let state: TileState
    let size: CGFloat
    /// A share of a turn, the terminal's own, so its tiles of every size turn together.
    let phase: Double
    @Environment(\.tokens) private var tokens
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    private var outer: CGFloat { size + 2 * TileGeometry.ringOut }
    private var round: CGFloat { size * TileGeometry.corner + TileGeometry.ringOut }

    var body: some View {
        switch state {
        case .working: orbit(tokens.agentWorking)
        case .needsYou: breathing(tokens.agentAttention)
        default: EmptyView()
        }
    }

    private func orbit(_ color: Color) -> some View {
        let stroke = TileGeometry.line(size)
        return TimelineView(.animation(minimumInterval: 1 / 60, paused: reduceMotion)) { context in
            Canvas { canvas, area in
                let turn = reduceMotion ? nil : fract(context.date.timeIntervalSinceReferenceDate / SidebarMotion.orbitTurn + phase)
                OrbitLight.draw(in: &canvas, area: area, round: round, stroke: stroke, color: color, turn: turn)
            }
        }
        .frame(width: outer, height: outer)
        .frame(width: size, height: size)
        .allowsHitTesting(false)
    }

    private func breathing(_ color: Color) -> some View {
        let edge = RoundedRectangle(cornerRadius: round).strokeBorder(color, lineWidth: TileGeometry.line(size))
        return TimelineView(.animation(minimumInterval: 1 / 30, paused: reduceMotion)) { context in
            let t = fract(context.date.timeIntervalSinceReferenceDate / SidebarMotion.breathe + phase)
            let least = SidebarMotion.breatheLeast
            edge.opacity(reduceMotion ? 1 : least + (1 - least) * (sin(t * 2 * .pi) * 0.5 + 0.5))
        }
        .frame(width: outer, height: outer)
        .frame(width: size, height: size)
        .allowsHitTesting(false)
    }

    private func fract(_ x: Double) -> Double { x - x.rounded(.down) }
}

/// The ring of the light: a faint track and a tail of steps brightening toward the head, along the rounded
/// rectangle's edge from the top's middle, clockwise.
enum OrbitLight {
    static func draw(in canvas: inout GraphicsContext, area: CGSize, round: CGFloat, stroke: CGFloat, color: Color, turn: Double?) {
        let inset = stroke / 2
        let (w, h, r) = (area.width - stroke, area.height - stroke, max(round - inset, 0))
        func point(_ s: Double) -> CGPoint {
            let p = perimeterPoint(w: w, h: h, r: r, s: s)
            return CGPoint(x: p.x + inset, y: p.y + inset)
        }
        func arc(_ from: Double, _ to: Double, alpha: Double) {
            var path = Path()
            let count = max(Int((to - from) * 96), 1)
            path.move(to: point(from))
            for step in 1...count { path.addLine(to: point(from + (to - from) * Double(step) / Double(count))) }
            canvas.stroke(path, with: .color(color.opacity(alpha)), style: StrokeStyle(lineWidth: stroke, lineJoin: .round))
        }
        guard let turn else { return arc(0, 1, alpha: 1) }
        arc(0, 1, alpha: TileGeometry.track)
        let tail = turn - TileGeometry.span
        for step in 0..<TileGeometry.steps {
            let from = tail + TileGeometry.span * Double(step) / Double(TileGeometry.steps)
            let to = tail + TileGeometry.span * Double(step + 1) / Double(TileGeometry.steps)
            let alpha = Double(step + 1) / Double(TileGeometry.steps)
            arc(from, to, alpha: alpha * alpha)
        }
    }

    /// The point `s` of the way round a `w`×`h` rectangle with corners of radius `r`; `s` wraps.
    static func perimeterPoint(w: CGFloat, h: CGFloat, r rawR: CGFloat, s: Double) -> CGPoint {
        let r = min(rawR, w / 2, h / 2)
        let (sw, sh) = (w - 2 * r, h - 2 * r)
        let quarter = Double.pi * r / 2
        let total = 2 * sw + 2 * sh + 4 * quarter
        guard total > 0 else { return CGPoint(x: w / 2, y: h / 2) }
        var d = (s - s.rounded(.down)) * total
        func corner(_ cx: CGFloat, _ cy: CGFloat, _ start: Double) -> CGPoint {
            let a = start + d / max(quarter, .leastNonzeroMagnitude) * .pi / 2
            return CGPoint(x: cx + r * cos(a), y: cy + r * sin(a))
        }
        // Half the top, the corners and sides clockwise, then the other half of the top.
        if d <= sw / 2 { return CGPoint(x: w / 2 + d, y: 0) }
        d -= sw / 2
        if d <= quarter { return corner(w - r, r, -.pi / 2) }
        d -= quarter
        if d <= sh { return CGPoint(x: w, y: r + d) }
        d -= sh
        if d <= quarter { return corner(w - r, h - r, 0) }
        d -= quarter
        if d <= sw { return CGPoint(x: w - r - d, y: h) }
        d -= sw
        if d <= quarter { return corner(r, h - r, .pi / 2) }
        d -= quarter
        if d <= sh { return CGPoint(x: 0, y: h - r - d) }
        d -= sh
        if d <= quarter { return corner(r, r, .pi) }
        d -= quarter
        return CGPoint(x: r + min(d, sw / 2), y: 0)
    }
}

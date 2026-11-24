// The tile's light and its breathing edge as Core Animation layers (`LayerDrawing`): the render server turns the light
// and breathes the edge, where a timeline redrew them from the main thread for every frame (`style/tile/orbit.rs`,
// `edge.rs`). The light is a faint track and a tail of steps that brighten toward the head; each step is a stretch of
// the ring's line, moved along it by its stroke's start and end.

import AppKit
import DesignSystem
import SwiftUI

enum TileLightLayers {
    /// The ring's line: the edge of a rounded rectangle `stroke` inside `area`, from the middle of its top, clockwise,
    /// `laps` times round. The origin is the top left.
    static func ring(in area: CGRect, round: CGFloat, stroke: CGFloat, laps: Int = 1) -> CGPath {
        let box = area.insetBy(dx: stroke / 2, dy: stroke / 2)
        let r = min(max(round - stroke / 2, 0), box.width / 2, box.height / 2)
        let path = CGMutablePath()
        path.move(to: CGPoint(x: box.midX, y: box.minY))
        for _ in 0..<laps {
            // With the origin at the top left a growing angle turns clockwise, so each corner is a quarter from its start.
            let corners: [(CGPoint, CGFloat)] = [
                (CGPoint(x: box.maxX - r, y: box.minY + r), -.pi / 2), (CGPoint(x: box.maxX - r, y: box.maxY - r), 0),
                (CGPoint(x: box.minX + r, y: box.maxY - r), .pi / 2), (CGPoint(x: box.minX + r, y: box.minY + r), .pi),
            ]
            for (centre, start) in corners {
                path.addArc(center: centre, radius: r, startAngle: start, endAngle: start + .pi / 2, clockwise: false)
            }
            path.addLine(to: CGPoint(x: box.midX, y: box.minY))
        }
        return path
    }

    /// The light: the track, and the tail's steps turning once every `SidebarMotion.orbitTurn`, `phase` of a turn ahead.
    static func orbit(in area: CGRect, round: CGFloat, stroke: CGFloat, color: Color, phase: Double) -> [CALayer] {
        let track = line(ring(in: area, round: round, stroke: stroke), in: area, stroke: stroke, color: color, alpha: TileGeometry.track)
        // Two laps of the ring, so a step that crosses the top's middle is still one stretch of the line: a step runs from
        // the second lap's start to its end and begins again a lap back, at the same place.
        let laps = ring(in: area, round: round, stroke: stroke, laps: 2)
        let steps = (0..<TileGeometry.steps).map { step -> CALayer in
            let from = -TileGeometry.span + TileGeometry.span * Double(step) / Double(TileGeometry.steps)
            let to = from + TileGeometry.span / Double(TileGeometry.steps)
            let alpha = Double(step + 1) / Double(TileGeometry.steps)
            let layer = line(laps, in: area, stroke: stroke, color: color, alpha: alpha * alpha)
            (layer.strokeStart, layer.strokeEnd) = ((1 + from) / 2, (1 + to) / 2)
            layer.add(turning("strokeStart", from: (1 + from) / 2, phase: phase), forKey: "start")  // text: Core Animation keys
            layer.add(turning("strokeEnd", from: (1 + to) / 2, phase: phase), forKey: "end")  // text: Core Animation keys
            return layer
        }
        return [track] + steps
    }

    /// The edge that breathes while a terminal waits: between `SidebarMotion.breatheLeast` and full strength along a sine,
    /// a breath every `SidebarMotion.breathe`.
    static func breathing(in area: CGRect, round: CGFloat, stroke: CGFloat, color: Color, phase: Double) -> [CALayer] {
        let edge = line(ring(in: area, round: round, stroke: stroke), in: area, stroke: stroke, color: color, alpha: 1)
        let breath = CAKeyframeAnimation(keyPath: "opacity")  // text: a Core Animation key path
        let least = SidebarMotion.breatheLeast
        // The sine as 48 straight pieces: no piece is further than 0.2 % of the range from it.
        breath.values = (0...48).map { least + (1 - least) * (sin(Double($0) / 48 * 2 * .pi) * 0.5 + 0.5) }
        breath.loop(every: SidebarMotion.breathe, phase: phase)
        edge.add(breath, forKey: "breath")  // text: a Core Animation key
        return [edge]
    }

    private static func line(_ path: CGPath, in area: CGRect, stroke: CGFloat, color: Color, alpha: Double) -> CAShapeLayer {
        let layer = CAShapeLayer()
        layer.frame = area
        layer.path = path
        layer.fillColor = nil
        let colour = NSColor(color)
        layer.strokeColor = colour.withAlphaComponent(colour.alphaComponent * alpha).cgColor
        (layer.lineWidth, layer.lineJoin) = (stroke, .round)
        return layer
    }

    /// A stroke's end moving one lap of the two (half the line) for every turn of the light.
    private static func turning(_ keyPath: String, from: Double, phase: Double) -> CABasicAnimation {
        let turn = CABasicAnimation(keyPath: keyPath)
        (turn.fromValue, turn.toValue) = (from, from + 0.5)
        turn.loop(every: SidebarMotion.orbitTurn, phase: phase)
        return turn
    }
}

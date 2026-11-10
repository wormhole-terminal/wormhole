// An SVG elliptical arc as cubic curves: the endpoint form a path's `A` command gives is turned into the centre form
// (SVG 1.1 implementation notes, F.6.5) and cut into pieces of at most a quarter turn, each a cubic Bézier.

import SwiftUI

enum SVGArc {
    static func add(to path: inout Path, from start: CGPoint, to end: CGPoint, radii: CGSize, turn degrees: CGFloat,
                    large: Bool, sweep: Bool) {
        var (rx, ry) = (abs(radii.width), abs(radii.height))
        guard start != end else { return }
        guard rx > 0, ry > 0 else { return path.addLine(to: end) }
        let phi = degrees * .pi / 180
        let (cosPhi, sinPhi) = (cos(phi), sin(phi))
        let (dx, dy) = ((start.x - end.x) / 2, (start.y - end.y) / 2)
        let (x1, y1) = (cosPhi * dx + sinPhi * dy, -sinPhi * dx + cosPhi * dy)
        // Radii too small to span the endpoints grow until they just do.
        let grow = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry)
        if grow > 1 { rx *= grow.squareRoot(); ry *= grow.squareRoot() }
        let numerator = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1
        let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1
        let factor = (large == sweep ? -1 : 1) * max(0, numerator / denominator).squareRoot()
        let (cx1, cy1) = (factor * rx * y1 / ry, -factor * ry * x1 / rx)
        let centre = CGPoint(x: cosPhi * cx1 - sinPhi * cy1 + (start.x + end.x) / 2,
                             y: sinPhi * cx1 + cosPhi * cy1 + (start.y + end.y) / 2)
        let from = atan2((y1 - cy1) / ry, (x1 - cx1) / rx)
        var sweepBy = atan2((-y1 - cy1) / ry, (-x1 - cx1) / rx) - from
        if sweep, sweepBy < 0 { sweepBy += 2 * .pi } else if !sweep, sweepBy > 0 { sweepBy -= 2 * .pi }

        let pieces = max(1, Int((abs(sweepBy) / (.pi / 2)).rounded(.up)))
        let step = sweepBy / CGFloat(pieces)
        // The control points sit this far along the tangent for an arc of `step` (4/3 · tan(step / 4)).
        let reach = 4 / 3 * tan(step / 4)
        func at(_ angle: CGFloat, tangent: CGFloat = 0) -> CGPoint {
            let (x, y) = (rx * (cos(angle) - tangent * sin(angle)), ry * (sin(angle) + tangent * cos(angle)))
            return CGPoint(x: centre.x + cosPhi * x - sinPhi * y, y: centre.y + sinPhi * x + cosPhi * y)
        }
        for piece in 0..<pieces {
            let (a, b) = (from + step * CGFloat(piece), from + step * CGFloat(piece + 1))
            path.addCurve(to: piece == pieces - 1 ? end : at(b), control1: at(a, tangent: reach),
                          control2: at(b, tangent: -reach))
        }
    }
}

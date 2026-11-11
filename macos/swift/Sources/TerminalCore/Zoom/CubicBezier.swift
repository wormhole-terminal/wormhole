// A CSS `cubic-bezier(x1, y1, x2, y2)` timing curve from (0, 0) to (1, 1), solved with Newton's method and
// bisection as core's `view::motion::ease` does. Foundation only: SwiftUI's `UnitCurve` is not allowed this
// low (the terminal view's layer has no UI framework above it).

import Foundation

public struct CubicBezier: Equatable, Sendable {
    public let x1, y1, x2, y2: Double

    public init(_ x1: Double, _ y1: Double, _ x2: Double, _ y2: Double) { (self.x1, self.y1, self.x2, self.y2) = (x1, y1, x2, y2) }

    /// How far along the curve is when `progress` of the time has passed (0 and 1 map to themselves).
    public func value(at progress: Double) -> Double {
        guard progress > 0, progress < 1 else { return min(max(progress, 0), 1) }
        func at(_ a: Double, _ b: Double, _ s: Double) -> Double {
            3 * a * s * (1 - s) * (1 - s) + 3 * b * s * s * (1 - s) + s * s * s
        }
        func slope(_ a: Double, _ b: Double, _ s: Double) -> Double {
            3 * a * (1 - s) * (1 - s) + 6 * (b - a) * s * (1 - s) + 3 * (1 - b) * s * s
        }
        var s = progress
        for _ in 0..<8 {
            let error = at(x1, x2, s) - progress
            if abs(error) < 1e-9 { return at(y1, y2, s) }
            let flatness = slope(x1, x2, s)
            if abs(flatness) < 1e-6 { break }
            s -= error / flatness
        }
        var (low, high) = (0.0, 1.0)
        s = progress
        for _ in 0..<40 {
            let x = at(x1, x2, s)
            if abs(x - progress) < 1e-9 { break }
            if x < progress { low = s } else { high = s }
            s = (low + high) / 2
        }
        return at(y1, y2, s)
    }
}

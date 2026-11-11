// The text size easing to a new one over 120 ms, ease-out (0.2, 0, 0, 1), in quarter points so the glyph atlas holds
// a handful of sizes on the way, not one per frame (`view::motion::text_zoom::Ease`). The terminal view samples it
// each frame; a second press goes on from the size the first reached; the grid's size is told once, on arrival.
// Reduce Motion cuts: the host does not make an ease at all.

import Foundation

public struct TextZoomEase: Equatable, Sendable {
    public struct Sample: Equatable, Sendable {
        public var size: Double
        /// It has arrived: `size` is the target, and nothing moves any more.
        public var done: Bool
    }

    public static let duration: TimeInterval = 0.12
    public static let quantum = 0.25
    /// `EASE_OUT` of core's `view::motion::ease`.
    private static let curve = CubicBezier(0.2, 0, 0, 1)

    private let from: Double
    private let to: Double
    private let began: Date

    public init(from: Double, to: Double, began: Date) { (self.from, self.to, self.began) = (from, to, began) }

    public func at(_ now: Date) -> Sample {
        let t = now.timeIntervalSince(began) / Self.duration
        if t >= 1 { return Sample(size: to, done: true) }
        return Sample(size: Self.quantise(value(at: t)), done: false)
    }

    /// Turned to a new target at `now`: from the size it has reached (not rounded, so the size does not jump).
    public func retargeted(at now: Date, to target: Double) -> TextZoomEase {
        let t = now.timeIntervalSince(began) / Self.duration
        return TextZoomEase(from: t >= 1 ? to : value(at: t), to: target, began: now)
    }

    private func value(at t: Double) -> Double { from + (to - from) * Self.curve.value(at: t) }

    public static func quantise(_ size: Double) -> Double { (size / quantum).rounded() * quantum }
}

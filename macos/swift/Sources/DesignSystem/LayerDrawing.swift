// Drawing that Core Animation animates by itself: layers built once and handed to the render server, for what repeats
// for as long as it shows (a status dot that breathes, a light that circles a tile). SwiftUI works its own animations out
// on the main thread for every frame, in whatever window they are: one breathing status dot took 344 ms of each second
// there, the same dot as a layer 4 ms, which is what the window takes with nothing moving (`studio-perf idle` in the Theme
// Studio's window, release build, macOS 27.2). A change of state stays SwiftUI's (`motion`, `motionTransition`): this is
// only for the loops.

import AppKit
import SwiftUI

/// Layers that fill the view's frame, drawn and animated by Core Animation. `key` is everything they are made from (a
/// colour, a phase): they are built again when it changes, when the frame does, and when the view joins a window.
public struct LayerDrawing<Key: Hashable>: NSViewRepresentable {
    private let key: Key
    private let layers: @MainActor (CGRect) -> [CALayer]

    /// `layers` builds them for the view's bounds, whose origin is the top left; it adds their animations itself.
    public init(key: Key, layers: @escaping @MainActor (CGRect) -> [CALayer]) {
        (self.key, self.layers) = (key, layers)
    }

    /// The view that holds the layers; it gets them in `updateNSView`, which SwiftUI calls once it is made.
    public func makeNSView(context: Context) -> LayerDrawingView { LayerDrawingView() }

    /// Builds the layers again when `key` is another than the one they were built from.
    public func updateNSView(_ view: LayerDrawingView, context: Context) { view.show(AnyHashable(key), layers) }
}

/// The view under a `LayerDrawing`: it takes no clicks and draws nothing itself.
public final class LayerDrawingView: NSView {
    private var key: AnyHashable?
    private var layers: (@MainActor (CGRect) -> [CALayer])?
    private var built = CGSize.zero

    init() {
        super.init(frame: .zero)
        wantsLayer = true
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

    override public var isFlipped: Bool { true }

    /// What is under it takes the click: the drawing is never a control.
    override public func hitTest(_ point: NSPoint) -> NSView? { nil }

    func show(_ key: AnyHashable, _ layers: @escaping @MainActor (CGRect) -> [CALayer]) {
        self.layers = layers
        guard key != self.key else { return }
        self.key = key
        build()
    }

    override public func layout() {
        super.layout()
        if bounds.size != built { build() }
    }

    /// A view that comes back into a window (a row scrolled out of a lazy list and in again) starts its layers afresh, so
    /// nothing rests on Core Animation keeping an animation for a layer that was off screen.
    override public func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window != nil { build() }
    }

    private func build() {
        built = bounds.size
        layer?.sublayers = bounds.isEmpty ? [] : layers?(bounds)
    }
}

extension CALayer {
    /// A filled circle in `frame` that breathes: its opacity goes to `faint` and back, a breath each way.
    public static func breathing(_ colour: Color, in frame: CGRect, faint: Double) -> CALayer {
        pulsing(colour, in: frame, keyPath: "opacity", to: faint)  // text: a Core Animation key path
    }

    /// A filled circle in `frame` that swells to `scale` times its size and back, a breath each way.
    public static func swelling(_ colour: Color, in frame: CGRect, scale: Double) -> CALayer {
        pulsing(colour, in: frame, keyPath: "transform.scale", to: scale)  // text: a Core Animation key path
    }

    /// `Motion.breath` as Core Animation has it: the same ease in and out (SwiftUI's `easeInOut` and `easeInEaseOut` are
    /// both the curve 0.42, 0, 0.58, 1), there and back for ever, as `phaseAnimator` over two phases plays it.
    private static func pulsing(_ colour: Color, in frame: CGRect, keyPath: String, to value: Double) -> CALayer {
        let circle = CALayer()
        circle.frame = frame
        circle.cornerRadius = min(frame.width, frame.height) / 2
        circle.backgroundColor = NSColor(colour).cgColor
        let pulse = CABasicAnimation(keyPath: keyPath)
        (pulse.fromValue, pulse.toValue) = (1, value)
        pulse.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        pulse.loop(every: Motion.breathSeconds, andBack: true)
        circle.add(pulse, forKey: keyPath)
        return circle
    }
}

extension CAAnimation {
    /// Plays for ever, a round every `seconds` (and as long back again with `andBack`), and is at any moment where a
    /// clock that started at zero would have it, `phase` of a round ahead. So layers built at different times are in
    /// step, and layers built again (their colour changed) go on from where the old ones were.
    public func loop(every seconds: Double, phase: Double = 0, andBack: Bool = false) {
        (duration, autoreverses, repeatCount) = (seconds, andBack, .infinity)
        // A begin time of zero would mean "now", so it is one round earlier than the phase asks.
        beginTime = -(phase + 1) * seconds * (andBack ? 2 : 1)
    }
}

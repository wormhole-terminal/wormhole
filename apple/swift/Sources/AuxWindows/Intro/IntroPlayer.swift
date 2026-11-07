// Plays the intro around a window and tells the page how far it has got (`intro/mod.rs`). One plays at a time: a
// new one replaces the one playing. It runs on the display link, so a frame is drawn when the screen wants it.
// Under Reduce Motion the glow stands in for any style: nothing moves, the page is simply there.

import AppKit
import AuxModel
import Observation
import QuartzCore

public struct IntroOptions {
    /// Whether the window was showing as it began (a replay) or never was (a first launch).
    public var start: IntroStart = .shown
    /// Whether the traffic lights fade away as the run begins: a replay over a window that shows them.
    public var lights = false
    /// The run hands over to the terminal welcome, not the intro window's page (`nil`): Portal has no ship.
    public var handover: Handover?
    /// Over the whole screen, or just above the window (a test instance kept behind other apps).
    public var immersive = true
    public init() {}
}

@MainActor @Observable
public final class IntroPlayer {
    /// How a skipped run takes to settle: the window back to rest and the overlay gone.
    private static let settle: Float = 0.3

    /// How far the run playing now has got; `nil` when none is.
    public private(set) var progress: Progress?
    /// Where the page's logo is in the window: its middle's x and y and its side, points from the top left.
    @ObservationIgnored public var landing: SIMD3<Float>?
    /// Called once when a run finishes or is replaced.
    @ObservationIgnored public var onFinish: (() -> Void)?

    public struct Progress: Equatable {
        public var style: IntroStyle
        public var seconds: Float
        public var start: IntroStart
        /// It was cut short: the page is whole, and the window is settling.
        public var skipped: Bool
    }

    @ObservationIgnored private var run: Run?
    @ObservationIgnored private let reduceMotion: () -> Bool

    public init(reduceMotion: @escaping () -> Bool = { NSWorkspace.shared.accessibilityDisplayShouldReduceMotion }) {
        self.reduceMotion = reduceMotion
    }

    /// The page's parts as far as the playing run lets them in: all of them when none plays.
    public var entrance: Entrance {
        guard let progress, !progress.skipped else { return .done }
        // A replay's window is still showing, and leaving, with the page it had.
        if progress.start == .shown, progress.seconds < IntroPlayer.leaving { return .done }
        return progress.style.entrance(at: progress.seconds)
    }

    /// While the window of a replay is leaving, before the page comes in again.
    static let leaving: Float = 0.3

    public var isEntering: Bool { progress != nil && entrance.least < 1 }

    /// Seconds into the run playing, for the terminal welcome's hand-over timing.
    public var elapsed: Float? { progress?.seconds }

    /// Plays `style` around `window` (the glow instead under Reduce Motion); `colors` from the theme's tokens.
    public func play(_ style: IntroStyle, over window: NSWindow, options: IntroOptions = IntroOptions(), colors: IntroColors) {
        stop()
        let style = style.withMotion(reduceMotion: reduceMotion())
        let setup = IntroSetup(
            style: style, colors: colors, start: options.start, lights: options.lights && style != .glow,
            ship: options.handover?.ship ?? true, immersive: options.immersive)
        do {
            let overlay = try IntroOverlay(over: window, setup: setup, landing: { [weak self] in self?.landing })
            run = Run(overlay: overlay, style: style, start: options.start) { [weak self] in self?.tick($0) }
            progress = Progress(style: style, seconds: 0, start: options.start, skipped: false)
        } catch {
            // No Metal, or the shader failed: the intro doesn't play, and the page simply shows.
            progress = nil
        }
    }

    /// Cuts the run short: the window settles back and the overlay fades away.
    public func skip() { run?.skipRequested = true }

    public func stop() {
        guard let run else { return }
        run.stop()
        self.run = nil
        progress = nil
        onFinish?()
    }

    private func tick(_ run: Run) {
        guard run === self.run else { return }
        let time = Float(CACurrentMediaTime() - run.started)
        let seconds = run.style.seconds
        if run.skipRequested, run.cut == nil {
            let t = min(time / seconds, 1)
            run.cut = (time, t, run.overlay.motion(at: t))
        }
        let over: Bool
        if let (at, t, from) = run.cut {
            let settled = min(max((time - at) / Self.settle, 0), 1)
            run.overlay.frame(IntroFrame(t: t, time: time, lights: nil, settling: (from, settled)))
            over = time - at >= Self.settle
        } else {
            let t = min(time / seconds, 1)
            let lights = IntroStyle.lights(at: time, start: run.start)
            run.overlay.frame(IntroFrame(t: t, time: time, lights: lights, settling: nil))
            over = time >= seconds
        }
        progress = Progress(style: run.style, seconds: time, start: run.start, skipped: run.cut != nil)
        if over { stop() }
    }
}

/// The playing run: its overlay and the display link that draws it.
@MainActor
private final class Run: NSObject {
    let overlay: IntroOverlay
    let style: IntroStyle
    let start: IntroStart
    let started = CACurrentMediaTime()
    var skipRequested = false
    var cut: (at: Float, t: Float, from: WindowMotion)?
    private var link: CADisplayLink?
    private let onTick: (Run) -> Void

    init(overlay: IntroOverlay, style: IntroStyle, start: IntroStart, onTick: @escaping (Run) -> Void) {
        (self.overlay, self.style, self.start, self.onTick) = (overlay, style, start, onTick)
        super.init()
        link = overlay.window.contentView?.displayLink(target: self, selector: #selector(step))
        link?.add(to: .main, forMode: .common)
    }

    @objc private func step() { onTick(self) }

    func stop() {
        link?.invalidate()
        link = nil
        overlay.close()
    }
}

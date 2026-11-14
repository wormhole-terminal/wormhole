// A window opened for a torn-off terminal growing from the chip it was let go as (GPUI's `window/grow.rs`; the pose at each
// moment is the core's `view::motion::tear_off`). The window opens at its final size, so its terminal is sized once and the
// grid never reflows while it grows; what grows is the part of it that shows, a rectangle from the chip's to the whole
// window, on a body the colour of the chip, the content fading in over it. Until it is over the window has no shadow and no
// traffic lights; each comes back when the growth ends.

import AppKit
import DesignSystem
import SidebarUI
import SwiftUI
import WindowsModel
import WorkspaceModel

@MainActor
final class TearOffGrow {
    /// What a window is to grow from: the chip in the window's own space (y down), and the terminal on its face.
    struct Start {
        var chip: CGRect
        var terminal: TerminalSummary?
    }

    /// The pose the overlay draws, changed each frame.
    @MainActor @Observable
    final class Pose {
        var grow: TearOffGrow.Frame
        init(_ grow: TearOffGrow.Frame) { self.grow = grow }
    }

    typealias Frame = TearOffPose

    private let window: NSWindow
    private let content: NSView
    private let backend: any WindowsBackend
    private let start: Start
    private let pose: Pose
    private let mask = CAShapeLayer()
    private var timer: Timer?
    private var began = Date()
    /// Where in its growth a held window stands (`hold`), in seconds of it.
    private var held: TimeInterval?
    private var onEnd: () -> Void

    /// The control folder's `grow-hold <t>`: every growing window stands at that point of its growth (0 the chip, 1 the
    /// window) and stays, for a shot; nil (`grow-hold off`) lets it grow on from there (GPUI's `window::grow::hold`). Set it
    /// before the tear-off to catch a window from its first frame.
    static var hold: Double? {
        didSet { for grow in growing.allObjects { grow.follow() } }
    }

    /// The windows growing now, for `hold`.
    private static let growing = NSHashTable<TearOffGrow>.weakObjects()

    /// Hides what would show the whole window (its shadow, its lights) and puts the first pose: the chip.
    init?(
        window: NSWindow, start: Start, appearance: SidebarAppearance, themes: ThemeStore = .shared, backend: any WindowsBackend,
        onEnd: @escaping () -> Void = {}
    ) {
        guard let content = window.contentView else { return nil }
        (self.window, self.content, self.backend, self.start, self.onEnd) = (window, content, backend, start, onEnd)
        let first = backend.tearOffGrow(chip: start.chip, size: content.bounds.size, radius: Self.radius, elapsed: 0)
        pose = Pose(first)
        window.hasShadow = false
        Self.lights(window, shown: false)
        content.wantsLayer = true
        content.layer?.mask = mask
        let overlay = NSHostingView(rootView: GrowOverlay(pose: pose, start: start, appearance: appearance, themes: themes))
        overlay.frame = content.bounds
        overlay.autoresizingMask = [.width, .height]
        content.addSubview(overlay)
        apply(first)
    }

    /// The window is on screen: the growth runs for the core's duration.
    func run() {
        began = Date()
        Self.growing.add(self)
        follow()
    }

    /// The growth's clock running, or the window stood at the held point with no clock (`hold`).
    private func follow() {
        timer?.invalidate()
        timer = nil
        if let hold = Self.hold {
            let at = min(max(hold, 0), 1) * backend.tearOffGrowDuration
            held = at
            let frame = backend.tearOffGrow(chip: start.chip, size: content.bounds.size, radius: Self.radius, elapsed: at)
            pose.grow = frame
            apply(frame)
            return
        }
        if let at = held { began = Date(timeIntervalSinceNow: -at) }
        held = nil
        let timer = Timer(timeInterval: 1.0 / 60, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.tick() }
        }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    private func tick() {
        let frame = backend.tearOffGrow(
            chip: start.chip, size: content.bounds.size, radius: Self.radius, elapsed: Date().timeIntervalSince(began))
        pose.grow = frame
        apply(frame)
        if frame.done { finish() }
    }

    /// What shows of the window: the content is cut to the rectangle (its corners round), and the overlay's body and face fill it.
    private func apply(_ frame: Frame) {
        mask.path = CGPath(roundedRect: frame.shown, cornerWidth: frame.radius, cornerHeight: frame.radius, transform: nil)
        mask.frame = content.bounds
    }

    private func finish() {
        timer?.invalidate()
        timer = nil
        Self.growing.remove(self)
        content.layer?.mask = nil
        content.subviews.last(where: { $0 is NSHostingView<GrowOverlay> })?.removeFromSuperview()
        window.hasShadow = true
        Self.lights(window, shown: true)
        onEnd()
    }

    /// The window's own corners (`radius_lg` of the theme is the card's; a window's are the system's, about this).
    private static let radius: CGFloat = 16

    private static func lights(_ window: NSWindow, shown: Bool) {
        for kind in [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton] {
            window.standardWindowButton(kind)?.alphaValue = shown ? 1 : 0
        }
    }
}

/// The body the chip grew into and the chip's own face on it, which the window's content fades in over.
private struct GrowOverlay: View {
    let pose: TearOffGrow.Pose
    let start: TearOffGrow.Start
    let appearance: SidebarAppearance
    let themes: ThemeStore

    var body: some View {
        let grow = pose.grow
        ZStack(alignment: .topLeading) {
            RoundedRectangle(cornerRadius: grow.radius)
                .fill(themes.tokens.surfaceElevated.opacity(1 - grow.content))
                .frame(width: grow.shown.width, height: grow.shown.height)
                .offset(x: grow.shown.minX, y: grow.shown.minY)
            TearOffChipView(terminal: start.terminal, title: start.terminal?.title ?? "", appearance: appearance, bleed: false, themes: themes)
                .opacity(grow.face)
                .offset(x: grow.shown.minX, y: grow.shown.minY)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .allowsHitTesting(false)
        .themed(themes)
    }
}

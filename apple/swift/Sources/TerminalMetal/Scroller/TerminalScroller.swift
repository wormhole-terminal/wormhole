// The scroll bar: a real `NSScroller`, so it looks and behaves as the system's does (overlay thumbs that show
// while scrolling and fade, or the legacy gutter with a track; click in the track pages or jumps by the
// system setting, Option does the other; VoiceOver reads it). Its position is the history the terminal holds:
// the top is the oldest line, the bottom the live screen. The scrolling itself is the engine's; this maps the
// scroller's hit to a view offset. Behaviour after `wormhole-term` `scrollbar.rs` and `terminal_view/render/scrollbar.rs`.

import AppKit

@MainActor
final class TerminalScroller: NSObject {
    /// How long the overlay scroller stays after the last scroll, and how long it takes to fade (`scrollbar.rs`).
    static let hold: DispatchTimeInterval = .milliseconds(1000)
    static let fade: TimeInterval = 0.25
    /// How near the right edge the pointer brings the overlay scroller up (`scrollbar.rs` `ZONE_W`).
    static let zone: CGFloat = 16

    let view = NSScroller(frame: NSRect(x: 0, y: 0, width: 15, height: 100))
    /// The offset (lines back from the live screen) a hit asks for.
    var onScroll: ((Int) -> Void)?
    private var history = 0, rows = 0, offset = 0
    private var held = false
    private var hide: DispatchWorkItem?

    override init() {
        super.init()
        view.scrollerStyle = NSScroller.preferredScrollerStyle
        view.target = self
        view.action = #selector(hit(_:))
        view.isEnabled = false
        view.setAccessibilityLabel(TerminalWord.scrollbarLabel.text)
        applyStyle()
    }

    var style: NSScroller.Style { view.scrollerStyle }

    /// The minimap stands in for the bar: hidden, and no gutter.
    var isSuppressed = false {
        didSet {
            hide?.cancel()
            view.isHidden = isSuppressed || style == .overlay
        }
    }

    /// Width the grid gives up: the legacy gutter, nothing for an overlay thumb.
    var gutterWidth: CGFloat { style == .legacy && !isSuppressed ? NSScroller.scrollerWidth(for: .regular, scrollerStyle: .legacy) : 0 }

    func layout(in bounds: CGRect) {
        let width = NSScroller.scrollerWidth(for: .regular, scrollerStyle: style)
        view.frame = NSRect(x: bounds.maxX - width, y: 0, width: width, height: bounds.height)
    }

    /// The history's length, the rows on screen and how far back the view is. A move shows an overlay thumb.
    func update(history: Int, rows: Int, offset: Int) {
        guard (history, rows, offset) != (self.history, self.rows, self.offset) else { return }
        let moved = offset != self.offset
        (self.history, self.rows, self.offset) = (history, rows, offset)
        view.isEnabled = history > 0
        view.knobProportion = history > 0 ? CGFloat(rows) / CGFloat(history + rows) : 1
        view.doubleValue = history > 0 ? Double(history - min(offset, history)) / Double(history) : 1
        if moved { reveal() }
    }

    /// The pointer is at `x` in a view `width` wide: near the right edge it holds an overlay thumb up.
    func pointer(x: CGFloat, in width: CGFloat) {
        let near = x >= width - Self.zone
        guard near != held else { return }
        held = near
        if near { reveal() }
    }

    /// The scroller sits on the terminal's background, not on a system surface: draw it for that background's
    /// brightness (light knob and dark track on a dark terminal), whatever the system appearance is.
    func adapt(toBackground rgb: SIMD3<UInt8>) {
        // Rec. 601 weights, the same split the app uses to tell programs whether the theme is light or dark.
        let luma = (0.299 * Double(rgb.x) + 0.587 * Double(rgb.y) + 0.114 * Double(rgb.z)) / 255
        let dark = luma < 0.5
        view.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
        view.knobStyle = dark ? .light : .dark
    }

    /// The system's scroller style changed (System Settings, a mouse plugged in): the owner lays out again.
    func applyStyle() {
        view.scrollerStyle = NSScroller.preferredScrollerStyle
        hide?.cancel()
        view.isHidden = style == .overlay || isSuppressed
        view.alphaValue = 1
    }

    func reveal() {
        hide?.cancel()
        guard !isSuppressed else { return }
        view.isHidden = false
        view.alphaValue = 1
        guard style == .overlay else { return }
        let item = DispatchWorkItem { [weak self] in MainActor.assumeIsolated { self?.fadeOut() } }
        hide = item
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.hold, execute: item)
    }

    private func fadeOut() {
        // A pointer held near the edge, or a drag in progress, keeps it.
        guard !held, NSEvent.pressedMouseButtons == 0 else { return reveal() }
        guard !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion else {
            view.isHidden = true
            return
        }
        NSAnimationContext.runAnimationGroup { context in
            context.duration = Self.fade
            view.animator().alphaValue = 0
        } completionHandler: { [weak self] in
            MainActor.assumeIsolated { if self?.view.alphaValue == 0 { self?.view.isHidden = true } }
        }
    }

    @objc private func hit(_ sender: NSScroller) {
        onScroll?(Self.offset(hit: sender.hitPart, value: sender.doubleValue, history: history, rows: rows, current: offset))
    }

    /// The offset a hit asks for: the thumb's position, or a page (keeping a line of the old page in view) toward the click.
    static func offset(hit part: NSScroller.Part, value: Double, history: Int, rows: Int, current: Int) -> Int {
        let page = max(1, rows - 1)
        let wanted: Int =
            switch part {
            case .knob, .knobSlot: Int(((1 - value) * Double(history)).rounded())
            case .decrementPage: current + page
            case .incrementPage: current - page
            case .decrementLine: current + 1
            case .incrementLine: current - 1
            default: current
            }
        return min(max(wanted, 0), history)
    }
}

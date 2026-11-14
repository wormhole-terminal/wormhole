// The terminal view: a layer-backed NSView whose layer is a CAMetalLayer, drawn from a display link that
// the source's wake starts and that pauses itself as soon as a tick finds nothing new. Nothing else runs
// while the window is idle: no timer, no polling (the cursor blink is one one-shot timer, and only while a
// blinking cursor is shown in the key window).
//
// Everything here runs on the main thread; the one thing that does not is the wake closure given to the
// source, which only sets an atomic and hops to the main queue.

import AppKit
import QuartzCore
import Synchronization
import TerminalCore

@MainActor
public final class MetalTerminalView: NSView {
    public let source: any FrameSource
    public let renderer: MetalRenderer
    public var onTitle: ((String) -> Void)?
    public var onBell: (() -> Void)?
    public var onExit: ((Int32?) -> Void)?
    /// Something was copied (the selection, a link): the host may say so.
    public var onCopied: ((String) -> Void)?
    /// Clear Buffer left the history in a session host that cannot forget it: it is back at the next attach.
    public var onHistoryKept: (() -> Void)?
    /// Rows were rebuilt by a frame (output, a scroll): what is drawn from the terminal's state beside the grid (block
    /// marks, the minimap, find's ticks) may be read again.
    public var onContentChanged: (() -> Void)?
    /// A program set the clipboard (OSC 52): the host may say so.
    public var onProgramCopy: ((String) -> Void)?
    /// A program asks to read the clipboard (the name of the program in front, if known): the host asks the person and
    /// answers with `answerClipboard`. Closed unanswered when it times out.
    public var onClipboardAsk: ((String?) -> Void)?
    public var onClipboardAskClosed: (() -> Void)?
    /// The pointer is over viewport row `row` of the grid (`nil`: it left, or is past the grid): block hover.
    public var onPointerRow: ((Int?) -> Void)?
    /// Typing, a paste or the wheel: the view is not where the last ⌘↑ / ⌘↓ left it (the block cursor is forgotten).
    public var onInput: (() -> Void)?
    /// Find's bar is open: the matches among the viewport rows are painted after each frame (`+Find`).
    public var findActive = false { didSet { if findActive != oldValue { refreshFound() } } }
    /// Width the grid leaves free on the right for the minimap, which takes the scroll bar's place (`+Minimap`).
    public var minimapWidth: CGFloat = 0 { didSet { if minimapWidth != oldValue { minimapChanged() } } }
    /// The context menu for the click that is being handled (the window's: `TerminalMenu`); `nil` opens none.
    public var contextMenu: (@MainActor () -> NSMenu?)?
    /// Records every draw's phases when set (the measuring mode).
    public var sampler: FrameSampler?
    public internal(set) var statistics = ViewStatistics()
    public var isLinkRunning: Bool { link.map { !$0.isPaused } ?? false }
    /// Which Option key sends ESC before the character.
    /// `option_as_meta` (Settings): off until the setting says otherwise, as the core's default is.
    public var optionAsMeta = OptionAsMeta.off
    /// Whether a ⌘ key (its name, its modifiers) is one the application's menus own: the terminal never takes those for its own.
    public var isAppKey: (String, NSEvent.ModifierFlags) -> Bool = { _, _ in false }
    /// The keymap's action id for a ⌘ key in the Terminal or Edit scope (its name, its modifiers), or nil: which keys the
    /// terminal takes itself is the table's (`keymap::terminal_action`). Asked only for ⌘ keys, never for typing.
    public var terminalAction: (String, NSEvent.ModifierFlags) -> String? = { _, _ in nil }
    /// A key the window takes before the terminal hears it (a project action's own key): asked with the key's name
    /// (`KeyMap`'s) and its modifiers for a ⌘ key and an F-key only; true when the window took it.
    public var claimKey: (String, NSEvent.ModifierFlags) -> Bool = { _, _ in false }
    /// ⌘↑ / ⌘↓ (and the keys that fall back to them): the window jumps this terminal to the previous or next prompt.
    public var onPrompt: (PromptDirection) -> Void = { _ in }
    /// A grid of this size whatever the view's size (the measuring mode's 200 x 69).
    public var fixedGrid: (cols: Int, rows: Int)?
    /// How long a blinking cursor is on and off; `nil` keeps it steady (xterm's and macOS's own text cursor rate).
    public var blinkInterval: Duration? = .milliseconds(530)
    /// Command is held over a link a Probe's rule made: what to say of it, where it really goes and which Probe said so (`nil`: none).
    public var onLinkReadout: ((String?) -> Void)?
    /// Command-click on a file a Probe's rule named (`path[:line[:column]]`): the host finds it in the terminal's folder.
    public var onFileLink: ((String) -> Void)?
    /// How a link is opened (the system's; tests replace it).
    public var linkOpener: (OpenTarget) -> Void = { LinkOpener.system($0) }
    /// Which parts of the bell happen, and the colour and corner of its glow (the host's tokens).
    public var bell = BellStyle()
    public var bellColor = NSColor.systemOrange
    /// The tint over the pane while files are dragged over it (the theme's `state.highlight`); the layer exists only while it shows.
    public var dropTint = NSColor.controlAccentColor.withAlphaComponent(0.2)
    var dropLayer: CALayer?
    public var bellCornerRadius: CGFloat = 8

    // Text zoom and fonts (see MetalTerminalView+Zoom).
    /// The fonts as Settings has them; the zoom is added to their size.
    var baseFonts: TerminalFontSettings
    /// Points over the size in Settings (⌘+ ⌘- ⌘0).
    public internal(set) var zoom = 0.0
    var zoomEase: TextZoomEase?
    /// Whether a zoom eases (core's Motion feature); off, or under Reduce Motion, it cuts.
    public var animatesZoom = true
    /// A zoom has arrived and the program was told the new grid (columns, rows): the host may show its size chip.
    public var onZoomSettled: ((_ cols: Int, _ rows: Int) -> Void)?

    private var link: CADisplayLink?
    private var started = false
    private var redrawRequested = true
    var blinkItem: DispatchWorkItem?
    var textBlinkItem: DispatchWorkItem?
    var glowLayer: CALayer?
    var glowItem: DispatchWorkItem?
    /// When the program may next be told a size while the window is dragged, and the layout that comes when one held back is due.
    let resizeGate = ResizeGate()
    var resizeItem: DispatchWorkItem?
    /// The grid the program was last told (`syncSurface`).
    public private(set) var gridSize = GridSize(cols: 0, rows: 0) { didSet { if gridSize != oldValue { onGridChange?() } } }
    /// Told when the grid the program has changed (the status bar's size reads it).
    public var onGridChange: (() -> Void)?
    private let wake = WakeRelay()

    // Input method state (see Keyboard/).
    var composition = MarkedTextModel()
    var pendingKey: KeyInput?
    var handledByInputMethod = false

    // Pointer, selection and scroller (see Pointer/, Scroller/).
    var interaction: (any TerminalInteraction)?
    var pointer = PointerState()
    /// A selection may exist: ask for its spans after each frame until one comes back empty.
    var selectionMayExist = false
    /// Find's matches are painted (`refreshFound`).
    var foundShown = false
    /// A span shows its text under the pointer (`updateReveal`): the pointer's leaving must say so.
    var revealing = false
    var focusReported = false
    let scroller = TerminalScroller()

    public init(source: any FrameSource, configuration: RendererConfiguration, frame: NSRect = NSRect(x: 0, y: 0, width: 960, height: 600)) throws {
        self.source = source
        renderer = try MetalRenderer(configuration: configuration)
        baseFonts = configuration.fonts
        super.init(frame: frame)
        renderer.images.provider = { [source] key in source.inlineImage(key: key) }
        wantsLayer = true
        layerContentsRedrawPolicy = .never
        if let layer = layer as? CAMetalLayer {
            layer.device = renderer.device
            layer.pixelFormat = .bgra8Unorm
            layer.framebufferOnly = true
            layer.isOpaque = true
            layer.maximumDrawableCount = 3
            // A frame of another size is never scaled to the layer's: it stays where it is (the clear colour is the terminal's).
            layer.contentsGravity = .topLeft
            layer.colorspace = CGColorSpace(name: CGColorSpace.sRGB)
            layer.contentsScale = CGFloat(configuration.scale)
        }
        addSubview(scroller.view)
        scroller.adapt(toBackground: configuration.palette.background)
        scroller.onScroll = { [weak self] offset in self?.scroll(toOffset: offset) }
        registerForDraggedTypes([.fileURL, .string])
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

    public override func makeBackingLayer() -> CALayer { CAMetalLayer() }
    public override var acceptsFirstResponder: Bool { true }
    public override var isFlipped: Bool { true }
    public override var isOpaque: Bool { backgroundOpacity >= 1 }

    /// How opaque the terminal's own background is (the window's opacity, 1 under Reduce Transparency).
    public var backgroundOpacity: Double = 1 {
        didSet {
            guard backgroundOpacity != oldValue else { return }
            renderer.backgroundAlpha = backgroundOpacity
            metalLayer?.isOpaque = backgroundOpacity >= 1
            renderer.invalidate()
            requestRedraw()
        }
    }
    private var metalLayer: CAMetalLayer? { layer as? CAMetalLayer }

    /// A new theme's colours: the renderer's palette, and the scroll bar drawn for the new background. The engine hears
    /// them too, so a program that asks (OSC 4, 10, 11, 12) is answered with the colours this terminal is drawn in,
    /// whichever profile its window is in (`share_palette` in the GPUI app's terminal view).
    public func setPalette(_ palette: TerminalPalette) {
        renderer.setPalette(palette)
        source.setEngineColors(palette.engineSlots)
        scroller.adapt(toBackground: palette.background)
        requestRedraw()
    }

    // MARK: lifecycle

    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window == nil {
            invalidateLink()
            source.setOnScreen(false)
            return
        }
        // `NSView.displayLink` follows the screen the view is on and (unlike CVDisplayLink) needs no setup
        // of its own; the proxy keeps the link from retaining the view.
        let proxy = LinkProxy(self)
        let link = displayLink(target: proxy, selector: #selector(LinkProxy.tick(_:)))
        link.isPaused = true
        link.add(to: .main, forMode: .common)
        self.link = link
        NotificationCenter.default.addObserver(
            self, selector: #selector(keyStateChanged), name: NSWindow.didBecomeKeyNotification, object: window)
        NotificationCenter.default.addObserver(
            self, selector: #selector(keyStateChanged), name: NSWindow.didResignKeyNotification, object: window)
        reportOnScreen()
        for name in [NSScroller.preferredScrollerStyleDidChangeNotification] {
            NotificationCenter.default.addObserver(self, selector: #selector(scrollerStyleChanged), name: name, object: nil)
        }
        NSWorkspace.shared.notificationCenter.addObserver(
            self, selector: #selector(accessibilityOptionsChanged), name: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil)
        renderer.setFocused(window?.isKeyWindow ?? false)
        syncSurface()
        startSourceIfNeeded()
        requestRedraw()
    }

    private func startSourceIfNeeded() {
        guard !started, gridSize.cols > 0 else { return }
        started = true
        let relay = wake
        do {
            defer { interaction = source.interaction }
            try source.start(size: gridSize) { [weak self] in
                // Any thread, no lock held: note the wake and hop. One hop per drain, however much output.
                guard !relay.queued.exchange(true, ordering: .acquiringAndReleasing) else { return }
                DispatchQueue.main.async {
                    MainActor.assumeIsolated {
                        relay.queued.store(false, ordering: .releasing)
                        self?.wakeOnMain()
                    }
                }
            }
        } catch {
            FileHandle.standardError.write(Data("source failed to start: \(error)\n".utf8))  // text: a diagnostic written to stderr
        }
    }

    /// Stop everything the view started. Explicit, like the terminal's `close`: a `deinit` only frees.
    public func shutdown() {
        invalidateLink()
        blinkItem?.cancel()
        blinkItem = nil
        source.stop()
    }

    private func invalidateLink() {
        link?.invalidate()
        link = nil
        NotificationCenter.default.removeObserver(self)
        NSWorkspace.shared.notificationCenter.removeObserver(self)
    }

    // MARK: size

    public override func layout() {
        super.layout()
        scroller.layout(in: bounds)
        syncSurface()
    }

    public override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        syncSurface()
    }

    /// Drawable size and grid size from the view's size. A change of grid goes to the source, which answers
    /// with a full frame of the new size.
    func syncSurface() {
        let scale = CGFloat(renderer.scale)
        let pixels = CGSize(width: (bounds.width * scale).rounded(.down), height: (bounds.height * scale).rounded(.down))
        guard pixels.width >= 1, pixels.height >= 1 else { return }
        if let layer = metalLayer, layer.drawableSize != pixels {
            layer.drawableSize = pixels
            renderer.invalidate()
            requestRedraw()
            // What was drawn is for the old size: draw again now, with this layout's transaction, not on the next tick.
            if started, window != nil { drawFrame(transaction: true) }
        }
        // A zoom on its way keeps the grid the program has: it is told once, when the size has arrived.
        guard zoomEase == nil else { return }
        // The legacy scroll bar's gutter is not grid.
        // The padding is on every side, and the right one also keeps the scroller's or minimap's strip clear.
        // The cells the area holds, by the core's rule (the GPUI pane's): a screen pixel short of a whole cell still holds it,
        // as a window whose size AppKit rounds to pixels may be (GPUI's slack while a drag steps by cells, Swift's default).
        let padding = MetalRenderer.padding
        let area = (width: bounds.width - scroller.gutterWidth - minimapWidth - 2 * padding, height: bounds.height - 2 * padding)
        let (cols, rows) = fixedGrid ?? renderer.grid(fittingPoints: Double(area.width), Double(area.height), slack: 1 / Double(scale))
        let cell = renderer.cell
        let size = GridSize(cols: cols, rows: rows, cellWidth: Float(cell.width), cellHeight: Float(cell.height))
        guard size != gridSize else { return }
        // A window being dragged tells the program a size at most every 100 ms (the last one always, and once more when the
        // drag ends: `viewDidEndLiveResize`), as a full-screen program redraws everything for each size.
        if started {
            let step = resizeGate.step(live: inLiveResize, differs: true)
            guard step.apply else {
                if let wake = step.wakeIn {
                    resizeItem?.cancel()
                    let item = DispatchWorkItem { [weak self] in MainActor.assumeIsolated { self?.syncSurface() } }
                    resizeItem = item
                    DispatchQueue.main.asyncAfter(deadline: .now() + wake, execute: item)
                }
                return
            }
            resizeItem?.cancel()
        }
        gridSize = size
        if started {
            source.resize(size)
            requestRedraw()
        } else {
            startSourceIfNeeded()
        }
    }

    public override func viewDidEndLiveResize() {
        super.viewDidEndLiveResize()
        syncSurface()
    }

    // MARK: the display link

    /// Called from the main queue after the source woke: start the link if it is paused.
    private func wakeOnMain() {
        statistics.wakes += 1
        link?.isPaused = false
    }

    public func requestRedraw() {
        redrawRequested = true
        link?.isPaused = false
    }

    fileprivate func tick() {
        statistics.ticks += 1
        advanceZoom()
        let needFrame = handleEvents() || redrawRequested
        guard needFrame else { return pauseLink() }
        redrawRequested = false
        switch drawFrame().outcome {
        case .drawn: break
        case .busy:
            // The GPU has three frames queued: draw again on the next tick, with the link running.
            statistics.busySkips += 1
            redrawRequested = true
        case .noDrawable: redrawRequested = true
        }
        // A tick that found a frame with no change leaves nothing pending: the link pauses on the next one.
    }

    private func pauseLink() {
        statistics.pauses += 1
        link?.isPaused = true
    }

    /// Everything queued by the source. Drained before `frame()`: output that arrives in between is in the
    /// frame and wakes again. Returns whether new output is waiting.
    private func handleEvents() -> Bool {
        var output = false
        for event in source.drainEvents() {
            switch event {
            case .output: output = true
            case .title(let title): onTitle?(title)
            case .resetTitle: onTitle?("")
            case .bell:
                ringBell()
                onBell?()
            case .exited(let code):
                renderer.setCursorHidden(true)
                requestRedraw()
                onExit?(code)
            case .clipboardWrite, .clipboardRead, .clipboardAsk, .clipboardAskClosed: handleClipboard(event)
            // As a key typed here: the jump between prompts is forgotten and the cursor holds still (GPUI's `send_strokes`).
            case .controlTyped:
                onInput?()
                holdCursor()
            }
        }
        if output { refreshPointerShape() }
        return output
    }

    /// A program turned mouse tracking on or off: the arrow and the I-beam trade places (`render/mod.rs:267`).
    private func refreshPointerShape() {
        let takes = interaction?.takesPointer ?? false
        guard takes != pointer.programTakes else { return }
        pointer.programTakes = takes
        window?.invalidateCursorRects(for: self)
    }

    /// Fetch, build, draw: one frame, timed by phase (`FrameStats.swift` says what each is). With `force`,
    /// draws even when the frame changed nothing, and waits for a free frame instead of skipping: the
    /// measuring mode's forced draws. With `transaction` it is shown with the layout being committed (`MetalRenderer.draw`).
    @discardableResult
    public func drawFrame(force: Bool = false, transaction: Bool = false) -> FrameSample {
        let cpuStart = MonotonicClock.threadCPU()
        let a0 = AllocationCounter.allocations
        let t0 = MonotonicClock.now()
        let data = source.frame()
        let t1 = MonotonicClock.now()
        let a1 = AllocationCounter.allocations
        var rebuilt = 0
        do { rebuilt = try renderer.apply(frame: data) } catch {
            FileHandle.standardError.write(Data("bad frame: \(error)\n".utf8))  // text: a diagnostic written to stderr
        }
        let t2 = MonotonicClock.now()
        let a2 = AllocationCounter.allocations
        refreshSelection()
        refreshFound()
        updateScroller()
        if rebuilt > 0 { onContentChanged?() }
        let a3 = AllocationCounter.allocations
        var timings = DrawTimings(outcome: .drawn)
        if force || renderer.needsDraw {
            if let layer = metalLayer {
                timings = renderer.draw(in: layer, blocking: force, withTransaction: transaction)
            } else {
                timings.outcome = .noDrawable
            }
            if timings.outcome == .drawn { statistics.framesDrawn += 1 }
        }
        updateBlink()
        let a4 = AllocationCounter.allocations
        var sample = FrameSample(
            fetchNs: t1 - t0, applyNs: t2 - t1, assembleNs: timings.assembleNs, encodeNs: timings.encodeNs,
            waitNs: timings.waitNs, cpuNs: MonotonicClock.threadCPU() - cpuStart, rowsRebuilt: rebuilt,
            changedRows: renderer.scene.header?.changedRows ?? 0, outcome: timings.outcome)
        sample.allocations = PartAllocations(fetch: a1 - a0, apply: a2 - a1, view: a3 - a2, draw: a4 - a3)
        sampler?.record(sample)
        return sample
    }

    // MARK: screenshots

    /// Write a PNG of what the renderer holds, read back from a texture (no window, no Screen Recording).
    public func writeScreenshot(to url: URL) throws {
        try renderer.renderToImage(padded: true).writePNG(to: url)
    }
}

/// Holds the display link's target weakly: `CADisplayLink` retains its target until invalidated.
@MainActor
private final class LinkProxy: NSObject {
    weak var view: MetalTerminalView?
    init(_ view: MetalTerminalView) { self.view = view }
    @objc func tick(_ link: CADisplayLink) { view?.tick() }
}

/// Whether a wake hop is already queued for the main thread.
private final class WakeRelay: Sendable {
    let queued = Atomic<Bool>(false)
}

/// Collects `FrameSample`s from the view's draws.
@MainActor
public final class FrameSampler {
    public private(set) var samples: [FrameSample] = []
    public init() {}
    func record(_ sample: FrameSample) { samples.append(sample) }
    public func reset() { samples.removeAll() }
}

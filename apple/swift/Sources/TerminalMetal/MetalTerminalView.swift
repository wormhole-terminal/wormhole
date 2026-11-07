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
    /// Records every draw's phases when set (the measuring mode).
    public var sampler: FrameSampler?
    public internal(set) var statistics = ViewStatistics()
    public var isLinkRunning: Bool { link.map { !$0.isPaused } ?? false }
    /// Which Option key sends ESC before the character.
    public var optionAsMeta = OptionAsMeta.left
    /// A grid of this size whatever the view's size (the measuring mode's 200 x 69).
    public var fixedGrid: (cols: Int, rows: Int)?
    /// How long a blinking cursor is on and off; `nil` keeps it steady (xterm's and macOS's own text cursor rate).
    public var blinkInterval: Duration? = .milliseconds(530)
    /// How a link is opened (the system's; tests replace it).
    public var linkOpener: (OpenTarget) -> Void = { LinkOpener.system($0) }
    /// Which parts of the bell happen, and the colour and corner of its glow (the host's tokens).
    public var bell = BellStyle()
    public var bellColor = NSColor.systemOrange
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
    /// The grid the program was last told (`syncSurface`).
    private(set) var gridSize = GridSize(cols: 0, rows: 0)
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
    var hasSelection = false
    var focusReported = false
    let scroller = TerminalScroller()

    public init(source: any FrameSource, configuration: RendererConfiguration, frame: NSRect = NSRect(x: 0, y: 0, width: 960, height: 600)) throws {
        self.source = source
        renderer = try MetalRenderer(configuration: configuration)
        baseFonts = configuration.fonts
        super.init(frame: frame)
        wantsLayer = true
        layerContentsRedrawPolicy = .never
        if let layer = layer as? CAMetalLayer {
            layer.device = renderer.device
            layer.pixelFormat = .bgra8Unorm
            layer.framebufferOnly = true
            layer.isOpaque = true
            layer.maximumDrawableCount = 3
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
    public override var isOpaque: Bool { true }
    private var metalLayer: CAMetalLayer? { layer as? CAMetalLayer }

    /// A new theme's colours: the renderer's palette, and the scroll bar drawn for the new background.
    public func setPalette(_ palette: TerminalPalette) {
        renderer.setPalette(palette)
        scroller.adapt(toBackground: palette.background)
        requestRedraw()
    }

    // MARK: lifecycle

    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window == nil {
            invalidateLink()
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
            FileHandle.standardError.write(Data("source failed to start: \(error)\n".utf8))
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
            requestRedraw()
        }
        // A zoom on its way keeps the grid the program has: it is told once, when the size has arrived.
        guard zoomEase == nil else { return }
        // The legacy scroll bar's gutter is not grid.
        let gridWidth = max(1, Int(((bounds.width - scroller.gutterWidth) * scale).rounded(.down)))
        let (cols, rows) = fixedGrid ?? renderer.grid(fittingPixels: gridWidth, Int(pixels.height))
        let cell = renderer.cellPixels
        let size = GridSize(cols: cols, rows: rows, cellWidth: Float(Double(cell.width) / renderer.scale), cellHeight: Float(Double(cell.height) / renderer.scale))
        guard size != gridSize else { return }
        gridSize = size
        if started {
            source.resize(size)
            requestRedraw()
        } else {
            startSourceIfNeeded()
        }
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
            case .exited(let code): onExit?(code)
            }
        }
        return output
    }

    /// Fetch, build, draw: one frame, timed by phase (`FrameStats.swift` says what each is). With `force`,
    /// draws even when the frame changed nothing, and waits for a free frame instead of skipping: the
    /// measuring mode's forced draws.
    @discardableResult
    public func drawFrame(force: Bool = false) -> FrameSample {
        let cpuStart = MonotonicClock.threadCPU()
        let t0 = MonotonicClock.now()
        let data = source.frame()
        let t1 = MonotonicClock.now()
        var rebuilt = 0
        do { rebuilt = try renderer.apply(frame: data) } catch {
            FileHandle.standardError.write(Data("bad frame: \(error)\n".utf8))
        }
        let t2 = MonotonicClock.now()
        refreshSelection()
        updateScroller()
        var timings = DrawTimings(outcome: .drawn)
        if force || renderer.needsDraw {
            if let layer = metalLayer {
                timings = renderer.draw(in: layer, blocking: force)
            } else {
                timings.outcome = .noDrawable
            }
            if timings.outcome == .drawn { statistics.framesDrawn += 1 }
        }
        updateBlink()
        let sample = FrameSample(
            fetchNs: t1 - t0, applyNs: t2 - t1, assembleNs: timings.assembleNs, encodeNs: timings.encodeNs,
            waitNs: timings.waitNs, cpuNs: MonotonicClock.threadCPU() - cpuStart, rowsRebuilt: rebuilt,
            changedRows: renderer.scene.header?.changedRows ?? 0, outcome: timings.outcome)
        sampler?.record(sample)
        return sample
    }

    // MARK: screenshots

    /// Write a PNG of what the renderer holds, read back from a texture (no window, no Screen Recording).
    public func writeScreenshot(to url: URL) throws {
        try renderer.renderToImage().writePNG(to: url)
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

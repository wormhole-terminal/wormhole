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
    /// Records every draw's phases when set (the measuring mode).
    public var sampler: FrameSampler?
    public private(set) var statistics = ViewStatistics()
    /// Option on the left side sends ESC before the character.
    public var leftOptionAsMeta = true
    /// How long a blinking cursor is on and off; `nil` keeps it steady.
    public var blinkInterval: Duration? = .milliseconds(530)

    private var link: CADisplayLink?
    private var started = false
    private var redrawRequested = true
    private var blinkItem: DispatchWorkItem?
    private var gridSize = GridSize(cols: 0, rows: 0)
    private let wake = WakeRelay()

    // Input method state (see TerminalInput.swift).
    var markedString = ""
    var markedCaret = 0
    var pendingKey: KeyInput?
    var handledByInputMethod = false

    public init(source: any FrameSource, configuration: RendererConfiguration, frame: NSRect = NSRect(x: 0, y: 0, width: 960, height: 600)) throws {
        self.source = source
        renderer = try MetalRenderer(configuration: configuration)
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
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

    public override func makeBackingLayer() -> CALayer { CAMetalLayer() }
    public override var acceptsFirstResponder: Bool { true }
    public override var isFlipped: Bool { true }
    public override var isOpaque: Bool { true }
    private var metalLayer: CAMetalLayer? { layer as? CAMetalLayer }

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
        syncSurface()
        startSourceIfNeeded()
        requestRedraw()
    }

    private func startSourceIfNeeded() {
        guard !started, gridSize.cols > 0 else { return }
        started = true
        let relay = wake
        do {
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
    }

    // MARK: size

    public override func layout() {
        super.layout()
        syncSurface()
    }

    public override func viewDidChangeBackingProperties() {
        super.viewDidChangeBackingProperties()
        syncSurface()
    }

    /// Drawable size and grid size from the view's size. A change of grid goes to the source, which answers
    /// with a full frame of the new size.
    private func syncSurface() {
        let scale = CGFloat(renderer.scale)
        let pixels = CGSize(width: (bounds.width * scale).rounded(.down), height: (bounds.height * scale).rounded(.down))
        guard pixels.width >= 1, pixels.height >= 1 else { return }
        if let layer = metalLayer, layer.drawableSize != pixels {
            layer.drawableSize = pixels
            requestRedraw()
        }
        let (cols, rows) = renderer.grid(fittingPixels: Int(pixels.width), Int(pixels.height))
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
            case .bell: onBell?()
            case .exited(let code): onExit?(code)
            }
        }
        return output
    }

    /// Fetch, build, draw: one frame, timed by phase. With `force`, draws even when the frame changed
    /// nothing (the measuring mode's forced draws).
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
        var outcome = DrawOutcome.drawn
        if force || renderer.needsDraw || renderer.counters.frames <= 1 {
            outcome = metalLayer.map { renderer.draw(in: $0) } ?? .noDrawable
            if outcome == .drawn { statistics.framesDrawn += 1 }
        }
        let t3 = MonotonicClock.now()
        updateBlink()
        let sample = FrameSample(
            fetchNs: t1 - t0, buildNs: t2 - t1, encodeNs: t3 - t2, cpuNs: MonotonicClock.threadCPU() - cpuStart,
            rowsRebuilt: rebuilt, changedRows: renderer.scene.header?.changedRows ?? 0, outcome: outcome)
        sampler?.record(sample)
        return sample
    }

    // MARK: focus and blink

    @objc private func keyStateChanged(_ note: Notification) {
        renderer.setFocused(window?.isKeyWindow ?? false)
        updateBlink()
        requestRedraw()
    }

    public override func becomeFirstResponder() -> Bool {
        renderer.setFocused(true)
        requestRedraw()
        return true
    }

    public override func resignFirstResponder() -> Bool {
        renderer.setFocused(false)
        requestRedraw()
        return true
    }

    /// A blinking cursor in the key window flips every interval with a one-shot timer, armed when the flip
    /// is due and never otherwise.
    private func updateBlink() {
        let blinking = renderer.scene.cursor?.blinking == true && window?.isKeyWindow == true && blinkInterval != nil
        if blinking {
            guard blinkItem == nil, let interval = blinkInterval else { return }
            let item = DispatchWorkItem { [weak self] in
                MainActor.assumeIsolated {
                    guard let self else { return }
                    self.blinkItem = nil
                    self.statistics.blinkFires += 1
                    self.renderer.setCursorBlinkOn(!self.renderer.scene.cursorOn)
                    self.requestRedraw()
                }
            }
            blinkItem = item
            DispatchQueue.main.asyncAfter(deadline: .now() + .nanoseconds(Int(interval.components.attoseconds / 1_000_000_000) + Int(interval.components.seconds) * 1_000_000_000), execute: item)
        } else if blinkItem != nil || !renderer.scene.cursorOn {
            blinkItem?.cancel()
            blinkItem = nil
            renderer.setCursorBlinkOn(true)
            requestRedraw()
        }
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

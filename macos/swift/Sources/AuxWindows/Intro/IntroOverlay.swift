// The intro on screen around one window: a borderless, click-through window over the whole screen the app window is
// on, above the menu bar and the Dock, whose content is a `CAMetalLayer` that the shader draws into every frame.
// The shader leaves the app window's own rectangle clear, so it shows through. While Emerge, Collapse, FTL and the
// wormhole styles play, the app window's layers are scaled about their centre and the window faded in;
// closing the overlay puts both back.

import AppKit
import AuxModel
import QuartzCore

@MainActor
final class IntroOverlay {
    /// The logo's side on the page before it has been measured.
    private static let logoFallback: Float = 96
    /// Drawable pixels per point: the effect is soft, and a whole Retina screen of noise at 2x every frame costs
    /// four times as much for nothing anyone would see.
    private static let renderScale: CGFloat = 1

    let window: NSWindow
    let layer = CAMetalLayer()
    private let renderer: IntroRenderer
    private let parent: NSWindow
    private let setup: IntroSetup
    private let scaled: [CALayer]
    private let saved: (alpha: CGFloat, shadow: Bool)
    private let base: IntroUniforms
    private let origin, screen, windowSize: SIMD2<Float>
    private let landing: () -> SIMD3<Float>?

    init(over parent: NSWindow, setup: IntroSetup, landing: @escaping () -> SIMD3<Float>?) throws {
        guard let screenFrame = (parent.screen ?? NSScreen.main)?.frame else { throw IntroRenderer.SetupError.noDevice }
        renderer = try IntroRenderer()
        (self.parent, self.setup, self.landing) = (parent, setup, landing)
        let parentFrame = parent.frame
        // AppKit's screen coordinates grow upwards; the shader's from the top left.
        origin = [Float(parentFrame.minX - screenFrame.minX), Float(screenFrame.maxY - parentFrame.maxY)]
        windowSize = [Float(parentFrame.width), Float(parentFrame.height)]
        screen = [Float(screenFrame.width), Float(screenFrame.height)]
        base = IntroUniforms(screen: screen, origin: origin, size: windowSize, scale: Float(Self.renderScale), colors: setup.colors)

        window = NSWindow(contentRect: screenFrame, styleMask: .borderless, backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.isOpaque = false
        window.backgroundColor = .clear
        window.hasShadow = false
        window.ignoresMouseEvents = true
        window.collectionBehavior = [.transient, .ignoresCycle, .fullScreenAuxiliary]
        layer.device = renderer.device
        layer.pixelFormat = IntroRenderer.pixelFormat
        layer.isOpaque = false
        layer.contentsScale = Self.renderScale
        layer.drawableSize = CGSize(width: screenFrame.width * Self.renderScale, height: screenFrame.height * Self.renderScale)
        let content = NSView(frame: CGRect(origin: .zero, size: screenFrame.size))
        // A layer-hosting view: the layer is set before `wantsLayer`.
        content.layer = layer
        content.wantsLayer = true
        window.contentView = content

        // The window's layers that grow with it: its content and anything beside it in the frame (titlebar, blur).
        scaled = setup.style == .vortex || setup.style == .glow
            ? [] : (parent.contentView?.superview?.subviews.compactMap(\.layer) ?? [])
        saved = (parent.alphaValue, parent.hasShadow)
        // The window server's shadow would show the window's full size around a small one.
        if !scaled.isEmpty { parent.hasShadow = false }
        if setup.immersive {
            window.level = .statusBar
            window.orderFrontRegardless()
        } else {
            parent.addChildWindow(window, ordered: .above)
        }
        frame(IntroFrame(t: 0, time: 0, lights: IntroStyle.lights(at: 0, start: setup.start), settling: nil))
    }

    /// How the window moves at `t`: what a skip settles it from.
    func motion(at t: Float) -> WindowMotion { setup.style.windowMotion(at: t, start: setup.start) }

    func frame(_ frame: IntroFrame) {
        var motion = motion(at: frame.t)
        var opacity: Float = 1
        if let (from, settled) = frame.settling {
            motion = from.settling(settled)
            opacity = 1 - settled
        }
        moveWindow(motion)
        layer.opacity = opacity
        if setup.lights, let lights = frame.lights {
            for kind in [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton] {
                parent.standardWindowButton(kind)?.alphaValue = CGFloat(lights)
            }
        }
        var uniforms = base.at(setup.style, t: frame.t, time: frame.time, start: setup.start)
        // The shader leaves the window's rectangle clear as far as the window shows.
        (uniforms.winScale, uniforms.winAlpha) = (motion.scale, motion.alpha)
        let (shipped, trail) = uniforms.withShip(
            setup.style, ship: setup.ship, time: frame.time, screen: screen, landing: currentLanding)
        var withShip = shipped
        // After a skip the ship goes with the rest.
        if let (_, settled) = frame.settling { withShip.shipFX.y *= 1 - settled }
        guard let drawable = layer.nextDrawable(),
            let buffer = renderer.encode(into: drawable.texture, uniforms: withShip, trail: trail)
        else { return }
        buffer.present(drawable)
        buffer.commit()
    }

    /// The logo on screen: where the page measured it, or, before it has, a guess at where it sits.
    private var currentLanding: Landing {
        let measured = landing() ?? [windowSize.x / 2, windowSize.y * 0.2, Self.logoFallback]
        return Landing(centre: origin + [measured.x, measured.y], size: measured.z)
    }

    private func moveWindow(_ motion: WindowMotion) {
        guard !scaled.isEmpty else { return }
        parent.alphaValue = CGFloat(motion.alpha) * saved.alpha
        for layer in scaled { layer.transform = Self.scale(CGFloat(motion.scale), about: layer) }
    }

    /// A scale by `s` about the centre of `layer`: a layer scales about its anchor, so the scale is moved back by
    /// what it shifts the centre (`t = (c - a)(1 - s)`).
    private static func scale(_ s: CGFloat, about layer: CALayer) -> CATransform3D {
        let size = layer.bounds.size
        let (dx, dy) = (size.width * (0.5 - layer.anchorPoint.x), size.height * (0.5 - layer.anchorPoint.y))
        return CATransform3DTranslate(CATransform3DMakeScale(s, s, 1), dx * (1 - s) / s, dy * (1 - s) / s, 0)
    }

    /// Closes the overlay and puts the window back.
    func close() {
        for layer in scaled { layer.transform = CATransform3DIdentity }
        if !scaled.isEmpty {
            parent.alphaValue = saved.alpha
            parent.hasShadow = saved.shadow
            parent.invalidateShadow()
        }
        parent.removeChildWindow(window)
        window.orderOut(nil)
        window.close()
    }
}

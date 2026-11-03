import AppKit

/// The window's own look, as the main windows have it: the opacity, the blur behind it and Liquid Glass, when the
/// person ticked Settings under "Also translucent" (and the system doesn't want windows opaque, which the app folds
/// into `translucent`). Layers under the SwiftUI content, bottom first: the blur, a plain fill (under glass without
/// blur, and the opacity's tint without glass) and the glass. The sidebar keeps its own material.
@MainActor final class WindowLook {
    struct State: Equatable {
        var translucent = false, blur = false, glass = false, dark = true
        var opacity = 1.0
    }

    let root = NSView()
    private let effect = NSVisualEffectView()
    private let fill = NSBox()
    private let glassView: NSView?
    private(set) var state = State()
    private weak var window: NSWindow?

    init(window: NSWindow, content: NSView) {
        self.window = window
        glassView = NSClassFromString("NSGlassEffectView").flatMap { ($0 as? NSView.Type)?.init() }
        effect.material = .hudWindow
        effect.blendingMode = .behindWindow
        effect.state = .active
        fill.boxType = .custom
        fill.titlePosition = .noTitle
        fill.borderWidth = 0
        fill.contentViewMargins = .zero
        for v in [effect, fill] + (glassView.map { [$0] } ?? []) + [content] {
            v.translatesAutoresizingMaskIntoConstraints = false
            root.addSubview(v)
            NSLayoutConstraint.activate([
                v.leadingAnchor.constraint(equalTo: root.leadingAnchor), v.trailingAnchor.constraint(equalTo: root.trailingAnchor),
                v.topAnchor.constraint(equalTo: root.topAnchor), v.bottomAnchor.constraint(equalTo: root.bottomAnchor),
            ])
        }
        apply(state, force: true)
    }

    /// Whether this Mac has Liquid Glass (macOS 26).
    var hasGlass: Bool { glassView != nil }

    func apply(_ new: State, force: Bool = false) {
        guard force || new != state, let window else { return }
        state = new
        let glass = new.translucent && new.glass && glassView != nil
        let blur = new.translucent && new.blur
        // Like the main window: the material follows the theme, not the system.
        let appearance = NSAppearance(named: new.dark ? .darkAqua : .aqua)
        effect.appearance = appearance
        glassView?.appearance = appearance
        effect.isHidden = !blur
        glassView?.isHidden = !glass
        // Opaque: the window's own colour. Translucent: that colour at the opacity (under the glass, a fill only
        // when there is no blur to show through).
        window.isOpaque = !new.translucent
        window.backgroundColor = new.translucent ? .clear : .windowBackgroundColor
        let alpha = new.translucent ? (glass ? (blur ? 0 : 1) : new.opacity) : 1
        fill.fillColor = NSColor.windowBackgroundColor.withAlphaComponent(alpha)
        fill.isHidden = alpha <= 0
    }

    /// What is in force, for devctl: the window's own flags and the layers' visibility.
    func readout() -> String {
        let w = window
        let alpha = w?.backgroundColor.usingColorSpace(.sRGB)?.alphaComponent ?? -1
        return "translucent=\(state.translucent) opaque=\(w?.isOpaque ?? true) window_alpha=\(alpha) blur_visible=\(!effect.isHidden) glass_visible=\(!(glassView?.isHidden ?? true)) glass_available=\(hasGlass) fill_alpha=\(fill.isHidden ? 0 : (fill.fillColor.usingColorSpace(.sRGB)?.alphaComponent ?? -1)) opacity=\(state.opacity) dark=\(state.dark)"
    }
}

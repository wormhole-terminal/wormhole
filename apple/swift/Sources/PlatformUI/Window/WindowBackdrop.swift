// The blur and the glass behind a window (`platform/vibrancy.rs`): a plain `NSVisualEffectView` behind the
// content, and with Liquid Glass an `NSGlassEffectView` over it. The material follows the theme, not the system:
// a light theme in Dark Mode sits on the light material, so the appearance is set on the effect views alone, never
// on the window, which keeps following the system (and so keeps reporting its changes, which Match system hears).

import AppKit
import DesignSystem

@MainActor
public final class WindowBackdrop {
    public let blur = NSVisualEffectView()
    public let glass = NSGlassEffectView()
    private weak var window: NSWindow?

    public init() {
        blur.blendingMode = .behindWindow
        blur.state = .active
        blur.material = .underWindowBackground
        blur.autoresizingMask = [.width, .height]
        glass.autoresizingMask = [.width, .height]
        glass.isHidden = true
    }

    /// Puts the views behind `content` in the window's frame view and makes the window see-through.
    public func install(in window: NSWindow) {
        self.window = window
        window.isOpaque = false
        window.backgroundColor = .clear
        guard let frame = window.contentView?.superview else { return }
        for view in [blur, glass] where view.superview == nil {
            view.frame = frame.bounds
            frame.addSubview(view, positioned: .below, relativeTo: window.contentView)
        }
    }

    public func setBlur(enabled: Bool) { blur.isHidden = !enabled }

    /// Liquid Glass over the blur; `radius` is the window's corner, `tint` the theme's glass tint.
    public func setGlass(enabled: Bool, radius: CGFloat = 0, tint: NSColor? = nil) {
        glass.isHidden = !enabled
        glass.cornerRadius = radius
        glass.tintColor = tint
    }

    /// The theme's appearance on the effect views.
    public func follow(_ appearance: ThemeAppearance) {
        let name: NSAppearance.Name = appearance == .dark ? .darkAqua : .aqua
        blur.appearance = NSAppearance(named: name)
        glass.appearance = NSAppearance(named: name)
        // Reduce Transparency draws an opaque material (the system's own rule for a sidebar or HUD).
        blur.material = NSWorkspace.shared.accessibilityDisplayShouldReduceTransparency ? .windowBackground : .underWindowBackground
    }
}

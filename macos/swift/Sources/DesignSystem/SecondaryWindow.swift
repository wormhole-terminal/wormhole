// The Settings, Theme Studio and Session Hosts windows and "Also translucent" (`settings_translucent`,
// `studio_translucent`, `hosts_translucent`): on, the window lets the desktop through as the main window does, at its
// opacity and over its blur (`style::window_surfaces`, `wants_blur`); off, it is opaque. Reduce Transparency and Increase
// Contrast make it opaque either way. The surfaces are the theme's own; this is only what lies under them.

import AppKit
import SwiftUI

/// Which of the three windows a backdrop is for.
public enum SecondaryWindow: Sendable {
    case settings, studio, hosts
}

/// What `Settings` says of the three windows and of the main window's look they borrow.
public struct SecondaryLook: Sendable, Equatable {
    public var settings = true
    public var studio = true
    public var hosts = true
    public var opacity = 0.8
    public var blur = true

    public init() {}

    func translucent(_ window: SecondaryWindow) -> Bool {
        switch window {
        case .settings: settings
        case .studio: studio
        case .hosts: hosts
        }
    }
}

private struct SecondaryBackdrop: ViewModifier {
    let window: SecondaryWindow
    let store: ThemeStore

    func body(content: Content) -> some View {
        let look = store.secondary
        let seesThrough =
            look.translucent(window) && look.opacity < 1 && !store.reduceTransparency && !store.increaseContrast
        content
            .background {
                ZStack {
                    // Opaque under the surfaces unless the desktop is meant to show.
                    if !seesThrough { store.tokens.terminal.background.color }
                    if seesThrough && look.blur { SecondaryBlur() }
                }
                .ignoresSafeArea()
                .accessibilityHidden(true)
            }
            .background(WindowOpacity(opaque: !seesThrough))
    }
}

/// The blur behind a see-through window; its appearance follows the theme, not macOS, as the main window's does.
private struct SecondaryBlur: NSViewRepresentable {
    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = .hudWindow
        view.blendingMode = .behindWindow
        view.state = .active
        return view
    }

    func updateNSView(_ view: NSVisualEffectView, context: Context) {}
}

/// Takes the window's own fill away while the desktop shows through it, and gives it back when the window is opaque.
private struct WindowOpacity: NSViewRepresentable {
    let opaque: Bool

    final class Probe: NSView {
        var wantsOpaque = true { didSet { apply() } }

        override func viewDidMoveToWindow() { apply() }

        private func apply() {
            window?.isOpaque = wantsOpaque
            window?.backgroundColor = wantsOpaque ? .windowBackgroundColor : .clear
        }
    }

    func makeNSView(context: Context) -> Probe { Probe() }

    func updateNSView(_ probe: Probe, context: Context) { probe.wantsOpaque = opaque }
}

extension View {
    /// What lies under this window's surfaces, by its "Also translucent" switch.
    @MainActor public func secondaryBackdrop(_ window: SecondaryWindow, store: ThemeStore = .shared) -> some View {
        modifier(SecondaryBackdrop(window: window, store: store))
    }
}

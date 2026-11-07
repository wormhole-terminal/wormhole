// The terminal area: a plain container whose subview is the selected terminal's Metal view. Other
// terminals' views leave the window (their display links pause with them) but stay alive in `TerminalViews`.
// A switch cross-fades: the new view fades in over the old one, which is removed when it is covered.

import AppKit
import SwiftUI
import WorkspaceModel

struct TerminalHost: NSViewRepresentable {
    let views: TerminalViews
    let selected: TerminalID?
    let ids: [TerminalID]

    func makeNSView(context: Context) -> NSView {
        let host = NSView()
        host.setAccessibilityRole(.group)
        host.setAccessibilityLabel("Terminal")
        return host
    }

    func updateNSView(_ host: NSView, context: Context) {
        views.retain(only: Set(ids))
        guard let id = selected, let wanted = views.view(for: id) else {
            host.subviews.forEach { $0.removeFromSuperview() }
            return
        }
        guard !(wanted.superview === host && host.subviews.last === wanted) else { return }
        wanted.removeFromSuperview()
        wanted.frame = host.bounds
        wanted.autoresizingMask = [.width, .height]
        let animate = !context.environment.accessibilityReduceMotion && !host.subviews.isEmpty
        wanted.alphaValue = animate ? 0 : 1
        host.addSubview(wanted)
        // The keys go to the terminal that was just shown, not to whatever had focus.
        host.window?.makeFirstResponder(wanted)
        guard animate else { return uncover(host) }
        NSAnimationContext.runAnimationGroup { context in
            context.duration = WindowMotion.terminalSwitch
            wanted.animator().alphaValue = 1
        } completionHandler: {
            MainActor.assumeIsolated { uncover(host) }
        }
    }

    /// Everything under the topmost terminal is hidden by it; take it out of the window, opaque again for its
    /// next turn.
    @MainActor private func uncover(_ host: NSView) {
        for view in host.subviews.dropLast() {
            view.removeFromSuperview()
            view.alphaValue = 1
        }
    }
}

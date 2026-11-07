// The window every secondary window shares (`aux_window.rs`): titled, with the title bar transparent over a
// full-size content view so the page reaches the top edge, opaque, themed, one at a time per kind. Esc and ⌘W close
// it (Esc is the window's `cancelOperation`, ⌘W the standard Close item). The title is hidden but Mission Control
// and VoiceOver read it.

import AppKit
import DesignSystem
import SwiftUI

final class AuxNSWindow: NSWindow {
    var closesOnEscape = true

    override func cancelOperation(_ sender: Any?) {
        if closesOnEscape { performClose(sender) } else { super.cancelOperation(sender) }
    }
}

@MainActor
public final class AuxWindowController: NSWindowController {
    private static var open: [String: AuxWindowController] = [:]
    private let key: String

    /// Brings the window of `key` forward if it is open, else makes it. `content` builds the page once.
    @discardableResult
    static func show<Content: View>(
        key: String, title: String, size: CGSize, minSize: CGSize? = nil, resizable: Bool = true,
        closesOnEscape: Bool = true, @ViewBuilder content: () -> Content
    ) -> AuxWindowController {
        if let existing = open[key] {
            existing.showWindow(nil)
            return existing
        }
        let controller = AuxWindowController(
            key: key, title: title, size: size, minSize: minSize, resizable: resizable,
            closesOnEscape: closesOnEscape, page: content().themed())
        open[key] = controller
        controller.showWindow(nil)
        return controller
    }

    private init<Page: View>(
        key: String, title: String, size: CGSize, minSize: CGSize?, resizable: Bool, closesOnEscape: Bool, page: Page
    ) {
        self.key = key
        var mask: NSWindow.StyleMask = [.titled, .closable, .fullSizeContentView]
        if resizable { mask.insert([.resizable, .miniaturizable]) }
        let window = AuxNSWindow(contentRect: CGRect(origin: .zero, size: size), styleMask: mask, backing: .buffered, defer: false)
        window.closesOnEscape = closesOnEscape
        window.title = title
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.isReleasedWhenClosed = false
        if let minSize { window.minSize = minSize }
        let hosting = NSHostingView(rootView: page)
        hosting.sizingOptions = []
        window.contentView = hosting
        window.center()
        super.init(window: window)
        NotificationCenter.default.addObserver(
            forName: NSWindow.willCloseNotification, object: window, queue: .main
        ) { [weak self] _ in
            MainActor.assumeIsolated { self.map { Self.open[$0.key] = nil } }
        }
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

    /// The open window of `key`, for tests and for a window that needs telling something new (trust).
    static func current(_ key: String) -> AuxWindowController? { open[key] }
}

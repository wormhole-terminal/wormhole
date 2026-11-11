// A native context menu on a SwiftUI view, built as it is asked for: for a menu SwiftUI's `.contextMenu` cannot hold
// (a row of colour dots, app icons) or one the app builds from the core's facts. The catcher lies over the view and
// takes only the secondary click (right, or ⌃-click); every other event passes through to the view under it.

import AppKit
import SwiftUI

extension View {
    /// Right-click and ⌃-click open the menu `make` returns (nil: none).
    public func nativeContextMenu(_ make: @escaping @MainActor () -> NSMenu?) -> some View {
        overlay { NativeMenuCatcher(make: make) }
    }
}

/// What a check that asks a view for its menu sets while it looks for the view (it has no event to show).
@MainActor
public enum NativeContextMenu {
    public static var probing = false
}

private struct NativeMenuCatcher: NSViewRepresentable {
    let make: @MainActor () -> NSMenu?

    func makeNSView(context: Context) -> CatcherView { CatcherView() }
    func updateNSView(_ view: CatcherView, context: Context) { view.make = make }
}

private final class CatcherView: NSView {
    var make: @MainActor () -> NSMenu? = { nil }

    override func hitTest(_ point: NSPoint) -> NSView? {
        guard !NativeContextMenu.probing else { return super.hitTest(point) }
        guard let event = NSApp.currentEvent else { return nil }
        let secondary = event.type == .rightMouseDown || event.type == .rightMouseUp
            || (event.type == .leftMouseDown && event.modifierFlags.contains(.control))
        return secondary ? super.hitTest(point) : nil
    }

    override func menu(for event: NSEvent) -> NSMenu? { make() }
}

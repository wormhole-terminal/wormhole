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

/// A native menu item that runs a closure: for a menu built where its actions are (a view's own verbs).
@MainActor
public final class ActionMenuItem: NSMenuItem {
    private let run: () -> Void

    public init(_ title: String, enabled: Bool = true, checked: Bool = false, run: @escaping () -> Void) {
        self.run = run
        super.init(title: title, action: #selector(pick), keyEquivalent: "")
        target = self
        isEnabled = enabled
        state = checked ? .on : .off
    }

    @available(*, unavailable)
    public required init(coder: NSCoder) { fatalError("not decoded") }

    @objc private func pick() { run() }
}

extension EnvironmentValues {
    /// Shows a menu the keys opened under a frame (in the window, from its top left); a test instance keeps it unshown.
    @Entry public var showKeyMenu: @MainActor (NSMenu, CGRect) -> Void = { _, _ in }
}

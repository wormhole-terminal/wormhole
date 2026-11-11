// While an injected event is routed, the window acts as the key window of the active app, as it would be for a person's
// click or key: AppKit's `keyWindow` and the window's `isKeyWindow` say so, and the views on the way to a press accept it
// as a first click (`acceptsFirstMouse(for:)`, which the window and its gesture recognizers ask, and SwiftUI's views answer
// for themselves). Outside an injected event every answer is AppKit's own. The methods are patched on first use, so an
// instance that is never driven runs AppKit unchanged.

import AppKit
import ObjectiveC

@MainActor
enum InFront {
    private static weak var window: NSWindow?
    private static var patched: Set<OpaquePointer> = []

    /// Runs `body` (which hands an event to AppKit) with `window` in front and the views from `view` up accepting first clicks.
    static func during(_ window: NSWindow, along view: NSView? = nil, _ body: () -> Void) {
        patchOnce()
        for view in sequence(first: view, next: { $0?.superview }).compactMap({ $0 }) { patchFirstMouse(object_getClass(view)) }
        self.window = window
        defer { self.window = nil }
        body()
    }

    private static func patchOnce() {
        typealias KeyWindow = @convention(c) (NSApplication, Selector) -> NSWindow?
        typealias IsKey = @convention(c) (NSWindow, Selector) -> Bool
        let keyWindow = #selector(getter: NSApplication.keyWindow), isKey = #selector(getter: NSWindow.isKeyWindow)
        guard let appMethod = class_getInstanceMethod(NSApplication.self, keyWindow), let windowMethod = class_getInstanceMethod(NSWindow.self, isKey),
            patched.insert(appMethod).inserted, patched.insert(windowMethod).inserted
        else { return }
        let appOriginal = unsafeBitCast(method_getImplementation(appMethod), to: KeyWindow.self)
        let windowOriginal = unsafeBitCast(method_getImplementation(windowMethod), to: IsKey.self)
        // why: off the main thread there is no injected event, and AppKit's own answer stands.
        let app: @convention(block) (NSApplication) -> NSWindow? = { receiver in
            (Thread.isMainThread ? MainActor.assumeIsolated { window } : nil) ?? appOriginal(receiver, keyWindow)
        }
        let key: @convention(block) (NSWindow) -> Bool = { receiver in
            (Thread.isMainThread && MainActor.assumeIsolated { receiver === window }) || windowOriginal(receiver, isKey)
        }
        method_setImplementation(appMethod, imp_implementationWithBlock(app))
        method_setImplementation(windowMethod, imp_implementationWithBlock(key))
    }

    private static func patchFirstMouse(_ cls: AnyClass?) {
        typealias Answer = @convention(c) (NSView, Selector, NSEvent?) -> Bool
        let selector = #selector(NSView.acceptsFirstMouse(for:))
        guard let method = class_getInstanceMethod(cls, selector), patched.insert(method).inserted else { return }
        let original = unsafeBitCast(method_getImplementation(method), to: Answer.self)
        let replacement: @convention(block) (NSView, NSEvent?) -> Bool = { view, event in
            // why: AppKit asks on the main thread only, where `during` runs too.
            MainActor.assumeIsolated { window != nil } || original(view, selector, event)
        }
        method_setImplementation(method, imp_implementationWithBlock(replacement))
    }
}

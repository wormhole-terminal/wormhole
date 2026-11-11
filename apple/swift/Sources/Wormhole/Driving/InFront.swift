// While an injected event is routed, the window acts as the key window of the active app, as it would be for a person's
// click or key: AppKit's `keyWindow` and the window's `isKeyWindow` say so, and the views on the way to a press accept it
// as a first click (`acceptsFirstMouse(for:)`, which the window and its gesture recognizers ask, and SwiftUI's views answer
// for themselves). The methods are replaced only while the event is routed and put back as it returns, so outside an injected
// event every answer is AppKit's own, and an instance that is never driven runs AppKit unchanged.

import AppKit
import ObjectiveC

@MainActor
enum InFront {
    private static weak var window: NSWindow?
    /// The methods this replaces, each with the block that stands in for it while an event is routed.
    private static var replacements: [(method: Method, block: IMP)] = []
    private static var known: Set<OpaquePointer> = []

    /// Runs `body` (which hands an event to AppKit) with `window` in front and the views from `view` up accepting first clicks.
    static func during(_ window: NSWindow, along view: NSView? = nil, _ body: () -> Void) {
        prepareKeyWindow()
        for view in sequence(first: view, next: { $0?.superview }).compactMap({ $0 }) { prepareFirstMouse(object_getClass(view)) }
        let outer = self.window
        self.window = window
        // Each method goes back to what it was when the event is through (the IMP that `method_setImplementation` replaced).
        let originals = replacements.map { method_setImplementation($0.method, $0.block) }
        defer {
            for (swap, original) in zip(replacements, originals) { method_setImplementation(swap.method, original) }
            self.window = outer
        }
        body()
    }

    private static func prepareKeyWindow() {
        typealias KeyWindow = @convention(c) (NSApplication, Selector) -> NSWindow?
        typealias IsKey = @convention(c) (NSWindow, Selector) -> Bool
        let keyWindow = #selector(getter: NSApplication.keyWindow), isKey = #selector(getter: NSWindow.isKeyWindow)
        guard let appMethod = class_getInstanceMethod(NSApplication.self, keyWindow), let windowMethod = class_getInstanceMethod(NSWindow.self, isKey),
            known.insert(appMethod).inserted, known.insert(windowMethod).inserted
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
        replacements.append((appMethod, imp_implementationWithBlock(app)))
        replacements.append((windowMethod, imp_implementationWithBlock(key)))
    }

    private static func prepareFirstMouse(_ cls: AnyClass?) {
        typealias Answer = @convention(c) (NSView, Selector, NSEvent?) -> Bool
        let selector = #selector(NSView.acceptsFirstMouse(for:))
        guard let method = class_getInstanceMethod(cls, selector), known.insert(method).inserted else { return }
        let original = unsafeBitCast(method_getImplementation(method), to: Answer.self)
        let replacement: @convention(block) (NSView, NSEvent?) -> Bool = { view, event in
            // why: AppKit asks on the main thread only, where `during` runs too.
            MainActor.assumeIsolated { window != nil } || original(view, selector, event)
        }
        replacements.append((method, imp_implementationWithBlock(replacement)))
    }
}

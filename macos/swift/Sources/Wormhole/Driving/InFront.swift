// While an injected event is routed, the window acts as the key window of the active app, as it would be for a person's
// click or key: AppKit's `keyWindow` and the window's `isKeyWindow` say so, and the views on the way to a press accept it
// as a first click (`acceptsFirstMouse(for:)`, which the window and its gesture recognizers ask, and SwiftUI's views answer
// for themselves). The methods are replaced only while the event is routed and put back as it returns, so outside an injected
// event every answer is AppKit's own, and an instance that is never driven runs AppKit unchanged.
//
// A key goes one step further (`key`): the application hands a key to the window it holds for key itself, whatever the event
// names and whatever `keyWindow` answers. In a test instance that is the first workspace window: a key sent to a second
// window was typed into the first one's terminal, and a ⌘ key sent to Settings was asked of the workspace's views (a shortcut
// recorder in Settings heard nothing). So while a key is routed, the window the application picks hands it on to the window
// it was sent to: its monitors, its order (the views' `performKeyEquivalent`, the menus, then `keyDown`) and the window's own
// dispatch all run as for a real key.

import AppKit
import ObjectiveC

@MainActor
enum InFront {
    private static weak var window: NSWindow?
    /// Whether the event being routed is a key: the window the application picks for it hands it on to `window`.
    private static var routesKey = false
    /// The methods this replaces, each with the block that stands in for it while an event is routed.
    private static var replacements: [(method: Method, block: IMP)] = []
    private static var known: Set<OpaquePointer> = []

    /// Runs `body` (which hands an event to AppKit) with `window` in front and the views from `view` up accepting first clicks.
    /// `key`: `body` hands the application a key event meant for `window` (see the head of this file).
    static func during(_ window: NSWindow, along view: NSView? = nil, key: Bool = false, _ body: () -> Void) {
        prepareKeyWindow()
        prepareKeyDispatch()
        let outerKey = routesKey
        routesKey = key
        defer { routesKey = outerKey }
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

    /// `NSWindow.sendEvent(_:)` and `performKeyEquivalent(with:)`, the two calls the application makes on the window it picks
    /// for a key: while a key is routed, another window than the one it was sent to passes the call on to that one (by a
    /// message, so a window class's own override runs).
    private static func prepareKeyDispatch() {
        typealias Send = @convention(c) (NSWindow, Selector, NSEvent) -> Void
        typealias Equivalent = @convention(c) (NSWindow, Selector, NSEvent) -> Bool
        let send = #selector(NSWindow.sendEvent(_:)), equivalent = #selector(NSWindow.performKeyEquivalent(with:))
        guard let sendMethod = class_getInstanceMethod(NSWindow.self, send), let equivalentMethod = class_getInstanceMethod(NSWindow.self, equivalent),
            known.insert(sendMethod).inserted, known.insert(equivalentMethod).inserted
        else { return }
        let sendOriginal = unsafeBitCast(method_getImplementation(sendMethod), to: Send.self)
        let equivalentOriginal = unsafeBitCast(method_getImplementation(equivalentMethod), to: Equivalent.self)
        // The window a routed key is meant for, when that is not `receiver`: a sheet over the window it was sent to takes the
        // window's keys, as it does for a person.
        // why: AppKit dispatches events on the main thread only, where `during` runs too.
        func meant(for receiver: NSWindow) -> NSWindow? {
            guard Thread.isMainThread else { return nil }
            return MainActor.assumeIsolated {
                guard routesKey, let window else { return nil }
                let target = window.attachedSheet ?? window
                return target === receiver ? nil : target
            }
        }
        let sends: @convention(block) (NSWindow, NSEvent) -> Void = { receiver, event in
            if let target = meant(for: receiver) { target.sendEvent(event) } else { sendOriginal(receiver, send, event) }
        }
        let asks: @convention(block) (NSWindow, NSEvent) -> Bool = { receiver, event in
            if let target = meant(for: receiver) { return target.performKeyEquivalent(with: event) }
            return equivalentOriginal(receiver, equivalent, event)
        }
        replacements.append((sendMethod, imp_implementationWithBlock(sends)))
        replacements.append((equivalentMethod, imp_implementationWithBlock(asks)))
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

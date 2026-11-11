// The pointer of the control folder: NSEvents made in the process and given to the window (`sendEvent`), which routes
// them as it does the window server's: hit-testing, the pressed view keeping the drag, gesture recognizers, SwiftUI's
// gestures. Nothing goes through the window server, so the real pointer, the other apps and the window's place behind
// them are untouched. Entering and leaving views is told to the views directly (tracking areas are the window
// server's). A right-click asks the view for its menu (`menu(for:)`, which SwiftUI's `.contextMenu` answers too) and
// keeps it unshown, since a shown menu would hold the app in AppKit's tracking loop: `menu` lists it, `menu-pick`
// presses an item.
//
// A background instance is never the active app, so AppKit would treat every click as a window's first click and every
// key as going nowhere; while an injected event is routed the window acts as the key window of an active app (`InFront`).

import AppKit
import DesignSystem

@MainActor
final class Injector {
    private var pressedOn: String?
    /// The tracking areas the pointer is inside, to tell them when it leaves.
    private var inside: [NSTrackingArea] = []
    private var eventNumber = 0
    /// The last menu a right-click found, for `menu` and `menu-pick`.
    private(set) var menu: NSMenu?
    /// The window the pointer acts on by title (Settings, the Studio); nil is the workspace's in front.
    var windowTitle: String?

    /// A point given from the window content's top left, in points (as `window-shot` at 1x), in the window's own space.
    func location(_ x: Double, _ y: Double, in window: NSWindow) -> NSPoint {
        guard let content = window.contentView else { return NSPoint(x: x, y: window.frame.height - y) }
        return content.convert(NSPoint(x: x, y: content.isFlipped ? y : content.bounds.height - y), to: nil)
    }

    /// The deepest view at a window point, as NSWindow's own dispatch finds it.
    func view(at point: NSPoint, in window: NSWindow) -> NSView? {
        // The window's frame view has no superview: its hit test takes its own coordinates.
        guard let frame = window.contentView?.superview ?? window.contentView else { return nil }
        return frame.hitTest(frame.convert(point, from: nil))
    }

    /// A press there. A view that follows a press in a loop of its own (a window's edge, a text field, a scroller) reads
    /// what comes next from AppKit's queue and would wait there for a release no script sends, so the rest of the gesture
    /// is queued ahead of the press: the moves of `path` and the release at its end. SwiftUI drains that queue too, so a
    /// press that comes back with the queue emptied had its whole gesture; one that finds it untouched is held, and the
    /// queued events are taken back. `hold` presses without queuing anything (`mouse-down`), and is refused over a view
    /// that would track it in a loop.
    @discardableResult
    func press(
        _ point: NSPoint, in window: NSWindow, clicks: Int = 1, flags: NSEvent.ModifierFlags = [], path: [NSPoint] = [], hold: Bool = false
    ) -> String {
        guard let target = view(at: point, in: window) else { return "nothing at \(point)" }
        if hold, target is NSControl || target === window.contentView?.superview {
            return "error: \(name(of: target)) follows a press in a loop of its own; use drag or click"
        }
        move(point, in: window, flags: flags)
        let ahead = hold ? [] : path.map { mouse(.leftMouseDragged, $0, window, flags: flags, number: .max) }
            + [mouse(.leftMouseUp, path.last ?? point, window, clicks: clicks, flags: flags, number: .max)]
        ahead.forEach { NSApp.postEvent($0, atStart: false) }
        send(.leftMouseDown, point, window, clicks: clicks, flags: flags)
        var left = 0
        while let queued = NSApp.nextEvent(matching: [.leftMouseDragged, .leftMouseUp], until: .distantPast, inMode: .default, dequeue: false),
            queued.eventNumber == .max
        {
            _ = NSApp.nextEvent(matching: [.leftMouseDragged, .leftMouseUp], until: .distantPast, inMode: .default, dequeue: true)
            left += 1
        }
        guard left < ahead.count else {
            pressedOn = name(of: target)
            return "down on \(name(of: target))"
        }
        return "down on \(name(of: target)), the gesture played to its release"
    }

    /// Whether the last press is still held (false after a view tracked the whole gesture in its own loop).
    var isPressed: Bool { pressedOn != nil }

    /// The pointer moved: a drag while pressed, else a move (entering and leaving views on the way).
    @discardableResult
    func move(_ point: NSPoint, in window: NSWindow, flags: NSEvent.ModifierFlags = []) -> String {
        if let pressedOn {
            send(.leftMouseDragged, point, window, flags: flags)
            return "dragged \(pressedOn)"
        }
        let target = view(at: point, in: window)
        let crossed = cross(to: point, from: target, in: window, flags: flags)
        return "over \(target.map(name) ?? "nothing") \(crossed)"
    }

    /// What the window server does as the pointer moves: each tracking area on the way to the view under it hears it enter,
    /// move and leave, as the area's own events (a plain mouse event would make an area's owner throw on `trackingArea`).
    @discardableResult
    private func cross(to point: NSPoint, from target: NSView?, in window: NSWindow, flags: NSEvent.ModifierFlags) -> String {
        let now = sequence(first: target, next: { $0?.superview }).compactMap { $0 }.flatMap { view in
            view.trackingAreas.filter { area in
                (area.options.contains(.inVisibleRect) ? view.visibleRect : area.rect).contains(view.convert(point, from: nil))
            }
        }
        func tell(_ type: NSEvent.EventType, _ area: NSTrackingArea) {
            let event = NSEvent.enterExitEvent(
                with: type, location: point, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
                windowNumber: window.windowNumber, context: nil, eventNumber: 0,
                trackingNumber: Int(bitPattern: Unmanaged.passUnretained(area).toOpaque()), userData: nil)
            guard let event, let owner = area.owner as? NSResponder else { return }
            type == .mouseEntered ? owner.mouseEntered(with: event) : owner.mouseExited(with: event)
        }
        for area in inside where !now.contains(where: { $0 === area }) && area.options.contains(.mouseEnteredAndExited) { tell(.mouseExited, area) }
        for area in now where !inside.contains(where: { $0 === area }) && area.options.contains(.mouseEnteredAndExited) { tell(.mouseEntered, area) }
        inside = now
        let moved = mouse(.mouseMoved, point, window, flags: flags)
        let owners = now.filter { $0.options.contains(.mouseMoved) }.compactMap { $0.owner as? NSResponder }
        owners.forEach { $0.mouseMoved(with: moved) }
        // The view under the pointer hears the move too, as the window's own dispatch tells it.
        if let target, !owners.contains(where: { $0 === target }) { target.mouseMoved(with: moved) }
        return "areas=\(now.count) owners=\(owners.count)"

    }

    func release(_ point: NSPoint, in window: NSWindow, clicks: Int = 1, flags: NSEvent.ModifierFlags = []) -> String {
        let target = pressedOn ?? view(at: point, in: window).map(name) ?? "nothing"
        pressedOn = nil
        send(.leftMouseUp, point, window, clicks: clicks, flags: flags)
        return "up on \(target)"
    }

    /// The wheel over a point: `dy` points down (negative up) and `dx` points right (negative left), as a trackpad's precise deltas.
    func scroll(_ point: NSPoint, in window: NSWindow, dy: Double, dx: Double = 0) -> String {
        guard let target = view(at: point, in: window) else { return "nothing at \(point)" }
        let screen = window.convertPoint(toScreen: point)
        let height = NSScreen.screens.first?.frame.height ?? 0
        guard let cg = CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 2, wheel1: Int32(-dy), wheel2: Int32(-dx), wheel3: 0)
        else { return "error: no wheel event" }
        cg.location = CGPoint(x: screen.x, y: height - screen.y)
        cg.setIntegerValueField(.mouseEventWindowUnderMousePointer, value: Int64(window.windowNumber))
        cg.setIntegerValueField(.mouseEventWindowUnderMousePointerThatCanHandleThisEvent, value: Int64(window.windowNumber))
        guard let event = NSEvent(cgEvent: cg) else { return "error: no wheel event" }
        target.scrollWheel(with: event)
        return "scrolled \(name(of: target))"
    }

    /// A menu opened by a key (⌃↩ on a row), kept unshown like a right-click's.
    func keep(_ menu: NSMenu) { self.menu = menu }

    /// What a right-click (or a Control-click) opens there, kept unshown; nil when no view there has a menu, and then the
    /// click is the view's own (a program that takes the mouse).
    func contextMenu(_ point: NSPoint, in window: NSWindow) -> NSMenu? {
        let event = mouse(.rightMouseDown, point, window)
        // A native menu's catcher answers a hit test only for a secondary click; no event is current here.
        NativeContextMenu.probing = true
        let hit = view(at: point, in: window)
        NativeContextMenu.probing = false
        for view in sequence(first: hit, next: { $0?.superview }).compactMap({ $0 }) {
            guard let menu = view.menu(for: event) else { continue }
            menu.delegate?.menuNeedsUpdate?(menu)
            menu.update()
            self.menu = menu
            return menu
        }
        send(.rightMouseDown, point, window)
        send(.rightMouseUp, point, window)
        return nil
    }

    /// Presses the menu's item as a click would: by number (from 1, as `menu` lists them) or title, `A > B` for a submenu's.
    func pick(_ choice: String) -> String {
        guard var menu else { return "error: no menu" }
        let path = choice.components(separatedBy: " > ")
        for (depth, step) in path.enumerated() {
            let items = menu.items.filter { !$0.isSeparatorItem }
            let found = Int(step).flatMap { items.indices.contains($0 - 1) ? items[$0 - 1] : nil } ?? items.first { $0.title == step }
            guard let item = found else { return "error: no item \(step)" }
            if depth < path.count - 1 {
                guard let submenu = item.submenu else { return "error: \(step) has no submenu" }
                menu = submenu
                continue
            }
            guard item.isEnabled else { return "error: \(item.title) is off" }
            self.menu = nil
            menu.performActionForItem(at: menu.index(of: item))
            return "picked \(item.title)"
        }
        return "error: nothing picked"
    }

    /// Dismisses the menu as Esc would; false when there was none.
    func closeMenu() -> Bool {
        defer { menu = nil }
        return menu != nil
    }

    private func send(_ type: NSEvent.EventType, _ point: NSPoint, _ window: NSWindow, clicks: Int = 1, flags: NSEvent.ModifierFlags = []) {
        InFront.during(window, along: view(at: point, in: window)) { window.sendEvent(mouse(type, point, window, clicks: clicks, flags: flags)) }
    }

    /// `number` marks the events queued ahead of a press, so they can be told from any others.
    private func mouse(
        _ type: NSEvent.EventType, _ point: NSPoint, _ window: NSWindow, clicks: Int = 1, flags: NSEvent.ModifierFlags = [], number: Int? = nil
    ) -> NSEvent {
        // A press and its release share a number, as the window server's do.
        if number == nil, type == .leftMouseDown || type == .rightMouseDown { eventNumber += 1 }
        let pressing = [.leftMouseDown, .leftMouseDragged, .rightMouseDown].contains(type)
        // `mouseEvent` fails only for a type that is not a mouse event's; every caller passes one.
        return NSEvent.mouseEvent(
            with: type, location: point, modifierFlags: flags, timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: window.windowNumber, context: nil, eventNumber: number ?? eventNumber, clickCount: type == .mouseMoved ? 0 : clicks,
            pressure: pressing ? 1 : 0)!
    }

    private func name(of view: NSView) -> String { String(describing: type(of: view)) }
}

// The control folder's file drag and window close (`WORMHOLE_DEVCTL`), GPUI's `devctl/input.rs` and `sessions.rs`: `drag <x> <y> <path>`
// hovers a file over the window at a point, as when dragging from Finder (the terminal tints), `drag-end` abandons it, and
// `close-window` closes the window the command acts on as its red button does. The drop itself is `drop-files <path>` (`ControlScripted`,
// which pastes the paths as a drop does). The hover is made with a stand-in for the dragging session the system would hand the view
// (`StandInDrag`), so the views' own `draggingEntered` and `draggingExited` run; the real pointer and Finder are never involved.

import AppKit
import Foundation

/// What `NSDraggingInfo` says for a file hovering at a point: the file on a board of its own (never the system's), copy as the operation.
private final class StandInDrag: NSObject, NSDraggingInfo {
    let draggingPasteboard: NSPasteboard
    private let location: NSPoint
    private weak var window: NSWindow?

    @MainActor init(path: String, at location: NSPoint, in window: NSWindow) {
        draggingPasteboard = NSPasteboard.withUniqueName()
        draggingPasteboard.writeObjects([URL(fileURLWithPath: path) as NSURL])
        (self.location, self.window) = (location, window)
    }

    var draggingDestinationWindow: NSWindow? { window }
    var draggingSourceOperationMask: NSDragOperation { .copy }
    var draggingLocation: NSPoint { location }
    var draggedImageLocation: NSPoint { location }
    var draggedImage: NSImage? { nil }
    var draggingSource: Any? { nil }
    var draggingSequenceNumber: Int { 1 }
    var draggingFormation: NSDraggingFormation = .default
    var animatesToDestination = false
    var numberOfValidItemsForDrop = 1
    var springLoadingHighlight: NSSpringLoadingHighlight { .none }
    func resetSpringLoading() {}
    func slideDraggedImage(to screenPoint: NSPoint) {}
    override func namesOfPromisedFilesDropped(atDestination dropDestination: URL) -> [String]? { nil }

    func enumerateDraggingItems(
        options enumOpts: NSDraggingItemEnumerationOptions = [], for view: NSView?, classes classArray: [AnyClass],
        searchOptions: [NSPasteboard.ReadingOptionKey: Any] = [:], using block: @escaping (NSDraggingItem, Int, UnsafeMutablePointer<ObjCBool>) -> Void
    ) {}
}

/// The view a file is hovering over, until `drag-end`.
@MainActor private enum Hovering { static var over: (view: NSView, info: StandInDrag)? }

extension AppController {
    /// False when `command` is not one of these.
    func runDrag(command: String, argument: String) -> Bool {
        switch command {
        case "drag":
            let words = argument.split(separator: " ", maxSplits: 2).map(String.init)
            guard words.count == 3, let x = Double(words[0]), let y = Double(words[1]), let window else { reply("error: drag <x> <y> <path>"); return true }
            let point = injector.location(x, y, in: window)
            // The nearest view up from the one under the point that takes drags (a terminal registers for files and text).
            let target = injector.view(at: point, in: window).flatMap { under in
                sequence(first: under, next: { $0.superview }).first { !$0.registeredDraggedTypes.isEmpty }
            }
            guard let target else { reply("drag \(argument): nothing under the point takes a drop"); return true }
            let info = StandInDrag(path: words[2], at: point, in: window)
            if let (previous, old) = Hovering.over { previous.draggingExited(old) }
            let operation = target.draggingEntered(info)
            Hovering.over = (target, info)
            reply("drag \(argument): \(type(of: target)) \(operation == .copy ? "copy" : "refused")")
        case "drag-end":
            guard let (view, info) = Hovering.over else { reply("error: nothing is being dragged"); return true }
            view.draggingExited(info)
            Hovering.over = nil
            reply("drag-end")
        case "close-window":
            // As the red button: it asks first while something runs in the window (a test instance holds the question for
            // `question confirm|cancel`), and the last window ends the app instead (saved).
            guard let window else { reply("error: no window"); return true }
            reply("close-window \(front.id)")
            // From the run loop, as a click comes, not from this command's job of the main queue: `performClose` waits a moment
            // in a run loop of its own, a quit asked for meanwhile (the last window) would start inside it, and a run loop
            // inside a main-queue job serves no other, so the quit's question would never be put.
            RunLoop.main.perform(inModes: [.common]) { MainActor.assumeIsolated { window.performClose(nil) } }
        default: return false
        }
        return true
    }
}

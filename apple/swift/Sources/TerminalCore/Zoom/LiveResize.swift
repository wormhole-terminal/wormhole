// A window dragged to another size (`view::motion::live_resize`): what the program in its terminal is told meanwhile, where
// each step of the drag lands and the strip a window off the cell grid leaves when the drag ends. The rules are the core's:
// `UniFFISource` installs them over the FFI, and without them (a build with no Rust) nothing snaps and every size is told
// at once. Asking AppKit whether a drag is on (`NSWindow.inLiveResize`) and hearing it end are the window's.

import CoreGraphics
import Foundation
import Synchronization

public struct ResizeStep: Equatable, Sendable {
    /// Tell the program the size the window gives it now.
    public var apply: Bool
    /// Draw this terminal again after this long: a size is held back and this is when it is due.
    public var wakeIn: TimeInterval?

    public init(apply: Bool, wakeIn: TimeInterval? = nil) { (self.apply, self.wakeIn) = (apply, wakeIn) }
}

/// One terminal's gate (the core's `ResizeGate`): a size told during a live resize at most every 100 ms, the last always.
public protocol ResizeGating: AnyObject, Sendable {
    /// The frame now: `live` says the window is being dragged, `differs` that the size the window gives the terminal
    /// is not the one the program has. The caller does what `apply` says; the gate counts it as done.
    func step(live: Bool, differs: Bool) -> ResizeStep
}

/// The core's live-resize rules, as the window and the terminal view ask them.
public protocol ResizeRules: Sendable {
    func snapped(_ proposed: CGSize, from: CGSize, spare: CGSize, cell: CGSize, least: CGSize) -> CGSize
    func settle(spare: CGSize, cell: CGSize) -> CGSize?
    func settledFrame(_ frame: CGRect, strip: CGSize, visible: CGRect, heldLeft: Bool, heldTop: Bool) -> CGRect?
    func makeGate() -> any ResizeGating
}

private struct NoRules: ResizeRules {
    func snapped(_ proposed: CGSize, from: CGSize, spare: CGSize, cell: CGSize, least: CGSize) -> CGSize { proposed }
    func settle(spare: CGSize, cell: CGSize) -> CGSize? { nil }
    func settledFrame(_ frame: CGRect, strip: CGSize, visible: CGRect, heldLeft: Bool, heldTop: Bool) -> CGRect? { nil }
    func makeGate() -> any ResizeGating { Always() }

    final class Always: ResizeGating {
        func step(live: Bool, differs: Bool) -> ResizeStep { ResizeStep(apply: differs) }
    }
}

public enum ResizeSteps {
    private static let installed = Mutex<any ResizeRules>(NoRules())

    /// Makes `rules` the live-resize rules of every window and terminal.
    public static func install(_ rules: any ResizeRules) { installed.withLock { $0 = rules } }
    static var rules: any ResizeRules { installed.withLock { $0 } }

    /// The size a window dragged to `proposed` is held to, so its terminal's area keeps whole cells: `spare` is the strip the
    /// area has beyond its grid now, counted from `current` (the size where the drag began). Never under `least`.
    public static func snapped(_ proposed: CGSize, from current: CGSize, spare: CGSize, cell: CGSize, least: CGSize) -> CGSize {
        rules.snapped(proposed, from: current, spare: spare, cell: cell, least: least)
    }

    /// How far the window is to shrink, across and down, so it holds whole cells with no strip left; nil when nothing is.
    public static func settle(spare: CGSize, cell: CGSize) -> CGSize? { rules.settle(spare: spare, cell: cell) }

    /// The frame (AppKit's space) the window is to take off `strip`, nil for a tiled or screen-filling window. The edges the
    /// person did not hold stay where they are.
    public static func settledFrame(_ frame: CGRect, strip: CGSize, visible: CGRect, heldLeft: Bool, heldTop: Bool) -> CGRect? {
        rules.settledFrame(frame, strip: strip, visible: visible, heldLeft: heldLeft, heldTop: heldTop)
    }
}

/// A terminal view's own gate, from the installed rules.
public final class ResizeGate: Sendable {
    private let gate = ResizeSteps.rules.makeGate()

    public init() {}

    public func step(live: Bool, differs: Bool) -> ResizeStep { gate.step(live: live, differs: differs) }
}

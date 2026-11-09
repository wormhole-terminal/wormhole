// What the app's windows need kept beyond one window's own state: which windows there are, what each held when the app
// last ran, the ids they share, where each was. The core keeps them (`WindowsBridge`); `MemoryWindows` stands in for the
// sample and the tests. The shell decides nothing here: it asks for the windows to open, opens one for each and tells
// the backend when something about one changed.

import Foundation
import WorkspaceModel

public typealias WindowID = Int

/// A window saved at the last run, in the order to open them (the one that was in front last is last).
public struct SavedWindow: Sendable, Equatable {
    public var id: WindowID
    public var frame: WindowFrame?
    public var terminals: Int

    public init(id: WindowID, frame: WindowFrame?, terminals: Int) {
        (self.id, self.frame, self.terminals) = (id, frame, terminals)
    }
}

/// A window opened for something that asked: its id and its first terminal's are known before it is on screen.
public struct PlannedWindow: Sendable, Equatable {
    public var window: WindowID
    public var terminal: TerminalID

    public init(window: WindowID, terminal: TerminalID) {
        (self.window, self.terminal) = (window, terminal)
    }
}

@MainActor
public protocol WindowsBackend: AnyObject {
    /// The windows saved, to open in this order.
    func saved() -> [SavedWindow]
    /// An id for a window that starts empty (a first launch) or is given terminals that move into it.
    func allocWindowId() -> WindowID
    /// A window laid out like `from` (its columns and panel, no groups) with one new terminal in `folder`; its workspace
    /// starts from the plan when it asks to be restored.
    func plan(from: (any WorkspaceBackend)?, folder: String?) -> PlannedWindow
    /// Write what `workspace` holds as window `window`'s record, with where the window is. False when it failed.
    @discardableResult func save(window: WindowID, workspace: any WorkspaceBackend, frame: WindowFrame?) -> Bool
    /// Which window is in front, so the next launch brings the same one forward.
    func saveFront(_ window: WindowID)
    /// The person closed the window: it and what it held are forgotten.
    func forget(_ window: WindowID)
    /// Where to open a window saved as `frame` with these displays attached: still reachable stays, else it is brought
    /// onto a display.
    func fit(_ frame: WindowFrame, on displays: [DisplayArea]) -> WindowFrame
    /// Where a window opened from `frame` goes on `display`: a step down and to the right, or the corner.
    func cascade(_ frame: WindowFrame, on display: DisplayArea) -> WindowFrame
    /// The smallest window content: the sidebar beside a usable terminal.
    var minSize: CGSize { get }
}

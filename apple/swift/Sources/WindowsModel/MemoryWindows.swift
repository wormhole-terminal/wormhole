// The windows kept in memory, this run only: the sample's and the tests'. It follows the contract of the core's
// (ids that never repeat, a forgotten window gone) but writes nothing and restores nothing.

import Foundation
import WorkspaceModel

@MainActor
public final class MemoryWindows: WindowsBackend {
    public private(set) var records: [WindowID: WindowFrame?] = [:]
    public private(set) var front: WindowID?
    public private(set) var forgotten: [WindowID] = []
    private var nextWindow = 1
    private var nextTerminal = 1_000

    public init() {}

    public func saved() -> [SavedWindow] { [] }

    public func allocWindowId() -> WindowID {
        defer { nextWindow += 1 }
        return nextWindow
    }

    public func plan(from: (any WorkspaceBackend)?, folder: String?) -> PlannedWindow {
        defer { nextTerminal += 1 }
        return PlannedWindow(window: allocWindowId(), terminal: nextTerminal)
    }

    @discardableResult
    public func save(window: WindowID, workspace: any WorkspaceBackend, frame: WindowFrame?) -> Bool {
        records[window] = frame
        return true
    }

    public func saveFront(_ window: WindowID) { front = window }

    public func forget(_ window: WindowID) {
        records[window] = nil
        forgotten.append(window)
    }

    public func fit(_ frame: WindowFrame, on displays: [DisplayArea]) -> WindowFrame { frame }

    /// The core's `WINDOW_MIN_*`, for the sample that runs without it.
    public var minSize: CGSize { CGSize(width: 560, height: 400) }

    /// The core's step, so the sample's second window does not open exactly over the first.
    public func cascade(_ frame: WindowFrame, on display: DisplayArea) -> WindowFrame {
        WindowFrame(x: frame.x + 26, y: frame.y + 26, width: frame.width, height: frame.height, display: display.id)
    }
}

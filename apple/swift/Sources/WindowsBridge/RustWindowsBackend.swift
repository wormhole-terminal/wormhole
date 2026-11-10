// The app's windows kept by the core (`wormhole-ffi` `Windows`, over core's store): the windows saved at the last run, the
// ids all windows share, a record per window written when something changed. A window's own workspace is made here too
// (`workspace(window:setup:)`) because it must come from the same registry: its terminal ids are the app's.

import Foundation
import WindowsModel
import WorkspaceBridge
import WorkspaceModel
import WormholeBindings

@MainActor
public final class RustWindowsBackend: WindowsBackend {
    fileprivate let windows: Windows

    /// The profile Wormhole is in. Throws `FfiError.NotIsolated` outside an experiment data folder, or when the
    /// database cannot be opened.
    public init() throws {
        windows = try Windows()
    }

    private init(_ windows: Windows) { self.windows = windows }

    /// Another profile's windows, open at once with these: its own database and saved windows, the same ids.
    public func beside(profile: String) throws -> RustWindowsBackend { RustWindowsBackend(try windows.beside(profile: profile)) }

    /// Window `window`'s workspace, among this backend's: filled by `restore()` from what was saved or planned for it.
    public func workspace(window: WindowID, setup: WorkspaceSetup) -> RustWorkspaceBackend {
        RustWorkspaceBackend(setup: setup, windows: windows, window: window)
    }

    public func saved() -> [WindowsModel.SavedWindow] {
        windows.saved().map { WindowsModel.SavedWindow(id: Int($0.id), frame: $0.frame.map(WindowFrame.init), terminals: Int($0.terminals)) }
    }

    public func allocWindowId() -> WindowID { Int(windows.allocWindowId()) }

    public func plan(from: (any WorkspaceBackend)?, folder: String?) -> WindowsModel.PlannedWindow {
        let planned = windows.plan(from: (from as? RustWorkspaceBackend)?.core, folder: folder)
        return WindowsModel.PlannedWindow(window: Int(planned.window), terminal: Int(planned.terminal))
    }

    @discardableResult
    public func save(window: WindowID, workspace: any WorkspaceBackend, frame: WindowFrame?) -> Bool {
        guard let rust = workspace as? RustWorkspaceBackend else { return false }
        return windows.save(window: Int64(window), workspace: rust.core, frame: frame?.record)
    }

    public func saveFront(_ window: WindowID) { windows.saveFront(window: Int64(window)) }

    public func forget(_ window: WindowID) { windows.forget(window: Int64(window)) }

    public func rehome(window: WindowID, workspace: any WorkspaceBackend, frame: WindowFrame?, to: any WindowsBackend) -> Bool {
        guard let rust = workspace as? RustWorkspaceBackend, let target = to as? RustWindowsBackend else { return false }
        return windows.rehome(window: Int64(window), workspace: rust.core, frame: frame?.record, to: target.windows)
    }

    public func tearOffLanding(over stack: [TearOffWindow], at point: CGPoint, from source: WindowID, covered: Bool) -> TearOffLanding {
        let windows = stack.map { TearWindow(window: $0.window.map(Int64.init), bounds: $0.bounds.tearRect) }
        switch WormholeBindings.tearOffLanding(stack: windows, x: Float(point.x), y: Float(point.y), source: Int64(source), covered: covered) {
        case .stay: return .stay
        case .newWindow: return .newWindow
        case .into(let window): return .into(Int(window))
        }
    }

    public func tearOffWindow(at point: CGPoint, size: CGSize, visible: CGRect) -> CGRect {
        let placed = WormholeBindings.tearOffWindow(
            x: Float(point.x), y: Float(point.y), width: Float(size.width), height: Float(size.height), visible: visible.tearRect)
        return CGRect(x: Double(placed.x), y: Double(placed.y), width: Double(placed.w), height: Double(placed.h))
    }

    public func fit(_ frame: WindowFrame, on displays: [DisplayArea]) -> WindowFrame {
        WindowFrame(fitFrame(frame: frame.record, displays: displays.map(\.record)))
    }

    public func cascade(_ frame: WindowFrame, on display: DisplayArea) -> WindowFrame {
        WindowFrame(cascadeFrame(from: frame.record, display: display.record))
    }

    public var minSize: CGSize {
        let least = windowMinSize()
        return CGSize(width: Double(least.width), height: Double(least.height))
    }
}

private extension CGRect {
    var tearRect: TearRect { TearRect(x: Float(minX), y: Float(minY), w: Float(width), h: Float(height)) }
}

// Where one window's state and its terminals come from. With the FFI linked and a program asked for (`--shell`,
// WORMHOLE_SHELL), the core's workspace keeps the list and every terminal is a real program that it started, among the
// app's windows (`WindowsHost`); otherwise the sample workspace and the mock sources stand in, over the same store and
// views.

import TerminalCore
import WindowsModel
import WorkspaceModel

#if WITH_UNIFFI
    import WindowsBridge
    import WorkspaceBridge
#endif

/// The app's windows: what is saved of them and where each window's workspace comes from.
@MainActor
struct WindowsHost {
    let backend: any WindowsBackend
    /// A window's workspace and the source of each of its terminals (nil when its program did not start); no function at
    /// all means the sample workspace and the mock sources.
    let workspace: ((WindowID) -> (backend: any WorkspaceBackend, source: (TerminalID) -> (any FrameSource)?))?

    /// Throws when the core's store cannot be opened (the data folder is not the experiment's, or is damaged).
    static func make(_ options: Options) throws -> WindowsHost {
        #if WITH_UNIFFI
            if let program = options.program {
                let windows = try RustWindowsBackend()
                CoreWindowRules.install()
                var first = true
                return WindowsHost(backend: windows) { id in
                    // Persistent terminals follow Settings' flag (off in a release build unless unlocked). The terminals
                    // are saved under their window's id, and one window brings in the hosts nobody names.
                    let hosting = HostingSetup(window: Int64(id), adoptsOrphans: first)
                    first = false
                    let backend = windows.workspace(window: id, setup: WorkspaceSetup(program: program, hosting: hosting))
                    return (backend, { backend.frameSource(for: $0) })
                }
            }
        #endif
        return WindowsHost(backend: MemoryWindows(), workspace: nil)
    }
}

@MainActor
struct WorkspaceHost {
    let store: WorkspaceStore
    /// The source of a terminal that the core's workspace runs (nil when its program did not start); no function at all
    /// means the mock sources.
    let source: ((TerminalID) -> (any FrameSource)?)?

    /// Window `window`, filled from what was saved or planned for it. A real window with nothing saved starts with one
    /// terminal, as the GPUI app does, unless `empty` says terminals are about to move into it; `closed` is what a
    /// fixed grid (a measuring mode) asks: both columns shut, the terminal is all the window shows.
    static func make(_ options: Options, windows: WindowsHost, window: WindowID, empty: Bool = false) -> WorkspaceHost {
        let host: WorkspaceHost
        if let made = windows.workspace?(window) {
            host = WorkspaceHost(store: WorkspaceStore(backend: made.backend), source: made.source)
            if !empty {
                // What was saved or planned for it: its terminals back (in their session hosts when those run).
                host.store.restore()
                if host.store.state.live.isEmpty { host.store.send(.newTerminal) }
            }
        } else {
            host = WorkspaceHost(store: WorkspaceStore(backend: MockWorkspaceBackend()), source: nil)
        }
        if options.grid != nil {
            host.store.send(.toggleSidebar)
            host.store.send(.togglePanel)
        }
        return host
    }
}

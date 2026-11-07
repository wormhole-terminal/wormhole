// Where the window's state and its terminals come from. With the FFI linked and a program asked for (`--shell`,
// WORMHOLE_SHELL), the core's workspace keeps the list and every terminal is a real program that it started; otherwise the
// sample workspace and the mock sources stand in, over the same store and views.

import TerminalCore
import WorkspaceModel

#if WITH_UNIFFI
    import WorkspaceBridge
#endif

@MainActor
struct WorkspaceHost {
    let store: WorkspaceStore
    /// The source of a terminal that the core's workspace runs (nil when its program did not start); no function at all
    /// means the mock sources.
    let source: ((TerminalID) -> (any FrameSource)?)?

    /// A real window starts with one terminal, as the GPUI app does; `closed` is what a fixed grid (a measuring mode) asks:
    /// both columns shut, the terminal is all the window shows.
    static func make(_ options: Options) -> WorkspaceHost {
        let host: WorkspaceHost
        #if WITH_UNIFFI
            if let program = options.program {
                // Persistent terminals follow Settings' flag (off in a release build unless unlocked); the terminals the
                // last run left in session hosts come back, else the window starts with one.
                let backend = RustWorkspaceBackend(setup: WorkspaceSetup(program: program, hosting: HostingSetup()))
                host = WorkspaceHost(store: WorkspaceStore(backend: backend), source: { backend.frameSource(for: $0) })
                if backend.restoreSaved() == 0 { host.store.send(.newTerminal) }
            } else {
                host = sample
            }
        #else
            host = sample
        #endif
        if options.grid != nil {
            host.store.send(.toggleSidebar)
            host.store.send(.togglePanel)
        }
        return host
    }

    private static var sample: WorkspaceHost {
        WorkspaceHost(store: WorkspaceStore(backend: MockWorkspaceBackend()), source: nil)
    }
}

// The agents' hooks and the control line for the Swift app, over the Rust workspace. Two calls: the executable
// answers a command line before it is an app, and the app starts the server beside its workspace.
//
// Every terminal gets `WORMHOLE_TAB_ID`, `WORMHOLE_SOCKET` and `WORMHOLE_BIN` from `Terminal.startForTab`; the
// installed hook scripts run `$WORMHOLE_BIN hook …`, so the app's own executable must do that command line.

import Foundation
import WormholeBindings

public enum HookBridge {
    /// What a command line asked of this executable (`hook`, `statusline`, `list`, `send-text`, …): done, with its
    /// exit code, for `main` to exit with. `nil` for a launch. Call it first, before anything else is set up.
    public static func exitCodeIfCommandLine(_ arguments: [String] = CommandLine.arguments) -> Int32? {
        cliExitCode(arguments: arguments)
    }

    /// Become the running instance of the data folder and serve its socket. Throws `HookError.AlreadyServed` when
    /// another process does (it was asked to come forward: this one should quit), `.NotIsolated` outside an
    /// experiment folder. `dataDir` `nil` is the process's own (`WORMHOLE_DATA_DIR`).
    @MainActor
    public static func start(
        workspace: Workspace, usage: UsageService, access: AccessDesk, dataDir: URL? = nil
    ) throws -> HookServerHandle {
        try HookServerHandle(workspace: workspace, usage: usage, access: access, dataDir: dataDir)
    }
}

// What the window needs to run its terminals in session hosts (persistent terminals). Nil in `WorkspaceSetup` keeps every
// terminal in the app's process, as before.

import Foundation

public struct HostingSetup: Sendable {
    /// The folder the saved terminals and the hosts live in; nil is the process's own (`WORMHOLE_DATA_DIR`).
    public var dataDirectory: String?
    /// A host program to start instead of the one beside the app (tests, development). The release library ignores core's
    /// own `WORMHOLE_HOST_BIN`, so the variable is read here and handed over.
    public var hostBinary: String?
    /// The id the terminals' saved rows are kept under.
    public var window: Int64

    public init(
        dataDirectory: String? = nil, hostBinary: String? = ProcessInfo.processInfo.environment["WORMHOLE_HOST_BIN"],
        window: Int64 = 1
    ) {
        (self.dataDirectory, self.hostBinary, self.window) = (dataDirectory, hostBinary, window)
    }
}

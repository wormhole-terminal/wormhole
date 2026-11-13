// The seam for the app's own log lines, beside the words' seam because every module already depends on this one. The
// app installs a sink at launch (`LaunchLogging`: the core's daily files); a module without one (a test, a build without
// the core) loses the line, as `Words` falls back to English.

import Foundation
import Synchronization

public enum AppLog {
    private static let sink = Mutex<(@Sendable (_ level: String, _ message: String) -> Void)?>(nil)

    public static func install(_ sink: (@Sendable (_ level: String, _ message: String) -> Void)?) {
        self.sink.withLock { $0 = sink }
    }

    /// One line into the log; `level` is `error`, `warn`, `info` or `debug`.
    public static func log(_ level: String, _ message: String) {
        let send = sink.withLock { $0 }
        send?(level, message)
    }
}

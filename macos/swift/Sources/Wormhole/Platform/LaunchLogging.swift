// The Swift app's logs: the core's subscriber and panic hook are installed once at launch (`logging.rs` of the bridge, the
// GPUI app's `app/logging.rs`), and the app's own lines go into the same daily files through `log`.

import Foundation
import AppLogging

#if WITH_UNIFFI
    import WormholeBindings
#endif

enum LaunchLogging {
    /// Before anything logs: files under `<data folder>/logs` (a test instance's own folder), `WORMHOLE_LOG` as the level,
    /// a panic in `last-panic.txt`; and the data folder's files of earlier versions made owner-only.
    static func start() {
        #if WITH_UNIFFI
            _ = loggingStart()
            makeDataDirPrivate()
            // The modules below the app log through `AppLog`; their lines land in the same files.
            AppLog.install { level, message in logLine(level: level, message: message) }
        #endif
    }

    /// One line of the app's own into the log; `level` is `error`, `warn`, `info` or `debug`.
    static func log(_ level: String, _ message: String) {
        #if WITH_UNIFFI
            logLine(level: level, message: message)
        #endif
    }
}

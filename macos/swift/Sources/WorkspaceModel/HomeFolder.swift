// Where the person's home is, by the core's rule (`wormhole_term::sys::paths::home`): `$HOME` when it is set and not
// empty, else the one the system lists for the user. A test instance sets `$HOME` to a scratch folder, and the core
// writes a terminal's folder with `~` for that one: every `~` the app writes out again goes through here, never through
// `NSHomeDirectory()` or `expandingTildeInPath`, which name the real home whatever `$HOME` says.

import Foundation

public enum HomeFolder {
    /// The home folder's path.
    public static var path: String {
        if let home = ProcessInfo.processInfo.environment["HOME"], !home.isEmpty { return home }
        return NSHomeDirectory()
    }

    /// `path` with a leading `~` written out; any other path as it is.
    public static func expanding(_ path: String) -> String {
        path == "~" ? self.path : path.hasPrefix("~/") ? self.path + path.dropFirst(1) : path
    }
}

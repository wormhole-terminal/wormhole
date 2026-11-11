// The core's rules for a file of a repository, which the panel's "Open in" and Reveal follow (`repo_file_resolve`,
// `open_file_run`, `open_file_tooltip`): a path that leaves the repository (a planted link), is gone or is not a file is
// never opened or revealed. The app installs core's at launch (`FilesBridge.installRules()`); until then, in previews and
// tests, a stand-in answers: the path under its root, when something is there.

import Foundation

public enum RepoFiles {
    // why: written once on the main thread at launch, before any window reads them; every later access is a read.
    nonisolated(unsafe) public static var resolver: @Sendable (_ root: String, _ relative: String) -> String? = { root, relative in
        let full = root + "/" + relative
        return FileManager.default.fileExists(atPath: full) ? full : nil
    }
    /// Asks the app with this bundle identifier (found in `appFolder`) to open the file at the line.
    nonisolated(unsafe) public static var opener: @Sendable (_ bundle: String, _ path: String, _ line: Int?, _ appFolder: String?) -> Bool = { _, _, _, _ in false }
    nonisolated(unsafe) public static var tooltip: @Sendable (_ bundle: String, _ app: String, _ line: Int?) -> String = { _, app, _ in "Open in \(app)" }

    /// Where `relative` is inside `root` now, or nil.
    public static func resolve(root: String, relative: String) -> String? { resolver(root, relative) }
}

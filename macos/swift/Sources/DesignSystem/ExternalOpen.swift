// The one door to what the app hands to the system to open: a folder in the Finder, a file in its editor, a link in the
// browser. A test instance (`--background`) is diverted to a file in its data folder (one line a call, `open <target>`), so a
// check never starts a person's browser, editor or Finder window, and the control folder can read what would have opened.

import AppKit
import Synchronization

public enum ExternalOpen {
    private static let log = Mutex<URL?>(nil)

    /// Record the calls in `file` from now on (a test instance's), or `nil` to open for real.
    public static func divert(to file: URL?) { log.withLock { $0 = file } }

    /// Opens `url` with the system (a file or folder in the app that opens it, a link in the browser).
    public static func open(_ url: URL) {
        if !record("open \(url.isFileURL ? url.path : url.absoluteString)") { NSWorkspace.shared.open(url) } // text: the test instance's record of a launch
    }

    /// Opens `target` in the app with bundle identifier `bundle` found at `app` (Open in…); a test instance records it.
    @MainActor
    public static func open(_ target: URL, in bundle: String, at app: URL) {
        if !record("open \(target.path) in \(bundle)") {  // text: a test instance's record
            NSWorkspace.shared.open([target], withApplicationAt: app, configuration: NSWorkspace.OpenConfiguration())
        }
    }

    /// Shows `urls` in a Finder window with them selected.
    public static func reveal(_ urls: [URL]) {
        if !record("reveal " + urls.map(\.path).joined(separator: " ")) { NSWorkspace.shared.activateFileViewerSelecting(urls) } // text: the test instance's record of a launch
    }

    /// Writes the line when diverted; false when calls open for real.
    private static func record(_ line: String) -> Bool {
        guard let file = log.withLock({ $0 }) else { return false }
        try? FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        let existing = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
        try? (existing + line + "\n").write(to: file, atomically: true, encoding: .utf8)
        return true
    }
}

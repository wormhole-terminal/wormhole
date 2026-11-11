// The one door to launching things outside the app: an address in the browser, a file in Finder. The app's windows go
// through it, never through `NSWorkspace`: a test instance (`--background`) is diverted to a file in its data folder, so it
// never opens the person's browser or Finder, and the control folder reads what would have launched (`launches`).
// The core's own launches (a file in an editor at a line) are written to the same file by `open_file_record_to`.

import AppKit
import Synchronization

public enum Launcher {
    private static let file = Mutex<URL?>(nil)

    /// Write launches to `file` from now on (a test instance's), or `nil` to launch for real.
    public static func divert(to file: URL?) { Self.file.withLock { $0 = file } }

    /// Where launches are written; nil when they are real.
    public static var diverted: URL? { file.withLock { $0 } }

    /// What would have been launched, a line each; nil when launches are real.
    public static func log() -> String? { diverted.map { (try? String(contentsOf: $0, encoding: .utf8)) ?? "" } }

    public static func open(_ url: URL) {
        guard record("open \(url.absoluteString)") else { return _ = NSWorkspace.shared.open(url) }
    }

    public static func reveal(_ urls: [URL]) {
        guard record("reveal \(urls.map(\.path).joined(separator: " "))") else { return NSWorkspace.shared.activateFileViewerSelecting(urls) }
    }

    /// Appends `line` and answers true when launches are diverted.
    private static func record(_ line: String) -> Bool {
        guard let file = diverted else { return false }
        try? FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        let existing = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
        try? (existing + line + "\n").write(to: file, atomically: true, encoding: .utf8)
        return true
    }
}

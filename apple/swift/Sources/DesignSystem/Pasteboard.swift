// The one door to the clipboard. The app's windows copy and paste through it, never through `NSPasteboard.general`: a
// test instance (`--background`) is diverted to a file in its data folder, so it never reads or replaces the person's
// clipboard, and the control folder can read what was "copied" (`pasteboard`) or set what a paste finds (`pasteboard-set`).

import AppKit
import Synchronization

public enum Pasteboard {
    private struct State {
        var file: URL?
        var changes = 0
    }
    private static let state = Mutex(State())

    /// Keep the clipboard in `file` from now on (a test instance's), or `nil` for the system's.
    public static func divert(to file: URL?) { state.withLock { $0.file = file } }

    public static var isDiverted: Bool { state.withLock { $0.file != nil } }

    /// The text on it, nil when there is none.
    public static func text() -> String? {
        guard let file = state.withLock({ $0.file }) else { return NSPasteboard.general.string(forType: .string) }
        return try? String(contentsOf: file, encoding: .utf8)
    }

    public static func setText(_ text: String) {
        let file = state.withLock { state -> URL? in
            state.changes += 1
            return state.file
        }
        guard let file else {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
            return
        }
        try? FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
        try? text.write(to: file, atomically: true, encoding: .utf8)
    }

    /// Something to paste: text (files on the system's clipboard count too, as `NSPasteboard` has them).
    public static var canPaste: Bool {
        if isDiverted { return text()?.isEmpty == false }
        return NSPasteboard.general.canReadObject(forClasses: [NSString.self, NSURL.self], options: nil)
    }

    /// Changes with every copy, to tell whether an action copied anything.
    public static var changeCount: Int {
        state.withLock { $0.file == nil ? NSPasteboard.general.changeCount : $0.changes }
    }
}

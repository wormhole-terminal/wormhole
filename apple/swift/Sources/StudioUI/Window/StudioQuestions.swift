// The questions the Studio asks, as the system's own sheets: save before closing or opening another theme, the
// file another application changed, moving a theme to the Trash. A sheet on the window, answered by a button;
// Esc is Cancel, the first button is the default (`modal.rs`, `switch.rs`, `theme_trash`).

import AppKit

@MainActor
enum StudioQuestions {
    enum Answer { case save, discard, cancel }

    /// "Do you want to save the changes to “name”?": Save, Don't Save, Cancel.
    static func unsaved(name: String, changes: Int, on window: NSWindow?) async -> Answer {
        let alert = NSAlert()
        alert.messageText = StudioWords.closeMessage(name)
        alert.informativeText = StudioWords.closeDetail(changes)
        alert.addButton(withTitle: StudioWords.saveShort)
        alert.addButton(withTitle: StudioWords.dontSave)
        alert.addButton(withTitle: StudioWords.cancel)
        switch await run(alert, on: window) {
        case .alertFirstButtonReturn: return .save
        case .alertSecondButtonReturn: return .discard
        default: return .cancel
        }
    }

    /// The file was changed by another application: replacing it keeps what was there as a saved version.
    static func changedOnDisk(name: String, on window: NSWindow?) async -> Bool {
        let alert = NSAlert()
        alert.messageText = StudioWords.changedMessage(name)
        alert.informativeText = StudioWords.changedDetail
        alert.addButton(withTitle: StudioWords.overwrite)
        alert.addButton(withTitle: StudioWords.cancel)
        return await run(alert, on: window) == .alertFirstButtonReturn
    }

    static func trash(name: String, on window: NSWindow?) async -> Bool {
        let alert = NSAlert()
        alert.messageText = StudioWords.trashMessage(name)
        alert.informativeText = StudioWords.trashDetail
        alert.alertStyle = .warning
        alert.addButton(withTitle: StudioWords.trash.replacingOccurrences(of: "…", with: ""))
        alert.addButton(withTitle: StudioWords.cancel)
        return await run(alert, on: window) == .alertFirstButtonReturn
    }

    private static func run(_ alert: NSAlert, on window: NSWindow?) async -> NSApplication.ModalResponse {
        guard let window else { return alert.runModal() }
        return await alert.beginSheetModal(for: window)
    }
}

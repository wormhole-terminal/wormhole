// The questions the Studio asks, as the system's own sheets: save before closing or opening another theme, the
// file another application changed, moving a theme to the Trash. A sheet on the window, answered by a button;
// Esc is Cancel, the first button is the default (`modal.rs`, `switch.rs`, `theme_trash`).

import AppKit

@MainActor
public enum StudioQuestions {
    enum Answer { case save, discard, cancel }

    /// What asks instead of the system's sheet: a test instance holds the question for its control folder (`question`), as it
    /// holds every other. Given the question's words and its buttons in order, it answers the place of the one pressed.
    public static var asker: (@MainActor (_ title: String, _ detail: String, _ buttons: [String]) async -> Int)?

    /// "Do you want to save the changes to “name”?": Save, Don't Save, Cancel.
    static func unsaved(name: String, changes: Int, opening next: String? = nil, on window: NSWindow?) async -> Answer {
        let alert = NSAlert()
        alert.messageText = next.map { StudioWords.switchMessage(name, $0) } ?? StudioWords.closeMessage(name)
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

    /// A theme of that name is in the themes folder: Replace overwrites it with this draft (the old file is kept as a saved version).
    static func exists(name: String, on window: NSWindow?) async -> Bool {
        let alert = NSAlert()
        alert.messageText = StudioWords.existsMessage(name)
        alert.informativeText = StudioWords.existsDetail
        alert.alertStyle = .warning
        alert.addButton(withTitle: StudioWords.replace)
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
        if let asker {
            let place = await asker(alert.messageText, alert.informativeText, alert.buttons.map(\.title))
            return NSApplication.ModalResponse(rawValue: NSApplication.ModalResponse.alertFirstButtonReturn.rawValue + place)
        }
        guard let window else { return alert.runModal() }
        return await alert.beginSheetModal(for: window)
    }
}

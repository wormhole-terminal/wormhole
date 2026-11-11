// What the right panel asks of the system, answered by the system: the clipboard, Finder, the default editor, the folder
// panel. What needs the terminal (typing at its prompt, pasting into it, taking the prompt away) is handed in by the
// window as closures, since the terminal's view is the window's, not the panel's.

import AppKit
import DesignSystem
import PanelModel
import TerminalUI
import WordsModel

@MainActor
final class SystemPanelHost: PanelHost {
    var terminal = Terminal()

    /// The terminal's side of the host; each defaults to nothing happening.
    struct Terminal {
        var paste: (String) -> Bool = { _ in false }
        var send: (String) -> Void = { _ in }
        var takePrompt: () -> String? = { nil }
        var focus: () -> Void = {}
        var open: (String, String?) -> Void = { _, _ in }
        /// The chip of the window says `notice`.
        var say: (Notice) -> Void = { _ in }
    }

    let editorName: String? = nil
    let homeFolder: String? = NSHomeDirectory()

    func copy(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    func copyPath(_ path: String) {
        guard !path.unicodeScalars.contains(where: { $0.properties.generalCategory == .control }) else {
            notify(Notice.failed(.copyPath).words)
            return terminal.say(.failed(.copyPath))
        }
        copy(path)
        notify(Words.text("files-spoken-path-copied", fallback: "Path copied"))
    }

    func reveal(_ path: String) { NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)]) }

    func open(_ path: String, line: Int?) { NSWorkspace.shared.open(URL(fileURLWithPath: path)) }

    func paste(_ text: String) -> Bool { terminal.paste(text) }
    func send(_ text: String) { terminal.send(text) }
    func takePrompt() -> String? { terminal.takePrompt() }
    func focusTerminal() { terminal.focus() }

    func chooseFolder() async -> String? {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Choose"
        return await withCheckedContinuation { continuation in
            panel.begin { continuation.resume(returning: $0 == .OK ? panel.url?.path : nil) }
        }
    }

    func notify(_ text: String) {
        Announcements.post(text)
    }

    func openTerminal(in folder: String, running command: String?) { terminal.open(folder, command) }
}

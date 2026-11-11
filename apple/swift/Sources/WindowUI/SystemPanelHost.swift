// What the right panel asks of the system, answered by the system: the clipboard, Finder, the default editor, the folder
// panel. What needs the terminal (typing at its prompt, pasting into it, taking the prompt away) is handed in by the
// window as closures, since the terminal's view is the window's, not the panel's.

import AppKit
import DesignSystem
import PanelModel
import PanesModel
import TerminalUI
import WordsModel

@MainActor
final class SystemPanelHost: PanelHost {
    var terminal = Terminal()

    /// The terminal's side of the host; each defaults to nothing happening.
    struct Terminal {
        var paste: (String) -> Bool = { _ in false }
        var send: (String) -> Void = { _ in }
        var submit: () -> Void = {}
        var takePrompt: () -> String? = { nil }
        var focus: () -> Void = {}
        var open: (String, String?) -> Void = { _, _ in }
        /// The chip of the window says `notice`.
        var say: (Notice) -> Void = { _ in }
    }

    /// The app the Open in button opens files in (the window's choice); Finder is a reveal, so it names no editor.
    var chosenApp: () -> OpenInApp? = { nil }
    var editorName: String? { chosenApp().flatMap { $0 == .finder ? nil : $0.name } }
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

    /// The core runs the app's own way of taking a line; a build without it opens the file in the default app.
    func open(_ path: String, line: Int?) {
        guard let app = chosenApp() else { return }
        let folder = NSWorkspace.shared.urlForApplication(withBundleIdentifier: app.bundle)?.path
        if !RepoFiles.opener(app.bundle, path, line, folder) { NSWorkspace.shared.open(URL(fileURLWithPath: path)) }
    }

    func openTooltip(line: Int?) -> String? { chosenApp().map { RepoFiles.tooltip($0.bundle, $0.name, line) } }

    func exists(_ path: String) -> Bool { FileManager.default.fileExists(atPath: path) }

    func paste(_ text: String) -> Bool { terminal.paste(text) }
    func send(_ text: String) { terminal.send(text) }
    func submit() { terminal.submit() }
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

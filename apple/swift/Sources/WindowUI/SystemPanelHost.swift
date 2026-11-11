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
    /// False in a test instance: opening and revealing are only logged.
    var launches: () -> Bool = { true }

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
        /// The window of everything the project would run.
        var showCommands: () -> Void = {}
        /// The apps Open in lists.
        var apps: () -> [OpenInApp] = { [] }
    }

    /// The app the Open in button opens files in (the window's choice); Finder is a reveal, so it names no editor.
    var chosenApp: () -> OpenInApp? = { nil }
    var editorName: String? { chosenApp().flatMap { $0 == .finder ? nil : $0.name } }
    let homeFolder: String? = NSHomeDirectory()

    func copy(_ text: String) {
        Pasteboard.setText(text)
    }

    func failed(_ what: PanelFailed) { terminal.say(.failed(what.notice)) }

    func copy(_ text: String, as what: PanelCopied?) {
        copy(text)
        guard let what else { return }
        terminal.say(.copied(what.notice))
    }

    func copyPath(_ path: String) {
        guard !path.unicodeScalars.contains(where: { $0.properties.generalCategory == .control }) else {
            notify(Notice.failed(.copyPath).words)
            return terminal.say(.failed(.copyPath))
        }
        copy(path)
        notify(Words.text("files-spoken-path-copied", fallback: "Path copied"))
    }

    func reveal(_ path: String) {
        guard launches() else { return notify("Would reveal \(path)") }
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: path)])
    }

    /// The core runs the app's own way of taking a line; a build without it opens the file in the default app.
    func open(_ path: String, line: Int?) {
        guard let app = chosenApp() else { return }
        guard launches() else { return notify("Would open \(path) in \(app.name)") }
        let folder = NSWorkspace.shared.urlForApplication(withBundleIdentifier: app.bundle)?.path
        if !RepoFiles.opener(app.bundle, path, line, folder) { NSWorkspace.shared.open(URL(fileURLWithPath: path)) }
    }

    var openApps: [OpenApp] {
        let chosen = chosenApp()
        return terminal.apps().map { OpenApp(bundle: $0.bundle, name: $0.name, isChosen: $0 == chosen) }
    }

    func open(_ path: String, inBundle bundle: String) {
        guard launches() else { return notify("Would open \(path) in \(bundle)") }
        let folder = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundle)?.path
        if !RepoFiles.opener(bundle, path, nil, folder) { NSWorkspace.shared.open(URL(fileURLWithPath: path)) }
    }

    func showProjectCommands() { terminal.showCommands() }

    func openTooltip(line: Int?) -> String? { chosenApp().map { RepoFiles.tooltip($0.bundle, $0.name, line) } }

    func stamp(of path: String) -> String? {
        guard let attributes = try? FileManager.default.attributesOfItem(atPath: path) else { return nil }
        return "\(attributes[.size] as? Int ?? 0) \((attributes[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0)"
    }

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

extension PanelCopied {
    /// What the chip says for a copy from the panel.
    var notice: Notice.Copied {
        switch self {
        case .error: .error
        case .objectId: .objectId
        case .commitHash: .commitHash
        case .commitShortHash: .commitShortHash
        case .commitMessage: .commitMessage
        case .commitMessageCut: .commitMessageCut
        case .commitSubject: .commitSubject
        case .commitAuthor: .commitAuthor
        }
    }
}

extension PanelFailed {
    var notice: Notice.Failed {
        switch self {
        case .copyCommit: .copyCommit
        case .renameName: .renameName
        case .renameExists: .renameExists
        case .renameDenied: .renameDenied
        case .renameGone: .renameGone
        case .renameFailed: .renameFailed
        }
    }
}

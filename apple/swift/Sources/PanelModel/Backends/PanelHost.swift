// What the panel asks of the window and the system: the clipboard, Finder, an editor, the terminal's prompt. Real
// closures instead of stubs, in one protocol the window implements and tests replace.

import WordsModel

@MainActor
public protocol PanelHost: AnyObject {
    func copy(_ text: String)
    /// A copy that says what it was on the notice chip ("Copied commit hash"); where there is no chip it is a plain copy.
    func copy(_ text: String, as what: PanelCopied?)
    /// Something the person asked for could not be done: the chip says why.
    func failed(_ what: PanelFailed)
    /// The path of a file or folder on the clipboard, plain and absolute. A path with a control character is not copied
    /// (it would act when pasted) and the notice chip says so (`files/open.rs` `copy_path`).
    func copyPath(_ path: String)
    func reveal(_ path: String)
    /// Opens a file in the editor the person chose (`Open in`), at a line when given.
    func open(_ path: String, line: Int?)
    /// The apps "Open in" lists (installed editors, Finder last), the chosen one marked; empty where the system lists none.
    var openApps: [OpenApp] { get }
    /// Opens a file or folder in that app (the menu's Open in submenu).
    func open(_ path: String, inBundle bundle: String)
    /// "View all N…": the window that lists everything the project would run, with the approval there.
    func showProjectCommands()
    /// The name on the "Open in" button's tooltip ("Zed"), nil when none is known or the chosen app is Finder (whose "open"
    /// is the reveal, so no second item).
    var editorName: String? { get }
    /// The words of the open action for the chosen app, at a line where it can: "Open in Zed at line 12" (core's).
    func openTooltip(line: Int?) -> String?
    /// The path's size and modification time as one word that differs when the file was written; nil when it is not there.
    func stamp(of path: String) -> String?
    /// Whether something is at this absolute path now (a file the working tree no longer has cannot be opened).
    func exists(_ path: String) -> Bool
    /// The person's home folder, which shortens paths to `~`.
    var homeFolder: String? { get }
    /// Pastes `text` into the terminal's prompt without pressing return. False where the program in the terminal
    /// would run each line of a paste as a command: the text is not pasted then.
    func paste(_ text: String) -> Bool
    /// Puts `text` at the terminal's prompt as a paste (bracketed where the program asked for it: a next step, a note, a project
    /// prompt), for the person to read and send.
    func send(_ text: String)
    /// Presses return in the terminal: what ⌘⇧↵ does after it sent a note's text.
    func submit()
    /// The text at the terminal's prompt, taken away (stash prompt into a note); nil when it is empty.
    func takePrompt() -> String?
    /// Gives the keys back to the terminal (esc from a viewer, after a paste).
    func focusTerminal()
    /// Choose Folder…: the system's panel, folders only.
    func chooseFolder() async -> String?
    /// A short line for the notice chip and VoiceOver ("Path copied", "Renamed to a.rs").
    func notify(_ text: String)
    /// Shows a tab of the panel (File History, a prompt copied into a note): the window's tab, and the panel with it.
    func show(tab: PanelTab)
    /// Whether the window offers that tab now (its feature is on).
    func offers(tab: PanelTab) -> Bool
    /// "New terminal in worktree": the terminal the worktree was made for.
    func openTerminal(in folder: String, running command: String?)
}

/// One app of the "Open in" submenu.
public struct OpenApp: Identifiable, Equatable, Sendable {
    public var bundle: String
    public var name: String
    public var isChosen: Bool
    /// An editor Wormhole knows, or Finder: a file is offered only these, a folder every app (`may_open_in`).
    public var isKnown: Bool
    public var id: String { bundle }

    public init(bundle: String, name: String, isChosen: Bool, isKnown: Bool = true) {
        (self.bundle, self.name, self.isChosen, self.isKnown) = (bundle, name, isChosen, isKnown)
    }
}

/// What the panel could not do, for the chip (`view::notice::Failed`).
public enum PanelFailed: Sendable {
    case copyCommit
    case renameName, renameExists, renameDenied, renameGone, renameFailed
}

/// What a copy from the panel was, for the chip's words (`view::notice::Copied`).
public enum PanelCopied: Sendable {
    case error, objectId
    case commitHash, commitShortHash, commitMessage, commitMessageCut, commitSubject, commitAuthor
}

extension PanelHost {
    public func copy(_ text: String, as what: PanelCopied?) { copy(text) }
    /// Where there is no chip, nothing says it (VoiceOver is told through `notify` where the caller has words).
    public func failed(_ what: PanelFailed) {}
    public func stamp(of path: String) -> String? { nil }
    public var openApps: [OpenApp] { [] }
    public func open(_ path: String, inBundle bundle: String) {}
    public func showProjectCommands() {}
    public func openTooltip(line: Int?) -> String? { editorName.map { Words.text("files-open-in-editor", ["editor": .text($0)], fallback: "Open in \($0)") } }
    public func exists(_ path: String) -> Bool { true }
    public func show(tab: PanelTab) {}
    public func offers(tab: PanelTab) -> Bool { true }
    /// Where there is no chip to refuse on: the copy, and VoiceOver's word for it.
    public func copyPath(_ path: String) {
        copy(path)
        notify(Words.text("files-spoken-path-copied", fallback: "Path copied"))
    }
}

// What the panel asks of the window and the system: the clipboard, Finder, an editor, the terminal's prompt. Real
// closures instead of stubs, in one protocol the window implements and tests replace.

@MainActor
public protocol PanelHost: AnyObject {
    func copy(_ text: String)
    func reveal(_ path: String)
    /// Opens a file in the editor the person chose (`Open in`), at a line when given.
    func open(_ path: String, line: Int?)
    /// The name on the "Open in" button's tooltip ("Zed"), nil when none is known.
    var editorName: String? { get }
    /// The person's home folder, which shortens paths to `~`.
    var homeFolder: String? { get }
    /// Pastes `text` into the terminal's prompt without pressing return. False where the program in the terminal
    /// would run each line of a paste as a command: the text is not pasted then.
    func paste(_ text: String) -> Bool
    /// Types `text` at the terminal's prompt (a next step, a note, a project prompt), for the person to read and send.
    func send(_ text: String)
    /// The text at the terminal's prompt, taken away (stash prompt into a note); nil when it is empty.
    func takePrompt() -> String?
    /// Gives the keys back to the terminal (esc from a viewer, after a paste).
    func focusTerminal()
    /// Choose Folder…: the system's panel, folders only.
    func chooseFolder() async -> String?
    /// A short line for the notice chip and VoiceOver ("Path copied", "Renamed to a.rs").
    func notify(_ text: String)
    /// "New terminal in worktree": the terminal the worktree was made for.
    func openTerminal(in folder: String, running command: String?)
}

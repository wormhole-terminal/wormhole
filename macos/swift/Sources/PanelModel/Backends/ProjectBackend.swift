// Where the Project tab's data comes from: the `.wormhole/` folder read under caps, the trust store and Touch ID,
// the per-user switches (`wormhole_core::project`). Everything that runs a command or changes the environment goes
// through here, where trust is checked; Swift only asks and shows the answer.

import WorkspaceModel

public protocol ProjectBackend: Sendable {
    /// The project at `folder` as the tab draws it; nil where there is no `.wormhole/`.
    func project(at folder: String) async -> ProjectState?
    /// What `folder`'s colour is, where it is kept and whether it may be changed.
    func folderPaint(of folder: String) async -> FolderPaint
    func setFolderColor(_ color: FolderColor?, of folder: String) async
    func setFolderColor(hex: String, of folder: String) async
    func createProject(at folder: String) async -> String?
    func setFeature(_ feature: ProjectFeature, on: Bool, at root: String) async
    /// Approves the project's files: Touch ID (or the password) first. False when it was declined.
    func approve(at root: String) async -> Bool
    func revoke(at root: String) async
    func run(action id: String, at root: String) async
    /// The id of the action whose key is `glyphs` (`⇧⌘R`), asked of the core at the moment of the key: nil unless the project is
    /// trusted and has Actions on right now (the key is then nobody's and goes on to the terminal).
    func actionID(forKey glyphs: String, at root: String) -> String?
    func send(prompt id: String, at root: String) async
    /// A prompt's text, for copying it into a note.
    func text(ofPrompt id: String, at root: String) async -> String?
    func openLayout(at root: String) async
    /// Fires when the `.wormhole/` folder or the trust changed.
    func changes(at folder: String) -> AsyncStream<Void>
    /// Whether Touch ID is set up, which words the approve button ("Trust with Touch ID", else "Trust (approve with password)").
    var biometricsAvailable: Bool { get }
    /// Why a worktree may not be made of `repo` (the core's rule: a project there must be trusted with its Settings on), or nil.
    func worktreeRefusal(of repo: String) async -> String?
    /// What a new worktree of `repo` runs first: its trusted project's `worktree.setup`.
    func worktreeSetup(of repo: String) async -> String?
    /// What went wrong with the person's own stores (approvals or switches reset, a failed save), which the tab says
    /// where there is no project too; nil when nothing did.
    func problem() async -> String?
}

extension ProjectBackend {
    public var biometricsAvailable: Bool { true }
    public func actionID(forKey glyphs: String, at root: String) -> String? { nil }
    public func worktreeRefusal(of repo: String) async -> String? { nil }
    public func worktreeSetup(of repo: String) async -> String? { nil }
    public func problem() async -> String? { nil }
}

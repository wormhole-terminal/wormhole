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
    func send(prompt id: String, at root: String) async
    /// A prompt's text, for copying it into a note.
    func text(ofPrompt id: String, at root: String) async -> String?
    func openLayout(at root: String) async
    /// Fires when the `.wormhole/` folder or the trust changed.
    func changes(at folder: String) -> AsyncStream<Void>
}

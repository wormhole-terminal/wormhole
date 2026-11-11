// Where a coding agent's hook events and a terminal's notes enter the window's state. The core's status machine decides
// what a hook means (`UserPromptSubmit` is working, `PermissionRequest` needs you, `Stop` finished); whoever receives the
// hooks calls this. The app that serves the hook socket (the GPUI app's control socket, `Wormhole hook`) is that caller;
// the Swift app does not serve it yet, so until it does an agent's status comes from its output and its process only.

import Foundation

@MainActor
public protocol AgentHookSink: AnyObject {
    /// An agent's hook for terminal `id` (what `WORMHOLE_TAB_ID` names): `agent` is its command ("claude"), `event` as the
    /// hook script spells it (`Stop`, `Notification:permission_prompt`), `message` the words it carries.
    func hook(id: TerminalID, agent: String, event: String, message: String?)
    /// A terminal's tasks (one flag each, done or not) and the text of each note, as the row counts what is still open.
    func setNotes(id: TerminalID, tasksDone: [Bool], notes: [String])
}

extension WorkspaceStore {
    /// The sink behind the store, when its backend is the real workspace; nil for the sample.
    public var hooks: (any AgentHookSink)? { backend as? any AgentHookSink }

    /// The window in front or behind, and whether it is seen at all.
    public func windowState(active: Bool, seen: Bool) { backend.setWindowState(active: active, seen: seen) }
}

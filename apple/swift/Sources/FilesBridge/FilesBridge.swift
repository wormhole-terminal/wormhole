// The right panel's Files, Project, Usage and Notes backends over `wormhole-core`, through the FFI (`wormhole-ffi`'s
// `files_panel`, FFI-CONTRACT.md "Files, Project, Usage and Notes"). Linked builds only.
//
// Wiring for the host: `let backends = try FilesBridge.makeBackends(authenticator:, effects:)`, then
// `FilesStore(backend: backends.files)`, `ProjectStore(backend: backends.project)`, `UsageStore(backend: backends.usage)`,
// and the Notes tab's `backends.notes` (load, save, sync). The data folder is core's (`WORMHOLE_DATA_DIR`, which must
// be `Wormhole-swift*`: the library refuses to start otherwise, so this throws).

import PanelModel
import PlatformModel
import WormholeBindings

public struct PanelBackends: Sendable {
    public let files: RustFilesBackend
    public let project: RustProjectBackend
    public let usage: RustUsageBackend
    public let notes: RustNotesPersistence
}

public enum FilesBridge {
    /// Core's notes text rules for the Notes tab (the paragraph ⌘↵ sends, a note's title), installed once at launch.
    public static func installNoteRules() {
        NoteRules.paragraphAt = { text, cursor in noteParagraphAt(text: text, cursor: UInt64(max(cursor, 0))) }
        NoteRules.title = { noteTitle(text: $0) }
    }

    /// - Parameters:
    ///   - authenticator: asked before a project is trusted (Touch ID or the password).
    ///   - effects: performs what a project's action, prompt or layout asks of the window, and says what was refused.
    public static func makeBackends(authenticator: any Authenticating, effects: any PanelEffectPerforming) throws -> PanelBackends {
        let project = try ProjectService(authenticator: OwnerAuthentication(authenticator))
        let relay = UsageRelay()
        let usage = try UsageService(listener: relay)
        relay.attach(usage)
        return PanelBackends(
            files: RustFilesBackend(service: try FilesService()),
            project: RustProjectBackend(service: project, effects: effects),
            usage: RustUsageBackend(service: usage, relay: relay),
            notes: RustNotesPersistence(notes: try SessionNotesStore(), project: project))
    }
}

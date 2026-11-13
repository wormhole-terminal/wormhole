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
    /// Core's rules for the panel, installed once at launch (`PlatformSetup`): the notes' text (the paragraph ⌘↵ sends, a note's
    /// title), how a line of code is drawn, sizes in words, and opening a repository's file in an editor.
    @MainActor public static func installRules() {
        NoteRules.paragraphAt = { text, cursor in noteParagraphAt(text: text, cursor: UInt64(max(cursor, 0))) }
        NoteRules.title = { noteTitle(text: $0) }
        NoteRules.body = { noteBody(text: $0) }
        NoteRules.cap = { lines, editing, showAll in
            noteCap(lines: UInt64(max(lines, 0)), editing: editing, showAll: showAll).map { Int($0) }
        }
        NoteRules.moveBy = { key, mods in
            switch noteItemKey(key: KeySpecRecord(mods: UInt8(truncatingIfNeeded: mods), key: key), empty: false) {
            case .moveUp: -1
            case .moveDown: 1
            default: nil
            }
        }
        NoteRules.closeStake = { notesCloseStake(openItems: UInt64(max($0, 0))) }
        VisibleRules.line = { text, tab in
            let line = visibleLine(text: text, tabWidth: UInt32(max(tab, 1)))
            return VisibleLine(text: line.text, offsets: line.offsets.map(Int.init), marks: line.marks.map { Int($0.start)..<Int($0.end) })
        }
        GitProblem.rule = { message, history in
            let words = gitProblem(message: message, history: history)
            return GitProblem(headline: words.headline, hint: words.hint, detail: words.detail, copy: words.copy)
        }
        TabSpeech.tab = { name, files, steps, notes in
            panelTabSpoken(tab: name, files: files.map { UInt32(max($0, 0)) }, openSteps: UInt32(max(steps, 0)), notes: UInt32(max(notes, 0)))
        }
        ByteSize.core = { byteSize(bytes: $0, binary: $1) }
        RepoFiles.resolver = { repoFileResolve(root: $0, relative: $1) }
        RepoFiles.knownApp = { openInKnown(bundle: $0) }
        RepoFiles.opener = { openFileRun(bundle: $0, path: $1, line: $2.map { UInt32(max($0, 0)) }, appFolder: $3) }
        RepoFiles.tooltip = { openFileTooltip(bundle: $0, app: $1, line: $2.map { UInt32(max($0, 0)) }) }
    }

    private static func makeUsage() throws -> RustUsageBackend {
        let relay = UsageRelay()
        let service = try UsageService(listener: relay)
        relay.attach(service)
        return RustUsageBackend(service: service, relay: relay)
    }

    /// - Parameters:
    ///   - authenticator: asked before a project is trusted (Touch ID or the password).
    ///   - effects: performs what a project's action, prompt or layout asks of the window, and says what was refused.
    ///   - profile: the window's own profile, whose database holds its notes (not the one Wormhole is in).
    ///   - usage: the app's one usage backend, which every window draws and the hook socket feeds (GPUI's app-wide
    ///     entity); nil makes one.
    public static func makeBackends(
        authenticator: any Authenticating, effects: any PanelEffectPerforming, profile: String, usage: RustUsageBackend? = nil
    ) throws -> PanelBackends {
        let project = try ProjectService(authenticator: OwnerAuthentication(authenticator))
        // The window's own profile's trust store and switches, whatever profile Wormhole is in (`hub_of(profile)`).
        _ = project.followProfile(profile: profile)
        let files = RustFilesBackend(service: try FilesService())
        // Its pins too are the window's own profile's (`features/files/pins.rs`).
        files.followProfile(profile)
        return try PanelBackends(
            files: files,
            project: RustProjectBackend(service: project, effects: effects, biometricsAvailable: authenticator.biometricsAvailable()),
            usage: usage ?? Self.makeUsage(),
            notes: RustNotesPersistence(notes: try SessionNotesStore.forProfile(profile: profile), project: project))
    }
}

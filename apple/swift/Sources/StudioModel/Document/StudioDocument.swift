// The theme under edit, as one observable: the draft file and the file as last saved, the version of a pair that
// is shown, the token selected, what the draft resolves to (Rust's answer, not ours), and the undo history. Edits
// go through `StudioDocument+Edit`; every one is resolved first, and one Rust refuses is not taken, so the
// preview never shows a broken theme. The window previews the resolved draft through `ThemeStore.preview`.

import DesignSystem
import Foundation
import Observation

@MainActor @Observable
public final class StudioDocument {
    public internal(set) var file: ThemeFile
    public internal(set) var saved: ThemeFile
    public internal(set) var subject: ThemeSubject
    /// The version of a pair being edited ("Light", "Dark"); `nil` for one theme.
    public var shown: String?
    public var selected: String?
    public var filter = TokenFilter()
    public internal(set) var tokens: ThemeTokens
    public internal(set) var fieldError: FieldError?
    public internal(set) var notice: StudioNotice?
    /// What reading the file had to do (made with a newer Wormhole, entries skipped), when the core is there to say.
    public internal(set) var report: StudioFileRead?
    /// Whether the theme it was copied from has changed since (Update from), kept while the base stays the same.
    public internal(set) var update = UpdateCheck()
    @ObservationIgnored var updateKey: String?
    /// Whether the app shows this theme now (see `refreshInUse`), and the pick that was asked of.
    public internal(set) var inUse = true
    @ObservationIgnored var shownPick = ""
    /// What the file looked like on disk when it was read or last written: a write over another is asked about.
    @ObservationIgnored var disk: String?
    @ObservationIgnored var history: UndoStack
    @ObservationIgnored var theme: Theme
    @ObservationIgnored let workspace: DraftWorkspace
    @ObservationIgnored let environment: StudioEnvironment
    @ObservationIgnored let store: ThemeStore
    @ObservationIgnored public let groups: [TokenGroup]

    public init(
        file: ThemeFile, subject: ThemeSubject, disk: String? = nil, shown: String? = nil, environment: StudioEnvironment,
        store: ThemeStore, report: StudioFileRead? = nil
    ) throws {
        let version = shown ?? file.versions.first
        let contrast = store.increaseContrast
        let workspace = try DraftWorkspace(
            resolver: environment.resolver, folder: environment.folder, editing: subject.url,
            look: environment.library.defaultLook)
        let first = try workspace.resolve(file, version: version, increaseContrast: contrast)
        let tokens = first.tokens(increaseContrast: contrast)
        (self.file, saved, self.subject, self.disk, self.shown) = (file, file, subject, disk, version)
        (self.environment, self.store, self.workspace) = (environment, store, workspace)
        (history, theme, self.tokens, self.report) = (UndoStack(start: file), first, tokens, report)
        groups = TokenGroup.all(library: environment.library, syntaxRoots: environment.knowledge.syntaxRoots(among: tokens.syntaxKeys))
        selected = "primary"
        store.preview = first
        refreshUpdate()
        refreshInUse()
    }

    public var name: String { file.name ?? "" }
    public var isDirty: Bool { file != saved }
    public var appearance: ThemeAppearance { theme.appearance }
    public var increaseContrast: Bool { store.increaseContrast }

    /// Puts the person's own pick back in every window: the Studio is closing.
    public func close() { store.preview = nil }

    /// Resolves again, because something outside the file changed (Increase Contrast, a version shown).
    public func refresh() {
        if let resolved = try? workspace.resolve(file, version: shown, increaseContrast: store.increaseContrast) {
            apply(resolved)
        }
    }

    func apply(_ resolved: Theme) {
        theme = resolved
        tokens = resolved.tokens(increaseContrast: store.increaseContrast)
        store.preview = resolved
        refreshUpdate()
    }

    /// Made with a newer Wormhole than this one: saving would lose what it does not know, so it is never written back.
    public var isNewer: Bool { report?.newer == true }

    /// Asks whether the base under the shown version still matches its source, when the base (or the version) changed.
    func refreshUpdate() {
        let key = file.baseSource(in: shown).map { "\($0.from)|\($0.copied ?? "")|\(shown ?? "")" }
        guard key != updateKey else { return }
        updateKey = key
        update = key == nil ? UpdateCheck() : ((try? environment.files?.updateCheck(file.json(), shown: shown)) ?? UpdateCheck())
    }
}

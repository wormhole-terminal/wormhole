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
        store: ThemeStore
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
        (history, theme, self.tokens) = (UndoStack(start: file), first, tokens)
        groups = TokenGroup.all(library: environment.library, syntaxRoots: environment.knowledge.syntaxRoots(among: tokens.syntaxKeys))
        selected = "primary"
        store.preview = first
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
    }
}

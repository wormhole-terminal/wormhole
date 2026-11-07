// The Studio window's state above one document: which theme is open, what saving, opening another and closing
// do when there are edits, and the name being typed for a copy that has no file yet. Questions are the system's
// sheets (`StudioQuestions`); everything about the theme itself is the document's.

import AppKit
import DesignSystem
import Observation
import StudioModel

@MainActor @Observable
public final class StudioSession {
    public private(set) var document: StudioDocument
    public var renaming = false
    /// Save was asked of a copy with no name of its own yet: the name is being asked for first.
    private var savingAfterRename = false
    @ObservationIgnored public weak var window: NSWindow?
    @ObservationIgnored let environment: StudioEnvironment
    @ObservationIgnored let store: ThemeStore

    /// Opens on `theme`, the one in use: its own file is edited in place, anything else as a copy.
    public init(theme: String, environment: StudioEnvironment, store: ThemeStore = .shared) throws {
        (self.environment, self.store) = (environment, store)
        document = try StudioDocument.open(theme: theme, environment: environment, store: store)
    }

    public var userThemeNames: [String] { store.userThemes.map(\.name) }
    public var builtInNames: [String] { environment.library.themes.map(\.name) }

    // MARK: Opening

    /// Opens another theme, asking about unsaved edits first. `nil` starts a new one from the current draft's base.
    public func open(_ name: String) async {
        guard await settleEdits() else { return }
        if let next = try? StudioDocument.open(theme: name, environment: environment, store: store) { replace(with: next) }
    }

    private func replace(with next: StudioDocument) {
        document.close()
        document = next
        store.preview = next.themePreview
    }

    // MARK: Saving

    public func save() async {
        if case .untitled = document.subject, !renamed {
            savingAfterRename = true
            renaming = true
            return
        }
        await write()
    }

    /// The name was typed: the title row's field is done. A copy waiting to be saved is saved now.
    public func finishRenaming(_ name: String) async {
        renaming = false
        document.rename(name)
        renamed = true
        if savingAfterRename { savingAfterRename = false; await write() }
    }

    private var renamed = false

    private func write() async {
        do {
            try document.save()
        } catch SaveError.changedOnDisk {
            if await StudioQuestions.changedOnDisk(name: document.name, on: window) { try? document.save(overwriting: true) }
        } catch {
            document.report(error)
        }
    }

    public func saveCopy() { do { try document.saveCopy() } catch { document.report(error) } }

    public func trash() async {
        guard document.subject.url != nil, await StudioQuestions.trash(name: document.name, on: window) else { return }
        if (try? document.trash()) != nil, let next = try? StudioDocument.open(theme: store.theme.name, environment: environment, store: store) {
            replace(with: next)
        }
    }

    // MARK: Closing

    /// Whether the window may close: asks about unsaved edits and puts the person's own pick back in every window.
    public func requestClose() async -> Bool {
        guard await settleEdits() else { return false }
        document.close()
        return true
    }

    /// Asks what to do with unsaved edits; `true` when the caller may go on (saved, or dropped).
    private func settleEdits() async -> Bool {
        guard document.isDirty else { return true }
        switch await StudioQuestions.unsaved(name: document.name, changes: document.changeCount, on: window) {
        case .cancel: return false
        case .discard: return true
        case .save:
            await save()
            return !document.isDirty
        }
    }
}

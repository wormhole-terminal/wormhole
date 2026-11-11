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
    /// The Update from… sheet is up.
    public var showsUpdate = false
    /// Save was asked of a copy with no name of its own yet: the name is being asked for first.
    private var savingAfterRename = false
    @ObservationIgnored public weak var window: NSWindow?
    /// The panel and pasteboard Export JSON and Copy JSON use.
    @ObservationIgnored public var outputs = StudioOutputs.system
    @ObservationIgnored let environment: StudioEnvironment
    @ObservationIgnored let store: ThemeStore

    /// Opens on `theme`, the one in use: its own file is edited in place, anything else as a copy.
    public init(theme: String, environment: StudioEnvironment, store: ThemeStore = .shared) throws {
        (self.environment, self.store) = (environment, store)
        document = try StudioDocument.open(theme: theme, environment: environment, store: store)
    }

    /// The themes the theme menu offers: the person's own, then the built-ins with a family's appearances together
    /// (core's list); without the core, the same as flat lists.
    public var themeList: ThemeList {
        if let list = environment.files?.themeList() { return list }
        let own = store.userThemes.map { ThemeChoice(name: $0.name, value: $0.path ?? $0.name) }
        let builtins = environment.library.themes.map {
            ThemeFamily(name: $0.name, wormhole: false, choices: [ThemeChoice(name: $0.name, value: $0.name)])
        }
        return ThemeList(own: own, builtins: builtins)
    }

    /// Whether the theme menu offers "This Project": the project of the terminal in front has a theme in effect.
    public var hasProjectTheme: Bool { environment.project?.overlay() != nil }

    // MARK: Opening

    /// Opens another theme by its name, asking about unsaved edits first.
    public func open(_ name: String) async {
        await switchTo(name) { try StudioDocument.open(theme: name, environment: self.environment, store: self.store) }
    }

    /// The menu's choice of one of the person's themes (by its file) or a built-in (by its name, as a copy to name).
    public func open(_ choice: ThemeChoice, builtin: Bool) async {
        await switchTo(choice.name) {
            builtin
                ? try StudioDocument.untitled(from: choice.value, environment: self.environment, store: self.store)
                : try StudioDocument.open(file: URL(fileURLWithPath: choice.value), environment: self.environment, store: self.store)
        }
    }

    /// New Theme…: a neutral palette of the appearance in use. New Theme From ▸: a copy of a theme to start from.
    public func openNew() async {
        await switchTo(StudioNames.untitled) {
            try StudioDocument.new(dark: self.store.shownAppearance == .dark, environment: self.environment, store: self.store)
        }
    }

    public func newFrom(_ choice: ThemeChoice, builtin: Bool) async {
        await switchTo(StudioNames.untitled(from: choice.name)) {
            try StudioDocument.untitled(from: choice.value, display: choice.name, environment: self.environment, store: self.store)
        }
    }

    public func openProject() async {
        await switchTo(StudioWords.menuProjectTheme) {
            try StudioDocument.open(theme: self.document.name, environment: self.environment, store: self.store)
        }
    }

    private func switchTo(_ next: String, _ make: () throws -> StudioDocument) async {
        guard await settleEdits(opening: next) else { return }
        do { replace(with: try make()) } catch { document.fail(StudioWords.openFailed) }
    }

    private func replace(with next: StudioDocument) {
        document.close()
        document = next
        store.preview = next.themePreview
        renamed = false
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

    private func write(overwriting: Bool = false, replacing: Bool = false) async {
        do {
            try document.save(overwriting: overwriting, replacing: replacing)
        } catch SaveError.changedOnDisk {
            if await StudioQuestions.changedOnDisk(name: document.name, on: window) { await write(overwriting: true) }
        } catch SaveError.exists(let name) {
            if await StudioQuestions.exists(name: name, on: window) { await write(replacing: true) }
        } catch {
            document.report(error)
        }
    }

    /// Duplicate (⇧⌘S): a copy under a free name, in a file of its own, which the window goes on editing.
    public func duplicate() {
        do { replace(with: try document.duplicate()) } catch {
            document.fail(StudioWords.copyFailed((error as? LocalizedError)?.errorDescription ?? "\(error)"))
        }
    }

    /// The Preview chip's Use: the theme is made the pick of its appearance, saving first when there is something to save.
    public func use() async {
        if document.isDirty || document.subject.url == nil {
            await save()
            guard !document.isDirty, !renaming, document.subject.url != nil else { return }
        }
        document.use()
    }

    public func useInProject() {
        do { try document.useInProject() } catch { document.report(error) }
    }

    public func addAppearance(_ to: ThemeAppearance) { document.addAppearance(to) }

    /// Update from…: the sheet lists what changed in the source since the base was copied.
    public func askUpdate() { showsUpdate = document.updateRows() != nil }

    public func applyUpdate() {
        document.applyUpdate()
        showsUpdate = false
    }

    public func trash() async {
        guard document.subject.url != nil, await StudioQuestions.trash(name: document.name, on: window) else { return }
        if (try? document.trash()) != nil, let next = try? StudioDocument.open(theme: store.shownPick, environment: environment, store: store) {
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
    private func settleEdits(opening: String? = nil) async -> Bool {
        guard document.isDirty else { return true }
        switch await StudioQuestions.unsaved(name: document.name, changes: document.changeCount, opening: opening, on: window) {
        case .cancel: return false
        case .discard: return true
        case .save:
            await save()
            return !document.isDirty
        }
    }
}

// Saving (`studio/save.rs`, `convert.rs`): the theme into the themes folder (what the draft overrides, expressions as
// typed), over its own file or, the first time, a new one named for it; the picks follow (a saved theme becomes the
// pick of its appearance unless it is one already, and a rename makes the picks that named it follow), a theme of the
// older format is written in the new one by its first save, a copy under a free name that is then what is edited, and
// the JSON to export or copy. A name another theme has asks Replace; a file another application changed since it was
// read asks Overwrite (`SaveError`); a file from a newer Wormhole is never written back.

import DesignSystem
import Foundation

public enum SaveError: Error, Equatable {
    case changedOnDisk, nameFirst
    /// Another theme file already has the name: Replace, or Cancel.
    case exists(String)
    /// The name is that of one version of a theme that ships several: never written over.
    case nameTaken(String)
    case newer, convert
}

extension StudioDocument {
    private var folder: any ThemeFolder { environment.folder }

    /// The files by the names they answer to (the file's name, and each version's full name), lower-cased.
    private func names(except url: URL?) -> (themes: [String: URL], versions: Set<String>) {
        var themes: [String: URL] = [:]
        var versions: Set<String> = []
        for other in folder.files() where other != url {
            guard let text = try? folder.read(other), let theme = try? ThemeFile(parsing: text) else { continue }
            themes[(theme.name ?? other.deletingPathExtension().lastPathComponent).lowercased()] = other
            for version in theme.versions { versions.insert(theme.themeName(version: version).lowercased()) }
        }
        return (themes, versions)
    }

    /// Writes the draft: over its own file (asking nothing but a change made under it), or as a new theme named for
    /// it (`replacing`: over the file that has the name, after the person said so). A project's theme goes back to the project.
    @discardableResult
    public func save(overwriting: Bool = false, replacing: Bool = false) throws -> URL {
        guard !name.isEmpty else { throw SaveError.nameFirst }
        if isNewer { throw SaveError.newer }
        switch subject {
        case .project: return try useInProject()
        case .own(let url): return try writeOwn(url, check: !overwriting)
        case .untitled: return try writeNew(replacing: replacing)
        }
    }

    private func writeOwn(_ url: URL, check: Bool) throws -> URL {
        var draft = file
        var converted = false
        // A theme with no id stored keeps the one it answered to (core's `save_id` keeps the file's own), so a rename
        // does not leave its earlier saves under an id it no longer has.
        if draft.isV1, draft.id == nil, let files = environment.files { draft.id = files.newID(name: name, itself: url) }
        if !draft.isV1, let files = environment.files {
            // The first save of a theme in the older format writes the new one: nothing on screen may change.
            let id = files.newID(name: name, itself: url)
            guard let text = try? files.convert(file.json(), id: id), let made = try? ThemeFile(parsing: text) else { throw SaveError.convert }
            (draft, converted) = (made, true)
        }
        if check, let disk, (try? folder.read(url)) != disk { throw SaveError.changedOnDisk }
        let was = saved.name
        let text = try canonical(draft)
        let lostComments = Self.hasComments(disk ?? "")
        try folder.write(text, to: url)
        file = draft
        finish(url: url, text: text, new: false, converted: converted, lostComments: lostComments)
        if was != draft.name { renamePicks(from: was ?? "", url: url) }
        return url
    }

    private func writeNew(replacing: Bool) throws -> URL {
        let known = names(except: nil)
        if known.versions.contains(name.lowercased()) { throw SaveError.nameTaken(name) }
        let same = known.themes[name.lowercased()]
        if same != nil, !replacing { throw SaveError.exists(name) }
        var draft = file
        if draft.isV1, draft.id == nil, let files = environment.files { draft.id = files.newID(name: name, itself: same) }
        let target = same ?? freeFile(for: name)
        let text = try canonical(draft)
        let lostComments = same.flatMap { try? folder.read($0) }.map(Self.hasComments) ?? false
        try folder.write(text, to: target)
        file = draft
        finish(url: target, text: text, new: true, converted: false, lostComments: lostComments)
        return target
    }

    /// "<name>.json", or "<name>-2.json" and on while that file holds something else.
    private func freeFile(for name: String) -> URL {
        let first = folder.directory.appendingPathComponent(ThemeFileNames.fileName(for: name))
        let taken = Set(folder.files())
        guard taken.contains(first) else { return first }
        let (stem, directory) = (first.deletingPathExtension().lastPathComponent, folder.directory)
        return (2...).lazy.map { directory.appendingPathComponent("\(stem)-\($0).json") }.first { !taken.contains($0) } ?? first
    }

    private func canonical(_ draft: ThemeFile) throws -> String {
        let text = draft.json()
        guard draft.isV1, let files = environment.files else { return text }
        return try files.canonical(text)
    }

    /// The file as it stands is now what was saved; the theme is a pick of its appearance if it was not one already.
    private func finish(url: URL, text: String, new: Bool, converted: Bool, lostComments: Bool) {
        subject = .own(url)
        (disk, saved) = (text, file)
        store.reloadThemes()
        let set = pickAfterSave(url)
        // The preview ends (the theme itself shows now); the next edit previews again.
        store.preview = nil
        var line = set ? StudioText.savedAndSet(url.path) : StudioText.saved(url.path)
        if converted { line += " " + StudioText.savedConverted }
        if lostComments { line += " " + StudioText.savedComments }
        notice = .saved(url, text: line)
        refreshInUse()
        updateKey = nil
        refreshUpdate()
    }

    /// What a pick holds to select this theme as the version `shown` of it: the id of a pair's version, else the file.
    public func pickValue(_ version: String?) -> String? {
        guard case .own(let url) = subject else { return nil }
        if let id = file.id, let version, file.isPair { return "\(id).\(version)" }
        return url.path
    }

    private func pickAfterSave(_ url: URL) -> Bool {
        guard let value = pickValue(shown) else { return false }
        if let picks = store.picks { return picks.pickAfterSave(value, dark: appearance == .dark, path: url.path) }
        guard let id = store.userThemes.first(where: { $0.name == name })?.id else { return false }
        store.setTheme(id)
        return true
    }

    /// A pick that named the theme by the name it had (or a file stem) names it by its file now (or, for a pair, by
    /// the id of the same version): nothing is lost. A pick that names a file or an id is not touched.
    private func renamePicks(from old: String, url: URL) {
        guard let picks = store.picks else { return }
        if file.isPair, let id = file.id {
            var renames = file.versions.map { (old: "\(old) \($0)", new: pickValue($0) ?? "\(id).\($0)") }
            if let first = file.versions.first { renames.append((old: old, new: pickValue(first) ?? url.path)) }
            picks.renamePicks(renames)
        } else {
            picks.renamePicks([(old: old, new: url.path)])
        }
        store.reloadThemes()
    }

    /// A copy under a free "<name> copy", in a file of its own, which the window goes on editing (Duplicate, ⇧⌘S): the
    /// theme as it stands, edits and all; the original stays as it was saved and the picks do not change. The new
    /// document, over the copy.
    public func duplicate() throws -> StudioDocument {
        guard !name.isEmpty else { throw SaveError.nameFirst }
        let (themes, versions) = names(except: nil)
        let copy = ThemeFileNames.copyName(name) { themes[$0.lowercased()] != nil || versions.contains($0.lowercased()) }
        var made = file
        made.name = copy
        if made.isV1, let files = environment.files { made.id = files.newID(name: copy, itself: nil) }
        let target = freeFile(for: copy)
        let text = try canonical(made)
        try folder.write(text, to: target)
        store.reloadThemes()
        let next = try StudioDocument(
            file: (try? ThemeFile(parsing: text)) ?? made, subject: .own(target), disk: text, shown: shown, environment: environment,
            store: store, report: report.map { StudioFileRead(version: $0.version) })
        next.notice = .copySaved(name: copy, target)
        return next
    }

    /// The theme as Export and Copy JSON write it: for a theme of the new format, the one standalone theme the shown
    /// appearance works out to, in the format every release reads.
    public var exportJSON: String {
        guard file.isV1, let files = environment.files, let text = try? files.export(file.json(), shown: shown) else { return file.json() }
        return text
    }
    public var exportFileName: String { ThemeFileNames.exportName(name) }

    /// For a pair, the sentence that says only the shown appearance went out.
    private var onlyShownNote: String? {
        guard file.isPair, let shown else { return nil }
        return StudioText.onlyShown(StudioText.appearanceWord(shown))
    }

    public func exported(to url: URL) throws {
        try exportJSON.write(to: url, atomically: true, encoding: .utf8)
        notice = .exported(url, note: onlyShownNote)
    }

    public func copiedJSON() { notice = .copiedJSON(note: onlyShownNote) }

    /// "Use in this project…": the theme as the `theme.json` of the project of the terminal in front.
    @discardableResult
    public func useInProject() throws -> URL {
        guard let project = environment.project else { throw SaveError.convert }
        if case .project(let root) = subject, let now = project.overlay()?.root, now != root {
            throw StudioFailure(StudioText.otherProject)
        }
        switch project.write(exportJSON) {
        case .success(let url):
            if case .project = subject { (saved, disk) = (file, nil) }
            notice = .saved(url, text: [StudioText.projectSaved(url.path), onlyShownNote].compactMap { $0 }.joined(separator: " "))
            return url
        case .failure(let failure):
            throw failure
        }
    }

    /// Says something went wrong, in the Studio's own words (a theme that could not be opened, a copy that could not be saved).
    public func fail(_ reason: String) { notice = .failed(reason) }

    /// The notice for a refusal or a failure, in the words the Studio uses.
    public func report(_ error: Error) {
        switch error {
        case SaveError.nameFirst: notice = .failed(StudioText.nameFirst)
        case SaveError.nameTaken(let taken): notice = .failed(StudioText.nameIsVersion(taken))
        case SaveError.exists(let taken): notice = .failed(StudioText.nameTaken(taken))
        case SaveError.newer: notice = .failed(StudioText.newerFile)
        case SaveError.convert: notice = .failed(StudioText.convertFailed)
        case let failure as StudioFailure: notice = .failed(StudioText.projectFailed(failure.reason))
        default: notice = .failed(StudioText.saveFailed((error as? LocalizedError)?.errorDescription ?? "\(error)"))
        }
    }

    /// Moves the theme's file to the Trash; the caller opens what is now in use.
    public func trash() throws {
        guard let url = subject.url else { return }
        do {
            try folder.trash(url)
            store.reloadThemes()
        } catch {
            notice = .failed(StudioText.trashFailed(name))
            throw error
        }
    }

    /// Whether JSON text has a `//` or `/* */` comment outside its strings (a save writes none back).
    static func hasComments(_ text: String) -> Bool {
        var (inString, escaped) = (false, false)
        var previous: Character?
        for c in text {
            if inString {
                if escaped { escaped = false } else if c == "\\" { escaped = true } else if c == "\"" { inString = false }
            } else if c == "\"" {
                inString = true
            } else if previous == "/", c == "/" || c == "*" {
                return true
            }
            previous = c
        }
        return false
    }
}

extension StudioDocument {
    /// How many tokens differ from the file as saved, for "3 changes. Your changes are lost if you don't save them."
    public var changeCount: Int { ThemeFile.changedTokens(saved, file, among: groups.flatMap(\.tokens)).count }

    /// The resolved draft, to show in every window (after another theme was opened).
    public var themePreview: Theme { theme }
}

// Opening the Studio on the theme in use (`studio/opening.rs`, `document/subject.rs`): a trusted project's theme in effect
// is edited as the project has it; a file of the person's own, in either format, in place (one that ships several
// versions only in the new format: a hand-written pair is only ever copied); anything else (a built-in, a pick that
// is not a file we can edit) becomes "Untitled (from X)", a copy that has nothing of its own yet and asks for its
// name on the first save. With the core's file rules (`StudioFiles`) a copy of a built-in or of a theme of the
// person's carries the values it was copied from in a `base` (version 1); without them it extends the theme by name.

import DesignSystem
import Foundation

extension StudioDocument {
    /// Opens `name` (what is in use: the shown theme's name, or one version of a file by its full name).
    public static func open(theme name: String, environment: StudioEnvironment, store: ThemeStore) throws -> StudioDocument {
        if let project = environment.project?.overlay(), let file = try? ThemeFile(parsing: project.text) {
            return try open(file: named(file, stem: nil), text: project.text, subject: .project(root: project.root), shown: nil, environment: environment, store: store)
        }
        if let found = locate(name, environment: environment, store: store), editable(found.file) {
            return try open(file: found.file, text: found.text, subject: .own(found.url), shown: found.shown, environment: environment, store: store)
        }
        return try untitled(from: name, environment: environment, store: store)
    }

    /// The theme menu's choice of one of the person's themes, by its file.
    public static func open(file url: URL, environment: StudioEnvironment, store: ThemeStore) throws -> StudioDocument {
        let text = try environment.folder.read(url)
        let file = named(try ThemeFile(parsing: text), stem: url.deletingPathExtension().lastPathComponent)
        guard editable(file) else {
            return try untitled(from: file.name ?? "", environment: environment, store: store)
        }
        return try open(file: file, text: text, subject: .own(url), shown: nil, environment: environment, store: store)
    }

    /// The menu's New Theme: a neutral palette of the appearance in use, nothing under it.
    public static func new(dark: Bool, environment: StudioEnvironment, store: ThemeStore) throws -> StudioDocument {
        let name = StudioText.untitled
        guard let files = environment.files, let file = try? ThemeFile(parsing: files.newTheme(dark: dark, name: name)) else {
            return try StudioDocument(file: ThemeFile(name: name), subject: .untitled(from: ""), environment: environment, store: store)
        }
        return try StudioDocument(file: file, subject: .untitled(from: ""), environment: environment, store: store)
    }

    /// A new theme that starts out as `base` (a built-in's name, or the file of a theme of the person's): "Untitled
    /// (from <base>)", and no file yet.
    public static func untitled(from base: String, display: String? = nil, environment: StudioEnvironment, store: ThemeStore) throws -> StudioDocument {
        let name = StudioText.untitledFrom(display ?? base)
        if let files = environment.files, let text = files.copyTheme(from: base, name: name), let file = try? ThemeFile(parsing: text) {
            return try StudioDocument(file: file, subject: .untitled(from: display ?? base), environment: environment, store: store, report: StudioFileRead(version: 1))
        }
        let file = ThemeFile(name: StudioText.customName(base), extends: base)
        return try StudioDocument(file: file, subject: .untitled(from: base), environment: environment, store: store)
    }

    private static func open(
        file: ThemeFile, text: String, subject: ThemeSubject, shown: String?, environment: StudioEnvironment, store: ThemeStore
    ) throws -> StudioDocument {
        let report = try? environment.files?.read(text)
        return try StudioDocument(file: file, subject: subject, disk: text, shown: shown, environment: environment, store: store, report: report)
    }

    /// A file that ships several versions is edited in place only once the Studio itself wrote it (the new format, whose
    /// pair it keeps whole); a hand-written one is only ever copied.
    private static func editable(_ file: ThemeFile) -> Bool { file.versions.isEmpty || file.isV1 }

    /// A theme without a name of its own is called what its file is called.
    private static func named(_ file: ThemeFile, stem: String?) -> ThemeFile {
        guard (file.name ?? "").trimmingCharacters(in: .whitespaces).isEmpty else { return file }
        var named = file
        named.name = stem ?? StudioText.defaultName
        return named
    }

    /// The file `name` is: a theme of the person's by its listed name (its path, and for a version of a pair, which),
    /// else by the family name or file name.
    private static func locate(_ name: String, environment: StudioEnvironment, store: ThemeStore) -> (url: URL, text: String, file: ThemeFile, shown: String?)? {
        let folder = environment.folder
        func same(_ a: String, _ b: String) -> Bool { a.caseInsensitiveCompare(b) == .orderedSame }
        let listed = store.userThemes.first { same($0.name, name) }.flatMap { $0.path.map { URL(fileURLWithPath: $0) } }
        for url in ([listed].compactMap { $0 } + folder.files()) {
            guard let text = try? folder.read(url), let parsed = try? ThemeFile(parsing: text) else { continue }
            let file = named(parsed, stem: url.deletingPathExtension().lastPathComponent)
            let family = file.name ?? ""
            if let version = file.versions.first(where: { same(file.themeName(version: $0), name) }) { return (url, text, file, version) }
            if same(family, name) || url == listed { return (url, text, file, nil) }
        }
        return nil
    }
}

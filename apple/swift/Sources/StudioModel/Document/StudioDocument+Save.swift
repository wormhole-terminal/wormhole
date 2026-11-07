// Saving: the theme into the themes folder (what the draft overrides, expressions as typed), a copy under a free
// name, the JSON to export or copy. A name another theme has is never overwritten silently, and a file another
// application changed since it was read is asked about first (`SaveError.changedOnDisk`).

import DesignSystem
import Foundation

public enum SaveError: Error, Equatable {
    case changedOnDisk, nameTaken(String), nameFirst
}

extension StudioDocument {
    private var folder: any ThemeFolder { environment.folder }

    private func names(except url: URL?) -> [String: URL] {
        var taken: [String: URL] = [:]
        for other in folder.files() where other != url {
            if let text = try? folder.read(other), let theme = try? ThemeFile(parsing: text) {
                for known in [theme.name ?? other.deletingPathExtension().lastPathComponent] + theme.versions.map({ theme.themeName(version: $0) }) {
                    taken[known.lowercased()] = other
                }
            }
        }
        return taken
    }

    /// Writes the draft to its file (or, the first time, to a new one named for it) and sets it as the theme.
    @discardableResult
    public func save(overwriting: Bool = false) throws -> URL {
        guard !name.isEmpty else { throw SaveError.nameFirst }
        let target = subject.url ?? folder.directory.appendingPathComponent(ThemeFileNames.fileName(for: name))
        if names(except: target)[name.lowercased()] != nil { throw SaveError.nameTaken(name) }
        if subject.url == nil, folder.files().contains(target) { throw SaveError.nameTaken(name) }
        if !overwriting, let disk, (try? folder.read(target)) != disk { throw SaveError.changedOnDisk }
        let text = file.json()
        try folder.write(text, to: target)
        (subject, disk, saved) = (.own(target), text, file)
        store.reloadThemes()
        if let id = store.userThemes.first(where: { $0.name == name })?.id { store.setTheme(id) }
        notice = .saved(target, setAsTheme: store.userThemes.contains { $0.name == name })
        return target
    }

    /// A copy under a free "<name> copy"; the theme in use and this draft stay as they are.
    @discardableResult
    public func saveCopy() throws -> URL {
        let taken = names(except: nil)
        let copy = ThemeFileNames.copyName(name) { taken[$0.lowercased()] != nil }
        var duplicate = file
        duplicate.name = copy
        let target = folder.directory.appendingPathComponent(ThemeFileNames.fileName(for: copy))
        try folder.write(duplicate.json(), to: target)
        store.reloadThemes()
        notice = .copySaved(name: copy, target)
        return target
    }

    /// The theme as Export and Copy JSON write it.
    public var exportJSON: String { file.json() }
    public var exportFileName: String { ThemeFileNames.exportName(name) }

    public func exported(to url: URL) throws {
        try exportJSON.write(to: url, atomically: true, encoding: .utf8)
        notice = .exported(url)
    }

    public func copiedJSON() { notice = .copiedJSON }

    /// The notice for a refusal or a failure, in the words the Studio uses.
    public func report(_ error: Error) {
        switch error {
        case SaveError.nameFirst: notice = .failed(StudioText.nameFirst)
        case SaveError.nameTaken(let taken): notice = .failed(StudioText.nameTaken(taken))
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
}

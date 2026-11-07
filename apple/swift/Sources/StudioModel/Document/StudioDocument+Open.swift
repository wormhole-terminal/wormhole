// Opening the Studio on the theme in use: a file of the person's own is edited in place; anything else (a built-in,
// a file in another format) becomes a copy that `extends` it by name and asks for its name on the first save
// (`studio/opening.rs`, `document/subject.rs`).

import DesignSystem
import Foundation

extension StudioDocument {
    /// Opens `name` (what `settings.theme` holds: a theme's name, or one version of a file by its full name).
    public static func open(theme name: String, environment: StudioEnvironment, store: ThemeStore) throws -> StudioDocument {
        let folder = environment.folder
        for url in folder.files() {
            guard let text = try? folder.read(url), let file = try? ThemeFile(parsing: text), file.root["version"] == nil else { continue }
            let family = file.name ?? url.deletingPathExtension().lastPathComponent
            let versions = file.versions.first { "\(family) \($0)" == name }
            if family == name || versions != nil {
                return try StudioDocument(file: file, subject: .own(url), disk: text, shown: versions, environment: environment, store: store)
            }
        }
        return try untitled(from: name, environment: environment, store: store)
    }

    /// A new theme that starts out as `base`: its name's "<base> Custom", and no file yet.
    public static func untitled(from base: String, environment: StudioEnvironment, store: ThemeStore) throws -> StudioDocument {
        let file = ThemeFile(name: StudioText.customName(base), extends: base)
        return try StudioDocument(file: file, subject: .untitled(from: base), environment: environment, store: store)
    }
}

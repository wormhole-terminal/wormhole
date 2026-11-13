// `StudioFiles` over the core: every rule about a theme file (its format and versions, a copy's base, ids, a pair, an
// update) is `wormhole-core`'s, asked through `studio_files.rs` with the profile's themes folder.

import Foundation
import StudioModel
import WormholeBindings

public struct FfiStudioFiles: StudioFiles {
    /// The themes folder, asked at each call: an open Studio follows a profile switch.
    let folder: @Sendable () -> URL
    var directory: String { folder().path }

    public init(directory: URL) { folder = { directory } }

    public init(folder: @escaping @Sendable () -> URL) { self.folder = folder }

    public func read(_ text: String) throws -> StudioFileRead {
        let read = try studioRead(text: text)
        return StudioFileRead(version: Int(read.version), newer: read.newer, skipped: Int(read.skipped))
    }

    public func canonical(_ text: String) throws -> String { try studioCanonical(text: text) }

    public func newTheme(dark: Bool, name: String) -> String { studioNewTheme(dark: dark, name: name) }

    public func copyTheme(from: String, name: String) -> String? { studioCopyTheme(themesDir: directory, from: from, name: name) }

    public func newID(name: String, itself: URL?) -> String { studioNewId(themesDir: directory, name: name, itself: itself?.path) }

    public func convert(_ text: String, id: String) throws -> String { try studioConvert(themesDir: directory, text: text, id: id) }

    public func addAppearance(_ text: String, dark: Bool, id: String) throws -> String {
        try studioAddAppearance(themesDir: directory, text: text, dark: dark, id: id)
    }

    public func updateCheck(_ text: String, shown: String?) throws -> UpdateCheck {
        let check = try studioUpdateCheck(themesDir: directory, text: text, shown: shown)
        let state: UpdateCheck.State =
            switch check.state {
            case "missing": .missing
            case "current": .current
            case "changed": .changed
            default: .none
            }
        return UpdateCheck(
            state: state, from: check.from, date: check.date,
            changes: check.changes.map { UpdateChange(key: $0.key, old: $0.old, new: $0.new, kept: $0.kept) })
    }

    public func updateApply(_ text: String, shown: String?) throws -> String { try studioUpdateApply(themesDir: directory, text: text, shown: shown) }

    public func export(_ text: String, shown: String?) throws -> String { try studioExport(themesDir: directory, text: text, shown: shown) }

    public func themeList() -> ThemeList {
        let list = studioThemeList(themesDir: directory)
        func choices(_ list: [StudioThemeChoice]) -> [ThemeChoice] { list.map { ThemeChoice(name: $0.name, value: $0.value) } }
        return ThemeList(
            own: choices(list.own),
            builtins: list.builtins.map { ThemeFamily(name: $0.name, wormhole: $0.wormhole, choices: choices($0.choices)) })
    }

    public func completions(_ text: String, caret: Int, target: String) -> FieldCompletions? {
        guard let found = studioCompletions(text: text, caret: UInt32(clamping: caret), target: target) else { return nil }
        return FieldCompletions(
            range: Int(found.start)..<Int(found.end),
            items: found.items.map { FieldCompletion(label: $0.label, insert: $0.insert, function: $0.function) })
    }

    public func contrastIssues(_ text: String, shown: String?) throws -> [ContrastIssue] {
        try studioContrastIssues(themesDir: directory, text: text, shown: shown).map { issue in
            ContrastIssue(
                check: ContrastCheck(fg: issue.fg, bg: issue.bg, underlay: issue.underlay, min: Double(issue.min), ratio: Double(issue.ratio)),
                fix: issue.fix.map { ContrastFix(token: $0.token, expression: $0.expression, label: $0.label) }, follows: issue.follows)
        }
    }
}

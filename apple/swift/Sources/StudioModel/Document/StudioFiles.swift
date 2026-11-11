// What the Studio asks of the core about theme files, as a protocol: reading one in whatever format it is in, writing
// the format this release writes (version 1: a `base` of copied values under the person's own blocks), making a
// new theme, a copy of another, a pair out of a single theme, and comparing a copy's base with its source. Every rule
// is core's (`wormhole-core`'s `theme::tokens::format`, the same the GPUI Studio is written over); text goes in and
// text comes out, so the draft stays a small mirror of the file's JSON and never decides what a version, a base or
// an id is. The FFI-backed one is `FfiStudioFiles` in `UniFFISource`; without the core there is none, and a draft
// then reads and writes the older format only.

import Foundation

public struct StudioFileRead: Sendable, Equatable {
    /// The version the file says it is: 0 when it says none.
    public var version: Int
    /// Made with a newer Wormhole: the Studio never writes it back.
    public var newer: Bool
    public var skipped: Int

    public init(version: Int, newer: Bool = false, skipped: Int = 0) { (self.version, self.newer, self.skipped) = (version, newer, skipped) }
}

public struct UpdateChange: Sendable, Equatable, Identifiable {
    /// `ui.primary`, `terminal.ansi.3`, `metrics.radius.sm`.
    public var key: String
    public var old: String?
    public var new: String?
    /// The theme sets it itself, so updating changes nothing the person sees.
    public var kept: Bool
    public var id: String { key }

    public init(key: String, old: String?, new: String?, kept: Bool) { (self.key, self.old, self.new, self.kept) = (key, old, new, kept) }
}

public struct UpdateCheck: Sendable, Equatable {
    public enum State: Sendable, Equatable { case none, missing, current, changed }
    public var state: State
    /// The source's name as people read it, and the day it was copied ("29 Oct 2026").
    public var from: String
    public var date: String
    public var changes: [UpdateChange]

    public init(state: State = .none, from: String = "", date: String = "", changes: [UpdateChange] = []) {
        (self.state, self.from, self.date, self.changes) = (state, from, date, changes)
    }
}

public struct ThemeChoice: Sendable, Equatable, Identifiable {
    public var name: String
    /// The file of a theme of the person's, or the name of a built-in.
    public var value: String
    public var id: String { value }

    public init(name: String, value: String) { (self.name, self.value) = (name, value) }
}

public struct ThemeFamily: Sendable, Equatable, Identifiable {
    public var name: String
    public var wormhole: Bool
    public var choices: [ThemeChoice]
    public var id: String { name }

    public init(name: String, wormhole: Bool, choices: [ThemeChoice]) { (self.name, self.wormhole, self.choices) = (name, wormhole, choices) }
}

public struct ThemeList: Sendable, Equatable {
    public var own: [ThemeChoice]
    public var builtins: [ThemeFamily]

    public init(own: [ThemeChoice] = [], builtins: [ThemeFamily] = []) { (self.own, self.builtins) = (own, builtins) }
}

public protocol StudioFiles: Sendable {
    func read(_ text: String) throws -> StudioFileRead
    /// The file in canonical form (version 1 text in the order and spelling core reads); an older file as it is.
    func canonical(_ text: String) throws -> String
    /// A new theme from nothing: the neutral palette of an appearance, no base, called `name`.
    func newTheme(dark: Bool, name: String) -> String
    /// A new theme called `name` with `from` copied into its base (a built-in's name, or a theme's file); `nil`
    /// when it cannot be copied from.
    func copyTheme(from: String, name: String) -> String?
    /// The id a theme called `name` gets (nobody else's); the file at `itself` does not count against it.
    func newID(name: String, itself: URL?) -> String
    /// An older file as one in the new format with `id`; refused unless it looks the same.
    func convert(_ text: String, id: String) throws -> String
    func addAppearance(_ text: String, dark: Bool, id: String) throws -> String
    func updateCheck(_ text: String, shown: String?) throws -> UpdateCheck
    func updateApply(_ text: String, shown: String?) throws -> String
    /// The theme as Export, Copy JSON and a project's `theme.json` write it: the format every release reads.
    func export(_ text: String, shown: String?) throws -> String
    func themeList() -> ThemeList
}

/// What the Studio asks of the project of the terminal in front: a trusted project's theme is the one in effect
/// while one of its terminals is in front, and "Use in this project" writes the draft as its `theme.json`.
@MainActor
public protocol StudioProject: Sendable {
    /// The theme in effect there: the project's folder name and its `theme.json` text; `nil` when none is.
    func overlay() -> (root: String, text: String)?
    /// Writes `json` as the project's theme: the file, or the reason it may not (the person's words).
    func write(_ json: String) -> Result<URL, StudioFailure>
}

public struct StudioFailure: Error, Equatable, LocalizedError, Sendable {
    public var reason: String
    public init(_ reason: String) { self.reason = reason }
    /// No terminal is in front, or the one in front is in no project with a `.wormhole` folder.
    public static let noTerminal = StudioFailure(StudioText.noTerminal)
    public static let noProjectFolder = StudioFailure(StudioText.noProjectFolder)
    public var errorDescription: String? { reason }
}

// What the palette asks the window for, and what it tells it: the actions the window offers now, the settings a query
// finds, the commits of the selected terminal's branch, and what running a result does. The window fills it (the core's
// keymap, settings index and git are behind it); the palette keeps no copy.

import Foundation
import WorkspaceModel

public struct PaletteSources: Sendable {
    public var actions: @MainActor @Sendable () -> ActionList
    /// The settings a query finds, in the index's order.
    public var settings: @MainActor @Sendable (String) -> [PaletteSetting]
    /// One setting by its id, for a recent one.
    public var setting: @MainActor @Sendable (String) -> PaletteSetting?
    public var commits: @MainActor @Sendable () -> [PaletteCommit]
    /// The profiles, for the rows that switch to one, open a window in it or move the window to it.
    public var profiles: @MainActor @Sendable () -> [PaletteProfile]
    /// Runs a keymap action by id: the router, as the menu and the key do.
    public var perform: @MainActor @Sendable (String) -> Void
    /// Opens a setting's row, or a commit in History.
    public var reveal: @MainActor @Sendable (Command.Target) -> Void
    /// The actions whose feature is off, which the sheet leaves out.
    public var hiddenActions: @MainActor @Sendable () -> [String]
    /// The global show/hide key in Apple's notation, when one is set (the sheet's footer says it).
    public var hotkey: @MainActor @Sendable () -> String?
    /// ⌘C on a setting: its config key goes on the clipboard.
    public var copySettingKey: @MainActor @Sendable (String) -> Void

    public init(
        actions: @escaping @MainActor @Sendable () -> ActionList = { ActionList() },
        settings: @escaping @MainActor @Sendable (String) -> [PaletteSetting] = { _ in [] },
        setting: @escaping @MainActor @Sendable (String) -> PaletteSetting? = { _ in nil },
        commits: @escaping @MainActor @Sendable () -> [PaletteCommit] = { [] },
        profiles: @escaping @MainActor @Sendable () -> [PaletteProfile] = { [] },
        perform: @escaping @MainActor @Sendable (String) -> Void = { _ in },
        reveal: @escaping @MainActor @Sendable (Command.Target) -> Void = { _ in },
        hiddenActions: @escaping @MainActor @Sendable () -> [String] = { [] },
        hotkey: @escaping @MainActor @Sendable () -> String? = { nil },
        copySettingKey: @escaping @MainActor @Sendable (String) -> Void = { _ in }
    ) {
        (self.hiddenActions, self.hotkey, self.profiles) = (hiddenActions, hotkey, profiles)
        (self.actions, self.settings, self.setting, self.commits) = (actions, settings, setting, commits)
        (self.perform, self.reveal, self.copySettingKey) = (perform, reveal, copySettingKey)
    }
}

extension CommandCatalog {
    /// Everything the palette lists for this window, scope and query: settings are asked for only by a query (or, empty,
    /// the ones chosen lately: `recent`), commits where they are listed.
    @MainActor
    public static func commands(
        for state: WorkspaceState, sources: PaletteSources, actions: ActionList, scope: PaletteScope, query: String,
        recent: [String]
    ) -> [Command] {
        let typed = !query.trimmingCharacters(in: .whitespaces).isEmpty
        let searching = scope == .all || scope == .settings
        let settings =
            typed
            ? (searching ? sources.settings(query) : [])
            : recent.compactMap { id in id.hasPrefix("setting-") ? sources.setting(String(id.dropFirst("setting-".count))) : nil }
        let commits = scope == .all || scope == .commits ? sources.commits() : []
        return commands(for: state, actions: actions, settings: settings, commits: commits, profiles: sources.profiles())
    }
}

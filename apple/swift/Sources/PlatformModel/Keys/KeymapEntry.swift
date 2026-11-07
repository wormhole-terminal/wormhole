// One row of the keymap: what menus call an action, its keys and where they work (`keymap::Entry`).

import Foundation

/// Where a key works. App keys work everywhere in a window; the terminal answers the editing keys too.
public enum KeyScope: String, Sendable, Codable {
    case app, edit, terminal, list, studio

    /// Whether one key press can be meant for both scopes (`Scope::overlaps`).
    public func overlaps(_ other: KeyScope) -> Bool {
        if self == other || self == .app || other == .app { return true }
        let editable: Set<KeyScope> = [.terminal, .list, .studio]
        return (self == .edit && editable.contains(other)) || (other == .edit && editable.contains(self))
    }
}

/// Whether an action exists yet: `later` rows only hold their keys; `system` is macOS's (⌘? Help search);
/// `reserved` is kept for what the key means in Terminal.app.
public enum KeyStage: String, Sendable, Codable {
    case now, later, system, reserved
}

public struct KeymapEntry: Sendable, Codable, Equatable, Identifiable {
    public var id: ActionID
    /// The menu title; for a toggle, the title while its thing is on ("Hide Sidebar").
    public var title: String
    /// A toggle's title while its thing is off ("Show Sidebar").
    public var titleOff: String?
    /// What the palette and a conflict message call it, when not the title.
    public var name: String?
    /// The first key is the one menus show; the others work too.
    public var keys: [KeySpec]
    public var scope: KeyScope
    public var stage: KeyStage
    public var palette: Bool
    public var debugOnly: Bool
    public var persistentOnly: Bool
    /// The feature (`config::FEATURES` key) and flag that hide the action while off.
    public var feature: String?
    public var flag: String?

    public var displayName: String { name ?? title }
    public var key: KeySpec? { keys.first }
    public var isLive: Bool { stage == .now }

    /// The menu title of a toggle that is `on`, or of any other action.
    public func title(on: Bool) -> String {
        on ? title : (titleOff ?? title)
    }

    func uses(_ spec: KeySpec) -> Bool { keys.contains { $0.sameKey(spec) } }
}

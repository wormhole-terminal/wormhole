// Who has a key already (`keymap::uses`): the app, macOS or the shell, for a conflict message or the shortcut
// recorder's warning.

import Foundation
import WordsModel

public enum KeyUse: Equatable, Sendable, CustomStringConvertible {
    case action(id: ActionID, name: String, key: KeySpec)
    /// macOS keeps it (Spotlight, screenshots, Spaces).
    case system(String)
    /// The shell and the programs in it need it: typing, control characters, Meta.
    case shell(String)

    public var description: String {
        switch self {
        case .action(_, let name, let key):
            Words.text("platform-key-is", ["key": .text(key.glyphText), "name": .text(name)], fallback: "\(key.glyphText) is \(name)")
        case .system(let what): Words.text("platform-key-system", ["what": .text(what)], fallback: "macOS uses it for \(what)")
        case .shell(let what): Words.text("platform-key-shell", ["what": .text(what)], fallback: "the shell needs it (\(what))")
        }
    }
}

extension Keymap {
    /// Who already has `spec`, if anyone. F-keys are free but F6 / ⇧F6; ⌘ keys never reach the shell.
    public func uses(_ spec: KeySpec) -> KeyUse? {
        if let entry = entries.first(where: { $0.uses(spec) }), let own = entry.keys.first(where: { $0.sameKey(spec) }) {
            return .action(id: entry.id, name: entry.displayName, key: own)
        }
        if let system = systemKeys.first(where: { $0.spec.sameKey(spec) }) { return .system(system.what) }
        if spec.mods.contains(.command) || spec.isFunctionKey { return nil }
        if spec.mods.contains(.control) { return .shell(Words.text("platform-key-shell-control", fallback: "a control character")) }
        return .shell(
            spec.mods.contains(.option)
                ? Words.text("platform-key-shell-meta", fallback: "Meta, word jumps and accents")
                : Words.text("platform-key-shell-typing", fallback: "typing"))
    }
}

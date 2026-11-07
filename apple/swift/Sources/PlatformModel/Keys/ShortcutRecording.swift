// Recording a shortcut (the global hotkey, the jump to the waiting terminal): what a key press does to the
// field, and whether the keys are taken (`keys/shortcut_record.rs`).

import Foundation

public enum Recorded: Equatable, Sendable {
    /// Esc alone: stop recording, keep the shortcut there was.
    case cancel
    /// Delete or Backspace alone: no shortcut.
    case clear
    /// A key without a modifier (other than an F-key) would swallow normal typing: keep waiting.
    case needsModifier
    case shortcut(KeySpec)
}

public enum ShortcutRecording {
    /// The key `key` (lowercase) pressed with `mods`. Modifier keys alone never get here.
    public static func record(_ mods: KeyMods, key: String) -> Recorded {
        let bare = mods.isEmpty
        switch key {
        case "escape" where bare: return .cancel
        case "backspace" where bare, "delete" where bare: return .clear
        default:
            let spec = KeySpec(mods, key)
            return bare && !spec.isFunctionKey ? .needsModifier : .shortcut(spec)
        }
    }

    /// What already has `spec`, as the field says it, or nothing. `own` is the action the field sets the key of
    /// (the jump key is Go to Waiting Terminal's), which doesn't count.
    public static func conflict(_ spec: KeySpec, own: ActionID? = nil, in keymap: Keymap) -> String? {
        switch keymap.uses(spec) {
        case nil: nil
        case .action(let id, _, _)? where id == own: nil
        case let used?: describe(used, spec)
        }
    }

    private static func describe(_ used: KeyUse, _ spec: KeySpec) -> String {
        switch used {
        case .action: used.description
        case .system(let what): "macOS uses \(spec.glyphText) for \(what)"
        case .shell: "The shell uses \(spec.glyphText)"
        }
    }
}

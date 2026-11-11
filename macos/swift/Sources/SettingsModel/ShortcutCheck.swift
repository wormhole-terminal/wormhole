// What the recorder row asks of the shortcut a key press recorded: keep waiting, take it, or refuse it and say
// why (the app, macOS or the shell already has the keys). The rules are `wormhole-core`'s (`shortcut_record`);
// a model without a checker leaves the row to its own plain rule.

public enum ShortcutCheck: Sendable, Equatable {
    /// Take it; the text is what `config.json` stores.
    case accepted(String)
    /// Esc alone: stop recording and keep the shortcut there was.
    case cancel
    /// Delete alone: no shortcut.
    case clear
    /// The key alone would swallow typing: keep waiting.
    case needsModifier
    /// Not taken, with the field's words (`The shell uses ⌥Space`).
    case refused(String)
}

public protocol ShortcutChecking: Sendable {
    /// `key` is the setting being recorded (`hotkey`, `jump_waiting_key`), `spec` `config.json`'s spelling.
    func check(key: String, spec: String) -> ShortcutCheck
}

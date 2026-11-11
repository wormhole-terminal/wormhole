// VoiceOver reads glyphs badly ("⌘" as "place of interest"), so a key's glyphs are said in words.

import Foundation
import WordsModel

public enum KeyCapSpeech {
    private static func word(_ glyph: Character) -> String? {
        switch glyph {
        case "⌃": Words.text("keycap-control", fallback: "Control")
        case "⌥": Words.text("keycap-option", fallback: "Option")
        case "⇧": Words.text("keycap-shift", fallback: "Shift")
        case "⌘": Words.text("keycap-command", fallback: "Command")
        case "↑": Words.text("keycap-up-arrow", fallback: "Up arrow")
        case "↓": Words.text("keycap-down-arrow", fallback: "Down arrow")
        case "←": Words.text("keycap-left-arrow", fallback: "Left arrow")
        case "→": Words.text("keycap-right-arrow", fallback: "Right arrow")
        case "⌫": Words.text("keycap-delete", fallback: "Delete")
        case "↩": Words.text("keycap-return", fallback: "Return")
        case "⎋": Words.text("keycap-escape", fallback: "Escape")
        case "⇥": Words.text("keycap-tab", fallback: "Tab")
        default: nil
        }
    }

    /// The glyphs of a chord as VoiceOver should say them ("Command O"); `esc` is Escape.
    public static func words(_ glyphs: String) -> String {
        if glyphs == "esc" { return Words.text("keycap-escape", fallback: "Escape") }
        return glyphs.map { word($0) ?? String($0) }.joined(separator: " ")
    }
}

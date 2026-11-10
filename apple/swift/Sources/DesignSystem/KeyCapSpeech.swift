// VoiceOver reads glyphs badly ("⌘" as "place of interest"), so a key's glyphs are said in words.

import Foundation

public enum KeyCapSpeech {
    private static let words: [Character: String] = [
        "⌃": "Control", "⌥": "Option", "⇧": "Shift", "⌘": "Command", "↑": "Up arrow", "↓": "Down arrow",
        "←": "Left arrow", "→": "Right arrow", "⌫": "Delete", "↩": "Return", "⎋": "Escape", "⇥": "Tab",
    ]

    /// The glyphs of a chord as VoiceOver should say them ("Command O"); `esc` is Escape.
    public static func words(_ glyphs: String) -> String {
        if glyphs == "esc" { return "Escape" }
        return glyphs.map { words[$0] ?? String($0) }.joined(separator: " ")
    }
}

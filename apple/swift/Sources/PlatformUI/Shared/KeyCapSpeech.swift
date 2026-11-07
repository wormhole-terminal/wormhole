// VoiceOver reads glyphs badly ("⌘" as "place of interest"), so a key's glyphs are said in words.

import Foundation

enum KeyCapSpeech {
    private static let words: [Character: String] = [
        "⌃": "Control", "⌥": "Option", "⇧": "Shift", "⌘": "Command", "↑": "Up arrow", "↓": "Down arrow",
        "←": "Left arrow", "→": "Right arrow", "⌫": "Delete", "↩": "Return", "⎋": "Escape", "⇥": "Tab",
    ]

    static func words(_ glyphs: String) -> String {
        glyphs.map { words[$0] ?? String($0) }.joined(separator: " ")
    }
}

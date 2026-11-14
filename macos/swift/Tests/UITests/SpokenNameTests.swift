import DesignSystem
import Testing

/// A button's VoiceOver name (`buttons.rs` `spoken_name` and its test): the key a tooltip ends in is left out.
struct SpokenNameTests {
    private func said(_ tooltip: String) -> String {
        let (words, key) = SpokenName.split(tooltip)
        return key.map { "\(words) | \($0)" } ?? words
    }

    @Test func aButtonsSpokenNameLeavesItsKeyOut() {
        #expect(said("Settings (⌘,)") == "Settings | ⌘,")
        #expect(said("Hide sidebar (⌃⌘S)") == "Hide sidebar | ⌃⌘S")
        #expect(said("Next match (F3)") == "Next match | F3")
        let waiting = "Show right panel (⌥⌘I) · hides the sidebar to make room"
        #expect(said(waiting) == waiting)
        #expect(said("Refresh") == "Refresh")
        #expect(said("Compare (main)") == "Compare (main)")
        #expect(said("Close find (esc)") == "Close find (esc)")
    }
}

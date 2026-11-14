// What VoiceOver calls a control whose tooltip ends in its key ("Split right (⌘D)"): the words alone, since key glyphs are
// never read as text (`buttons.rs` `spoken_name`, the GPUI app's own rule, not core's). The key is not lost: it is offered
// as the control's hint, in words ("Command D"), which VoiceOver says after the name.

import SwiftUI

public enum SpokenName {
    /// The tooltip's words and the key it ends in, or the whole tooltip and no key. A parenthesis counts as a key when it
    /// holds a modifier glyph or starts with F (a function key), as GPUI's rule: "Compare (main)" keeps its words.
    public static func split(_ tooltip: String) -> (words: String, key: String?) {
        guard tooltip.hasSuffix(")"), let open = tooltip.range(of: " (", options: .backwards) else { return (tooltip, nil) }
        let inside = String(tooltip[open.upperBound..<tooltip.index(before: tooltip.endIndex)])
        let isKey = inside.contains { "⌘⌥⇧⌃".contains($0) } || inside.hasPrefix("F")
        return isKey ? (String(tooltip[..<open.lowerBound]), inside) : (tooltip, nil)
    }
}

extension View {
    /// The tooltip as written, VoiceOver's name without its key and the key as the hint in words. `spoken` replaces the
    /// name where the tooltip alone would make a list of identical ones (`IconButton::spoken`).
    public func tooltipName(_ tooltip: String, spoken: String? = nil) -> some View {
        let (words, key) = SpokenName.split(tooltip)
        return help(tooltip)
            .accessibilityLabel(spoken ?? words)
            .accessibilityHint(key.map(KeyCapSpeech.words) ?? "")
    }
}

// The title row's measurer: the width of a text in the interface font in the faces the row draws in (the
// label role at 13 pt, the caption at 12), rounded up like GPUI's `layout_line(...).width.ceil()`.

import AppKit
import DesignSystem
import PanesModel

struct TextMeasurer: TextMeasuring {
    func width(_ text: String, _ face: TitleFace) -> Double {
        guard !text.isEmpty else { return 0 }
        let (role, weight): (TextRole, NSFont.Weight) =
            switch face {
            case .label: (.label, .regular)
            case .labelMedium: (.label, .medium)
            case .caption: (.caption, .regular)
            case .captionMedium: (.caption, .medium)
            }
        // Only ever called while a view body lays out, on the main actor; `TextRole`'s size follows the theme's font.
        let size = MainActor.assumeIsolated { role.size }
        let font = NSFont.systemFont(ofSize: size, weight: weight)
        return (text as NSString).size(withAttributes: [.font: font]).width.rounded(.up)
    }
}

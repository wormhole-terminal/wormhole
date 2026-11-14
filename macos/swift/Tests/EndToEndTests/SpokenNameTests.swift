import Foundation
import Testing

extension EndToEnd {
    /// Read through `a11y`: the window's NSAccessibility tree, one element a line as `role "label" (help)`. A button whose
    /// tooltip ends in its key ("Next match (⌘G)") is named without it, as the GPUI app's `spoken_name` does; the key
    /// stays in the tooltip.
    // Disabled, not counted as passing: asked in the process, the tree has AppKit's views and none of SwiftUI's, so nothing
    // here could fail. The assertions stay for the day the tree can be read.
    @Test(
        "VoiceOver names the find bar's and the title row's buttons without the keys their tooltips end in")
    @MainActor func spokenNames() async throws {
        try await Instance.with { app in
            await app.send(["find milk"])
            // The bar is up once the window's focus line says so (`keys`: `+find`).
            await app.until("keys") { $0.contains("+find") }
            let tree = await app.reply("a11y")
            // The buttons are SwiftUI's: `a11y` reads their nodes too (by their accessibility methods, once the application was
            // told an assistive client reads it). The rule is `SpokenNameTests` in UITests.
            for name in ["\"Next match\"", "\"Previous match\"", "\"Close find (esc)\""] { app.expect(tree, contains: name) }
            for keyed in ["\"Previous match (⇧⌘G)\"", "\"Next match (⌘G)\""] {
                app.expect(!tree.contains(keyed), "no button is named with its key, \(keyed):\n\(tree)")
            }
            // The split buttons show where the title row has room for them; where they do, their names have no key either.
            if tree.contains("\"Split right") {
                app.expect(tree, contains: "\"Split right\"")
                app.expect(!tree.contains("\"Split right (⌘D)\""), "the split button is named without its key:\n\(tree)")
            }
        }
    }
}

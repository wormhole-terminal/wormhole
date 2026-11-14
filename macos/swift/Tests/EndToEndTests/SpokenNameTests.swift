import Foundation
import Testing

extension EndToEnd {
    /// Read through `a11y`: the window's NSAccessibility tree, one element a line as `role "label" (help)`. A button whose
    /// tooltip ends in its key ("Next match (⌘G)") is named without it, as the GPUI app's `spoken_name` does; the key
    /// stays in the tooltip.
    @Test("VoiceOver names the find bar's and the title row's buttons without the keys their tooltips end in")
    @MainActor func spokenNames() async throws {
        try await Instance.with { app in
            await app.send(["find milk"])
            // The find field is AppKit's, so the tree has it once the bar is up.
            let tree = await app.until("a11y") { $0.contains("TextField = milk") }
            // The buttons are SwiftUI's, and SwiftUI builds its accessibility nodes only for an assistive app: asked in the
            // process, the tree has none of them (DRIVING.md, "What still needs a person"). Kept as the check it is, and known
            // not to hold from here: it reports itself the day the tree does list them. The rule is `SpokenNameTests` in UITests.
            withKnownIssue("a11y lists no SwiftUI control in a test instance: the buttons' names need VoiceOver or Accessibility Inspector") {
                #expect(tree.contains("\"Next match\""))
                #expect(tree.contains("\"Previous match\""))
                #expect(tree.contains("\"Close find (esc)\""))
            }
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

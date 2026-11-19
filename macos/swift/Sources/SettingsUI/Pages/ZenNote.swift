// The note a page carries while Zen mode is on (`settings_window/zen_note.rs`). Zen mode draws the window with less than
// the settings say and saves nothing, so the rows keep showing the person's own values; the note says why the window
// does not match them, and leaves Zen mode from the page. Only a page with a row Zen mode overrides carries it.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ZenNote: View {
    let leave: () -> Void

    /// The rows (by their id in the settings table) Zen mode overrides (`OVERRIDDEN`).
    private static let overridden: Set<String> = ["title-row", "status-bar", "minimap", "column-dividers"]

    /// Whether a row of `section` is one Zen mode overrides.
    static func affects(_ section: SettingsSchema.Section) -> Bool {
        section.groups.contains { group in group.entries.contains { overridden.contains($0.id) } }
    }

    /// What the note says (the control folder's `settings-page` reads it through `PageBanner`).
    static var text: String {
        Words.text(
            "zen-settings-note",
            fallback: "Zen mode is on, so the window shows less than these settings say. They keep your own values and apply again when you leave it.")
    }

    var body: some View {
        Banner(kind: PageBanner.zen.kind, text: Self.text) {
            ActionButton(title: Words.text("zen-settings-leave", fallback: "Leave Zen mode"), symbol: nil, frameId: "zen.leave", action: leave)
        }
    }
}

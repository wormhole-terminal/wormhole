// Opens the Licenses window. The list comes from a `LicenseSource` (the bundled file, later the core through the
// FFI); one that cannot be read shows a plain note instead of an empty window.

import AuxModel
import DesignSystem
import SwiftUI

@MainActor
public enum Licenses {
    /// The open window's browser, for the control folder's `licenses-search` and `licenses-select` (nil while it is closed).
    public private(set) static weak var browser: LicenseBrowser?

    public static func show(source: any LicenseSource) {
        AuxWindowController.show(
            key: "licenses", title: AuxText.licensesTitle, size: LicensesMetrics.size, minSize: LicensesMetrics.minSize
        ) {
            if let list = try? source.load() {
                page(list: list)
            } else {
                LicensesUnreadable()
            }
        }
    }
}

extension Licenses {
    private static func page(list: LicenseList) -> LicensesView {
        let browser = LicenseBrowser(list: list)
        Self.browser = browser
        return LicensesView(browser: browser)
    }
}

/// Shown for a list that could not be read; a test rules the bundled one out.
private struct LicensesUnreadable: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(AuxText.licensesUnreadable).font(TextRole.body.font).foregroundStyle(tokens.textMuted).padding(Space._6).auxPage()
    }
}

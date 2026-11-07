// A read-only value in a row: the app's version, the custom fonts found (from the host when it has them).

import DesignSystem
import SwiftUI

struct InfoValue: View {
    let name: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(name == "version" ? SettingsText.appVersion : "")
            .font(.system(size: TextRole.body.size)).monospacedDigit().foregroundStyle(tokens.textMuted)
    }
}

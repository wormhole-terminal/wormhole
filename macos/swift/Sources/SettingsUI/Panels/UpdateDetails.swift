// Privacy: what an update check sends (`updates.rs` `draw_check_details`), with the one header that says anything,
// built by core (`update::user_agent`).

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct UpdateDetails: View {
    let model: SettingsModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            part(
                Words.text("settings-updates-check-sends", fallback: "Every update check"),
                Words.text("settings-updates-check-sends-what", fallback: "The app’s name and version, and which update feed it asks for. Nothing else about this Mac."),
                example: model.host.updateUserAgent(version: SettingsText.appVersion))
            part(
                Words.text("settings-updates-check-server", fallback: "The download server"),
                Words.text("settings-updates-check-server-what", fallback: "Counts the requests it gets by app version, so we know which versions are in use. It stores no IP address and nothing that tells one Mac from another. GitHub, which serves the files, sees your IP address as with any download."),
                example: nil)
            part(
                Words.text("settings-updates-check-never", fallback: "Never sent"),
                Words.text("settings-updates-check-never-what", fallback: "Device IDs, account details, files and terminal contents."),
                example: nil)
        }
        .accessibilityElement(children: .contain)
    }

    private func part(_ title: String, _ detail: String, example: String?) -> some View {
        VStack(alignment: .leading, spacing: Space._1) {
            Text(title).textRole(.label, weight: .medium).foregroundStyle(tokens.textPrimary)
            Text(detail).textRole(.body).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
            if let example {
                Text(example).textRole(.caption, design: .monospaced).foregroundStyle(tokens.textMuted).textSelection(.enabled)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, Space._2)
    }
}

// Privacy: what an update check sends (`updates.rs` `draw_count_details`), with the header this Mac would send for the
// current sharing setting, built by core (`counted_user_agent`).

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct UpdateDetails: View {
    let model: SettingsModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        let counting = model.value("anonymous_count").bool ?? true
        let unstable = model.value("update_channel").string == "unstable"
        let agent = model.host.updateUserAgent(version: SettingsText.appVersion, counting: counting, unstable: unstable)
        VStack(alignment: .leading, spacing: 0) {
            part(
                Words.text("settings-updates-count-every-check", fallback: "Every update check"),
                counting
                    ? Words.text("settings-updates-count-every-check-counted", fallback: "The app version, requested update feed, update channel, chip type, and macOS major version.")
                    : Words.text("settings-updates-count-every-check-plain", fallback: "The app version and requested update feed."),
                example: agent)
            if counting {
                part(
                    Words.text("settings-updates-count-first-check", fallback: "First check each day or month"),
                    Words.text("settings-updates-count-first-check-why", fallback: "The server uses a day or month flag to estimate active installations. Both flags can be sent together. Day and month boundaries use UTC."),
                    example: "Wormhole-Count: day, month")  // text: protocol, sent as is
            }
            part(
                Words.text("settings-updates-count-never", fallback: "IP address and private data"),
                Words.text("settings-updates-count-never-what", fallback: "The download server sees your IP address. The update counter does not store it. Update requests exclude device IDs, account details, files, and terminal contents."),
                example: nil)
        }
        .accessibilityElement(children: .contain)
    }

    private func part(_ title: String, _ detail: String, example: String?) -> some View {
        VStack(alignment: .leading, spacing: Space._1) {
            Text(title).font(.system(size: TextRole.label.size, weight: .medium)).foregroundStyle(tokens.textPrimary)
            Text(detail).font(.system(size: TextRole.body.size)).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
            if let example {
                Text(example).font(.system(size: TextRole.caption.size, design: .monospaced)).foregroundStyle(tokens.textMuted).textSelection(.enabled)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, Space._2)
    }
}

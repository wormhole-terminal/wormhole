// The status bar's item that says where the selected terminal runs (`window/status_hosting.rs`): "Persistent" in a session
// host that outlives the app, "In app" when it ends with the app. Shown only while persistent terminals are on; a terminal
// whose host is gone says nothing (nothing runs there to be persistent or not).

import DesignSystem
import SwiftUI
import WordsModel
import WorkspaceModel

enum StatusHosting {
    /// The word and the sentence for `state`, or nil when the bar has no item for it.
    static func words(_ state: HostingState) -> (word: String, sentence: String)? {
        switch state {
        case .persistent, .detached:
            (
                Words.text("status-bar-hosting-persistent", fallback: "Persistent"),
                Words.text("status-bar-hosting-persistent.tooltip", fallback: "This terminal keeps running when Wormhole quits or updates.")
            )
        case .local:
            (
                Words.text("status-bar-hosting-in-app", fallback: "In app"),
                Words.text("status-bar-hosting-in-app.tooltip", fallback: "This terminal runs inside Wormhole and ends when Wormhole quits or updates.")
            )
        case .connecting:
            (
                Words.text("status-bar-hosting-connecting", fallback: "Connecting…"),
                Words.text("status-bar-hosting-connecting.tooltip", fallback: "Wormhole is still connecting to this terminal’s session host.")
            )
        case .lost, .unavailable: nil
        }
    }

    /// Quiet when persistent, in the warning colour when not: the one a person would want to notice.
    static func isWarning(_ state: HostingState) -> Bool { !state.survivesQuit }

    /// Whether the item says its word beside the icon at this width: below the counts' numbers it is the icon alone.
    static func showsWord(_ detail: StatusSummary.Detail) -> Bool { detail >= .numbers }

    static func symbol(_ state: HostingState) -> String {
        if case .connecting = state { return "arrow.triangle.2.circlepath" }
        return state.survivesQuit ? "externaldrive" : "exclamationmark.triangle"
    }
}

struct StatusHostingItem: View {
    let state: HostingState
    let word: Bool
    @Environment(\.chrome) private var colours

    var body: some View {
        if let words = StatusHosting.words(state) {
            HStack(spacing: Space._1) {
                Image(systemName: StatusHosting.symbol(state))
                if word { Text(words.word) }
            }
            .foregroundStyle(StatusHosting.isWarning(state) ? colours.attentionText : colours.muted)
            .help(words.sentence)
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(words.sentence)
        }
    }
}

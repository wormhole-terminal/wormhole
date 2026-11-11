// "View all N…" (`trust_window.rs`): every line the project would run, in full, 640 by 480, with the same approval there, the
// state of the trust, how many lines, "Nothing here would run." for none and the note that the project is no longer open.
// It follows the store's copy of the project, so what changed on disk shows. Esc closes it. (A sheet of the window here; the GPUI
// app opens a window of its own.)

import DesignSystem
import PanelModel
import SwiftUI

struct TrustLinesSheet: View {
    let store: ProjectStore
    let close: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: PanelMetrics.gap) {
            if let state = store.state {
                Text(state.notice.title).font(TextRole.heading.font).foregroundStyle(tokens.textPrimary)
                Text("\(state.name)/.wormhole \u{00B7} \(statusWords(state))").font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                if state.commands.isEmpty {
                    Text(panelText("project-all-none", "Nothing here would run.")).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
                } else {
                    Text(state.commands.count == 1 ? "1 line" : "\(CountText.grouped(state.commands.count)) lines")
                        .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    ScrollView { TrustLines(lines: state.commands) }
                }
            } else {
                Text(panelText("project-all-gone", "This project is no longer open in a terminal.")).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            }
            Spacer(minLength: 0)
            HStack(spacing: PanelMetrics.gap) {
                Spacer()
                if let state = store.state, state.status != .trusted {
                    NoticeButton(
                        title: store.approving ? panelText("project-waiting", "Waiting for approval\u{2026}") : panelText("project-trust", "Trust with Touch ID"),
                        primary: true, busy: store.approving, action: store.approve)
                }
                NoticeButton(title: panelText("project-close", "Close"), action: close).keyboardShortcut(.cancelAction)
            }
        }
        .padding(Space._4).frame(width: 640, height: 480).background(tokens.surfaceOverlay)
    }

    private func statusWords(_ state: ProjectState) -> String {
        switch state.status {
        case .trusted: "trusted"
        case .changed: "changed since you trusted it"
        default: "not trusted"
        }
    }
}

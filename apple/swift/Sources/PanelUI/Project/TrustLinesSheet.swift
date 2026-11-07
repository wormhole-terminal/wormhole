// "View all N…" (`trust_window.rs`): every line the project would run, in a window of its own, with the same approval
// there. Esc closes it.

import DesignSystem
import PanelModel
import SwiftUI

struct TrustLinesSheet: View {
    let state: ProjectState
    let close: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: PanelMetrics.gap) {
            Text(state.notice.title).font(TextRole.heading.font).foregroundStyle(tokens.textPrimary)
            Text(panelText("project-all-lines", "\(state.name)/.wormhole would run these \(state.commands.count) things once you trust it.")).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            ScrollView { TrustLines(lines: state.commands) }
            HStack {
                Spacer()
                NoticeButton(title: panelText("project-close", "Close"), action: close).keyboardShortcut(.cancelAction)
            }
        }
        .padding(Space._4).frame(width: 520, height: 420).background(tokens.surfaceOverlay)
    }
}

// The path's hover card: a popover under the folder with the whole path and three buttons: Open in Finder,
// Copy path (which says "Copied" for a moment) and Insert into terminal, which types the path at the focused
// terminal's prompt. Esc closes it (the popover does).

import DesignSystem
import SwiftUI

struct PathCard: View {
    let path: String
    let actions: TitleActions
    let hover: PathCardHover
    @State private var copied = false
    @Environment(\.tokens) private var tokens

    /// How long "Copied" stays (`PATH_CARD_COPIED_FOR`).
    private static let copiedFor: Duration = .milliseconds(1200)

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            Text(path).font(TextRole.body.font).foregroundStyle(tokens.textPrimary).textSelection(.enabled)
                .lineLimit(3).fixedSize(horizontal: false, vertical: true)
            HStack(spacing: Space._1) {
                cardButton("arrow.up.forward.app", PanesText.openInFinder) { hover.close(); actions.revealFolder() }
                cardButton(copied ? "checkmark" : "doc.on.doc", copied ? PanesText.copied : PanesText.copyPath) { copy() }
                cardButton("text.insert", PanesText.insertIntoTerminal, enabled: actions.canInsert) { hover.close(); actions.insertPath() }
            }
        }
        .padding(Space._3)
        .frame(minWidth: 240, maxWidth: 360, alignment: .leading)
        .onHover { hover.hover(.card, $0) }
    }

    private func cardButton(_ symbol: String, _ label: String, enabled: Bool = true, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Label(label, systemImage: symbol).font(TextRole.caption.font).labelStyle(.titleAndIcon)
        }
        .buttonStyle(.bordered).controlSize(.small).disabled(!enabled)
    }

    private func copy() {
        actions.copyPath()
        copied = true
        Task {
            try? await Task.sleep(for: Self.copiedFor)
            copied = false
        }
    }
}

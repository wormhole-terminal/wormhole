// A row's second line (`row/second_line.rs`): what the terminal has to report, else where it is, in the tone's colour
// (amber asks, green is done, red failed), a command in the code font, and at its end a faint clock or "new output".
// The words are the core's, read again by the store's one timer when they move with time.
// Under LineRoll a changed line rolls up to its new words instead of cross-fading.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SecondLineView: View {
    let line: SecondLine
    let isSelected: Bool
    let rolls: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            Text(line.text)
                .font(line.isCommand ? .system(size: TextRole.caption.size, design: .monospaced)
                    : TextRole.caption.font.weight(line.tone == .plain ? .medium : .regular))
                .foregroundStyle(color)
                .lineLimit(1)
                .id(rolls ? line.text : "")
                .transition(.push(from: .bottom))
            Spacer(minLength: 0)
            if let trail = line.trail {
                Text(trail).font(TextRole.caption.font.monospacedDigit()).foregroundStyle(tokens.textFaint)
                    .lineLimit(1).fixedSize()
            }
        }
        .frame(height: RowGeometry.metaLine)
        .clipped()
        .animation(rolls ? SidebarMotion.fold : nil, value: line.text)
    }

    private var color: Color {
        switch line.tone {
        case .plain: isSelected ? tokens.sidebarRowSelectedMuted : tokens.textMuted
        case .attention: tokens.agentAttentionText
        case .done: tokens.agentDoneText
        case .failed: tokens.statusDangerText
        }
    }
}

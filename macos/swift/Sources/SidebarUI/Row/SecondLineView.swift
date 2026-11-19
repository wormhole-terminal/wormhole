// A row's second line (`row/second_line.rs`): what the terminal has to report, else where it is, in the tone's colour
// (amber asks, green is done, red failed), a command in the code font, and at its end a faint clock or "new output".
// The words are the core's, read again by the store's one timer when they move with time.
// Under LineRoll a changed line rolls up to its new words instead of cross-fading; a change of tone rolls too, the same
// words in their new colour (`face.rs`: the words or the tone differ).

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SecondLineView: View {
    let line: SecondLine
    let isSelected: Bool
    let rolls: Bool
    /// Set when the line does not fit its room (the row's tooltip says it whole).
    @Binding var cut: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.reduceMotion) private var reduceMotion

    var body: some View {
        HStack(spacing: Space._2) {
            Text(line.text)
                .font(font)
                .foregroundStyle(color)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
                .cutProbe(line.text, font: font, cut: $cut)
                .id(rolling ? rollKey : "")
                .transition(.push(from: .bottom))
            if let trail = line.trail {
                Text(trail).font(tokens.font(.caption).monospacedDigit()).foregroundStyle(tokens.textFaint)
                    .lineLimit(1).fixedSize()
            }
        }
        .frame(height: RowGeometry.metaLine)
        .clipped()
        .animation(rolling ? SidebarMotion.fold : nil, value: rollKey)
    }

    /// The line rolls (the old words up out of the clip) only where motion is wanted.
    private var rolling: Bool { rolls && !reduceMotion }

    /// What a roll follows: the words and their tone.
    private var rollKey: String { "\(line.tone)\n\(line.text)" }

    private var font: Font {
        line.isCommand ? .system(size: tokens.size(.caption), design: .monospaced)
            : tokens.font(.caption).weight(line.tone == .plain ? .medium : .regular)
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

// The highlighted font as terminal text, on the theme's terminal colours at the settings' size and line height
// (`draw_specimen`), with what the font can and cannot draw under it. The text cross-fades when the font changes
// (a cut under Reduce Motion), so moving down the list reads as the same page changing its typeface.

import DesignSystem
import FontPickerModel
import SwiftUI

struct FontPreviewView: View {
    let model: FontPickerModel
    let look: PreviewLook
    @Environment(\.tokens) private var tokens

    var body: some View {
        let family = model.previewFamily
        VStack(alignment: .leading, spacing: Space._3) {
            sample(family)
            if model.slot.prefersMonospace { FontCoverageView(coverage: model.coverage(of: family)) }
            Spacer(minLength: 0)
        }
        .padding(Space._3)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .clipped()
        .accessibilityElement(children: .contain)
        .accessibilityLabel(FontWords.preview)
    }

    private func sample(_ family: String) -> some View {
        let terminal = tokens.terminal
        let lines = model.slot.prefersMonospace ? TerminalSample.lines : TerminalSample.interface
        return VStack(alignment: .leading, spacing: max(0, look.size * (look.lineHeight - 1.2))) {
            ForEach(lines.indices, id: \.self) { line in
                Text(styled(lines[line])).lineLimit(1).fixedSize()
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .font(FontFace.font(family, size: look.size, ligatures: look.ligatures, monospacedFallback: model.slot.prefersMonospace))
        .padding(Space._3)
        // A fixed width, so a line that is too long is cut at the edge as a terminal's is, not widen the column.
        .frame(width: FontPickerMetric.previewWidth - 2 * Space._3, alignment: .topLeading)
        .background(model.slot.prefersMonospace ? terminal.background.color : tokens.controlBackground, in: RoundedRectangle(cornerRadius: Radius.control))
        .clipShape(RoundedRectangle(cornerRadius: Radius.control))
        .id(family)
        .motionTransition(.opacity)
        .motion(Motion.selection, value: family)
        .accessibilityHidden(true)
    }

    /// A line's runs, each in its colour (`Text`'s `+` is deprecated: an attributed string is the way).
    private func styled(_ runs: [SampleRun]) -> AttributedString {
        runs.reduce(into: AttributedString()) { line, run in
            var piece = AttributedString(run.text)
            piece.foregroundColor = color(of: run.tone)
            line += piece
        }
    }

    private func color(of tone: SampleRun.Tone) -> Color {
        switch tone {
        case .foreground: model.slot.prefersMonospace ? tokens.terminal.foreground.color : tokens.textPrimary
        case .muted: tokens.textMuted
        case .ansi(let index): tokens.terminal.ansi[index].color
        }
    }
}

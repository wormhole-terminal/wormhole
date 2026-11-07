// The terminal's own colours: the sixteen palette slots (a click selects its token), and a few lines of text
// with a selection and the cursor on the terminal's background (`gallery/terminal.rs`).

import DesignSystem
import StudioModel
import SwiftUI

struct TerminalSpecimen: View {
    let context: GalleryContext
    @Environment(\.tokens) private var tokens

    var body: some View {
        let terminal = tokens.terminal
        VStack(alignment: .leading, spacing: Space._3) {
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: Space._1), count: 8), spacing: Space._1) {
                ForEach(0..<16, id: \.self) { slot in
                    let name = "terminal.ansi.\(slot)"
                    Button { context.select(name) } label: {
                        RoundedRectangle(cornerRadius: tokens.metrics.radiusSm).fill(terminal.ansi[slot].color).frame(height: 28)
                            .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radiusSm).strokeBorder(
                                context.selected == name ? tokens.primary : tokens.border, lineWidth: tokens.metrics.borderWidth))
                    }
                    .buttonStyle(.plain).help(name).accessibilityLabel(name)
                }
            }
            VStack(alignment: .leading, spacing: 2) {
                Text("~/Projects/wormhole  main").foregroundStyle(terminal.ansi[4].color)
                Text("$ cargo test").foregroundStyle(terminal.foreground.color)
                HStack(spacing: 0) {
                    Text("selected text").foregroundStyle(terminal.selectionForeground?.color ?? terminal.foreground.color)
                        .background(terminal.selectionBackground.color)
                    Text(" and the cursor ").foregroundStyle(terminal.foreground.color)
                    Rectangle().fill(terminal.cursor.color).frame(width: 8, height: 16)
                }
            }
            .font(TextRole.body.font.monospaced()).padding(Space._3).frame(maxWidth: .infinity, alignment: .leading)
            .background(terminal.background.color, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .accessibilityHidden(true)
        }
    }
}

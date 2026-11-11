// Find in scrollback: the real bar over matches in terminal text, the current one solid, and the bar again with a
// pattern that is not valid (`gallery/find.rs`). The bars are the terminal's own views, left to the sample searcher.

import DesignSystem
import SwiftUI
import TerminalUI

struct FindSpecimen: View {
    @State private var found = FindModel(searcher: MockSearching(lines: FindSpecimen.lines))
    @State private var invalid = FindModel(searcher: MockSearching(lines: FindSpecimen.lines))
    @Environment(\.tokens) private var tokens

    private static let lines = ["test git::split::tests::pairs ... FAILED", "panicked at src/git/split.rs:212:9"]

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            terminal
            FindBar(model: found, onClose: {})
            FindBar(model: invalid, onClose: {})
            // The bar's Lucide glyphs on their own (`LucideView`, the icons GPUI's `find_bar` takes from gpui-kit-assets).
            HStack(spacing: Space._3) {
                ForEach([Lucide.search, .caseSensitive, .wholeWord, .regex, .chevronUp, .chevronDown, .x], id: \.self) {
                    LucideView(icon: $0, size: Metric.glyph)
                }
            }
            .foregroundStyle(tokens.textMuted)
        }
        .allowsHitTesting(false)
        .task {
            found.open(seed: "split")
            invalid.open(seed: "error[")
            invalid.toggle(.regex)
        }
    }

    private var terminal: some View {
        let colours = tokens.terminal
        return VStack(alignment: .leading, spacing: Space._1) {
            line("test git::", "split", current: false, "::tests::pairs ... FAILED")
            line("panicked at src/git/", "split", current: true, ".rs:212:9")
        }
        .font(TextRole.body.font.monospaced()).foregroundStyle(colours.foreground.color)
        .padding(Space._3).frame(maxWidth: .infinity, alignment: .leading)
        .background(colours.background.color, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
        .accessibilityHidden(true)
    }

    private func line(_ before: String, _ match: String, current: Bool, _ after: String) -> some View {
        HStack(spacing: 0) {
            Text(before)
            Text(match).bold(current).foregroundStyle(current ? tokens.findMatchCurrentText : tokens.terminal.foreground.color)
                .background(current ? tokens.findMatchCurrent : tokens.findMatch)
            Text(after)
        }
    }
}

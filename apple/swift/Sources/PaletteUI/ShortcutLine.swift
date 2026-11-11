// One shortcut of the sheet: its label (what the filter found marked) and its keys at the right, each as a key cap,
// or for a run the first and the last with "…" between.

import DesignSystem
import PaletteModel
import SwiftUI
import WordsModel

struct ShortcutLine: View {
    let row: ShortcutRow
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            Text(MarkedText.title(row.label, ranges: row.hit.map { [$0] } ?? [], mark: tokens.primary, size: TextRole.label.size))
                .font(.system(size: TextRole.label.size))
                .lineLimit(1)
            Spacer(minLength: 0)
            HStack(spacing: Space._1) { caps }
        }
        .frame(height: Metric.row)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(Text(spoken))
    }

    private var spoken: String {
        let keys = row.keys.map { KeyCapSpeech.words($0.glyphs) }.joined(separator: ", ")
        return Words.text("palette-sheet-spoken", ["label": .text(row.label), "keys": .text(keys)], fallback: "\(row.label), \(keys)")
    }

    @ViewBuilder private var caps: some View {
        if row.isRun {
            if let first = row.keys.first { KeyCap(text: first.glyphs) }
            if row.keys.count > 1, let last = row.keys.last {
                Text("…").font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textFaint)
                KeyCap(text: last.glyphs)
            }
        } else {
            ForEach(Array(row.keys.enumerated()), id: \.offset) { KeyCap(text: $1.glyphs) }
        }
    }
}

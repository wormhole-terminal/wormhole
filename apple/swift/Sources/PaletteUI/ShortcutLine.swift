// One shortcut of the sheet: its label (what the filter found marked) and its keys at the right, each as a key cap,
// or for a run the first and the last with "…" between.

import DesignSystem
import PaletteModel
import SwiftUI

struct ShortcutLine: View {
    let row: ShortcutRow
    let hit: ShortcutHit
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            Text(MarkedText.title(row.label, ranges: hit.label.map { [$0] } ?? [], mark: tokens.primary, size: TextRole.label.size))
                .font(.system(size: TextRole.label.size))
                .lineLimit(1)
            Spacer(minLength: 0)
            HStack(spacing: Space._1) { caps }
        }
        .frame(height: Metric.row)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(Text("\(row.label), \(row.keys.map { KeyCap.spoken($0.glyphs) }.joined(separator: ", "))"))
    }

    @ViewBuilder private var caps: some View {
        switch row.style {
        case .each:
            ForEach(Array(row.keys.enumerated()), id: \.offset) { KeyCap(text: $1.glyphs) }
        case .range:
            if let first = row.keys.first { KeyCap(text: first.glyphs) }
            if row.keys.count > 1, let last = row.keys.last {
                Text("…").font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textFaint)
                KeyCap(text: last.glyphs)
            }
        }
    }
}

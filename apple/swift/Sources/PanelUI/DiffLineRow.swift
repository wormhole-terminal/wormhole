// One line of a diff: two number gutters, the +/- marker and the text. Lines wrap rather than scroll
// sideways, since the panel is narrow and a sideways scroller hides what changed.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffLineRow: View {
    let line: DiffLine
    @Environment(\.panelCopy) private var copy

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 0) {
            gutter(line.oldNumber)
            gutter(line.newNumber)
            Text(marker).foregroundStyle(markerColor).frame(width: 14)
            Text(line.text.isEmpty ? " " : line.text)
                .italic(line.kind == .noNewline)
                .foregroundStyle(line.kind == .noNewline ? .tertiary : .primary)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .font(.system(size: 11, design: .monospaced))
        .padding(.trailing, Space._2)
        .background(tint)
        .contextMenu { Button("Copy Line") { copy(line.text) } }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(spoken)
    }

    private func gutter(_ number: Int?) -> some View {
        Text(number.map(String.init) ?? "")
            .monospacedDigit()
            .foregroundStyle(.tertiary)
            .frame(width: 30, alignment: .trailing)
            .padding(.trailing, Space._1)
    }

    private var marker: String {
        switch line.kind {
        case .added: "+"
        case .removed: "\u{2212}"
        case .context, .noNewline: ""
        }
    }

    private var markerColor: Color { line.kind == .added ? PanelColors.added : PanelColors.removed }

    private var tint: Color {
        switch line.kind {
        case .added: PanelColors.addedTint
        case .removed: PanelColors.removedTint
        case .context, .noNewline: .clear
        }
    }

    private var spoken: String {
        switch line.kind {
        case .added: "Added: \(line.text)"
        case .removed: "Removed: \(line.text)"
        case .context: line.text
        case .noNewline: "No newline at end of file"
        }
    }
}

// The palette's scrolling list: group labels, results and notes, edge to edge, with the highlight that follows
// the selection. Only the rows' heights are fixed here; the card works out how tall the whole list may be.

import DesignSystem
import PaletteModel
import SwiftUI

extension PaletteRow {
    /// A label is one line, a result or a note a palette row.
    var height: CGFloat {
        if case .label = self { Metric.row } else { Metric.paletteRowHeight }
    }
}

struct ResultList: View {
    let rows: [PaletteRow]
    let scope: PaletteScope
    let selection: String?
    @Binding var hovered: String?
    let motion: OverlayMotion
    let run: (PaletteResult) -> Void

    @Namespace private var highlight
    @Environment(\.tokens) private var tokens

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(rows) { row in
                        switch row {
                        case .label(let title, let count): GroupLabel(title: title, count: count)
                        case .result(let result): resultRow(result)
                        case .note(let text): note(text)
                        }
                    }
                }
                .animation(motion.selection, value: selection)
            }
            .scrollBounceBehavior(.basedOnSize)
            // The first result brings its label into view with it, so the list opens at its top.
            .onChange(of: selection) { _, id in
                guard let id else { return }
                let first = rows.first { $0.result != nil }?.id
                if id == first, let top = rows.first?.id { proxy.scrollTo(top, anchor: .top) } else { proxy.scrollTo(id) }
            }
        }
    }

    private func resultRow(_ result: PaletteResult) -> some View {
        ResultRow(
            result: result, scope: scope, isSelected: result.id == selection, isHovered: result.id == hovered,
            selection: highlight, motion: motion
        )
        .id(result.id)
        .onHover { inside in
            if inside { hovered = result.id } else if hovered == result.id { hovered = nil }
        }
        .onTapGesture { run(result) }
    }

    private func note(_ text: String) -> some View {
        Text(text)
            .font(.system(size: TextRole.body.size))
            .foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._4)
            .frame(maxWidth: .infinity, alignment: .leading)
            .frame(height: Metric.paletteRowHeight)
    }
}

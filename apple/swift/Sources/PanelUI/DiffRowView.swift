// One row of the diff (`render_row`): a gutter of line numbers, the +/- marker and the code on a row tinted by
// what happened to it. A hunk is a band with its header; in split mode each side is half the row, hatched where it
// has no line.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffRowView: View {
    let row: DiffRow
    let style: DiffStyle
    let code: DiffCode
    @Environment(\.tokens) private var tokens

    var body: some View {
        Group {
            switch row {
            case let .hunk(header):
                Text(header).foregroundStyle(tokens.diffHunkText).lineLimit(1).truncationMode(.tail)
                    .padding(.horizontal, Space._3).frame(maxWidth: .infinity, minHeight: style.rowHeight, alignment: .leading)
                    .background(tokens.diffHunkBackground)
            case let .unified(cell):
                HStack(alignment: .top, spacing: 0) {
                    gutter(cell.old)
                    gutter(cell.new)
                    marker(cell.kind)
                    codeView(cell)
                }
                .background(fill(cell.kind))
            case let .split(old, new):
                HStack(spacing: 0) {
                    half(old, old: true)
                    half(new, old: false)
                }
            }
        }
        .font(.system(size: style.fontSize, design: .monospaced))
        .frame(minHeight: style.rowHeight)
    }

    private func half(_ cell: DiffCell?, old: Bool) -> some View {
        Group {
            if let cell {
                HStack(alignment: .top, spacing: 0) {
                    gutter(old ? cell.old : cell.new)
                    marker(cell.kind)
                    codeView(cell)
                }
                .background(fill(cell.kind))
            } else {
                DiffHatch(rowHeight: style.rowHeight)
            }
        }
        .frame(maxWidth: .infinity)
        .clipped()
    }

    private func gutter(_ number: Int?) -> some View {
        Text(number.map(String.init) ?? "").foregroundStyle(tokens.diffLineNumber)
            .padding(.trailing, Space._2).frame(width: style.gutter, height: style.rowHeight, alignment: .trailing)
    }

    private func marker(_ kind: DiffLine.Kind) -> some View {
        Text(kind == .added ? "+" : kind == .removed ? "\u{2212}" : "")
            .foregroundStyle(kind == .added ? tokens.diffAdded : tokens.diffRemoved)
            .frame(width: DiffStyle.marker, height: style.rowHeight, alignment: .leading)
    }

    private func codeView(_ cell: DiffCell) -> some View {
        HStack(spacing: Space._2) {
            Text(code.text(cell.text))
                .lineLimit(style.wrap ? nil : 1)
                .fixedSize(horizontal: !style.wrap, vertical: style.wrap)
            if cell.noNewline {
                Text("no newline at end").font(TextRole.caption.font).foregroundStyle(tokens.diffLineNumber).fixedSize()
            }
        }
        .frame(maxWidth: .infinity, minHeight: style.rowHeight, alignment: .leading)
        .padding(.trailing, Space._3)
        .clipped()
    }

    private func fill(_ kind: DiffLine.Kind) -> Color {
        switch kind {
        case .added: tokens.diffAddedBackground
        case .removed: tokens.diffRemovedBackground
        default: .clear
        }
    }
}

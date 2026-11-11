// One row of the diff (`render_row`): a gutter of line numbers, the +/- marker and the code on a row tinted by what
// happened to it. A hunk is a band with its header; in split mode each side is half the row, hatched where it has no
// line. In a diff against the working tree a click on a line writes a comment on it (the comment icon shows under the
// pointer), and a commented line wears a primary bar along its left edge.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffRowView: View {
    let row: DiffRow
    let style: DiffStyle
    let code: DiffCode
    var path = ""
    var review: DiffReview?
    @Environment(\.tokens) private var tokens

    var body: some View {
        Group {
            switch row {
            case let .hunk(header):
                Text(header).foregroundStyle(tokens.diffHunkText).lineLimit(1).truncationMode(.tail)
                    .padding(.horizontal, Space._3).frame(maxWidth: .infinity, minHeight: style.rowHeight, alignment: .leading)
                    .background(tokens.diffHunkBackground)
            case let .unified(cell):
                DiffLineView(cell: cell, old: true, new: true, style: style, code: code, path: path, review: review)
            case let .split(old, new):
                HStack(spacing: 0) {
                    half(old, old: true)
                    half(new, old: false)
                }
            }
        }
        .font(style.code.font())
        .frame(minHeight: style.rowHeight)
    }

    @ViewBuilder private func half(_ cell: DiffCell?, old: Bool) -> some View {
        Group {
            if let cell {
                DiffLineView(cell: cell, old: old, new: !old, style: style, code: code, path: path, review: review)
            } else {
                DiffHatch(rowHeight: style.rowHeight)
            }
        }
        .frame(maxWidth: .infinity)
        .clipped()
    }
}

/// One line of code with its numbers: both of them in unified, its own side's in split.
struct DiffLineView: View {
    let cell: DiffCell
    let old: Bool
    let new: Bool
    let style: DiffStyle
    let code: DiffCode
    let path: String
    let review: DiffReview?
    @State private var hovering = false
    @State private var editing: LineAt?
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            if old { gutter(cell.old) }
            if new { gutter(cell.new) }
            marker
            codeView
        }
        .background(fill)
        .overlay(alignment: .leading) {
            if let comment, new || cell.kind == .removed { Rectangle().fill(tokens.primary).frame(width: DiffStyle.commentBar).help(comment.text) }
        }
        .contentShape(Rectangle())
        .onHover { hovering = $0 }
        .onTapGesture { if review != nil { editing = LineAt(cell) } }
        .popover(item: $editing, arrowEdge: .trailing) { at in
            CommentEditor(
                place: ReviewComment(path: path, side: at.side, line: at.line, code: "", text: "").location, at: at, existing: comment,
                save: { text in
                    review?.put(ReviewComment(path: path, side: at.side, line: at.line, code: at.code, text: text))
                    editing = nil
                },
                remove: { review?.put(ReviewComment(path: path, side: at.side, line: at.line, code: "", text: "")); editing = nil },
                cancel: { editing = nil })
        }
    }

    private var comment: ReviewComment? {
        guard let at = LineAt(cell) else { return nil }
        return review?.review.on(path, at.side, at.line)
    }

    private func gutter(_ number: Int?) -> some View {
        Text(number.map(String.init) ?? "").foregroundStyle(tokens.diffLineNumber)
            .padding(.trailing, Space._2).frame(width: style.gutter, height: style.rowHeight, alignment: .trailing)
            .overlay(alignment: .leading) {
                if hovering, review != nil, number != nil {
                    PanelIcon("text.bubble", size: 10).foregroundStyle(tokens.primary).padding(.leading, 2)
                }
            }
    }

    private var marker: some View {
        Text(cell.kind == .added ? "+" : cell.kind == .removed ? "\u{2212}" : "")
            .foregroundStyle(cell.kind == .added ? tokens.diffAdded : tokens.diffRemoved)
            .frame(width: DiffStyle.marker, height: style.rowHeight, alignment: .leading)
    }

    private var codeView: some View {
        HStack(spacing: Space._2) {
            Text(code.text(cell.text, spans: cell.spans))
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

    private var fill: Color {
        switch cell.kind {
        case .added: tokens.diffAddedBackground
        case .removed: tokens.diffRemovedBackground
        default: .clear
        }
    }
}

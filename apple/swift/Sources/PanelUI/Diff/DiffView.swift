// The diff of one file (`features/git/diff/render.rs`): the header, then the rows on the window's own surface, opaque, so
// the tints and syntax colours are the ones their contrast rules were checked against. With room for 560 pt it can lay
// old against new in two columns (the layout switch shows only then); narrower it is unified. A file with nothing to
// show is hatched with a note saying why. Esc closes it. In a diff against the working tree the lines can be commented.

import DesignSystem
import PanelModel
import SwiftUI

public struct DiffView: View {
    let file: ChangedFile
    @Bindable var session: DiffSession
    var position: (index: Int, count: Int)?
    var step: (Int) -> Void = { _ in }
    var close: () -> Void = {}
    var review: DiffReview?
    /// False when the window draws the header itself, in the terminal's title row (`PanelCoverTitle`).
    var showsHeader = true
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont
    @Environment(\.tabWidth) private var tabWidth

    init(
        file: ChangedFile, session: DiffSession, position: (index: Int, count: Int)? = nil,
        step: @escaping (Int) -> Void = { _ in }, close: @escaping () -> Void = {}, review: DiffReview? = nil,
        showsHeader: Bool = true
    ) {
        (self.file, self.session, self.position, self.step, self.close, self.review) = (file, session, position, step, close, review)
        self.showsHeader = showsHeader
    }

    /// Room for old against new in two columns.
    static let splitMinWidth: CGFloat = 560

    public var body: some View {
        GeometryReader { geometry in
            // The layout switch and "esc close" need room for two columns and lines to lay out: a binary file, an LFS
            // pointer or a submodule has none, and its card keeps only the ×.
            let fits = geometry.size.width >= Self.splitMinWidth && Self.hasLines(session)
            VStack(spacing: 0) {
                if showsHeader {
                    DiffHeader(file: file, diff: session.parsed, position: position, splitFits: fits, split: $session.split, wrap: $session.wrap, step: step, close: close)
                    Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth)
                }
                content(split: session.split && fits, width: geometry.size.width)
                if let review, !review.review.isEmpty { ReviewBar(review: review) }
            }
            .animation(.snappy(duration: 0.16), value: review?.review.isEmpty)
        }
        .background(tokens.surfaceWindow)
        .onExitCommand(perform: close)
        .accessibilityLabel("Diff of \(file.name)")
    }

    @ViewBuilder private func content(split: Bool, width: CGFloat) -> some View {
        let diff = session.parsed
        let style = DiffStyle(code: codeFont, digits: diff.map(DiffRows.numberDigits) ?? 3, wrap: session.wrap, advance: codeFont.advance)
        let rows = diff.map { split ? DiffRows.split($0) : DiffRows.unified($0) } ?? []
        let note = noteText(diff)
        ZStack {
            if rows.isEmpty { DiffHatch(rowHeight: style.rowHeight) }
            ScrollView(session.wrap ? .vertical : [.vertical, .horizontal]) {
                LazyVStack(spacing: 0) {
                    ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                        DiffRowView(row: row, style: style, code: DiffCode(tokens: tokens, tabWidth: tabWidth), path: file.path, review: review)
                    }
                }
                // Unwrapped lines scroll sideways, so the rows are as wide as the longest line needs (their bars are
                // `maxWidth: .infinity`); never narrower than the card, so the bars of short rows reach its edge.
                .frame(width: session.wrap ? width : max(width, Self.widest(rows, split: split, style: style, tabWidth: tabWidth)), alignment: .topLeading)
            }
            .defaultScrollAnchor(.topLeading)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .overlay { if split { Rectangle().fill(tokens.border).frame(width: tokens.metrics.borderWidth).frame(maxHeight: .infinity) } }
            if let note { DiffNote(text: note) }
        }
        .id(file.id)
        .transition(.opacity)
    }

    /// The diff has lines to lay out (not a binary file, an LFS pointer or a submodule): the layout switch is for them.
    static func hasLines(_ session: DiffSession) -> Bool { session.parsed.map { !$0.isBinary && $0.note == nil } ?? true }

    /// What the longest line needs in an unwrapped diff: gutter, marker, its characters at the code font's advance
    /// (tabs expanded, the cap of `DiffCode.maxRowCharacters` applied) and the trailing pad; two columns in split.
    private static func widest(_ rows: [DiffRow], split: Bool, style: DiffStyle, tabWidth: Int) -> CGFloat {
        var characters = 0
        func note(_ cell: DiffCell?) {
            guard let text = cell?.text else { return }
            characters = max(characters, min(DiffCode.expandTabs(text, tabWidth: tabWidth).columns.last ?? text.count, DiffCode.maxRowCharacters + 1))
        }
        for row in rows {
            switch row {
            case .hunk: break
            case let .unified(cell): note(cell)
            case let .split(old, new): note(old); note(new)
            }
        }
        let column = style.gutter * (split ? 1 : 2) + DiffStyle.marker + CGFloat(characters) * style.advance + Space._3
        return split ? column * 2 : column
    }

    private func noteText(_ diff: DiffFile?) -> String? {
        switch session.phase {
        case .loading, .idle: panelText("diff-reading", "Reading the diff\u{2026}")
        case let .failed(why): why
        case .shown: DiffNote.text(for: file, diff: diff)
        }
    }
}

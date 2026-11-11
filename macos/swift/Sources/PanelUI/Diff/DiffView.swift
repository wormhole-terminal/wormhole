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
    /// The diff is of the working tree (against HEAD or a branch): its lines have an icon that opens the file at them.
    var opensLines = false
    /// False when the window draws the header itself, in the terminal's title row (`PanelCoverTitle`).
    var showsHeader = true
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont
    @Environment(\.tabWidth) private var tabWidth
    @Environment(\.panelHost) private var host
    @Environment(\.panelRoot) private var root
    /// How far the code is scrolled sideways, as the scroller says (clamped to what the longest line allows where it is used).
    @State private var scrollX: CGFloat = 0

    init(
        file: ChangedFile, session: DiffSession, position: (index: Int, count: Int)? = nil,
        step: @escaping (Int) -> Void = { _ in }, close: @escaping () -> Void = {}, review: DiffReview? = nil,
        opensLines: Bool = false, showsHeader: Bool = true
    ) {
        self.opensLines = opensLines
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
                    DiffHeader(file: file, diff: session.parsed, position: position, splitFits: fits, split: $session.split, wrap: $session.wrap, step: step, close: close, gone: session.gone, outOfDate: session.refreshError)
                    Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth)
                }
                content(split: session.split && fits, width: geometry.size.width)
                if let review, !review.review.isEmpty { ReviewBar(review: review) }
            }
            .animation(.snappy(duration: 0.16), value: review?.review.isEmpty)
        }
        .background(tokens.surfaceWindow)
        .marked("diff")
        .onExitCommand(perform: close)
        .accessibilityLabel(panelText("diff-spoken", ["name": .text(file.name)], "Diff of \(file.name)"))
    }

    @ViewBuilder private func content(split: Bool, width: CGFloat) -> some View {
        if let large = session.parsed?.tooLarge {
            TooLargeCard(
                large: large, path: file.path, revealLabel: review != nil ? panelText("history-menu-reveal", "Reveal in Finder") : panelText("diff-reveal-current", "Reveal Current File in Finder"),
                load: session.loadAnyway)
        } else {
            lines(split: split, width: width)
        }
    }

    @ViewBuilder private func lines(split: Bool, width: CGFloat) -> some View {
        let diff = session.parsed
        let base = DiffStyle(code: codeFont, digits: diff.map(DiffRows.numberDigits) ?? 3, wrap: session.wrap, advance: codeFont.advance)
        let rows = diff.map { split ? DiffRows.split($0) : DiffRows.unified($0) } ?? []
        let note = noteText(diff)
        // Sideways scrolling is as far as the longest line is wider than its column (`render_diff_body`); the gutters stay put.
        let columns = split ? 2 : 1
        let room = width / CGFloat(columns) - base.gutter * CGFloat(split ? 1 : 2) - DiffStyle.marker
        let textWidth = CGFloat(diff?.widest ?? 0) * base.advance
        let maxScroll = session.wrap || rows.isEmpty ? 0 : max(0, textWidth + Self.codeEndRoom - room)
        let style = base.scrolled(min(scrollX, maxScroll))
        ZStack {
            if rows.isEmpty { DiffHatch(rowHeight: style.rowHeight) }
            ScrollView(session.wrap ? .vertical : [.vertical, .horizontal]) {
                LazyVStack(spacing: 0) {
                    ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                        DiffRowView(row: row, style: style, code: DiffCode(tokens: tokens, tabWidth: tabWidth), path: file.path, review: review, open: lineOpen)
                    }
                }
                // The rows are as wide as the card and stay under the pointer while the scroller moves: only each line's code
                // slides (`DiffStyle.scrollX`), so the numbers don't scroll away. The scroller's extent is what the longest line needs.
                .frame(width: width)
                .offset(x: style.scrollX)
                .frame(width: width + maxScroll, alignment: .leading)
            }
            .scrollIndicators(.hidden, axes: .horizontal)
            .onScrollGeometryChange(for: CGFloat.self) { $0.contentOffset.x } action: { _, offset in scrollX = max(0, offset) }
            .defaultScrollAnchor(.topLeading)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .overlay { if split { Rectangle().fill(tokens.border).frame(width: tokens.metrics.borderWidth).frame(maxHeight: .infinity) } }
            .overlay {
                if maxScroll > 0, textWidth > room - Self.codeEndPad {
                    OverflowCue(
                        spans: Self.codeSpans(width: width, split: split, style: style), visible: room, scroll: style.scrollX, max: maxScroll)
                }
            }
            if let diff, case .shown = session.phase, Self.hasNotes(diff) {
                columnNotes(diff.notes, split: split)
            } else if let note {
                DiffNote(text: note)
            }
        }
        .id(file.id)
        .transition(.opacity)
    }

    /// What the icon on a line does: only for a file that is in the working tree now (a deleted one has nothing to open).
    private var lineOpen: LineOpen? {
        guard opensLines, let root, let on = RepoFiles.resolve(root: root, relative: file.path), host?.openTooltip(line: nil) != nil, !session.gone else { return nil }
        return LineOpen(
            words: { host?.openTooltip(line: $0) ?? "" }, action: { host?.open(on, line: $0) })
    }

    /// The room after the code in its column (`CODE_END_PAD`) and the margin scrolling allows past the longest line (24).
    private static let codeEndPad: CGFloat = 12
    private static let codeEndRoom: CGFloat = 24

    /// Where each column's code is in the body (`code_spans`): after its gutter(s) and marker, to the column's right end.
    private static func codeSpans(width: CGFloat, split: Bool, style: DiffStyle) -> [ClosedRange<CGFloat>] {
        let inset = style.gutter + DiffStyle.marker
        return split ? [inset...width / 2, (width / 2 + inset)...width] : [(inset + style.gutter)...width]
    }

    /// The diff has lines to lay out (not a binary file, an LFS pointer, a submodule or a patch too large): the layout switch is for them.
    static func hasLines(_ session: DiffSession) -> Bool {
        session.parsed.map { !$0.isBinary && $0.note == nil && !$0.isLFS && $0.tooLarge == nil } ?? true
    }

    private static func hasNotes(_ diff: DiffFile) -> Bool { diff.notes.unified != nil || diff.notes.split.contains { $0 != nil } }

    /// The note over each column that has nothing to show (two when split, one when unified), centred in it.
    @ViewBuilder private func columnNotes(_ notes: DiffNotes, split: Bool) -> some View {
        if split, notes.split.count == 2 {
            HStack(spacing: 0) {
                ForEach(0..<2, id: \.self) { side in
                    ZStack { if let note = notes.split[side] { SideNoteView(note: note) } }.frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }
        } else if let note = notes.unified {
            SideNoteView(note: note)
        }
    }

    private func noteText(_ diff: DiffFile?) -> String? {
        switch session.phase {
        case .loading, .idle: panelText("diff-reading", "Reading the diff\u{2026}")
        case let .failed(why): why
        case .shown: diff.flatMap { Self.hasNotes($0) ? nil : DiffNote.text(for: file, diff: $0) }
        }
    }
}

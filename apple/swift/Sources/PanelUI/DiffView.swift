// The diff of one file (`features/git/diff/render.rs`): the header, then the rows on the window's own surface,
// opaque, so the tints and syntax colours are the ones their contrast rules were checked against. With room for
// 560 pt it can lay old against new in two columns (the layout switch shows only then); narrower it is unified.
// A file with nothing to show is hatched with a note saying why. Esc closes it.

import DesignSystem
import PanelModel
import SwiftUI

public struct DiffView: View {
    let file: ChangedFile
    let diff: DiffFile?
    var position: (index: Int, count: Int)?
    var step: (Int) -> Void = { _ in }
    var close: () -> Void = {}
    @State private var split = true
    @State private var wrap = true
    @Environment(\.tokens) private var tokens

    public init(
        file: ChangedFile, diff: DiffFile?, position: (index: Int, count: Int)? = nil,
        step: @escaping (Int) -> Void = { _ in }, close: @escaping () -> Void = {}
    ) {
        (self.file, self.diff, self.position, self.step, self.close) = (file, diff, position, step, close)
    }

    public var body: some View {
        GeometryReader { geometry in
            let fits = geometry.size.width >= 560
            VStack(spacing: 0) {
                DiffHeader(file: file, diff: diff, position: position, splitFits: fits, split: $split, wrap: $wrap, step: step, close: close)
                Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth)
                content(split: split && fits)
            }
        }
        .background(tokens.surfaceWindow)
        .onExitCommand(perform: close)
    }

    @ViewBuilder private func content(split: Bool) -> some View {
        let style = DiffStyle(digits: diff.map(DiffRows.numberDigits) ?? 3, wrap: wrap)
        let rows = diff.map { split ? DiffRows.split($0) : DiffRows.unified($0) } ?? []
        let note = DiffNote.text(for: file, diff: diff)
        ZStack {
            if rows.isEmpty { DiffHatch(rowHeight: style.rowHeight) }
            ScrollView {
                LazyVStack(spacing: 0) {
                    ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                        DiffRowView(row: row, style: style, code: DiffCode(tokens: tokens, language: SyntaxLanguage.of(path: file.path)))
                    }
                }
            }
            .overlay { if split { Rectangle().fill(tokens.border).frame(width: tokens.metrics.borderWidth).frame(maxHeight: .infinity) } }
            if let note { DiffNote(text: note) }
        }
        .id(file.id)
        .transition(.opacity)
    }
}

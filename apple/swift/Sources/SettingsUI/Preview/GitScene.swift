// Changes & diffs' scene (`git_preview.rs`): the Changes panel's list (or tree) at the left and a diff at the
// right, split or unified, in the diff tokens, so List/Tree and Split/Unified show what they do.

import DesignSystem
import SettingsModel
import SwiftUI

struct GitScene: View {
    let model: SettingsModel
    private static let files: [(path: String, letter: String)] = [("src/rows.rs", "M"), ("src/nav.rs", "M"), ("notes.md", "A")]

    var body: some View {
        let t = PreviewTheme.tokens(model)
        let tree = model.value("changes_tree").bool ?? false
        let split = model.value("diff_split").bool ?? true
        HStack(spacing: Space._3) {
            VStack(alignment: .leading, spacing: 2) {
                if tree { Label("src", systemImage: "folder").font(.system(size: 12)).foregroundStyle(t.textMuted) }
                ForEach(Self.files, id: \.path) { file in
                    let inSrc = file.path.hasPrefix("src/")
                    HStack {
                        Text(tree && inSrc ? String(file.path.dropFirst(4)) : file.path).font(.system(size: 12)).lineLimit(1)
                        Spacer()
                        Text(file.letter).font(.system(size: 11, weight: .semibold)).foregroundStyle(file.letter == "A" ? t.diffAdded : t.gitBranch)
                    }
                    .padding(.leading, tree && inSrc ? Space._4 : 0)
                }
            }
            .frame(width: 150).frame(maxHeight: .infinity, alignment: .top).foregroundStyle(t.textPrimary)
            HStack(spacing: 1) {
                if split {
                    diff(t, rows: [.removed])
                    diff(t, rows: [.added, .addedMore])
                } else {
                    diff(t, rows: [.removed, .added, .addedMore])
                }
            }
            .frame(height: 112).frame(maxHeight: .infinity, alignment: .top)
            .clipShape(RoundedRectangle(cornerRadius: t.metrics.radius))
            .contrastOutline(RoundedRectangle(cornerRadius: t.metrics.radius), \.cardEdge)
            .motion(Motion.layout, value: split)
        }
    }

    private enum Kind { case removed, added, addedMore }

    private func diff(_ t: ThemeTokens, rows: [Kind]) -> some View {
        VStack(spacing: 0) {
            row(t, "@@ fn rows()", t.diffHunkBackground, t.diffHunkText) // text: sample diff
            ForEach(rows, id: \.self) { kind in
                switch kind {
                case .removed: row(t, "- let n = 4;", t.diffRemovedBackground, t.textPrimary) // text: sample diff
                case .added: row(t, "+ let n = 8;", t.diffAddedBackground, t.textPrimary) // text: sample diff
                case .addedMore: row(t, "+ rows.shrink();", t.diffAddedBackground, t.textPrimary) // text: sample diff
                }
            }
            row(t, "  rows.len()", .clear, t.textMuted) // text: sample diff
            Spacer(minLength: 0)
        }
        .background(t.terminal.background.color)
    }

    private func row(_ t: ThemeTokens, _ text: String, _ fill: Color, _ color: Color) -> some View {
        Text(text).font(.system(size: 11, design: .monospaced)).foregroundStyle(color).lineLimit(1)
            .frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, Space._2).frame(height: 18).background(fill)
    }
}

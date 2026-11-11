// Appearance's scene (`stage.rs`): the window in miniature with the theme, the status bar and the font as they
// are now: a sidebar with a terminal row, a title row, and a terminal with sample output in the theme's ANSI
// colours. Drawn from the picked theme's own tokens, so a pick is visible before it is applied.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct WindowScene: View {
    let model: SettingsModel
    private static let size = CGSize(width: 420, height: 196)
    private static let sidebarWidth: CGFloat = 140

    var body: some View {
        let t = PreviewTheme.tokens(model)
        let family = model.value("font_family").string ?? "Menlo" // text: a font's name
        let mono = PreviewFont.mono(family, size: 11)
        let shape = RoundedRectangle(cornerRadius: t.metrics.radiusLg)
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: Space._2) {
                HStack(spacing: Space._1 + 1) { ForEach(0..<3, id: \.self) { _ in Circle().fill(t.textFaint).frame(width: 8, height: 8) } }
                    .padding(.bottom, Space._1)
                HStack { Text(Words.text("sidebar-list", fallback: "Terminals")).font(.system(size: 12, weight: .bold)); Spacer(); Image(systemName: "plus").font(.system(size: 10)) }
                HStack(spacing: Space._2) {
                    AgentTile(tokens: t, size: Metric.tile - 8)
                    VStack(alignment: .leading, spacing: 0) {
                        Text("Sidebar cards").font(.system(size: 11, weight: .semibold)).lineLimit(1).foregroundStyle(t.sidebarRowSelectedText) // text: sample terminal
                        Text(Words.text("settings-menu-bar-preview-working", fallback: "Working")).font(.system(size: 10)).foregroundStyle(t.sidebarRowSelectedMuted)
                    }
                }
                .padding(Space._1 + 2).frame(maxWidth: .infinity, alignment: .leading)
                .background(t.sidebarRowSelected, in: RoundedRectangle(cornerRadius: Radius.row))
                Spacer(minLength: 0)
            }
            .foregroundStyle(t.textPrimary)
            .padding(Space._2).frame(width: Self.sidebarWidth).frame(maxHeight: .infinity, alignment: .top)
            .background(t.sidebarBackground)
            VStack(spacing: 0) {
                HStack(spacing: Space._2) {
                    Label("main", systemImage: "arrow.triangle.branch").font(.system(size: 10))
                        .foregroundStyle(t.gitBranch).padding(.horizontal, Space._1 + 2)
                        .background(t.gitBranchBackground, in: RoundedRectangle(cornerRadius: t.metrics.radiusChip))
                    AgentTile(tokens: t, size: 14)
                    Text("Sidebar cards").font(.system(size: 11, weight: .medium)).foregroundStyle(t.textPrimary) // text: sample terminal
                    Spacer()
                }
                .padding(.horizontal, Space._2).frame(height: Metric.titleRow - 8).background(t.titlebarBackground)
                VStack(alignment: .leading, spacing: 3) {
                    line(mono, [("wormhole", t.terminal.ansi[4]), (" on ", t.terminal.foreground), ("main", t.terminal.ansi[2])]) // text: sample terminal output
                    line(mono, [("$ ", t.terminal.ansi[8]), ("claude", t.terminal.foreground)])
                    line(mono, [("● ", t.terminal.ansi[2]), ("Read rows.rs", t.terminal.foreground)]) // text: sample terminal output
                    line(mono, [("✻ ", t.terminal.ansi[3]), ("Working…", t.terminal.ansi[3])]) // text: sample terminal output
                    Spacer(minLength: 0)
                }
                .padding(Space._2).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .background(t.terminal.background.color)
                if model.value("status_bar").bool ?? false {
                    HStack { Text("~/Projects/wormhole").font(.system(size: 10)); Spacer(); Text(Words.text("window-status-working", ["count": .whole(1)], fallback: "1 working")).font(.system(size: 10)) }
                        .foregroundStyle(t.textMuted).padding(.horizontal, Space._2).frame(height: 18).background(t.surfacePanel)
                }
            }
        }
        .frame(width: Self.size.width, height: Self.size.height)
        .clipShape(shape)
        .overlay(shape.strokeBorder(t.border, lineWidth: t.metrics.borderWidth))
    }

    private func line(_ font: Font, _ parts: [(String, RGBA)]) -> some View {
        parts.reduce(Text("")) { $0 + Text($1.0).foregroundColor($1.1.color) }.font(font)
    }
}

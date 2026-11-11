import DesignSystem
import SwiftUI

/// One theme at a glance: its terminal palette, a few rows dressed in the tokens the way the sidebar
/// uses them, and every token as a labelled chip by group. Everything is read from `tokens` alone.
struct ThemeSwatchSheet: View {
    let theme: Theme
    let contrast: Bool
    let catalog: [TokenInfo]

    private var tokens: ThemeTokens { theme.tokens(increaseContrast: contrast) }

    var body: some View {
        let t = tokens
        VStack(alignment: .leading, spacing: 14) {
            Text("\(theme.name)  ·  \(theme.id)  ·  \(theme.appearance.rawValue)\(contrast ? "  ·  increase contrast" : "")")
                .font(.system(size: 15, weight: .semibold)).foregroundStyle(t.textPrimary)
            HStack(alignment: .top, spacing: 14) {
                terminal(t)
                sidebar(t)
            }
            ForEach(groups, id: \.0) { group, tokens in
                VStack(alignment: .leading, spacing: 4) {
                    Text(group).font(.system(size: 11, weight: .semibold)).foregroundStyle(t.textMuted)
                    Grid(alignment: .leading, horizontalSpacing: 6, verticalSpacing: 6) {
                        ForEach(Array(stride(from: 0, to: tokens.count, by: 6)), id: \.self) { start in
                            GridRow {
                                ForEach(tokens[start..<min(start + 6, tokens.count)], id: \.name) { chip($0.name, t) }
                            }
                        }
                    }
                }
            }
        }
        .padding(16)
        .frame(width: 980, alignment: .topLeading)
        .background(t.surfaceWindow)
    }

    private var groups: [(String, [TokenInfo])] {
        var order: [String] = []
        var byGroup: [String: [TokenInfo]] = [:]
        for token in catalog where !token.name.hasPrefix("metrics.") {
            if byGroup[token.group] == nil { order.append(token.group) }
            byGroup[token.group, default: []].append(token)
        }
        return order.map { ($0, byGroup[$0] ?? []) }
    }

    private func terminal(_ t: ThemeTokens) -> some View {
        let p = t.terminal
        return VStack(alignment: .leading, spacing: 6) {
            Text("~/wormhole $ cargo test").font(.system(size: 12, design: .monospaced)).foregroundStyle(p.foreground.color)
            HStack(spacing: 0) {
                ForEach(0..<16, id: \.self) { p.ansi[$0].color.frame(width: 22, height: 22) }
            }
            HStack(spacing: 8) {
                Text("selection").padding(.horizontal, 4).background(p.selectionBackground.color)
                Text("cursor").padding(.horizontal, 4).background(p.cursor.color)
            }.font(.system(size: 11)).foregroundStyle(p.foreground.color)
        }
        .padding(12).frame(width: 420, alignment: .leading).background(p.background.color)
    }

    private func sidebar(_ t: ThemeTokens) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            row("wormhole", "main  +12 -3", t.agentWorking, selected: true, t)
            row("api server", "needs input", t.agentAttention, selected: false, t)
            row("docs", "finished", t.agentDone, selected: false, t)
            HStack(spacing: 6) {
                Text("⎇ feature").foregroundStyle(t.gitBranch).padding(.horizontal, 6).padding(.vertical, 2)
                    .background(t.gitBranchBackground, in: RoundedRectangle(cornerRadius: t.metrics.radiusChip))
                Text("warning").foregroundStyle(t.statusWarningText).padding(.horizontal, 6).padding(.vertical, 2)
                    .background(t.bannerWarningBackground, in: RoundedRectangle(cornerRadius: t.metrics.radiusChip))
                Text("Save").foregroundStyle(t.textOnPrimary).padding(.horizontal, 8).padding(.vertical, 2)
                    .background(t.primary, in: RoundedRectangle(cornerRadius: t.metrics.radius))
            }.font(.system(size: 11))
        }
        .padding(8).frame(width: 300, alignment: .leading).background(t.sidebarBackground)
    }

    private func row(_ title: String, _ meta: String, _ dot: Color, selected: Bool, _ t: ThemeTokens) -> some View {
        HStack(spacing: 8) {
            Circle().fill(dot).frame(width: 8, height: 8)
            VStack(alignment: .leading, spacing: 0) {
                Text(title).font(.system(size: 13)).foregroundStyle(selected ? t.sidebarRowSelectedText : t.textPrimary)
                Text(meta).font(.system(size: 11)).foregroundStyle(selected ? t.sidebarRowSelectedMuted : t.textMuted)
            }
            Spacer()
        }
        .padding(.horizontal, 8).padding(.vertical, 4)
        .background(selected ? t.sidebarRowSelected : .clear, in: RoundedRectangle(cornerRadius: 8))
    }

    private func chip(_ name: String, _ t: ThemeTokens) -> some View {
        let color = t.color(named: name)
        return HStack(spacing: 4) {
            ZStack {
                // A checkerboard-free hint of transparency: the chip over the window surface and over its opposite.
                t.terminal.background.color
                color
            }
            .frame(width: 26, height: 14)
            .overlay(RoundedRectangle(cornerRadius: 2).stroke(t.textFaint, lineWidth: 0.5))
            Text(name).font(.system(size: 9)).foregroundStyle(t.textMuted).lineLimit(1)
        }
        .frame(width: 150, alignment: .leading)
    }
}

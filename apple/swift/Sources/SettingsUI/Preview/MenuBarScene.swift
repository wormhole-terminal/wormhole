// Menu bar's scene (`menu_bar_preview.rs`): the status item in each of its states as the setting makes it — the
// icon alone, with the status dot, or with the dot and the count of terminals that need you. Off draws nothing.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct MenuBarScene: View {
    let model: SettingsModel
    private static let states: [(value: String, label: String)] = [
        ("status_count", Words.text("settings-menu-bar-status-and-count", fallback: "Status and count")),
        ("status", Words.text("settings-menu-bar-status", fallback: "Status")),
        ("plain", Words.text("settings-menu-bar-icon-only", fallback: "Icon only")),
    ]

    var body: some View {
        let t = PreviewTheme.tokens(model)
        let on = model.value("menu_bar").bool ?? true
        let current = model.value("menu_bar_icon").string ?? "status_count"
        VStack(spacing: Space._2) {
            ForEach(Self.states, id: \.value) { state in
                HStack(spacing: Space._3) {
                    Text(state.label).font(.system(size: TextRole.label.size)).foregroundStyle(state.value == current ? t.primary : t.textMuted)
                        .frame(width: 120, alignment: .trailing)
                    bar(t, icon: state.value, on: on).overlay(
                        RoundedRectangle(cornerRadius: t.metrics.radius).strokeBorder(state.value == current ? t.primary : .clear, lineWidth: 2))
                }
            }
            if !on {
                Text(Words.text("settings-menu-bar-off", fallback: "The menu bar item is off")).font(.system(size: TextRole.caption.size))
                    .foregroundStyle(t.textMuted)
            }
        }
        .motion(Motion.selection, value: current)
        .motion(Motion.layout, value: on)
    }

    private func bar(_ t: ThemeTokens, icon: String, on: Bool) -> some View {
        HStack(spacing: Space._3) {
            Spacer()
            Image(systemName: "wifi").font(.system(size: Metric.glyph))
            Image(systemName: "battery.75percent").font(.system(size: Metric.glyph))
            if on { item(t, icon: icon) }
            Text("Fri 9:41").font(.system(size: 13))
        }
        .foregroundStyle(t.textPrimary)
        .padding(.horizontal, Space._3).frame(height: 24).frame(maxWidth: 320)
        .background(t.surfacePanel, in: RoundedRectangle(cornerRadius: t.metrics.radius))
    }

    private func item(_ t: ThemeTokens, icon: String) -> some View {
        HStack(spacing: 3) {
            Image(systemName: "asterisk").font(.system(size: Metric.glyph, weight: .bold))
            if icon != "plain" { Circle().fill(t.agentAttention).frame(width: Metric.statusDot, height: Metric.statusDot) }
            if icon == "status_count" { Text("2").font(.system(size: 12, weight: .semibold)).monospacedDigit() }
        }
    }
}

// Menu bar's scene (`menu_bar_preview.rs`): the status item in each of its states as the setting makes it — the
// icon alone, with the status dot, or with the dot and the count of terminals that need you. Off draws nothing.

import DesignSystem
import SettingsModel
import SwiftUI

struct MenuBarScene: View {
    let model: SettingsModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        let t = PreviewTheme.tokens(model)
        let on = model.value("menu_bar").bool ?? true
        let icon = model.value("menu_bar_icon").string ?? "status_count"
        VStack(spacing: Space._3) {
            HStack(spacing: Space._3) {
                Spacer()
                Image(systemName: "wifi").font(.system(size: Metric.glyph))
                Image(systemName: "battery.75percent").font(.system(size: Metric.glyph))
                if on { item(t, icon: icon) }
                Text("Fri 9:41").font(.system(size: 13))
            }
            .foregroundStyle(t.textPrimary)
            .padding(.horizontal, Space._3).frame(height: 24).frame(maxWidth: 420)
            .background(t.surfacePanel, in: RoundedRectangle(cornerRadius: t.metrics.radius))
            if !on {
                Text(String(localized: "settings-menu-bar-off", defaultValue: "The menu bar item is off")).font(.system(size: TextRole.caption.size))
                    .foregroundStyle(t.textMuted)
            }
        }
        .motion(Motion.layout, value: icon)
        .motion(Motion.layout, value: on)
    }

    private func item(_ t: ThemeTokens, icon: String) -> some View {
        HStack(spacing: 3) {
            Image(systemName: "asterisk").font(.system(size: Metric.glyph, weight: .bold))
            if icon != "plain" { Circle().fill(t.agentAttention).frame(width: Metric.statusDot, height: Metric.statusDot) }
            if icon == "status_count" { Text("2").font(.system(size: 12, weight: .semibold)).monospacedDigit() }
        }
    }
}

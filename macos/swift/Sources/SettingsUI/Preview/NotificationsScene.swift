// Notifications' scene (`notifications_preview.rs`): the banners a person gets — a terminal that needs them and
// an agent that finished — drawn as macOS draws a banner. When the app in front shows nothing
// (`notify_in_front` = none) the banners are drawn as what arrives while Wormhole is behind.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct NotificationsScene: View {
    let model: SettingsModel

    var body: some View {
        let t = PreviewTheme.tokens(model)
        let sound = model.value("notify_sound").string ?? "needs_you"
        VStack(spacing: Space._3) {
            banner(t, title: "Flaky login test", body: "\(Words.text("settings-menu-bar-preview-needs-input", fallback: "Needs input")) · Allow edit?", badge: t.agentAttention, sounds: sound != "none")
            if model.value("notify_agent_done").bool ?? true {
                banner(t, title: "Rate limits", body: "\(Words.text("settings-menu-bar-preview-finished", fallback: "Finished")) · 4m", badge: t.agentDone, sounds: sound == "all")
            }
        }
        .frame(maxWidth: 360)
        .motion(Motion.layout, value: model.value("notify_agent_done"))
    }

    private func banner(_ t: ThemeTokens, title: String, body: String, badge: Color, sounds: Bool) -> some View {
        HStack(spacing: Space._3) {
            AgentTile(tokens: t, size: Metric.tile, badge: badge)
            VStack(alignment: .leading, spacing: 0) {
                Text("Wormhole").font(.system(size: 11)).foregroundStyle(t.textMuted) // text: the product name
                Text(title).font(.system(size: 13, weight: .semibold)).foregroundStyle(t.textPrimary)
                Text(body).font(.system(size: 12)).foregroundStyle(t.textMuted)
            }
            Spacer(minLength: 0)
            if sounds { Image(systemName: "speaker.wave.2").font(.system(size: Metric.glyph)).foregroundStyle(t.textFaint) }
        }
        .padding(Space._3)
        .background(t.surfaceElevated, in: RoundedRectangle(cornerRadius: t.metrics.radiusLg))
        .overlay(RoundedRectangle(cornerRadius: t.metrics.radiusLg).strokeBorder(t.border, lineWidth: t.metrics.borderWidth))
    }
}

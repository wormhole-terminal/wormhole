// A full-width notice in a panel that asks for something (`style::callout`; the project's trust notice), flat like a banner:
// the kind's tint in the control corners with no outline (Increase Contrast draws the kind's), the icon in the lead column and
// the title at its side, then the body lined up under the title.

import DesignSystem
import SwiftUI

struct Callout<Content: View>: View {
    enum Kind { case info, warning, danger }

    let kind: Kind
    let title: String
    @ViewBuilder let content: Content
    @Environment(\.tokens) private var tokens
    @Environment(\.colorSchemeContrast) private var contrast

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius, style: .continuous)
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._2) {
                PanelIcon(symbol, size: Metric.glyph).foregroundStyle(iconColour)
                    .frame(width: Metric.lead, height: Metric.lead)
                Text(title).font(TextRole.body.font.weight(.semibold)).foregroundStyle(tokens.textPrimary)
            }
            // The body starts under the title: past the lead column and its gap (`CALLOUT_INDENT`).
            VStack(alignment: .leading, spacing: Space._2) { content }.padding(.leading, Metric.lead + Space._2)
        }
        .padding(.horizontal, Space._4).padding(.vertical, Space._3)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(fill, in: shape)
        .overlay { if contrast == .increased { shape.strokeBorder(edge, lineWidth: tokens.metrics.borderWidth) } }
        .marked("callout")
        .padding(.horizontal, PanelMetrics.inset - PanelMetrics.cardPad).padding(.vertical, Space._1)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
    }

    private var symbol: String {
        switch kind {
        case .info: "info.circle"
        case .warning: "shield"
        case .danger: "exclamationmark.triangle"
        }
    }

    private var iconColour: Color {
        switch kind {
        case .info: tokens.statusInfo
        case .warning: tokens.statusWarning
        case .danger: tokens.statusDanger
        }
    }

    private var fill: Color {
        switch kind {
        case .info: tokens.bannerInfoBackground
        case .warning: tokens.bannerWarningBackground
        case .danger: tokens.bannerDangerBackground
        }
    }

    private var edge: Color {
        switch kind {
        case .info: tokens.bannerInfoBorder
        case .warning: tokens.bannerWarningBorder
        case .danger: tokens.bannerDangerBorder
        }
    }
}

/// The small buttons of a notice (`Button::small`): primary or secondary, caption text.
struct NoticeButton: View {
    let title: String
    var primary = false
    var busy = false
    let action: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            Text(title).font(TextRole.caption.font.weight(.medium))
                .foregroundStyle(primary ? tokens.textOnPrimary : tokens.buttonText)
                .padding(.horizontal, Space._3).frame(height: Metric.controlSmall + 2)
                .background(primary ? tokens.primary : tokens.buttonBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .opacity(busy ? 0.6 : 1)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain).disabled(busy)
    }
}

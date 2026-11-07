// A boxed notice (`style::callout`) in a banner's colours: a title, then what it says. The project trust notice is the
// warning one; the fill and the edge are the theme's `banner.*` tokens.

import DesignSystem
import SwiftUI

struct Callout<Content: View>: View {
    enum Kind { case info, warning, danger }

    let kind: Kind
    let title: String
    @ViewBuilder let content: Content
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius, style: .continuous)
        VStack(alignment: .leading, spacing: PanelMetrics.gap) {
            Text(title).font(TextRole.label.font).foregroundStyle(tokens.textPrimary)
            content
        }
        .padding(PanelMetrics.gap)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(fill, in: shape)
        .overlay { shape.strokeBorder(edge, lineWidth: tokens.metrics.borderWidth) }
        .padding(.horizontal, PanelMetrics.inset - PanelMetrics.cardPad).padding(.vertical, Space._1)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
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

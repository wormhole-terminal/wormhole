// One choice in a row of folder colours (`style::color_choice`): a 28 pt square holding the colour's swatch, a 16 pt
// square in the small corners filled with the colour and outlined with the hairline so a pale colour on a pale surface
// still has an edge, with a `text.primary` ring round it (a gap inside) when it is the chosen one. None is an empty
// outline with a slash. Where the colour can't be changed the choices don't light up and are dimmed, all but the chosen
// one, which keeps its colour so it still reads. The keyboard's place is the shared ring (`focus_outline`); VoiceOver hears
// the row's radio group (`FolderColorCard`).

import AppKit
import DesignSystem
import SwiftUI

struct ColorChoice: View {
    let name: String
    let color: Color?
    let chosen: Bool
    let enabled: Bool
    let action: () -> Void
    @State private var hovering = false
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    static let dot: CGFloat = 16
    /// The ring round a chosen choice and the gap inside it.
    private static let ring: CGFloat = 2
    private static let gap: CGFloat = 2

    var body: some View {
        Button(action: action) {
            swatch
                .frame(width: Self.dot + 2 * (Self.ring + Self.gap), height: Self.dot + 2 * (Self.ring + Self.gap))
                .overlay {
                    if chosen {
                        RoundedRectangle(cornerRadius: tokens.metrics.radiusSm + Self.ring + Self.gap)
                            .strokeBorder(tokens.textPrimary, lineWidth: Self.ring)
                    }
                }
                .frame(width: Metric.hitMin, height: Metric.hitMin)
                .background(enabled && hovering ? tokens.bandHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .opacity(!enabled && !chosen ? 0.5 : 1)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain).disabled(!enabled).onHover { hovering = $0 }
        .focused($focused)
        .focusEffectDisabled()
        .keyboardFocusRing(focused, radius: tokens.metrics.radius)
        .help(name)
        .accessibilityLabel(name)
    }

    @ViewBuilder private var swatch: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusSm)
        if let color {
            shape.fill(color).frame(width: Self.dot, height: Self.dot).overlay { shape.strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
        } else {
            shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth).frame(width: Self.dot, height: Self.dot)
                .overlay { PanelIcon("nosign", size: 10).foregroundStyle(tokens.textMuted) }
        }
    }
}

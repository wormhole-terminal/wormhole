// The windows' buttons: a filled primary for the one main action of a window or step (Get started, Install Update)
// and an outlined secondary for the rest (Copy, End, Check now), both from the theme's tokens (`primary` and
// `text.on_primary`, `button.*`, `border.strong`), so a theme colours them whether or not the window is in front (the system's
// prominent button goes grey in a background window). They are real `Button`s: focus, space and return, and
// VoiceOver come with them. Quiet text actions use `.plain`.

import DesignSystem
import SwiftUI

extension View {
    func primaryAction() -> some View { buttonStyle(ThemedButtonStyle(prominent: true)) }
    func secondaryAction() -> some View { buttonStyle(ThemedButtonStyle(prominent: false)) }
}

private struct ThemedButtonStyle: ButtonStyle {
    let prominent: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var enabled
    @Environment(\.controlSize) private var size
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius)
        let height = size == .small ? Metric.controlSmall + Space._1 : Metric.control
        return configuration.label
            .font(TextRole.body.font)
            .foregroundStyle(prominent ? tokens.textOnPrimary : tokens.buttonText)
            .padding(.horizontal, size == .small ? Space._2 : Space._3)
            .frame(minHeight: height)
            .background {
                if prominent {
                    shape.fill(hovering || configuration.isPressed ? tokens.primaryHover : tokens.primary)
                } else {
                    shape.fill(configuration.isPressed ? tokens.stateActive : hovering ? tokens.buttonHover : tokens.buttonBackground)
                }
            }
            .overlay { if !prominent { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) } }
            .contentShape(shape)
            .opacity(enabled ? 1 : 0.5)
            .pressed(configuration.isPressed)
            .onHover { hovering = $0 }
            .motion(Motion.press, value: hovering)
    }
}

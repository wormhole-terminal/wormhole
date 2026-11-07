// What both overlays share: a dimmed backdrop, a centred card and the way they come and go. The overlay owns
// its presented state so the host only shows or removes the view; leaving plays first, then `dismiss` runs.

import DesignSystem
import SwiftUI

struct OverlayChrome<Content: View>: View {
    let width: CGFloat
    let dismiss: @MainActor () -> Void
    /// The content gets `close`, which plays the exit animation and then calls `dismiss`.
    @ViewBuilder let content: (_ close: @escaping @MainActor () -> Void) -> Content

    @State private var presented: Bool
    @State private var closing = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    /// `startsPresented` is for snapshots, where `onAppear` never runs.
    init(
        width: CGFloat, startsPresented: Bool = false, dismiss: @escaping @MainActor () -> Void,
        @ViewBuilder content: @escaping (_ close: @escaping @MainActor () -> Void) -> Content
    ) {
        self.width = width
        self.dismiss = dismiss
        self.content = content
        _presented = State(initialValue: startsPresented)
    }

    private var motion: PaletteMotion { PaletteMotion(reduceMotion: reduceMotion) }

    var body: some View {
        ZStack(alignment: .top) {
            // A tap outside the card closes; the dim is a dark scrim in both appearances so the card lifts off.
            Color.black.opacity(presented ? 0.28 : 0)
                .animation(motion.backdrop, value: presented)
                .contentShape(Rectangle())
                .onTapGesture(perform: close)
                .accessibilityHidden(true)
            card
                .frame(width: width)
                .padding(.top, Space._6 * 3)
                .scaleEffect(presented ? 1 : motion.cardScale, anchor: .top)
                .opacity(presented ? 1 : 0)
        }
        .ignoresSafeArea()
        .onAppear { withAnimation(motion.present) { presented = true } }
        .accessibilityAddTraits(.isModal)
    }

    private var card: some View {
        content(close)
            .background(cardBackground)
            .clipShape(RoundedRectangle(cornerRadius: Radius.card, style: .continuous))
            // The hairline ring does the edge; the shadow is only enough to separate card from scrim.
            .overlay(
                RoundedRectangle(cornerRadius: Radius.card, style: .continuous)
                    .strokeBorder(Palette.divider, lineWidth: 1)
            )
            .shadow(color: .black.opacity(0.16), radius: 20, y: 8)
    }

    @ViewBuilder private var cardBackground: some View {
        if reduceTransparency {
            Color(nsColor: .windowBackgroundColor)
        } else {
            Rectangle().fill(.regularMaterial)
        }
    }

    private func close() {
        guard !closing else { return }
        closing = true
        withAnimation(motion.dismiss, completionCriteria: .removed) {
            presented = false
        } completion: {
            dismiss()
        }
    }
}

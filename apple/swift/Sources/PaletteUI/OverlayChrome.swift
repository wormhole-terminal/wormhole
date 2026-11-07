// What both overlays share: the scrim (`surface.overlay`, over the whole window, taking the clicks) and the way
// they come and go. The overlay owns its presented state, so the host only shows or removes the view; leaving
// plays first and `dismiss` runs when it is over. A click on the scrim closes.

import DesignSystem
import SwiftUI

/// What the content of an overlay is told: the window's size, and how to close. `close` plays the exit and then
/// runs `afterwards`, or the overlay's own `dismiss` when it is nil.
struct OverlayContext {
    let size: CGSize
    let close: @MainActor (_ afterwards: (@MainActor () -> Void)?) -> Void
}

struct OverlayChrome<Content: View>: View {
    /// The palette is there at once and fades out; the sheet fades in and is gone at once.
    enum Style { case palette, sheet }

    let style: Style
    let top: CGFloat
    let dismiss: @MainActor () -> Void
    let content: (OverlayContext) -> Content

    @State private var visible: Bool
    @State private var closing = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.tokens) private var tokens

    /// `startsPresented` is for snapshots, where `onAppear` never runs.
    init(
        style: Style, top: CGFloat, startsPresented: Bool = false, dismiss: @escaping @MainActor () -> Void,
        @ViewBuilder content: @escaping (OverlayContext) -> Content
    ) {
        self.style = style
        self.top = top
        self.dismiss = dismiss
        self.content = content
        _visible = State(initialValue: startsPresented || style == .palette)
    }

    private var motion: OverlayMotion { OverlayMotion(reduceMotion: reduceMotion) }

    var body: some View {
        GeometryReader { window in
            ZStack(alignment: .top) {
                tokens.surfaceOverlay
                    .contentShape(Rectangle())
                    .onTapGesture { close(then: nil) }
                    .accessibilityHidden(true)
                content(OverlayContext(size: window.size, close: { close(then: $0) }))
                    .padding(.top, top)
                    .frame(maxWidth: .infinity, alignment: .top)
            }
        }
        .ignoresSafeArea()
        .opacity(visible ? 1 : 0)
        .allowsHitTesting(!closing)
        .onAppear {
            guard style == .sheet, !visible else { return }
            withAnimation(motion.entrance) { visible = true }
        }
        .accessibilityAddTraits(.isModal)
    }

    private func close(then afterwards: (@MainActor () -> Void)?) {
        guard !closing else { return }
        closing = true
        let finish = afterwards ?? dismiss
        // The sheet has no exit animation; neither has anything under Reduce Motion: a cut.
        guard style == .palette, let exit = motion.exit else { return finish() }
        withAnimation(exit, completionCriteria: .logicallyComplete) {
            visible = false
        } completion: {
            finish()
        }
    }
}

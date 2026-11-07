// A keyboard-driven list above a detail pane. The list is as tall as its rows up to a share of the panel, so a
// short list does not leave a hole and a long one does not push the detail out; with nothing open it takes it all.
// Arrow keys call `step`; the selected row scrolls into view; a click anywhere in the list takes the keyboard.

import DesignSystem
import SwiftUI

struct SelectableListSplit<Top: View, Bottom: View>: View {
    let selection: AnyHashable?
    let hasDetail: Bool
    let label: String
    var maxTopShare: CGFloat = 0.45
    let step: (Int) -> Void
    /// → and ← on a list that folds, and ↩.
    var fold: ((Bool) -> Void)?
    var press: (() -> Void)?
    @ViewBuilder let top: Top
    @ViewBuilder let bottom: Bottom
    @FocusState private var focused: Bool
    @Environment(\.panelMotion) private var motion
    @Environment(\.tokens) private var tokens

    var body: some View {
        GeometryReader { geometry in
            VStack(spacing: 0) {
                top.frame(maxHeight: hasDetail ? geometry.size.height * maxTopShare : .infinity)
                if hasDetail {
                    Rectangle().fill(tokens.divider).frame(height: tokens.metrics.dividerWidth)
                    bottom.transition(motion.content)
                }
            }
            .animation(motion.fold, value: hasDetail)
        }
        .focusable()
        .focused($focused)
        .focusEffectDisabled()
        .simultaneousGesture(TapGesture().onEnded { focused = true })
        .onKeyPress(.downArrow) { step(1); return .handled }
        .onKeyPress(.upArrow) { step(-1); return .handled }
        .onKeyPress(.rightArrow) { fold.map { $0(true) } == nil ? .ignored : .handled }
        .onKeyPress(.leftArrow) { fold.map { $0(false) } == nil ? .ignored : .handled }
        .onKeyPress(.return) { press.map { $0() } == nil ? .ignored : .handled }
        .accessibilityLabel(label)
    }
}

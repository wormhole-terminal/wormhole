// A keyboard-driven list above a detail pane. The list is as tall as its rows up to a share of the panel, so a
// short list does not leave a hole and a long one does not push the detail out. Arrow keys call `step`;
// the selected row scrolls into view; a click anywhere in the list takes keyboard focus.

import DesignSystem
import SwiftUI

struct SelectableListSplit<Top: View, Bottom: View>: View {
    let selection: AnyHashable?
    let label: String
    var maxTopShare: CGFloat = 0.45
    let step: (Int) -> Void
    @ViewBuilder let top: Top
    @ViewBuilder let bottom: Bottom
    @State private var topHeight: CGFloat?
    @FocusState private var focused: Bool
    @Environment(\.panelMotion) private var motion

    var body: some View {
        GeometryReader { geometry in
            VStack(spacing: 0) {
                list.frame(height: min(topHeight ?? .infinity, geometry.size.height * maxTopShare))
                Rectangle().fill(Palette.divider).frame(height: 1)
                bottom
            }
        }
    }

    private var list: some View {
        ScrollViewReader { proxy in
            ScrollView {
                top
                    .padding(.vertical, Space._1)
                    .onGeometryChange(for: CGFloat.self, of: { $0.size.height }) { topHeight = $0 }
            }
            .onChange(of: selection) { _, id in if let id { proxy.scrollTo(id) } }
        }
        .animation(motion.fold, value: topHeight)
        .focusable()
        .focused($focused)
        .focusEffectDisabled()
        .simultaneousGesture(TapGesture().onEnded { focused = true })
        .onKeyPress(.downArrow) { step(1); return .handled }
        .onKeyPress(.upArrow) { step(-1); return .handled }
        .accessibilityLabel(label)
    }
}

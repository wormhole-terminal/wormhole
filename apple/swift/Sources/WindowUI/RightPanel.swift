// The right panel: its tab bar in the title row's height and the chosen tab's body below it. The body slides
// 10 pt in from the side of the tab it came from, fading in over 140 ms easing out, and the old one is gone
// at once (`view::motion::tabs::content`). Reduce Motion keeps only the fade.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct RightPanel: View {
    let tab: PanelTab
    let send: @MainActor (WorkspaceAction) -> Void
    /// The tab before this one, for which side the new body comes from. It is set after the body that needs it
    /// was built, so while that body is built it still holds the tab that was left.
    @State private var before: PanelTab?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    static let contentIn = Animation.timingCurve(0.2, 0, 0, 1, duration: 0.14)
    static let contentSlide: CGFloat = 10

    var body: some View {
        VStack(spacing: 0) {
            PanelTabBar(tab: tab) { send(.showPanelTab($0)) }
            PanelBody(tabName: tab.rawValue)
                .id(tab)
                .transition(.asymmetric(insertion: arrival, removal: .identity))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .clipped()
        }
        .animation(Self.contentIn, value: tab)
        .onChange(of: tab) { before = tab }
        .onAppear { before = tab }
    }

    private var arrival: AnyTransition {
        guard !reduceMotion else { return .opacity }
        let order = PanelTab.allCases
        let from = before.flatMap(order.firstIndex(of:)) ?? 0
        let to = order.firstIndex(of: tab) ?? 0
        let start = to < from ? -Self.contentSlide : Self.contentSlide
        return .offset(x: start).combined(with: .opacity)
    }
}

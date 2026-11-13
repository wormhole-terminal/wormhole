// The right panel: its tab bar in the title row's height and the chosen tab's body below it. The body slides
// 10 pt in from the side of the tab it came from, fading in over 140 ms easing out, and the old one is gone
// at once (`view::motion::tabs::content`). Reduce Motion keeps only the fade.

import DesignSystem
import PanelUI
import SwiftUI
import WorkspaceModel

struct RightPanel: View {
    let hub: WindowHub
    /// The tab before this one, for which side the new body comes from. It is set after the body that needs it
    /// was built, so while that body is built it still holds the tab that was left.
    @State private var before: PanelTab?
    @Environment(\.reduceMotion) private var reduceMotion

    static let contentIn = Animation.timingCurve(0.2, 0, 0, 1, duration: 0.14)
    static let contentSlide: CGFloat = 10

    private var tab: PanelTab { hub.state.panelTab }
    private var controller: PanelController { hub.panel }

    var body: some View {
        VStack(spacing: 0) {
            PanelTabBar(tab: tab, tabs: hub.panelTabs, select: { hub.send(.showPanelTab($0)) }, badges: badges, spokenTab: spokenTab)
                // The Notes tab's one action sits in the strip beside the panel toggle (`stash_button`).
                .overlay(alignment: .trailing) {
                    if tab == .notes { StashPromptButton(controller: controller).padding(.trailing, Chrome.panelToggleSlot) }
                }
            PanelBody(tabName: tab.rawValue, controller: controller)
                .id(tab)
                .environment(\.showKeyMenu, hub.actions.showKeyMenu)
                .transition(.asymmetric(insertion: arrival, removal: .identity))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .clipped()
        }
        .animation(Self.contentIn, value: tab)
        .onChange(of: tab) { before = tab }
        .onChange(of: hub.panelTabs) { hub.leaveMissingTab() }
        .onAppear { before = tab }
    }

    /// The counts the strip shows, read from the stores so a change redraws the strip.
    private var badges: [PanelTab: Int] {
        var out: [PanelTab: Int] = [:]
        if let count = controller.badge(for: .changes) { out[.changes] = count }
        return out
    }

    /// The tab's name with its count, as the core says it: the Notes tab's open steps and notes, Changes' files (nil outside a repository).
    private func spokenTab(_ tab: PanelTab) -> String? {
        hub.store.spoken(
            panelTab: tab, openSteps: controller.notes.model.tasks.remaining, notes: controller.notes.model.filledNotes,
            files: controller.changes.phase == .loaded ? controller.changes.files.count : nil)
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

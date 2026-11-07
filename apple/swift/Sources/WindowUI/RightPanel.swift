// The right panel: a tab strip in the title row's height and the chosen tab's content below it.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct RightPanel: View {
    let tab: PanelTab
    let send: @MainActor (WorkspaceAction) -> Void
    @Namespace private var indicator
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: Space._1) {
                ForEach(PanelTab.allCases, id: \.self) { candidate in
                    TabButton(tab: candidate, isSelected: candidate == tab, indicator: indicator) {
                        send(.showPanelTab(candidate))
                    }
                }
                Spacer(minLength: 0)
                ChromeButton(systemImage: "sidebar.right", label: "Hide panel") { send(.togglePanel) }
            }
            .padding(.leading, Space._3)
            .padding(.trailing, Space._2)
            .frame(height: Chrome.titleHeight)
            .background(WindowDragArea())
            Palette.divider.frame(height: 1)
            PanelBody(tabName: tab.rawValue)
        }
        // One animation for the indicator's slide and the body's cross-fade, so they move together.
        .animation(WindowMotion.tab(reduceMotion: reduceMotion), value: tab)
        .background(Chrome.surface)
    }
}

private struct TabButton: View {
    let tab: PanelTab
    let isSelected: Bool
    let indicator: Namespace.ID
    let action: @MainActor () -> Void

    var body: some View {
        Button(action: action) {
            Text(tab.title)
                .font(.callout)
                .foregroundStyle(isSelected ? Palette.textPrimary : Palette.textSecondary)
                .padding(.horizontal, Space._3)
                .padding(.vertical, Space._1)
                .background {
                    if isSelected {
                        RoundedRectangle(cornerRadius: Radius.chip)
                            .fill(Palette.hover)
                            .matchedGeometryEffect(id: "tab-indicator", in: indicator)
                    }
                }
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}

extension PanelTab {
    var title: String { rawValue.capitalized }
}

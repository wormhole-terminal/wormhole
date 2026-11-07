// The right panel: a tab strip in the title row's height and a body per tab. The bodies are placeholders:
// real Notes, Changes and History are out of scope for this experiment.

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
            PlaceholderBody(tab: tab)
                .id(tab)
                .transition(.opacity)
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

private struct PlaceholderBody: View {
    let tab: PanelTab

    var body: some View {
        VStack(spacing: Space._2) {
            Image(systemName: tab.symbol).font(.largeTitle)
            Text(tab.title).font(.headline)
            Text("Placeholder: this tab is not built in the Swift experiment.")
                .font(.caption)
                .multilineTextAlignment(.center)
        }
        .foregroundStyle(Palette.textSecondary)
        .padding(Space._6)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .accessibilityElement(children: .combine)
    }
}

extension PanelTab {
    var title: String { rawValue.capitalized }

    var symbol: String {
        switch self {
        case .notes: "note.text"
        case .changes: "plusminus"
        case .history: "clock.arrow.circlepath"
        }
    }
}

// The right panel's title row: its icon tabs (28 pt squares, 4 apart, as `icon_tab` draws them) from the 6 pt
// pad, and room at the end for the panel toggle that floats over it. One selection shape sits under the tabs
// and springs between them on the quick spring (400/30, `view::motion::tabs`), so a second switch mid-way
// carries on from where the first was; Reduce Motion moves it at once. Each tab is a button for VoiceOver in
// a tab list, and the arrow keys walk the list as one stop.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct PanelTabBar: View {
    let tab: PanelTab
    let select: @MainActor (PanelTab) -> Void
    @Namespace private var selection
    @Environment(\.chrome) private var colours
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    /// The quick spring the selection and the tabs' tint move on.
    static let spring = Animation.spring(Spring(mass: 1, stiffness: 400, damping: 30))

    var body: some View {
        HStack(spacing: Chrome.tabGap) {
            ForEach(Array(PanelTab.barOrder.enumerated()), id: \.element) { position, candidate in
                PanelTabButton(tab: candidate, isSelected: candidate == tab, namespace: selection) { select(candidate) }
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel(candidate.title)
                    .accessibilityValue("\(position + 1) of \(PanelTab.barOrder.count)")
                    .accessibilityAddTraits(candidate == tab ? [.isButton, .isSelected] : .isButton)
            }
            Spacer(minLength: 0)
        }
        .animation(reduceMotion ? nil : Self.spring, value: tab)
        .padding(.leading, Chrome.titleRowPad)
        .padding(.trailing, Chrome.panelToggleSlot)
        .frame(height: Chrome.titleHeight)
        .background(WindowDragArea())
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Right panel")
    }
}

private struct PanelTabButton: View {
    let tab: PanelTab
    let isSelected: Bool
    let namespace: Namespace.ID
    let action: @MainActor () -> Void
    @State private var hovering = false
    @Environment(\.chrome) private var colours

    var body: some View {
        Button(action: action) {
            glyph
                .frame(width: Chrome.iconTab, height: Chrome.iconTab)
                .background {
                    if isSelected {
                        RoundedRectangle(cornerRadius: colours.radius).fill(colours.selected)
                            .matchedGeometryEffect(id: "selection", in: namespace)
                    } else if hovering {
                        RoundedRectangle(cornerRadius: colours.radius).fill(colours.bandHover)
                    }
                }
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help(tab.title)
    }

    @ViewBuilder private var glyph: some View {
        let colour = isSelected ? colours.selectedText : hovering ? colours.text : colours.muted
        Group {
            if tab == .history {
                CommitGlyph(color: colour)
            } else {
                Image(systemName: tab.symbol).font(.system(size: Metric.glyph, weight: .medium)).foregroundStyle(colour)
            }
        }
        .frame(width: Chrome.iconTab, height: Chrome.iconTab)
    }
}

extension PanelTab {
    /// The tabs in the GPUI app's order, which the model's own order (the enum's) is not.
    static let barOrder: [PanelTab] = [.changes, .history, .notes]

    var title: String { rawValue.capitalized }

    /// Lucide's file-text, git-compare and history, as the GPUI app's tabs draw them, in the system's symbols.
    var symbol: String {
        switch self {
        case .notes: "doc.text"
        case .changes: "arrow.triangle.pull"
        case .history: "circle"
        }
    }
}

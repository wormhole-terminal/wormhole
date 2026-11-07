// The right panel's title row: its icon tabs (28 pt squares, 4 apart, as `icon_tab` draws them) from the 6 pt
// pad, and room at the end for the panel toggle that floats over it. One selection shape sits under the tabs
// and springs between them on the quick spring (400/30, `view::motion::tabs`), so a second switch mid-way
// carries on from where the first was; Reduce Motion moves it at once. Each tab is a button for VoiceOver in
// a tab list, and the arrow keys walk the list as one stop.

import DesignSystem
import PanelUI
import SwiftUI
import WorkspaceModel

struct PanelTabBar: View {
    let tab: PanelTab
    let select: @MainActor (PanelTab) -> Void
    /// The counts on the tabs that have one (the changed files, the agents that reported).
    var badges: [PanelTab: Int] = [:]
    @Namespace private var selection
    @Environment(\.chrome) private var colours
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    /// The quick spring the selection and the tabs' tint move on.
    static let spring = Animation.spring(Spring(mass: 1, stiffness: 400, damping: 30))

    var body: some View {
        HStack(spacing: Chrome.tabGap) {
            ForEach(Array(PanelTab.barOrder.enumerated()), id: \.element) { position, candidate in
                PanelTabButton(tab: candidate, isSelected: candidate == tab, badge: badges[candidate], namespace: selection) { select(candidate) }
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel(candidate.title)
                    .accessibilityValue(spoken(candidate, position: position))
                    .accessibilityAddTraits(candidate == tab ? [.isButton, .isSelected] : .isButton)
            }
            Spacer(minLength: 0)
        }
        .animation(reduceMotion ? nil : Self.spring, value: tab)
        .animation(reduceMotion ? nil : Self.spring, value: badges)
        .padding(.leading, Chrome.titleRowPad)
        .padding(.trailing, Chrome.panelToggleSlot)
        .frame(height: Chrome.titleHeight)
        .background(WindowDragArea())
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Right panel")
    }
}

extension PanelTabBar {
    /// "2 of 6", and the count where the tab has one ("Changes: 3 files" is the tab's own label's business).
    fileprivate func spoken(_ tab: PanelTab, position: Int) -> String {
        let place = "\(position + 1) of \(PanelTab.barOrder.count)"
        return badges[tab].map { "\(place), \($0)" } ?? place
    }
}

private struct PanelTabButton: View {
    let tab: PanelTab
    let isSelected: Bool
    let badge: Int?
    let namespace: Namespace.ID
    let action: @MainActor () -> Void
    @State private var hovering = false
    @Environment(\.chrome) private var colours

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._1) {
                glyph
                if let badge { TabCountBadge(count: badge, selected: isSelected) }
            }
            .padding(.horizontal, badge == nil ? 0 : Space._2)
            .frame(minWidth: Chrome.iconTab, minHeight: Chrome.iconTab)
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
    static let barOrder: [PanelTab] = [.changes, .history, .notes, .usage, .project, .files]

    var title: String { rawValue.capitalized }

    /// Lucide's file-text, git-compare, history, gauge, folder-cog and folder, as the GPUI app's tabs draw them, in the system's symbols.
    var symbol: String {
        switch self {
        case .notes: "doc.text"
        case .changes: "arrow.triangle.pull"
        case .history: "circle"
        case .usage: "gauge.with.dots.needle.50percent"
        case .project: "folder.badge.gearshape"
        case .files: "folder"
        }
    }
}

// The right panel's title row: its icon tabs (28 pt squares, 4 apart, as `icon_tab` draws them) from the 6 pt
// pad, and room at the end for the panel toggle that floats over it. One selection shape sits under the tabs
// and springs between them on the quick spring (400/30, `view::motion::tabs`), so a second switch mid-way
// carries on from where the first was; Reduce Motion moves it at once. Each tab is a button for VoiceOver in
// a tab list, and the arrow keys walk the list as one stop.

import DesignSystem
import PanelUI
import SwiftUI
import WordsModel
import WorkspaceModel

struct PanelTabBar: View {
    let tab: PanelTab
    /// The tabs on offer, in the bar's order: the core's (a feature that is off brings no tab); the sample's is all six.
    var tabs: [PanelTab] = PanelTab.barOrder
    let select: @MainActor (PanelTab) -> Void
    /// The counts on the tabs that have one (the changed files, the agents that reported).
    var badges: [PanelTab: Int] = [:]
    /// What VoiceOver says of a tab, with its counts (core's words), and its tooltip, which names its key.
    var spoken: (PanelTab) -> String = { $0.title }
    var tip: (PanelTab) -> String = { $0.title }
    @Namespace private var selection
    @Environment(\.chrome) private var colours
    @Environment(\.reduceMotion) private var reduceMotion

    /// The quick spring the selection and the tabs' tint move on.
    static let spring = Animation.spring(Spring(mass: 1, stiffness: 400, damping: 30))

    var body: some View {
        HStack(spacing: Chrome.tabGap) {
            ForEach(Array(tabs.enumerated()), id: \.element) { position, candidate in
                PanelTabButton(tab: candidate, isSelected: candidate == tab, badge: badges[candidate], tip: tip(candidate), namespace: selection) { select(candidate) }
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel(spoken(candidate))
                    .accessibilityValue(place(position))
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
        .accessibilityAddTraits(.isTabBar)
        .accessibilityLabel(Words.text("window-panel", fallback: "Right panel"))
    }
}

extension PanelTabBar {
    /// "2 of 6": the tab's place in the list (its counts are in its own words, `spoken`).
    fileprivate func place(_ position: Int) -> String {
        Words.text("window-tab-position", ["position": .whole(position + 1), "total": .whole(tabs.count)], fallback: "\(position + 1) of \(tabs.count)")
    }
}

private struct PanelTabButton: View {
    let tab: PanelTab
    let isSelected: Bool
    let badge: Int?
    let tip: String
    let namespace: Namespace.ID
    let action: @MainActor () -> Void
    @State private var hovering = false
    private let run = IconRun()
    @Environment(\.chrome) private var colours
    @Environment(WindowFocus.self) private var focus

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
                .focusRing(isSelected && focus.ring(.panelTabs), radius: colours.radius)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .marked("icon.tab")
        .onHover { hovering = $0 }
        // The glyph plays as its tab becomes the active one, never as the window opens or the panel comes back.
        .onChange(of: isSelected) { _, now in if now { run.start() } }
        .help(tip)
    }

    @ViewBuilder private var glyph: some View {
        let colour = isSelected ? colours.selectedText : hovering ? colours.text : colours.muted
        Group {
            if let lucide = tab.lucide { TabGlyph(icon: lucide, size: Chrome.glyph, run: run.count).foregroundStyle(colour) }
        }
        .frame(width: Chrome.iconTab, height: Chrome.iconTab)
    }
}

extension PanelTab {
    /// The tabs in the GPUI app's order, which the model's own order (the enum's) is not.
    static let barOrder: [PanelTab] = [.changes, .history, .notes, .usage, .project, .files]

    var title: String { rawValue.capitalized }

    /// Lucide's git-compare, git-commit-horizontal, file-text, gauge, folder and folder-tree, as the GPUI app's tabs draw them.
    var lucide: Lucide? {
        switch self {
        case .changes: .gitCompare
        case .usage: .gauge
        case .project: .folder
        case .files: .folderTree
        case .notes: .fileText
        case .history: .commit
        }
    }
}

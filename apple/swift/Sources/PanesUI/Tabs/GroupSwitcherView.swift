// The popover the group switcher opens (RF-02 b): a search field ("Go to group or terminal…"), the groups
// as folded cards (their tiles and count, the open one ringed, a pinned empty one saying "No terminals"),
// "Ungrouped" with its count, New group and "Show all groups in the sidebar". Typing lists the groups and
// terminals that match and ↩ goes to the first.

import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel

struct GroupSwitcherView: View {
    let model: TabBarModel
    let actions: TabBarActions
    let dismiss: () -> Void
    @State private var query = ""
    @FocusState private var searching: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        let hits = TabSets.search(model.sets, query) { model.names[$0] ?? "" }
        VStack(spacing: TabMetrics.cardGap) {
            TextField(PanesText.switcherSearch, text: $query)
                .textFieldStyle(.roundedBorder).focused($searching)
                .onSubmit { hits.first.map(go) }
            if query.trimmingCharacters(in: .whitespaces).isEmpty {
                ForEach(model.cards) { card in
                    // A pinned group with nothing in it is somewhere to start a terminal: its card has a +.
                    SetCardView(card: card, open: { go(.set(card.id)) }, add: card.pinned && card.count == 0 ? addAction(card) : nil)
                        .nativeContextMenu { model.sets.indices.contains(card.id) ? model.sets[card.id].groupID.flatMap(actions.groupMenu) : nil }
                }
                Divider()
                footer
            } else if hits.isEmpty {
                Text(PanesText.noMatches).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    .frame(maxWidth: .infinity, minHeight: Metric.row, alignment: .leading).padding(.horizontal, Space._2)
            } else {
                ForEach(Array(hits.enumerated()), id: \.offset) { _, hit in hitRow(hit) }
            }
        }
        .padding(Space._2)
        .frame(width: TabMetrics.popoverWidth)
        .onAppear { searching = true }
    }

    private var footer: some View {
        let keys = actions.groupKeys()
        return VStack(alignment: .leading, spacing: 0) {
            footerButton("plus", PanesText.newGroup) { dismiss(); actions.newGroup() }
            footerButton("sidebar.left", PanesText.showGroupsInSidebar) { dismiss(); actions.showGroupsInSidebar() }
            if !keys.hint.isEmpty {
                HStack {
                    Text(keys.hint)
                    Spacer(minLength: Space._2)
                    Text(keys.keys)
                }
                .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                .padding(.horizontal, Space._2).padding(.top, Space._1)
            }
        }
    }

    private func footerButton(_ symbol: String, _ label: String, _ run: @escaping () -> Void) -> some View {
        Button(action: run) {
            Label(label, systemImage: symbol).font(TextRole.body.font).foregroundStyle(tokens.textPrimary)
                .frame(maxWidth: .infinity, minHeight: Metric.row, alignment: .leading).contentShape(Rectangle())
        }
        .buttonStyle(.plain).padding(.horizontal, Space._2)
    }

    private func hitRow(_ hit: TabSets.Hit) -> some View {
        let (label, note): (String, String) =
            switch hit {
            case .set(let at): (model.sets[at].label, PanesText.groupNote(count: model.sets[at].members.count))
            case .terminal(let id, let at): (model.names[id] ?? "", model.sets[at].label)
            }
        return Button { go(hit) } label: {
            HStack {
                Text(label).font(TextRole.body.font).foregroundStyle(tokens.textPrimary)
                Spacer(minLength: Space._2)
                Text(note).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            }
            .frame(minHeight: Metric.row).padding(.horizontal, Space._2).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    private func addAction(_ card: SetCard) -> (() -> Void)? {
        guard let group = model.sets[card.id].groupID else { return nil }
        return { dismiss(); actions.newTerminal(group) }
    }

    private func go(_ hit: TabSets.Hit) {
        dismiss()
        switch hit {
        case .set(let at): actions.openSet(at)
        case .terminal(let id, _): actions.select(id)
        }
    }
}

/// A group's card: its name, its terminals as tiles and their count, ringed in the primary while it is open.
struct SetCardView: View {
    let card: SetCard
    let open: () -> Void
    var add: (() -> Void)?
    @Environment(\.tokens) private var tokens

    /// How strong the open group's ring is: the primary at this share (`RING`).
    private static let ring = 0.6

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
        Button(action: open) {
            VStack(alignment: .leading, spacing: Space._1) {
                HStack {
                    Text(card.label).font(TextRole.label.font).foregroundStyle(tokens.textPrimary)
                    if card.alert { Circle().fill(tokens.agentAttention).frame(width: Metric.statusDot - 2, height: Metric.statusDot - 2) }
                    Spacer(minLength: Space._2)
                    if let add { IconButton(lucide: .plus, label: LocalizedStringKey(PanesText.newTerminalIn(card.label)), action: add) }
                    CountBadge(card.count)
                }
                if card.count == 0 {
                    Text(card.pinned ? PanesText.noTerminals : "").font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                } else {
                    HStack(spacing: 4) { ForEach(Array(card.activities.prefix(8).enumerated()), id: \.offset) { _, activity in TerminalTile(activity: activity, size: Metric.tileMini) } }
                }
            }
            .padding(Space._2).frame(maxWidth: .infinity, alignment: .leading)
            .background(tokens.card, in: shape)
            .overlay { shape.strokeBorder(card.current ? tokens.primary.opacity(Self.ring) : tokens.cardEdge, lineWidth: card.current ? tokens.metrics.focusRingWidth : tokens.metrics.borderWidth) }
            .contentShape(shape)
        }
        .buttonStyle(.plain)
        .accessibilityLabel(PanesText.cardLabel(card.label, count: card.count, current: card.current, alert: card.alert))
    }
}

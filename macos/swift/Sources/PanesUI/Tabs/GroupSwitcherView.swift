// The popover the group switcher opens (RF-02 b): a search field ("Go to group or terminal…"), the groups
// as folded cards (their tiles and count on the group's wash, the open one ringed, a pinned empty one saying "No terminals"),
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
                    SetCardView(card: card, set: model.sets[card.id], actions: actions, open: { go(.set(card.id)) }, add: card.pinned && card.count == 0 ? addAction(card) : nil)
                        .nativeContextMenu { model.sets[card.id].groupID.flatMap(actions.groupMenu) }
                }
                Divider()
                footer
            } else if hits.isEmpty {
                Text(PanesText.noMatches).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted)
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
                .font(tokens.font(.caption)).foregroundStyle(tokens.textMuted)
                .padding(.horizontal, Space._2).padding(.top, Space._1)
            }
        }
    }

    private func footerButton(_ symbol: String, _ label: String, _ run: @escaping () -> Void) -> some View {
        Button(action: run) {
            Label(label, systemImage: symbol).font(tokens.font(.body)).foregroundStyle(tokens.textPrimary)
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
                hitMark(hit)
                Text(label).font(tokens.font(.body)).foregroundStyle(tokens.textPrimary)
                Spacer(minLength: Space._2)
                Text(note).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted)
            }
            .frame(minHeight: Metric.row).padding(.horizontal, Space._2).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    /// A terminal's own tile before its name, a group's colour dot before its (`switcher.rs`).
    @ViewBuilder private func hitMark(_ hit: TabSets.Hit) -> some View {
        switch hit {
        case .terminal(let id, _):
            if let tile = actions.tile?(id, TabMetrics.tile) { tile }
        case .set(let at):
            let colour = model.sets[at].groupID.flatMap { actions.cardWash($0, tokens) }
            Circle().fill(colour?.opacity(1) ?? tokens.textFaint).frame(width: Metric.statusDot, height: Metric.statusDot)
        }
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

/// A group's card: its name (which opens the group's popover), its terminals as tiles, loudest first, and their count, on
/// the group's wash, ringed in the primary while it is open. A click anywhere else on it goes to the group.
struct SetCardView: View {
    let card: SetCard
    let set: TabSet
    let actions: TabBarActions
    let open: () -> Void
    var add: (() -> Void)?
    @State private var editing = false
    @Environment(\.tokens) private var tokens

    /// How strong the open group's ring is: the primary at this share (`RING`).
    private static let ring = 0.6
    /// The most tiles a card shows.
    private static let tiles = 8

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
        VStack(alignment: .leading, spacing: Space._1) {
            HStack {
                name
                if card.alert { Circle().fill(card.alertKind == .failed ? tokens.statusDanger : tokens.agentAttention).frame(width: Metric.statusDot - 2, height: Metric.statusDot - 2) }
                Spacer(minLength: Space._2)
                if let add { IconButton(lucide: .plus, label: PanesText.newTerminalIn(card.label), action: add) }
                CountBadge(card.count)
            }
            if card.count == 0 {
                Text(card.pinned ? PanesText.noTerminals : "").font(tokens.font(.caption)).foregroundStyle(tokens.textMuted)
            } else {
                HStack(spacing: 4) { memberTiles }
            }
        }
        .padding(Space._2).frame(maxWidth: .infinity, alignment: .leading)
        .background {
            shape.fill(tokens.card)
            if let wash = set.groupID.flatMap({ actions.cardWash($0, tokens) }) { shape.fill(wash) }
        }
        .overlay { shape.strokeBorder(card.current ? tokens.primary.opacity(Self.ring) : tokens.cardEdge, lineWidth: card.current ? tokens.metrics.focusRingWidth : tokens.metrics.borderWidth) }
        .contentShape(shape)
        .onTapGesture(perform: open)
        .onChange(of: actions.renameAsk?.count) { if let group = set.groupID, group == actions.renameAsk?.group { editing = true } }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(PanesText.cardLabel(card.label, count: card.count, current: card.current, alert: card.alert))
        .accessibilityAddTraits(.isButton)
        .accessibilityAction(.default, open)
        .accessibilityActions {
            if let add { Button(PanesText.newTerminalIn(card.label), action: add) }
            if set.groupID != nil, actions.groupPopover != nil { Button(PanesText.editGroup) { editing = true } }
        }
    }

    /// The name opens the group's popover under it (`switcher.rs` `on_label`); "Ungrouped" has none.
    @ViewBuilder private var name: some View {
        let label = Text(card.label).font(tokens.font(.label)).foregroundStyle(tokens.textPrimary).lineLimit(1)
        if let group = set.groupID, let popover = actions.groupPopover {
            label.contentShape(Rectangle())
                .onTapGesture { editing = true }
                .popover(isPresented: $editing, arrowEdge: .bottom) { popover(group, { editing = false }) }
        } else {
            label
        }
    }

    /// The members' tiles as the list's rows draw them, loudest first; the plain tiles where the window gives none.
    @ViewBuilder private var memberTiles: some View {
        if let tile = actions.tile {
            ForEach(Array(actions.loudest(set.members).prefix(Self.tiles)), id: \.self) { id in if let mini = tile(id, Metric.tileMini) { mini } }
        } else {
            ForEach(Array(card.activities.prefix(Self.tiles).enumerated()), id: \.offset) { _, activity in TerminalTile(activity: activity, size: Metric.tileMini) }
        }
    }
}

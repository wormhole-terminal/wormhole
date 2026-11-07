// The picker in a card's head that Changes ("compare with") and History (which branch's log) each have
// (`features/git/picker.rs`): a chip for what is chosen, and under it a popover with a search box over the branches,
// tags and commits the repository lists, and the stashes in History. Return picks the first match, esc closes.
// Closed, it is `select::SelectTrigger::compact`: 24 tall, caption text in 500, a `border.strong` outline, the muted
// prefix ("vs"), the branch mark in `git.branch` and the name (HEAD when none), then the up/down chevrons.

import DesignSystem
import PanelModel
import SwiftUI

struct RevPicker: View {
    var prefix = ""
    let selected: Rev?
    let revs: [Rev]
    var headTitle = "HEAD"
    let load: () -> Void
    let pick: (Rev?) -> Void
    @State private var open = false
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button {
            open.toggle()
            if open { load() }
        } label: { chip }
            .buttonStyle(.plain)
            .onHover { hovering = $0 }
            .popover(isPresented: $open, arrowEdge: .bottom) {
                RevList(selected: selected, revs: revs, headTitle: headTitle) { rev in
                    open = false
                    pick(rev)
                }
            }
            .accessibilityLabel("\(prefix) \(selected?.title ?? headTitle)")
            .accessibilityHint(panelText("picker-hint", "Opens the list of branches"))
    }

    private var chip: some View {
        HStack(spacing: Space._1) {
            if !prefix.isEmpty { Text(prefix).foregroundStyle(tokens.textMuted) }
            if selected != nil { Image(systemName: "arrow.triangle.branch").font(.system(size: 12)).foregroundStyle(tokens.gitBranch) }
            Text(selected?.title ?? headTitle).lineLimit(1).truncationMode(.tail).foregroundStyle(tokens.textPrimary)
            Image(systemName: "chevron.up.chevron.down").font(.system(size: 10, weight: .medium)).foregroundStyle(tokens.textMuted)
        }
        .font(TextRole.caption.font.weight(.medium))
        .padding(.leading, Space._2).padding(.trailing, Space._1)
        .frame(height: 24).frame(maxWidth: 160)
        .background(hovering ? tokens.stateHover : .clear, in: shape)
        .overlay { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
        .contentShape(shape)
        .fixedSize()
    }

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: tokens.metrics.radius, style: .continuous) }
}

/// The popover's content: the search box and the list under it, grouped by what each ref is.
private struct RevList: View {
    let selected: Rev?
    let revs: [Rev]
    let headTitle: String
    let choose: (Rev?) -> Void
    @State private var query = ""
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    private var matches: [Rev] {
        query.isEmpty ? revs : revs.filter { $0.name.localizedCaseInsensitiveContains(query) || $0.detail.localizedCaseInsensitiveContains(query) }
    }

    var body: some View {
        VStack(spacing: 0) {
            TextField(panelText("picker-search", "Search branches"), text: $query)
                .textFieldStyle(.plain).font(TextRole.body.font).focused($focused)
                .padding(.horizontal, Space._3).frame(height: Metric.control)
                .onSubmit { if let first = matches.first { choose(first) } else if query.isEmpty { choose(nil) } }
            Rectangle().fill(tokens.divider).frame(height: tokens.metrics.dividerWidth)
            ScrollView {
                LazyVStack(spacing: 0) {
                    if query.isEmpty { item(title: headTitle, detail: "", symbol: "scope", checked: selected == nil) { choose(nil) } }
                    ForEach(matches) { rev in
                        item(title: rev.title, detail: rev.detail, symbol: symbol(rev.kind), checked: rev == selected) { choose(rev) }
                    }
                    if matches.isEmpty && !query.isEmpty { EmptyNote(text: panelText("picker-none", "Nothing matches.")) }
                }
                .padding(Space._1)
            }
        }
        .frame(width: 260, height: min(CGFloat(matches.count + 1) * Metric.row + Metric.control + 12, 320))
        .background(tokens.surfaceOverlay)
        .onAppear { focused = true }
        .onExitCommand { choose(selected) }
    }

    private func symbol(_ kind: Rev.Kind) -> String {
        switch kind {
        case .branch: "arrow.triangle.branch"
        case .tag: "tag"
        case .commit: "smallcircle.filled.circle"
        case .stashes: "tray.full"
        }
    }

    private func item(title: String, detail: String, symbol: String, checked: Bool, action: @escaping () -> Void) -> some View {
        PanelRow(action: action) { _ in
            HStack(spacing: PanelMetrics.gap) {
                Image(systemName: symbol).font(.system(size: 12)).foregroundStyle(tokens.textMuted).frame(width: Metric.lead)
                Text(title).font(TextRole.body.font).foregroundStyle(tokens.textPrimary).lineLimit(1)
                if !detail.isEmpty { Text(detail).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).lineLimit(1) }
                Spacer(minLength: 0)
                if checked { Image(systemName: "checkmark").font(.system(size: 11, weight: .semibold)).foregroundStyle(tokens.primary) }
            }
            .padding(.horizontal, Space._2).frame(minHeight: Metric.row)
        }
    }
}

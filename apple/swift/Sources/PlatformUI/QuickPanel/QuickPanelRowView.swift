// A result of the quick panel (`style::palette_result`): edge to edge, the lead in the lead column, the name, the
// words after it (in the state's colour for a waiting, finished or failed terminal), then the time or the key cap
// last. The selected row takes `list.selected` and semibold; one under the pointer takes `state.hover`.

import DesignSystem
import PlatformModel
import SwiftUI

struct QuickPanelRowView: View {
    let row: QuickPanelRow
    let isSelected: Bool
    let isHovered: Bool
    let selection: Namespace.ID
    let keymap: Keymap
    let now: Date
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: Space._2) {
            lead.frame(width: Metric.lead, height: Metric.lead)
            Text(title)
                .font(.system(size: TextRole.body.size, weight: isSelected ? .semibold : .medium))
                .lineLimit(1)
                .layoutPriority(1)
            if let detail {
                Text(detail)
                    .font(.system(size: TextRole.caption.size, design: plainCommand ? .monospaced : .default))
                    .foregroundStyle(detailColor)
                    .lineLimit(1)
            }
            Spacer(minLength: 0)
            trailing
        }
        .padding(.horizontal, Space._4)
        .frame(height: Metric.paletteRowHeight)
        .foregroundStyle(isSelected ? tokens.listSelectedText : tokens.textPrimary)
        .background {
            if isSelected {
                Rectangle().fill(tokens.listSelected).matchedGeometryEffect(id: "selection", in: selection)
            } else if isHovered {
                Rectangle().fill(tokens.stateHover)
            }
        }
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityLabel(Text(spoken))
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }

    private var terminal: AmbientTerminal? { if case .terminal(let terminal) = row { terminal } else { nil } }

    private var title: String {
        switch row {
        case .terminal(let terminal): terminal.name
        case .action(let item): item.title
        case .label(let title, _): title
        case .note(let text): text
        }
    }

    private var detail: String? {
        switch row {
        case .terminal(let terminal): terminal.detail(withAgent: false)
        case .action(let item): item.detail
        default: nil
        }
    }

    /// A command line, running or failed, reads in the code font; a program's words don't.
    private var plainCommand: Bool {
        guard let terminal else { return false }
        return !terminal.source.told && (terminal.state == .working || terminal.state == .failed)
    }

    private var detailColor: Color {
        switch terminal?.state {
        case .needsYou: tokens.agentAttentionText
        case .done: tokens.agentDoneText
        case .failed: tokens.statusDangerText
        default: isSelected ? tokens.listSelectedMuted : tokens.textMuted
        }
    }

    @ViewBuilder private var lead: some View {
        if let terminal {
            if terminal.isBell {
                Image(systemName: "bell.fill").font(.system(size: Metric.glyph - 3)).foregroundStyle(tokens.agentAttention)
            } else {
                StatusDot(StatusKind(terminal.state))
            }
        } else if case .action(let item) = row {
            Image(systemName: Self.symbol(item)).font(.system(size: Metric.glyph - 1, weight: .light))
        }
    }

    @ViewBuilder private var trailing: some View {
        if let terminal, let time = AmbientWording.elapsed(terminal.state, since: max(0, now.timeIntervalSince(terminal.since))) {
            Text(time).font(.system(size: TextRole.caption.size)).monospacedDigit().foregroundStyle(tokens.textFaint)
        } else if case .action(let item) = row, let keys = item.keys(keymap) {
            KeyCapView(text: keys)
        }
    }

    private var spoken: String {
        [title, detail, terminal.flatMap { AmbientWording.elapsed($0.state, since: max(0, now.timeIntervalSince($0.since))) }]
            .compactMap { $0 }.joined(separator: ", ")
    }

    private static func symbol(_ item: QuickPanelDo) -> String {
        switch item {
        case .newTerminal: "plus"
        case .openFolder: "folder"
        case .action: "play"
        }
    }
}

extension StatusKind {
    init(_ state: AmbientState) {
        self =
            switch state {
            case .idle, .exited: .idle
            case .working: .working
            case .needsYou: .attention
            case .done: .done
            case .failed: .failed
            }
    }
}

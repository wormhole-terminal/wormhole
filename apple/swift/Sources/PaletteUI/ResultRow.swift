// One result (`style::palette_result`): edge to edge, the lead in the lead column, the title, the detail after
// it, what sits at the right last. The selected one takes `list.selected` with its title in semibold and
// `list.selected.text`, its detail in `list.selected.muted`; one under the pointer takes `state.hover`.

import DesignSystem
import PaletteModel
import SwiftUI
import WorkspaceModel

struct ResultRow: View {
    let result: PaletteResult
    let scope: PaletteScope
    let isSelected: Bool
    let isHovered: Bool
    let selection: Namespace.ID
    let motion: OverlayMotion
    @Environment(\.tokens) private var tokens
    @Environment(\.codeFont) private var codeFont

    private var command: Command { result.command }
    private var detail: String? { scope == .terminals ? command.scopedDetail ?? command.detail : command.detail }

    private var muted: Color { isSelected ? tokens.listSelectedMuted : tokens.textMuted }
    /// The agent's question, under the name, in the Terminals scope.
    private var second: String? { result.hasSecondLine(in: scope) ? command.second : nil }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: Space._2) {
                lead
                title
                if let detail {
                    Text(detail)
                        .font(command.detailIsCode ? codeFont.font(TextRole.caption.size) : .system(size: TextRole.caption.size))
                        .foregroundStyle(muted)
                        .lineLimit(1)
                        .layoutPriority(0)
                }
                Spacer(minLength: 0)
                if let trailing = command.trailing {
                    Text(trailing)
                        .font(command.trailingIsCode ? codeFont.font(TextRole.caption.size) : .system(size: TextRole.caption.size))
                        .foregroundStyle(muted).lineLimit(1)
                }
                // A setting says what ↩ does on the row that is chosen.
                if isSelected, case .setting? = command.target { KeyCap(text: "↩") }
                if let shortcut = command.shortcut { KeyCap(text: shortcut.glyphs) }
            }
            if let second {
                Text(second).font(.system(size: TextRole.caption.size)).foregroundStyle(muted).lineLimit(1)
                    .padding(.leading, Metric.lead + Space._2)
            }
        }
        .padding(.horizontal, Space._4)
        .frame(height: second == nil ? Metric.paletteRowHeight : Metric.paletteRowTwoLines)
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
        .accessibilityLabel(Text([command.title, detail, second, spokenKey].compactMap { $0 }.joined(separator: ", ")))
        .accessibilityAddTraits(isSelected ? [.isButton, .isSelected] : .isButton)
    }

    private var title: some View {
        let weight: Font.Weight = isSelected ? .semibold : command.isStrong ? .medium : .regular
        let size = TextRole.body.size
        let mark = isSelected ? tokens.listSelectedText : tokens.primary
        return Text(MarkedText.title(command.title, ranges: result.ranges, mark: mark, size: size))
            .font(.system(size: size, weight: weight))
            .lineLimit(1)
            .layoutPriority(1)
    }

    @ViewBuilder private var lead: some View {
        Group {
            switch command.lead {
            case .symbol(let name):
                Image(systemName: name).font(.system(size: Metric.glyph - 1, weight: .light)).accessibilityHidden(true)
            case .status(let activity):
                StatusDot(StatusKind(activity))
            }
        }
        .frame(width: Metric.lead, height: Metric.lead)
    }

    private var spokenKey: String? { command.shortcut.map { KeyCapSpeech.words($0.glyphs) } }
}

extension StatusKind {
    init(_ activity: Activity) {
        self =
            switch activity {
            case .idle, .exited: .idle
            case .working: .working
            case .needsInput: .attention
            case .finished: .done
            case .failed: .failed
            }
    }
}

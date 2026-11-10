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

    private var command: Command { result.command }
    private var detail: String? { scope == .terminals ? command.scopedDetail ?? command.detail : command.detail }

    var body: some View {
        HStack(spacing: Space._2) {
            lead
            title
            if let detail {
                Text(detail)
                    .font(.system(size: TextRole.caption.size))
                    .foregroundStyle(isSelected ? tokens.listSelectedMuted : tokens.textMuted)
                    .lineLimit(1)
                    .layoutPriority(0)
            }
            Spacer(minLength: 0)
            if let shortcut = command.shortcut { KeyCap(text: shortcut.glyphs) }
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
        .accessibilityLabel(Text([command.title, command.detail, spokenKey].compactMap { $0 }.joined(separator: ", ")))
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

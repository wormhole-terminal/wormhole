// The quick panel's content (`status_item/quick_panel/view.rs`): the search row, the rows under their labels,
// Reply in place under a waiting row, and the footer's key hints, on the palette's card. It shows
// `QuickPanelModel` and reports nothing but intents to it; the keys come from the search field, which takes
// ↑ ↓ ⏎ and Esc before the field does.

import DesignSystem
import PlatformModel
import SwiftUI

public struct QuickPanelView: View {
    let model: QuickPanelModel
    var keymap: Keymap = .shared
    @Environment(\.tokens) private var tokens
    @Namespace private var selection
    @State private var hovered: String?

    public init(model: QuickPanelModel, keymap: Keymap = .shared) {
        self.model = model
        self.keymap = keymap
    }

    public var body: some View {
        VStack(spacing: 0) {
            search
            list
            footer
        }
        .frame(width: Metric.paletteWidth, height: model.height)
        .panelCard()
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Text("Quick panel"))
    }

    private var search: some View {
        HStack(spacing: Space._2) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: Metric.titleGlyph, weight: .light))
                .foregroundStyle(tokens.textMuted)
                .accessibilityHidden(true)
            PanelField(
                text: Binding(get: { model.query }, set: { model.query = $0 }), placeholder: "Search terminals and actions",
                size: TextRole.heading.size, textColor: tokens.textPrimary, placeholderColor: tokens.inputPlaceholder,
                caretColor: tokens.textPrimary, onKey: onKey)
            if let hotkey = model.hotkey { KeyCap(text: hotkey) }
        }
        .padding(.horizontal, Space._4)
        .frame(height: Metric.paletteSearchHeight)
        .overlay(alignment: .bottom) { PanelRule() }
    }

    private var list: some View {
        let rows = model.rows, selected = model.selectedIndex
        return ScrollView {
            LazyVStack(spacing: 0) {
                ForEach(Array(rows.enumerated()), id: \.element.id) { index, row in
                    line(row, selected: index == selected)
                    if let answering = model.answering, row.waiting?.tab == answering.tab { answer(answering) }
                }
            }
            .padding(.top, QuickPanelMetrics.listTop)
        }
        .scrollContentBackground(.hidden)
        .frame(maxHeight: .infinity)
    }

    @ViewBuilder private func line(_ row: QuickPanelRow, selected: Bool) -> some View {
        switch row {
        case .label(let title, let count): GroupLabelView(title: title, count: count)
        case .note(let text):
            Text(text)
                .font(.system(size: TextRole.body.size))
                .foregroundStyle(tokens.textMuted)
                .padding(.horizontal, Space._4)
                .frame(maxWidth: .infinity, minHeight: QuickPanelMetrics.rowHeight, alignment: .leading)
        case .terminal, .action:
            QuickPanelRowView(
                row: row, isSelected: selected, isHovered: hovered == row.id, selection: selection, keymap: keymap, now: model.now)
                .onHover { hovered = $0 ? row.id : (hovered == row.id ? nil : hovered) }
                .onTapGesture { model.select(row); model.run(row) }
                .motion(Motion.selection, value: selected)
        }
    }

    /// Under a waiting row while answering: what the agent asked and the reply field.
    private func answer(_ answering: QuickPanelModel.Answering) -> some View {
        VStack(alignment: .leading, spacing: Space._2) {
            Text(answering.question)
                .font(.system(size: TextRole.caption.size))
                .foregroundStyle(tokens.listSelectedMuted)
                .lineLimit(3)
            PanelField(
                text: Binding(get: { model.reply }, set: { model.reply = $0 }), placeholder: "Reply", size: TextRole.body.size,
                textColor: tokens.textPrimary, placeholderColor: tokens.inputPlaceholder, caretColor: tokens.textPrimary, onKey: onKey)
                .padding(.horizontal, Space._2)
                .frame(height: Metric.control)
                .background(tokens.surfaceElevated, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
                .overlay {
                    RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth)
                }
        }
        .padding(.vertical, Space._3)
        .padding(.trailing, Space._4)
        .padding(.leading, QuickPanelMetrics.answerIndent)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(tokens.listSelected)
        .transition(.opacity)
    }

    private var footer: some View {
        HStack(spacing: Space._4) {
            ForEach(model.footerHints, id: \.keys) { KeyHintView(keys: $0.keys, label: $0.label) }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, Space._4)
        .frame(height: Metric.paletteRowHeight)
        .overlay(alignment: .top) { PanelRule() }
    }

    private func onKey(_ key: PanelKey) -> Bool {
        switch key {
        case .escape:
            if model.answering == nil { model.close() } else { model.stopAnswering() }
        case .up where model.answering == nil: model.step(-1)
        case .down where model.answering == nil: model.step(1)
        case .enter: model.answering == nil ? model.run() : model.sendReply()
        default: return false
        }
        return true
    }
}

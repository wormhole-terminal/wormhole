// The folder picker (⌘J) as the GPUI app's native panel looks: a breadcrumb of where the person is, the field with
// its grey ghost completion, the rows like a menu's, the notes the list says, and the hint line under it, on the
// system's glass (an opaque elevated surface under Reduce Transparency). What a key does is `PickerKeys.decide`
// through the driver; this only shows the model and reports clicks.

import DesignSystem
import PlatformModel
import SwiftUI

public struct FolderPickerView: View {
    let driver: FolderPickerDriver
    @Environment(\.tokens) private var tokens
    @State private var hovered: String?

    public init(driver: FolderPickerDriver) { self.driver = driver }

    private var model: PickerModel { driver.model }

    /// How many lines of the list are in view: all of them up to the most the panel shows, and one for "No folders".
    static func rowCount(_ model: PickerModel) -> Int {
        min(max(model.lines.count + model.answer.notes.count, 1), FolderPickerMetrics.maxRows)
    }

    public var body: some View {
        VStack(spacing: 0) {
            if !model.answer.crumbs.isEmpty { crumbs }
            field
            list
            Text(PickerWording.hint)
                .font(.system(size: TextRole.caption.size))
                .foregroundStyle(tokens.textMuted)
                .frame(maxWidth: .infinity, minHeight: FolderPickerMetrics.hint, alignment: .leading)
                .padding(.horizontal, Space._3)
                .accessibilityHidden(true)
        }
        .padding(.vertical, Space._2)
        .frame(width: FolderPickerMetrics.width)
        .background { surface }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Text(PickerWording.name))
    }

    @ViewBuilder private var surface: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
        // The theme store follows the system's Reduce Transparency, so a snapshot can set it too.
        if ThemeStore.shared.reduceTransparency {
            shape.fill(tokens.surfaceElevated).overlay { shape.strokeBorder(tokens.borderStrong, lineWidth: tokens.metrics.borderWidth) }
        } else {
            shape.fill(.clear).glassEffect(.regular, in: shape)
        }
    }

    private var crumbs: some View {
        HStack(spacing: Space._1) {
            ForEach(Array(model.answer.crumbs.enumerated()), id: \.offset) { index, crumb in
                if index > 0 { Image(systemName: "chevron.right").font(.system(size: Metric.chevron)).foregroundStyle(tokens.textFaint) }
                Button(crumb.label) { driver.click(crumb: index) }
                    .buttonStyle(.plain)
                    .font(.system(size: TextRole.caption.size, weight: index == model.answer.crumbs.count - 1 ? .semibold : .regular))
                    .foregroundStyle(index == model.answer.crumbs.count - 1 ? tokens.textPrimary : tokens.textMuted)
                    .lineLimit(1)
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, Space._3)
        .frame(height: FolderPickerMetrics.crumbs)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Text("Where you are"))
    }

    private var field: some View {
        HStack(spacing: Space._2) {
            Image(systemName: "magnifyingglass").font(.system(size: Metric.glyph, weight: .light)).foregroundStyle(tokens.textMuted).accessibilityHidden(true)
            PanelField(
                text: Binding(get: { model.text }, set: { driver.typed($0) }), placeholder: PickerWording.search,
                size: TextRole.body.size, textColor: tokens.textPrimary, placeholderColor: tokens.inputPlaceholder,
                caretColor: tokens.textPrimary, ghost: model.ghost, ghostColor: tokens.textFaint,
                onSelection: { driver.field = PickerField(text: model.text, caret: $0, selected: $1) },
                onKey: { driver.key(Self.key($0)) })
        }
        .padding(.horizontal, Space._3)
        .frame(height: FolderPickerMetrics.field)
    }

    private var list: some View {
        let lines = model.lines
        return ScrollViewReader { proxy in
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(Array(lines.enumerated()), id: \.offset) { _, line in
                        switch line {
                        case .header(let title): GroupLabelView(title: title, count: nil)
                        case .row(let row):
                            FolderPickerRowView(row: row, isSelected: row.key == model.selectedKey, isHovered: hovered == row.key)
                                .id(row.key)
                                .onHover { hovered = $0 ? row.key : (hovered == row.key ? nil : hovered) }
                                .onTapGesture { driver.click(row.key) }
                        case .note(let text): noteLine(text)
                        }
                    }
                    ForEach(Array(model.answer.notes.enumerated()), id: \.offset) { _, note in noteLine(PickerWording.note(note)) }
                }
                .padding(.horizontal, Space._1)
            }
            .scrollContentBackground(.hidden)
            .frame(height: CGFloat(Self.rowCount(model)) * FolderPickerMetrics.row)
            .onChange(of: model.selectedKey) { _, key in if let key { proxy.scrollTo(key) } }
        }
    }

    private func noteLine(_ text: String) -> some View {
        Text(text)
            .font(.system(size: TextRole.caption.size))
            .foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._3)
            .frame(maxWidth: .infinity, minHeight: FolderPickerMetrics.row, alignment: .leading)
    }

    private static func key(_ key: PanelKey) -> PickerKey {
        switch key {
        case .up: .up
        case .down: .down
        case .pageUp: .pageUp
        case .pageDown: .pageDown
        case .enter: .enter
        case .tab: .tab
        case .escape: .escape
        case .left: .left
        case .right: .right
        case .commandDelete: .forget
        }
    }
}

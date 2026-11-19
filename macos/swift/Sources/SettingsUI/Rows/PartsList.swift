// Appearance → Title row: the row's parts in the order they stand, each with its switch, and under them the parts it
// does not show (`title_row.rs`). The list stored is the shown parts in order; a shown part is dragged to move it (or
// moved by VoiceOver's actions), and a hidden one, switched on, comes back at its default place.

import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct PartsList: View {
    let model: SettingsModel
    let key: String
    let parts: [ControlSpec.TitlePart]
    /// The part under the pointer: its row takes the hover fill (`arrange_row`).
    @State private var hovered: String?
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shown = (model.value(key).array ?? []).compactMap(\.string).compactMap { value in parts.first { $0.value == value } }
        let hidden = parts.filter { part in !shown.contains { $0.value == part.value } }
        VStack(spacing: 0) {
            ForEach(Array(shown.enumerated()), id: \.element.value) { index, part in row(part, shown: true, index: index, count: shown.count) }
            if !hidden.isEmpty {
                Text(Words.text("title-row-hidden", fallback: "Hidden")).textRole(.caption, weight: .semibold)
                    .foregroundStyle(tokens.textMuted).frame(maxWidth: .infinity, alignment: .leading).padding(.top, Space._3)
                    .accessibilityAddTraits(.isHeader)
                ForEach(hidden, id: \.value) { row($0, shown: false, index: 0, count: 0) }
            }
        }
        .frame(width: SettingsMetric.controlWidth + 80)
    }

    private func row(_ part: ControlSpec.TitlePart, shown: Bool, index: Int, count: Int) -> some View {
        HStack(spacing: Space._2) {
            if shown { Image(Icon.grip).foregroundStyle(tokens.textFaint).accessibilityHidden(true) }
            VStack(alignment: .leading, spacing: 0) {
                Text(part.label).textRole(.label).foregroundStyle(shown ? tokens.textPrimary : tokens.textFaint)
                if let note = note(part) { Text(note).textRole(.caption).foregroundStyle(tokens.textMuted) }
            }
            Spacer()
            if part.hideable { SettingToggle(label: part.label, isOn: shown) { toggle(part, on: $0) } }
        }
        .frame(minHeight: Metric.control)
        .background(hovered == part.value ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .contentShape(Rectangle())
        .onHover { inside in
            if inside { hovered = part.value } else if hovered == part.value { hovered = nil }
        }
        .draggable(shown ? part.value : "") { Text(part.label).padding(Space._2).background(tokens.surfaceOverlay) }
        .dropDestination(for: String.self) { items, _ in
            guard shown, let from = items.first.flatMap(order.firstIndex(of:)) else { return false }
            move(from: from, to: index)
            return true
        }
        .accessibilityElement(children: .combine)
        .accessibilityAction(named: Words.text("title-row-move-up", fallback: "Move up")) { if shown, index > 0 { move(from: index, to: index - 1) } }
        .accessibilityAction(named: Words.text("title-row-move-down", fallback: "Move down")) { if shown, index < count - 1 { move(from: index, to: index + 1) } }
    }

    /// What a part says about itself: what the flexible space does, and that the status bar shows the terminal's status while on.
    private func note(_ part: ControlSpec.TitlePart) -> String? { Self.note(part, model) }

    /// The same, for the control folder's `settings-page`.
    static func note(_ part: ControlSpec.TitlePart, _ model: SettingsModel) -> String? {
        switch part.value {
        case "space": Words.text("title-row-part-space.description", fallback: "Parts above it sit at the left, parts below it at the right")
        case "terminal_status" where model.value("status_bar").bool ?? false:
            Words.text("title-row-status-in-bar", fallback: "The status bar shows it while it's on")
        default: nil
        }
    }

    /// The stored parts, in order.
    private var order: [String] { (model.value(key).array ?? []).compactMap(\.string) }

    private func move(from: Int, to: Int) {
        var next = order
        guard next.indices.contains(from), next.indices.contains(to), from != to else { return }
        next.insert(next.remove(at: from), at: to)
        model.set(key, .array(next.map(JSONValue.string)))
    }

    /// A part switched off leaves the list; one switched on takes its default place among the shown ones.
    private func toggle(_ part: ControlSpec.TitlePart, on: Bool) {
        // Core's rule (`Settings::show_title_part`) where there is a core; the same place worked out here where there is none.
        if model.host.showTitlePart(part.value, shown: on) { return }
        var shown = order
        if on {
            // After the nearest shown part that comes before it by default, else first (`TitleRow::show`).
            let place = parts.firstIndex { $0.value == part.value } ?? 0
            let at = parts[..<place].reversed().lazy.compactMap { before in shown.firstIndex(of: before.value) }.first.map { $0 + 1 } ?? 0
            shown.insert(part.value, at: at)
        } else {
            shown.removeAll { $0 == part.value }
        }
        model.set(key, .array(shown.map(JSONValue.string)))
    }
}

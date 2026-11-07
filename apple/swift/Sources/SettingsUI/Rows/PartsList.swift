// Appearance → Title row: the row's parts with a switch each (`title_row.rs`). The list stored is the shown
// parts in order, the rest hidden; a part switched on comes back at its default place. Dragging to reorder
// is not built yet.

import DesignSystem
import SettingsModel
import SwiftUI

struct PartsList: View {
    let model: SettingsModel
    let key: String
    let parts: [ControlSpec.TitlePart]
    @Environment(\.tokens) private var tokens

    var body: some View {
        let shown = Set((model.value(key).array ?? []).compactMap(\.string))
        VStack(spacing: 0) {
            ForEach(parts, id: \.value) { part in
                HStack {
                    Text(part.label).font(.system(size: TextRole.label.size))
                        .foregroundStyle(shown.contains(part.value) ? tokens.textPrimary : tokens.textFaint)
                    Spacer()
                    if part.hideable {
                        FlatSwitch(isOn: shown.contains(part.value), label: part.label) { on in toggle(part, on: on) }
                    }
                }
                .frame(minHeight: Metric.control)
            }
        }
        .frame(width: SettingsMetric.controlWidth + 80)
    }

    /// A part switched off leaves the list; one switched on takes its default place among the shown ones.
    private func toggle(_ part: ControlSpec.TitlePart, on: Bool) {
        let order = parts.map(\.value)
        var shown = (model.value(key).array ?? []).compactMap(\.string)
        if on {
            shown.append(part.value)
            shown.sort { (order.firstIndex(of: $0) ?? 0) < (order.firstIndex(of: $1) ?? 0) }
        } else {
            shown.removeAll { $0 == part.value }
        }
        model.set(key, .array(shown.map(JSONValue.string)))
    }
}

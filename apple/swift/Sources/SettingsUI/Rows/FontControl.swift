// A font row (`font_picker.rs`): the family in a select that opens the font picker (an empty choice follows another
// slot and says so in its face) and the size as a stepper, in whole points within the slot's range. The families, their
// order and the user's font files are the library's (core's rules); the value is the settings' alone.

import DesignSystem
import FontPickerModel
import FontPickerUI
import SettingsModel
import SwiftUI

struct FontControl: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry
    let slot: ControlSpec.FontSlot
    @Environment(\.fontLibrary) private var library

    var body: some View {
        if let library { FontRow(model: model, entry: entry, slot: slot, library: library) }
    }
}

private struct FontRow: View {
    let model: SettingsModel
    let entry: SettingsSchema.Entry
    let slot: ControlSpec.FontSlot
    @State private var picker: FontPickerModel

    init(model: SettingsModel, entry: SettingsSchema.Entry, slot: ControlSpec.FontSlot, library: FontLibrary) {
        (self.model, self.entry, self.slot) = (model, entry, slot)
        let family = entry.keys[0]
        _picker = State(
            initialValue: FontPickerModel(
                slot: FontPickerSlot(key: slot.slot, inheritLabel: slot.inherit, sizes: slot.min...slot.max),
                library: library, current: { model.value(family).string ?? "" },
                inherited: { Self.inherited(slot.slot, model) }, pick: { model.set(family, .string($0)) }))
    }

    var body: some View {
        let size = model.value(entry.keys[1]).number ?? slot.min
        HStack(spacing: Space._3) {
            FontPicker(model: picker, look: look(size: size)) { FontSelectFace(picker: picker, resolved: Self.inherited(slot.slot, model)) }
                .accessibilityLabel(entry.label)
            ValueStepper(
                value: "\(Int(size)) pt", label: entry.label,
                decrease: size > slot.min ? { model.set(entry.keys[1], .number(max(slot.min, size - 1))) } : nil,
                increase: size < slot.max ? { model.set(entry.keys[1], .number(min(slot.max, size + 1))) } : nil)
        }
        // The page's scene shows the font being looked at in the open picker, and goes back when it closes.
        .onChange(of: picker.isOpen ? picker.previewFamily : nil) { _, family in
            model.fontPreview = family.map { FontPreview(family: $0, size: size) }
        }
    }

    private func look(size: Double) -> PreviewLook {
        PreviewLook(
            size: size, lineHeight: slot.slot == "terminal" ? model.value("line_height").number : nil,
            ligatures: model.value("font_ligatures").bool ?? true)
    }

    /// What an empty family comes to: the code font is the terminal's, the interface font the system's.
    private static func inherited(_ slot: String, _ model: SettingsModel) -> String {
        slot == "code" ? model.value("font_family").string ?? "" : FontFace.system
    }
}

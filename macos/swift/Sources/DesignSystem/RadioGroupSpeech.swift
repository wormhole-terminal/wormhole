// Choices drawn as pictures (theme cards, folder colours) heard as what they are: a radio group of radio buttons, the
// chosen one selected (`choice_card`, `color_choice`: `Role::RadioButton`, toggled while chosen). SwiftUI gives a drawn
// button no radio role, so a real radio-group picker stands in for the drawn choices, as `Segmented` does with a segmented
// one, and pressing a radio picks its choice. The drawn choices keep the pointer and the keyboard.

import SwiftUI

extension View {
    /// VoiceOver hears this view as a radio group named `label`, a radio button for each title, the one at `selected` chosen
    /// (nil: none is). `pick` gets the index of the radio pressed; while not `enabled` the group is off.
    public func radioGroupSpeech(
        _ label: String, titles: [String], selected: Int?, enabled: Bool = true, pick: @escaping (Int) -> Void
    ) -> some View {
        accessibilityRepresentation {
            Picker(label, selection: Binding(get: { selected }, set: { if let index = $0 { pick(index) } })) {
                ForEach(titles.indices, id: \.self) { index in Text(titles[index]).tag(Optional(index)) }
            }
            .pickerStyle(.radioGroup)
            .disabled(!enabled)
        }
    }
}

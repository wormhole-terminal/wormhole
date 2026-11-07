// A font picker: whatever the caller draws as the control (its select, in the settings row) opens a popover with
// the search, the families and the preview (`FontPickerContent`). The popover is the system's, so Esc, a click
// outside and VoiceOver's escape close it as they close any popover; typing goes straight into the search.

import DesignSystem
import FontPickerModel
import SwiftUI

public struct FontPicker<Face: View>: View {
    @Bindable var model: FontPickerModel
    let look: PreviewLook
    let face: Face
    @Environment(\.tokens) private var tokens

    public init(model: FontPickerModel, look: PreviewLook, @ViewBuilder face: () -> Face) {
        self.model = model
        self.look = look
        self.face = face()
    }

    public var body: some View {
        Button { model.isOpen.toggle() } label: { face }
            .buttonStyle(.plain)
            .popover(isPresented: $model.isOpen, arrowEdge: .bottom) {
                // A popover is a window of its own: it gets the theme again, as the info card's does.
                FontPickerContent(model: model, look: look).environment(\.tokens, tokens)
            }
            .accessibilityAddTraits(.isButton)
            .accessibilityValue(model.isInherited ? (model.slot.inheritLabel ?? "") : model.selected)
    }
}

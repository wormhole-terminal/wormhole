// The one text field of the settings pages: the shared flat field (`DesignSystem` `FlatField`, `style::text_field`). It applies what was typed when it loses focus or
// takes Return, as the GPUI fields do.

import DesignSystem
import SwiftUI

struct InputField: View {
    let placeholder: String
    @Binding var text: String
    var width: CGFloat = SettingsMetric.fieldWidth
    var frameId: String?
    let onCommit: () -> Void

    var body: some View {
        FlatField(placeholder, text: $text, onCommit: onCommit)
            .frame(width: width)
            .recordFrame(frameId)
    }
}

// One of the field's three toggles (`find_bar.rs`: `icon_button(...).in_row().selected(...)`): the shared icon button on a
// 22 pt face, in the selected tint while on, `band.hover` under the pointer. A toggle to VoiceOver ("Match case, on");
// Space flips it.

import DesignSystem
import SwiftUI

struct FindToggle: View {
    let option: FindOption
    let model: FindModel

    var body: some View {
        IconButton(lucide: icon, label: L10n.findOption(option)) { model.toggle(option) }
            .inRow()
            .toggled(model.options[option])
    }

    /// The Lucide icons the GPUI bar uses (`option_icon`).
    private var icon: Lucide {
        switch option {
        case .matchCase: .caseSensitive
        case .wholeWord: .wholeWord
        case .regex: .regex
        }
    }
}

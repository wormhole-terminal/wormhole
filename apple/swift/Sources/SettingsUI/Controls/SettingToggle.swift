// A switch, or with `isCheckbox` a checkbox beside its words: the system's toggles, so the keyboard, VoiceOver and the
// look are the system's. The theme reaches them through the window's tint. Stateless: it draws `isOn` and reports a change.

import DesignSystem
import SwiftUI

struct SettingToggle: View {
    let label: String
    let isOn: Bool
    var isCheckbox = false
    let onChange: (Bool) -> Void

    var body: some View {
        let toggle = Toggle(label, isOn: Binding(get: { isOn }, set: onChange))
        if isCheckbox { toggle.toggleStyle(.checkbox).marked("checkbox") } else { toggle.toggleStyle(.switch).labelsHidden().marked("switch") }
    }
}

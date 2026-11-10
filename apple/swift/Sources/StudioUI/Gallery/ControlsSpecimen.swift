// The rest of what a settings page is made of, native controls coloured by the theme: a slider, a stepper, a
// segmented control and the profile chip, and the four banners (`gallery/settings.rs`).

import DesignSystem
import PanesUI
import SwiftUI

struct ControlsSpecimen: View {
    @State private var level = 0.6
    @State private var amount = 4
    @State private var segment = 0
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            HStack(spacing: Space._4) {
                Slider(value: $level)
                Stepper("\(amount)", value: $amount, in: 0...12)
            }
            Picker("", selection: $segment) { Text("One").tag(0); Text("Two").tag(1); Text("Three").tag(2) }
                .pickerStyle(.segmented).labelsHidden()
            ProfileChip(choices: TabSamples.profiles)
            VStack(spacing: Space._2) {
                Banner(kind: .info, text: "A banner says something quietly.")
                Banner(kind: .warning, text: "This project asks to run commands.")
                Banner(kind: .danger, text: "That did not work.")
                Banner(kind: .success, text: "Saved.")
            }
        }
        .tint(tokens.primary)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

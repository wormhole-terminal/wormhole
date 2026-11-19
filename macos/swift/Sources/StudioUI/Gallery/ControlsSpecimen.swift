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
                Stepper("\(amount)", value: $amount, in: 0...12).marked("stepper")
            }
            Picker("", selection: $segment) { Text("One").tag(0); Text("Two").tag(1); Text("Three").tag(2) } // text: the Studio gallery's specimen
                .pickerStyle(.segmented).labelsHidden().marked("segmented")
            ProfileChip(choices: TabSamples.profiles)
            VStack(spacing: Space._2) {
                Banner(kind: .info, text: "A banner says something quietly.") // text: the Studio gallery's specimen
                Banner(kind: .warning, text: "This project asks to run commands.") // text: the Studio gallery's specimen
                Banner(kind: .danger, text: "That did not work.") // text: the Studio gallery's specimen
                Banner(kind: .success, text: "Saved.") // text: the Studio gallery's specimen
            }
        }
        .tint(tokens.primary)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

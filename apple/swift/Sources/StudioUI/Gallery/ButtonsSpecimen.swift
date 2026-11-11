// The button variants and sizes: the system's styles coloured by the theme (what a Settings page uses) and the
// windows' own themed primary and secondary (`gallery/buttons.rs`).
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import AuxWindows
import DesignSystem
import SwiftUI

struct ButtonsSpecimen: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            HStack(spacing: Space._2) {
                Button("Primary") {}.buttonStyle(.borderedProminent).marked("button.primary")
                Button("Button") {}.buttonStyle(.bordered).marked("button")
                Button("Ghost") {}.buttonStyle(.plain)
                Button("Link") {}.buttonStyle(.link).foregroundStyle(tokens.textLink)
                Button("Danger", role: .destructive) {}.buttonStyle(.bordered)
            }
            HStack(spacing: Space._2) {
                ForEach([ControlSize.mini, .small, .regular, .large], id: \.self) { size in
                    Button("\(String(describing: size))") {}.buttonStyle(.borderedProminent).controlSize(size)
                }
            }
            AuxButtonsSpecimen()
        }
        .tint(tokens.primary)
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

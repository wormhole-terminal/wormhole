// The controls a settings page is made of, native ones coloured by the theme: buttons, a field, a switch, a
// checkbox, a slider, a stepper, a segmented control and a menu, and the four banners (`gallery/settings.rs`).

import DesignSystem
import SwiftUI

struct ControlsSpecimen: View {
    @State private var text = ""
    @State private var on = true
    @State private var checked = true
    @State private var level = 0.6
    @State private var amount = 4
    @State private var segment = 0
    @State private var choice = "Tokyo Night"
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            HStack(spacing: Space._2) {
                Button("Primary") {}.buttonStyle(.borderedProminent)
                Button("Secondary") {}.buttonStyle(.bordered)
                Button("Link") {}.buttonStyle(.link).foregroundStyle(tokens.textLink)
            }
            TextField("A field", text: $text).textFieldStyle(.roundedBorder)
            HStack(spacing: Space._4) {
                Toggle("Switch", isOn: $on).toggleStyle(.switch)
                Toggle("Checkbox", isOn: $checked).toggleStyle(.checkbox)
            }
            HStack(spacing: Space._4) {
                Slider(value: $level)
                Stepper("\(amount)", value: $amount, in: 0...12)
            }
            Picker("", selection: $segment) { Text("One").tag(0); Text("Two").tag(1); Text("Three").tag(2) }
                .pickerStyle(.segmented).labelsHidden()
            Picker("Theme", selection: $choice) { Text("Tokyo Night").tag("Tokyo Night"); Text("Nord").tag("Nord") }
                .pickerStyle(.menu)
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

struct Banner: View {
    enum Kind { case info, warning, danger, success }
    let kind: Kind
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let (fill, edge, icon, name) = look
        HStack(spacing: Space._2) {
            Image(systemName: icon).foregroundStyle(edge)
            Text(text).font(TextRole.body.font)
            Spacer(minLength: 0)
        }
        .padding(Space._3)
        .background(fill, in: shape)
        .overlay(shape.strokeBorder(edge.opacity(0.35), lineWidth: tokens.metrics.borderWidth))
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(name): \(text)")
    }

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: tokens.metrics.radius) }

    private var look: (Color, Color, String, String) {
        switch kind {
        case .info: (tokens.bannerInfoBackground, tokens.statusInfo, "info.circle", "Information")
        case .warning: (tokens.bannerWarningBackground, tokens.statusWarning, "exclamationmark.triangle", "Warning")
        case .danger: (tokens.bannerDangerBackground, tokens.statusDanger, "xmark.octagon", "Error")
        case .success: (tokens.bannerSuccessBackground, tokens.statusSuccess, "checkmark.circle", "Done")
        }
    }
}

import SwiftUI

/// The colour row: a Theme pill (and Primary for the branch), eight colours, and the system colour well for Custom.
struct ColourRow: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t

    var isBranch: Bool { row.swatches == "branch" }
    var cur: String { m.get(row.key, row) }

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            RowText(row: row, hint: hint)
            HStack(spacing: Space.s2) {
                pill("Theme"); if isBranch { pill("Primary") }
                ForEach(accentColours, id: \.0) { n, hex in dot(n, Color(hex: hex)) }
                ColorPicker(m.w("Custom"), selection: Binding(get: { m.customColour(row.key) }, set: { m.set(row.key + "#custom", $0.hexString); m.set(row.key, "custom") }), supportsOpacity: false)
                    .labelsHidden().overlay { if cur == "custom" { Circle().strokeBorder(m.customColour(row.key), lineWidth: 2).padding(-3).allowsHitTesting(false) } }
            }
        }.padding(.vertical, Space.s1)
    }

    var hint: String {
        switch cur {
        case "theme": m.w("The theme’s · %@", m.theme(light: !t.dark).blue.hexString)
        case "primary": m.w("The primary colour · %@", t.primary.hexString)
        case "custom": m.w("Yours · %@, in every theme", m.customColour(row.key).hexString)
        default: m.w("Yours · %@, in every theme", accentColours.first { snakeID($0.0) == cur }.map { Color(hex: $0.1).hexString } ?? "")
        }
    }

    func pill(_ n: String) -> some View {
        let sel = cur == snakeID(n)
        return Button { m.set(row.key, snakeID(n)) } label: {
            Text(m.w(n)).role(.body).padding(.horizontal, 9).frame(height: 22)
                .background(Capsule().fill(sel ? t.primary : Color.secondary.opacity(0.18))).foregroundStyle(sel ? t.onPrimary : .secondary)
        }.buttonStyle(.plain)
    }

    func dot(_ n: String, _ colour: Color) -> some View {
        Button { m.set(row.key, snakeID(n)) } label: {
            Circle().fill(colour).frame(width: 20, height: 20)
                .overlay { if cur == snakeID(n) { Circle().strokeBorder(colour, lineWidth: 2).padding(-4) } }.padding(4)
        }.buttonStyle(.plain).help(m.w(n))
    }
}

extension Color {
    var hexString: String {
        let c = NSColor(self).usingColorSpace(.sRGB) ?? .black
        return String(format: "#%02x%02x%02x", Int((c.redComponent * 255).rounded()), Int((c.greenComponent * 255).rounded()), Int((c.blueComponent * 255).rounded()))
    }
}

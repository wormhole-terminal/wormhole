// "Folder colour" (`panel.rs` `render_folder_color`): the eight colours and None in a row, the folder's marked. It is the same
// setting as the band's menu. Where it is kept depends on the folder: the person's own, or the project's `config.json`
// when it has one; the line under the row says which. A project that locks its colour shows the row dimmed, and a hex
// colour it sets by hand is a swatch of its own, matching none of the names.

import DesignSystem
import PanelModel
import SwiftUI
import WorkspaceModel

struct FolderColorCard: View {
    let store: ProjectStore
    @State private var picked = Color.accentColor
    @State private var choosing = false
    @State private var folded = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        let paint = store.paint
        PanelCard {
            CardBand(title: panelText("project-folder-color", "Folder colour"), folded: folded, onToggle: { folded.toggle() })
            if !folded { choices(paint) }
        }
        .animation(.snappy(duration: 0.16), value: folded)
    }

    @ViewBuilder private func choices(_ paint: FolderPaint) -> some View {
        Group {
            HStack(spacing: 0) {
                ColorChoice(name: "None", color: nil, chosen: paint.chosen == nil && paint.custom == nil, enabled: paint.enabled) { store.setColor(nil) }
                if let hex = paint.custom {
                    ColorChoice(name: "Custom colour \(hex)", color: Color(hex: hex), chosen: true, enabled: false) {}
                        .help("Custom colour \(hex), set in config.json")
                }
                ForEach(FolderColor.allCases, id: \.self) { name in
                    ColorChoice(name: name.rawValue.capitalized, color: FolderTint.color(name, in: tokens), chosen: paint.chosen == name, enabled: paint.enabled) { store.setColor(name) }
                }
                if paint.enabled { picker }
                Spacer(minLength: 0)
            }
            // The first dot lines up with the panel's inset, inside its hit area.
            .padding(.horizontal, PanelMetrics.inset - (Metric.hitMin - ColorChoice.dot) / 2)
            Text(paint.note).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                .padding(.horizontal, PanelMetrics.inset).padding(.bottom, Space._2)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    /// Any other colour: the system's picker in a popover, which saves a hex colour.
    private var picker: some View {
        BandButton(symbol: "plus", label: panelText("project-pick-colour", "Pick a colour")) { choosing.toggle() }
            .popover(isPresented: $choosing) {
                ColorPicker(panelText("project-pick-colour", "Pick a colour"), selection: $picked, supportsOpacity: false)
                    .padding(Space._3).onChange(of: picked) { _, color in store.setColor(hex: color.hexString) }
            }
    }
}

extension Color {
    init(hex: String) {
        let digits = hex.dropFirst()
        let full = digits.count == 3 ? digits.map { "\($0)\($0)" }.joined() : String(digits)
        let value = UInt32(full, radix: 16) ?? 0
        self.init(.sRGB, red: Double(value >> 16 & 255) / 255, green: Double(value >> 8 & 255) / 255, blue: Double(value & 255) / 255)
    }

    /// `#rrggbb`, which is what a project's `config.json` holds.
    var hexString: String {
        let c = NSColor(self).usingColorSpace(.sRGB) ?? .black
        return String(format: "#%02x%02x%02x", Int((c.redComponent * 255).rounded()), Int((c.greenComponent * 255).rounded()), Int((c.blueComponent * 255).rounded()))
    }
}

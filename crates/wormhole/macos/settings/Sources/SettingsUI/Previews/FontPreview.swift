import SwiftUI
import AppKit

/// Fonts: the app's specimen. Terminal | Code | Interface in the theme's terminal colours; the chosen font shows in it.
struct FontPreview: View {
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m

    var body: some View {
        let th = m.theme(light: !t.dark)
        let tab = m.specimenTab
        let slot = ["Terminal": 0, "Code": 1, "Interface": 2][tab] ?? 0
        let real = m.preview?.fonts[["terminal", "code", "interface"][slot]]
        var name = m.hoverFont ?? real?.family ?? m.value("Fonts:\(slot):0")
        if name == "same_as_terminal" { name = m.value("Fonts:0:0") }
        let size = CGFloat(real?.size ?? Double(Int(m.value("Fonts:\(slot):0#size")) ?? [14, 13, 13][slot]))
        let drawn = min(size, 18)
        let lh = tab == "Terminal" ? (m.preview.map { String(format: "%.2f", $0.lineHeight) } ?? m.str("Fonts", 0, 1)) : "1.40"
        let lig = m.preview?.ligatures ?? m.on("Fonts", 0, 2)
        let facts = ["\(name == ".SystemUIFont" ? m.w("System font") : name) \(Int(size)) pt"] + (tab == "Terminal" ? [m.w("line height %@", lh)] : []) + (tab != "Interface" ? [m.w(lig ? "ligatures on" : "ligatures off")] : []) + [m.w("%@ colours", m.pairName)]
        // CSS line-height is a multiple of the size; SwiftUI adds to the font's own line height.
        let extra = max(0, (Double(lh) ?? 1.4) * drawn - NSFont.systemFont(ofSize: drawn).boundingRectForFont.height)

        func code(_ parts: [(String, Color)], ligatures: Bool = true) -> some View {
            var out = AttributedString()
            for (text, colour) in parts {
                var r = AttributedString(text); r.foregroundColor = colour
                if !(ligatures && lig) { r.ligature = 0 }
                out += r
            }
            return Text(out).font(specimenFont(name, size: drawn)).lineLimit(1).fixedSize().lineSpacing(extra)
        }
        return VStack(spacing: 0) {
            HStack(spacing: Space.s3) {
                HStack(spacing: 0) {
                    ForEach(["Terminal", "Code", "Interface"], id: \.self) { n in
                        Button { m.specimenTab = n } label: {
                            Text(m.w(n)).role(.body).padding(.horizontal, 10).frame(height: 22)
                                .background(RoundedRectangle(cornerRadius: Radius.chip).fill(n == tab ? t.buttonHover : .clear))
                                .foregroundStyle(n == tab ? t.text : t.muted)
                        }.buttonStyle(.plain)
                    }
                }.padding(2).background(RoundedRectangle(cornerRadius: Radius.tile).fill(t.input))
                Text(m.w("Shows the font you chose")).role(.body).foregroundStyle(t.faint).lineLimit(1)
                Spacer(minLength: 0)
            }.padding(Space.s2)
            Rule(colour: m.line(t))
            VStack(alignment: .leading, spacing: 0) {
                code([("→ ", th.green), ("wormhole ", th.blue), ("(main) ", th.magenta), ("cargo test --workspace", th.fg)])
                code([("test result: ", th.fg), ("ok", th.green), (". 57 passed; 0 failed", th.fg)])
                code([("fn ", th.magenta), ("resolve", th.blue), ("(&self) -> Result<Tokens> { a != b && c >= d }", th.fg)])
                code([("0O 1lI {}[] => != ~ |> <= www // ligatures", th.fg.opacity(0.55))])
            }.padding(.horizontal, Space.s4).padding(.vertical, Space.s3).frame(maxWidth: .infinity, alignment: .leading).background(th.bg).clipped()
            Rule(colour: m.line(t))
            HStack(spacing: Space.s4) { ForEach(facts, id: \.self) { Text($0) }; Spacer(minLength: 0) }
                .role(.body).foregroundStyle(t.muted).lineLimit(1).padding(.horizontal, Space.s4).padding(.vertical, Space.s2)
        }
    }
}


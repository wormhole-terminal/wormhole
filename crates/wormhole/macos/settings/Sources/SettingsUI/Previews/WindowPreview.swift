import SwiftUI

/// Appearance: one window as the settings draw it: theme, opacity, blur, glass, dividers, status bar, title-row parts.
struct WindowPreview: View {
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m
    var body: some View {
        let th = m.theme(light: !t.dark)
        let opacity = (Double(m.str("Appearance", 2, 0).split(separator: " ").first ?? "100") ?? 100) / 100
        let blur = m.on("Appearance", 2, 1), glass = m.on("Appearance", 2, 2)
        let dividers = m.on("Appearance", 3, 1), status = m.on("Appearance", 3, 2)
        let ln = m.line(t)
        let shown = titleParts.filter { m.on("Appearance:4:0" + $0.0) }.map(\.0)
        let left = shown.filter { ["Folder", "Branch", "Branch status", "Terminal name", "Terminal status"].contains($0) }
        let right = shown.filter { !left.contains($0) }
        let branch = m.swatch("Appearance:1:1") ?? th.green

        func part(_ n: String) -> some View {
            Text(titlePartText[n] ?? "").role(.caption, weight: n == "Branch" ? .semibold : .regular).lineLimit(1)
                .foregroundStyle(n == "Branch" ? branch : (n == "Folder" || n == "Terminal name") ? th.fg : th.fg.opacity(0.6))
        }
        return VStack(spacing: 0) {
            HStack(spacing: Space.s2) {
                HStack(spacing: Space.s1) {
                    ForEach([Palette.trafficRed, Palette.trafficYellow, Palette.trafficGreen], id: \.self) { Circle().fill($0).frame(width: 8, height: 8) }
                }.padding(.trailing, Space.s1)
                ForEach(left, id: \.self) { part($0) }
                Spacer(minLength: 0)
                ForEach(right, id: \.self) { part($0) }
            }.padding(.horizontal, 10).frame(height: 28).clipped()
            Rule(colour: ln)
            HStack(spacing: 0) {
                VStack(spacing: 6) {
                    ForEach(0..<3, id: \.self) { i in
                        RoundedRectangle(cornerRadius: Radius.chip)
                            .fill(glass ? th.fg.opacity(i == 0 ? 0.16 : 0.07) : (i == 0 ? (m.accent ?? th.blue).opacity(0.85) : th.fg.opacity(0.12)))
                            .overlay { if glass { RoundedRectangle(cornerRadius: Radius.chip).strokeBorder(m.tileLine) } }
                            .frame(height: 16)
                    }
                    Spacer(minLength: 0)
                }.padding(.horizontal, Space.s2).padding(.vertical, 10).frame(width: 120)
                if dividers { Rule(colour: th.fg.opacity(0.12), vertical: true) }
                VStack(alignment: .leading, spacing: Space.s1) {
                    TermLine(text: "~/wormhole $ cargo test", colour: th.blue)
                    Text("   Compiling wormhole v1.5.0").font(terminalFont(11)).foregroundStyle(th.fg).lineLimit(1)
                    TermLine(text: "test result: ok. 412 passed", colour: th.green)
                    TermLine(text: "~/wormhole $ ", colour: th.blue)
                    Spacer(minLength: 0)
                }.padding(10).frame(maxWidth: .infinity, alignment: .leading)
            }.frame(height: 150)
            if status {
                Rule(colour: ln)
                HStack(spacing: 10) {
                    Text("main"); Text("3 changed"); Spacer(minLength: 0); Text("zsh")
                }.role(.micro).foregroundStyle(th.fg.opacity(0.7)).padding(.horizontal, 10).frame(height: 20)
            }
        }
        .background { ZStack { if blur && opacity < 1 { Rectangle().fill(.ultraThinMaterial) }; th.bg.opacity(opacity) } }
        .clipShape(RoundedRectangle(cornerRadius: Radius.window))
        .overlay(RoundedRectangle(cornerRadius: Radius.window).strokeBorder(ln))
        .shadow(color: .black.opacity(0.45), radius: 13, y: 5)
        .padding(Space.s3)
    }
}

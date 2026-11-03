import SwiftUI

/// Terminal: two split terminals, with or without gaps and the minimap.
struct TerminalPreview: View {
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m
    var body: some View {
        let th = m.theme(light: !t.dark)
        let minimap = m.on("Terminal", 0, 2), gaps = m.on("Terminal", 0, 3)
        let lines: [[(String, Color)]] = [
            [("~/api $ cargo test", th.blue), ("running 412 tests", th.fg), ("test limits::burst ... ok", th.fg), ("test limits::refill ... ok", th.fg), ("test result: ok", th.green), ("~/api $ ", th.blue)],
            [("~/api $ git log --oneline", th.blue), ("a41f2c9 limits: refill", th.yellow), ("9e03b11 limits: burst", th.yellow), ("77d0a2e api: routes", th.yellow), ("~/api $ ", th.blue)],
        ]
        let maps: [[Double]] = [[60, 85, 40, 70, 90, 55, 30, 75, 65, 45, 80, 35], [70, 50, 85, 40, 60, 90, 45, 30, 75, 55, 65, 40]]
        return HStack(spacing: gaps ? Space.s2 : 0) {
            ForEach(0..<2, id: \.self) { i in
                HStack(spacing: 0) {
                    VStack(alignment: .leading, spacing: 3) {
                        ForEach(Array(lines[i].enumerated()), id: \.offset) { _, l in TermLine(text: l.0, colour: l.1) }
                        Spacer(minLength: 0)
                    }.padding(10).frame(maxWidth: .infinity, alignment: .leading)
                    if minimap {
                        Rule(colour: m.line(t), vertical: true)
                        VStack(alignment: .leading, spacing: 2) {
                            ForEach(Array(maps[i].enumerated()), id: \.offset) { _, w in
                                GeometryReader { g in RoundedRectangle(cornerRadius: 1).fill(th.fg.opacity(0.28)).frame(width: g.size.width * w / 100, height: 3) }.frame(height: 3)
                            }
                            Spacer(minLength: 0)
                        }.padding(.horizontal, 3).padding(.vertical, Space.s2).frame(width: 18)
                    }
                }
                .background(th.bg)
                .clipShape(RoundedRectangle(cornerRadius: gaps ? Radius.control : 0))
                .overlay { if gaps { RoundedRectangle(cornerRadius: Radius.control).strokeBorder(th.fg.opacity(0.08)) } }
                .overlay(alignment: .trailing) { if !gaps && i == 0 { Rule(colour: th.fg.opacity(0.16), vertical: true) } }
            }
        }.padding(gaps ? Space.s2 : 0).frame(height: 200)
    }
}

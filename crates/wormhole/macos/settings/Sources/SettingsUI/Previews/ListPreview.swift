import SwiftUI

/// A pulsing ring for "needs input": holds still under Reduce Motion.
struct Pulse: ViewModifier {
    var on: Bool
    @Environment(\.accessibilityReduceMotion) var reduce
    @State private var phase = false
    func body(content: Content) -> some View {
        content.shadow(color: Palette.yellow.opacity(on ? (phase ? 0.35 : 0) : 0), radius: 0, x: 0, y: 0)
            .background(Circle().fill(Palette.yellow.opacity(on ? (phase ? 0.35 : 0) : 0)).scaleEffect(phase ? 1.9 : 1))
            .onAppear { if on && !reduce { withAnimation(.easeInOut(duration: 0.8).repeatForever(autoreverses: true)) { phase = true } } }
    }
}

struct SessionRowSpec { let name, status, tile: String; let glyph: String; let tone: Tone; let added, removed: String; let note: Bool
    enum Tone { case work, input, done, idle } }

/// A terminal in the list: tile with its status dot, name, status line, changed files, notes mark.
struct SessionRow: View {
    let spec: SessionRowSpec; let height: CGFloat; let dense: Bool; let selected: Bool
    let dot: Bool; let changes: Bool; let notes: Bool; let tileFill: Color
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m
    var tone: Color { switch spec.tone { case .work: t.primary; case .input: Palette.yellow; case .done: Palette.green; case .idle: t.muted } }
    var body: some View {
        let side: CGFloat = dense ? 18 : 22
        HStack(spacing: Space.s2) {
            RoundedRectangle(cornerRadius: Radius.control).fill(tileFill).frame(width: side, height: side)
                .overlay(RoundedRectangle(cornerRadius: Radius.control).strokeBorder(m.tileLine))
                .overlay { GlyphIcon(d: spec.glyph, size: 12) }
                .overlay(alignment: .bottomTrailing) {
                    if dot && spec.tone != .idle {
                        Circle().fill(tone).frame(width: 8, height: 8).overlay(Circle().strokeBorder(t.sidebar, lineWidth: 2).padding(-1))
                            .modifier(Pulse(on: spec.tone == .input)).offset(x: 3, y: 3)
                    }
                }
            VStack(alignment: .leading, spacing: 0) {
                Text(m.w(spec.name)).role(.body, weight: selected ? .semibold : .regular).lineLimit(1).foregroundStyle(selected ? t.onPrimary : t.text)
                if !dense { Text(m.w(spec.status)).role(.caption).lineLimit(1).foregroundStyle(selected ? t.onPrimary : tone) }
            }
            Spacer(minLength: 0)
            if changes && !spec.added.isEmpty {
                HStack(spacing: 4) { Text(spec.added).foregroundStyle(Palette.green); Text(spec.removed).foregroundStyle(t.removed) }.role(.micro, mono: true)
            }
            if notes && spec.note { GlyphIcon(d: GlyphPath.notes, size: 11, colour: t.muted, line: 1.6) }
        }.padding(.horizontal, Space.s2).frame(height: height)
            .background(RoundedRectangle(cornerRadius: Radius.tile).fill(selected ? t.primary : .clear))
    }
}

/// Terminals & sidebar and Coding agents: the Terminals list beside an agent's pane.
struct ListPreview: View {
    let sessions: Bool
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m
    var body: some View {
        let th = m.theme(light: !t.dark)
        let density = sessions ? m.str("Sessions", 0, 1) : "regular"
        let cards = sessions ? m.on("Sessions", 1, 0) : true
        let fade = sessions && m.on("Sessions", 1, 1)
        let folders = sessions ? m.on("Sessions", 1, 2) : true
        let changes = sessions && m.on("Sessions", 3, 0), dot = sessions ? m.on("Sessions", 3, 5) : true, notes = sessions && m.on("Sessions", 3, 6)
        let h: CGFloat = ["Regular": 42, "Compact": 34, "Dense": 26][density] ?? 42
        let rows: [SessionRowSpec] = sessions
            ? [.init(name: "scratch", status: "Idle", tile: "button", glyph: GlyphPath.prompt, tone: .idle, added: "", removed: "", note: false),
               .init(name: "Sidebar cards", status: "Working · 4m", tile: "claude", glyph: GlyphPath.claude, tone: .work, added: "+48", removed: "−12", note: true),
               .init(name: "cargo test", status: "Finished · 1m", tile: "dark", glyph: GlyphPath.prompt, tone: .done, added: "+3", removed: "−1", note: false)]
            : [.init(name: "Rate limits", status: "Working · 4m", tile: "claude", glyph: GlyphPath.claude, tone: .work, added: "", removed: "", note: false),
               .init(name: "Flaky login test", status: "Needs input · Allow edit?", tile: "indigo", glyph: GlyphPath.prompt, tone: .input, added: "", removed: "", note: false),
               .init(name: "Release notes", status: "Finished · 1m", tile: "black", glyph: GlyphPath.spark, tone: .done, added: "", removed: "", note: false)]
        let tiles: [String: Color] = ["button": t.button, "claude": Palette.claude, "dark": Palette.tileDark, "indigo": Palette.indigo, "black": Color(hex: 0x111111)]
        let group = VStack(spacing: 2) {
            VStack(alignment: .leading, spacing: 0) {
                Text("api").role(.body, weight: .semibold)
                if folders { Text("~/Projects/api").role(.caption).foregroundStyle(t.muted) }
            }.frame(maxWidth: .infinity, alignment: .leading).padding(.leading, 6).padding(.trailing, 6).padding(.top, 2).padding(.bottom, 6)
            ForEach(Array(rows.enumerated()), id: \.offset) { i, r in
                SessionRow(spec: r, height: h, dense: density == "dense", selected: i == 1, dot: dot, changes: changes, notes: notes, tileFill: tiles[r.tile] ?? t.button)
            }
        }
        return HStack(spacing: 0) {
            Group {
                if cards { group.padding(Space.s1).background(RoundedRectangle(cornerRadius: Radius.card).fill(Palette.indigo.opacity(fade ? 0.14 : 0.26))) }
                else { group.overlay(alignment: .top) { Rectangle().fill(Palette.indigo.opacity(0.6)).frame(height: 2) } }
            }.frame(maxHeight: .infinity, alignment: .top).padding(.horizontal, Space.s2).padding(.vertical, 10).frame(width: 220).background(t.sidebar)
            AgentPane(th: th)
        }.frame(height: 210)
    }
}

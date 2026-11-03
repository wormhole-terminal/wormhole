import SwiftUI

/// Motion: the list with one terminal coming in and one status changing, looping. Holds still under Reduce Motion.
struct MotionPreview: View {
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m
    @Environment(\.accessibilityReduceMotion) var reduce
    @Environment(\.controlActiveState) private var activeState
    private var paused: Bool { reduce || activeState == .inactive }
    var body: some View {
        let th = m.theme(light: !t.dark)
        HStack(spacing: 0) {
            TimelineView(.animation(minimumInterval: 1.0 / 30, paused: paused)) { ctx in
                let p = paused ? (1.0, 0.0) : phase(ctx.date)
                VStack(spacing: 2) {
                    row("scratch", "Idle", t.button, .idle, false)
                    row("Sidebar cards", "Working · 4m", Palette.claude, .work, true).opacity(p.0).offset(x: p.1)
                    row("Flaky login test", "Needs input", Palette.indigo, .input, false)
                    Spacer(minLength: 0)
                }.padding(.horizontal, Space.s2).padding(.vertical, 10).frame(width: 220).background(t.sidebar)
            }
            AgentPane(th: th)
        }.frame(height: 170)
    }

    /// The board's wmIn: 2.4 s, hidden to 15 %, in by 45 %, then alternating back.
    func phase(_ d: Date) -> (Double, Double) {
        let s = d.timeIntervalSinceReferenceDate.truncatingRemainder(dividingBy: 4.8)
        let p = s < 2.4 ? s / 2.4 : (4.8 - s) / 2.4
        let k = min(1, max(0, (p - 0.15) / 0.3)), e = 1 - pow(1 - k, 2)
        return (e, -14 * (1 - e))
    }

    func row(_ name: String, _ status: String, _ tile: Color, _ tone: SessionRowSpec.Tone, _ coming: Bool) -> some View {
        HStack(spacing: Space.s2) {
            RoundedRectangle(cornerRadius: Radius.control).fill(tile).frame(width: 22, height: 22).modifier(Pulse(on: tone == .input))
            VStack(alignment: .leading, spacing: 0) {
                Text(name).role(.body).foregroundStyle(coming ? t.onPrimary : t.text)
                Text(status).role(.caption).foregroundStyle(coming ? t.onPrimary : tone == .input ? Palette.yellow : t.muted)
            }
            Spacer(minLength: 0)
        }.padding(.horizontal, Space.s2).frame(height: 38)
            .background(RoundedRectangle(cornerRadius: Radius.tile).fill(coming ? t.primary : .clear))
    }
}

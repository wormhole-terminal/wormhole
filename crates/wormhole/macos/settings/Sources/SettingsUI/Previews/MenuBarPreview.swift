import SwiftUI

/// Menu bar & hotkey: the menu bar extra and its menu of terminals waiting for you.
struct MenuBarPreview: View {
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m
    var body: some View {
        let shown = m.on("MenuBar", 0, 0), shows = m.str("MenuBar", 0, 1)
        VStack(spacing: 0) {
            HStack(spacing: Space.s3) {
                Spacer(minLength: 0)
                if shown {
                    HStack(spacing: Space.s1) {
                        RoundedRectangle(cornerRadius: 2).stroke(t.text, lineWidth: 1.4).frame(width: 12, height: 9)
                            .overlay { GlyphIcon(d: GlyphPath.prompt, size: 9, colour: t.text, line: 1.4) }
                        if shows != "Icon only" { Circle().fill(Palette.yellow).frame(width: 6, height: 6) }
                        if shows == "Status and count" { Text("2") }
                    }.padding(.horizontal, 5).frame(height: 18).background(RoundedRectangle(cornerRadius: 4).fill(t.buttonHover))
                }
                Text("Thu 09:41")
            }.role(.caption).padding(.horizontal, 10).frame(height: 24).background(t.titlebar).foregroundStyle(t.text)
            if shown {
                VStack(spacing: 2) {
                    Text("Waiting for you").role(.micro, weight: .semibold).foregroundStyle(t.muted).padding(.horizontal, 6).padding(.vertical, 2).frame(maxWidth: .infinity, alignment: .leading)
                    HStack { Text("Flaky login test"); Spacer(); Text("⌘1") }.padding(.horizontal, 6).padding(.vertical, 4)
                        .background(RoundedRectangle(cornerRadius: Radius.chip).fill(t.primary))
                    HStack { Text("Rate limits"); Spacer(); Text("⌘2").foregroundStyle(t.muted) }.padding(.horizontal, 6).padding(.vertical, 4)
                }.role(.body).padding(6).frame(width: 240)
                    .background(RoundedRectangle(cornerRadius: Radius.card).fill(t.elevated)).shadow(color: .black.opacity(0.45), radius: 14, y: 10)
                    .padding(.top, 6).padding(.trailing, 10).frame(maxWidth: .infinity, alignment: .trailing)
            }
        }.padding(.bottom, Space.s3).background(t.panel)
    }
}

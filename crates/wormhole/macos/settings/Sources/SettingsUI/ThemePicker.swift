import SwiftUI

/// Appearance → Theme: pair cards (dark half, light half) with names and a check, or two lists when the
/// themes are chosen separately; Show all, Import Theme…, Theme Studio.
struct ThemePicker: View {
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @State private var showAll = false
    let cols = Array(repeating: GridItem(.flexible(), spacing: Space.s2), count: 4)

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s3) {
            if m.separateThemes {
                section(m.w("In Dark Mode"), m.themeData.darks, light: false); section(m.w("In Light Mode"), m.themeData.lights, light: true)
            } else {
                LazyVGrid(columns: cols, spacing: Space.s3) {
                    ForEach(showAll ? m.themeData.pairs : Array(m.themeData.pairs.prefix(8)), id: \.name) { p in
                        card(name: p.name, selected: m.pairName == p.name, halves: [p.dark, p.light]) { m.set(m.themeRowKey, p.name) }
                    }
                }
            }
            HStack(spacing: Space.s2) {
                if m.themeData.pairs.count > 8 {
                    Button(showAll ? m.w("Show fewer") : m.w("Show all %@", String(m.themeData.pairs.count))) { showAll.toggle() }.buttonStyle(.link)
                }
                Spacer()
                Btn("Import Theme…") { m.perform("Import Theme…", m.themeRowKey) }; Btn("Theme Studio") { m.perform("Theme Studio", m.themeRowKey) }
            }
        }.padding(.vertical, Space.s1)
    }

    func section(_ title: String, _ keys: [String], light: Bool) -> some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Text(title).role(.body, weight: .semibold).foregroundStyle(t.muted)
            LazyVGrid(columns: cols, spacing: Space.s3) {
                ForEach(keys, id: \.self) { k in
                    card(name: m.themeData.names[k] ?? shortThemeName(k), selected: m.themeChoice(light: light) == k, halves: [k]) { m.set(m.themeRowKey + (light ? "#light" : "#dark"), k) }
                }
            }
        }
    }

    func card(name: String, selected: Bool, halves: [String], pick: @escaping () -> Void) -> some View {
        Button(action: pick) {
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 0) { ForEach(halves, id: \.self) { half($0) } }.frame(height: 60)
                    .clipShape(RoundedRectangle(cornerRadius: Radius.tile))
                    .overlay(RoundedRectangle(cornerRadius: Radius.tile).strokeBorder(selected ? t.primary : .clear, lineWidth: 2))
                HStack(spacing: 4) {
                    Text(name).role(.heading, weight: selected ? .bold : .regular).lineLimit(1)
                    if selected { Image(systemName: "checkmark").font(.system(size: 10, weight: .bold)).foregroundStyle(t.primary) }
                }.foregroundStyle(selected ? t.text : t.muted).padding(.leading, 4)
            }.contentShape(Rectangle())
        }.buttonStyle(.plain)
    }

    /// One half of a card's picture: the theme's background, a sidebar strip and five lines in its colours.
    func half(_ key: String) -> some View {
        let c = m.themePalette(key)
        let light = c.bg.luminance > 0.5
        let ink = light ? Color.black : Color.white
        return HStack(spacing: 0) {
            VStack(spacing: 4) { ForEach(0..<2, id: \.self) { _ in Capsule().fill(ink.opacity(0.18)).frame(width: 4, height: 3) }; Spacer(minLength: 0) }
                .padding(.horizontal, 4).padding(.vertical, Space.s2).frame(width: 22).frame(maxHeight: .infinity).background(ink.opacity(light ? 0.04 : 0.03))
            VStack(alignment: .leading, spacing: 4) {
                ForEach(Array([(c.blue, 70.0), (c.green, 85), (c.yellow, 55), (c.fg, 75), (c.red, 45)].enumerated()), id: \.offset) { _, l in
                    GeometryReader { g in Capsule().fill(l.0).frame(width: g.size.width * l.1 / 100, height: 3) }.frame(height: 3)
                }
            }.padding(.horizontal, 6).padding(.vertical, 10).frame(maxWidth: .infinity, alignment: .leading)
        }.frame(maxWidth: .infinity).background(c.bg)
    }
}

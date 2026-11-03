import SwiftUI

/// Appearance → Theme: pair cards (dark half, light half) with names and a check, or two lists when the
/// themes are chosen separately; Show all, Import Theme…, Theme Studio.
struct ThemePicker: View {
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var showAll = false
    let cols = [GridItem(.adaptive(minimum: 104), spacing: Space.s2)]

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
                    Button(showAll ? m.w("Show fewer") : m.w("Show all %@", String(m.themeData.pairs.count))) { withAnimation(Nav.motion(reduceMotion)) { showAll.toggle() } }.buttonStyle(.link)
                }
                Spacer()
                Btn("Import Theme…") { FilePanel.theme(m, m.themeRowKey) }; Btn("Theme Studio") { m.perform("theme_studio", m.themeRowKey) }
            }
            if let failure = m.store.notes[m.themeRowKey] { Text(failure).role(.caption).foregroundStyle(t.warning) }
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
        ThemeCard(name: name, selected: selected, pick: pick) {
            HStack(spacing: 0) { ForEach(halves, id: \.self) { half($0) } }
        }
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

/// Hover and press feedback stay local to a single card; selection keeps a stable label and checkmark slot.
private struct ThemeCard<Preview: View>: View {
    let name: String
    let selected: Bool
    let pick: () -> Void
    @ViewBuilder let preview: () -> Preview
    @Environment(\.tok) private var t
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var hovering = false

    var body: some View {
        Button(action: pick) {
            VStack(alignment: .leading, spacing: Space.s2) {
                preview().frame(height: 60)
                    .clipShape(RoundedRectangle(cornerRadius: Radius.tile))
                    .overlay {
                        RoundedRectangle(cornerRadius: Radius.tile)
                            .strokeBorder(selected ? t.primary : Color.primary.opacity(hovering ? 0.3 : 0.12), lineWidth: selected ? 2 : 1)
                    }
                HStack(spacing: Space.s1) {
                    Text(name).role(.body, weight: selected ? .semibold : .regular).lineLimit(1)
                    Spacer(minLength: 0)
                    Image(systemName: "checkmark.circle.fill").foregroundStyle(t.primary)
                        .opacity(selected ? 1 : 0).accessibilityHidden(true)
                }.foregroundStyle(selected ? Color.primary : .secondary)
            }.contentShape(Rectangle())
        }
        .buttonStyle(ThemeCardStyle(reduceMotion: reduceMotion))
        .onHover { hovering = $0 }
        .animation(Nav.motion(reduceMotion), value: hovering)
        .animation(Nav.motion(reduceMotion), value: selected)
        .help(name).accessibilityLabel(name)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

private struct ThemeCardStyle: ButtonStyle {
    let reduceMotion: Bool
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .opacity(configuration.isPressed ? 0.8 : 1)
            .scaleEffect(configuration.isPressed && !reduceMotion ? 0.98 : 1)
            .animation(Nav.motion(reduceMotion), value: configuration.isPressed)
    }
}

import SwiftUI

/// The page's preview: label, the card and its caption. Each preview kind is its own file and reads the model.
struct PreviewCard: View {
    let page: Page
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m
    var body: some View {
        Section {
            VStack(alignment: .leading, spacing: Space.s2) {
                card.background(t.panel).clipShape(RoundedRectangle(cornerRadius: Radius.card))
                    .overlay(RoundedRectangle(cornerRadius: Radius.card).strokeBorder(m.line(t)))
                // The caption is part of the row, not the section footer: a grouped form's footer sets its own text alignment.
                Text(previewCaption[page.preview] ?? "").role(.body).foregroundStyle(t.muted)
                    .multilineTextAlignment(.leading).fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading).padding(.leading, 10)
            }
            .listRowInsets(EdgeInsets()).listRowBackground(Color.clear)
        } header: {
            HStack(spacing: 6) { Text("PREVIEW").role(.caption, weight: .bold).tracking(0.6); Text("· " + (previewName[page.preview] ?? "")).role(.caption).foregroundStyle(.tertiary) }
        }
    }

    @ViewBuilder var card: some View {
        switch page.preview {
        case "window": WindowPreview()
        case "terminal": TerminalPreview()
        case "sidebar": ListPreview(sessions: true)
        case "agents": ListPreview(sessions: false)
        case "motion": MotionPreview()
        case "diff": DiffPreview()
        case "banner": BannerPreview()
        case "specimen": FontPreview()
        case "menubar": MenuBarPreview()
        default: EmptyView()
        }
    }
}

extension SettingsModel {
    /// The neutral line every preview draws between parts: off with Borders off, so nothing moves.
    func line(_ t: Tok) -> Color { on("Appearance", 3, 0) ? t.border : .clear }
    /// A tile's inner hairline.
    var tileLine: Color { on("Appearance", 3, 0) ? .white.opacity(0.1) : .clear }
}

/// A one-point rule.
struct Rule: View {
    let colour: Color; var vertical = false
    var body: some View { Rectangle().fill(colour).frame(width: vertical ? 1 : nil, height: vertical ? nil : 1) }
}

/// The terminal lines a pane shows: text in a theme colour, in the terminal font.
struct TermLine: View {
    let text: String; let colour: Color; var size: CGFloat = 11
    @Environment(SettingsModel.self) var m
    var body: some View { Text(text).font(m.terminalFont(size)).foregroundStyle(colour).lineLimit(1).truncationMode(.tail) }
}

struct AgentPane: View {
    let th: ThemePalette
    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            TermLine(text: "~/Projects/api $ claude", colour: th.blue)
            TermLine(text: "✻ Reading src/limits.rs", colour: th.fg)
            TermLine(text: "✻ Editing 2 files…", colour: th.fg)
            TermLine(text: "  ⎿  +48 −12", colour: th.green)
            TermLine(text: "Allow edit of src/limits.rs? (y/n)", colour: th.yellow)
            Spacer(minLength: 0)
        }.padding(.horizontal, Space.s3).padding(.vertical, 10).frame(maxWidth: .infinity, alignment: .leading).background(th.bg)
    }
}

// The controls macOS draws for the app, which a theme does not change: a tooltip (`.help`), a menu (`NSMenu`), a question
// (an alert on the window) and a scroll bar. The GPUI app draws its own of each from the theme (`gallery/navigation.rs`,
// `selects.rs`, `windows.rs`); here each is the system's control itself with a line saying so, so the gallery still shows
// everything a person meets. The tooltip's tokens are drawn by the readouts, the one menu the app draws by Menus.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SwiftUI

struct SystemSpecimen: View {
    @State private var asks = false

    /// The scrolled sample's height: three of its lines show (`Metric.row` each).
    private static let lines = 3

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            SystemNote(StudioWords.systemTooltip) {
                Label(StudioWords.tooltipHere, systemImage: "cursorarrow.rays").help(StudioWords.tooltipSample)
            }
            SystemNote(StudioWords.systemMenu) {
                Menu(StudioWords.showMenu) {
                    Button("Tokyo Night") {}
                    Button("Nord") {}
                    Divider()
                    Button("Move to Trash…", role: .destructive) {}
                }
                .fixedSize()
            }
            SystemNote(StudioWords.systemDialog) {
                Button(StudioWords.showDialog) { asks = true }
                    .alert("Save “Dusk” before closing?", isPresented: $asks) {
                        Button("Save") {}
                        Button("Don’t save", role: .destructive) {}
                        Button("Cancel", role: .cancel) {}
                    } message: {
                        Text("2 changes. If you don’t save, the app goes back to Tokyo Night.")
                    }
            }
            SystemNote(StudioWords.systemScrollbar) {
                ScrollView {
                    VStack(alignment: .leading, spacing: 0) {
                        ForEach(1...12, id: \.self) { Text("Line \($0)").font(TextRole.body.font).frame(height: Metric.row) }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .scrollIndicators(.visible)
                .frame(width: Metric.row * 4, height: Metric.row * CGFloat(Self.lines))
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

/// A control the system draws, with the line that says so under it.
struct SystemNote<Content: View>: View {
    let note: String
    @ViewBuilder let content: Content
    @Environment(\.tokens) private var tokens

    init(_ note: String, @ViewBuilder content: () -> Content) {
        self.note = note
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            content
            Text(note).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
        }
    }
}

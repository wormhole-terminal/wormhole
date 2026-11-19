// An open note's text while nobody writes in it (`features/notes/view.rs` `note_preview`): its lines as they wrap in
// the row. Longer than the core's cap (`note_cap`, twelve lines) only that many show, the last two fading out, then
// "Show all · N lines". The lines are counted by SwiftUI's own layout: the whole text is laid out unseen at the row's
// width, beside one line and two lines of the same style, whose difference is a line's pitch.

import DesignSystem
import PanelModel
import SwiftUI

struct NotePreview: View {
    let text: String
    let showAll: Bool
    let open: () -> Void
    let expand: () -> Void
    @State private var full: CGFloat = 0
    @State private var one: CGFloat = 0
    @State private var two: CGFloat = 0
    @Environment(\.tokens) private var tokens

    /// How opaque the last lines of a cut note are, the last one first (`NOTE_FADE`).
    private static let fade: [Double] = [0.3, 0.65]

    private var pitch: CGFloat { two - one }

    /// The wrapped lines; until the first layout has answered, the text's own lines.
    private var lines: Int {
        guard pitch > 0, full > 0 else { return text.split(separator: "\n", omittingEmptySubsequences: false).count }
        return max(1, Int(((full - one) / pitch).rounded()) + 1)
    }

    var body: some View {
        let cap = NoteRules.cap(lines, false, showAll)
        VStack(alignment: .leading, spacing: 0) {
            styled(text).foregroundStyle(tokens.textPrimary)
                .lineLimit(cap)
                .frame(maxWidth: .infinity, alignment: .leading)
                .mask { mask(cut: cap != nil) }
                .background { measures }
                .contentShape(Rectangle())
                .onTapGesture(perform: open)
            if cap != nil {
                Button(panelText("notes-show-all", ["count": .text(CountText.grouped(lines))], "Show all · \(CountText.grouped(lines)) lines"), action: expand)
                    .buttonStyle(.plain).font(tokens.font(.body)).foregroundStyle(tokens.primary).padding(.top, Space._1)
            }
        }
    }

    private func styled(_ text: String) -> some View {
        Text(verbatim: text).font(tokens.font(.body)).lineSpacing(PanelMetrics.noteLineGap)
    }

    /// The same text unseen: whole at this width, and one and two lines of it, each reporting its height.
    private var measures: some View {
        ZStack(alignment: .topLeading) {
            styled(text).fixedSize(horizontal: false, vertical: true).onGeometryChange(for: CGFloat.self, of: \.size.height) { full = $0 }
            styled("A").fixedSize().onGeometryChange(for: CGFloat.self, of: \.size.height) { one = $0 }  // text: a measuring line, never shown
            styled("A\nA").fixedSize().onGeometryChange(for: CGFloat.self, of: \.size.height) { two = $0 }  // text: two measuring lines, never shown
        }
        .frame(maxWidth: .infinity, alignment: .topLeading)
        .hidden()
        .accessibilityHidden(true)
    }

    /// Everything shows but a cut note's last two lines, which fade a line each, the last one most.
    private func mask(cut: Bool) -> some View {
        VStack(spacing: 0) {
            Color.black
            if cut, pitch > 0 {
                ForEach(Self.fade.indices.reversed(), id: \.self) { n in
                    Color.black.opacity(Self.fade[n]).frame(height: pitch)
                }
            }
        }
    }
}

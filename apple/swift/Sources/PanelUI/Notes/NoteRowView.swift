// A note (`features/notes/view.rs` `render_note`): the chevron in the lead column and its first line as the title;
// open, the lines after it show under the title in the body style, the last two fading when more than twelve are
// cut, with "Show all · N lines" below. Clicking the text writes in it, and the title then quiets down so it does
// not read as duplicated. The actions show under the pointer. Esc ends the writing and gives the keys back to the
// terminal; a note left empty is thrown away; ⌘↵ sends the selection, else the paragraph under the cursor (⇧ also
// submits); ⌫ in an empty note deletes it and opens the one above.

import DesignSystem
import PanelModel
import SwiftUI

struct NoteRowView: View {
    @Binding var note: Note
    /// The note's id, kept apart: once the model swaps (another terminal's notes) or the note goes, the binding's index is stale
    /// and reading `note` traps, so a blur after that must not touch it.
    let noteID: Note.ID
    let store: NotesStore
    let send: () -> Void
    let delete: () -> Void
    var moveBy: (Int) -> Void = { _ in }
    @State private var hovering = false
    @State private var editing = false
    @State private var showAll = false
    @State private var selection: TextSelection?
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens
    @Environment(\.panelMotion) private var motion

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: PanelMetrics.gap) {
                Button { note.collapsed.toggle() } label: {
                    Disclosure(collapsed: note.collapsed).contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel(note.title)
                .accessibilityValue(note.collapsed ? panelText("panel-spoken-folded", "folded") : panelText("panel-spoken-open", "open"))
                Text(note.title).font(TextRole.body.font)
                    .foregroundStyle(editing ? tokens.textMuted : tokens.textPrimary)
                    .lineLimit(1).frame(maxWidth: .infinity, alignment: .leading).contentShape(Rectangle())
                    .onTapGesture { open() }
                if hovering || editing { RowActions(send: send, delete: delete).transition(.opacity) }
            }
            .padding(.horizontal, PanelMetrics.inset)
            .frame(minHeight: Metric.row)
            if !note.collapsed { bodyText.transition(motion.hunk) }
        }
        .background(hovering || editing ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: Radius.row, style: .continuous))
        .onHover { hovering = $0 }
        .animation(motion.fold, value: note.collapsed)
        .animation(motion.fold, value: showAll)
        .onAppear(perform: openIfFresh)
        .onChange(of: store.freshNote) { openIfFresh() }
    }

    private func openIfFresh() {
        guard store.freshNote == note.id else { return }
        open()
        store.consumeFresh()
    }

    @ViewBuilder private var bodyText: some View {
        let lines = note.body.split(separator: "\n", omittingEmptySubsequences: false)
        if editing {
            TextEditor(text: $note.text, selection: $selection).font(TextRole.body.font).scrollContentBackground(.hidden)
                .focused($focused).frame(minHeight: PanelMetrics.noteLine * 3)
                .overlay(alignment: .topLeading) { placeholder }
                .onExitCommand { endWriting(); store.leave() }
                .onKeyPress(.return, phases: .down) { press in
                    guard press.modifiers.contains(.command) else { return .ignored }
                    store.send(text: sent, submit: press.modifiers.contains(.shift))
                    return .handled
                }
                .onKeyPress(.delete, phases: .down) { _ in
                    guard note.text.isEmpty else { return .ignored }
                    editing = false
                    store.removeEmpty(note)
                    return .handled
                }
                .reorderKeys(moveBy)
                .onChange(of: focused) { _, now in if !now { endWriting() } }
                .padding(.leading, PanelMetrics.inset + Metric.lead).padding(.trailing, PanelMetrics.inset - 5)
        } else if !note.body.isEmpty {
            let cut = !showAll && lines.count > PanelMetrics.noteMaxLines
            VStack(alignment: .leading, spacing: 0) {
                Text(lines.prefix(cut ? PanelMetrics.noteMaxLines : lines.count).joined(separator: "\n"))
                    .font(TextRole.body.font).foregroundStyle(tokens.textPrimary)
                    .lineSpacing(PanelMetrics.noteLineGap)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .mask(fade(cut))
                    .onTapGesture { open() }
                if cut {
                    Button(panelText("notes-show-all", ["count": .text(CountText.grouped(lines.count))], "Show all · \(CountText.grouped(lines.count)) lines")) { showAll = true }
                        .buttonStyle(.plain).font(TextRole.body.font).foregroundStyle(tokens.primary).padding(.top, Space._1)
                }
            }
            .padding(.leading, PanelMetrics.inset + Metric.lead + PanelMetrics.gap).padding(.trailing, PanelMetrics.inset)
            .padding(.bottom, Space._2)
        }
    }

    /// What ⌘↵ sends: the selection, else the paragraph under the cursor (core's rule, `paragraphs::paragraph_at`).
    private var sent: String {
        guard case let .selection(range)? = selection?.indices else { return note.text }
        let text = note.text
        guard range.isEmpty else { return String(text[range]) }
        return NoteRules.paragraphAt(text, text.utf8.distance(from: text.startIndex, to: range.lowerBound))
    }

    /// The editor's hint while it is empty, in the body style at the text's own inset.
    @ViewBuilder private var placeholder: some View {
        if note.text.isEmpty {
            Text(panelText("notes-placeholder", "Write a note or a prompt draft\u{2026}  \u{2318}\u{21A9} sends the paragraph under the cursor (or the selection); \u{2318}\u{21E7}\u{21A9} also submits."))
                .font(TextRole.body.font).foregroundStyle(tokens.textFaint).padding(.leading, 5).padding(.top, 0).allowsHitTesting(false)
        }
    }

    private func endWriting() {
        guard editing, store.model.notes.contains(where: { $0.id == noteID }) else { return }
        editing = false
        store.finishWriting(note)
    }

    /// The last two lines of a cut note fade out, the last one most.
    private func fade(_ cut: Bool) -> some View {
        LinearGradient(
            stops: cut ? [.init(color: .black, location: 0.7), .init(color: .black.opacity(0.3), location: 1)] : [.init(color: .black, location: 0), .init(color: .black, location: 1)],
            startPoint: .top, endPoint: .bottom)
    }

    private func open() {
        note.collapsed = false
        editing = true
        focused = true
    }
}

// What the diff says where it shows no lines (`features/git/diff/notes.rs`): a chip on the card's surface, so it reads over the
// hatch, with its detail under it in quiet figures (a binary file's sizes, what an LFS pointer stands for, the object to copy);
// and the card of a patch too large to load unasked.

import DesignSystem
import PanelModel
import SwiftUI

/// A column's note: its chip and detail, and over an LFS object's detail a button that copies the whole id.
struct SideNoteView: View {
    let note: SideNote
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host

    var body: some View {
        VStack(spacing: Space._2) {
            chip(note.text)
            if let detail = note.detail {
                HStack(spacing: Space._1) {
                    Text(detail).font(TextRole.caption.font).monospacedDigit().foregroundStyle(tokens.textMuted).lineLimit(1)
                    // Beside the text, so the note stays where it is; it takes no room until the pointer is over the line.
                    if let oid = note.oid {
                        BandButton(symbol: "doc.on.doc", label: panelText("diff-copy-oid", "Copy object id"), inRow: true) {
                            host?.copy(oid, as: .objectId)
                            host?.notify(panelText("notice-copied-object-id", "Copied object id"))
                        }
                        .opacity(hovering ? 1 : 0)
                    }
                }
                .onHover { hovering = $0 }
            }
        }
    }

    private func chip(_ text: String) -> some View {
        Text(text).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._2).padding(.vertical, Space._1)
            .background(tokens.surfaceWindow, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
    }
}

/// One plain line of words on the card: reading, a failure, a submodule's commits.
struct DiffNote: View {
    let text: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
            .padding(.horizontal, Space._2).padding(.vertical, Space._1)
            .background(tokens.surfaceWindow, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
            .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
    }

    /// The note for a file with no rows when core gave no notes (the sample, a submodule): why, from the diff and the change.
    static func text(for file: ChangedFile, diff: DiffFile?) -> String? {
        if let note = diff?.note { return note }
        guard diff?.hunks.isEmpty ?? true else { return nil }
        if diff?.isBinary == true { return "Binary file" }
        return file.kind == .added || file.kind == .untracked ? "Empty file" : "No content changes"
    }
}

/// In place of a diff too large to load unasked: how big it is and how many lines changed, then Load diff, and Reveal in
/// Finder for a file on disk (`features/git/diff/notes.rs` `render_too_large`).
struct TooLargeCard: View {
    let large: TooLarge
    let path: String
    let revealLabel: String
    let load: () -> Void
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host
    @Environment(\.panelRoot) private var root

    var body: some View {
        VStack(spacing: Space._3) {
            Text(large.title).font(TextRole.body.font).foregroundStyle(tokens.textPrimary)
            Text(large.detail).font(TextRole.caption.font).foregroundStyle(tokens.textMuted).multilineTextAlignment(.center)
            HStack(spacing: Space._2) {
                NoticeButton(title: panelText("diff-load-anyway", "Load diff"), primary: true, action: load)
                if let resolved = root.flatMap({ RepoFiles.resolve(root: $0, relative: path) }) {
                    NoticeButton(title: revealLabel) { host?.reveal(resolved) }
                }
            }
        }
        .padding(Space._4).frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

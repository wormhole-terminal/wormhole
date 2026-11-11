// What the last save, export or copy did, under the title row: in the status colours, with Reveal in Finder for
// a file that was written, and what a copy waiting for its name asks for (`render_notice`).

import AppKit
import DesignSystem
import StudioModel
import SwiftUI

struct NoticeRow: View {
    @Bindable var session: StudioSession
    @Environment(\.tokens) private var tokens

    var body: some View {
        if session.renaming {
            row(text: StudioWords.namePrompt, failed: false, file: nil)
        } else if let notice = session.document.notice {
            row(text: notice.text, failed: notice.isFailure, file: notice.file)
        }
    }

    private func row(text: String, failed: Bool, file: URL?) -> some View {
        HStack(spacing: Space._2) {
            Image(systemName: failed ? "exclamationmark.triangle.fill" : "checkmark.circle.fill")
                .foregroundStyle(failed ? tokens.statusWarning : tokens.statusSuccess)
            Text(text).font(TextRole.caption.font).foregroundStyle(failed ? tokens.statusWarningText : tokens.textMuted)
                .lineLimit(2).textSelection(.enabled)
            Spacer(minLength: 0)
            if let file {
                Button(StudioWords.reveal) { NSWorkspace.shared.activateFileViewerSelecting([file]) }
                    .buttonStyle(.plain).font(TextRole.caption.font).foregroundStyle(tokens.textLink)
                // A saved pair that does not yet follow the system's light and dark offers to.
                if session.document.canFollowSystem {
                    Button(StudioWords.useWithMatchSystem) { session.document.followSystem() }
                        .buttonStyle(.plain).font(TextRole.caption.font).foregroundStyle(tokens.textLink)
                }
            }
            // A file from a newer Wormhole is never written back; a copy of it is.
            if failed, session.document.isNewer {
                Button(StudioWords.newerDuplicate) { session.duplicate() }
                    .buttonStyle(.plain).font(TextRole.caption.font).foregroundStyle(tokens.textLink)
            }
        }
        .padding(.horizontal, Space._4).padding(.vertical, Space._2)
        .background(failed ? tokens.bannerWarningBackground : tokens.surfaceWindow)
        .hairline(.bottom)
        .transition(.opacity)
        .accessibilityElement(children: .combine)
    }
}

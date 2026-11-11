// The explanation behind a warning's ⓘ (`notice/info.rs`): a popover under the button, in plain words, with what went wrong in
// the error's own words (to select, or copy, and report) and a way to the logs. Only a terminal that could not get a session
// host has one; the other warnings keep their ⓘ with nothing more to say, as in the GPUI app.

import AppKit
import DesignSystem
import SwiftUI

struct NoticeInfoView: View {
    /// What the error said, already one short line (core's `reason`); `nil`: nothing to report.
    let reason: String?
    let openLogs: () -> Void
    @Environment(\.tokens) private var tokens
    @State private var copied = false

    /// How long "Copy reason" says "Copied" (`COPIED_FOR`).
    private static let copiedFor = Duration.milliseconds(1200)
    private static let width: CGFloat = 320

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            Text(L10n.noticeInfoHostHeading).font(.system(size: TextRole.heading.size, weight: .semibold))
            line(L10n.noticeInfoHostWhat)
            line(L10n.noticeInfoHostOthers)
            line(L10n.noticeInfoHostTry)
            if let reason {
                line(L10n.noticeInfoHostReason)
                Text(reason)
                    .font(.system(size: TextRole.caption.size, design: .monospaced))
                    .textSelection(.enabled)
                    .padding(Space._2)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(tokens.surfaceElevated, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusSm))
            }
            HStack(spacing: Space._2) {
                button("folder", L10n.noticeInfoLogs, openLogs)
                if let reason {
                    button(copied ? "checkmark" : "doc.on.doc", copied ? L10n.noticeInfoCopied : L10n.noticeInfoCopy) { copy(reason) }
                }
            }
        }
        .padding(Space._4)
        .frame(width: Self.width, alignment: .leading)
        .foregroundStyle(tokens.textPrimary)
    }

    private func line(_ text: String) -> some View {
        Text(text).font(.system(size: TextRole.body.size)).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
    }

    private func button(_ symbol: String, _ title: String, _ action: @escaping () -> Void) -> some View {
        Button(action: action) { Label(title, systemImage: symbol).font(.system(size: TextRole.body.size)) }
    }

    private func copy(_ reason: String) {
        Pasteboard.setText(reason)
        copied = true
        Task {
            try? await Task.sleep(for: Self.copiedFor)
            copied = false
        }
    }
}

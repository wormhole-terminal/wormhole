// The opened commit's block under its row (`features/git/history/details_view.rs`): the subject in full with the
// button that copies the message, the message (compact, with "Show more" when something is left out), the refs
// that point at it, then under a rule who wrote it and when, its hash with the copy buttons, its parents as links
// and how much it changed. One level in (x 32), caption text. What it says comes finished from the core
// (`CommitDetails`); this lays it out.

import AppKit
import DesignSystem
import PanelModel
import SwiftUI

struct CommitDetailsView: View {
    let commit: Commit
    let details: CommitDetails?
    let error: String?
    let store: HistoryStore
    /// The parents the log lists: only those can be opened from here.
    let listed: Set<String>
    let open: (String) -> Void
    @State private var whole = false
    @State private var width: CGFloat = 0
    @Environment(\.panelCopy) private var copy
    @Environment(\.panelHost) private var host
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: PanelMetrics.gap) {
            HStack(alignment: .top, spacing: PanelMetrics.gap) {
                Text(details?.subject ?? commit.subject).font(TextRole.body.font.weight(.semibold)).foregroundStyle(tokens.textPrimary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                copyButton(panelText("history-copy-message", "Copy message"), .message)
            }
            message
            if let details, !details.refs.isEmpty { refs(details) }
            facts
        }
        .font(TextRole.caption.font)
        .onGeometryChange(for: CGFloat.self, of: \.size.width) { width = $0 }
        .padding(.leading, PanelMetrics.inset + Metric.indent).padding(.trailing, PanelMetrics.inset)
        .padding(.vertical, Space._2)
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(details?.spoken ?? "\(commit.short), \(commit.subject), \(commit.author)")
        .contextMenu { CommitCopies(commit: commit, store: store) }
    }

    @ViewBuilder private var message: some View {
        if let details {
            let compact = store.compact(lineChars: max(CaptionMeasure.characters(in: width), 16))
            let more = compact?.more ?? false
            if whole {
                if !details.whole.isEmpty { line(details.whole, tokens.textPrimary) }
                if !details.trailers.isEmpty { line(details.trailers, tokens.textMuted) }
                if let note = details.continues { line(note, tokens.textMuted) }
                if let note = details.tooLong { line(note, tokens.textMuted) }
            } else if let text = compact?.text, !text.isEmpty {
                line(text, tokens.textPrimary)
            }
            if more { ShowMore(whole: whole) { whole.toggle() } }
        } else if let error {
            line(error, tokens.statusDangerText)
        } else {
            line(panelText("history-reading-details", "Reading details\u{2026}"), tokens.textMuted)
        }
    }

    private func line(_ text: String, _ color: Color) -> some View {
        Text(text).foregroundStyle(color).frame(maxWidth: .infinity, alignment: .leading)
    }

    private func refs(_ details: CommitDetails) -> some View {
        HStack(spacing: Space._1) {
            CommitRefs(refs: details.refs)
            if let more = details.refsMore { Text(more).foregroundStyle(tokens.textMuted) }
        }
    }

    private var facts: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            if let details {
                ForEach(Array(details.people.enumerated()), id: \.offset) { _, fact in CommitFact(label: fact.label) { Text(styled(fact)) } }
            } else {
                CommitFact(label: panelText("history-author", "Author")) { Text(commit.author) }
            }
            CommitFact(label: panelText("history-commit", "Commit")) {
                Text(commit.short).font(.system(size: TextRole.caption.size, design: .monospaced))
            } trailing: {
                HStack(spacing: 0) {
                    copyButton(panelText("history-copy-hash", "Copy hash"), .hash)
                    BandMenu(symbol: "ellipsis", label: panelText("history-more-copy", "More ways to copy")) { CommitCopies(commit: commit, store: store) }
                }
            }
            if let details, !details.parents.isEmpty { parents(details) }
            if let details { CommitFact(label: details.changes.label) { Text(styled(details.changes)) } }
        }
        .padding(.top, Space._2)
        .overlay(alignment: .top) { Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth) }
    }

    private func parents(_ details: CommitDetails) -> some View {
        CommitFact(label: details.parentsLabel) {
            HStack(spacing: PanelMetrics.gap) {
                ForEach(details.parents, id: \.sha) { parent in
                    let mono = Font.system(size: TextRole.caption.size, design: .monospaced)
                    if listed.contains(parent.sha) {
                        Button(parent.short) { open(parent.sha) }.buttonStyle(.plain).font(mono).foregroundStyle(tokens.textLink)
                            .accessibilityLabel(parent.spoken)
                    } else {
                        Text(parent.short).font(mono)
                    }
                }
                if let more = details.parentsMore { Text(more).foregroundStyle(tokens.textMuted) }
            }
        }
    }

    private func copyButton(_ label: String, _ what: CommitCopy) -> some View {
        BandButton(symbol: "doc.on.doc", label: label, inRow: true) {
            Task { if let text = await store.text(of: what, for: commit) { copy(text, store.copied(what, for: commit)) } else { host?.failed(.copyCommit) } }
        }
    }

    /// One line of text whose stretches have the colours the core gave them.
    private func styled(_ fact: CommitDetails.Fact) -> AttributedString {
        fact.spans.reduce(into: AttributedString()) { text, span in
            var run = AttributedString(span.text)
            run.foregroundColor = switch span.tone {
            case .plain: tokens.textPrimary
            case .muted: tokens.textMuted
            case .added: tokens.diffAdded
            case .removed: tokens.diffRemoved
            }
            text += run
        }
    }
}

/// About how many characters of caption text fit in `width`: the width of a line of ordinary words, divided out, a
/// little under it so that what is cut or offered as "more" errs on the side of offering (GPUI `measure::caption_chars`).
@MainActor private enum CaptionMeasure {
    static func characters(in width: CGFloat) -> Int {
        let sample = "The quick brown fox jumps over"
        let wide = sample.size(withAttributes: [.font: NSFont.systemFont(ofSize: TextRole.caption.size)]).width
        return Int(width / max(wide / CGFloat(sample.count), 1) * 0.95)
    }
}

private struct ShowMore: View {
    let whole: Bool
    let action: () -> Void
    @State private var hovering = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: action) {
            HStack(spacing: Space._1) {
                Text(whole ? panelText("history-show-less", "Show less") : panelText("history-show-more", "Show more"))
                PanelIcon(whole ? "chevron.up" : "chevron.down", size: 12)
            }
            .foregroundStyle(hovering ? tokens.textLink : tokens.textMuted)
            .padding(.horizontal, Space._1).frame(height: Metric.controlSmall)
            .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
        }
        .buttonStyle(.plain).onHover { hovering = $0 }
    }
}

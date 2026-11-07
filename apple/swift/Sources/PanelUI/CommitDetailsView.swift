// The opened commit's block under its row (`features/git/history/details_view.rs`): the subject in full with the
// button that copies the message, the message (compact, with "Show more" when something is left out), the refs
// that point at it, then under a rule who wrote it and when, its hash with the copy buttons, its parents as links
// and how much it changed. One level in (x 32), caption text.

import DesignSystem
import PanelModel
import SwiftUI

struct CommitDetailsView: View {
    let commit: Commit
    let now: Int
    let copy: (String) -> Void
    let open: (String) -> Void
    @State private var whole = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: PanelMetrics.gap) {
            HStack(alignment: .top, spacing: PanelMetrics.gap) {
                Text(commit.subject).font(TextRole.body.font.weight(.semibold)).foregroundStyle(tokens.textPrimary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                BandButton(symbol: "doc.on.doc", label: "Copy message", inRow: true) { copy(commit.subject + "\n\n" + commit.body) }
            }
            message
            if !commit.refs.isEmpty { CommitRefs(refs: commit.refs) }
            facts
        }
        .font(TextRole.label.font.weight(.regular))
        .padding(.leading, PanelMetrics.inset + Metric.indent).padding(.trailing, PanelMetrics.inset)
        .padding(.vertical, Space._2)
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Details of \(commit.short), by \(commit.author.name)")
    }

    @ViewBuilder private var message: some View {
        let lines = commit.body.split(separator: "\n", omittingEmptySubsequences: false)
        if !lines.isEmpty {
            let more = lines.count > 3
            Text(more && !whole ? lines.prefix(3).joined(separator: "\n") : commit.body)
                .foregroundStyle(tokens.textPrimary).frame(maxWidth: .infinity, alignment: .leading)
            if more { ShowMore(whole: whole) { whole.toggle() } }
        }
    }

    private var facts: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            CommitFact(label: "Author") {
                VStack(alignment: .leading, spacing: 0) {
                    Text(parts([(commit.author.name, tokens.textPrimary), (" \(commit.author.email)", tokens.textMuted)]))
                    Text("\(commit.author.stamp) · \(Age.words(seconds: now - commit.author.time))").foregroundStyle(tokens.textMuted)
                }
            }
            CommitFact(label: "Commit") {
                Text(commit.short).font(.system(size: TextRole.label.size, design: .monospaced))
            } trailing: {
                HStack(spacing: 0) {
                    BandButton(symbol: "doc.on.doc", label: "Copy hash", inRow: true) { copy(commit.sha) }
                    BandButton(symbol: "ellipsis", label: "More ways to copy", inRow: true) { copy(commit.short) }
                }
            }
            if !commit.parents.isEmpty {
                CommitFact(label: commit.parents.count == 1 ? "Parent" : "Parents") {
                    HStack(spacing: PanelMetrics.gap) {
                        ForEach(commit.parents, id: \.self) { parent in
                            Button(String(parent.prefix(7))) { open(parent) }.buttonStyle(.plain)
                                .font(.system(size: TextRole.label.size, design: .monospaced)).foregroundStyle(tokens.textLink)
                                .accessibilityLabel("Open parent \(parent.prefix(7))")
                        }
                    }
                }
            }
            if !commit.files.isEmpty { changes }
        }
        .padding(.top, Space._2)
        .overlay(alignment: .top) { Rectangle().fill(tokens.border).frame(height: tokens.metrics.borderWidth) }
    }

    /// One line of text whose stretches have colours of their own.
    private func parts(_ pieces: [(String, Color)]) -> AttributedString {
        pieces.reduce(into: AttributedString()) { text, piece in
            var run = AttributedString(piece.0)
            run.foregroundColor = piece.1
            text += run
        }
    }

    private var changes: some View {
        let summary = commit.changeSummary
        return CommitFact(label: "Changes") {
            Text(parts([(summary.files + " ", tokens.textPrimary), (summary.added + " ", tokens.diffAdded), (summary.removed, tokens.diffRemoved)]))
        }
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
                Text(whole ? "Show less" : "Show more")
                Image(systemName: whole ? "chevron.up" : "chevron.down").font(.system(size: 10, weight: .medium))
            }
            .foregroundStyle(hovering ? tokens.textLink : tokens.textMuted)
            .padding(.horizontal, Space._1).frame(height: Metric.controlSmall)
            .background(hovering ? tokens.stateHover : .clear, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
        }
        .buttonStyle(.plain).onHover { hovering = $0 }
    }
}

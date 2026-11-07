// The window behind the trust notice's "View all N…": every line a project wants to run, in full, with what runs
// set apart in its own colour, and the same approval the notice offers, so a long list is reviewed and approved in
// one place (`trust_window.rs`). Built like About: opaque, one at a time, closed with Esc or ⌘W.

import AuxModel
import DesignSystem
import SwiftUI

struct TrustView: View {
    let model: TrustViewModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        let review = model.review
        VStack(spacing: 0) {
            header(review)
            Rule()
            content(review)
            Rule()
            footer(review)
        }
        .auxPage()
        .onAppear { model.start() }
        .onDisappear { model.stop() }
    }

    private func header(_ review: TrustReview) -> some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._2) {
                Image(systemName: "folder").foregroundStyle(tokens.textMuted)
                Text("\(review.name)/\(review.directory)").font(TextRole.heading.font).lineLimit(1).truncationMode(.middle)
                    .accessibilityAddTraits(.isHeader)
                Spacer()
                StatusBadge(text: review.status.badge, color: color(review.status))
            }
            Text(review.summary).font(TextRole.body.font).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
        }
        .padding(.horizontal, Space._6).padding(.bottom, Space._3)
    }

    @ViewBuilder private func content(_ review: TrustReview) -> some View {
        if let problem = review.problem {
            Text(problem).font(TextRole.body.font).foregroundStyle(tokens.statusDangerText).padding(Space._6)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        } else if review.lines.isEmpty {
            Text(AuxText.trustNothing).font(TextRole.body.font).foregroundStyle(tokens.textMuted).padding(Space._6)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        } else {
            List(review.lines) { line in
                TrustLineView(line: line)
                    .listRowSeparator(.hidden).listRowInsets(EdgeInsets(top: Space._1, leading: Space._6, bottom: Space._1, trailing: Space._6))
            }
            .listStyle(.plain).scrollContentBackground(.hidden)
        }
    }

    private func footer(_ review: TrustReview) -> some View {
        HStack {
            Text(AuxText.trustLines(review.lines.count)).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
            Spacer()
            if review.canApprove {
                Button(model.approveLabel) { model.approve() }.primaryAction().controlSize(.small)
                    .disabled(review.approving).keyboardShortcut(.defaultAction)
            }
        }
        .padding(.horizontal, Space._6).frame(height: Metric.titleRow)
    }

    private func color(_ status: TrustStatus) -> Color {
        switch status {
        case .trusted: tokens.statusSuccessText
        case .changed: tokens.statusWarningText
        case .untrusted: tokens.textMuted
        }
    }
}

/// "action test (key ⌃⌘T): run `cargo test` (current)": what runs set apart in `trust.command`.
private struct TrustLineView: View {
    let line: TrustLine
    @Environment(\.tokens) private var tokens

    var body: some View {
        var text = AttributedString(line.what)
        if !line.runs.isEmpty {
            var runs = AttributedString(" " + line.runs)
            runs.foregroundColor = tokens.trustCommand
            text += runs
        }
        if !line.note.isEmpty { text += AttributedString(" " + line.note) }
        return Text(text).font(.system(size: TextRole.caption.size, design: .monospaced)).foregroundStyle(tokens.textMuted)
            .textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading)
    }
}

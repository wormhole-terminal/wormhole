// The terminals found on this Mac, a card each (`import/found.rs`, `review.rs`): its name and how many settings it says,
// and, opened, the files read, what would be brought over as rows to tick, what does not carry over and why, and
// "Import selected". Nothing is written until that button.

import AppKit
import DesignSystem
import SettingsModel
import SwiftUI
import WordsModel

struct ImportCards: View {
    let model: SettingsModel
    @Environment(\.tokens) private var tokens

    var body: some View {
        let records = model.host.panelRecords()
        if records.scanningImports && records.imports.isEmpty {
            PanelNote(text: Words.text("settings-import-scanning", fallback: "Looking for terminals…"))
        } else if records.imports.isEmpty {
            none
        } else {
            VStack(spacing: 0) {
                ForEach(Array(records.imports.enumerated()), id: \.element.id) { index, record in
                    if index > 0 { Rectangle().fill(tokens.settingsDivider).frame(height: tokens.metrics.borderWidth) }
                    ImportCard(draft: model.importDraft, host: model.host, record: record)
                }
            }
        }
    }

    /// What was looked for, so the page does not read as broken.
    private var none: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            Image(systemName: "magnifyingglass").foregroundStyle(tokens.textMuted)
            Text(Words.text("settings-import-none", fallback: "No other terminal's settings found"))
                .font(.system(size: TextRole.body.size, weight: .semibold))
            Text(Words.text("settings-import-none-detail", fallback: "Wormhole looked for Ghostty, iTerm2, Terminal.app, Alacritty, kitty, WezTerm, Warp, Zed and VS Code in the places they keep their settings, and found none."))
                .font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
        }
        .frame(maxWidth: .infinity, alignment: .leading).padding(.vertical, Space._3)
        .accessibilityElement(children: .combine)
    }
}

private struct ImportCard: View {
    let draft: ImportDraft
    let host: any SettingsHost
    let record: ImportRecord
    @Environment(\.tokens) private var tokens

    var body: some View {
        let open = draft.open == record.id
        VStack(alignment: .leading, spacing: Space._3) {
            Button { draft.toggle(record.id) } label: {
                HStack(spacing: Space._2) {
                    Text(record.app).font(.system(size: TextRole.body.size, weight: .semibold)).foregroundStyle(tokens.textPrimary)
                    Text(record.summary).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
                        .padding(.horizontal, Space._2).background(tokens.controlBackground, in: Capsule())
                    Spacer(minLength: 0)
                    Image(systemName: open ? "chevron.up" : "chevron.down").foregroundStyle(tokens.textMuted)
                }
                .frame(minHeight: Metric.control).contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("\(record.app), \(record.summary)")
            .accessibilityAddTraits(.isButton)
            .accessibilityValue(open ? "expanded" : "collapsed")
            if open, let review = draft.reviews[record.id] { ImportReviewView(draft: draft, host: host, id: record.id, review: review) }
        }
        .padding(.vertical, Space._2)
        .motion(Motion.layout, value: open)
    }
}

private struct ImportReviewView: View {
    let draft: ImportDraft
    let host: any SettingsHost
    let id: String
    let review: ImportReview
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            label(Words.text("settings-import-files-read", fallback: "Files read"))
            ForEach(review.files, id: \.self) { file in mono((file as NSString).abbreviatingWithTildeInPath).foregroundStyle(tokens.textMuted) }
            label(Words.text("settings-import-take", fallback: "Brings over"))
            if review.items.isEmpty {
                Text(Words.text("settings-import-nothing-to-take", fallback: "Nothing in these settings can be brought over."))
                    .font(.system(size: TextRole.body.size)).foregroundStyle(tokens.textMuted)
            }
            ForEach(Array(review.items.enumerated()), id: \.offset) { index, item in row(index, item) }
            if !review.skipped.isEmpty {
                label(Words.text("settings-import-leave", fallback: "Doesn't carry over")).padding(.top, Space._2)
                ForEach(Array(review.skipped.enumerated()), id: \.offset) { _, skip in
                    VStack(alignment: .leading, spacing: Space._1) {
                        mono(skip.key).foregroundStyle(tokens.textMuted)
                        Text(skip.reason).font(.system(size: TextRole.body.size))
                    }
                    .padding(.vertical, Space._1)
                    .accessibilityElement(children: .combine)
                }
            }
            footer
        }
    }

    private func label(_ text: String) -> some View {
        Text(text).font(.system(size: TextRole.caption.size, weight: .semibold)).foregroundStyle(tokens.textMuted)
            .accessibilityAddTraits(.isHeader)
    }

    private func mono(_ text: String) -> some View {
        Text(text).font(.system(size: TextRole.caption.size, design: .monospaced)).lineLimit(1).truncationMode(.middle)
    }

    /// One setting: its tick, name, what the other terminal says and what Wormhole would set. A shell command is taller:
    /// the whole command, and that it runs in every new terminal.
    private func row(_ index: Int, _ item: ImportReview.Item) -> some View {
        VStack(alignment: .leading, spacing: Space._1) {
            HStack(spacing: Space._3) {
                FlatCheckbox(title: item.label, isOn: draft.isTicked(id, index)) { draft.tick(id, index, $0) }
                    .frame(width: SettingsMetric.importLabelWidth, alignment: .leading)
                mono(item.from).foregroundStyle(tokens.textMuted).frame(maxWidth: .infinity, alignment: .leading)
                if item.command == nil {
                    Image(systemName: "arrow.right").foregroundStyle(tokens.textFaint).accessibilityHidden(true)
                    Text(item.value).font(.system(size: TextRole.body.size)).lineLimit(1)
                        .frame(width: SettingsMetric.importValueWidth, alignment: .leading)
                }
            }
            if let command = item.command {
                Text(command).font(.system(size: TextRole.caption.size, design: .monospaced))
                    .padding(.horizontal, Space._2).padding(.vertical, Space._1)
                    .background(tokens.stateHover, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
                    .padding(.leading, Metric.control)
                Label(Words.text("settings-import-runs-everywhere", fallback: "Wormhole would run this in every new terminal, so it isn't ticked until you tick it."), systemImage: "exclamationmark.triangle")
                    .font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.statusWarningText).padding(.leading, Metric.control)
            }
        }
        .frame(minHeight: Metric.row)
    }

    private var footer: some View {
        let selected = draft.selected(id)
        return HStack(spacing: Space._3) {
            ActionButton(title: Words.text("settings-import-apply", fallback: "Import selected"), symbol: "arrow.down.to.line") { draft.apply(id) }
                .disabled(selected == 0)
            Text(host.importSelectedLine(selected: selected, total: review.items.count)).font(.system(size: TextRole.caption.size)).foregroundStyle(tokens.textMuted)
            Spacer(minLength: 0)
            if let outcome = draft.outcome, outcome.id == id {
                Text(outcome.text).font(.system(size: TextRole.body.size))
                    .foregroundStyle(outcome.failed ? tokens.statusDangerText : tokens.statusSuccessText)
            }
        }
        .padding(.top, Space._2)
        .overlay(alignment: .top) { Rectangle().fill(tokens.settingsDivider).frame(height: tokens.metrics.borderWidth) }
    }
}

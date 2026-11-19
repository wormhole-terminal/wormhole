// The diff's header row (`features/git/diff/header.rs`): where the file sits among the changed ones with the arrows
// that step through them, its name, what happened to it (New file, Deleted, Renamed from x, Binary, LFS) as badges,
// its counts in the code font, the wrap button, and with room for a split view the layout switch and "esc close";
// narrower, a × alone. 44 tall with the switch, 40 without.

import DesignSystem
import PanelModel
import SwiftUI

struct DiffHeader: View {
    let file: ChangedFile
    let diff: DiffFile?
    let position: (index: Int, count: Int)?
    let splitFits: Bool
    @Binding var split: Bool
    @Binding var wrap: Bool
    let step: (Int) -> Void
    let close: () -> Void
    /// The file is no longer in the status ("No longer changed"); the last re-read failed ("Out of date", with why).
    var gone = false
    var outOfDate: String?
    /// From History: the commit's short hash, after "File N of M" in the primary.
    var commit: String?
    /// In the terminal's title row the row gives the height and the insets.
    var inTitleRow = false
    @Environment(\.tokens) private var tokens
    @Environment(\.panelHost) private var host
    @Environment(\.panelRoot) private var root

    var body: some View {
        HStack(spacing: PanelMetrics.gap) {
            BandButton(symbol: "chevron.left", label: panelText("diff-previous-file", "Previous file"), inRow: true) { step(-1) }
            BandButton(symbol: "chevron.right", label: panelText("diff-next-file", "Next file"), inRow: true) { step(1) }
            if let position {
                Text(panelText("diff-file-position", ["index": .whole(position.index + 1), "count": .whole(position.count)], "File \(String(position.index + 1)) of \(String(position.count))")).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted).fixedSize()
            }
            if let commit { Text(commit).font(tokens.font(.caption)).foregroundStyle(tokens.primary).fixedSize() }
            // The path crumb: the file's icon, its folders quiet and its name loud. It gives way first, down to
            // nothing, so the badges and controls at the end stay whole (the GPUI title row elides it the same way).
            FileIcon(kind: .file(file.name), tint: tokens.textMuted)
            crumb.lineLimit(1).truncationMode(.head).layoutPriority(-1).clipped()
            Spacer(minLength: 0)
            ForEach(badges, id: \.text) { badge in StatusBadge(text: badge.text, color: badge.color) }
            if let outOfDate { StatusBadge(text: panelText("diff-badge-out-of-date", "Out of date"), color: tokens.statusWarningText).help(outOfDate) }
            // Counts and the wrap button are for lines: not a binary file, an LFS pointer or a patch not loaded.
            if case let .counts(added, removed) = file.lines, diff?.isLFS != true, diff?.isBinary != true, diff?.tooLarge == nil {
                HStack(spacing: Space._1) {
                    Text("+\(CountText.grouped(added))").foregroundStyle(tokens.diffAdded)
                    Text("\u{2212}\(CountText.grouped(removed))").foregroundStyle(tokens.diffRemoved)
                }
                .font(.system(size: tokens.size(.caption), design: .monospaced)).fixedSize()
                // A toggle in the selected tint while on, as GPUI's `icon_button(...).selected(wrap)`.
                BandButton(symbol: "text.append", label: panelText("diff-wrap-lines", "Wrap long lines"), inRow: true, selected: wrap) { wrap.toggle() }
            }
            // The file in the chosen editor, when it is on disk (a deleted file has nothing to open).
            if let on = onDisk, let open = host?.openTooltip(line: nil) {
                BandButton(symbol: "arrow.up.forward.square", label: open, inRow: true, spoken: BandButton.openFile(open, file: file.name)) {
                    host?.open(on, line: nil)
                }
            }
            if splitFits {
                Segmented([(panelText("diff-split", "Split"), true), (panelText("diff-unified", "Unified"), false)], selection: $split)
                Button(action: close) {
                    HStack(spacing: Space._1) {
                        KeyCap(text: "esc", height: Metric.titleControl)
                        Text(panelText("diff-close", "close")).font(tokens.font(.caption)).foregroundStyle(tokens.textMuted)
                    }.padding(.horizontal, Space._2).frame(height: Metric.hitMin).contentShape(Rectangle())
                }.buttonStyle(.plain).accessibilityLabel(panelText("diff-close-spoken", "Close diff"))
            } else {
                BandButton(symbol: "xmark", label: panelText("diff-close-tooltip", "Close diff (esc)"), inRow: true, action: close)
            }
        }
        .padding(.horizontal, inTitleRow ? 0 : PanelMetrics.inset)
        .frame(height: inTitleRow ? nil : splitFits ? 44 : 40)
    }

    private var onDisk: String? { root.flatMap { RepoFiles.resolve(root: $0, relative: file.path) } }

    private var crumb: Text {
        let folders = file.path.split(separator: "/", omittingEmptySubsequences: false).dropLast().joined(separator: "/")
        let quiet = Text(folders.isEmpty ? "" : folders + "/").foregroundStyle(tokens.textMuted)
        return Text("\(quiet)\(Text(file.name).foregroundStyle(tokens.textPrimary))").font(tokens.font(.body))
    }

    private var badges: [(text: String, color: Color)] {
        Self.badges(file: file, diff: diff, gone: gone).map { ($0.text, tokens[keyPath: $0.tone]) }
    }

    /// The badges' words in the header's order, each with the token it is drawn in (the control folder's `diff-header` reads
    /// the same list).
    static func badges(file: ChangedFile, diff: DiffFile?, gone: Bool) -> [(text: String, tone: KeyPath<ThemeTokens, Color>)] {
        var out: [(String, KeyPath<ThemeTokens, Color>)] = []
        if file.kind == .added || file.kind == .untracked { out.append((panelText("diff-badge-new", "New file"), \.statusSuccessText)) }
        if file.kind == .deleted { out.append((panelText("changes-kind-deleted", "Deleted"), \.statusDangerText)) }
        if file.kind == .renamed {
            // The old name whenever the old path is known, also for a move that kept the name.
            let old = file.originalPath.map { $0.split(separator: "/").last.map(String.init) ?? $0 }
            out.append((old.map { panelText("diff-badge-renamed-from", ["path": .text($0)], "Renamed from \($0)") } ?? panelText("changes-kind-renamed", "Renamed"), \.primary))
        }
        if let words = file.submodule { out.append((words, \.textMuted)) }
        if gone { out.append((panelText("diff-badge-gone", "No longer changed"), \.textMuted)) }
        if diff?.isBinary == true { out.append((panelText("diff-badge-binary", "Binary"), \.textMuted)) }
        if file.lfs || diff?.isLFS == true { out.append(("LFS", \.textMuted)) } // text: Git LFS, a product name
        if let mode = diff?.modeChange { out.append((mode, \.textMuted)) }
        if diff?.noHighlight == true { out.append((panelText("diff-badge-no-highlighting", "No highlighting"), \.textMuted)) }
        return out
    }
}

/// A short status word in a colour (`style::status_badge`): outlined in it, the word in it, no fill, in the chip corners.
struct StatusBadge: View {
    let text: String
    let color: Color
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text).font(tokens.font(.caption)).foregroundStyle(color).lineLimit(1).fixedSize()
            .padding(.horizontal, Space._2).frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .overlay { RoundedRectangle(cornerRadius: tokens.metrics.radiusChip).strokeBorder(color, lineWidth: tokens.metrics.borderWidth) }
    }
}

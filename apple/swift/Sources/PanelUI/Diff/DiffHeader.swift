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
    /// In the terminal's title row the row gives the height and the insets.
    var inTitleRow = false
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(spacing: PanelMetrics.gap) {
            BandButton(symbol: "chevron.left", label: "Previous file", inRow: true) { step(-1) }
            BandButton(symbol: "chevron.right", label: "Next file", inRow: true) { step(1) }
            if let position, splitFits {
                Text("File \(position.index + 1) of \(position.count)").font(TextRole.caption.font).foregroundStyle(tokens.textMuted).fixedSize()
            }
            Text(file.name).font(TextRole.body.font).foregroundStyle(tokens.textPrimary).lineLimit(1).truncationMode(.middle)
            Spacer(minLength: 0)
            ForEach(badges, id: \.text) { badge in StatusBadge(text: badge.text, color: badge.color) }
            if case let .counts(added, removed) = file.lines {
                HStack(spacing: Space._1) {
                    Text("+\(CountText.grouped(added))").foregroundStyle(tokens.diffAdded)
                    Text("\u{2212}\(CountText.grouped(removed))").foregroundStyle(tokens.diffRemoved)
                }
                .font(.system(size: TextRole.caption.size, design: .monospaced)).fixedSize()
                BandButton(symbol: "text.append", label: "Wrap long lines", inRow: true) { wrap.toggle() }
                    .opacity(wrap ? 1 : 0.6)
            }
            if splitFits {
                Segmented([("Split", true), ("Unified", false)], selection: $split)
                Button(action: close) {
                    HStack(spacing: Space._1) {
                        KeyCap(text: "esc", height: Metric.titleControl)
                        Text("close").font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    }.padding(.horizontal, Space._2).frame(height: Metric.hitMin).contentShape(Rectangle())
                }.buttonStyle(.plain).accessibilityLabel("Close diff")
            } else {
                BandButton(symbol: "xmark", label: "Close diff (esc)", inRow: true, action: close)
            }
        }
        .padding(.horizontal, inTitleRow ? 0 : PanelMetrics.inset)
        .frame(height: inTitleRow ? nil : splitFits ? 44 : 40)
    }

    private var badges: [(text: String, color: Color)] {
        var out: [(String, Color)] = []
        if file.kind == .added || file.kind == .untracked { out.append(("New file", tokens.statusSuccessText)) }
        if file.kind == .deleted { out.append(("Deleted", tokens.statusDangerText)) }
        if file.kind == .renamed {
            out.append((file.renamedFrom.map { "Renamed from \($0)" } ?? "Renamed", tokens.primary))
        }
        if diff?.isBinary == true { out.append(("Binary", tokens.textMuted)) }
        if file.lfs { out.append(("LFS", tokens.textMuted)) }
        return out
    }
}

/// A short status word in a colour (`style::status_badge`): the word on a 15 % tint of it, in the chip corners.
struct StatusBadge: View {
    let text: String
    let color: Color
    @Environment(\.tokens) private var tokens

    var body: some View {
        Text(text).font(TextRole.caption.font).foregroundStyle(color).lineLimit(1).fixedSize()
            .padding(.horizontal, Space._2).frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
            .background(color.opacity(0.15), in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
    }
}

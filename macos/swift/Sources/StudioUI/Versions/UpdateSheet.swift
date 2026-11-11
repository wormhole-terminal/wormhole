// Update from the theme this one was copied from: what changed in it since (key, the value that was copied, the value
// now), the rows that are the theme's own marked "yours · kept" because updating leaves them as they are, and Update, which
// replaces the base as one undoable step (`studio/update_from.rs`).

import DesignSystem
import StudioModel
import SwiftUI

struct UpdateSheet: View {
    let check: UpdateCheck
    let apply: () -> Void
    let cancel: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            Text(StudioWords.updateTitle(check.from)).font(TextRole.heading.font)
            Text(StudioWords.updateCount(check.changes.count, check.date)).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            ScrollView {
                VStack(spacing: 0) { ForEach(check.changes) { row($0) } }
            }
            .frame(maxHeight: 320)
            HStack {
                Spacer()
                Button(StudioWords.cancel, action: cancel).keyboardShortcut(.cancelAction)
                Button(StudioWords.updateApply, action: apply).keyboardShortcut(.defaultAction)
            }
        }
        .padding(Space._4).frame(width: 520).background(tokens.surfaceElevated)
        .accessibilityElement(children: .contain)
    }

    private func row(_ change: UpdateChange) -> some View {
        HStack(spacing: Space._2) {
            Text(change.key).font(TextRole.caption.font.monospaced()).lineLimit(1).truncationMode(.middle).frame(width: 150, alignment: .leading)
            value(change.old)
            Image(systemName: "arrow.right").foregroundStyle(tokens.textFaint).font(TextRole.caption.font)
            value(change.new)
            Spacer(minLength: 0)
            if change.kept { StatusBadge(text: StudioWords.updateKept, color: tokens.textFaint) }
        }
        .padding(.vertical, Space._1)
    }

    /// A colour as a chip and its text, any other value as text, nothing as a dash.
    @ViewBuilder private func value(_ value: String?) -> some View {
        if let value {
            HStack(spacing: Space._1) {
                if let color = Self.swatch(value) {
                    RoundedRectangle(cornerRadius: tokens.metrics.radiusSm).fill(color).frame(width: Metric.tileSmall, height: Metric.tileSmall)
                }
                Text(value).font(TextRole.caption.font).foregroundStyle(tokens.textFaint).lineLimit(1).truncationMode(.middle)
            }
        } else {
            Text("\u{2014}").foregroundStyle(tokens.textFaint)
        }
    }

    /// The chip of a value written as `#rrggbb` or `#rrggbbaa`; anything else (an expression) has none.
    private static func swatch(_ value: String) -> Color? {
        guard value.hasPrefix("#"), [7, 9].contains(value.count), let bits = UInt64(value.dropFirst(), radix: 16) else { return nil }
        let long = value.count == 9
        let channel = { (shift: UInt64) in Double((bits >> shift) & 0xFF) / 255 }
        return Color(.sRGB, red: channel(long ? 24 : 16), green: channel(long ? 16 : 8), blue: channel(long ? 8 : 0), opacity: long ? channel(0) : 1)
    }
}

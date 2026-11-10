// The design language's numbers: sizes, insets and motion, each with its value, so a constant that is not a
// component still shows in the Studio (`gallery/metrics.rs`).

import DesignSystem
import SwiftUI

struct MetricsSpecimen: View {
    @Environment(\.tokens) private var tokens

    private static let sizes: [(String, [CGFloat])] = [
        ("control · controlSmall · titleControl", [Metric.control, Metric.controlSmall, Metric.titleControl]),
        ("row · rowTwoLine", [Metric.row, Metric.rowTwoLine]),
        ("band · titleRow · statusBar", [Metric.band, Metric.titleRow, Metric.statusBar]),
        ("edgeInset · lead · indent", [Metric.edgeInset, Metric.lead, Metric.indent]),
        ("glyph · titleGlyph · hitMin", [Metric.glyph, Metric.titleGlyph, Metric.hitMin]),
        ("badgeHeight · keyCapHeight · keyCapMinWidth", [Metric.badgeHeight, Metric.keyCapHeight, Metric.keyCapMinWidth]),
        ("statusDot · statusHalo · ring · chevron", [Metric.statusDot, Metric.statusHalo, Metric.ring, Metric.chevron]),
        ("tile · tileMini · tileSmall", [Metric.tile, Metric.tileMini, Metric.tileSmall]),
        ("palette width · search · row", [Metric.paletteWidth, Metric.paletteSearchHeight, Metric.paletteRowHeight]),
        ("Space _1 · _2 · _3 · _4 · _6", [Space._1, Space._2, Space._3, Space._4, Space._6]),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            table("Sizes", Self.sizes.map { ($0.0, $0.1.map(Self.points).joined(separator: " · ")) })
            table("Motion", Motion.timings.map { ($0.name, "\(Int($0.seconds * 1000)) ms") } + [("pressScale", "\(Motion.pressScale)")])
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func table(_ title: String, _ rows: [(String, String)]) -> some View {
        VStack(alignment: .leading, spacing: Space._1) {
            SectionLabel(title)
            ForEach(rows, id: \.0) { name, value in
                HStack(spacing: Space._4) {
                    Text(name).foregroundStyle(tokens.textMuted).frame(width: 240, alignment: .leading)
                    Text(value).foregroundStyle(tokens.textPrimary).monospacedDigit()
                }
                .font(TextRole.caption.font)
            }
        }
    }

    private static func points(_ value: CGFloat) -> String { NumberFormat.points(Double(value)) }
}

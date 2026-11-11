// The text that heads or fills a panel: a title, a section label open and folded, an empty note, a toolbar of controls
// and a selected tint with its count (`gallery/labels.rs`).

import DesignSystem
import SwiftUI

struct LabelsSpecimen: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            Text("Next steps").font(TextRole.heading.font)
            VStack(alignment: .leading, spacing: 0) {
                SectionLabel("Section label", isCollapsed: false)
                SectionLabel("Folded section label", isCollapsed: true)
            }
            // The line icons the panels draw (`LucideView`): stroked in the foreground, their width scaling with the box.
            HStack(spacing: Space._3) {
                ForEach([Lucide.pin, .link, .note, .check, .x], id: \.self) { LucideView(icon: $0, size: Metric.glyph) }
            }
            .foregroundStyle(tokens.textMuted)
            Text("Nothing here yet.").font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            HStack(spacing: Space._2) {
                Label("main", systemImage: "arrow.triangle.branch").font(TextRole.caption.font)
                Spacer(minLength: 0)
                Text("a select's own text").font(TextRole.caption.font)
            }
            .padding(.horizontal, Space._2).frame(height: Metric.control)
            .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth))
            HStack(spacing: Space._2) {
                Text("Changes").font(TextRole.body.font)
                Text("3").font(TextRole.caption.font.monospacedDigit())
                    .padding(.horizontal, Space._1).frame(minWidth: Metric.badgeHeight, minHeight: Metric.badgeHeight)
                    .background(tokens.stateSelectedInset, in: Capsule())
            }
            .foregroundStyle(tokens.textOnSelected)
            .padding(.horizontal, Space._2).frame(height: Metric.control)
            .background(tokens.stateSelected, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        }
        .frame(maxWidth: 320, alignment: .leading)
    }
}

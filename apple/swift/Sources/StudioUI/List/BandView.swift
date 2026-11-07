// A group's band in the token list: a chevron, its name, its size, and on the right a warning with how many of
// its tokens fall short of a contrast minimum and a dot when something in it was edited since the last save.
// Folds like the sidebar's folder bands (`style::band` with `Section`).

import DesignSystem
import StudioModel
import SwiftUI

struct BandView: View {
    let group: TokenGroup
    let size: Int
    let isOpen: Bool
    /// A filter lists what it found with every band open, so nothing folds then.
    let folds: Bool
    let lowCount: Int
    let changed: Bool
    let toggle: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        Button(action: toggle) {
            HStack(spacing: Space._2) {
                Image(systemName: "chevron.right")
                    .font(.system(size: Metric.chevron, weight: .bold))
                    .rotationEffect(.degrees(isOpen ? 90 : 0))
                    .frame(width: Metric.lead)
                    .opacity(folds ? 1 : 0.4)
                    .motion(Motion.layout, value: isOpen)
                Text(group.name).font(TextRole.label.font)
                CountBadge(size)
                Spacer(minLength: 0)
                if lowCount > 0 {
                    Label("\(lowCount)", systemImage: "exclamationmark.triangle.fill")
                        .labelStyle(WarningLabel(tokens: tokens))
                }
                if changed { ChangedDot() }
            }
            .padding(.horizontal, Metric.edgeInset)
            .frame(height: Metric.band)
            .background(tokens.band)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!folds)
        .foregroundStyle(tokens.textPrimary)
        .accessibilityLabel(group.name)
        .accessibilityValue(isOpen ? "expanded" : "collapsed")
        .accessibilityHint(lowCount > 0 ? "\(lowCount) fall short of a contrast minimum" : "")
    }
}

private struct WarningLabel: LabelStyle {
    let tokens: ThemeTokens

    func makeBody(configuration: Configuration) -> some View {
        HStack(spacing: Space._1) {
            configuration.icon.foregroundStyle(tokens.statusWarning)
            configuration.title.foregroundStyle(tokens.statusWarningText)
        }
        .font(TextRole.caption.font)
    }
}

/// The small dot that says "edited since the last save".
struct ChangedDot: View {
    @Environment(\.tokens) private var tokens

    var body: some View {
        Circle().fill(tokens.primary).frame(width: Metric.statusDot / 2, height: Metric.statusDot / 2)
            .accessibilityLabel("changed")
    }
}

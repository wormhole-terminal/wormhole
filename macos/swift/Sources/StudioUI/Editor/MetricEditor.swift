// A metric: a stepper and a slider instead of the picker, Follow for one whose rule is another metric, and a
// preview of what it shapes (a corner for a radius, an outline for a width). A number of pixels, nothing else.

import DesignSystem
import StudioModel
import SwiftUI

struct MetricEditor: View {
    @Bindable var document: StudioDocument
    let token: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let value = document.tokens.metric(named: token) ?? 0
        VStack(alignment: .leading, spacing: Space._3) {
            HStack(spacing: Space._3) {
                Stepper(value: binding(value), in: 0...StudioMetrics.metricMax, step: 1) {
                    Text(StudioWords.px(NumberText.pixels(value))).font(TextRole.body.font.monospacedDigit())
                }
                Slider(value: binding(value), in: 0...StudioMetrics.metricMax, step: 1).tint(tokens.primary)
                    .accessibilityLabel(token)
            }
            if let source = followed {
                let sourceValue = document.tokens.metric(named: "metrics.\(source)") ?? 0
                Toggle(isOn: Binding(get: { document.file.own(token, in: document.shown) == nil }, set: { follows in
                    document.set(token, to: follows ? "" : NumberText.pixels(value))
                })) { Text(StudioWords.metricFollow(source, NumberText.pixels(sourceValue))).font(TextRole.caption.font) }
                .toggleStyle(.switch).controlSize(.small).foregroundStyle(tokens.textMuted)
            }
            Text(token.hasSuffix("width") ? StudioWords.helpWidth : StudioWords.helpRadius)
                .font(TextRole.caption.font).foregroundStyle(tokens.textFaint).fixedSize(horizontal: false, vertical: true)
            if let error = document.fieldError, error.token == token {
                Text(error.message).font(TextRole.caption.font).foregroundStyle(tokens.statusDanger)
            }
            preview(value)
        }
    }

    /// What the metric shapes, drawn with the draft (the window previews it): the count badge, a key cap, a chip and a
    /// status badge, and how many components draw with the token.
    private func preview(_ value: Double) -> some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack {
                SectionLabel(StudioWords.metricPreview(StudioWords.px(NumberText.pixels(value))))
                Spacer()
                Text(StudioWords.metricUses(document.uses(of: token).count)).font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
            }
            HStack(spacing: Space._2) {
                CountBadge(3)
                KeyCap(text: "⌘1")
                Text("ui-unify").font(TextRole.caption.font).padding(.horizontal, Space._2).frame(height: Metric.badgeHeight)  // text-file: sample content, as the gallery's
                    .background(tokens.chipTint, in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
                StatusBadge(text: "trusted", color: tokens.statusSuccessText)  // text-file: sample content, as the gallery's
            }
            .padding(Space._3)
            .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.border))
        }
    }

    /// The metric a rule names (`radius_chip` follows `radius_sm`), for the Follow switch.
    private var followed: String? {
        document.rule(for: token).flatMap { $0.allSatisfy { $0.isLetter || $0 == "_" } ? $0 : nil }
    }

    private func binding(_ value: Double) -> Binding<Double> {
        Binding(get: { value }, set: { document.set(token, to: NumberText.pixels($0), merging: true) })
    }
}

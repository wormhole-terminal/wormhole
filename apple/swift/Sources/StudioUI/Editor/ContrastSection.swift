// The contrast pairs the selected token is part of, as the text or as the fill: each with a tick or a warning,
// its ratio in the status colour and the minimum it is held to (`contrast_section`).

import DesignSystem
import StudioModel
import SwiftUI

struct ContrastSection: View {
    @Bindable var document: StudioDocument
    let token: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let checks = document.checks(of: token)
        if !checks.isEmpty {
            EditorSection {
                SectionLabel(checks.contains { $0.bg == token } ? StudioWords.contrastOnFill : StudioWords.contrast)
                ForEach(Array(checks.enumerated()), id: \.offset) { _, check in
                    HStack(alignment: .top, spacing: Space._2) {
                        Image(systemName: check.passes ? "checkmark" : "exclamationmark.triangle.fill")
                            .font(TextRole.caption.font).foregroundStyle(check.passes ? tokens.statusSuccess : tokens.statusWarning)
                            .frame(width: Metric.lead)
                        VStack(alignment: .leading, spacing: 0) {
                            Text(check.fg).codeFont().lineLimit(1).truncationMode(.middle)
                            Text(StudioWords.contrastOn(check.bg == token ? token : check.bg)).font(TextRole.caption.font)
                                .foregroundStyle(tokens.textFaint)
                        }
                        Spacer(minLength: 0)
                        RatioText(check: check)
                    }
                    .accessibilityElement(children: .combine)
                    .accessibilityLabel("\(check.fg) \(StudioWords.contrastOn(check.bg)), \(check.passes ? "passes" : "falls short")")
                    .accessibilityValue(String(format: "%.1f to 1, minimum %@", check.ratio, ratioText(check.min)))
                }
            }
        }
    }
}

/// "4.6:1  min 4.5": a ratio in its status colour, and the minimum.
struct RatioText: View {
    let check: ContrastCheck
    @Environment(\.tokens) private var tokens

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: Space._2) {
            Text(String(format: "%.1f:1", check.ratio)).fontWeight(.semibold)
                .foregroundStyle(check.passes ? tokens.statusSuccessText : tokens.statusWarningText)
            Text(StudioWords.contrastMin(ratioText(check.min))).foregroundStyle(tokens.textFaint)
        }
        .font(TextRole.caption.font.monospacedDigit())
    }
}

func ratioText(_ minimum: Double) -> String { minimum == minimum.rounded() ? String(Int(minimum)) : String(minimum) }

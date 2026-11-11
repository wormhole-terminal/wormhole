// The colour of a token: the system's colour picker (a panel with a palette, sliders and an eyedropper), the
// value as the theme works it out, the opacity, and where it comes from. Dragging in the picker is one undo step.
// The theme's sixteen palette colours are the picker's own suggestions.

import AppKit
import DesignSystem
import StudioModel
import SwiftUI

struct ColorPickerRow: View {
    @Bindable var document: StudioDocument
    let token: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        let color = document.resolvedColor(token)
        HStack(spacing: Space._3) {
            ColorPicker(token, selection: binding(color), supportsOpacity: true).labelsHidden()
            VStack(alignment: .leading, spacing: 0) {
                Text(shown(color)).font(TextRole.body.font.monospaced()).lineLimit(1).truncationMode(.middle)
                Text(color.alpha >= 1 ? StudioWords.opaque : StudioWords.alpha(Int((color.alpha * 100).rounded())))
                    .font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
            }
            Spacer(minLength: 0)
            OriginTag(isError: document.fieldError != nil, origin: document.origin(of: token), hasRule: document.rule(for: token) != nil)
        }
    }

    /// What the theme says itself as typed; while the field holds something Rust refused, the rule (what is shown).
    private func shown(_ color: RGBA) -> String {
        document.file.own(token, in: document.shown) ?? (document.fieldError != nil ? document.rule(for: token) : nil) ?? color.hex
    }

    private func binding(_ color: RGBA) -> Binding<Color> {
        Binding(
            get: { color.color },
            set: { new in
                guard let picked = NSColor(new).usingColorSpace(.sRGB) else { return }
                let hex = RGBA(picked.redComponent, picked.greenComponent, picked.blueComponent, picked.alphaComponent).hex
                document.set(token, to: hex, merging: true)
            })
    }
}

/// "override", "derived" or "theme": where the value comes from, or that the field's last value is the one kept.
struct OriginTag: View {
    let isError: Bool
    let origin: TokenOrigin
    let hasRule: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        let (label, color) = isError ? (StudioWords.tagLastGood, tokens.statusWarningText)
            : origin == .yours ? (StudioWords.originYours, tokens.primary)
            : origin == .base ? (StudioWords.originBase, tokens.textMuted) : (hasRule ? "derived" : "theme", tokens.textMuted)
        Text(label).font(TextRole.caption.font).foregroundStyle(color)
            .padding(.horizontal, Space._2).frame(height: Metric.badgeHeight)
            .background(color.opacity(0.14), in: RoundedRectangle(cornerRadius: tokens.metrics.radiusChip))
    }
}

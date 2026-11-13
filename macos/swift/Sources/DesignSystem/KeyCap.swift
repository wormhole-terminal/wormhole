// A shortcut drawn as a key cap (`style::keycap`): one flat `control.background` pill for the whole chord, as tall as
// a caption line with room and at least as wide as "⌘1", so ⌘1 and esc line up. Increase Contrast gives it its outline
// back. `KeyHintView` is a cap and what it does, as footers write it ("↩ open", "esc close").

import SwiftUI

/// One chord drawn as a key cap.
public struct KeyCap: View {
    let text: String
    let height: CGFloat
    @Environment(\.tokens) private var tokens

    /// `height` is the key cap's own unless a title row's control height is wanted.
    public init(text: String, height: CGFloat = Metric.keyCapHeight) { (self.text, self.height) = (text, height) }

    public var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusChip)
        Text(text)
            .font(.system(size: tokens.size(.caption), weight: .medium))
            .monospacedDigit()
            .foregroundStyle(tokens.textPrimary)
            .lineLimit(1)
            .fixedSize()
            .padding(.horizontal, Space._1)
            .frame(minWidth: Metric.keyCapMinWidth, minHeight: height, maxHeight: height)
            .background(tokens.controlBackground, in: shape)
            .contrastOutline(shape, \.borderStrong)
            .marked("keycap")
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(Text(KeyCapSpeech.words(text)))
    }
}

/// A key cap and what it does, in a footer.
public struct KeyHintView: View {
    let keys: String
    let label: String
    @Environment(\.tokens) private var tokens

    public init(keys: String, label: String) { (self.keys, self.label) = (keys, label) }

    public var body: some View {
        HStack(spacing: Space._2) {
            KeyCap(text: keys)
            Text(label).font(.system(size: tokens.size(.caption))).foregroundStyle(tokens.textMuted)
        }
        .fixedSize()
    }
}

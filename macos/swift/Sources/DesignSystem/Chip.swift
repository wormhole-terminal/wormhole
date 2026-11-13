// The one chip (`style::pill`, `neutral_chip`, `tint_chip`, `extra_chip`, `branch_chip`): a short label or icon in chip
// corners on a fill. What it owns is what every chip shares: the corners, the optional outline (always, or only under
// Increase Contrast where a fill alone is no edge), the hover fill, the selected look (`chip.selected` with its text), the
// disabled colour and the selected trait for VoiceOver. A caller passes the colours and sizes it has; the chip adds none.

import SwiftUI

public struct Chip<Content: View>: View {
    let fill: Color?
    let text: Color
    let outline: Color?
    let hoverFill: Color?
    let selected: Bool
    let height: CGFloat?
    let leading: CGFloat
    let trailing: CGFloat
    let content: Content
    @State private var hovering = false
    @Environment(\.tokens) private var tokens
    @Environment(\.isEnabled) private var enabled

    /// `outline`: a line drawn always, in that colour; none: only Increase Contrast draws one round a filled chip.
    /// `hoverFill`: what the fill becomes under the pointer (a chip that is clicked); `selected`: the active chip.
    public init(
        fill: Color?, text: Color, outline: Color? = nil, hoverFill: Color? = nil, selected: Bool = false, height: CGFloat? = nil,
        leading: CGFloat = Space._2, trailing: CGFloat? = nil, @ViewBuilder content: () -> Content
    ) {
        (self.fill, self.text, self.outline, self.hoverFill, self.selected, self.height) = (fill, text, outline, hoverFill, selected, height)
        (self.leading, self.trailing) = (leading, trailing ?? leading)
        self.content = content()
    }

    public var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusChip)
        let shown = selected ? tokens.chipSelected : (hovering && enabled ? hoverFill ?? fill : fill)
        content
            .foregroundStyle(!enabled ? tokens.textDisabled : selected ? tokens.textOnSelected : text)
            .padding(.leading, leading).padding(.trailing, trailing)
            .frame(height: height)
            .background { if let shown { shape.fill(shown) } }
            .overlay { if let outline { shape.strokeBorder(outline, lineWidth: tokens.metrics.borderWidth) } }
            .modifier(ChipContrast(shape: shape, filled: shown != nil && outline == nil))
            .contentShape(shape)
            .onHover { if hoverFill != nil { hovering = $0 } }
            .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

private struct ChipContrast: ViewModifier {
    let shape: RoundedRectangle
    let filled: Bool

    func body(content: Content) -> some View {
        if filled { content.contrastOutline(shape, \.borderStrong) } else { content }
    }
}

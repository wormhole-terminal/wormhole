// A borderless symbol button with hover and press feedback. The label is required: it is the VoiceOver name and the tooltip.

import SwiftUI

public struct IconButton: View {
    public let symbol: String
    /// The Lucide glyph GPUI's button draws, where one is generated; otherwise `symbol`'s SF Symbol.
    public var lucide: Lucide?
    public let label: LocalizedStringKey
    public let action: () -> Void
    /// A pressed toggle or an active filter (`IconButton::marked`): the selected tint and its text colour.
    public var isSelected = false
    @State private var hovering = false
    @Environment(\.isEnabled) private var enabled
    private let run = IconRun()
    /// Where the view that asked for it was built, for inspect mode.
    let file: StaticString
    let line: UInt

    public init(symbol: String, label: LocalizedStringKey, file: StaticString = #fileID, line: UInt = #line, action: @escaping () -> Void) {
        self.symbol = symbol
        self.label = label
        self.action = action
        (self.file, self.line) = (file, line)
    }

    public init(lucide: Lucide, label: LocalizedStringKey, file: StaticString = #fileID, line: UInt = #line, action: @escaping () -> Void) {
        self.init(symbol: "", label: label, file: file, line: line, action: action)
        self.lucide = lucide
    }

    /// This button as a toggle that is on while `on`: it takes the selected tint and says so to VoiceOver.
    public func selected(_ on: Bool) -> IconButton {
        var copy = self
        copy.isSelected = on
        return copy
    }

    public var body: some View {
        Button {
            run.start()
            action()
        } label: {
            Group {
                if let lucide { LucideView(icon: lucide, size: Metric.glyph).iconMotion(IconMotion.of(lucide), run: run.count) } else {
                    Image(systemName: symbol).font(.system(size: Metric.glyph, weight: .medium)).iconMotion(IconMotion.of(symbol: symbol), run: run.count)
                }
            }
            .frame(width: Metric.control, height: Metric.control)
        }
        .buttonStyle(IconButtonStyle(hovering: hovering && enabled, selected: isSelected))
        .marked("icon.button", style: "size control · radius row · glyph glyph", file: file, line: line)  // text: inspect mode's note, in the scales' names
        .onHover { hovering = $0 }
        .help(label)
        .accessibilityLabel(label)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}

private struct IconButtonStyle: ButtonStyle {
    @Environment(\.tokens) private var tokens
    let hovering: Bool
    let selected: Bool
    @Environment(\.isEnabled) private var enabled

    func makeBody(configuration: Configuration) -> some View {
        let shape = RoundedRectangle(cornerRadius: Radius.row)
        // Nothing to do now (an Undo with no step): the faint colour, no hover and no press (`IconButton::disabled`).
        let pressed = enabled && configuration.isPressed
        let text = !enabled ? tokens.textFaint : selected ? tokens.sidebarRowSelectedText : hovering || pressed ? tokens.textPrimary : tokens.textMuted
        let fill = selected && !pressed ? tokens.stateSelected : pressed ? tokens.stateActive : hovering ? tokens.stateHover : .clear
        return configuration.label
            .foregroundStyle(text)
            .background(fill, in: shape)
            .contentShape(shape)
            .pressed(pressed)
            .motion(Motion.press, value: hovering)
    }
}

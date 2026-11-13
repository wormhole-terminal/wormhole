// A keyboard-driven list above a detail pane. The list is as tall as its rows up to a share of the panel, so a
// short list does not leave a hole and a long one does not push the detail out; with nothing open it takes it all.
// The keys walk the tab through its `PanelWalking` (the core's list rules); the row the keys are on wears the ring and
// scrolls into view; a click anywhere in the list takes the keyboard.

import AppKit
import DesignSystem
import PanelModel
import SwiftUI

struct SelectableListSplit<Top: View, Bottom: View>: View {
    let hasDetail: Bool
    let label: String
    var maxTopShare: CGFloat = 0.45
    let walker: any PanelWalking
    @ViewBuilder let top: Top
    @ViewBuilder let bottom: Bottom
    @FocusState private var focused: Bool
    @State private var typed = TypedAhead()
    @Environment(PanelKeyboard.self) private var keyboard: PanelKeyboard?
    @Environment(\.panelMotion) private var motion
    @Environment(\.panelHost) private var host
    @Environment(\.tokens) private var tokens

    var body: some View {
        GeometryReader { geometry in
            VStack(spacing: 0) {
                top.frame(maxHeight: hasDetail ? geometry.size.height * maxTopShare : .infinity)
                if hasDetail {
                    Rectangle().fill(tokens.divider).frame(height: tokens.metrics.dividerWidth)
                    bottom.transition(motion.content)
                }
            }
            .animation(motion.fold, value: hasDetail)
        }
        .environment(\.walkCursor, focused ? walker.cursor : nil)
        .focusable()
        .focused($focused)
        .focusEffectDisabled()
        .simultaneousGesture(TapGesture().onEnded { focused = true })
        // The window's F6 and ⇥ ask the list for the keyboard; it tells the window when it has it.
        .onChange(of: keyboard?.requests) {
            walker.enter(list: keyboard?.wanted ?? 0)
            focused = true
        }
        .onChange(of: focused) { _, now in keyboard?.focusChanged(now) }
        .onAppear {
            keyboard?.walker = walker
            keyboard?.appeared()
        }
        .onDisappear { keyboard?.disappeared(focused: focused) }
        .onKeyPress(phases: [.down, .repeat], action: handle)
        .onCopyCommand {
            guard let text = walker.copyCursor() else { return [] }
            // With a chip to say it, the host copies and says it; else the system's copy puts the text on the clipboard.
            if let said = walker.copiedAs, let host {
                host.copy(text, as: said)
                return []
            }
            return [NSItemProvider(object: text as NSString)]
        }
        .accessibilityLabel(label)
    }

    private func handle(_ press: KeyPress) -> KeyPress.Result {
        guard press.modifiers.isDisjoint(with: [.command, .control]) else { return .ignored }
        let option = press.modifiers.contains(.option)
        let shift = press.modifiers.contains(.shift)
        switch press.key {
        case .upArrow: walker.walk(option ? .first : .up, marking: shift)
        case .downArrow: walker.walk(option ? .last : .down, marking: shift)
        case .leftArrow: walker.walk(.left)
        case .rightArrow: walker.walk(.right)
        case .home: walker.walk(.first)
        case .end: walker.walk(.last)
        case .return: walker.pressCursor(space: false)
        case .space: walker.pressCursor(space: true)
        default:
            guard !option, let letter = press.characters.first, letter.isLetter || letter.isNumber else { return .ignored }
            walker.typeAhead(typed.push(String(letter)))
        }
        return .handled
    }
}

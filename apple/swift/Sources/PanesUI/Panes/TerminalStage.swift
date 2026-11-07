// The terminal area's two states: the panes, or a cover over them. In the app the Changes tab's diff replaces
// the terminal on screen (and the title row shows the diff's own title); the terminals underneath stay alive
// and sized, only hidden and out of the keyboard's and VoiceOver's way, and the cover crossfades in and out
// (a plain fade under Reduce Motion, which is what it is otherwise too: a diff is not an arrival).

import DesignSystem
import SwiftUI

public struct TerminalStage<Panes: View, Cover: View>: View {
    let panes: Panes
    let cover: Cover?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(@ViewBuilder panes: () -> Panes, cover: () -> Cover?) { (self.panes, self.cover) = (panes(), cover()) }

    public var body: some View {
        ZStack {
            panes
                .opacity(cover == nil ? 1 : 0)
                .allowsHitTesting(cover == nil)
                .accessibilityHidden(cover != nil)
            if let cover { cover.transition(.opacity).zIndex(1) }
        }
        .animation(reduceMotion ? Motion.fade : Motion.selection, value: cover == nil)
    }
}

/// The title row's slot: the terminal's breadcrumb, or the cover's own title while a cover is up.
public struct TitleSlot<Row: View, CoverTitle: View>: View {
    let row: Row
    let coverTitle: CoverTitle?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(@ViewBuilder row: () -> Row, coverTitle: () -> CoverTitle?) { (self.row, self.coverTitle) = (row(), coverTitle()) }

    public var body: some View {
        ZStack {
            if let coverTitle { coverTitle.transition(.opacity) } else { row.transition(.opacity) }
        }
        .animation(reduceMotion ? Motion.fade : Motion.selection, value: coverTitle == nil)
    }
}

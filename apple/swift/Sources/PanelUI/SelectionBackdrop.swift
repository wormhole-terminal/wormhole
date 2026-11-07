// The row highlight shared by every list. One shape per list is matched between rows, so changing the
// selection slides it instead of switching two rows. Reduce Motion drops the matching and cross-fades.

import DesignSystem
import SwiftUI

extension View {
    func selectionBackdrop(selected: Bool, hovered: Bool, in space: Namespace.ID) -> some View {
        modifier(SelectionBackdrop(selected: selected, hovered: hovered, space: space))
    }
}

private struct SelectionBackdrop: ViewModifier {
    let selected: Bool
    let hovered: Bool
    let space: Namespace.ID
    @Environment(\.panelMotion) private var motion

    func body(content: Content) -> some View {
        content.background {
            ZStack {
                if hovered && !selected { shape.fill(Palette.hover) }
                if selected {
                    if motion.reduced {
                        shape.fill(Palette.selected).transition(.opacity)
                    } else {
                        shape.fill(Palette.selected).matchedGeometryEffect(id: "selection", in: space)
                    }
                }
            }
        }
    }

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: Radius.chip, style: .continuous) }
}

// The one fill (`one_background`): what each column paints, and which lines stay while it is on. The columns are drawn side by
// side (`WindowBody.panes` clips the terminal to the room the open columns leave, also while one slides), so a surface of the
// same colour never lies over another and no band shows darker where two would add up. The fill is chosen in one place,
// `ChromeColours.oneFill`.

import DesignSystem
import SwiftUI

extension WindowLook {
    /// What a column paints behind itself at the window's opacity: its own colour from the theme, or the one fill's.
    func columnSurface(_ own: Color, _ colours: ChromeColours) -> Color { surface(oneBackground ? colours.oneFill : own) }

    /// How far the cards stand from the window's edges and from each other: with the one fill, one distance for every column and
    /// the tab strip (`Chrome.surfaceInset`, which is also the terminal card's own); off, each column keeps its own.
    var margins: ColumnMargins { oneBackground ? ColumnMargins(edge: Chrome.surfaceInset) : .own }

    /// Whether the lines between the columns are drawn. Liquid Glass hides them, as it always has. With the one fill they are gone
    /// too, except under Increase Contrast: the tint that told the columns apart is gone, and the GPUI app draws them under
    /// Increase Contrast for that reason (`settings/derive.rs` `dividers`: "columns told apart by a tint alone would not be told
    /// apart enough"). The Column dividers setting decides only while the one fill is off.
    var drawsColumnDividers: Bool {
        if glassOn { return false }
        return oneBackground ? themes.increaseContrast : columnDividers
    }

    /// Whether the line along the top of the status bar is drawn: under the same rules as the dividers, and on its own with the
    /// one fill off, as before.
    var drawsStatusRule: Bool {
        if glassOn { return false }
        return !oneBackground || themes.increaseContrast
    }
}

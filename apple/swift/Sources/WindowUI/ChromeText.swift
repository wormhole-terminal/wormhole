// The type the chrome uses beyond the design system's four roles: the breadcrumb's and the bar's `label`, the
// GPUI app's `text_label` (13 pt, here medium for a name; semibold is its section bands').

import SwiftUI

enum ChromeText {
    static let label = Font.system(size: 13, weight: .medium)
}

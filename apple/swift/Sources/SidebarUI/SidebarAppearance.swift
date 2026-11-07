// What the person chose about how the list looks, which the model does not carry: Settings → Terminals &
// sidebar → Density, and the colour a group wears. The window passes it in; the defaults are the shipping app's.

import DesignSystem
import SwiftUI

public struct SidebarAppearance: Equatable {
    public var density: SidebarDensity
    /// A group's wash, by group id: its folder colour as the theme draws it (`GroupColor::wash`). No entry, no wash.
    public var groupWash: [String: Color]
    /// Shows the ⌘1–9 keycaps whatever ⌘ is doing: for pictures and tests.
    public var forcesHints: Bool

    public init(density: SidebarDensity = .regular, groupWash: [String: Color] = [:], forcesHints: Bool = false) {
        self.density = density
        self.groupWash = groupWash
        self.forcesHints = forcesHints
    }
}

extension EnvironmentValues {
    /// What the list's rows measure at the chosen density, read by every part of it.
    @Entry var sidebarMetrics = SidebarMetrics()
    /// The ⌘N keycaps are showing (⌘ held on its own for a moment).
    @Entry var showsHints = false
    /// The stop the keys are on, while the list has the keyboard: it wears the focus ring.
    @Entry var keyedStop: SidebarStop?
}

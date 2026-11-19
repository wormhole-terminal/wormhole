// The list a picker's chip opens, for the Theme Studio's gallery (`studio/gallery/selects.rs`' opened menu with its search
// row): the one menu the app draws itself, in a popover; every other menu is the system's.

import DesignSystem
import PanelModel
import SwiftUI

/// The rev picker's opened list over sample branches, a tag and a commit, for the Studio's gallery.
public struct RevListSpecimen: View {
    @Environment(\.tokens) private var tokens

    private static let revs = [
        Rev(name: "main"), Rev(name: "feature/panel-files"), Rev(name: "v1.6.3", kind: .tag),
        Rev(name: "4c1d9e2", kind: .commit, detail: "sidebar: pin a terminal"), // text: sample (a commit's subject)
    ]

    /// A specimen takes nothing: its sample is its own.
    public init() {}

    /// The sample, drawn by the app's own views.
    public var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radiusLg)
        RevList(selected: Self.revs[0], revs: Self.revs, headTitle: "HEAD", hint: nil, takesKeys: false) { _ in } // text: HEAD is git's name for the current commit
            .clipShape(shape)
            .overlay { shape.strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth) }
            .allowsHitTesting(false)
    }
}

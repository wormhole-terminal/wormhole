// What ⌘ held over a link a Probe's rule made says: the rule's hover words, where the link really goes (always shown:
// a rule's words must not stand in for it) and which Probe said so. The pill is the size chip's (`style::size_readout`),
// in the pane's bottom left corner, and takes no clicks.

import DesignSystem
import SwiftUI

public struct LinkReadout: View {
    let text: String?

    public init(text: String?) { self.text = text }

    public var body: some View {
        if let text {
            ReadoutPill(role: .body, cornerRadius: nil) {
                Text(text).padding(.horizontal, Space._3).padding(.vertical, Space._1)
            }
            .padding(Space._2)
            .allowsHitTesting(false)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
            .accessibilityLabel(text)
        }
    }
}

// The secondary windows' parts on sample data, for the Theme Studio's gallery (`gallery/windows.rs`, `buttons.rs`): the
// themed buttons, a status badge and the Session Hosts window's cards, drawn by the views the windows themselves use.

import AuxModel
import DesignSystem
import SwiftUI

/// The secondary windows' buttons, for the Studio's gallery.
public struct AuxButtonsSpecimen: View {
    public init() {}

    public var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._2) {
                Button("Primary") {}.primaryAction()
                Button("Secondary") {}.secondaryAction()
                Button("Disabled") {}.secondaryAction().disabled(true)
            }
            HStack(spacing: Space._2) {
                Button("Small primary") {}.primaryAction().controlSize(.small)
                Button("Small secondary") {}.secondaryAction().controlSize(.small)
            }
        }
    }
}

/// A status badge and host cards as the Session Hosts window draws them, for the Studio's gallery.
public struct AuxWindowsSpecimen: View {
    @Environment(\.tokens) private var tokens

    public init() {}

    public var body: some View {
        VStack(alignment: .leading, spacing: Space._3) {
            HStack(spacing: Space._2) {
                StatusBadge(text: "Attached", color: tokens.statusSuccess)
                StatusBadge(text: "Detached", color: tokens.statusWarning)
                StatusBadge(text: "Stale", color: tokens.textMuted)
            }
            ForEach(HostsSamples.report.cards.prefix(2)) { card in
                HostCardView(card: card, copied: false, copy: {}, end: {}, forget: {})
            }
        }
        .allowsHitTesting(false)
    }
}

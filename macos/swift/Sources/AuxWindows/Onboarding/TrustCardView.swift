// The folder's trust, offered on the agents step when the project has a `.wormhole/` folder: what trusting it would
// let run, "Trust with Touch ID…" and Later. Nothing is trusted until the button is pressed (`render_sheet_trust`).

import AuxModel
import DesignSystem
import SwiftUI

struct TrustCardView: View {
    let card: TrustCard
    let trust: () -> Void
    let later: () -> Void
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            Rule()
            HStack(spacing: Space._2) {
                Image(systemName: "shield").foregroundStyle(tokens.textMuted)
                Text(AuxText.trustHas(name: card.name, dir: card.directory)).font(TextRole.body.font.weight(.medium))
            }
            Text("\(AuxText.trustWorksNow) \(AuxText.trustWaits(card.commands))")
                .font(TextRole.caption.font).foregroundStyle(tokens.textMuted).fixedSize(horizontal: false, vertical: true)
            HStack(spacing: Space._2) {
                Button(action: trust) {
                    Label(card.approving ? AuxText.trustWaiting : AuxText.trustTouchID, systemImage: "touchid")
                }
                .secondaryAction().controlSize(.small).disabled(card.approving)
                Button(AuxText.trustLater, action: later).buttonStyle(.plain).foregroundStyle(tokens.textMuted)
            }
        }
        .padding(.horizontal, Space._6).padding(.vertical, Space._3)
    }
}

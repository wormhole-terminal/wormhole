// The notice over the pane that its host could not start, until it is dismissed (`render_host_notice`): a warning
// banner 8 pt under the top and 16 pt from the sides, with a ghost Dismiss.

import DesignSystem
import SwiftUI

public struct HostNoticeBanner: View {
    let text: String
    let actions: PaneBarActions
    @Environment(\.tokens) private var tokens

    public init(text: String, actions: PaneBarActions) { (self.text, self.actions) = (text, actions) }

    public var body: some View {
        Banner(kind: .warning, text: text) {
            Button(L10n.dismiss, action: actions.dismissHostNotice)
                .buttonStyle(.plain)
                .font(.system(size: TextRole.label.size))
                .foregroundStyle(tokens.textLink)
                .padding(.horizontal, Space._2)
        }
        .padding(.top, Space._2).padding(.horizontal, Space._4)
    }
}

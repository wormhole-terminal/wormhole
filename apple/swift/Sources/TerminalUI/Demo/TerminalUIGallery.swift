// The whole module on one sheet, for offscreen renders and previews: the stacked pane on top, the pane bars and
// the agent cards below. Static: what a real window animates is in the states it rests in.

import DesignSystem
import SwiftUI

public struct TerminalUIGallery: View {
    let model: GalleryModel
    @Environment(\.tokens) private var tokens

    public init(model: GalleryModel) { self.model = model }

    public var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            GalleryPane(model: model).frame(height: 460)
            HStack(alignment: .top, spacing: Space._4) {
                VStack(spacing: Space._3) {
                    strip(.ask(program: "ssh"), exited: false)
                    strip(.hostUnavailable(.busy, endFailure: nil), exited: false)
                    strip(.hostNotResponding(endFailure: nil), exited: false)
                    strip(.notice(.info, "A program copied 12 lines"), exited: true)
                }
            }
        }
        .padding(Space._4)
        .frame(width: 1240)
        .background(tokens.surfaceWindow)
        .foregroundStyle(tokens.textPrimary)
    }

    private func strip(_ bar: PaneBar, exited: Bool) -> some View {
        ZStack {
            tokens.terminal.background.color
            PaneOverlays(state: PaneOverlayState(bar: bar, hostNotice: nil, exited: exited), shell: "zsh", exitCode: 130,
                         gridWidth: 700, actions: model.actions)
        }
        .frame(height: exited ? 120 : 76)
        .clipShape(RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
    }
}

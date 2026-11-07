// The bar over the grid's bottom edge (`render/pane_bar.rs`): a banner with the words and the buttons of what it
// asks. Try again comes first, so a momentary failure does not steer anyone to ending the host. A grid at least
// 560 pt wide has room for the long label of the second clipboard button.

import DesignSystem
import SwiftUI

public struct PaneBarView: View {
    let bar: PaneBar
    let actions: PaneBarActions
    /// The grid's width in points, for the label that fits.
    let gridWidth: CGFloat

    /// `ROOMY_PX` in `render/pane_bar.rs`.
    private static let roomyWidth: CGFloat = 560

    public init(bar: PaneBar, actions: PaneBarActions, gridWidth: CGFloat) {
        (self.bar, self.actions, self.gridWidth) = (bar, actions, gridWidth)
    }

    public var body: some View {
        switch bar {
        case let .ask(program):
            PaneBanner(kind: .warning, text: L10n.clipboardAsk(program)) {
                HStack(spacing: Space._1) {
                    BarButton(title: L10n.clipboardOnce, help: L10n.clipboardOnceHelp) { actions.answerClipboard(.once) }
                    BarButton(title: gridWidth >= Self.roomyWidth ? L10n.clipboardAlwaysLong : L10n.clipboardAlways,
                              help: L10n.clipboardAlwaysHelp) { actions.answerClipboard(.always) }
                    BarButton(title: L10n.clipboardDeny, help: L10n.clipboardDenyHelp) { actions.answerClipboard(.deny) }
                }
            }
        case let .notice(kind, text):
            PaneBanner(kind: kind, text: text) { EmptyView() }
        case let .hostUnavailable(kind, failure):
            PaneBanner(kind: .warning, text: failure.map(L10n.hostsEndFailed) ?? L10n.hostsUnavailable(kind)) {
                HStack(spacing: Space._1) {
                    if kind.isTransient { BarButton(title: L10n.hostsRetry, action: actions.tryAttachAgain) }
                    BarButton(title: L10n.hostsNew, action: actions.openNewTerminalHere)
                    BarButton(title: L10n.hostsEnd) { actions.endHost(hard: false) }
                }
            }
        case let .hostNotResponding(failure):
            PaneBanner(kind: .warning, text: failure.map(L10n.hostsEndFailed) ?? L10n.hostsNotResponding) {
                HStack(spacing: Space._1) {
                    BarButton(title: L10n.hostsWait, action: actions.waitForHost)
                    BarButton(title: L10n.hostsEnd) { actions.endHost(hard: true) }
                }
            }
        }
    }
}

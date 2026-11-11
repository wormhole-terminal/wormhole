// The ports the terminal's programs listen on: a chip for each of the first ones the row has room for
// (`portsStep` chips fewer than usual, at least one), each a globe and `localhost:3000` that opens in the
// browser, and a "+n" menu for the others.

import DesignSystem
import PanesModel
import SwiftUI

struct PortChips: View {
    let ports: [UInt16]
    let step: Int
    @Environment(\.tokens) private var tokens

    var body: some View {
        let own = Ports.chips(ports).shown
        let shown = Array(own.prefix(max(min(own.count - step, own.count), 1)))
        let rest = Array(ports.dropFirst(shown.count))
        HStack(spacing: TitleMetrics.chipGap) {
            ForEach(shown, id: \.self) { port in
                Button { TitleClicks.openPort(port) } label: {
                    ExtraChip(text: Ports.label(port), symbol: ("globe", tokens.textPrimary))
                }
                .buttonStyle(.plain)
                .help(Ports.words(port)).accessibilityLabel(Ports.words(port))
            }
            if !rest.isEmpty { more(rest) }
        }
    }

    /// The ports without a chip, in a menu under "+n": each opens in the browser.
    private func more(_ rest: [UInt16]) -> some View {
        Menu {
            ForEach(rest, id: \.self) { port in
                Button(Ports.label(port)) { TitleClicks.openPort(port) }
            }
        } label: {
            ExtraChip(text: "+\(rest.count)", filled: false)
        }
        .menuStyle(.button).buttonStyle(.plain).menuIndicator(.hidden).fixedSize()
        .help(Ports.moreWords(rest)).accessibilityLabel(Ports.moreWords(rest))
    }
}

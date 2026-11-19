// text-file: a test driver's replies on the control folder, not interface text
// The control folder's `hosts-window` and `hosts-forget <id>`: the Session Hosts window as its own model holds it
// (`HostsWindow.model`, what the cards are drawn from and what follows the hosts folder by itself), and a stale card's
// Forget (`HostsViewModel.forget`, what the card's button calls). `hosts-list` asks the hosts anew instead: it says what
// is there, not what the window shows. Replies:
//   hosts-window cards="<n>"                 (`hosts-window none` before the window's first look)
//   card <id> <attached|detached|orphan|stale> "<standing as the card says it>" note="<the card's note>" [open] [attention] [can-end] [can-forget]
//     fact <field> "<value>" [warning|danger]     per fact of the card, in its order (hostPid, shellPid, program, link, …)
//   hosts-forget <id>                         or `error: …` (the window closed, no such card, a card that is not stale)

import AuxModel
import AuxWindows
import DesignSystem

extension AppController {
    /// False when `command` is not one of these.
    func runHostsWindow(command: String, argument: String) -> Bool {
        guard command == "hosts-window" || command == "hosts-forget" else { return false }
        guard let model = HostsWindow.model else { reply("error: the Hosts window is not open (`hosts`)"); return true }
        if command == "hosts-forget" {
            guard let card = model.report?.cards.first(where: { $0.id == argument }) else { reply("error: no card \(argument)"); return true }
            guard card.canForget else { reply("error: \(argument) is not stale: its card has no Forget"); return true }
            model.forget(argument)
            reply("hosts-forget \(argument)")
            return true
        }
        guard let report = model.report else { reply("hosts-window none"); return true }
        var lines = [Readout.line("hosts-window", pairs: [("cards", String(report.cards.count))])]
        for card in report.cards {
            lines.append(Readout.line(
                "card", [card.id, "\(card.standing)"], quoted: [card.standing.label], pairs: [("note", card.note)],
                flags: [
                    Readout.flag("open", model.open.contains(card.id)), Readout.flag("attention", card.needsAttention),
                    Readout.flag("can-end", card.canEnd), Readout.flag("can-forget", card.canForget),
                ]))
            lines += card.facts.map { Readout.under("fact", ["\($0.field)"], quoted: [$0.value], flags: [$0.tone == .normal ? "" : "\($0.tone)"]) }
        }
        reply(lines.joined(separator: "\n"))
        return true
    }
}

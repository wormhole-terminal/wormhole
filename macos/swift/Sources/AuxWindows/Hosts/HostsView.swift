// The Session Hosts window (Help > Session Hosts, while persistent terminals are on): what the processes that keep
// terminals running behind the app are doing, so a person can see whether things are right. The header, then a
// card per host in the hosts folder. End asks first (a system sheet); Forget and Copy do not.

import AppKit
import AuxModel
import DesignSystem
import SwiftUI

struct HostsView: View {
    let model: HostsViewModel
    let sheetWindow: () -> NSWindow?
    @Environment(\.tokens) private var tokens
    @State private var position = ScrollPosition()

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let report = model.report {
                HostsHeader(report: report, copiedAll: model.copied == .all) { model.copy(.all) }
                ScrollView {
                    if report.cards.isEmpty {
                        note(AuxText.hostsEmpty(report.folder))
                    } else {
                        LazyVStack(spacing: Space._3) {
                            ForEach(report.cards) { card in
                                HostCardView(
                                    card: card, open: model.open.contains(card.id), toggle: { model.toggle(card.id) },
                                    copied: model.copied == .host(card.id), copy: { model.copy(.host(card.id)) },
                                    end: { ask(toEnd: card.id) }, forget: { model.forget(card.id) })
                            }
                        }
                        .padding(.horizontal, Space._4).padding(.bottom, Space._4)
                        .motion(Motion.layout, value: report.cards.map(\.id))
                    }
                }
                .scrollPosition($position)
                .onChange(of: model.scrollRequest?.serial) { if let request = model.scrollRequest { position.scrollTo(y: request.offset) } }
            } else {
                note(AuxText.hostsLoading)
            }
        }
        .auxPage()
        .onAppear { model.start() }
        .onDisappear { model.stop() }
    }

    private func note(_ text: String) -> some View {
        Text(text).font(TextRole.body.font).foregroundStyle(tokens.textMuted).padding(.horizontal, Space._4).padding(.vertical, Space._2)
    }

    private func ask(toEnd id: String) {
        guard let question = model.endQuestion(id) else { return }
        Task { if case .confirm = await QuestionSheet(question).ask(in: sheetWindow()) { model.end(id) } }
    }
}

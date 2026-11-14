// text-file: the control folder's `onboarding-state` (dev tooling), not interface text
// The first run's page as text (`onboarding-state`): what `OnboardingView` and its step draw, read from the session they
// draw from. Nothing here decides. The shape is `DesignSystem.Readout`'s:
//
//     onboarding-state folder number="1" folder="~/src/api" first-run
//     card "Where do you work?" "The first terminal starts …" counter="1 of 3"
//     button back "Back"
//     button main "Continue"
//     button skip "Skip setup"
//     place recent "api" detail="~/src" picked
//     agent claude "Claude Code" hooks="installed"
//     trust "api/ has a .wormhole/ folder" "Its theme and notes work now. …" button="Trust with Touch ID…" root="/…/api" approving
//     primer "Jot a note for this folder" keys="⌥⌘N"
//     problem "…"

import AuxModel
import DesignSystem
import Foundation

@MainActor
public enum OnboardingReadout {
    public static func text(_ session: OnboardingSession) -> String {
        let step = session.step
        var lines = [
            Readout.line(
                "onboarding-state", [name(step)],
                pairs: [("number", String(step.number)), ("folder", session.flow.folder.map { Place.tilde($0, home: .homeDirectory) } ?? "-")],
                flags: [Readout.flag("first-run", session.firstRun), Readout.flag("last", session.flow.isLast)])
        ]
        if step != .welcome {
            lines.append(
                Readout.line(
                    "card", quoted: [OnboardingView.title(session), OnboardingView.intro(session)],
                    pairs: [("counter", AuxText.stepCounter(step.number, of: OnboardingStep.counted, noAgents: step == .primer))]))
            // `StepFooter` and `StepCard`'s head.
            lines.append(Readout.line("button", ["back"], quoted: [AuxText.back]))
            lines.append(Readout.line("button", ["main"], quoted: [step.isLast ? AuxText.start : AuxText.next]))
            lines.append(Readout.line("button", ["skip"], quoted: [AuxText.skipSetup]))
        }
        switch step {
        case .folder:
            // `FolderStep`: the recent folders first (under Recents), then the places; the picked one is marked.
            let picked = session.flow.folder
            for place in session.places.filter({ $0.kind == .recent }) + session.places.filter({ $0.kind != .recent }) {
                lines.append(
                    Readout.line(
                        "place", [place.kind == .recent ? "recent" : "place"], quoted: [place.name], pairs: [("detail", place.detail)],
                        flags: [Readout.flag("picked", place.url == picked)]))
            }
        case .agents:
            for agent in session.agents ?? [] {
                lines.append(Readout.line("agent", [agent.id], quoted: [agent.name], pairs: [("hooks", "\(agent.hooks)")]))
            }
            if session.agents == nil { lines.append(Readout.line("agents", quoted: [AuxText.lookingForAgents])) }
            if let card = session.trustCard {
                lines.append(
                    Readout.line(
                        "trust", quoted: [AuxText.trustHas(name: card.name, dir: card.directory), "\(AuxText.trustWorksNow) \(AuxText.trustWaits(card.commands))"],
                        pairs: [("button", card.approving ? AuxText.trustWaiting : AuxText.trustTouchID), ("later", AuxText.trustLater), ("root", card.root?.path ?? "-")],
                        flags: [Readout.flag("approving", card.approving)]))
            }
        case .primer:
            for (thing, keys) in PrimerStep.things(notesOn: session.backend.notesOn) {
                lines.append(Readout.line("primer", quoted: [thing], pairs: [("keys", keys.joined(separator: " "))]))
            }
        case .welcome, .look: break
        }
        if let problem = session.problem { lines.append(Readout.line("problem", quoted: [problem])) }
        return lines.joined(separator: "\n")
    }

    private static func name(_ step: OnboardingStep) -> String {
        switch step {
        case .welcome: "welcome"
        case .folder: "folder"
        case .look: "look"
        case .agents: "agents"
        case .primer: "primer"
        }
    }
}

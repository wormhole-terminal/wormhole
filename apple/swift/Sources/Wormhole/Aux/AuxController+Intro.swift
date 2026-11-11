// The developer actions that play the intro around the window in front (Play Intro: Emerge … Voyage) and the
// terminal welcome's three hand-overs (Play Welcome: Cursor, Portal, Logo). A run plays whatever Reduce Motion
// says is asked, except that the glow stands in for every style under it, as it does everywhere.

import AppKit
import AuxModel
import AuxWindows
import DesignSystem

extension AuxController {
    /// Plays `style` over the front window as a replay: the window leaves into the wormhole and comes back.
    func playIntro(_ style: IntroStyle, handover: Handover? = nil) {
        guard let window = app.window else { return }
        var options = IntroOptions()
        options.lights = true
        options.handover = handover
        options.immersive = !app.policy.background
        introPlayer.play(style, over: window, options: options, colors: IntroColors(tokens: ThemeStore.shared.tokens))
    }

    /// Play Welcome (Help ▸ Welcome, the palette, `welcome`): the start animation with its ship docking as `handover` has it, and
    /// a new terminal that runs the welcome program (`AppController.openWelcomeTerminal`, the one entry to it).
    func playWelcome(_ handover: Handover) {
        playIntro(handover.style, handover: handover)
        app.openWelcomeTerminal(handover)
    }
}

extension AuxController {
    /// The control folder's `onboarding <welcome|folder|look|agents|primer|close>`: the intro window at that step, as
    /// a first run walks to it (nothing is installed or trusted on the way: Continue never does either).
    func showOnboarding(step: String) {
        if step == "close" { onboarding?.close(); onboarding = nil; return }
        let wanted: [String: OnboardingStep] = ["welcome": .welcome, "folder": .folder, "look": .look, "agents": .agents, "primer": .primer]
        guard let target = wanted[step] else { return }
        if onboarding == nil { showOnboarding(firstRun: true) }
        guard let session = onboarding?.session else { return }
        for _ in 0..<4 where session.step != target && !(target == .primer && session.step == .agents) {
            session.send(.continue)
        }
    }
}

extension AppController {
    /// `intro <style>` and `welcome <cursor|portal|logo>`, the two developer actions' control verbs.
    func runIntro(command: String, argument: String) {
        if command == "intro", let style = IntroStyle(rawValue: argument) {
            aux.playIntro(style)
        } else if command == "welcome", let handover = Handover(rawValue: argument) {
            aux.playWelcome(handover)
        } else {
            return reply("error: \(command) takes \((command == "intro" ? IntroStyle.allCases.map(\.rawValue) : Handover.allCases.map(\.rawValue)).joined(separator: "|"))")
        }
        reply("\(command) \(argument)")
    }
}

extension AppController {
    /// The one entry point to the welcome's terminal side: a new terminal in the window in front that runs this app's own
    /// `welcome` program in front of the shell the terminal would have run (core's `welcome_command`: `welcome -- <program> <args>`), as
    /// GPUI's `open_welcome_session` does. The program itself (`Wormhole welcome`, core's `onboarding::welcome::run`) is
    /// the settings and onboarding lane's.
    func openWelcomeTerminal(_ handover: Handover) {
        // The hand-over (which of Cursor, Portal, Logo the ship docks as) reaches the program through the app side of its token and
        // plan (`Action::Welcome`), which is the settings lane's; the terminal only needs the program.
        guard let exe = Bundle.main.executablePath else { return }
        let shell = options.program ?? "/bin/zsh"
        hub.send(.newTerminalRunning(program: exe, arguments: ["welcome", "--", shell]))
    }
}

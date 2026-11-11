// The developer actions that play the intro around the window in front (Play Intro: Emerge … Voyage) and the
// terminal welcome's three hand-overs (Play Welcome: Cursor, Portal, Logo). A run plays whatever Reduce Motion
// says is asked, except that the glow stands in for every style under it, as it does everywhere.

import AppKit
import AuxModel
import AuxWindows
import DesignSystem
import TerminalMetal
#if WITH_UNIFFI
    import WormholeBindings
#endif

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

    /// Play Welcome (Help ▸ Welcome, the palette, `welcome`, a first run): the start animation with its ship docking as `handover` has it,
    /// and a new terminal that runs the welcome program (`AppController.openWelcomeTerminal`, the one entry to it).
    func playWelcome(_ handover: Handover, firstRun: Bool = false) {
        #if WITH_UNIFFI
            // The ship docks on the program's real `>` (or the logo's chevron): core's cell, in the grid of the terminal in front.
            if let dock = welcomeDock(handover: handover.rawValue) {
                introPlayer.landingSource = { [weak self] in self?.landing(of: dock) }
            } else {
                introPlayer.landingSource = nil
            }
        #endif
        playIntro(handover.style, handover: handover)
        app.openWelcomeTerminal(handover, firstRun: firstRun)
    }
}

#if WITH_UNIFFI
    extension AuxController {
        /// Where the ship docks on screen: the middle of `dock`'s chevron in the grid of the terminal in front, and the logo's
        /// size that makes the ship's chevron as tall as the cells say (`Landing.onChevron`). Points from the window's top left,
        /// as the page's logo is measured; nil until the terminal is on screen.
        func landing(of dock: WelcomeDock) -> SIMD3<Float>? {
            guard let (chevron, height) = chevron(of: dock) else { return nil }
            let landing = Landing.onChevron(chevron, height: height)
            return [landing.centre.x, landing.centre.y, landing.size]
        }

        /// The middle of `dock`'s chevron in the window (points from its top left) and how tall it is.
        func chevron(of dock: WelcomeDock) -> (point: SIMD2<Float>, height: Float)? {
            guard let view = app.selectedView, let window = view.window, let content = window.contentView else { return nil }
            let frame = view.convert(view.bounds, to: nil)
            let cell = view.renderer.cell
            // The grid is anchored top left, a padding from the edges.
            let origin = SIMD2<Float>(Float(frame.minX + MetalRenderer.padding), Float(content.bounds.height - frame.maxY + MetalRenderer.padding))
            return (origin + SIMD2<Float>(Float(cell.width) * dock.col, Float(cell.height) * dock.row), Float(cell.height) * dock.rows)
        }
    }
#endif

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
    /// GPUI's `open_welcome_session` does. With the `terminal_welcome` flag on the terminal gets a one-time token and the core answers the
    /// program's requests (`AuxController.startWelcomeTerminal`); without it the program is the read-only tour.
    func openWelcomeTerminal(_ handover: Handover, firstRun: Bool = false) {
        if aux.startWelcomeTerminal(handover, firstRun: firstRun) { return }
        guard let exe = Bundle.main.executablePath else { return }
        let shell = options.program ?? "/bin/zsh"
        hub.send(.newTerminalRunning(program: exe, arguments: ["welcome", "--", shell]))
    }
}

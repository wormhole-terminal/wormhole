// The intros played again over the window in front (Help's "Play Intro" and "Play Welcome" entries): the run leaves the
// window as it is and brings it back, with the traffic lights fading. A test instance keeps the effect above the
// window instead of taking the screen.

import AppKit
import AuxModel
import AuxWindows
import DesignSystem

extension AuxController {
    func playIntro(_ style: IntroStyle, handover: Handover? = nil) {
        guard let window = app.window else { return }
        var options = IntroOptions()
        options.lights = true
        options.handover = handover
        options.immersive = !app.policy.background
        introPlayer.play(style, over: window, options: options, colors: IntroColors(tokens: ThemeStore.shared.tokens))
    }
}

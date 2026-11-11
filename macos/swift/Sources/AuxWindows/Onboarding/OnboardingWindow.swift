// Opens the intro window and plays the wormhole around it: the window arrives out of the wormhole and its page
// comes in part by part. A first run starts with the window never shown; a replay (from About or the palette)
// starts over a window that shows and leaves as the run begins. Reduce Motion plays the still glow and the page is
// simply there. The window holds the keyboard while it is open; closing it puts the traffic lights back.

import AppKit
import AuxModel
import DesignSystem
import SwiftUI

@MainActor
public final class OnboardingWindow {
    public let session: OnboardingSession
    public let player: IntroPlayer
    private var controller: AuxWindowController?
    private let immersive: Bool

    /// `immersive` false keeps the effect just above the window (a test instance kept behind other apps).
    public init(backend: any OnboardingBackend, firstRun: Bool, immersive: Bool = true, player: IntroPlayer = IntroPlayer()) {
        session = OnboardingSession(backend: backend, firstRun: firstRun)
        self.player = player
        self.immersive = immersive
        session.onEnd = { [weak self] folder, firstRun in
            backend.finish(folder: folder, firstRun: firstRun)
            self?.close()
        }
    }

    /// The page is up: the front window opens no shortcut sheet under it.
    public var isOpen: Bool { controller?.window?.isVisible == true }

    public func show(store: ThemeStore = .shared) {
        let (session, player) = (self.session, self.player)
        let controller = AuxWindowController.show(
            key: "onboarding", title: AuxText.onboardingLabel, size: OnboardingMetrics.windowSize,
            minSize: OnboardingMetrics.windowMin, closesOnEscape: false
        ) {
            OnboardingView(session: session, player: player, replay: { [weak self] in self?.replay() })
        }
        self.controller = controller
        Task { await session.scan() }
        guard let window = controller.window else { return }
        NotificationCenter.default.addObserver(forName: NSWindow.willCloseNotification, object: window, queue: .main) { [weak player] _ in
            MainActor.assumeIsolated { player?.stop() }
        }
        // First launch: the window was never shown, so it stays out of sight until the wormhole delivers it.
        var options = IntroOptions()
        options.start = session.firstRun ? .hidden : .shown
        options.lights = !session.firstRun
        options.immersive = immersive
        player.play(.firstLaunch, over: window, options: options, colors: IntroColors(tokens: store.tokens))
    }

    /// Plays the intro again over the window as it is: it leaves, and the welcome page comes back with the run.
    public func replay(store: ThemeStore = .shared) {
        guard let window = controller?.window else { return }
        var options = IntroOptions()
        options.immersive = immersive
        player.play(.firstLaunch, over: window, options: options, colors: IntroColors(tokens: store.tokens))
        Task {
            // one-shot: the intro's leaving motion ends, then the page is shown again
            try? await Task.sleep(for: .seconds(Double(IntroPlayer.leaving)))
            session.replay()
        }
    }

    public func close() {
        player.stop()
        controller?.close()
    }
}

// The intro window's page: the welcome page or a step's card over the whole window, opaque, with the wormhole's light
// falling faintly from the top and the replay button in the corner. Each part comes in as the animation allows. It
// holds the keyboard: ←, →, ⏎, Esc and R, and the mouse for all of it; a key or click while the page is still coming
// in only cuts the run short (`onboarding/page.rs`, `keys.rs`).

import AuxModel
import DesignSystem
import SwiftUI

struct OnboardingView: View {
    let session: OnboardingSession
    let player: IntroPlayer
    let replay: () -> Void
    @Environment(\.tokens) private var tokens
    @FocusState private var focused: Bool

    var body: some View {
        let entrance = player.entrance
        ZStack(alignment: .topTrailing) {
            LinearGradient(colors: [tokens.primary.opacity(OnboardingMetrics.glowAlpha), .clear], startPoint: .top, endPoint: .bottom)
                .containerRelativeFrame(.vertical) { height, _ in height * OnboardingMetrics.glowHeight }
                .frame(maxHeight: .infinity, alignment: .top).opacity(Double(entrance.logo)).allowsHitTesting(false)
                .accessibilityHidden(true)
            // Centred by its margins: where it doesn't fit it starts at the top and scrolls.
            ScrollView {
                page(entrance).frame(maxWidth: .infinity).padding(.top, Metric.titleRow).padding(.bottom, Space._6)
                    .containerRelativeFrame(.vertical, alignment: .center)
            }
            .scrollBounceBehavior(.basedOnSize)
            IconButton(symbol: "arrow.counterclockwise", label: "\(AuxText.replayIntro) (R)", action: replay)
                .padding(.top, (Metric.titleRow - Metric.titleControl) / 2).padding(.trailing, Space._3)
                .entering(entrance.corner)
        }
        .auxPage(titleRow: false)
        // The page is opaque over the whole window (`window_surfaces(cx, false)`: the window token at full alpha); the
        // translucent token alone lets the window's own fill through, which read about 7 levels lighter.
        .background { (tokens.rgba(named: "surface.window").map { RGBA($0.red, $0.green, $0.blue).color } ?? tokens.surfaceWindow).ignoresSafeArea() }
        .focusable().focused($focused).focusEffectDisabled()
        .onAppear { focused = true }
        .onTapGesture { if player.isEntering { player.skip() } }
        .onKeyPress(phases: .down) { press in handle(press) }
        .onExitCommand { session.send(.skip) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(AuxText.onboardingLabel)
        .accessibilityAddTraits(.isModal)
    }

    @ViewBuilder private func page(_ entrance: Entrance) -> some View {
        if session.step == .welcome {
            WelcomePage(session: session, player: player)
        } else {
            StepCard(session: session, title: title, intro: intro) { stepBody }.entering(entrance.body)
        }
    }

    @ViewBuilder private var stepBody: some View {
        switch session.step {
        case .folder: FolderStep(session: session)
        case .look: LookStep(look: session.look, preferences: session.preferences)
        case .agents: AgentsStep(session: session)
        case .primer: PrimerStep(session: session)
        case .welcome: EmptyView()
        }
    }

    private var title: String { Self.title(session) }
    private var intro: String { Self.intro(session) }

    /// The card's title and intro for the step (the control folder's `onboarding-state` reads them too).
    static func title(_ session: OnboardingSession) -> String {
        switch session.step {
        case .folder: AuxText.folderTitle
        case .look: AuxText.lookTitle
        case .agents: AuxText.agentsTitle
        case .primer: AuxText.primerTitle
        case .welcome: AuxText.onboardingLabel
        }
    }

    static func intro(_ session: OnboardingSession) -> String {
        switch session.step {
        case .folder: AuxText.folderIntro
        case .look: AuxText.lookIntro(settingsKeys: session.backend.settingsKeys)
        case .agents: AuxText.agentsIntro(found: session.agents?.count ?? 0)
        case .primer: AuxText.primerIntro
        case .welcome: ""
        }
    }

    /// While the page comes in a key only cuts the animation short; afterwards the page's own keys work, and the
    /// window's shortcuts (anything with a modifier) are left to the menus.
    private func handle(_ press: KeyPress) -> KeyPress.Result {
        guard press.modifiers.isEmpty else { return .ignored }
        if player.isEntering { player.skip(); return .handled }
        switch press.key {
        case .rightArrow:
            if !session.flow.isLast { session.send(.continue) }
        case .leftArrow: session.send(.back)
        case .return: session.send(session.flow.isLast ? .start : .continue)
        default:
            // A digit answers the walk's question on the Look step (`tailor_key`).
            if session.step == .look, let n = Int(press.characters), n >= 1, session.look.walk?.stage == .asking {
                session.look.act(.answer(n - 1))
                return .handled
            }
            guard press.characters.lowercased() == "r" else { return .ignored }
            replay()
        }
        return .handled
    }
}

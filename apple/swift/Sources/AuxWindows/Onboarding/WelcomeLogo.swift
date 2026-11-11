// The app's mark on the welcome page, where Voyage's ship docks. It measures itself in the window's coordinates and
// tells the player every time its place changes, so a window that is resized is followed (`welcome_page::logo`).

import AuxModel
import SwiftUI

struct WelcomeLogo: View {
    let progress: Float
    let player: IntroPlayer

    var body: some View {
        Image(nsImage: NSApp.applicationIconImage)
            .resizable()
            .frame(width: OnboardingMetrics.mark, height: OnboardingMetrics.mark)
            .opacity(Double(progress))
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { frame in
                player.landing = [Float(frame.midX), Float(frame.midY), Float(frame.width)]
            }
            .accessibilityHidden(true)
    }
}

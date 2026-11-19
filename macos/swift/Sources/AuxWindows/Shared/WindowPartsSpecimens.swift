// Two of the secondary windows drawn small on sample data, for the Theme Studio's gallery (`gallery/windows.rs`): the
// first run's step card (where it is, the step, its footer with the dots) and the Licenses window's parts (the package
// rows, one selected, and the selected one's text). The views are the windows' own; the keys the real card takes (Return,
// ⌘O) stay with the Studio (`stepKeys`). About is the system's panel and has no view here.

import AuxModel
import DesignSystem
import SwiftUI

extension EnvironmentValues {
    /// Whether a step's card takes its keys (Return for Continue, ⌘O for Open folder): off for a picture of it in another
    /// window, which must not answer that window's keys.
    @Entry var stepKeys = true
}

/// The first run's card on its first step, for the Studio's gallery.
public struct FirstRunSpecimen: View {
    @State private var session = FirstRunSpecimen.sample()

    /// A specimen takes nothing: its sample is its own.
    public init() {}

    /// The sample, drawn by the app's own views.
    public var body: some View {
        StepCard(session: session, title: AuxText.folderTitle, intro: AuxText.folderIntro) { FolderStep(session: session) }
            .frame(height: OnboardingMetrics.sheetMinHeight)
            .environment(\.stepKeys, false)
            .allowsHitTesting(false)
    }

    /// A first run past the welcome page, over the sample backend.
    private static func sample() -> OnboardingSession {
        let session = OnboardingSession(backend: SampleOnboardingBackend(), firstRun: true)
        session.send(.continue)
        return session
    }
}

/// The Licenses window's package rows and the selected package's text, for the Studio's gallery.
public struct LicensesSpecimen: View {
    @Environment(\.tokens) private var tokens

    /// A specimen takes nothing: its sample is its own.
    public init() {}

    /// The sample, drawn by the app's own views.
    public var body: some View {
        let list = SampleLicenses.list
        HStack(alignment: .top, spacing: 0) {
            VStack(spacing: 0) {
                ForEach(Array(list.packages.enumerated()), id: \.element.id) { index, package in
                    PackageRow(package: package, selected: index == 0)
                }
                Text(AuxText.licensesCount(shown: list.packages.count, total: list.packages.count))
                    .font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                    .padding(.horizontal, Space._2).padding(.vertical, Space._2).frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(Space._2)
            .frame(width: LicensesMetrics.listWidth)
            .background(tokens.surfacePanel)
            Rectangle().fill(tokens.divider).frame(width: tokens.metrics.dividerWidth)
            PackageDetail(package: list.packages[0], texts: list.texts(of: list.packages[0]))
        }
        // Half the window's least height: enough for the rows, and the text scrolls under the system's scroller.
        .frame(height: LicensesMetrics.minSize.height / 2)
        .foregroundStyle(tokens.textPrimary)
        .background(tokens.surfaceWindow)
        .clipShape(RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
        .allowsHitTesting(false)
    }
}

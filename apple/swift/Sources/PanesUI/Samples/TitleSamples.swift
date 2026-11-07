// Sample facts for the title row, and an Open in… backend that needs no system: the apps a developer's Mac
// would list, for previews and snapshots.

import AppKit
import PanesModel
import WorkspaceModel

public enum TitleSamples {
    /// The reference shot (`dark-notes`): a folder, a branch with changes, an agent, the buttons.
    public static let agent = TitleFacts(
        terminalID: 2, folder: "/private/tmp/ref-repo", shownFolder: "/private/tmp/ref-repo", activity: .working, agent: .claude,
        label: "claude", branch: .some("main"), branchStatus: BranchStatus(dirty: true), row: TitleRow.default)

    /// Everything the row can say, for the narrow-row cases.
    public static let full = TitleFacts(
        terminalID: 3, folder: "/Users/ben/Projects/wormhole/crates/core", shownFolder: "~/Projects/wormhole/crates/core",
        activity: .needsInput, agent: .claude, label: "sidebar polish", words: "needs input · 4m",
        branch: .some("feature/pane-headers"), branchStatus: BranchStatus(dirty: true, ahead: 2, behind: 1, upstream: "origin/main"),
        gauge: GaugeFact(model: "Opus 4.5", context: 82), ports: [3000, 5173, 8080],
        pullRequest: PullRequestFact(number: 142, state: .open, checks: .failing))

    public static let plain = TitleFacts(
        terminalID: 1, folder: "/Users/ben", shownFolder: "~", label: "zsh", row: TitleRow.default)
}

@MainActor
public struct SampleOpenIn: OpenInBackend {
    public init() {}

    public func apps() -> [OpenInApp] {
        [OpenInApp(bundle: "dev.zed.Zed", name: "Zed", monogram: "Z"), OpenInApp(bundle: "com.microsoft.VSCode", name: "Visual Studio Code", monogram: "VS"), .finder]
    }

    public func choice() -> OpenInChoice { OpenInChoice(app: apps()[0], missing: nil) }
    public func open(_ folder: String, in app: OpenInApp) {}
}

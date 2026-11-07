// Sample data for the UI modules' previews, screenshots and tests: a window as a working day leaves it.
// Three coloured groups (one folded) among loose terminals, pinned ones in and out of a group, a split,
// agents in every state with times, and every `Activity`.

import Foundation

public enum MockWorkspace {
    public static let state: WorkspaceState = {
        let now = Date()
        func minutesAgo(_ minutes: Double) -> Date { now.addingTimeInterval(-minutes * 60) }
        let app = "~/Projects/terminal/wormhole"
        let web = "~/Projects/terminal/web"
        let terminals = [
            TerminalSummary(
                id: 1, title: "sidebar polish", branch: "sidebar-polish", activity: .working, agent: .claude,
                activitySince: minutesAgo(3), folder: app, noteCount: 2, changeCount: 7),
            TerminalSummary(
                id: 2, title: "cargo test", branch: "main", activity: .working,
                runningCommand: "cargo test -p wormhole-term", runningSince: minutesAgo(1.2), folder: app),
            TerminalSummary(
                id: 3, title: "diff perf", branch: "diff-perf", activity: .needsInput, isPinned: true, unread: 1,
                agent: .claude, message: "Approve cargo test?", activitySince: minutesAgo(1),
                folder: "\(app)/crates/wormhole-term", changeCount: 12, worktreeOf: app),
            TerminalSummary(id: 4, title: "zsh", branch: "main", folder: app),
            TerminalSummary(
                id: 5, title: "landing hero", branch: "hero-copy", activity: .working, agent: .codex,
                message: "Editing hero.tsx…", activitySince: minutesAgo(8), folder: web, splitID: 1, changeCount: 4, repoID: "web"),
            TerminalSummary(
                id: 6, title: "pnpm build", branch: "hero-copy", activity: .working,
                runningCommand: "pnpm build", runningSince: minutesAgo(0.3), folder: web, splitID: 1,
                changeCount: 4, repoID: "web"),
            TerminalSummary(
                id: 7, title: "deploy preview", branch: "hero-copy", activity: .failed, unread: 3,
                lastFailure: CommandFailure(command: "pnpm deploy", exitCode: 1, at: minutesAgo(2)), folder: web,
                changeCount: 4, repoID: "web"),
            TerminalSummary(id: 8, title: "ssh build-mac", folder: "~/infra", isHiddenFromAgents: true),
            TerminalSummary(
                id: 9, title: "terraform plan", branch: "network-split", activity: .finished, unread: 1,
                folder: "~/infra"),
            TerminalSummary(id: 10, title: "htop", isPinned: true, folder: "~"),
            TerminalSummary(
                id: 11, title: "docs pass", activity: .finished, agent: .codex, activitySince: minutesAgo(12),
                folder: "~/Documents/notes", noteCount: 3),
            TerminalSummary(
                id: 12, title: "release 1.7", branch: "release/1.7", activity: .needsInput, isPinned: true,
                unread: 1, agent: .claude, message: "Run the release script?", activitySince: minutesAgo(5),
                folder: app, noteCount: 1),
            TerminalSummary(id: 13, title: "scratch", folder: "/tmp", forgetsText: true),
        ]
        let groups = [
            TerminalGroup(id: "wormhole", name: "Wormhole", members: [1, 2, 3, 4], color: .blue, folder: app),
            TerminalGroup(id: "website", name: "Website", members: [5, 6, 7], color: .purple, folder: web),
            TerminalGroup(id: "infra", name: "Infra", members: [8, 9], isCollapsed: true, color: .cyan, folder: "~/infra"),
        ]
        // Groups and loose terminals side by side; the pinned loose ones (10, 12) lead whatever the order says.
        let arrangement: [TopItem] = [.group("wormhole"), .group("website"), .terminal(11), .group("infra"), .terminal(13)]
        return WorkspaceState(terminals: terminals, groups: groups, selected: 1, arrangement: arrangement)
    }()
}

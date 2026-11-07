// Sample data for the UI modules' previews, screenshots and tests: a window as a working day leaves it.
// Three groups (one folded), pinned terminals in and out of a group, and every `Activity`.

import Foundation

public enum MockWorkspace {
    public static let state: WorkspaceState = {
        let app = "~/Projects/terminal/wormhole"
        let web = "~/Projects/terminal/web"
        let terminals = [
            TerminalSummary(
                id: 1, title: "claude · sidebar polish", subtitle: app, branch: "sidebar-polish",
                activity: .working),
            TerminalSummary(
                id: 2, title: "cargo test", subtitle: app, branch: "main", activity: .finished, unread: 2),
            TerminalSummary(
                id: 3, title: "claude · diff perf", subtitle: "\(app)/crates/wormhole-term",
                branch: "diff-perf", activity: .needsInput, isPinned: true, unread: 1),
            TerminalSummary(id: 4, title: "zsh", subtitle: app, branch: "main"),
            TerminalSummary(
                id: 5, title: "codex · landing hero", subtitle: web, branch: "hero-copy", activity: .working),
            TerminalSummary(id: 6, title: "pnpm dev", subtitle: web, branch: "main"),
            TerminalSummary(
                id: 7, title: "deploy preview", subtitle: web, branch: "hero-copy", activity: .failed,
                unread: 3),
            TerminalSummary(id: 8, title: "ssh build-mac", subtitle: "~/infra"),
            TerminalSummary(
                id: 9, title: "terraform plan", subtitle: "~/infra", branch: "network-split",
                activity: .finished),
            TerminalSummary(id: 10, title: "htop", subtitle: "~", isPinned: true),
            TerminalSummary(id: 11, title: "notes", subtitle: "~/Documents/notes"),
            TerminalSummary(
                id: 12, title: "claude · release 1.7", subtitle: app, branch: "release/1.7",
                activity: .needsInput, isPinned: true, unread: 1),
            TerminalSummary(id: 13, title: "scratch", subtitle: "/tmp"),
        ]
        let groups = [
            TerminalGroup(id: "wormhole", name: "Wormhole", members: [1, 2, 3, 4]),
            TerminalGroup(id: "website", name: "Website", members: [5, 6, 7]),
            TerminalGroup(id: "infra", name: "Infra", members: [8, 9], isCollapsed: true),
        ]
        return WorkspaceState(terminals: terminals, groups: groups, selected: 1)
    }()
}

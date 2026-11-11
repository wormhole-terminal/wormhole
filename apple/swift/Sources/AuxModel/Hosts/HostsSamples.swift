// Sample hosts for previews and tests: one of each standing, one with a version mismatch.

import Foundation

public enum HostsSamples {
    private static func facts(pid: Int, shell: Int, program: String, version: String = "1.6.3", tone: HostFact.Tone = .normal) -> [HostFact] {
        [
            HostFact(.hostPid, "\(pid)"), HostFact(.shellPid, "\(shell)"), HostFact(.program, program),
            HostFact(.memory, "9.4 MiB"), HostFact(.version, version, tone: tone), HostFact(.protocolRange, "1 to 3, compatible"),
            HostFact(.started, "2026-11-07 09:12 UTC (up 3 h 20 min)"), HostFact(.answer, "Yes, program running, 120\u{D7}32, 4,210 lines of history"),
            HostFact(.link, "Repaint, then stream; 1.2 MiB in, 4 KiB out; 311 frames in, 40 out"),
            HostFact(.folder, "~/Projects/terminal"), HostFact(.socket, "~/Library/Application Support/Wormhole/hosts/h-7f3a.sock", tone: .muted),
            HostFact(.meta, "~/Library/Application Support/Wormhole/hosts/h-7f3a.json", tone: .muted),
        ]
    }

    public static let report = HostsReport(
        appVersion: "1.6.3", protocolText: "3", folder: "~/Library/Application Support/Wormhole/hosts", flagOn: true,
        program: .found(path: "…/Wormhole.app/Contents/MacOS/wormhole-host"),
        cards: [
            HostCard(
                id: "h-7f3a", standing: .attached, note: AuxText.hostsAttachedTo(window: 1, title: "api"),
                facts: facts(pid: 4182, shell: 4190, program: "claude"), attachedTitle: "api"),
            HostCard(
                id: "h-21c9", standing: .detached, note: AuxText.hostsDetachedNote,
                facts: facts(pid: 3901, shell: 3905, program: "zsh", version: "1.6.2 (This app is 1.6.3)", tone: .warning),
                needsAttention: true),
            HostCard(id: "h-0b44", standing: .orphan, note: AuxText.hostsOrphanNote, facts: facts(pid: 2210, shell: 2214, program: "npm run dev")),
            HostCard(
                id: "h-e5d0", standing: .stale, note: "Its process is gone",
                facts: [HostFact(.hostPid, "Unknown"), HostFact(.meta, "~/Library/Application Support/Wormhole/hosts/h-e5d0.json", tone: .muted)]),
        ])
}

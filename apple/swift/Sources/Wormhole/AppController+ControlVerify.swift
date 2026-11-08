// The control folder's commands that show what a surface would say without a pointer, a key window or a system surface:
// the palette's rows for a query (and running its first result as ↩ does), the shortcut sheet's sections, the actions
// the keymap lists that nothing implements yet, the words of another language, a command a moment from now (to catch an
// animation half way) and what the menu bar extra, the Dock and the quick panel would list for the terminals the core
// has. The models are the real ones; only the NSStatusItem, the Dock tile and the panel (never made in `--background`)
// are left out.

import AppKit
import Foundation
import PaletteModel
import PlatformModel
import WorkspaceModel

#if WITH_UNIFFI
    import WordsBridge
#endif

extension AppController {
    func runVerify(command: String, argument: String) -> Bool {
        switch command {
        case "palette-rows", "palette-run": runPalette(command, argument)
        case "shortcut-rows":
            reply(ShortcutSheet.sections.map { s in "\(s.title)\n" + s.rows.map { "  \($0.label)  \($0.keysText)" }.joined(separator: "\n") }.joined(separator: "\n"))
        case "actions-missing":
            let wanted = Keymap.shared.entries.filter { $0.stage == .now && !$0.debugOnly }
            let missing = wanted.filter { !router.implemented.contains($0.id) }
            reply("keymap now=\(wanted.count) implemented=\(wanted.count - missing.count) missing=\(missing.count)\n" + missing.map { $0.id.rawValue }.joined(separator: " "))
        case "ambient": reply(ambientText(query: argument))
        case "after":
            // `after <ms> <command>`: a command a moment from now, so a shot can catch an animation half way.
            let words = argument.split(separator: " ", maxSplits: 1).map(String.init)
            guard words.count == 2, let ms = Int(words[0]) else { reply("error: usage after <ms> <command>"); return true }
            DispatchQueue.main.asyncAfter(deadline: .now() + .milliseconds(ms)) { [weak self] in
                MainActor.assumeIsolated { self?.run(command: words[1]) }
            }
        case "language":
            // GPUI's `language en-XA`: the words of that language from here on (windows opened afterwards read them).
            #if WITH_UNIFFI
                WordsBridge.install(tag: argument == "system" ? nil : argument)
                reply("language \(argument)")
            #endif
        default: return runVerifyPages(command: command, argument: argument)
        }
        return true
    }

    /// `palette-rows <query>` lists what the palette shows; `palette-run <query>` runs its first result.
    private func runPalette(_ command: String, _ argument: String) {
        let (scope, query) = PaletteScope.parse(argument)
        let rows = PaletteRows.rows(scope: scope, query: query, commands: CommandCatalog.commands(for: hub.state))
        guard command == "palette-run" else { return reply(rows.map(Self.line).joined(separator: "\n")) }
        guard let found = rows.compactMap(\.result).first?.command else { return reply("palette-run nothing found") }
        if let id = found.appAction { router.run(ActionID(rawValue: id)) } else if let action = found.action { hub.send(action) }
        reply("palette-run \(found.title)")
    }

    /// The status item's menu and icon, the Dock's menu and badge and the quick panel's rows for the terminals as they are.
    private func ambientText(query: String) -> String {
        let terminals = ambientTerminals
        var snapshot = StatusSnapshot()
        snapshot.terminals = terminals
        let status = StatusMenu.build(snapshot, now: Date())
        let counts = AmbientCounts(of: terminals)
        return "icon \(status.icon)\n" + MenuDump.lines(status.menu, depth: 1) + "dock\n" + MenuDump.lines(DockMenu.build(terminals, now: Date()), depth: 1)
            + "badge needs_you=\(DockBadgeMode.needsYou.label(for: counts) ?? "none") needs_you_done=\(DockBadgeMode.needsYouDone.label(for: counts) ?? "none")\n"
            + "quick\n" + QuickPanelRows.rows(terminals, [.newTerminal(cwd: nil)], query: query).map { "  \($0)" }.joined(separator: "\n")
    }

    private static func line(_ row: PaletteRow) -> String {
        switch row {
        case .label(let title, let count): "# \(title)\(count.map { " (\($0))" } ?? "")"
        case .result(let r): "  \(r.command.title)\(r.command.detail.map { "  - \($0)" } ?? "")\(r.command.shortcut.map { "  [\($0.glyphs)]" } ?? "")  <\(r.command.id)>"
        case .note(let text): "  (\(text))"
        }
    }
}

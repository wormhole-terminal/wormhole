import Foundation
import Testing

/// The menu bar extra's menu (PL04). A test instance makes no status item, so its menu is read as the model the item is
/// given: `ambient` builds it with the same `StatusMenu.build` over the app's terminals and prints it before the Dock's.
extension EndToEnd {
    /// The status item's part of an `ambient` reply: from the icon's line to the Dock's.
    private static func statusMenu(_ ambient: String) -> String {
        ambient.components(separatedBy: "\ndock\n").first ?? ambient
    }

    @Test("the menu bar extra's menu lists a waiting agent under Needs input and the rest under Other Terminals; Jump to Next Waiting is on only then (PL04)")
    @MainActor func statusItemMenu() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            await app.send(["key cmd-t"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let waiting = two.live.first, let other = two.live.last else { return }

            // Nobody waits: no section, both terminals under Other Terminals, and nothing to jump to.
            let idle = Self.statusMenu(await app.until("ambient") { $0.contains("Other Terminals") })
            app.expect(!idle.contains("# Needs input"), "no Needs input section while nobody waits:\n\(idle)")
            app.expect(idle, contains: "Other Terminals  ⇥ 2 ▸")
            app.expect(idle, contains: "Jump to Next Waiting  (disabled)")

            try await app.report(agent: "codex", event: "PermissionRequest", terminal: waiting, becomes: "needsInput")
            let menu = Self.statusMenu(await app.until("ambient") { $0.contains("# Needs input") })
            let lines = menu.split(separator: "\n").map(String.init)
            // The section's first row is the waiting terminal's, by the name its sidebar row has.
            let name = (await app.rows()).row(waiting)?.title ?? ""
            let head = lines.firstIndex { $0.contains("# Needs input") } ?? 0
            app.expect(lines.indices.contains(head + 1) && lines[head + 1].contains(name), "the waiting terminal \"\(name)\" is listed under Needs input:\n\(menu)")
            app.expect(menu, contains: "Other Terminals  ⇥ 1 ▸")
            app.expect(!menu.contains("Jump to Next Waiting  (disabled)"), "Jump to Next Waiting is on while one waits:\n\(menu)")
            app.expect(lines.first != idle.split(separator: "\n").first.map(String.init), "the icon says someone waits: \(lines.first ?? "")")
            // The fixed rows, in the menu's order.
            let fixed = ["New Terminal", "Open Folder…", "Show Wormhole", "Jump to Next Waiting", "Settings…", "Check for Updates…", "Quit Wormhole"]
            let places = fixed.map { title in lines.firstIndex { $0.trimmingCharacters(in: .whitespaces).hasPrefix(title) } ?? -1 }
            app.expect(!places.contains(-1) && places == places.sorted(), "the menu's own rows in order: \(places)\n\(menu)")

            // Its rows run: the waiting terminal's row selects it from the other one.
            await app.send(["select \(other)", "ambient-run open-tab \(waiting)"])
            await app.until("rows") { Listing($0).selected == waiting }
        }
    }
}

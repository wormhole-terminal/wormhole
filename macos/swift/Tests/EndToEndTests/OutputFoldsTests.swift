import Foundation
import Testing

/// Output folds past the one closed marker `foldAndExport` shows: what a fold hides and gives back when its marker opens,
/// Edit ▸ Show Hidden Lines, "Never hide lines", and the rules that refuse a fold (`wormhole_term::folds`). Read from the last
/// frame: `probes` lists the marker rows the view was given, `screen` the rows it drew.
extension EndToEnd {
    /// The fold's lines: a run of numbers no other command of these scenarios prints.
    private static let folded = 7101...7112

    /// zsh with prompt marks, `seq 7101 7112` run and pushed off the live screen by a longer listing (only the history folds),
    /// the folds' flag on. The block's number in `blocks`, nil (the failure recorded) when it is not there.
    @MainActor private static func foldable(_ app: Instance) async -> Int? {
        await app.markPrompts()
        await app.flags(["output_folds": true])
        let command = "/usr/bin/seq \(folded.lowerBound) \(folded.upperBound)"
        await app.shell(command)
        await app.shell("/usr/bin/seq 1 160")
        let listing = await app.until("blocks") { !Instance.blocks($0, running: "/usr/bin/seq 1 160").isEmpty }
        guard let block = Instance.blocks(listing, running: command).first else {
            app.fail("seq's block:\n\(listing)")
            return nil
        }
        return block.number
    }

    /// How many of the fold's lines the view drew.
    private static func drawn(_ screen: String) -> Int {
        let lines = Set(screen.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) })
        return folded.filter { lines.contains(String($0)) }.count
    }

    /// The wheel to the top of the history, where the fold is, then the frame's markers and rows once `done` holds for both.
    @MainActor @discardableResult private static func top(_ app: Instance, _ done: (_ probes: String, _ screen: String) -> Bool) async -> (probes: String, screen: String) {
        var last = (probes: "", screen: "")
        let start = Date()
        repeat {
            let said = await app.send(["scroll 700 400 -6000", "frames 2", "probes", "screen", "delay 150"])
            last = (Instance.plain(said[2]), said[3])
            if done(last.probes, last.screen) { return last }
        } while Date().timeIntervalSince(start) < 20
        app.fail("the top of the history never showed what was expected:\n\(last.probes)\n\(last.screen)")
        return last
    }

    @Test("a fold hides its lines behind one marker; the marker opens and closes it; Show Hidden Lines opens every fold")
    @MainActor func foldOpensClosesAndShowsAll() async throws {
        try await Instance.with(shell: .zsh) { app in
            guard let block = await Self.foldable(app) else { return }
            // Nothing folded yet: every line is drawn and Edit ▸ Show Hidden Lines is off.
            let open = await Self.top(app) { _, screen in Self.drawn(screen) == Self.folded.count }
            app.expect(!open.probes.contains("[closed]") && !open.probes.contains("[open]"), "no marker before a fold: \(open.probes)")
            app.expect(await app.reply("available showHiddenLines"), contains: "false")

            await app.send(["block \(block) fold"])
            let closed = await Self.top(app) { probes, _ in probes.contains("[closed]") }
            // The marker counts what it hides, and the rows are not drawn: what is left of the twelve is at most the two the
            // rules keep beside the command (a fold of fewer than three is never made).
            app.expect(closed.probes.contains("lines hidden"), "the marker says how many lines it hides: \(closed.probes)")
            let left = Self.drawn(closed.screen)
            app.expect(left <= Self.folded.count - 3, "the fold's lines are not drawn (\(left) of \(Self.folded.count) are):\n\(closed.screen)")
            app.expect(await app.reply("available showHiddenLines"), contains: "true")

            // The marker's click (`fold-toggle`: the first marker, as the person's click toggles it): open shows every line again
            // and keeps its marker, a second click hides them.
            await app.send(["fold-toggle"])
            let opened = await Self.top(app) { probes, screen in probes.contains("[open]") && Self.drawn(screen) == Self.folded.count }
            app.expect(!opened.probes.contains("[closed]"), "the one fold is open: \(opened.probes)")
            await app.send(["fold-toggle"])
            await Self.top(app) { probes, screen in probes.contains("[closed]") && Self.drawn(screen) == left }

            // Edit ▸ Show Hidden Lines opens it too.
            await app.send(["action showHiddenLines"])
            await Self.top(app) { probes, screen in !probes.contains("[closed]") && Self.drawn(screen) == Self.folded.count }
        }
    }

    @Test("Never hide lines shows what was folded and makes no new fold; turned off, a block folds again")
    @MainActor func neverHideLines() async throws {
        try await Instance.with(shell: .zsh) { app in
            guard let block = await Self.foldable(app) else { return }
            await app.send(["block \(block) fold"])
            let closed = await Self.top(app) { probes, _ in probes.contains("[closed]") }
            let left = Self.drawn(closed.screen)

            await app.send(["set never_hide_lines=true"])
            await app.until("get never_hide_lines") { $0.contains("true") }
            // Every line is back and there is no marker to click.
            await Self.top(app) { probes, screen in !probes.contains("[closed]") && Self.drawn(screen) == Self.folded.count }
            // Fold Output does nothing while it is on.
            await app.send(["block \(block) fold", "wait-idle 2000"])
            let refused = await Self.top(app) { _, screen in Self.drawn(screen) == Self.folded.count }
            app.expect(!refused.probes.contains("[closed]"), "no fold is made while lines are never hidden: \(refused.probes)")

            await app.send(["set never_hide_lines=false"])
            await app.until("get never_hide_lines") { $0.contains("false") }
            // The block folds once the setting allows it again (the app follows the file a moment after `set` wrote it).
            var folded = (probes: "", screen: "")
            let start = Date()
            repeat {
                await app.send(["block \(block) fold"])
                folded = await Self.top(app) { _, _ in true }
            } while !folded.probes.contains("[closed]") && Date().timeIntervalSince(start) < 20
            app.expect(folded.probes.contains("[closed]") && Self.drawn(folded.screen) == left, "the block folds again:\n\(folded.probes)\n\(folded.screen)")
        }
    }

    @Test("a fold is refused on the live screen and while a program reports the mouse; a bell opens the recent folds")
    @MainActor func foldGuards() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.markPrompts()
            await app.flags(["output_folds": true])
            // A block whose output is still on the live screen: Fold Output makes no fold.
            let command = "/usr/bin/seq \(Self.folded.lowerBound) \(Self.folded.upperBound)"
            await app.shell(command)
            let listing = await app.until("blocks") { !Instance.blocks($0, running: command).isEmpty }
            guard let block = Instance.blocks(listing, running: command).first else { return app.fail("seq's block:\n\(listing)") }
            let live = (await app.send(["block \(block.number) fold", "wait-idle 2000", "frames 2", "probes", "screen"]))
            app.expect(!live[3].contains("[closed]"), "the live screen never folds: \(live[3])")
            app.expect(Self.drawn(live[4]) == Self.folded.count, "its lines stay drawn:\n\(live[4])")
            app.expect(await app.reply("available showHiddenLines"), contains: "false")

            // In the history (sixty lines push it off the screen, and leave it within a screen of it: "recent" for the bell below),
            // but a program reports the mouse (its coordinates are screen rows): no fold either.
            await app.shell("/usr/bin/seq 1 60")
            await app.until("blocks") { !Instance.blocks($0, running: "/usr/bin/seq 1 60").isEmpty }
            await app.shell(#"printf '\e[?1000h'"#)
            await app.until("blocks") { $0.contains("printf") }
            // Not looked for with the wheel, which is the program's while it reports the mouse: no fold exists, so Edit ▸ Show
            // Hidden Lines stays off (the engine's own `has_folds`).
            let mouse = await app.send(["block \(block.number) fold", "wait-idle 2000", "frames 2", "available showHiddenLines"])
            app.expect(mouse[3], contains: "false")
            await app.shell(#"printf '\e[?1000l'"#)
            await app.until("blocks") { $0.components(separatedBy: "printf").count > 2 }

            // Without it the same block folds; a bell then opens the recent folds, and they stay open.
            var folded = (probes: "", screen: "")
            let start = Date()
            repeat {
                await app.send(["block \(block.number) fold"])
                folded = await Self.top(app) { _, _ in true }
            } while !folded.probes.contains("[closed]") && Date().timeIntervalSince(start) < 20
            app.expect(folded.probes.contains("[closed]"), "the block folds once nothing reports the mouse: \(folded.probes)")
            await app.shell(#"printf '\a'"#)
            await Self.top(app) { probes, screen in probes.contains("[open]") && Self.drawn(screen) == Self.folded.count }
        }
    }
}

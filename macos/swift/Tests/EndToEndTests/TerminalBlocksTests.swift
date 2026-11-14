import Foundation
import Testing

/// Command blocks, from the prompt marks a shell integration writes (`markPrompts`, OSC 133): what `blocks` lists, the Edit
/// menu's actions on a block (`block <n> …`), ⌘↑ and the target it leaves, the context menu over a block, Fold Output and
/// Export Text As…, and the banners a program or a long command makes (`notices`, the instance's log of them).
extension EndToEnd {
    @Test("commands become blocks with their state; Copy Output, Copy Command and Put Command at Prompt act on one; ⌘↑ is forgotten on typing")
    @MainActor func commandBlocks() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.markPrompts()
            await app.shell("echo first-out")
            await app.shell("false")
            await app.shell("echo second-out")
            let listing = await app.until("blocks") { !Instance.blocks($0, running: "echo second-out").isEmpty }
            guard let first = Instance.blocks(listing, running: "echo first-out").first, let failed = Instance.blocks(listing, running: "false").first,
                let second = Instance.blocks(listing, running: "echo second-out").first
            else { return app.fail("the three commands are blocks:\n\(listing)") }
            app.expect(first.line, contains: "exit 0")
            app.expect(failed.line, contains: "exit 1")
            app.expect(Instance.outputStart(first.line) != nil, "echo's block has output: \(first.line)")

            await app.send(["block \(first.number) copy-output"])
            app.expect(await app.pasteboard().hasPrefix("first-out"), "Copy Output copied echo's output")
            await app.send(["block \(first.number) copy-command"])
            app.expect(await app.pasteboard().trimmingCharacters(in: .whitespaces) == "echo first-out", "Copy Command copied the command")
            // Put at the prompt, not run: the screen's last prompt has it, and no third `first-out` line comes.
            await app.send(["block \(first.number) to-prompt"])
            await app.screen { $0.split(separator: "\n").contains { $0.hasPrefix("$ echo first-out") } }
            await app.send(["type \u{15}"])

            // The Edit menu acts on the newest block whose command ran until ⌘↑ lands on another.
            app.expect(await app.reply("block"), contains: "command=echo second-out")
            await app.send(["key cmd-up", "key cmd-up"])
            let jumped = await app.reply("block")
            app.expect(!jumped.contains("command=echo second-out"), "⌘↑ moved the target off the newest block: \(jumped)")
            // A key typed forgets the jump (`MetalTerminalView.onInput`).
            await app.send(["text x"])
            await app.until("block") { $0.contains("command=echo second-out") }
            await app.send(["key cmd-up", "key cmd-up", "pasteboard-set y", "key cmd-v"])
            await app.until("block") { $0.contains("command=echo second-out") }
            await app.send(["type \u{15}"])

            // The context menu over echo's output offers its block, Put Command at Prompt on since the shell waits at its prompt.
            guard let row = Instance.outputStart(first.line) else { return }
            await app.send(["pointer \(row) 0"])
            let menu = await app.reply("context-menu")
            for item in ["Copy Command [", "Copy Output [", "Put Command at Prompt ["] {
                app.expect(menu.split(separator: "\n").contains { $0.hasPrefix(item) && !$0.contains(" off]") }, "\(item) is on:\n\(menu)")
            }
        }
    }

    @Test("Fold Output is offered only with its feature on, folds a block's output, and Export Text As keeps the folded lines")
    @MainActor func foldAndExport() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.markPrompts()
            await app.shell("/usr/bin/seq 101 112")
            let listing = await app.until("blocks") { !Instance.blocks($0, running: "/usr/bin/seq 101 112").isEmpty }
            guard let block = Instance.blocks(listing, running: "/usr/bin/seq 101 112").first, let row = Instance.outputStart(block.line) else {
                return app.fail("seq's block with its output:\n\(listing)")
            }
            await app.flags(["output_folds": false])
            await app.send(["pointer \(row) 0"])
            await app.until("context-menu") { !$0.contains("Fold Output") && $0.contains("Copy Output") }
            await app.flags(["output_folds": true])
            let menu = await app.until("context-menu") { $0.contains("Fold Output") }
            let items = menu.split(separator: "\n").map(String.init)
            let put = items.firstIndex { $0.hasPrefix("Put Command at Prompt") }, fold = items.firstIndex { $0.hasPrefix("Fold Output") }
            app.expect(put != nil && fold == put.map { $0 + 1 }, "Fold Output comes right after Put Command at Prompt:\n\(menu)")

            await app.send(["block \(block.number) fold"])
            let folded = await app.until("probes") { $0.contains("[closed]") }
            app.expect(Instance.plain(folded).contains("markers ["), "a marker for the folded output: \(folded)")

            // Export Text As… writes into the instance's own exports folder (never the person's Documents), folded lines too.
            let file = app.scratch.data.appendingPathComponent("exports/blocks-export.txt")
            await app.send(["action exportText"])
            await app.until("windows") { $0.contains("sheet=true") }
            await app.send(["sheet ok \(file.path)"])
            app.expect(await app.waitFor(file), "the export was written: \(file.path)")
            let text = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
            app.expect((101...112).allSatisfy { text.contains("\n\($0)\n") || text.hasPrefix("\($0)\n") }, "every line of the fold is in the export:\n\(text)")
        }
    }

    @Test("a program's OSC 9 and 777 notices, a long command's finish and failure, and `Wormhole notify` are banners")
    @MainActor func programNotices() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.markPrompts()
            let before = Listing(await app.reply("rows"))
            guard let terminal = before.selected else { return app.fail("a terminal is selected") }
            await app.shell(#"printf '\e]9;hello-from-nine\a'"#)
            await app.until("notices") { $0.contains("hello-from-nine") }
            await app.shell(#"printf '\e]777;notify;Seven-Title;seven-body\a'"#)
            await app.until("notices") { $0.contains("Seven-Title: seven-body") }
            // The same words again are a banner again.
            await app.shell(#"printf '\e]777;notify;Seven-Title;seven-body\a'"#)
            await app.until("notices") { $0.components(separatedBy: "Seven-Title: seven-body").count == 3 }

            // A command that runs longer than `notify_command_after` seconds says it finished, or failed, with its name.
            await app.send(["set notify_command_after=2"])
            await app.shell("/bin/sleep 3")
            await app.until("notices", timeout: 30) { $0.contains("sleep") }
            let finished = (await app.reply("notices")).components(separatedBy: "sleep").count
            await app.shell("/bin/sleep 3; false")
            await app.until("notices", timeout: 30) { $0.components(separatedBy: "sleep").count > finished }

            // `Wormhole notify` from inside the terminal: a banner in the caller's words; the row's state does not move.
            let said = await app.reply("control --from \(terminal) notify --title Called-Title --body called-body")
            app.expect(!said.hasPrefix("error"), "the request was taken: \(said)")
            await app.until("notices") { $0.contains("Called-Title: called-body") }
            let after = Listing(await app.reply("rows"))
            app.expect(after.row(terminal)?.activity == before.row(terminal)?.activity, "the row's state is the same:\n\(before.text)\n\(after.text)")
        }
    }
}

import Foundation
import Testing

/// The title row's pull request (P04's PR info): asked of `gh` only in a trusted project, shown as the chip, opened by a click.
/// `gh` is never the person's: the instance looks programs up on its own agent path only (`WORMHOLE_AGENT_PATH`, core
/// `agents::find_on_path`, which `git::pr::program` uses), and there a stand-in script answers as `gh pr view --json` does and
/// writes down how it was called. Nothing reaches a network or an account.
extension EndToEnd {
    /// `gh pr view`'s answer for an open pull request whose checks failed and whose reviewer asked for changes.
    static let pullRequestJSON =
        #"{"number":142,"url":"https://github.invalid/e2e/repo/pull/142","state":"OPEN","isDraft":false,"reviewDecision":"CHANGES_REQUESTED","statusCheckRollup":[{"status":"COMPLETED","conclusion":"FAILURE"}]}"#

    /// The stand-in `gh` on the instance's agent path (made by `Instance.with(agents: ["gh"])`), rewritten to answer
    /// `pullRequestJSON` and to append a line per call to `log`: its folder, its arguments and the variables it was given.
    @MainActor static func standInGH(_ app: Instance, log: URL) throws {
        let gh = app.scratch.root.appendingPathComponent("agent-path/gh")
        let script = """
            #!/bin/sh
            echo "$(pwd -P)|$*|prompt=${GH_PROMPT_DISABLED:-unset}|pager=${GH_PAGER:-unset}|gitconfig=${GIT_CONFIG_GLOBAL:-unset}" >> '\(log.path)'
            printf '%s\\n' '\(pullRequestJSON)'

            """
        try Data(script.utf8).write(to: gh)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: gh.path)
    }

    // GPUI: `window/title/extras.rs:183` (asked only for a branch whose project is trusted, on the next tick), `:82` (shown only
    // while it is), core `git::pr::pull_request` (`pr view --json …`, the scrubbed environment), `PullRequest::words`.
    @Test("the pull request is asked of gh only once the project is trusted, the chip says it in core's words and opens it; revoked, it goes")
    @MainActor func pullRequestOnlyInATrustedProject() async throws {
        try await Instance.with(auth: "allow", agents: ["gh"]) { app in
            let log = app.scratch.root.appendingPathComponent("gh-calls.txt")
            try Self.standInGH(app, log: log)
            let repo = try app.scratch.repository("pr-repo")
            try app.scratch.project(repo, files: ["actions.json": Self.sayAction])
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            // The terminal is on the repository's branch (the title row says it) and its status is read: the branch is known.
            await app.until("title") { $0.contains("main") }
            await app.until("changes") { $0.contains("phase=loaded") }

            // Untrusted: `gh` reads the repository's git configuration, which nobody has approved, so it is never started.
            app.expect(!FileManager.default.fileExists(atPath: log.path), "gh ran in an untrusted project:\n\((try? String(contentsOf: log, encoding: .utf8)) ?? "")")
            app.expect(await app.reply("pull-request"), contains: "error: no pull request")

            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }

            // Trusted: asked on the next look (GPUI's next tick), and the chip is the core's answer.
            let title = await app.until("title") { $0.contains("number: 142") }
            app.expect(title.contains("number: 142"), "the chip is shown once the project is trusted:\n\(title)")
            let calls = ((try? String(contentsOf: log, encoding: .utf8)) ?? "").split(separator: "\n").map(String.init)
            let call = calls.first ?? ""
            app.expect(call.contains("/home/pr-repo|"), "gh ran in the repository: \(call)")
            app.expect(call.contains("|pr view --json number,url,state,isDraft,reviewDecision,statusCheckRollup|"), "the fixed arguments: \(call)")
            app.expect(call.contains("prompt=1") && call.contains("pager=cat"), "no prompts, no pager: \(call)")
            // The app's own environment has GIT_CONFIG_GLOBAL; gh is given only what it needs to reach a host.
            app.expect(call.contains("gitconfig=unset"), "the environment is scrubbed: \(call)")

            // A click on the chip opens the address gh gave (a test instance records it instead of starting the browser).
            let clicked = await app.reply("pull-request")
            app.expect(clicked, contains: "pull-request #142 Pull request #142, open, checks failing, changes requested")
            app.expect(await app.reply("launches"), contains: "open https://github.invalid/e2e/repo/pull/142")

            // Revoked: the row shows a pull request only for a trusted project (GPUI's `pull_request_branch`), so the chip goes.
            await app.send(["project-revoke"])
            await app.until("project-state") { $0.contains("status=untrusted") }
            let gone = await app.until("title", timeout: 10) { !$0.contains("number: 142") }
            app.expect(!gone.contains("number: 142"), "no chip once the trust is revoked:\n\(gone)")
        }
    }
}

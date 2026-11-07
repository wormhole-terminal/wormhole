@testable import SettingsBridge
import SettingsModel
import WormholeBindings
import XCTest

final class RecordMappingTests: XCTestCase {
    private func agent(onPath: Bool, _ hooks: SettingsHookState) -> AgentRecord {
        RecordMapping.agent(
            SettingsAgentRow(
                id: "claude", name: "Claude Code", twoLetterMark: "CC", onPath: onPath, hooks: hooks, detail: "detail",
                configPath: nil, note: nil))
    }

    func testAnAgentsStateAndButtonFollowItsHooks() {
        XCTAssertEqual(agent(onPath: true, .installed).state, .ready)
        XCTAssertEqual(agent(onPath: true, .installed).hooks, .remove)
        XCTAssertEqual(agent(onPath: true, .partial).hooks, .repair)
        XCTAssertEqual(agent(onPath: true, .partial).state, .needsRepair)
        XCTAssertEqual(agent(onPath: true, .notInstalled).hooks, .install)
        XCTAssertEqual(agent(onPath: true, .notInstalled).state, .statusFromOutput)
        XCTAssertNil(agent(onPath: true, .noHooks).hooks, "no hook support, no button")
        XCTAssertEqual(agent(onPath: false, .installed).state, .notInstalled, "not on PATH is listed apart")
        XCTAssertNil(agent(onPath: false, .installed).hooks)
        XCTAssertEqual(agent(onPath: true, .unreadable(reason: "bad json")).detail, "detail · bad json")
    }

    func testTheProfileRecordIsTheActiveOnesWithHomeForAnEmptyStart() {
        let row = { (id: String, active: Bool, start: String) in
            SettingsProfileRow(
                id: id, name: id.capitalized, isDefault: id == "default", isActive: active, startDir: start, gitName: "Ada",
                gitEmail: "ada@example.com")
        }
        let profile = RecordMapping.profile([row("default", false, ""), row("work", true, "~/work")])
        XCTAssertEqual(profile?.name, "Work")
        XCTAssertEqual(profile?.startFolder, "~/work")
        XCTAssertEqual(RecordMapping.profile([row("default", true, "")])?.startFolder, "~")
        XCTAssertNil(RecordMapping.profile([]))
    }

    func testAnImportRowKeepsItsWords() {
        let record = RecordMapping.imported(
            SettingsImportRow(id: "ghostty", app: "Ghostty", summary: "13 settings", settings: 13, skipped: 2, files: []))
        XCTAssertEqual(record, ImportRecord(id: "ghostty", app: "Ghostty", summary: "13 settings"))
    }
}

final class LiveRecordMappingTests: XCTestCase {
    func testTheCommandLineRowKeepsCoresSentencesAndOffersUpdateOnlyForAStaleCopy() {
        let installed = CliToolRecord(
            state: .installed, command: "wh", shownPath: "~/.local/bin/wh", folder: "/Users/x/.local/bin",
            headline: "wh in any terminal. Installed at ~/.local/bin/wh", note: "stale", current: false)
        let row = RecordMapping.commandLine(installed)
        XCTAssertEqual(row.state, .installed)
        XCTAssertFalse(row.current)
        XCTAssertEqual(row.note, "stale")
        XCTAssertEqual(row.folder, "/Users/x/.local/bin")
        XCTAssertEqual(RecordMapping.commandLine(CliToolRecord(
            state: .foreign, command: "wh", shownPath: nil, folder: nil, headline: "other", note: "n", current: false)).state, .foreign)
        let off = RecordMapping.pathNote(CliToolPathRecord(verdict: .off, text: "not on PATH", adviceFile: "~/.zshrc", adviceLine: "export PATH=…"))
        XCTAssertEqual(off.verdict, .off)
        XCTAssertEqual(off.line, "export PATH=…")
        XCTAssertEqual(RecordMapping.pathNote(CliToolPathRecord(verdict: .on, text: "ok", adviceFile: nil, adviceLine: nil)).verdict, .on)
    }

    func testAnApprovalKeepsWhetherItIsStoredAndTheProjectMarkNeedsARowItSets() {
        let held = AccessApprovalRecord(
            id: 3, stored: false, label: "Claude Code", how: .session, agent: "claude", detail: "While it runs",
            location: "", allowed: ["Read"])
        let row = RecordMapping.approval(held)
        XCTAssertEqual(row.number, 3)
        XCTAssertFalse(row.isStored)
        XCTAssertEqual(row.id, "held-3")
        XCTAssertEqual(RecordMapping.approval(AccessApprovalRecord(
            id: 3, stored: true, label: "x", how: .always, agent: nil, detail: "", location: "", allowed: [])).id, "stored-3")

        let project = ProjectSettingsRecord(name: "app", dir: ".wormhole", root: "/p/app", note: "Set by app/.wormhole/config.json", json: "{}")
        XCTAssertNil(RecordMapping.project(project, entries: []), "a project that sets no row marks none")
        XCTAssertEqual(RecordMapping.project(project, entries: ["font-terminal"])?.entries, ["font-terminal"])
        XCTAssertEqual(RecordMapping.project(project, entries: ["font-terminal"])?.note, "Set by app/.wormhole/config.json")
    }
}

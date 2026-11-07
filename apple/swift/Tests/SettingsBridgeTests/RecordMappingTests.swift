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

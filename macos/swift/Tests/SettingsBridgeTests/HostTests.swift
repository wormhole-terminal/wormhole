import Foundation
import SettingsBridge
import SettingsModel
import WormholeBindings
import XCTest

@MainActor
final class HostTests: BridgeTestCase {
    func testTheSelectsAndFoldersComeFromCore() throws {
        var opened: [URL] = []
        var platform = RustSettingsHost.Platform()
        platform.open = { opened.append($0) }
        var performed: [SettingsAction] = []
        platform.perform = { performed.append($0) }
        let host = try makeKit(platform: platform).host

        XCTAssertEqual(host.options(for: .shells).first?.value, .string(""), "the login shell is the empty value")
        XCTAssertEqual(host.options(for: .iconThemes).last?.value, .string("None"))
        host.perform(.openConfig)
        XCTAssertEqual(opened.map(\.lastPathComponent), ["config.json"])
        host.perform(.makeDefaultTerminal)
        XCTAssertEqual(performed, [.makeDefaultTerminal], "what only the app can do stays its closure")
    }

    func testResetAllPutsEveryChangedKeyBack() throws {
        let kit = try makeKit()
        kit.backend.set("opacity", to: .number(0.5))
        kit.backend.set("diff_wrap", to: .bool(true))
        kit.host.perform(.resetAll)
        XCTAssertEqual(kit.backend.values["opacity"], kit.schema.defaults["opacity"], "the model sees it at once")
        XCTAssertEqual(kit.backend.values["diff_wrap"], kit.schema.defaults["diff_wrap"])
    }

    func testHooksAreInstalledOnlyUnderTheAgentHome() throws {
        let host = try makeKit().host
        host.perform(.install, agent: "claude")
        let agentHome = URL(fileURLWithPath: try XCTUnwrap(ProcessInfo.processInfo.environment["WORMHOLE_AGENT_HOME"]))
        let settings = agentHome.appendingPathComponent(".claude/settings.json")
        eventually("the hooks file") { FileManager.default.fileExists(atPath: settings.path) }
        XCTAssertNil(host.failure)

        host.perform(.install, agent: "no-such-agent")
        eventually("the refusal") { host.failure != nil }

        unsetenv("WORMHOLE_AGENT_HOME")
        XCTAssertThrowsError(try settingsSetAgentHooks(agent: "claude", install: true), "never the real home")
    }

    func testTerminalsToImportFromAreListed() throws {
        let home = folder.appendingPathComponent("import-home/.config/ghostty")
        try FileManager.default.createDirectory(at: home, withIntermediateDirectories: true)
        try Data("font-size = 15\nbackground-opacity = 0.9\n".utf8).write(to: home.appendingPathComponent("config"))
        let host = try makeKit().host
        eventually("the scan") { host.panelRecords().imports.contains { $0.id == "ghostty" } }
        let ghostty = try XCTUnwrap(host.panelRecords().imports.first)
        XCTAssertEqual(ghostty.app, "Ghostty")
        XCTAssertTrue(ghostty.summary.hasSuffix("settings"), ghostty.summary)
    }

    func testProfilesAreMadeSwitchedAndDeleted() throws {
        let host = try makeKit().host
        XCTAssertEqual(host.panelRecords().profile.name, "Default")
        let id = try host.createProfile(name: "Work")
        try host.switchProfile(to: id)
        XCTAssertEqual(host.panelRecords().profile.name, "Work")
        try host.switchProfile(to: "default")
        try host.deleteProfile(id)
        XCTAssertEqual(try host.profiles().count, 1)
        XCTAssertThrowsError(try host.deleteProfile("default"))
    }
}

@MainActor
final class HostListsTests: BridgeTestCase {
    func testAProfileIsMadeFromTheHostAndDeletedWithCoresRefusals() throws {
        let host = try makeKit().host
        XCTAssertNil(host.makeProfile(name: "Work", copyOf: nil))
        let work = try XCTUnwrap(host.panelRecords().profiles.first { $0.name == "Work" })
        XCTAssertTrue(work.deleteQuestion.contains("Work"), work.deleteQuestion)
        XCTAssertTrue(work.copyLabel.contains("Work"), work.copyLabel)
        XCTAssertNotNil(host.makeProfile(name: "  ", copyOf: nil), "a blank name is core's refusal, said in its words")
        XCTAssertNotNil(host.removeProfile("default"), "Default cannot go")
        XCTAssertNil(host.removeProfile(work.id))
        XCTAssertFalse(host.panelRecords().profiles.contains { $0.name == "Work" })
    }

    func testAnApprovalsRevokeGoesToTheApp() throws {
        var revoked: [(UInt64, Bool)] = []
        var platform = RustSettingsHost.Platform()
        platform.approvals = {
            [AccessApprovalRecord(id: 7, stored: true, label: "Claude Code", how: .always, agent: "claude", detail: "For good", location: "~/bin", allowed: ["Read"], capabilities: [.read])]
        }
        platform.revokeApproval = { revoked.append(($0, $1)) }
        let host = try makeKit(platform: platform).host
        host.windowOpened()
        let approval = try XCTUnwrap(host.panelRecords().approvals.first)
        XCTAssertEqual(approval.detail, "For good")
        host.revoke(approval)
        XCTAssertEqual(revoked.map(\.0), [7])
        XCTAssertEqual(revoked.map(\.1), [true])
    }

    func testAProjectsSettingsAreLayeredAndMarkedAndForgottenAgain() throws {
        let kit = try makeKit()
        let project = ProjectSettingsRecord(
            name: "app", dir: ".wormhole", root: "/p/app", note: "Set by app/.wormhole/config.json", json: #"{"font_size": 20}"#)
        kit.host.follow(project: project)
        XCTAssertEqual(kit.backend.values["font_size"], .number(20), "the project's value shows over the person's")
        XCTAssertEqual(kit.host.panelRecords().project?.entries.contains("font-terminal"), true)
        kit.host.follow(project: nil)
        XCTAssertEqual(kit.backend.values["font_size"], kit.schema.defaults["font_size"])
        XCTAssertNil(kit.host.panelRecords().project)
    }
}

import Foundation
import SettingsBridge
import SettingsModel
import WormholeBindings
import XCTest

/// The pages that were placeholders, over the real core and throwaway folders: a flag turned on makes its page appear in the
/// nav's list, a skill is written only under the agent home, and the Probes desk and the project service answer.
@MainActor
final class ExtensionsHostTests: BridgeTestCase {
    private final class NeverAsked: OwnerAuthenticator, @unchecked Sendable {
        func authenticate(reason: String) -> OwnerAnswer { .cancelled }
    }

    func testAFlagShowsItsPageAndTheDefaultLeavesNoEntry() throws {
        let host = try makeKit().host
        // A release build reads no switches until ⌃⌥⌘F unlocks them, and then shows their page.
        XCTAssertEqual(host.shownSections()?.contains("featureFlags"), false)
        XCTAssertTrue(host.toggleFlagsUnlocked())
        XCTAssertEqual(host.shownSections()?.contains("featureFlags"), true)
        let row = try XCTUnwrap(host.flags().first { $0.key == "probes" })
        host.setFlag("probes", on: !row.defaultOn)
        XCTAssertEqual(host.flags().first { $0.key == "probes" }?.on, !row.defaultOn)
        XCTAssertEqual(host.shownSections()?.contains("extensions"), !row.defaultOn)
        host.setFlag("probes", on: row.defaultOn)
        XCTAssertEqual(host.shownSections()?.contains("extensions"), row.defaultOn)
    }

    func testASkillIsWrittenOnlyUnderTheAgentHome() throws {
        let host = try makeKit().host
        eventually("the skill rows") { !host.panelRecords().mcpCommand.isEmpty }
        host.perform(skill: "claude", install: true)
        let agentHome = URL(fileURLWithPath: try XCTUnwrap(ProcessInfo.processInfo.environment["WORMHOLE_AGENT_HOME"]))
        let skill = agentHome.appendingPathComponent(".claude/skills/wormhole")
        eventually("the skill folder") { FileManager.default.fileExists(atPath: skill.path) }

        unsetenv("WORMHOLE_AGENT_HOME")
        XCTAssertThrowsError(try settingsSetSkill(word: "claude", install: true), "never the real home")
    }

    func testTheProbesDeskAndTheProjectsServiceAnswer() async throws {
        var platform = RustSettingsHost.Platform()
        platform.owner = { NeverAsked() }
        platform.projects = { try? ProjectService(authenticator: NeverAsked()) }
        let host = try makeKit(platform: platform).host
        let probes = try XCTUnwrap(host.probes)
        let list = await probes.list()
        XCTAssertEqual(list.rows, [], "no Probe in a fresh data folder")
        XCTAssertTrue(list.folderPath.hasSuffix("/probes"))
        let projects = try XCTUnwrap(host.projects)
        let defaults = await projects.defaults()
        XCTAssertEqual(defaults.map(\.key).sorted(), ["actions", "layout", "notes", "prompts", "settings", "theme"])
        let known = await projects.known()
        XCTAssertEqual(known, [])
    }
}

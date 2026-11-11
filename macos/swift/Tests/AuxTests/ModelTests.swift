import XCTest
@testable import AuxModel

@MainActor
final class ModelTests: XCTestCase {
    // MARK: first run (ported from core `onboarding` tests)

    func testThePlacesAreTheFindersAndTheRecentFolders() {
        let home = URL(fileURLWithPath: "/Users/me")
        let recents = [URL(fileURLWithPath: "/Users/me/Projects/terminal"), URL(fileURLWithPath: "/Users/me/gone")]
        let exists = { (url: URL) in url.path != "/Users/me/gone" && url.path != "/Users/me/Developer" }
        let listed = Place.list(home: home, recents: recents, exists: exists)
        XCTAssertEqual(listed.map(\.name), ["terminal", "Home", "Desktop", "Documents"])
        XCTAssertEqual(listed.map(\.detail), ["~/Projects", "~", "~/Desktop", "~/Documents"])
        let all = Place.list(home: home, recents: [], exists: { _ in true })
        XCTAssertEqual(all.map(\.kind), [.home, .desktop, .documents, .developer])
        XCTAssertEqual(Place.tilde(URL(fileURLWithPath: "/opt/work"), home: home), "/opt/work")
    }

    func testContinueAndBackWalkTheWelcomePageAndTheThreeSteps() {
        var flow = OnboardingFlow(agentsFound: true)
        XCTAssertEqual(flow.step.number, 0)
        XCTAssertEqual(flow.apply(.back), .none)
        XCTAssertEqual(flow.step, .welcome, "Back stops at the welcome page")
        _ = flow.apply(.continue); XCTAssertEqual(flow.step, .folder)
        _ = flow.apply(.continue); XCTAssertEqual(flow.step, .look)
        _ = flow.apply(.continue); XCTAssertEqual(flow.step, .agents)
        XCTAssertEqual(flow.step.number, OnboardingStep.counted)
        _ = flow.apply(.back); XCTAssertEqual(flow.step, .look)
    }

    func testAReplayIsTheWelcomePageAlone() {
        var replay = OnboardingFlow.replay()
        XCTAssertTrue(replay.isLast)
        XCTAssertEqual(replay.apply(.continue), .finish(folder: nil))
        XCTAssertEqual(replay.apply(.skip), .skip)
        XCTAssertEqual(replay.step, .welcome, "it never goes on to the setup")
        var first = OnboardingFlow(agentsFound: true)
        XCTAssertEqual(first.apply(.start), .none, "Start before the last step moves on")
        XCTAssertEqual(first.step, .folder)
    }

    func testWithoutAgentsTheThirdStepIsThePrimer() {
        var flow = OnboardingFlow(agentsFound: false)
        for _ in 0..<3 { _ = flow.apply(.continue) }
        XCTAssertEqual(flow.step, .primer)
        flow.setAgentsFound(true)
        XCTAssertEqual(flow.step, .agents, "the agents turned up after all")
    }

    func testStartFinishesWithTheFolderAndSkipWithNothing() {
        var flow = OnboardingFlow(agentsFound: true)
        let folder = URL(fileURLWithPath: "/work/app")
        _ = flow.apply(.pickFolder(folder))
        for _ in 0..<3 { _ = flow.apply(.continue) }
        XCTAssertEqual(flow.apply(.start), .finish(folder: folder))
        var fresh = OnboardingFlow(agentsFound: true)
        XCTAssertEqual(fresh.apply(.skip), .skip)
    }

    func testHooksAndTrustComeOnlyFromTheirOwnButtonsOnTheAgentsStep() {
        var flow = OnboardingFlow(agentsFound: true)
        XCTAssertEqual(flow.apply(.install("claude")), .none)
        for _ in 0..<3 { _ = flow.apply(.continue) }
        XCTAssertEqual(flow.apply(.install("codex")), .installHooks("codex"))
        let folder = URL(fileURLWithPath: "/work/app")
        XCTAssertEqual(flow.apply(.trust(folder)), .trust(folder))
        // Walking every path never yields either on its own.
        var walk = OnboardingFlow(agentsFound: true)
        for event in [OnboardingEvent.continue, .continue, .continue, .back, .start, .continue, .start] {
            let action = walk.apply(event)
            if case .installHooks = action { XCTFail("navigation installed hooks") }
            if case .trust = action { XCTFail("navigation trusted a folder") }
        }
    }

    func testTheSessionInstallsOnlyOnItsOwnButtonAndFinishesWithTheFolder() async {
        let backend = SampleOnboardingBackend()
        let session = OnboardingSession(backend: backend, firstRun: true)
        var ended: (URL?, Bool)?
        session.onEnd = { ended = ($0, $1) }
        await session.scan()
        XCTAssertEqual(session.agents?.count, 3)
        for _ in 0..<3 { session.send(.continue) }
        session.send(.install("codex"))
        try? await Task.sleep(for: .milliseconds(100))
        XCTAssertEqual(session.agents?.first { $0.id == "codex" }?.hooks, .installed)
        XCTAssertNil(ended, "installing is not finishing")
        session.send(.trust(URL(fileURLWithPath: "/work")))
        XCTAssertEqual(backend.trusted.count, 1)
        session.send(.start)
        XCTAssertEqual(ended?.1, true)
    }

    // MARK: licenses (ported from core `update::licenses`)

    func testLicensesSearchAndRejectTextsThatDoNotExist() throws {
        let json = #"{"texts": ["MIT text", "Apache text"], "crates": [{"name": "alpha", "version": "1.0.0", "license": "MIT OR Apache-2.0", "repository": null, "texts": [0, 1]}, {"name": "beta-core", "version": "2.3.0", "license": "Zlib", "repository": "https://example.com/beta", "texts": []}]}"#
        let list = try LicenseList.parse(Data(json.utf8))
        XCTAssertEqual(list.texts(of: list.packages[0]), ["MIT text", "Apache text"])
        XCTAssertEqual(list.search(""), [0, 1])
        XCTAssertEqual(list.search("zlib"), [1])
        XCTAssertEqual(list.search("MIT  apache"), [0])
        XCTAssertEqual(list.search("beta 2.3"), [1])
        XCTAssertThrowsError(try LicenseList.parse(Data(#"{"texts": [], "crates": [{"name": "x", "version": "1", "license": null, "repository": null, "texts": [3]}]}"#.utf8)))
        let browser = LicenseBrowser(list: list)
        browser.selected = 1
        browser.query = "beta"
        XCTAssertEqual(browser.selected, 1, "the open license stays while it is listed")
        browser.query = "alpha"
        XCTAssertEqual(browser.selected, 0, "else the first match shows")
        XCTAssertEqual(browser.countLine, "1 of 2 packages")
    }

    func testTheShippedListIsReadable() throws {
        let list = try FileLicenseSource(url: WindowSnapshotTests.licensesURL).load()
        XCTAssertGreaterThan(list.packages.count, 100)
        XCTAssertTrue(list.packages.contains { list.texts(of: $0).first != nil })
        XCTAssertTrue(list.packages.contains { $0.kind == .font })
    }

    func testTheAboutPanelCarriesTheVersionLine() {
        let info = AppInfo(version: "1.6.3", copyright: "Copyright \u{A9} 2026", debugBuild: true)
        XCTAssertEqual(info.versionLine, "Version 1.6.3 (debug build)")
        XCTAssertEqual(info.aboutPanelOptions["ApplicationVersion"] as? String, "1.6.3 (debug build)")
        XCTAssertEqual(info.aboutPanelOptions["Version"] as? String, "")
        XCTAssertEqual(info.aboutPanelOptions["Copyright"] as? String, "Copyright \u{A9} 2026")
    }

    // MARK: updates, hosts, trust

    func testAnUpdateWalksFromAvailableToInstalledAndRestarts() async {
        let backend = SampleUpdateBackend()
        let model = UpdateViewModel(backend: backend)
        model.start()
        guard case .available(let release) = model.phase else { return XCTFail("\(model.phase)") }
        model.install(release)
        try? await Task.sleep(for: .milliseconds(1500))
        XCTAssertEqual(model.phase, .installed(version: "1.7.0"))
        XCTAssertNil(model.restartQuestion, "nothing running: it restarts at once")
        backend.running = [RunningTerminal(name: "api", status: .busy)]
        XCTAssertNotNil(model.restartQuestion)
        model.channel = .preview
        XCTAssertEqual(backend.channel, .preview)
        model.stop()
    }

    func testTheCheckedLineSaysHowLongAgo() {
        let model = UpdateViewModel(backend: SampleUpdateBackend(phase: .idle))
        XCTAssertEqual(model.checkedLine(), "Not checked yet.")
        XCTAssertEqual(UpdateViewModel.progressText(received: 0, expected: 0), nil)
        XCTAssertEqual(UpdateViewModel.fraction(received: 5, expected: 20), 0.25)
    }

    func testAHostCardStartsFoldedAndKeepsWhatIsWrongInView() async {
        let model = HostsViewModel(backend: SampleHostsBackend()) { _ in }
        await model.refresh()
        let cards = model.report!.cards
        XCTAssertTrue(model.open.isEmpty, "a card starts folded")
        model.toggle(cards[0].id)
        XCTAssertEqual(model.open, [cards[0].id])
        model.toggle(cards[0].id)
        XCTAssertTrue(model.open.isEmpty)
        // The folded card shows its program and every fact that is a warning or a danger, and nothing else.
        let card = HostCard(
            id: "h-1", standing: .attached, note: "Window 1",
            facts: [HostFact(.program, "zsh"), HostFact(.version, "1.2 (this app is 1.3)", tone: .warning), HostFact(.socket, "/tmp/s", tone: .muted)])
        XCTAssertEqual(card.program, "zsh")
        XCTAssertEqual(card.problems.map(\.field), [.version])
    }

    func testHostsCopyEndAndForget() async {
        let backend = SampleHostsBackend()
        var pasted: [String] = []
        let model = HostsViewModel(backend: backend) { pasted.append($0) }
        await model.refresh()
        XCTAssertEqual(model.report?.cards.count, 4)
        model.copy(.host("h-7f3a"))
        XCTAssertEqual(model.copied, .host("h-7f3a"))
        XCTAssertTrue(pasted[0].hasPrefix("h-7f3a (Attached)"))
        model.copy(.all)
        XCTAssertTrue(pasted[1].contains("Hosts folder:"))
        XCTAssertEqual(model.endQuestion("h-7f3a")?.title, "End session host h-7f3a?")
        XCTAssertEqual(model.endQuestion("h-7f3a")?.detail, "The program in the terminal \u{201C}api\u{201D} gets a hangup and the terminal ends.")
        XCTAssertTrue(model.report!.cards.first { $0.id == "h-e5d0" }!.canForget)
        XCTAssertFalse(model.report!.cards.first { $0.id == "h-e5d0" }!.canEnd)
        model.end("h-21c9"); model.forget("h-e5d0")
        try? await Task.sleep(for: .milliseconds(100))
        XCTAssertEqual(backend.ended, ["h-21c9"])
        XCTAssertEqual(backend.forgotten, ["h-e5d0"])
        XCTAssertEqual(model.report?.cards.count, 2)
    }

    func testTrustApprovesOnlyWhatIsNotTrustedAndReloads() async {
        let backend = SampleTrustBackend()
        let model = TrustViewModel(backend: backend)
        XCTAssertTrue(model.review.canApprove)
        XCTAssertEqual(model.approveLabel, "Trust with Touch ID")
        model.approve()
        try? await Task.sleep(for: .milliseconds(100))
        XCTAssertEqual(backend.approvals, 1)
        XCTAssertEqual(model.review.status, .trusted)
        XCTAssertFalse(model.review.canApprove)
        let gone = TrustReview(name: "x", directory: ".wormhole", status: .untrusted, lines: [], problem: "This project is no longer open in a terminal.")
        XCTAssertFalse(gone.canApprove)
    }
}

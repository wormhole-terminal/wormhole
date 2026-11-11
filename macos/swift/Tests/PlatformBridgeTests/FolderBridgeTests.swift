import Foundation
import PlatformModel
import Testing

@testable import PlatformBridge

/// Core's folder providers behind the picker's seam, over a throwaway tree: never the person's folders.
@Suite(.serialized) struct FolderBridgeTests {
    private let root: URL
    private let folders: CoreFolders

    init() throws {
        let made = FileManager.default.temporaryDirectory.appending(path: "Wormhole-swift-folders-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: made, withIntermediateDirectories: true)
        // The system reports the real path of /var.
        root = made.resolvingSymlinksInPath()
        for dir in ["Café/under", "Projects/terminal/wormhole", "zeta/Café au lait"] {
            try FileManager.default.createDirectory(at: root.appending(path: dir), withIntermediateDirectories: true)
        }
        try FileManager.default.createSymbolicLink(at: root.appending(path: "Projects/terminal/back"), withDestinationURL: root)
        folders = PlatformBridge.makeFolders()
    }

    private func query(_ text: String, cwd: URL? = nil) -> PickerQuery {
        PickerQuery(text: text, context: PickerContext(cwd: (cwd ?? root).path, home: root.path))
    }

    @Test func theFoldersBelowTheTerminalsAreListedInOrder() async {
        let answer = await folders.providers[0].answer(query(""))
        #expect(answer.rows.map(\.label) == ["Café", "Projects", "zeta"])
        #expect(answer.rows[0].insert == "Café/" && answer.rows[0].descend == "Café/")
        #expect(answer.rows[0].icon == "folder" && answer.rows[0].kind == "folder")
        #expect(answer.crumbs.first?.label == "~", "under home it starts at the tilde")
    }

    @Test func marksAreCharactersOfTheLabelEvenAfterAMultiByteLetter() async {
        let answer = await folders.providers[0].answer(query("caf"))
        #expect(answer.rows.first?.label == "Café")
        #expect(answer.rows.first?.marks == [0..<3])
        // The match is after the "é", which is two bytes: byte 9 is character 8.
        let later = await folders.providers[0].answer(query("zeta/lait"))
        #expect(later.rows.first?.label == "Café au lait")
        #expect(later.rows.first?.marks == [8..<12])
    }

    @Test func aNameIsFoundAcrossLevelsAndALinkBackToTheRootDoesNotRunAway() async {
        let fuzzy = await folders.providers[0].answer(query("pr/te"))
        #expect(fuzzy.rows.map(\.label) == ["Projects/terminal"])
        let looped = await folders.providers[0].answer(query("Projects/terminal/back/Projects/terminal/back/"))
        #expect(looped.rows.count < 100)
    }

    @Test func aVeryDeepFolderAnswersWithEveryLevelInItsCrumbs() async throws {
        var deep = root.appending(path: "deep")
        for n in 0..<100 { deep = deep.appending(path: "d\(n)") }
        try FileManager.default.createDirectory(at: deep, withIntermediateDirectories: true)
        let answer = await folders.providers[0].answer(query("", cwd: deep))
        #expect(answer.rows.isEmpty)
        #expect(answer.crumbs.count > 100)
    }

    @Test func chevronsAndBranchesDecorateTheRowsInView() async throws {
        let git = root.appending(path: "repo")
        try FileManager.default.createDirectory(at: git.appending(path: ".git"), withIntermediateDirectories: true)
        try "ref: refs/heads/feature/x\n".write(to: git.appending(path: ".git/HEAD"), atomically: true, encoding: .utf8)
        #expect(folders.decorators.count == 2)
        #expect(folders.decorators.allSatisfy { $0.kinds == ["folder"] })
        let chevrons = await folders.decorators[0].decorate([root.appending(path: "zeta").path, root.appending(path: "Café/under").path])
        #expect(chevrons.first { $0.key == root.appending(path: "zeta").path }?.chevron == true)
        let branches = await folders.decorators[1].decorate([git.path])
        #expect(branches.first?.badge == "feature/x")
    }

    @Test func recentFoldersAreRankedAndForgettableAndSurviveAHostRestart() async throws {
        // The temp folder is never a recent place, so a saved list stands in for visits.
        #expect(!folders.recordVisit(root.path))
        let now = Int(Date().timeIntervalSince1970)
        let saved = """
            {"entries":[{"path":"\(root.path)/zeta","score":1.0,"last":\(now)},{"path":"\(root.path)/Projects","score":3.0,"last":\(now)}]}
            """
        let again = PlatformBridge.makeFolders(recentsJSON: saved, recentTitle: "Recent")
        let listed = await again.providers[1].answer(query(""))
        #expect(listed.sections.last == "Recent")
        #expect(listed.rows.map(\.label) == ["Projects", "zeta"])
        #expect(listed.rows.allSatisfy { $0.forgettable })
        #expect(again.forget("\(root.path)/Projects"))
        #expect(!again.recentsJSON().contains("Projects"))
    }

    @Test func cancellingATaskDoesNotHangAndTheAnswerIsDropped() async {
        let provider = folders.providers[0]
        let query = query("")
        let tasks = (0..<30).map { _ in Task { await provider.answer(query) } }
        for task in tasks { task.cancel() }
        for task in tasks { _ = await task.value }
        // The next question is still answered: cancelled jobs gave their places back.
        let answer = await provider.answer(query)
        #expect(answer.rows.count == 3)
    }
}

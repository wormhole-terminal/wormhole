import PlatformModel
import Testing

/// The context menus against the Rust app's own outlines (`row_menu.rs`, `path_menu.rs` tests).
@Suite struct ContextMenuTests {
    private func lines(_ nodes: [MenuNode]) -> String { MenuDump.lines(nodes, depth: 0) }

    @Test func aLoneTerminalOffersNoGroupAndNoNeighbour() {
        var facts = RowMenuFacts(id: 7)
        facts.cwd = "/tmp"
        #expect(lines(RowMenu.nodes(facts)) == """
            Rename…
            Add terminal to new group
            Add terminal to group ▸  (disabled)
            Split right
            Close terminal
            ---
            Show beside… ▸  (disabled)
            Pin to top
            Don't keep this terminal's text
            ---
            Reveal in Finder
            Copy path

            """)
    }

    @Test func groupsNeighboursAndStateShowUp() {
        var facts = RowMenuFacts(id: 7)
        facts.pinned = true
        facts.forgetText = true
        facts.inSplit = true
        facts.cwd = nil
        facts.groups = [.init(id: 1, name: "Work", current: true), .init(id: 2, name: "Play"), .init(id: 3, name: "Closed", allowed: false)]
        facts.beside = [.init(id: 9, name: "api")]
        #expect(lines(RowMenu.nodes(facts)) == """
            Rename…
            Add terminal to new group
            Add terminal to group ▸
              ✓ Work
              Play
              Closed  (disabled)
            Remove from group
            Split right
            Close terminal
            ---
            Show beside… ▸
              api
            Remove from split
            Unpin
            ✓ Don't keep this terminal's text
            ---
            Reveal in Finder  (disabled)
            Copy path  (disabled)

            """)
    }

    @Test func theTextChoiceIsOffWhileNoTextIsKept() {
        var facts = RowMenuFacts(id: 1)
        facts.restoring = false
        #expect(lines(RowMenu.nodes(facts)).contains("Don't keep this terminal's text  (disabled)"))
    }

    @Test func hideFromAgentsIsOfferedWhenAskedAndTickedWhileHidden() {
        func line(_ hide: Bool?) -> String? {
            var facts = RowMenuFacts(id: 1)
            facts.agentHide = hide
            return lines(RowMenu.nodes(facts)).split(separator: "\n").first { $0.contains("Hide from Agents") }.map(String.init)
        }
        #expect(line(nil) == nil && line(false) == "Hide from Agents" && line(true) == "✓ Hide from Agents")
    }

    @Test func aChoiceAnswersWithItsKindAndTheTerminal() throws {
        var facts = RowMenuFacts(id: 7)
        facts.groups = [.init(id: 4, name: "Work")]
        let nodes = RowMenu.nodes(facts)
        guard case .submenu(_, let inner, _, _) = nodes[2], case .item(let pick) = inner[0] else { Issue.record("no group submenu"); return }
        #expect(pick.command == .context(ContextAction("row.intoGroup", subject: 7, argument: "4")))
        #expect(Set(RowMenu.Kind.allCases.map(\.key)).count == RowMenu.Kind.allCases.count)
    }

    @Test func aFolderUnderHomeListsItsParentsDownToTheVolume() {
        #expect(lines(PathMenu.nodes(folder: "/Users/ben/Projects/tube", shown: "~/Projects/tube", volume: "Macintosh HD", exited: false)) == """
            # ~/Projects/tube
            ---
            Open in Finder
            Copy path
            Insert into terminal
            ---
            tube
            Projects
            ben
            Users
            Macintosh HD

            """)
        #expect(lines(PathMenu.nodes(folder: "/tmp", shown: "/tmp", volume: "Macintosh HD", exited: true)).contains("Insert into terminal  (disabled)"))
    }

    @Test func pathChains() {
        func names(_ path: String) -> [String] { PathChain.of(path, volume: "Macintosh HD").map(\.name) }
        #expect(names("/Users/ben/Projects/tube") == ["tube", "Projects", "ben", "Users", "Macintosh HD"])
        #expect(PathChain.of("/Users/ben/Projects", volume: "V").map(\.path) == ["/Users/ben/Projects", "/Users/ben", "/Users", "/"])
        #expect(names("/tmp") == ["tmp", "Macintosh HD"] && names("/") == ["Macintosh HD"])
        #expect(names("/Volumes/Work/clients/acme") == ["acme", "clients", "Work"] && names("/Volumes/Work") == ["Work"])
        #expect(names("/Volumes") == ["Volumes", "Macintosh HD"])
        #expect(names("a/b") == ["b", "a"] && names("").isEmpty)
    }

    @Test func aGroupMenuHasDotsFolderAndActions() {
        var facts = GroupMenuFacts(group: 3)
        facts.colours = [.init(name: "Red", token: "folder.red"), .init(name: "Blue", token: "folder.blue")]
        facts.chosen = 1
        facts.folder = "~/Projects"
        facts.onlyInFolder = true
        let text = lines(GroupMenu.nodes(facts))
        #expect(text == """
            Rename…
            ---
            Swatches Red, Blue
            Other colour…
            No colour
            ---
            ~/Projects  (disabled)
            Set folder…
            Clear folder
            ✓ Only terminals in this folder
            ---
            New terminal in group
            Ungroup
            Close group
            Pin group
            Move group to new window

            """)
        facts.chosen = nil
        #expect(lines(GroupMenu.nodes(facts)).contains("✓ No colour"))
        guard case .swatches(let row) = GroupMenu.nodes(facts)[2] else { Issue.record("no swatches"); return }
        #expect(row.command(1) == ContextAction("group.colour", subject: 3, argument: "1"))
    }

    @Test func foldOutputIsOfferedOnlyOverABlockThatCanBeFolded() {
        var facts = TerminalMenuFacts()
        facts.block = .init(id: 4, hasCommand: true, hasOutput: true, atPrompt: true)
        #expect(!lines(TerminalMenu.nodes(facts)).contains("Fold Output"))
        facts.block?.canFold = true
        let shown = lines(TerminalMenu.nodes(facts))
        #expect(shown.contains("Fold Output"))
        #expect(shown.firstRange(of: "Put Command at Prompt")!.lowerBound < shown.firstRange(of: "Fold Output")!.lowerBound)
    }

    @Test func theTerminalMenuShowsTheKeysOfTheMenuBar() {
        var facts = TerminalMenuFacts()
        facts.canCopy = true
        facts.link = "https://example.com"
        facts.block = .init(id: 4, hasCommand: true, hasOutput: false, atPrompt: false)
        #expect(lines(TerminalMenu.nodes(facts)) == """
            Open Link
            Copy Link
            ---
            Copy  ⌘C
            Paste  ⌘V  (disabled)
            Select All  ⌘A
            ---
            Find…  ⌘F
            ---
            Copy Command
            Copy Output  (disabled)
            Put Command at Prompt  (disabled)
            ---
            Split Right  ⌘D
            Split Down  ⇧⌘D
            ---
            Clear Buffer  ⌘K

            """)
    }

    @Test func panelMenus() {
        #expect(lines(PanelMenus.file(paths: ["a.rs"], onDisk: true, opensInFinder: false, historyOn: true)) == """
            Open
            Reveal in Finder
            ---
            Copy Path
            Copy Relative Path
            ---
            File History

            """)
        #expect(lines(PanelMenus.file(paths: ["a.rs", "b.rs"], onDisk: true, opensInFinder: false, historyOn: false)) == "Copy Paths\nCopy Relative Paths\n")
        #expect(lines(PanelMenus.commit(sha: "abc", toggle: true, parents: [("p1", "p1")])).split(separator: "\n").first == "Show less")
        #expect(lines(PanelMenus.commit(sha: "abc", parents: [("p1", "p1")])).hasSuffix("---\nOpen Parent p1\n"))
    }
}

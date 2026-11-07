@testable import StudioModel
import XCTest

final class FileTests: XCTestCase {
    func testCommentsAndTrailingCommasAreRead() throws {
        let file = try ThemeFile(parsing: "// mine\n{ \"name\": \"A // not a comment\", /* x */ \"ui\": { \"primary\": \"#fff\", }, }")
        XCTAssertEqual(file.name, "A // not a comment")
        XCTAssertEqual(file.own("primary"), "#fff")
    }

    func testAnEditChangesOnlyItsKeyAndKeepsTheRest() throws {
        var file = try ThemeFile(parsing: "{ \"$schema\": \"x\", \"name\": \"A\", \"future\": [1, 2], \"ui\": { \"primary\": \"#fff\" } }")
        file.set("git.branch", "primary / 20%")
        file.set("primary", nil)
        XCTAssertEqual(file.json(), """
            {
              "$schema": "x",
              "name": "A",
              "ui": {
                "git.branch": "primary / 20%"
              },
              "future": [
                1,
                2
              ]
            }

            """)
    }

    func testEveryKindOfTokenLivesWhereCoreLooksForIt() {
        var file = ThemeFile(name: "A", extends: "Nord")
        file.set("terminal.background", "#101010")
        file.set("terminal.ansi.4", "#7aa2f7")
        file.set("syntax.keyword", "#bb9af7")
        file.set("metrics.radius", "6px")
        file.set("surface.panel", "terminal.background")
        XCTAssertEqual(
            file.json(),
            """
            {
              "name": "A",
              "extends": "Nord",
              "terminal": {
                "ansi": {
                  "4": "#7aa2f7"
                },
                "background": "#101010"
              },
              "ui": {
                "surface.panel": "terminal.background"
              },
              "syntax": {
                "keyword": "#bb9af7"
              },
              "metrics": {
                "radius": 6
              }
            }

            """)
        XCTAssertEqual(file.own("metrics.radius"), "6")
        XCTAssertEqual(file.own("terminal.ansi.4"), "#7aa2f7")
    }

    func testASyntaxStyleKeepsItsBoldWhenTheColourChanges() throws {
        var file = try ThemeFile(parsing: "{ \"syntax\": { \"keyword\": { \"color\": \"#111111\", \"bold\": true } } }")
        XCTAssertEqual(file.own("syntax.keyword"), "#111111")
        file.set("syntax.keyword", "#222222")
        XCTAssertTrue(file.json().contains("\"bold\": true") && file.json().contains("#222222"))
        file.set("syntax.keyword", nil)
        XCTAssertTrue(file.json().contains("\"bold\": true") && !file.json().contains("color"))
    }

    func testAPairKeepsAppearanceValuesInTheirVersionAndSharedOnesAtTheTop() throws {
        var file = try ThemeFile(
            parsing: "{ \"name\": \"S\", \"extends\": \"Nord\", \"variants\": [{ \"name\": \"Light\" }, { \"name\": \"Dark\" }] }")
        XCTAssertTrue(file.isPair)
        XCTAssertEqual(file.themeName(version: "Dark"), "S Dark")
        file.set("surface.panel", "#222222", version: "Dark", group: "Surfaces")
        file.set("primary", "#ff6600", version: "Dark", group: "Primary")
        XCTAssertEqual(file.own("surface.panel", at: .version, version: "Dark"), "#222222")
        XCTAssertNil(file.own("surface.panel", in: "Light"), "the other version does not see it")
        XCTAssertEqual(file.own("primary", at: .top, version: "Dark"), "#ff6600", "the primary is shared")
        file.move("surface.panel", to: .top, version: "Dark")
        XCTAssertEqual(file.own("surface.panel", in: "Light"), "#222222")
    }

    func testAFileThatIsNotAnObjectIsRefused() {
        XCTAssertThrowsError(try ThemeFile(parsing: "[1]"))
        XCTAssertThrowsError(try ThemeFile(parsing: "{ \"ui\": "))
    }
}

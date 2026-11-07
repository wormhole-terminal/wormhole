import Testing
import WordsModel

@testable import WordsBridge

/// `wormhole-text`'s catalogs as the interface's words: one table per language, read by id.
@Suite(.serialized) struct WordsBridgeTests {
    @Test func englishHoldsEveryPlainMessageByIdAndIndex() throws {
        let table = try #require(WordTable(tag: "en"))
        #expect(table.ids.count == table.texts.count && table.ids.count > 1500)
        #expect(table.text(for: "folder-picker-empty") == "No folders")
        let at = try #require(table.ids.firstIndex(of: "folder-picker-empty"))
        #expect(table[at] == "No folders")
        #expect(table.text(for: "folder-picker-cut-folders") == nil, "a message with variables is Rust's to format")
        #expect(table.text(for: "no-such-message") == nil)
    }

    @Test func thePseudoLanguageTransformsEveryWord() throws {
        let pseudo = try #require(WordTable(tag: "en-XA"))
        #expect(pseudo.text(for: "folder-picker-empty")?.contains("⟦") == true)
        #expect(WordTable(tag: "xx") == nil)
        #expect(try #require(WordTable(tag: "en")).hash == pseudo.hash, "one id list for every language")
    }

    @Test func aLanguageIsNegotiatedFromThePreferredOnesAndEnglishIsTheFallback() {
        #expect(WordTable.preferring(["de-DE", "en-US"]).tag == "en")
        #expect(WordTable.preferring([]).tag == "en")
    }

    @Test func installedWordsAnswerTheSeamAndTheFallbackShowsOtherwise() {
        defer { Words.install(nil) }
        #expect(Words.text("folder-picker-empty", fallback: "none") == "none", "nothing installed")
        WordsBridge.install(tag: "en")
        #expect(Words.text("folder-picker-empty", fallback: "none") == "No folders")
        #expect(Words.text("no-such-message", fallback: "English") == "English")
    }
}

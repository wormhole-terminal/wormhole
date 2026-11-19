import Foundation
import Testing

@testable import Wormhole

/// A language by name is devctl's to ask for (`app/language.rs:7-8`: a release build cannot show the pseudo-language;
/// `devctl/language.rs:17`). The Swift app's devctl is its control folder, so `WORMHOLE_LANGUAGE` counts only beside it.
@MainActor
struct LaunchLanguageTests {
    private let asked = ["WORMHOLE_LANGUAGE": "en-XA"]

    @Test func withoutAControlFolderTheVariableIsNotRead() {
        #expect(PlatformSetup.launchLanguage(Options(), environment: asked) == nil)
    }

    @Test func besideAControlFolderItNamesTheLanguage() {
        var options = Options()
        options.devctl = URL(fileURLWithPath: "/tmp/Wormhole-swift-language-test/ctl")
        #expect(PlatformSetup.launchLanguage(options, environment: asked) == "en-XA")
        // Not set, or empty: the system's list decides, as without it.
        #expect(PlatformSetup.launchLanguage(options, environment: [:]) == nil)
        #expect(PlatformSetup.launchLanguage(options, environment: ["WORMHOLE_LANGUAGE": ""]) == nil)
    }
}

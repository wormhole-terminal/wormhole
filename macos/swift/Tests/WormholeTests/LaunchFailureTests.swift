import Foundation
import Testing

@testable import Wormhole

/// A launch that cannot go on is shown on screen only to an app launched as an app: a background instance, a test instance and a bare
/// executable never put a modal question up (it would hang the suite that started them), and a launch that has no options yet has none to show.
struct LaunchFailureTests {
    private static func options(_ arguments: [String] = [], _ environment: [String: String] = [:]) throws -> Options {
        try Options.parse(arguments, environment: environment)
    }

    @Test func anAppLaunchedAsAnAppShowsIt() throws {
        #expect(LaunchFailure.showsAlert(options: try Self.options(), isAppBundle: true))
    }

    @Test func aBackgroundInstanceNeverShowsIt() throws {
        #expect(!LaunchFailure.showsAlert(options: try Self.options(["--background"]), isAppBundle: true))
        #expect(!LaunchFailure.showsAlert(options: try Self.options([], ["WORMHOLE_BACKGROUND": "1"]), isAppBundle: true))
    }

    @Test func aTestInstanceNeverShowsIt() throws {
        // A control folder marks a test instance whether or not `--background` came with it.
        #expect(!LaunchFailure.showsAlert(options: try Self.options([], ["WORMHOLE_DEVCTL": "/tmp/Wormhole-swift-x/ctl"]), isAppBundle: true))
        #expect(!LaunchFailure.showsAlert(options: try Self.options(["--background"], ["WORMHOLE_DEVCTL": "/tmp/Wormhole-swift-x/ctl"]), isAppBundle: true))
    }

    @Test func aBareExecutableNeverShowsIt() throws {
        #expect(!LaunchFailure.showsAlert(options: try Self.options(), isAppBundle: false))
    }

    @Test func aLaunchThatFailedBeforeItsOptionsExistedNeverShowsIt() {
        #expect(!LaunchFailure.showsAlert(options: nil, isAppBundle: true))
    }

    @Test func theTestRunnerIsNotAnAppBundle() {
        // The default reads the running executable: the test process must never be taken for an app that may put a question up.
        #expect(Bundle.main.bundleURL.pathExtension != "app")
        #expect(!LaunchFailure.showsAlert(options: (try? Self.options()) ?? Options()))
    }
}

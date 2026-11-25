import Foundation
import Testing

/// A command line typed outside a tab (`wh sessions list`) has no `WORMHOLE_DATA_DIR`: a terminal's shell never inherits it. The core
/// then resolves its folder from the 1.x app's own, so the app settles the folder before the command line runs. The 1.x folder here is a
/// decoy in a scratch `HOME`, with a host's leftover in it that `sessions list` would show if the command line read it.
extension EndToEnd {
    @Test("a command line without WORMHOLE_DATA_DIR reads and creates nothing in a folder named Wormhole")
    @MainActor func aCommandLineWithoutTheVariableNeverOpensTheRealAppsFolder() async throws {
        let scratch = try Scratch()
        defer { scratch.remove() }
        guard let program = Instance.executable() else { throw ScratchError(description: "the Wormhole app is not built") }
        let support = scratch.home.appendingPathComponent("Library/Application Support")
        let decoy = support.appendingPathComponent("Wormhole/hosts")
        try FileManager.default.createDirectory(at: decoy, withIntermediateDirectories: true)
        try Data("not a meta file\n".utf8).write(to: decoy.appendingPathComponent("abcdefghij12.meta"))

        let ran = try Spawn.run(program.path, ["sessions", "list"], environment: scratch.cleanEnvironment, in: scratch.root)
        #expect(ran.status == 0, "exit \(ran.status):\n\(ran.output)")
        #expect(!ran.output.contains("abcdefghij12"), "the decoy of the 1.x folder is listed:\n\(ran.output)")
        #expect(ran.output == "no sessions\n", "\(ran.output)")

        // It only read: the decoy is as it was, and no folder was made beside it (the dev identity's own is where it looked).
        let made = try FileManager.default.contentsOfDirectory(atPath: support.path).sorted()
        #expect(made == ["Wormhole"], "\(made)")
        let kept = try FileManager.default.contentsOfDirectory(atPath: decoy.path)
        #expect(kept == ["abcdefghij12.meta"], "\(kept)")
    }

    @Test("a command line with a variable that names another folder is refused, and one that names a test folder is answered there")
    @MainActor func aCommandLineIsRefusedForAFolderThatIsNotItsOwn() async throws {
        let scratch = try Scratch()
        defer { scratch.remove() }
        guard let program = Instance.executable() else { throw ScratchError(description: "the Wormhole app is not built") }
        let others = scratch.root.appendingPathComponent("Wormhole")
        let refused = try Spawn.run(
            program.path, ["sessions", "list"], environment: scratch.cleanEnvironment.merging(["WORMHOLE_DATA_DIR": others.path]) { $1 }, in: scratch.root)
        #expect(refused.status == 2, "exit \(refused.status):\n\(refused.output)")
        #expect(refused.output.contains("is not a folder named Wormhole-swift*"), "\(refused.output)")

        let answered = try Spawn.run(
            program.path, ["sessions", "list"], environment: scratch.cleanEnvironment.merging(["WORMHOLE_DATA_DIR": scratch.data.path]) { $1 }, in: scratch.root)
        #expect(answered.status == 0 && answered.output == "no sessions\n", "exit \(answered.status):\n\(answered.output)")
    }
}

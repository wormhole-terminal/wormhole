// text-file: the control folder's replies, not interface text
// What this machine can say about the intro without playing it, for the control folder's `intro-support`: are the shader sources in
// the app's resources, is there a Metal device, does the shader compile on it. A play that simply does not show cannot tell a
// packaging mistake (a source not found) from a Mac with no usable GPU (a virtual machine); this answers the three apart, and a
// bundle check reads it (`macos/scripts/check-bundle.sh`). Read-only: nothing is drawn and no window opens.

import Foundation
import Metal

public enum IntroSupport {
    /// Three lines: `shaders <found>/<total> <folder>` (or `shaders <found>/<total> missing <names>`), `metal <device's name|none>`
    /// and `compile ok|failed: <why>|skipped: <why>`.
    public static func report() -> String { report(locate: IntroShader.url(of:), device: MTLCreateSystemDefaultDevice) }

    static func report(locate: (String) -> URL?, device: () -> MTLDevice?) -> String {
        let located = IntroShader.files.map { (name: $0, url: locate($0)) }
        let missing = located.filter { $0.url == nil }.map(\.name)
        let count = "shaders \(located.count - missing.count)/\(located.count)"
        let shaders = missing.isEmpty
            ? "\(count) \(located[0].url?.deletingLastPathComponent().path ?? "")" : "\(count) missing \(missing.joined(separator: ", "))"
        let gpu = device()
        let compile: String
        if !missing.isEmpty {
            compile = "skipped: the sources are not found"
        } else if let gpu {
            do {
                // The renderer itself, from the sources the intro loads: a pipeline that builds is what playing needs.
                _ = try IntroRenderer(device: gpu)
                compile = "ok"
            } catch {
                compile = "failed: \(describe(error))"
            }
        } else {
            compile = "skipped: there is no Metal device"
        }
        return [shaders, "metal \(gpu?.name ?? "none")", "compile \(compile)"].joined(separator: "\n")
    }

    /// An error on one line (a compiler's diagnostics span many), cut where a log line stays readable.
    static func describe(_ error: Error) -> String {
        let one = String(describing: error).split(whereSeparator: \.isNewline).joined(separator: " ")
        return String(one.prefix(400))
    }
}

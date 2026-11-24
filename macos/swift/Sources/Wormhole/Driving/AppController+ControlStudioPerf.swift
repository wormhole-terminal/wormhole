// text-file: a test driver's replies on the control folder, not interface text
// `studio-perf <interaction> [n]`: what one step of a person's work in the Theme Studio costs the main thread. The
// interactions: `open` (the window made and drawn; the Studio must be closed), `set` (the selected colour changed as a drag
// in the colour picker changes it), `type` (a value typed into the field, a key at a time), `select` (another token),
// `area` (the next area chip), `filter` (the token filter typed a letter at a time), `undo` (undo, then redo),
// `appearance` (a pair's other appearance), `resize` (the window 120 points wider, then back), `scroll` (the gallery
// turned 120 points, down for forty samples and up again) and `idle [seconds]` (nothing: what the open window costs by itself).
//
// A sample is the step itself (`model`: the document, with the core's answers), then the window laid out (`update`:
// SwiftUI evaluates again what the step invalidated), then drawn and committed (`draw`), each wall time on the main thread;
// `total` is their sum, the time the window cannot answer. `cpu` is the main thread's CPU time over the sample and the
// 50 ms after it, so what a step leaves for later turns of the run loop is counted too. One line of medians and maxima
// (after the first sample, which warms caches) goes to `studio-perf.txt`, with the core calls the step made.
//
// `idle` has no step. Its line is the main thread's and the whole process's CPU time for each second the window stood
// there, and how late the main thread came back from a 5 ms wait (`late`): what a click or a key would have waited.
//
// Not in any of it: the render server's and the GPU's work, and the frames of an animation while the window is not seen
// (`occluded`): a background instance's window is behind the others.

import AppKit
import DesignSystem
import StudioModel
import StudioUI

extension AppController {
    func runStudioPerf(_ argument: String) async {
        let words = argument.split(separator: " ").map(String.init)
        let count = max(2, words.dropFirst().first.flatMap { Int($0) } ?? 20)
        guard let interaction = words.first else { return reply("error: studio-perf open|set|type|select|area|filter|undo|appearance|resize|scroll [n]") }
        if interaction == "open" { return await studioPerfOpen() }
        if interaction == "idle" { return await studioPerfIdle(seconds: Double(words.dropFirst().first ?? "") ?? 3) }
        guard let session = studio.session, let window = session.window, let content = window.contentView else {
            return reply("error: the Studio is not open (`studio`)")
        }
        guard let step = studioPerfStep(interaction, session: session, window: window, content: content) else {
            return reply("error: studio-perf cannot \(interaction) here (a colour token selected? a pair open?)")
        }
        var samples: [StudioPerfSample] = []
        let callsAtStart = session.document.coreCalls
        for index in 0..<count {
            samples.append(await studioPerfSample(content) { step(index) })
        }
        studioPerfReport(interaction, samples, calls: session.document.coreCalls - callsAtStart, window: window)
    }

    /// The window left alone: what its own animations cost, and how long the main thread keeps a caller waiting meanwhile.
    private func studioPerfIdle(seconds: Double) async {
        guard let window = studio.session?.window else { return reply("error: the Studio is not open (`studio`)") }
        let wait = 5.0
        let main = Self.mainThreadCPU(), process = Self.processCPU(), start = CFAbsoluteTimeGetCurrent()
        var late: [Double] = []
        while CFAbsoluteTimeGetCurrent() - start < seconds {
            let asked = CFAbsoluteTimeGetCurrent()
            await pause(Int(wait))
            late.append(max(0, (CFAbsoluteTimeGetCurrent() - asked) * 1000 - wait))
        }
        let elapsed = CFAbsoluteTimeGetCurrent() - start
        late.sort()
        let line = String(
            format: "studio-perf idle seconds=%.1f main_cpu=%.0fms/s process_cpu=%.0fms/s late=%.1f/%.1f (ms, median/max) occluded=%@",
            elapsed, (Self.mainThreadCPU() - main) / elapsed, (Self.processCPU() - process) / elapsed, late[late.count / 2], late[late.count - 1],
            "\(!window.occlusionState.contains(.visible))")
        devctl?.append(line, to: "studio-perf.txt")
        reply(line)
    }

    /// The window opened as the menu opens it, laid out and drawn once.
    private func studioPerfOpen() async {
        guard !studio.isOpen else { return reply("error: the Studio is open; close it first (`studio-do close`)") }
        var opened: NSView?
        let sample = await studioPerfSample({ opened }) {
            self.studio.show()
            opened = self.studio.session?.window?.contentView
        }
        guard let window = studio.session?.window else { return reply("error: the Studio did not open") }
        studioPerfReport("open", [sample, sample], calls: studio.session?.document.coreCalls ?? StudioCoreCalls(), window: window)
    }

    private func studioPerfSample(_ content: NSView, _ step: () -> Void) async -> StudioPerfSample {
        await studioPerfSample({ content }, step)
    }

    private func studioPerfSample(_ content: () -> NSView?, _ step: () -> Void) async -> StudioPerfSample {
        let cpu = Self.mainThreadCPU(), start = CFAbsoluteTimeGetCurrent()
        step()
        let stepped = CFAbsoluteTimeGetCurrent()
        content()?.layoutSubtreeIfNeeded()
        let updated = CFAbsoluteTimeGetCurrent()
        content()?.displayIfNeeded()
        CATransaction.flush()
        let drawn = CFAbsoluteTimeGetCurrent()
        await pause(50)
        return StudioPerfSample(
            model: (stepped - start) * 1000, update: (updated - stepped) * 1000, draw: (drawn - updated) * 1000,
            cpu: Self.mainThreadCPU() - cpu)
    }

    /// What sample `index` of an interaction does; nil when the Studio cannot do it as it stands.
    private func studioPerfStep(_ interaction: String, session: StudioSession, window: NSWindow, content: NSView) -> ((Int) -> Void)? {
        let document = session.document
        let centre = content.convert(NSPoint(x: content.bounds.midX, y: content.bounds.midY), to: nil)
        switch interaction {
        case "set":
            guard let token = document.selected, !token.hasPrefix("metrics.") else { return nil }
            // A hue further each time, so every sample is a value the draft has not got.
            return { index in document.set(token, to: String(format: "#%02x6f%02x", (index * 37) % 256, 255 - (index * 53) % 256), merging: true) }
        case "type":
            guard let token = document.selected, !token.hasPrefix("metrics.") else { return nil }
            let typed = "mix(primary, surface.window, 30%)"
            return { index in
                let text = String(typed.prefix(index % typed.count + 1))
                document.fieldText = text
                document.validate(token, typing: text)
            }
        case "select":
            let tokens = ["text.muted", "surface.panel", "terminal.ansi.4", "border", "primary"]
            return { index in document.selected = tokens[index % tokens.count] }
        case "area":
            let areas = StudioArea.allCases
            return { index in document.filter.area = areas[(index + 1) % areas.count] }
        case "filter":
            let typed = "surface"
            return { index in document.filter.query = String(typed.prefix((index + 1) % (typed.count + 1))) }
        case "undo":
            guard document.undoDepth > 0 else { return nil }
            return { index in if index.isMultiple(of: 2) { document.undo() } else { document.redo() } }
        case "appearance":
            let versions = document.file.versions
            guard versions.count == 2 else { return nil }
            return { _ in document.switchVersion(to: versions.first { $0 != document.shown } ?? versions[0]) }
        case "resize":
            let size = content.bounds.size
            return { index in window.setContentSize(NSSize(width: size.width + (index.isMultiple(of: 2) ? 120 : 0), height: size.height)) }
        case "scroll":
            // Down for the first half of the samples and up for the rest: the specimens that come into view are built.
            return { [injector] index in _ = injector.scroll(centre, in: window, dy: index < 40 ? 120 : -120) }
        default:
            return nil
        }
    }

    private func studioPerfReport(_ interaction: String, _ samples: [StudioPerfSample], calls: StudioCoreCalls, window: NSWindow) {
        let steady = Array(samples.dropFirst())
        func part(_ name: String, _ value: (StudioPerfSample) -> Double) -> String {
            let sorted = steady.map(value).sorted()
            return String(format: "%@=%.1f/%.1f", name, sorted[sorted.count / 2], sorted[sorted.count - 1])
        }
        let line = "studio-perf \(interaction) n=\(steady.count) "
            + [part("model", \.model), part("update", \.update), part("draw", \.draw), part("total", \.total), part("cpu", \.cpu)].joined(separator: " ")
            + " (ms, median/max) first_total=\(String(format: "%.1f", samples[0].total)) core=\(calls.words)"
            + " occluded=\(!window.occlusionState.contains(.visible))"
        devctl?.append(line, to: "studio-perf.txt")
        reply(line)
    }

    /// The calling thread's CPU time, and every thread's of the process, in milliseconds.
    private static func mainThreadCPU() -> Double { Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)) / 1e6 }
    private static func processCPU() -> Double { Double(clock_gettime_nsec_np(CLOCK_PROCESS_CPUTIME_ID)) / 1e6 }
}

private struct StudioPerfSample {
    let model: Double
    let update: Double
    let draw: Double
    let cpu: Double
    var total: Double { model + update + draw }
}

extension StudioCoreCalls {
    fileprivate var words: String { "resolve:\(resolves),contrast:\(contrasts),update:\(updateChecks),complete:\(completions)" }
}

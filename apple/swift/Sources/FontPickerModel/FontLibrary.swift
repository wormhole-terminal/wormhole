// The fonts a Mac has, loaded once for a window's pickers and kept: the installed families first, then which of
// them are monospace (the slower question), plus the bundled family and the user's font files (registered for this
// process by the app, so each name draws in its own face). The system is asked off the main actor.

import Foundation
import Observation

@MainActor @Observable
public final class FontLibrary {
    public private(set) var catalog = FontCatalog()
    @ObservationIgnored public let rules: any FontRules
    @ObservationIgnored public let system: any SystemFonts
    @ObservationIgnored public let recents: any FontRecents
    @ObservationIgnored private var started = false

    public init(rules: any FontRules, system: any SystemFonts, recents: any FontRecents = MemoryFontRecents()) {
        (self.rules, self.system, self.recents) = (rules, system, recents)
    }

    /// A library that already knows its families (a sample for the gallery, a test): nothing is asked of the system.
    public convenience init(
        rules: any FontRules, system: any SystemFonts, recents: any FontRecents = MemoryFontRecents(), known catalog: FontCatalog
    ) {
        self.init(rules: rules, system: system, recents: recents)
        (self.catalog, started) = (catalog, true)
    }

    /// Starts listing and classifying (once) without waiting; a picker shows what is there so far.
    public func ensureLoaded() { Task { await load() } }

    /// Reads the fonts folder again: the files may have changed since the picker last opened.
    public func refreshCustom() async {
        guard started else { return }
        await apply(custom: await scan())
    }

    /// Lists and classifies (once), returning when both answers are in.
    public func load() async {
        guard !started else { return }
        started = true
        let system = system
        let custom = await scan()
        let installed = await Task.detached { system.families() }.value
        catalog.installed = installed
        await apply(custom: custom)
        let traits = await Task.detached { system.traits(of: installed) }.value
        var monospace = Set(traits.filter(\.value.monospace).keys)
        monospace.formUnion(catalog.bundled)
        monospace.formUnion(catalog.custom.filter(\.monospace).map(\.name))
        catalog.symbolic = Set(traits.filter(\.value.symbolic).keys)
        catalog.monospace = monospace
    }

    private func scan() async -> CustomFonts {
        let rules = rules
        return await Task.detached { rules.scanCustomFonts() }.value
    }

    /// A custom font named like the bundled one replaces it (it is the person's choice), so it is listed once.
    private func apply(custom: CustomFonts) async {
        let system = system
        let replaced = custom.families.contains { $0.name == rules.bundledFamily }
        // The custom files are the app's to register and keep in step with the folder (`SettingsBridge`'s `FontFolder`).
        let files = replaced ? [] : rules.bundledFiles
        await Task.detached { system.register(files) }.value
        catalog.custom = custom.families
        catalog.bundled = replaced ? [] : [rules.bundledFamily]
        if catalog.monospace != nil {
            catalog.monospace = (catalog.monospace ?? []).union(catalog.bundled).union(custom.families.filter(\.monospace).map(\.name))
        }
    }
}

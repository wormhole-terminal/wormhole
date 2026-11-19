// swift-tools-version: 6.2
import Foundation
import PackageDescription

// The Metal terminal view of the Swift UI experiment, with a synthetic frame source so the renderer is
// developed and measured without Rust. It builds without the FFI; the real source (`Sources/UniFFISource`)
// joins only when `WormholeFFI.xcframework` has been linked in beside this file by
// `macos/scripts/build-ffi.sh macos/swift`, which also links the generated `Sources/WormholeBindings/`.

let ffiLinked = FileManager.default.fileExists(atPath: Context.packageDirectory + "/WormholeFFI.xcframework")

/// The frame loop is a hot loop over unsafe buffers; exclusivity checks on the class properties it touches
/// cost a measurable share there (plan 3.2), and nothing in these targets relies on them.
let hotLoopSettings: [SwiftSetting] = [.unsafeFlags(["-enforce-exclusivity=unchecked"])]

var targets: [Target] = [
    // The packed frame, the `FrameSource` seam and its synthetic implementation. Foundation only.
    .target(name: "TerminalCore", swiftSettings: hotLoopSettings),
    // The Metal view: CoreText, the glyph atlas, instanced quads, the display link, keyboard and IME.
    .target(name: "TerminalMetal", dependencies: ["TerminalCore", "DesignSystem", "WordsModel", "AppLogging"], swiftSettings: hotLoopSettings),
    .testTarget(name: "TerminalCoreTests", dependencies: ["TerminalCore", "TerminalMetal"]),

    // The window around the terminal. Layers, lowest first; each depends only on the ones above it here.
    // The window's state as plain values and the actions on it. Foundation only.
    .target(name: "WorkspaceModel"),
    // The app's windows as plain values: which there are, where each was, the seam the core keeps them behind. Foundation only.
    .target(name: "WindowsModel", dependencies: ["WorkspaceModel"]),
    .testTarget(name: "WindowsTests", dependencies: ["WindowsModel", "WorkspaceModel"]),
    // Tokens and shared components (SwiftUI). Knows nothing of terminals. `themes.json` is every bundled
    // theme's resolved tokens, written from `wormhole-core` by `macos/scripts/dump-themes.sh`.
    .target(name: "DesignSystem", dependencies: ["WordsModel"], resources: [.copy("Resources/themes.json")]),
    // The sidebar's list, rows and header.
    .target(name: "SidebarUI", dependencies: ["WorkspaceModel", "DesignSystem", "WordsModel", "PanesModel"]),
    // Title rows, status bar, right panel and the window that holds them with the terminal view.
    .target(
        name: "WindowUI",
        dependencies: [
            "WorkspaceModel", "DesignSystem", "SidebarUI", "PanelModel", "PanelUI", "FileIconTheme", "PaletteModel", "PaletteUI",
            "TerminalMetal", "TerminalCore", "TerminalUI", "PanesModel", "PanesUI", "PlatformModel", "WindowsModel", "WordsModel",
        ]),
    // The right panel's tabs as plain values and pure logic: tasks, the diff parser, commits. Foundation only.
    .target(name: "PanelModel", dependencies: ["WorkspaceModel", "WordsModel"]),
    // The right panel's tab contents: Notes, Changes (with the diff), History, Files, Project and Usage.
    .target(name: "PanelUI", dependencies: ["PanelModel", "DesignSystem", "WorkspaceModel", "WordsModel"]),
    // The app's file icon theme (Catppuccin and Lucide SVGs in Zed's icon theme format) and its lookup, behind the
    // panel's `FileIconProviding`. `macos/scripts/dump-icon-theme.sh` copies the files from `wormhole-core`.
    .target(
        name: "FileIconTheme", dependencies: ["PanelModel"],
        resources: [.copy("Resources/wormhole-icons.json"), .copy("Resources/icons")]),
    .testTarget(name: "PanelTests", dependencies: ["PanelModel", "PanelUI", "DesignSystem", "FileIconTheme", "WorkspaceModel"]),
    // The command palette's commands, fuzzy search and the shortcut sheet's rows. Foundation only.
    .target(name: "PaletteModel", dependencies: ["WorkspaceModel", "WordsModel"]),
    // The palette and shortcut sheet overlays.
    .target(name: "PaletteUI", dependencies: ["PaletteModel", "WorkspaceModel", "DesignSystem", "WordsModel"]),
    .testTarget(name: "PaletteTests", dependencies: ["PaletteModel", "PaletteUI", "WorkspaceModel", "DesignSystem"]),
    // The SwiftUI around the terminal: find bar, command blocks, pane bars, notice chip, zoom and resize readout,
    // minimap, print and export. Over protocols that the Rust FFI backs later; no dependency on a feature module.
    .target(name: "TerminalUI", dependencies: ["DesignSystem", "TerminalCore", "WordsModel"]),
    .testTarget(name: "TerminalUITests", dependencies: ["TerminalUI", "DesignSystem", "TerminalCore"]),
    // The settings window's schema (generated from the Rust settings index by `macos/scripts/dump-settings.sh`),
    // values, search, change tracking and the backend seam. Foundation only.
    .target(name: "SettingsModel", dependencies: ["FontPickerModel", "WordsModel"], resources: [.copy("Resources/settings.json")]),
    // The settings window: nav, search, preview strip, the rows and their controls, drawn from the schema.
    .target(name: "SettingsUI", dependencies: ["SettingsModel", "DesignSystem", "WordsModel", "WorkspaceModel", "SidebarUI", "AuxModel", "AuxWindows", "FontPickerModel", "FontPickerUI"]),
    .testTarget(name: "SettingsTests", dependencies: ["SettingsModel", "SettingsUI", "DesignSystem", "FontPickerModel", "FontPickerUI"]),
    // The font picker's state and rules as plain values: the families and where each comes from, the list for a query
    // (core's rules, or the same in Swift), the highlight and its keys, recents, the preview's text. Foundation only.
    .target(name: "FontPickerModel", dependencies: ["WordsModel"]),
    // The font picker's views (search, list, preview with what the font can draw) and CoreText behind its questions.
    .target(name: "FontPickerUI", dependencies: ["FontPickerModel", "DesignSystem"]),
    .testTarget(name: "FontPickerTests", dependencies: ["FontPickerModel", "FontPickerUI", "DesignSystem"]),
    // Splits and panes (the tree, its geometry, drops, presets), the title row's parts and fit rules and the
    // tab sets, as plain values. Foundation and the workspace model.
    .target(name: "PanesModel", dependencies: ["WorkspaceModel", "WordsModel"]),
    // The panes' area with headers, dividers and drop hints, the title row's pieces and the tab bar.
    .target(name: "PanesUI", dependencies: ["PanesModel", "WorkspaceModel", "DesignSystem", "WordsModel", "SidebarUI"]),
    .testTarget(name: "PanesTests", dependencies: ["PanesModel", "PanesUI", "WorkspaceModel", "DesignSystem"]),
    // The secondary windows' data as plain values and pure logic: the intro's timing and ship, the first-run
    // steps, licenses, updates, session hosts, agent access, trust and the close questions. Foundation only.
    .target(name: "AuxModel", dependencies: ["WordsModel"]),
    // The secondary windows: onboarding with the Metal wormhole intro, About, Licenses, Updates, Session
    // Hosts, the agent access sheet, project trust and the close and quit questions. `Resources/shader` is the
    // GPUI app's own intro shader, copied in by `macos/scripts/build-ffi.sh` (git-ignored), so the two never drift.
    .target(
        name: "AuxWindows", dependencies: ["AuxModel", "DesignSystem", "FontPickerModel", "FontPickerUI"],
        resources: [.copy("Resources/shader")]),
    .testTarget(name: "AuxTests", dependencies: ["AuxModel", "AuxWindows", "DesignSystem"]),
    .testTarget(name: "WorkspaceModelTests", dependencies: ["WorkspaceModel"]),
    // The seam for the interface's words (`wormhole-text`'s ids and tables behind it, when linked). Foundation only.
    .target(name: "WordsModel"),
    // The seam for the app's own log lines: the modules below the app write through it into the core's log files. Foundation only.
    .target(name: "AppLogging"),
    // The macOS edge as plain values and pure logic: the keymap (core's table as data), the menu bar and context
    // menus, notifications, the ambient surfaces, the folder picker's model, Touch ID's protocol. Foundation only.
    .target(name: "PlatformModel", dependencies: ["WordsModel", "WorkspaceModel"], resources: [.copy("Resources/keymap.json")]),
    // AppKit and UserNotifications around it: menus, hotkey, banners, status item, Dock, quick panel, picker panel.
    .target(name: "PlatformUI", dependencies: ["PlatformModel", "DesignSystem", "WordsModel", "AppLogging"]),
    .testTarget(name: "PlatformTests", dependencies: ["PlatformModel", "PlatformUI", "DesignSystem"]),
    // The Theme Studio's logic: a theme file as a draft, resolved by Rust, with undo, contrast and versions.
    .target(name: "StudioModel", dependencies: ["DesignSystem", "WordsModel"]),
    // The Theme Studio window: token list, gallery of every component, token editor.
    .target(name: "StudioUI", dependencies: ["StudioModel", "DesignSystem", "SidebarUI", "PanelUI", "PaletteUI", "WorkspaceModel", "PanelModel", "PaletteModel", "WordsModel", "FontPickerUI", "PanesUI", "TerminalUI", "AuxWindows", "SettingsUI", "PlatformModel", "PlatformUI"]),
    .testTarget(name: "StudioTests", dependencies: ["StudioModel", "StudioUI", "DesignSystem"]),
    .testTarget(
        name: "UITests",
        dependencies: ["WorkspaceModel", "DesignSystem", "SidebarUI", "WindowUI", "PanesModel", "PanelModel", "PlatformModel", "WindowsModel"]),
]

var appDependencies: [Target.Dependency] = [
    "TerminalCore", "TerminalMetal", "WorkspaceModel", "WindowsModel", "WindowUI", "SettingsModel", "SettingsUI", "DesignSystem",
    "PanesModel", "PanesUI", "PanelModel", "PlatformModel", "PlatformUI", "AuxModel", "AuxWindows", "TerminalUI",
    "PaletteModel", "PanelModel", "PanelUI", "StudioModel", "StudioUI", "SidebarUI", "PanesModel", "WordsModel", "FileIconTheme", "FontPickerModel", "AppLogging",
]
// The view's input: pointer, selection, links, keyboard and input method, on the mock and (when the FFI is
// linked) on the real engine.
var inputTestDependencies: [Target.Dependency] = ["TerminalCore", "TerminalMetal"]
var inputTestSettings: [SwiftSetting] = []

var appSettings: [SwiftSetting] = []

if ffiLinked {
    targets += [
        // The Rust static library with UniFFI's C header and module map.
        .binaryTarget(name: "WormholeFFI", path: "WormholeFFI.xcframework"),
        // Generated by uniffi-bindgen. Swift 5 mode: the generated code predates our strictness, and ours
        // stays at Swift 6 (plan 2.2).
        .target(
            name: "WormholeBindings",
            dependencies: ["WormholeFFI"],
            swiftSettings: [.swiftLanguageMode(.v5)],
            linkerSettings: [
                // What the Rust crates behind the library link.
                .linkedFramework("Security"),
                .linkedFramework("CoreFoundation"),
                .linkedFramework("CoreServices"),
                .linkedFramework("SystemConfiguration"),
                .linkedLibrary("iconv"),
            ]
        ),
        .target(name: "UniFFISource", dependencies: ["TerminalCore", "WormholeBindings", "DesignSystem"]),
        // The window's state kept by the Rust workspace: actions in, snapshots out as WorkspaceModel's values, and the
        // programs started for the terminals it opens.
        .target(
            name: "WorkspaceBridge",
            dependencies: ["WorkspaceModel", "WormholeBindings", "UniFFISource", "TerminalCore", "PanesModel"]),
        // The app's windows over the core's store: what each held at the last run, the ids they share, where each was.
        .target(name: "WindowsBridge", dependencies: ["WindowsModel", "WorkspaceBridge", "WorkspaceModel", "WormholeBindings"]),
        .testTarget(
            name: "WindowsBridgeTests",
            dependencies: ["WindowsBridge", "WindowsModel", "WorkspaceBridge", "WorkspaceModel", "WormholeBindings", "UniFFISource", "TerminalCore"]),
        // The theme resolver against the real Rust loader: a user theme file, colour picks, an opacity.
        .testTarget(name: "ThemeFFITests", dependencies: ["DesignSystem", "UniFFISource", "StudioModel", "StudioBridge", "SettingsUI", "SettingsModel", "StudioUI"]),
        // The mapping and the real workspace behind the store: an action through the FFI, a snapshot back.
        .testTarget(name: "WorkspaceBridgeTests", dependencies: ["WorkspaceBridge", "WorkspaceModel", "PanesModel"]),
        // The panel's Files, Project, Usage and Notes backends over the Rust core (trust enforced in Rust).
        .target(name: "FilesBridge", dependencies: ["PanelModel", "PlatformModel", "WordsModel", "WorkspaceModel", "WormholeBindings"]),
        .testTarget(name: "FilesBridgeTests", dependencies: ["FilesBridge", "PanelModel", "PlatformModel", "WormholeBindings", "WorkspaceModel"]),
        // The settings window over the Rust core: values, writes, search, shortcut rules, the agents, imports and
        // profiles. Replaces the mock backend and host.
        .target(name: "SettingsBridge", dependencies: ["SettingsModel", "FontPickerModel", "DesignSystem", "WormholeBindings", "WorkspaceModel"]),
        // The Theme Studio's theme file rules from the core: formats, copies, pairs, updates (`StudioFiles`).
        .target(name: "StudioBridge", dependencies: ["StudioModel", "DesignSystem", "WormholeBindings"]),
        .testTarget(name: "SettingsBridgeTests", dependencies: ["SettingsBridge", "SettingsModel", "FontPickerModel", "WormholeBindings"]),
        // The auxiliary windows' backends (update, session hosts, project trust, agent access, first run, licenses).
        .target(name: "AuxBridge", dependencies: ["AuxModel", "PlatformModel", "WormholeBindings"]),
        .testTarget(name: "AuxBridgeTests", dependencies: ["AuxBridge", "AuxModel", "PlatformModel"]),
        // The agents' hooks and the control line (`Wormhole list|send-text|…`) served over the Rust workspace, and
        // the executable's own command line (`hook`, `statusline`) for `main` to answer first.
        .target(name: "HookBridge", dependencies: ["WormholeBindings"]),
        .testTarget(name: "HookBridgeTests", dependencies: ["HookBridge", "WormholeBindings"]),
        // The built app, started as a test instance on scratch folders and driven through its control folder: a person's
        // scenarios asserted on what the app reports (`macos/parity/DRIVING.md`, "End-to-end tests"). It imports nothing of the app:
        // it runs the `Wormhole` product that `swift build` and `swift test` make, and skips itself when that is not there.
        .testTarget(name: "EndToEndTests"),
    ]
    appDependencies += [
        "UniFFISource", "WorkspaceBridge", "WindowsBridge", "GitBridge", "FilesBridge", "AuxBridge", "SettingsBridge", "PlatformBridge", "WordsBridge", "HookBridge", "StudioBridge",
    ]
    // The right panel's git over the Rust core: `GitBackend` and `WorktreeBackend` for the window to hand to the panel.
    targets.append(.target(name: "GitBridge", dependencies: ["PanelModel", "WordsModel", "WormholeBindings"]))
    targets.append(.testTarget(name: "GitBridgeTests", dependencies: ["GitBridge", "PanelModel", "WormholeBindings"]))
    // The macOS edge's logic from core: the keymap and menu bar, the notification rules, the folder picker's
    // providers and the palette's search; and the interface's words.
    targets += [
        .target(name: "PlatformBridge", dependencies: ["PlatformModel", "PaletteModel", "WordsModel", "WormholeBindings"]),
        .target(name: "WordsBridge", dependencies: ["WordsModel", "WormholeBindings"]),
        .testTarget(
            name: "PlatformBridgeTests",
            dependencies: ["PlatformBridge", "PlatformModel", "PaletteModel", "PaletteUI", "DesignSystem", "WorkspaceModel", "WormholeBindings"]),
        .testTarget(name: "WordsBridgeTests", dependencies: ["WordsBridge", "WordsModel", "WormholeBindings"]),
    ]
    appSettings.append(.define("WITH_UNIFFI"))
    inputTestDependencies.append("UniFFISource")
    inputTestSettings.append(.define("WITH_UNIFFI"))
}

targets.append(.testTarget(name: "TerminalInputTests", dependencies: inputTestDependencies, swiftSettings: inputTestSettings))

// The app's own coordinators, which no module below it holds (what the notes.md merge saves).
targets.append(.testTarget(name: "WormholeTests", dependencies: ["Wormhole", "WindowUI", "WorkspaceModel", "PanelModel", "PanelUI", "SettingsModel"]))
targets.append(
    .executableTarget(
        name: "Wormhole",
        dependencies: appDependencies,
        exclude: ["Info.plist"],
        swiftSettings: appSettings,
        linkerSettings: [
            // The bundle identity of a bare executable: Bundle.main reads it from this section, so the
            // isolation check (`Isolation.swift`) sees the same identifier inside and outside an app bundle.
            .unsafeFlags([
                "-Xlinker", "-sectcreate", "-Xlinker", "__TEXT", "-Xlinker", "__info_plist",
                "-Xlinker", Context.packageDirectory + "/Sources/Wormhole/Info.plist",
            ])
        ]
    ))

let package = Package(
    name: "WormholeMetalSpike",
    platforms: [.macOS(.v26)],
    products: [
        // The executable keeps the product's name (the updater looks for `Contents/MacOS/Wormhole`).
        .executable(name: "Wormhole", targets: ["Wormhole"])
    ],
    targets: targets
)

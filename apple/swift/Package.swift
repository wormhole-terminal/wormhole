// swift-tools-version: 6.2
import Foundation
import PackageDescription

// The Metal terminal view of the Swift UI experiment, with a synthetic frame source so the renderer is
// developed and measured without Rust. It builds without the FFI; the real source (`Sources/UniFFISource`)
// joins only when `WormholeFFI.xcframework` has been linked in beside this file by
// `apple/scripts/build-ffi.sh apple/swift`, which also links the generated `Sources/WormholeBindings/`.

let ffiLinked = FileManager.default.fileExists(atPath: Context.packageDirectory + "/WormholeFFI.xcframework")

/// The frame loop is a hot loop over unsafe buffers; exclusivity checks on the class properties it touches
/// cost a measurable share there (plan 3.2), and nothing in these targets relies on them.
let hotLoopSettings: [SwiftSetting] = [.unsafeFlags(["-enforce-exclusivity=unchecked"])]

var targets: [Target] = [
    // The packed frame, the `FrameSource` seam and its synthetic implementation. Foundation only.
    .target(name: "TerminalCore", swiftSettings: hotLoopSettings),
    // The Metal view: CoreText, the glyph atlas, instanced quads, the display link, keyboard and IME.
    .target(name: "TerminalMetal", dependencies: ["TerminalCore"], swiftSettings: hotLoopSettings),
    .testTarget(name: "TerminalCoreTests", dependencies: ["TerminalCore", "TerminalMetal"]),

    // The window around the terminal. Layers, lowest first; each depends only on the ones above it here.
    // The window's state as plain values and the actions on it. Foundation only.
    .target(name: "WorkspaceModel"),
    // Tokens and shared components (SwiftUI). Knows nothing of terminals. `themes.json` is every bundled
    // theme's resolved tokens, written from `wormhole-core` by `apple/scripts/dump-themes.sh`.
    .target(name: "DesignSystem", resources: [.copy("Resources/themes.json")]),
    // The sidebar's list, rows and header.
    .target(name: "SidebarUI", dependencies: ["WorkspaceModel", "DesignSystem"]),
    // Title rows, status bar, right panel and the window that holds them with the terminal view.
    .target(
        name: "WindowUI",
        dependencies: ["WorkspaceModel", "DesignSystem", "SidebarUI", "PanelModel", "PanelUI", "FileIconTheme", "PaletteUI", "TerminalMetal"]),
    // The right panel's tabs as plain values and pure logic: tasks, the diff parser, commits. Foundation only.
    .target(name: "PanelModel", dependencies: ["WorkspaceModel"]),
    // The right panel's tab contents: Notes, Changes (with the diff), History, Files, Project and Usage.
    .target(name: "PanelUI", dependencies: ["PanelModel", "DesignSystem", "WorkspaceModel"]),
    // The app's file icon theme (Catppuccin and Lucide SVGs in Zed's icon theme format) and its lookup, behind the
    // panel's `FileIconProviding`. `apple/scripts/dump-icon-theme.sh` copies the files from `wormhole-core`.
    .target(
        name: "FileIconTheme", dependencies: ["PanelModel"],
        resources: [.copy("Resources/wormhole-icons.json"), .copy("Resources/icons")]),
    .testTarget(name: "PanelTests", dependencies: ["PanelModel", "PanelUI", "DesignSystem", "FileIconTheme", "WorkspaceModel"]),
    // The command palette's commands, fuzzy search and the shortcut sheet's rows. Foundation only.
    .target(name: "PaletteModel", dependencies: ["WorkspaceModel"]),
    // The palette and shortcut sheet overlays.
    .target(name: "PaletteUI", dependencies: ["PaletteModel", "WorkspaceModel", "DesignSystem"]),
    .testTarget(name: "PaletteTests", dependencies: ["PaletteModel", "PaletteUI", "WorkspaceModel", "DesignSystem"]),
    // The SwiftUI around the terminal: find bar, command blocks, pane bars, notice chip, zoom and resize readout,
    // minimap, print and export. Over protocols that the Rust FFI backs later; no dependency on a feature module.
    .target(name: "TerminalUI", dependencies: ["DesignSystem", "TerminalCore"]),
    .testTarget(name: "TerminalUITests", dependencies: ["TerminalUI", "DesignSystem", "TerminalCore"]),
    // The settings window's schema (generated from the Rust settings index by `apple/scripts/dump-settings.sh`),
    // values, search, change tracking and the backend seam. Foundation only.
    .target(name: "SettingsModel", resources: [.copy("Resources/settings.json")]),
    // The settings window: nav, search, preview strip, the rows and their controls, drawn from the schema.
    .target(name: "SettingsUI", dependencies: ["SettingsModel", "DesignSystem"]),
    .testTarget(name: "SettingsTests", dependencies: ["SettingsModel", "SettingsUI", "DesignSystem"]),
    // Splits and panes (the tree, its geometry, drops, presets), the title row's parts and fit rules and the
    // tab sets, as plain values. Foundation and the workspace model.
    .target(name: "PanesModel", dependencies: ["WorkspaceModel"]),
    // The panes' area with headers, dividers and drop hints, the title row's pieces and the tab bar.
    .target(name: "PanesUI", dependencies: ["PanesModel", "WorkspaceModel", "DesignSystem"]),
    .testTarget(name: "PanesTests", dependencies: ["PanesModel", "PanesUI", "WorkspaceModel", "DesignSystem"]),
    // The secondary windows' data as plain values and pure logic: the intro's timing and ship, the first-run
    // steps, licenses, updates, session hosts, agent access, trust and the close questions. Foundation only.
    .target(name: "AuxModel"),
    // The secondary windows: onboarding with the Metal wormhole intro, About, Licenses, Updates, Session
    // Hosts, the agent access sheet, project trust and the close and quit questions. `Resources/shader` is the
    // GPUI app's own intro shader, linked (not copied), so the two never drift.
    .target(
        name: "AuxWindows", dependencies: ["AuxModel", "DesignSystem"],
        resources: [.copy("Resources/shader")]),
    .testTarget(name: "AuxTests", dependencies: ["AuxModel", "AuxWindows", "DesignSystem"]),
    .testTarget(name: "WorkspaceModelTests", dependencies: ["WorkspaceModel"]),
    // The macOS edge as plain values and pure logic: the keymap (core's table as data), the menu bar and context
    // menus, notifications, the ambient surfaces, the folder picker's model, Touch ID's protocol. Foundation only.
    .target(name: "PlatformModel", resources: [.copy("Resources/keymap.json")]),
    // AppKit and UserNotifications around it: menus, hotkey, banners, status item, Dock, quick panel, picker panel.
    .target(name: "PlatformUI", dependencies: ["PlatformModel", "DesignSystem"]),
    .testTarget(name: "PlatformTests", dependencies: ["PlatformModel", "PlatformUI", "DesignSystem"]),
    .testTarget(name: "UITests", dependencies: ["WorkspaceModel", "DesignSystem", "SidebarUI", "WindowUI"]),
]

var appDependencies: [Target.Dependency] = [
    "TerminalCore", "TerminalMetal", "WorkspaceModel", "WindowUI", "SettingsModel", "SettingsUI",
]
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
    ]
    // The theme resolver against the real Rust loader: a user theme file, colour picks, an opacity.
    targets.append(.testTarget(name: "ThemeFFITests", dependencies: ["DesignSystem", "UniFFISource"]))
    appDependencies.append("UniFFISource")
    appSettings.append(.define("WITH_UNIFFI"))
}

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

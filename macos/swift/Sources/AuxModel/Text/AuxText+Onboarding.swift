// The intro window's words (`onboarding/` of the GPUI app: the welcome page and F-38's three steps).

import Foundation

extension AuxText {
    public static var onboardingLabel: String { tr("onboarding-dialog-label", "Welcome to Wormhole") }
    public static var tagline: String { tr("onboarding-tagline", "A terminal for working with AI coding agents.") }
    public static var specimenCaption: String {
        tr("onboarding-specimen", "Which agent is working, which needs input, which is done.")
    }
    public static var getStarted: String { tr("onboarding-get-started", "Get started") }
    public static var backToWormhole: String { tr("onboarding-back-to-wormhole", "Back to Wormhole") }
    public static var skipIntro: String { tr("onboarding-skip-intro", "Skip intro") }
    public static var skipSetup: String { tr("onboarding-skip-setup", "Skip setup") }
    public static var replayIntro: String { tr("onboarding-replay", "Replay intro") }
    public static var hintContinue: String { tr("onboarding-hint-continue", "continue") }
    public static var hintSkip: String { tr("onboarding-hint-skip", "skip") }
    public static var back: String { tr("tailor-back", "Back") }
    public static var next: String { tr("onboarding-continue", "Continue") }
    public static var start: String { tr("onboarding-start", "Start") }
    public static func stepCounter(_ n: Int, of total: Int, noAgents: Bool) -> String {
        noAgents
            ? tr("onboarding-step-counter-no-agents", ["step": .whole(n), "total": .whole(total)], fallback: "Step \(n) of \(total) \u{B7} no agents found")
            : tr("onboarding-step-counter", ["step": .whole(n), "total": .whole(total)], fallback: "Step \(n) of \(total)")
    }

    public static var folderTitle: String { tr("onboarding-folder-title", "Where do you work?") }
    public static var folderIntro: String {
        tr("onboarding-folder-intro", "Wormhole is a terminal first. Pick a folder to start in; every terminal remembers its own.")
    }
    public static var openFolder: String { tr("onboarding-open-folder", "Open folder…") }
    public static var openFolderDetail: String {
        tr("onboarding-open-folder-detail", "A terminal starts there; git changes, history and branch follow it.")
    }
    public static func startsIn(_ folder: String) -> String { tr("onboarding-starts-in", ["folder": .text(folder)], fallback: "Starts in \(folder)") }
    public static var recents: String { tr("onboarding-recents", "Recents") }
    public static var places: String { tr("onboarding-places", "Places") }
    public static var dropFolder: String { tr("onboarding-drop-folder", "Or drop a folder here") }
    public static var placeHome: String { tr("onboarding-place-home", "Home") }
    public static var placeDesktop: String { tr("onboarding-place-desktop", "Desktop") }
    public static var placeDocuments: String { tr("onboarding-place-documents", "Documents") }
    public static var placeDeveloper: String { tr("onboarding-place-developer", "Developer") }
    public static var startHere: String { tr("onboarding-panel-start-here", "Start here") }

    public static var lookTitle: String { tr("onboarding-look-title", "Make it yours") }
    public static func lookIntro(settingsKeys: String) -> String {
        tr("onboarding-look-intro", ["keys": .text(settingsKeys)], fallback: "Everything changes live, and all of it is in Settings (\(settingsKeys)) later.")
    }
    public static var primaryColour: String { tr("onboarding-primary", "Primary colour") }
    /// The Look step's theme cards, as VoiceOver names their group (Settings' word).
    public static var theme: String { tr("settings-appearance-theme", "Theme") }
    public static var terminalFont: String { tr("onboarding-font", "Terminal font") }
    /// The link under the theme cards (GPUI writes it as a literal in `look_step.rs`: it has no catalog message).
    public static var importFromOthers: String { tr("onboarding-import-from", "Import from Ghostty, iTerm2 or Alacritty\u{2026}") }
    public static var importTheme: String { tr("onboarding-panel-import-theme", "Import theme") }
    public static var ligatures: String { tr("settings-fonts-font-ligatures", "Ligatures") }
    public static var themeLabel: String { tr("settings-appearance-theme", "Theme") }
    public static var fontSize: String { tr("settings-import-setting-font-size", "Font size") }

    public static var agentsTitle: String { tr("settings-section-agents", "Coding agents") }
    public static func agentsIntro(found: Int) -> String {
        tr("onboarding-agents-intro", ["count": .whole(found)], fallback: "Found \(found) on this Mac. Install status hooks for the ones you want; they report when an agent works, waits for you or is done.")
    }
    public static var lookingForAgents: String { tr("settings-agents-looking-for-agents", "Looking for agents…") }
    public static var hooksIn: String { tr("onboarding-hooks-in", "Hooks in") }
    public static var installed: String { tr("settings-agents-group-installed", "Installed") }
    public static var install: String { tr("settings-agents-install", "Install") }
    public static var repair: String { tr("onboarding-repair", "Repair") }
    public static var onPath: String { tr("onboarding-on-path", "On PATH") }
    public static var onPathOutput: String { tr("onboarding-on-path-output", "On PATH \u{B7} status from its output") }
    public static var readingOutput: String { tr("settings-agents-reading-output", "Reading output") }
    public static func notFound(more: Int) -> String {
        tr("onboarding-not-found", ["count": .whole(more)], fallback: "Not found on PATH \u{B7} \(more) more recognised")
    }
    public static var installsNothing: String {
        tr("onboarding-installs-nothing", "Start installs nothing; only Install does. Hooks add entries of their own and back up what they change. Skip any agent: Settings \u{203A} Coding agents adds or removes hooks later.")
    }
    public static var notifyBackground: String {
        tr("onboarding-notify", "Notify me when a background terminal needs me or finishes")
    }
    public static var notifyCommand: String {
        tr("onboarding-notify-command", "Notify me when a long command in a background terminal finishes")
    }
    public static func trustHas(name: String, dir: String) -> String { tr("onboarding-trust-has", ["name": .text(name), "dir": .text(dir)], fallback: "\(name)/ has a \(dir)/ folder") }
    public static func trustWaits(_ commands: Int) -> String {
        switch commands {
        case 0: tr("onboarding-trust-waits-0", "Nothing in it runs commands yet.")
        case 1: tr("onboarding-trust-waits-1", "Its action runs a command, so it waits until you trust the folder.")
        default: tr("onboarding-trust-waits-n", ["count": .whole(commands)], fallback: "Its actions run \(commands) commands, so they wait until you trust the folder.")
        }
    }
    public static var trustWorksNow: String { tr("onboarding-trust-works-now", "Its theme and notes work now.") }
    public static var trustTouchID: String { tr("settings-projects-trust-with-touch-id", "Trust with Touch ID…") }
    public static var trustLater: String { tr("onboarding-trust-later", "Later") }

    public static var primerTitle: String { tr("onboarding-primer-title", "Ready when you are") }
    public static var primerIntro: String {
        tr("onboarding-primer-intro", "Wormhole didn\u{2019}t find a coding agent on this Mac. Nothing here depends on one.")
    }
    public static var useAsTerminal: String { tr("onboarding-use-as-terminal", "Use Wormhole as a terminal") }
    public static var useAsTerminalDetail: String {
        tr("onboarding-use-as-terminal-detail", "The git panel works in any shell.")
    }
    public static var useAsTerminalDetailNotes: String {
        tr("onboarding-use-as-terminal-detail-notes", "Notes, next steps and the git panel work in any shell.")
    }
    public static var tryNote: String { tr("onboarding-try-note", "Jot a note for this folder") }
    public static var thingsToTry: String { tr("onboarding-things-to-try", "Things to try first") }
    public static var tryJump: String { tr("onboarding-try-jump", "Jump to anything") }
    public static var tryShortcuts: String { tr("onboarding-try-shortcuts", "See every shortcut") }
    public static var tryChanges: String { tr("onboarding-try-changes", "Show what changed in git") }
    public static var tryPanels: String { tr("onboarding-try-panels", "Hide the side panels") }
    public static func noAgentFound(others: Int) -> String {
        tr("onboarding-no-agent", ["count": .whole(others)], fallback: "No coding agent found. Install Claude Code, Codex or one of \(others) others later and Wormhole recognises it in any terminal, then offers its hooks once.")
    }

    public static var introEmerge: String { tr("intro-style-emerge", "Emerge") }
    public static var introVortex: String { tr("intro-style-vortex", "Edge Vortex") }
    public static var introCollapse: String { tr("intro-style-collapse", "Collapse") }
    public static var introFTL: String { tr("intro-style-ftl", "FTL Jump") }
    public static var introAurora: String { tr("intro-style-aurora", "Aurora") }
    public static var introTentacles: String { tr("intro-style-tentacles", "Tentacles") }
    public static var introVoyage: String { tr("intro-style-voyage", "Voyage") }
    public static var introGlow: String { tr("intro-style-glow", "Glow") }
}

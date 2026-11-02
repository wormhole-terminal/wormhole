# Settings window, projects page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-projects = Project folders
    .intro = Change the defaults for every project, or pick one to set it up just for you.

settings-projects-projects = Project folders
    .description = The .wormhole folder of a project: trust, and which of its files you use
    .keywords = .wormhole project folder trust touch id actions prompts layout notes theme settings

## settings_window/sections/projects.rs (labels, hints and notes drawn by this page's code)

settings-projects-search-projects = Search projects
settings-projects-all-projects-defaults = All projects (defaults)
# The "For" row: whose choices the rows under it are. Its menu lists "All projects (defaults)" and each known project.
settings-projects-for = For
settings-projects-for-note = All projects sets the defaults. A project can still turn a feature on or off just for you: pick it here.
# The group of the switches and menus, for all projects or for the picked one.
settings-projects-use-from-projects = Use from projects
settings-projects-trusted-projects = Trusted projects
settings-projects-projects-allowed-to-run-commands = Projects allowed to run commands
settings-projects-none-yet = No trusted projects yet
settings-projects-none-hint = Pick a project in the For menu and choose Trust with Touch ID: it can then run the commands it asks for.
settings-projects-settings = Settings
settings-projects-revoke = Revoke
settings-projects-approval-isnt-available-on-this-mac = Approval isn't available on this Mac (no Touch ID or login password), so projects can't be trusted.
settings-projects-trusted = Trusted
settings-projects-changed-since-you-trusted-it = Changed since you trusted it
settings-projects-not-trusted = Not trusted
settings-projects-revoke-trust = Revoke trust
settings-projects-waiting-for-approval = Waiting for approval…
settings-projects-trust-with-touch-id = Trust with Touch ID…
settings-projects-only-for-you-nothing-is-written = Only for you: nothing is written into the project folder.
settings-projects-default-on = Default (on)
settings-projects-default-off = Default (off)
settings-projects-on = On
settings-projects-off = Off
settings-projects-problems = Problems
settings-projects-open-in-a-new-terminal = Open in a new terminal
settings-projects-actions-the-terminal-layout-and-env = Actions, the terminal layout and env/shell may run.
settings-projects-nothing-in-it-needs-trust = Nothing in it needs trust.
settings-projects-review-the-commands-below-and-trust = Review the commands below and trust it again.
settings-projects-nothing-in-it-needs-trust-now = Nothing in it needs trust now.
settings-projects-nothing-in-it-can-run-commands = Nothing in it can run commands until you trust it.
settings-projects-no-notes-md-yet-on-creates = No notes.md yet. On creates it and syncs this project's terminals' notes

## Sentences with a hole in them

# The line under a project that is gone from disk. { $folder } is where it was.
settings-projects-missing-folder = { $folder } has no .wormhole folder anymore.

# The button that shows a project's folder in the Finder. { $folder } is the folder's name.
settings-projects-show-in-finder = Show { $folder } in Finder

# What the Trust row says of a project that is trusted but has no commands. { $applies } says what trust
# applies ("its settings in config.json and actions in actions.json"), in English for now.
settings-projects-trust-applies = Applies { $applies }.

# The same while the project changed since it was trusted.
settings-projects-trust-again = Trust it again to apply { $applies }.

# The same while the project is not trusted yet.
settings-projects-trust-would-apply = Trusting it applies { $applies }.

# A feature's line when its file is there but waits for trust. { $description } is what the feature does.
settings-projects-feature-needs-trust = { $description }. Only after you trust the project

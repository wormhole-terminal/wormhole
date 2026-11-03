# Settings window, projects page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-projects = Project folders
    .intro = Change the defaults for every project, or pick one to set it up just for you.

settings-projects-projects = Project folders
    .description = The .wormhole folder of a project: trust, and which of its files you use
    .keywords = .wormhole project folder trust touch id actions prompts layout notes theme settings

## settings_window/sections/projects.rs (labels, hints and notes drawn by this page's code)

settings-projects-all-projects-defaults = All projects (defaults)
# The "For" row: whose choices the rows under it are. Its menu lists "All projects (defaults)" and each known project.
settings-projects-for = For
settings-projects-for-note = All projects sets the defaults. A project can still turn a feature on or off just for you: pick it here.
settings-projects-default-on = Default (on)
settings-projects-default-off = Default (off)
settings-projects-on = On
settings-projects-off = Off
settings-projects-no-notes-md-yet-on-creates = No notes.md yet. On creates it and syncs this project's terminals' notes

## Sentences with a hole in them

# The line under a project that is gone from disk. { $folder } is where it was.
settings-projects-missing-folder = { $folder } has no .wormhole folder anymore.

# A feature's line when its file is there but waits for trust. { $description } is what the feature does.
settings-projects-feature-needs-trust = { $description }. Only after you trust the project

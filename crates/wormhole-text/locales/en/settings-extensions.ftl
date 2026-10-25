# Settings window, Extensions page (Probes): the page's title and intro, its one row, the list of Probes and the
# sheet that asks before one is turned on. A Probe is a folder with a probe.toml that adds to Wormhole.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-extensions = Extensions
    .intro = Probes are folders that add to Wormhole. A new or changed one stays off until you approve exactly what it asks for.

settings-extensions-probes = Probes
    .description = Folders with a probe.toml in Wormhole's data folder, and what each is allowed to do
    .keywords = probe probes extension extensions plugin permissions approve touch id probe.toml

## The page (settings_window/sections/extensions.rs)

# The tooltip of the button that looks for Probes again.
settings-extensions-refresh = Scan the Probes folder again

settings-extensions-looking = Looking for Probes…

settings-extensions-none-title = No Probes yet

# { $folder } is the Probes folder, shown with ~ for the home folder.
settings-extensions-none-where = Probes live in { $folder }, one folder each with a probe.toml in it.

# { $command } is the command line, as the terminal types it: "Wormhole probe new".
settings-extensions-none-new = Make one with { $command }, or copy a folder in. It stays off until you approve it here.

settings-extensions-show-folder = Show in Finder

# { $count } is a whole number: the most entries the Probes folder may have.
settings-extensions-too-many = The Probes folder has more than { $count } entries, so none are listed. Move the ones you don't use out of it.

# Under a Probe's name. { $id } is its id, { $version } its version and { $author } who wrote it.
settings-extensions-identity = { $id } · version { $version } · by { $author }

settings-extensions-state-off = Off
settings-extensions-state-on = On
settings-extensions-state-changed = Changed
settings-extensions-state-invalid = Can't be enabled
settings-extensions-state-unsupported = Needs code

settings-extensions-changed-note = Its files changed since you approved it, so it is off until you approve it again.

settings-extensions-unsupported-note = It runs code, which this version of Wormhole does not support yet.

# { $reason } is why the Probe can't be turned on, in English.
settings-extensions-invalid-reason = Can't be enabled: { $reason }

settings-extensions-approve-again = Approve again…

# Something went wrong. { $name } is the Probe's name and { $reason } says why, in English.
settings-extensions-not-approved = { $name } was not turned on: { $reason }
# As above, for turning a Probe off.
settings-extensions-not-turned-off = { $name } could not be turned off: { $reason }
# As above, for a Probe that could not be read for its sheet.
settings-extensions-cannot-approve = { $name } cannot be turned on: { $reason }

## The approval sheet

# { $name } is the Probe's name.
settings-extensions-sheet-title = Turn on { $name }
# As above, for a Probe that changed since it was approved.
settings-extensions-sheet-title-again = Approve { $name } again

settings-extensions-asks-for = It asks for
settings-extensions-asks-for-nothing = It asks for no permissions. It can only add what its files describe.

# One line each; a switch beside it leaves that permission out. { $program } is a program's name.
settings-extensions-perm-process = Run { $program }
# { $name } is the name of an environment variable (a token, a proxy).
settings-extensions-perm-env = Hand the secret { $name } to the programs it runs
# { $host } is a web address' host.
settings-extensions-perm-open = Open links on { $host }
settings-extensions-perm-notify = Show notifications
# { $agent } is a coding agent the Probe defines, whose hook entries it may write.
settings-extensions-perm-agents = Write the hook entries of its agent { $agent }
settings-extensions-perm-code = Run code inside Wormhole
# For the permissions a later version brings. { $key } names the permission and { $detail } says what.
settings-extensions-perm-other = { $key }: { $detail }

# The tag on a permission that is confirmed with Touch ID.
settings-extensions-touch-id = Touch ID

# Under a program. { $program } is its name.
settings-extensions-runs-anything = { $program } can run anything it is given.
# As above.
settings-extensions-runs-folder-code = { $program } runs code that the folder it starts in brings.
settings-extensions-read-only-git = Read-only git, run by Wormhole itself.
settings-extensions-own-script = A script from its own folder, exactly as approved.

# Under a program: the command lines that run it. Wording of the heading.
settings-extensions-runs-as = Runs as

settings-extensions-combinations = Dangerous together
# { $what } says what the pair can do, in English.
settings-extensions-combination = { $what }

settings-extensions-since = Since you approved it
settings-extensions-files-changed = Its files changed.
# { $what } is a permission, in English.
settings-extensions-now-asks = Now also asks for: { $what }
# As above.
settings-extensions-no-longer-asks = No longer asks for: { $what }

# { $count } is a whole number and { $fingerprint } the first digits of the fingerprint of its files.
settings-extensions-files-fingerprint = files: { $count }, fingerprint { $fingerprint }…

settings-extensions-confirm-touch-id = Approving needs Touch ID, or your password.
settings-extensions-confirm-click = Approving needs only a click.

settings-extensions-cancel = Cancel
settings-extensions-approve = Approve
settings-extensions-approve-touch-id = Approve with Touch ID…
settings-extensions-waiting = Waiting for approval…

# The reason the system's Touch ID dialog shows. { $name } is the Probe's name.
settings-extensions-auth-reason = turn on the Probe “{ $name }” and let it do what it asks

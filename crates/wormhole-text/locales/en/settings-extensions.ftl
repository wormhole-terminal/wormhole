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
# The state of a Probe whose stored approval was not made by this Wormhole (or was changed after): it is off.
settings-extensions-state-unsealed = Not approved by this Wormhole

settings-extensions-changed-note = Its files changed since you approved it, so it is off until you approve it again.

# Under a Probe in the state above. It can be approved again here, or the stored approval forgotten.
settings-extensions-unsealed-note = An approval is stored for it that this Wormhole did not make, or that was changed afterwards, so it is off. Approve it again, or forget the approval.

# A notice over the list: Wormhole could not read the key it checks approvals with, so every Probe is off.
# Nothing was deleted; scanning again asks the Keychain once more.
settings-extensions-key-unavailable = Wormhole couldn't read its key from the Keychain, so extensions stay off. Unlock the Keychain or allow access when it asks, then scan again.

settings-extensions-unsupported-note = It runs code, which this version of Wormhole does not support yet.

# The reason under a Probe that can't be turned on (its state already says so). { $reason } is in English.
settings-extensions-invalid-reason = { $reason }

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

settings-extensions-combinations = Dangerous together
# What a pair of permissions can do together; it follows "It". { $what } is the end of the sentence, in English.
settings-extensions-combination = It { $what }.

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

## Why something did not work (the detail after "was not turned on" and "cannot be turned on", and under a row)

settings-extensions-err-bad-name = The folder's name is not a Probe id.
settings-extensions-err-link = A Probe is a folder of its own here, not a link.

# { $id } is the id probe.toml gives the Probe.
settings-extensions-err-id-mismatch = Its probe.toml says its id is { $id }, not the folder's name.

# { $reason } is what reading the folder said, in English.
settings-extensions-err-read = { $reason }

settings-extensions-err-needs-code = It runs code, which this version of Wormhole does not support yet.

# { $reason } is what the system said, in English.
settings-extensions-err-copy = Its files could not be copied for it to run from: { $reason }
# As above, for the approvals file.
settings-extensions-err-store = The approval could not be saved: { $reason }

# { $reason } is what the system said, in English.
settings-extensions-err-key = The key the approval is sealed with could not be read from the Keychain: { $reason }

settings-extensions-err-unconfirmed = The yes was for another version of it. Open the sheet again.

# Under a row whose approved copy is not what was approved.
settings-extensions-copy-missing = Its approved copy is missing, so it is off until you approve it again.
# { $why } is what reading the copy said, in English.
settings-extensions-copy-unreadable = Its approved copy cannot be read ({ $why }), so it is off until you approve it again.
settings-extensions-copy-differs = Its approved copy is not what you approved, so it is off until you approve it again.

# The button on a row that is not on but still has an approval stored: it forgets it and deletes the copy.
settings-extensions-forget = Forget approval

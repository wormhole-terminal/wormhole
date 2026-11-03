# Settings window, Extensions page (Probes): the page's title and intro, its one row, the list of Probes and the
# sheet that asks before one is turned on. A Probe is a folder with a probe.toml that adds to Wormhole.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-extensions = Extensions
    .intro = Probes are folders that add to Wormhole. A new or changed one stays off until you approve exactly what it asks for.

settings-extensions-probes = Probes
    .description = Folders with a probe.toml in Wormhole's data folder, and what each is allowed to do
    .keywords = probe probes extension extensions plugin permissions approve touch id probe.toml

## The page (settings_window/sections/extensions.rs)

# The line of a folder that holds no readable Probe, where a Probe's own name and version would be.
settings-extensions-folder-hint = A folder in the Probes folder

settings-extensions-looking = Looking for Probes…

settings-extensions-none-title = No Probes yet

# { $count } is a whole number: the most entries the Probes folder may have.
settings-extensions-too-many = The Probes folder has more than { $count } entries, so none are listed. Move the ones you don't use out of it.

# Under a Probe's name. { $id } is its id, { $version } its version and { $author } who wrote it.
settings-extensions-identity = { $id } · version { $version } · by { $author }

settings-extensions-changed-note = Its files changed since you approved it, so it is off until you approve it again.

# Under a Probe in the state above. It can be approved again here, or the stored approval forgotten.
settings-extensions-unsealed-note = An approval is stored for it that this Wormhole did not make, or that was changed afterwards, so it is off. Approve it again, or forget the approval.

# A notice over the list: Wormhole could not read the key it checks approvals with, so every Probe is off.
# Nothing was deleted; scanning again asks the Keychain once more.
settings-extensions-key-unavailable = Wormhole couldn't read its key from the Keychain, so extensions stay off. Unlock the Keychain or allow access when it asks, then scan again.

# A notice over the list, and the error of an approval, in a development build started on the real data
# folder: it keeps no approval keys there, so every Probe is off. Not the Keychain's doing.
# WORMHOLE_DATA_DIR is an environment variable's name and stays as it is.
settings-extensions-dev-data-folder = A development build keeps no extension approvals in the real data folder, so extensions stay off. Start it with WORMHOLE_DATA_DIR set to a scratch folder.

settings-extensions-unsupported-note = It runs code, which this version of Wormhole does not support yet.

# The reason under a Probe that can't be turned on (its state already says so). { $reason } is in English.
settings-extensions-invalid-reason = { $reason }

## The approval sheet

# { $name } is the Probe's name.
settings-extensions-sheet-title = Turn on { $name }
# As above, for a Probe that changed since it was approved.
settings-extensions-sheet-title-again = Approve { $name } again

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

# What a pair of permissions can do together; it follows "It". { $what } is the end of the sentence, in English.
settings-extensions-combination = It { $what }.

settings-extensions-files-changed = Its files changed.
# { $what } is a permission, in English.
settings-extensions-now-asks = Now also asks for: { $what }
# As above.
settings-extensions-no-longer-asks = No longer asks for: { $what }

# The reason the system's Touch ID dialog shows. { $name } is the Probe's name.
settings-extensions-auth-reason = turn on the Probe “{ $name }” and let it do what it asks

## Why something did not work (the detail after "was not turned on" and "cannot be turned on", and under a row)

# Under a row whose approved copy is not what was approved.
settings-extensions-copy-missing = Its approved copy is missing, so it is off until you approve it again.
# { $why } is what reading the copy said, in English.
settings-extensions-copy-unreadable = Its approved copy cannot be read ({ $why }), so it is off until you approve it again.
settings-extensions-copy-differs = Its approved copy is not what you approved, so it is off until you approve it again.


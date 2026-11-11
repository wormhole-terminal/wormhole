# Settings window, updates page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-updates = Updates
    .intro = Signed releases, checked in the background.

## Updates

settings-updates-group-updates = Updates

settings-updates-version = Version
    .keywords = release build

settings-updates-auto-update = Update automatically
    .description = Checks in the background and installs on the next restart
    .keywords = download auto

settings-updates-update-channel = Channel
    .description = Unstable also gets a build of every change once it passes the tests, and shows the feature flags
    .keywords = beta nightly unstable stable prerelease dev builds main feature flags

settings-updates-update-status = Check for updates
    .keywords = install restart release notes check skip

## Privacy

settings-updates-group-privacy = Privacy
    .blurb = { -brand } has no analytics, telemetry or crash reports. The one request { -brand } itself makes is the update check, which goes to its download server and from there to GitHub.

settings-updates-check-details = What an update check sends
    .keywords = privacy data sent user agent ip address analytics telemetry tracking statistics count

## settings_window/sections/updates.rs (labels, hints and notes drawn by this page's code)

settings-updates-stable = Stable
settings-updates-unstable = Unstable
settings-updates-checked-just-now = Checked just now.
settings-updates-checked-a-minute-ago = Checked a minute ago.
settings-updates-checked-an-hour-ago = Checked an hour ago.
settings-updates-fixes-a-security-issue = Fixes a security issue.
settings-updates-development-build = Development build: updates only work in released builds.
settings-updates-not-checked-yet = Not checked yet.
settings-updates-checking = Checking…
settings-updates-wormhole-is-up-to-date = { -brand } is up to date.
settings-updates-downloading = Downloading…
settings-updates-skip-this-version = Skip this version
settings-updates-install-update = Install update
settings-updates-dont-restart = Don't restart
settings-updates-restart-now = Restart now
settings-updates-check-now = Check now

## settings_window/sections/updates.rs: what an update check sends

settings-updates-check-sends = Every update check
settings-updates-check-sends-what = The app’s name and version, and which update feed it asks for. Nothing else about this Mac.
settings-updates-check-server = The download server
settings-updates-check-server-what = Counts the requests it gets by app version, so we know which versions are in use. It stores no IP address and nothing that tells one Mac from another. GitHub, which serves the files, sees your IP address as with any download.
settings-updates-check-never = Never sent
settings-updates-check-never-what = Device IDs, account details, files and terminal contents.

## The status line: when it was last checked, what is available, what is installed

# Under "Check for updates" after a check. { $count } is a whole number of minutes (2 to 59), hours (up
# to 47) or days.
settings-updates-checked-minutes-ago = { $count ->
    [one] Checked { $count } minute ago.
   *[other] Checked { $count } minutes ago.
}
# The same with hours. { $count } is a whole number.
settings-updates-checked-hours-ago = { $count ->
    [one] Checked { $count } hour ago.
   *[other] Checked { $count } hours ago.
}
# The same with days. { $count } is a whole number.
settings-updates-checked-days-ago = { $count ->
    [one] Checked { $count } day ago.
   *[other] Checked { $count } days ago.
}

# After a check that found nothing new. { $checked } is when it was ("Checked 4 minutes ago.").
settings-updates-up-to-date-checked = { -brand } is up to date. { $checked }

# A release that fixes a security issue. { $version } is its number.
settings-updates-security-available = Security update available: { -brand } { $version }.

# Any other release. { $version } is its number.
settings-updates-version-available = Version { $version } is available.

# What a security update fixes, by the severity its feed gives it.
settings-updates-fixes-critical = Fixes a critical-severity security issue.
settings-updates-fixes-high = Fixes a high-severity security issue.
settings-updates-fixes-medium = Fixes a medium-severity security issue.
settings-updates-fixes-low = Fixes a low-severity security issue.

# After an update was downloaded and installed. { $version } is its number. The first is while the
# app waits for the terminals to be idle before it restarts by itself.
settings-updates-installed-waits = Version { $version } is installed. Restarts when your terminals are idle.
# The same once the app has no reason to wait. { $version } is its number.
settings-updates-installed-restart = Version { $version } is installed. Restart to use it.

# The heading over a release's notes. { $version } is its number.
settings-updates-whats-new = What's new in { $version }

## After the channel changed with a build of the old one in hand

# Going back to Stable while an Unstable build was downloading or being installed. { $version } is its number.
settings-updates-unstable-removed = The Unstable build { $version } was removed before it replaced anything. Stable offers its own releases.
# Going back to Stable after an Unstable build was installed; it cannot be taken off again. { $version } is its number.
settings-updates-unstable-stays = This Mac has the Unstable build { $version } installed. Stable takes over with the first Stable release newer than it.

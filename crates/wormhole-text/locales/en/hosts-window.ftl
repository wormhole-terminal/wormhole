# The Session Hosts window (Help → Session Hosts, shown while persistent terminals are on): what the
# processes that keep terminals running behind the app are doing. A debugging view, so it names things
# exactly: ids, process numbers, paths. The text it copies for a bug report is English and not here.

hosts-window-title = Session Hosts

# The header strip: this app, the folder the hosts live in, the feature's switch, the host program found.
# { $version } is the app's version, e.g. 1.6.2, and { $protocol } its protocol, e.g. 1 (minor 2).
hosts-header-app = Wormhole { $version }, protocol { $protocol }
hosts-header-folder = Hosts folder
hosts-header-flag = Persistent terminals
hosts-header-flag-on = On
hosts-header-flag-off = Off
hosts-header-binary = Host program
hosts-header-binary-missing = Not found beside the app

# The counts under the header.
hosts-count-attached = Attached
hosts-count-detached = Detached
hosts-count-orphan = Orphans
hosts-count-stale = Stale

# The empty list; { $folder } is the hosts folder.
hosts-empty = No session hosts in { $folder }.
hosts-loading = Looking for session hosts…
hosts-off-note = Persistent terminals are off, so new terminals start inside the app. Hosts left over from earlier are listed below.

# A host's state, as the badge on its card says it.
hosts-standing-attached = Attached
hosts-standing-detached = Detached
hosts-standing-orphan = Orphan
hosts-standing-stale = Stale

# Where an attached host's terminal is. { $window } is the window's number, { $title } the terminal's name.
hosts-standing-attached-to = Window { $window }, “{ $title }”
hosts-standing-detached-note = Running, no terminal of this app holds it
hosts-standing-orphan-note = Running, and no saved terminal names it

# Why a stale host's files are left over.
hosts-stale-unreadable = The meta file can’t be read
hosts-stale-pid-gone = Its process is gone
hosts-stale-socket-missing = No socket where the meta file says
hosts-stale-no-meta = A socket with no meta file

# The names of the facts on a card.
hosts-field-host-pid = Host process
hosts-field-shell-pid = Shell process
hosts-field-program = Running now
hosts-field-folder = Folder
hosts-field-started = Started
hosts-field-version = Host version
hosts-field-protocol = Protocol
hosts-field-answer = Answers
hosts-field-memory = Memory
hosts-field-link = Connection
hosts-field-socket = Socket
hosts-field-meta = Meta file

hosts-unknown = Unknown

# { $when } is a time like 2026-10-30 14:03:07 UTC, { $uptime } how long ago, like 3h 12m.
hosts-started = { $when } (up { $uptime })

# The host runs another version than this app. { $version } is the app's.
hosts-version-differs = This app is { $version }

# A protocol version: { $major } is the major, the one that must match, { $minor } the minor within it.
hosts-protocol-minor = { $major } (minor { $minor })

# The one major a host speaks: { $major }.
hosts-range-one = { $major }

# The majors a host speaks, from { $min } to { $max }.
hosts-range-span = { $min } to { $max }

# A host that is not running has no protocol to fit; { $range } is the majors its meta file says.
hosts-range-not-running = { $range }, not running

# How this app's protocol fits the host's. { $range } is the host's majors, like 1 or 1 to 2.
hosts-fit-compatible = { $range }, compatible
# { $range } is the host's majors, like 1 or 1 to 2.
hosts-fit-too-old = { $range }, too old for this app
# { $range } is the host's majors, like 1 or 1 to 2.
hosts-fit-too-new = { $range }, too new for this app

# What the host said when asked over its socket.
hosts-answer-not-asked = Not asked
# { $cols } and { $rows } are the terminal's size, { $lines } the lines of history the host keeps.
hosts-answer-running = Yes, program running, { $cols }×{ $rows }, { $lines } lines of history
hosts-answer-exited = Yes, but its program has ended
hosts-answer-incompatible = Yes, but it speaks no protocol this app does
# { $why } is the reason, e.g. connecting: Connection refused.
hosts-answer-none = No answer: { $why }

# This app's connection to an attached host: the stream asked for, then what went over it.
# { $received } and { $sent } are sizes like 12 KB.
hosts-link-sync = Repaint, then stream
hosts-link-raw = Raw stream
hosts-link-headless = Headless (not on screen)
# { $mode } is one of the three lines above; { $frames_in } and { $frames_out } count frames.
hosts-link-traffic = { $mode }; { $received } in, { $sent } out; { $frames_in } frames in, { $frames_out } out
hosts-link-lost = Connection lost
hosts-link-detached = Detached; the host keeps running
hosts-link-none = Connecting…

# The buttons on a card and in the header.
hosts-action-end = End
hosts-action-forget = Forget
hosts-action-copy = Copy details
hosts-action-copy-all = Copy all
hosts-action-copied = Copied

# The question before ending a host; { $id } is the host's id, { $title } the terminal's name.
hosts-end-title = End session host { $id }?
# { $title } is the terminal's name.
hosts-end-detail-attached = The program in the terminal “{ $title }” gets a hangup and the terminal ends.
hosts-end-detail = The program in it gets a hangup and ends.
hosts-end-confirm = End Host

# The status bar's item that says where the selected terminal runs: in a session host of its own, which
# keeps it running when Wormhole quits or updates, or inside the app. Shown only while persistent
# terminals are on. The word is the item's label; the tooltip is a sentence for a person who hovers it
# and is also what VoiceOver says.

# Status bar: the terminal runs in a session host and outlives the app.
status-bar-hosting-persistent = Persistent
    .tooltip = This terminal keeps running when Wormhole quits or updates.

# Status bar: the app is still connecting to the terminal's session host.
status-bar-hosting-connecting = Connecting…
    .tooltip = Wormhole is still connecting to this terminal's session host.

# Status bar: the terminal runs inside the app (no session host could be used, or it is a Welcome
# terminal), so it ends with the app.
status-bar-hosting-in-app = In app
    .tooltip = This terminal runs inside Wormhole and ends when Wormhole quits or updates.

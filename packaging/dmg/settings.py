# dmgbuild settings for the download image (scripts/make-dmg.sh runs `dmgbuild -s` on this file). dmgbuild
# writes the window layout into the image's .DS_Store itself, so no Finder, desktop session or permission
# is involved. Passed in with -D: app (the .app), background (the TIFF), icon (the volume icon, .icns).
# The window and icon positions are where packaging/dmg/make-background.swift's arrow expects them.
import os.path

app = defines["app"]  # noqa: F821  (dmgbuild provides `defines`)
name = os.path.basename(app)

format = "ULFO"
filesystem = "HFS+"
files = [app]
symlinks = {"Applications": "/Applications"}
icon = defines["icon"]  # noqa: F821
background = defines["background"]  # noqa: F821

show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
default_view = "icon-view"
# The frame of the window, title bar included, with its origin at the screen's bottom left: 400 pt of content,
# the size of the background picture.
window_rect = ((200, 350), (660, 428))
icon_size = 128
text_size = 13
icon_locations = {name: (165, 185), "Applications": (495, 185)}

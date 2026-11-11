#!/usr/bin/env python3
"""FAKE settings files of every terminal the Import page knows, in a scratch home (never a real one): fake_home.py <home>."""
import os, plistlib, sys

home = sys.argv[1]
def w(path, text, binary=False):
    full = os.path.join(home, path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    open(full, "wb" if binary else "w").write(text)

w(".config/ghostty/config", "font-family = Menlo\nfont-size = 17\nbackground-opacity = 0.92\nscrollback-limit = 20000000\ncursor-style = bar\n")
w(".config/alacritty/alacritty.toml", '[font]\nsize = 12.0\n[font.normal]\nfamily = "Menlo"\n[window]\nopacity = 0.9\n[scrolling]\nhistory = 5000\n')
w(".config/kitty/kitty.conf", "font_family Menlo\nfont_size 13.0\nscrollback_lines 3000\nbackground_opacity 0.85\nmacos_option_as_alt yes\n")
w(".config/wezterm/wezterm.lua", 'return {\n  font_size = 14.0,\n  scrollback_lines = 4000,\n  window_background_opacity = 0.9,\n  font = wezterm.font("Menlo"),\n}\n')
w(".warp/settings.toml", '[appearance.text]\nfont_name = "Menlo"\nfont_size = 14.0\nline_height_ratio = 1.3\n[terminal]\nmaximum_grid_size = 20000\ncopy_on_select = false\nuse_audible_bell = true\n')
w(".config/zed/settings.json", '{"terminal": {"font_size": 15, "font_family": "Menlo", "line_height": {"custom": 1.4}}, "buffer_font_size": 14}\n')
w("Library/Application Support/Code/User/settings.json", '{"terminal.integrated.fontSize": 13, "terminal.integrated.fontFamily": "Menlo", "terminal.integrated.scrollback": 8000}\n')
iterm = {"Default Bookmark Guid": "G1", "New Bookmarks": [{"Guid": "G1", "Name": "Default", "Normal Font": "Menlo-Regular 14", "Scrollback Lines": 6000, "Transparency": 0.1}]}
w("Library/Preferences/com.googlecode.iterm2.plist", plistlib.dumps(iterm, fmt=plistlib.FMT_BINARY), binary=True)

def archive(root, others):
    objs = ["$null", root] + others
    return plistlib.dumps({"$archiver": "NSKeyedArchiver", "$objects": objs, "$top": {"root": plistlib.UID(1)}, "$version": 100000}, fmt=plistlib.FMT_BINARY)
font = archive({"NSName": plistlib.UID(2), "NSSize": 15.0, "NSfFlags": 16}, ["Menlo-Regular"])
term = {"Default Window Settings": "Basic", "Window Settings": {"Basic": {"Font": font, "shellExitAction": 1}}}
w("Library/Preferences/com.apple.Terminal.plist", plistlib.dumps(term, fmt=plistlib.FMT_BINARY), binary=True)
print("fake home ready:", home)

#!/usr/bin/env python3
"""Cut the three motion files into folders (phase M0 of the motion plan), one pure move per step.

  scripts/motion-moves.py motion      style/motion.rs  -> style/motion/{mod,timing,easing}.rs
  scripts/motion-moves.py tile        style/tile.rs    -> style/tile/{mod,face,orbit,edge,badge}.rs
  scripts/motion-moves.py urgency     style/tile/mod.rs -> style/status.rs (the Urgency conversion, beside StatusGlyph)
  scripts/motion-moves.py sidebar     sidebar/motion.rs -> sidebar/motion/{mod,list}.rs
  scripts/motion-moves.py rows        sidebar/rows.rs -> sidebar/row/{mod,frame,face,tile,second_line,label}.rs
  scripts/motion-moves.py STEP --check [--base REV]
                                      verify that the working tree is a pure move of the files as REV had them
                                      (default HEAD: before the commit; after it, pass the commit's parent)

Each step is one commit and changes no behaviour: the file is `git mv`ed to its new home and its top-level
items (a doc comment, attributes and the item, as the file wrote them) go to the files listed below, in the
order they had, verbatim. What the script adds is only what a move needs: `mod` lines, `use` lines (the old
file's imports, re-split per file), and a wider visibility (`pub(super)`, or `pub(in crate::sidebar)`) on the
items another file now calls across the new boundary. Every such change is listed in the step's spec below.

A step that has run already finds its source gone and does nothing. It does not run cargo: after it run
`cargo fmt --all` (it sorts the `mod` lines and imports), then the gate. `--check` fails when an item
differs, token for token, between the files at REV and the files now (a widened visibility, a rewrapped
signature and `use`/`mod` lines are not differences).
Python 3.9, standard library only.
"""
from __future__ import annotations

import argparse
import collections
import os
import re
import subprocess
import sys
from typing import Dict, List, NamedTuple, Optional, Tuple, Tuple

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
APP = "crates/wormhole/src"

ITEM_RE = re.compile(r"^(?:pub(?:\([^)]*\))?\s+)?(const|fn|struct|enum|static|type|mod|trait)\s+(\w+)")


class Chunk(NamedTuple):
    key: str  # "fn orbit", "const BADGE", "impl From<StatusGlyph> for Urgency", "mod tests"
    text: List[str]  # its lines, without the blank lines after it
    gap: int  # blank lines that followed it in the source


def run(*args: str) -> str:
    return subprocess.run(args, cwd=REPO, check=True, capture_output=True, text=True).stdout


def key_of(line: str) -> Optional[str]:
    if line.startswith("impl"):
        return line.rstrip(" {")
    m = ITEM_RE.match(line)
    return "%s %s" % (m.group(1), m.group(2)) if m else None


def chunks_of(source: str) -> List[Chunk]:
    """The file's top-level items. A chunk starts at the first doc or attribute line of a column-0 run and
    lasts to the line before the next one; `use` lines and the `//!` header are not chunks."""
    lines = source.split("\n")
    starts: List[int] = []
    prev = ""
    for at, line in enumerate(lines):
        if line and not line[0].isspace() and line[0] not in "})]":
            continuing = prev.startswith(("///", "#[")) and not prev[0].isspace()
            if not continuing and not line.startswith(("//!", "use ", "pub use ", "pub(super) use ")):
                starts.append(at)
        prev = line
    out: List[Chunk] = []
    for n, at in enumerate(starts):
        end = starts[n + 1] if n + 1 < len(starts) else len(lines)
        block = lines[at:end]
        gap = 0
        while block and block[-1] == "":
            block.pop()
            gap += 1
        item = next((l for l in block if not l.startswith(("///", "#["))), "")
        key = key_of(item)
        if key is None:
            raise SystemExit("cannot name the item at line %d: %s" % (at + 1, item))
        out.append(Chunk(key, block, gap))
    return out


class File(NamedTuple):
    items: List[str]  # chunk keys, in source order
    uses: List[str]  # `use` lines (and `mod` lines) this file starts with, after its header
    header: str  # "" for none, "source" for the source's own `//!` lines, or the text itself
    widen: Dict[str, str] = {}  # chunk key -> visibility prefix to put on its item line
    edits: Dict[str, List[Tuple[str, str]]] = {}  # chunk key -> (line, replacement): a field widened, say
    edits: Dict[str, List[Tuple[str, str]]] = {}  # chunk key -> (line, replacement): a field widened, say


class Step(NamedTuple):
    src: str  # relative to the app crate's src/
    first: str  # where `git mv` puts it
    files: Dict[str, File]  # path (relative to the folder of `first`) -> contents
    rewrite: Dict[str, str] = {}  # for a file that is only renamed: exact line replacements
    widen_all: str = ""  # a visibility that replaces every `pub(super)` of the new files, if they are in a folder
    callers: Dict[str, Dict[str, str]] = {}  # other files (relative to src/) -> exact replacements for the new path


class Relocate(NamedTuple):
    """Items that move from one existing file to another."""

    src: str  # relative to the app crate's src/
    dst: str
    items: List[str]  # chunk keys
    after: str  # the chunk of `dst` they go behind
    dst_use: str  # a `use` line `dst` needs, put after `dst_use_after`
    dst_use_after: str
    drop_use: str  # the `use` line `src` no longer needs


TILE_USES_MOD = [
    "mod badge;",
    "mod edge;",
    "mod face;",
    "mod orbit;",
    "mod ring;",
    "",
    "use gpui_kit::prelude::*;",
    "use gpui_kit::*;",
    "use wormhole_core::agents::AgentId;",
    "use wormhole_core::view::sidebar::Urgency;",
    "",
    "use self::badge::badge;",
    "use self::edge::breathing_edge;",
    "use self::face::{Paint, Resolved, resolve};",
    "use self::orbit::orbit;",
    "use super::scale::{TILE, TILE_MINI};",
    "use super::status::StatusGlyph;",
    "use crate::icons::glyph;",
    "use crate::settings::UiColors;",
]

ROW_USES_MOD = [
    "mod face;",
    "mod frame;",
    "mod label;",
    "mod second_line;",
    "mod tile;",
    "",
    "use std::time::Duration;",
    "",
    "use gpui_kit::component::tooltip::Tooltip;",
    "use gpui_kit::prelude::*;",
    "use gpui_kit::*;",
    "use wormhole_core::view::sidebar::hints::keyshortcut;",
    "",
    "use self::face::RowFace;",
    "pub(super) use self::face::{PLACE_CHARS, Place, row_face};",
    "pub(super) use self::frame::Frame;",
    "pub(super) use self::second_line::time_text;",
    "pub(super) use self::tile::{glyph_for, tile_for, tile_parts};",
    "#[cfg(test)]",
    "use self::label::row_label;",
    "#[cfg(test)]",
    "use self::second_line::{SecondLine, second_line};",
    "",
    "use super::facts::RowInfo;",
    "use super::{Sidebar, SidebarEvent, two_line_tooltip};",
    "use crate::settings::UiColors;",
    "use crate::sidebar::SidebarStop;",
    "use crate::sidebar::terminal_drag::DragOrigin;",
    "use crate::style;",
]
ROW_USES_FACE = [
    "use gpui_kit::assets::IconName;",
    "use gpui_kit::component::{Icon, Sizable};",
    "use gpui_kit::prelude::*;",
    "use gpui_kit::*;",
    "use wormhole_core::view::sidebar::SplitMarks;",
    "use wormhole_core::view::sidebar::hints::keycap;",
    "use wormhole_core::view::sidebar::wording::place_detail;",
    "",
    "use super::label::row_label;",
    "use super::second_line::{SecondLine, second_line};",
    "use super::tile::tile_for;",
    "use super::Frame;",
    "use super::super::{BODY_REM, CAPTION_REM, two_line_tooltip};",
    "use crate::sidebar::facts::RowInfo;",
    "use crate::style::home_dir;",
    "use crate::style::{self, SessionRow, TextRole};",
]

RELOCATIONS: Dict[str, Relocate] = {
    "urgency": Relocate(
        src="style/tile/mod.rs",
        dst="style/status.rs",
        items=["impl From<StatusGlyph> for Urgency"],
        after="impl StatusGlyph",
        dst_use="use wormhole_core::view::sidebar::Urgency;",
        dst_use_after="use wormhole_core::notify::{State, Terminal};",
        drop_use="use wormhole_core::view::sidebar::Urgency;",
    ),
}

STEPS: Dict[str, Step] = {
    "motion": Step(
        src="style/motion.rs",
        first="style/motion/mod.rs",
        files={
            "mod.rs": File(items=[], uses=["mod easing;", "mod timing;", "", "pub use easing::*;", "pub use timing::*;"],
                           header="source"),
            "timing.rs": File(
                items=[
                    "const MOTION_INSTANT", "const MOTION_EXIT_FAST", "const MOTION_FAST", "const MOTION_POPOVER",
                    "const MOTION_BASE", "const MOTION_MOVE", "const MOTION_PANEL_EXIT", "const MOTION_PANEL",
                    "const SPIN_PERIOD", "const TILE_ORBIT", "const TILE_BREATHE", "const TILE_BREATHE_LEAST",
                    "const STATUS_PULSE", "const PULSE_RANGE", "const ANIMATION_FPS", "const TOOLTIP_DELAY",
                    "const HINT_HOLD", "const RESIZE_HANDLE_HOVER_DELAY", "const PATH_CARD_OPEN_DELAY",
                    "const PATH_CARD_CLOSE_DELAY", "const PATH_CARD_COPIED_FOR",
                ],
                uses=["use std::time::Duration;"],
                header="//! How long things take: durations, the loops' periods and the delays before something shows.\n"
                       "//! The curves they run along are in [`super::easing`].",
            ),
            "easing.rs": File(
                items=[
                    "struct CubicBezier", "impl CubicBezier", "const EASE_OUT", "const EASE_IN", "const EASE_IN_OUT",
                    "const LINEAR", "mod tests",
                ],
                uses=[],
                header="//! How a motion eases: CSS-style cubic-bezier curves as GPUI easing functions.",
            ),
        },
    ),
    "tile": Step(
        src="style/tile.rs",
        first="style/tile/mod.rs",
        files={
            "mod.rs": File(
                items=[
                    "enum TileFace", "struct StatusTile", "fn status_tile", "fn status_tile_at", "fn status_tile_on",
                    "impl From<StatusGlyph> for Urgency", "const SHEEN_REACH", "const SQUIRCLE",
                    "const FACE_SHARE", "const MONOGRAM_SHARE",
                ],
                uses=TILE_USES_MOD,
                header="source",
            ),
            "ring.rs": File(
                items=[
                    "const RING_OUT", "const ORBIT_STROKE", "const ORBIT_STROKE_MINI", "const LINE_THIN_BELOW",
                    "fn line",
                ],
                uses=[],
                header="//! What the light and the amber edge share: how far they sit outside the tile and how thick\n"
                       "//! their line is for a tile of a size.",
                widen={"const RING_OUT": "pub(super)", "const LINE_THIN_BELOW": "pub(super)", "fn line": "pub(super)"},
            ),
            "face.rs": File(
                items=["enum Paint", "enum Resolved", "fn resolve"],
                uses=[
                    "use gpui_kit::*;",
                    "use wormhole_core::agents::{self, TileColor};",
                    "",
                    "use super::TileFace;",
                    "use crate::settings::UiColors;",
                ],
                header="//! What the tile is filled with and what it shows inside: the face resolved against the agent\n"
                       "//! registry, every colour a token. Must not draw anything itself.",
                widen={"enum Paint": "pub(super)", "enum Resolved": "pub(super)", "fn resolve": "pub(super)"},
            ),
            "orbit.rs": File(
                items=[
                    "const ORBIT_SPAN", "const ORBIT_STEPS", "const ORBIT_TRACK", "fn orbit", "fn ring_canvas",
                    "const RING_SAMPLES", "fn perimeter_point", "mod tests",
                ],
                uses=[
                    "use std::f32::consts::PI;",
                    "",
                    "use gpui_kit::prelude::*;",
                    "use gpui_kit::*;",
                    "",
                    "use super::ring::{RING_OUT, line};",
                    "use crate::style::motion::{ANIMATION_FPS, TILE_ORBIT};",
                ],
                header="//! The light that circles a working terminal's tile: a line of a few steps fading behind a head,\n"
                       "//! over a faint track, along the rounded rectangle's outside edge.",
                widen={"fn orbit": "pub(super)"},
            ),
            "edge.rs": File(
                items=["fn breathing_edge"],
                uses=[
                    "use gpui_kit::prelude::*;",
                    "use gpui_kit::*;",
                    "",
                    "use super::ring::{RING_OUT, line};",
                    "use crate::settings::UiColors;",
                    "use crate::style::motion::{ANIMATION_FPS, TILE_BREATHE, TILE_BREATHE_LEAST};",
                ],
                header="//! The amber edge of a tile whose agent needs input, breathing between two strengths.",
                widen={"fn breathing_edge": "pub(super)"},
            ),
            "badge.rs": File(
                items=[
                    "const BADGE", "const BADGE_MINI", "const BADGE_RIM", "const BADGE_OUT", "fn badge",
                    "fn badge_mark",
                ],
                uses=[
                    "use gpui_kit::prelude::*;",
                    "use gpui_kit::*;",
                    "",
                    "use super::ring::LINE_THIN_BELOW;",
                    "use crate::icons::glyph;",
                    "use crate::settings::UiColors;",
                    "use crate::style::scale::TextRole;",
                    "use crate::style::status::StatusGlyph;",
                ],
                header="//! The badge in a tile's corner for the states its edge doesn't tell: needs input, finished,\n"
                       "//! failed, the bell and new output.",
                widen={"fn badge": "pub(super)"},
            ),
        },
    ),
    "sidebar": Step(
        src="sidebar/motion.rs",
        first="sidebar/motion/list.rs",
        files={
            "mod.rs": File(
                items=[],
                uses=["mod list;", "", "pub(super) use list::{ItemKey, ListMotion};"],
                header="//! How the terminal list moves: today only the slide of rows that changed place ([`list`]).\n"
                       "//! Each motion is a file of its own beside it; the sidebar reaches them through this module.",
            ),
        },
        rewrite={
            "use super::Sidebar;": "use crate::sidebar::Sidebar;",
        },
    ),
    "rows": Step(
        src="sidebar/rows.rs",
        first="sidebar/row/mod.rs",
        files={
            "mod.rs": File(
                items=["const MOVE_FLASH", "const CLOSE_X_TOP", "fn close_right", "impl Sidebar", "mod tests"],
                uses=ROW_USES_MOD,
                header="source",
            ),
            "frame.rs": File(
                items=["const NAME_FADE", "const FIT_SLACK", "struct Frame", "impl Frame"],
                uses=[
                    "use gpui_kit::*;",
                    "",
                    "use super::super::input::SidebarInput;",
                    "use crate::settings::{UiColors, settings_in, ui_in};",
                    "use crate::style::{self, Metrics};",
                ],
                header="//! What every row of one frame shares: the settings it is drawn by and how text is measured, for a\n"
                       "//! name that is cut off to fade out and a tooltip to know when it is needed.",
                edits={"struct Frame": [("    show_dot: bool,", "    pub(in crate::sidebar) show_dot: bool,"),
                                        ("    show_notes: bool,", "    pub(in crate::sidebar) show_notes: bool,")]},
            ),
            "face.rs": File(
                items=[
                    "const TRAIL_W", "const KEYCAP_TRAIL_W", "const DETAIL_W", "const MARK_W", "const LINE_GAP",
                    "const MONO_CHAR_W", "const CLOCK_W", "const PLACE_CHARS", "fn split_line_x", "struct Place",
                    "struct RowFace", "fn row_face",
                ],
                uses=ROW_USES_FACE,
                edits={"struct RowFace": [
                    ("    tooltip: Option<(SharedString, SharedString)>,", "    pub(super) tooltip: Option<(SharedString, SharedString)>,"),
                    ("    label: String,", "    pub(super) label: String,"),
                    ("    group: SharedString,", "    pub(super) group: SharedString,"),
                ]},
                header="//! A row as it looks, before anything answers to it: where it sits in the listing (`Place`), what it\n"
                       "//! shows and what its tooltip and VoiceOver say (`RowFace`). Must not wire anything (click, drag, menu).",
            ),
            "tile.rs": File(
                items=["fn tile_for", "fn tile_parts", "fn glyph_for"],
                uses=[
                    "use gpui_kit::*;",
                    "use wormhole_core::view::sidebar::RowState;",
                    "",
                    "use super::Frame;",
                    "use super::super::facts::RowInfo;",
                    "use crate::style::{self, StatusGlyph, StatusTile, TileFace};",
                ],
                header="//! A terminal's status tile and the glyph its state is drawn with: its agent's logo or the terminal\n"
                       "//! symbol, and its state round it; the bell is a badge on a terminal with nothing louder to say.",
            ),
            "second_line.rs": File(
                items=["struct SecondLine", "fn second_line", "fn time_text", "fn tone_of"],
                uses=[
                    "use wormhole_core::view::sidebar::RowState;",
                    "use wormhole_core::view::sidebar::wording::{agent_detail, ago, clock, took};",
                    "",
                    "use super::super::facts::RowInfo;",
                    "use crate::style::RowTone;",
                ],
                header="//! A row's second line: what the terminal has to report, or where it is, and the part of it that moves\n"
                       "//! with time (so a tick knows when to draw again).",
            ),
            "label.rs": File(
                items=["fn row_label"],
                uses=[
                    "use wormhole_core::view::sidebar::RowState;",
                    "use wormhole_core::view::sidebar::spoken::{self, RowSpeech, SplitSpeech};",
                    "",
                    "use super::Place;",
                    "use super::super::facts::RowInfo;",
                ],
                header="//! What VoiceOver says of a row: its name, what it is doing, its split and its place in the list.",
                widen={"fn row_label": "pub(super)"},
            ),
        },
        widen_all="pub(in crate::sidebar)",
        callers={
            "sidebar/mod.rs": {"mod rows;": "mod row;", "rows::Frame::new": "row::Frame::new", "rows::Place {": "row::Place {"},
            "sidebar/switcher.rs": {"super::rows::": "super::row::"},
            "sidebar/drag.rs": {"super::rows::": "super::row::"},
            "sidebar/groups.rs": {"super::rows::": "super::row::"},
            "sidebar/tab_bar.rs": {"super::rows::": "super::row::"},
            "sidebar/facts.rs": {"use super::rows;": "use super::row;", "rows::time_text": "row::time_text"},
            "sidebar/demo.rs": {"super::rows::Frame": "super::row::Frame", "`rows::glyph_for`": "`row::glyph_for`"},
            "sidebar/group_head.rs": {"super::rows::": "super::row::"},
            "sidebar/motion/leaving.rs": {"crate::sidebar::rows::": "crate::sidebar::row::"},
        },
    ),
}


def vis_widen(step_name: str) -> str:
    return "pub(in crate::sidebar)" if step_name == "sidebar" else "pub(super)"


def header_lines(source: str) -> List[str]:
    return [l for l in source.split("\n") if l.startswith("//!")]


def render(step_name: str, step: Step, name: str, spec: File, source: str, chunks: Dict[str, Chunk]) -> str:
    out: List[str] = []
    if spec.header == "source":
        out += header_lines(source)
    elif spec.header:
        out += spec.header.split("\n")
    if out:
        out.append("")
    if spec.uses:
        out += spec.uses
        out.append("")
    prev: Optional[Chunk] = None
    for at, key in enumerate(spec.items):
        chunk = chunks[key]
        text = list(chunk.text)
        if step.widen_all:
            text = [l.replace("pub(super)", step.widen_all) for l in text]
        for old, new in spec.edits.get(key, []):
            if old not in text:
                raise SystemExit("%s: %s has no line %r" % (step_name, key, old))
            text = [new if l == old else l for l in text]
        if key in spec.widen:
            first_item = next(i for i, l in enumerate(text) if not l.startswith(("///", "#[")))
            line = text[first_item]
            bare = re.sub(r"^pub(\([^)]*\))?\s+", "", line)
            text[first_item] = "%s %s" % (spec.widen[key], bare)
        if prev is not None:
            out += [""] * (prev.gap if chunk.key == next_in_source(chunks, prev.key) else 1)
        out += text
        prev = chunk
    return "\n".join(out).rstrip("\n") + "\n"


def next_in_source(chunks: Dict[str, Chunk], key: str) -> Optional[str]:
    keys = list(chunks)
    at = keys.index(key)
    return keys[at + 1] if at + 1 < len(keys) else None


def apply_step(name: str) -> int:
    step = STEPS[name]
    src = os.path.join(REPO, APP, step.src)
    if not os.path.exists(src):
        print("%s: %s is gone already, nothing to do" % (name, step.src))
        return 0
    with open(src, encoding="utf-8") as f:
        source = f.read()
    first = os.path.join(REPO, APP, step.first)
    os.makedirs(os.path.dirname(first), exist_ok=True)
    subprocess.run(["git", "mv", src, first], cwd=REPO, check=True)
    if step.rewrite:
        text = source
        for old, new in step.rewrite.items():
            if old not in text:
                raise SystemExit("%s: line to rewrite not found: %s" % (name, old))
            text = text.replace(old, new)
        if name == "sidebar":
            text = text.replace("pub(super)", vis_widen(name))
        with open(first, "w", encoding="utf-8") as f:
            f.write(text)
    chunk_list = chunks_of(source)
    chunks = {c.key: c for c in chunk_list}
    if len(chunks) != len(chunk_list):
        raise SystemExit("%s: two items share a name" % name)
    wanted = [k for spec in step.files.values() for k in spec.items]
    if step.files and wanted:
        missing = [k for k in wanted if k not in chunks]
        left = [k for k in chunks if k not in wanted]
        dup = [k for k, n in collections.Counter(wanted).items() if n > 1]
        if missing or left or dup:
            raise SystemExit("%s: missing %s, unplaced %s, placed twice %s" % (name, missing, left, dup))
    folder = os.path.dirname(first)
    for rel, spec in step.files.items():
        path = os.path.join(folder, rel)
        with open(path, "w", encoding="utf-8") as f:
            f.write(render(name, step, rel, spec, source, chunks))
        subprocess.run(["git", "add", path], cwd=REPO, check=True)
        print("%s: wrote %s" % (name, os.path.relpath(path, REPO)))
    subprocess.run(["git", "add", first], cwd=REPO, check=True)
    for rel, edits in step.callers.items():
        path = os.path.join(REPO, APP, rel)
        with open(path, encoding="utf-8") as f:
            text = f.read()
        for old, new in edits.items():
            if old not in text:
                raise SystemExit("%s: %s has no %s" % (name, rel, old))
            text = text.replace(old, new)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        subprocess.run(["git", "add", path], cwd=REPO, check=True)
    return 0


def apply_relocation(name: str) -> int:
    move = RELOCATIONS[name]
    src, dst = (os.path.join(REPO, APP, p) for p in (move.src, move.dst))
    with open(src, encoding="utf-8") as f:
        source = f.read()
    with open(dst, encoding="utf-8") as f:
        target = f.read()
    chunks = {c.key: c for c in chunks_of(source)}
    if not all(key in chunks for key in move.items):
        print("%s: %s has no %s any more, nothing to do" % (name, move.src, move.items))
        return 0
    anchor = chunks_of(target)
    keys = [c.key for c in anchor]
    if move.after not in keys:
        raise SystemExit("%s: %s has no %s" % (name, move.dst, move.after))
    after = anchor[keys.index(move.after)]
    # The moved items go behind `after`: after its last line, with a blank line between.
    lines = target.split("\n")
    last = next(i for i in range(len(lines)) if lines[i:i + len(after.text)] == after.text) + len(after.text)
    moved: List[str] = []
    for key in move.items:
        moved += [""] + chunks[key].text
    lines[last:last] = moved
    text = "\n".join(lines).replace(move.dst_use_after, move.dst_use_after + "\n" + move.dst_use, 1)
    with open(dst, "w", encoding="utf-8") as f:
        f.write(text)
    # Out of the source: the items, the blank lines they leave, and the import they were the last user of.
    for key in move.items:
        chunk = chunks[key]
        block = "\n".join(chunk.text) + "\n" + "\n" * chunk.gap
        if block not in source:
            raise SystemExit("%s: cannot find %s in %s" % (name, key, move.src))
        source = source.replace(block, "", 1)
    source = source.replace(move.drop_use + "\n", "", 1)
    with open(src, "w", encoding="utf-8") as f:
        f.write(source)
    subprocess.run(["git", "add", src, dst], cwd=REPO, check=True)
    print("%s: moved %s from %s to %s" % (name, ", ".join(move.items), move.src, move.dst))
    return 0


def norm(chunk: Chunk) -> str:
    """An item without whitespace and without a widened visibility: what a move must keep, token for token
    (rustfmt may rewrap a signature the wider visibility made too long)."""
    text = re.sub(r"\s+", "", "\n".join(chunk.text))
    text = re.sub(r"pub\(super\)|pub\(incrate::sidebar\)", "", text)
    return re.sub(r",([)\]}])", r"\1", text)  # a rewrapped parameter list gains a trailing comma


def items_of(text: str) -> List[Chunk]:
    """The items of a file, without its `mod x;` declarations."""
    return [c for c in chunks_of(text) if not (c.key.startswith("mod ") and c.text[-1].endswith(";"))]


def check_step(name: str, base: str) -> int:
    """The items of the files as `base` had them are the items of the files now, whole and nowhere else."""
    if name in RELOCATIONS:
        move = RELOCATIONS[name]
        before_paths = after_paths = [move.src, move.dst]
    else:
        step = STEPS[name]
        before_paths = [step.src]
        folder = os.path.join(REPO, APP, os.path.dirname(step.first))
        after_paths = [os.path.relpath(os.path.join(folder, f), os.path.join(REPO, APP)) for f in sorted(os.listdir(folder))]
    old: List[Chunk] = []
    for path in before_paths:
        try:
            old += items_of(run("git", "show", "%s:%s/%s" % (base, APP, path)))
        except subprocess.CalledProcessError:
            print("%s: %s is not in %s; pass --base <the commit before the move>" % (name, path, base))
            return 2
    new: List[Chunk] = []
    for path in after_paths:
        with open(os.path.join(REPO, APP, path), encoding="utf-8") as f:
            new += items_of(f.read())
    a = collections.Counter(norm(c) for c in old)
    b = collections.Counter(norm(c) for c in new)
    for text in sorted((a - b).elements()):
        print("only before:", text[:100])
    for text in sorted((b - a).elements()):
        print("only now:", text[:100])
    print("%s: %s" % (name, "a pure move" if a == b else "NOT a pure move"))
    return 0 if a == b else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("step", choices=sorted([*STEPS, *RELOCATIONS]))
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--base", default="HEAD", help="the revision the files are compared with (default HEAD)")
    args = parser.parse_args()
    if args.check:
        return check_step(args.step, args.base)
    return apply_relocation(args.step) if args.step in RELOCATIONS else apply_step(args.step)


if __name__ == "__main__":
    sys.exit(main())

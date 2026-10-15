#!/usr/bin/env python3
"""Regenerate the terminal engine's Unicode tables from the Unicode Character Database.

  scripts/gen-unicode.py           download what is missing, write the files
  scripts/gen-unicode.py --check   exit 1 if the files are out of date (writes nothing)

Writes
  crates/wormhole-vt/src/unicode/tables.rs         one byte per code point (cell width, grapheme
                                                   cluster class, emoji variation base) in a
                                                   three-stage table, plus the lookup function
  crates/wormhole-vt/tests/data/GraphemeBreakTest.txt   Unicode's own test for the grapheme rules,
                                                   copied unchanged

The UCD files are cached outside the repository (target/ucd/<version>/, or --cache DIR, or
$WORMHOLE_UCD_CACHE); delete the folder to download them again. The output is deterministic: running
the script twice gives the same files. To move to a new Unicode version change UNICODE_VERSION and
run it; the checks on the files' headers and on the properties' overlaps stop it when a rule of
the algorithm needs a second look.

Needs only Python 3 (standard library) and a network for the first run.
"""

import argparse
import os
import sys
import tempfile
import urllib.request

UNICODE_VERSION = "18.0.0"
BASE_URL = "https://www.unicode.org/Public/%s/ucd/"
MAX_FILE_BYTES = 8 * 1024 * 1024

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
TABLES_OUT = os.path.join(ROOT, "crates", "wormhole-vt", "src", "unicode", "tables.rs")
TEST_OUT = os.path.join(ROOT, "crates", "wormhole-vt", "tests", "data", "GraphemeBreakTest.txt")

# name -> path below the version's ucd/ folder
FILES = {
    "eaw": "EastAsianWidth.txt",
    "gc": "extracted/DerivedGeneralCategory.txt",
    "emoji": "emoji/emoji-data.txt",
    "variation": "emoji/emoji-variation-sequences.txt",
    "gcb": "auxiliary/GraphemeBreakProperty.txt",
    "gbtest": "auxiliary/GraphemeBreakTest.txt",
    "derived": "DerivedCoreProperties.txt",
    "props": "PropList.txt",
    "indic": "IndicSyllabicCategory.txt",
}

MAX_CP = 0x10FFFF

# The grapheme cluster classes, in the order the engine's rules rely on: the control group, the
# extenders and the Hangul jamo are contiguous ranges. GCB values with InCB or Extended_Pictographic
# folded in, because those never overlap (checked below), so one number says all the rules need.
CLASSES = [
    "OTHER",
    "CR",
    "LF",
    "CONTROL",
    "EXTEND",  # Grapheme_Cluster_Break=Extend, InCB=None
    "EXTEND_INCB",  # Extend with InCB=Extend: a combining mark inside a conjunct
    "LINKER",  # Extend with InCB=Linker: a virama
    "ZWJ",
    "SPACING_MARK",
    "PREPEND",
    "REGIONAL_INDICATOR",
    "L",
    "V",
    "T",
    "LV",
    "LVT",
    "EXTENDED_PICTOGRAPHIC",
    "CONSONANT",  # InCB=Consonant
    "LINKER_OTHER",  # InCB=Linker without Grapheme_Cluster_Break=Extend (Unicode 18: U+1CF5, U+1CF6, U+11A3A)
]
CLASS = {name: index for index, name in enumerate(CLASSES)}
START = len(CLASSES)  # the state before the first character; never in a table

GCB_NAMES = {
    "CR": "CR",
    "LF": "LF",
    "Control": "CONTROL",
    "Extend": "EXTEND",
    "ZWJ": "ZWJ",
    "Regional_Indicator": "REGIONAL_INDICATOR",
    "Prepend": "PREPEND",
    "SpacingMark": "SPACING_MARK",
    "L": "L",
    "V": "V",
    "T": "T",
    "LV": "LV",
    "LVT": "LVT",
}

# Byte layout of a code point's properties.
WIDTH_MASK = 0b11
VS_BASE = 0b100
CLASS_SHIFT = 3

# Hangul medial vowels and final consonants take no cell of their own: a syllable is drawn from
# its initial consonant (U+1100..U+115F, A960..A97C, wide), and the rest only add to it.
ZERO_WIDTH_RANGES = [(0x1160, 0x11FF), (0xD7B0, 0xD7FF)]
SOFT_HYPHEN = 0x00AD


def fail(message):
    sys.exit("gen-unicode: " + message)


# ---- downloading ----------------------------------------------------------------------------


def cache_dir(args):
    base = args.cache or os.environ.get("WORMHOLE_UCD_CACHE") or os.path.join(ROOT, "target", "ucd")
    return os.path.join(base, UNICODE_VERSION)


def fetch(name, directory):
    """The text of a UCD file, downloaded once into the cache."""
    path = os.path.join(directory, FILES[name].replace("/", "_"))
    if not os.path.exists(path):
        url = (BASE_URL % UNICODE_VERSION) + FILES[name]
        print("downloading " + url, file=sys.stderr)
        with urllib.request.urlopen(url, timeout=60) as response:
            data = response.read(MAX_FILE_BYTES + 1)
        if len(data) > MAX_FILE_BYTES:
            fail(url + " is larger than expected")
        os.makedirs(directory, exist_ok=True)
        handle, temp = tempfile.mkstemp(dir=directory)
        with os.fdopen(handle, "wb") as out:
            out.write(data)
        os.replace(temp, path)
    with open(path, "rb") as handle:
        data = handle.read()
    text = data.decode("utf-8")
    # The UCD files say their version in the file name on line 1, the emoji files as "Version: 17.0".
    header = "\n".join(text.split("\n", 12)[:12])
    short = UNICODE_VERSION.rsplit(".", 1)[0]
    if UNICODE_VERSION not in header and ("Version: " + short) not in header:
        fail("%s: expected a Unicode %s file, its header is %r" % (FILES[name], UNICODE_VERSION, header[:80]))
    return text, data


# ---- parsing --------------------------------------------------------------------------------


def entries(text):
    """(first, last, [other fields]) for each data line of a UCD file."""
    for line in text.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        fields = [field.strip() for field in line.split(";")]
        first, _, last = fields[0].partition("..")
        first = int(first, 16)
        last = int(last, 16) if last else first
        yield first, last, fields[1:]


def require(text, needle, name):
    if needle not in text:
        fail("%s no longer says %r; its defaults changed, check the generator" % (name, needle))


def read_properties(directory):
    eaw_text, _ = fetch("eaw", directory)
    gc_text, _ = fetch("gc", directory)
    emoji_text, _ = fetch("emoji", directory)
    variation_text, _ = fetch("variation", directory)
    gcb_text, _ = fetch("gcb", directory)
    derived_text, _ = fetch("derived", directory)
    props_text, _ = fetch("props", directory)
    indic_text, _ = fetch("indic", directory)

    # East_Asian_Width: unlisted is N, except these blocks' unassigned code points.
    require(eaw_text, "# @missing: 0000..10FFFF; N", "EastAsianWidth.txt")
    for needle in ("CJK Unified Ideographs Extension A: U+3400..U+4DBF", "U+F900..U+FAFF", "U+20000..U+2FFFD"):
        require(eaw_text, needle, "EastAsianWidth.txt")
    eaw = ["N"] * (MAX_CP + 1)
    for first, last in [(0x3400, 0x4DBF), (0x4E00, 0x9FFF), (0xF900, 0xFAFF), (0x20000, 0x2FFFD), (0x30000, 0x3FFFD)]:
        for cp in range(first, last + 1):
            eaw[cp] = "W"
    for first, last, fields in entries(eaw_text):
        for cp in range(first, last + 1):
            eaw[cp] = fields[0]

    # General_Category: unlisted is Cn.
    gc = ["Cn"] * (MAX_CP + 1)
    for first, last, fields in entries(gc_text):
        for cp in range(first, last + 1):
            gc[cp] = fields[0]

    # Emoji_Presentation, Extended_Pictographic.
    emoji_presentation = set()
    pictographic = set()
    for first, last, fields in entries(emoji_text):
        if fields[0] == "Emoji_Presentation":
            emoji_presentation.update(range(first, last + 1))
        elif fields[0] == "Extended_Pictographic":
            pictographic.update(range(first, last + 1))

    # Characters that have an emoji (VS16) and a text (VS15) variation sequence.
    emoji_style = set()
    text_style = set()
    for line in variation_text.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        sequence, style = [field.strip() for field in line.split(";")[:2]]
        base, selector = [int(part, 16) for part in sequence.split()]
        if style == "emoji style" and selector == 0xFE0F:
            emoji_style.add(base)
        elif style == "text style" and selector == 0xFE0E:
            text_style.add(base)
        else:
            fail("emoji-variation-sequences.txt: unexpected line %r" % line)
    if emoji_style != text_style:
        fail("emoji-variation-sequences.txt: emoji and text style bases differ")

    # Grapheme_Cluster_Break: unlisted is Other.
    require(gcb_text, "# @missing: 0000..10FFFF; Other", "GraphemeBreakProperty.txt")
    gcb = ["OTHER"] * (MAX_CP + 1)
    for first, last, fields in entries(gcb_text):
        name = GCB_NAMES.get(fields[0])
        if name is None:
            fail("GraphemeBreakProperty.txt: unknown value %r" % fields[0])
        for cp in range(first, last + 1):
            gcb[cp] = name

    # Indic_Conjunct_Break: unlisted is None.
    require(derived_text, "# @missing: 0000..10FFFF; InCB; None", "DerivedCoreProperties.txt")
    incb = ["None"] * (MAX_CP + 1)
    for first, last, fields in entries(derived_text):
        if fields[0] == "InCB":
            if fields[1] not in ("Linker", "Consonant", "Extend"):
                fail("DerivedCoreProperties.txt: unknown InCB value %r" % fields[1])
            for cp in range(first, last + 1):
                incb[cp] = fields[1]

    # Prepended_Concatenation_Mark: format characters (Cf) that are drawn, a sign before a number.
    concatenation = set()
    for first, last, fields in entries(props_text):
        if fields[0] == "Prepended_Concatenation_Mark":
            concatenation.update(range(first, last + 1))
    if not concatenation:
        fail("PropList.txt has no Prepended_Concatenation_Mark")

    # Indic_Syllabic_Category=Virama: the marks that join a consonant to the next. Most are Mn; the
    # few that are spacing marks (Mc) still take no cell of their own.
    viramas = set()
    for first, last, fields in entries(indic_text):
        if fields[0] == "Virama":
            viramas.update(range(first, last + 1))
    if not viramas:
        fail("IndicSyllabicCategory.txt has no Virama")

    return eaw, gc, emoji_presentation, pictographic, emoji_style, gcb, incb, concatenation, viramas


# ---- the per-code-point byte ----------------------------------------------------------------


def grapheme_class(cp, gcb, incb, pictographic):
    """One class out of GCB, InCB and Extended_Pictographic, which the rules treat separately but
    which never overlap in a way that matters; anything that does overlap stops the script."""
    base, conjunct, pict = gcb[cp], incb[cp], cp in pictographic
    if conjunct == "Consonant":
        if base != "OTHER" or pict:
            fail("U+%04X: InCB=Consonant with Grapheme_Cluster_Break=%s / Extended_Pictographic=%s" % (cp, base, pict))
        return "CONSONANT"
    if pict:
        if base != "OTHER" or conjunct != "None":
            fail("U+%04X: Extended_Pictographic with Grapheme_Cluster_Break=%s / InCB=%s" % (cp, base, conjunct))
        return "EXTENDED_PICTOGRAPHIC"
    if conjunct == "Linker" and base == "OTHER":
        return "LINKER_OTHER"
    if base == "EXTEND":
        return {"None": "EXTEND", "Extend": "EXTEND_INCB", "Linker": "LINKER"}[conjunct]
    if conjunct != "None" and base != "ZWJ":
        fail("U+%04X: InCB=%s with Grapheme_Cluster_Break=%s" % (cp, conjunct, base))
    if base == "ZWJ" and conjunct != "Extend":
        fail("U+%04X: ZWJ is expected to be InCB=Extend, is %s" % (cp, conjunct))
    return base


def cell_width(cp, eaw, gc, emoji_presentation, gcb, concatenation, viramas):
    # U+00AD is drawn at the end of a broken line, and the number signs of Arabic and Syriac (U+0600
    # ..) are drawn over the number that follows: both have a glyph, unlike the other format characters.
    if cp == SOFT_HYPHEN or cp in concatenation:
        return 1
    # Marks, format characters and controls take no cell: Mn, Me, Cf, Cc. So do the unassigned code
    # points that default to Control (default ignorable ones), as they become Cf when assigned.
    if gc[cp] in ("Mn", "Me", "Cf", "Cc") or (gc[cp] == "Cn" and gcb[cp] == "CONTROL"):
        return 0
    if cp in viramas:
        return 0
    if any(first <= cp <= last for first, last in ZERO_WIDTH_RANGES):
        return 0
    if eaw[cp] in ("W", "F") or cp in emoji_presentation:
        return 2
    return 1


def build_bytes(directory):
    eaw, gc, emoji_presentation, pictographic, emoji_style, gcb, incb, concatenation, viramas = read_properties(directory)
    values = []
    for cp in range(MAX_CP + 1):
        if 0xD800 <= cp <= 0xDFFF:  # surrogates are no char; the value of the most common block
            values.append(1 | (CLASS["OTHER"] << CLASS_SHIFT))
            continue
        value = cell_width(cp, eaw, gc, emoji_presentation, gcb, concatenation, viramas)
        value |= (CLASS[grapheme_class(cp, gcb, incb, pictographic)]) << CLASS_SHIFT
        if cp in emoji_style:
            value |= VS_BASE
        values.append(value)
    return values


# ---- the table ------------------------------------------------------------------------------


def index_width(count):
    return 1 if count <= 256 else 2


def split(values, bits):
    """Deduplicated blocks of 2**bits entries: (block table, index of each original block)."""
    size = 1 << bits
    blocks = {}
    table = []
    index = []
    for start in range(0, len(values), size):
        block = tuple(values[start : start + size])
        if len(block) < size:
            block = block + (block[-1],) * (size - len(block))
        if block not in blocks:
            blocks[block] = len(blocks)
            table.extend(block)
        index.append(blocks[block])
    return table, index


def layout(stage3, index3, low_bits, mid_bits):
    """Three stages: the high bits pick a block of stage 2, the middle bits an entry of it that
    picks a block of stage 3, the low bits an entry of that: the value."""
    stage2, stage1 = split(index3, mid_bits)
    size = (
        len(stage1) * index_width(len(stage2) >> mid_bits)
        + len(stage2) * index_width(len(stage3) >> low_bits)
        + len(stage3)
    )
    return size, stage1, stage2, stage3


def best_layout(values):
    """The split of the code point's bits that makes the smallest tables (fewer bits in stage 2
    and 3 on a tie: smaller blocks, shorter to scan)."""
    best = None
    for low_bits in range(1, 9):
        stage3, index3 = split(values, low_bits)
        for mid_bits in range(1, 9):
            candidate = layout(stage3, index3, low_bits, mid_bits)
            key = (candidate[0], mid_bits + low_bits)
            if best is None or key < best[0]:
                best = (key, candidate, mid_bits, low_bits)
    return best[1], best[2], best[3]


# ---- writing --------------------------------------------------------------------------------


def array(name, kind, items, doc):
    """A static array of integers, packed into lines of at most 110 columns (rustfmt would keep
    its own layout, so the item is skipped by it)."""
    lines = ["/// " + doc, "#[rustfmt::skip]", "pub static %s: [%s; %d] = [" % (name, kind, len(items))]
    line = "   "
    for item in items:
        text = " %d," % item
        if len(line) + len(text) > 110:
            lines.append(line)
            line = "   "
        line += text
    lines.append(line)
    lines.append("];")
    return "\n".join(lines)


def render(values):
    (size, stage1, stage2, stage3), mid_bits, low_bits = best_layout(values)
    high_shift = mid_bits + low_bits
    width1 = index_width(len(stage2) >> mid_bits)
    width2 = index_width(len(stage3) >> low_bits)
    kinds = {1: "u8", 2: "u16"}
    out = []
    out.append(
        """//! Unicode %(version)s properties of every code point: the cell width, the grapheme cluster
//! class and whether the character has an emoji variation sequence, in one byte.
//!
//! Generated by `scripts/gen-unicode.py` from the Unicode Character Database %(version)s; do not
//! edit. To regenerate (it downloads the UCD files into `target/ucd/` the first time):
//!
//! ```sh
//! python3 scripts/gen-unicode.py
//! ```
//!
//! The Unicode data files are (c) Unicode, Inc., under the Unicode License v3
//! (<https://www.unicode.org/terms_of_use.html>).
//!
//! The byte: bits 0-1 the width in cells (0, 1 or 2), bit 2 whether the character has an emoji
//! variation sequence (it widens with VS16 and narrows with VS15), bits 3-7 its class. The table is
//! three stages (%(size)d bytes together): the code point's high bits pick a block of `STAGE2`,
//! the middle bits an entry of it that picks a block of `STAGE3`, the low bits the byte."""
        % {"version": UNICODE_VERSION, "size": size}
    )
    out.append("")
    major, minor, patch = UNICODE_VERSION.split(".")
    out.append("/// The Unicode version of the data.")
    out.append("pub const UNICODE_VERSION: (u8, u8, u8) = (%s, %s, %s);" % (major, minor, patch))
    out.append("")
    out.append("pub const WIDTH_MASK: u8 = 0b%s;" % format(WIDTH_MASK, "b"))
    out.append("pub const VS_BASE: u8 = 0b%s;" % format(VS_BASE, "b"))
    out.append("pub const CLASS_SHIFT: u32 = %d;" % CLASS_SHIFT)
    out.append("")
    out.append("/// The grapheme cluster classes, as `props(c) >> CLASS_SHIFT`.")
    out.append("pub mod class {")
    for name in CLASSES:
        out.append("    pub const %s: u8 = %d;" % (name, CLASS[name]))
    out.append("    /// The state before the first character: not the class of any code point.")
    out.append("    pub const START: u8 = %d;" % START)
    out.append("}")
    out.append("")
    out.append("const MID_BITS: u32 = %d;" % mid_bits)
    out.append("const LOW_BITS: u32 = %d;" % low_bits)
    out.append("")
    out.append(array("STAGE1", kinds[width1], stage1, "High bits -> block of `STAGE2`."))
    out.append("")
    out.append(array("STAGE2", kinds[width2], stage2, "Blocks of %d entries -> block of `STAGE3`." % (1 << mid_bits)))
    out.append("")
    out.append(array("STAGE3", "u8", stage3, "Blocks of %d properties bytes." % (1 << low_bits)))
    out.append("")
    out.append(
        """/// The properties byte of a code point.
#[inline]
pub fn props(c: u32) -> u8 {
    let block2 = STAGE1[(c >> (MID_BITS + LOW_BITS)) as usize] as usize;
    let block3 = STAGE2[(block2 << MID_BITS) + ((c >> LOW_BITS) as usize & ((1 << MID_BITS) - 1))] as usize;
    STAGE3[(block3 << LOW_BITS) + (c as usize & ((1 << LOW_BITS) - 1))]
}"""
    )
    out.append("")
    sys.stderr.write(
        "tables: %d bytes (stage 1: %d x %d, stage 2: %d x %d, stage 3: %d), %d-bit middle, %d-bit low\n"
        % (size, len(stage1), width1, len(stage2), width2, len(stage3), mid_bits, low_bits)
    )
    return "\n".join(out)


def write_or_check(path, data, check):
    """True when the file is as it should be (after writing, unless checking)."""
    try:
        with open(path, "rb") as handle:
            current = handle.read()
    except FileNotFoundError:
        current = None
    if current == data:
        return True
    if check:
        print("out of date: " + os.path.relpath(path, ROOT), file=sys.stderr)
        return False
    os.makedirs(os.path.dirname(path), exist_ok=True)
    handle, temp = tempfile.mkstemp(dir=os.path.dirname(path))
    with os.fdopen(handle, "wb") as out:
        out.write(data)
    os.chmod(temp, 0o644)
    os.replace(temp, path)
    print("wrote " + os.path.relpath(path, ROOT), file=sys.stderr)
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--check", action="store_true", help="exit 1 if the files are out of date; write nothing")
    parser.add_argument("--cache", help="where the UCD files are kept (default: target/ucd)")
    args = parser.parse_args()

    directory = cache_dir(args)
    values = build_bytes(directory)
    tables = render(values).encode("utf-8")
    _, test_data = fetch("gbtest", directory)

    ok = write_or_check(TABLES_OUT, tables, args.check)
    ok = write_or_check(TEST_OUT, test_data, args.check) and ok
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()

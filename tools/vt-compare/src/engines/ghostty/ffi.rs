//! The one place that calls libghostty-vt: a safe [`Terminal`] over the C shim (`shim/ghostty_shim.c`,
//! written against the library's public headers), so the adapter above it has no `unsafe`.
//!
//! The shim's functions take integers and pointers only; every call below says why it is sound.
#![allow(unsafe_code)]

use std::ffi::c_int;
use std::ptr::NonNull;

use super::super::Region;

/// Codepoints a cell can report (a longer cluster is cut; no real one comes close).
pub const MAX_CODEPOINTS: usize = 32;

/// `WhCell.attrs` bits, as `shim/ghostty_shim.h` defines them.
pub mod attr {
    pub const BOLD: u16 = 1;
    pub const DIM: u16 = 2;
    pub const ITALIC: u16 = 4;
    pub const BLINK: u16 = 8;
    pub const INVERSE: u16 = 16;
    pub const HIDDEN: u16 = 32;
    pub const STRIKE: u16 = 64;
    pub const OVERLINE: u16 = 128;
    pub const UNDERLINE: u16 = 256;
    pub const DOUBLE_UNDERLINE: u16 = 512;
    pub const CURLY_UNDERLINE: u16 = 1024;
    pub const DOTTED_UNDERLINE: u16 = 2048;
    pub const DASHED_UNDERLINE: u16 = 4096;
}

/// `WhCell.fg_kind` and friends.
pub const COLOR_PALETTE: u8 = 1;
/// A 24-bit color: the three bytes of the array.
pub const COLOR_RGB: u8 = 2;

/// An opaque terminal; only ever behind a pointer.
#[repr(C)]
struct WhGhostty {
    _private: [u8; 0],
}

/// One cell as the shim reports it (`WhCell` in the header).
#[repr(C)]
pub struct Cell {
    pub codepoints: [u32; MAX_CODEPOINTS],
    pub n_codepoints: u32,
    /// 1, 2, or 0 for the second half of a wide character.
    pub width: u8,
    /// The blank left at the end of a row when a wide character wrapped to the next.
    pub spacer_head: u8,
    pub fg_kind: u8,
    pub bg_kind: u8,
    pub ul_kind: u8,
    pub fg: [u8; 3],
    pub bg: [u8; 3],
    pub ul: [u8; 3],
    pub attrs: u16,
}

unsafe extern "C" {
    fn wh_ghostty_new(
        cols: u16,
        rows: u16,
        scrollback_lines: usize,
        palette: *const u8,
        foreground: *const u8,
        background: *const u8,
        cursor: *const u8,
    ) -> *mut WhGhostty;
    fn wh_ghostty_free(h: *mut WhGhostty);
    fn wh_ghostty_feed(h: *mut WhGhostty, data: *const u8, len: usize);
    fn wh_ghostty_resize(h: *mut WhGhostty, cols: u16, rows: u16) -> c_int;
    fn wh_ghostty_replies(h: *const WhGhostty, len: *mut usize) -> *const u8;
    fn wh_ghostty_clear_replies(h: *mut WhGhostty);
    fn wh_ghostty_cursor(h: *const WhGhostty, col: *mut u16, row: *mut u16, visible: *mut c_int) -> c_int;
    fn wh_ghostty_alt_screen(h: *const WhGhostty) -> c_int;
    fn wh_ghostty_title(h: *const WhGhostty, len: *mut usize) -> *const u8;
    fn wh_ghostty_scrollback_rows(h: *const WhGhostty) -> usize;
    fn wh_ghostty_cell(h: *const WhGhostty, history: c_int, x: u16, y: u32, out: *mut Cell) -> c_int;
    fn wh_ghostty_link(h: *const WhGhostty, history: c_int, x: u16, y: u32, buf: *mut u8, cap: usize) -> i64;
    fn wh_ghostty_row_wrapped(h: *const WhGhostty, history: c_int, y: u32) -> c_int;
    fn wh_ghostty_alloc_counts(allocs: *mut u64, bytes: *mut u64);
}

fn history(region: Region) -> c_int {
    c_int::from(region == Region::History)
}

/// Allocator calls and bytes libghostty-vt has asked for so far, over the whole process.
pub fn alloc_counts() -> (u64, u64) {
    let (mut allocs, mut bytes) = (0u64, 0u64);
    // SAFETY: both pointers are to live locals, which the function fills and does not keep.
    unsafe { wh_ghostty_alloc_counts(&mut allocs, &mut bytes) };
    (allocs, bytes)
}

/// A libghostty-vt terminal, owned: freed on drop.
pub struct Terminal {
    raw: NonNull<WhGhostty>,
}

impl Terminal {
    /// A `cols` x `rows` terminal keeping about `scrollback` lines, answering color queries with
    /// `palette` (256 RGB triples), `foreground`, `background` and `cursor` (RGB triples).
    /// `None` when the library could not make one.
    pub fn new(
        cols: usize,
        rows: usize,
        scrollback: usize,
        palette: &[u8; 768],
        foreground: [u8; 3],
        background: [u8; 3],
        cursor: [u8; 3],
    ) -> Option<Terminal> {
        // SAFETY: every pointer is to a live array of the length the shim reads (768, 3, 3, 3
        // bytes); it copies them and keeps none.
        let raw = unsafe {
            wh_ghostty_new(
                cols as u16,
                rows as u16,
                scrollback,
                palette.as_ptr(),
                foreground.as_ptr(),
                background.as_ptr(),
                cursor.as_ptr(),
            )
        };
        NonNull::new(raw).map(|raw| Terminal { raw })
    }

    /// Feed bytes to the terminal's parser.
    pub fn feed(&mut self, bytes: &[u8]) {
        // SAFETY: `raw` is live; the library reads `bytes.len()` bytes from the pointer during the call.
        unsafe { wh_ghostty_feed(self.raw.as_ptr(), bytes.as_ptr(), bytes.len()) }
    }

    /// Resize; false when the library refused.
    pub fn resize(&mut self, cols: usize, rows: usize) -> bool {
        // SAFETY: `raw` is a live terminal.
        unsafe { wh_ghostty_resize(self.raw.as_ptr(), cols as u16, rows as u16) == 0 }
    }

    /// What the terminal wrote to the program since the last call.
    pub fn take_replies(&mut self) -> Vec<u8> {
        let mut len = 0usize;
        // SAFETY: `raw` is live. The shim returns a buffer of `len` readable bytes (or null when
        // empty), which stays valid until the next call that feeds the terminal or clears the
        // replies; it is copied here, before either.
        let replies = unsafe {
            let ptr = wh_ghostty_replies(self.raw.as_ptr(), &mut len);
            if ptr.is_null() || len == 0 { Vec::new() } else { std::slice::from_raw_parts(ptr, len).to_vec() }
        };
        // SAFETY: `raw` is live.
        unsafe { wh_ghostty_clear_replies(self.raw.as_ptr()) };
        replies
    }

    /// `(column, row, shown)`, from 0.
    pub fn cursor(&self) -> (usize, usize, bool) {
        let (mut col, mut row, mut visible) = (0u16, 0u16, 0 as c_int);
        // SAFETY: `raw` is live; the three pointers are to live locals the shim writes once.
        unsafe { wh_ghostty_cursor(self.raw.as_ptr(), &mut col, &mut row, &mut visible) };
        (usize::from(col), usize::from(row), visible != 0)
    }

    /// Whether the alternate screen shows.
    pub fn alt_screen(&self) -> bool {
        // SAFETY: `raw` is a live terminal.
        unsafe { wh_ghostty_alt_screen(self.raw.as_ptr()) != 0 }
    }

    /// The window title the program set, `None` when none.
    pub fn title(&self) -> Option<String> {
        let mut len = 0usize;
        // SAFETY: `raw` is live. The title is `len` readable bytes (or null when there is none),
        // borrowed until the next call that feeds or resizes the terminal; it is copied here, before.
        let bytes = unsafe {
            let ptr = wh_ghostty_title(self.raw.as_ptr(), &mut len);
            if ptr.is_null() || len == 0 { None } else { Some(std::slice::from_raw_parts(ptr, len)) }
        };
        bytes.map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }

    /// Lines of history.
    pub fn scrollback_rows(&self) -> usize {
        // SAFETY: `raw` is a live terminal.
        unsafe { wh_ghostty_scrollback_rows(self.raw.as_ptr()) }
    }

    /// Cell `(x, y)` of the screen or the history, `None` out of range.
    pub fn cell(&self, region: Region, x: usize, y: usize) -> Option<Cell> {
        let mut cell = std::mem::MaybeUninit::<Cell>::uninit();
        // SAFETY: `raw` is live; `cell` is a writable `Cell`, which the shim zeroes before it fills
        // it, so it is initialized whenever the call returns 0, the only case it is read.
        let status = unsafe {
            wh_ghostty_cell(self.raw.as_ptr(), history(region), x as u16, y as u32, cell.as_mut_ptr())
        };
        // SAFETY: initialized by the shim when `status` is 0, see above.
        (status == 0).then(|| unsafe { cell.assume_init() })
    }

    /// The hyperlink target of a cell, `None` when it has none.
    pub fn link(&self, region: Region, x: usize, y: usize) -> Option<String> {
        let mut buf = vec![0u8; 256];
        loop {
            // SAFETY: `raw` is live; the shim writes at most `buf.len()` bytes to `buf`.
            let len = unsafe {
                wh_ghostty_link(
                    self.raw.as_ptr(),
                    history(region),
                    x as u16,
                    y as u32,
                    buf.as_mut_ptr(),
                    buf.len(),
                )
            };
            if len <= 0 {
                return None;
            }
            let len = len as usize;
            if len <= buf.len() {
                buf.truncate(len);
                return Some(String::from_utf8_lossy(&buf).into_owned());
            }
            buf.resize(len, 0);
        }
    }

    /// Whether the row continues in the next one because the text wrapped.
    pub fn row_wrapped(&self, region: Region, y: usize) -> bool {
        // SAFETY: `raw` is a live terminal.
        unsafe { wh_ghostty_row_wrapped(self.raw.as_ptr(), history(region), y as u32) == 1 }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // SAFETY: `raw` came from `wh_ghostty_new` and is freed exactly once, here.
        unsafe { wh_ghostty_free(self.raw.as_ptr()) }
    }
}

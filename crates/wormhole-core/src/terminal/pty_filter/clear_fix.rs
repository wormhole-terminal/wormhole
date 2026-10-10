//! Makes `clear` erase the scrollback too.
//!
//! macOS `/usr/bin/clear` writes `ESC[3J ESC[H ESC[2J`: erase scrollback
//! (ED 3) *before* clearing the screen (ED 2). xterm's ED 2 simply blanks the
//! screen, but alacritty's ED 2 first scrolls the screen into the history, so
//! the ED 3 that ran just before it has nothing to erase and the old screen is
//! still there when you scroll up. alacritty_terminal parses the PTY output on
//! its own thread without a hook, so the PTY reader swaps the two into the
//! order Linux ncurses uses (`ESC[H ESC[2J ESC[3J`), which leaves an empty
//! screen and an empty scrollback. A bare `ESC[H ESC[2J` (zsh's Ctrl-L,
//! `tput clear`) is untouched and still keeps the screen in the scrollback.
//! `super::FilteredPty` runs the rewrite on every read, before the OSC tap.

use std::io::{self, Read};
use std::sync::LazyLock;

use memchr::memmem;

/// What macOS `/usr/bin/clear` writes for `TERM=xterm-256color`.
const CLEAR_ERASE_FIRST: &[u8] = b"\x1b[3J\x1b[H\x1b[2J";
/// The same bytes with the scrollback erased after the screen is cleared.
const CLEAR_ERASE_LAST: &[u8] = b"\x1b[H\x1b[2J\x1b[3J";

/// Rewrites [`CLEAR_ERASE_FIRST`] in a byte stream, including when a read
/// splits it: a chunk ending in the start of the sequence is held back and
/// completed by the next read.
#[derive(Default)]
pub(super) struct ClearRewriter {
    held: Vec<u8>,
}

impl ClearRewriter {
    pub(super) fn read_from(&mut self, inner: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let held = self.held.len();
            if buf.len() <= held {
                // Too small to add anything to the held bytes: hand them over as they are.
                let n = buf.len();
                buf.copy_from_slice(&self.held[..n]);
                self.held.drain(..n);
                return Ok(n);
            }
            buf[..held].copy_from_slice(&self.held);
            self.held.clear();
            let got = match inner.read(&mut buf[held..]) {
                Ok(got) => got,
                // Nothing more to complete the sequence with (yet): pass the
                // held bytes on unchanged rather than stall them. The next read
                // reports the error or end of stream again.
                Err(_) if held > 0 => 0,
                Err(err) => return Err(err),
            };
            if got == 0 {
                return Ok(held);
            }
            let len = held + got;
            rewrite(&mut buf[..len]);
            let tail = partial_tail(&buf[..len]);
            self.held.extend_from_slice(&buf[len - tail..len]);
            if tail < len {
                return Ok(len - tail);
            }
            // Everything read so far may be the start of the sequence: read on.
        }
    }
}

/// Rewrite every [`CLEAR_ERASE_FIRST`] in `bytes`. This runs on every PTY read
/// while the reader holds the terminal's lease (the UI waits behind it), so it
/// uses a SIMD substring search rather than comparing a window at each byte.
fn rewrite(bytes: &mut [u8]) {
    static FINDER: LazyLock<memmem::Finder<'static>> =
        LazyLock::new(|| memmem::Finder::new(CLEAR_ERASE_FIRST));
    let n = CLEAR_ERASE_FIRST.len();
    let mut start = 0;
    while let Some(at) = FINDER.find(&bytes[start..]) {
        let at = start + at;
        bytes[at..at + n].copy_from_slice(CLEAR_ERASE_LAST);
        start = at + n;
    }
}

/// Length of the longest suffix of `bytes` that is a proper prefix of
/// [`CLEAR_ERASE_FIRST`].
fn partial_tail(bytes: &[u8]) -> usize {
    (1..CLEAR_ERASE_FIRST.len().min(bytes.len() + 1))
        .rev()
        .find(|&k| bytes.ends_with(&CLEAR_ERASE_FIRST[..k]))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io::{self, Read};

    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::{Column, Line};
    use alacritty_terminal::term::{Config, Term};
    use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};

    use super::{CLEAR_ERASE_FIRST, CLEAR_ERASE_LAST, ClearRewriter};
    use crate::terminal::GridSize;

    /// Hands out fixed chunks, then reports `WouldBlock` like a drained PTY.
    struct Chunks(VecDeque<Vec<u8>>);

    impl Read for Chunks {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let Some(mut chunk) = self.0.pop_front() else {
                return Err(io::ErrorKind::WouldBlock.into());
            };
            let n = chunk.len().min(buf.len());
            buf[..n].copy_from_slice(&chunk[..n]);
            if n < chunk.len() {
                self.0.push_front(chunk.split_off(n));
            }
            Ok(n)
        }
    }

    /// Reads everything the way alacritty's event loop does: until the
    /// reader would block.
    fn filtered(chunks: &[&[u8]]) -> Vec<u8> {
        // An empty read means end of stream, so there are no empty chunks.
        let mut inner = Chunks(chunks.iter().filter(|c| !c.is_empty()).map(|c| c.to_vec()).collect());
        let mut rewriter = ClearRewriter::default();
        let mut out = Vec::new();
        let mut buf = [0u8; 64];
        loop {
            match rewriter.read_from(&mut inner, &mut buf) {
                Ok(0) => break,
                Ok(n) => out.extend_from_slice(&buf[..n]),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) => panic!("read failed: {err}"),
            }
        }
        out
    }

    fn term() -> Term<VoidListener> {
        let size = GridSize { cols: 20, rows: 5, cell_width: 8.0, cell_height: 16.0 };
        Term::new(Config { scrolling_history: 1000, ..Config::default() }, &size, VoidListener)
    }

    /// Thirty lines of output, then `clear`, then a prompt.
    fn session() -> Vec<u8> {
        let mut bytes = Vec::new();
        for i in 0..30 {
            bytes.extend_from_slice(format!("line {i}\r\n").as_bytes());
        }
        bytes.extend_from_slice(CLEAR_ERASE_FIRST);
        bytes.extend_from_slice(b"$ ");
        bytes
    }

    fn screen_text(term: &Term<VoidListener>) -> String {
        let grid = term.grid();
        (0..grid.screen_lines())
            .map(|line| {
                (0..grid.columns()).map(|col| grid[Line(line as i32)][Column(col)].c).collect::<String>()
            })
            .map(|line| line.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn engine_alone_keeps_the_screen_that_clear_erased() {
        // Documents why the rewrite exists; if alacritty changes this, it can go.
        let mut term = term();
        Processor::<StdSyncHandler>::new().advance(&mut term, &session());
        assert!(term.grid().history_size() > 0);
    }

    #[test]
    fn clear_empties_screen_and_scrollback() {
        let bytes = session();
        // Every way a read can split the output, including inside the sequence.
        for split in 0..=bytes.len() {
            let (a, b) = bytes.split_at(split);
            let mut term = term();
            Processor::<StdSyncHandler>::new().advance(&mut term, &filtered(&[a, b]));
            assert_eq!(term.grid().history_size(), 0, "split at {split}");
            assert_eq!(screen_text(&term), "$\n\n\n\n", "split at {split}");
            let cursor = term.grid().cursor.point;
            assert_eq!((cursor.line, cursor.column), (Line(0), Column(2)), "split at {split}");
        }
    }

    /// Throughput of the rewrite over 64 KiB chunks of colored output (it runs
    /// on every PTY read). Run with `--ignored --nocapture`.
    #[test]
    #[ignore = "measurement; run with --ignored --nocapture"]
    fn rewrite_throughput() {
        let mut chunk = Vec::new();
        while chunk.len() < 64 * 1024 {
            chunk.extend_from_slice(b"\x1b[32m+ added\x1b[0m line of \x1b[1mdiff\x1b[0m output\r\n");
        }
        chunk.truncate(64 * 1024);
        let mut work = chunk.clone();
        let rounds = 4000;
        let start = std::time::Instant::now();
        for _ in 0..rounds {
            work.copy_from_slice(&chunk);
            super::rewrite(std::hint::black_box(&mut work));
        }
        let copy_start = std::time::Instant::now();
        for _ in 0..rounds {
            work.copy_from_slice(std::hint::black_box(&chunk));
        }
        let copy = copy_start.elapsed();
        let rewrite = (copy_start - start).saturating_sub(copy);
        let per_chunk = rewrite / rounds;
        let gbps = (64.0 * 1024.0) / per_chunk.as_secs_f64() / 1e9;
        eprintln!("rewrite: {per_chunk:?} per 64 KiB chunk, {gbps:.2} GB/s");
    }

    #[test]
    fn rewrites_only_the_erase_first_clear() {
        assert_eq!(filtered(&[CLEAR_ERASE_FIRST]), CLEAR_ERASE_LAST);
        assert_eq!(filtered(&[b"a\x1b[3J\x1b[H", b"\x1b[2Jb"]), b"a\x1b[H\x1b[2J\x1b[3Jb");
        // Byte at a time, with a second ED 3 in front.
        let doubled = b"\x1b[3J\x1b[3J\x1b[H\x1b[2J";
        let chunks: Vec<&[u8]> = doubled.chunks(1).collect();
        assert_eq!(filtered(&chunks), b"\x1b[3J\x1b[H\x1b[2J\x1b[3J");
        // Ctrl-L's clear keeps the screen in the scrollback; leave it alone.
        for untouched in
            [&b"\x1b[H\x1b[2J"[..], b"\x1b[3J", b"\x1b[3J\x1b[H", b"\x1b[3J\x1b[Hx\x1b[2J", b"plain \x1b"]
        {
            assert_eq!(filtered(&[untouched]), untouched);
            let chunks: Vec<&[u8]> = untouched.chunks(1).collect();
            assert_eq!(filtered(&chunks), untouched);
        }
    }
}

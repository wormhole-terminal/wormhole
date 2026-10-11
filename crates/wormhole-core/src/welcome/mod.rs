//! `Wormhole welcome`: the first run as a program in the terminal itself.
//!
//! Where the intro window stands in for the terminal, this is the terminal: the app starts the first
//! terminal's shell as `Wormhole welcome -- <the shell it would have run>`, the window and its sidebar
//! show as usual, and the program types its greeting into the real PTY and offers the steps as keyboard
//! menus (where you work, how it looks, coding agents). When it ends it clears the screen, prints a
//! compact banner and `exec`s the shell, in the folder picked, in that same terminal.
//!
//! It plays into the start animation. The app's first answer says how long to wait (`wait_ms`: until
//! the ship has docked, or, with no ship, until the light lifts away), and the three hand-overs differ in
//! where the ship lands: on the `>` of the mark at [`MARK_CELL`] (Cursor), on the chevron of the
//! block-art logo at [`LOGO`] (Logo), or nowhere (Portal). Those cells are fixed so the app can aim.
//!
//! The choices are the app's to make, so each goes over the control socket
//! (`ipc::control::WelcomeOp`), authorised by a token only this terminal has ([`Gate`]). Without a
//! token or a reachable app (another terminal app, a hand-typed run) the welcome is a read-only tour:
//! the same pages, no actions, and one dim line that says so.
//!
//! - `ansi`: the screen as a grid of styled cells and the bytes that draw it (palette colours only);
//! - `keys`: keys, and SGR mouse clicks, read from the terminal's bytes;
//! - `model`: the state: steps (core's `onboarding` model), cursors, and what each key does;
//! - `render`: the pages, a pure function of the model, the size and the clock;
//! - `art`: the mark's and the logo's places, and the logo's pixels;
//! - `banner`: what the terminal shows at the hand-over;
//! - `facts`: the folders and agents the pages list, read from the machine;
//! - `gate`: the token rules;
//! - `driver`: one tick of the program, without any I/O, so it can be scripted in tests;
//! - `tty`: raw mode, the size and waiting for a key (the only `libc` here);
//! - `run`: the loop around the driver, the request thread and the final `exec`.

mod ansi;
mod art;
mod banner;
mod driver;
pub mod facts;
mod gate;
mod keys;
mod model;
mod render;
mod run;
mod tty;

pub use art::{
    DockCell, LOGO, LOGO_CHEVRON_CELL, LOGO_DOCK, MARGIN, MARK_CELL, MARK_CHEVRON_ROWS, MARK_DOCK, Rect, TOP,
    dock,
};
pub use gate::{Gate, Registry, TOKEN_VAR, new_token};
pub use render::TAGLINE;
pub use run::{Args, HELP, command, parse_args, run};

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod snapshots;
#[cfg(test)]
mod walkthrough;

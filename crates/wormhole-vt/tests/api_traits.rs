//! What a host may rely on about the public types, checked when this file compiles: a field that
//! is not `Send` (an `Rc`, a raw pointer) or an error that loses `std::error::Error` fails the
//! build of the crate's tests here, not the build of someone else's application.
//!
//! The terminal and what moves between threads with it are `Send + Sync`: a host parses on one
//! thread and reads on another, usually under a lock, and a read-only view may be shared. Only the
//! public API is named, so this is also a check that these types are reachable by their paths.

use wormhole_vt::event::{Event, Progress, ProgressState, PromptKind, ShellMark, Terminator};
use wormhole_vt::image::{ImageError, ImageHandle, ImagePlacement, ImageUpload};
use wormhole_vt::input::{InputModes, KeyEvent, MouseEvent};
use wormhole_vt::parser::Parser;
use wormhole_vt::passthrough::{Passthrough, PassthroughError};
use wormhole_vt::repaint::{Gaps, NotReady, Repainted, Seeds};
use wormhole_vt::window::{WindowGeometry, WindowOp};
use wormhole_vt::{
    Builder, Cell, Color, Colors, Cursor, Damage, DroppedHistory, HistoryLine, Hyperlink, Limits, Modes, Pin,
    Point, Policy, RowExtras, SelectionRange, Span, Terminal, ViewRow,
};

const fn send_sync<T: Send + Sync>() {}
const fn plain_data<T: Send + Sync + Clone + std::fmt::Debug + PartialEq>() {}
const fn error<T: std::error::Error + Send + Sync + 'static>() {}

const _: () = {
    // The engine and the things a host builds it from.
    send_sync::<Terminal>();
    send_sync::<Builder>();
    send_sync::<Parser>();
    send_sync::<Damage>();
    send_sync::<Passthrough>();
    send_sync::<Limits>();
    send_sync::<Policy>();

    // What comes out of it moves to another thread (an event loop, a renderer) and is compared in tests.
    plain_data::<Event>();
    plain_data::<ShellMark>();
    plain_data::<Progress>();
    plain_data::<ProgressState>();
    plain_data::<PromptKind>();
    plain_data::<Terminator>();
    plain_data::<ImageUpload>();
    plain_data::<ImagePlacement>();
    plain_data::<ImageHandle>();
    plain_data::<DroppedHistory>();
    plain_data::<HistoryLine>();
    plain_data::<ViewRow>();
    plain_data::<RowExtras>();
    plain_data::<Hyperlink>();
    plain_data::<Cell>();
    plain_data::<Color>();
    plain_data::<Colors>();
    plain_data::<Modes>();
    plain_data::<Cursor>();
    plain_data::<Point>();
    plain_data::<Pin>();
    plain_data::<Span>();
    plain_data::<SelectionRange>();
    plain_data::<WindowGeometry>();
    plain_data::<WindowOp>();
    plain_data::<Repainted>();
    plain_data::<Seeds>();
    plain_data::<Gaps>();
    plain_data::<InputModes>();

    // What goes in is built on the host's thread too.
    send_sync::<KeyEvent<'static>>();
    send_sync::<MouseEvent>();

    // Errors are errors: `?` converts them, `Box<dyn Error>` holds them, other threads carry them.
    error::<PassthroughError>();
    error::<ImageError>();
    error::<NotReady>();
};

#[test]
fn the_traits_are_checked_at_compile_time() {
    // Nothing to run: the constants above are the test. A failing one is a compile error.
}

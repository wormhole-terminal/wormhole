//! The small values several messages share: sizes, colors, versions, feature bits and the enums
//! whose set may grow in a later minor.

use crate::cursor::{Reader, Writer};
use crate::error::WireError;

/// An enum that crosses the wire as an integer. A value this version does not know decodes to
/// `Unknown(v)` instead of failing: a newer peer may have added it, and the receiver decides
/// whether it can live with that.
macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident : $repr:ty { $($(#[$vmeta:meta])* $var:ident = $val:expr),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name {
            $($(#[$vmeta])* $var,)+
            /// A value from a newer minor.
            Unknown($repr),
        }

        impl $name {
            pub const fn to_wire(self) -> $repr {
                match self { $(Self::$var => $val,)+ Self::Unknown(v) => v }
            }
            pub const fn from_wire(v: $repr) -> Self {
                match v { $($val => Self::$var,)+ v => Self::Unknown(v) }
            }
        }
    };
}
pub(crate) use wire_enum;

/// A terminal's size: the grid and the cell in pixels. Cells are in 1/16 pixel, so a fractional
/// font metric such as 8.4 survives the trip and the field stays an integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
    pub cell_w: u16,
    pub cell_h: u16,
}

impl Size {
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self { cols: r.u16()?, rows: r.u16()?, cell_w: r.u16()?, cell_h: r.u16()? })
    }

    pub(crate) fn write(&self, w: &mut Writer<'_>) {
        w.u16(self.cols);
        w.u16(self.rows);
        w.u16(self.cell_w);
        w.u16(self.cell_h);
    }
}

/// A list a message holds: borrowed from the sender's slice when encoding, owned (decoded) when
/// read. Like `Cow<[T]>` but covariant in `'a`, so a `Message<'static>` constant can stand in
/// wherever a shorter-lived message is wanted (`Cow` goes through `ToOwned` and cannot).
#[derive(Clone, Debug)]
pub enum List<'a, T> {
    Borrowed(&'a [T]),
    Owned(Vec<T>),
}

impl<T> std::ops::Deref for List<'_, T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match self {
            Self::Borrowed(s) => s,
            Self::Owned(v) => v,
        }
    }
}

impl<T: PartialEq> PartialEq for List<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: Eq> Eq for List<'_, T> {}

impl<T> From<Vec<T>> for List<'_, T> {
    fn from(v: Vec<T>) -> Self {
        Self::Owned(v)
    }
}

impl<'a, T> From<&'a [T]> for List<'a, T> {
    fn from(s: &'a [T]) -> Self {
        Self::Borrowed(s)
    }
}

/// A color, red green blue.
pub type Rgb = [u8; 3];

/// A program's or an app's version, `major.minor.patch`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl AppVersion {
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self { major: r.u16()?, minor: r.u16()?, patch: r.u16()? })
    }

    pub(crate) fn write(&self, w: &mut Writer<'_>) {
        w.u16(self.major);
        w.u16(self.minor);
        w.u16(self.patch);
    }
}

/// Optional behavior, a bit each. A peer uses a feature only when both sides announced it
/// (`Hello.features & Welcome.features`), so an older host degrades the app instead of failing it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Features(pub u64);

impl Features {
    /// LZ4 frames (`flags::COMPRESSED`).
    pub const COMPRESSION: Self = Self(1 << 0);
    /// `PageGet` / `Page`.
    pub const HISTORY_PAGES: Self = Self(1 << 1);
    /// `FindStart` / `FindResults`.
    pub const FIND: Self = Self(1 << 2);
    /// `Attach.resume_from`: the host keeps a ring of recent output.
    pub const RESUME_RING: Self = Self(1 << 3);
    /// `Sync` streams (host-synthesized repaints under flow control).
    pub const SYNC: Self = Self(1 << 4);

    pub const NONE: Self = Self(0);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

wire_enum! {
    /// What a connection may do (`Hello.role`).
    Role: u8 {
        /// Sets the size, sends input; one at a time.
        Controller = 1,
        /// Watches; follows the host's size.
        Observer = 2,
        /// `GetInfo` and `Kill` only, on every host version.
        Admin = 3,
    }
}

wire_enum! {
    /// Whether the app's theme is light or dark (`SetColorScheme`): the host's engine answers
    /// `CSI ? 996 n` with it and tells the programs that asked for mode 2031.
    ColorScheme: u8 {
        Dark = 1,
        Light = 2,
    }
}

wire_enum! {
    /// Whether the host is still running the terminal's program (`Welcome.state`).
    HostState: u8 {
        Running = 0,
        Exited = 1,
    }
}

wire_enum! {
    /// What an attachment receives of the output (`Attach.stream`).
    StreamMode: u8 {
        /// Events and counters only.
        None = 0,
        /// The program's bytes as they come.
        Raw = 1,
        /// Host-synthesized repaints.
        Sync = 2,
    }
}

wire_enum! {
    /// Why a connection was detached by the host (`Detached.reason`).
    DetachReason: u8 {
        /// Another controller took over.
        TakenOver = 1,
        /// The host is ending.
        HostClosing = 2,
        /// The host refused the connection's requests.
        Rejected = 3,
        /// It was too slow to follow.
        TooSlow = 4,
    }
}

/// How the program ended: the exit code, or the signal that killed it (0 when none did).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitInfo {
    pub code: i32,
    pub signal: u8,
    /// Milliseconds since the Unix epoch.
    pub at_ms: u64,
}

impl ExitInfo {
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self { code: r.i32()?, signal: r.u8()?, at_ms: r.u64()? })
    }

    pub(crate) fn write(&self, w: &mut Writer<'_>) {
        w.i32(self.code);
        w.u8(self.signal);
        w.u64(self.at_ms);
    }
}

/// What the scrollbar and the sidebar read (`Counters`, and inside `Welcome`). The scrollbar's total
/// is `history_lines + screen_rows`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counters {
    /// Bumps when a resize reflows the history: line numbers restart.
    pub epoch: u32,
    pub history_lines: u32,
    /// Lines the history has dropped since the host started.
    pub trimmed_total: u64,
    pub screen_rows: u16,
    pub alt_screen: bool,
    /// Counts output; equal revisions mean an unchanged screen.
    pub revision: u64,
}

impl Counters {
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self {
            epoch: r.u32()?,
            history_lines: r.u32()?,
            trimmed_total: r.u64()?,
            screen_rows: r.u16()?,
            alt_screen: r.bool()?,
            revision: r.u64()?,
        })
    }

    pub(crate) fn write(&self, w: &mut Writer<'_>) {
        w.u32(self.epoch);
        w.u32(self.history_lines);
        w.u64(self.trimmed_total);
        w.u16(self.screen_rows);
        w.bool(self.alt_screen);
        w.u64(self.revision);
    }
}

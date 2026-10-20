//! The control data of one kitty graphics command: `key=value,key=value,…`.
//!
//! Spec: <https://sw.kovidgoyal.net/kitty/graphics-protocol/> ("Control data reference"). Every
//! key is one letter; values are integers, or one letter for the keys that name an action, a
//! medium, a compression or a delete. Keys the engine does not know are skipped, as the protocol
//! asks of a terminal that does not support an extension.

use super::reply::Error;

/// Most control data one command may carry; the keys of the protocol fit in a fraction of this.
pub(super) const CONTROL_MAX: usize = 2048;

/// `a`: what the command does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    /// `t` (the default): store the picture.
    Transmit,
    /// `T`: store it and place it at the cursor.
    TransmitDisplay,
    /// `p`: place a stored picture.
    Put,
    /// `d`: delete placements or pictures.
    Delete,
    /// `q`: ask whether this would work, without storing anything.
    Query,
    /// `f`, `a`, `c`: animation frames, their control and composition (not supported).
    Animation,
}

/// `t`: where the data comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Medium {
    /// `d` (the default): in the command.
    Direct,
    /// `f`, `t`, `s`: a file, a temporary file, shared memory (the engine does no I/O).
    Elsewhere,
}

/// `q`: which replies the program does not want.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Quiet {
    /// 0 (the default): all of them.
    #[default]
    Normal,
    /// 1: not the `OK`s.
    NoOk,
    /// 2: none.
    Silent,
}

/// `d`: which placements or pictures a delete is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Selector {
    /// `a` (the default): placements that are on the screen.
    Visible,
    /// `i`: those of the picture with id `i` (and placement `p` if given).
    ById,
    /// `n`: those of the newest picture with number `I` (and placement `p` if given).
    ByNumber,
    /// `c`: those that cover the cursor's cell.
    AtCursor,
    /// `f`: animation frames (none here).
    Frames,
    /// `p`: those that cover the cell `x`, `y`.
    AtCell,
    /// `q`: those that cover the cell `x`, `y` and have z-index `z`.
    AtCellWithZ,
    /// `r`: those of pictures with an id from `x` to `y`.
    IdRange,
    /// `x`: those that cover column `x`.
    InColumn,
    /// `y`: those that cover row `y`.
    InRow,
    /// `z`: those with z-index `z`.
    WithZ,
}

/// One command's keys, with the protocol's defaults where a key is absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Command {
    pub action: Action,
    /// `f`: 24 RGB, 32 RGBA, 100 PNG.
    pub format: u32,
    pub medium: Medium,
    /// `s`, `v`: the pixel size, for raw data.
    pub width: u32,
    pub height: u32,
    /// `o=z`: the data is zlib-compressed.
    pub zlib: bool,
    /// `m=1`: more chunks follow.
    pub more: bool,
    pub quiet: Quiet,
    /// `i`: the picture's id, 0 for none.
    pub id: u32,
    /// `I`: the picture's number, 0 for none.
    pub number: u32,
    /// `p`: the placement's id, 0 for none.
    pub placement: u32,
    /// `x`, `y`, `w`, `h`: the part of the picture to show, in pixels (0 width or height: to the
    /// edge). For deletes `x` and `y` are cell coordinates, counted from 1.
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    /// `X`, `Y`: pixels from the first cell's corner to the picture.
    pub offset_x: u32,
    pub offset_y: u32,
    /// `c`, `r`: the cells to cover (0: from the picture's size).
    pub cols: u32,
    pub rows: u32,
    /// `z`: stacking order.
    pub z: i32,
    /// `C=1`: leave the cursor where it was.
    pub cursor_stays: bool,
    /// `U=1`: a virtual placement.
    pub unicode_placeholder: bool,
    /// `P`, `Q`, `H`, `V`: a placement relative to another (any of them given).
    pub relative: bool,
    /// `d`, with the case saying whether to free the picture's data too.
    pub selector: Selector,
    pub free: bool,
}

impl Default for Command {
    fn default() -> Self {
        Command {
            action: Action::Transmit,
            format: 32,
            medium: Medium::Direct,
            width: 0,
            height: 0,
            zlib: false,
            more: false,
            quiet: Quiet::Normal,
            id: 0,
            number: 0,
            placement: 0,
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            offset_x: 0,
            offset_y: 0,
            cols: 0,
            rows: 0,
            z: 0,
            cursor_stays: false,
            unicode_placeholder: false,
            relative: false,
            selector: Selector::Visible,
            free: false,
        }
    }
}

impl Command {
    /// Read the control data. A key that fails to parse makes the command invalid, but the ids
    /// before it are kept, so the program can still be told which command failed.
    pub fn parse(control: &[u8]) -> (Command, Option<Error>) {
        let mut command = Command::default();
        let mut error = None;
        if control.len() > CONTROL_MAX {
            return (command, Some(Error::Invalid("control data too long")));
        }
        for pair in control.split(|&b| b == b',').filter(|pair| !pair.is_empty()) {
            if let Err(err) = command.set(pair) {
                error.get_or_insert(err);
            }
        }
        if error.is_none() && command.id != 0 && command.number != 0 {
            error = Some(Error::Invalid("both an id and a number"));
        }
        (command, error)
    }

    fn set(&mut self, pair: &[u8]) -> Result<(), Error> {
        let [key, b'=', value @ ..] = pair else {
            return Err(Error::Invalid("a key without a value"));
        };
        let unsigned = || -> Result<u32, Error> {
            std::str::from_utf8(value)
                .ok()
                .and_then(|text| text.parse::<u32>().ok())
                .ok_or(Error::Invalid("a value that is not an unsigned number"))
        };
        let signed = || -> Result<i32, Error> {
            std::str::from_utf8(value)
                .ok()
                .and_then(|text| text.parse::<i64>().ok())
                .map(|n| n.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
                .ok_or(Error::Invalid("a value that is not a number"))
        };
        let letter = || -> Result<u8, Error> {
            match value {
                [c] if c.is_ascii_alphabetic() => Ok(*c),
                _ => Err(Error::Invalid("a value that is not a letter")),
            }
        };
        match key {
            b'a' => {
                self.action = match letter()? {
                    b't' => Action::Transmit,
                    b'T' => Action::TransmitDisplay,
                    b'p' => Action::Put,
                    b'd' => Action::Delete,
                    b'q' => Action::Query,
                    b'f' | b'a' | b'c' => Action::Animation,
                    _ => return Err(Error::Invalid("an action the protocol does not have")),
                }
            }
            b'f' => self.format = unsigned()?,
            b't' => {
                self.medium = match letter()? {
                    b'd' => Medium::Direct,
                    b'f' | b't' | b's' => Medium::Elsewhere,
                    _ => return Err(Error::Invalid("a transmission medium the protocol does not have")),
                }
            }
            b's' => self.width = unsigned()?,
            b'v' => self.height = unsigned()?,
            b'o' => self.zlib = letter()? == b'z',
            b'm' => self.more = unsigned()? == 1,
            b'q' => {
                self.quiet = match unsigned()? {
                    0 => Quiet::Normal,
                    1 => Quiet::NoOk,
                    _ => Quiet::Silent,
                }
            }
            b'i' => self.id = unsigned()?,
            b'I' => self.number = unsigned()?,
            b'p' => self.placement = unsigned()?,
            b'x' => self.x = unsigned()?,
            b'y' => self.y = unsigned()?,
            b'w' => self.w = unsigned()?,
            b'h' => self.h = unsigned()?,
            b'X' => self.offset_x = unsigned()?,
            b'Y' => self.offset_y = unsigned()?,
            b'c' => self.cols = unsigned()?,
            b'r' => self.rows = unsigned()?,
            b'z' => self.z = signed()?,
            b'C' => self.cursor_stays = unsigned()? == 1,
            b'U' => self.unicode_placeholder = unsigned()? == 1,
            b'P' | b'Q' | b'H' | b'V' => {
                signed()?;
                self.relative = true;
            }
            b'd' => {
                let c = letter()?;
                self.free = c.is_ascii_uppercase();
                self.selector = match c.to_ascii_lowercase() {
                    b'a' => Selector::Visible,
                    b'i' => Selector::ById,
                    b'n' => Selector::ByNumber,
                    b'c' => Selector::AtCursor,
                    b'f' => Selector::Frames,
                    b'p' => Selector::AtCell,
                    b'q' => Selector::AtCellWithZ,
                    b'r' => Selector::IdRange,
                    b'x' => Selector::InColumn,
                    b'y' => Selector::InRow,
                    b'z' => Selector::WithZ,
                    _ => return Err(Error::Invalid("a delete the protocol does not have")),
                };
            }
            // `S` and `O` (the size and offset of a file read), animation keys, … : not acted on.
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Command, Medium, Quiet, Selector};
    use crate::terminal::graphics::reply::Error;

    fn parse(text: &str) -> Command {
        let (command, error) = Command::parse(text.as_bytes());
        assert_eq!(error, None, "{text}");
        command
    }

    #[test]
    fn an_empty_command_is_a_transmission_of_rgba_with_the_protocols_defaults() {
        let c = parse("");
        assert_eq!(
            (c.action, c.format, c.medium, c.quiet),
            (Action::Transmit, 32, Medium::Direct, Quiet::Normal)
        );
        assert_eq!((c.id, c.number, c.placement, c.z), (0, 0, 0, 0));
        assert!(!c.more && !c.zlib && !c.cursor_stays && !c.unicode_placeholder);
        assert_eq!((c.selector, c.free), (Selector::Visible, false));
    }

    #[test]
    fn every_key_is_read() {
        let c = parse(
            "a=T,f=100,t=d,s=10,v=20,o=z,m=1,q=2,i=31,p=7,x=1,y=2,w=3,h=4,X=5,Y=6,c=8,r=9,z=-3,C=1,U=1,S=99,O=3",
        );
        assert_eq!((c.action, c.format, c.medium), (Action::TransmitDisplay, 100, Medium::Direct));
        assert_eq!((c.width, c.height, c.zlib, c.more, c.quiet), (10, 20, true, true, Quiet::Silent));
        assert_eq!((c.id, c.number, c.placement), (31, 0, 7));
        assert_eq!((c.x, c.y, c.w, c.h, c.offset_x, c.offset_y), (1, 2, 3, 4, 5, 6));
        assert_eq!((c.cols, c.rows, c.z, c.cursor_stays, c.unicode_placeholder), (8, 9, -3, true, true));
        assert_eq!(parse("I=5").number, 5);
        assert_eq!(parse("q=1").quiet, Quiet::NoOk);
        assert_eq!(parse("t=f").medium, Medium::Elsewhere);
        assert_eq!(parse("a=q").action, Action::Query);
        assert_eq!(parse("a=f").action, Action::Animation);
        assert!(parse("P=3,H=1").relative);
    }

    #[test]
    fn every_delete_selector_in_both_cases() {
        let all = [
            ('a', Selector::Visible),
            ('i', Selector::ById),
            ('n', Selector::ByNumber),
            ('c', Selector::AtCursor),
            ('f', Selector::Frames),
            ('p', Selector::AtCell),
            ('q', Selector::AtCellWithZ),
            ('r', Selector::IdRange),
            ('x', Selector::InColumn),
            ('y', Selector::InRow),
            ('z', Selector::WithZ),
        ];
        for (letter, selector) in all {
            let lower = parse(&format!("a=d,d={letter}"));
            assert_eq!((lower.selector, lower.free), (selector, false), "{letter}");
            let upper = parse(&format!("a=d,d={}", letter.to_ascii_uppercase()));
            assert_eq!((upper.selector, upper.free), (selector, true), "{letter}");
        }
    }

    #[test]
    fn bad_input_is_an_error_that_keeps_the_ids_read_so_far() {
        for bad in ["a", "i=x", "i=-1", "a=TT", "a=z", "t=x", "d=!", "s=99999999999", "=1", "i=1,I=2"] {
            let (_, error) = Command::parse(bad.as_bytes());
            assert!(matches!(error, Some(Error::Invalid(_))), "{bad}");
        }
        let (command, error) = Command::parse(b"i=4,p=2,a=Q,q=2");
        assert!(error.is_some());
        assert_eq!((command.id, command.placement, command.quiet), (4, 2, Quiet::Silent));
        let long = vec![b'x'; 5000];
        assert!(Command::parse(&long).1.is_some());
        assert!(Command::parse(&[b'i', b'=', 0xFF]).1.is_some());
        assert_eq!(Command::parse(b"z=99999999999").0.z, i32::MAX, "signed values are clamped");
        assert_eq!(Command::parse(b"k=7,j=x").1, None, "keys the protocol does not have are skipped");
    }
}

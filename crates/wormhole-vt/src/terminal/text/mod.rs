//! Text: printing characters (ASCII runs, wide characters, clusters), the character sets they go
//! through, and SGR, which sets how they look.

pub(super) mod charset;
pub(super) mod print;
pub(super) mod sgr;
pub(super) mod sgr_stack;

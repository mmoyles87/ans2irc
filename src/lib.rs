//! Convert ANSI art to mIRC-coloured text, one IRC message per line, in as
//! few bytes as the picture allows.

pub mod ansi;
pub mod cp437;
pub mod decode;
pub mod encode;
pub mod fit;
pub mod palette;
pub mod sauce;

/// One character cell: a glyph and its mIRC foreground and background.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: u8,
    pub bg: u8,
}

/// IRC's limit on a message, line ending included.
pub const MESSAGE_LIMIT: usize = 512;

/// Bytes left for the text of a message from `mask` (nick!user@host) to
/// `target`, as the people receiving it see it.
pub fn budget_for(mask: &str, target: &str) -> usize {
    let overhead = format!(":{mask} PRIVMSG {target} :\r\n").len();
    MESSAGE_LIMIT.saturating_sub(overhead)
}

#[cfg(test)]
mod tests {
    #[test]
    fn budget() {
        assert_eq!(super::budget_for("nick!user@host.example", "#chan"), 512 - 41);
    }
}

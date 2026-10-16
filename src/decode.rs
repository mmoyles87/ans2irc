//! Read mIRC formatting back into cells, to check what a client will show.

use crate::Cell;

/// A cell as a client draws it; None is the client's default colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drawn {
    pub ch: char,
    pub fg: Option<u8>,
    pub bg: Option<u8>,
}

fn digits(chars: &[char], i: &mut usize) -> Option<u8> {
    let mut n = None;
    for _ in 0..2 {
        match chars.get(*i).and_then(|c| c.to_digit(10)) {
            Some(d) => {
                n = Some(n.unwrap_or(0) * 10 + d as u8);
                *i += 1;
            }
            None => break,
        }
    }
    n
}

fn colour(n: u8) -> Option<u8> {
    if n < 99 {
        Some(n)
    } else {
        None
    }
}

/// The cells an IRC line draws. Formatting other than colour is skipped.
pub fn decode(line: &str) -> Vec<Drawn> {
    let chars: Vec<char> = line.chars().collect();
    let (mut fg, mut bg) = (None, None);
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\x03' => {
                i += 1;
                match digits(&chars, &mut i) {
                    None => (fg, bg) = (None, None),
                    Some(f) => {
                        fg = colour(f);
                        if chars.get(i) == Some(&',') && chars.get(i + 1).is_some_and(char::is_ascii_digit) {
                            i += 1;
                            bg = digits(&chars, &mut i).and_then(colour);
                        }
                    }
                }
            }
            '\x0f' => {
                (fg, bg) = (None, None);
                i += 1;
            }
            '\x02' | '\x11' | '\x16' | '\x1d' | '\x1e' | '\x1f' => i += 1,
            ch => {
                out.push(Drawn { ch, fg, bg });
                i += 1;
            }
        }
    }
    out
}

#[derive(PartialEq, Debug)]
enum Look {
    Solid(Option<u8>),
    Half(Option<u8>, Option<u8>),
    Split(Option<u8>, Option<u8>),
    Shade(u8, Option<u8>, Option<u8>),
    Glyph(char, Option<u8>, Option<u8>),
}

fn look(ch: char, fg: Option<u8>, bg: Option<u8>, swap_shades: bool) -> Look {
    let half = |top, bottom| {
        if top == bottom {
            Look::Solid(top)
        } else {
            Look::Half(top, bottom)
        }
    };
    let split = |l, r| if l == r { Look::Solid(l) } else { Look::Split(l, r) };
    match ch {
        ' ' | '\u{a0}' => Look::Solid(bg),
        '█' => Look::Solid(fg),
        _ if fg == bg && fg.is_some() => Look::Solid(fg),
        '▀' => half(fg, bg),
        '▄' => half(bg, fg),
        '▌' => split(fg, bg),
        '▐' => split(bg, fg),
        '░' if swap_shades => Look::Shade(1, fg, bg),
        '▓' if swap_shades => Look::Shade(1, bg, fg),
        '▒' if swap_shades => Look::Shade(2, fg.min(bg), fg.max(bg)),
        _ => Look::Glyph(ch, fg, bg),
    }
}

/// Whether a client shows `drawn` the way `cell` looks.
pub fn looks_same(cell: &Cell, drawn: &Drawn, swap_shades: bool) -> bool {
    look(cell.ch, Some(cell.fg), Some(cell.bg), swap_shades) == look(drawn.ch, drawn.fg, drawn.bg, swap_shades)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_codes() {
        let d = decode("\x034,12a\x0309\x37\x03b\x02\x02,");
        assert_eq!(
            d,
            [
                Drawn {
                    ch: 'a',
                    fg: Some(4),
                    bg: Some(12)
                },
                Drawn {
                    ch: '7',
                    fg: Some(9),
                    bg: Some(12)
                },
                Drawn {
                    ch: 'b',
                    fg: None,
                    bg: None
                },
                Drawn {
                    ch: ',',
                    fg: None,
                    bg: None
                },
            ]
        );
    }
}

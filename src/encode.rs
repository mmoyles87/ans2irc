//! Write a row of cells as one IRC line in as few bytes as possible.
//!
//! Many cells can be drawn more than one way: a space shows only its
//! background, so it looks the same as a full block in that colour; a top
//! half block is a bottom half block with the colours swapped; any glyph whose
//! colours match is a solid cell. Which way is shortest depends on the colour
//! codes around it, so the encoder searches all of them for the line with the
//! fewest bytes. Colour numbers are written with one digit wherever the next
//! character can't be read as part of the number.

use crate::Cell;

/// Colours the client is drawing with; None is its own default.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct State {
    fg: Option<u8>,
    bg: Option<u8>,
}

const START: State = State { fg: None, bg: None };

/// What a way of drawing a cell needs: Some(colour), or None for "anything".
#[derive(Clone, Copy)]
struct Way {
    glyph: char,
    fg: Option<u8>,
    bg: Option<u8>,
}

#[derive(Clone, Copy, Default)]
pub struct Options {
    /// Also swap ░ for ▓ with the colours reversed, and ▒ for itself. The
    /// shading patterns differ a little between the two, so it's opt-in.
    pub swap_shades: bool,
}

fn blank(ch: char) -> bool {
    ch == ' ' || ch == '\u{a0}'
}

/// The ways to draw a cell: at most two.
fn ways(cell: &Cell, opts: &Options) -> ([Way; 2], usize) {
    let (f, b) = (cell.fg, cell.bg);
    let solid = |c| {
        (
            [
                Way {
                    glyph: ' ',
                    fg: None,
                    bg: Some(c),
                },
                Way {
                    glyph: '█',
                    fg: Some(c),
                    bg: None,
                },
            ],
            2,
        )
    };
    if blank(cell.ch) {
        return solid(b);
    }
    if cell.ch == '█' || f == b {
        return solid(f);
    }
    let pair = |a: char, z: char| {
        (
            [
                Way {
                    glyph: a,
                    fg: Some(f),
                    bg: Some(b),
                },
                Way {
                    glyph: z,
                    fg: Some(b),
                    bg: Some(f),
                },
            ],
            2,
        )
    };
    match cell.ch {
        '▀' => pair('▀', '▄'),
        '▄' => pair('▄', '▀'),
        '▌' => pair('▌', '▐'),
        '▐' => pair('▐', '▌'),
        '░' if opts.swap_shades => pair('░', '▓'),
        '▓' if opts.swap_shades => pair('▓', '░'),
        '▒' if opts.swap_shades => pair('▒', '▒'),
        ch => {
            let way = Way {
                glyph: ch,
                fg: Some(f),
                bg: Some(b),
            };
            ([way, way], 1)
        }
    }
}

/// A run of formatting codes, kept on the stack: the longest is six bytes.
#[derive(Clone, Copy)]
struct Code {
    buf: [u8; 8],
    len: u8,
}

impl Code {
    const EMPTY: Code = Code { buf: [0; 8], len: 0 };

    fn push(&mut self, b: u8) {
        self.buf[self.len as usize] = b;
        self.len += 1;
    }

    /// A colour number, with a leading zero only when `next` is a digit.
    fn number(&mut self, n: Option<u8>, next: char) {
        let n = n.unwrap_or(99);
        if n >= 10 || next.is_ascii_digit() {
            self.push(b'0' + n / 10);
        }
        self.push(b'0' + n % 10);
    }

    fn colour(fg: Option<u8>, bg: Option<u8>, next: char) -> Code {
        let mut c = Code::EMPTY;
        c.push(0x03);
        match bg {
            Some(b) => {
                c.number(fg, ',');
                c.push(b',');
                c.number(Some(b), next);
            }
            None => c.number(fg, next),
        }
        c
    }

    fn bytes(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }
}

/// The ways to get from `cur` to colours that draw `way`: (codes, new state).
fn moves(cur: State, way: &Way, out: &mut Vec<(Code, State)>) {
    out.clear();
    let g = way.glyph;
    let bg_change = way.bg.filter(|&b| cur.bg != Some(b));
    let fg_change = way.fg.filter(|&f| cur.fg != Some(f));
    if let Some(b) = bg_change {
        // a background needs a foreground number in front of it; when the
        // glyph doesn't care which, try the one already set and short ones
        let mut fgs = [way.fg.or(cur.fg), None, None];
        if way.fg.is_none() {
            fgs[1] = Some(b);
            if cur.fg.is_some_and(|f| f >= 10) {
                fgs[2] = Some(1);
            }
        }
        for (i, &f) in fgs.iter().enumerate() {
            if i > 0 && (f.is_none() || fgs[..i].contains(&f)) {
                continue;
            }
            out.push((Code::colour(f, Some(b), g), State { fg: f, bg: Some(b) }));
        }
        return;
    }
    let Some(f) = fg_change else {
        out.push((Code::EMPTY, cur));
        return;
    };
    if g == ',' {
        // "\x034," would read the comma as the start of a background
        if let Some(b) = cur.bg {
            out.push((
                Code::colour(Some(f), Some(b), g),
                State {
                    fg: Some(f),
                    bg: Some(b),
                },
            ));
        }
        let mut c = Code::colour(Some(f), None, 'x');
        c.push(0x02);
        c.push(0x02);
        out.push((
            c,
            State {
                fg: Some(f),
                bg: cur.bg,
            },
        ));
    } else {
        out.push((
            Code::colour(Some(f), None, g),
            State {
                fg: Some(f),
                bg: cur.bg,
            },
        ));
    }
}

/// A trailing space needs a reset after it, or clients that strip trailing
/// whitespace lose its background.
const END_GUARD: &str = "\x0f";

/// (colour state, last glyph a space)
type Key = (State, bool);

#[derive(Clone, Copy)]
struct Entry {
    key: Key,
    cost: u32,
    prev: u32,
    code: Code,
    glyph: char,
}

/// The cheapest encoding of `cells`: its length, and the line itself when
/// `path` is set.
fn search(cells: &[Cell], opts: &Options, path: bool) -> (usize, String) {
    if cells.is_empty() {
        return (0, String::new());
    }
    let start = Entry {
        key: (START, false),
        cost: 0,
        prev: 0,
        code: Code::EMPTY,
        glyph: ' ',
    };
    let mut layers: Vec<Vec<Entry>> = Vec::new();
    let mut cur = vec![start];
    let mut next: Vec<Entry> = Vec::new();
    let mut mv = Vec::with_capacity(3);
    for cell in cells {
        let (options, n) = ways(cell, opts);
        next.clear();
        for (i, e) in cur.iter().enumerate() {
            for way in &options[..n] {
                moves(e.key.0, way, &mut mv);
                for &(code, state) in &mv {
                    let cost = e.cost + code.len as u32 + way.glyph.len_utf8() as u32;
                    let key = (state, way.glyph == ' ');
                    let entry = Entry {
                        key,
                        cost,
                        prev: i as u32,
                        code,
                        glyph: way.glyph,
                    };
                    match next.iter_mut().find(|x| x.key == key) {
                        Some(x) if cost < x.cost => *x = entry,
                        Some(_) => {}
                        None => next.push(entry),
                    }
                }
            }
        }
        // any state is one colour code (at most 6 bytes) from any other, so
        // one further than that behind the best can't win
        let best = next.iter().map(|e| e.cost).min().unwrap_or(0);
        next.retain(|e| e.cost <= best + 6);
        std::mem::swap(&mut cur, &mut next);
        if path {
            layers.push(cur.clone());
        }
    }
    let finish = |e: &Entry| e.cost as usize + if e.key.1 { END_GUARD.len() } else { 0 };
    let (mut at, last) = cur.iter().enumerate().min_by_key(|(_, e)| (finish(e), e.key)).unwrap();
    let total = finish(last);
    if !path {
        return (total, String::new());
    }
    let mut bytes = Vec::with_capacity(total);
    for layer in layers.iter().rev() {
        let e = &layer[at];
        let mut glyph = [0u8; 4];
        bytes.extend(e.glyph.encode_utf8(&mut glyph).as_bytes().iter().rev());
        bytes.extend(e.code.bytes().iter().rev());
        at = e.prev as usize;
    }
    bytes.reverse();
    if last.key.1 {
        bytes.extend(END_GUARD.as_bytes());
    }
    (total, String::from_utf8(bytes).expect("codes and glyphs are UTF-8"))
}

/// The shortest IRC line that draws `cells`.
pub fn encode(cells: &[Cell], opts: &Options) -> String {
    search(cells, opts, true).1
}

/// The length of encode(cells), without building it.
pub fn encoded_len(cells: &[Cell], opts: &Options) -> usize {
    search(cells, opts, false).0
}

/// Plain text, no colour.
pub fn plain(cells: &[Cell]) -> String {
    let mut s: String = cells.iter().map(|c| c.ch).collect();
    s.truncate(s.trim_end().len());
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode;

    fn cells(spec: &[(char, u8, u8)]) -> Vec<Cell> {
        spec.iter().map(|&(ch, fg, bg)| Cell { ch, fg, bg }).collect()
    }

    fn same_look(a: &[Cell], line: &str) {
        let got = decode(line);
        assert_eq!(got.len(), a.len(), "{line:?}");
        for (want, got) in a.iter().zip(&got) {
            assert!(
                crate::decode::looks_same(want, got, false),
                "{want:?} vs {got:?} in {line:?}"
            );
        }
    }

    #[test]
    fn single_digit_unless_a_digit_follows() {
        let row = cells(&[('a', 4, 1), ('7', 9, 1)]);
        let line = encode(&row, &Options::default());
        assert_eq!(line, "\x034,1a\x0309\x37");
        same_look(&row, &line);
    }

    #[test]
    fn spaces_and_blocks_swap() {
        // a red block between red-background spaces needs no colour change
        let row = cells(&[(' ', 15, 4), ('█', 4, 1), (' ', 15, 4)]);
        let line = encode(&row, &Options::default());
        assert_eq!(line, "\x034,4   \x0f");
        same_look(&row, &line);
    }

    #[test]
    fn half_blocks_swap() {
        let row = cells(&[('▀', 4, 2), ('▄', 2, 4), ('▀', 4, 2)]);
        let line = encode(&row, &Options::default());
        assert_eq!(line, "\x032,4▄▄▄");
        same_look(&row, &line);
    }

    #[test]
    fn comma_after_a_colour() {
        let row = cells(&[('a', 4, 1), (',', 9, 1)]);
        let line = encode(&row, &Options::default());
        same_look(&row, &line);
    }

    #[test]
    fn trailing_space_keeps_its_background() {
        let row = cells(&[('x', 4, 2), (' ', 0, 2)]);
        let line = encode(&row, &Options::default());
        assert!(line.ends_with(' ') || line.ends_with('█') || line.ends_with('\x0f'));
        same_look(&row, &line);
    }
}

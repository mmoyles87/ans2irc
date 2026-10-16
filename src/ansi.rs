//! Draw an ANSI file (CP437 text with escape codes) onto a grid of cells.

use crate::cp437;
use crate::palette::{nearest, xterm_rgb, ANSI_BRIGHT, ANSI_NORMAL};
use crate::sauce;
use crate::Cell;

/// A broken file's cursor jumps must not make the canvas eat memory.
pub const MAX_ROWS: usize = 5000;
pub const MAX_COLS: usize = 400;

pub struct Options {
    /// Columns; None takes the SAUCE width, then `default_width`.
    pub width: Option<usize>,
    pub default_width: usize,
    /// Blink as a bright background; None takes the SAUCE flag, then on.
    pub ice: Option<bool>,
    pub tab_size: usize,
    /// Columns to drop from the left and right.
    pub crop_left: usize,
    pub crop_right: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            width: None,
            default_width: 80,
            ice: None,
            tab_size: 8,
            crop_left: 0,
            crop_right: 0,
        }
    }
}

const DEFAULT: Cell = Cell {
    ch: ' ',
    fg: ANSI_NORMAL[7],
    bg: ANSI_NORMAL[0],
};

struct Pen {
    row: usize,
    col: usize,
    wrap: bool,
    fg: usize,
    bg: usize,
    bold: bool,
    blink: bool,
    xfg: Option<u8>,
    xbg: Option<u8>,
    saved: (usize, usize),
}

struct Canvas {
    rows: Vec<Vec<Option<Cell>>>,
    width: usize,
    ice: bool,
    tab_size: usize,
    pen: Pen,
}

impl Canvas {
    fn colours(&self) -> (u8, u8) {
        let p = &self.pen;
        let fg = p
            .xfg
            .unwrap_or(if p.bold { ANSI_BRIGHT[p.fg] } else { ANSI_NORMAL[p.fg] });
        let bg = p.xbg.unwrap_or(if p.blink && self.ice {
            ANSI_BRIGHT[p.bg]
        } else {
            ANSI_NORMAL[p.bg]
        });
        (fg, bg)
    }

    fn put(&mut self, ch: char) {
        if self.pen.wrap {
            self.pen.row += 1;
            self.pen.col = 0;
            self.pen.wrap = false;
        }
        if self.pen.row >= MAX_ROWS {
            return;
        }
        while self.rows.len() <= self.pen.row {
            self.rows.push(vec![None; self.width]);
        }
        let (fg, bg) = self.colours();
        self.rows[self.pen.row][self.pen.col] = Some(Cell { ch, fg, bg });
        if self.pen.col == self.width - 1 {
            self.pen.wrap = true;
        } else {
            self.pen.col += 1;
        }
    }

    fn sgr(&mut self, params: &[u32]) {
        let p = &mut self.pen;
        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => {
                    p.fg = 7;
                    p.bg = 0;
                    p.bold = false;
                    p.blink = false;
                    p.xfg = None;
                    p.xbg = None;
                }
                1 => p.bold = true,
                5 => p.blink = true,
                21 | 22 => p.bold = false,
                25 => p.blink = false,
                n @ 30..=37 => {
                    p.fg = (n - 30) as usize;
                    p.xfg = None;
                }
                n @ 90..=97 => {
                    p.fg = (n - 90) as usize;
                    p.xfg = Some(ANSI_BRIGHT[p.fg]);
                }
                n @ 40..=47 => {
                    p.bg = (n - 40) as usize;
                    p.xbg = None;
                }
                n @ 100..=107 => {
                    p.bg = (n - 100) as usize;
                    p.xbg = Some(ANSI_BRIGHT[p.bg]);
                }
                39 => {
                    p.fg = 7;
                    p.xfg = None;
                }
                49 => {
                    p.bg = 0;
                    p.xbg = None;
                }
                n @ (38 | 48) => {
                    let colour = match params.get(i + 1) {
                        Some(5) if i + 2 < params.len() => {
                            let c = nearest(xterm_rgb(params[i + 2].min(255) as u8));
                            i += 2;
                            c
                        }
                        Some(2) if i + 4 < params.len() => {
                            let [r, g, b] = [params[i + 2], params[i + 3], params[i + 4]].map(|v| v.min(255));
                            i += 4;
                            nearest((r << 16) | (g << 8) | b)
                        }
                        _ => break,
                    };
                    if n == 38 {
                        p.xfg = Some(colour);
                    } else {
                        p.xbg = Some(colour);
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn csi(&mut self, fin: u8, raw: &[u8]) {
        let raw: Vec<u8> = raw.iter().copied().skip_while(|b| b"?=>".contains(b)).collect();
        let params: Vec<u32> = if raw.is_empty() {
            Vec::new()
        } else {
            raw.split(|&b| b == b';')
                .map(|p| {
                    if !p.is_empty() && p.iter().all(u8::is_ascii_digit) {
                        p.iter()
                            .fold(0u32, |n, d| n.saturating_mul(10).saturating_add((d - b'0') as u32))
                    } else {
                        0
                    }
                })
                .collect()
        };
        let first = params.first().copied().unwrap_or(0);
        let n = first.max(1) as usize;
        let last_col = self.width - 1;
        if fin == b'm' {
            self.sgr(if params.is_empty() { &[0] } else { &params });
            return;
        }
        if fin == b't' && params.len() == 4 {
            // PabloDraw 24-bit colour
            let [r, g, b] = [params[1], params[2], params[3]].map(|v| v.min(255));
            let colour = nearest((r << 16) | (g << 8) | b);
            if params[0] == 1 {
                self.pen.xfg = Some(colour);
            } else {
                self.pen.xbg = Some(colour);
            }
            return;
        }
        let p = &mut self.pen;
        if b"ABCDHfJKsu".contains(&fin) {
            p.wrap = false;
        }
        match fin {
            b'A' => p.row = p.row.saturating_sub(n),
            b'B' => p.row = p.row.saturating_add(n),
            b'C' => p.col = (p.col + n).min(last_col),
            b'D' => p.col = p.col.saturating_sub(n),
            b'H' | b'f' => {
                p.row = (params.first().copied().unwrap_or(1) as usize).saturating_sub(1);
                p.col = (params.get(1).copied().unwrap_or(1) as usize)
                    .saturating_sub(1)
                    .min(last_col);
            }
            b'J' if first == 2 => {
                self.rows.clear();
                p.row = 0;
                p.col = 0;
            }
            b'K' => {
                if let Some(row) = self.rows.get_mut(p.row) {
                    let span = match first {
                        0 => p.col..self.width,
                        1 => 0..p.col + 1,
                        _ => 0..self.width,
                    };
                    for c in span {
                        row[c] = None;
                    }
                }
            }
            b's' => p.saved = (p.row, p.col),
            b'u' => (p.row, p.col) = p.saved,
            _ => {}
        }
    }
}

/// Draw `data` and return its rows, with the trailing black spaces of each
/// row and the blank rows above and below the art dropped.
pub fn render(data: &[u8], opts: &Options) -> Vec<Vec<Cell>> {
    let (data, sauce) = sauce::split(data);
    let width = opts
        .width
        .or(sauce.width)
        .unwrap_or(opts.default_width)
        .clamp(1, MAX_COLS);
    let mut c = Canvas {
        rows: Vec::new(),
        width,
        ice: opts.ice.or(sauce.ice).unwrap_or(true),
        tab_size: opts.tab_size.max(1),
        pen: Pen {
            row: 0,
            col: 0,
            wrap: false,
            fg: 7,
            bg: 0,
            bold: false,
            blink: false,
            xfg: None,
            xbg: None,
            saved: (0, 0),
        },
    };
    let n = data.len();
    let mut i = 0;
    while i < n {
        let b = data[i];
        if b == 0x1b {
            if i + 1 < n && data[i + 1] == b'[' {
                let mut j = i + 2;
                while j < n && (0x30..=0x3f).contains(&data[j]) {
                    j += 1; // parameters
                }
                let mut k = j;
                while k < n && (0x20..=0x2f).contains(&data[k]) {
                    k += 1; // intermediates
                }
                if k >= n {
                    break;
                }
                if k == j {
                    c.csi(data[k], &data[i + 2..j]);
                }
                i = k + 1;
            } else if i + 1 < n && b"()*+".contains(&data[i + 1]) {
                i += 3; // ESC ( B: character set
            } else {
                i += 2;
            }
            continue;
        }
        i += 1;
        match b {
            0x0a => {
                c.pen.row += 1;
                c.pen.col = 0;
                c.pen.wrap = false;
            }
            0x0d => {
                c.pen.col = 0;
                c.pen.wrap = false;
            }
            0x09 => {
                let t = c.tab_size;
                c.pen.col = (c.pen.col + t - c.pen.col % t).min(width - 1);
                c.pen.wrap = false;
            }
            0x00 => c.put(' '),
            _ => c.put(cp437::glyph(b)),
        }
    }

    let start = opts.crop_left.min(width);
    let end = width.saturating_sub(opts.crop_right).max(start);
    let mut rows: Vec<Vec<Cell>> = c
        .rows
        .into_iter()
        .map(|row| {
            let mut cells: Vec<Cell> = row[start..end].iter().map(|c| c.unwrap_or(DEFAULT)).collect();
            // trailing black space is invisible on the black background art assumes
            while cells.last().is_some_and(|c| c.ch == ' ' && c.bg == ANSI_NORMAL[0]) {
                cells.pop();
            }
            cells
        })
        .collect();
    while rows.last().is_some_and(Vec::is_empty) {
        rows.pop();
    }
    let lead = rows.iter().take_while(|r| r.is_empty()).count();
    rows.drain(..lead);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(rows: &[Vec<Cell>]) -> Vec<String> {
        rows.iter().map(|r| r.iter().map(|c| c.ch).collect()).collect()
    }

    #[test]
    fn colours_and_wrap() {
        let rows = render(
            b"\x1b[1;31mA\x1b[0;44mB",
            &Options {
                width: Some(1),
                ..Default::default()
            },
        );
        assert_eq!(text(&rows), ["A", "B"]);
        assert_eq!((rows[0][0].fg, rows[0][0].bg), (4, 1));
        assert_eq!((rows[1][0].fg, rows[1][0].bg), (15, 2));
    }

    #[test]
    fn ice_blink_is_bright_background() {
        let rows = render(b"\x1b[5;41mX", &Options::default());
        assert_eq!(rows[0][0].bg, 4);
        let rows = render(
            b"\x1b[5;41mX",
            &Options {
                ice: Some(false),
                ..Default::default()
            },
        );
        assert_eq!(rows[0][0].bg, 5);
    }

    #[test]
    fn cursor_moves_and_trims() {
        let rows = render(b"\n\nab\x1b[2Dc\x1b[1;5Hd   \n\n", &Options::default());
        assert_eq!(text(&rows), ["    d", "", "cb"]);
    }

    #[test]
    fn xterm_and_truecolour() {
        let rows = render(b"\x1b[38;5;196m\x1b[48;2;0;0;255mZ", &Options::default());
        assert_eq!((rows[0][0].fg, rows[0][0].bg), (4, 60));
    }
}

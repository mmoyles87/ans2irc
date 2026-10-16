//! Make a line fit in an IRC message.
//!
//! When the shortest encoding is still too long, colours are merged first:
//! the run of cells whose recolouring to a neighbour's colours changes the
//! picture least for each byte it saves goes first, and so on until the line
//! fits. Only if that runs out are cells cut from the right.

use crate::encode::{encode, encoded_len, Options};
use crate::palette::distance;
use crate::Cell;

#[derive(Debug, Default, PartialEq)]
pub struct Fitted {
    pub line: String,
    /// Cells recoloured to make it fit.
    pub recoloured: usize,
    /// Cells cut from the right.
    pub cut: usize,
}

/// How much of a cell is foreground, roughly.
fn ink(ch: char) -> f64 {
    match ch {
        ' ' | '\u{a0}' => 0.0,
        '█' => 1.0,
        '▀' | '▄' | '▌' | '▐' | '▒' => 0.5,
        '░' => 0.25,
        '▓' => 0.75,
        _ => 0.3,
    }
}

/// The colours a cell shows: a space's foreground and a block's background
/// don't show, so they're set to the other one.
fn shown(cell: &Cell) -> (u8, u8) {
    match cell.ch {
        ' ' | '\u{a0}' => (cell.bg, cell.bg),
        '█' => (cell.fg, cell.fg),
        _ => (cell.fg, cell.bg),
    }
}

fn change(cell: &Cell, fg: u8, bg: u8) -> f64 {
    let k = ink(cell.ch);
    k * distance(cell.fg, fg) + (1.0 - k) * distance(cell.bg, bg)
}

struct Merge {
    score: f64,
    cells: Vec<Cell>,
    count: usize,
}

/// The recolouring of one run to a neighbour's colours that loses the least
/// for each byte it saves. With `forced`, the least change of any, saving or
/// not, so a line that no single merge shortens still gets somewhere.
fn best_merge(cells: &[Cell], size: usize, opts: &Options, forced: bool) -> Option<Merge> {
    let mut runs = Vec::new();
    let mut start = 0;
    for i in 1..=cells.len() {
        if i == cells.len() || shown(&cells[i]) != shown(&cells[start]) {
            runs.push(start..i);
            start = i;
        }
    }
    if runs.len() < 2 {
        return None;
    }
    let mut best: Option<Merge> = None;
    for (r, run) in runs.iter().enumerate() {
        let (own_fg, own_bg) = shown(&cells[run.start]);
        let neighbours = [
            r.checked_sub(1).map(|p| runs[p].start),
            runs.get(r + 1).map(|n| n.start),
        ];
        for &at in neighbours.iter().flatten() {
            let (nf, nb) = shown(&cells[at]);
            // the whole look of the neighbour, or just one of its colours
            let all = [(nf, nb), (nf, own_bg), (own_fg, nb)];
            let targets = if forced { &all[..1] } else { &all[..] };
            for &(fg, bg) in targets {
                if (fg, bg) == (own_fg, own_bg) {
                    continue;
                }
                let mut trial = cells.to_vec();
                let mut cost = 0.0;
                for c in &mut trial[run.clone()] {
                    cost += change(c, fg, bg);
                    c.fg = fg;
                    c.bg = bg;
                }
                let saved = size.saturating_sub(encoded_len(&trial, opts));
                if saved == 0 && !forced {
                    continue;
                }
                let score = if forced { cost } else { cost / saved as f64 };
                if best.as_ref().map_or(true, |b| score < b.score) {
                    best = Some(Merge {
                        score,
                        cells: trial,
                        count: run.len(),
                    });
                }
            }
        }
    }
    best
}

/// The shortest line for `cells` that fits in `budget` bytes.
pub fn fit(cells: &[Cell], budget: usize, opts: &Options) -> Fitted {
    let mut size = encoded_len(cells, opts);
    if size <= budget {
        return Fitted {
            line: encode(cells, opts),
            ..Default::default()
        };
    }
    let mut cells = cells.to_vec();
    let mut recoloured = 0;
    while size > budget {
        let Some(m) = best_merge(&cells, size, opts, false).or_else(|| best_merge(&cells, size, opts, true)) else {
            break;
        };
        cells = m.cells;
        recoloured += m.count;
        size = encoded_len(&cells, opts);
    }
    // still too long: cut from the right, keeping as much as fits
    let mut keep = cells.len();
    if size > budget {
        let (mut lo, mut hi) = (0, cells.len());
        while lo < hi {
            let mid = (lo + hi + 1) / 2;
            if encoded_len(&cells[..mid], opts) <= budget {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        keep = lo;
    }
    Fitted {
        line: encode(&cells[..keep], opts),
        recoloured,
        cut: cells.len() - keep,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stripes(n: usize) -> Vec<Cell> {
        (0..n)
            .map(|i| Cell {
                ch: 'x',
                fg: (i % 2 * 4) as u8,
                bg: 1,
            })
            .collect()
    }

    #[test]
    fn fits_without_changes_when_it_can() {
        let row = stripes(4);
        let f = fit(&row, 400, &Options::default());
        assert_eq!((f.recoloured, f.cut), (0, 0));
        assert_eq!(f.line, encode(&row, &Options::default()));
    }

    #[test]
    fn recolours_before_cutting() {
        let row = stripes(40);
        let full = encode(&row, &Options::default()).len();
        let f = fit(&row, full - 20, &Options::default());
        assert!(f.line.len() <= full - 20);
        assert!(f.recoloured > 0);
        assert_eq!(f.cut, 0);
    }

    #[test]
    fn cuts_when_nothing_else_works() {
        let row: Vec<Cell> = (0..50).map(|_| Cell { ch: 'x', fg: 4, bg: 1 }).collect();
        let f = fit(&row, 20, &Options::default());
        assert!(f.line.len() <= 20);
        assert!(f.cut > 0);
    }
}

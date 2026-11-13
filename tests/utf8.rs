//! UTF-8 .ans files: curly quotes, emoji and Unicode blocks, as a 20-column
//! piece written a character to a glyph, with no cell for the column an
//! emoji covers.

use std::io::Write;
use std::process::{Command, Stdio};

use ans2irc::ansi::{render, Options};
use ans2irc::decode::decode;
use ans2irc::encode::{self, encode};
use ans2irc::fit::fit;
use ans2irc::wide::wide;

const WIDTH: u16 = 20;

fn sauce(width: u16, height: u16) -> Vec<u8> {
    let mut rec = vec![0u8; 128];
    rec[..7].copy_from_slice(b"SAUCE00");
    rec[94] = 1;
    rec[95] = 1;
    rec[96..98].copy_from_slice(&width.to_le_bytes());
    rec[98..100].copy_from_slice(&height.to_le_bytes());
    rec[105] = 1;
    let mut data = vec![0x1a];
    data.extend(rec);
    data
}

fn piece() -> Vec<u8> {
    let mut data = concat!(
        "\x1b[0;1;33;44m▀▄█░▒▓\x1b[0;31m ’quote“ok”\x1b[0m\r\n",
        // 18 characters, 20 columns: the row wraps on its own
        "\x1b[0;37;44myou 🫵😂 lol        \x1b[0m\r\n",
        "\x1b[0;32;41m█▓▒░\x1b[0;1;34m▄▀ 🫵\x1b[0m\r\n",
    )
    .as_bytes()
    .to_vec();
    data.extend(sauce(WIDTH, 3));
    data
}

fn columns(line: &str) -> usize {
    decode(line).iter().map(|d| if wide(d.ch) { 2 } else { 1 }).sum()
}

fn text(line: &str) -> String {
    decode(line).iter().map(|d| d.ch).collect()
}

fn run(args: &[&str], input: &[u8]) -> (Vec<u8>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ans2irc"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().unwrap();
    (out.stdout, String::from_utf8(out.stderr).unwrap())
}

#[test]
fn lines_carry_the_real_characters() {
    let (out, _) = run(&[], &piece());
    let out = String::from_utf8(out).expect("UTF-8 out");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3, "{out:?}");
    assert_eq!(text(lines[0]), "▀▄█░▒▓ ’quote“ok”");
    assert_eq!(text(lines[1]).trim_end(), "you 🫵😂 lol");
    assert!(text(lines[2]).ends_with("▄▀ 🫵"), "{:?}", lines[2]);
    assert!(!out.contains('Γ') && !out.contains('≡'), "read as CP437: {out:?}");
    let first = decode(lines[0]);
    assert_eq!((first[0].fg, first[0].bg), (Some(8), Some(2)));
    assert_eq!((first[7].fg, first[7].bg), (Some(5), Some(1)));
}

#[test]
fn emoji_line_is_the_full_width() {
    let rows = render(&piece(), &Options::default());
    let line = encode(&rows[1], &encode::Options::default());
    assert_eq!(columns(&line), WIDTH as usize, "{line:?}");
    // 18 glyphs: the emoji's second columns have no cell
    assert_eq!(rows[1].len(), 18);
}

#[test]
fn budget_cut_never_splits_a_character() {
    let rows = render(&piece(), &Options::default());
    let opts = encode::Options::default();
    for row in &rows {
        for budget in 0..=encode(row, &opts).len() {
            let f = fit(row, budget, &opts);
            assert!(f.line.len() <= budget, "{budget}: {:?}", f.line);
            // what is left is whole glyphs from the start of the row; a
            // recolour may make a narrow glyph a solid cell, nothing else
            let got = decode(&f.line);
            assert_eq!(got.len(), row.len() - f.cut, "{budget}: {:?}", f.line);
            for (cell, drawn) in row.iter().zip(&got) {
                if drawn.ch != cell.ch {
                    assert!(!wide(cell.ch), "{budget}: {:?}", f.line);
                    assert!(" █▀▄▌▐".contains(drawn.ch), "{budget}: {:?}", f.line);
                }
            }
        }
    }
    // and the same through the program
    for budget in ["10", "15", "20", "25"] {
        let (out, _) = run(&["-m", budget], &piece());
        let out = String::from_utf8(out).expect("a cut split a character");
        let n: usize = budget.parse().unwrap();
        assert!(out.lines().all(|l| l.len() <= n), "{budget}: {out:?}");
    }
}

#[test]
fn wide_glyphs_wrap_at_the_width() {
    let mut data = "x🫵😂🫵😂🫵\r\n".as_bytes().to_vec();
    data.extend(sauce(10, 1));
    let rows = render(&data, &Options::default());
    let text: Vec<String> = rows.iter().map(|r| r.iter().map(|c| c.ch).collect()).collect();
    // the fifth emoji would start in the last column, so it starts the next row
    assert_eq!(text, ["x🫵😂🫵😂", "🫵"]);
    let rows = render(
        "ab😂".as_bytes(),
        &Options {
            width: Some(3),
            ..Default::default()
        },
    );
    assert_eq!(rows.len(), 2);
}

#[test]
fn cursor_moves_count_two_columns() {
    let rows = render("😂\x1b[2Cx\r\n\x1b[1;6Hy".as_bytes(), &Options::default());
    let text: String = rows[0].iter().map(|c| c.ch).collect();
    assert_eq!(text, "😂  xy");
}

#[test]
fn half_a_wide_glyph_overwritten_blanks_the_rest() {
    let rows = render("ab😂cd\x1b[1;4Hz".as_bytes(), &Options::default());
    let text: String = rows[0].iter().map(|c| c.ch).collect();
    assert_eq!(text, "ab zcd");
}

#[test]
fn byte_order_mark_is_dropped() {
    let rows = render("\u{feff}’hi”".as_bytes(), &Options::default());
    let text: String = rows[0].iter().map(|c| c.ch).collect();
    assert_eq!(text, "’hi”");
}

#[test]
fn cp437_stays_cp437() {
    // block bytes are not valid UTF-8, and the curly quote's bytes read as CP437
    let rows = render(b"\xdb\xdf\xb0 \xe2\x80\x99\xdc", &Options::default());
    let text: String = rows[0].iter().map(|c| c.ch).collect();
    assert_eq!(text, "█▀░ ΓÇÖ▄");
    // plain ASCII reads the same either way
    let rows = render(b"\x1b[31mhi", &Options::default());
    assert_eq!(rows[0].len(), 2);
}

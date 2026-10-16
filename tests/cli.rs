use std::io::Write;
use std::process::{Command, Stdio};

fn run(args: &[&str], input: &[u8]) -> (String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ans2irc"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// A row of 80 cells, each a different colour from its neighbour.
fn rainbow() -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..80 {
        data.extend(format!("\x1b[{};{}mx", 30 + i % 8, 40 + (i + 3) % 8).as_bytes());
    }
    data
}

#[test]
fn converts_stdin() {
    let (out, _) = run(&[], b"\x1b[1;31mHi\x1b[0m");
    assert_eq!(out, "\x034,1Hi\n");
}

#[test]
fn every_line_fits_the_budget() {
    let (out, err) = run(&["-v", "-m", "200"], &rainbow());
    assert!(out.lines().all(|l| l.len() <= 200), "{out:?}");
    assert!(err.contains("recoloured"), "{err}");
}

#[test]
fn budget_from_mask_and_target() {
    let (out, _) = run(
        &["--for", "someone!user@a.very.long.host.example.net", "#channel"],
        &rainbow(),
    );
    let budget = 512 - ":someone!user@a.very.long.host.example.net PRIVMSG #channel :\r\n".len();
    assert!(out.lines().all(|l| l.len() <= budget));
}

#[test]
fn plain_text() {
    let (out, _) = run(&["-n"], b"\x1b[31mab\x1b[0m   ");
    assert_eq!(out, "ab\n");
}

#[test]
fn bad_option() {
    let (_, err) = run(&["--nope"], b"");
    assert!(err.contains("unknown option"));
}

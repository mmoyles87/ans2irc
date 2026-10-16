use std::io::{self, Read, Write};
use std::process::ExitCode;

use ans2irc::{ansi, budget_for, encode, fit};

const USAGE: &str = "\
usage: ans2irc [options] [file]

Convert ANSI art (CP437 with escape codes, SAUCE optional) to mIRC-coloured
UTF-8, one IRC message per line. Reads standard input without a file.

  -w, --width N           columns (default: the SAUCE width, else 80)
  -l, --crop-left N       drop N columns from the left
  -r, --crop-right N      drop N columns from the right
  -t, --tab-size N        tab stops every N columns (default: 8)
  -m, --max-bytes N       fit every line in N bytes
  -f, --for MASK TARGET   fit every line for messages from MASK (nick!user@host)
                          to TARGET (#channel or nick)
      --ice, --no-ice     blink is a bright background, or blink
                          (default: the SAUCE flag, else --ice)
  -s, --swap-shades       let the encoder swap ░ for ▓ with the colours reversed
  -n, --no-colour         plain text
  -v, --verbose           report line sizes on standard error
  -h, --help              this help
  -V, --version           the version
";

struct Args {
    file: Option<String>,
    ansi: ansi::Options,
    encode: encode::Options,
    budget: Option<usize>,
    colour: bool,
    verbose: bool,
}

fn number(flag: &str, value: Option<String>) -> Result<usize, String> {
    let value = value.ok_or_else(|| format!("{flag} needs a number"))?;
    value.parse().map_err(|_| format!("{flag}: not a number: {value}"))
}

fn parse(mut argv: impl Iterator<Item = String>) -> Result<Option<Args>, String> {
    let mut a = Args {
        file: None,
        ansi: ansi::Options::default(),
        encode: encode::Options::default(),
        budget: None,
        colour: true,
        verbose: false,
    };
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("ans2irc {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "-w" | "--width" => a.ansi.width = Some(number(&arg, argv.next())?.max(1)),
            "-l" | "--crop-left" => a.ansi.crop_left = number(&arg, argv.next())?,
            "-r" | "--crop-right" => a.ansi.crop_right = number(&arg, argv.next())?,
            "-t" | "--tab-size" => a.ansi.tab_size = number(&arg, argv.next())?.max(1),
            "-m" | "--max-bytes" => a.budget = Some(number(&arg, argv.next())?),
            "-f" | "--for" => {
                let (Some(mask), Some(target)) = (argv.next(), argv.next()) else {
                    return Err(format!("{arg} needs a mask and a target"));
                };
                a.budget = Some(budget_for(&mask, &target));
            }
            "--ice" => a.ansi.ice = Some(true),
            "--no-ice" => a.ansi.ice = Some(false),
            "-s" | "--swap-shades" => a.encode.swap_shades = true,
            "-n" | "--no-colour" | "--no-color" => a.colour = false,
            "-v" | "--verbose" => a.verbose = true,
            "-" => a.file = None,
            s if s.starts_with('-') => return Err(format!("unknown option: {s}")),
            _ if a.file.is_some() => return Err("one file at a time".into()),
            _ => a.file = Some(arg),
        }
    }
    Ok(Some(a))
}

/// Files bigger than this are refused rather than hung on.
const MAX_FILE: u64 = 4 << 20;

fn read(file: &Option<String>) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    match file {
        Some(path) => std::fs::File::open(path)?.take(MAX_FILE + 1).read_to_end(&mut data)?,
        None => io::stdin().lock().take(MAX_FILE + 1).read_to_end(&mut data)?,
    };
    if data.len() as u64 > MAX_FILE {
        return Err(io::Error::new(io::ErrorKind::Other, "bigger than 4 MB, not ANSI art"));
    }
    Ok(data)
}

fn main() -> ExitCode {
    let args = match parse(std::env::args().skip(1)) {
        Ok(Some(a)) => a,
        Ok(None) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ans2irc: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let data = match read(&args.file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("ans2irc: {}: {e}", args.file.as_deref().unwrap_or("stdin"));
            return ExitCode::from(1);
        }
    };
    let rows = ansi::render(&data, &args.ansi);
    let (mut longest, mut recoloured, mut cut) = (0, (0, 0), (0, 0));
    let mut out = io::BufWriter::new(io::stdout().lock());
    for row in &rows {
        let line = if !args.colour {
            encode::plain(row)
        } else if let Some(budget) = args.budget {
            let f = fit::fit(row, budget, &args.encode);
            if f.recoloured > 0 {
                recoloured = (recoloured.0 + 1, recoloured.1 + f.recoloured);
            }
            if f.cut > 0 {
                cut = (cut.0 + 1, cut.1 + f.cut);
            }
            f.line
        } else {
            encode::encode(row, &args.encode)
        };
        longest = longest.max(line.len());
        if writeln!(out, "{line}").is_err() {
            return ExitCode::from(1);
        }
    }
    let _ = out.flush();
    if args.verbose {
        let mut report = format!("{} lines, longest {} bytes", rows.len(), longest);
        if let Some(b) = args.budget {
            report += &format!(" (budget {b})");
        }
        if recoloured.0 > 0 {
            report += &format!(", {} lines recoloured to fit ({} cells)", recoloured.0, recoloured.1);
        }
        eprintln!("ans2irc: {report}");
    }
    if cut.0 > 0 {
        eprintln!(
            "ans2irc: {} lines cut to fit ({} cells lost from the right)",
            cut.0, cut.1
        );
    }
    ExitCode::SUCCESS
}

//! The `veel` command for the language alone: `check`, `fmt` and `print` (Veel's
//! `docs/design/compiler.md`; task 1a-04). It runs without Annilay: `veel check` checks a slice
//! project folder against the spec's section 17, `veel fmt` writes its files in canonical form
//! (section 12), and `veel print` shows one file's canonical form without writing it.
//!
//! [`run`] is the whole command, its output returned rather than printed, so tests drive it as
//! the binary would.

use std::path::{Path, PathBuf};

/// What the command printed and the code it exits with.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// What it printed to standard output.
    pub stdout: String,
    /// What it printed to standard error.
    pub stderr: String,
    /// 0 when all is well, 1 when the check found errors or a file is not canonical, 2 for a
    /// command it cannot run.
    pub code: i32,
}

/// The help `veel --help` prints.
pub const HELP: &str = "\
veel: Veel's command for the language alone (Annilay's project language)

Usage:
  veel check <project>...        Check each project: a folder of .veel files read as one
                                 (or a single .veel file). Prints each diagnostic as
                                 `file:line:column severity[code]: message`, then its notes
                                 and `fix:` suggestions, and exits 1 if any is an error.
  veel fmt [--check] <path>...   Write every .veel file under each path in canonical form
                                 (spec, section 12). With --check, write nothing: list each
                                 file that is not canonical and exit 1 if there is one.
  veel print <file>              Print one file's canonical form; write nothing.
  veel --help | --version

A project is the phase 1a slice of the spec (Veel's docs/spec/veel-v0.md, section 17):
`source` and `sequence` at the top of a file; in a sequence `rate`, `size`, at most one
`timecode` line and `track` lines; in a track `clip` lines, each with `from=@source`, a source
range and `at`; in a clip `effect @handle qtblend` blocks and at most one `crossfade`.
Veelindy's manifest, veelindy.veel at the root, is left to Veelindy.
Times are whole frames: a bare integer is frames at the sequence's rate; `s`, `ms` and
non-drop timecode convert exactly, and a time that is not a whole frame is an error naming the
nearest exact choices. Ranges are half-open: `in..out` or `in..+length`.

Exit codes: 0 clean, 1 errors found or files not canonical, 2 a command that cannot run.
";

/// Runs `veel` with `args` (without the program's name).
pub fn run(args: &[String]) -> Outcome {
    let mut out = Outcome::default();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["--help" | "-h" | "help", ..] => out.stdout.push_str(HELP),
        ["--version" | "-V", ..] => {
            out.stdout = format!("veel {}\n", env!("CARGO_PKG_VERSION"));
        }
        ["check", paths @ ..] if !paths.is_empty() => check(paths, &mut out),
        ["fmt", "--check", paths @ ..] if !paths.is_empty() => fmt(paths, true, &mut out),
        ["fmt", paths @ ..] if !paths.is_empty() && !paths.contains(&"--check") => {
            fmt(paths, false, &mut out);
        }
        ["print", file] => print(Path::new(file), &mut out),
        _ => usage(&mut out, &format!("cannot run `veel {}`", args.join(" "))),
    }
    out
}

fn usage(out: &mut Outcome, why: &str) {
    out.stderr = format!("veel: {why}; `veel --help` lists the commands\n");
    out.code = 2;
}

fn check(paths: &[&str], out: &mut Outcome) {
    for path in paths {
        let path = Path::new(path);
        let checked = if path.is_dir() {
            veel_check::check_folder(path)
        } else {
            std::fs::read_to_string(path).map(|text| {
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                veel_check::check(&[(name, text)])
            })
        };
        match checked {
            Ok(c) => {
                out.stderr.push_str(&c.render());
                if !c.is_clean() && out.code == 0 {
                    out.code = 1;
                }
            }
            Err(e) => {
                out.stderr
                    .push_str(&format!("veel: cannot read {}: {e}\n", path.display()));
                out.code = 2;
            }
        }
    }
}

/// Every `.veel` file at or under `path`, sorted.
fn veel_files(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    if !path.is_dir() {
        std::fs::metadata(path)?;
        return Ok(vec![path.to_path_buf()]);
    }
    Ok(veel_check::project_files(path)?
        .into_iter()
        .map(|(_, p)| p)
        .collect())
}

fn fmt(paths: &[&str], only_check: bool, out: &mut Outcome) {
    for path in paths {
        let files = match veel_files(Path::new(path)) {
            Ok(f) => f,
            Err(e) => {
                out.stderr
                    .push_str(&format!("veel: cannot read {path}: {e}\n"));
                out.code = 2;
                continue;
            }
        };
        for file in files {
            let result = std::fs::read_to_string(&file).and_then(|text| {
                let canonical = veel_print::canonical(&text);
                if canonical == text {
                    return Ok(());
                }
                if only_check {
                    out.stdout
                        .push_str(&format!("{}: not in canonical form\n", file.display()));
                    if out.code == 0 {
                        out.code = 1;
                    }
                    return Ok(());
                }
                std::fs::write(&file, canonical)?;
                out.stdout
                    .push_str(&format!("{}: formatted\n", file.display()));
                Ok(())
            });
            if let Err(e) = result {
                out.stderr
                    .push_str(&format!("veel: cannot format {}: {e}\n", file.display()));
                out.code = 2;
            }
        }
    }
}

fn print(file: &Path, out: &mut Outcome) {
    match std::fs::read_to_string(file) {
        Ok(text) => out.stdout = veel_print::canonical(&text),
        Err(e) => {
            out.stderr = format!("veel: cannot read {}: {e}\n", file.display());
            out.code = 2;
        }
    }
}

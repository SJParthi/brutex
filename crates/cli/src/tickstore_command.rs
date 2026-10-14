//! `cli tickstore-write ZIP ROOT [--run-id ID] [--workers N]`: one GDFL day
//! zip written into the tick store under `ROOT` by
//! [`pull::tickstore_write::write_day`], the same function the app's chain
//! runs, so the verb and the function write the same bytes (E2E-02, R4-08).
//!
//! The day file goes where the readers open it, derived from the zip's own
//! canonical name; a held day is verified and left as it is; a leftover
//! temporary is refused by name and left; nothing is removed. Every outcome
//! is one line of `ROOT/tickstore-write.log` and one `pull.tickstore.write`
//! event. A build with no commit stamp does not write, as every store writer
//! (`store_write`, DB-B71): the log line names the writer, and an unstamped
//! one could not say which code that was.

use std::ffi::OsStr;
use std::fmt::Write as _;
use std::path::Path;

use pull::tickstore_write::{Outcome, WRITER, WriteOptions, write_day};

use crate::{FAILED, MISUSED, OK};

/// The verb's argument list, printed with a refusal of its words.
const ARGUMENTS: &str = "usage: cli tickstore-write ZIP ROOT [--run-id ID] [--workers N]\n";

/// The parsed words.
struct Words<'a> {
    zip: &'a str,
    root: &'a str,
    run_id: Option<&'a str>,
    workers: Option<usize>,
}

/// `ZIP ROOT` and the two optional flags, each at most once.
fn parse<'a>(arguments: &[&'a str]) -> Result<Words<'a>, String> {
    let [zip, root, rest @ ..] = arguments else {
        return Err("ZIP and ROOT are required".to_owned());
    };
    let mut words = Words {
        zip,
        root,
        run_id: None,
        workers: None,
    };
    let mut rest = rest.iter();
    while let Some(flag) = rest.next() {
        let value = rest
            .next()
            .ok_or_else(|| format!("`{flag}` needs a value"))?;
        match *flag {
            "--run-id" if words.run_id.is_none() => words.run_id = Some(value),
            "--workers" if words.workers.is_none() => {
                let n = value
                    .parse::<usize>()
                    .ok()
                    .filter(|n| (1..=64).contains(n))
                    .ok_or_else(|| format!("--workers `{value}` is not 1 to 64"))?;
                words.workers = Some(n);
            }
            other => {
                return Err(format!(
                    "`{other}` is not a flag of this verb, or is given twice"
                ));
            }
        }
    }
    Ok(words)
}

/// A run id when none is given: the clock and the process, which no other run
/// shares.
fn fresh_run_id() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    format!("cli-{secs}-{}", std::process::id())
}

/// The verb with the stamp and the allowance named, so every arm is driven by
/// a test without touching the process environment.
pub(crate) fn command_with(
    arguments: &[&str],
    commit: Option<&str>,
    allow: Option<&OsStr>,
    out: &mut String,
) -> u8 {
    let words = match parse(arguments) {
        Ok(words) => words,
        Err(why) => {
            out.push_str("refused: ");
            out.push_str(&why);
            out.push('\n');
            out.push_str(ARGUMENTS);
            return MISUSED;
        }
    };
    if let Some(why) = crate::store_write::unstamped_write_refusal(commit, allow) {
        out.push_str("refused: ");
        out.push_str(&why);
        out.push('\n');
        return FAILED;
    }
    let run_id = words.run_id.map_or_else(fresh_run_id, str::to_owned);
    let mut options = WriteOptions::new(&run_id);
    if let Some(workers) = words.workers {
        options = options.with_workers(workers);
    }
    let run = write_day(Path::new(words.zip), Path::new(words.root), &options);
    let _infallible = writeln!(
        out,
        "tickstore-write: writer {WRITER}, commit {}, run {run_id}, {} ms",
        commit.unwrap_or("unstamped"),
        run.millis
    );
    let mut code = match &run.result {
        Ok(outcome) => {
            let r = outcome.report();
            let _infallible = write!(
                out,
                "{}: {} entries ({} columnar, {} raw), {} rows, {} CSV bytes, {} zip bytes, \
                 {} stored bytes, {} outside the day folder\n  {}\n",
                outcome.verdict(),
                r.entries,
                r.columnar,
                r.raw_fallbacks,
                r.rows,
                r.csv_bytes,
                r.zip_bytes,
                r.stored_bytes,
                r.outside_folder,
                r.path.display()
            );
            if matches!(outcome, Outcome::Held(_)) {
                out.push_str("  nothing was written: the day file is held and matches its zip\n");
            }
            OK
        }
        Err(why) => {
            out.push_str("refused: ");
            out.push_str(&why.to_string());
            out.push('\n');
            FAILED
        }
    };
    if let Some(why) = &run.log_error {
        out.push_str("the log line was NOT written: ");
        out.push_str(why);
        out.push('\n');
        code = FAILED;
    }
    code
}

/// The verb against this build's stamp and the process's allowance.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    command_with(
        arguments,
        crate::commit_stamp(),
        std::env::var_os(crate::store_write::ALLOW_UNSTAMPED_VAR).as_deref(),
        out,
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;

    /// A stamp shaped like the one `build.rs` writes.
    const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";

    /// A stored-only zip of `members`, in order: invented files only.
    fn zip_of(members: &[(&str, &[u8])]) -> Vec<u8> {
        let crc = |bytes: &[u8]| {
            let mut c = flate2::Crc::new();
            c.update(bytes);
            c.sum()
        };
        let le16 = |out: &mut Vec<u8>, v: usize| {
            out.extend_from_slice(&u16::try_from(v).unwrap().to_le_bytes());
        };
        let le32 = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_le_bytes());
        let (mut out, mut central) = (Vec::new(), Vec::new());
        for (name, data) in members {
            let offset = u32::try_from(out.len()).unwrap();
            let len = u32::try_from(data.len()).unwrap();
            for (record, signature) in [(&mut out, 0x0403_4b50), (&mut central, 0x0201_4b50)] {
                le32(record, signature);
                if signature == 0x0201_4b50 {
                    le16(record, 20);
                }
                le16(record, 20);
                le16(record, 0);
                le16(record, 0);
                le16(record, 0x6000);
                le16(record, 0x5B39);
                le32(record, crc(data));
                le32(record, len);
                le32(record, len);
                le16(record, name.len());
                le16(record, 0);
            }
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
            for _ in 0..3 {
                le16(&mut central, 0);
            }
            le32(&mut central, 0);
            le32(&mut central, offset);
            central.extend_from_slice(name.as_bytes());
        }
        let at = u32::try_from(out.len()).unwrap();
        out.extend_from_slice(&central);
        le32(&mut out, 0x0605_4b50);
        le16(&mut out, 0);
        le16(&mut out, 0);
        le16(&mut out, members.len());
        le16(&mut out, members.len());
        le32(&mut out, u32::try_from(central.len()).unwrap());
        le32(&mut out, at);
        le16(&mut out, 0);
        out
    }

    /// An invented NIFTY options day zip in a fresh folder: its path.
    fn staged(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "brutex-tsw-cli-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut rows = String::from("Ticker,Time,LTP,LTQ\r\n");
        for r in 0..120_u32 {
            let _infallible = write!(
                rows,
                "NIFTY30SEP2699000CE,{:02}:{:02}:{:02},{}.{:02},{}\r\n",
                9 + r / 3_600,
                (15 + r / 60) % 60,
                r % 60,
                7_000 + r % 7,
                r % 100,
                75 * (1 + r % 3)
            );
        }
        let zip = zip_of(&[
            ("GFDLNFO_TICK_29092026/", b""),
            ("GFDLNFO_TICK_29092026/Options/", b""),
            (
                "GFDLNFO_TICK_29092026/Options/NIFTY30SEP2699000CE.NFO.csv",
                rows.as_bytes(),
            ),
        ]);
        let path = dir.join("GFDLNFO_TICK_29092026.zip");
        std::fs::write(&path, zip).unwrap();
        path
    }

    /// **THE VERB AND THE FUNCTION WRITE IDENTICAL BYTES**: one zip, one
    /// root through `cli tickstore-write`, another through `write_day`.
    #[test]
    fn the_cli_verb_and_the_function_write_identical_bytes() {
        let zip = staged("same");
        let base = zip.parent().unwrap();
        let (by_verb, by_function) = (base.join("verb"), base.join("function"));
        let mut out = String::new();
        let code = command_with(
            &[
                zip.to_str().unwrap(),
                by_verb.to_str().unwrap(),
                "--run-id",
                "verb",
                "--workers",
                "2",
            ],
            Some(STAMP),
            None,
            &mut out,
        );
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains("written: 3 entries (1 columnar, 0 raw), 120 rows"),
            "{out}"
        );
        assert!(out.contains(WRITER), "{out}");
        let run = write_day(&zip, &by_function, &WriteOptions::new("function"));
        let Ok(Outcome::Written(report)) = run.result else {
            panic!("{run:?}");
        };
        let rel = report.path.strip_prefix(&by_function).unwrap();
        assert_eq!(
            rel,
            Path::new("options/2026/SEP_2026/GFDLNFO_TICK_29092026.bts")
        );
        assert_eq!(
            std::fs::read(by_verb.join(rel)).unwrap(),
            std::fs::read(&report.path).unwrap(),
            "byte for byte"
        );
        // A second press writes nothing and says so.
        let mut again = String::new();
        let code = command_with(
            &[zip.to_str().unwrap(), by_verb.to_str().unwrap()],
            Some(STAMP),
            None,
            &mut again,
        );
        assert_eq!(code, OK, "{again}");
        assert!(again.contains("held: 3 entries"), "{again}");
        assert!(again.contains("nothing was written"), "{again}");
    }

    /// An unstamped build does not write, unless told to; a refusal of the
    /// work exits FAILED with its code, of the words MISUSED with the usage.
    #[test]
    fn the_verb_refuses_by_name() {
        let zip = staged("refuse");
        let root = zip.parent().unwrap().join("root");
        let (z, r) = (zip.to_str().unwrap(), root.to_str().unwrap());
        let mut out = String::new();
        assert_eq!(command_with(&[z, r], None, None, &mut out), FAILED);
        assert!(out.contains("no commit stamp"), "{out}");
        assert!(!root.exists(), "nothing written");
        for words in [
            vec![z],
            vec![z, r, "--run-id"],
            vec![z, r, "--workers", "0"],
            vec![z, r, "--workers", "x"],
            vec![z, r, "--level", "9"],
            vec![z, r, "--run-id", "a", "--run-id", "b"],
        ] {
            let mut out = String::new();
            assert_eq!(
                command_with(&words, Some(STAMP), None, &mut out),
                MISUSED,
                "{words:?}"
            );
            assert!(out.contains(ARGUMENTS), "{out}");
        }
        let mut out = String::new();
        let bad = zip.with_file_name("GFDLNFO_TICK_29092026 (1).zip");
        std::fs::copy(&zip, &bad).unwrap();
        assert_eq!(
            command_with(
                &[bad.to_str().unwrap(), r],
                None,
                Some(OsStr::new("1")),
                &mut out
            ),
            FAILED
        );
        assert!(out.contains("refused: name_not_canonical"), "{out}");
        assert!(out.contains("commit unstamped"), "{out}");
        assert!(fresh_run_id().starts_with("cli-"));
    }
}

//! v3.2 (checklist item 10, the audit's fix 4): NSE F&O BAN days. NSE's dated ban lists `fo_secban_DDMMYYYY.csv` (nsearchives
//! /archives/fo/sec_ban/; "Securities in Ban For Trade Date DD-MON-YYYY:" then `n,SYMBOL` rows): session S reads S's OWN file (the list
//! is for that trade date; the downloads lane's READY.md). A stock in the ban period takes no FRESH position: a stock option of a banned
//! underlying is refused that day (`fno_ban`, counted). A session with no file held is LABELLED and counted, never assumed clean in
//! silence (`ban_list_missing`; its events are not refused, the label says so).

use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::cal::days_from_civil;

/// The dated ban lists: session day -> the banned symbols (an empty set = a NIL file).
#[derive(Clone, Debug, Default)]
pub struct Bans {
    pub by: BTreeMap<i64, HashSet<String>>,
    /// Directories read and files parsed.
    pub sources: Vec<String>,
}

/// The run's lists.
pub static BANS: OnceLock<Bans> = OnceLock::new();
/// Stock events refused for a ban; stock events on a session without a ban file (not refused, labelled).
pub static REFUSED: AtomicU64 = AtomicU64::new(0);
pub static MISSING: AtomicU64 = AtomicU64::new(0);

/// "DD-MON-YYYY" or the file name's DDMMYYYY -> day.
fn day_of_ddmmyyyy(s: &str) -> Option<i64> {
    if s.len() != 8 {
        return None;
    }
    let d: i64 = s.get(0..2)?.parse().ok()?;
    let m: i64 = s.get(2..4)?.parse().ok()?;
    let y: i64 = s.get(4..8)?.parse().ok()?;
    Some(days_from_civil(y, m, d))
}

impl Bans {
    /// Reads every `fo_secban_DDMMYYYY.csv` of a directory (the trade date inside must equal the name's).
    ///
    /// # Errors
    /// An unreadable directory or a file that is not NSE's ban list.
    pub fn add_dir(&mut self, dir: &Path) -> Result<(), String> {
        // v3.4 E5: under PORTABLE the entries in name order
        let ents: Vec<(String, std::path::PathBuf)> = if crate::kit::on(crate::kit::PORTABLE) {
            crate::kit::sorted_dir(dir).map_err(|e| format!("--ban-dir {e}"))?
        } else {
            std::fs::read_dir(dir).map_err(|e| format!("--ban-dir {}: {e}", dir.display()))?.flatten().map(|e| (e.file_name().to_string_lossy().into_owned(), e.path())).collect()
        };
        let mut n = 0_usize;
        for (name, path) in ents {
            let Some(dd) = name.strip_prefix("fo_secban_").and_then(|x| x.strip_suffix(".csv")) else { continue };
            let Some(day) = day_of_ddmmyyyy(dd) else { continue };
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;
            let mut lines = text.lines();
            let head = lines.next().unwrap_or("");
            if !head.starts_with("Securities in Ban") {
                return Err(format!("{name}: not NSE's ban list (first line {head:?})"));
            }
            let mut set = HashSet::new();
            for l in lines {
                let f: Vec<&str> = l.split(',').map(str::trim).collect();
                if let Some(s) = f.get(1)
                    && !s.is_empty()
                {
                    set.insert((*s).to_owned());
                }
            }
            self.by.insert(day, set);
            n += 1;
        }
        self.sources.push(format!("{} ({n} files)", dir.display()));
        Ok(())
    }

    /// Whether `sym` is banned on `day`: Some(true / false), None = no ban file for that session.
    #[must_use]
    pub fn status(&self, day: i64, sym: &str) -> Option<bool> {
        self.by.get(&day).map(|s| s.contains(sym))
    }
}

/// Loads every `--ban-dir` of the command line.
///
/// # Errors
/// A refused directory.
pub fn from_args(a: &[String]) -> Result<Option<String>, String> {
    let dirs: Vec<&String> = a.iter().enumerate().filter(|(_, x)| *x == "--ban-dir").filter_map(|(i, _)| a.get(i + 1)).collect();
    if dirs.is_empty() {
        return Ok(None);
    }
    let mut b = Bans::default();
    for d in dirs {
        b.add_dir(Path::new(d))?;
    }
    let words = format!("{}; sessions {} ({} .. {})", b.sources.join(" + "), b.by.len(), b.by.keys().next().map_or("-".to_owned(), |&d| crate::cal::fmt_day(d)), b.by.keys().next_back().map_or("-".to_owned(), |&d| crate::cal::fmt_day(d)));
    let _ = BANS.set(b);
    Ok(Some(words))
}

/// The refusal test of a STOCK event (true = refuse for a ban). Counts both outcomes that matter.
#[must_use]
pub fn refuse(day: i64, sym: &str) -> bool {
    // MUTANT m_v32_ban_ignored: the ban lists read but never applied
    if cfg!(feature = "m_v32_ban_ignored") {
        return false;
    }
    let Some(b) = BANS.get() else { return false };
    match b.status(day, sym) {
        Some(true) => {
            REFUSED.fetch_add(1, Ordering::Relaxed);
            true
        }
        Some(false) => false,
        None => {
            MISSING.fetch_add(1, Ordering::Relaxed);
            false
        }
    }
}

/// The counters as printed.
#[must_use]
pub fn counts_line() -> String {
    format!("F&O ban: stock event pricings refused fno_ban {}; stock event pricings on a session WITHOUT a ban file (not refused, labelled) {}", REFUSED.load(Ordering::Relaxed), MISSING.load(Ordering::Relaxed))
}

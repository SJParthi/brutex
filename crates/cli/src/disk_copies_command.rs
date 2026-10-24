//! `cli disk-copies`: the copies register from a terminal, always a fresh
//! walk (CC-5, FM-607; D-7040..D-7048).
//!
//! It reads `disk.extra_copy_roots` from the store's settings, walks every
//! copy root and every walked live root once through
//! [`pull::copies_register::walk`], and prints each root with its bytes,
//! reader, proof and twin, every second name of a live file as a `HAZARD`
//! line, and the retirement list, which holds no command: "the operator
//! removes these by hand; the app never does". It reads and never writes:
//! no file is created, moved, renamed or removed (`cli::store_write` lists
//! it among the readers).
//!
//! # Exit codes (D-7047)
//!
//! * `0` — every root walked whole and no live file has a second name.
//! * `1` — a live archive is hard-linked under another name (the hazard), a
//!   root was not walked whole, the setting is unset (a named wait, never a
//!   default), or the store root is refused. The last line says which.
//! * `2` — an argument was given: the verb takes none.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pull::copies_register::{self, Fs, Register, Sources, State, Twin};
use store::settings::{Key, Reader};

use crate::{FAILED, MISUSED, OK};

/// The verb's usage line.
const USAGE: &str =
    "usage: cli disk-copies   (no arguments: a fresh walk of disk.extra_copy_roots)";

/// The register's text, one line per root, live root and hazard, then the
/// retirement list.
fn render(out: &mut String, register: &Register) {
    let _ = writeln!(
        out,
        "DISK COPIES: walked {} in {} us (a fresh walk); {} copy root(s); extra copies {} bytes",
        copies_register::ist_text(register.walked_at_micros),
        register.walk_micros,
        register.roots.len(),
        register.extra_bytes
    );
    for root in &register.roots {
        let path = copies_register::quoted(&root.path);
        match &root.state {
            State::Refused(why) => {
                let _ = writeln!(out, "ROOT {path} REFUSED: {why}");
                continue;
            }
            State::Walked { whole } => {
                let _ = writeln!(
                    out,
                    "ROOT {path} {}: {} file(s), {} bytes, {} its own, newest {}",
                    if *whole { "walked" } else { "NOT WALKED WHOLE" },
                    root.files,
                    root.bytes,
                    root.extra_bytes,
                    root.newest_mtime_s.map_or_else(
                        || "none".to_owned(),
                        |s| copies_register::ist_text(s.saturating_mul(1_000_000))
                    ),
                );
            }
        }
        let reader = root.reader.as_ref().map_or_else(
            || "none found".to_owned(),
            |reader| {
                format!(
                    "{} {} ({})",
                    reader.kind.word(),
                    copies_register::quoted(&reader.path),
                    reader.how
                )
            },
        );
        let proof = root
            .proof
            .as_deref()
            .map_or_else(|| "none".to_owned(), copies_register::quoted);
        let _ = writeln!(out, "  reader: {reader}; proof: {proof}");
        let _ = writeln!(
            out,
            "  twin: {}{}",
            match &root.twin {
                Twin::Proven { .. } => "proven: ",
                Twin::NotProven(_) => "",
            },
            root.twin.words()
        );
        if let Some(live) = &root.inside {
            let _ = writeln!(
                out,
                "  inside the live {} root {}",
                live.kind.word(),
                copies_register::quoted(&live.path)
            );
        }
        for (path, why) in &root.faults {
            let _ = writeln!(out, "  FAULT {}: {why}", copies_register::quoted(path));
        }
    }
    for live in &register.live {
        let _ = writeln!(
            out,
            "LIVE {} {} ({}): {}",
            live.root.kind.word(),
            copies_register::quoted(&live.root.path),
            live.root.named_by,
            live.why
                .clone()
                .unwrap_or_else(|| format!("walked, {} file(s)", live.files))
        );
    }
    for unset in &register.unset_live {
        let _ = writeln!(out, "LIVE NOT SET {unset}");
    }
    for hazard in &register.hazards {
        let _ = writeln!(
            out,
            "HAZARD {} is a second name of the live {} file {} (st_dev {}, st_ino {}, nlink {}){}",
            copies_register::quoted(&hazard.path),
            hazard.kind.word(),
            copies_register::quoted(&hazard.live),
            hazard.dev,
            hazard.ino,
            hazard.nlink,
            if hazard.partial {
                ": a partial-download name, the resumed-write case"
            } else {
                ""
            }
        );
    }
    out.push_str(&copies_register::retirement_text(register));
}

/// The verb, with its store root, filesystem, environment and clock passed
/// in so every arm is driven by a test.
pub(crate) fn command_with(
    arguments: &[&str],
    store: &dyn Fn() -> Result<PathBuf, String>,
    fs: &dyn Fs,
    env: &dyn Fn(&str) -> Option<OsString>,
    now: &dyn Fn() -> i64,
    out: &mut String,
) -> u8 {
    if !arguments.is_empty() {
        let _ = writeln!(out, "DISK COPIES MISUSED: it takes no argument\n{USAGE}");
        return MISUSED;
    }
    let root = match store() {
        Ok(root) => root,
        Err(why) => {
            let _ = writeln!(out, "DISK COPIES REFUSED: {why}");
            return FAILED;
        }
    };
    let config = match configured(&root, env) {
        Ok(config) => config,
        Err(why) => {
            let _ = writeln!(out, "DISK COPIES WAITING: {why}");
            return FAILED;
        }
    };
    let register = copies_register::walk(fs, &config, now());
    copies_register::note_walked(&register);
    render(out, &register);
    if !register.hazards.is_empty() {
        let _ = writeln!(
            out,
            "DISK COPIES: {} name(s) of live files are hard-linked under another name (exit 1)",
            register.hazards.len()
        );
        FAILED
    } else if !register.whole() {
        let _ = writeln!(out, "DISK COPIES: a root was not walked whole (exit 1)");
        FAILED
    } else {
        let _ = writeln!(out, "DISK COPIES: no hazard; every root walked whole");
        OK
    }
}

/// The configuration the settings of `root` name, or why nothing is walked.
fn configured(
    root: &Path,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<copies_register::Config, String> {
    let mut reader = Reader::new(root);
    let fault = reader.refresh().err();
    let newest = reader.newest();
    let saved = |key: Key| newest.and_then(|newest| newest.settings.get(key));
    copies_register::config(&Sources {
        copy_roots: saved(Key::ExtraCopyRoots),
        source_gdfl: saved(Key::SourceGdfl),
        source_masters: saved(Key::SourceMasters),
        store_root: root,
        env,
    })
    .map_err(|wait| match fault {
        Some(fault) => format!(
            "{wait}; and the settings directory {} cannot be read: {}",
            fault.path.display(),
            fault.why
        ),
        None => wait.to_string(),
    })
}

/// The verb as dispatched: the CLI's store root, the host's filesystem, the
/// process environment and the wall clock.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    command_with(
        arguments,
        &crate::store_write::root_for("disk-copies", arguments),
        &copies_register::Host,
        &|name| std::env::var_os(name),
        &copies_register::now_micros,
        out,
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

    use store::settings::{self, Change};

    use super::*;

    /// A new, empty folder.
    fn scratch(name: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-disk-copies-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// Saves `disk.extra_copy_roots` = `value`.
    fn save_roots(store: &Path, value: &str) {
        let mut reader = Reader::new(store);
        let _ = reader.refresh();
        let change = Change {
            expected: reader.highest(),
            set: vec![(Key::ExtraCopyRoots, value.to_owned())],
            unset: Vec::new(),
            changed_by: "test".to_owned(),
            reason: "disk-copies test".to_owned(),
            at_micros: 1_700_000_000_000_000,
        };
        settings::save(&mut reader, &change, |_| Ok(())).unwrap();
    }

    /// A store, a copy root and an archive outside it, with one partial
    /// download hard-linked to the archive's year zip when `linked`.
    fn fixture(name: &str, linked: bool) -> (PathBuf, PathBuf, PathBuf) {
        let base = scratch(name);
        let store = base.join("store");
        let archive = base.join("Options");
        let queue = base.join("queue");
        for dir in [&store, &archive, &queue] {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(archive.join("2020.zip"), b"invented year zip").unwrap();
        std::fs::write(queue.join("2021.zip.part"), b"invented partial").unwrap();
        if linked {
            std::fs::hard_link(archive.join("2020.zip"), queue.join("2020.p.zip.part")).unwrap();
        }
        save_roots(&store, &queue.display().to_string());
        (base, store, archive)
    }

    /// The environment naming `archive` as the options archives.
    fn env_of(archive: &Path) -> impl Fn(&str) -> Option<OsString> + '_ {
        move |name| {
            (name == pull::gdfl_run::NFO_ARCHIVES_VAR).then(|| archive.as_os_str().to_owned())
        }
    }

    /// A HARD-LINKED PARTIAL NAME exits non-zero naming it, the archive and
    /// `(st_dev, st_ino)`, with the retirement list and its sentence; and every
    /// run walks afresh, whatever the run before it saw (FM-607; D-7047).
    #[test]
    fn a_hazard_exits_non_zero_and_every_run_walks_afresh() {
        let (base, store, archive) = fixture("hazard", true);
        let clock = AtomicI64::new(1_000_000);
        let now = || clock.fetch_add(1_000_000, Ordering::SeqCst);
        let env = env_of(&archive);
        let mut out = String::new();
        let code = command_with(
            &[],
            &|| Ok(store.clone()),
            &copies_register::Host,
            &env,
            &now,
            &mut out,
        );
        assert_eq!(code, FAILED, "{out}");
        assert!(out.contains("HAZARD \""), "{out}");
        assert!(out.contains("2020.p.zip.part"), "{out}");
        assert!(out.contains("the resumed-write case"), "{out}");
        assert!(
            out.contains(&format!("{:?}", archive.join("2020.zip").as_os_str())),
            "{out}"
        );
        assert!(out.contains(copies_register::RETIREMENT_SENTENCE), "{out}");
        assert!(out.contains("1970-01-01 05:30:01 IST"), "{out}");
        assert!(
            out.lines()
                .last()
                .unwrap()
                .contains("hard-linked under another name (exit 1)")
        );
        // THE LINK IS REMOVED BY HAND, as the list says: the next run sees it
        // at once, with no cache to outlive it.
        std::fs::remove_file(base.join("queue/2020.p.zip.part")).unwrap();
        let mut out = String::new();
        let code = command_with(
            &[],
            &|| Ok(store.clone()),
            &copies_register::Host,
            &env,
            &now,
            &mut out,
        );
        assert_eq!(code, OK, "{out}");
        assert!(
            out.contains("1970-01-01 05:30:02 IST"),
            "a fresh walk: {out}"
        );
        assert!(!out.contains("HAZARD"), "{out}");
        assert!(
            out.lines()
                .last()
                .unwrap()
                .contains("no hazard; every root walked whole")
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// UNSET IS A NAMED WAIT: exit 1, the key named, nothing walked; an
    /// argument is a misuse; a refused store root is named.
    #[test]
    fn unset_misused_and_refused_are_each_named() {
        let base = scratch("unset");
        let mut out = String::new();
        let walked = AtomicI64::new(0);
        let now = || walked.fetch_add(1, Ordering::SeqCst);
        let env = |_: &str| None;
        let code = command_with(
            &[],
            &|| Ok(base.clone()),
            &copies_register::Host,
            &env,
            &now,
            &mut out,
        );
        assert_eq!(code, FAILED);
        assert!(
            out.starts_with("DISK COPIES WAITING: disk.extra_copy_roots is not set"),
            "{out}"
        );
        assert_eq!(walked.load(Ordering::SeqCst), 0, "nothing was walked");
        // A SETTINGS DIRECTORY THAT CANNOT BE LISTED is named beside the wait.
        std::fs::create_dir_all(base.join("settings")).unwrap();
        std::fs::write(settings::dir(&base), b"a file where a folder belongs").unwrap();
        let mut out = String::new();
        let code = command_with(
            &[],
            &|| Ok(base.clone()),
            &copies_register::Host,
            &env,
            &now,
            &mut out,
        );
        assert_eq!(code, FAILED);
        assert!(out.contains("cannot be read"), "{out}");
        let mut out = String::new();
        assert_eq!(
            command_with(
                &["--now"],
                &|| Ok(base.clone()),
                &copies_register::Host,
                &env,
                &now,
                &mut out
            ),
            MISUSED
        );
        assert!(
            out.contains("takes no argument") && out.contains(USAGE),
            "{out}"
        );
        let mut out = String::new();
        let code = command_with(
            &[],
            &|| Err("no store root (invented)".to_owned()),
            &copies_register::Host,
            &env,
            &now,
            &mut out,
        );
        assert_eq!(code, FAILED);
        assert_eq!(out, "DISK COPIES REFUSED: no store root (invented)\n");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A ROOT THAT IS GONE exits non-zero as not walked whole, says why, and
    /// is never offered for retirement; a proven twin and a root inside a live
    /// root are said on their lines.
    #[test]
    fn a_root_not_walked_whole_exits_non_zero() {
        let (base, store, archive) = fixture("gone", false);
        let gone = base.join("gone");
        let inside = archive.join("kept-inside");
        std::fs::create_dir_all(&gone).unwrap();
        std::fs::create_dir_all(&inside).unwrap();
        save_roots(
            &store,
            &format!(
                "{}|{}|{}",
                base.join("queue").display(),
                gone.display(),
                inside.display()
            ),
        );
        std::fs::remove_dir_all(&gone).unwrap();
        let env = env_of(&archive);
        let mut out = String::new();
        let code = command_with(
            &[],
            &|| Ok(store.clone()),
            &copies_register::Host,
            &env,
            &|| 0,
            &mut out,
        );
        assert_eq!(code, FAILED, "{out}");
        assert!(out.contains("REFUSED: cannot be read"), "{out}");
        assert!(out.contains("reader: nfo_archives"), "{out}");
        assert!(out.contains("LIVE NOT SET cm_archive"), "{out}");
        assert!(out.contains("LIVE nfo_archives"), "{out}");
        assert!(
            out.lines()
                .last()
                .unwrap()
                .contains("a root was not walked whole (exit 1)")
        );
        let text = copies_register::retirement_text(&copies_register::walk(
            &copies_register::Host,
            &copies_register::config(&Sources {
                copy_roots: Some(&gone.display().to_string()),
                source_gdfl: None,
                source_masters: None,
                store_root: &store,
                env: &env,
            })
            .unwrap(),
            0,
        ));
        assert!(
            !text.contains("gone"),
            "a refused root is never offered: {text}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}

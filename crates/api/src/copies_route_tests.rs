//! Tests of `GET /disk/copies.json` (CC-5, FM-607; D-7040..D-7048). Every
//! store root and folder is a scratch folder this file makes and removes;
//! every byte is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use pull::copies_register::{self, Cache, Ending, Fs, Host, Meta};
use serde_json::Value;
use store::settings::{self, Change, Key, Reader};

use super::*;

/// A new, empty store root.
fn root(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = crate::scratch::path(&format!(
        "copies-route-{name}-{}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Saves `disk.extra_copy_roots` = `value` as the next generation.
fn save_roots(store: &Path, value: &str) {
    let mut reader = Reader::new(store);
    let _ = reader.refresh();
    let change = Change {
        expected: reader.highest(),
        set: vec![(Key::ExtraCopyRoots, value.to_owned())],
        unset: Vec::new(),
        changed_by: "test".to_owned(),
        reason: "copies route test".to_owned(),
        at_micros: 1_700_000_000_000_000,
    };
    settings::save(&mut reader, &change, |_| Ok(())).unwrap();
}

/// The answer as JSON, with its status.
fn json_of(answer: (StatusCode, String)) -> (StatusCode, Value) {
    let (status, body) = answer;
    (
        status,
        serde_json::from_str(&body).unwrap_or_else(|why| panic!("{why}: {body}")),
    )
}

/// No variable is set.
fn no_env(_: &str) -> Option<OsString> {
    None
}

/// An unset `disk.extra_copy_roots` is a named wait through the route: `200`,
/// the key named, nothing walked, no register, never a default folder; and
/// the settings route serves the key with no value, the same sentence as its
/// source, and the register as its reader (D-7040).
#[test]
fn an_unset_key_is_a_named_wait_and_the_settings_route_names_its_reader() {
    let store = root("unset");
    let cache = Cache::new();
    let (status, body) = json_of(answer(&store, &cache, &Host, &no_env, &|| 1));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "waiting", "{body}");
    assert_eq!(body["key"], "disk.extra_copy_roots");
    assert_eq!(body["walked"], false);
    assert!(
        body["why"].as_str().unwrap().contains("never a default"),
        "{body}"
    );
    assert!(body["settings_degraded"].is_null());
    assert!(body.get("roots").is_none(), "no register is answered");
    assert_eq!(
        copies_register::total(&cache),
        copies_register::Total::NotWalked
    );
    let snapshot = settings_route::snapshot(&store);
    let effective = settings_route::effective(&snapshot, Key::ExtraCopyRoots);
    assert_eq!(effective.value, None);
    assert_eq!(effective.origin, "default");
    assert_eq!(effective.source, copies_register::UNSET_WAIT);
    let reader = settings_route::reader_of(Key::ExtraCopyRoots).unwrap();
    assert!(reader.contains("/disk/copies.json") && reader.contains("cli disk-copies"));
    // A DEGRADED SETTINGS DIRECTORY with the key unset is still a wait, and
    // says the settings are degraded.
    let dir = settings::dir(&store);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(settings::path_of(&store, 1), b"not a settings file").unwrap();
    let (status, body) = json_of(answer(&store, &cache, &Host, &no_env, &|| 1));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["state"], "waiting");
    assert!(body["settings_degraded"].as_str().is_some(), "{body}");
    let _ = std::fs::remove_dir_all(&store);
}

/// A saved key answers the register: each root with its bytes, files, reader
/// ("none found"), proof and `duplicates_live`; the second name of a live
/// archive as `"hazard": true` naming the archive and `(st_dev, st_ino)`;
/// the retirement list and its text with the fixed sentence; and when it
/// was walked and how old the walk is, the second answer served from
/// memory with the same walk (FM-607; amendment (f)).
#[test]
fn a_saved_key_answers_the_register_with_its_red_rows() {
    let store = root("saved");
    // THE FOLDERS LIE OUTSIDE THE STORE ROOT: one inside it is read by the
    // store, and has that reader (D-7044).
    let folders = root("saved-folders");
    let archive = folders.join("invented-Options");
    let queue = folders.join("invented-queue");
    std::fs::create_dir_all(&archive).unwrap();
    std::fs::create_dir_all(queue.join("run")).unwrap();
    std::fs::write(archive.join("2020.zip"), b"invented year zip").unwrap();
    std::fs::hard_link(
        archive.join("2020.zip"),
        queue.join("2020.preserved.1.zip.part"),
    )
    .unwrap();
    std::fs::write(queue.join("2021.zip.part"), b"invented partial").unwrap();
    std::fs::write(
        queue.join("run/ALL_VERIFIED.txt"),
        b"every archive passed\n",
    )
    .unwrap();
    save_roots(&store, &queue.display().to_string());
    let archive_text = archive.display().to_string();
    let env = move |name: &str| {
        (name == pull::gdfl_run::NFO_ARCHIVES_VAR).then(|| OsString::from(&archive_text))
    };
    let cache = Cache::new();
    let (status, body) = json_of(answer(&store, &cache, &Host, &env, &|| 5_000_000));
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["state"], "walked");
    assert_eq!(body["fresh"], true);
    assert_eq!(body["age_ms"], 0);
    assert_eq!(body["walked_at_micros"], 5_000_000);
    assert_eq!(body["walked_at"], "1970-01-01 05:30:05 IST");
    assert_eq!(body["roots_walked"].as_array().unwrap().len(), 2);
    let root0 = &body["roots"][0];
    assert_eq!(root0["path"], queue.display().to_string());
    assert_eq!(root0["state"], "walked");
    assert_eq!(root0["files"], 3);
    assert_eq!(
        root0["extra_bytes"],
        b"invented partial".len() + b"every archive passed\n".len()
    );
    assert_eq!(root0["reader"], "none found");
    assert_eq!(
        root0["proof"],
        queue.join("run/ALL_VERIFIED.txt").display().to_string()
    );
    assert_eq!(root0["duplicates_live"], true);
    assert_eq!(root0["retirable"], true);
    let hazards = body["hazards"].as_array().unwrap();
    assert_eq!(hazards.len(), 1);
    assert_eq!(body["hazard_count"], 1);
    assert_eq!(hazards[0]["hazard"], true);
    assert_eq!(
        hazards[0]["live"],
        archive.join("2020.zip").display().to_string()
    );
    assert_eq!(hazards[0]["kind"], "nfo_archives");
    assert_eq!(hazards[0]["partial"], true);
    {
        use std::os::unix::fs::MetadataExt as _;
        let meta = std::fs::metadata(archive.join("2020.zip")).unwrap();
        assert_eq!(hazards[0]["ino"], meta.ino());
        assert_eq!(hazards[0]["dev"], meta.dev());
    }
    let list = body["retirement_list"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[1]["hazard"], true);
    assert_eq!(list[1]["bytes"], 0);
    let text = body["retirement_text"].as_str().unwrap();
    assert!(
        text.contains(copies_register::RETIREMENT_SENTENCE),
        "{text}"
    );
    assert_eq!(
        body["retirement_sentence"],
        copies_register::RETIREMENT_SENTENCE
    );
    assert_eq!(body["extra_copies_bytes"], root0["extra_bytes"]);
    assert!(body["note"].as_str().unwrap().contains("next walk"));
    std::thread::sleep(std::time::Duration::from_millis(3));
    let (_, again) = json_of(answer(&store, &cache, &Host, &env, &|| 9_000_000));
    assert_eq!(again["fresh"], false);
    assert_eq!(again["walked_at_micros"], 5_000_000, "the same walk");
    assert!(again["age_ms"].as_u64().unwrap() >= 3, "{again}");
    assert_eq!(
        again["hazards"], body["hazards"],
        "the held body, unchanged"
    );
    let _ = std::fs::remove_dir_all(&store);
    let _ = std::fs::remove_dir_all(&folders);
}

/// The host, but the `lstat` of one name waits while it is held: a walk in
/// progress, for a test to ask beside.
struct Slow {
    hold: PathBuf,
    holding: AtomicBool,
    held: AtomicBool,
}

impl Fs for Slow {
    fn lstat(&self, path: &Path) -> io::Result<Meta> {
        if path == self.hold {
            self.held.store(true, Ordering::SeqCst);
            while self.holding.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        Host.lstat(path)
    }
    fn list(&self, dir: &Path, each: &mut dyn FnMut(OsString)) -> io::Result<()> {
        Host.list(dir, each)
    }
    fn lines(&self, path: &Path, each: &mut dyn FnMut(&[u8])) -> Result<Ending, String> {
        Host.lines(path, each)
    }
}

/// A request that arrives while the first walk since start runs is a `503`
/// naming the walk, at once, never a wait on it; once the walk lands it is
/// served (D-7046).
#[test]
fn a_request_during_the_first_walk_is_a_503_naming_it() {
    let store = root("walking");
    let copy = store.join("invented-copy");
    std::fs::create_dir_all(&copy).unwrap();
    std::fs::write(copy.join("a.bin"), b"abc").unwrap();
    save_roots(&store, &copy.display().to_string());
    let slow = std::sync::Arc::new(Slow {
        hold: copy.join("a.bin"),
        holding: AtomicBool::new(true),
        held: AtomicBool::new(false),
    });
    let cache = std::sync::Arc::new(Cache::new());
    let first = {
        let (slow, cache, store) = (slow.clone(), cache.clone(), store.clone());
        std::thread::spawn(move || answer(&store, &cache, slow.as_ref(), &no_env, &|| 1))
    };
    while !slow.held.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let began = std::time::Instant::now();
    let (status, body) = json_of(answer(&store, &cache, slow.as_ref(), &no_env, &|| 2));
    assert!(
        began.elapsed() < std::time::Duration::from_secs(5),
        "never a wait"
    );
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["state"], "walking");
    assert!(body["error"].as_str().unwrap().contains("ask again"));
    slow.holding.store(false, Ordering::SeqCst);
    let (status, body) = json_of(first.join().unwrap());
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["roots"][0]["files"], 1);
    let _ = std::fs::remove_dir_all(&store);
}

/// The handler itself, through the ops-read pool and the process's cache:
/// `200`, JSON, no caching, and a register whose unset live roots are named.
#[tokio::test]
async fn the_handler_answers_through_the_ops_read_pool() {
    let store = root("handler");
    std::fs::create_dir_all(store.join("masters")).unwrap();
    let copy = store.join("invented-copy");
    std::fs::create_dir_all(&copy).unwrap();
    std::fs::write(copy.join("x.csv"), b"12345").unwrap();
    save_roots(&store, &copy.display().to_string());
    let site = Loaded::new(crate::server::Site::load(&store.join("masters"), &store));
    let (status, headers, body) = copies_json(axum::extract::State(site)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        headers[axum::http::header::CONTENT_TYPE],
        "application/json; charset=utf-8"
    );
    assert_eq!(headers[axum::http::header::CACHE_CONTROL], "no-store");
    assert!(
        headers.get(axum::http::header::RETRY_AFTER).is_none(),
        "an answer in full never asks to be retried"
    );
    for asks_again in [
        StatusCode::SERVICE_UNAVAILABLE,
        StatusCode::TOO_MANY_REQUESTS,
    ] {
        assert_eq!(
            headers_for(asks_again)[axum::http::header::RETRY_AFTER],
            "1"
        );
    }
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["state"], "walked");
    assert_eq!(body["roots"][0]["bytes"], 5);
    assert!(
        body["live"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["kind"] == "store")
    );
    let _ = std::fs::remove_dir_all(&store);
}

/// A name that is not UTF-8 is served in its escaped form, never a lossy
/// guess; a refused root has no figures, only why.
#[test]
fn a_name_that_is_not_utf8_is_served_escaped_and_a_refused_root_has_no_figures() {
    use std::os::unix::ffi::OsStrExt as _;
    let odd = Path::new(std::ffi::OsStr::from_bytes(b"/invented/bad\xff"));
    assert_eq!(
        path_json(odd),
        Value::String("\"/invented/bad\\xFF\"".to_owned())
    );
    assert_eq!(path_json(Path::new("/ok")), Value::String("/ok".to_owned()));
    let store = root("refused");
    let gone = store.join("invented-gone");
    std::fs::create_dir_all(&gone).unwrap();
    save_roots(&store, &gone.display().to_string());
    std::fs::remove_dir_all(&gone).unwrap();
    let (status, body) = json_of(answer(&store, &Cache::new(), &Host, &no_env, &|| 1));
    assert_eq!(status, StatusCode::OK);
    let root0 = &body["roots"][0];
    assert_eq!(root0["state"], "refused");
    assert!(
        root0["refused"].as_str().unwrap().contains("No such file"),
        "{root0}"
    );
    assert!(root0["bytes"].is_null() && root0["files"].is_null() && root0["extra_bytes"].is_null());
    assert_eq!(body["whole"], false);
    assert_eq!(body["retirement_list"].as_array().unwrap().len(), 0);
    let _ = std::fs::remove_dir_all(&store);
}

/// The host, except that one folder refuses its listing and another lists,
/// beside its own names, the name `again`: a second way into itself, as a
/// second mount of one folder is.
struct Odd {
    refuse: PathBuf,
    again: PathBuf,
}

impl Fs for Odd {
    fn lstat(&self, path: &Path) -> io::Result<Meta> {
        if path == self.again.join("again") {
            return Host.lstat(&self.again);
        }
        Host.lstat(path)
    }

    fn list(&self, dir: &Path, each: &mut dyn FnMut(OsString)) -> io::Result<()> {
        if dir == self.refuse {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        Host.list(dir, each)?;
        if dir == self.again {
            each(OsString::from("again"));
        }
        Ok(())
    }

    fn lines(&self, path: &Path, each: &mut dyn FnMut(&[u8])) -> Result<Ending, String> {
        Host.lines(path, each)
    }
}

/// EVERY SHAPE A ROOT TAKES ON THE WIRE: a root with a folder that will not
/// list is `partial` with its fault, the folder it met twice named once in
/// `revisited` and its file with a name outside every root in
/// `linked_elsewhere`; a copy whose `VERIFIED.tsv` names the live archive
/// file as it stands today is `proven` with its rows counted; a folder
/// inside the tick store, outside the parts its readers read, has no reader
/// and names the live root it lies `inside`; and a folder holding the
/// options archives lists them in `skipped_roots` (D-7043..D-7045).
#[test]
fn a_partial_root_a_proven_twin_and_a_folder_inside_a_live_root_are_served() {
    use std::os::unix::fs::MetadataExt as _;
    let store = root("shapes");
    let folders = root("shapes-folders");
    let around = folders.join("invented-around");
    let archive = around.join("Options");
    let twin = folders.join("invented-twin");
    let ticks = folders.join("invented-ticks");
    let prove = ticks.join("prove");
    let partial = folders.join("invented-partial");
    for dir in [&archive, &twin, &prove, &partial.join("shut")] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(archive.join("2020.zip"), b"invented year zip").unwrap();
    std::fs::write(twin.join("2020.zip"), b"invented year zip").unwrap();
    let m = std::fs::metadata(archive.join("2020.zip")).unwrap();
    std::fs::write(
        twin.join("VERIFIED.tsv"),
        format!(
            "name\tsource_identity\n2020.zip\t{}:{}:{}:{}:{}:{}:{}\n",
            m.dev(),
            m.ino(),
            m.size(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec()
        ),
    )
    .unwrap();
    std::fs::write(prove.join("p.prove.out"), b"ALL-PROVED\n").unwrap();
    std::fs::write(partial.join("kept.bin"), b"kept").unwrap();
    std::fs::hard_link(partial.join("kept.bin"), folders.join("elsewhere.bin")).unwrap();
    save_roots(
        &store,
        &format!(
            "{}|{}|{}|{}",
            twin.display(),
            prove.display(),
            partial.display(),
            around.display()
        ),
    );
    let (archive_text, ticks_text) = (archive.display().to_string(), ticks.display().to_string());
    let env = move |name: &str| match name {
        pull::gdfl_run::NFO_ARCHIVES_VAR => Some(OsString::from(&archive_text)),
        pull::gdfl_run::TICKSTORE_VAR => Some(OsString::from(&ticks_text)),
        _ => None,
    };
    let fs = Odd {
        refuse: partial.join("shut"),
        again: partial.clone(),
    };
    let (status, body) = json_of(answer(&store, &Cache::new(), &fs, &env, &|| 1));
    assert_eq!(status, StatusCode::OK, "{body}");
    let roots = body["roots"].as_array().unwrap();
    assert_eq!(roots[0]["twin"]["proven"], true, "{}", roots[0]);
    assert_eq!(roots[0]["twin"]["sources"], 1);
    assert_eq!(roots[0]["receipts"][0]["kind"], "verified_tsv");
    assert_eq!(roots[1]["reader"], "none found");
    assert_eq!(roots[1]["inside"]["kind"], "tick_store");
    assert_eq!(roots[1]["inside"]["path"], ticks.display().to_string());
    assert_eq!(roots[2]["state"], "partial", "{}", roots[2]);
    assert_eq!(roots[2]["bytes"], 4, "what was read is counted");
    assert_eq!(
        roots[2]["faults"][0]["path"],
        partial.join("shut").display().to_string()
    );
    assert_eq!(roots[2]["retirable"], false);
    assert_eq!(
        roots[2]["revisited"][0],
        partial.join("again").display().to_string()
    );
    assert_eq!(
        roots[2]["linked_elsewhere"][0],
        partial.join("kept.bin").display().to_string()
    );
    assert_eq!(roots[3]["skipped_roots"][0], archive.display().to_string());
    let _ = std::fs::remove_dir_all(&store);
    let _ = std::fs::remove_dir_all(&folders);
}

/// ONE RENDERING PER WALK: asked again for the same register, the body is
/// the one already rendered, the same allocation, never rendered twice. The
/// process holds one rendering, so another test's walk landing in between
/// is told apart by asking until two renderings in a row are of this one.
#[test]
fn a_register_is_rendered_once_however_often_it_is_served() {
    let store = root("rendered");
    let copy = root("rendered-copy");
    std::fs::write(copy.join("a.bin"), b"abc").unwrap();
    save_roots(&store, &copy.display().to_string());
    let config = configured(&store, &no_env).unwrap();
    let register = std::sync::Arc::new(copies_register::walk(&Host, &config, 1));
    let same = (0..1_000).any(|_| {
        let first = rendered(&register);
        std::sync::Arc::ptr_eq(&first, &rendered(&register))
    });
    assert!(same, "a second ask renders nothing");
    assert!(rendered(&register).contains("\"files\":1,"));
    let _ = std::fs::remove_dir_all(&store);
    let _ = std::fs::remove_dir_all(&copy);
}

/// A FULL OPS-READ POOL is refused by name, `429`, the register's read
/// named; an admitted answer passes through untouched.
#[test]
fn a_full_pool_is_refused_by_name_and_an_admitted_answer_passes() {
    let (status, body) = admitted(Err(crate::detail::RunError::Saturated));
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(body.contains("a copies register read"), "{body}");
    assert_eq!(headers_for(status)[axum::http::header::RETRY_AFTER], "1");
    assert_eq!(
        admitted(Ok((StatusCode::OK, "{}".to_owned()))),
        (StatusCode::OK, "{}".to_owned())
    );
}

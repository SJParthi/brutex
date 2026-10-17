#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]
//! The GDFL Dropbox fetch's tests (E2E-01, PERF-13, R4-07).
//!
//! Every byte here is invented: the zips are built in code from members of
//! filler text (`gdfl_fixtures::zip`), no vendor row is quoted, and the
//! "Dropbox" is a loopback fixture server this file runs itself, speaking the
//! three routes the module calls. Every store root is a fresh scratch folder;
//! the real store is never named.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use super::*;
use crate::disk_projection::{
    FIXTURE_TOLERANCE_PERCENT, KindKey, PlanItem, Space, Totals, Unit, project, within,
};
use crate::gdfl_fixtures::{Method, scratch, zip};

// -- the fixture server -------------------------------------------------------

/// One request as the server read it.
#[derive(Debug, Clone)]
struct Request {
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

/// One scripted answer.
#[derive(Clone)]
enum Reply {
    /// Status, headers, body.
    Full(u16, Vec<(String, String)>, Vec<u8>),
    /// Status, headers, body, and the bytes actually sent before the socket
    /// closes (the header states the whole body).
    Cut(u16, Vec<(String, String)>, Vec<u8>, usize),
}

fn json_reply(status: u16, value: &serde_json::Value) -> Reply {
    Reply::Full(
        status,
        vec![("Content-Type".into(), "application/json".into())],
        value.to_string().into_bytes(),
    )
}

/// A file the fake folder holds.
#[derive(Debug, Clone)]
struct FakeFile {
    path: String,
    bytes: Vec<u8>,
    rev: String,
    hash: String,
}

/// The fake Dropbox: a folder, and every misbehaviour a test switches on.
#[derive(Default)]
struct Fake {
    files: Vec<FakeFile>,
    folders: Vec<String>,
    token: String,
    page_size: usize,
    /// Names whose FIRST download is cut at this fraction (percent).
    cut_first: HashMap<String, usize>,
    /// Names already cut once.
    were_cut: HashSet<String>,
    /// 429 answers left before success; `usize::MAX` is a storm.
    throttle: usize,
    retry_after: Option<u64>,
    /// 5xx answers left before success.
    fail5xx: usize,
    /// Downloads answer 404 after this many succeeded.
    gone_after: Option<usize>,
    downloads_ok: usize,
    /// Names served as an HTML page.
    html: HashSet<String>,
    /// Answer every Range request with a 200 and the whole body.
    ignore_range: bool,
    /// Answer every Range request with a 206 whose range is wrong.
    wrong_range: bool,
    /// The listing page (0-based, over the whole walk) whose socket drops.
    drop_page: Option<usize>,
    pages_served: usize,
    /// A whole answer for every `list_folder`.
    list_override: Option<Reply>,
    /// The listing calls its entries `items`.
    rename_entries: bool,
    /// A `Location` every download answers with, as a 302.
    redirect_to: Option<String>,
    /// Set while a `Range` request is answered: the operator presses Stop
    /// during a resumed download.
    stop_on_range: Option<Arc<AtomicBool>>,
}

impl Fake {
    fn file(&mut self, path: &str, bytes: Vec<u8>) {
        let rev = format!("rev{:04}", self.files.len());
        let hash = format!("hash-{}", crate::gdfl_archive::crc32(&bytes));
        self.files.push(FakeFile {
            path: path.to_owned(),
            bytes,
            rev,
            hash,
        });
    }

    fn answer(&mut self, request: &Request) -> Reply {
        let auth = request
            .headers
            .get("authorization")
            .cloned()
            .unwrap_or_default();
        if auth != format!("Bearer {}", self.token) {
            return json_reply(
                401,
                &serde_json::json!({"error_summary": "invalid_access_token/"}),
            );
        }
        if self.throttle > 0 {
            if self.throttle != usize::MAX {
                self.throttle -= 1;
            }
            let mut headers = Vec::new();
            if let Some(secs) = self.retry_after {
                headers.push(("Retry-After".to_owned(), secs.to_string()));
            }
            return Reply::Full(429, headers, b"too many requests".to_vec());
        }
        if self.fail5xx > 0 {
            self.fail5xx -= 1;
            return Reply::Full(503, Vec::new(), b"unavailable".to_vec());
        }
        match request.path.as_str() {
            "/2/files/list_folder" | "/2/files/list_folder/continue" => self.list(request),
            "/2/sharing/get_shared_link_file" => self.download(request),
            "/redirected" => {
                let saved = self.redirect_to.take();
                let reply = self.download(request);
                self.redirect_to = saved;
                reply
            }
            _ => Reply::Full(404, Vec::new(), Vec::new()),
        }
    }

    fn list(&mut self, request: &Request) -> Reply {
        if let Some(reply) = &self.list_override {
            return reply.clone();
        }
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        let (folder, offset) = if let Some(cursor) = body.get("cursor").and_then(|c| c.as_str()) {
            let (folder, offset) = cursor.rsplit_once('@').unwrap();
            (folder.to_owned(), offset.parse::<usize>().unwrap())
        } else {
            (body["path"].as_str().unwrap().to_owned(), 0)
        };
        let page = self.pages_served;
        self.pages_served += 1;
        let mut entries: Vec<serde_json::Value> = Vec::new();
        for sub in &self.folders {
            if parent(sub) == folder {
                entries.push(serde_json::json!({".tag": "folder", "name": leaf(sub)}));
            }
        }
        for file in &self.files {
            if parent(&file.path) == folder {
                entries.push(serde_json::json!({
                    ".tag": "file",
                    "name": leaf(&file.path),
                    "size": file.bytes.len(),
                    "rev": file.rev,
                    "content_hash": file.hash,
                }));
            }
        }
        let size = if self.page_size == 0 {
            usize::MAX
        } else {
            self.page_size
        };
        let end = offset.saturating_add(size).min(entries.len());
        let has_more = end < entries.len();
        let key = if self.rename_entries {
            "items"
        } else {
            "entries"
        };
        let value = serde_json::json!({
            key: entries[offset..end].to_vec(),
            "cursor": format!("{folder}@{end}"),
            "has_more": has_more,
        });
        if self.drop_page == Some(page) {
            let bytes = value.to_string().into_bytes();
            let half = bytes.len() / 2;
            return Reply::Cut(
                200,
                vec![("Content-Type".into(), "application/json".into())],
                bytes,
                half,
            );
        }
        json_reply(200, &value)
    }

    fn download(&mut self, request: &Request) -> Reply {
        if let Some(location) = &self.redirect_to {
            return Reply::Full(302, vec![("Location".into(), location.clone())], Vec::new());
        }
        if self
            .gone_after
            .is_some_and(|after| self.downloads_ok >= after)
        {
            return json_reply(
                409,
                &serde_json::json!({"error_summary": "shared_link_not_found/"}),
            );
        }
        let arg: serde_json::Value =
            serde_json::from_str(&request.headers["dropbox-api-arg"]).unwrap();
        let path = arg["path"].as_str().unwrap();
        let Some(file) = self.files.iter().find(|f| f.path == path).cloned() else {
            return json_reply(
                409,
                &serde_json::json!({"error_summary": "path/not_found/"}),
            );
        };
        let name = leaf(&file.path);
        let result = serde_json::json!({"name": name, "rev": file.rev, "size": file.bytes.len()})
            .to_string();
        if self.html.contains(&name) {
            return Reply::Full(
                200,
                vec![("Content-Type".into(), "text/html".into())],
                b"<html>sign in</html>".to_vec(),
            );
        }
        let mut headers = vec![
            (
                "Content-Type".to_owned(),
                "application/octet-stream".to_owned(),
            ),
            ("Dropbox-API-Result".to_owned(), result),
        ];
        let range = request
            .headers
            .get("range")
            .and_then(|r| r.strip_prefix("bytes="))
            .and_then(|r| r.strip_suffix('-'))
            .map(|r| r.parse::<usize>().unwrap());
        if let Some(cut) = self.cut_first.get(&name).copied()
            && self.were_cut.insert(name.clone())
        {
            let at = file.bytes.len() * cut / 100;
            return Reply::Cut(200, headers, file.bytes.clone(), at);
        }
        self.downloads_ok += 1;
        if range.is_some()
            && let Some(flag) = &self.stop_on_range
        {
            flag.store(true, std::sync::atomic::Ordering::Release);
        }
        match range {
            Some(from) if !self.ignore_range => {
                let total = file.bytes.len();
                let start = if self.wrong_range { from + 1 } else { from };
                headers.push((
                    "Content-Range".to_owned(),
                    format!("bytes {start}-{}/{total}", total - 1),
                ));
                Reply::Full(206, headers, file.bytes[from.min(total)..].to_vec())
            }
            _ => Reply::Full(200, headers, file.bytes),
        }
    }
}

fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(p, _)| p.to_owned())
        .unwrap_or_default()
}

fn leaf(path: &str) -> String {
    path.rsplit_once('/').map_or(path, |(_, l)| l).to_owned()
}

/// A loopback server answering every request from `fake`, one connection a
/// request (`Connection: close`).
struct Server {
    base: String,
    fake: Arc<Mutex<Fake>>,
    log: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    fn start(fake: Fake) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let fake = Arc::new(Mutex::new(fake));
        let log = Arc::new(Mutex::new(Vec::new()));
        let (f, l) = (Arc::clone(&fake), Arc::clone(&log));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let Some(request) = read_request(&mut stream) else {
                    continue;
                };
                let reply = f.lock().unwrap().answer(&request);
                l.lock().unwrap().push(request);
                write_reply(&mut stream, reply);
            }
        });
        Self { base, fake, log }
    }

    fn endpoints(&self) -> Endpoints {
        Endpoints::loopback(&self.base)
    }

    fn downloads(&self) -> Vec<Request> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.path == "/2/sharing/get_shared_link_file")
            .cloned()
            .collect()
    }
}

fn read_request(stream: &mut std::net::TcpStream) -> Option<Request> {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).ok()? == 0 {
            return None;
        }
        head.push(byte[0]);
    }
    let text = String::from_utf8_lossy(&head).into_owned();
    let mut lines = text.split("\r\n");
    let path = lines.next()?.split(' ').nth(1)?.to_owned();
    let mut headers = HashMap::new();
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
        }
    }
    let len: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0_u8; len];
    stream.read_exact(&mut body).ok()?;
    Some(Request {
        path,
        headers,
        body,
    })
}

fn write_reply(stream: &mut std::net::TcpStream, reply: Reply) {
    let (status, headers, body, send) = match reply {
        Reply::Full(s, h, b) => {
            let n = b.len();
            (s, h, b, n)
        }
        Reply::Cut(s, h, b, n) => (s, h, b, n),
    };
    let mut head = format!(
        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (k, v) in headers {
        let _ = write!(head, "{k}: {v}\r\n");
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body[..send]);
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

// -- the doubles --------------------------------------------------------------

/// A token source over a fixed list: the first read, then each re-read.
struct Tokens(Mutex<Vec<String>>, Mutex<usize>);

impl Tokens {
    fn new(tokens: &[&str]) -> Self {
        Self(
            Mutex::new(tokens.iter().map(|t| (*t).to_owned()).collect()),
            Mutex::new(0),
        )
    }
}

impl TokenSource for Tokens {
    fn read(&self) -> Result<Secret, Wait> {
        let tokens = self.0.lock().unwrap();
        let first = tokens.first().ok_or(Wait::TokenMissing {
            why: "none configured".into(),
        })?;
        Ok(Secret::new(first.clone()).unwrap())
    }

    fn reread(&self, rejected: &Secret) -> Result<Secret, Wait> {
        let mut at = self.1.lock().unwrap();
        *at += 1;
        let tokens = self.0.lock().unwrap();
        let next = tokens.get(*at).or_else(|| tokens.last()).unwrap();
        let fresh = Secret::new(next.clone()).unwrap();
        if fresh == *rejected {
            return Err(Wait::TokenDead);
        }
        Ok(fresh)
    }
}

/// A meter whose free bytes drop by `per_call` on every reading.
struct Meter {
    total: u64,
    free: Mutex<u64>,
    per_call: u64,
    calls: Mutex<usize>,
}

impl Meter {
    fn roomy() -> Self {
        Self::new(1 << 40, 1 << 40, 0)
    }

    fn new(total: u64, free: u64, per_call: u64) -> Self {
        Self {
            total,
            free: Mutex::new(free),
            per_call,
            calls: Mutex::new(0),
        }
    }
}

impl FreeSpace for Meter {
    fn space(&self, _path: &Path) -> Result<Space, String> {
        *self.calls.lock().unwrap() += 1;
        let mut free = self.free.lock().unwrap();
        let now = *free;
        *free = free.saturating_sub(self.per_call);
        Ok(Space {
            total: self.total,
            free: now,
        })
    }
}

/// A hand-off that records every file it is given.
#[derive(Default)]
struct Recorder {
    got: Vec<String>,
    refuse: bool,
    /// Renames the store root away after this many files (the drive is
    /// unplugged).
    unplug_after: Option<(usize, PathBuf)>,
}

impl Handoff for Recorder {
    fn hand(&mut self, file: &Verified) -> Handed {
        self.got.push(file.name.clone());
        if let Some((after, root)) = &self.unplug_after
            && self.got.len() == *after
        {
            std::fs::rename(root, root.with_extension("unplugged")).unwrap();
        }
        Handed {
            ok: !self.refuse,
            verdict: if self.refuse {
                "refused_for_the_test".into()
            } else {
                "written".into()
            },
        }
    }
}

// -- the inputs ---------------------------------------------------------------

const LINK: &str = "https://www.dropbox.com/sh/fixture0000/invented?dl=0";

/// An invented day zip: `members` stored members of filler text whose
/// length is about `size` each, seeded by `seed`.
fn day_zip(seed: u32, members: usize, size: usize) -> Vec<u8> {
    let names: Vec<String> = (0..members)
        .map(|i| format!("FIXTURE/INVENTED{seed:05}_{i:03}.TXT"))
        .collect();
    let bodies: Vec<Vec<u8>> = (0..members)
        .map(|i| {
            let wobble = (seed as usize * 7 + i * 13) % 97;
            let n = size + wobble * size / 970;
            (0..n).map(|k| b"invented-filler-"[(k + i) % 16]).collect()
        })
        .collect();
    let parts: Vec<(&str, &[u8], Method)> = names
        .iter()
        .zip(&bodies)
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    zip(&parts)
}

/// A store root and a tick root under one scratch folder, and a config.
struct Roots {
    store: PathBuf,
    tick: PathBuf,
}

fn roots(tag: &str) -> Roots {
    let base = scratch(&format!("dropbox-{tag}"));
    let store = base.join("store");
    let tick = base.join("ticks");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(&tick).unwrap();
    Roots { store, tick }
}

fn config() -> Config {
    Config::parse(&format!("link = \"{LINK}\"\n")).unwrap()
}

fn fast() -> Backoff {
    Backoff {
        max_attempts: 4,
        max_total: Duration::from_mins(1),
        base: Duration::from_millis(1),
        cap: Duration::from_millis(20),
        second: Duration::from_millis(1),
    }
}

fn press(
    server: &Server,
    roots: &Roots,
    token: &dyn TokenSource,
    meter: &dyn FreeSpace,
    config: Result<Config, Wait>,
    handoff: &mut dyn Handoff,
) -> Report {
    press_with(server, roots, token, meter, config, handoff, None)
}

/// [`press`] with the operator's Stop flag.
fn press_with(
    server: &Server,
    roots: &Roots,
    token: &dyn TokenSource,
    meter: &dyn FreeSpace,
    config: Result<Config, Wait>,
    handoff: &mut dyn Handoff,
    stop: Option<&AtomicBool>,
) -> Report {
    let endpoints = server.endpoints();
    let fetch = Fetch {
        store_root: &roots.store,
        tick_root: &roots.tick,
        endpoints: &endpoints,
        token,
        meter,
        floor: Floor::Percent(15),
        backoff: fast(),
        stop,
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(&fetch, config, handoff))
}

fn journal_text(roots: &Roots) -> String {
    std::fs::read_to_string(journal_path(&roots.store)).unwrap_or_default()
}

fn codes(report: &Report) -> Vec<(String, &'static str)> {
    report
        .refused
        .iter()
        .map(|r| (leaf(&r.path), r.code))
        .collect()
}

fn fake_with(files: &[(&str, Vec<u8>)]) -> Fake {
    let mut fake = Fake {
        token: "tok-one".into(),
        ..Fake::default()
    };
    for (path, bytes) in files {
        fake.file(path, bytes.clone());
    }
    fake
}

// -- the failing-first tests the package names --------------------------------

#[test]
fn a_fixture_folder_with_a_truncated_a_bad_crc_a_renamed_and_a_duplicate_file_is_refused_by_name_and_resumes_after_a_cut()
 {
    let good = day_zip(1, 6, 4_000);
    let mut truncated = day_zip(2, 6, 4_000);
    truncated.truncate(truncated.len() * 7 / 10);
    let mut bad_crc = day_zip(3, 4, 4_000);
    // Flip one byte inside the first stored member's data.
    bad_crc[30 + "FIXTURE/INVENTED00003_000.TXT".len() + 10] ^= 0x55;
    let renamed_new = day_zip(5, 3, 2_000);
    let mut fake = fake_with(&[
        ("/2019/GFDLNFO_TICK_01072019.zip", good.clone()),
        ("/2019/GFDLNFO_TICK_02072019.zip", truncated),
        ("/2019/GFDLNFO_TICK_03072019.zip", bad_crc),
        ("/2019/GFDLNFO_TICK_04072019 (1).zip", day_zip(4, 2, 1_000)),
        ("/2019/GFDLNFO_TICK_05072019.zip", renamed_new.clone()),
        ("/2019/GFDLNFO_TICK_06072019.zip", good.clone()),
    ]);
    fake.folders.push("/2019".into());
    fake.page_size = 2;
    fake.cut_first
        .insert("GFDLNFO_TICK_01072019.zip".into(), 40);
    let server = Server::start(fake);
    let roots = roots("five-kinds");
    let mut handoff = Recorder::default();
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut handoff,
    );

    assert_eq!(report.outcome, Outcome::Incomplete, "{report:#?}");
    assert_eq!(report.listed, 6);
    assert!(
        report.pages >= 4,
        "the folder was read over its pages: {}",
        report.pages
    );
    let refused = codes(&report);
    assert!(
        refused.contains(&("GFDLNFO_TICK_04072019 (1).zip".into(), "not_a_day_zip")),
        "{refused:?}"
    );
    assert!(
        refused.contains(&("GFDLNFO_TICK_06072019.zip".into(), "content_duplicate")),
        "{refused:?}"
    );
    assert!(
        refused.contains(&("GFDLNFO_TICK_02072019.zip".into(), "zip_unreadable")),
        "{refused:?}"
    );
    assert!(
        refused.contains(&("GFDLNFO_TICK_03072019.zip".into(), "crc_mismatch")),
        "{refused:?}"
    );
    let verified: Vec<&str> = report.verified.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(
        verified,
        ["GFDLNFO_TICK_01072019.zip", "GFDLNFO_TICK_05072019.zip"]
    );
    assert_eq!(handoff.got, verified);
    // The cut file resumed with a Range from where the cut left it.
    let ranges: Vec<String> = server
        .downloads()
        .iter()
        .filter_map(|r| r.headers.get("range").cloned())
        .collect();
    let cut_at = good.len() * 40 / 100;
    assert_eq!(
        ranges,
        [format!("bytes={cut_at}-")],
        "one resume, from the cut"
    );
    let staged = std::fs::read(&report.verified[0].path).unwrap();
    assert_eq!(
        staged, good,
        "the resumed file is the vendor's bytes, whole"
    );
    // The refused parts are kept, never deleted.
    let run_dir = report.staging.clone().unwrap();
    assert!(run_dir.join("GFDLNFO_TICK_03072019.zip.part").is_file());
    assert!(run_dir.join("GFDLNFO_TICK_02072019.zip.part").is_file());
    let text = journal_text(&roots);
    assert!(text.contains("reason=crc_mismatch"), "{text}");
    assert!(
        text.lines()
            .any(|l| l.starts_with("v=1 cut ") && l.contains("name=\"GFDLNFO_TICK_01072019.zip\"")),
        "{text}"
    );

    // PRESS TWO: upstream renamed the verified 05 to 08 (same content hash),
    // and a renamed-in new file 09 arrived. The renamed duplicate is refused
    // by name, the new file is verified, nothing is fetched twice.
    {
        let mut fake = server.fake.lock().unwrap();
        let moved = fake
            .files
            .iter()
            .position(|f| f.path.ends_with("05072019.zip"))
            .unwrap();
        fake.files[moved].path = "/2019/GFDLNFO_TICK_08072019.zip".into();
        fake.file("/2019/GFDLNFO_TICK_09072019.zip", day_zip(9, 2, 3_000));
    }
    let before = server.downloads().len();
    let mut second = Recorder::default();
    let again = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut second,
    );
    let refused = codes(&again);
    assert!(
        refused.contains(&("GFDLNFO_TICK_08072019.zip".into(), "content_duplicate")),
        "{refused:?}"
    );
    assert!(
        second.got.contains(&"GFDLNFO_TICK_09072019.zip".to_owned()),
        "{:?}",
        second.got
    );
    let fetched: Vec<String> = server.downloads()[before..]
        .iter()
        .map(|r| r.headers["dropbox-api-arg"].clone())
        .collect();
    assert!(
        fetched.iter().all(|a| !a.contains("01072019")),
        "a verified file is never fetched again: {fetched:?}"
    );
}

#[test]
fn nothing_is_overwritten_and_every_file_has_a_journal_line() {
    let a = day_zip(11, 3, 2_000);
    let b = day_zip(12, 3, 2_000);
    let server = Server::start(fake_with(&[
        ("/GFDLNFO_TICK_01082019.zip", a.clone()),
        ("/GFDLNFO_TICK_02082019.zip", b),
        ("/GFDLCM_INDICES_TICK_01082019.zip", day_zip(13, 2, 1_000)),
    ]));
    let roots = roots("no-overwrite");
    // The tick store already holds the index day: a published file whose
    // bytes and inode must not change.
    let held = DayIdentity::of_zip_name("GFDLCM_INDICES_TICK_01082019.zip")
        .unwrap()
        .path_under(&roots.tick);
    std::fs::create_dir_all(held.parent().unwrap()).unwrap();
    std::fs::write(&held, b"a published day file, invented").unwrap();
    let inode = |p: &Path| std::os::unix::fs::MetadataExt::ino(&std::fs::metadata(p).unwrap());
    let held_inode = inode(&held);
    let mut handoff = Recorder::default();
    let first = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut handoff,
    );
    assert_eq!(first.outcome, Outcome::Complete, "{first:#?}");
    assert_eq!(first.held, 1);
    assert_eq!(first.verified.len(), 2);
    let staged = first.verified[0].path.clone();
    let staged_inode = inode(&staged);
    // A second press over the same listing downloads nothing and changes
    // nothing on disk.
    let before = server.downloads().len();
    let mut again = Recorder::default();
    let second = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut again,
    );
    assert_eq!(second.outcome, Outcome::Complete, "{second:#?}");
    assert_eq!(server.downloads().len(), before, "nothing fetched twice");
    assert_eq!(
        std::fs::read(&held).unwrap(),
        b"a published day file, invented"
    );
    assert_eq!(inode(&held), held_inode);
    assert_eq!(std::fs::read(&staged).unwrap(), a);
    assert_eq!(inode(&staged), staged_inode);
    // Every listed file has its lines: held, listed, part, downloading,
    // downloaded, verified and handed.
    let text = journal_text(&roots);
    for name in ["GFDLNFO_TICK_01082019.zip", "GFDLNFO_TICK_02082019.zip"] {
        for verb in [
            "listed",
            "part",
            "downloading",
            "downloaded",
            "verified",
            "handed",
        ] {
            assert!(
                text.lines()
                    .any(|l| l.starts_with(&format!("v=1 {verb} ")) && l.contains(name)),
                "{verb} {name}:\n{text}"
            );
        }
    }
    assert!(
        text.lines()
            .any(|l| l.starts_with("v=1 held ") && l.contains("GFDLCM_INDICES_TICK_01082019.zip"))
    );
    // Every line parses, and the link itself is in none of them.
    assert!(text.lines().all(|l| parse_line(l).is_some()), "{text}");
    assert!(
        !text.contains("dropbox.com"),
        "the link never reaches the journal"
    );
    assert!(!text.contains("fixture0000"));
    assert_eq!(status(&roots.store).unwrap().unwrap().verb, "end");
}

#[test]
fn a_missing_link_is_a_named_wait() {
    use std::os::unix::fs::PermissionsExt as _;
    let server = Server::start(fake_with(&[(
        "/GFDLNFO_TICK_01082019.zip",
        day_zip(1, 1, 100),
    )]));
    let roots = roots("missing-link");
    let home = scratch("dropbox-home");
    let path = config_path(&home);
    let missing = Config::load(&path).unwrap_err();
    assert_eq!(missing.code(), "dropbox_link_missing");
    assert!(missing.to_string().contains("gdfl-dropbox.toml"));
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Err(missing.clone()),
        &mut Recorder::default(),
    );
    assert_eq!(report.outcome, Outcome::Waiting(missing));
    assert!(
        server.log.lock().unwrap().is_empty(),
        "no request without a link"
    );
    let last = status(&roots.store).unwrap().unwrap();
    assert_eq!(last.verb, "end");
    assert_eq!(last.get("verdict"), Some("waiting"));
    assert!(
        journal_text(&roots).contains("v=1 wait ")
            && journal_text(&roots).contains("reason=dropbox_link_missing")
    );

    // A malformed or too-open config is a named wait too, never a default.
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "link = https://www.dropbox.com/x\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(
        Config::load(&path).unwrap_err().code(),
        "dropbox_config_too_open"
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        Config::load(&path).unwrap_err().code(),
        "dropbox_link_malformed"
    );
    std::fs::write(&path, format!("# the operator's link\nlink = \"{LINK}\"\n")).unwrap();
    let loaded = Config::load(&path).unwrap();
    assert_eq!(loaded, config());
    assert!(
        !format!("{loaded:?}").contains("dropbox.com"),
        "Debug prints the print only"
    );
    // A token nobody configured is a named wait as well.
    let none = press(
        &server,
        &roots,
        &Tokens::new(&[]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(none.outcome.word(), "waiting");
    assert!(matches!(
        none.outcome,
        Outcome::Waiting(Wait::TokenMissing { .. })
    ));
}

#[test]
fn the_config_grammar_refuses_by_name() {
    for (text, words) in [
        ("", "no `link` line"),
        ("link = \"http://www.dropbox.com/sh/a\"\n", "https"),
        ("link = \"https://evil.example/sh/a\"\n", "dropbox.com"),
        ("link = \"https://www.dropbox.com/a b\"\n", "space"),
        (
            "link = \"https://www.dropbox.com/a\"\nlink = \"https://www.dropbox.com/b\"\n",
            "twice",
        ),
        ("colour = \"red\"\n", "unknown key"),
        ("just words\n", "not `key"),
        ("link = 'x'\n", "quoted"),
        ("link = \"a\\\"b\"\n", "quoted"),
    ] {
        let why = Config::parse(text).unwrap_err();
        assert!(why.contains(words), "{text:?}: {why}");
        assert!(
            !why.contains("dropbox.com/a"),
            "a refusal never quotes the link: {why}"
        );
    }
    let accepted =
        Config::parse(&format!("link = \"{LINK}\"\naccept_link_print = \"abc\"\n")).unwrap();
    assert_eq!(accepted.accept_link_print.as_deref(), Some("abc"));
    assert_eq!(config().print().len(), 32);
    assert_ne!(config().print(), accepted.print().replace('0', "1"));
}

#[test]
fn a_plan_past_the_floor_is_refused_before_any_byte() {
    let server = Server::start(fake_with(&[
        ("/GFDLNFO_TICK_01082019.zip", day_zip(21, 3, 2_000)),
        ("/GFDLNFO_TICK_02082019.zip", day_zip(22, 3, 2_000)),
    ]));
    let roots = roots("floor");
    // 1,000,000 bytes, 150,001 free: 15% is 150,000, so one byte to write
    // would leave less than the floor.
    let meter = Meter::new(1_000_000, 150_001, 0);
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &meter,
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &report.outcome else {
        panic!("{report:#?}")
    };
    assert_eq!(refusal.code, "disk_floor");
    assert!(
        server.downloads().is_empty(),
        "no download request before the floor passed"
    );
    assert!(report.staging.is_none(), "no run folder, no part, no byte");
    let staged: Vec<_> = std::fs::read_dir(staging_root(&roots.store))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        staged,
        [std::ffi::OsString::from(".lock")],
        "nothing but the lock"
    );
    assert!(journal_text(&roots).contains("reason=disk_floor"));
}

#[test]
fn the_floor_reached_mid_run_stops_before_the_next_file() {
    let a = day_zip(31, 3, 3_000);
    let b = day_zip(31, 3, 3_000);
    let size = u64::try_from(a.len()).unwrap();
    let server = Server::start(fake_with(&[
        ("/GFDLNFO_TICK_01082019.zip", a),
        (
            "/GFDLNFO_TICK_02082019.zip",
            b.iter().rev().copied().collect(),
        ),
    ]));
    let roots = roots("floor-mid");
    // The plan fits (two files and the floor), and so does the first file;
    // the reading before the second finds the disk filled by someone else
    // (each reading finds one file's bytes fewer). 15% of 100 sizes is 15.
    let total = 100 * size;
    let meter = Meter::new(total, 17 * size, size);
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &meter,
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(report.verified.len(), 1, "{report:#?}");
    let Outcome::Refused(refusal) = &report.outcome else {
        panic!("{report:#?}")
    };
    assert_eq!(refusal.code, "disk_floor");
    assert!(refusal.detail.contains("GFDLNFO_TICK_02082019.zip"));
    assert_eq!(
        server.downloads().len(),
        1,
        "the second file was never asked for"
    );
}

#[test]
fn thirty_fixture_days_land_within_the_stated_tolerance() {
    // A measured sample of ten days, projected to thirty, then thirty fetched.
    let days: Vec<(String, Vec<u8>)> = (1..=40_u32)
        .map(|n| {
            let day = crate::session::Day::from_days(18_000 + n).unwrap();
            let name = format!(
                "GFDLNFO_TICK_{:02}{:02}{:04}.zip",
                day.day(),
                day.month(),
                day.year()
            );
            (format!("/{name}"), day_zip(n, 4, 2_500))
        })
        .collect();
    let sample_bytes: u64 = days[..10]
        .iter()
        .map(|(_, b)| u64::try_from(b.len()).unwrap())
        .sum();
    let mut totals = Totals::new();
    totals
        .add_kind(
            KindKey::new("gdfl", "options", "dropbox_zip"),
            sample_bytes,
            10,
        )
        .unwrap();
    let projection = project(
        &totals.seal(),
        &[PlanItem {
            unit: Unit::Kind(KindKey::new("gdfl", "options", "dropbox_zip")),
            days: 30,
            instruments: 1,
        }],
    );
    let files: Vec<(&str, Vec<u8>)> = days[10..]
        .iter()
        .map(|(p, b)| (p.as_str(), b.clone()))
        .collect();
    let server = Server::start(fake_with(&files));
    let roots = roots("tolerance");
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(report.outcome, Outcome::Complete, "{report:#?}");
    assert_eq!(report.verified.len(), 30);
    let actual: u64 = report.verified.iter().map(|v| v.bytes).sum();
    assert!(
        within(projection.bytes, actual, FIXTURE_TOLERANCE_PERCENT),
        "projected {} against {actual} written",
        projection.bytes
    );
}

// -- the failure matrix rows ---------------------------------------------------

#[test]
fn a_link_whose_listing_holds_no_gdfl_day_zip_is_refused_before_any_download() {
    let roots_a = roots("no-gdfl");
    let server = Server::start(fake_with(&[
        ("/holiday-photos.zip", day_zip(1, 1, 100)),
        ("/notes.txt", b"x".to_vec()),
    ]));
    let report = press(
        &server,
        &roots_a,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &report.outcome else {
        panic!("{report:#?}")
    };
    assert_eq!(refusal.code, "link_holds_no_gdfl_day_zip");
    assert!(
        refusal.detail.contains("names_seen=2"),
        "{}",
        refusal.detail
    );
    assert!(server.downloads().is_empty());
    // 404, an HTML page and a file link.
    for (reply, code) in [
        (Reply::Full(404, Vec::new(), Vec::new()), "link_unreachable"),
        (
            Reply::Full(
                200,
                vec![("Content-Type".into(), "text/html".into())],
                b"<html></html>".to_vec(),
            ),
            "link_unreachable",
        ),
        (
            json_reply(
                409,
                &serde_json::json!({"error_summary": "path/not_folder/.."}),
            ),
            "link_is_a_file",
        ),
        (
            json_reply(
                409,
                &serde_json::json!({"error_summary": "shared_link_not_found/"}),
            ),
            "link_unreachable",
        ),
    ] {
        let mut fake = fake_with(&[]);
        fake.list_override = Some(reply);
        let server = Server::start(fake);
        let roots = roots("bad-link");
        let report = press(
            &server,
            &roots,
            &Tokens::new(&["tok-one"]),
            &Meter::roomy(),
            Ok(config()),
            &mut Recorder::default(),
        );
        let Outcome::Refused(refusal) = &report.outcome else {
            panic!("{report:#?}")
        };
        assert_eq!(refusal.code, code);
        assert!(
            report.staging.is_none(),
            "nothing staged for a refused listing"
        );
        assert!(journal_text(&roots).contains(&format!("reason={code}")));
    }
}

#[test]
fn s1_a_paged_listing_is_read_to_its_end_and_a_cut_listing_refuses() {
    let mut fake = fake_with(&[]);
    for n in 0..2_500_u32 {
        fake.files.push(FakeFile {
            path: format!("/bulk/INVENTED_{n:05}.txt"),
            bytes: Vec::new(),
            rev: format!("r{n}"),
            hash: String::new(),
        });
    }
    fake.file("/GFDLNFO_TICK_01082019.zip", day_zip(1, 1, 100));
    fake.folders.push("/bulk".into());
    fake.page_size = 1_000;
    let server = Server::start(fake);
    let roots = roots("paged");
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(report.listed, 2_501, "{report:#?}");
    assert_eq!(report.pages, 4);
    assert_eq!(report.outcome, Outcome::Complete);
    // The same folder with page 2 dropping its socket.
    {
        let mut fake = server.fake.lock().unwrap();
        fake.pages_served = 0;
        fake.drop_page = Some(2);
    }
    let roots = Roots {
        store: roots.store.clone(),
        tick: roots.tick.clone(),
    };
    let cut = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &cut.outcome else {
        panic!("{cut:#?}")
    };
    assert_eq!(refusal.code, "listing_incomplete");
    let text = journal_text(&roots);
    let last_listing = text
        .lines()
        .rfind(|l| l.starts_with("v=1 listing "))
        .unwrap();
    assert!(last_listing.contains("complete=false"), "{last_listing}");
}

#[test]
fn a_listing_whose_entries_field_is_renamed_is_refused_not_empty_and_a_listing_smaller_than_the_inventory_is_listing_shrank()
 {
    let mut fake = fake_with(&[
        ("/GFDLNFO_TICK_01082019.zip", day_zip(1, 1, 100)),
        ("/GFDLNFO_TICK_02082019.zip", day_zip(2, 1, 100)),
    ]);
    fake.rename_entries = true;
    let server = Server::start(fake);
    let roots = roots("shape");
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &report.outcome else {
        panic!("{report:#?}")
    };
    assert_eq!(refusal.code, "listing_shape_unknown");
    assert_eq!(refusal.detail, "field=entries");
    server.fake.lock().unwrap().rename_entries = false;
    let whole = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(whole.outcome, Outcome::Complete, "{whole:#?}");
    server.fake.lock().unwrap().files.pop();
    let shrank = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &shrank.outcome else {
        panic!("{shrank:#?}")
    };
    assert_eq!(refusal.code, "listing_shrank");
    assert_eq!(refusal.detail, "was=2 now=1");
}

#[test]
fn a_link_to_a_different_shared_folder_is_refused_until_the_operator_records_it() {
    let server = Server::start(fake_with(&[(
        "/GFDLNFO_TICK_01082019.zip",
        day_zip(1, 1, 100),
    )]));
    let roots = roots("link-changed");
    let first = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(first.outcome, Outcome::Complete);
    let other = Config::parse("link = \"https://www.dropbox.com/sh/another/invented\"\n").unwrap();
    let print = other.print();
    let refused = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(other),
        &mut Recorder::default(),
    );
    assert_eq!(
        refused.outcome,
        Outcome::Waiting(Wait::LinkChanged {
            was: config().print(),
            now: print.clone()
        })
    );
    let accepted = Config::parse(&format!(
        "link = \"https://www.dropbox.com/sh/another/invented\"\naccept_link_print = \"{print}\"\n"
    ))
    .unwrap();
    let ok = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(accepted),
        &mut Recorder::default(),
    );
    assert_eq!(ok.outcome, Outcome::Complete, "{ok:#?}");
}

#[test]
fn names_that_are_unsafe_collide_or_name_one_day_twice_are_refused_by_name() {
    let one = day_zip(1, 1, 100);
    let mut fake = fake_with(&[
        ("/GFDLNFO_TICK_01042024.zip", one.clone()),
        ("/gfdlnfo_tick_01042024.zip", day_zip(2, 1, 100)),
        ("/a/GFDLNFO_TICK_02042024.zip", day_zip(3, 1, 100)),
        ("/b/GFDLNFO_TICK_02042024.zip", day_zip(4, 1, 100)),
        ("/..GFDLNFO_TICK_03042024.zip", day_zip(5, 1, 100)),
        ("/BSE_TICK_03042024.zip", day_zip(6, 1, 100)),
        ("/GFDLNFO_TICK_05042024.zip", day_zip(7, 1, 100)),
    ]);
    fake.folders.extend(["/a".to_owned(), "/b".to_owned()]);
    let server = Server::start(fake);
    let roots = roots("names");
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let refused = codes(&report);
    assert!(
        refused.contains(&("GFDLNFO_TICK_01042024.zip".into(), "names_collide")),
        "{refused:?}"
    );
    assert!(
        refused.contains(&("gfdlnfo_tick_01042024.zip".into(), "names_collide")),
        "{refused:?}"
    );
    assert_eq!(
        refused
            .iter()
            .filter(|(_, c)| *c == "two_files_for_one_day")
            .count(),
        2,
        "{refused:?}"
    );
    assert!(
        refused.contains(&("..GFDLNFO_TICK_03042024.zip".into(), "name_unsafe")),
        "{refused:?}"
    );
    assert!(
        refused.contains(&("BSE_TICK_03042024.zip".into(), "out_of_scope_exchange")),
        "{refused:?}"
    );
    assert_eq!(report.verified.len(), 1);
    assert_eq!(report.verified[0].name, "GFDLNFO_TICK_05042024.zip");
    assert_eq!(unsafe_name("a/b"), Some("a path separator"));
    assert_eq!(unsafe_name("a\\b"), Some("a path separator"));
    assert_eq!(unsafe_name(""), Some("an empty or dot name"));
    assert_eq!(unsafe_name(".."), Some("an empty or dot name"));
    assert_eq!(unsafe_name("x\u{7}"), Some("a control character"));
    assert_eq!(unsafe_name(".hidden.zip"), Some("a hidden name"));
    assert_eq!(unsafe_name("GFDLNFO_TICK_05042024.zip"), None);
}

#[test]
fn s1_retry_after_is_honoured_and_bounded() {
    let mut fake = fake_with(&[("/GFDLNFO_TICK_01082019.zip", day_zip(1, 1, 100))]);
    fake.throttle = 3;
    fake.retry_after = Some(2);
    let server = Server::start(fake);
    let roots = roots("throttle");
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(report.outcome, Outcome::Complete, "{report:#?}");
    // Three waits of Retry-After 2 at the test's 1 ms a second.
    assert_eq!(report.waited, Duration::from_millis(6));
    // A storm: 429 forever ends in a named refusal after the bound.
    server.fake.lock().unwrap().throttle = usize::MAX;
    let before = server.log.lock().unwrap().len();
    let storm = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &storm.outcome else {
        panic!("{storm:#?}")
    };
    assert_eq!(refusal.code, "throttled");
    assert!(
        refusal.detail.contains("retry_after=2"),
        "{}",
        refusal.detail
    );
    assert_eq!(
        server.log.lock().unwrap().len() - before,
        4,
        "max_attempts requests, no more"
    );
    // 5xx backs off and then refuses by its own name.
    {
        let mut fake = server.fake.lock().unwrap();
        fake.throttle = 0;
        fake.fail5xx = usize::MAX;
    }
    let down = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &down.outcome else {
        panic!("{down:#?}")
    };
    assert_eq!(refusal.code, "server_error");
    // Waits are capped and double without a Retry-After.
    let b = fast();
    assert_eq!(b.wait(1, None), Duration::from_millis(2));
    assert_eq!(b.wait(30, None), Duration::from_millis(20));
    assert_eq!(b.wait(1, Some(u64::MAX)), Duration::from_millis(20));
    assert_eq!(Backoff::DEFAULT.wait(1, Some(3)), Duration::from_secs(3));
}

#[test]
fn s1_a_dead_dropbox_token_is_a_named_wait_never_a_refresh() {
    let server = Server::start(fake_with(&[(
        "/GFDLNFO_TICK_01082019.zip",
        day_zip(1, 1, 100),
    )]));
    let roots = roots("token");
    // The store answers the same dead value twice: one re-read, then a wait.
    let dead = press(
        &server,
        &roots,
        &Tokens::new(&["tok-dead"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(dead.outcome, Outcome::Waiting(Wait::TokenDead));
    let log = server.log.lock().unwrap().clone();
    assert_eq!(
        log.len(),
        1,
        "one request, then the re-read; no refresh route is called"
    );
    assert!(log.iter().all(|r| r.path == "/2/files/list_folder"));
    // A rotated token is taken on the re-read.
    let rotated = press(
        &server,
        &roots,
        &Tokens::new(&["tok-dead", "tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(rotated.outcome, Outcome::Complete, "{rotated:#?}");
}

#[test]
fn s1_a_resume_never_stitches_two_versions() {
    // (a) the server ignores the Range and answers 200 with the whole body:
    // the bytes go to a NEW part, never appended.
    for (ignore, wrong) in [(true, false), (false, true)] {
        let a = day_zip(41, 4, 3_000);
        let mut fake = fake_with(&[("/GFDLNFO_TICK_01082019.zip", a.clone())]);
        fake.cut_first
            .insert("GFDLNFO_TICK_01082019.zip".into(), 40);
        fake.ignore_range = ignore;
        fake.wrong_range = wrong;
        let server = Server::start(fake);
        let roots = roots("stitch");
        let report = press(
            &server,
            &roots,
            &Tokens::new(&["tok-one"]),
            &Meter::roomy(),
            Ok(config()),
            &mut Recorder::default(),
        );
        assert_eq!(report.outcome, Outcome::Complete, "{report:#?}");
        assert_eq!(std::fs::read(&report.verified[0].path).unwrap(), a);
        let run_dir = report.staging.clone().unwrap();
        // The first part keeps the cut 40%; the verified file came from a
        // second part.
        let first = std::fs::metadata(run_dir.join("GFDLNFO_TICK_01082019.zip.part"))
            .unwrap()
            .len();
        assert_eq!(first, u64::try_from(a.len() * 40 / 100).unwrap());
        assert!(journal_text(&roots).contains("v=1 restarted "));
    }
    assert_eq!(content_range("bytes 5-9/10"), Some((5, 9, 10)));
    assert_eq!(
        content_range("bytes 4294967296-8589934591/8589934592"),
        Some((4_294_967_296, 8_589_934_591, 8_589_934_592))
    );
    assert_eq!(content_range("bytes */10"), None);
    assert_eq!(content_range("items 1-2/3"), None);
}

#[test]
fn s1_stop_and_crash_both_resume_the_same_part() {
    // A press killed mid-file leaves a part and a `part` line; the next
    // press resumes it with a Range and fetches nothing it already has.
    let a = day_zip(51, 4, 3_000);
    let server = Server::start(fake_with(&[("/GFDLNFO_TICK_01082019.zip", a.clone())]));
    let roots = roots("crash");
    let crashed_run = staging_root(&roots.store).join("1-1-1");
    std::fs::create_dir_all(&crashed_run).unwrap();
    let part = crashed_run.join("GFDLNFO_TICK_01082019.zip.part");
    std::fs::write(&part, &a[..a.len() * 6 / 10]).unwrap();
    let rev = server.fake.lock().unwrap().files[0].rev.clone();
    append_line(
        &journal_path(&roots.store),
        &format!(
            "v=1 part run=1-1-1 name=\"GFDLNFO_TICK_01082019.zip\" rev=\"{rev}\" part={}",
            quoted(&part.display().to_string())
        ),
    )
    .unwrap();
    // A torn last line, as a crash mid-write leaves one.
    let mut journal = std::fs::OpenOptions::new()
        .append(true)
        .open(journal_path(&roots.store))
        .unwrap();
    journal
        .write_all(b"v=1 downloading run=1-1-1 name=\"GFDL")
        .unwrap();
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(report.outcome, Outcome::Complete, "{report:#?}");
    assert_eq!(report.resumed, 1);
    let ranges: Vec<String> = server
        .downloads()
        .iter()
        .filter_map(|r| r.headers.get("range").cloned())
        .collect();
    assert_eq!(ranges, [format!("bytes={}-", a.len() * 6 / 10)]);
    assert_eq!(std::fs::read(&report.verified[0].path).unwrap(), a);
    // The torn line was closed, and every whole line still parses.
    assert!(
        journal_text(&roots)
            .lines()
            .filter(|l| !l.is_empty())
            .count()
            > 3
    );
    let memory = Memory::load(&journal_path(&roots.store)).unwrap();
    assert_eq!(
        memory.torn, 1,
        "the closed torn line is counted, never read"
    );
    std::fs::write(journal_path(&roots.store), "a foreign line\n").unwrap();
    assert!(
        Memory::load(&journal_path(&roots.store)).is_err(),
        "a foreign line is refused"
    );
}

#[test]
fn s1_a_revoked_link_stops_the_download_stage_and_keeps_what_was_verified() {
    let mut fake = fake_with(&[
        ("/GFDLNFO_TICK_01082019.zip", day_zip(1, 1, 500)),
        ("/GFDLNFO_TICK_02082019.zip", day_zip(2, 1, 500)),
        ("/GFDLNFO_TICK_05082019.zip", day_zip(3, 1, 500)),
    ]);
    fake.gone_after = Some(2);
    let server = Server::start(fake);
    let roots = roots("revoked");
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &report.outcome else {
        panic!("{report:#?}")
    };
    assert_eq!(refusal.code, "link_revoked");
    assert_eq!(report.verified.len(), 2);
    for v in &report.verified {
        assert!(v.path.is_file(), "a verified file is kept");
    }
    assert_eq!(
        codes(&report),
        [("GFDLNFO_TICK_05082019.zip".to_owned(), "remote_gone")]
    );
}

#[test]
fn s1_an_html_body_is_never_promoted_as_a_zip_and_a_foreign_redirect_is_refused() {
    let mut fake = fake_with(&[("/GFDLNFO_TICK_01082019.zip", day_zip(1, 1, 500))]);
    fake.html.insert("GFDLNFO_TICK_01082019.zip".into());
    let server = Server::start(fake);
    let roots = roots("html");
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(
        codes(&report),
        [("GFDLNFO_TICK_01082019.zip".to_owned(), "not_a_zip")]
    );
    assert!(
        !report
            .staging
            .clone()
            .unwrap()
            .join("GFDLNFO_TICK_01082019.zip")
            .exists()
    );
    {
        let mut fake = server.fake.lock().unwrap();
        fake.html.clear();
        fake.redirect_to = Some("http://foreign.example/steal".into());
    }
    let redirected = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(
        codes(&redirected),
        [("GFDLNFO_TICK_01082019.zip".to_owned(), "redirect_refused")]
    );
    assert!(redirected.refused[0].detail.contains("foreign.example"));
    let e = Endpoints::dropbox();
    assert!(e.allows("content.dropboxapi.com"));
    assert!(e.allows("uc123.dl.dropboxusercontent.com"));
    assert!(!e.allows("dropboxusercontent.com.evil.example"));
    assert!(!e.allows("evildropboxusercontent.com"));
}

#[test]
fn the_drive_unplugged_mid_run_is_store_lost() {
    let server = Server::start(fake_with(&[
        ("/GFDLNFO_TICK_01082019.zip", day_zip(1, 1, 500)),
        ("/GFDLNFO_TICK_02082019.zip", day_zip(2, 1, 500)),
    ]));
    let roots = roots("unplug");
    let mut handoff = Recorder {
        unplug_after: Some((1, roots.store.clone())),
        ..Recorder::default()
    };
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut handoff,
    );
    let Outcome::Refused(refusal) = &report.outcome else {
        panic!("{report:#?}")
    };
    assert_eq!(refusal.code, "store_lost");
    assert_eq!(server.downloads().len(), 1);
    assert!(
        report.journal_error.is_some(),
        "a journal that cannot be written is named, never dropped"
    );
}

#[test]
fn a_member_stated_past_the_byte_cap_is_refused_before_inflating_and_sizes_are_u64() {
    // A stored member whose central record states 64 MiB + 1: refused on
    // the stated length before any inflate.
    let mut big = day_zip(1, 1, 100);
    let eocd = big.len() - 22;
    let cd = u32::from_le_bytes(big[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    big[cd + 24..cd + 28]
        .copy_from_slice(&u32::try_from(MEMBER_BYTE_CAP + 1).unwrap().to_le_bytes());
    let file = Listed {
        path: "/GFDLNFO_TICK_01082019.zip".into(),
        name: "GFDLNFO_TICK_01082019.zip".into(),
        size: u64::try_from(big.len()).unwrap(),
        rev: String::new(),
        hash: String::new(),
    };
    let dir = scratch("dropbox-cap");
    let path = dir.join("GFDLNFO_TICK_01082019.zip");
    std::fs::write(&path, &big).unwrap();
    assert_eq!(verify(&path, &file).unwrap_err().0, "member_over_byte_cap");
    // A listed size past 4 GiB is a size like any other.
    let past = Listed {
        size: (5_u64 << 30),
        ..file.clone()
    };
    assert_eq!(verify(&path, &past).unwrap_err().0, "size_mismatch");
    let mut fake = fake_with(&[]);
    fake.files.push(FakeFile {
        path: "/GFDLNFO_TICK_02082019.zip".into(),
        bytes: Vec::new(),
        rev: "r".into(),
        hash: String::new(),
    });
    let server = Server::start(fake);
    let roots = roots("zero");
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(
        codes(&report),
        [("GFDLNFO_TICK_02082019.zip".to_owned(), "size_unlikely")]
    );
}

#[test]
fn a_staging_root_inside_the_tick_store_is_refused_before_any_io() {
    let server = Server::start(fake_with(&[(
        "/GFDLNFO_TICK_01082019.zip",
        day_zip(1, 1, 100),
    )]));
    let base = scratch("dropbox-overlap");
    let roots = Roots {
        store: base.clone(),
        tick: base.clone(),
    };
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &report.outcome else {
        panic!("{report:#?}")
    };
    assert_eq!(refusal.code, "staging_overlaps_published");
    assert!(server.log.lock().unwrap().is_empty());
    assert!(!staging_root(&base).exists());
    // A store root that is not there is store_lost.
    let gone = Roots {
        store: base.join("nope"),
        tick: base.join("t"),
    };
    let lost = press(
        &server,
        &gone,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert!(matches!(&lost.outcome, Outcome::Refused(r) if r.code == "store_lost"));
}

#[test]
fn two_presses_at_once_never_share_a_staging_folder() {
    let server = Server::start(fake_with(&[(
        "/GFDLNFO_TICK_01082019.zip",
        day_zip(1, 1, 100),
    )]));
    let roots = roots("busy");
    let staging = staging_root(&roots.store);
    std::fs::create_dir_all(&staging).unwrap();
    let holder = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(staging.join(".lock"))
        .unwrap();
    holder.lock().unwrap();
    writeln!(&holder, "run=other pid=1").unwrap();
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    let Outcome::Refused(refusal) = &report.outcome else {
        panic!("{report:#?}")
    };
    assert_eq!(refusal.code, "run_busy");
    assert!(refusal.detail.contains("run=other"), "{}", refusal.detail);
    holder.unlock().unwrap();
    let ok = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(ok.outcome, Outcome::Complete, "{ok:#?}");
}

#[test]
fn a_held_import_day_is_not_fetched_and_a_writer_refusal_is_named() {
    let server = Server::start(fake_with(&[
        ("/GFDLNFO_TICK_01082019.zip", day_zip(1, 1, 100)),
        ("/GFDLNFO_TICK_02082019.zip", day_zip(2, 1, 100)),
    ]));
    let roots = roots("import-done");
    std::fs::create_dir_all(roots.store.join("imports")).unwrap();
    std::fs::write(
        crate::gdfl_import::journal_path(&roots.store),
        "run 1 definition=4\nbegin options 2019-08-01 *\ndone options 2019-08-01 * definition=4 run=1\n",
    )
    .unwrap();
    let mut handoff = Recorder {
        refuse: true,
        ..Recorder::default()
    };
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut handoff,
    );
    assert_eq!(report.held, 1, "{report:#?}");
    assert_eq!(handoff.got, ["GFDLNFO_TICK_02082019.zip"]);
    assert_eq!(report.outcome, Outcome::Incomplete);
    assert_eq!(
        report.not_handed,
        [(
            "GFDLNFO_TICK_02082019.zip".to_owned(),
            "refused_for_the_test".to_owned()
        )]
    );
    // The real writer is wired: an invented zip it cannot file is refused by
    // its own code and named.
    let options = WriteOptions::new("r4-07-test");
    let mut writer = TickStoreHandoff::new(roots.tick.clone(), options);
    let handed = writer.hand(&report.verified[0]);
    assert!(!handed.verdict.is_empty());
}

#[test]
fn the_journal_line_grammar_round_trips_and_status_reads_one_tail() {
    let text = format!(
        "v=1 refused run=1 name={} detail={}",
        quoted("a \"b\" \\ c\u{1}"),
        quoted("x y")
    );
    let fields = parse_line(&text).unwrap();
    assert_eq!(fields.verb, "refused");
    assert_eq!(fields.get("name"), Some("a \"b\" \\ c\u{1}"));
    assert_eq!(fields.get("detail"), Some("x y"));
    for bad in [
        "",
        "refused run=1",
        "v=2 refused",
        "v=1",
        "v=1 a b",
        "v=1 x name=\"open",
        "v=1 x n=\"\\u{zz}\"",
        "v=1 x n=\"\\u(1)\"",
    ] {
        assert!(parse_line(bad).is_none(), "{bad:?}");
    }
    let roots = roots("status");
    assert_eq!(status(&roots.store).unwrap(), None);
    let path = journal_path(&roots.store);
    for n in 0..5_000 {
        append_line(&path, &format!("v=1 listed run=r name=\"n{n}\" bytes={n}")).unwrap();
    }
    let last = status(&roots.store).unwrap().unwrap();
    assert_eq!(last.get("name"), Some("n4999"));
    assert_eq!(
        ascii_json(&serde_json::json!({"p": "/é"})),
        "{\"p\":\"/\\u00e9\"}"
    );
    assert_eq!(ascii_json(&serde_json::json!("𝄞")), "\"\\ud834\\udd1e\"");
}

#[test]
fn the_press_future_is_send() {
    fn is_send<T: Send>(_: &T) {}
    let endpoints = Endpoints::dropbox();
    let roots = roots("send");
    let tokens = Tokens::new(&["t"]);
    let meter = Meter::roomy();
    let fetch = Fetch {
        store_root: &roots.store,
        tick_root: &roots.tick,
        endpoints: &endpoints,
        token: &tokens,
        meter: &meter,
        floor: Floor::Percent(15),
        backoff: Backoff::DEFAULT,
        stop: None,
    };
    let mut handoff = Recorder::default();
    let future = run(&fetch, Err(Wait::TokenDead), &mut handoff);
    is_send(&future);
    drop(future);
    assert_eq!(*meter.calls.lock().unwrap(), 0);
}

#[test]
fn s1_enospc_mid_body_leaves_a_named_part_and_resumes() {
    let a = day_zip(61, 4, 3_000);
    let server = Server::start(fake_with(&[("/GFDLNFO_TICK_01082019.zip", a.clone())]));
    let roots = roots("enospc");
    let full_at = u64::try_from(a.len() * 6 / 10).unwrap();
    FULL_AT.with(|cell| cell.set(Some(full_at)));
    let full = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    FULL_AT.with(|cell| cell.set(None));
    let Outcome::Refused(refusal) = &full.outcome else {
        panic!("{full:#?}")
    };
    assert_eq!(refusal.code, "disk_full", "{refusal:?}");
    assert!(
        refusal.detail.contains("stage=download"),
        "{}",
        refusal.detail
    );
    assert!(
        full.verified.is_empty(),
        "nothing promoted from a part the disk cut"
    );
    let run_dir = full.staging.clone().unwrap();
    assert!(!run_dir.join("GFDLNFO_TICK_01082019.zip").exists());
    let kept = std::fs::metadata(run_dir.join("GFDLNFO_TICK_01082019.zip.part"))
        .unwrap()
        .len();
    assert!(
        kept <= full_at,
        "the part holds what was written before the disk filled: {kept}"
    );
    assert!(journal_text(&roots).contains("reason=disk_full"));
    // Space freed: the next press resumes the kept part from its length.
    let freed = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(freed.outcome, Outcome::Complete, "{freed:#?}");
    assert_eq!(std::fs::read(&freed.verified[0].path).unwrap(), a);
    let ranges: Vec<String> = server
        .downloads()
        .iter()
        .filter_map(|r| r.headers.get("range").cloned())
        .collect();
    if kept > 0 {
        assert_eq!(
            ranges,
            [format!("bytes={kept}-")],
            "resumed from the kept part"
        );
    }
    // Any other write failure keeps its own name.
    let other = write_failed(
        Path::new("/x.part"),
        7,
        &std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    );
    assert_eq!(other.code, "staging_write_failed");
    assert!(other.detail.contains("at 7 bytes"));
}

#[test]
fn a_redelivered_held_day_is_named_never_skipped_and_never_fetched_over() {
    let server = Server::start(fake_with(&[
        ("/GFDLNFO_TICK_01082019.zip", day_zip(71, 2, 500)),
        ("/GFDLNFO_TICK_02082019.zip", day_zip(72, 2, 500)),
    ]));
    let roots = roots("redelivered");
    let first = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(first.outcome, Outcome::Complete, "{first:#?}");
    // Both days are now held by the tick store (the writer filed them).
    for name in ["GFDLNFO_TICK_01082019.zip", "GFDLNFO_TICK_02082019.zip"] {
        let held = DayIdentity::of_zip_name(name)
            .unwrap()
            .path_under(&roots.tick);
        std::fs::create_dir_all(held.parent().unwrap()).unwrap();
        std::fs::write(&held, b"held, invented").unwrap();
    }
    // GDFL re-delivers day 01 with corrected bytes: a new content hash.
    {
        let mut fake = server.fake.lock().unwrap();
        fake.files[0].bytes = day_zip(73, 2, 500);
        fake.files[0].hash = "hash-corrected".into();
        fake.files[0].rev = "rev-corrected".into();
    }
    let before = server.downloads().len();
    let again = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(again.outcome, Outcome::Incomplete, "{again:#?}");
    assert_eq!(again.held, 1, "day 02 is unchanged and held");
    let refused = codes(&again);
    assert_eq!(
        refused,
        [("GFDLNFO_TICK_01082019.zip".to_owned(), "redelivered")]
    );
    assert!(
        again.refused[0].detail.contains("hash-corrected"),
        "{}",
        again.refused[0].detail
    );
    assert!(again.refused[0].detail.contains("held_tick_store"));
    assert_eq!(
        server.downloads().len(),
        before,
        "nothing held is fetched over"
    );
    assert!(journal_text(&roots).contains("reason=redelivered"));
}

#[test]
fn s1_a_stop_mid_download_keeps_the_part_and_the_next_press_resumes_it() {
    let a = day_zip(81, 4, 3_000);
    let b = day_zip(82, 2, 1_000);
    let flag = Arc::new(AtomicBool::new(false));
    let mut fake = fake_with(&[
        ("/GFDLNFO_TICK_01082019.zip", a.clone()),
        ("/GFDLNFO_TICK_02082019.zip", b),
    ]);
    fake.cut_first
        .insert("GFDLNFO_TICK_01082019.zip".into(), 40);
    fake.stop_on_range = Some(Arc::clone(&flag));
    let server = Server::start(fake);
    let roots = roots("stop");
    let stopped = press_with(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
        Some(&flag),
    );
    let Outcome::Refused(refusal) = &stopped.outcome else {
        panic!("{stopped:#?}")
    };
    assert_eq!(refusal.code, "stopped");
    assert!(stopped.verified.is_empty());
    let cut_at = a.len() * 40 / 100;
    let text = journal_text(&roots);
    assert!(
        text.lines().any(|l| l.starts_with("v=1 stopped ")
            && l.contains("stage=download")
            && l.contains(&format!("bytes={cut_at}"))),
        "{text}"
    );
    let asked: Vec<String> = server
        .downloads()
        .iter()
        .map(|r| r.headers["dropbox-api-arg"].clone())
        .collect();
    assert!(
        asked.iter().all(|a| !a.contains("02082019")),
        "nothing after the Stop is asked for"
    );
    // Stop pressed before a file: nothing is asked for at all.
    let before = server.downloads().len();
    let held = AtomicBool::new(true);
    let early = press_with(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
        Some(&held),
    );
    assert!(
        matches!(&early.outcome, Outcome::Refused(r) if r.code == "stopped"),
        "{early:#?}"
    );
    assert_eq!(server.downloads().len(), before);
    // The next press resumes the kept part from the cut, then takes the rest.
    flag.store(false, std::sync::atomic::Ordering::Release);
    server.fake.lock().unwrap().stop_on_range = None;
    let resumed = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(resumed.outcome, Outcome::Complete, "{resumed:#?}");
    assert_eq!(resumed.resumed, 1);
    assert_eq!(std::fs::read(&resumed.verified[0].path).unwrap(), a);
    let last_ranges: Vec<String> = server.downloads()[before..]
        .iter()
        .filter_map(|r| r.headers.get("range").cloned())
        .collect();
    assert_eq!(last_ranges, [format!("bytes={cut_at}-")]);
}

#[test]
fn s1_download_client_follows_allowlisted_redirects_and_names_its_proxy() {
    let a = day_zip(91, 2, 800);
    let mut fake = fake_with(&[("/GFDLNFO_TICK_01082019.zip", a.clone())]);
    fake.redirect_to = Some(String::new());
    let server = Server::start(fake);
    server.fake.lock().unwrap().redirect_to = Some(format!("{}/redirected", server.base));
    let roots = roots("redirect-ok");
    let report = press(
        &server,
        &roots,
        &Tokens::new(&["tok-one"]),
        &Meter::roomy(),
        Ok(config()),
        &mut Recorder::default(),
    );
    assert_eq!(report.outcome, Outcome::Complete, "{report:#?}");
    assert_eq!(std::fs::read(&report.verified[0].path).unwrap(), a);
    assert!(
        server
            .log
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.path == "/redirected"),
        "the allowlisted redirect was followed"
    );
    // A proxy is named by host only, never with its credentials.
    assert_eq!(
        proxy_host_of("http://user:secret@proxy.example:3128/"),
        Some("proxy.example:3128".to_owned())
    );
    assert_eq!(
        proxy_host_of("proxy.example:8080"),
        Some("proxy.example:8080".to_owned())
    );
    assert_eq!(proxy_host_of("  "), None);
    assert_eq!(
        Endpoints::loopback(&server.base).proxy_in_use(),
        None,
        "a client that uses no proxy names none"
    );
}

#[test]
fn a_config_folder_others_can_write_is_a_named_wait() {
    use std::os::unix::fs::PermissionsExt as _;
    let home = scratch("dropbox-home-open");
    let path = config_path(&home);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, format!("link = \"{LINK}\"\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::set_permissions(
        path.parent().unwrap(),
        std::fs::Permissions::from_mode(0o777),
    )
    .unwrap();
    let wait = Config::load(&path).unwrap_err();
    assert!(
        matches!(wait, Wait::ConfigFolderOpen { mode: 0o777, .. }),
        "{wait:?}"
    );
    assert_eq!(wait.code(), "dropbox_config_too_open");
    assert!(wait.to_string().contains("chmod go-w"), "{wait}");
    // A folder only others can read (the usual 0755) is kept.
    std::fs::set_permissions(
        path.parent().unwrap(),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    assert_eq!(Config::load(&path).unwrap(), config());
}

// -- repair round 1: the second agent's six blocking defects -------------------

mod repair_r1 {
    use super::*;
    use std::time::Instant;

    /// A held tick-store day file for `name`, as the writer would leave it.
    fn hold(roots: &Roots, name: &str) {
        let path = DayIdentity::of_zip_name(name)
            .unwrap()
            .path_under(&roots.tick);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"held by the test, invented").unwrap();
    }

    fn go(server: &Server, roots: &Roots, handoff: &mut dyn Handoff) -> Report {
        press(
            server,
            roots,
            &Tokens::new(&["tok-one"]),
            &Meter::roomy(),
            Ok(config()),
            handoff,
        )
    }

    /// A press on its own thread with `backoff` and `stop` against
    /// `endpoints`; its outcome and how long it took, or `None` after
    /// `secs`.
    fn threaded(
        endpoints: Endpoints,
        roots: &Roots,
        backoff: Backoff,
        stop: Arc<AtomicBool>,
        secs: u64,
    ) -> Option<(Outcome, Duration)> {
        let (store, tick) = (roots.store.clone(), roots.tick.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let token = Tokens::new(&["tok-one"]);
            let meter = Meter::roomy();
            let fetch = Fetch {
                store_root: &store,
                tick_root: &tick,
                endpoints: &endpoints,
                token: &token,
                meter: &meter,
                floor: Floor::Percent(15),
                backoff,
                stop: Some(&stop),
            };
            let start = Instant::now();
            let report = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(run(&fetch, Ok(config()), &mut Recorder::default()));
            let _ = tx.send((report.outcome, start.elapsed()));
        });
        rx.recv_timeout(Duration::from_secs(secs)).ok()
    }

    fn refused_code(outcome: &Outcome) -> &'static str {
        match outcome {
            Outcome::Refused(refusal) => refusal.code,
            _ => "not_refused",
        }
    }

    /// R407-01 (C1): a content duplicate with NO listed content hash is
    /// refused by its SHA-256 on press one, and stays refused on presses two
    /// and three after the original is held: never handed, never fetched
    /// again, and its refusal journalled once. Failed before the repair:
    /// press two handed `GFDLNFO_TICK_04072019.zip`.
    #[test]
    fn a_duplicate_refused_by_its_sha256_stays_refused_on_every_later_press() {
        let same = day_zip(31, 2, 3_000);
        let mut fake = fake_with(&[
            ("/GFDLNFO_TICK_03072019.zip", same.clone()),
            ("/GFDLNFO_TICK_04072019.zip", same),
        ]);
        for file in &mut fake.files {
            file.hash = String::new();
        }
        let server = Server::start(fake);
        let roots = roots("r1-dup");
        let mut first = Recorder::default();
        let one = go(&server, &roots, &mut first);
        assert_eq!(first.got, vec!["GFDLNFO_TICK_03072019.zip".to_owned()]);
        assert_eq!(
            codes(&one),
            vec![("GFDLNFO_TICK_04072019.zip".to_owned(), "content_duplicate")]
        );
        hold(&roots, "GFDLNFO_TICK_03072019.zip");
        let downloads = server.downloads().len();
        for press_no in 2..=3 {
            let mut again = Recorder::default();
            let report = go(&server, &roots, &mut again);
            assert!(
                again.got.is_empty(),
                "press {press_no} handed {:?}",
                again.got
            );
            assert_eq!(
                codes(&report),
                vec![("GFDLNFO_TICK_04072019.zip".to_owned(), "content_duplicate")],
                "press {press_no}"
            );
            assert!(
                report.refused[0]
                    .detail
                    .contains("GFDLNFO_TICK_03072019.zip"),
                "{:?}",
                report.refused[0]
            );
            assert_eq!(report.held, 1);
            assert_eq!(report.outcome, Outcome::Incomplete);
        }
        assert_eq!(server.downloads().len(), downloads, "nothing fetched again");
        let text = journal_text(&roots);
        let refusals = text
            .lines()
            .filter(|l| {
                l.starts_with("v=1 refused ")
                    && l.contains("GFDLNFO_TICK_04072019.zip")
                    && l.contains("reason=content_duplicate")
            })
            .count();
        assert_eq!(refusals, 1, "journalled once:\n{text}");
        let verified = text
            .lines()
            .filter(|l| l.starts_with("v=1 verified ") && l.contains("GFDLNFO_TICK_04072019.zip"))
            .count();
        assert_eq!(verified, 1, "verified once, never read again:\n{text}");
    }

    /// The memory keeps the FIRST name a hash was verified under.
    #[test]
    fn the_memory_keeps_the_first_name_of_a_hash() {
        let mut memory = Memory::default();
        for name in ["A.zip", "B.zip"] {
            memory.read(
                &parse_line(&format!(
                    "v=1 verified run=r at_ms=1 name=\"{name}\" rev=\"1\" hash=\"\" \
                     path=\"/p/{name}\" bytes=1 entries=1 sha256=abc how=downloaded"
                ))
                .unwrap(),
            );
        }
        assert_eq!(memory.hashes.get("abc").map(String::as_str), Some("A.zip"));
        memory.read(
            &parse_line(
                "v=1 refused run=r at_ms=1 name=\"B.zip\" rev=\"1\" \
                 reason=content_duplicate of=\"A.zip\" detail=\"x\"",
            )
            .unwrap(),
        );
        assert_eq!(
            memory
                .duplicates
                .get(&("B.zip".to_owned(), "1".to_owned()))
                .map(String::as_str),
            Some("A.zip")
        );
    }

    /// R407-05 (C2): a listing that says `has_more` forever with no entry
    /// is refused `listing_incomplete` after `MAX_STALLED_PAGES` pages. Failed
    /// before the repair: tens of thousands of requests and no end.
    #[test]
    fn an_endless_empty_listing_is_refused_after_a_bounded_number_of_pages() {
        let mut fake = fake_with(&[]);
        fake.list_override = Some(json_reply(
            200,
            &serde_json::json!({"entries": [], "cursor": "c", "has_more": true}),
        ));
        let server = Server::start(fake);
        let roots = roots("r1-endless");
        let got = threaded(
            server.endpoints(),
            &roots,
            fast(),
            Arc::new(AtomicBool::new(false)),
            20,
        );
        let requests = server.log.lock().unwrap().len();
        let (outcome, _) = got.expect("the press ended");
        assert_eq!(refused_code(&outcome), "listing_incomplete", "{outcome:?}");
        assert!(
            requests <= MAX_STALLED_PAGES + 1,
            "{requests} listing requests"
        );
        let text = journal_text(&roots);
        assert!(
            text.lines().any(|l| l.starts_with("v=1 listing ")
                && l.contains("complete=false")
                && l.contains("reason=listing_incomplete")),
            "{text}"
        );
    }

    /// R407-05 (C2, C8): a page served again and again (the same entries,
    /// the same cursor) is one listing of each file, its repeats counted,
    /// and a stall refused by name; never a file listed twice.
    #[test]
    fn a_page_served_again_is_a_repeat_and_a_stall_never_a_second_file() {
        let mut fake = fake_with(&[]);
        fake.list_override = Some(json_reply(
            200,
            &serde_json::json!({
                "entries": [{
                    ".tag": "file",
                    "name": "GFDLNFO_TICK_01072019.zip",
                    "path_display": "/GFDLNFO_TICK_01072019.zip",
                    "size": 100,
                    "rev": "0123456789a",
                }],
                "cursor": "same",
                "has_more": true,
            }),
        ));
        let server = Server::start(fake);
        let roots = roots("r1-repeat");
        let report = go(&server, &roots, &mut Recorder::default());
        let Outcome::Refused(refusal) = &report.outcome else {
            panic!("{report:#?}");
        };
        assert_eq!(refusal.code, "listing_incomplete", "{refusal:?}");
        assert!(refusal.detail.contains("kept the cursor"), "{refusal:?}");
        assert!(server.log.lock().unwrap().len() <= MAX_STALLED_PAGES + 1);
    }

    /// R407-11 (C2): Stop pressed before the press, over an endless
    /// listing, ends it `stopped` at once. Failed before the repair: the
    /// press never returned.
    #[test]
    fn stop_ends_a_press_during_its_listing() {
        let mut fake = fake_with(&[]);
        fake.list_override = Some(json_reply(
            200,
            &serde_json::json!({"entries": [], "cursor": "c", "has_more": true}),
        ));
        let server = Server::start(fake);
        let roots = roots("r1-stop-listing");
        let got = threaded(
            server.endpoints(),
            &roots,
            fast(),
            Arc::new(AtomicBool::new(true)),
            20,
        );
        let (outcome, took) = got.expect("the press ended");
        assert_eq!(refused_code(&outcome), "stopped", "{outcome:?}");
        assert!(took < Duration::from_secs(5), "{took:?}");
        assert!(server.log.lock().unwrap().len() <= 1);
    }

    /// R407-11: Stop pressed during a long back-off sleep ends the press
    /// within a poll, not after the sleep.
    #[test]
    fn stop_ends_a_press_during_a_back_off_sleep() {
        let mut fake = fake_with(&[("/GFDLNFO_TICK_01072019.zip", day_zip(1, 1, 50))]);
        fake.throttle = usize::MAX;
        let server = Server::start(fake);
        let roots = roots("r1-stop-sleep");
        let stop = Arc::new(AtomicBool::new(false));
        let slow = Backoff {
            max_attempts: 50,
            max_total: Duration::from_hours(1),
            base: Duration::from_mins(1),
            cap: Duration::from_mins(10),
            second: Duration::from_secs(1),
        };
        let flag = Arc::clone(&stop);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            flag.store(true, Ordering::Release);
        });
        let got = threaded(server.endpoints(), &roots, slow, stop, 30);
        let (outcome, took) = got.expect("the press ended");
        assert_eq!(refused_code(&outcome), "stopped", "{outcome:?}");
        assert!(took < Duration::from_secs(10), "{took:?}");
    }

    /// R407-11: Stop pressed while a request waits on a server that never
    /// answers ends the press within a poll, not after the read timeout.
    #[test]
    fn stop_ends_a_press_whose_request_is_never_answered() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let mut held = Vec::new();
            for stream in listener.incoming() {
                held.push(stream);
            }
        });
        let roots = roots("r1-stop-silent");
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            flag.store(true, Ordering::Release);
        });
        let got = threaded(Endpoints::loopback(&base), &roots, fast(), stop, 30);
        let (outcome, took) = got.expect("the press ended");
        assert_eq!(refused_code(&outcome), "stopped", "{outcome:?}");
        assert!(took < Duration::from_secs(10), "{took:?}");
    }

    /// A press waits on `429` no longer than its total: a storm under a
    /// small `max_total` is `throttled`, naming the total.
    #[test]
    fn the_total_wait_of_a_press_is_capped() {
        let mut fake = fake_with(&[("/GFDLNFO_TICK_01072019.zip", day_zip(1, 1, 50))]);
        fake.throttle = usize::MAX;
        let server = Server::start(fake);
        let roots = roots("r1-total");
        let capped = Backoff {
            max_attempts: 1_000,
            max_total: Duration::from_millis(40),
            base: Duration::from_millis(5),
            cap: Duration::from_millis(10),
            second: Duration::from_millis(1),
        };
        let got = threaded(
            server.endpoints(),
            &roots,
            capped,
            Arc::new(AtomicBool::new(false)),
            30,
        );
        let (outcome, _) = got.expect("the press ended");
        let Outcome::Refused(refusal) = &outcome else {
            panic!("{outcome:?}");
        };
        assert_eq!(refusal.code, "throttled", "{refusal:?}");
        assert!(refusal.detail.contains("total wait"), "{refusal:?}");
    }

    /// One complete press, then `fragment` appended raw to the journal as a
    /// crash would leave it; presses two to four are never
    /// `journal_unreadable`, and each later press counts the torn line.
    fn torn_tail_never_blocks(tag: &str, fragment: &[u8]) {
        let server = Server::start(fake_with(&[(
            "/GFDLNFO_TICK_05082019.zip",
            day_zip(5, 1, 100),
        )]));
        let roots = roots(tag);
        let one = go(&server, &roots, &mut Recorder::default());
        assert_eq!(one.outcome, Outcome::Complete, "{one:#?}");
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(journal_path(&roots.store))
            .unwrap();
        file.write_all(fragment).unwrap();
        drop(file);
        for press_no in 2..=4 {
            let report = go(&server, &roots, &mut Recorder::default());
            assert_eq!(
                report.outcome,
                Outcome::Complete,
                "{tag} press {press_no}: {report:#?}"
            );
            assert_eq!(report.journal_torn, 1, "{tag} press {press_no}");
        }
        let text = String::from_utf8_lossy(&std::fs::read(journal_path(&roots.store)).unwrap())
            .into_owned();
        assert!(
            text.lines()
                .any(|l| l.starts_with("v=1 begin ") && l.contains("torn=1")),
            "{tag}: the torn line is journalled"
        );
        assert_eq!(status(&roots.store).unwrap().unwrap().verb, "end");
    }

    /// R407-02 (C3): a line torn after two bytes. Failed before the repair:
    /// press three and every later press refused `journal_unreadable`.
    #[test]
    fn a_line_torn_inside_its_first_bytes_costs_one_line() {
        torn_tail_never_blocks("r1-torn-short", b"v=");
        torn_tail_never_blocks("r1-torn-v", b"v");
    }

    /// R407-02 (C3b): a NUL tail a lost block left, alone or after a line's
    /// start. Failed before the repair, as above.
    #[test]
    fn a_nul_tail_costs_one_line() {
        torn_tail_never_blocks("r1-torn-nul", &[0_u8; 37]);
        torn_tail_never_blocks("r1-torn-v-nul", b"v=1 listed run=x name=\"A\0\0\0\0");
    }

    /// R407-02: a line cut inside a two-byte character (the journal keeps
    /// names as they are) is torn, not an unreadable journal.
    #[test]
    fn a_line_cut_inside_a_character_costs_one_line() {
        torn_tail_never_blocks("r1-torn-utf8", b"v=1 refused run=x name=\"kopie \xc3");
    }

    /// A line this journal never writes, in the middle, is still refused by
    /// its number: the tolerance is for torn writes only.
    #[test]
    fn a_foreign_line_is_still_refused_by_its_number() {
        let roots = roots("r1-foreign");
        let path = journal_path(&roots.store);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            b"v=1 begin run=a at_ms=1 link=x\nhello, not a journal\nv=1 end run=a at_ms=2 verdict=complete\n",
        )
        .unwrap();
        let why = Memory::load(&path).unwrap_err();
        assert!(why.contains("line 2: not a journal line"), "{why}");
        assert_eq!(status(&roots.store).unwrap().unwrap().verb, "end");
    }

    /// The torn-shape rule on its own.
    #[test]
    fn the_torn_shapes_are_prefixes_of_this_journals_lines_or_nul() {
        for torn in [
            &b"v"[..],
            b"v=",
            b"v=1",
            b"v=1 ",
            b"v=1 begin run=",
            b"\0\0\0",
            b"\0 rev=\"3\"",
            b"v=\0\0",
            b"v=1 x \xc3",
        ] {
            assert!(torn_shape(torn), "{torn:?}");
        }
        for foreign in [&b""[..], b"hello", b"v=2 begin", b"x=1 ", b" v=1 begin"] {
            assert!(!torn_shape(foreign), "{foreign:?}");
        }
    }

    /// The O(1) status read (C4): non-ASCII journal lines, the 64 KiB
    /// window starting on a UTF-8 continuation byte. Failed before the
    /// repair: `stream did not contain valid UTF-8`.
    #[test]
    fn status_reads_the_last_line_whatever_byte_its_window_starts_on() {
        let roots = roots("r1-status-utf8");
        let path = journal_path(&roots.store);
        let mut journal = Journal {
            path: path.clone(),
            run: "r".into(),
            lines: 0,
            error: None,
        };
        let mut hit = false;
        for i in 0..4_000 {
            journal.line(
                "refused",
                &[
                    (
                        "name",
                        quoted(&format!(
                            "GFDLNFO_TICK_01072019 \u{2013} kopie {i} \u{e9}t\u{e9}.zip"
                        )),
                    ),
                    ("reason", "not_a_day_zip".to_owned()),
                ],
            );
            let bytes = std::fs::read(&path).unwrap();
            if bytes.len() > 65_536 && (bytes[bytes.len() - 65_536] & 0xC0) == 0x80 {
                hit = true;
                break;
            }
        }
        assert!(hit, "the window never started inside a character");
        let last = status(&roots.store).unwrap().unwrap();
        assert_eq!(last.verb, "refused");
        assert!(last.get("name").unwrap().contains('\u{e9}'));
        assert!(journal.error.is_none());
    }

    /// The status read passes a torn last line over, reads a last line
    /// longer than 64 KiB by widening its window, and names a tail with no
    /// whole line in `MAX_STATUS_WINDOW` bytes rather than answer `None`.
    #[test]
    fn status_passes_a_torn_line_widens_for_a_long_line_and_names_a_lineless_tail() {
        let roots = roots("r1-status-wide");
        let path = journal_path(&roots.store);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let long = "a".repeat(200 * 1_024);
        let mut text =
            format!("v=1 begin run=r at_ms=1 link=x\nv=1 ignored run=r at_ms=2 count={long}\n");
        std::fs::write(&path, &text).unwrap();
        let last = status(&roots.store).unwrap().unwrap();
        assert_eq!(last.verb, "ignored");
        assert_eq!(last.get("count").map(str::len), Some(long.len()));
        text.push_str("v=1 end run=r at_\n");
        std::fs::write(&path, &text).unwrap();
        assert_eq!(status(&roots.store).unwrap().unwrap().verb, "ignored");
        let huge = "b".repeat(usize::try_from(MAX_STATUS_WINDOW).unwrap() + 10);
        std::fs::write(
            &path,
            format!("v=1 begin run=r at_ms=1 link=x\nv=1 ignored run=r count={huge}\n"),
        )
        .unwrap();
        let why = status(&roots.store).unwrap_err();
        assert!(why.contains("no whole journal line"), "{why}");
        std::fs::write(&path, b"v=").unwrap();
        assert_eq!(status(&roots.store).unwrap(), None);
    }

    /// R407-02 (C5): a journal that cannot be written stops the press before
    /// any byte is fetched: `journal_unwritable`, nothing handed, nothing
    /// downloaded, on every press. Failed before the repair: Complete, the
    /// file handed, and downloaded again by press two.
    #[test]
    fn a_journal_that_cannot_be_written_stops_the_press_before_any_byte() {
        use std::os::unix::fs::PermissionsExt as _;
        let server = Server::start(fake_with(&[(
            "/GFDLNFO_TICK_08082019.zip",
            day_zip(8, 1, 100),
        )]));
        let roots = roots("r1-unwritable");
        let dir = journal_path(&roots.store).parent().unwrap().to_owned();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let mut reports = Vec::new();
        let mut handed = Vec::new();
        for _ in 0..2 {
            let mut recorder = Recorder::default();
            reports.push(go(&server, &roots, &mut recorder));
            handed.extend(recorder.got);
        }
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        for report in &reports {
            assert_eq!(
                refused_code(&report.outcome),
                "journal_unwritable",
                "{report:#?}"
            );
            assert!(report.journal_error.is_some());
            assert_eq!(report.listed, 0, "nothing listed: {report:#?}");
        }
        assert!(handed.is_empty(), "{handed:?}");
        assert!(server.log.lock().unwrap().is_empty(), "no request at all");
    }

    /// L-R407-3 (C6): a press over N held days adds a constant number of
    /// journal lines once each held day has its line. Failed before the
    /// repair: every press added N + 4 lines, one sync each.
    #[test]
    fn held_days_are_journalled_once_not_once_a_press() {
        let names: Vec<String> = (1..=40)
            .map(|i| {
                let (day, month) = (1 + (i - 1) % 28, 1 + (i - 1) / 28);
                format!("GFDLNFO_TICK_{day:02}{month:02}2019.zip")
            })
            .collect();
        let files: Vec<(String, Vec<u8>)> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                (
                    format!("/{name}"),
                    day_zip(u32::try_from(i).unwrap() + 1, 1, 30),
                )
            })
            .chain(std::iter::once((
                "/not a day zip.zip".to_owned(),
                day_zip(99, 1, 30),
            )))
            .collect();
        let refs: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(p, b)| (p.as_str(), b.clone())).collect();
        let server = Server::start(fake_with(&refs));
        let roots = roots("r1-held");
        for name in &names {
            hold(&roots, name);
        }
        let mut added = Vec::new();
        let mut before = 0;
        for _ in 0..3 {
            let report = go(&server, &roots, &mut Recorder::default());
            assert_eq!(report.held, names.len());
            assert_eq!(codes(&report).len(), 1, "{report:#?}");
            assert!(report.staging.is_none(), "nothing wanted, no run folder");
            let lines = journal_text(&roots).lines().count();
            added.push(lines - before);
            before = lines;
        }
        assert!(added[0] > names.len(), "{added:?}");
        assert_eq!(added[1], added[2], "{added:?}");
        assert!(added[1] <= 6, "a later press adds a constant: {added:?}");
        let text = journal_text(&roots);
        assert_eq!(
            text.lines()
                .filter(|l| l.starts_with("v=1 held ") && l.contains(&names[0]))
                .count(),
            1,
            "{text}"
        );
        assert_eq!(
            text.lines()
                .filter(|l| l.starts_with("v=1 picks ") && l.contains("held=40"))
                .count(),
            3
        );
        assert!(server.downloads().is_empty());
    }

    /// C12: a store root that is there but may not be written is
    /// `store_not_writable`, never `store_lost`.
    #[test]
    fn a_read_only_store_root_is_not_writable_not_lost() {
        use std::os::unix::fs::PermissionsExt as _;
        let server = Server::start(fake_with(&[(
            "/GFDLNFO_TICK_08082019.zip",
            day_zip(8, 1, 100),
        )]));
        let roots = roots("r1-readonly");
        std::fs::set_permissions(&roots.store, std::fs::Permissions::from_mode(0o555)).unwrap();
        let report = go(&server, &roots, &mut Recorder::default());
        std::fs::set_permissions(&roots.store, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            refused_code(&report.outcome),
            "store_not_writable",
            "{report:#?}"
        );
    }

    /// R407-18 with presses at once: the journal's folder made by two first
    /// writers at the same moment is one folder, never a lost line. Failed
    /// before the repair: the writer that lost the race to `create_dir` got
    /// `File exists`, and since a lost line now stops the press, a press
    /// beside another on a fresh store ended `journal_unwritable` (the
    /// second agent's C7: 1 of 20).
    #[test]
    fn first_writers_at_once_share_the_journal_folder_and_lose_no_line() {
        const WRITERS: usize = 16;
        for round in 0..40 {
            let roots = roots(&format!("r1-race-{round}"));
            let path = journal_path(&roots.store);
            assert!(!path.parent().unwrap().exists());
            let gate = std::sync::Barrier::new(WRITERS);
            let results: Vec<Result<(), String>> = std::thread::scope(|scope| {
                let handles: Vec<_> = (0..WRITERS)
                    .map(|writer| {
                        let (gate, path) = (&gate, &path);
                        scope.spawn(move || {
                            gate.wait();
                            append_line(
                                path,
                                &format!("v=1 end run=w{writer} at_ms={round} verdict=run_busy"),
                            )
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });
            assert!(
                results.iter().all(Result::is_ok),
                "round {round}: {results:?}"
            );
            let text = std::fs::read_to_string(&path).unwrap();
            assert_eq!(text.lines().count(), WRITERS, "{text}");
            assert!(text.lines().all(|l| parse_line(l).is_some()), "{text}");
        }
    }

    /// Presses at once on a fresh store: one holds the staging lock, the
    /// others are `run_busy` or run after it; none is `journal_unwritable`
    /// and every journal line is whole.
    #[test]
    fn presses_at_once_on_a_fresh_store_never_lose_a_journal_line() {
        let server = Server::start(fake_with(&[
            ("/GFDLNFO_TICK_09082019.zip", day_zip(9, 2, 2_000)),
            ("/GFDLNFO_TICK_12082019.zip", day_zip(12, 2, 2_000)),
        ]));
        let world = roots("r1-presses-at-once");
        let endpoints = server.endpoints();
        let gate = std::sync::Barrier::new(12);
        let words: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..12)
                .map(|_| {
                    let (world, endpoints, gate) = (&world, &endpoints, &gate);
                    scope.spawn(move || {
                        let token = Tokens::new(&["tok-one"]);
                        let meter = Meter::roomy();
                        let fetch = Fetch {
                            store_root: &world.store,
                            tick_root: &world.tick,
                            endpoints,
                            token: &token,
                            meter: &meter,
                            floor: Floor::Percent(15),
                            backoff: fast(),
                            stop: None,
                        };
                        gate.wait();
                        let report = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap()
                            .block_on(run(&fetch, Ok(config()), &mut Recorder::default()));
                        match &report.outcome {
                            Outcome::Refused(why) => why.code.to_owned(),
                            other => other.word().to_owned(),
                        }
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert!(
            words.iter().all(|w| w == "complete" || w == "run_busy"),
            "{words:?}"
        );
        assert!(words.iter().any(|w| w == "complete"), "{words:?}");
        let text = journal_text(&world);
        assert!(text.lines().all(|l| parse_line(l).is_some()), "{text}");
    }

    /// The measurement behind docs/06 L-R407-3 (run with `--ignored
    /// --nocapture`): three presses over 1,000 held days, the lines and the
    /// time each adds, and `Memory::load` and `status` p50, p99 and max over
    /// 31 reads of journals of 1,000, 10,000 and 100,000 lines, some of them
    /// non-ASCII. Asserts the constant per-press growth it reports.
    #[test]
    #[ignore = "a measurement: run by name with --ignored --nocapture"]
    fn measure_the_journal_costs_of_a_press() {
        let mut names = Vec::new();
        'outer: for year in 2019..2027 {
            for month in 1..=12 {
                for day in 1..=28 {
                    names.push(format!("GFDLNFO_TICK_{day:02}{month:02}{year}.zip"));
                    if names.len() == 1_000 {
                        break 'outer;
                    }
                }
            }
        }
        let files: Vec<(String, Vec<u8>)> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                (
                    format!("/{name}"),
                    day_zip(u32::try_from(i).unwrap() + 1, 1, 30),
                )
            })
            .collect();
        let refs: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(p, b)| (p.as_str(), b.clone())).collect();
        let server = Server::start(fake_with(&refs));
        let world = roots("r1-measure");
        for name in &names {
            hold(&world, name);
        }
        let mut before = 0;
        let mut added = Vec::new();
        for press_no in 1..=3 {
            let start = Instant::now();
            let report = go(&server, &world, &mut Recorder::default());
            let took = start.elapsed();
            let lines = journal_text(&world).lines().count();
            println!(
                "MEASURE press {press_no}: n=1000 held={} outcome={} took_ms={} lines_added={}",
                report.held,
                report.outcome.word(),
                took.as_millis(),
                lines - before
            );
            added.push(lines - before);
            before = lines;
        }
        assert_eq!(added[1], added[2]);
        assert!(added[1] <= 6, "{added:?}");
        for count in [1_000_usize, 10_000, 100_000] {
            let roots = roots(&format!("r1-measure-{count}"));
            let path = journal_path(&roots.store);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut text = String::new();
            for i in 0..count {
                let _ = writeln!(
                    text,
                    "v=1 verified run=r at_ms={i} name=\"GFDLNFO_TICK_{:02}072019 \u{e9}t\u{e9} {i}.zip\" \
                     rev=\"{i:011}\" hash=\"\" path=\"/s/{i}\" bytes=100 entries=1 sha256={i:064x} how=downloaded",
                    1 + i % 28
                );
            }
            std::fs::write(&path, &text).unwrap();
            let mut load = Vec::new();
            let mut tail = Vec::new();
            for _ in 0..31 {
                let start = Instant::now();
                let memory = Memory::load(&path).unwrap();
                load.push(start.elapsed());
                assert_eq!(memory.verified.len(), count);
                let start = Instant::now();
                assert!(status(&roots.store).unwrap().is_some());
                tail.push(start.elapsed());
            }
            load.sort();
            tail.sort();
            println!(
                "MEASURE lines={count} bytes={} Memory::load p50={:?} p99={:?} max={:?} | status p50={:?} p99={:?} max={:?} (n=31 each, wall clock, debug build)",
                text.len(),
                load[15],
                load[30],
                load[30],
                tail[15],
                tail[30],
                tail[30]
            );
        }
    }

    /// A redirect to an allowlisted host under another scheme (here `https`
    /// where the routes are `http`; in the app, `http` where they are
    /// `https`: a downgrade) is `redirect_refused`, never followed.
    #[test]
    fn a_redirect_that_changes_the_scheme_is_refused() {
        let mut fake = fake_with(&[("/GFDLNFO_TICK_01082019.zip", day_zip(1, 1, 500))]);
        fake.redirect_to = Some("https://127.0.0.1:9/redirected".into());
        let server = Server::start(fake);
        let roots = roots("r1-scheme");
        let report = go(&server, &roots, &mut Recorder::default());
        assert_eq!(
            codes(&report),
            [("GFDLNFO_TICK_01082019.zip".to_owned(), "redirect_refused")],
            "{report:#?}"
        );
        assert!(
            report.refused[0].detail.contains("scheme=https"),
            "{:?}",
            report.refused[0]
        );
    }
}

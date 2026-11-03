#![cfg(test)]
//! BASE-COV-PULL repair 2 (D-7334): the Dropbox token as `ParameterStoreToken`
//! reads it (`CLAUDE.md` §8): through the credential reader, over an
//! in-memory secret source a test answers, never AWS. Every path segment and
//! value is invented.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt as _;

use super::*;
use crate::config::CredentialPath;
use crate::gdfl_fixtures::{Method, scratch, zip};
use crate::secret::SecretError;

/// A credential config naming the token field for `gdfl` (`with`) or naming
/// no field of `gdfl` at all.
fn config(with: bool) -> CredentialConfig {
    let vendor = if with {
        "[vendor.gdfl]\nvendor = \"vendorthree\"\nfields = [\"dropbox_token\"]\n"
    } else {
        "[vendor.dhan]\nvendor = \"vendortwo\"\nfields = [\"fieldone\"]\n"
    };
    CredentialConfig::parse(&format!(
        "org    = \"orgone\"\nenv    = \"testenv\"\nregion = \"ap-south-1\"\n\n{vendor}"
    ))
    .unwrap()
}

/// A secret source that answers every read the same.
struct Answers(Result<&'static str, SecretError>);

impl SecretSource for Answers {
    fn read(&self, _path: &CredentialPath<'_>) -> Result<Secret, SecretError> {
        self.0.and_then(|value| Secret::new(value.to_owned()))
    }
}

fn secret(value: &str) -> Secret {
    Secret::new(value.to_owned()).unwrap()
}

/// The token is read from the path the config names; a re-read after a
/// rejection that returns the same value is a dead token, never a retry with
/// it; a source that refuses is a missing token in the source's words; and
/// a config that names no token field for `gdfl` is a missing token on both
/// doors (extremes at once: the rejected value read again, and no field).
#[test]
fn the_dropbox_token_is_read_where_the_config_names_it_and_a_dead_one_is_named() {
    let token = ParameterStoreToken::new(
        config(true),
        CredentialReader::new(Answers(Ok("zq-invented-token"))),
    );
    let read = token.read().unwrap();
    assert_eq!(read, secret("zq-invented-token"));
    assert_eq!(token.reread(&read).unwrap_err(), Wait::TokenDead);
    let refused = ParameterStoreToken::new(
        config(true),
        CredentialReader::new(Answers(Err(SecretError::NotFound))),
    );
    let Wait::TokenMissing { why } = refused.read().unwrap_err() else {
        panic!("a refused read is a missing token");
    };
    assert!(!why.is_empty());
    let unnamed = ParameterStoreToken::new(
        config(false),
        CredentialReader::new(Answers(Ok("zq-invented-token"))),
    );
    let Wait::TokenMissing { why } = unnamed.read().unwrap_err() else {
        panic!("no field is a missing token");
    };
    assert!(
        why.starts_with("the credential config names no dropbox_token for gdfl: "),
        "{why}"
    );
    let Wait::TokenMissing { why } = unnamed.reread(&read).unwrap_err() else {
        panic!("no field is a missing token on the re-read too");
    };
    assert!(!why.is_empty());
}

// -- BASE-COV-PULL-PS3 (db): what a direct call or a small loopback socket ---
// -- reaches. Every link, name, byte and host is invented. -------------------

/// The shared link these tests configure: invented, never a real folder.
const LINK: &str = "https://www.dropbox.com/sh/fixture0000/invented?dl=0";

/// `future` run to its end on a runtime of its own.
fn block<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

/// A new scratch folder only its owner may write (mode 0o755).
fn closed_folder(tag: &str) -> PathBuf {
    let folder = scratch(tag);
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
    folder
}

/// `bytes` written to `path` with `mode`.
fn write_mode(path: &Path, bytes: &[u8], mode: u32) {
    std::fs::write(path, bytes).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// A loopback server answering every connection with `reply`, as raw bytes,
/// once the request's head is read; its base URL.
fn raw_server(reply: &'static [u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut head = Vec::new();
            let mut byte = [0_u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                head.push(byte[0]);
            }
            stream.write_all(reply).unwrap();
            stream.shutdown(std::net::Shutdown::Both).unwrap();
        }
    });
    base
}

/// A client of this test's own, without a proxy.
fn plain_client() -> reqwest::Client {
    crate::ensure_tls_provider();
    reqwest::Client::builder().no_proxy().build().unwrap()
}

/// Every wait names what only the operator can do, in words that never quote
/// the link (`Wait::fmt`, each variant).
#[test]
fn every_wait_names_what_the_operator_must_do() {
    let words = |wait: Wait| (wait.code(), wait.to_string());
    assert_eq!(
        words(Wait::LinkMalformed {
            why: "line 3: unknown key".into()
        }),
        (
            "dropbox_link_malformed",
            "the Dropbox config is not readable: line 3: unknown key".to_owned()
        )
    );
    assert_eq!(
        words(Wait::ConfigTooOpen { mode: 0o644 }),
        (
            "dropbox_config_too_open",
            "the Dropbox config is readable by group or world (mode 644); chmod 600 it".to_owned()
        )
    );
    // The widest mode there is: every bit named, in octal.
    assert!(
        Wait::ConfigTooOpen { mode: 0o777 }
            .to_string()
            .contains("(mode 777)")
    );
    assert_eq!(
        words(Wait::TokenMissing { why: "none".into() }),
        ("dropbox_token_missing", "no Dropbox token: none".to_owned())
    );
    assert_eq!(Wait::TokenDead.code(), "dropbox_token_dead");
    assert!(
        Wait::TokenDead
            .to_string()
            .contains("nothing here mints or refreshes one")
    );
    let changed = Wait::LinkChanged {
        was: "aa".into(),
        now: "bb".into(),
    };
    assert_eq!(changed.code(), "link_changed");
    assert!(
        changed
            .to_string()
            .contains("add `accept_link_print = \"bb\"`")
    );
}

/// A config path that is not one readable file of at most
/// `MAX_CONFIG_BYTES` is `dropbox_link_malformed`, naming why, never read as
/// a default: a path under a file, a folder at the path, one byte past the
/// cap, and bytes that are not UTF-8. Exactly the cap is read (extremes: the
/// cap and one past it).
#[test]
fn a_config_that_is_not_one_small_readable_file_is_a_named_malformed_wait() {
    let home = closed_folder("dropbox-cfg-shapes");
    let folder = home.join(".brutex");
    std::fs::create_dir(&folder).unwrap();
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();
    // A path through a file: the status read fails, not as missing.
    let plain = folder.join("plain");
    write_mode(&plain, b"x", 0o600);
    let under = plain.join("gdfl-dropbox.toml");
    assert_eq!(
        Config::load(&under),
        Err(Wait::LinkMalformed {
            why: format!(
                "{}: {}",
                under.display(),
                std::io::Error::from_raw_os_error(20)
            )
        })
    );
    // A folder where the file belongs.
    let path = config_path(&home);
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let not_a_file = Wait::LinkMalformed {
        why: format!(
            "{} is not a file of at most {MAX_CONFIG_BYTES} bytes",
            path.display()
        ),
    };
    assert_eq!(Config::load(&path), Err(not_a_file.clone()));
    std::fs::remove_dir(&path).unwrap();
    // Exactly the cap, a valid config padded by a comment, is read.
    let line = format!("link = \"{LINK}\"\n#");
    let padded = format!(
        "{line}{}",
        "x".repeat(usize::try_from(MAX_CONFIG_BYTES).unwrap() - line.len())
    );
    assert_eq!(padded.len() as u64, MAX_CONFIG_BYTES);
    write_mode(&path, padded.as_bytes(), 0o600);
    assert_eq!(
        Config::load(&path).unwrap(),
        Config::parse(&format!("link = \"{LINK}\"\n")).unwrap()
    );
    // One byte past it is refused unread.
    write_mode(&path, format!("{padded}x").as_bytes(), 0o600);
    assert_eq!(Config::load(&path), Err(not_a_file));
    // Bytes that are not UTF-8.
    write_mode(&path, b"link = \"\xff\xfe\"\n", 0o600);
    let Err(Wait::LinkMalformed { why }) = Config::load(&path) else {
        panic!("not UTF-8 is malformed");
    };
    assert_eq!(
        why,
        format!("{}: stream did not contain valid UTF-8", path.display())
    );
    // Blank lines and lines of spaces are skipped, as comments are.
    assert_eq!(
        Config::parse(&format!("\n   \n\t\nlink = \"{LINK}\"\n\n")),
        Config::parse(&format!("link = \"{LINK}\"\n"))
    );
}

/// A path with no folder above it (the root) has no folder to refuse; the
/// file's own checks still apply to it (`check_folder`'s first `else`).
#[test]
fn a_path_with_no_folder_above_it_has_no_folder_to_refuse() {
    let meta = std::fs::metadata("/").unwrap();
    assert_eq!(check_folder(Path::new("/"), &meta), Ok(()));
}

/// OUT OF THE BOX: a config named by a bare file name has no folder that can
/// be read back (its folder is the empty path), so it is refused as malformed,
/// never read from whatever the working folder happens to be. Run in a child
/// of its own (D-0995's runner), because it moves the process's working
/// folder.
#[test]
fn a_config_named_without_a_folder_is_refused_not_read_from_the_working_folder() {
    crate::support::where_permission_binds(
        "gdfl_dropbox::cov_tests::a_config_named_without_a_folder_is_refused_not_read_from_the_working_folder",
        || {
            let here = closed_folder("dropbox-cfg-bare");
            std::env::set_current_dir(&here).unwrap();
            write_mode(
                Path::new("gdfl-dropbox.toml"),
                format!("link = \"{LINK}\"\n").as_bytes(),
                0o600,
            );
            assert_eq!(
                Config::load(Path::new("gdfl-dropbox.toml")),
                Err(Wait::LinkMalformed {
                    why: format!(": {}", std::io::Error::from_raw_os_error(2))
                })
            );
        },
    );
}

/// FM-242: a config its folder's owner does not own is refused even in a
/// folder only that owner may write. No test can give a file away without
/// root, so the config is a link, made in the test's own folder, to a file
/// the host already has that another user owns and only that user may read
/// (the first of a short list that is so; the test's own file and `/` lead
/// the list and each fail one half of that test). Run where the permission
/// bits bind (D-0995): there the test is not root, so root's file is another
/// owner's.
#[test]
fn a_config_its_folder_owner_does_not_own_is_a_named_wait() {
    crate::support::where_permission_binds(
        "gdfl_dropbox::cov_tests::a_config_its_folder_owner_does_not_own_is_a_named_wait",
        || {
            use std::os::unix::fs::MetadataExt as _;
            let folder = closed_folder("dropbox-cfg-foreign");
            let mine = folder.join("mine");
            write_mode(&mine, b"x", 0o600);
            let me = std::fs::metadata(&mine).unwrap().uid();
            let candidates = [
                mine.as_path(),
                Path::new("/"),
                Path::new("/proc/1/environ"),
                Path::new("/root"),
                Path::new("/etc/master.passwd"),
                Path::new("/private/var/db/sudo"),
            ];
            #[allow(
                clippy::verbose_bit_mask,
                reason = "the mask `Config::load` itself reads: no group or world bit"
            )]
            let foreign = candidates
                .iter()
                .find(|path| {
                    std::fs::metadata(path)
                        .is_ok_and(|meta| meta.uid() != me && meta.mode() & 0o077 == 0)
                })
                .expect("a file another user owns that only its owner may read");
            let link = folder.join("gdfl-dropbox.toml");
            std::os::unix::fs::symlink(foreign, &link).unwrap();
            let wait = Config::load(&link).unwrap_err();
            assert_eq!(
                wait,
                Wait::ConfigFolderOpen {
                    folder: folder.clone(),
                    mode: 0o755
                },
                "{}",
                foreign.display()
            );
            assert_eq!(wait.code(), "dropbox_config_too_open");
        },
    );
}

/// A link of exactly 2,048 bytes is kept and one of 2,049 is refused by the
/// length rule, whose words never quote it (extremes: the bound and past it).
#[test]
fn a_link_past_2048_bytes_is_refused_and_one_of_2048_is_kept() {
    let head = "https://www.dropbox.com/sh/";
    let at_bound = format!("{head}{}", "a".repeat(2_048 - head.len()));
    assert_eq!(at_bound.len(), 2_048);
    assert_eq!(check_link(&at_bound), Ok(()));
    let past = format!("{at_bound}a");
    let why = check_link(&past).unwrap_err();
    assert_eq!(
        why,
        "the link holds a space, a control or a non-ASCII character"
    );
    assert!(!why.contains("dropbox.com/sh"), "{why}");
    let refused = Config::parse(&format!("link = \"{past}\"\n")).unwrap_err();
    assert_eq!(refused, why);
}

/// The proxy is named by host only, from the first variable of the four that
/// is set, and never for a client that uses no proxy, whatever the
/// environment says. The real environment is read too: whatever it names
/// carries no scheme and no credentials (`proxy_in_use`).
#[test]
fn the_proxy_is_named_by_host_from_the_first_variable_set() {
    let env = |pairs: &'static [(&'static str, &'static str)]| {
        move |name: &str| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        }
    };
    let dropbox = Endpoints::dropbox();
    assert_eq!(
        dropbox.proxy_named_by(env(&[
            ("all_proxy", "http://d.example:4"),
            ("ALL_PROXY", "http://c.example:3"),
            ("https_proxy", "http://user:secret@b.example:2/"),
            ("HTTPS_PROXY", "http://a.example:1"),
        ])),
        Some("a.example:1".to_owned())
    );
    assert_eq!(
        dropbox.proxy_named_by(env(&[
            ("all_proxy", "http://d.example:4"),
            ("https_proxy", "http://user:secret@b.example:2/"),
        ])),
        Some("b.example:2".to_owned()),
        "the credentials are never echoed"
    );
    assert_eq!(
        dropbox.proxy_named_by(env(&[("all_proxy", "socks5://d.example:4")])),
        Some("d.example:4".to_owned())
    );
    // Set but empty is no proxy; nothing set is none.
    assert_eq!(dropbox.proxy_named_by(env(&[("HTTPS_PROXY", " ")])), None);
    assert_eq!(dropbox.proxy_named_by(env(&[])), None);
    // A client that uses no proxy names none, whatever is set.
    assert_eq!(
        Endpoints::loopback("http://127.0.0.1:9")
            .proxy_named_by(env(&[("HTTPS_PROXY", "http://a.example:1")])),
        None
    );
    // This process's own environment, whatever it holds: never a scheme,
    // never credentials, never an empty name.
    let real = dropbox.proxy_in_use();
    let named = real.clone().unwrap_or_default();
    assert!(!named.contains('@') && !named.contains("://"), "{real:?}");
    assert_ne!(real.as_deref(), Some(""));
    // The download client for Dropbox's own hosts is built with the
    // environment's proxy (it never calls `no_proxy`).
    assert!(client(&dropbox).is_ok());
}

/// A transport failure is named by its class and its host, and by the proxy
/// it went through when there is one, never by the link (FM-030): a timeout,
/// a refused connect, a refused redirect, a cut body, and a failure of none
/// of those classes (a status error here).
#[test]
fn a_transport_failure_names_its_class_its_host_and_its_proxy() {
    // A port nobody listens on.
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    // A server that takes the connection and never answers.
    let silent = TcpListener::bind("127.0.0.1:0").unwrap();
    let silent_port = silent.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let mut held = Vec::new();
        for stream in silent.incoming() {
            held.push(stream);
        }
    });
    let redirecting =
        raw_server(b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\n\r\n");
    let cut = raw_server(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n0123456789");
    let failing = raw_server(b"HTTP/1.1 500 Internal\r\nContent-Length: 0\r\n\r\n");
    let errors = block(async {
        let plain = plain_client();
        let connect = plain
            .get(format!("http://127.0.0.1:{closed}/x"))
            .send()
            .await
            .unwrap_err();
        let timeout = plain
            .get(format!("http://127.0.0.1:{silent_port}/x"))
            .timeout(Duration::from_millis(200))
            .send()
            .await
            .unwrap_err();
        let refusing = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                attempt.error("refused by the test's policy")
            }))
            .build()
            .unwrap();
        let redirect = refusing
            .get(format!("{redirecting}/x"))
            .send()
            .await
            .unwrap_err();
        let mut response = plain.get(format!("{cut}/x")).send().await.unwrap();
        let mut body = Vec::new();
        let decode = loop {
            match response.chunk().await {
                Ok(Some(chunk)) => body.extend_from_slice(&chunk),
                Ok(None) => panic!("a cut body ended cleanly"),
                Err(why) => break why,
            }
        };
        let status = plain
            .get(format!("{failing}/x"))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap_err();
        [connect, timeout, redirect, decode, status]
    });
    let [connect, timeout, redirect, decode, status] = &errors;
    // A cut body read chunk by chunk is a decode error, never a body error:
    // `reqwest::Response::chunk` wraps every body error so, and carries no
    // URL, so its words name an empty host.
    assert!(decode.is_decode() && !decode.is_body(), "{decode:?}");
    assert_eq!(decode.url(), None);
    let words: Vec<String> = errors
        .iter()
        .map(|why| transport_words(why, None))
        .collect();
    assert_eq!(
        words,
        [
            "could not connect (host 127.0.0.1)",
            "timed out (host 127.0.0.1)",
            "redirect (host 127.0.0.1)",
            "the body was cut (host )",
            "failed (host 127.0.0.1)",
        ],
        "{connect:?} {timeout:?} {redirect:?} {status:?}"
    );
    // Through a proxy, the proxy is named by host, beside the vendor's host.
    assert_eq!(
        transport_words(connect, Some("proxy.example:3128")),
        "could not connect (host 127.0.0.1, via_proxy=proxy.example:3128)"
    );
}

/// The first page's refusal reads at most 64 KiB of the error body: past
/// that the summary is not read at all (`summary=none`), even one that would
/// have said "not a folder" (extreme: a 70 KiB error body). A body of
/// exactly its cap is read whole; one byte past it is refused by its size.
#[test]
fn a_first_refusal_past_64_kib_names_no_summary_and_a_page_past_its_cap_is_refused() {
    let small = raw_server(
        b"HTTP/1.1 409 Conflict\r\nContent-Length: 36\r\n\r\n{\"error_summary\":\"path/not_folder/\"}",
    );
    let padded = format!(
        "{{\"error_summary\":\"path/not_folder/\",\"pad\":\"{}\"}}",
        "z".repeat(70 * 1_024)
    );
    let big: &'static [u8] = Box::leak(
        format!(
            "HTTP/1.1 409 Conflict\r\nContent-Length: {}\r\n\r\n{padded}",
            padded.len()
        )
        .into_bytes()
        .into_boxed_slice(),
    );
    let big = raw_server(big);
    let ten = raw_server(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n0123456789");
    let (file, cut, whole, past) = block(async {
        let client = plain_client();
        let answer = |base: String| {
            let client = client.clone();
            async move {
                let response = client.get(format!("{base}/x")).send().await.unwrap();
                Answer {
                    status: response.status().as_u16(),
                    response,
                }
            }
        };
        let file = refused_listing(answer(small).await).await;
        let cut = refused_listing(answer(big).await).await;
        let whole = body_capped(answer(ten.clone()).await.response, 10, None).await;
        let past = body_capped(answer(ten).await.response, 9, None).await;
        (file, cut, whole, past)
    });
    assert_eq!(
        file,
        RunRefusal::new(
            "link_is_a_file",
            "the shared link names a file, not a folder"
        )
    );
    assert_eq!(
        cut,
        RunRefusal::new("link_unreachable", "status=409 summary=none")
    );
    assert_eq!(whole, Ok(b"0123456789".to_vec()));
    assert_eq!(past, Err("a page past 9 bytes".to_owned()));
}

/// A `deleted` entry is skipped and an entry of a tag this build does not
/// know is `listing_shape_unknown` by its field, never skipped (`parse_page`).
#[test]
fn a_listing_page_skips_a_deleted_entry_and_refuses_an_unknown_tag() {
    let page = parse_page(
        br#"{"entries":[{".tag":"deleted","name":"GFDLNFO_TICK_01082019.zip"},{".tag":"file","name":"A.zip","size":3,"rev":"r1"}],"cursor":"c1","has_more":true}"#,
        "/f",
    )
    .ok()
    .unwrap();
    assert!(page.folders.is_empty());
    assert_eq!(
        page.files,
        [Listed {
            path: "/f/A.zip".into(),
            name: "A.zip".into(),
            size: 3,
            rev: "r1".into(),
            hash: String::new(),
        }]
    );
    assert_eq!((page.cursor.as_str(), page.has_more), ("c1", true));
    let unknown = parse_page(
        br#"{"entries":[{".tag":"symlink","name":"x"}],"cursor":"c","has_more":false}"#,
        "",
    )
    .err()
    .unwrap();
    assert_eq!(
        unknown,
        RunRefusal::new("listing_shape_unknown", "field=.tag")
    );
}

/// The refusal a walk's page ends in, or a panic naming what it was.
fn refusal_of(taken: Result<usize, Stop>) -> RunRefusal {
    match taken {
        Err(Stop::Refused(refusal)) => refusal,
        Err(Stop::Wait(wait)) => panic!("a wait: {wait:?}"),
        Ok(added) => panic!("taken: {added}"),
    }
}

fn page_of(files: Vec<Listed>, folders: Vec<String>) -> Page {
    Page {
        files,
        folders,
        cursor: String::new(),
        has_more: false,
    }
}

fn file_named(n: usize) -> Listed {
    Listed {
        path: format!("/{n}"),
        name: n.to_string(),
        size: 1,
        rev: String::new(),
        hash: String::new(),
    }
}

/// A folder a page lists twice is one folder and one repeat; a walk of
/// exactly `MAX_LISTED` files and `MAX_FOLDERS` folders is kept, and one
/// more of either is `listing_incomplete` by name (extremes: each cap and one
/// past it).
#[test]
fn a_walk_counts_a_folder_listed_twice_once_and_refuses_one_past_each_cap() {
    let mut walk = Walk::default();
    let added = walk
        .take(page_of(Vec::new(), vec!["/a".into(), "/a".into()]), 2)
        .ok()
        .unwrap();
    assert_eq!((added, walk.repeats), (1, 1));
    assert_eq!(walk.queue, [("/a".to_owned(), 3)]);
    let files: Vec<Listed> = (0..MAX_LISTED).map(file_named).collect();
    let mut full = Walk::default();
    assert_eq!(
        full.take(page_of(files, Vec::new()), 0).ok(),
        Some(MAX_LISTED)
    );
    assert_eq!(
        refusal_of(full.take(page_of(vec![file_named(MAX_LISTED)], Vec::new()), 0)),
        RunRefusal::new(
            "listing_incomplete",
            format!("more than {MAX_LISTED} files")
        )
    );
    let folders: Vec<String> = (0..MAX_FOLDERS).map(|n| format!("/d{n}")).collect();
    let mut wide = Walk::default();
    assert_eq!(
        wide.take(page_of(Vec::new(), folders), 0).ok(),
        Some(MAX_FOLDERS)
    );
    assert_eq!(
        refusal_of(wide.take(page_of(Vec::new(), vec!["/one-more".into()]), 0)),
        RunRefusal::new(
            "listing_incomplete",
            format!("more than {MAX_FOLDERS} folders")
        )
    );
}

/// A line whose first pair is not the version is never a journal line, and a
/// batch of no lines writes nothing, not even the journal file.
#[test]
fn a_line_not_led_by_its_version_is_refused_and_an_empty_batch_writes_nothing() {
    assert_eq!(parse_line("x=1 begin run=1"), None);
    assert_eq!(parse_line("v=1 begin run=1").unwrap().verb, "begin");
    let dir = scratch("dropbox-batch");
    let mut journal = Journal {
        path: dir.join("journal").join("gdfl-dropbox.journal"),
        run: "r".into(),
        lines: 0,
        error: None,
    };
    journal.batch(&[]);
    assert_eq!((journal.lines, journal.error.as_deref()), (0, None));
    assert!(!dir.join("journal").exists(), "nothing made");
}

fn fields(line: &str) -> Fields {
    parse_line(line).unwrap()
}

/// A journal line missing the field its verb is read by changes nothing it
/// would have changed: a listing with no link, a complete listing with no
/// count (it still answers the link's pending shrink), a part with no part,
/// a verified line with no path, a re-verify failure of another path, and a
/// held or handed line with no name (`Memory::read`).
#[test]
fn memory_reads_a_line_without_its_key_field_as_no_change() {
    let mut memory = Memory::default();
    memory.read(&fields("v=1 listing run=r complete=true count=3"));
    assert!(memory.complete.is_empty() && memory.shrinks.is_empty());
    memory.read(&fields(
        "v=1 listing run=r link=P complete=false reason=listing_shrank was=3 count=2 shrink=1-2-3",
    ));
    assert_eq!(
        memory.shrinks.get("P"),
        Some(&Shrink {
            was: 3,
            now: 2,
            token: "1-2-3".into()
        })
    );
    memory.read(&fields("v=1 listing run=r link=P complete=true"));
    assert!(memory.complete.is_empty(), "no count, no baseline");
    assert!(
        memory.shrinks.is_empty(),
        "a complete line answers the shrink"
    );
    memory.read(&fields("v=1 part run=r name=\"A.zip\" rev=\"1\""));
    assert!(memory.parts.is_empty());
    memory.read(&fields(
        "v=1 verified run=r name=\"A.zip\" rev=\"1\" hash=\"\" sha256=abc",
    ));
    assert!(
        memory.verified.is_empty(),
        "no path, nothing to offer again"
    );
    assert_eq!(memory.unhanded.get("A.zip"), Some(&vec!["abc".to_owned()]));
    memory.read(&fields(
        "v=1 verified run=r name=\"B.zip\" rev=\"1\" hash=\"\" path=\"/s/B.zip\" sha256=def",
    ));
    memory.read(&fields(
        "v=1 reverify_failed run=r name=\"B.zip\" rev=\"1\" path=\"/s/other.zip\"",
    ));
    assert_eq!(
        memory.verified.get(&("B.zip".to_owned(), "1".to_owned())),
        Some(&PathBuf::from("/s/B.zip")),
        "a failure of another copy keeps this one"
    );
    memory.read(&fields("v=1 held run=r reason=held_tick_store"));
    assert!(memory.held.is_empty());
    memory.read(&fields("v=1 handed run=r ok=true verdict=\"written\""));
    assert!(memory.hashes.is_empty() && memory.writer_refused.is_empty());
    assert_eq!(memory.unhanded.len(), 2, "no name hands nothing on");
}

/// A journal that is one empty line is not this module's: refused by its
/// number, never read as empty.
#[test]
fn a_journal_of_one_empty_line_is_refused_by_its_number() {
    let dir = scratch("dropbox-blank");
    let path = dir.join("gdfl-dropbox.journal");
    std::fs::write(&path, b"\n").unwrap();
    assert_eq!(
        Memory::load(&path).unwrap_err(),
        format!("{} line 1: not a journal line", path.display())
    );
}

/// An append to a path with no folder, or under a folder that is a file, is
/// refused with the path and why; nothing is made in its place.
#[test]
fn an_append_with_no_folder_or_under_a_file_is_refused_by_its_path() {
    assert_eq!(
        append_line(Path::new(""), "v=1 end run=1"),
        Err(format!(": {}", std::io::Error::from_raw_os_error(2)))
    );
    let dir = scratch("dropbox-append-file");
    let folder = dir.join("journal");
    std::fs::write(&folder, b"a file where the folder belongs").unwrap();
    let path = folder.join("gdfl-dropbox.journal");
    assert_eq!(
        append_line(&path, "v=1 end run=1"),
        Err(format!(
            "{}: {}",
            path.display(),
            std::io::Error::from_raw_os_error(17)
        ))
    );
    assert_eq!(
        std::fs::read(&folder).unwrap(),
        b"a file where the folder belongs"
    );
}

/// A relative path none of whose folders exists is kept as written: there is
/// nothing to resolve (`absolute`'s stop at the empty path). A lone dot is an
/// unsafe name like `..` and the empty name.
#[test]
fn a_relative_path_nothing_of_which_exists_is_kept_as_written() {
    let path = Path::new("brutex-no-such-relative-folder/inner");
    assert_eq!(absolute(path), path.to_path_buf());
    assert_eq!(unsafe_name("."), Some("an empty or dot name"));
}

fn listed_as(name: &str, size: u64) -> Listed {
    Listed {
        path: format!("/{name}"),
        name: name.into(),
        size,
        rev: String::new(),
        hash: String::new(),
    }
}

/// `verify` refuses a name that is no day zip, a zip that lists no member
/// (extreme: the smallest zip there is), and a stored member whose central
/// record states more bytes than its data holds, each by name and before a
/// CRC is read.
#[test]
fn verify_refuses_no_day_name_an_empty_zip_and_a_member_shorter_than_stated() {
    let dir = scratch("dropbox-verify");
    let empty = zip(&[]);
    let path = dir.join("GFDLNFO_TICK_01082019.zip");
    std::fs::write(&path, &empty).unwrap();
    let size = empty.len() as u64;
    assert_eq!(
        verify(&path, &listed_as("holiday.zip", size)).unwrap_err(),
        (
            "not_a_day_zip",
            "not a canonical GDFL day zip name".to_owned()
        )
    );
    assert_eq!(
        verify(&path, &listed_as("GFDLNFO_TICK_01082019.zip", size)).unwrap_err(),
        ("zip_holds_no_member", "the zip lists no member".to_owned())
    );
    let mut short = zip(&[("ZZ/INVENTED.TXT", b"invented filler", Method::Stored)]);
    let eocd = short.len() - 22;
    let cd = u32::from_le_bytes(short[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    // The central record's stated length, five bytes past the data.
    short[cd + 24..cd + 28].copy_from_slice(&20_u32.to_le_bytes());
    std::fs::write(&path, &short).unwrap();
    assert_eq!(
        verify(
            &path,
            &listed_as("GFDLNFO_TICK_01082019.zip", short.len() as u64)
        )
        .unwrap_err(),
        (
            "member_size_mismatch",
            "ZZ/INVENTED.TXT is not 20 bytes".to_owned()
        )
    );
}

/// A part cannot be made in a run folder that is gone, and the 65th part of
/// one file is refused rather than opened over the 64 that exist (extreme:
/// the part cap).
#[test]
fn a_part_is_refused_in_a_missing_run_folder_and_past_sixty_four() {
    let dir = scratch("dropbox-parts");
    let mut journal = Journal {
        path: dir.join("gdfl-dropbox.journal"),
        run: "r".into(),
        lines: 0,
        error: None,
    };
    let file = listed_as("GFDLNFO_TICK_01082019.zip", 10);
    let gone = dir.join("gone-run");
    let Err(Stop::Refused(missing)) = new_part(&gone, &file, &mut journal) else {
        panic!("a part in a missing folder");
    };
    assert_eq!(
        missing,
        RunRefusal::new(
            "staging_write_failed",
            format!(
                "{}: {}",
                gone.join("GFDLNFO_TICK_01082019.zip.part").display(),
                std::io::Error::from_raw_os_error(2)
            )
        )
    );
    let run = dir.join("run");
    std::fs::create_dir(&run).unwrap();
    std::fs::write(run.join("GFDLNFO_TICK_01082019.zip.part"), b"one").unwrap();
    for generation in 2..=64 {
        std::fs::write(
            run.join(format!("GFDLNFO_TICK_01082019.zip.part{generation}")),
            b"kept",
        )
        .unwrap();
    }
    let Err(Stop::Refused(full)) = new_part(&run, &file, &mut journal) else {
        panic!("a 65th part");
    };
    assert_eq!(
        full,
        RunRefusal::new(
            "staging_write_failed",
            "64 parts of GFDLNFO_TICK_01082019.zip exist"
        )
    );
    assert_eq!(
        std::fs::read(run.join("GFDLNFO_TICK_01082019.zip.part")).unwrap(),
        b"one",
        "no part is opened over"
    );
    assert_eq!(journal.lines, 0, "no part line for a part never made");
}

/// The tick-store writer (R4-08) as the hand-off takes a day zip it can file
/// and says so by its verdict (`TickStoreHandoff::hand`'s taken arm).
#[test]
fn the_tick_store_writer_takes_a_day_it_can_file() {
    let dir = scratch("dropbox-writer");
    let stem = "GFDLCM_INDICES_TICK_01082019";
    let name = format!("{stem}.zip");
    let csv = b"Ticker,Date,Time,LTP\r\nZZIDX,01/08/2019,09:15:00,1.00\r\nZZIDX,01/08/2019,09:15:01,1.01\r\n";
    let bytes = zip(&[
        (&format!("{stem}/"), b"", Method::Stored),
        (&format!("{stem}/ZZIDX.NSE_IDX.csv"), csv, Method::Stored),
    ]);
    let path = dir.join(&name);
    std::fs::write(&path, &bytes).unwrap();
    let tick = dir.join("ticks");
    std::fs::create_dir(&tick).unwrap();
    let identity = DayIdentity::of_zip_name(&name).unwrap();
    let verified = Verified {
        name: name.clone(),
        day: identity.day,
        path,
        bytes: bytes.len() as u64,
        entries: 2,
        sha256: String::new(),
    };
    let mut writer = TickStoreHandoff::new(tick.clone(), WriteOptions::new("ps3-db"));
    assert_eq!(
        writer.hand(&verified),
        Handed {
            ok: true,
            verdict: "written".into()
        }
    );
    assert!(identity.path_under(&tick).is_file());
}

/// The cause of a press refused `run_busy` under load (the coordinator's
/// report on `repair_r1::a_duplicate_refused_by_its_sha256_stays_refused_on_every_later_press`):
/// a process started while a press holds its staging lock (another test's
/// spawn, until the child's exec) holds a copy of the lock's description, and
/// a lock released only by closing stayed held through that copy. Here the
/// copy is made by hand, so the race is not needed: the copy outlives the
/// press's guard, and the next press still takes the lock at once, because
/// the guard releases it on the description itself. While the first holds
/// it, a second press is `run_busy` naming the holder. Failed before the
/// release by name: the second take was `run_busy` naming the ended run.
#[test]
fn a_staging_lock_is_released_while_a_copy_of_its_description_lives() {
    let store = scratch("dropbox-lock-copy");
    let staging = staging_root(&store);
    let first = lock_staging(&store, &staging, "1-1-1").ok().unwrap();
    let copy = first.0.try_clone().unwrap();
    let Err(Stop::Refused(busy)) = lock_staging(&store, &staging, "1-1-2") else {
        panic!("a second press beside the first");
    };
    assert_eq!(
        busy,
        RunRefusal::new(
            "run_busy",
            format!(
                "another press holds the staging lock: run=1-1-1 pid={}",
                std::process::id()
            )
        )
    );
    drop(first);
    let Ok(second) = lock_staging(&store, &staging, "1-1-2") else {
        panic!("the copy of an ended press's description kept the lock");
    };
    drop(copy);
    drop(second);
    assert_eq!(
        std::fs::read_to_string(staging.join(".lock")).unwrap(),
        format!(
            "run=1-1-1 pid={pid}\nrun=1-1-2 pid={pid}\n",
            pid = std::process::id()
        )
    );
}

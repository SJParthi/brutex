//! The GDFL Dropbox fetch inside the app: step 1 of the one-click chain
//! (E2E-01, R4-07, D-4986).
//!
//! # What one press does
//!
//! 1. Reads the operator's shared link from `~/.brutex/gdfl-dropbox.toml`
//!    ([`Config`]), a local untracked file read here and nowhere else.
//!    Missing, malformed, or readable by group or world: a named
//!    [`Wait`], journalled, never a default.
//! 2. Lists the shared folder to its end, page after page and folder after
//!    folder ([`list`]). A listing cut part way is `listing_incomplete` and no
//!    "up to date" claim is made; a listing smaller than the last complete
//!    one for the same link is `listing_shrank`; a listing holding no GDFL
//!    day zip is `link_holds_no_gdfl_day_zip`.
//! 3. Wants every canonical GDFL day zip ([`DayIdentity::of_zip_name`]) the
//!    store does not hold: not in the tick store's day table
//!    ([`DayIdentity::path_under`]) and not closed `done` for the whole scope
//!    in the import journal ([`crate::gdfl_import::done_whole_days`]). Every
//!    other name is refused by name before any byte moves: a conflict copy or
//!    a renamed file (`not_a_day_zip`), a separator or `..` (`name_unsafe`),
//!    two names one volume cannot tell apart (`names_collide`), one day in
//!    two folders (`two_files_for_one_day`), BSE or MCX
//!    (`out_of_scope_exchange`, `CLAUDE.md` §1), and content another file
//!    already holds (`content_duplicate`).
//! 4. Checks the free-space floor BEFORE each file
//!    ([`crate::disk_projection::floor_check`]); a file past it is refused
//!    `disk_floor` and the run stops before that file's first byte.
//! 5. Downloads each wanted file into a new staging folder per run,
//!    `<store>/staging/gdfl-dropbox/<run>/`, as `<name>.part` opened with
//!    `create_new`. A cut download resumes with an HTTP `Range` request, in
//!    the same run and on the next press (the part is found from the journal
//!    and its length on disk). A resume appends only on `206` whose
//!    `Content-Range` starts at the part's length and states the listed size;
//!    a `200`, a `416`, a wrong range or a changed revision starts a NEW part
//!    beside the old one (`remote_changed`), so two versions are never
//!    stitched.
//! 6. Verifies each finished part: its size equals the listing's, its zip
//!    central directory is read whole ([`crate::gdfl_archive::zip_entries`]),
//!    every member's stated length is under [`MEMBER_BYTE_CAP`] before it is
//!    inflated, and every member's CRC-32 matches
//!    ([`crate::gdfl_archive::member_bytes`], [`crate::gdfl_archive::crc32`]).
//!    Only then is the part renamed to `<name>` in its run folder, and only
//!    where that name does not exist.
//! 7. Hands every verified file to the tick-store writer ([`Handoff`],
//!    [`TickStoreHandoff`]: [`crate::tickstore_write::write_day`], R4-08).
//!
//! # Never overwrite, never delete
//!
//! No path this module writes is ever truncated, replaced or removed: a run
//! folder is created new, a part is created new or appended, a verified file
//! is a rename onto a name that does not exist, and the journal is appended.
//! A refused part stays where it is, named in the journal.
//!
//! # The journal
//!
//! `<store>/journal/gdfl-dropbox.journal`, append-only, one line per state
//! change: `begin`, `wait`, `listing`, `listed`, `held`, `part`,
//! `downloading`, `cut`, `downloaded`, `verified`, `refused <reason>`,
//! `handed` and `end`. The shared link never reaches it: a run names the
//! link by its print, the first 16 bytes of its SHA-256 in hex
//! ([`Config::print`]). [`status`] reads its last line: O(1), one tail
//! read. Every file line is also a `pull.gdfl.dropbox` event.
//!
//! # The network
//!
//! Dropbox API v2's documented routes, through the workspace's own client
//! (`reqwest`): `files/list_folder` with `shared_link` (non-recursive only,
//! so each folder is listed in turn) and `files/list_folder/continue` with
//! its cursor until `has_more` is false; `sharing/get_shared_link_file` with
//! `Dropbox-API-Arg`. Source: the Dropbox API spec, `files.stone` and
//! `shared_links.stone` in github.com/dropbox/dropbox-api-spec, read
//! 16 Oct 2026 (`docs/00-charter.md`). **UNVERIFIED**: whether a shared
//! folder lists without a token (the routes name the `files.metadata.read`
//! and `sharing.read` scopes, which are token scopes), whether the content
//! route honours `Range`, and the exact error body of a file link. So a
//! token is read from Parameter Store, `CLAUDE.md` §8's way
//! ([`ParameterStoreToken`], vendor `gdfl`, field [`TOKEN_FIELD`]), re-read
//! once when Dropbox rejects it, and never minted or refreshed here; and
//! this module is proved against a loopback fixture server only.
//!
//! The download client is its own: connect and idle-read timeouts and no
//! total timeout (a day zip is hundreds of megabytes), no automatic
//! decompression, and redirects followed only to the allowlisted hosts
//! ([`Endpoints`]); any other `Location` is `redirect_refused`. A `429` and a
//! `5xx` back off by name, honouring `Retry-After` up to a cap, at most
//! [`Backoff::max_attempts`] times, then refuse the run `throttled` or
//! `server_error`. Sizes and offsets are `u64` throughout, so a zip past
//! 4 GiB is one more file.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read as _, Seek as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use brutex_core::vendor::Vendor;

use crate::config::CredentialConfig;
use crate::disk_projection::{Floor, FreeSpace, floor_check};
use crate::gdfl_archive::{crc32, member_bytes, zip_entries};
use crate::secret::{CredentialHalt, CredentialReader, Secret, SecretSource};
use crate::tickstore_write::{DayIdentity, Tree, WriteOptions, write_day};

/// The event target every file line is emitted under.
pub const TARGET: &str = "pull.gdfl.dropbox";

/// The config file under the operator's home.
pub const CONFIG_REL: &str = ".brutex/gdfl-dropbox.toml";

/// The largest config file read.
pub const MAX_CONFIG_BYTES: u64 = 4_096;

/// The Parameter Store field holding the Dropbox token, under vendor `gdfl`.
pub const TOKEN_FIELD: &str = "dropbox_token";

/// The largest member a day zip may state, checked before it is inflated:
/// the largest measured GDFL file is 6.1 MB (FM-042), so 64 MiB is ten times
/// it.
pub const MEMBER_BYTE_CAP: u64 = 64 << 20;

/// The largest file the listing may state: 1 TiB. A day zip past 4 GiB is
/// fetched like any other; this bounds only a listing that is not GDFL's.
pub const MAX_FILE_BYTES: u64 = 1 << 40;

/// The most entries one listing reads before it stops and says so.
pub const MAX_LISTED: usize = 200_000;

/// The most folders one listing descends into.
pub const MAX_FOLDERS: usize = 4_096;

/// The deepest folder one listing descends into.
pub const MAX_DEPTH: usize = 8;

/// The largest listing page read.
pub const MAX_PAGE_BYTES: usize = 32 << 20;

/// How many cut downloads of one file a run resumes before it moves on.
pub const MAX_RESUMES: u32 = 4;

/// The config file's path under `home`.
#[must_use]
pub fn config_path(home: &Path) -> PathBuf {
    home.join(CONFIG_REL)
}

/// The journal's path under the store root.
#[must_use]
pub fn journal_path(store_root: &Path) -> PathBuf {
    store_root.join("journal").join("gdfl-dropbox.journal")
}

/// The staging root under the store root.
#[must_use]
pub fn staging_root(store_root: &Path) -> PathBuf {
    store_root.join("staging").join("gdfl-dropbox")
}

// -- the config ---------------------------------------------------------------

/// Why a press waits: something only the operator can supply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wait {
    /// No config file.
    LinkMissing {
        /// Where it was looked for.
        path: PathBuf,
    },
    /// The config file is not one this build reads.
    LinkMalformed {
        /// Why.
        why: String,
    },
    /// The config file is readable by group or world.
    ConfigTooOpen {
        /// Its permission bits.
        mode: u32,
    },
    /// The folder holding the config is writable by group or world, or the
    /// file is not owned by the folder's owner: someone else could put a
    /// link there (FM-242).
    ConfigFolderOpen {
        /// The folder.
        folder: PathBuf,
        /// Its permission bits.
        mode: u32,
    },
    /// The shared link differs from the one the journal last used, and the
    /// operator has not recorded it.
    LinkChanged {
        /// The print the journal last used.
        was: String,
        /// The print of the link now configured.
        now: String,
    },
    /// No token could be read.
    TokenMissing {
        /// Why, without the path's account segments.
        why: String,
    },
    /// Dropbox rejected the token and the re-read returned the same value.
    TokenDead,
}

impl Wait {
    /// The wait's code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::LinkMissing { .. } => "dropbox_link_missing",
            Self::LinkMalformed { .. } => "dropbox_link_malformed",
            Self::ConfigTooOpen { .. } | Self::ConfigFolderOpen { .. } => "dropbox_config_too_open",
            Self::LinkChanged { .. } => "link_changed",
            Self::TokenMissing { .. } => "dropbox_token_missing",
            Self::TokenDead => "dropbox_token_dead",
        }
    }
}

impl core::fmt::Display for Wait {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LinkMissing { path } => write!(
                f,
                "waiting for the shared link: put `link = \"<the GDFL shared folder link>\"` in {} \
                 (mode 600)",
                path.display()
            ),
            Self::LinkMalformed { why } => write!(f, "the Dropbox config is not readable: {why}"),
            Self::ConfigTooOpen { mode } => write!(
                f,
                "the Dropbox config is readable by group or world (mode {mode:o}); chmod 600 it"
            ),
            Self::ConfigFolderOpen { folder, mode } => write!(
                f,
                "{} is writable by group or world (mode {mode:o}) or does not own the Dropbox \
                 config; chmod go-w it and keep the config the folder owner's",
                folder.display()
            ),
            Self::LinkChanged { was, now } => write!(
                f,
                "the shared link changed (was {was}, now {now}); if the new folder is right, add \
                 `accept_link_print = \"{now}\"` to the config"
            ),
            Self::TokenMissing { why } => write!(f, "no Dropbox token: {why}"),
            Self::TokenDead => f.write_str(
                "Dropbox rejected the token and the re-read returned the same value; it is dead \
                 and nothing here mints or refreshes one",
            ),
        }
    }
}

/// The validated config: the shared link and an optional accepted print.
#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    link: String,
    accept_link_print: Option<String>,
}

impl core::fmt::Debug for Config {
    /// The link's print only: the link itself is never printed.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Config(link print {})", self.print())
    }
}

impl Config {
    /// Reads and checks the config at `path`.
    ///
    /// # Errors
    ///
    /// [`Wait::LinkMissing`], [`Wait::ConfigTooOpen`] or
    /// [`Wait::LinkMalformed`].
    pub fn load(path: &Path) -> Result<Self, Wait> {
        use std::os::unix::fs::PermissionsExt as _;
        let meta = match std::fs::metadata(path) {
            Ok(meta) => meta,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
                return Err(Wait::LinkMissing {
                    path: path.to_owned(),
                });
            }
            Err(why) => {
                return Err(Wait::LinkMalformed {
                    why: format!("{}: {why}", path.display()),
                });
            }
        };
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(Wait::ConfigTooOpen { mode });
        }
        check_folder(path, &meta)?;
        if !meta.is_file() || meta.len() > MAX_CONFIG_BYTES {
            return Err(Wait::LinkMalformed {
                why: format!(
                    "{} is not a file of at most {MAX_CONFIG_BYTES} bytes",
                    path.display()
                ),
            });
        }
        let text = std::fs::read_to_string(path).map_err(|why| Wait::LinkMalformed {
            why: format!("{}: {why}", path.display()),
        })?;
        Self::parse(&text).map_err(|why| Wait::LinkMalformed { why })
    }

    /// Parses `key = "value"` lines; `#` starts a comment. Keys: `link`
    /// (required) and `accept_link_print`.
    ///
    /// # Errors
    ///
    /// The sentence naming the line or value refused. It never quotes the
    /// link.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut link = None;
        let mut accept = None;
        for (number, raw) in text.lines().enumerate() {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let at = number + 1;
            let (key, value) = trimmed
                .split_once('=')
                .ok_or_else(|| format!("line {at} is not `key = \"value\"`"))?;
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .filter(|v| !v.contains('"') && !v.contains('\\'))
                .ok_or_else(|| format!("line {at}: the value is not one quoted string"))?;
            let slot = match key.trim() {
                "link" => &mut link,
                "accept_link_print" => &mut accept,
                other => return Err(format!("line {at}: unknown key {other:?}")),
            };
            if slot.replace(value.to_owned()).is_some() {
                return Err(format!("line {at}: {} is set twice", key.trim()));
            }
        }
        let link = link.ok_or_else(|| "no `link` line".to_owned())?;
        check_link(&link)?;
        Ok(Self {
            link,
            accept_link_print: accept,
        })
    }

    /// The link's print: the first 16 bytes of its SHA-256, in hex. The only
    /// form of the link a journal, log or page ever carries.
    #[must_use]
    pub fn print(&self) -> String {
        print_of(&self.link)
    }
}

/// The config's folder may not be writable by group or world, and the file
/// must be owned by the folder's owner (FM-242). A folder that is only
/// readable (the usual `0755` home folder) is kept: the file's own `600`
/// keeps the link unreadable, and a readable folder shows only names.
fn check_folder(path: &Path, meta: &std::fs::Metadata) -> Result<(), Wait> {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
    let Some(folder) = path.parent() else {
        return Ok(());
    };
    let folder_meta = std::fs::metadata(folder).map_err(|why| Wait::LinkMalformed {
        why: format!("{}: {why}", folder.display()),
    })?;
    let mode = folder_meta.permissions().mode() & 0o777;
    if mode & 0o022 != 0 || folder_meta.uid() != meta.uid() {
        return Err(Wait::ConfigFolderOpen {
            folder: folder.to_owned(),
            mode,
        });
    }
    Ok(())
}

/// The print of `text`.
fn print_of(text: &str) -> String {
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(text.as_bytes());
    digest
        .iter()
        .take(16)
        .fold(String::with_capacity(32), |mut out, byte| {
            use std::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// A shared link this build sends: `https://` on a Dropbox host, printable
/// ASCII, no space.
fn check_link(link: &str) -> Result<(), String> {
    let rest = link
        .strip_prefix("https://")
        .ok_or_else(|| "the link is not an https:// link".to_owned())?;
    let host = rest.split(['/', '?']).next().unwrap_or_default();
    let dropbox = host == "dropbox.com" || host.ends_with(".dropbox.com");
    if !dropbox {
        return Err("the link's host is not dropbox.com".to_owned());
    }
    if link.len() > 2_048 || !link.bytes().all(|b| b.is_ascii_graphic()) {
        return Err("the link holds a space, a control or a non-ASCII character".to_owned());
    }
    Ok(())
}

// -- the token ----------------------------------------------------------------

/// Where the Dropbox token comes from.
pub trait TokenSource: Send + Sync {
    /// The token.
    ///
    /// # Errors
    ///
    /// [`Wait::TokenMissing`].
    fn read(&self) -> Result<Secret, Wait>;

    /// The token again after Dropbox rejected `rejected`.
    ///
    /// # Errors
    ///
    /// [`Wait::TokenDead`] when the re-read returns the same value, or
    /// [`Wait::TokenMissing`].
    fn reread(&self, rejected: &Secret) -> Result<Secret, Wait>;
}

/// The token read from Parameter Store at the path the credential config
/// names for vendor `gdfl`, field [`TOKEN_FIELD`] (`CLAUDE.md` §8).
pub struct ParameterStoreToken<S> {
    config: CredentialConfig,
    reader: CredentialReader<S>,
}

impl<S> ParameterStoreToken<S> {
    /// A token source over `config` and `reader`.
    #[must_use]
    pub const fn new(config: CredentialConfig, reader: CredentialReader<S>) -> Self {
        Self { config, reader }
    }
}

fn halt_wait(halt: &CredentialHalt) -> Wait {
    match halt {
        CredentialHalt::DeadToken { .. } => Wait::TokenDead,
        other => Wait::TokenMissing {
            why: other.to_string(),
        },
    }
}

impl<S: SecretSource + Send + Sync> TokenSource for ParameterStoreToken<S> {
    fn read(&self) -> Result<Secret, Wait> {
        let path = self
            .config
            .path_for(Vendor::Gdfl, TOKEN_FIELD)
            .map_err(|why| Wait::TokenMissing {
                why: format!("the credential config names no {TOKEN_FIELD} for gdfl: {why}"),
            })?;
        self.reader
            .read(Vendor::Gdfl, &path)
            .map_err(|halt| halt_wait(&halt))
    }

    fn reread(&self, rejected: &Secret) -> Result<Secret, Wait> {
        let path = self
            .config
            .path_for(Vendor::Gdfl, TOKEN_FIELD)
            .map_err(|why| Wait::TokenMissing {
                why: why.to_string(),
            })?;
        self.reader
            .reread_after_rejection(Vendor::Gdfl, &path, rejected)
            .map_err(|halt| halt_wait(&halt))
    }
}

// -- the endpoints and the client ---------------------------------------------

/// Where the API and content routes live, and where a redirect may go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    api: String,
    content: String,
    allow: Vec<String>,
    proxy: bool,
}

impl Endpoints {
    /// Dropbox's own hosts (the API spec's `api` and `content` hosts).
    #[must_use]
    pub fn dropbox() -> Self {
        Self {
            api: "https://api.dropboxapi.com".to_owned(),
            content: "https://content.dropboxapi.com".to_owned(),
            allow: vec![
                "api.dropboxapi.com".to_owned(),
                "content.dropboxapi.com".to_owned(),
                ".dropboxusercontent.com".to_owned(),
            ],
            proxy: true,
        }
    }

    /// A fixture server on loopback: both routes on `base`, no proxy.
    #[cfg(test)]
    pub(crate) fn loopback(base: &str) -> Self {
        Self {
            api: base.to_owned(),
            content: base.to_owned(),
            allow: vec!["127.0.0.1".to_owned()],
            proxy: false,
        }
    }

    /// The proxy the environment names for this client, by host only, or
    /// `None` when there is none or the client uses none.
    fn proxy_in_use(&self) -> Option<String> {
        if !self.proxy {
            return None;
        }
        ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
            .iter()
            .find_map(|name| std::env::var(name).ok())
            .and_then(|value| proxy_host_of(&value))
    }

    fn allows(&self, host: &str) -> bool {
        self.allow.iter().any(|allowed| {
            allowed.strip_prefix('.').map_or(host == allowed, |suffix| {
                host.strip_suffix(suffix).is_some_and(|h| h.ends_with('.'))
            })
        })
    }
}

/// How the run waits on a `429` or a `5xx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    /// The most attempts of one request.
    pub max_attempts: u32,
    /// The first wait without a `Retry-After`; doubled each attempt.
    pub base: Duration,
    /// The longest single wait.
    pub cap: Duration,
    /// What one `Retry-After` second is (one second, except in a test).
    pub second: Duration,
}

impl Backoff {
    /// Five attempts, 2 s doubling, at most 5 minutes a wait.
    pub const DEFAULT: Self = Self {
        max_attempts: 5,
        base: Duration::from_secs(2),
        cap: Duration::from_mins(5),
        second: Duration::from_secs(1),
    };

    fn wait(&self, attempt: u32, retry_after: Option<u64>) -> Duration {
        let wanted = retry_after.map_or_else(
            || self.base.saturating_mul(1_u32 << attempt.min(16)),
            |secs| {
                self.second
                    .saturating_mul(u32::try_from(secs).unwrap_or(u32::MAX))
            },
        );
        wanted.min(self.cap)
    }
}

fn client(endpoints: &Endpoints) -> Result<reqwest::Client, String> {
    crate::ensure_tls_provider();
    let allowed = endpoints.clone();
    let policy = reqwest::redirect::Policy::custom(move |attempt| {
        let host = attempt.url().host_str().unwrap_or_default().to_owned();
        if attempt.previous().len() < 3 && allowed.allows(&host) {
            attempt.follow()
        } else {
            attempt.stop()
        }
    });
    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .read_timeout(Duration::from_mins(2))
        .no_gzip()
        .redirect(policy);
    if !endpoints.proxy {
        builder = builder.no_proxy();
    }
    builder
        .build()
        .map_err(|why| format!("the download client could not be built: {why}"))
}

// -- the listing --------------------------------------------------------------

/// One file the shared folder lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// Its path from the link's root: `/2019/GFDLNFO_TICK_01072019.zip`.
    pub path: String,
    /// Its name.
    pub name: String,
    /// Its size in bytes, as listed.
    pub size: u64,
    /// Its revision, as listed (empty when none is).
    pub rev: String,
    /// Its content hash, as listed (empty when none is).
    pub hash: String,
}

/// Why a run stopped without a complete answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRefusal {
    /// The code.
    pub code: &'static str,
    /// The words.
    pub detail: String,
}

impl RunRefusal {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// What a listing ended in.
enum Stop {
    Wait(Wait),
    Refused(RunRefusal),
}

impl From<Wait> for Stop {
    fn from(wait: Wait) -> Self {
        Self::Wait(wait)
    }
}

impl From<RunRefusal> for Stop {
    fn from(refusal: RunRefusal) -> Self {
        Self::Refused(refusal)
    }
}

/// One request's context: the client, the token, the backoff and its total.
struct Wire<'a> {
    client: reqwest::Client,
    token: Secret,
    source: &'a dyn TokenSource,
    backoff: Backoff,
    waited: Duration,
    /// The proxy every request goes through, by host only, or `None`.
    via: Option<String>,
}

/// A response with its status and the headers the fetch reads.
struct Answer {
    response: reqwest::Response,
    status: u16,
}

impl Wire<'_> {
    /// Sends what `build` makes, backing off on `429` and `5xx` and
    /// re-reading the token once on `401`.
    async fn send(
        &mut self,
        what: &str,
        build: &(dyn Fn(&reqwest::Client, &str) -> reqwest::RequestBuilder + Sync),
    ) -> Result<Answer, Stop> {
        let mut attempt = 0_u32;
        let mut reread = false;
        loop {
            let request = build(&self.client, self.token.expose());
            let response = request.send().await.map_err(|why| {
                RunRefusal::new(
                    "transport",
                    format!("{what}: {}", transport_words(&why, self.via.as_deref())),
                )
            })?;
            let status = response.status().as_u16();
            if status == 401 && !reread {
                reread = true;
                self.token = self.source.reread(&self.token)?;
                continue;
            }
            if status == 401 {
                return Err(Wait::TokenDead.into());
            }
            if status == 429 || (500..600).contains(&status) {
                attempt += 1;
                let retry_after = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.trim().parse::<u64>().ok());
                if attempt >= self.backoff.max_attempts {
                    let code = if status == 429 {
                        "throttled"
                    } else {
                        "server_error"
                    };
                    return Err(RunRefusal::new(
                        code,
                        format!(
                            "{what}: answered {status} {attempt} times (retry_after={})",
                            retry_after.map_or_else(|| "none".to_owned(), |s| s.to_string())
                        ),
                    )
                    .into());
                }
                let pause = self.backoff.wait(attempt, retry_after);
                self.waited = self.waited.saturating_add(pause);
                tokio::time::sleep(pause).await;
                continue;
            }
            return Ok(Answer { response, status });
        }
    }
}

/// A transport failure's words, naming a timeout, a connect failure or a
/// redirect stop, never the link.
fn transport_words(why: &reqwest::Error, via: Option<&str>) -> String {
    let class = if why.is_timeout() {
        "timed out"
    } else if why.is_connect() {
        "could not connect"
    } else if why.is_redirect() {
        "redirect"
    } else if why.is_body() || why.is_decode() {
        "the body was cut"
    } else {
        "failed"
    };
    let host = why
        .url()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_default();
    // A failure through a proxy names it (FM-030): a refused CONNECT is the
    // proxy's answer, not the vendor's, and not a dead credential.
    match via {
        Some(proxy) => format!("{class} (host {host}, via_proxy={proxy})"),
        None => format!("{class} (host {host})"),
    }
}

/// The host and port of a proxy URL, without its scheme, path or any
/// `user:password@` (never echoed). `None` for an empty value.
fn proxy_host_of(value: &str) -> Option<String> {
    let rest = value.trim();
    let rest = rest.split_once("://").map_or(rest, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    (!host.is_empty()).then(|| host.to_owned())
}

/// JSON with every non-ASCII character escaped, as an HTTP header holds it.
fn ascii_json(value: &serde_json::Value) -> String {
    let text = value.to_string();
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut units = [0_u16; 2];
            for unit in c.encode_utf16(&mut units) {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{unit:04x}");
            }
        }
    }
    out
}

/// Reads a whole body of at most `cap` bytes.
async fn body_capped(
    response: reqwest::Response,
    cap: usize,
    via: Option<&str>,
) -> Result<Vec<u8>, String> {
    let mut response = response;
    let mut out = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|why| transport_words(&why, via))?
    {
        if out.len() + chunk.len() > cap {
            return Err(format!("a page past {cap} bytes"));
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

/// One listing page, parsed strictly.
struct Page {
    files: Vec<Listed>,
    folders: Vec<String>,
    cursor: String,
    has_more: bool,
}

fn shape(field: &str) -> RunRefusal {
    RunRefusal::new("listing_shape_unknown", format!("field={field}"))
}

/// A page of `list_folder` under the folder `folder`.
fn parse_page(body: &[u8], folder: &str) -> Result<Page, RunRefusal> {
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| RunRefusal::new("link_unreachable", "status=200 the listing is not JSON"))?;
    let entries = value
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| shape("entries"))?;
    let cursor = value
        .get("cursor")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| shape("cursor"))?
        .to_owned();
    let has_more = value
        .get("has_more")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| shape("has_more"))?;
    let mut page = Page {
        files: Vec::new(),
        folders: Vec::new(),
        cursor,
        has_more,
    };
    for entry in entries {
        let text = |field: &str| entry.get(field).and_then(serde_json::Value::as_str);
        let tag = text(".tag").ok_or_else(|| shape(".tag"))?;
        let name = text("name").ok_or_else(|| shape("name"))?;
        let path = format!("{folder}/{name}");
        match tag {
            "file" => page.files.push(Listed {
                path,
                name: name.to_owned(),
                size: entry
                    .get("size")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or_else(|| shape("size"))?,
                rev: text("rev").unwrap_or_default().to_owned(),
                hash: text("content_hash").unwrap_or_default().to_owned(),
            }),
            "folder" => page.folders.push(path),
            "deleted" => {}
            _ => return Err(shape(".tag")),
        }
    }
    Ok(page)
}

/// The refusal a first page's non-success status names.
async fn refused_listing(answer: Answer) -> RunRefusal {
    let status = answer.status;
    let summary = body_capped(answer.response, 64 * 1_024, None)
        .await
        .ok()
        .and_then(|body| serde_json::from_slice::<serde_json::Value>(&body).ok())
        .and_then(|v| {
            v.get("error_summary")
                .and_then(|s| s.as_str().map(str::to_owned))
        })
        .unwrap_or_default();
    if status == 409 && summary.starts_with("path/not_folder") {
        return RunRefusal::new(
            "link_is_a_file",
            "the shared link names a file, not a folder",
        );
    }
    RunRefusal::new(
        "link_unreachable",
        format!(
            "status={status} summary={}",
            if summary.is_empty() { "none" } else { &summary }
        ),
    )
}

/// Every file the shared folder lists, every page of every folder.
async fn list(
    wire: &mut Wire<'_>,
    endpoints: &Endpoints,
    link: &str,
) -> Result<(Vec<Listed>, usize), Stop> {
    let mut files = Vec::new();
    let mut queue = vec![(String::new(), 0_usize)];
    let mut folders = 0_usize;
    let mut pages = 0_usize;
    while let Some((folder, depth)) = queue.pop() {
        folders += 1;
        if folders > MAX_FOLDERS || depth > MAX_DEPTH {
            return Err(RunRefusal::new(
                "listing_incomplete",
                format!("more than {MAX_FOLDERS} folders or deeper than {MAX_DEPTH}"),
            )
            .into());
        }
        let mut cursor: Option<String> = None;
        loop {
            let first = cursor.is_none();
            let (url, body) = match &cursor {
                None => (
                    format!("{}/2/files/list_folder", endpoints.api),
                    serde_json::json!({"path": folder, "shared_link": {"url": link}, "recursive": false}),
                ),
                Some(c) => (
                    format!("{}/2/files/list_folder/continue", endpoints.api),
                    serde_json::json!({"cursor": c}),
                ),
            };
            let text = body.to_string();
            let answer = wire
                .send("listing", &|client, token| {
                    client
                        .post(&url)
                        .bearer_auth(token)
                        .header(reqwest::header::CONTENT_TYPE, "application/json")
                        .body(text.clone())
                })
                .await
                .map_err(|stop| incomplete_unless_first(stop, first && folder.is_empty(), pages))?;
            if !(200..300).contains(&answer.status) {
                if first && folder.is_empty() {
                    return Err(refused_listing(answer).await.into());
                }
                return Err(RunRefusal::new(
                    "listing_incomplete",
                    format!("pages={pages} a later page answered {}", answer.status),
                )
                .into());
            }
            let bytes = body_capped(answer.response, MAX_PAGE_BYTES, wire.via.as_deref())
                .await
                .map_err(|why| {
                    RunRefusal::new("listing_incomplete", format!("pages={pages} {why}"))
                })?;
            let page = parse_page(&bytes, &folder)?;
            pages += 1;
            files.extend(page.files);
            queue.extend(page.folders.into_iter().map(|f| (f, depth + 1)));
            if files.len() > MAX_LISTED {
                return Err(RunRefusal::new(
                    "listing_incomplete",
                    format!("more than {MAX_LISTED} files"),
                )
                .into());
            }
            if !page.has_more {
                break;
            }
            cursor = Some(page.cursor);
        }
    }
    Ok((files, pages))
}

/// A transport failure after the first page is `listing_incomplete`.
fn incomplete_unless_first(stop: Stop, first: bool, pages: usize) -> Stop {
    match stop {
        Stop::Refused(refusal) if !first && refusal.code == "transport" => {
            Stop::Refused(RunRefusal::new(
                "listing_incomplete",
                format!("pages={pages} {}", refusal.detail),
            ))
        }
        Stop::Refused(refusal) if first && refusal.code == "transport" => {
            Stop::Refused(RunRefusal::new("link_unreachable", refusal.detail))
        }
        other => other,
    }
}

// -- the journal --------------------------------------------------------------

/// `text` quoted for a journal line: `"`, with `\` and `"` escaped and any
/// control character written as `\u{..}`.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{{{:x}}}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// One journal line's fields: its verb and its `key=value` pairs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fields {
    /// The verb.
    pub verb: String,
    /// The pairs.
    pub pairs: BTreeMap<String, String>,
}

impl Fields {
    /// The value of `key`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs.get(key).map(String::as_str)
    }
}

/// Parses one journal line: `v=1 <verb> key=value ...`, values bare or
/// quoted. `None` for a line this module never writes.
#[must_use]
pub fn parse_line(line: &str) -> Option<Fields> {
    let mut fields = Fields::default();
    let mut chars = line.chars().peekable();
    let mut first = true;
    loop {
        while chars.peek() == Some(&' ') {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c == ' ' {
                break;
            }
            key.push(c);
            chars.next();
        }
        if chars.peek() != Some(&'=') {
            if fields.verb.is_empty() && !first {
                fields.verb = key;
                continue;
            }
            return None;
        }
        chars.next();
        let mut value = String::new();
        if chars.peek() == Some(&'"') {
            chars.next();
            loop {
                match chars.next()? {
                    '"' => break,
                    '\\' => match chars.next()? {
                        'u' => {
                            if chars.next()? != '{' {
                                return None;
                            }
                            let mut hex = String::new();
                            loop {
                                match chars.next()? {
                                    '}' => break,
                                    h => hex.push(h),
                                }
                            }
                            value.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                        }
                        other => value.push(other),
                    },
                    c => value.push(c),
                }
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c == ' ' {
                    break;
                }
                value.push(c);
                chars.next();
            }
        }
        if first {
            if key != "v" || value != "1" {
                return None;
            }
            first = false;
            continue;
        }
        fields.pairs.insert(key, value);
    }
    (!fields.verb.is_empty()).then_some(fields)
}

/// The append-only journal of one run.
struct Journal {
    path: PathBuf,
    run: String,
    lines: u64,
    error: Option<String>,
}

impl Journal {
    /// Appends `verb` with `pairs` as one line, made durable; a failure is
    /// kept and named in the run's report, never dropped.
    fn line(&mut self, verb: &str, pairs: &[(&str, String)]) {
        let mut text = format!(
            "v=1 {verb} run={} at_ms={}",
            self.run,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis())
        );
        for (key, value) in pairs {
            text.push(' ');
            text.push_str(key);
            text.push('=');
            text.push_str(value);
        }
        if let Err(why) = append_line(&self.path, &text) {
            self.error.get_or_insert(why);
        } else {
            self.lines += 1;
        }
    }
}

/// Appends one whole line to `path`, closing a torn last line first.
fn append_line(path: &Path, line: &str) -> Result<(), String> {
    let refused = |why: std::io::Error| format!("{}: {why}", path.display());
    // ONE folder, never a chain: a store root that is gone (the drive was
    // unplugged) is refused, never made again somewhere else.
    if let Some(dir) = path.parent()
        && !dir.is_dir()
    {
        std::fs::create_dir(dir).map_err(refused)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .map_err(refused)?;
    let mut text = String::with_capacity(line.len() + 2);
    if file.metadata().map_err(refused)?.len() > 0 {
        let mut last = [0_u8; 1];
        file.seek(std::io::SeekFrom::End(-1)).map_err(refused)?;
        file.read_exact(&mut last).map_err(refused)?;
        if last != *b"\n" {
            text.push('\n');
        }
    }
    text.push_str(line);
    text.push('\n');
    file.write_all(text.as_bytes()).map_err(refused)?;
    file.sync_data().map_err(refused)
}

/// What earlier runs left that this one reads.
#[derive(Debug, Default)]
struct Memory {
    /// The link print of the last `begin` line.
    last_link: Option<String>,
    /// The file count of the last complete listing, per link print.
    complete: HashMap<String, u64>,
    /// `(name, rev)` to the verified file's path.
    verified: HashMap<(String, String), PathBuf>,
    /// Listed or computed content hashes of verified files, to their name.
    hashes: HashMap<String, String>,
    /// `(name, rev)` to the newest part's path, not yet promoted or refused.
    parts: HashMap<(String, String), PathBuf>,
    /// A name to the `(rev, listed content hash)` it was last verified with:
    /// what a re-delivery of a held day is told apart by (FM-007).
    delivered: HashMap<String, (String, String)>,
    /// Lines of this journal's own shape that do not parse: a torn line a
    /// crash left, closed by the next append. Counted, never read.
    torn: usize,
}

impl Memory {
    /// Reads every whole line of the journal: O(lines), once per press. A
    /// torn last line is ignored.
    fn load(path: &Path) -> Result<Self, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(why) => return Err(format!("{}: {why}", path.display())),
        };
        let mut memory = Self::default();
        let whole = text.rsplit_once('\n').map_or("", |(whole, _)| whole);
        for (number, line) in whole.lines().enumerate() {
            match parse_line(line) {
                Some(fields) => memory.read(&fields),
                None if line.starts_with("v=1 ") => memory.torn += 1,
                None => {
                    return Err(format!(
                        "{} line {}: not a journal line",
                        path.display(),
                        number + 1
                    ));
                }
            }
        }
        Ok(memory)
    }

    fn read(&mut self, fields: &Fields) {
        let key = || {
            (
                fields.get("name").unwrap_or_default().to_owned(),
                fields.get("rev").unwrap_or_default().to_owned(),
            )
        };
        match fields.verb.as_str() {
            "begin" => self.last_link = fields.get("link").map(str::to_owned),
            "listing" if fields.get("complete") == Some("true") => {
                if let (Some(link), Some(count)) = (
                    fields.get("link"),
                    fields.get("count").and_then(|c| c.parse().ok()),
                ) {
                    self.complete.insert(link.to_owned(), count);
                }
            }
            "part" => {
                if let Some(path) = fields.get("part") {
                    self.parts.insert(key(), PathBuf::from(path));
                }
            }
            "verified" => {
                let key = key();
                self.parts.remove(&key);
                for hash in [fields.get("hash"), fields.get("sha256")]
                    .into_iter()
                    .flatten()
                {
                    if !hash.is_empty() {
                        self.hashes.insert(hash.to_owned(), key.0.clone());
                    }
                }
                if let Some(hash) = fields.get("hash").filter(|hash| !hash.is_empty()) {
                    self.delivered
                        .insert(key.0.clone(), (key.1.clone(), hash.to_owned()));
                }
                if let Some(path) = fields.get("path") {
                    self.verified.insert(key, PathBuf::from(path));
                }
            }
            "refused" if fields.get("part").is_some() => {
                self.parts.remove(&key());
            }
            _ => {}
        }
    }
}

/// The journal's last whole line, read from at most its last 64 KiB: O(1)
/// in the journal's length. `None` for a missing or empty journal.
///
/// # Errors
///
/// The path and why it could not be read.
pub fn status(store_root: &Path) -> Result<Option<Fields>, String> {
    let path = journal_path(store_root);
    let refused = |why: std::io::Error| format!("{}: {why}", path.display());
    let mut file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(refused(why)),
    };
    let len = file.metadata().map_err(refused)?.len();
    let from = len.saturating_sub(64 * 1_024);
    file.seek(std::io::SeekFrom::Start(from)).map_err(refused)?;
    let mut tail = String::new();
    file.take(64 * 1_024)
        .read_to_string(&mut tail)
        .map_err(refused)?;
    let whole = tail.rsplit_once('\n').map_or("", |(whole, _)| whole);
    Ok(whole.lines().next_back().and_then(parse_line))
}

// -- the hand-off -------------------------------------------------------------

/// One verified file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// Its listed name.
    pub name: String,
    /// The day it states.
    pub day: crate::session::Day,
    /// Where it lies, in this run's staging folder.
    pub path: PathBuf,
    /// Its bytes.
    pub bytes: u64,
    /// Its members.
    pub entries: usize,
    /// Its SHA-256, in hex.
    pub sha256: String,
}

/// What a hand-off answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handed {
    /// Whether the writer took the file.
    pub ok: bool,
    /// The writer's verdict or refusal code.
    pub verdict: String,
}

/// Where a verified file goes next.
pub trait Handoff: Send {
    /// Hands `file` on.
    fn hand(&mut self, file: &Verified) -> Handed;
}

/// The tick-store writer (R4-08) as the hand-off.
#[derive(Debug, Clone)]
pub struct TickStoreHandoff {
    root: PathBuf,
    options: WriteOptions,
}

impl TickStoreHandoff {
    /// Writes into the tick store under `root` with `options`.
    #[must_use]
    pub const fn new(root: PathBuf, options: WriteOptions) -> Self {
        Self { root, options }
    }
}

impl Handoff for TickStoreHandoff {
    fn hand(&mut self, file: &Verified) -> Handed {
        let run = write_day(&file.path, &self.root, &self.options);
        match run.result {
            Ok(outcome) => Handed {
                ok: true,
                verdict: outcome.verdict().to_owned(),
            },
            Err(why) => Handed {
                ok: false,
                verdict: why.code().to_owned(),
            },
        }
    }
}

// -- the run ------------------------------------------------------------------

/// One press's inputs.
pub struct Fetch<'a> {
    /// The store root (journal and staging live under it).
    pub store_root: &'a Path,
    /// The tick store's root, whose day table says what is held.
    pub tick_root: &'a Path,
    /// The routes.
    pub endpoints: &'a Endpoints,
    /// Where the token comes from.
    pub token: &'a dyn TokenSource,
    /// Where free space is measured.
    pub meter: &'a dyn FreeSpace,
    /// The floor kept free.
    pub floor: Floor,
    /// How `429` and `5xx` are waited on.
    pub backoff: Backoff,
    /// The operator's Stop: read before each file and between body chunks.
    /// A stopped file keeps its part, journalled `stopped`, and the next
    /// press resumes it (FM-033).
    pub stop: Option<&'a AtomicBool>,
}

impl Fetch<'_> {
    /// Whether the operator pressed Stop.
    fn stopped(&self) -> bool {
        self.stop.is_some_and(|flag| flag.load(Ordering::Acquire))
    }
}

/// How a press ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every wanted file was verified and handed on.
    Complete,
    /// The listing was whole but some files were refused or not handed.
    Incomplete,
    /// The run stopped: the code and the words.
    Refused(RunRefusal),
    /// The run waits on the operator.
    Waiting(Wait),
}

impl Outcome {
    /// The outcome's word.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
            Self::Refused(_) => "refused",
            Self::Waiting(_) => "waiting",
        }
    }
}

/// A refused file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRefusal {
    /// Its listed path.
    pub path: String,
    /// The code.
    pub code: &'static str,
    /// The words.
    pub detail: String,
}

/// What one press did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The run's id.
    pub run: String,
    /// The run's staging folder, when one was made.
    pub staging: Option<PathBuf>,
    /// Files the listing named.
    pub listed: usize,
    /// Listing pages read.
    pub pages: usize,
    /// Files already held.
    pub held: usize,
    /// Files wanted.
    pub wanted: usize,
    /// Files verified this run, in order.
    pub verified: Vec<Verified>,
    /// Files refused, in order.
    pub refused: Vec<FileRefusal>,
    /// Hand-offs that did not take their file, by name.
    pub not_handed: Vec<(String, String)>,
    /// Resumed downloads.
    pub resumed: usize,
    /// The total wait on `429` and `5xx`.
    pub waited: Duration,
    /// Why a journal line could not be written, if one could not.
    pub journal_error: Option<String>,
    /// How it ended.
    pub outcome: Outcome,
}

/// A unique run id: the wall-clock millisecond and this process.
fn run_id() -> String {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis()),
        std::process::id(),
        SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

/// One press: lists, picks, downloads, verifies and hands on, journalling
/// every step. `config` is [`Config::load`]'s answer, so a missing link is
/// journalled as a wait.
pub async fn run(
    fetch: &Fetch<'_>,
    config: Result<Config, Wait>,
    handoff: &mut dyn Handoff,
) -> Report {
    let mut journal = Journal {
        path: journal_path(fetch.store_root),
        run: run_id(),
        lines: 0,
        error: None,
    };
    let mut report = Report {
        run: journal.run.clone(),
        staging: None,
        listed: 0,
        pages: 0,
        held: 0,
        wanted: 0,
        verified: Vec::new(),
        refused: Vec::new(),
        not_handed: Vec::new(),
        resumed: 0,
        waited: Duration::ZERO,
        journal_error: None,
        outcome: Outcome::Complete,
    };
    let outcome = match press(fetch, config, handoff, &mut journal, &mut report).await {
        Ok(()) if report.refused.is_empty() && report.not_handed.is_empty() => Outcome::Complete,
        Ok(()) => Outcome::Incomplete,
        Err(Stop::Wait(wait)) => {
            journal.line(
                "wait",
                &[
                    ("reason", wait.code().to_owned()),
                    ("detail", quoted(&wait.to_string())),
                ],
            );
            Outcome::Waiting(wait)
        }
        Err(Stop::Refused(refusal)) => {
            journal.line(
                "refused",
                &[
                    ("reason", refusal.code.to_owned()),
                    ("detail", quoted(&refusal.detail)),
                ],
            );
            Outcome::Refused(refusal)
        }
    };
    journal.line(
        "end",
        &[
            ("verdict", outcome.word().to_owned()),
            ("listed", report.listed.to_string()),
            ("wanted", report.wanted.to_string()),
            ("verified", report.verified.len().to_string()),
            ("refused", report.refused.len().to_string()),
            ("waited_ms", report.waited.as_millis().to_string()),
        ],
    );
    let _dropped = telemetry::emit(
        &telemetry::Event::info(TARGET, "run")
            .with("run", telemetry::Value::Str(&report.run))
            .with("verdict", telemetry::Value::Str(outcome.word()))
            .with(
                "verified",
                telemetry::Value::Uint(u64::try_from(report.verified.len()).unwrap_or(u64::MAX)),
            )
            .with(
                "refused",
                telemetry::Value::Uint(u64::try_from(report.refused.len()).unwrap_or(u64::MAX)),
            ),
    );
    report.journal_error = journal.error.take();
    report.outcome = outcome;
    report
}

/// The press, ending in a wait or a run refusal or `Ok` with every file's
/// verdict in `report`.
async fn press(
    fetch: &Fetch<'_>,
    config: Result<Config, Wait>,
    handoff: &mut dyn Handoff,
    journal: &mut Journal,
    report: &mut Report,
) -> Result<(), Stop> {
    let config = config?;
    let print = config.print();
    let staging = staging_root(fetch.store_root);
    check_roots(fetch, &staging)?;
    // Held until the press returns: a second press is `run_busy`.
    let _lock = lock_staging(fetch.store_root, &staging, &journal.run)?;
    let memory =
        Memory::load(&journal.path).map_err(|why| RunRefusal::new("journal_unreadable", why))?;
    if let Some(was) = &memory.last_link
        && *was != print
        && config.accept_link_print.as_deref() != Some(print.as_str())
    {
        return Err(Wait::LinkChanged {
            was: was.clone(),
            now: print,
        }
        .into());
    }
    journal.line("begin", &[("link", print.clone())]);
    let token = fetch.token.read()?;
    let mut wire = Wire {
        client: client(fetch.endpoints).map_err(|why| RunRefusal::new("transport", why))?,
        token,
        source: fetch.token,
        backoff: fetch.backoff,
        waited: Duration::ZERO,
        via: fetch.endpoints.proxy_in_use(),
    };
    let picked = listing(fetch, &mut wire, &config.link, &memory, journal, report).await?;
    plan_floor(fetch, &staging, &picked, &memory, journal)?;
    let run_dir = staging.join(&journal.run);
    std::fs::create_dir(&run_dir)
        .map_err(|why| RunRefusal::new("store_lost", format!("{}: {why}", run_dir.display())))?;
    report.staging = Some(run_dir.clone());
    let ctx = FileCtx {
        fetch,
        link: &config.link,
        run_dir: &run_dir,
        memory: &memory,
    };
    fetch_all(&ctx, &mut wire, &picked.wanted, handoff, journal, report).await
}

/// The staging root made under the store root ONE FOLDER AT A TIME, never
/// a chain (a store root that is gone, the drive unplugged, is refused and
/// never made again under `/Volumes`), and its lock taken: `run_busy` names
/// the holder.
fn lock_staging(store_root: &Path, staging: &Path, run: &str) -> Result<std::fs::File, Stop> {
    let lost = |path: &Path, why: std::io::Error| {
        RunRefusal::new("store_lost", format!("{}: {why}", path.display()))
    };
    let rest = staging.strip_prefix(store_root).unwrap_or(staging);
    let mut at = store_root.to_path_buf();
    for part in rest.components() {
        at.push(part);
        match std::fs::create_dir(&at) {
            Ok(()) => {}
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && at.is_dir() => {}
            Err(why) => return Err(lost(&at, why).into()),
        }
    }
    let path = staging.join(".lock");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&path)
        .map_err(|why| lost(&path, why))?;
    if lock.try_lock().is_err() {
        let mut holder = String::new();
        let _ = (&lock).read_to_string(&mut holder);
        return Err(RunRefusal::new(
            "run_busy",
            format!(
                "another press holds the staging lock: {}",
                holder.lines().next_back().unwrap_or("unknown")
            ),
        )
        .into());
    }
    let _ = writeln!(&lock, "run={run} pid={}", std::process::id());
    Ok(lock)
}

/// Lists the link to its end, refuses a shrunken listing and sorts it,
/// journalling the listing and every file's pick.
async fn listing(
    fetch: &Fetch<'_>,
    wire: &mut Wire<'_>,
    link: &str,
    memory: &Memory,
    journal: &mut Journal,
    report: &mut Report,
) -> Result<Picked, Stop> {
    let print = print_of(link);
    let listed = list(wire, fetch.endpoints, link).await;
    report.waited = wire.waited;
    let (files, pages) = match listed {
        Ok(found) => found,
        Err(stop) => {
            if let Stop::Refused(refusal) = &stop {
                journal.line(
                    "listing",
                    &[
                        ("link", print),
                        ("complete", "false".to_owned()),
                        ("reason", refusal.code.to_owned()),
                    ],
                );
            }
            return Err(stop);
        }
    };
    report.listed = files.len();
    report.pages = pages;
    let count = u64::try_from(files.len()).unwrap_or(u64::MAX);
    if let Some(&was) = memory.complete.get(&print)
        && count < was
    {
        return Err(RunRefusal::new("listing_shrank", format!("was={was} now={count}")).into());
    }
    let picked = pick(&files, fetch, memory)?;
    journal.line(
        "listing",
        &[
            ("link", print),
            ("complete", "true".to_owned()),
            ("count", count.to_string()),
            ("pages", pages.to_string()),
            ("wanted", picked.wanted.len().to_string()),
        ],
    );
    record_picks(&picked, journal, report);
    Ok(picked)
}

/// THE PLAN'S FLOOR, BEFORE ANY FILE: the listed bytes of every wanted file
/// not already verified, against the floor (D-4987). Exact, not an
/// extrapolation: the listing states each size.
fn plan_floor(
    fetch: &Fetch<'_>,
    staging: &Path,
    picked: &Picked,
    memory: &Memory,
    journal: &mut Journal,
) -> Result<(), Stop> {
    let plan: u64 = picked
        .wanted
        .iter()
        .filter(|f| {
            !memory
                .verified
                .contains_key(&(f.name.clone(), f.rev.clone()))
        })
        .map(|f| f.size)
        .fold(0, u64::saturating_add);
    let space = fetch
        .meter
        .space(staging)
        .map_err(|why| RunRefusal::new("free_space_unknown", why))?;
    if let Err(refusal) = floor_check(space, plan, fetch.floor) {
        return Err(RunRefusal::new(refusal.code(), refusal.to_string()).into());
    }
    journal.line(
        "floor",
        &[
            ("plan_bytes", plan.to_string()),
            ("free", space.free.to_string()),
            ("floor", fetch.floor.to_string()),
        ],
    );
    Ok(())
}

/// Downloads, verifies and hands on every wanted file in turn; a verified
/// file whose SHA-256 another name already holds is `content_duplicate`.
async fn fetch_all(
    ctx: &FileCtx<'_>,
    wire: &mut Wire<'_>,
    wanted: &[Listed],
    handoff: &mut dyn Handoff,
    journal: &mut Journal,
    report: &mut Report,
) -> Result<(), Stop> {
    let mut seen_sha: HashMap<String, String> = ctx.memory.hashes.clone();
    for file in wanted {
        if ctx.fetch.stopped() {
            journal.line(
                "stopped",
                &[
                    ("stage", "download".to_owned()),
                    ("before", quoted(&file.name)),
                ],
            );
            return Err(RunRefusal::new(
                "stopped",
                format!("the operator pressed Stop before {}", file.name),
            )
            .into());
        }
        let result = fetch_one(ctx, wire, file, journal, report).await;
        report.waited = wire.waited;
        if let Some(verified) = result? {
            if let Some(of) = seen_sha
                .get(&verified.sha256)
                .filter(|of| **of != verified.name)
            {
                refuse_file(
                    journal,
                    report,
                    file,
                    "content_duplicate",
                    format!("the same bytes as {of}"),
                );
                continue;
            }
            seen_sha.insert(verified.sha256.clone(), verified.name.clone());
            hand(handoff, &verified, journal, report);
            report.verified.push(verified);
        }
    }
    Ok(())
}

/// The staging root may not be, hold or lie inside the tick store, and the
/// store root must be there.
fn check_roots(fetch: &Fetch<'_>, staging: &Path) -> Result<(), Stop> {
    if !fetch.store_root.is_dir() {
        return Err(RunRefusal::new(
            "store_lost",
            format!("{} is not a folder", fetch.store_root.display()),
        )
        .into());
    }
    let tick = absolute(fetch.tick_root);
    let staging = absolute(staging);
    if tick.starts_with(&staging) || staging.starts_with(&tick) {
        return Err(RunRefusal::new(
            "staging_overlaps_published",
            format!(
                "staging {} and the tick store {} overlap",
                staging.display(),
                tick.display()
            ),
        )
        .into());
    }
    Ok(())
}

/// `path` made absolute and resolved as far as it exists.
fn absolute(path: &Path) -> PathBuf {
    let mut existing = path.to_path_buf();
    let mut rest = Vec::new();
    while !existing.exists() {
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_owned());
                existing = parent.to_path_buf();
            }
            _ => break,
        }
    }
    let mut out = std::fs::canonicalize(&existing).unwrap_or(existing);
    for name in rest.into_iter().rev() {
        out.push(name);
    }
    out
}

/// The listing sorted into wanted, held and refused files.
struct Picked {
    wanted: Vec<Listed>,
    held: Vec<(Listed, &'static str)>,
    refused: Vec<(Listed, &'static str, String)>,
    ignored: usize,
}

/// Sorts the listing. O(files), each one name parse and two probes.
fn pick(files: &[Listed], fetch: &Fetch<'_>, memory: &Memory) -> Result<Picked, Stop> {
    let done = crate::gdfl_import::done_whole_days(fetch.store_root).map_err(|why| {
        RunRefusal::new("journal_unreadable", format!("the import journal: {why}"))
    })?;
    let mut sorted: Vec<&Listed> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    let mut names = Names {
        folded: HashMap::with_capacity(sorted.len()),
        exact: HashMap::with_capacity(sorted.len()),
    };
    for file in &sorted {
        *names
            .folded
            .entry(file.name.to_ascii_lowercase())
            .or_default() += 1;
        *names.exact.entry(file.name.clone()).or_default() += 1;
    }
    let mut picked = Picked {
        wanted: Vec::new(),
        held: Vec::new(),
        refused: Vec::new(),
        ignored: 0,
    };
    let mut hashes: HashMap<&str, &str> = HashMap::new();
    let mut gdfl = 0_usize;
    for file in sorted {
        let identity = match names.verdict(file) {
            Ok(identity) => identity,
            Err(Some((code, detail, day_zip))) => {
                gdfl += usize::from(day_zip);
                picked.refused.push((file.clone(), code, detail));
                continue;
            }
            Err(None) => {
                picked.ignored += 1;
                continue;
            }
        };
        gdfl += 1;
        if let Some(why) = held_by(&identity, fetch, &done) {
            // A HELD DAY WHOSE LISTED CONTENT CHANGED is a re-delivery: named,
            // never skipped as held and never fetched over what is held
            // (FM-007). Importing it is a new generation, the writer's step.
            if let Some(changed) = redelivery(memory, file) {
                let detail = format!("{why}; {changed}");
                picked.refused.push((file.clone(), "redelivered", detail));
            } else {
                picked.held.push((file.clone(), why));
            }
            continue;
        }
        if !file.hash.is_empty() {
            let earlier = hashes
                .get(file.hash.as_str())
                .map(|name| (*name).to_owned())
                .or_else(|| {
                    memory
                        .hashes
                        .get(&file.hash)
                        .filter(|n| **n != file.name)
                        .cloned()
                });
            if let Some(of) = earlier {
                picked.refused.push((
                    file.clone(),
                    "content_duplicate",
                    format!("the same content hash as {of}"),
                ));
                continue;
            }
            hashes.insert(file.hash.as_str(), file.name.as_str());
        }
        picked.wanted.push(file.clone());
    }
    if gdfl == 0 {
        let sample: Vec<&str> = files.iter().take(5).map(|f| f.name.as_str()).collect();
        return Err(RunRefusal::new(
            "link_holds_no_gdfl_day_zip",
            format!("names_seen={} sample={}", files.len(), sample.join(",")),
        )
        .into());
    }
    Ok(picked)
}

/// How often each listed name occurs, exactly and with its case folded.
struct Names {
    folded: HashMap<String, usize>,
    exact: HashMap<String, usize>,
}

/// A name's refusal: its code, its words, and whether it IS a canonical day
/// zip name (so a listing of only refused day zips is not "no GDFL zip").
type NameRefusal = (&'static str, String, bool);

impl Names {
    /// The day a listed name is, or why it is refused (`Some`), or `None`
    /// for a name that is not a zip at all (ignored, counted). O(1).
    fn verdict(&self, file: &Listed) -> Result<DayIdentity, Option<NameRefusal>> {
        if let Some(why) = unsafe_name(&file.name) {
            return Err(Some(("name_unsafe", why.to_owned(), false)));
        }
        if !file.name.to_ascii_lowercase().ends_with(".zip") {
            return Err(None);
        }
        let upper = file.name.to_ascii_uppercase();
        if upper.contains("BSE") || upper.contains("MCX") {
            let why = "BSE and MCX are not pulled (CLAUDE.md section 1)".to_owned();
            return Err(Some(("out_of_scope_exchange", why, false)));
        }
        let exact = self.exact.get(&file.name).copied().unwrap_or(0);
        if self
            .folded
            .get(&file.name.to_ascii_lowercase())
            .copied()
            .unwrap_or(0)
            > exact
        {
            let why = "another listed name differs from it only in case".to_owned();
            return Err(Some(("names_collide", why, false)));
        }
        let Ok(identity) = DayIdentity::of_zip_name(&file.name) else {
            return Err(Some((
                "not_a_day_zip",
                "not a canonical GDFL day zip name".to_owned(),
                false,
            )));
        };
        if exact > 1 {
            let why = "the same day zip is listed in two folders".to_owned();
            return Err(Some(("two_files_for_one_day", why, true)));
        }
        if file.size == 0 || file.size > MAX_FILE_BYTES {
            return Err(Some((
                "size_unlikely",
                format!("listed size {}", file.size),
                true,
            )));
        }
        Ok(identity)
    }
}

/// Why the store already holds `identity`'s day, or `None`: the tick
/// store's day file, else an import closed `done` for the whole scope.
fn held_by(
    identity: &DayIdentity,
    fetch: &Fetch<'_>,
    done: &std::collections::HashSet<String>,
) -> Option<&'static str> {
    let kind = match identity.tree {
        Tree::Options => "options",
        Tree::Cm(crate::gdfl_cm::CmKind::Indices) => "indices",
        Tree::Cm(crate::gdfl_cm::CmKind::Stocks) => "stocks",
    };
    if identity.path_under(fetch.tick_root).exists() {
        Some("held_tick_store")
    } else if done.contains(&format!("{kind} {}", identity.day)) {
        Some("held_import_done")
    } else {
        None
    }
}

/// The words naming how a held file's listing differs from the delivery
/// this journal verified under its name, or `None` when it does not (or
/// nothing was verified under it). Only a listed content hash decides: a
/// changed `rev` with the same hash is the same bytes.
fn redelivery(memory: &Memory, file: &Listed) -> Option<String> {
    let (rev, hash) = memory.delivered.get(&file.name)?;
    (!file.hash.is_empty() && *hash != file.hash).then(|| {
        format!(
            "the listed content hash changed since rev {rev} was verified (was {hash}, now {}); \
             nothing held was fetched over or replaced",
            file.hash
        )
    })
}

/// Why `name` may not become a file name, or `None`.
fn unsafe_name(name: &str) -> Option<&'static str> {
    if name.is_empty() || name == "." || name == ".." {
        return Some("an empty or dot name");
    }
    if name.contains('/') || name.contains('\\') {
        return Some("a path separator");
    }
    if name.contains("..") {
        return Some("a `..`");
    }
    if name.chars().any(char::is_control) {
        return Some("a control character");
    }
    if name.starts_with('.') {
        return Some("a hidden name");
    }
    None
}

/// Journals every held, refused and wanted file of the listing.
fn record_picks(picked: &Picked, journal: &mut Journal, report: &mut Report) {
    report.held = picked.held.len();
    report.wanted = picked.wanted.len();
    for (file, why) in &picked.held {
        journal.line(
            "held",
            &[("name", quoted(&file.name)), ("reason", (*why).to_owned())],
        );
    }
    for (file, code, detail) in &picked.refused {
        refuse_file(journal, report, file, code, detail.clone());
    }
    for file in &picked.wanted {
        journal.line(
            "listed",
            &[
                ("name", quoted(&file.name)),
                ("path", quoted(&file.path)),
                ("bytes", file.size.to_string()),
                ("rev", quoted(&file.rev)),
                ("hash", quoted(&file.hash)),
            ],
        );
    }
    if picked.ignored > 0 {
        journal.line("ignored", &[("count", picked.ignored.to_string())]);
    }
}

/// Journals, emits and reports one refused file.
fn refuse_file(
    journal: &mut Journal,
    report: &mut Report,
    file: &Listed,
    code: &'static str,
    detail: String,
) {
    journal.line(
        "refused",
        &[
            ("name", quoted(&file.name)),
            ("rev", quoted(&file.rev)),
            ("reason", code.to_owned()),
            ("detail", quoted(&detail)),
        ],
    );
    emit_file(&journal.run, file, code);
    report.refused.push(FileRefusal {
        path: file.path.clone(),
        code,
        detail,
    });
}

/// The `pull.gdfl.dropbox` event of one file.
fn emit_file(run: &str, file: &Listed, verdict: &str) {
    let day = DayIdentity::of_zip_name(&file.name)
        .map(|identity| identity.day.to_string())
        .unwrap_or_default();
    let _dropped = telemetry::emit(
        &telemetry::Event::info(TARGET, verdict)
            .with("run", telemetry::Value::Str(run))
            .with("day", telemetry::Value::Str(&day))
            .with("file", telemetry::Value::Str(&file.name))
            .with("bytes", telemetry::Value::Uint(file.size)),
    );
}

/// Hands one verified file on and journals the answer.
fn hand(
    handoff: &mut dyn Handoff,
    verified: &Verified,
    journal: &mut Journal,
    report: &mut Report,
) {
    let handed = handoff.hand(verified);
    journal.line(
        "handed",
        &[
            ("name", quoted(&verified.name)),
            ("ok", handed.ok.to_string()),
            ("verdict", quoted(&handed.verdict)),
        ],
    );
    if !handed.ok {
        report
            .not_handed
            .push((verified.name.clone(), handed.verdict));
    }
}

/// What one file's download reads.
struct FileCtx<'a> {
    fetch: &'a Fetch<'a>,
    link: &'a str,
    run_dir: &'a Path,
    memory: &'a Memory,
}

/// A part file being written.
struct Part {
    path: PathBuf,
    resumed: bool,
}

/// Downloads, verifies and promotes one file. `Ok(None)` when the file was
/// refused (journalled); `Err` when the run must stop.
async fn fetch_one(
    ctx: &FileCtx<'_>,
    wire: &mut Wire<'_>,
    file: &Listed,
    journal: &mut Journal,
    report: &mut Report,
) -> Result<Option<Verified>, Stop> {
    store_there(ctx)?;
    let key = (file.name.clone(), file.rev.clone());
    if let Some(path) = ctx.memory.verified.get(&key).filter(|p| p.is_file()) {
        return match verify(path, file) {
            Ok(verified) => {
                journal.line("verified", &verified_pairs(&verified, file, "earlier"));
                emit_file(&journal.run, file, "verified");
                Ok(Some(verified))
            }
            Err((code, detail)) => {
                refuse_file(journal, report, file, code, detail);
                Ok(None)
            }
        };
    }
    // THE FLOOR BEFORE THIS FILE'S FIRST BYTE, and before its part exists:
    // the whole listed size, so a restart into a new part is covered too.
    let space = ctx
        .fetch
        .meter
        .space(ctx.run_dir)
        .map_err(|why| RunRefusal::new("free_space_unknown", why))?;
    if let Err(refusal) = floor_check(space, file.size, ctx.fetch.floor) {
        let detail = refusal.to_string();
        refuse_file(journal, report, file, refusal.code(), detail.clone());
        return Err(RunRefusal::new(
            "disk_floor",
            format!("stopped before {}: {detail}", file.name),
        )
        .into());
    }
    let part = match ctx.memory.parts.get(&key).filter(|p| p.is_file()) {
        Some(path) => Part {
            path: path.clone(),
            resumed: true,
        },
        None => new_part(ctx.run_dir, file, journal)?,
    };
    let Some(part) = fill_part(ctx, wire, file, part, journal, report).await? else {
        return Ok(None);
    };
    journal.line(
        "downloaded",
        &[
            ("name", quoted(&file.name)),
            ("part", quoted(&part.display().to_string())),
            ("bytes", file.size.to_string()),
        ],
    );
    let mut verified = match verify(&part, file) {
        Ok(verified) => verified,
        Err((code, detail)) => {
            refuse_part(journal, report, file, &part, code, detail);
            return Ok(None);
        }
    };
    let target = ctx.run_dir.join(&file.name);
    if std::fs::symlink_metadata(&target).is_ok() {
        refuse_file(
            journal,
            report,
            file,
            "name_taken",
            format!("{} exists and is never overwritten", target.display()),
        );
        return Ok(None);
    }
    // Verified once, then renamed in one step on one volume: the same inode,
    // so the bytes verified are the bytes promoted.
    std::fs::rename(&part, &target)
        .map_err(|why| RunRefusal::new("store_lost", format!("{}: {why}", target.display())))?;
    verified.path = target;
    journal.line("verified", &verified_pairs(&verified, file, "downloaded"));
    emit_file(&journal.run, file, "verified");
    Ok(Some(verified))
}

/// The store root is still there (the drive was not unplugged).
fn store_there(ctx: &FileCtx<'_>) -> Result<(), Stop> {
    if ctx.fetch.store_root.is_dir() {
        return Ok(());
    }
    Err(RunRefusal::new(
        "store_lost",
        format!("{} is gone", ctx.fetch.store_root.display()),
    )
    .into())
}

/// Downloads into `part` until it holds the listed size, resuming a cut by
/// `Range` and restarting into a new part when a resume cannot be trusted.
/// `Ok(None)` when the file was refused (journalled).
async fn fill_part(
    ctx: &FileCtx<'_>,
    wire: &mut Wire<'_>,
    file: &Listed,
    mut part: Part,
    journal: &mut Journal,
    report: &mut Report,
) -> Result<Option<PathBuf>, Stop> {
    let mut cuts = 0_u32;
    loop {
        store_there(ctx)?;
        let have = std::fs::metadata(&part.path).map_or(0, |m| m.len());
        if have > file.size {
            part = new_part(ctx.run_dir, file, journal)?;
            continue;
        }
        if have == file.size {
            return Ok(Some(part.path));
        }
        if part.resumed && have > 0 {
            report.resumed += 1;
        }
        match download(ctx, wire, file, &part, have, journal).await? {
            Got::Done => {}
            Got::Cut(bytes) => {
                cuts += 1;
                journal.line(
                    "cut",
                    &[
                        ("name", quoted(&file.name)),
                        ("part", quoted(&part.path.display().to_string())),
                        ("bytes", bytes.to_string()),
                    ],
                );
                if cuts > MAX_RESUMES {
                    refuse_file(
                        journal,
                        report,
                        file,
                        "download_cut",
                        format!("cut {cuts} times; the part is kept for the next press"),
                    );
                    return Ok(None);
                }
                part.resumed = true;
            }
            Got::Stopped(bytes) => {
                journal.line(
                    "stopped",
                    &[
                        ("stage", "download".to_owned()),
                        ("name", quoted(&file.name)),
                        ("part", quoted(&part.path.display().to_string())),
                        ("bytes", bytes.to_string()),
                    ],
                );
                return Err(RunRefusal::new("stopped", format!("the operator pressed Stop during {}; its part is kept and resumed by the next press", file.name)).into());
            }
            Got::Restart(why) => {
                journal.line(
                    "restarted",
                    &[("name", quoted(&file.name)), ("reason", why.to_owned())],
                );
                part = new_part(ctx.run_dir, file, journal)?;
            }
            Got::Refused(code, detail) => {
                let gone = code == "remote_gone";
                refuse_file(journal, report, file, code, detail.clone());
                if gone {
                    return Err(RunRefusal::new(
                        "link_revoked",
                        format!(
                            "{} answered {detail} after the listing; what was verified is kept",
                            file.name
                        ),
                    )
                    .into());
                }
                return Ok(None);
            }
        }
    }
}

/// Journals, emits and reports a refused finished part, which is kept.
fn refuse_part(
    journal: &mut Journal,
    report: &mut Report,
    file: &Listed,
    part: &Path,
    code: &'static str,
    detail: String,
) {
    journal.line(
        "refused",
        &[
            ("name", quoted(&file.name)),
            ("rev", quoted(&file.rev)),
            ("part", quoted(&part.display().to_string())),
            ("reason", code.to_owned()),
            ("detail", quoted(&detail)),
        ],
    );
    emit_file(&journal.run, file, code);
    report.refused.push(FileRefusal {
        path: file.path.clone(),
        code,
        detail,
    });
}

fn verified_pairs(verified: &Verified, file: &Listed, how: &str) -> Vec<(&'static str, String)> {
    vec![
        ("name", quoted(&file.name)),
        ("rev", quoted(&file.rev)),
        ("hash", quoted(&file.hash)),
        ("path", quoted(&verified.path.display().to_string())),
        ("bytes", verified.bytes.to_string()),
        ("entries", verified.entries.to_string()),
        ("sha256", verified.sha256.clone()),
        ("how", how.to_owned()),
    ]
}

/// A new part file in the run folder: `<name>.part`, else `<name>.part2`
/// and on, created with `create_new` so nothing is ever opened over.
fn new_part(run_dir: &Path, file: &Listed, journal: &mut Journal) -> Result<Part, Stop> {
    for generation in 1_u32..=64 {
        let name = if generation == 1 {
            format!("{}.part", file.name)
        } else {
            format!("{}.part{generation}", file.name)
        };
        let path = run_dir.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => {
                journal.line(
                    "part",
                    &[
                        ("name", quoted(&file.name)),
                        ("rev", quoted(&file.rev)),
                        ("part", quoted(&path.display().to_string())),
                    ],
                );
                return Ok(Part {
                    path,
                    resumed: false,
                });
            }
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(why) => {
                return Err(RunRefusal::new(
                    "staging_write_failed",
                    format!("{}: {why}", path.display()),
                )
                .into());
            }
        }
    }
    Err(RunRefusal::new(
        "staging_write_failed",
        format!("64 parts of {} exist", file.name),
    )
    .into())
}

/// What one download request ended in.
enum Got {
    Done,
    Cut(u64),
    /// The operator pressed Stop with this many bytes in the part.
    Stopped(u64),
    Restart(&'static str),
    Refused(&'static str, String),
}

/// `bytes a-b/total` from a `Content-Range`.
fn content_range(text: &str) -> Option<(u64, u64, u64)> {
    let rest = text.trim().strip_prefix("bytes ")?;
    let (span, total) = rest.split_once('/')?;
    let (start, end) = span.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?, total.parse().ok()?))
}

/// One request for `file` from byte `from`, appended to `part`.
async fn download(
    ctx: &FileCtx<'_>,
    wire: &mut Wire<'_>,
    file: &Listed,
    part: &Part,
    from: u64,
    journal: &mut Journal,
) -> Result<Got, Stop> {
    journal.line(
        "downloading",
        &[
            ("name", quoted(&file.name)),
            ("part", quoted(&part.path.display().to_string())),
            ("from", from.to_string()),
            ("bytes", file.size.to_string()),
        ],
    );
    let url = format!(
        "{}/2/sharing/get_shared_link_file",
        ctx.fetch.endpoints.content
    );
    let arg = ascii_json(&serde_json::json!({"url": ctx.link, "path": file.path}));
    let answer = wire
        .send("download", &|client, token| {
            let request = client
                .post(&url)
                .bearer_auth(token)
                .header("Dropbox-API-Arg", arg.clone())
                .header(reqwest::header::ACCEPT_ENCODING, "identity");
            if from > 0 {
                request.header(reqwest::header::RANGE, format!("bytes={from}-"))
            } else {
                request
            }
        })
        .await;
    let answer = match answer {
        Ok(answer) => answer,
        Err(Stop::Refused(refusal)) if refusal.code == "transport" => return Ok(Got::Cut(from)),
        Err(stop) => return Err(stop),
    };
    if let Some(got) = judge(&answer, file, from) {
        return Ok(got);
    }
    let start = if answer.status == 206 { from } else { 0 };
    if start != from {
        return Ok(Got::Restart("range_ignored"));
    }
    write_body(answer.response, &part.path, file.size, ctx.fetch).await
}

/// The verdict a download's status and headers force, or `None` to write
/// its body.
fn judge(answer: &Answer, file: &Listed, from: u64) -> Option<Got> {
    let headers = answer.response.headers();
    if (300..400).contains(&answer.status) {
        let host = headers
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split("//").nth(1))
            .and_then(|v| v.split('/').next())
            .unwrap_or("none");
        return Some(Got::Refused("redirect_refused", format!("location={host}")));
    }
    if answer.status == 416 {
        return Some(Got::Restart("remote_changed"));
    }
    if answer.status == 404 || answer.status == 409 {
        return Some(Got::Refused(
            "remote_gone",
            format!("status={}", answer.status),
        ));
    }
    if answer.status != 200 && answer.status != 206 {
        return Some(Got::Refused(
            "http_status",
            format!("status={}", answer.status),
        ));
    }
    let html = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.to_ascii_lowercase().starts_with("text/html"));
    if html {
        return Some(Got::Refused(
            "not_a_zip",
            "the body is an HTML page".to_owned(),
        ));
    }
    let rev = headers
        .get("Dropbox-API-Result")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok())
        .and_then(|v| v.get("rev").and_then(|r| r.as_str().map(str::to_owned)));
    if let Some(rev) = rev
        && !file.rev.is_empty()
        && rev != file.rev
    {
        return Some(Got::Refused(
            "remote_changed",
            format!("listed rev {} served rev {rev}", file.rev),
        ));
    }
    if answer.status == 206 {
        let range = headers
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(content_range);
        match range {
            Some((start, _, total)) if start == from && total == file.size => {}
            _ => return Some(Got::Restart("remote_changed")),
        }
    }
    None
}

/// Appends the body to `path`, at most `size` bytes in all, reading the
/// operator's Stop between chunks.
async fn write_body(
    response: reqwest::Response,
    path: &Path,
    size: u64,
    fetch: &Fetch<'_>,
) -> Result<Got, Stop> {
    let mut response = response;
    let mut out = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|why| {
            RunRefusal::new("staging_write_failed", format!("{}: {why}", path.display()))
        })?;
    let mut have = out
        .metadata()
        .map_err(|why| {
            RunRefusal::new("staging_write_failed", format!("{}: {why}", path.display()))
        })?
        .len();
    loop {
        if fetch.stopped() {
            let _ = out.sync_data();
            return Ok(Got::Stopped(have));
        }
        let chunk = match response.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(_) => {
                let _ = out.sync_data();
                return Ok(Got::Cut(have));
            }
        };
        let len = u64::try_from(chunk.len()).unwrap_or(u64::MAX);
        if have.saturating_add(len) > size {
            let _ = out.sync_data();
            return Ok(Got::Refused(
                "size_mismatch",
                format!("the body runs past the listed {size} bytes"),
            ));
        }
        if let Err(why) = write_chunk(&mut out, have, &chunk) {
            let _ = out.sync_data();
            return Err(write_failed(path, have, &why).into());
        }
        have += len;
    }
    out.sync_data().map_err(|why| {
        RunRefusal::new("staging_write_failed", format!("{}: {why}", path.display()))
    })?;
    if have < size {
        return Ok(Got::Cut(have));
    }
    Ok(Got::Done)
}

/// The run refusal a failed part write names: a full disk is `disk_full`
/// (FM-018), anything else `staging_write_failed`. Either way the part is
/// kept, named, never promoted, and the next press resumes it.
fn write_failed(path: &Path, have: u64, why: &std::io::Error) -> RunRefusal {
    let code = if why.kind() == std::io::ErrorKind::StorageFull {
        "disk_full"
    } else {
        "staging_write_failed"
    };
    RunRefusal::new(
        code,
        format!(
            "stage=download {} at {have} bytes: {why}; the part is kept for the next press",
            path.display()
        ),
    )
}

#[cfg(test)]
thread_local! {
    /// A test's full disk: the first chunk that would take a part past this
    /// many bytes fails with `StorageFull`, as `ENOSPC` does.
    pub(crate) static FULL_AT: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// One chunk appended to a part.
fn write_chunk(out: &mut std::fs::File, have: u64, chunk: &[u8]) -> std::io::Result<()> {
    #[cfg(test)]
    if let Some(at) = FULL_AT.with(std::cell::Cell::get)
        && have.saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX)) > at
    {
        return Err(std::io::Error::from(std::io::ErrorKind::StorageFull));
    }
    let _ = have;
    out.write_all(chunk)
}

/// Checks a finished file: its size, its central directory read whole,
/// every member under the cap and its CRC.
fn verify(path: &Path, file: &Listed) -> Result<Verified, (&'static str, String)> {
    let day = DayIdentity::of_zip_name(&file.name)
        .map_err(|_| {
            (
                "not_a_day_zip",
                "not a canonical GDFL day zip name".to_owned(),
            )
        })?
        .day;
    let source = std::fs::File::open(path)
        .map_err(|why| ("staging_unreadable", format!("{}: {why}", path.display())))?;
    let len = source
        .metadata()
        .map_err(|why| ("staging_unreadable", format!("{}: {why}", path.display())))?
        .len();
    if len != file.size {
        return Err((
            "size_mismatch",
            format!("listed {} bytes, have {len}", file.size),
        ));
    }
    let entries =
        zip_entries(&source, 0, len).map_err(|why| ("zip_unreadable", why.to_string()))?;
    if entries.is_empty() {
        return Err(("zip_holds_no_member", "the zip lists no member".to_owned()));
    }
    for entry in &entries {
        if entry.len > MEMBER_BYTE_CAP {
            return Err((
                "member_over_byte_cap",
                format!(
                    "{} states {} bytes; the cap is {MEMBER_BYTE_CAP}",
                    entry.name, entry.len
                ),
            ));
        }
        let bytes = member_bytes(&source, entry.locator, &entry.name, entry.len)
            .map_err(|why| ("member_unreadable", format!("{}: {why}", entry.name)))?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != entry.len {
            return Err((
                "member_size_mismatch",
                format!("{} is not {} bytes", entry.name, entry.len),
            ));
        }
        if crc32(&bytes) != entry.crc32 {
            return Err(("crc_mismatch", format!("member {}", entry.name)));
        }
    }
    Ok(Verified {
        name: file.name.clone(),
        day,
        path: path.to_owned(),
        bytes: len,
        entries: entries.len(),
        sha256: sha256_of(&source).map_err(|why| ("staging_unreadable", why))?,
    })
}

/// The SHA-256 of a whole file, in hex.
fn sha256_of(file: &std::fs::File) -> Result<String, String> {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    let mut reader: &std::fs::File = file;
    let mut buf = vec![0_u8; 1 << 20];
    reader
        .seek(std::io::SeekFrom::Start(0))
        .map_err(|why| why.to_string())?;
    loop {
        let n = reader.read(&mut buf).map_err(|why| why.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(buf.get(..n).unwrap_or_default());
    }
    Ok(hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            use std::fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
            out
        }))
}

#[cfg(test)]
#[path = "gdfl_dropbox_tests.rs"]
mod tests;

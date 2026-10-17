//! `cli watchdog` and `cli launch-agent`: the app's supervisor and watchdog,
//! outside the app (R3-32, D-4996, FM-196).
//!
//! # What runs the app, and what notices when it does not answer
//!
//! Two launchd agents, both written by `cli launch-agent` at install time and
//! never tracked: `CLAUDE.md` §2 allows no `.plist` in the repository (gate 1;
//! amendment A5 of the round-3 plan), so the agent files are the output of
//! this verb, not files in the tree. The operator installs them from
//! `docs/36-ops-runbook.md` §9; this crate installs nothing.
//!
//! - `in.brutex.app` runs `api serve ADDR`. Its `KeepAlive` is a `PathState`
//!   on the store root, so launchd keeps the app alive while the store drive is
//!   mounted and does not start it again while the drive is absent:
//!   *"the job will be kept alive as long as the path exists"*
//!   (`launchd.plist(5)`, `KeepAlive` → `PathState`). `ThrottleInterval`
//!   bounds a refusal loop to one start per interval (*"jobs will not be
//!   spawned more than once every 10 seconds"* by default, ibid.).
//! - `in.brutex.watchdog` runs `cli watchdog URL` every `StartInterval`
//!   seconds. Each run is ONE probe of `/health` with a 5-second limit; the
//!   memory between runs is one small state file in a directory off the store
//!   drive, so an unmounted drive cannot silence the watchdog that should say
//!   so.
//!
//! # The rule the watchdog keeps
//!
//! An outage is [`DOWN_AFTER`] probes in a row with no answer. Its one notice
//! goes out at the probe that makes it an outage — at most two probes, 120 s at
//! the default interval, after the last answer — and never again for the same
//! outage; the first answer after it sends exactly one more, "back". A single
//! missed probe that is answered at the next one is a `blip`: one log line, no
//! notice, which is what a launchd restart inside the minute looks like.
//! An answer is any HTTP status line: `/health` answers 503 when the data is
//! not clean, and that is the app answering — the health page says the rest.
//!
//! # Why the notice is a page and not a Notification Center banner
//!
//! A macOS banner from a program needs either `osascript` (an AppleScript
//! interpreter, `Family::Script` in gate 0's scan) or an Objective-C binding,
//! and `CLAUDE.md` §2 forbids both without exception. The notice is therefore
//! a small HTML page written under the state directory and handed to `open`,
//! the platform's own handler that `api` already uses for the browser
//! (D-1202): the operator's default browser shows it. Every notice and every
//! failure to deliver one is also a line of `watchdog.log`.
//!
//! # Cost
//!
//! One run is one TCP connect, one request, at most [`MAX_STATUS_LINE`] plus
//! one read's bytes, one state read of at most [`MAX_STATE_BYTES`] + 1 bytes,
//! at most a few appended log lines, one page created by name and one rename
//! (two when a notice is sent: it is saved before it is sent): O(1) in the
//! length of the log and the number of pages, because neither is ever read or
//! listed
//! (`cli::watchdog_command::tests::a_run_never_lists_the_notices_or_reads_the_log`).
//! A held lock is waited for at most [`LOCK_WAIT`]; the drive check is one
//! `stat` per missing component of the state directory's path. The only thing
//! that grows is `watchdog.log`, by one line per transition.

use std::fmt::Write as _;
use std::io::{Read as _, Write as _};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::{FAILED, MISUSED, OK};

/// Probes in a row with no answer that make an outage. Two, so a restart that
/// lands inside one interval is a blip and the notice still comes within two
/// probes of the app going down.
pub const DOWN_AFTER: u32 = 2;

/// The limit on one probe: connect, ask and read the status line.
pub const PROBE_LIMIT: Duration = Duration::from_secs(5);

/// The longest status line read before the answer is called "not HTTP".
pub const MAX_STATUS_LINE: usize = 1024;

/// The longest `open` may take to hand a notice to the browser before it is
/// killed and the notice counted as not delivered.
const OPEN_LIMIT: Duration = Duration::from_secs(10);

/// The watchdog's memory between probes, in the state directory.
pub const STATE_FILE: &str = "watchdog.state";

/// One line per transition, appended; never rewritten.
pub const LOG_FILE: &str = "watchdog.log";

/// Held for the length of one probe, so two probes never interleave.
pub const LOCK_FILE: &str = "watchdog.lock";

/// The notice pages, one new file each, never overwritten.
pub const NOTICE_DIR: &str = "notices";

/// The app agent's launchd label.
pub const APP_LABEL: &str = "in.brutex.app";

/// The watchdog agent's launchd label.
pub const WATCHDOG_LABEL: &str = "in.brutex.watchdog";

const WATCH_USAGE: &str = "usage: cli watchdog URL --state-dir DIR [--store DIR] [--timeout-ms N] [--interval-seconds N] [--notify open|record]";
const AGENT_USAGE: &str = "usage: cli launch-agent app API ADDR --store DIR --log-dir DIR [--env KEY=VALUE ...] [--throttle-seconds N] [--max-files N] [--keep-awake]\n       cli launch-agent watchdog CLI URL --state-dir DIR [--store DIR] [--interval-seconds N]";

/// Whether the verb in `first` runs without a store root.
///
/// `main` refuses every other verb before dispatch when the store root is
/// missing. The watchdog must run exactly when it is missing — an unmounted
/// drive is one of the outages it reports — and `launch-agent` writes nothing
/// and reads no store.
#[must_use]
pub fn needs_no_store(first: Option<&std::ffi::OsStr>) -> bool {
    first.is_some_and(|word| word == "watchdog" || word == "launch-agent")
}

/// Whether `main` prints where events went for the verb in `first`.
///
/// `cli launch-agent`'s stdout is the agent file and nothing else, so its
/// banner goes to stderr. `cli watchdog` runs 1,440 times a day into one
/// launchd log whose agent already names the events folder (`docs/36` §9.5),
/// so it prints none; a run whose events are NOT recorded still says so on
/// stderr (`announce_unlogged`). Every other verb prints it on stdout.
#[must_use]
pub fn banner_on_stderr(first: Option<&std::ffi::OsStr>) -> Option<bool> {
    match first {
        Some(word) if word == "watchdog" => None,
        Some(word) if word == "launch-agent" => Some(true),
        _ => Some(false),
    }
}

// ---------------------------------------------------------------------------
// The address
// ---------------------------------------------------------------------------

/// Where `/health` is asked: plain `http://` on a literal IP or `localhost`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    addr: SocketAddr,
    host: String,
    path: String,
}

impl Target {
    /// Reads `url`, or names what is wrong with it.
    ///
    /// No name is resolved: a resolver can block for longer than the probe's
    /// limit, so only an IP literal or `localhost` (127.0.0.1) is accepted.
    ///
    /// # Errors
    ///
    /// Anything but `http://HOST[:PORT][/PATH]` with such a host.
    pub fn parse(url: &str) -> Result<Self, String> {
        if url.is_empty() || url.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(format!(
                "`{}` is not an address: it is empty or holds a space or a control character",
                url.escape_debug()
            ));
        }
        let rest = url.strip_prefix("http://").ok_or_else(|| {
            format!(
                "`{url}` is not an http:// address; the watchdog asks a local app over plain HTTP"
            )
        })?;
        let (authority, path) = rest.find('/').map_or((rest, "/"), |at| rest.split_at(at));
        if authority.contains('@') {
            return Err(format!(
                "`{url}` carries a user name; the watchdog sends none"
            ));
        }
        let (host, port) = split_host_port(authority)?;
        let ip: IpAddr = if host == "localhost" {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        } else {
            host.parse().map_err(|_| {
                format!(
                    "host `{host}` is not an IP address or `localhost`; the watchdog resolves no names, so a slow resolver cannot stretch its limit"
                )
            })?
        };
        Ok(Self {
            addr: SocketAddr::new(ip, port),
            host: authority.to_owned(),
            path: path.to_owned(),
        })
    }

    /// The socket address asked.
    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

fn split_host_port(authority: &str) -> Result<(&str, u16), String> {
    let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
        let (inner, after) = bracketed
            .split_once(']')
            .ok_or_else(|| format!("`{authority}` opens an IPv6 bracket and never closes it"))?;
        match after {
            "" => (inner, None),
            _ => (
                inner,
                Some(after.strip_prefix(':').ok_or_else(|| {
                    format!("`{authority}`: only `:PORT` may follow the bracket")
                })?),
            ),
        }
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    if host.is_empty() {
        return Err(format!("`{authority}` names no host"));
    }
    // Digits only, and no leading zero: `u16::from_str` alone takes `+80`
    // and `0080`, two spellings of a port nobody writes on purpose.
    let port = match port {
        None => 80,
        Some(text) if !text.bytes().all(|b| b.is_ascii_digit()) || text.starts_with('0') => {
            return Err(format!(
                "`{text}` is not a port from 1 to 65535 (digits only, no sign, no leading zero)"
            ));
        }
        Some(text) => match text.parse::<u16>() {
            Ok(0) | Err(_) => return Err(format!("`{text}` is not a port from 1 to 65535")),
            Ok(port) => port,
        },
    };
    Ok((host, port))
}

// ---------------------------------------------------------------------------
// One probe
// ---------------------------------------------------------------------------

/// What one probe found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// An HTTP status line came back within the limit.
    Answered {
        /// The status code, 503 included: the app answered.
        status: u16,
        /// From the connect to the status line.
        millis: u64,
    },
    /// Nothing that is an answer came back within the limit.
    NoAnswer {
        /// Why, in words: refused, timed out, closed, not HTTP.
        why: String,
        /// How long the probe waited.
        millis: u64,
    },
}

impl Outcome {
    fn answered(&self) -> bool {
        matches!(self, Self::Answered { .. })
    }
}

/// Asks `target` once, within `limit` from the first byte to the status line.
#[must_use]
pub fn probe(target: &Target, limit: Duration) -> Outcome {
    let started = Instant::now();
    let asked = ask(target, started, limit);
    let millis = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    match asked {
        Ok(status) => Outcome::Answered { status, millis },
        Err(why) => Outcome::NoAnswer { why, millis },
    }
}

fn left(started: Instant, limit: Duration) -> Result<Duration, String> {
    let left = limit.saturating_sub(started.elapsed());
    if left.is_zero() {
        Err(format!("no answer within {} ms", limit.as_millis()))
    } else {
        Ok(left)
    }
}

fn io_word(why: &std::io::Error, limit: Duration, doing: &str) -> String {
    match why.kind() {
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
            format!("no answer within {} ms", limit.as_millis())
        }
        _ => format!("the connection failed while {doing}: {why}"),
    }
}

fn ask(target: &Target, started: Instant, limit: Duration) -> Result<u16, String> {
    let mut stream = std::net::TcpStream::connect_timeout(&target.addr, left(started, limit)?)
        .map_err(|why| match why.kind() {
            std::io::ErrorKind::ConnectionRefused => {
                format!("connection refused: nothing listens on {}", target.addr)
            }
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                format!("no connection within {} ms", limit.as_millis())
            }
            _ => format!("could not connect to {}: {why}", target.addr),
        })?;
    stream
        .set_write_timeout(Some(left(started, limit)?))
        .map_err(|why| io_word(&why, limit, "asking"))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: brutex-watchdog\r\nConnection: close\r\n\r\n",
        target.path, target.host
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|why| io_word(&why, limit, "asking"))?;
    let mut head: Vec<u8> = Vec::with_capacity(256);
    let mut chunk = [0_u8; 512];
    loop {
        if let Some(end) = head.windows(2).position(|pair| pair == b"\r\n") {
            return status_of(head.get(..end).unwrap_or_default());
        }
        if head.len() > MAX_STATUS_LINE {
            return Err(format!(
                "not an HTTP answer: no status line in the first {MAX_STATUS_LINE} bytes"
            ));
        }
        stream
            .set_read_timeout(Some(left(started, limit)?))
            .map_err(|why| io_word(&why, limit, "reading"))?;
        let read = stream
            .read(&mut chunk)
            .map_err(|why| io_word(&why, limit, "reading"))?;
        if read == 0 {
            return Err(if head.is_empty() {
                "the connection closed with no answer".to_owned()
            } else {
                "the connection closed inside the status line".to_owned()
            });
        }
        head.extend_from_slice(chunk.get(..read).unwrap_or_default());
    }
}

fn status_of(line: &[u8]) -> Result<u16, String> {
    let text = String::from_utf8_lossy(line);
    let mut words = text.split(' ');
    let version = words.next().unwrap_or_default();
    let code = words.next().unwrap_or_default();
    let status = (version.starts_with("HTTP/1.") && code.len() == 3)
        .then(|| code.parse::<u16>().ok())
        .flatten()
        .filter(|status| (100..=599).contains(status));
    status.ok_or_else(|| {
        let shown: String = text.chars().take(64).collect();
        format!("not an HTTP answer: `{}`", shown.escape_debug())
    })
}

// ---------------------------------------------------------------------------
// The watchdog's memory and its one rule
// ---------------------------------------------------------------------------

/// Where the app stood at the last probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// No probe recorded yet.
    Fresh,
    /// The last probe was answered.
    Up,
    /// Fewer than [`DOWN_AFTER`] probes in a row unanswered.
    Suspect,
    /// [`DOWN_AFTER`] or more probes in a row unanswered: an outage.
    Down,
}

impl Phase {
    fn word(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::Up => "up",
            Self::Suspect => "suspect",
            Self::Down => "down",
        }
    }

    fn from_word(word: &str) -> Option<Self> {
        match word {
            "fresh" => Some(Self::Fresh),
            "up" => Some(Self::Up),
            "suspect" => Some(Self::Suspect),
            "down" => Some(Self::Down),
            _ => None,
        }
    }
}

/// The state file, whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// Where the app stood.
    pub phase: Phase,
    /// When the current phase began (unix seconds): for an outage, its first
    /// unanswered probe.
    pub since: u64,
    /// Unanswered probes in a row.
    pub missed: u32,
    /// The last answered probe, if any.
    pub last_answer: Option<u64>,
    /// The last probe of any kind.
    pub last_probe: u64,
    /// Whether this outage's notice reached `open`.
    pub down_notified: bool,
    /// A notice this state was saved for BEFORE it was handed to `open`; set
    /// only between that save and the save after delivery. A run that finds
    /// it set ended mid-notice (killed, or its last save failed): the notice
    /// is counted as sent and never sent again, so a notice goes out at most
    /// once per outage even when its run dies.
    pub pending: Option<Notice>,
}

/// The state line's head as it is written now; v2 added `pending=`.
const STATE_HEAD: &str = "watchdog-state v2";

/// The first version's head: still read (append-only history, `CLAUDE.md`
/// §3 rule 8), never written. A v1 line has no `pending=` and means none.
const STATE_HEAD_V1: &str = "watchdog-state v1";

/// The longest state file read. A v2 line is under 200 bytes, so anything
/// longer is damage, kept aside without being read whole: the read is O(1)
/// whatever was left at the name.
pub const MAX_STATE_BYTES: u64 = 512;

impl State {
    /// Nothing recorded.
    #[must_use]
    pub fn fresh() -> Self {
        Self {
            phase: Phase::Fresh,
            since: 0,
            missed: 0,
            last_answer: None,
            last_probe: 0,
            down_notified: false,
            pending: None,
        }
    }

    /// The state file's one line.
    #[must_use]
    pub fn encode(&self) -> String {
        format!(
            "{STATE_HEAD} phase={} since={} missed={} last_answer={} last_probe={} down_notified={} pending={}\n",
            self.phase.word(),
            self.since,
            self.missed,
            self.last_answer
                .map_or_else(|| "-".to_owned(), |at| at.to_string()),
            self.last_probe,
            u8::from(self.down_notified),
            self.pending.map_or("-", notice_word),
        )
    }

    /// Reads the state file's line back, or names the first thing wrong.
    ///
    /// # Errors
    ///
    /// Any line [`State::encode`] did not write: torn, edited, another version.
    pub fn decode(text: &str) -> Result<Self, String> {
        let line = text
            .strip_suffix('\n')
            .ok_or("the state line has no end: it was cut short")?;
        let after = |head: &str| {
            line.strip_prefix(head)
                .and_then(|rest| rest.strip_prefix(' '))
        };
        let (fields, v1) = match (after(STATE_HEAD), after(STATE_HEAD_V1)) {
            (Some(fields), _) => (fields, false),
            (None, Some(fields)) => (fields, true),
            (None, None) => {
                return Err(format!(
                    "the state line begins neither `{STATE_HEAD}` nor `{STATE_HEAD_V1}`"
                ));
            }
        };
        let mut parts = fields.split(' ');
        let mut field = |key: &str| {
            parts
                .next()
                .and_then(|part| part.strip_prefix(key))
                .and_then(|part| part.strip_prefix('='))
                .ok_or_else(|| format!("the state line has no `{key}=` where it belongs"))
        };
        let number = |key: &str, text: &str| {
            text.parse::<u64>()
                .map_err(|_| format!("`{key}={text}` is not a count"))
        };
        let phase = field("phase")?;
        let phase = Phase::from_word(phase).ok_or_else(|| format!("`{phase}` is not a phase"))?;
        let since = number("since", field("since")?)?;
        let missed_text = field("missed")?;
        let missed = u32::try_from(number("missed", missed_text)?)
            .map_err(|_| format!("`missed={missed_text}` is out of range"))?;
        let last_answer = match field("last_answer")? {
            "-" => None,
            text => Some(number("last_answer", text)?),
        };
        let last_probe = number("last_probe", field("last_probe")?)?;
        let down_notified = match field("down_notified")? {
            "0" => false,
            "1" => true,
            other => return Err(format!("`down_notified={other}` is not 0 or 1")),
        };
        let pending = if v1 {
            None
        } else {
            match field("pending")? {
                "-" => None,
                "down" => Some(Notice::Down),
                "back" => Some(Notice::Back),
                other => return Err(format!("`pending={other}` is not -, down or back")),
            }
        };
        if parts.next().is_some() {
            return Err(format!(
                "the state line carries more fields than {} has",
                if v1 { "v1" } else { "v2" }
            ));
        }
        Ok(Self {
            phase,
            since,
            missed,
            last_answer,
            last_probe,
            down_notified,
            pending,
        })
    }
}

/// What one probe changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    /// The first answer this state directory has seen.
    Watching,
    /// An unanswered probe, short of an outage.
    Suspect,
    /// An answer after fewer than [`DOWN_AFTER`] unanswered probes.
    Blip {
        /// The unanswered probes before it.
        missed: u32,
    },
    /// The probe that made an outage: its one notice.
    Down,
    /// Still down, and the outage's notice has not been delivered: tried again.
    DownAgain,
    /// The first answer after an outage whose notice was delivered.
    Back {
        /// From the outage's first unanswered probe to this answer.
        down_seconds: u64,
        /// The unanswered probes in the outage.
        missed: u32,
    },
    /// The first answer after an outage whose notice never reached `open`.
    BackUnnotified {
        /// From the outage's first unanswered probe to this answer.
        down_seconds: u64,
        /// The unanswered probes in the outage.
        missed: u32,
    },
}

impl Transition {
    fn word(&self) -> &'static str {
        match self {
            Self::Watching => "watching",
            Self::Suspect => "suspect",
            Self::Blip { .. } => "blip",
            Self::Down => "down",
            Self::DownAgain => "down-again",
            Self::Back { .. } => "back",
            Self::BackUnnotified { .. } => "back-unnotified",
        }
    }

    /// The notice this transition sends, if any.
    #[must_use]
    pub fn notice(&self) -> Option<Notice> {
        match self {
            Self::Down | Self::DownAgain => Some(Notice::Down),
            Self::Back { .. } => Some(Notice::Back),
            _ => None,
        }
    }
}

/// The two notices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice {
    /// The app stopped answering.
    Down,
    /// The app answers again.
    Back,
}

/// The watchdog's one rule, with no clock and no file: the next state after a
/// probe that found `outcome` at `now`, and what changed.
///
/// A [`Notice::Down`] is assumed delivered (`down_notified`); the caller sets
/// it back when `open` fails, so the next probe tries again.
#[must_use]
pub fn step(prev: &State, outcome: &Outcome, now: u64) -> (State, Option<Transition>) {
    let answered = outcome.answered();
    let up = State {
        phase: Phase::Up,
        since: now,
        missed: 0,
        last_answer: Some(now),
        last_probe: now,
        down_notified: false,
        pending: None,
    };
    match (prev.phase, answered) {
        (Phase::Fresh, true) => (up, Some(Transition::Watching)),
        (Phase::Up, true) => (
            State {
                since: prev.since,
                ..up
            },
            None,
        ),
        (Phase::Suspect, true) => (
            up,
            Some(Transition::Blip {
                missed: prev.missed,
            }),
        ),
        (Phase::Down, true) => {
            let down_seconds = now.saturating_sub(prev.since);
            let transition = if prev.down_notified {
                Transition::Back {
                    down_seconds,
                    missed: prev.missed,
                }
            } else {
                Transition::BackUnnotified {
                    down_seconds,
                    missed: prev.missed,
                }
            };
            (up, Some(transition))
        }
        (Phase::Fresh | Phase::Up | Phase::Suspect, false) => {
            let missed = if prev.phase == Phase::Suspect {
                prev.missed.saturating_add(1)
            } else {
                1
            };
            let since = if prev.phase == Phase::Suspect {
                prev.since
            } else {
                now
            };
            let outage = missed >= DOWN_AFTER;
            let next = State {
                phase: if outage { Phase::Down } else { Phase::Suspect },
                since,
                missed,
                last_answer: prev.last_answer,
                last_probe: now,
                down_notified: outage,
                pending: None,
            };
            let transition = if outage {
                Transition::Down
            } else {
                Transition::Suspect
            };
            (next, Some(transition))
        }
        (Phase::Down, false) => {
            let next = State {
                missed: prev.missed.saturating_add(1),
                last_probe: now,
                down_notified: true,
                pending: None,
                ..prev.clone()
            };
            let transition = (!prev.down_notified).then_some(Transition::DownAgain);
            (next, transition)
        }
    }
}

/// Intervals past which an earlier unanswered probe no longer counts toward
/// an outage. Two misses are "in a row" only if no probe of the watchdog's own
/// was skipped between them; a gap longer than two intervals means one was
/// (the Mac slept or was off), so the miss before it is history, not half an
/// outage. Without this, a miss at shutdown and one at the next login a day
/// later made a DOWN notice the moment the operator logged in (verify-R3-32-r1
/// B2).
pub const STALE_INTERVALS: u64 = 2;

/// The state a probe at `now` is judged against, when the last probe left a
/// [`Phase::Suspect`] older than [`STALE_INTERVALS`] intervals: that miss is
/// dropped and the probe is judged as if the app had last answered. `None`
/// when nothing expires. Pure: no clock and no file.
#[must_use]
pub fn expire_stale(prev: &State, now: u64, interval_seconds: u64) -> Option<State> {
    let limit = interval_seconds.saturating_mul(STALE_INTERVALS);
    (prev.phase == Phase::Suspect && now.saturating_sub(prev.last_probe) > limit).then(|| State {
        phase: Phase::Up,
        since: now,
        missed: 0,
        down_notified: false,
        pending: None,
        ..prev.clone()
    })
}

// ---------------------------------------------------------------------------
// Delivering a notice
// ---------------------------------------------------------------------------

/// Hands a notice page to the operator.
pub trait Notify {
    /// Shows `page`, or says why it could not.
    ///
    /// # Errors
    ///
    /// The handler could not be started, failed, or did not finish in time.
    fn notify(&mut self, page: &Path) -> Result<(), String>;
}

/// `open PAGE`: the platform's handler shows the page in the default browser.
#[derive(Debug, Default)]
pub struct Opener;

impl Notify for Opener {
    fn notify(&mut self, page: &Path) -> Result<(), String> {
        let mut child = std::process::Command::new("open")
            .arg(page)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|why| format!("`open` could not start: {why}"))?;
        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(status)) => return Err(format!("`open` exited {status}")),
                Ok(None) if started.elapsed() >= OPEN_LIMIT => {
                    let _killed = child.kill();
                    let _reaped = child.wait();
                    return Err(format!(
                        "`open` did not finish within {} s and was stopped",
                        OPEN_LIMIT.as_secs()
                    ));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(why) => return Err(format!("`open` could not be waited on: {why}")),
            }
        }
    }
}

/// Writes the page and the log line, opens nothing: for a machine with no
/// browser, and for tests.
#[derive(Debug, Default)]
pub struct RecordOnly;

impl Notify for RecordOnly {
    fn notify(&mut self, _page: &Path) -> Result<(), String> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// One run: state, probe outcome, log, notice
// ---------------------------------------------------------------------------

/// What one run watches.
#[derive(Debug, Clone)]
pub struct Watch {
    /// The address as the operator wrote it.
    pub url: String,
    /// The watchdog's own directory: state, log, lock, notices.
    pub state_dir: PathBuf,
    /// The store root, looked at when the app does not answer.
    pub store: Option<PathBuf>,
    /// The agent's `StartInterval`: how far apart two probes "in a row" may
    /// be ([`expire_stale`]).
    pub interval_seconds: u64,
}

/// The interval assumed when `--interval-seconds` is not given: the watchdog
/// agent's default `StartInterval`.
pub const DEFAULT_INTERVAL_SECONDS: u64 = 60;

/// How long a run waits for the state directory's lock before it refuses.
/// A run takes milliseconds, so a lock held longer is another run that is
/// stuck, or a second watchdog on the same directory; a lock held for a
/// moment is a child process of this program's own (any spawn duplicates the
/// descriptor until its `exec` closes it), which must not cost a probe
/// (verify-R3-32-r1, blocking 1).
pub const LOCK_WAIT: Duration = Duration::from_secs(1);

/// What one run did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ran {
    /// The state saved.
    pub state: State,
    /// The transition, if the probe changed anything.
    pub transition: Option<Transition>,
    /// The lines appended to `watchdog.log`, in order.
    pub lines: Vec<String>,
    /// The notice page written, if any.
    pub page: Option<PathBuf>,
    /// Why a notice could not be delivered, if one could not.
    pub undelivered: Option<String>,
}

/// Makes the state directory and takes its lock for the length of one run.
fn lock_dir(dir: &Path) -> Result<std::fs::File, String> {
    std::fs::create_dir_all(dir.join(NOTICE_DIR)).map_err(|why| {
        format!(
            "the watchdog's state directory `{}` cannot be made: {why}",
            dir.display()
        )
    })?;
    let lock_path = dir.join(LOCK_FILE);
    let lock = std::fs::File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|why| format!("`{}` cannot be opened: {why}", lock_path.display()))?;
    lock_within(&lock, &lock_path, LOCK_WAIT)?;
    Ok(lock)
}

/// Takes `lock`, waiting at most `wait` for a holder to let go: a short,
/// doubling pause between tries, never a blocking `lock` that could wait for
/// ever. Still held after `wait`: refused by name.
fn lock_within(lock: &std::fs::File, lock_path: &Path, wait: Duration) -> Result<(), String> {
    let started = Instant::now();
    let mut pause = Duration::from_millis(5);
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {
                let waited = started.elapsed();
                if waited >= wait {
                    return Err(format!(
                        "another `cli watchdog` holds `{}` (waited {} ms); this probe recorded nothing",
                        lock_path.display(),
                        waited.as_millis()
                    ));
                }
                std::thread::sleep(pause.min(wait.saturating_sub(waited)));
                pause = pause.saturating_mul(2).min(Duration::from_millis(100));
            }
            Err(std::fs::TryLockError::Error(why)) => {
                return Err(format!("`{}` cannot be locked: {why}", lock_path.display()));
            }
        }
    }
}

/// The lock file's one line, written only by a run that probed and then could
/// not save its state: when it probed. The state's `last_probe` cannot say so
/// (the save is what failed), and without it the stale-suspect rule read the
/// probes of a directory that could not be written as probes that never ran:
/// after two intervals the first miss "expired", the withheld DOWN notice
/// became a `suspect` again, the log said the Mac had slept when it had not,
/// and the notice after the directory was writable again came one probe late.
/// The lock file is the one file every run already holds open for writing,
/// and an existing file stays writable in a directory made read-only.
const ATTEMPT_HEAD: &str = "watchdog-attempt v1";

/// The stamp's fixed length: head, space, 20 digits, newline. One length, so a
/// new stamp overwrites the old one whole at offset 0 and no truncate is needed.
const ATTEMPT_BYTES: usize = ATTEMPT_HEAD.len() + 1 + 20 + 1;

/// Records `now` as a probe whose state could not be saved, in the lock file
/// this run holds. O(1): one 41-byte write at offset 0 and one sync.
fn stamp_attempt(lock: &std::fs::File, now: u64) -> Result<(), String> {
    use std::os::unix::fs::FileExt as _;
    let line = format!("{ATTEMPT_HEAD} {now:020}\n");
    lock.write_all_at(line.as_bytes(), 0)
        .and_then(|()| lock.sync_data())
        .map_err(|why| format!("the probe could not be recorded in `{LOCK_FILE}` either: {why}"))
}

/// What the lock file says about the last probe that could not be saved.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Attempt {
    /// Nothing written: no run has failed to save here since the file was made.
    None,
    /// A probe at this moment ran and could not save its state.
    At(u64),
    /// Something that is not a stamp: named, and read as no stamp.
    Unreadable(String),
}

/// Reads the lock file's stamp: at most [`ATTEMPT_BYTES`] + 1 bytes at offset
/// 0, whatever else the file holds. O(1).
fn read_attempt(lock: &std::fs::File) -> Attempt {
    use std::os::unix::fs::FileExt as _;
    let mut buf = [0_u8; ATTEMPT_BYTES + 1];
    let mut got = 0;
    while got < buf.len() {
        match lock.read_at(
            buf.get_mut(got..).unwrap_or_default(),
            u64::try_from(got).unwrap_or(u64::MAX),
        ) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(why) if why.kind() == std::io::ErrorKind::Interrupted => {}
            Err(why) => return Attempt::Unreadable(format!("it cannot be read: {why}")),
        }
    }
    let bytes = buf.get(..got).unwrap_or_default();
    if bytes.is_empty() {
        return Attempt::None;
    }
    let digits = bytes
        .strip_suffix(b"\n")
        .and_then(|rest| rest.strip_prefix(ATTEMPT_HEAD.as_bytes()))
        .and_then(|rest| rest.strip_prefix(b" "))
        .filter(|digits| digits.len() == 20 && digits.iter().all(u8::is_ascii_digit));
    match digits
        .and_then(|digits| std::str::from_utf8(digits).ok())
        .and_then(|digits| digits.parse::<u64>().ok())
    {
        Some(at) if got == ATTEMPT_BYTES => Attempt::At(at),
        _ => Attempt::Unreadable(format!(
            "its {got} byte(s) are not one `{ATTEMPT_HEAD}` line of {ATTEMPT_BYTES} bytes"
        )),
    }
}

/// The lock file's stamp as this run judges by it: a stamp that is not one is
/// rewritten as the zero stamp (no unsaved probe), so the line that names it
/// is written once, not every minute after. O(1): one bounded read and, only
/// for a damaged stamp, one truncate and one 41-byte write.
fn attempt_of(lock: &std::fs::File) -> Attempt {
    match read_attempt(lock) {
        Attempt::Unreadable(why) => Attempt::Unreadable(
            match lock
                .set_len(0)
                .map_err(|err| err.to_string())
                .and_then(|()| stamp_attempt(lock, 0))
            {
                Ok(()) => format!("{why}; rewritten as no stamp"),
                Err(err) => format!("{why}; and it could not be rewritten: {err}"),
            },
        ),
        other => other,
    }
}

/// What is at the state file's name.
enum StateRead {
    /// Nothing has the name.
    Missing,
    /// A state line, read back.
    Good(State),
    /// Something that is not a state: kept aside, and the run starts fresh.
    Damaged(String),
    /// The host refused the read itself: the run is refused.
    Refused(String),
}

/// Reads the state file: at most [`MAX_STATE_BYTES`] + 1 bytes, through the
/// one read door (`store::safe_open`), so a directory, a pipe, a link, binary
/// bytes or a huge file at the name is damage with a reason, never a run that
/// waits, reads without bound or refuses for ever (verify-R3-32-r1 B4, B7).
fn read_state(path: &Path) -> StateRead {
    let file = match store::safe_open::open_read(path) {
        Ok(file) => file,
        Err(refusal) if refusal.is_missing() => return StateRead::Missing,
        Err(refusal) if refusal.found().is_some() => {
            return StateRead::Damaged(refusal.to_string());
        }
        Err(refusal) => return StateRead::Refused(format!("cannot be read: {refusal}")),
    };
    let mut bytes = Vec::new();
    if let Err(why) = file.take(MAX_STATE_BYTES + 1).read_to_end(&mut bytes) {
        return StateRead::Refused(format!("cannot be read: {why}"));
    }
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_STATE_BYTES {
        return StateRead::Damaged(format!(
            "it is longer than {MAX_STATE_BYTES} bytes, which no state line is"
        ));
    }
    match String::from_utf8(bytes) {
        Err(why) => StateRead::Damaged(format!("it is not UTF-8 text: {why}")),
        Ok(text) => match State::decode(&text) {
            Ok(state) => StateRead::Good(state),
            Err(why) => StateRead::Damaged(why),
        },
    }
}

/// The saved state; a damaged one is kept aside by name, never deleted, and
/// the run starts fresh with a line saying so. Only a host refusal to read
/// (not damage) refuses the run.
fn load_state(dir: &Path, now: u64, lines: &mut Vec<String>) -> Result<State, String> {
    let state_path = dir.join(STATE_FILE);
    match read_state(&state_path) {
        StateRead::Good(state) => Ok(state),
        StateRead::Missing => Ok(State::fresh()),
        StateRead::Refused(why) => Err(format!("`{}` {why}", state_path.display())),
        StateRead::Damaged(why) => {
            let kept = dir.join(format!("{STATE_FILE}.unreadable-{now}"));
            std::fs::rename(&state_path, &kept).map_err(|err| {
                format!(
                    "`{}` is unreadable ({why}) and could not be kept aside: {err}",
                    state_path.display()
                )
            })?;
            lines.push(format!(
                "{} state-unreadable why=\"{}\" kept={} starting=fresh",
                utc(now),
                why.escape_debug(),
                relative(dir, &kept)
            ));
            Ok(State::fresh())
        }
    }
}

/// One run of the watchdog over a probe outcome already taken at `now`.
///
/// # The order, and why it is this one
///
/// A notice is SAVED BEFORE IT IS SENT: the transition's line is appended,
/// then the state is saved with [`State::pending`] naming the notice, and
/// only then is the page written and handed to `open`; the answer is a second
/// line and a second save. So a state that cannot be saved withholds the
/// notice (one `notice-withheld` line, the run refused) instead of sending it
/// again every minute, and a run killed while `open` runs leaves `pending`
/// behind, which the next run names (`notify-unknown`) and counts as sent.
/// At most one notice per outage holds through both (verify-R3-32-r1 B3, B5).
///
/// # Errors
///
/// The state directory cannot be made or written, another run holds its
/// lock past [`LOCK_WAIT`], or the log or state cannot be saved. A notice
/// that `open` could not deliver is NOT an error here: it is in
/// [`Ran::undelivered`] and the log.
pub fn watch_once(
    watch: &Watch,
    now: u64,
    outcome: &Outcome,
    notify: &mut dyn Notify,
) -> Result<Ran, String> {
    let dir = &watch.state_dir;
    let held = lock_dir(dir)?;
    let log = dir.join(LOG_FILE);
    let mut lines = Vec::new();
    let prev = load_state(dir, now, &mut lines)?;
    let prev = judged_against(prev, attempt_of(&held), watch, now, &mut lines);

    let (mut next, transition) = step(&prev, outcome, now);
    let store_word = match outcome {
        Outcome::NoAnswer { .. } => watch.store.as_deref().map(store_state),
        Outcome::Answered { .. } => None,
    };
    let mut page = None;
    let mut undelivered = None;
    let mut appended = 0;
    if let Some(transition) = &transition {
        lines.push(transition_line(
            watch,
            &prev,
            outcome,
            transition,
            now,
            store_word.as_deref(),
        ));
        if let Some(notice) = transition.notice() {
            append_lines(&log, &lines)?;
            appended = lines.len();
            let claimed = State {
                pending: Some(notice),
                ..next.clone()
            };
            let saved = save_state(dir, &claimed);
            if let Err(why) = &saved {
                return Err(withhold(&log, now, notice, &unsaved(&held, now, why)));
            }
            cleared_line(&log, now, saved.ok().flatten(), &mut lines, &mut appended)?;
            let delivered = write_page(
                dir,
                notice,
                watch,
                &prev,
                outcome,
                transition,
                now,
                store_word.as_deref(),
            )
            .and_then(|written| {
                let answer = notify.notify(&written);
                page = Some(written);
                answer
            });
            let shown = page
                .as_deref()
                .map_or_else(|| "-".to_owned(), |written| relative(dir, written));
            match delivered {
                Ok(()) => {
                    lines.push(format!(
                        "{} notified notice={} page={shown}",
                        utc(now),
                        notice_word(notice),
                    ));
                }
                Err(why) => {
                    if notice == Notice::Down {
                        next.down_notified = false;
                    }
                    lines.push(format!(
                        "{} notify-failed notice={} page={shown} why=\"{}\"",
                        utc(now),
                        notice_word(notice),
                        why.escape_debug()
                    ));
                    undelivered = Some(why);
                }
            }
        }
    }

    append_lines(&log, lines.get(appended..).unwrap_or_default())?;
    appended = lines.len();
    let cleared = save_state(dir, &next).map_err(|why| unsaved(&held, now, &why))?;
    cleared_line(&log, now, cleared, &mut lines, &mut appended)?;
    if let Some(transition) = &transition {
        note_change(watch, outcome, transition, &next);
    }
    Ok(Ran {
        state: next,
        transition,
        lines,
        page,
        undelivered,
    })
}

/// A save that failed: the probe is stamped in the lock file
/// ([`stamp_attempt`]) so the next run knows it ran, and the refusal names
/// both failures when the stamp fails too.
fn unsaved(lock: &std::fs::File, now: u64, why: &str) -> String {
    match stamp_attempt(lock, now) {
        Ok(()) => why.to_owned(),
        Err(also) => format!("{why}; {also}"),
    }
}

/// The state a probe is judged against: the saved one, with a notice a dead
/// run left `pending` named (`notify-unknown`, counted as sent), a clock
/// stepped back named, and a stale suspect expired ([`expire_stale`]) — stale
/// counted from the later of the state's last probe and the lock file's
/// stamp of a probe that ran but could not be saved ([`stamp_attempt`]).
fn judged_against(
    mut prev: State,
    attempt: Attempt,
    watch: &Watch,
    now: u64,
    lines: &mut Vec<String>,
) -> State {
    if let Some(notice) = prev.pending.take() {
        lines.push(format!(
            "{} notify-unknown notice={} sent_at={} why=\"the run that handed this notice to `open` ended before it saved the answer; it is counted as sent and is not sent again\"",
            utc(now),
            notice_word(notice),
            utc(prev.last_probe)
        ));
    }
    if prev.phase != Phase::Fresh && now < prev.last_probe {
        lines.push(format!(
            "{} clock-back last_probe={} now={} by_seconds={}",
            utc(now),
            utc(prev.last_probe),
            utc(now),
            prev.last_probe - now
        ));
    }
    let stamped = match attempt {
        Attempt::None => None,
        Attempt::At(at) => Some(at),
        Attempt::Unreadable(why) => {
            lines.push(format!(
                "{} attempt-stamp-unreadable why=\"{}\" read_as=none",
                utc(now),
                why.escape_debug()
            ));
            None
        }
    };
    // A stamp later than the saved probe and not in the future: probes ran
    // since that miss and could not be saved, so none was skipped.
    let unsaved_probe = stamped.filter(|at| *at > prev.last_probe && *at <= now);
    let seen = State {
        last_probe: unsaved_probe.unwrap_or(prev.last_probe),
        ..prev.clone()
    };
    if let (Some(at), Phase::Suspect) = (unsaved_probe, prev.phase)
        && expire_stale(&prev, now, watch.interval_seconds).is_some()
        && expire_stale(&seen, now, watch.interval_seconds).is_none()
    {
        lines.push(format!(
            "{} suspect-kept last_probe={} unsaved_probe={} why=\"probes ran since that miss and could not save their state, so no probe was skipped and the miss still counts\"",
            utc(now),
            utc(prev.last_probe),
            utc(at)
        ));
    }
    if let Some(expired) = expire_stale(&seen, now, watch.interval_seconds) {
        lines.push(format!(
            "{} suspect-expired last_probe={} gap_seconds={} limit_seconds={} why=\"a probe was skipped since that miss (the Mac slept or was off), so it is not half of an outage\"",
            utc(now),
            utc(seen.last_probe),
            now.saturating_sub(seen.last_probe),
            watch.interval_seconds.saturating_mul(STALE_INTERVALS)
        ));
        prev = expired;
    }

    prev
}

/// A notice whose state could not be saved first: one `notice-withheld`
/// line (if the log still takes one) and the run's refusal, naming both.
fn withhold(log: &Path, now: u64, notice: Notice, why: &str) -> String {
    let withheld = format!(
        "{} notice-withheld notice={} why=\"the state could not be saved before the notice, and a notice whose sending is not remembered would go out again every minute: {}\"",
        utc(now),
        notice_word(notice),
        why.escape_debug()
    );
    let logged = append_lines(log, std::slice::from_ref(&withheld))
        .err()
        .map_or_else(String::new, |err| {
            format!("; and the log refused that line: {err}")
        });
    format!(
        "the {} notice was withheld, not sent: {why}{logged}",
        notice_word(notice)
    )
}

/// The one telemetry event of a run that changed something.
fn note_change(watch: &Watch, outcome: &Outcome, transition: &Transition, next: &State) {
    crate::note(
        &telemetry::Event::new(
            if outcome.answered() {
                telemetry::Level::Info
            } else {
                telemetry::Level::Warn
            },
            "cli.watchdog",
            "the app's /health changed state",
        )
        .with("transition", transition.word())
        .with("url", watch.url.as_str())
        .with("missed", u64::from(next.missed)),
    );
}

/// The store root's state, in words, for a probe that was not answered.
#[must_use]
pub fn store_state(store: &Path) -> String {
    match std::fs::metadata(store) {
        Ok(meta) if meta.is_dir() => "mounted".to_owned(),
        Ok(_) => "not-a-directory".to_owned(),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => "absent".to_owned(),
        Err(why) if why.kind() == std::io::ErrorKind::PermissionDenied => "denied".to_owned(),
        Err(why) => format!("unreadable({})", why.kind()),
    }
}

fn store_sentence(word: &str) -> &'static str {
    match word {
        "mounted" => "The store drive is mounted: the app itself is not answering.",
        "absent" => "The store root does not exist: is the drive mounted?",
        "denied" => {
            "The store root exists but this program may not read it: macOS may be blocking access to the drive."
        }
        "not-a-directory" => "The store root is not a directory.",
        _ => "The store root could not be inspected.",
    }
}

fn notice_word(notice: Notice) -> &'static str {
    match notice {
        Notice::Down => "down",
        Notice::Back => "back",
    }
}

fn relative(dir: &Path, path: &Path) -> String {
    path.strip_prefix(dir).unwrap_or(path).display().to_string()
}

fn transition_line(
    watch: &Watch,
    prev: &State,
    outcome: &Outcome,
    transition: &Transition,
    now: u64,
    store: Option<&str>,
) -> String {
    let mut line = format!("{} {} url={}", utc(now), transition.word(), watch.url);
    match outcome {
        Outcome::Answered { status, millis } => {
            let _ = write!(line, " status={status} ms={millis}");
        }
        Outcome::NoAnswer { why, millis } => {
            let _ = write!(line, " ms={millis} why=\"{}\"", why.escape_debug());
        }
    }
    match transition {
        Transition::Blip { missed } => {
            let _ = write!(line, " missed={missed}");
        }
        Transition::Back {
            down_seconds,
            missed,
        }
        | Transition::BackUnnotified {
            down_seconds,
            missed,
        } => {
            let _ = write!(
                line,
                " missed={missed} down_seconds={down_seconds} since={}",
                utc(prev.since)
            );
        }
        Transition::Down | Transition::DownAgain => {
            let _ = write!(
                line,
                " missed={} since={} last_answer={}",
                prev.missed.saturating_add(1),
                utc(
                    if prev.phase == Phase::Suspect || prev.phase == Phase::Down {
                        prev.since
                    } else {
                        now
                    }
                ),
                prev.last_answer.map_or_else(|| "never".to_owned(), utc)
            );
        }
        Transition::Watching | Transition::Suspect => {}
    }
    if let Some(store) = store {
        let _ = write!(line, " store={store}");
    }
    line
}

#[allow(
    clippy::too_many_arguments,
    reason = "one page from the one run's facts; a struct would only rename them"
)]
fn write_page(
    dir: &Path,
    notice: Notice,
    watch: &Watch,
    prev: &State,
    outcome: &Outcome,
    transition: &Transition,
    now: u64,
    store: Option<&str>,
) -> Result<PathBuf, String> {
    let (title, colour) = match notice {
        Notice::Down => ("brutex app is DOWN", "#b00020"),
        Notice::Back => ("brutex app is back", "#1b5e20"),
    };
    let mut body = String::new();
    let _ = write!(
        body,
        "<p>{} asked at {} ({}).</p>",
        escape_html(&watch.url),
        ist(now),
        utc(now)
    );
    match (notice, outcome, transition) {
        (Notice::Down, Outcome::NoAnswer { why, .. }, _) => {
            let since = if prev.phase == Phase::Suspect || prev.phase == Phase::Down {
                prev.since
            } else {
                now
            };
            let _ = write!(
                body,
                "<p>No answer to {} probes in a row, the first at {}. Last answer: {}.</p><p>Why: {}</p>",
                prev.missed.saturating_add(1),
                ist(since),
                prev.last_answer.map_or_else(|| "never".to_owned(), ist),
                escape_html(why)
            );
            if let Some(store) = store {
                let _ = write!(body, "<p>{}</p>", store_sentence(store));
            }
            body.push_str(
                "<p>launchd starts the app again while the store drive is mounted. This page is the one notice of this outage; another comes when the app answers.</p>",
            );
        }
        (
            Notice::Back,
            Outcome::Answered { status, .. },
            Transition::Back {
                down_seconds,
                missed,
            },
        ) => {
            let _ = write!(
                body,
                "<p>The app answers again (HTTP {status}) after {missed} unanswered probes; the first unanswered probe was {down_seconds} s ago, at {}.</p>",
                ist(prev.since)
            );
        }
        _ => {}
    }
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{title}</title></head><body style=\"font-family:system-ui,sans-serif;margin:2rem;max-width:40rem\"><h1 style=\"color:{colour}\">{title}</h1>{body}<p style=\"color:#555\">Written by <code>cli watchdog</code>; every probe that changed anything is a line of {}.</p></body></html>\n",
        escape_html(&dir.join(LOG_FILE).display().to_string())
    );
    let notices = dir.join(NOTICE_DIR);
    let stem = format!("{}-{}", compact_utc(now), notice_word(notice));
    for attempt in 0..16_u32 {
        let name = if attempt == 0 {
            format!("{stem}.html")
        } else {
            format!("{stem}-{attempt}.html")
        };
        let path = notices.join(name);
        match std::fs::File::options()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(html.as_bytes())
                    .and_then(|()| file.sync_all())
                    .map_err(|why| format!("`{}` could not be written: {why}", path.display()))?;
                return Ok(path);
            }
            Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(why) => {
                return Err(format!("`{}` could not be made: {why}", path.display()));
            }
        }
    }
    Err(format!(
        "sixteen notice pages named `{stem}` already exist in `{}`",
        notices.display()
    ))
}

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn append_lines(path: &Path, lines: &[String]) -> Result<(), String> {
    if lines.is_empty() {
        return Ok(());
    }
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    let mut file = std::fs::File::options()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|why| format!("`{}` cannot be opened to append: {why}", path.display()))?;
    file.write_all(text.as_bytes())
        .and_then(|()| file.sync_data())
        .map_err(|why| format!("`{}` could not be appended to: {why}", path.display()))
}

/// Saves `state`: a temporary born through `store::staging::create` (gate 28,
/// R3-16: it opens nothing already at the name, so a planted second link of
/// another file is refused, not truncated), synced, then renamed over the
/// state. The temporary's name is this directory's alone because the run
/// holds the directory's lock across the create and the rename, the staging
/// door's precondition.
///
/// Returns the bytes of a dead run's leftover temporary whose name the door
/// removed first, which the caller logs ([`cleared_line`]).
fn save_state(dir: &Path, state: &State) -> Result<Option<u64>, String> {
    let tmp = dir.join(format!("{STATE_FILE}.tmp"));
    let path = dir.join(STATE_FILE);
    let staged = store::staging::create(&tmp)
        .map_err(|why| format!("`{}` cannot be made: {why}", tmp.display()))?;
    let cleared = staged.cleared;
    let mut file = staged.file;
    file.write_all(state.encode().as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|why| format!("`{}` could not be written: {why}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|why| {
        format!(
            "`{}` could not replace `{}`: {why}",
            tmp.display(),
            path.display()
        )
    })?;
    Ok(cleared)
}

/// Logs a leftover temporary the staging door cleared, at once: the staging
/// door does not log, its caller does (`store::staging`).
fn cleared_line(
    log: &Path,
    now: u64,
    cleared: Option<u64>,
    lines: &mut Vec<String>,
    appended: &mut usize,
) -> Result<(), String> {
    if let Some(bytes) = cleared {
        lines.push(format!(
            "{} state-tmp-cleared bytes={bytes} why=\"a run that ended between writing its state and renaming it left its temporary; that name was removed and no other file changed\"",
            utc(now)
        ));
        append_lines(log, lines.get(*appended..).unwrap_or_default())?;
        *appended = lines.len();
    }
    Ok(())
}

fn civil(unix: u64, offset: i64) -> (i64, i64, i64, i64, i64, i64) {
    let seconds = i64::try_from(unix)
        .unwrap_or(i64::MAX)
        .saturating_add(offset);
    let days = seconds.div_euclid(86_400);
    let of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = telemetry::civil_from_days(days);
    (
        year,
        month,
        day,
        of_day / 3_600,
        of_day % 3_600 / 60,
        of_day % 60,
    )
}

fn utc(unix: u64) -> String {
    let (year, month, day, hour, minute, second) = civil(unix, 0);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn compact_utc(unix: u64) -> String {
    let (year, month, day, hour, minute, second) = civil(unix, 0);
    format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z")
}

fn ist(unix: u64) -> String {
    let (year, month, day, hour, minute, second) = civil(unix, pull::session::IST_OFFSET_SECS);
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} IST")
}

// ---------------------------------------------------------------------------
// `cli watchdog`
// ---------------------------------------------------------------------------

/// `cli watchdog URL --state-dir DIR [--store DIR] [--timeout-ms N]
/// [--notify open|record]`: one probe, its line, its notice.
///
/// Exits 0 when the app answered, 1 when it did not or when the run itself
/// could not record what it found, 2 on a misuse.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    let parsed = match parse_watch(arguments) {
        Ok(parsed) => parsed,
        Err(why) => return misuse(out, &why, WATCH_USAGE),
    };
    let (watch, target, limit, opens) = parsed;
    let outcome = probe(&target, limit);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let ran = if opens {
        watch_once(&watch, now, &outcome, &mut Opener)
    } else {
        watch_once(&watch, now, &outcome, &mut RecordOnly)
    };
    report(out, &watch, &outcome, ran)
}

fn report(out: &mut String, watch: &Watch, outcome: &Outcome, ran: Result<Ran, String>) -> u8 {
    let ran = match ran {
        Ok(ran) => ran,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}");
            return FAILED;
        }
    };
    match outcome {
        Outcome::Answered { status, millis } => {
            let _ = write!(
                out,
                "watchdog: {} answered {status} in {millis} ms",
                watch.url
            );
        }
        Outcome::NoAnswer { why, millis } => {
            let _ = write!(
                out,
                "watchdog: {} gave no answer in {millis} ms ({why}); {} in a row",
                watch.url, ran.state.missed
            );
        }
    }
    if let Some(transition) = &ran.transition {
        let _ = write!(out, "; {}", transition.word());
    }
    out.push('\n');
    if let Some(why) = &ran.undelivered {
        let _ = writeln!(out, "the notice was NOT delivered: {why}");
    }
    if outcome.answered() && ran.undelivered.is_none() {
        OK
    } else {
        FAILED
    }
}

fn misuse(out: &mut String, why: &str, usage: &str) -> u8 {
    let _ = writeln!(out, "refused: {why}\n{usage}");
    MISUSED
}

/// Whether a directory is off the store drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drive {
    /// On another device than the store root: what the agents need.
    Other,
    /// No store root was named: nothing to be off.
    NoStore,
    /// The store root could not be looked at (not mounted, or not readable),
    /// so the drive was NOT compared; the reason, in words.
    Unchecked(String),
}

/// The device a path is on, read at the path or, when it does not exist yet,
/// at its nearest existing ancestor, and that ancestor. One `stat` per
/// missing component: bounded by the path's depth, never by a directory's
/// size. A link anywhere on the way is followed, so a symbolic link into the
/// drive, a `..` back into it and another spelling on a case-insensitive
/// volume all come out as the drive's own device (verify-R3-32-r1 A1-A5).
///
/// # Errors
///
/// A component the host refuses to look at for any reason but absence.
pub fn device_of(path: &Path) -> Result<(u64, PathBuf), String> {
    use std::os::unix::fs::MetadataExt as _;
    for ancestor in path.ancestors() {
        match std::fs::metadata(ancestor) {
            Ok(meta) => return Ok((meta.dev(), ancestor.to_path_buf())),
            Err(why)
                if matches!(
                    why.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) => {}
            Err(why) => {
                return Err(format!(
                    "`{}` cannot be looked at: {why}",
                    ancestor.display()
                ));
            }
        }
    }
    Err(format!("no part of `{}` exists", path.display()))
}

/// Refuses `dir` (named by `flag`) on the store drive, which an unmounted
/// drive would take away from the watchdog or launchd log that should report
/// it: first by name (inside the store root as written, which needs no
/// drive), then by DEVICE when the store root can be looked at — the same
/// `st_dev` as the store root's is the same drive, however the path is
/// spelt. A store root that cannot be looked at leaves the device unchecked,
/// and the answer says so ([`Drive::Unchecked`]); each caller decides what
/// that costs.
///
/// # Errors
///
/// `dir` is inside the store root, on the store root's device, or cannot be
/// looked at.
pub fn off_store(flag: &str, dir: &Path, store: Option<&Path>) -> Result<Drive, String> {
    use std::os::unix::fs::MetadataExt as _;
    let Some(store) = store else {
        return Ok(Drive::NoStore);
    };
    let cost = if flag == "--log-dir" {
        "launchd could not write why the app stopped while the drive is away"
    } else {
        "an unmounted drive would silence the watchdog that should report it"
    };
    if dir.starts_with(store) {
        return Err(format!(
            "{flag} `{}` is inside the store root: {cost}",
            dir.display()
        ));
    }
    let store_dev = match std::fs::metadata(store) {
        Ok(meta) => meta.dev(),
        Err(why) => {
            return Ok(Drive::Unchecked(format!(
                "the store root `{}` could not be looked at ({why}), so whether {flag} `{}` is on another drive was NOT checked; render again with the drive mounted",
                store.display(),
                dir.display()
            )));
        }
    };
    let (dev, at) = device_of(dir)?;
    if dev == store_dev {
        return Err(format!(
            "{flag} `{}` is on the store drive (device {dev}, read at `{}`, is the store root's): {cost}",
            dir.display(),
            at.display()
        ));
    }
    Ok(Drive::Other)
}

fn absolute(flag: &str, text: &str) -> Result<PathBuf, String> {
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(format!(
            "{flag} `{}` is empty or holds a control character",
            text.escape_debug()
        ));
    }
    let path = PathBuf::from(text);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(format!(
            "{flag} `{text}` is not an absolute path; launchd starts jobs with no working directory of yours"
        ))
    }
}

/// A whole number from `low` to `high`, written in digits alone: `u64::from_str`
/// also takes a leading `+`, and a leading zero, spellings no operator writes on
/// purpose, so both are refused here as the URL's port is.
fn bounded(flag: &str, text: &str, low: u64, high: u64) -> Result<u64, String> {
    let plain = !text.is_empty()
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    plain
        .then(|| text.parse::<u64>().ok())
        .flatten()
        .filter(|n| (low..=high).contains(n))
        .ok_or_else(|| {
            format!(
                "{flag} `{}` is not a whole number from {low} to {high} (digits only, no sign, no leading zero)",
                text.escape_debug()
            )
        })
}

/// Pairs `--flag VALUE` words; a bare flag in `switches` stands alone.
fn flags<'a>(
    words: &[&'a str],
    known: &[&str],
    switches: &[&str],
) -> Result<Vec<(&'a str, Option<&'a str>)>, String> {
    let mut out = Vec::new();
    let mut rest = words.iter();
    while let Some(&word) = rest.next() {
        if switches.contains(&word) {
            if out.iter().any(|(seen, _)| *seen == word) {
                return Err(format!("{word} is given twice"));
            }
            out.push((word, None));
        } else if known.contains(&word) {
            let value = rest.next().ok_or_else(|| format!("{word} needs a value"))?;
            if word != "--env" && out.iter().any(|(seen, _)| *seen == word) {
                return Err(format!("{word} is given twice"));
            }
            out.push((word, Some(*value)));
        } else {
            return Err(format!("`{word}` is not a flag this verb takes"));
        }
    }
    Ok(out)
}

fn value_of<'a>(pairs: &[(&'a str, Option<&'a str>)], flag: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(seen, _)| *seen == flag)
        .and_then(|(_, value)| *value)
}

fn parse_watch(arguments: &[&str]) -> Result<(Watch, Target, Duration, bool), String> {
    let (url, rest) = arguments
        .split_first()
        .ok_or("the URL of /health is missing")?;
    let target = Target::parse(url)?;
    let pairs = flags(
        rest,
        &[
            "--state-dir",
            "--store",
            "--timeout-ms",
            "--interval-seconds",
            "--notify",
        ],
        &[],
    )?;
    let state_dir = absolute(
        "--state-dir",
        value_of(&pairs, "--state-dir")
            .ok_or("--state-dir is required: the watchdog's memory between probes")?,
    )?;
    let store = value_of(&pairs, "--store")
        .map(|text| absolute("--store", text))
        .transpose()?;
    // A store root that cannot be looked at leaves the device unchecked and
    // the probe runs: an unmounted or blocked drive is what it reports.
    off_store("--state-dir", &state_dir, store.as_deref())?;
    let interval_seconds = interval_of(&pairs)?;
    let limit = match value_of(&pairs, "--timeout-ms") {
        Some(text) => Duration::from_millis(bounded("--timeout-ms", text, 1, 60_000)?),
        None => PROBE_LIMIT,
    };
    let opens = match value_of(&pairs, "--notify") {
        None | Some("open") => true,
        Some("record") => false,
        Some(other) => {
            return Err(format!("--notify `{other}` is neither `open` nor `record`"));
        }
    };
    Ok((
        Watch {
            url: (*url).to_owned(),
            state_dir,
            store,
            interval_seconds,
        },
        target,
        limit,
        opens,
    ))
}

// ---------------------------------------------------------------------------
// `cli launch-agent`
// ---------------------------------------------------------------------------

/// The app agent's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppAgent {
    /// The `api` binary.
    pub api: PathBuf,
    /// `ip:port` to serve on.
    pub addr: SocketAddr,
    /// The store root: `BRUTEX_STORE`, and the path launchd waits for.
    pub store: PathBuf,
    /// Where launchd writes the app's output, and its working directory.
    pub log_dir: PathBuf,
    /// More environment, in the order given.
    pub env: Vec<(String, String)>,
    /// `ThrottleInterval`, seconds.
    pub throttle_seconds: u64,
    /// The soft open-file limit (FM-088: launchd's own is 256).
    pub max_files: u64,
    /// Run under `/usr/bin/caffeinate -i`, as the deploy script does today.
    pub keep_awake: bool,
    /// Why `--log-dir` could not be proved off the store drive, if it could
    /// not (the store root was not there to compare with): said in the
    /// agent file itself.
    pub unchecked: Option<String>,
}

/// The watchdog agent's settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchdogAgent {
    /// The `cli` binary.
    pub cli: PathBuf,
    /// The `/health` address.
    pub url: String,
    /// The watchdog's state directory, off the store drive.
    pub state_dir: PathBuf,
    /// The store root, named in a notice when the app does not answer.
    pub store: Option<PathBuf>,
    /// `StartInterval`, seconds.
    pub interval_seconds: u64,
    /// Why `--state-dir` could not be proved off the store drive, if it
    /// could not: said in the agent file itself.
    pub unchecked: Option<String>,
}

/// The environment the app agent sets itself, which `--env` may not repeat.
const OWN_ENV: [&str; 2] = ["BRUTEX_NO_OPEN", "BRUTEX_STORE"];

/// `cli launch-agent app|watchdog ...`: prints one agent file. Writes nothing.
pub(crate) fn launch_agent(arguments: &[&str], out: &mut String) -> u8 {
    let rendered = match arguments.split_first() {
        Some((&"app", rest)) => parse_app(rest).map(|agent| render_app(&agent)),
        Some((&"watchdog", rest)) => parse_watchdog(rest).map(|agent| render_watchdog(&agent)),
        Some((other, _)) => Err(format!("`{other}` is not an agent: `app` or `watchdog`")),
        None => Err("which agent: `app` or `watchdog`".to_owned()),
    };
    match rendered {
        Ok(text) => {
            out.push_str(&text);
            OK
        }
        Err(why) => misuse(out, &why, AGENT_USAGE),
    }
}

fn existing_file(flag: &str, text: &str) -> Result<PathBuf, String> {
    let path = absolute(flag, text)?;
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("{flag} `{text}` is not a file on this machine"))
    }
}

fn parse_app(arguments: &[&str]) -> Result<AppAgent, String> {
    let [api, addr, rest @ ..] = arguments else {
        return Err("the app agent needs API and ADDR".to_owned());
    };
    let api = existing_file("API", api)?;
    let addr: SocketAddr = addr
        .parse()
        .ok()
        .filter(|addr: &SocketAddr| addr.port() != 0)
        .ok_or_else(|| format!("ADDR `{addr}` is not IP:PORT with a port from 1 to 65535"))?;
    let pairs = flags(
        rest,
        &[
            "--store",
            "--log-dir",
            "--env",
            "--throttle-seconds",
            "--max-files",
        ],
        &["--keep-awake"],
    )?;
    let store = absolute(
        "--store",
        value_of(&pairs, "--store").ok_or("--store is required: the path launchd waits for")?,
    )?;
    let log_dir = absolute(
        "--log-dir",
        value_of(&pairs, "--log-dir")
            .ok_or("--log-dir is required: where launchd writes the app's output")?,
    )?;
    if !log_dir.is_dir() {
        return Err(format!(
            "--log-dir `{}` is not a directory; make it first",
            log_dir.display()
        ));
    }
    let drive = off_store("--log-dir", &log_dir, Some(&store))?;
    let mut env: Vec<(String, String)> = Vec::new();
    for (_, value) in pairs.iter().filter(|(flag, _)| *flag == "--env") {
        let pair = value.unwrap_or_default();
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("--env `{pair}` is not KEY=VALUE"))?;
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(format!("--env key `{key}` is not A-Z, 0-9 and _"));
        }
        if OWN_ENV.contains(&key) {
            return Err(format!(
                "--env `{key}` is set by the agent itself (BRUTEX_STORE from --store; BRUTEX_NO_OPEN=1 so a restart opens no browser)"
            ));
        }
        if value.chars().any(char::is_control) {
            return Err(format!("--env `{key}`'s value holds a control character"));
        }
        if env.iter().any(|(seen, _)| seen == key) {
            return Err(format!("--env `{key}` is given twice"));
        }
        env.push((key.to_owned(), value.to_owned()));
    }
    let throttle_seconds = value_of(&pairs, "--throttle-seconds")
        .map_or(Ok(10), |text| bounded("--throttle-seconds", text, 10, 600))?;
    let max_files = value_of(&pairs, "--max-files").map_or(Ok(65_536), |text| {
        bounded("--max-files", text, 256, 1_048_576)
    })?;
    Ok(AppAgent {
        api,
        addr,
        store,
        log_dir,
        env,
        throttle_seconds,
        max_files,
        keep_awake: pairs.iter().any(|(flag, _)| *flag == "--keep-awake"),
        unchecked: unchecked_of(drive),
    })
}

fn parse_watchdog(arguments: &[&str]) -> Result<WatchdogAgent, String> {
    let [cli, url, rest @ ..] = arguments else {
        return Err("the watchdog agent needs CLI and URL".to_owned());
    };
    let cli = existing_file("CLI", cli)?;
    Target::parse(url)?;
    let pairs = flags(rest, &["--state-dir", "--store", "--interval-seconds"], &[])?;
    let state_dir = absolute(
        "--state-dir",
        value_of(&pairs, "--state-dir")
            .ok_or("--state-dir is required: the watchdog's memory between probes")?,
    )?;
    let store = value_of(&pairs, "--store")
        .map(|text| absolute("--store", text))
        .transpose()?;
    let drive = off_store("--state-dir", &state_dir, store.as_deref())?;
    let interval_seconds = interval_of(&pairs)?;
    Ok(WatchdogAgent {
        cli,
        url: (*url).to_owned(),
        state_dir,
        store,
        interval_seconds,
        unchecked: unchecked_of(drive),
    })
}

/// `--interval-seconds`, from 10 to 3600, or [`DEFAULT_INTERVAL_SECONDS`]:
/// one rule for the agent's `StartInterval` and the verb that runs under it.
fn interval_of(pairs: &[(&str, Option<&str>)]) -> Result<u64, String> {
    value_of(pairs, "--interval-seconds").map_or(Ok(DEFAULT_INTERVAL_SECONDS), |text| {
        bounded("--interval-seconds", text, 10, 3_600)
    })
}

fn unchecked_of(drive: Drive) -> Option<String> {
    match drive {
        Drive::Unchecked(why) => Some(why),
        Drive::Other | Drive::NoStore => None,
    }
}

/// An XML comment that says `text`. A comment may not hold `--`, so runs of
/// dashes lose two at a time (`--state-dir` reads `state-dir`).
fn xml_comment(text: &str) -> String {
    let mut said = text.to_owned();
    while said.contains("--") {
        said = said.replace("--", "");
    }
    format!("\t<!-- NOT CHECKED: {said} -->\n")
}

fn xml(text: &str) -> String {
    escape_html(text)
}

fn plist(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n{body}</dict>\n</plist>\n"
    )
}

fn key_string(out: &mut String, key: &str, value: &str) {
    let _ = writeln!(
        out,
        "\t<key>{}</key>\n\t<string>{}</string>",
        xml(key),
        xml(value)
    );
}

fn key_integer(out: &mut String, key: &str, value: u64) {
    let _ = writeln!(
        out,
        "\t<key>{}</key>\n\t<integer>{value}</integer>",
        xml(key)
    );
}

fn key_array(out: &mut String, key: &str, values: &[String]) {
    let _ = writeln!(out, "\t<key>{}</key>\n\t<array>", xml(key));
    for value in values {
        let _ = writeln!(out, "\t\t<string>{}</string>", xml(value));
    }
    out.push_str("\t</array>\n");
}

fn key_dict(out: &mut String, key: &str, pairs: &[(String, String)]) {
    let _ = writeln!(out, "\t<key>{}</key>\n\t<dict>", xml(key));
    for (k, v) in pairs {
        let _ = writeln!(
            out,
            "\t\t<key>{}</key>\n\t\t<string>{}</string>",
            xml(k),
            xml(v)
        );
    }
    out.push_str("\t</dict>\n");
}

fn shown(path: &Path) -> String {
    path.display().to_string()
}

/// The `in.brutex.app` agent file.
#[must_use]
pub fn render_app(agent: &AppAgent) -> String {
    let mut body = String::new();
    if let Some(why) = &agent.unchecked {
        body.push_str(&xml_comment(why));
    }
    key_string(&mut body, "Label", APP_LABEL);
    let mut program = Vec::new();
    if agent.keep_awake {
        program.push("/usr/bin/caffeinate".to_owned());
        program.push("-i".to_owned());
    }
    program.push(shown(&agent.api));
    program.push("serve".to_owned());
    program.push(agent.addr.to_string());
    key_array(&mut body, "ProgramArguments", &program);
    let mut env = vec![
        ("BRUTEX_STORE".to_owned(), shown(&agent.store)),
        ("BRUTEX_NO_OPEN".to_owned(), "1".to_owned()),
    ];
    env.extend(agent.env.iter().cloned());
    key_dict(&mut body, "EnvironmentVariables", &env);
    // KEPT ALIVE WHILE THE STORE ROOT EXISTS: started when the drive mounts,
    // started again when the app exits, and not started while it is away.
    let _ = writeln!(
        body,
        "\t<key>KeepAlive</key>\n\t<dict>\n\t\t<key>PathState</key>\n\t\t<dict>\n\t\t\t<key>{}</key>\n\t\t\t<true/>\n\t\t</dict>\n\t</dict>",
        xml(&shown(&agent.store))
    );
    key_integer(&mut body, "ThrottleInterval", agent.throttle_seconds);
    key_string(&mut body, "ProcessType", "Interactive");
    let _ = writeln!(
        body,
        "\t<key>SoftResourceLimits</key>\n\t<dict>\n\t\t<key>NumberOfFiles</key>\n\t\t<integer>{}</integer>\n\t</dict>",
        agent.max_files
    );
    key_string(&mut body, "WorkingDirectory", &shown(&agent.log_dir));
    let log = shown(&agent.log_dir.join("app.log"));
    key_string(&mut body, "StandardOutPath", &log);
    key_string(&mut body, "StandardErrorPath", &log);
    plist(&body)
}

/// The `in.brutex.watchdog` agent file.
#[must_use]
pub fn render_watchdog(agent: &WatchdogAgent) -> String {
    let mut body = String::new();
    if let Some(why) = &agent.unchecked {
        body.push_str(&xml_comment(why));
    }
    key_string(&mut body, "Label", WATCHDOG_LABEL);
    let mut program = vec![
        shown(&agent.cli),
        "watchdog".to_owned(),
        agent.url.clone(),
        "--state-dir".to_owned(),
        shown(&agent.state_dir),
    ];
    if let Some(store) = &agent.store {
        program.push("--store".to_owned());
        program.push(shown(store));
    }
    // The verb is told the interval it runs at, so two probes further apart
    // than two intervals are not "in a row" (`expire_stale`).
    program.push("--interval-seconds".to_owned());
    program.push(agent.interval_seconds.to_string());
    key_array(&mut body, "ProgramArguments", &program);
    key_dict(
        &mut body,
        "EnvironmentVariables",
        &[(
            "BRUTEX_LOG_DIR".to_owned(),
            shown(&agent.state_dir.join("events")),
        )],
    );
    key_integer(&mut body, "StartInterval", agent.interval_seconds);
    body.push_str("\t<key>RunAtLoad</key>\n\t<true/>\n");
    key_string(&mut body, "ProcessType", "Background");
    let log = shown(&agent.state_dir.join("launchd.log"));
    key_string(&mut body, "StandardOutPath", &log);
    key_string(&mut body, "StandardErrorPath", &log);
    plist(&body)
}

#[cfg(test)]
#[path = "watchdog_command_tests.rs"]
mod tests;

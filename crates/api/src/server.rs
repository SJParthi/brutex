//! The operator's window onto the store, and the process that serves it.
//!
//! Everything the binary does lives here rather than in `main.rs`, so that
//! every branch of it is reachable from a test. `main.rs` holds one call and
//! nothing else — a line of logic in a `main` is a line no unit test can enter,
//! and `docs/04-invariants.md` X-06 does not exempt binaries.
//!
//! # What the page is for
//!
//! It shows what decoded, what was declined and — crucially — what could not
//! be read at all. A refused row is the thing that hides an instrument, so it
//! is on the page rather than in a log. Since the equity gate landed it also
//! shows every ISIN two vendors disagree about, because a disagreement nobody
//! sees decides the run on its own.

use crate::catalog::{Catalog, PAGE_ROWS, Selection};
use crate::{
    assets, audit, audit_json, autopilot, bars, census, constituents, coverage, ingest, master,
    merge, render,
};
use brutex_core::vendor::Vendor;
use pull::session::{Day, IstMoment};
use std::fmt::Write as _;
use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

/// The default listen address when none is given.
///
/// Loopback, not `0.0.0.0`: this page is an operator's window onto a local
/// store, and a default that listens on every interface is a decision nobody
/// took.
pub const DEFAULT_ADDR: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 8080);

/// The longest request body either form may send.
///
/// # Why the number is small and why it is stated
///
/// The two forms are five short fields between them: a target slug, a series
/// slug, an underlying symbol of at most `brutex_core::symbol::SYMBOL_CAPACITY`
/// bytes, and two ten-character dates. Percent-encoding is at worst 3× per
/// byte. 8 KiB is roughly 80× the largest honest body and still small enough
/// that a body over it is unambiguously not a form this server serves.
///
/// It is stated because a bound nobody wrote down is a bound nobody owns.
/// `axum` applies a 2 MiB default, so a 2 MiB `to_uppercase()` of a field that
/// `Symbol::new` then refuses for being over 24 bytes was reachable, and the
/// only thing standing between this server and that allocation was a
/// dependency's default value. A request past this answers `413` — the refusal
/// is the framework's and it is loud, not a truncation.
const MAX_FORM_BYTES: usize = 8 * 1024;

/// The signal that stops the server.
///
/// A boxed future rather than a type parameter, deliberately. A generic
/// `serve` is a *different function* for every caller — one for the binary's
/// ctrl-C, one for each test's ready future — and coverage is accounted per
/// instantiation, so the binary's copy would be permanently short of the arms
/// only a test exercises. One allocation per process buys one function with
/// one set of counters. The same reasoning applies to the argument list below.
pub type Shutdown = std::pin::Pin<Box<dyn Future<Output = std::io::Result<()>> + Send>>;

/// What the operator asked the binary to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Bind and serve until the shutdown signal arrives.
    Serve(SocketAddr),
    /// Print the decode tallies for every master and exit.
    Report,
}

impl Command {
    /// Parses the command line.
    ///
    /// # Errors
    ///
    /// A usage message naming what was not understood. An unrecognised
    /// argument is a refusal rather than a fallback to serving: a typo that
    /// silently starts a server is a typo nobody finds.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut args = args.iter().map(String::as_str);
        match (args.next(), args.next()) {
            // No command at all serves, because that is what the operator has
            // always typed. A WRONG command does not: see the last arm.
            (None, _) | (Some("serve"), None) => Ok(Self::Serve(DEFAULT_ADDR)),
            (Some("serve"), Some(addr)) => addr
                .parse()
                .map(Self::Serve)
                .map_err(|e| format!("not a socket address: {addr:?} ({e})")),
            (Some("report"), None) => Ok(Self::Report),
            (Some(other), _) => Err(format!(
                "unknown argument {other:?}; usage: api [serve [ADDR] | report]"
            )),
        }
    }
}

/// Where each vendor's master is expected.
#[must_use]
pub fn master_paths(dir: &Path) -> Vec<(Vendor, PathBuf)> {
    // DERIVED FROM THE REGISTRY, not written out. This was a hand-built list of
    // two pairs, so adding a feed meant editing this function — and forgetting
    // to meant a vendor whose master was never read, reported as "this vendor
    // lists nothing" rather than as the wiring bug it was.
    //
    // `Vendor::master_file` is a `match`, so a new variant does not compile
    // until it names its file. Everything downstream of here already iterates
    // `Vendor::ALL`; this was the one place that did not.
    Vendor::MASTERED
        .into_iter()
        .map(|vendor| (vendor, dir.join(vendor.master_file())))
        .collect()
}

/// The directory the masters are read from.
///
/// `BRUTEX_MASTERS`, or `$HOME/.brutex/masters`. This is the only place the
/// environment is consulted; everything below takes the directory as an
/// argument, so nothing else has to touch process-wide state to be
/// deterministic — and nothing can, since setting an environment variable is
/// `unsafe` under edition 2024 and this crate forbids `unsafe`.
///
/// # Errors
///
/// Neither variable set. See [`masters_dir_from`].
pub fn masters_dir() -> Result<PathBuf, String> {
    masters_dir_from(std::env::var_os("BRUTEX_MASTERS"))
}

/// The masters directory implied by a value of `BRUTEX_MASTERS`.
///
/// Split from [`masters_dir`] so both outcomes are testable without mutating
/// the environment of a process running tests in parallel.
///
/// # Errors
///
/// No value and no `HOME`. See [`default_masters_dir_from`].
fn masters_dir_from(value: Option<std::ffi::OsString>) -> Result<PathBuf, String> {
    value.map_or_else(default_masters_dir, |v| Ok(PathBuf::from(v)))
}

/// The directory the bar store and its manifests are read from.
///
/// `BRUTEX_STORE`, or `$HOME/.brutex/store`. Split exactly as
/// [`masters_dir`] is, and for the same reason: the environment is consulted in
/// one place and every function below takes the directory as an argument.
///
/// # Errors
///
/// Neither variable set. See [`store_dir_from`].
pub fn store_dir() -> Result<PathBuf, String> {
    store_dir_from(std::env::var_os("BRUTEX_STORE"), std::env::var_os("HOME"))
}

/// The store directory implied by values of `BRUTEX_STORE` and `HOME`.
///
/// Both outcomes have to be testable and a test cannot set either variable:
/// `set_var` is `unsafe` under edition 2024, this crate forbids `unsafe`, and
/// mutating process-wide state would race every other test in the binary.
///
/// # A relative fallback for the store is a REFUSAL, not a default
///
/// This returned `PathBuf::from(".")` when `HOME` was unset, and a test asserted
/// it: *"no HOME is a broken environment, not a supported one"*. The sentence
/// was right and the return value contradicted it. `.` is the process working
/// directory, which for the run configuration this is launched from is the
/// **repository checkout** — so the first append creates `bars/`, `manifest/`
/// and `audit/` inside the git tree, under names CI gate 1 never sees because
/// it walks `git ls-files` and these are untracked. The banner printed
/// `store:   .`, which reads as a deliberate relative path rather than as a
/// broken environment.
///
/// `CLAUDE.md` §8 is the governing rule and it is about configuration in
/// general, not only about credentials: *a missing or malformed configuration
/// halts loudly — there is no default and no fallback.* A store root is the
/// most consequential path this process holds, so an absent one halts.
///
/// An explicit `BRUTEX_STORE` is honoured whatever it says, relative included:
/// that is an operator's stated choice, and refusing a choice is a different
/// act from inventing one.
///
/// # Errors
///
/// Both `BRUTEX_STORE` and `HOME` unset: a sentence naming both variables, the
/// working directory the old fallback would have written into, and what to set.
fn store_dir_from(
    value: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, String> {
    if let Some(value) = value {
        return Ok(PathBuf::from(value));
    }
    let Some(home) = home else {
        return Err(format!(
            "REFUSED: no store root. BRUTEX_STORE is unset and so is HOME, so there is \
             nothing to derive $HOME/.brutex/store from. This used to fall back to \
             \".\" — the working directory, which here is {} — and the first append \
             would have built bars/, manifest/ and audit/ inside it. Set BRUTEX_STORE \
             to an absolute path, or run under an environment that has HOME.",
            std::env::current_dir()
                .map_or_else(|e| format!("unreadable ({e})"), |p| p.display().to_string())
        ));
    };
    Ok(PathBuf::from(home).join(".brutex").join("store"))
}

/// Where the masters live when `BRUTEX_MASTERS` says nothing.
///
/// `$HOME/.brutex/masters`, not the working directory. The masters are ~50 MB
/// of vendor CSV and `.csv` is not an allowed tracked extension (CLAUDE.md §2),
/// so they can never live in the repository — which means "the working
/// directory" is only ever right when the operator happens to have `cd`-ed
/// somewhere specific. Launching from anywhere else found no files and rendered
/// `UNAVAILABLE`, correctly reporting a real absence caused entirely by the
/// default.
///
/// # Errors
///
/// `HOME` unset. That used to fall back to `.` and the page then said
/// `UNAVAILABLE` for both vendors — a true sentence about a wrong directory,
/// which is worse than a refusal because it names the vendor rather than the
/// environment. See [`default_masters_dir_from`].
fn default_masters_dir() -> Result<PathBuf, String> {
    default_masters_dir_from(std::env::var_os("HOME"))
}

/// The default implied by a value of `HOME`.
///
/// Split for the same reason [`masters_dir_from`] is: both outcomes have to be
/// testable, and a test cannot unset `HOME` — `set_var` is `unsafe` under
/// edition 2024, this crate forbids `unsafe`, and mutating process-wide state
/// would race every other test in the binary.
///
/// # Errors
///
/// `HOME` unset, for the reason [`store_dir_from`] gives at length: a relative
/// fallback is a wrong answer that reads like a chosen one, and `CLAUDE.md` §8
/// leaves no default and no fallback for a configured path.
fn default_masters_dir_from(home: Option<std::ffi::OsString>) -> Result<PathBuf, String> {
    home.map_or_else(
        || {
            Err(String::from(
                "REFUSED: no masters directory. BRUTEX_MASTERS is unset and so is HOME, \
                 so there is nothing to derive $HOME/.brutex/masters from. This used to \
                 fall back to the working directory and then report both vendors as \
                 missing, which names the wrong thing: the vendors were never looked \
                 for where they live. Set BRUTEX_MASTERS to an absolute path, or run \
                 under an environment that has HOME.",
            ))
        },
        |home| Ok(PathBuf::from(home).join(".brutex").join("masters")),
    )
}

/// What one read of the masters produced: the universe, the notes, and whether
/// any of it may be believed.
#[derive(Debug)]
pub struct Read {
    /// One entry per distinct instrument.
    pub merged: merge::Merged,
    /// Everything an operator has to be told: per-vendor tallies, every
    /// decline reason, every unreadable row's reason, and every disagreement.
    pub notes: Vec<String>,
    /// The same lines, prepared for rendering: the text each one draws and
    /// whether it is loud, both decided here rather than on every request.
    ///
    /// Derived from [`Self::notes`] inside [`Self::new`] rather than set beside
    /// it, for the reason [`Self::unavailable`] is: two fields a caller fills
    /// separately are two fields that can disagree.
    ///
    /// It exists because a note's LENGTH is data. One `coverage` line names
    /// every index a feed could not resolve, so the note text grows with the
    /// universe, and the renderer read every byte of it — five substring
    /// searches for the loud words, twice over, plus a separator count in the
    /// tail — to draw a line truncated to 160 bytes. That was gate 8's C-15
    /// breach: 1,444 - 2,346 ps per instrument per request against a 1,000 ps
    /// ceiling, on all thirty `instruments_html_from` lines. The full text is
    /// still here for `/health` and the startup banner; only the RENDER path
    /// stopped rereading it. `docs/05-decisions.md` D-0130.
    pub notes_view: render::Notes,
    /// Whether a vendor was never read at all.
    ///
    /// Distinct from a merge conflict, and tracked separately because "the two
    /// vendors disagree" and "there was only one vendor" are different facts
    /// that must not collapse into one status.
    ///
    /// Derived from [`Self::unread`] rather than set beside it, so the boolean
    /// and the list cannot disagree.
    pub unavailable: bool,
    /// WHICH vendor was never read, and what refused it.
    ///
    /// The boolean above answers "was any master missing" and every consumer
    /// of it — `/health`, the exit code, the dashboard — asks exactly that.
    /// `/instruments.json` asks a different question: the list it returns is
    /// **one feed's**, so a Groww master that failed to decode empties the body
    /// for `?feed=groww` and changes nothing for `?feed=dhan`. A boolean cannot
    /// separate those two responses and the notes could only be grepped, so the
    /// route returned `[]` at 200 either way. D-0124.
    pub unread: Vec<(Vendor, String)>,
    /// How many rows were declined under a listing class nobody recognises.
    ///
    /// Not a merge disagreement — it is one vendor's file using a code this
    /// engine has never measured, which is how an alphabet moves under a
    /// gate. Counted here so it reaches the status and the exit code, because
    /// the reason string alone sat beside six routine ones and read like them.
    pub unrecognised: usize,

    /// How many rows no decoder could read, across every vendor.
    ///
    /// Carried as a field because `is_clean` must consult it, and a count
    /// folded into a note string is not consultable.
    pub unreadable: usize,
    /// Every ordering and every filter the pages offer, decided once.
    ///
    /// The reason it is here and not built per request is `docs/05-decisions.md`
    /// D-0042: the masters are parsed once into an `Arc<Site>`, and that parse
    /// is the only place a whole-universe pass may happen.
    pub catalog: Catalog,
    /// What each published NSE tier resolves to, per vendor, keyed on
    /// `(exchange, ISIN)`.
    ///
    /// Here for the same reason [`Self::catalog`] is: it is a whole-universe
    /// pass, so it happens once at load and never on a request. Before D-0117
    /// there was no join at all — `core` held the constituent lists, `master`
    /// held the vendor rows, and nothing turned a tier into the ids a pull must
    /// name. See [`constituents::Join`].
    pub constituents: constituents::Join,
    /// What each FEED can actually name in each spot target, and every name it
    /// cannot.
    ///
    /// Here for the reason [`Self::catalog`] and [`Self::constituents`] are: it
    /// is a whole-universe pass, so it happens once at load and never on a
    /// request (D-0039/D-0042). The five list-defined targets are read straight
    /// off [`Self::constituents`] — the join D-0117 built — so this holds no
    /// second answer to "who is in the NIFTY 200", and the two master-counted
    /// ones share one fold. See [`coverage::Coverage`]. D-0120.
    pub coverage: coverage::Coverage,
}

impl Read {
    /// Builds a read, computing everything a request must not compute.
    ///
    /// Struct-literal construction is deliberately not the way in: the
    /// precomputed [`Catalog`] is the whole of D-0042, and a caller that filled
    /// the fields by hand would get a page that silently went back to scanning.
    #[must_use]
    pub fn new(
        merged: merge::Merged,
        mut notes: Vec<String>,
        unread: Vec<(Vendor, String)>,
        unrecognised: usize,
        unreadable: usize,
    ) -> Self {
        let catalog = Catalog::build(&merged);
        // THE JOIN IS BUILT HERE, AND ITS BUCKETS ARE NOTES.
        //
        // A tier that resolves to nothing for a feed is a result, and a result
        // an operator never sees is the same as no result: the `/ingest`
        // universe menu said `no target` beside every NIFTY tier and nothing
        // anywhere said which names failed to resolve, or whether any had. Each
        // line names the four buckets and their sum. D-0117.
        let constituents = constituents::Join::build(&merged);
        notes.extend(constituents.notes());
        // AND WHAT EACH FEED REACHES, WHICH IS A DIFFERENT NUMBER.
        //
        // The join answers per `(vendor, tier)` for the five targets a
        // published list defines. The two it cannot — the swept pair and the
        // reference indices — have no published list to be a fraction of, and
        // those are precisely the two where the universe count and the feed's
        // reach diverge most: `indices` is 35 merged rows, of which Groww lists
        // 24 and Dhan 15. A note per shortfall, so the divergence is on
        // `/health` rather than discovered mid-run. D-0120.
        let coverage = coverage::Coverage::build(&merged, &constituents);
        notes.extend(coverage.notes());
        // LAST, because it reads the finished list. Every `extend` above must
        // already have run: a view built before them would render a page that
        // is missing exactly the lines the load discovered. D-0130.
        let notes_view = render::Notes::build(&notes);
        Self {
            merged,
            notes,
            notes_view,
            // ONE SOURCE, TWO SHAPES. `unavailable` was a separate `bool` a
            // caller set beside the notes; nothing made the two agree, and a
            // list that grows a vendor while the boolean stays false is the
            // silent-degradation shape this file keeps finding. D-0124.
            unavailable: !unread.is_empty(),
            unread,
            unrecognised,
            unreadable,
            catalog,
            constituents,
            coverage,
        }
    }

    /// Whether this read is fit to be believed.
    ///
    /// A missing vendor counts. So does any disagreement. So does a listing
    /// class nobody recognises — that one is the difference between a routine
    /// bond and an alphabet moving under us.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        // `errors` is load-bearing here and was missing, which made this the
        // worst shape of bug this repository hunts: a plausible wrong answer.
        //
        // A permutation audit traced the path. If BOTH master files fail to
        // decode entirely, no row ever reaches the series gate, so
        // `unrecognised` stays 0; nothing is kept, so there is nothing to
        // disagree about and `merged.verdict()` is `Clean`; the files were
        // found, so `unavailable` is false. Every term was true and
        // `status()` answered "ok" over a universe of zero instruments.
        //
        // A monitor reading one word would have seen a healthy server. The
        // decode failures were never hidden — `errors_by_reason` renders them
        // on the page — but the machine-readable word did not consult them,
        // which is `CLAUDE.md` §4's "fallback that hides a failure" arriving
        // by omission rather than by design.
        //
        // `api::unit::a_total_decode_failure_is_not_clean` is what holds it up.
        self.unreadable == 0
            && !self.unavailable
            && self.unrecognised == 0
            && self.merged.verdict() == merge::Verdict::Clean
    }

    /// The one-word status a machine reads first.
    #[must_use]
    pub fn status(&self) -> &'static str {
        if self.is_clean() { "ok" } else { "DEGRADED" }
    }

    /// What became of ONE feed's master, and the sentence that says why.
    ///
    /// [`Self::status`] answers for the whole read, which is what `/health` and
    /// the exit code want. A route that returns one feed's list needs the
    /// per-feed answer, because a Groww master that failed to decode empties
    /// `?feed=groww` and leaves `?feed=dhan` complete — one status word cannot
    /// carry both, and before D-0124 neither reached the caller at all.
    #[must_use]
    pub fn master(&self, vendor: Vendor) -> (MasterState, String) {
        if let Some((_, why)) = self.unread.iter().find(|(v, _)| *v == vendor) {
            return (
                MasterState::Unavailable,
                format!(
                    "{}: UNAVAILABLE — {why}. The list below is what could be read, \
                     which for this feed is nothing — it is NOT this feed listing \
                     nothing.",
                    vendor.as_str()
                ),
            );
        }
        if !Vendor::MASTERED.contains(&vendor) {
            return (
                MasterState::NotMastered,
                format!(
                    "{}: this build parses no instrument master for this feed, so it \
                     names no instruments. An empty list is the true answer.",
                    vendor.as_str()
                ),
            );
        }
        (
            MasterState::Read,
            format!(
                "{}: master read; {} instrument(s) in the merged universe",
                vendor.as_str(),
                self.merged.by_key.len()
            ),
        )
    }
}

/// What became of one feed's instrument master.
///
/// Three states and no boolean, for the reason [`Broker`] gives about its own
/// pair: "this feed has no master in this build" and "this feed's master would
/// not decode" produce the SAME empty list on `/instruments.json`, and only one
/// of them is a failure. Collapsing them is the fallback that hides a failure
/// `CLAUDE.md` §4 bans. D-0124.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterState {
    /// The file was found and decoded. The list below it is real.
    Read,
    /// The file is one this build expects and it could not be read. Every
    /// number derived from it is absent rather than zero.
    Unavailable,
    /// This feed has no master file in this build at all — it is an archive
    /// source, not a broker whose scrip file is parsed. An empty list for it is
    /// a true answer, not a failed read.
    NotMastered,
}

impl MasterState {
    /// The one stable word that goes on the wire.
    ///
    /// A total match rather than a `matches!` at each call site, for the reason
    /// [`census::Census::name`] gives: a fourth state fails to compile here
    /// rather than falling through a wildcard somewhere else.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Unavailable => "UNAVAILABLE",
            Self::NotMastered => "not-mastered",
        }
    }
}

/// Every vendor's kept listings, merged, plus one note per vendor.
///
/// A vendor whose file is missing produces a note saying so and no listings.
/// It does not produce an empty success: `UNAVAILABLE` on the page is the
/// difference between "this vendor lists nothing" and "this vendor was never
/// read", and collapsing the two is exactly the silent degradation
/// `CLAUDE.md` §4 forbids.
#[must_use]
pub fn universe(dir: &Path) -> Read {
    let mut notes = Vec::new();
    let mut sources = Vec::new();
    let mut unread: Vec<(Vendor, String)> = Vec::new();
    let mut unrecognised = 0;
    let mut unreadable = 0;
    for (vendor, path) in master_paths(dir) {
        match master::load(&path, vendor) {
            Ok(l) => {
                let mut note = format!(
                    "{}: {} kept, {} declined, {} unreadable",
                    vendor.as_str(),
                    l.kept.len(),
                    l.skipped_total(),
                    l.errors.len()
                );
                unreadable += l.errors.len();
                for (reason, n) in l.skipped_by_reason() {
                    let _ = write!(note, " · {reason} {n}");
                }
                notes.push(note);
                // AN UNREADABLE ROW SAYS WHY, AND WHERE. The reasons were
                // collected with their line numbers and then read only for
                // `.len()`, so `104 unreadable` was the whole of what an
                // operator was told and the cause had to be grepped out of the
                // raw CSV. Grouped by reason, so the output is bounded without
                // any reason being hidden.
                for (reason, n, first) in l.errors_by_reason() {
                    notes.push(format!(
                        "{} UNREADABLE · {reason} ×{n}, first at line {first}",
                        vendor.as_str()
                    ));
                }
                // The COUNT says an alphabet moved; the CODE says which one.
                for (code, n) in &l.unrecognised {
                    unrecognised += n;
                    notes.push(format!(
                        "{} UNRECOGNISED LISTING CLASS · {code:?} ×{n} — \
                         this is not a bond, it is a code this engine has never seen",
                        vendor.as_str()
                    ));
                }
                sources.push(merge::Source {
                    vendor,
                    kept: l.kept,
                    declined: l.declined,
                });
            }
            Err(e) => {
                // THE REASON IS KEPT BESIDE THE VENDOR, not only inside a
                // sentence. `notes` renders; it does not answer "was THIS feed
                // read", and `/instruments.json` has to ask exactly that before
                // it may call an empty list an empty universe. D-0124.
                notes.push(format!("{}: UNAVAILABLE — {e}", vendor.as_str()));
                unread.push((vendor, e));
            }
        }
    }
    let merged = merge::merge(&sources);
    for line in &merged.conflicts {
        notes.push(format!("ISIN CONFLICT · {line}"));
    }
    for line in &merged.eligibility {
        notes.push(format!("ELIGIBILITY CONFLICT · {line}"));
    }
    for (name, present, both) in merged.universe_census() {
        notes.push(format!(
            "{name}: {present} resolved, {both} confirmed by every master read"
        ));
    }
    // An index carries no ISIN, so nothing cross-checks its identity. Saying
    // which members rest on one vendor is the only honest substitute.
    let alone = merged.single_vendor_members();
    if !alone.is_empty() {
        notes.push(format!(
            "UNCHECKED IDENTITY · {} universe member(s) named by one vendor only: {}",
            alone.len(),
            alone.join(", ")
        ));
    }
    Read::new(merged, notes, unread, unrecognised, unreadable)
}

/// Liveness plus the decode tallies, so a machine can check what a human sees.
///
/// The first line is `ok` or `DEGRADED`, and it is not decoration: it used to
/// be an unconditional `ok` printed beside `dhan: UNAVAILABLE`, so a monitor
/// reading the status, the exit code or the HTTP code saw green while one of
/// the two vendors had never been read.
#[must_use]
pub fn report(dir: &Path) -> (String, bool) {
    report_from(&universe(dir))
}

/// The decode report, from a universe that is **already loaded**.
///
/// Split for the same reason as [`instruments_html_from`]: `/health` is what a
/// monitor polls, and polling it must not re-parse both masters. See that
/// function for the measured cost.
#[must_use]
pub fn report_from(read: &Read) -> (String, bool) {
    let mut out = format!("{}\n", read.status());
    for note in &read.notes {
        // Writing to a String is infallible; `unwrap_used` and `expect_used`
        // are denied workspace-wide and neither belongs here.
        let _ = writeln!(out, "{note}");
    }
    let _ = writeln!(
        out,
        "merged: {} instruments, {} isin conflicts, {} eligibility conflicts",
        read.merged.len(),
        read.merged.conflicts.len(),
        read.merged.eligibility.len()
    );
    (out, read.is_clean())
}

/// Parses `?q=...` without a query-string dependency.
///
/// `+` and `%XX` are decoded because a symbol may be typed with spaces.
/// Anything undecodable is kept literally rather than dropped — a query must
/// never silently become a different query.
#[must_use]
pub fn parse_query(raw: &str) -> String {
    param(raw, "q")
}

/// One named parameter out of a query string, decoded.
///
/// Named rather than positional because the page now carries two — the search
/// text and the sort column — and reading the second by position would make
/// `?sort=isin&q=NIFTY` mean something different from `?q=NIFTY&sort=isin`.
#[must_use]
pub fn param(raw: &str, name: &str) -> String {
    let prefix = format!("{name}=");
    for pair in raw.split('&') {
        if let Some(v) = pair.strip_prefix(prefix.as_str()) {
            return percent_decode(v);
        }
    }
    String::new()
}

/// EVERY value a repeated field carries, in the order they were sent.
///
/// # Why a second reader and not a wider `param`
///
/// [`param`] answers "what did they say for this field", which is the right
/// question for a field that appears once — a target, a window, a feed. A
/// repeated field asks a different question and must not be answered by the
/// first match: `member=NIFTY&member=BANKNIFTY` read through `param` is
/// `NIFTY`, and a request for two instruments silently becomes a request for
/// one.
///
/// One pass over the query string, one allocation per value returned. The
/// caller bounds how many it will accept — an unbounded repeated field is
/// unbounded input, and `docs/07-o1-architecture.md` law 5 says bound it at the
/// boundary.
#[must_use]
pub fn params(raw: &str, name: &str) -> Vec<String> {
    let prefix = format!("{name}=");
    raw.split('&')
        .filter_map(|pair| pair.strip_prefix(prefix.as_str()))
        .map(percent_decode)
        .collect()
}

/// The zero-based page number a query string asks for.
///
/// Anything unparseable is page one. This is the one parameter that is
/// defaulted rather than refused, and it is defensible only because a page
/// number selects a VIEW: it can never change what the data says, so a mangled
/// bookmark should land on the first page rather than on an error.
#[must_use]
pub fn page_number(raw: &str) -> usize {
    param(raw, "page").parse().unwrap_or(0)
}

/// Decodes `+` and `%XX` escapes.
#[must_use]
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    // `while let` rather than `while i < len` with an indexed read: the
    // indexed form needs a `None` arm that cannot happen, and a branch no test
    // can enter is a branch nobody has checked.
    while let Some(&c) = b.get(i) {
        match c {
            b'+' => out.push(b' '),
            b'%' => match hex_escape(b, i) {
                Some(byte) => {
                    out.push(byte);
                    i += 2;
                }
                // A truncated or non-hex escape is kept LITERALLY. Dropping it
                // would turn one query into a different query in silence.
                None => out.push(b'%'),
            },
            _ => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The byte a `%XX` at `at` encodes, or `None` if it is not one.
fn hex_escape(b: &[u8], at: usize) -> Option<u8> {
    let hi = hex_digit(*b.get(at + 1)?)?;
    let lo = hex_digit(*b.get(at + 2)?)?;
    // At most 15 * 16 + 15 = 255, so this cannot overflow a byte.
    Some(hi * 16 + lo)
}

/// The value of one hexadecimal digit, in either case.
fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Renders the instruments page for one query.
///
/// # Why the notes are not part of the title
///
/// They were, and only when the query was empty — the search branch built a
/// title from the query alone and dropped `notes` on the floor. `notes` is the
/// sole carrier of `<vendor>: UNAVAILABLE` and of every conflict line, so
/// typing anything into the search box made the page stop saying that a
/// vendor's master had never been read, and a row rendered with one vendor tag
/// was byte-identical to a genuine single-vendor listing. With `PAGE_ROWS` at
/// 200 against thousands of instruments, search is the only way to reach most
/// of them, so that was not a corner case — it was the normal path. The notes
/// are now a banner the page always carries, whatever was typed.
#[must_use]
pub fn instruments_html(dir: &Path, query: &str) -> String {
    instruments_html_from(&universe(dir), query, "", false, "", 0)
}

/// The instruments page, rendered from a universe that is **already loaded**.
///
/// # Why this split exists
///
/// [`instruments_html`] reads both masters from disk on every call. Serving a
/// page through it parsed ~200,000 CSV rows to render 200 of them, and
/// measured **150 ms per request** against the real files — cost proportional
/// to the size of the masters, on a path `CLAUDE.md` §3 rule 4 requires to be
/// constant.
///
/// No test caught it. A fixture of four rows parses in microseconds and passes
/// with 100% coverage; only the 50 MB file shows the shape of the curve. That
/// is the difference between *covered* and *correct*.
///
/// The masters are therefore read **once**, at startup, and every request
/// renders from that. Re-reading is now an explicit operator action rather
/// than a side effect of looking at the page.
#[must_use]
pub fn instruments_html_from(
    read: &Read,
    query: &str,
    sort: &str,
    all: bool,
    universe_filter: &str,
    page: usize,
) -> String {
    let needle = query.to_uppercase();

    // THE TRACKED UNIVERSE, and nothing else.
    //
    // The masters carry every NSE listing the gate accepts as a real share --
    // about 2,700 per vendor. The engine tracks a strict subset: the NIFTY
    // Total Market constituents, plus the index series. The other ~1,900 NSE
    // equities are decoded, counted and declined here rather than in the
    // decoder, because "is this a share" and "is this a share I follow" are
    // different questions and collapsing them would mean anything outside the
    // list never got validated at all.
    //
    // F&O needs no clause: every F&O stock underlying is already a Total
    // Market constituent -- 208 of 208, measured, zero outside. F&O is a
    // filter label on instruments already here, not a third source of rows.
    // `all` lifts the universe filter so every listing the gate accepted is
    // reachable. The DEFAULT is the tracked universe, because a page opening on
    // 2,700 rows hides the 785 that matter — but a filter with no way past it
    // hides a bug instead, so the escape hatch is a link on the page, not a
    // recompile.
    //
    // EVERY ONE OF THOSE DECISIONS IS MADE AT LOAD TIME, not here. The scope,
    // the pill and the ordering pick 1 of 48 lists that already exist; the pill
    // COUNTS are four `usize` reads. What this function used to do -- fold the
    // whole map, filter it into a fresh `Vec`, sort that vector and reverse it,
    // on every request, to draw a fixed 200 rows -- is `docs/05-decisions.md`
    // D-0042 and it is gone. `crates/api/benches/ratio.rs` asserts the shape.
    let selection = Selection::new(all, universe_filter, sort);
    let counts = read.catalog.counts(selection);
    let total = counts.all;

    // PAGING, so nothing is unreachable.
    //
    // The cap alone rendered 200 of 785 and offered no way to the rest —
    // scrolling cannot reveal rows that were never sent, and the page said
    // "showing 200" as though that were the whole answer. A bound with no
    // navigation past it is the same class of defect as a filter with no
    // escape hatch.
    //
    // The offset is clamped to the last page rather than refused: `?page=999`
    // is a stale bookmark, not an attack, and it should land somewhere real.
    //
    // SEARCH IS THE ONE PATH THAT STILL LOOKS AT ROWS IT WILL NOT DRAW, and it
    // says so: `Catalog::search` narrows by a trigram index and names the two
    // cases that stay linear. `docs/06-limits.md` §24 carries the measurement.
    let view = if needle.is_empty() {
        read.catalog.page(selection, page)
    } else {
        read.catalog.search(selection, &needle, page)
    };
    let (rows, matched, page, last_page) = (view.rows, view.matched, view.page, view.last_page);

    // `total` is the whole universe; `matched` is what the filter selected.
    // Reporting the right one keeps the page honest about what it looked at.
    let (title, denominator) = if needle.is_empty() {
        (format!("brutex · instruments · {}", read.status()), total)
    } else {
        (
            format!(
                "brutex · {} · search {query:?} · {matched} matched",
                read.status()
            ),
            matched,
        )
    };
    render::instruments_page(&render::View {
        title: &title,
        total: denominator,
        rows: &rows,
        query,
        sort,
        all,
        counts,
        active: universe_filter,
        page,
        last_page,
        notes: &read.notes_view,
    })
}

/// The dashboard.
///
/// Every figure is a counter already in memory. Nothing here scans, so the page
/// costs the same whether the store holds two instruments or two hundred
/// thousand — which is the whole point of `docs/04-invariants.md` C-01.
async fn home(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> axum::response::Html<String> {
    axum::response::Html(dashboard_html(&site.read))
}

/// The dashboard, from a universe already loaded.
#[must_use]
pub fn dashboard_html(read: &Read) -> String {
    // TWO WHOLE-MAP SCANS USED TO BE HERE, under a docstring that already
    // claimed "nothing here scans". Measured before D-0042: 1,720 ns at 2
    // instruments, 138,702 ns at 50,000 — an 80× curve under a comment saying
    // it was flat. Both are now counters computed once at load.
    let (counts, both) = read.catalog.dashboard_counts();
    let disputes = read.merged.conflicts.len() + read.merged.eligibility.len();

    let (all, fno, ntm, idx) = (counts.all, counts.fno, counts.ntm, counts.index);
    let n = |v: usize| v.to_string();
    let stats = [
        render::Stat {
            label: "Tracked",
            value: &n(all),
            note: "NIFTY Total Market + indices",
            loud: false,
        },
        render::Stat {
            label: "NIFTY Total Market",
            value: &n(ntm),
            note: "constituents",
            loud: false,
        },
        render::Stat {
            label: "F&O underlyings",
            value: &n(fno),
            note: "all inside Total Market",
            loud: false,
        },
        render::Stat {
            label: "Indices",
            value: &n(idx),
            note: "NSE index series",
            loud: false,
        },
        render::Stat {
            label: "Confirmed by both feeds",
            value: &n(both),
            note: "cross-checked identity",
            loud: false,
        },
        render::Stat {
            label: "Disagreements",
            value: &n(disputes),
            note: "identity + eligibility",
            loud: disputes > 0,
        },
    ];
    render::dashboard_page(read.status(), &stats, &read.notes_view)
}

/// The instruments page.
async fn page(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> axum::response::Html<String> {
    let raw = uri.query().unwrap_or("");
    let typed = parse_query(raw);
    let sort = param(raw, "sort");
    let all = param(raw, "all") == "1";
    let u = param(raw, "u");
    let page = page_number(raw);
    axum::response::Html(instruments_html_from(
        &site.read, &typed, &sort, all, &u, page,
    ))
}

/// The browser's copy of the bounded universe, for the type-ahead.
///
/// # Why the whole set, and why once
///
/// The engine surface is bounded: ~750 NIFTY Total Market equities plus ~35 NSE
/// indices. That is small enough to hand the browser the entire searchable set
/// in one response, which is what makes a keystroke a Map probe instead of a
/// request. A request per character is ~800 requests to type one symbol and
/// makes keystroke latency a function of the network.
///
/// Hand-written JSON rather than a serialiser, because adding one would be a
/// dependency for six fields whose shapes are all known here, and every value
/// below is escaped through [`render::json_string`] rather than trusted.
///
/// # Two silent answers this route used to give, and what now separates them
///
/// Both were `200` with no status anywhere on the wire, while `Read::notes` and
/// [`Read::status`] — which `/health` reads and answers `503` from — sat one
/// field away and were discarded here.
///
/// * **The counter would not load.** `rows_for` answers `None` for an
///   unreadable census, so `bars_of` summed to `0` for every row and the page
///   drew the same em dash a genuinely un-pulled instrument shows. The universe
///   was intact; the count was not a measurement.
/// * **The selected feed's master would not decode.** No row then carries that
///   feed's id, the filter admits nothing, and the body is `[]` — indistinguishable
///   from a feed that lists nothing. [`Read::master`] is the per-feed answer the
///   whole-read boolean could not give.
///
/// Both now answer `503` and stamp the reason: [`UNIVERSE_STATUS_HEADER`],
/// [`MASTER_STATE_HEADER`], [`MASTER_NOTE_HEADER`] and the three census headers
/// [`census_headers`] writes. **The body shape does not move** — it is a JSON
/// array in every state, exactly as it was. D-0124.
async fn instruments_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    // THE LIST IS THE SELECTED FEED'S, NOT THE MERGE OF ALL OF THEM.
    //
    // This returned the merged universe whatever feed was chosen, so switching
    // the picker changed nothing. That is wrong twice over: the two brokers do
    // NOT list the same instruments — `GIFTNIFTY` is Dhan-only — so the page
    // offered symbols the selected feed cannot be asked for, and `held` was
    // computed feed-agnostically, so a row read HELD while the chosen feed held
    // nothing for it.
    //
    // An instrument belongs to a feed's list when that feed's master gave it an
    // id. `ids` is indexed by the vendor's own discriminant, so the test is one
    // array index per row and not a lookup.
    let feed = ingest::parse_vendor(&param(uri.query().unwrap_or(""), "feed"))
        .unwrap_or(brutex_core::vendor::Vendor::Dhan);
    // SORTED, GROUPED, AND HELD-FIRST. `by_key` is a HashMap, so iterating it
    // gave the browser TATACOMM, GMRAIRPORT, PINELABS — an order stable per
    // process and meaningless to a human. Nothing was findable by scrolling and
    // 35 indices were scattered through 750 equities.
    //
    // Three keys: instruments this store HOLDS come first because those are the
    // ones that can be charted; indices before equities because they are the
    // engine's own surface and there are 35 against 750; then alphabetical.
    //
    // O(n log n) once per request over a bounded ~800, replacing an unbounded
    // amount of human scanning.
    let mut listing: Vec<_> = site
        .read
        .merged
        .by_key
        .iter()
        .filter(|(_, entry)| {
            crate::catalog::tracked(entry.universe)
                && entry.ids.get(feed as usize).copied().flatten().is_some()
        })
        .collect();
    // HOW MANY BARS, not whether any. "HELD" was store jargon that leaked onto
    // the screen: it told an operator a boolean when the question they actually
    // have is "how much of this do I have". A count answers both — zero IS the
    // boolean, and 1,125 is what the boolean threw away.
    // FRESH, so a row's bar count moves as a backfill lands. See `census_now`.
    let (censuses, entries) = census_now(&site);
    // ONE PASS OVER THE CENSUS, NOT ONE PER INSTRUMENT. See `bars_by_symbol`.
    let held = bars_by_symbol(censuses.iter().find(|c| c.vendor == feed), &entries);
    let bars_of =
        |sym: brutex_core::symbol::Symbol| -> u64 { held.get(&sym).copied().unwrap_or(0) };
    listing.sort_unstable_by_key(|(key, _)| {
        (
            bars_of(key.underlying) == 0,
            key.kind != brutex_core::instrument::Kind::Index,
            key.underlying,
        )
    });

    let mut out = String::from("[");
    for (n, (key, entry)) in listing.into_iter().enumerate() {
        let bars = bars_of(key.underlying);
        // THE TRACKED UNIVERSE ONLY — the same predicate the page uses.
        //
        // This iterated the whole master and shipped 2,780 listings while the
        // page beside it said 785. A type-ahead that suggests instruments the
        // operator's own universe excludes is worse than none: it offers a
        // symbol, the click lands on a page that does not list it, and nothing
        // says why. The bound is ~800 by decision, and this is one of the
        // places that must honour it rather than one of the places that
        // quietly does not.
        if n > 0 {
            out.push(',');
        }
        let canonical = key.to_string();
        let _ = write!(
            out,
            // EXCHANGE AND SEGMENT TRAVEL WITH THE ROW, because the store is
            // keyed on them: the path IS the index, so a chart request names
            // every part rather than sending a synthetic id the server would
            // have to resolve back into one.
            // `universe` and `held` travel with the row so the browser can
            // GROUP without asking again: F&O vs NIFTY Total Market vs index is
            // the distinction the operator reads, and it is a bitset here.
            //
            // TWO UNIVERSE FIELDS, AND BOTH ARE LOAD-BEARING.
            //   `universe`  — the frozen one. `index`, `fno`, `ntm`, those
            //                 joined by `+`, or `other`, and never anything
            //                 else. The pages in production split it on `+`;
            //                 widening it would change a field they already
            //                 read. See `universe_label`.
            //   `universes` — the whole set, one token per bit, including the
            //                 four NIFTY tiers D-0089 appended. A new reader
            //                 uses this and needs no split; an old reader never
            //                 sees it. See `universe_tokens`.
            // The second is not a rename of the first and the first is not
            // deprecated by the second: an instrument's membership is a set,
            // and the string can only ever be a lossy view of it.
            r#"{{"symbol":{},"key":{},"kind":{},"exchange":{},"segment":{},"universe":{},"universes":{},"bars":{bars},"href":{}}}"#,
            render::json_string(key.underlying.as_str()),
            render::json_string(&canonical),
            render::json_string(&format!("{:?}", key.kind)),
            render::json_string(key.exchange.as_str()),
            render::json_string(key.segment.as_str()),
            render::json_string(&universe_label(entry.universe)),
            universes_json(entry.universe),
            render::json_string(&format!(
                "/instruments?q={}",
                render::query_value(&canonical)
            )),
        );
    }
    out.push(']');

    // THE READ'S OWN VERDICT, ON THE WIRE. `read.status()` and `read.master`
    // were computed at startup and consulted by `/health` and the HTML pages
    // and by nothing on this route, which is the whole defect: the numbers
    // below are only measurements when the master decoded and the counter
    // loaded, and until now nothing said which of those held.
    let census = censuses.iter().find(|c| c.vendor == feed);
    let (master, master_note) = site.read.master(feed);
    let mut headers = census_headers(census);
    headers.insert(
        axum::http::HeaderName::from_static(UNIVERSE_STATUS_HEADER),
        note_header(site.read.status()),
    );
    headers.insert(
        axum::http::HeaderName::from_static(MASTER_STATE_HEADER),
        axum::http::HeaderValue::from_static(master.name()),
    );
    headers.insert(
        axum::http::HeaderName::from_static(MASTER_NOTE_HEADER),
        note_header(&master_note),
    );
    // 503 FOR THE TWO STATES WHERE EVERY NUMBER BELOW IS ABSENT RATHER THAN
    // MEASURED, and 200 for everything else — including a merge disagreement,
    // which makes `status()` say `DEGRADED` while the list and the counts are
    // both real. Refusing the type-ahead for a routine ISIN conflict would take
    // the console down for a fact the header already carries.
    let code = if census_is_unreadable(census) || master == MasterState::Unavailable {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    } else {
        axum::http::StatusCode::OK
    };
    (code, headers, out)
}

/// Bars held per symbol, folded out of the census in ONE pass.
///
/// # What this replaces, and what it cost
///
/// The closure this replaces scanned the whole entry vector for every symbol:
///
/// ```text
/// entries.iter().filter(|(series, _)| series.symbol == sym)
/// ```
///
/// and it was the `sort_unstable_by_key` key as well as the emitted field, so
/// the scan ran once per *comparison* — about 22.6 scans per row at n=785, plus
/// one per emitted row: ~18,565 full passes over the census for one request.
/// The cost was the PRODUCT of the tracked universe and the census, on a
/// response whose size never moves. Measured, in an optimised build with no
/// disk I/O, at universe 785: 750 entries 15.5 ms, 9,000 entries 165.9 ms,
/// 43,422 entries 819.2 ms — for 25 KB of JSON. `docs/06-limits.md` §34
/// projects a 93,776-row census, and `/instruments.json` is on the load path of
/// every page in the front end (`web/src/lib/index.svelte.js`).
///
/// Now: one pass over the entries, a hash probe per symbol, `CLAUDE.md` §3
/// rule 4. The per-request cost is O(entries + universe) and no longer their
/// product. `api::server::instrument_bar_counts_are_one_pass_over_the_census`
/// holds the pass count at one for two universe sizes, so a future edit that
/// reintroduces the inner scan fails by cost and not by taste.
///
/// THE NAME WAS WRONG AND THE TEST BENEATH IT WAS EMPTY, and the two failures
/// were one. It read `api::unit::…`, and there is no `unit` module anywhere in
/// this crate — the test is in `server.rs`'s own `mod tests` — while the test
/// itself folded an entries vector of length ZERO and asserted `0 == 0`. Gate
/// 12 was satisfied by both, because it resolves the crate and the final
/// segment and asks whether a proof is NAMED; its own log says it cannot ask
/// whether the claim is true. The module segment is `server` rather than
/// `server::tests` for that resolver's sake: it matches on `crate::mod::fn` and
/// a fourth segment makes it look for an `fn tests`, which is the form every
/// other citation in this file already avoids.
///
/// The iterator is taken generically for that test: it counts what it yields.
fn bars_by_symbol<'a>(
    census: Option<&census::VendorCensus>,
    entries: impl IntoIterator<Item = &'a (census::Series, store::path::YearMonth)>,
) -> std::collections::HashMap<brutex_core::symbol::Symbol, u64> {
    // PRE-SIZED FROM THE ENTRIES, not from the census. Gate 11 rule 3 refuses a
    // map that grows by rehashing, and this function exists to remove a cost —
    // it replaced an O(entries x universe) nested scan, so a map that rehashes
    // its way up hands part of that saving straight back. The ceiling is one
    // symbol per entry: the loop below inserts at most once per entry and
    // usually far fewer, since a symbol repeats across months. `size_hint().0`
    // is a LOWER bound by contract, which is what a capacity wants — never an
    // over-allocation on an iterator that cannot say how long it is.
    let entries = entries.into_iter();
    let mut held = std::collections::HashMap::with_capacity(entries.size_hint().0);
    let Some(census) = census else {
        // NOT AN EMPTY MAP OF ZEROES — an empty map, which every caller reads
        // as "no count for this symbol". The distinction is the one D-0124
        // makes on the wire: an unreadable census is not a store of zeroes.
        return held;
    };
    for (series, month) in entries {
        if let Some(rows) = census.rows_for(&series.at(*month)) {
            *held.entry(series.symbol).or_insert(0) += rows;
        }
    }
    held
}

/// The sentence every route answers with when `feed=` names a vendor this
/// build cannot read.
///
/// # One sentence, three surfaces
///
/// `/universes.json`, `/store.json` and `/store` refuse the same input for the
/// same reason. Three spellings of "no such feed" is three sentences that
/// drift, and an operator who reads two of them learns two different things
/// about one rule — the second-definition failure `CLAUDE.md` §3 rule 1 and
/// D-0013 are both about. The list is built from `Vendor::ALL`, so a fifth
/// vendor is named here with no edit.
///
/// It takes no argument on purpose: what was ASKED FOR belongs to the caller's
/// envelope — a JSON field on one surface, a receipt row on the other — and
/// folding it in here would make the sentence unshareable again.
fn no_such_feed() -> String {
    let known: Vec<&str> = Vendor::ALL.into_iter().map(Vendor::as_str).collect();
    format!(
        "this build reads no feed called that. The feeds it knows are {}.",
        known.join(", ")
    )
}

/// [`no_such_feed`] as the body a JSON route answers with, naming what arrived.
///
/// Both halves are `render::json_string`d: `asked` is whatever was in the query
/// string, so it is untrusted text going onto a wire a browser parses.
fn no_such_feed_json(asked: &str) -> String {
    format!(
        r#"{{"refused":{},"feed":{}}}"#,
        render::json_string(&no_such_feed()),
        render::json_string(asked),
    )
}

/// One sentence saying how much of a target the chosen feed can actually name.
///
/// Three answers, and none of them is a bare number:
///
/// * the feed reaches everything the target holds — the count, and that it is
///   all of it, so an operator is not left wondering what the denominator was;
/// * the feed is short — both numbers and the first names, because "500
///   requested, 486 fetched" with no list is the failure `CLAUDE.md` §4 calls a
///   fallback that hides one;
/// * the feed publishes no instrument master at all — an archive, whose folder
///   of CSVs is its own listing, so the master has nothing to say and says so
///   rather than reporting a zero it did not measure.
///
/// The names are capped at five with the remainder counted, and the cap is
/// stated in the sentence. This is a fixed-width fact on an HTML receipt beside
/// a journal record of 256 bytes; the whole list is on `/universes.json`, which
/// is the surface that can carry it, and the sentence says so.
fn reach_text(target: ingest::SpotTarget, feed: pull::vendor::Feed, site: &Site) -> String {
    /// How many unresolved names the receipt spells before it counts the rest.
    const SHOWN: usize = 5;
    let Some(covered) = feed
        .store_vendor()
        .and_then(|vendor| site.read.coverage.of(vendor, target))
    else {
        return format!(
            "{} publishes no instrument master — a folder of CSVs is its own listing — \
             so the master cannot say, and nothing here counts on its behalf",
            feed.display()
        );
    };
    let short = covered.accounted().saturating_sub(covered.matched);
    if short == 0 {
        return format!(
            "{} — every name this target holds, by {} id",
            covered.matched,
            feed.display()
        );
    }
    let names: Vec<&str> = covered
        .unresolved
        .iter()
        .take(SHOWN)
        .map(|u| u.symbol.as_str())
        .collect();
    let rest = short.saturating_sub(names.len());
    let tail = if rest == 0 {
        String::new()
    } else {
        format!(" and {rest} more")
    };
    format!(
        "{} of {} — {short} cannot be named by {}: {}{tail}. Every one of them, with its \
         reason, is on /universes.json?feed={}",
        covered.matched,
        covered.accounted(),
        feed.display(),
        names.join(", "),
        feed.wire(),
    )
}

/// What each spot target resolves to FOR ONE FEED — the counts, and every name
/// that did not resolve.
///
/// # The route the `/ingest` universe menu was missing
///
/// That menu drew four NIFTY tiers as `no target` and could not do otherwise:
/// nothing on the wire said what a tier resolves to for the selected feed, so a
/// page could either disable the row or synthesise a request the server cannot
/// honour. `/instruments.json` carries the membership — the `universes` array
/// has named `n500`, `n200`, `n100` and `n50` since D-0089 — but membership is
/// not reachability. A NIFTY 50 constituent that Groww's master has no row for
/// is in the tier and cannot be fetched from that feed, and folding
/// `/instruments.json` rows counts it or does not depending on which of two
/// filters a browser happens to apply.
///
/// This answers the question directly, once per feed: the slug to POST as
/// `target=`, the number that belongs on the control, and the buckets behind
/// the shortfall. D-0120.
///
/// # Cost
///
/// One [`coverage::Coverage::of`] per target — seven array indices — plus the
/// string. Nothing walks the universe: [`Read::coverage`] was built at load
/// (D-0039/D-0042), and the only thing that grows with the answer is the
/// `unresolved` list, which is the whole reason the operator asked.
///
/// A feed this build cannot read is REFUSED BY NAME rather than answered as
/// Dhan. `/instruments.json` defaults an unknown feed and that is a shipped
/// behaviour this does not copy: a page that mistypes a feed here would
/// otherwise enable four controls against a set the other broker reaches.
async fn universe_reach_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let json = "application/json; charset=utf-8";
    let asked = param(uri.query().unwrap_or(""), "feed");
    let Some(feed) = ingest::parse_vendor(&asked) else {
        // THE SENTENCE IS NOT WRITTEN HERE ANY MORE. It moved to
        // [`no_such_feed_json`] when `/store.json` and `/store` stopped
        // answering an unknown feed as Dhan and needed the same words.
        return (
            axum::http::StatusCode::BAD_REQUEST,
            [(axum::http::header::CONTENT_TYPE, json)],
            no_such_feed_json(&asked),
        );
    };
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, json)],
        site.read.coverage.json(feed),
    )
}

/// The day a history floor resolves to as of `today`, or `None` when it names
/// no day at all.
///
/// # One arithmetic site, so the page and the pull cannot disagree
///
/// A rolling floor moves every day, so what it *is* has to be computed. It is
/// computed HERE by handing `clamp_to_floor` a window that starts at the epoch
/// and ends today: the clamped window begins exactly at the floor. That is not
/// a trick for its own sake — it means the day `/feeds.json` shows an operator
/// is, by construction, the day their pull will be clamped to. A second
/// implementation of "five years back" would be a second answer.
///
/// `None` for the two floors that name no day: `HistoryFloor::Unbounded`,
/// where the vendor says it holds everything, and `HistoryFloor::Unstated`,
/// where nobody has said anything. The emitted `kind` is what tells those two
/// apart — this function must not, because they resolve identically.
fn floor_oldest(
    floor: pull::vendor::HistoryFloor,
    today: pull::session::Day,
) -> Option<pull::session::Day> {
    use pull::vendor::HistoryFloor;
    match floor {
        HistoryFloor::Unbounded | HistoryFloor::Unstated => None,
        HistoryFloor::Fixed { .. }
        | HistoryFloor::Rolling { .. }
        | HistoryFloor::RollingMonths { .. } => {
            let epoch = pull::session::Day::from_days(0).ok()?;
            let whole = pull::session::Window::new(epoch, today).ok()?;
            // THE DAY THE CALLER NAMED, which is what makes this testable.
            clamp_to_floor(whole, floor, today)
                .ok()
                .map(pull::session::Window::from)
        }
    }
}

/// One history claim as JSON object FIELDS — no braces, so the binding claim
/// can be flattened into its row and a contested one nested under a key.
///
/// # Four kinds, four different words
///
/// `rolling`, `fixed`, `none` and `unknown`. The last two are the pair a
/// caller must never collapse: `none` is the vendor stating it holds
/// everything, `unknown` is nobody having stated anything. Emitting `null` for
/// both and letting the reader guess is exactly the fallback `CLAUDE.md` §4
/// bans — so the day is `null` in both cases and the WORD carries the fact.
///
/// `oldest` is the resolved day and is `null` whenever there is none to
/// resolve, including when the clock could not be read: a `rolling` kind with
/// a null day says "it moves and this answer could not place it", which is
/// still not "no floor".
fn claim_fields(
    claim: Option<pull::vendor::FloorClaim>,
    today: Option<pull::session::Day>,
) -> String {
    use pull::vendor::HistoryFloor;

    // NO CLAIM IS A CLAIM OF NOTHING, not a claim of everything. An unrecorded
    // rung answers `unknown` with a null source.
    let (floor, source, standing) = match claim {
        Some(made) => (made.floor, Some(made.source), Some(made.standing)),
        None => (HistoryFloor::Unstated, None, None),
    };
    let (kind, unit, count) = match floor {
        HistoryFloor::Fixed { .. } => ("fixed", None, None),
        HistoryFloor::Rolling { years } => ("rolling", Some("y"), Some(u64::from(years))),
        HistoryFloor::RollingMonths { months } => ("rolling", Some("m"), Some(u64::from(months))),
        HistoryFloor::Unbounded => ("none", None, None),
        HistoryFloor::Unstated => ("unknown", None, None),
    };
    // The stated date of a fixed floor, rendered from the three numbers the
    // descriptor holds rather than through a calendar — `oldest` below is the
    // one that goes through the clamp, and it is the one a caller compares.
    let from = match floor {
        HistoryFloor::Fixed { year, month, day } => {
            render::json_string(&format!("{year:04}-{month:02}-{day:02}"))
        }
        _ => "null".to_owned(),
    };
    let oldest = match today.and_then(|day| floor_oldest(floor, day)) {
        Some(day) => render::json_string(&day.to_string()),
        None => "null".to_owned(),
    };
    let unit = match unit {
        Some(word) => render::json_string(word),
        None => "null".to_owned(),
    };
    let count = match count {
        Some(n) => n.to_string(),
        None => "null".to_owned(),
    };
    let source = match source {
        Some(text) => render::json_string(text),
        None => "null".to_owned(),
    };
    // WHAT KIND OF SOURCE SAID IT, as the wire's own word.
    //
    // Emitted beside `source` rather than parsed out of it. The page has to
    // say WHICH of two disagreeing claims binds — it drew that from a second,
    // hand-maintained table in the browser until D-0131 deleted it — and
    // deciding it by matching prose against the string "the operator" would be
    // the same second copy wearing a regular expression.
    //
    // `null` only where there is no claim at all, which `kind":"unknown"`
    // already says. A claim always has a standing; that is what makes the
    // binding rule checkable.
    let standing = match standing {
        Some(who) => render::json_string(who.word()),
        None => "null".to_owned(),
    };
    format!(
        r#""kind":{},"unit":{unit},"n":{count},"from":{from},"oldest":{oldest},"source":{source},"standing":{standing}"#,
        render::json_string(kind),
    )
}

/// The wire word for what one record at a feed's finest rung IS.
///
/// Its own function, and not an arm inlined into `finest_fields`, for one
/// reason: `FinestKind::Tick` is constructed by NO descriptor in this build and
/// the `const` block under `DESCRIPTORS` makes a row that claims one a build
/// failure. An arm nothing reaches is an uncovered region, and a region that
/// can never run is the thing `CLAUDE.md` §9's 100% floor has no way to
/// forgive. Lifted out, all three arms are reachable from a test that names
/// them — see `finest_kind_words_are_three_and_the_tick_word_is_one_of_them`.
///
/// The word is `snapshot` rather than `conflated_snapshot` because it is a KEY,
/// and `label` beside it carries the sentence.
const fn finest_kind_word(kind: pull::vendor::FinestKind) -> &'static str {
    match kind {
        pull::vendor::FinestKind::Tick => "tick",
        pull::vendor::FinestKind::ConflatedSnapshot => "snapshot",
        pull::vendor::FinestKind::Bar => "bar",
    }
}

/// One feed's GRANULARITY FLOOR, as a JSON object.
///
/// The finest rung this vendor can **ever** serve, what one record at that rung
/// is, why nothing finer exists in the vendor's own terms, and where those
/// words were read — `pull::vendor::GranularityFloor`, whole, with nothing
/// paraphrased on the way out.
///
/// # Why the whole thing crosses, rather than the number
///
/// Because the number is the half that cannot be trusted alone. Two of the four
/// feeds bottom out at one second and NEITHER serves a tick: what sits on that
/// second is a conflated snapshot of the best bid, the best ask and the best
/// last price, and every print between two snapshots was discarded before the
/// file was written and cannot be recovered by any reader. A page handed `1s`
/// and nothing else is free to write *tick* beside it — a claim about the data
/// that the data does not support, made in a label, which is exactly the
/// substitution `CLAUDE.md` §4 bans.
///
/// So `kind` travels as a word, and `tick_stream` and `conflated` travel beside
/// it as `FinestKind::is_tick_stream` and `FinestKind::is_conflated` — the two
/// questions that decide what may be printed next to a number, answered here by
/// the enum rather than by a reader matching on a string and getting it subtly
/// different. `because` and `source` travel verbatim: `CLAUDE.md` §3 rule 1
/// makes a vendor claim with no source indistinguishable from one somebody
/// typed, and a browser that received the claim without the citation would have
/// nothing to show an operator who asks how it is known.
///
/// One field read, three `const fn` calls and no allocation beyond the strings
/// the descriptor already holds. No filesystem call — this route renders on
/// every page load, which is why the folder's REACH is `/folder.json` and not
/// here.
fn finest_fields(feed: pull::vendor::Feed) -> String {
    let floor = feed.descriptor().granularity_floor;
    format!(
        r#"{{"rung":{},"kind":{},"label":{},"tick_stream":{},"conflated":{},"because":{},"source":{}}}"#,
        render::json_string(floor.finest.dir()),
        render::json_string(finest_kind_word(floor.kind)),
        render::json_string(floor.kind.label()),
        floor.kind.is_tick_stream(),
        floor.kind.is_conflated(),
        render::json_string(floor.because),
        render::json_string(floor.source),
    )
}

/// Every feed this build can read, for the browser's feed selector.
///
/// Built from `DESCRIPTORS`, so a fifth feed appears in the UI the day its row
/// exists and nothing in the front end names a vendor.
///
/// # `kind` — REST or a folder, and it is emitted ONCE
///
/// This route used to carry the same two-valued fact TWICE: a `transport` word
/// (`broker` / `archive`) matched off `Transport`'s two arms right here, and a
/// `kind` word (`rest` / `folder`) read from `pull::vendor::SourceKind`. Two
/// spellings of one split, produced by two independent `match`es in one
/// function, is the defect this endpoint exists to remove rather than an
/// example of it — and the browser had begun cross-deriving them, reading
/// `kind` with a fallback that guessed it from `transport`.
///
/// `transport` is GONE from the wire. `kind`, `kind_label` and `verb` are all
/// `SourceKind`'s, all `const fn`, and the enum is reached exactly once per feed
/// through `Feed::source_kind`. Everything that turns on the split — whether a
/// credential is required, whether a quota exists, whether a history floor is
/// the right question, and whether the honest verb is *pull* or *read* — is
/// asked of that one value.
///
/// # `finest` — how FINE this vendor can ever answer, and who says so
///
/// A second floor, and not the first one. `history` below answers *how far
/// back*; this answers *how fine*, and only one of the two refusals is
/// permanent. See `finest_fields`, which also says why the tick-versus-conflated
/// distinction crosses the wire as a field instead of being left to a reader.
///
/// # `history` — how far back each rung answers, and who says so
///
/// # `history` — how far back each rung answers, and who says so
///
/// The floor is a property of the feed AND of the rung: Groww's own interval
/// table gives its day row "Full history" and its one-minute row "Last 3
/// months". Until `pull::vendor::Descriptor::history` existed there was no
/// field to read, so the /ingest page held that table itself — a vendor fact
/// living in a browser, versioned separately from the vendor row it describes,
/// which is the copy that goes stale. It is emitted here so the page can stop
/// carrying literals.
///
/// One entry per rung the feed serves, PLUS any rung it records a floor for
/// without serving it — `served` says which, and a floor recorded for a
/// withdrawn rung is a fact worth keeping rather than a capability being
/// advertised.
async fn feeds_json(
    axum::extract::State(_site): axum::extract::State<Loaded>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    let mut out = String::from("[");
    // ONE CLOCK READ FOR THE WHOLE ANSWER, so two feeds' rolling floors are
    // resolved against the same day. Reading it per row could straddle
    // midnight and emit two answers that disagree by a day for no reason a
    // reader could see. `None` when the clock is unusable, which every rolling
    // floor below then reports as a null day beside a `rolling` kind — never
    // as an absent floor.
    let today = ingest::ist_day(std::time::SystemTime::now()).ok();
    for (n, feed) in pull::vendor::Feed::ALL.into_iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        // WHERE THIS FEED'S BYTES COME FROM, READ ONCE.
        //
        // Every question below that used to `match` on `Transport`'s two arms
        // separately — the emitted word, and whether an empty store means "not
        // pulled yet" or "not bought" — is asked of this one value. Three
        // independent matches on one enum in one function is three chances to
        // disagree, and the third of them is what decided whether a feed was
        // offered at all.
        let kind = feed.source_kind();

        // THE HISTORY FLOORS, ONE ROW PER RUNG.
        //
        // Walked over the whole ladder rather than over the descriptor's
        // table, so a rung a feed SERVES with no recorded floor still appears
        // — as `unknown`, which is the honest answer and the one the page
        // renders as "claims nothing". A rung that is neither served nor
        // recorded is not emitted at all: there is nothing to say about it.
        let mut history = String::from("[");
        let mut rungs = 0usize;
        for rung in pull::vendor::Granularity::ALL {
            let row = feed.descriptor().history_row(rung);
            let served = feed.serves(rung);
            if !served && row.is_none() {
                continue;
            }
            if rungs > 0 {
                history.push(',');
            }
            rungs += 1;
            let contested = match row.and_then(|r| r.contested) {
                Some(other) => format!("{{{}}}", claim_fields(Some(other), today)),
                None => "null".to_owned(),
            };
            let _ = write!(
                history,
                r#"{{"rung":{},"served":{served},{},"binds_because":{},"contested":{contested}}}"#,
                render::json_string(rung.dir()),
                claim_fields(row.map(|r| r.binding), today),
                render::json_string(row.map_or("", |r| r.binds_because)),
            );
        }
        history.push(']');

        // CAN THIS FEED ACTUALLY SERVE?
        //
        // A feed listed but unusable is worse than one absent: it invites a
        // pull that cannot work and answers with a failure the operator has to
        // decode. TrueData and GDFL are the live case — samples were supplied,
        // the real data has not been bought, and offering them implies a
        // capability that does not exist.
        //
        // The signal is the CENSUS, which is already loaded and is a header
        // read rather than a directory walk — this is a per-render path and an
        // O(files) probe here would be the cost `/store` exists to avoid.
        //
        // NOT HIDDEN, DISABLED, WITH THE REASON. §4 forbids the silent version:
        // a feed that vanishes teaches the operator nothing, and one that says
        // "no data held, and no credential configured" tells them exactly which
        // of the two things to do.
        // THE CENSUS READ IS GONE, and its absence is the fix rather than a
        // tidy-up. It counted what had been INGESTED and was then used to
        // answer whether a feed could BE ingested — the circularity that made
        // both archive feeds unreachable. Readiness below asks the kind and the
        // store prefix; nothing here needs a counter.
        // A BROKER IS READY WITH AN EMPTY STORE. Its credential is what proves
        // entitlement and the first pull is what fills the store; an empty
        // census means "nothing pulled yet", not "not owned". Using the census
        // as the entitlement signal for a broker is circular — it could never
        // be selected to do the pull that would make it selectable.
        //
        // An ARCHIVE is different: there is no credential, so the only evidence
        // of ownership is data read from files the operator bought.
        //
        // ASKED AS "DOES A CREDENTIAL PROVE ENTITLEMENT HERE", which is
        // `SourceKind::needs_credential` and is the actual reason, rather than
        // as a second `match` on the transport that happens to land the same
        // way. The two were separate and could have drifted; the predicate is
        // now the same value `kind`, `kind_label` and `verb` are read from.
        // THE PREDICATE IS THE FEED'S KIND AND ITS STORE PREFIX — NOT ITS CENSUS.
        //
        // `a966a6f` moved archive readiness onto the folder and put the call in
        // the `Some(_)` arm of a match on `held`. That was DEAD CODE, and the
        // audit caught it: `held` is `Some` only when the census is
        // `Census::Held` (`census.rs` returns `None` for `Absent` and
        // `Unreadable`), so a never-ingested TrueData or GDFL produced
        // `held == None`, fell to the `None` arm, and was refused. The fix sat
        // behind the very condition it was written to remove and the
        // circularity survived it — manifest needs an ingest, an ingest needs
        // ready, ready needed the manifest.
        //
        // AND THE ARM IT FELL TO LIED. It said "<feed> has no store prefix",
        // while `Feed::store_vendor` returns `Some(Vendor::TrueData)` and
        // `Some(Vendor::Gdfl)`. The operator was told to buy the archive he had
        // already bought and to give the feed a prefix it already had — two
        // remedies that are both no-ops, so following the page could never
        // clear the state.
        //
        // `held` is therefore not consulted at all. It answered a question
        // about INGEST HISTORY and readiness is a question about ENTITLEMENT.
        let (ready, why) = if kind.needs_credential() {
            // A broker's credential is the entitlement, so it is ready with an
            // empty store: the first pull is what fills it.
            (true, String::new())
        } else if feed.store_vendor().is_none() {
            // The one case the old `None` arm was actually written for, and now
            // the only case that can reach this sentence.
            (
                false,
                format!(
                    "{} has no store prefix, so nothing it pulled could be filed",
                    feed.display()
                ),
            )
        } else {
            // An archive has no credential, so the evidence is the files the
            // operator bought — whatever the census has or has not recorded.
            archive_ready(feed)
        };

        // THE KIND, ITS LABEL AND THE VERB — ALL THREE FROM ONE VALUE, ALL
        // THREE FREE.
        //
        // `kind` is `pull::vendor::SourceKind`, the operator's rule of 12 Aug
        // 2026 as a type. `kind_label` is that type's own words, so a page and
        // a refusal cannot call the same feed two different things — the page
        // used to print `transport`, whose words (`broker`, `archive`) were
        // this file's and nothing else's. `verb` is the honest word for getting
        // data out of it: a folder is READ, never pulled. All three are
        // `const fn`, so none costs a filesystem call and all three can sit on
        // this per-render path.
        //
        // The REACH of a folder feed is NOT here: it cannot be answered
        // without walking the folder, and this route renders on every page
        // load. `/folder.json` answers it on demand — see `crate::folder`.
        // WHICH EXPIRED-F&O SHAPE THIS FEED ANSWERS IN, and it is not a
        // yes/no.
        //
        // Three states, because there are three: a feed with no derivative
        // history at all, one addressed by CONTRACT NAME through a discovery
        // walk, and one addressed by STRIKE OFFSET with no names to discover.
        // The two that serve need entirely different request builders, so a
        // page that knew only "serves F&O" would still send the wrong one.
        //
        // It is here because the browser had no way to ask. Measured on
        // 2026-08-19: the ingest page fired an expired-F&O request at every
        // ticked feed, and the ones at Dhan came back 502 -- correctly, since
        // Dhan publishes no name to discover -- after paying for a credential
        // read and a socket each. A page cannot decline what it cannot see.
        let fno = match feed.descriptor().transport {
            pull::vendor::Transport::Http(spec) => {
                if spec.fno.by_name().is_some() {
                    "by_name"
                } else if spec.fno.by_offset().is_some() {
                    "by_strike_offset"
                } else {
                    "none"
                }
            }
            // AN ARCHIVE HAS NO DISCOVERY ENDPOINT and never will: its expired
            // contracts are files on the operator's disk, not an answer to a
            // GET. Reported as its own word rather than folded into `none`,
            // because "this vendor does not serve it" and "this is not a
            // vendor" send an operator to different places.
            pull::vendor::Transport::LocalArchive(_) => "local_folder",
        };
        let _ = write!(
            out,
            r#"{{"wire":{},"display":{},"kind":{},"kind_label":{},"verb":{},"ready":{ready},"why":{},"fno":{},"finest":{finest},"history":{history}}}"#,
            render::json_string(feed.wire()),
            render::json_string(feed.display()),
            render::json_string(match kind {
                pull::vendor::SourceKind::Rest => "rest",
                pull::vendor::SourceKind::Folder => "folder",
            }),
            render::json_string(kind.label()),
            render::json_string(kind.verb()),
            render::json_string(&why),
            render::json_string(fno),
            finest = finest_fields(feed),
        );
    }
    out.push(']');
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )],
        out,
    )
}

/// One instrument-month of bars, as JSON, for the chart.
///
/// `?feed=<wire>&exchange=NSE&segment=INDEX&symbol=NIFTY&month=YYYY-MM`
///
/// # Paisa all the way out
///
/// Prices leave here as the `i64` paisa they are stored as. The browser divides
/// by 100 exactly once, where a canvas needs a number to draw — `CLAUDE.md` §7
/// says a float has no business near a price, and this endpoint keeps that true
/// right up to the pixel.
///
/// # A positional read per bar, and the whole month is one pass
///
/// `BarFile::read_record` reaches bar N by arithmetic — the header length plus
/// `N × record_stride`, one multiply and one add — and then reads exactly one
/// record's bytes. The records before it are not touched, so the *work* of
/// reaching the last bar of a month is the work of reaching the first.
/// `store::geometry::addressing_is_arithmetic_and_flat` asserts that across a
/// long walk: every index's offset is exactly one stride past the one before
/// it, at index 0 and at 400,000 alike.
///
/// This heading read **"O(1) per bar"** and the sentence under it said "the
/// cost of the last bar equals the cost of the first". The arithmetic is
/// constant and that is what is named above. The `pread` under it has never
/// been timed here, and a page cache is not a bound — so the claim is the
/// shape, not the nanoseconds, and it says which.
///
/// A month is at most 375 × ~22 bars and is sent whole: the chart pans and
/// zooms locally after that, with no request per viewport change.
///
/// `open_interest` is `i64::MIN` when the vendor sent none — the null sentinel
/// §7 reserves. It becomes JSON `null` rather than a number, because zero means
/// The rung a bars request names, defaulting to the one-minute grid.
///
/// # Why a default at all, and why THIS one
///
/// `?timeframe=` is absent on every link written before the store held a
/// second rung, so refusing an absent value would break bookmarks and the
/// chart's own range strip. One minute is the rung the store has always held
/// and the only one those callers could have meant.
///
/// A value that is PRESENT and unknown is refused by name rather than
/// defaulted: `?timeframe=1hour` is a question about a rung this store has no
/// directory for, and answering it with one-minute bars would file the answer
/// under a length nobody asked for. `CLAUDE.md` §4.
///
/// The bars page with no rows and a named trouble, at a given status.
///
/// Extracted so the handler stays inside the workspace's 100-line ceiling: the
/// refusal arms all render the SAME page shape and differ only in the sentence
/// and the status, and repeating the eleven-field literal per arm is how two of
/// them drift apart.
/// The six parts that address one bar file, as the page renders them.
///
/// Grouped because they travel together and are meaningless apart: a refusal
/// that names five of six is what sent the operator to `…/1min/…` for a store
/// holding `1day`. Passing them as one value also keeps `empty_bars_page`
/// inside the workspace's argument ceiling — the lint and the design agree.
struct BarsAddress<'a> {
    symbol: &'a str,
    segment: &'a str,
    vendor: &'a str,
    month: &'a str,
    timeframe: &'a str,
}

fn empty_bars_page(
    status: axum::http::StatusCode,
    site: &Site,
    at: &BarsAddress<'_>,
    trouble: &str,
) -> (axum::http::StatusCode, String) {
    (
        status,
        render::bars_page(&render::BarsView {
            symbol: at.symbol,
            segment: at.segment,
            vendor: at.vendor,
            month: at.month,
            rows: &[],
            total: 0,
            page: 0,
            last_page: 0,
            timeframe: at.timeframe,
            trouble: Some(trouble),
            store_root: &site.store_root.display().to_string(),
        }),
    )
}

/// The rung a bars request names, defaulting to the one-minute grid.
///
/// # Why a default at all, and why THIS one
///
/// `?timeframe=` is absent on every link written before the store held a
/// second rung, so refusing an absent value would break bookmarks. One minute
/// is the rung the store has always held and the only one those callers could
/// have meant.
///
/// A value that is PRESENT and unknown is refused BY NAME rather than
/// defaulted: `?timeframe=1hour` is a question about a rung this store has no
/// directory for, and answering it with one-minute bars would file the answer
/// under a length nobody asked for. `CLAUDE.md` §4.
///
/// # Errors
///
/// The named rung has no directory in this store.
///
/// One comparison per known rung — two — so this is constant work.
fn timeframe_param(query: &str) -> Result<store::path::Timeframe, String> {
    let raw = param(query, "timeframe");
    if raw.is_empty() {
        return Ok(store::path::Timeframe::MINUTE_1);
    }
    // THE STORE'S OWN LIST, NOT A SECOND COPY.
    //
    // This named `MINUTE_1` and `DAY_1` directly, under a comment claiming the
    // store had no such list. It does: `Timeframe::KNOWN` (store/src/path.rs),
    // which `Timeframe::from_secs` already walks. Naming them here made exactly
    // the second list the comment said it was avoiding — and a third rung added
    // to `KNOWN` would have been silently unreachable from the wire.
    store::path::Timeframe::KNOWN
        .iter()
        .copied()
        .find(|t| t.as_str() == raw)
        .ok_or_else(|| {
            format!(
                "{raw:?} is not a rung this store has a directory for. It holds {}.",
                store::path::Timeframe::KNOWN
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(" and ")
            )
        })
}

/// zero and writing 0 for "not sent" is a lie in the data.
async fn bars_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let query = uri.query().unwrap_or("");
    // A fresh header array per return, because `HeaderName` is not `Copy` and
    // one binding cannot be moved into three arms.
    let json = || {
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json; charset=utf-8",
        )]
    };
    let refuse = |why: String| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            json(),
            format!(r#"{{"error":{}}}"#, render::json_string(&why)),
        )
    };

    let Some(vendor) = ingest::parse_vendor(&param(query, "feed")) else {
        return refuse(format!(
            "{:?} is not a feed this build can read",
            param(query, "feed")
        ));
    };
    // `YYYY-MM`, the same spelling every other route uses.
    let raw_month = param(query, "month");
    let Some(month) = raw_month
        .split_once('-')
        .and_then(|(y, m)| store::path::YearMonth::new(y.parse().ok()?, m.parse().ok()?).ok())
    else {
        return refuse(format!("{raw_month:?} is not a YYYY-MM month"));
    };
    let timeframe = match timeframe_param(query) {
        Ok(tf) => tf,
        Err(why) => return refuse(why),
    };
    let file = match bars::open(
        &site.store_root,
        vendor,
        &param(query, "exchange"),
        &param(query, "segment"),
        &param(query, "symbol"),
        timeframe,
        month,
    ) {
        Ok(file) => file,
        Err(why) => return refuse(why),
    };

    // THE WHOLE MONTH, in one pass. `page` reads by index, so this is
    // `n_valid` seeks of fixed length and nothing scans.
    let held = usize::try_from(file.header().n_valid).unwrap_or(usize::MAX);
    let (rows, faults) = bars::page(&file, 0, held);

    let mut out = String::with_capacity(rows.len() * 96 + 32);
    out.push('[');
    for (n, bar) in rows.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        // `lightweight-charts` takes UTC seconds. The store holds micros.
        let _ = write!(
            out,
            r#"{{"t":{},"o":{},"h":{},"l":{},"c":{},"v":{},"oi":{}}}"#,
            bar.ts_micros / 1_000_000,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
            if bar.open_interest == store::format::OI_NULL {
                "null".to_owned()
            } else {
                bar.open_interest.to_string()
            }
        );
    }
    out.push(']');

    // A FAULTY RECORD IS NOT SILENTLY SKIPPED. `page` returns what it could
    // read and what it could not; dropping the second half would draw a chart
    // with a hole in it and no way to know.
    if !faults.is_empty() {
        return (
            axum::http::StatusCode::PARTIAL_CONTENT,
            json(),
            format!(
                r#"{{"bars":{out},"faults":{}}}"#,
                render::json_string(&faults.join("; "))
            ),
        );
    }
    (axum::http::StatusCode::OK, json(), out)
}

/// The census as it is on disk RIGHT NOW, not as it was at startup.
///
/// # Why this is read per request
///
/// `Site` loads the census once when the process starts and holds it. That was
/// right when a pull was a separate invocation, and became wrong the moment the
/// served binary could pull: a run through this process writes 772 bar files
/// and 278,112 records, and every page went on showing the startup snapshot.
/// An operator watching a backfill saw nothing move, which reads as a broken
/// run rather than a stale cache.
///
/// It is cheap enough to do per request BECAUSE of what the census is: one
/// header read per vendor plus a mapped entry region, never a directory walk.
/// That is the whole point of `docs/06-limits.md` §32's layer — answering "what
/// do I hold" without listing anything.
///
/// The cached copy on `Site` stays for the pages that were built against it;
/// this is for the ones that must be current.
/// `pub(crate)` so `/audit.json` reads the same fresh census this does, rather
/// than growing a second spelling of "read the manifests now, not at startup".
pub(crate) fn census_now(
    site: &Site,
) -> (
    Vec<census::VendorCensus>,
    Vec<(census::Series, store::path::YearMonth)>,
) {
    let censuses = census::read_all(&site.store_root);
    let entries = census::held_entries(&censuses);
    (censuses, entries)
}

/// Why one percentage is not a number.
///
/// **Five reasons, five different sentences on screen.** A single `null`
/// covering all of them would collapse five distinct facts into one, which is
/// the fallback that hides a failure `CLAUDE.md` §4 bans — "degrade loudly and
/// name the reason". Each variant's [`Unknown::code`] is what goes on the wire
/// and what the page keys its prose off; the page never invents a sentence for
/// a code it does not know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unknown {
    /// The instrument can be split, bonused or otherwise re-based, and no
    /// threshold that would detect it is sourced anywhere in this repository.
    ///
    /// **This is checked FIRST, ahead of whether a close was even recorded.**
    /// It is a standing property of the instrument, not of today's data: an
    /// operator told "no close recorded" would re-ingest the month and expect a
    /// number, and for an equity none would ever arrive. The permanent reason
    /// outranks the temporary one.
    CorporateActionUnverified,
    /// The census holds the month and no close was ever read for it — a
    /// version-1 entry, or a version-2 one written before the file was priced.
    /// `pull::manifest::Closes::UNKNOWN`, D-0067.
    NotRecorded,
    /// The previous month is not in this census. The first month an instrument
    /// holds is the ordinary case; a gap in the middle is the other one.
    NoEarlierMonth,
    /// The base of the ratio is zero paisa. `CLAUDE.md` §7 makes zero a real
    /// price, and `docs/02-store-format.md` §3 makes an all-zero record a legal
    /// flat bar, so this arm is reachable rather than defensive — and a ratio
    /// with no base is undefined, never `0`, never `∞`.
    BaseNotPositive,
    /// The move times 10,000 leaves `i64`. Refused rather than wrapped.
    Overflow,
}

impl Unknown {
    /// The wire code. `snake_case`, stable, and the page's key.
    const fn code(self) -> &'static str {
        match self {
            Self::CorporateActionUnverified => "corporate_action_unverified",
            Self::NotRecorded => "not_recorded",
            Self::NoEarlierMonth => "no_earlier_month",
            Self::BaseNotPositive => "base_not_positive",
            Self::Overflow => "overflow",
        }
    }
}

/// Whether a corporate action can re-base this segment's prices.
///
/// `docs/05-decisions.md` D-0018, in its own words: **"Indices never split;
/// equities do."** An index is a computed level, so a constituent's split moves
/// the divisor and not the level. Cash equities split, bonus and consolidate.
/// F&O contracts are written on those equities and NSE adjusts the contract
/// when the underlying is adjusted, so a derivative inherits the hazard.
///
/// The match is exhaustive on purpose: a fourth segment fails to compile here
/// rather than defaulting into whichever answer was written last.
const fn can_be_rebased(segment: brutex_core::instrument::Segment) -> bool {
    match segment {
        brutex_core::instrument::Segment::Index => false,
        brutex_core::instrument::Segment::Cash | brutex_core::instrument::Segment::Fno => true,
    }
}

/// Basis points between two paisa closes, in integer arithmetic throughout.
///
/// `CLAUDE.md` §7: prices are paisa integers, never a float. A ratio derived
/// from money is the same arithmetic and gets the same treatment —
/// `clippy::float_arithmetic` is a workspace deny (D-0061) and no `f64` appears
/// at any step here. **A month that moved 1.25% is `125`.**
///
/// # Rounding: half AWAY FROM ZERO, not half up
///
/// §7's half-up rule is about snapping a *price* to the tick grid at the *write
/// boundary*. This is neither a price nor a write. Half-up would render −0.5 bp
/// as `0` and +0.5 bp as `1`, so a gain and its mirror-image loss would print
/// different magnitudes — visible the moment the column is sorted. Away from
/// zero is symmetric. D-0069.
///
/// # Why `i64` and not `i32`
///
/// `i32` is unsafe and the counterexample uses only values the store can hold:
/// a base of `1` paisa and a last close of ₹2,147.50 — an ordinary NSE share
/// price — gives 2,147,490,000 bp, past `i32::MAX`. Proven by
/// `api::server::basis_points_do_not_fit_i32_and_an_ordinary_price_proves_it`.
///
/// # Preconditions, carried by the type rather than by a comment
///
/// Both arguments come from `pull::manifest::Closes::paisa`, and
/// `Closes::known` refuses every negative, so both are in `0..=i64::MAX`. That
/// is what makes the subtraction below unable to overflow, and it is why there
/// is no `checked_sub` arm no input could ever enter.
///
/// # Errors
///
/// [`Unknown::BaseNotPositive`] when the base is zero, [`Unknown::Overflow`]
/// when the scaled move leaves `i64`.
fn basis_points(first_paisa: i64, last_paisa: i64) -> Result<i64, Unknown> {
    // A RATIO NEEDS A BASE, AND ZERO IS NOT ONE. Not rendered as 0.00%, not as
    // infinity, not as an empty cell that means five other things.
    if first_paisa <= 0 {
        return Err(Unknown::BaseNotPositive);
    }
    // Both operands are in `0..=i64::MAX`, so this is in `-i64::MAX ..=
    // i64::MAX` and cannot overflow. See the precondition above.
    let delta = last_paisa - first_paisa;
    let Some(scaled) = delta.checked_mul(10_000) else {
        return Err(Unknown::Overflow);
    };
    // `first_paisa >= 1`, so neither the divide-by-zero nor the `i64::MIN / -1`
    // case exists. Rust truncates toward zero, so the remainder carries the
    // sign of `scaled` and the correction below is away from zero.
    let whole = scaled / first_paisa;
    let rest = scaled % first_paisa;
    let (step, magnitude) = if rest >= 0 { (1, rest) } else { (-1, -rest) };
    // `magnitude * 2 >= first_paisa`, written as a subtraction because
    // `magnitude < first_paisa` makes `first_paisa - magnitude` safe and the
    // doubling would not be.
    if magnitude >= first_paisa - magnitude {
        // Rounding at all requires `first_paisa >= 2` — at a base of 1 the
        // remainder is always 0 — so `|whole| <= i64::MAX / 2` here and the
        // step cannot overflow.
        return Ok(whole + step);
    }
    Ok(whole)
}

/// One month's own percentage change: its first close to its last close.
///
/// # Why intra-month and not month-over-month
///
/// A row of this table is an instrument-**month**, so "previous % change" is
/// the previous month's *same statistic*, not the previous bar's. Given that,
/// the month's own change is a property of the row itself: it needs no
/// neighbour, it is defined for an instrument's first held month, and the
/// "previous" column costs one probe instead of two. D-0069 records the
/// alternative and what it was measured to cost.
///
/// # `closes` is an `Option`, and that is what makes both columns one function
///
/// `None` is "the census does not hold that month" — reachable only from the
/// **previous**-month call site, because the current month is in hand by the
/// time this is asked about it. Passing the census's answer rather than a
/// pre-decided verdict is what puts the corporate-action gate ahead of the
/// missing-neighbour case in *both* columns. It has to be: an equity told
/// `no_earlier_month` invites an operator to ingest the previous month and wait
/// for a number that a sourced threshold, not a pull, is what unlocks.
///
/// # Errors
///
/// [`Unknown::CorporateActionUnverified`] for anything that can be re-based,
/// [`Unknown::NoEarlierMonth`] when the census holds no such month,
/// [`Unknown::NotRecorded`] when it holds the month and no close, and whatever
/// [`basis_points`] refuses.
fn month_change(
    segment: brutex_core::instrument::Segment,
    closes: Option<pull::manifest::Closes>,
) -> Result<i64, Unknown> {
    // FIRST, AND UNCONDITIONALLY. `docs/05-decisions.md` D-0018 decided the
    // behaviour — refuse the window loudly, never back-adjust from a source no
    // vendor has been verified to supply — and grep finds zero implementation
    // of the detector in `crates/`. No threshold is sourced in this repository,
    // so an equity's percentage is refused rather than printed with a caveat
    // beside it: the number travels and the caveat does not.
    if can_be_rebased(segment) {
        return Err(Unknown::CorporateActionUnverified);
    }
    let Some(closes) = closes else {
        return Err(Unknown::NoEarlierMonth);
    };
    let Some((first, last)) = closes.paisa() else {
        return Err(Unknown::NotRecorded);
    };
    basis_points(first, last)
}

/// The month before this one, or `None` at the first month the store addresses.
///
/// Arithmetic on the calendar, never a loop over it, and there is deliberately
/// **no** `next`: `CLAUDE.md` §3 rule 7 says a computation at month N may read
/// months 0..N, and the cheapest enforcement is that the function that walks
/// forward does not exist.
fn month_before(month: store::path::YearMonth) -> Option<store::path::YearMonth> {
    let (year, ordinal) = if month.month() > 1 {
        (month.year(), month.month() - 1)
    } else {
        // `YearMonth::new` refuses a year below 1970, and that refusal is this
        // function's `None`: 1970-01 has no month before it that the store can
        // name. `month.year()` is at least 1970, so the subtraction is safe.
        (month.year() - 1, 12)
    };
    store::path::YearMonth::new(year, ordinal).ok()
}

/// The state of the selected feed's census, as one stable word.
///
/// `held` · `absent` · `unreadable` — [`census::Census::name`]'s words, which
/// are `/audit.json`'s words. A feed with no census row in this build reads
/// `absent`, exactly as `audit_json::store_block` renders it, so the two
/// surfaces cannot drift into two vocabularies for one fact.
pub const CENSUS_STATE_HEADER: &str = "x-brutex-census-state";
/// The sentence [`census::VendorCensus::note`] produces for that state.
pub const CENSUS_NOTE_HEADER: &str = "x-brutex-census-note";
/// What loading the census had to step over, or empty when it stepped over
/// nothing. `/audit.json` writes `null` here; a header has no `null`, and an
/// **always-present, sometimes-empty** value keeps "nothing was stepped over"
/// and "this build does not say" apart — an absent header is the second.
pub const CENSUS_DEGRADED_HEADER: &str = "x-brutex-census-degraded";
/// [`Read::status`] — `ok` or `DEGRADED`, the same word `/health` answers with.
pub const UNIVERSE_STATUS_HEADER: &str = "x-brutex-universe-status";
/// [`MasterState::name`] for the feed the request named.
pub const MASTER_STATE_HEADER: &str = "x-brutex-master-state";
/// The sentence behind [`MASTER_STATE_HEADER`].
pub const MASTER_NOTE_HEADER: &str = "x-brutex-master-note";

/// The longest a stamped note may be.
///
/// A note carries a filesystem path and a refusal in the refusal's own words,
/// and neither is bounded by anything this crate owns. `docs/07-o1-architecture.md`
/// law 5 is bound every input at the boundary, and a response header block is a
/// boundary: an unbounded one is a request-sized allocation an operator's own
/// directory name decides. Truncation is marked, never silent.
const NOTE_HEADER_CHARS: usize = 400;

/// One note, in the only alphabet a header value may hold.
///
/// # Why a sanitiser rather than a fallible conversion
///
/// `HeaderValue::from_str` refuses anything outside visible ASCII, and every
/// string stamped here holds a path — which on this platform is arbitrary bytes
/// — and a vendor's own refusal text, which has already been observed to carry
/// `·` and `—`. A `unwrap_or(<empty>)` at the call site would answer a corrupt
/// census with a BLANK reason, which is the failure this whole change exists to
/// remove, arriving one layer lower down. So the alphabet is enforced here and
/// the conversion below cannot fail.
///
/// Every character outside `0x20..=0x7E` becomes `?`; the count is preserved so
/// a mangled note still reads as a note. Truncation is marked with `...`.
///
/// # Why the conversion below has no failure arm
///
/// `HeaderValue::from_str` refuses a byte outside `0x20..=0x7E`, and every byte
/// this builds is inside it. A `Result` arm here would be a project region no
/// input could enter, which the coverage gate cannot hold and `unreachable!()`
/// would only rename. [`note_alphabet`] is the guarantee, and
/// `api::server::a_hostile_note_is_still_a_header` is what holds it up.
#[allow(
    clippy::expect_used,
    reason = "`note_alphabet` emits only 0x20..=0x7E, which is exactly the \
              alphabet a header value admits, so this cannot refuse. A fallback \
              arm would be a project region no input could enter, which the \
              coverage gate cannot hold and `unreachable!()` would only rename; \
              the panic named here lives in `http`."
)]
fn note_header(text: &str) -> axum::http::HeaderValue {
    axum::http::HeaderValue::from_str(&note_alphabet(text))
        .expect("a sanitised note is visible ASCII")
}

/// One note, reduced to the alphabet a header value admits and bounded.
///
/// Split from [`note_header`] so the alphabet is assertable directly, against
/// inputs a fixture can hold — a header value's own accessor hands back bytes
/// and would make the test about `http`'s parser rather than about this rule.
fn note_alphabet(text: &str) -> String {
    let mut clean = String::with_capacity(text.len().min(NOTE_HEADER_CHARS));
    for (n, ch) in text.chars().enumerate() {
        if n >= NOTE_HEADER_CHARS {
            clean.push_str("...");
            break;
        }
        clean.push(if ch.is_ascii_graphic() || ch == ' ' {
            ch
        } else {
            '?'
        });
    }
    clean
}

/// The three census facts, stamped on a JSON response that cannot carry them
/// in its body.
///
/// # Why headers and not a field
///
/// `/store.json` answers a JSON **array** and `/db` reads it as one —
/// `rows = Array.isArray(j) ? j : []`. Wrapping the rows in an object to make
/// room for a status would turn every existing reader into a reader of `[]`,
/// which is the very outcome being fixed. A header is additive: an array stays
/// an array, and a reader that wants the reason asks for it by name.
///
/// # What a page must read
///
/// `r.headers.get('x-brutex-census-state')` — `unreadable` means the array is
/// EMPTY BECAUSE THE COUNTER IS DAMAGED, not because the store is. The sentence
/// to show is `x-brutex-census-note`. The response also carries a non-200 for
/// that state, so a reader that only checks `r.ok` stops calling it empty.
/// D-0124.
fn census_headers(census: Option<&census::VendorCensus>) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(
        axum::http::HeaderName::from_static(CENSUS_STATE_HEADER),
        axum::http::HeaderValue::from_static(match census.map(|c| &c.state) {
            Some(census::Census::Held { .. }) => "held",
            Some(census::Census::Unreadable { .. }) => "unreadable",
            // A feed with no row is `absent`, which is what
            // `audit_json::store_block` already answers for the same input.
            Some(census::Census::Absent) | None => "absent",
        }),
    );
    headers.insert(
        axum::http::HeaderName::from_static(CENSUS_NOTE_HEADER),
        note_header(&census.map_or_else(
            || String::from("no census row for this feed in this build"),
            census::VendorCensus::note,
        )),
    );
    headers.insert(
        axum::http::HeaderName::from_static(CENSUS_DEGRADED_HEADER),
        note_header(
            &census
                .and_then(census::VendorCensus::degraded)
                .unwrap_or_default(),
        ),
    );
    headers
}

/// Whether the selected feed's counter refused to load.
///
/// The one state where a body of `[]` and a body of every row this store holds
/// are produced by the same code path, and the reason a status code moves.
fn census_is_unreadable(census: Option<&census::VendorCensus>) -> bool {
    matches!(
        census.map(|c| &c.state),
        Some(census::Census::Unreadable { .. })
    )
}

/// The census row for one key — the counters and the closes in **one probe**.
///
/// `pull::manifest::Manifest::held` is one hash probe into a table the load
/// walk built once, so this reads `rows` and both closes for the cost the old
/// `rows_for` paid for `rows` alone. Proven by
/// `api::server::a_row_and_its_neighbour_cost_two_probes_and_no_file`.
fn held_row(
    census: &census::VendorCensus,
    key: &pull::manifest::EntryKey,
) -> Option<pull::manifest::Held> {
    match census.state {
        census::Census::Held { ref manifest } => manifest.held(key),
        census::Census::Absent | census::Census::Unreadable { .. } => None,
    }
}

/// One row of `/store.json`, appended to `out`.
///
/// Written as `"chg_bps": <n>, "chg_why": null` **or** `"chg_bps": null,
/// "chg_why": "<code>"` — both keys always present, exactly one of them a
/// value. A key that is sometimes absent makes "unknown" and "the field does
/// not exist in this build" the same thing on the wire, and the browser cannot
/// tell them apart.
fn write_change(out: &mut String, key: &str, change: Result<i64, Unknown>) {
    match change {
        Ok(bps) => {
            let _ = write!(out, r#","{key}_bps":{bps},"{key}_why":null"#);
        }
        Err(why) => {
            let _ = write!(
                out,
                r#","{key}_bps":null,"{key}_why":{}"#,
                render::json_string(why.code())
            );
        }
    }
}

/// The body of `/store.json` for one feed.
///
/// Split out of the handler so every arm is reachable from a test without a
/// socket — the same argument this module's own header makes about `main.rs`.
///
/// # Cost
///
/// **Two hash probes per row and no syscall**: this month's row, and the same
/// series at [`month_before`]. Crossing a month crosses a manifest *entry*, not
/// a file — no `open`, no `stat`, no `pread`, because the census is already
/// resident. Proven by
/// `api::server::a_row_and_its_neighbour_cost_two_probes_and_no_file`.
///
/// What is **not** constant is the caller: `census_now` re-reads and re-walks
/// every manifest per request, which is O(entries) and was before this existed.
/// `docs/06-limits.md` §41 records it rather than this comment hiding it.
fn store_body(
    censuses: &[census::VendorCensus],
    entries: &[(census::Series, store::path::YearMonth)],
    feed: Vendor,
) -> String {
    // ONE LOOKUP FOR THE WHOLE RESPONSE, not one per row. `Vendor::ALL` is four
    // long so the old per-row `find` was cheap, but it was O(vendors) inside an
    // O(entries) loop for a value that cannot change between iterations — and
    // a feed with no census at all is now one comparison rather than one per
    // entry that could never match.
    let Some(census) = censuses.iter().find(|c| c.vendor == feed) else {
        return String::from("[]");
    };
    // THE ANSWER SAYS WHO PRODUCED IT. Until this field existed no byte of a
    // `/store.json` body named the feed, so a reader that got the wrong one had
    // no way to notice — which is half of why substituting Dhan for an unknown
    // feed went unseen for as long as it did. The other half is now a `400`;
    // see `store_json`.
    //
    // ENCODED ONCE, not once per row. `Vendor::as_str` is a fixed token, and
    // allocating its quoted form again for each of the 43,422 entries
    // `docs/06-limits.md` §34 projects is a per-row cost with no reader.
    let feed_json = render::json_string(feed.as_str());
    let mut out = String::from("[");
    let mut n = 0usize;
    for (series, month) in entries {
        let Some(row) = held_row(census, &series.at(*month)) else {
            continue;
        };
        if n > 0 {
            out.push(',');
        }
        n += 1;
        // THE ROW'S OWN RUNG, NOT A LITERAL.
        //
        // This wrote `"timeframe":"1min"` on every row, unconditionally, while
        // `Series` has carried the real `Timeframe` since D-0055 and the store
        // has filed under `1day/` and `1min/` accordingly. The consequences all
        // landed on the reader, and all of them looked like data faults:
        //
        //   * `/db`'s TF column was a sort control over one repeated value —
        //     20,516 identical cells that could never differ.
        //   * `/db`'s completeness divided DAILY bars by a 375-bar MINUTE
        //     session, so a complete 1day month read as ~99.7% missing.
        //   * the Markets chart asked for `…/1min/2021-08.bin` for an
        //     instrument stored under `1day/`, and answered "That read failed"
        //     about a file that was never supposed to exist.
        //
        // The store was right the whole time. `Timeframe::as_str` is the SAME
        // word the path segment uses — `1min`, `1day` — so a reader that keys
        // off this string and a reader that keys off the directory cannot
        // disagree again.
        // THE WINDOW THIS MONTH ACTUALLY COVERS, not just how many bars it
        // holds. The census has carried `first_ts_micros` and `last_ts_micros`
        // per entry since it was written; nothing put them on the wire, so a
        // reader could say "21 bars" and not "09 Aug 2021 to 31 Aug 2021".
        // Deriving them from the month name would be a guess — a month holding
        // one bar covers one day, and which day is a fact only the entry has.
        //
        // Microseconds, unconverted. The browser owns the IST rendering, the
        // same way it owns the decimal point on basis points: one encoding on
        // the wire, one place that formats it. `CLAUDE.md` §7's reasoning
        // applied to time rather than money.
        let _ = write!(
            out,
            r#"{{"feed":{feed_json},"instrument":{},"month":{},"timeframe":{},"rows":{},"first_ts":{},"last_ts":{}"#,
            render::json_string(&series.to_string()),
            render::json_string(&month.to_string()),
            render::json_string(series.timeframe.as_str()),
            row.entry.rows,
            row.entry.first_ts_micros,
            row.entry.last_ts_micros
        );
        write_change(
            &mut out,
            "chg",
            month_change(series.segment, Some(row.closes)),
        );
        // THE PREVIOUS MONTH IS A PROBE, NOT A SCAN, AND NOT AN ASSUMPTION. A
        // month the census does not hold is `no_earlier_month` — never zero,
        // and never this month's number repeated.
        let before = month_before(*month).and_then(|at| held_row(census, &series.at(at)));
        write_change(
            &mut out,
            "prev_chg",
            month_change(series.segment, before.map(|held| held.closes)),
        );
        out.push('}');
    }
    out.push(']');
    out
}

/// What the store holds for ONE feed, as JSON, for the DB page.
///
/// `?feed=<wire>` — and one feed only. No response from this endpoint ever
/// carries two feeds' numbers, because no page in this product compares them.
///
/// Percentage change is returned as INTEGER BASIS POINTS, not a float. A month
/// that moved 1.25% is `125`, and the browser inserts the decimal point for
/// display exactly as it does for prices. `CLAUDE.md` §7 bans the float for
/// money; a ratio derived from money is the same arithmetic and gets the same
/// treatment.
///
/// **That paragraph was a promise nothing kept.** It described this shape while
/// the response carried four fields and no price at all; D-0067 put the two
/// closes in the census and D-0069 made the arithmetic real. Four fields carry
/// it now, and every one of them is total:
///
/// | Field | Meaning |
/// |---|---|
/// | `chg_bps` | this month's first close to its last close, in basis points — `125` is +1.25% — or `null` |
/// | `chg_why` | `null` when `chg_bps` is a number, otherwise the reason code it is not |
/// | `prev_chg_bps` | the **previous month's** same statistic, or `null` |
/// | `prev_chg_why` | the reason `prev_chg_bps` is `null`, or `null` |
///
/// Exactly one of each pair is non-`null`, always, on every row. An unknown
/// percentage is `null` and a named reason — it is **never** `0`, because zero
/// is a real month that closed where it opened. The five codes are
/// [`Unknown::code`]: `corporate_action_unverified`, `not_recorded`,
/// `no_earlier_month`, `base_not_positive`, `overflow`.
///
/// # An empty array is not one fact, and it used to be answered as one
///
/// `held_row` folds `Census::Absent` and `Census::Unreadable` into the same
/// `None`, so every row was skipped either way and the body was `[]` at 200
/// with nothing else on the wire. **A corrupt manifest over a store holding
/// millions of bars was byte-identical to a store holding none** — asserted
/// equal, socket to socket — and `/db` then rendered "this feed holds no bars",
/// which is a claim about the store made from a fact about its counter.
///
/// Two things now separate them, and neither changes the body's SHAPE:
///
/// * the status is `503` when the counter would not load, so a reader that only
///   tests `r.ok` leaves the success path;
/// * [`CENSUS_STATE_HEADER`], [`CENSUS_NOTE_HEADER`] and
///   [`CENSUS_DEGRADED_HEADER`] carry `/audit.json`'s own three words.
///
/// A consumer expecting an array still receives an array, in every state **of
/// the store**. D-0124.
///
/// # The one answer that is not an array, and why it is not a state of the store
///
/// A `feed=` this build cannot read is refused: `400`, and
/// [`no_such_feed_json`]'s `{"refused":…,"feed":…}` — the same body
/// `/universes.json` has answered since D-0120. Nothing was looked at, because
/// there was nothing to look at, so there is no census state to report and the
/// array shape has nothing to carry.
///
/// It used to be answered as **Dhan**, at `200`, with no field on the wire
/// saying which feed replied. That made "the store holds nothing for Groww" and
/// "you spelled Groww wrong" byte-identical — a real vendor's counters under
/// another vendor's label, which is the fallback that hides a failure
/// `CLAUDE.md` §4 bans and exactly what [`bars_html`] already refuses for
/// `?vendor=`.
///
/// The array promise survives where it is load-bearing. `/db` reads this body
/// as `rows = Array.isArray(j) ? j : []` (see [`census_headers`]), so the
/// object lands as an empty list rather than as a parse error, and `r.ok` is
/// already false at `400`. What it does NOT survive is a reader that parses the
/// body before checking the status and indexes it as an array; nothing here
/// measures whether such a reader exists, and `web/src` is not one.
///
/// An **absent** `feed=` is untouched and still means Dhan.
/// [`ingest::parse_vendor`] answers the empty string itself, so the `None` this
/// arm matches is only ever a feed that was NAMED and is not one this build
/// reads. Every row now carries `"feed"` as well, so the default states itself
/// instead of being assumed.
/// THE SCRUB, over HTTP — the counter checked against the files it counts.
///
/// # Why this route exists and `/store.json` does not answer the question
///
/// Every completeness surface in this product — `/store.json`, `/db`, the
/// coverage grid, and `crate::ladder`'s gate that decides whether the next rung
/// may run — answers from the census, which is one file validated only against
/// itself. This is the only route that OPENS A BAR FILE to check the census is
/// telling the truth about it.
///
/// # It never repairs
///
/// `pull::scrub` compares and this reports. A counter silently corrected before
/// an operator saw the disagreement is a repair nobody audited — `CLAUDE.md`
/// §4 wants the reason surfaced rather than swallowed by a fix.
///
/// # Cost
///
/// O(1) per entry, and the walk is the ask: a scrub of a vendor is a scrub of
/// every month it claims. Nothing is sorted and nothing is read whole.
async fn verify_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    let query = uri.query().unwrap_or("");
    let asked = param(query, "feed");
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
    );
    let Some(feed) = ingest::parse_vendor(&asked) else {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            no_such_feed_json(&asked),
        );
    };
    // FRESH, NEVER THE STARTUP SNAPSHOT. A scrub answering from a census read
    // at boot would verify a store that has since been written to.
    let (censuses, _entries) = census_now(&site);
    let Some(census) = censuses.iter().find(|c| c.vendor == feed) else {
        return (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            headers,
            no_such_feed_json(&asked),
        );
    };

    let report = crate::verify::vendor(&site.store_root, census);
    // A DISAGREEMENT IS NOT A SERVER FAULT, so it is 200 with the finding in
    // the body: the request was answered correctly and the ANSWER is bad news.
    // A refusal — a counter that could not be read at all — is 503, because
    // then the question was never asked.
    let code = if report.refused.is_some() {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    } else {
        axum::http::StatusCode::OK
    };
    let t = report.tally;
    let named = report
        .named
        .iter()
        .map(|one| render::json_string(one))
        .collect::<Vec<_>>()
        .join(",");
    let body = format!(
        "{{\"feed\":{},\"verified\":{},\"say\":{},\"seen\":{},\"agreed\":{},\
         \"missing\":{},\"rows\":{},\"bounds\":{},\"unreadable\":{},\
         \"undrawn\":{},\"findings\":[{named}],\"refused\":{}}}",
        render::json_string(feed.as_str()),
        report.verified(),
        render::json_string(&report.say()),
        t.seen(),
        t.agreed,
        t.missing,
        t.rows,
        t.bounds,
        t.unreadable,
        report.undrawn,
        report
            .refused
            .as_ref()
            .map_or_else(|| "null".to_owned(), |why| render::json_string(why)),
    );
    (code, headers, body)
}

async fn store_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::http::HeaderMap, String) {
    let query = uri.query().unwrap_or("");
    // REFUSED BY NAME, NEVER SUBSTITUTED. The `.unwrap_or(Vendor::Dhan)` that
    // stood here was unreachable for an ABSENT feed — `parse_vendor` answers
    // the empty string with Dhan itself — so the only input it ever caught was
    // a feed somebody NAMED and this build does not read. See the doc block
    // above for what answering it as Dhan cost.
    let asked = param(query, "feed");
    let Some(feed) = ingest::parse_vendor(&asked) else {
        // The census headers are deliberately NOT stamped: they describe a
        // feed's counter, and there is no feed here to have one. Only the
        // content type, so a browser parses the refusal as what it is.
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json; charset=utf-8"),
        );
        return (
            axum::http::StatusCode::BAD_REQUEST,
            headers,
            no_such_feed_json(&asked),
        );
    };

    // FRESH, NOT THE STARTUP SNAPSHOT. See `census_now`.
    let (censuses, entries) = census_now(&site);
    let census = censuses.iter().find(|c| c.vendor == feed);
    // 503, FOR THE REASON `health` GIVES: a monitor reads the status code and
    // nothing else. The body stays an array so the readers that only want rows
    // are untouched.
    let code = if census_is_unreadable(census) {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    } else {
        axum::http::StatusCode::OK
    };
    let headers = census_headers(census);

    (code, headers, store_body(&censuses, &entries, feed))
}

/// The health endpoint.
///
/// 200 only when the read is clean. A degraded universe answers 503, because a
/// monitor reads the status code and nothing else — this used to return 200
/// with `ok` on the first line while a vendor had never been read.
async fn health(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> (axum::http::StatusCode, String) {
    let (body, clean) = report_from(&site.read);
    let code = if clean {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };
    (code, body)
}

/// What this build can and cannot do, named exactly.
///
/// # This constant replaced one that was false in five separate ways
///
/// It read:
///
/// ```text
/// crates/pull exposes no vendor fetch and no rate governor in this build:
/// there is no pull::fetch and no pull::rate, and docs/04-invariants.md P-01
/// through P-04 still stand at '—' … A request is still parsed, validated and
/// echoed back … and is then REFUSED with 503. No vendor is contacted and
/// nothing is written.
/// ```
///
/// By the time anyone read it: `crates/pull/src/fetch.rs` was 521 lines,
/// `crates/pull/src/rate.rs` was 822, the four invariant rows had moved to
/// `◐ ✓ ✗ ◐`, the local-archive path was writing bar files and a manifest, and
/// a run that stored 62,978 bars still answered `NOT STARTED`. It was written
/// before any of that landed and nothing moved it — the same defect class this
/// repository keeps catching itself with: a comment asserting what the code
/// contradicts. Gate 12 cannot see it, because a stale claim about a *module*
/// is not a cost claim.
///
/// # What is true
///
/// `pull::fetch` is the transport seam and `pull::rate::Governor` is D-0037's
/// adaptive governor; both exist. `crates/pull/src/http.rs` now exists too, and
/// `crates/pull/Cargo.toml` declares `reqwest` — **so a socket can be opened
/// from this process for the first time.** What is still missing is the wiring:
/// `HttpSource` is asynchronous and the `/pull` route is synchronous, so the
/// route does not call it and no credential has yet been read on this path.
///
/// The distinction matters and the banner must keep it. "There is no socket"
/// and "there is a socket nothing calls" are different states, and an operator
/// who is told the first when the second is true will look in the wrong place.
///
/// The claim is checkable rather than taken: every name in it is a path, and
/// `api::server::tests::the_ingest_page_names_what_exists_and_what_does_not` is
/// what stops it going stale silently a second time — which is exactly what it
/// caught when `reqwest` was added.
/// What the page says when the broker IS reachable from this process.
///
/// # This constant exists because its sibling went stale twice
///
/// [`HTTP_UNAVAILABLE`] says "no credential has been read and no vendor is
/// contacted from this process". That was true when it was written. It stopped
/// being true when `broker_answer` landed: the served process sets
/// [`Broker::Live`], reads the token out of Parameter Store, and gets an answer
/// from Dhan — and the page went on saying the opposite, because the constant
/// was rendered unconditionally and nothing consulted [`Site::broker`].
///
/// Its own documentation predicted this. It says an operator "told the first
/// when the second is true will look in the wrong place", and names a test as
/// what "stops it going stale silently a second time". The test asserted the
/// SENTENCE, not the FACT — so it could catch someone editing the words and
/// could not catch the words becoming false. That is the same shape as every
/// other defect found in this repository this week: the check verifies the code
/// against itself.
///
/// The repair is not a better sentence. It is that the two texts are now chosen
/// by [`Site::broker`], which is the value that actually decides, and the test
/// asserts the correspondence rather than either string.
pub const HTTP_LIVE: &str = "THE HTTP PATH IS WIRED TO THIS ROUTE. The credential \
     is read from AWS Parameter Store through pull::ssm, the descriptor drives \
     the request, and pull::http::HttpSource::window_async puts it on a socket. \
     WHICH PATH A PULL TAKES IS DECIDED BY THE FEED'S OWN TRANSPORT, not by \
     whether the folder box is blank: an HTTP feed is asked over the network \
     and an archive feed reads the folder, whatever else the form says. Before \
     anything is spent, the feed's rate budget is charged through \
     pull::rate::Governor, held per feed on the site so its buckets survive \
     between requests — a request that will not be issued costs no credential \
     read and no socket. WHAT IS STILL MISSING, so this sentence does not \
     overstate itself the way its predecessors did: a window longer than the \
     vendor's per-request cap is still sent whole rather than split, so a \
     multi-year range is refused by the vendor or silently truncated by it \
     (pull::session::split_window computes the chunks and has no caller yet); \
     a window that stores nothing still reports STORED; and the expired-F&O \
     endpoints are not modelled at all.";

/// What the page says when the broker is NOT reachable from this process.
///
/// Every test site is [`Broker::Refused`], so this is the text the suite sees.
/// It is still accurate for that state — and it is now reached only through
/// [`halt_for`], never rendered unconditionally, which is the whole repair.
/// See [`HTTP_LIVE`] for what went wrong and why the choice moved to a value.
pub const HTTP_UNAVAILABLE: &str = "THE LOCAL-ARCHIVE PATH RUNS. THE HTTP PATH \
     IS BUILT BUT NOT YET WIRED TO THIS ROUTE. crates/pull/src/fetch.rs, \
     crates/pull/src/rate.rs and crates/pull/src/http.rs are all present — \
     pull::fetch::BarSource is the transport seam, pull::rate::Governor is the \
     adaptive governor of D-0037, and pull::http::HttpSource is the vendor \
     client, driven entirely by the descriptor in pull::vendor. What is missing \
     is one join: HttpSource answers through window_async, this route is \
     synchronous, and nothing here calls it — so no credential has been read \
     and no vendor is contacted from this process. A spot pull that names \
     a local vendor folder therefore RUNS: it reads the files, writes bar files \
     and records the manifest, and the counters below are that run's. A spot \
     pull with the folder left blank, and every expired-F&O request, is \
     understood, echoed back with the exact dates that would go on the wire, \
     and then REFUSED with 503 — nothing is written.";

/// One governor per feed, built from the feed's own declared budget.
///
/// **Indexed by discriminant, not searched.** `Feed` is `#[repr(u8)]` with
/// explicit discriminants and `Feed::ALL` is in table order, so slot *i* is
/// feed *i* and the lookup at the call site is one array index. A fifth feed
/// widens the vector and no code here learns its name.
///
/// A slot is `None` when the feed declares no HTTP transport — it has no budget
/// because it makes no requests. That is not a silent exemption: an archive
/// feed never reaches the code that consults this, because the transport routes
/// it to the CSV reader instead.
///
/// The NUMBERS come from `pull::vendor`'s descriptor rows, which cite
/// `docs/00-charter.md` §4 for each one and carry its verification lane in the
/// constant's own name — `GROWW_PER_SECOND_UNVERIFIED` says in the identifier
/// that the figure is chosen rather than measured. Nothing is invented here and
/// nothing is defaulted: a `None` in a `Budget` field means the vendor
/// publishes no bound for that span, and `Governor::new` treats it as such.
fn feed_budgets() -> Vec<Option<pull::rate::Governor>> {
    pull::vendor::Feed::ALL
        .into_iter()
        .map(|feed| match feed.descriptor().transport {
            pull::vendor::Transport::Http(spec) => pull::rate::Governor::new(
                spec.budget.per_second,
                spec.budget.per_minute,
                spec.budget.per_day,
            )
            .ok(),
            pull::vendor::Transport::LocalArchive(_) => None,
        })
        .collect()
}

/// Everything every request renders from, read once at startup.
///
/// `Arc` rather than a clone per request: [`Read`] owns a `HashMap` of every
/// instrument, and cloning it per request would trade one O(rows) cost for
/// another. An `Arc` clone is a refcount bump — constant, and the same bytes
/// are shared by every concurrent request.
///
/// Shared **immutably**, so there is no lock on the read path and no
/// contention that grows with concurrent readers.
#[derive(Debug)]
pub struct Site {
    /// The rate budget every HTTP feed spends from, held across requests.
    ///
    /// # Why this is on the site and not in `broker_window`
    ///
    /// A token bucket is STATE. A governor constructed inside the function that
    /// asks it for permission starts full every time, admits every request, and
    /// is an elaborate way of writing `true` — which is what an unwired
    /// governor amounts to, and `crates/pull/src/rate.rs` had zero callers.
    ///
    /// The budget is per FEED, keyed by discriminant so the lookup is an array
    /// index rather than a search — a fifth feed widens the array and nothing
    /// here learns its name. `None` in a slot is a feed that publishes no bound
    /// at all, which is the archive feeds and is not a silent exemption: they
    /// never reach this code, because the transport routes them elsewhere.
    ///
    /// A `Mutex` rather than an atomic because `Governor::admit` reads three
    /// windows and charges all three only if every one affords a permit — that
    /// is a single decision over three numbers, and splitting it would let a
    /// request the day window refused still drain the second window.
    pub budgets: std::sync::Mutex<Vec<Option<pull::rate::Governor>>>,
    /// The instrument universe, merged from both masters.
    pub read: Read,
    /// One manifest census per vendor, in [`Vendor::ALL`] order.
    pub censuses: Vec<census::VendorCensus>,
    /// The coverage grid's instrument axis — every series the censuses hold,
    /// plus the two the engine sweeps. Sorted, so a row's ordinal is stable
    /// across reloads and restarts.
    pub series: Vec<census::Series>,
    /// Folders holding CSVs, walked once at startup rather than per render.
    pub folders: Vec<String>,
    /// Every instrument-month some vendor holds, newest first — the rows
    /// `/store` opens on, as opposed to the mostly-empty product of the axis.
    pub entries: Vec<(census::Series, store::path::YearMonth)>,
    /// Whether this process may reach a live broker.
    ///
    /// # Why this is a field and not an `if cfg!(test)`
    ///
    /// The moment the broker path was joined, seven tests that pass an empty
    /// folder started **making real authenticated calls to Dhan** — and one of
    /// them proved it by failing with `DH-905 securityId is required`, a reply
    /// from the live API. A test suite that reaches a broker is slow, flaky,
    /// spends the operator's rate budget, and fails for reasons that are not
    /// the code's; it would be quarantined within a week.
    ///
    /// `cfg!(test)` would not have caught it either: these are integration
    /// tests of a library, and the flag is false in the binary they drive. So
    /// it is a value on the site, set to [`Broker::Live`] exactly once — in
    /// [`Site::load`], which only the served process calls — and left
    /// [`Broker::Refused`] everywhere else.
    pub broker: Broker,
    /// How many instruments each spot target covers **in the merged
    /// universe**, in [`ingest::SpotTarget::ALL`] order.
    ///
    /// Counted once, here, rather than folded over the universe per request:
    /// the form shows a real number and the page still costs O(rows shown).
    /// Sized from the enum, not from a literal. It was `[usize; 3]` and D-0105
    /// appended four targets; a hand-written length is a second place the
    /// target count lives, and the shorter of the two silently drops the tail.
    ///
    /// **FEED-AGNOSTIC, AND THAT IS NOT THE NUMBER FOR A BUTTON.** This is what
    /// NSE and the two masters between them name — the union. A pull runs
    /// against ONE feed, and a feed can only fetch what its own master gives it
    /// an id for: `indices` is 35 here and Groww lists 24 of them. Anything
    /// that knows which feed was chosen must read [`Self::coverage`] instead;
    /// this stays because "how big is this set" is a real question and the
    /// legacy `/pull` form is rendered before a feed is picked. D-0120.
    pub targets: [usize; ingest::SpotTarget::ALL.len()],
    /// Where the manifests were read from, named on the page so an absence is
    /// actionable rather than mysterious.
    pub store_root: PathBuf,
    /// The backfill that drives itself, and the operator's two controls over
    /// it.
    ///
    /// # Why it lives on the site
    ///
    /// `Site` is already the `Arc` the handlers and the background task both
    /// hold, so this is the one value both ends of pause/resume can see without
    /// a second shared thing to keep in step. It holds atomics and one lock
    /// rather than a channel because the check inside a 773-instrument sweep
    /// must be a relaxed load: a sweep that took a lock per instrument would
    /// make the operator's Pause wait on the vendor.
    ///
    /// Constructing it does **not** start anything. The task is spawned by
    /// `run_in` alone, and it refuses to run unless [`Site::broker`] is
    /// [`Broker::Live`] — which only [`Site::serving`] sets. A test that builds
    /// a `Site` therefore gets the controls and no backfill.
    pub autopilot: autopilot::Control,
    /// When the manifests above were read, in epoch seconds.
    ///
    /// **A counter on this page is as old as this process.** The censuses are
    /// read once into an `Arc<Site>` — D-0039, and re-reading them per request
    /// is the O(entries) cost that split exists to remove — so a pull performed
    /// by *this running server* is on disk and in the journal while these
    /// counters still say what they said at startup. That is a real staleness
    /// and it is now said out loud on `/store` rather than left for an operator
    /// to discover: [`store_html`] compares this against the newest audit
    /// record, which is one 256-byte read.
    pub loaded_at: i64,
}

impl Site {
    /// Assembles the derived counts once, from parts a caller already holds.
    #[must_use]
    pub fn new(read: Read, censuses: Vec<census::VendorCensus>, store_root: PathBuf) -> Self {
        let mut targets = [0usize; ingest::SpotTarget::ALL.len()];
        for (key, entry) in &read.merged.by_key {
            for (slot, target) in ingest::SpotTarget::ALL.into_iter().enumerate() {
                // THE TARGET'S OWN PREDICATE, not a second copy of it.
                //
                // This was `match target { Swept => key.is_sweepable(), _ =>
                // entry.universe.contains(target.universe()) }` — a CATCH-ALL,
                // and the one site the compiler could not have pointed at when
                // D-0105 appended four variants. It happened to be right for
                // them, which is the problem: a `_` arm decides for every
                // variant that will ever exist, including the ones whose
                // membership is not a single bit test. `SpotTarget::names` is
                // what `broker_run` filters the run by, so counting with
                // anything else is how a form comes to show a number no run
                // will match — the exact defect `names` was added to remove.
                let counted = target.names(key, entry.universe);
                if counted && let Some(n) = targets.get_mut(slot) {
                    *n += 1;
                }
            }
        }
        // THE GRID'S AXIS IS READ OFF THE CENSUSES, NOT OUT OF THE MASTER.
        //
        // It used to be `grid_instruments(read.merged.by_key.keys())` — the
        // instrument master filtered to spot indices — and the store on this
        // operator's disk holds 194 months of expired futures under the vendor
        // archive's own names. The two vocabularies never met, so every probe
        // missed and the page reported `0 of 200 held` beside `62,978 rows`.
        // `census::held_series` takes the keys the manifests actually contain
        // and adds the two swept series so a fresh install still names what it
        // is missing; there is no longer a universe to fall back FROM, because
        // the universe was never the right source for this page.
        //
        // Computed here, once, for the reason the censuses themselves are
        // (D-0039): it is O(keys log keys), and that belongs at startup beside
        // the manifest load rather than inside a request.
        let series = census::held_series(&censuses);
        // THE FOLDER SUGGESTION LIST, WALKED ONCE. It was walked on every
        // `/pull` render and cost 72 ms against `/store`'s 1 ms, growing with
        // `~/Downloads` -- a directory this repository does not own and cannot
        // bound. Same move D-0039 made for the master. See render::folder_suggestions.
        let folders = render::folder_suggestions();
        // WHAT THE STORE ACTUALLY HOLDS, as rows rather than as a product.
        // The axis above crossed with 36 months is 7,056 cells of which 194
        // are held, so `/store` opened on 36 blank BANKNIFTY rows and the real
        // entries began on page 2. Computed here for the same reason the axis
        // is: once, at startup. See census::held_entries.
        let entries = census::held_entries(&censuses);
        Self {
            budgets: std::sync::Mutex::new(feed_budgets()),
            read,
            censuses,
            series,
            entries,
            folders,
            // REFUSED BY DEFAULT. `Site::load` is the one place that turns it
            // on, so a test constructing a `Site` any other way cannot reach a
            // broker by omission.
            broker: Broker::Refused,
            targets,
            store_root,
            // NOT STARTED HERE, AND PAUSED UNLESS ASKED. This is the controls
            // and the status only; the task that acts on them is spawned by
            // `run_in`. `Control::serving` reads `BRUTEX_AUTOPILOT` and leaves
            // the flag set unless it says `run`, so starting this binary to
            // look at a page does not thereby contact a vendor.
            autopilot: autopilot::Control::serving(),
            loaded_at: ingest::epoch_secs(std::time::SystemTime::now()),
        }
    }

    /// The whole site, read off disk once.
    #[must_use]
    pub fn load(masters: &Path, store_root: &Path) -> Self {
        Self::new(
            universe(masters),
            census::read_all(store_root),
            store_root.to_path_buf(),
        )
    }

    /// The same site, permitted to reach a live broker.
    ///
    /// **Called by `run_in` and by nothing else.** [`Site::load`] deliberately
    /// does not set it: the test helpers call `load` too, and a flag that a
    /// test can switch on by using the ordinary constructor is not a guard. It
    /// is opt-in at the one call site that serves HTTP to a human.
    #[must_use]
    pub fn serving(masters: &Path, store_root: &Path) -> Self {
        Self {
            broker: Broker::Live,
            ..Self::load(masters, store_root)
        }
    }

    /// The journal every pull against this store root appends to.
    ///
    /// Derived from [`Site::store_root`] rather than stored beside it: two
    /// fields that must agree about which store this is are two fields that can
    /// disagree, and the journal belongs to the store, not to the process.
    #[must_use]
    pub fn journal(&self) -> audit::Journal {
        audit::Journal::at(&self.store_root)
    }
}

/// The newest record in a journal, or nothing.
///
/// One `metadata` call and one 256-byte read, whatever the file holds — the
/// bound [`audit::Journal::page`] is built for, proven by
/// `api::audit::the_tail_reads_only_the_records_it_shows`. Never fails: a
/// journal that will not read is reported by [`audit::Journal::look`] and this
/// simply has nothing to show.
#[must_use]
pub fn newest_record(journal: &audit::Journal, log: &audit::Log) -> Option<audit::Record> {
    journal
        .page(log.records(), 0, 1)
        .ok()?
        .into_iter()
        .next()
        .and_then(|entry| entry.decoded.ok())
}

/// The journal, as the page footer states it.
#[must_use]
fn journal_note<'a>(
    path: &'a str,
    log: &'a audit::Log,
    trouble: &'a str,
) -> render::JournalNote<'a> {
    match *log {
        audit::Log::Absent => render::JournalNote {
            path,
            records: 0,
            bytes: 0,
            present: false,
            trouble: None,
        },
        audit::Log::Unreadable { .. } => render::JournalNote {
            path,
            records: 0,
            bytes: 0,
            present: true,
            trouble: Some(trouble),
        },
        audit::Log::Held {
            records,
            bytes,
            torn,
        } => render::JournalNote {
            path,
            records,
            bytes,
            present: true,
            trouble: torn.map(|_ignored| trouble),
        },
    }
}

/// What is wrong with the journal file itself, in one sentence.
///
/// Built separately from [`journal_note`] because it has to outlive the borrow
/// the view takes of it, and because a sentence an operator reads is not a
/// field an enum carries.
///
/// `pub(crate)` so `/audit.json` reports the torn tail in the same words the
/// HTML page does. Two sentences about one file is two sentences that can
/// disagree about it.
#[must_use]
pub(crate) fn journal_trouble(log: &audit::Log) -> String {
    match *log {
        audit::Log::Absent => String::new(),
        audit::Log::Unreadable { ref reason } => format!(
            "UNREADABLE — {reason}. No run can be recorded until that is fixed, \
             and a pull that cannot be recorded says so on its own answer page."
        ),
        audit::Log::Held { torn, records, .. } => torn.map_or_else(String::new, |spare| {
            format!(
                "TORN WRITE — {spare} byte(s) past the last whole record. A \
                 process was killed inside an append. The {records} whole \
                 record(s) before it are unaffected and still read; nothing \
                 here repairs the tail."
            )
        }),
    }
}

/// The shared, immutable site every handler renders from.
pub type Loaded = std::sync::Arc<Site>;

/// Answers with `f`'s page when the clock names a day, and with a named
/// refusal when it does not.
///
/// # Why the day arrives as an argument
///
/// The same reason [`run_in`] takes a directory and [`masters_dir_from`] takes
/// a value: a branch that only a broken machine clock could enter is a branch
/// no test can hold, and an untestable arm inside three handlers is three arms
/// nobody has checked. Every page that needs today goes through here, so the
/// refusal is written once and both of its arms are reachable from a test.
fn dated<F>(
    today: Result<Day, ingest::Refusal>,
    scope: &'static str,
    f: F,
) -> (axum::http::StatusCode, String)
where
    F: FnOnce(Day) -> (axum::http::StatusCode, String),
{
    match today {
        Ok(day) => f(day),
        Err(why) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            refusal_html(scope, &why),
        ),
    }
}

/// The ingest page.
///
/// GET, and GET does nothing but render. Starting a pull is a POST to
/// `/pull/spot` or `/pull/fno`, so a crawler, a refresh or a back button
/// cannot begin one.
async fn pull_get(
    axum::extract::State(site): axum::extract::State<Loaded>,
) -> (axum::http::StatusCode, axum::response::Html<String>) {
    let (code, body) = dated(ingest::today_ist(), "Ingest", |today| {
        (axum::http::StatusCode::OK, pull_html(&site, today))
    });
    (code, axum::response::Html(body))
}

/// `YYYY-MM-DD HH:MM:SS` in IST, from epoch seconds.
///
/// A clock value no calendar can name is said rather than swallowed: a record
/// whose timestamp is unusable is still a record of a run that happened, and
/// blanking the row would lose it.
#[must_use]
pub fn ist_stamp(secs: i64) -> String {
    IstMoment::from_epoch_secs(secs).map_or_else(
        |why| format!("epoch second {secs} — {why}"),
        |at| {
            format!(
                "{} {:02}:{:02}:{:02} IST",
                at.day(),
                at.minute_of_day() / 60,
                at.minute_of_day() % 60,
                at.second_of_minute()
            )
        },
    )
}

/// A recorded window, or a dash when the record carries none.
///
/// Both ends zero means "this record is about a request that never got as far
/// as a window" — a refused form, for instance. It is a dash and not
/// `1970-01-01..=1970-01-01`, because a date nobody asked for is worse than no
/// date at all.
#[must_use]
pub fn window_text(from: u32, to: u32) -> String {
    if from == 0 && to == 0 {
        return "—".to_owned();
    }
    match (Day::from_days(from), Day::from_days(to)) {
        (Ok(a), Ok(b)) => format!("{a}..={b}"),
        _ => format!("day {from}..=day {to} — outside the calendar this build can name"),
    }
}

/// The ingest page, from a site already loaded.
#[must_use]
pub fn pull_html(site: &Site, today: Day) -> String {
    let targets: Vec<(ingest::SpotTarget, usize)> = ingest::SpotTarget::ALL
        .into_iter()
        .enumerate()
        .map(|(i, t)| (t, site.targets.get(i).copied().unwrap_or(0)))
        .collect();
    let journal = site.journal();
    let log = journal.look();
    let trouble = journal_trouble(&log);
    let path = journal.path.display().to_string();
    // NOT A FABRICATED PROGRESS BAR, AND NO LONGER A FABRICATED ABSENCE
    // EITHER. There is nothing running — a pull is one synchronous POST — but
    // runs HAVE happened, and the last one's counters are on disk. The panel
    // reads them. When there is no record it still says dashes, and the
    // sentence beside them names the file that is empty rather than a module
    // that is not.
    let last = newest_record(&journal, &log);
    let stamp = last.as_ref().map(|r| ist_stamp(r.at_unix_secs));
    let window = last.as_ref().and_then(|r| {
        match (Day::from_days(r.from_days), Day::from_days(r.to_days)) {
            (Ok(a), Ok(b)) => pull::session::Window::new(a, b).ok(),
            _ => None,
        }
    });
    let capture = match (last.as_ref(), stamp.as_ref(), window) {
        (Some(record), Some(when), Some(window)) => Some(render::Capture {
            what: &record.source,
            when,
            outcome: record.outcome.label(),
            loud: record.outcome.is_loud(),
            window,
            fetched: record.rows_read,
            stored: record.bars_stored,
            folded: record.rows_folded,
            drops: record.drops,
            took_micros: record.elapsed_micros,
        }),
        _ => None,
    };
    let no_capture = if last.is_some() {
        "The last record carries no usable window, so its counters are not shown \
         beside one — see /audit for the record itself."
    } else {
        "No pull has been recorded against this store root yet, so every counter \
         below is an em dash and not a zero: nothing has been measured, and a \
         zero would be a claim that it had."
    };
    render::pull_page(&render::PullView {
        today,
        targets: &targets,
        capture,
        no_capture,
        journal: journal_note(&path, &log, &trouble),
        // CHOSEN FROM THE VALUE THAT DECIDES, not stated as a constant. The
        // predecessor rendered `HTTP_UNAVAILABLE` unconditionally and therefore
        // told a served process — which sets `Broker::Live`, reads the token and
        // reaches Dhan — that no vendor is contacted from it.
        halt: Some(halt_for(site.broker)),
        notes: &site.read.notes_view,
        folders: &site.folders,
    })
}

/// Which sentence describes this process, from the value that decides.
///
/// One function so the two texts cannot be chosen differently in two places,
/// and so a test can assert the CORRESPONDENCE — `Live` gets the live text,
/// `Refused` gets the other — rather than pinning either string. Pinning the
/// string is what let the old sentence stay green while it became false.
#[must_use]
pub const fn halt_for(broker: Broker) -> &'static str {
    match broker {
        Broker::Live => HTTP_LIVE,
        Broker::Refused => HTTP_UNAVAILABLE,
    }
}

/// The page a refused request answers with.
#[must_use]
pub fn refusal_html(scope: &str, why: &ingest::Refusal) -> String {
    let facts = [(
        "Outcome",
        "nothing was requested, no vendor was contacted, and nothing was written".to_owned(),
    )];
    render::receipt_page(&render::Receipt {
        scope,
        verdict: "REFUSED",
        reason: &why.to_string(),
        good: false,
        facts: &facts,
        footnote: "Nothing here was written to the store.",
    })
}

/// The refusal page, plus what became of the record of the refusal.
///
/// # The asymmetry this removes
///
/// Every accepted run's receipt already names the journal's answer:
/// [`recorded_fact`] renders `NO — this run is NOT in the journal. {why}` on an
/// `Err`, under a doc comment citing `CLAUDE.md` §4. Nine paths honoured that.
/// The three refusal paths did not — each was
///
/// ```text
/// let _ignored = journal.append(&record);
/// ```
///
/// so on a store root mounted read-only, or a full disk, or an `audit`
/// directory a file is sitting where it should be — a state this crate's own
/// `emitted.rs` drives on purpose and asserts refuses — the refusal was
/// answered with a page that says nothing about the journal, and `/audit` then
/// showed no such refusal ever happening. The autopilot page says it outright:
/// *"A failure that appears here and not there is a failure that was never
/// written down"* — and the write that failed was the one this page hid.
///
/// The append happens HERE rather than at the call site, so the outcome and the
/// page cannot come apart: there is no way to record a refusal and answer with a
/// page that does not carry the result.
fn refused_and_recorded(
    scope: &str,
    why: &ingest::Refusal,
    journal: &audit::Journal,
    record: &audit::Record,
) -> String {
    let facts = [
        (
            "Outcome",
            "nothing was requested, no vendor was contacted, and nothing was written".to_owned(),
        ),
        recorded_fact(journal, record),
    ];
    render::receipt_page(&render::Receipt {
        scope,
        verdict: "REFUSED",
        reason: &why.to_string(),
        good: false,
        facts: &facts,
        footnote: "Nothing here was written to the store.",
    })
}

/// The page a request refused for a **named** reason answers with.
///
/// # Why this is not [`accepted_html`] with an extra fact
///
/// It differs in the one line the reader actually reads. `accepted_html` fills
/// its headline from [`halt_for`], which on a serving process is [`HTTP_LIVE`] —
/// a paragraph explaining that the credential comes from Parameter Store and
/// which transport decides the path. All true, and none of it an answer to *why
/// did my pull not run*.
///
/// So a request refused for a reason this build KNOWS — the pull order, a
/// window the feed will not serve — rendered under a headline about socket
/// plumbing, with its actual reason twentieth in a table. The reason is the
/// headline here, and `halt_for`'s paragraph stays where it belongs: on the
/// answers that have no more specific thing to say.
fn refused_html(scope: &str, mut facts: Vec<(&'static str, String)>, why: &str) -> String {
    facts.push(("Status", "NOT STARTED".to_owned()));
    render::receipt_page(&render::Receipt {
        scope,
        verdict: "NOT STARTED",
        reason: why,
        good: false,
        facts: &facts,
        footnote: "Nothing here was written to the store.",
    })
}

/// The page a valid request answers with, given that nothing can run.
fn accepted_html(scope: &str, mut facts: Vec<(&'static str, String)>, broker: Broker) -> String {
    facts.push(("Status", "NOT STARTED".to_owned()));
    render::receipt_page(&render::Receipt {
        scope,
        verdict: "NOT STARTED",
        reason: halt_for(broker),
        good: false,
        facts: &facts,
        footnote: "Nothing here was written to the store.",
    })
}

/// The page a run that actually stored bars answers with.
///
/// **A separate receipt, because the shared one was lying.** Every successful
/// local ingest — 194 members, 62,978 bars, a manifest rewritten — came back
/// under the verdict `NOT STARTED`, with [`HTTP_UNAVAILABLE`]'s predecessor as
/// its reason, a red `badge bad`, and the closing line *"Nothing here was
/// written to the store."* Four claims on one page, all four false, because
/// there was one `accepted_html` and it stamped every answer alike.
fn stored_html(
    scope: &str,
    verdict: &str,
    reason: &str,
    facts: &[(&'static str, String)],
) -> String {
    let good = verdict == audit::Outcome::Stored.label();
    render::receipt_page(&render::Receipt {
        scope,
        verdict,
        reason,
        good,
        facts,
        footnote: if good {
            "The bars named above ARE on the store root named above, and the \
             manifest counts them."
        } else {
            "Whatever landed before the failure is on disk and is described \
             above; nothing beyond it is claimed."
        },
    })
}

/// The window's facts, including the correction the operator never has to make.
fn window_facts(window: pull::session::Window) -> Vec<(&'static str, String)> {
    vec![
        // `Window`'s own Display, which is `from..=to` — the inclusive
        // notation. Writing the two ends out here instead would be a second
        // definition of what the range means, and the two would drift.
        ("Window", window.to_string()),
        (
            "From",
            format!("{} — inclusive", window.from()),
        ),
        ("To", format!("{} — inclusive", window.to())),
        ("Calendar days", window.days().to_string()),
        (
            "toDate on the wire",
            window.wire_to().map_or_else(
                |e| format!("REFUSED — {e}"),
                |d| format!("{d} — the day AFTER your last day, because the vendor's toDate is not inclusive"),
            ),
        ),
        ("Timeframe", "1 minute".to_owned()),
    ]
}

/// What one spot request is answered with, given a day to check the window
/// against.
///
/// Split from the handler for the same reason [`fno_answer`] is: the gate must
/// be driven by a value, not by the machine's clock — `CLAUDE.md` §3 rule 5, and
/// a gate that reads the clock inside cannot be tested at its own boundary
/// without waiting for midnight.
///
/// **This split is the fix for a real defect.** The F&O side has had this shape
/// since it was written; the spot side went straight to the parser with no
/// `today` at all, so the only thing stopping a window that ends in the future
/// was the `max` attribute the page renders. The panel one div over says *"an
/// attribute is a courtesy, a parser is a rule"* — and spot had only the
/// courtesy. An attribute is absent from a `curl`, from a replayed POST, and
/// from any client that is not the browser the form was rendered for.
async fn spot_answer(
    body: &str,
    today: Day,
    now: std::time::SystemTime,
    site: &Site,
) -> (axum::http::StatusCode, String) {
    let journal = site.journal();
    match ingest::parse_spot(body, today) {
        Err(why) => {
            let record = audit::Record::refused(
                audit::Scope::Spot,
                audit::Outcome::Refused,
                now,
                &param(body, "target"),
                &why.to_string(),
            );
            // A REFUSAL IS RECORDED TOO. "What was asked" includes the requests
            // that were not honoured — an operator debugging a form that never
            // starts anything needs the refusals more than the successes.
            // AND THE RECORD'S OWN OUTCOME IS ON THE PAGE. See
            // `refused_and_recorded`.
            (
                axum::http::StatusCode::BAD_REQUEST,
                refused_and_recorded("Spot pull", &why, &journal, &record),
            )
        }
        Ok(asked) => {
            let slot = ingest::SpotTarget::ALL
                .into_iter()
                .position(|t| t == asked.target)
                .unwrap_or(0);
            let in_universe = site.targets.get(slot).copied().unwrap_or(0);
            let mut facts = vec![
                ("Target", asked.target.label().to_owned()),
                (
                    "Instruments covered",
                    format!("{in_universe} in the merged universe"),
                ),
                // THE SECOND NUMBER, AND IT IS THE ONE THAT DECIDES.
                //
                // The receipt used to print the line above alone. It is the
                // union of what both masters name, and this run reaches exactly
                // one feed: on the masters read on 2026-08-12, `indices` is 35
                // there and Groww lists 24 of them. An operator was told 35,
                // watched eleven instruments refuse by name, and had nothing on
                // the receipt that had predicted it. D-0120.
                (
                    "This feed can name",
                    reach_text(asked.target, asked.feed, site),
                ),
            ];
            facts.extend(window_facts(asked.window));

            // THE TRANSPORT CHOOSES THE PATH. Nothing else does.
            //
            // This fork used to read `if param(body, "folder").is_empty()`, and
            // that one line was the whole N-feed problem in miniature. A blank
            // textbox meant "broker" and a filled one meant "archive", so the
            // operator's actual choice of feed did not decide anything: picking
            // Groww and typing a folder ran Groww's name down the CSV reader,
            // and picking an archive vendor and leaving the box blank sent it
            // to the credential store to look for a token it does not have.
            //
            // Now the feed's own descriptor decides. HTTP feeds get the rules
            // that are about talking to a vendor — token, rate budget, market
            // hours, and never a window reaching today. Archive feeds get none
            // of them, because a folder of CSVs an operator has already bought
            // has no session to be mid-way through and no token to expire.
            //
            // A fifth feed inherits the correct half by declaring its transport
            // and nothing else. That is the property this fork exists to keep.
            facts.push(("Feed", asked.feed.display().to_owned()));
            // THE RUNG, ON THE RECEIPT. It decides the directory the bars land
            // in, and a receipt that named the feed and the window but not the
            // bar length would leave an operator unable to say which of two
            // directories a run wrote to.
            facts.push(("Bar length", asked.granularity.to_string()));
            match asked.feed.descriptor().transport {
                // THE BROKER PATH. The credential is read from Parameter Store
                // (D-0051), the descriptor drives the request, and the bars
                // take the same route to disk a folder's do —
                // `pull::ingest::from_window`.
                pull::vendor::Transport::Http(_) => {
                    broker_answer(asked, now, site, &journal, facts).await
                }
                // THE LOCAL-ARCHIVE PATH. No socket, no token, no governor.
                pull::vendor::Transport::LocalArchive(_) => {
                    let folder = param(body, "folder");
                    if folder.is_empty() {
                        // REFUSED BY NAME rather than quietly falling through to
                        // the broker path, which is what the old fork did. There
                        // is no credential to try and nothing to fall back to:
                        // an archive feed with no folder is a request with no
                        // subject. `CLAUDE.md` §4 bans the silent version.
                        let why = ingest::Refusal::ArchiveFolderMissing {
                            feed: asked.feed.display(),
                        };
                        // NOTED LIKE EVERY OTHER REFUSAL. This variant was the
                        // one built HERE rather than inside `parse_spot_inner`,
                        // so it never passed through `note_refused` and was the
                        // only refusal shape invisible in the log — which makes
                        // the log imply it never happens. The journal entry below
                        // is the audit record, not a log event; the two surfaces
                        // are separate and an operator reading `/logs` saw
                        // nothing.
                        //
                        // "One event per submission" still holds by construction:
                        // `parse_spot` returned `Ok` for this body, so no earlier
                        // "form refused" event exists for it.
                        ingest::note_refused("spot", &why);
                        let record = audit::Record::refused(
                            audit::Scope::Spot,
                            audit::Outcome::Refused,
                            now,
                            &param(body, "target"),
                            &why.to_string(),
                        );
                        return (
                            axum::http::StatusCode::BAD_REQUEST,
                            refused_and_recorded("Spot pull", &why, &journal, &record),
                        );
                    }
                    facts.push(("Source", format!("local folder · {folder}")));
                    facts.push(("Store root", site.store_root.display().to_string()));
                    local_answer(&asked, &folder, now, site, &journal, facts)
                }
            }
        }
    }
}

/// Land one instrument's bodies, and count them.
///
/// Split out of `broker_answer` when that grew a loop over the universe: the
/// per-instrument work is identical for all ~800 and the only thing that varies
/// is which `BrokerWindow` it is handed.
///
/// Every chunk lands. `from_window` runs once per body rather than once per
/// form submission, and the counters are summed — reporting only the last of
/// eighty-one chunks would make eighty chunks' bars vanish from a receipt whose
/// entire purpose is that they do not.
fn land_one(landed: &BrokerWindow, site: &Site) -> pull::ingest::Ingested {
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        // NOT ON THE WIRE ON THIS PATH. An archive addresses a FILE and a
        // landing record addresses bars already in hand, so no request
        // parameter is built from this and no vendor ever sees it. Stated
        // anyway because `BarRequest` has no `Default` — a field that can be
        // omitted is a field a later caller omits by accident.
        listing: pull::vendor::Listing::Equity,
        // THE TEMPLATE'S WINDOW, AND EVERY BODY OVERRIDES IT BELOW. Landing a
        // body against this value is the defect the loop's comment names; it
        // stands here only because `BarRequest` has no `Default`.
        window: landed.window,
        // THE OPERATOR'S RUNG, NOT A LITERAL — and it is the one field that
        // decides three things at once: which directory the bars land in, how
        // wide the fold bucket is, and whether the session filter applies at
        // all. It was `Cadence::Minute` beside a `Timeframe::MINUTE_1`, and a
        // daily pull with either left behind is not a smaller answer: a daily
        // bar is stamped at midnight, so every one of them is counted
        // `BeforeSessionOpen` and the receipt reads as a clean zero.
        granularity: landed.granularity,
    };
    let plan = pull::ingest::Plan {
        columns: pull::csv::Columns::Gdfl,
        request: &request,
        // FROM THE DESCRIPTOR, NOT A LITERAL. Dhan stamps epoch seconds and
        // Groww's row says milliseconds; writing either here makes the other
        // vendor unfetchable, which is what a hardcoded `EpochSecondsUtc` did
        // to Groww.
        encoding: landed.spec.timestamps,
        // `http::decode_body` already converted rupees to paisa, so the plan
        // must say Paisa — `DECODED_PRICE_SCALE` names this trap.
        scale: pull::http::DECODED_PRICE_SCALE,
        // Resolved through `Feed::store_vendor`. A literal here files one
        // broker's prices under another's prefix.
        vendor: landed.store_vendor,
        // THE INSTRUMENT'S OWN EXCHANGE AND SEGMENT, not literals.
        //
        // `Segment::Index` was hardcoded, so all 750 equities were filed under
        // `NSE/INDEX/` — the DB page showed `NSE-INDEX-360ONE`, and 360ONE is
        // an equity. The path IS the index in this store, so a wrong segment is
        // not a labelling slip: it is the bar living at the wrong address, and
        // a later reader asking for `NSE/CASH/360ONE` finds nothing while the
        // data sits one directory over.
        //
        // Same shape as the `Vendor::Dhan` literal that put GDFL futures under
        // `bars/dhan/`, and the `Symbol::new("NIFTY")` that made spot pull one
        // instrument. A literal where a value belongs.
        exchange: landed.exchange,
        segment: landed.segment,
        contract: landed.contract,
    };
    let mut done = pull::ingest::Ingested::default();
    for (chunk, body) in &landed.bodies {
        // EACH BODY IS FILTERED AGAINST THE WINDOW IT WAS FETCHED FOR.
        //
        // One `BarRequest` was built above and reused for every body, carrying
        // the operator's whole multi-year range — so `pull::fetch::land`'s
        // `request.window.verdict(..)` compared a one-month answer against a
        // five-year window and `BeforeWindow`/`AfterWindow` were unreachable
        // for every row of every chunk. The census that exists to name a row
        // the vendor should not have sent could not fire, and a run that lost
        // 80 of 122 members reported `0 rows dropped`.
        //
        // Rebuilt per body rather than mutated: `Plan` is `Copy` and holds
        // `&BarRequest`, so the borrow must outlive nothing but this iteration.
        let request = pull::fetch::BarRequest {
            window: *chunk,
            instrument_id: request.instrument_id.clone(),
            ..request
        };
        let plan = pull::ingest::Plan {
            request: &request,
            ..plan
        };
        done.absorb(pull::ingest::from_window(
            body,
            &landed.instrument,
            &landed.origin,
            &site.store_root,
            plan,
        ));
    }
    done
}

/// One broker run: credential, request, bars, receipt.
///
/// # The join, finally
///
/// Every piece of this has existed and been tested for some time —
/// `pull::http::HttpSource` is the client, `pull::ssm` reads the credential,
/// `pull::ingest::from_window` lands the bars — and nothing called them in
/// order. This does.
///
/// # Every failure is named, and none of them writes anything
///
/// A pull that cannot get a credential, cannot reach the broker, or gets an
/// answer it cannot decode must say **which**, because those send an operator
/// to three different places: their AWS role, their network, and the
/// descriptor. `CLAUDE.md` §4 — degrade loudly and name the reason. Nothing
/// below writes a bar until the window has decoded.
///
/// The credential is read on every run and never cached. `CLAUDE.md` §8: a
/// stale token is re-read, and this repository never mints one.
async fn broker_answer(
    asked: ingest::SpotRequest,
    now: std::time::SystemTime,
    site: &Site,
    journal: &audit::Journal,
    mut facts: Vec<(&'static str, String)>,
) -> (axum::http::StatusCode, String) {
    facts.push(("Source", "broker · HTTPS".to_owned()));
    facts.push(("Store root", site.store_root.display().to_string()));

    // THE SWEEP ITSELF, shared verbatim with the autopilot. Everything below
    // this line is the receipt; everything inside it is the pull.
    // FRESH, NOT `site.censuses`: see `broker_run`. One manifest read against a
    // call that is about to open sockets is free in the only units that matter.
    let run = broker_run(&asked, site, &census::read_all(&site.store_root)).await;

    // A refusal, recorded and rendered, with the reason it carries.
    let refuse = |facts: Vec<(&'static str, String)>, why: &str, code: axum::http::StatusCode| {
        let record = audit::Record::refused(
            audit::Scope::Spot,
            audit::Outcome::NotStarted,
            now,
            asked.target.label(),
            why,
        )
        .with_window(asked.window);
        let mut facts = facts;
        facts.push(("Refused because", why.to_owned()));
        facts.push(recorded_fact(journal, &record));
        // NOT `refused_html`, and the difference is load-bearing on a
        // non-serving process: this closure's headline is `halt_for`, whose
        // `Broker::Refused` text states that NO VENDOR IS CONTACTED from here.
        // That assurance is the answer to a question the specific reason does
        // not address, and `a_valid_window_is_echoed_with_the_wire_date_and_        // still_starts_nothing` reads it back. Giving this site a specific
        // headline as well means carrying that assurance into the facts first.
        (code, accepted_html("Spot pull", facts, site.broker))
    };

    // BEFORE ANY SOCKET. See `Broker` for what this is guarding against and
    // how it was found. `broker_run` is the one that checks it, so an internal
    // caller is guarded by the same line rather than by a second copy of it.
    if let Some(blocked) = run.blocked.as_ref() {
        // THE REASON AND THE CODE ARE THE RUN'S OWN — see `Blocked`. This block
        // restated both as literals, which was true while `Broker::Refused` was
        // the only producer and became a lie the moment the pull order became
        // the second: an out-of-sequence request would have been reported as a
        // broker outage, with a 503 to match. `autopilot::tick` already read the
        // reason this way; only the receipt did not.
        let record = audit::Record::refused(
            audit::Scope::Spot,
            audit::Outcome::NotStarted,
            now,
            asked.target.label(),
            &blocked.why,
        )
        .with_window(asked.window);
        facts.push(("Refused because", blocked.why.clone()));
        facts.push(recorded_fact(journal, &record));
        return (blocked.code, refused_html("Spot pull", facts, &blocked.why));
    }

    facts.push(("Instruments attempted", run.attempted.to_string()));
    facts.push(("Instruments reached", run.reached.to_string()));
    if !run.refused.is_empty() {
        facts.push((
            "Instruments refused",
            format!(
                "{} — first: {}",
                run.refused.len(),
                run.refused.first().map_or("", String::as_str)
            ),
        ));
    }
    // STOPPING IS NOT FAILING, and it is not silent either. A sweep the
    // operator interrupted says so on its own receipt, with the count it got
    // to, so a short run is never mistaken for a complete one.
    if let Some(ref why) = run.stopped {
        facts.push(("Stopped", why.clone()));
    }

    if run.reached == 0 {
        let first = run
            .refused
            .first()
            .map_or("no instrument in the universe could be reached", |w| {
                w.as_str()
            });
        // 502 SAID THE VENDOR FAILED WHEN THE VENDOR WAS NEVER ASKED.
        //
        // `BAD_GATEWAY` means an upstream answered badly. Measured on a real
        // run: 213 attempted, 0 reached, 13.9 ms — no socket was opened, no
        // credential was read, and every refusal was decidable from the form.
        // The page drew `Last pull HTTP 502`, which an operator reads as *the
        // broker is down*, when the truth was *you asked for a day whose
        // session had not closed*.
        //
        // That is a status blaming a third party for this side's own refusal —
        // `CLAUDE.md` §4's fallback that hides a failure, wearing the wrong
        // name rather than no name.
        //
        // A run that never reached the wire is a BAD REQUEST: everything that
        // stopped it was answerable from the request itself. A run that DID
        // reach a vendor and got nothing back keeps 502, because there the
        // upstream really is the thing that failed. `BrokerRun::touched_wire`
        // is the difference, and it is recorded by the run rather than guessed
        // at from the refusal's prose.
        let status = if run.touched_wire {
            axum::http::StatusCode::BAD_GATEWAY
        } else {
            axum::http::StatusCode::BAD_REQUEST
        };
        return refuse(facts, first, status);
    }
    landed_answer(
        &run.total,
        asked.window,
        now,
        &run.origin,
        journal,
        facts,
        run.took,
    )
}

/// What one sweep over the tracked universe did.
///
/// The counters and the reasons, with no HTML anywhere in it — so the same run
/// can become a receipt for a browser or a state transition for the autopilot
/// without either one re-deriving what the other means.
#[derive(Debug, Default)]
pub(crate) struct BrokerRun {
    /// Whether ANY instrument in this run got as far as opening a socket.
    ///
    /// # Why the status depends on it
    ///
    /// A run that reached nothing used to answer `502 BAD_GATEWAY`, which says
    /// an upstream failed. Measured on a real run: 213 attempted, 0 reached,
    /// 13.9 ms — no socket, no credential read, every refusal decidable from
    /// the form. The page drew `Last pull HTTP 502` and an operator read it as
    /// *the broker is down*.
    ///
    /// This is the difference between "this side refused" and "the vendor did",
    /// and it is RECORDED by the run rather than guessed at from a refusal's
    /// prose — a substring match on an error message is how a 403 came to be
    /// filed as a transport blip elsewhere in this file.
    pub touched_wire: bool,
    /// Instruments the sweep set out to fetch.
    pub attempted: usize,
    /// Instruments that answered.
    pub reached: usize,
    /// One reason per instrument that did not, prefixed with its name.
    pub refused: Vec<String>,
    /// Where the bars came from, as the last reached instrument reported it.
    pub origin: String,
    /// Everything that landed, summed.
    pub total: pull::ingest::Ingested,
    /// Set when the run never opened a socket at all, with the reason.
    ///
    /// Distinct from an empty `reached`: nothing was attempted, so nothing can
    /// be concluded about the vendor from it.
    pub blocked: Option<Blocked>,
    /// Set when the operator stopped the sweep part-way, with the reason.
    pub stopped: Option<String>,
    /// How long it took, in microseconds.
    pub took: u64,
}

impl BrokerRun {
    /// A run that never opened a socket, and why.
    ///
    /// Every other counter stays at its default and that is the POINT: nothing
    /// was attempted, so nothing may be concluded about the vendor — which is
    /// exactly what `blocked` means as against an empty `reached`.
    fn blocked(why: String, code: axum::http::StatusCode) -> Self {
        Self {
            blocked: Some(Blocked { why, code }),
            ..Self::default()
        }
    }

    /// Refused because this process may not reach a live broker.
    ///
    /// `503`: the vendor path genuinely is unavailable to this process.
    fn unreachable_broker() -> Self {
        // THE ASSURANCE TRAVELS WITH THE REASON. "No vendor was contacted"
        // used to reach the reader through `halt_for`, because the receipt's
        // headline came from the broker state rather than from the refusal.
        // Now that the headline is the refusal's own words, the words have to
        // carry it — it answers a question the bare refusal does not, and
        // `a_valid_window_is_echoed_with_the_wire_date_and_still_starts_nothing`
        // reads it back off the rendered page.
        //
        // AND IT MAY NOT SAY "CREDENTIAL", which a draft of this sentence did,
        // as "no credential was read". `autopilot::classify` matches that word
        // as a SUBSTRING and `observe` turns a feed-wide credential fault into
        // a permanent `Halt::Credential` — so the phrasing alone converted a
        // transport-shaped refusal that should back off into a halt telling the
        // operator their Parameter Store token was dead. It is not. This text
        // is load-bearing input to a classifier, not prose;
        // `a_whole_round_runs_and_a_refused_broker_is_named_on_the_page` is
        // what caught it and what still holds it.
        Self::blocked(
            "this process may not reach a live broker, so no vendor was \
             contacted from this process. The served binary sets Broker::Live; \
             nothing else does, so a test cannot spend the operator's rate \
             budget by omission."
                .to_owned(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
        )
    }

    /// Refused because the pull order puts this request out of sequence.
    ///
    /// `409` and NOT a `503`: nothing upstream is unavailable, and the reason
    /// carries the pass to run instead. See [`Blocked`] for what a wrong code
    /// here has already cost once.
    fn out_of_order(why: String) -> Self {
        Self::blocked(why, axum::http::StatusCode::CONFLICT)
    }
}

/// Why a run never opened a socket, and what it answers with.
///
/// # The status travels WITH the reason, and it has to
///
/// Two unrelated facts block a run: this process may not reach a live broker,
/// and the pull order puts this request out of sequence. They were one literal
/// while there was one producer, and the receipt restated it — so the day the
/// second producer landed, an order refusal would have been reported to the
/// operator as a broker outage.
///
/// That is the same defect [`BrokerRun::touched_wire`] documents costing an
/// operator a diagnosis once already: a run that reached nothing answered `502`,
/// and the page drew *Last pull HTTP 502*, which reads as **the broker is
/// down**. A `503` on "pull the day pass first" is that mistake with a different
/// number. Pairing the code with the reason at the point the reason is known is
/// what stops a third producer inheriting the wrong one.
#[derive(Debug, Clone)]
pub(crate) struct Blocked {
    /// The reason, in the producer's own words.
    pub why: String,
    /// What the receipt answers with.
    pub code: axum::http::StatusCode,
}

/// The run's **resolved** parameters, at `Info`, before the first request.
///
/// The form says `2020-01-01` and the vendor's history floor clamps it; the
/// form says "swept indices" and the target filter decides what that resolved
/// to. An operator reading a log needs the second number in each pair, and
/// neither was written anywhere: the ingest page shows a figure it labels an
/// **estimate**, and the page is gone when the process restarts.
///
/// The result is discarded for the reason `pull::ingest`'s `note_*` helpers
/// give — a level-filtered event legitimately reaches no file.
fn note_run_started(asked: &ingest::SpotRequest, instruments: usize) {
    // STAMP EVERY LATER EVENT WITH THIS RUN, before the first of them.
    //
    // A log file spanning three backfills is three interleaved stories, and an
    // operator who hands the folder to somebody who was not here needs to take
    // one. The id is the start instant in milliseconds — monotonic on any sane
    // clock, unique unless two runs begin in the same millisecond, and readable
    // as a timestamp by a human who has nothing else to go on.
    //
    // Set on the SINK rather than threaded through every note helper, for the
    // reason `emit` reads a global at all: a parameter on every function
    // between here and a leaf is one somebody forgets, and the site they forget
    // is the one being diagnosed. Cleared by `note_run_finished`.
    if let Some(sink) = telemetry::global() {
        sink.set_run(telemetry::now_millis().unsigned_abs());
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("pull.run", "started")
            .with("feed", telemetry::Value::Str(asked.feed.wire()))
            .with("target", telemetry::Value::Str(asked.target.label()))
            .with("rung", telemetry::Value::Str(asked.granularity.dir()))
            .with(
                "from",
                telemetry::Value::Str(&asked.window.from().to_string()),
            )
            .with("to", telemetry::Value::Str(&asked.window.to().to_string()))
            .with("instruments", telemetry::Value::Uint(instruments as u64)),
    );
}

/// The run's own verdict, on the one surface that survives a restart.
///
/// `Warn` when the books do not balance, because that is the sentence the
/// receipt puts in red and the log had no equivalent of. Every figure is the
/// one the receipt renders, so the two can be compared rather than reconciled.
fn note_run_finished(out: &BrokerRun, balanced: bool) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            if balanced && out.total.failures.is_empty() {
                telemetry::Level::Info
            } else {
                telemetry::Level::Warn
            },
            "pull.run",
            if balanced {
                "finished"
            } else {
                "finished — BOOKS DO NOT BALANCE"
            },
        )
        .with("attempted", telemetry::Value::Uint(out.attempted as u64))
        .with("reached", telemetry::Value::Uint(out.reached as u64))
        .with("members", telemetry::Value::Uint(out.total.members as u64))
        .with(
            "rows_read",
            telemetry::Value::Uint(out.total.rows_read as u64),
        )
        .with(
            "bars_stored",
            telemetry::Value::Uint(out.total.bars_stored as u64),
        )
        .with(
            "failed",
            telemetry::Value::Uint(out.total.failures.len() as u64),
        )
        .with("took_micros", telemetry::Value::Uint(out.took)),
    );
    // CLEARED AFTER THE LAST EVENT OF THE RUN, so a served request that arrives
    // between backfills is not filed under the one that just ended. An event
    // outside a run carries no run, and says so by omitting the key.
    if let Some(sink) = telemetry::global() {
        sink.set_run(0);
    }
}

/// The universe, one instrument at a time — the whole of what a spot pull does.
///
/// # Why this is its own function
///
/// Two callers need this loop: `/pull/spot` and [`crate::autopilot`]. A second
/// implementation for the background one would be a second answer to what a
/// pull *is* — which month split it uses, which floor it clamps to, which
/// governor it waits on, which credential it reads — and the two would drift on
/// the first change to either. So the autopilot decides *what* to ask for and
/// this decides nothing at all except the order.
///
/// # Cost
///
/// O(universe) requests, which is the work itself. The stop check is one
/// relaxed atomic load per instrument.
/// One member that reached the vendor and still did not land: logged, then
/// recorded against the run.
///
/// # Why this exists at all
///
/// Every `telemetry::emit` site in this workspace sat on a **refusal** path, so
/// a member that reached the vendor and died at the store took the silent arm.
/// The only record was [`crate::autopilot`]'s `fail`, an in-memory ring bounded
/// by `MAX_FAILURES` that dies with the process. Measured: a run that refused
/// 10 of 16 members in 18 seconds wrote **zero** log lines — both lines in
/// `logs/events.ndjson` were `api.serve listening` — and after a restart
/// nothing anywhere named which months had failed. The cause was reconstructed
/// by decoding bar files by hand, which is not a diagnostic procedure.
///
/// # Why it is not per row
///
/// One event per failed **member**, and the member count is bounded by the
/// chunk count. A per-row event on a one-minute backfill would be millions and
/// would make the rolling sink the thing deciding what the operator gets to
/// know. See D-0072.
///
/// # Why it lives here and not in `crates/pull`
///
/// `crates/pull` declares no telemetry dependency and does not gain one: that
/// would add an edge `CLAUDE.md` §5's graph does not carry. The event is
/// emitted at the `api` layer, beside the two that were already here.
fn note_member_failure(
    failure: &pull::ingest::Failure,
    month: &str,
    asked: &ingest::SpotRequest,
    site: &Site,
) {
    let noted = telemetry::emit(
        &telemetry::Event::error("pull.spot", "member did not land")
            .with("instrument", telemetry::Value::Str(&failure.instrument))
            .with("month", telemetry::Value::Str(month))
            .with("feed", telemetry::Value::Str(asked.feed.wire()))
            .with("rung", telemetry::Value::Str(asked.granularity.dir()))
            .with("why", telemetry::Value::Str(&failure.why)),
    );
    debug_assert!(
        noted.is_written() || telemetry::global().is_none(),
        "a member that reached the vendor and did not land is the failure this \
         event exists to name"
    );
    site.autopilot
        .fail(&failure.instrument, month, &failure.why);
}

/// What the pull order has to say about this request, or [`None`] to proceed.
///
/// The rule and its reasoning are `crate::ladder`'s; this turns the [`Gate`] it
/// returns into the sentence an operator reads, and decides what the three
/// states of a census mean here.
///
/// [`Gate`]: crate::ladder::Gate
///
/// # An unreadable census REFUSES, and that is the whole reason this is not a
/// two-arm match
///
/// [`census::Census`] has three states and only one carries a `Manifest`.
///
/// - **Held** — probe it. The ordinary path.
/// - **Absent** — the ordinary state before the first ingest. Nothing is held,
///   which is a real answer and not a missing one: `|_| false` refuses a minute
///   pull for want of the day pass and lets the day pull itself straight
///   through, because nothing precedes it.
/// - **Unreadable** — a file that exists and would not load. There is no honest
///   answer here at all. Reading it as "nothing held" would refuse a day pull
///   the operator could have run, and reading it as "everything held" would
///   open the gate on the strength of a file this build just refused, which is
///   `CLAUDE.md` §4's fallback that hides a failure. So it refuses and names the
///   census's own words.
///
/// `None` is returned for an ungated feed and for a rung with no store
/// directory — the latter because `pull::ingest::Plan::timeframe` already
/// refuses it AT THE WRITE BOUNDARY in that refusal's own words, and a second
/// refusal here would be a worse-worded duplicate of a better one.
fn ladder_refusal(
    asked: &ingest::SpotRequest,
    targets: &[brutex_core::instrument::InstrumentKey],
    censuses: &[census::VendorCensus],
) -> Option<String> {
    use crate::ladder::{self, Gate};

    if !ladder::is_gated(asked.feed) {
        return None;
    }
    let rung = asked.granularity.store_timeframe()?;
    // ONE `Wanted` PER INSTRUMENT-MONTH, each carrying its OWN venue and
    // segment — see `ladder::Wanted`, where both fields say why.
    let months = ladder::months_of(asked.window);
    let wanted: Vec<ladder::Wanted> = targets
        .iter()
        .flat_map(|key| {
            // THE MONTHS THIS INSTRUMENT'S LEG ACTUALLY COVERS. A futures
            // contract reaches back one month, because it was trading before
            // the month it is named for — `ladder::months_for` carries the rule
            // and the reason. Spot is the window's own months.
            let leg = ladder::Leg::of(key.kind);
            let mine = ladder::months_for(leg, &months);
            mine.into_iter().map(move |month| ladder::Wanted {
                // THE INSTRUMENT'S OWN LEG, from its own kind. A request can
                // name instruments in more than one leg and the order is about
                // each of them separately — the same reason `segment` and
                // `exchange` are per instrument here.
                leg,
                symbol: key.underlying,
                exchange: key.exchange,
                segment: key.segment,
                month,
            })
        })
        .collect();

    let state = asked
        .feed
        .store_vendor()
        .and_then(|v| censuses.iter().find(|c| c.vendor == v))
        .map(|c| &c.state);

    let gate = match state {
        Some(census::Census::Unreadable { reason }) => {
            return Some(format!(
                "the pull order cannot be checked, so nothing was asked for. \
                 {}'s census exists and would not load — {reason}. Until it \
                 does, this build cannot tell whether the {} pass this run \
                 needs has landed, and it will not open a socket on a guess.",
                asked.feed.display(),
                ladder::precedes(rung).map_or(rung, |first| first).as_str(),
            ));
        }
        Some(census::Census::Held { manifest }) => {
            ladder::gate(ladder::probing(manifest), asked.feed, rung, &wanted)
        }
        None | Some(census::Census::Absent) => ladder::gate(|_| false, asked.feed, rung, &wanted),
    };

    match gate {
        Gate::Open => None,
        Gate::LegFirst {
            needs,
            at,
            missing,
            of,
        } => Some(format!(
            "{} comes first, and it has to FINISH: {missing} of {of} \
             instrument-months have no {} bars behind them at {}. Nothing was \
             asked for. The order is spot, then expired futures, then expired \
             options — and each of the three completes BOTH its day pass and \
             its minute pass before the next begins, because a leg that is only \
             half pulled cannot be checked against anything.",
            needs.label(),
            needs.label(),
            at.as_str(),
        )),
        Gate::RungFirst { needs, missing, of } => Some(format!(
            // 5.8× is 81 minute-windows against 14 day-windows per instrument
            // for the same span — `crate::ladder`'s module doc, from
            // `crates/store`'s own arithmetic rather than from this sentence.
            "the {first} pass comes first: {missing} of {of} instrument-months \
             are not held at {first}. Nothing was asked for. Pull {first} over \
             this same window, then run this — the cheap pass is 5.8× fewer \
             requests for the same span, and it is what finds a wrong symbol or \
             date range before the expensive one pays for it.",
            first = needs.as_str(),
        )),
        Gate::SegmentFirst {
            needs,
            at,
            missing,
            of,
        } => Some(format!(
            "the underlying comes first: {missing} of {of} derivative \
             instrument-months have no {} bars behind them at {}. Nothing was \
             asked for. A derivative window with no spot behind it cannot be \
             verified against anything, so pull {} over this window first.",
            needs.as_str(),
            at.as_str(),
            needs.as_str(),
        )),
    }
}

pub(crate) async fn broker_run(
    asked: &ingest::SpotRequest,
    site: &Site,
    censuses: &[census::VendorCensus],
) -> BrokerRun {
    let started = std::time::Instant::now();
    // BEFORE ANY SOCKET. See `Broker` for what this is guarding against and how
    // it was found.
    if site.broker == Broker::Refused {
        return BrokerRun::unreachable_broker();
    }

    // THE UNIVERSE, ONE INSTRUMENT AT A TIME.
    //
    // This called `broker_window` ONCE and the callee hardcoded NIFTY, so spot
    // did 1/800th of the job whatever universe was selected. That single
    // literal is the whole of "spot does not work".
    //
    // The set is the operator's own tracked universe — the same
    // `catalog::tracked` predicate the page and `/instruments.json` use, so the
    // three cannot disagree about what "every instrument" means.
    //
    // PER-INSTRUMENT ISOLATION. One instrument that fails does not abort the
    // other 799: its reason is recorded against its own name and the loop
    // continues. Over ~11,200 requests a run that dies on the first network
    // blip is a run that never finishes, and a single `?` here would be that.
    let mut targets: Vec<brutex_core::instrument::InstrumentKey> = site
        .read
        .merged
        .by_key
        .iter()
        // TRACKED **AND** NAMED BY THE TARGET THE OPERATOR CHOSE.
        //
        // `tracked` alone is the whole 765-name surface, so every run swept
        // everything and the three-way control on the form decided nothing but
        // the label on the receipt. `Swept indices` said 2, and 360ONE was
        // request 1 of 785. Both predicates now bind: `tracked` is what the
        // engine may hold, `names` is what this request asked for.
        .filter(|(key, entry)| {
            crate::catalog::tracked(entry.universe) && asked.target.names(key, entry.universe)
        })
        // AND THE INSTRUMENTS ACTUALLY TICKED, WHICH THIS DID NOT ASK.
        //
        // The page draws a per-instrument picker, says `1 of 213 ticked` and
        // prints `ASKED 1 instrument(s)`. The request carried only the target,
        // so this loop expanded it to all 213 — measured: an operator ticked
        // NIFTY alone and the store came back holding GLENMARK, IOC,
        // ASIANPAINT, BHEL and two hundred more across 430 files. The button,
        // the receipt and the run were three answers to one question.
        //
        // ONE HASH PROBE PER CANDIDATE. `members` is a `HashSet`, so narrowing
        // costs the same against 750 names as against one, and the bound does
        // not move when either side grows.
        //
        // AN EMPTY SET IS THE WHOLE TARGET, not nothing. A request naming no
        // member is asking for the set — which is what `target=` means alone,
        // and is what the autopilot sends. The dangerous direction is the other
        // one, and it cannot happen: members that resolve to nothing leave an
        // empty list, and `broker_run` reports zero attempted rather than
        // quietly widening back to everything.
        .filter(|(key, _)| asked.members.is_empty() || asked.members.contains(&key.underlying))
        .map(|(key, _)| *key)
        .collect();
    // Sorted so a run is reproducible: `HashMap` order is not stable between
    // processes, and an unordered backfill resumes in a different place after
    // every restart.
    targets.sort_unstable_by_key(|k| k.underlying);

    // THE PULL ORDER, AND THIS IS WHERE IT BITES. `crate::ladder` carries the
    // rule and the operator's own words for it.
    //
    // AFTER the target list is built, because the gate probes the store for
    // THESE instruments and cannot be asked before they are known. BEFORE
    // `note_run_started`, because a run the order refuses never started — and
    // before any socket, which is the entire point: the cheap day pass exists
    // to find a wrong feed, symbol or window in 14 requests instead of 81.
    // THE CENSUS IS THE CALLER'S, AND IT MUST BE A FRESH ONE.
    //
    // This read `site.censuses`, which is filled once in `Site::load` and never
    // again — D-0039, and the field's own doc says so. So a day pass written by
    // THIS process was invisible to the gate for the life of the process: the
    // bars were on disk, the manifest counted them, and the minute run that
    // followed was still refused for want of them. The documented workaround
    // was to restart the server, which is a workaround for a bug.
    //
    // It is a parameter rather than a read inside here so the two callers can
    // each answer honestly: `autopilot::tick` already takes a fresh census
    // either side of this call, and a hand run reads one per request.
    if let Some(why) = ladder_refusal(asked, &targets, censuses) {
        return BrokerRun::out_of_order(why);
    }

    note_run_started(asked, targets.len());
    let mut out = BrokerRun {
        attempted: targets.len(),
        ..BrokerRun::default()
    };
    // THE STOP GENERATION, CAPTURED ONCE. A run compares against the value it
    // started with, so a pause that arrives after this run began stops it and a
    // pause that happened before it began does not.
    let epoch = site.autopilot.epoch();

    // The month these bars are for, named once rather than per instrument.
    let month = asked
        .window
        .from()
        .year_month()
        .map_or_else(|_| String::new(), |m| m.to_string());

    for (index, instrument) in targets.iter().enumerate() {
        // PAUSE BITES WITHIN A CELL, NOT WITHIN A MONTH. One relaxed load. A
        // month is five to thirty-seven minutes on this store, and an operator
        // who presses Pause must not wait out the other seven hundred
        // instruments to be obeyed.
        if site.autopilot.stopped(epoch) {
            out.stopped = Some(format!(
                "{} — stopped after {} of {} instruments. The partial month is \
                 refilled on resume, because the resume point is the store's own.",
                crate::autopilot::CANCELLED,
                out.reached,
                targets.len()
            ));
            break;
        }
        // WHAT IS ON THE WIRE, RIGHT NOW. This is the difference between a page
        // that shows a backfill working and one an operator cannot tell from a
        // hang — D-0057's `now` object. One uncontended lock per instrument,
        // against an instrument that costs a network round trip, so it is free
        // in the only units that matter.
        site.autopilot.publish(|status| {
            status.now = Some(crate::autopilot::InFlight {
                instrument: instrument.underlying.to_string(),
                month: month.clone(),
                timeframe: asked.granularity.to_string(),
                feed: asked.feed.display().to_owned(),
                index: index.saturating_add(1),
                of: targets.len(),
                since: std::time::Instant::now(),
            });
        });
        match broker_window(asked, instrument, site).await {
            Err(why) => {
                // THE MARKER IS READ AND REMOVED HERE, so it never reaches an
                // operator and never reaches the journal. One instrument that
                // got as far as a socket makes the whole run wire-touching:
                // the question the status answers is whether the VENDOR was
                // ever asked, and once it has been, it has been.
                let reached_wire = why.starts_with(WIRE_REACHED);
                out.touched_wire = out.touched_wire || reached_wire;
                let why = why.trim_start_matches(WIRE_REACHED);
                let why = format!("{}: {why}", instrument.underlying);
                // THE WHOLE REASON, WHERE IT IS NOT TRUNCATED.
                //
                // The audit journal is a FIXED 256-byte stride, so a refusal
                // longer than the record is cut off — and these are: the
                // journal reports `note_bytes: 635` beside a note that stops
                // mid-sentence, exactly where the vendor's own words begin.
                // The operator is then told a request failed and not why, which
                // is the state this whole session was spent diagnosing by hand.
                //
                // The rolling log has no such stride, so the reason lands whole
                // here and the journal keeps its constant-width record. Two
                // surfaces, each doing what its format can actually do.
                let emitted = telemetry::emit(
                    &telemetry::Event::error("pull.spot", "instrument refused")
                        .with(
                            "instrument",
                            telemetry::Value::Str(&instrument.underlying.to_string()),
                        )
                        .with("month", telemetry::Value::Str(&month))
                        .with("feed", telemetry::Value::Str(asked.feed.wire()))
                        .with("index", telemetry::Value::Uint(index as u64 + 1))
                        .with("of", telemetry::Value::Uint(targets.len() as u64))
                        .with("why", telemetry::Value::Str(&why)),
                );
                debug_assert!(
                    emitted.is_written() || telemetry::global().is_none(),
                    "a refusal that cannot be logged is the defect this event exists to remove"
                );
                site.autopilot
                    .fail(&instrument.underlying.to_string(), &month, &why);
                out.refused.push(why);
            }
            Ok(landed) => {
                out.reached += 1;
                out.origin.clone_from(&landed.origin);
                let landed_one = land_one(&landed, site);
                // A MEMBER THAT REACHED THE VENDOR AND STILL DID NOT LAND IS A
                // FAILURE, and it is a different one from a refusal — the
                // socket worked and the store did not. Both belong on the page;
                // only naming the first would hide the disk.
                for failure in &landed_one.failures {
                    note_member_failure(failure, &month, asked, site);
                }
                out.total.absorb(landed_one);
            }
        }
    }
    // NOTHING IS ON THE WIRE ANY MORE. Left set, a finished run would keep
    // claiming to be fetching the last instrument it touched for as long as the
    // process lived.
    site.autopilot.publish(|status| status.now = None);
    out.took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let balanced = out.total.balances();
    note_run_finished(&out, balanced);
    out
}

/// The credential, the client and one window — or the reason there is none.
///
/// Returns the window, what to file it under, and where it came from.
/// A window from a broker, **carrying the descriptor row it came from**.
///
/// # Why the spec travels with the bars
///
/// This used to return `(RawWindow, String, String)` and drop the `HttpSpec` on
/// the floor. The caller then had to state the timestamp encoding and the store
/// vendor for itself, and it did — as two literals, `EpochSecondsUtc` and
/// `Vendor::Dhan`, sixteen lines after the code above had resolved which vendor
/// was actually asked for and refused an unknown one by name.
///
/// The effect was that `pull::vendor` governed six fields of the request and
/// none of the answer. Every Groww window arrived stamped in milliseconds, was
/// read as seconds, landed some six hundred thousand years out, and was refused
/// at row 0 — so Groww was selectable on the form and could never return a bar.
/// Correcting only that would then have filed Groww's prices under Dhan's store
/// prefix, which `Feed::store_vendor`'s own documentation calls destroying
/// D-0019's per-vendor independence irreversibly. That function existed the
/// whole time and had no caller anywhere in the workspace.
///
/// Returning the spec is what makes the comment forty lines above true — "adding
/// a broker is a row in `crate::vendor`, not an edit here". A literal at this
/// seam is an edit here.
struct BrokerWindow {
    /// The bars, exactly as the vendor sent them — ONE BODY PER LEGAL CHUNK,
    /// in window order.
    ///
    /// A vendor caps how much history one request may name, so a window wider
    /// than that cap is several requests and several answers. It was one field
    /// and one request, which is why a multi-year range was handed whole to a
    /// vendor that would not serve it.
    ///
    /// EACH BODY CARRIES THE WINDOW IT WAS FETCHED FOR. It was a bare
    /// `Vec<RawWindow>`, and `RawWindow` is `{ rows }` — so the chunk boundary
    /// `fetch_chunks` had just computed died at that return, and `land_one` had
    /// nothing left to filter a body against but the operator's whole range.
    /// `Window::verdict` could then never answer `BeforeWindow` or
    /// `AfterWindow` for any row of any chunk, which is why a run that lost 80
    /// of 122 members reported `0 rows dropped` from a counter that could not
    /// fire. A counter that cannot be non-zero reporting zero is the fallback
    /// that hides a failure `CLAUDE.md` §4 bans.
    bodies: Vec<(pull::session::Window, pull::fetch::RawWindow)>,
    /// Which instrument they are.
    instrument: String,
    /// The URL they came from, for the receipt.
    origin: String,
    /// The row that describes this feed's wire format. Read for its timestamp
    /// encoding rather than re-stated by the caller.
    spec: pull::vendor::HttpSpec,
    /// Where these bars belong, from the instrument rather than a literal.
    pub exchange: &'static str,
    /// Where these bars belong, from the instrument rather than a literal.
    pub segment: &'static str,
    /// The window these bodies cover, so the lander need not be handed the
    /// request as well.
    pub window: pull::session::Window,
    /// The rung they were asked for, travelling with them for the same reason
    /// the spec does: the lander would otherwise have to re-state it, and a
    /// re-stated literal here files daily bars under `1min/`.
    pub granularity: pull::vendor::Granularity,
    /// The contract these bars belong to, or `None` for spot.
    ///
    /// Carried rather than derived because only the caller knows which series
    /// it asked the broker for: the answer is a list of bars and nothing in it
    /// names the contract. `None` is what makes the store path one level
    /// shallower, so a derivative that lost this on the way here would file its
    /// bars in the underlying's own directory — where the next spot pull for
    /// that month would append to them.
    contract: Option<brutex_core::instrument::Contract>,
    /// The store prefix these bars belong under, resolved through
    /// [`pull::vendor::Feed::store_vendor`] and never assumed.
    store_vendor: brutex_core::vendor::Vendor,
}

/// A broker is never asked for a day that has not finished.
///
/// A session still running yields a PARTIAL day. The store is append-only with
/// idempotent re-append, so a half day is not a smaller answer that a later run
/// improves — it is either refused on the corrected re-pull, for being a
/// different set of bars under the same key, or it is a permanent gap nobody
/// notices because the month file exists and the census counts it. `CLAUDE.md`
/// §3 rule 5 makes reruns safe; it cannot make a half-written day whole.
///
/// Yesterday is the newest day that is certainly finished, whatever hour this
/// runs at, and it needs no session table to know that.
///
/// # Why this is not in `parse_window`
///
/// Both ingest paths share that parser. This rule is about TALKING TO A VENDOR,
/// so it belongs beside the others that are: the rate budget, the credential,
/// the market-hours window. A `Transport::LocalArchive` feed is CSV files an
/// operator has bought and placed in a folder — there is no vendor to be
/// mid-session with, and a file's last day is not a question about the clock.
fn finished_day_only(asked: &ingest::SpotRequest) -> Result<(), String> {
    let now = ingest::ist_moment(std::time::SystemTime::now())
        .map_err(|why| format!("the clock is unusable, so today cannot be established: {why}"))?;
    let today = now.day();

    // TODAY IS ASKABLE ONCE ITS SESSION HAS CLOSED, and it was not.
    //
    // This refused `to >= today` unconditionally — at 21:11 IST, five and a
    // half hours after the exchange shut, on a day whose bars had been final
    // since 15:30. Measured: 213 instruments attempted, 0 reached, every one
    // refused in 13.9 ms with "a session that is still running yields a partial
    // day". No session was running.
    //
    // The BROWSER already had this right. `web/src/routes/ingest/+page.svelte`
    // computes its ceiling as *today if the session has closed, else the
    // previous session*, and says so in a sentence beside the date box. So the
    // page offered a day and the server refused it — an operator following the
    // page's own instruction got a 502 and a message about a session that had
    // ended hours earlier.
    //
    // WHAT IS UNCHANGED IS THE RULE THIS EXISTS FOR. A partial day must never
    // be stored: the store is append-only, so a half session written now can
    // never be completed, only refused later. That is still enforced — the test
    // is now whether the session has ENDED rather than whether the calendar day
    // has. A future day, and today while the market is open, both still refuse.
    //
    // The close comes from the session table rather than a literal 15:30,
    // because the close MOVES: NSE's CAS change put the index at 15:15 from
    // 2026-08-03 while cash kept 15:30, and a hardcoded time would have been
    // wrong for one of them from that date (D-0151).
    //
    // THIS COMMENT USED TO CITE MUHURAT, AND THAT WAS A CLAIM ABOUT A ROW THAT
    // DOES NOT EXIST. It said a literal "would refuse a finished Muhurat day
    // for forty-five minutes", which reads as though the table holds the 2025
    // Muhurat hours. It does not, no session row for any Muhurat date exists,
    // and `SessionRow` carries ONE open and ONE close so it could not hold an
    // evening session beside a regular one. `docs/06-limits.md` §68 records
    // what closing that would take. Reading the table is still right; the
    // reason it is right is the CAS split, which is real.
    // NSE CASH, because that is the venue whose 15:30 close this is about and
    // the one every spot instrument in the engine surface trades on. A venue
    // with no row for the day — a holiday, or a date past the table — answers
    // `Err`, and that is read as CLOSED: a day the exchange did not trade has
    // no session left to finish, so nothing is gained by refusing it.
    // ONE LOOKUP. A day the exchange did not trade answers `Err`, and that reads
    // as CLOSED — there is no session left to finish, so nothing is gained by
    // refusing it.
    let closed = pull::vendor::Venue::NseCash
        .hours_on(today)
        .map_or(true, |session| {
            now.minute_of_day() >= session.close_minute()
        });
    if asked.window.to() > today || (asked.window.to() == today && !closed) {
        // ONE COPY of this prose, in the `Refusal` that owns it. A second
        // hand-written sentence here would be the thing that drifts.
        return Err(ingest::Refusal::WindowReachesToday {
            to: asked.window.to(),
            today,
        }
        .to_string());
    }
    Ok(())
}

/// Spend one rate permit, WAITING for it rather than refusing.
///
/// # Why waiting is the right answer and refusing is not
///
/// The governor does not merely say no — it says *when*. This uses that.
///
/// Measured on the real universe with the refusing version: 785 instruments
/// attempted, **6 reached, 779 refused**, the whole run over in 5.2 seconds,
/// every refusal reading `Groww's minute budget is spent. The next request is
/// admitted in 0.198s. Nothing was asked of the vendor.` The governor was
/// right every time and the vendor was never troubled; the pull simply would
/// not wait a fifth of a second. Refusing turns "slow down" into "give up",
/// and a backfill that gives up is one a human has to restart — the manual
/// intervention this exists to remove.
///
/// # Why it is bounded, and why the bound is loud
///
/// [`MAX_ADMISSION_WAITS`] attempts, after which the refusal is returned
/// unchanged. Each attempt sleeps the governor's own arithmetic, so this is 64
/// *earned permits* of patience rather than 64 blind retries. A governor still
/// denying after that many full waits is not congested, it is misconfigured —
/// a ceiling below one request per span makes every wait futile — and
/// `CLAUDE.md` §4 bans a fallback that hides a failure.
///
/// # Errors
///
/// A poisoned budget lock, or a feed with no HTTP transport: returned on the
/// first attempt without sleeping, because neither becomes true later.
async fn await_budget(feed: pull::vendor::Feed, site: &Site) -> Result<(), String> {
    let mut last = String::new();
    for _ in 0..MAX_ADMISSION_WAITS {
        // ONE `admit` CALL PER ATTEMPT, and the lock is dropped before the
        // sleep.
        //
        // `admit` is not a question, it is a WITHDRAWAL: on `Admit` it has
        // already spent the permit. Asking twice — once to decide and once to
        // read the wait — spends a permit and throws it away, which over a
        // 97,524-request backfill is a leak measured in thousands. The verdict
        // is taken once and both branches are served from it.
        //
        // The lock is released before the sleep because holding a
        // `std::sync::Mutex` across an await point would stall every other
        // instrument for the duration of one instrument's wait, turning a
        // governor into a global serialiser.
        let verdict = {
            let mut budgets = site.budgets.lock().map_err(|_| {
                format!(
                    "the rate budget for {} cannot be read: another request \
                     panicked while holding it, so the allowance already spent \
                     is unknown. Nothing is issued on an unknown budget.",
                    feed.display()
                )
            })?;
            let Some(Some(governor)) = budgets.get_mut(feed as usize) else {
                return Err(format!(
                    "{} has no rate budget, which means it declares no HTTP \
                     transport. Reaching this function is a routing error \
                     rather than an operator one.",
                    feed.display()
                ));
            };
            governor.admit(monotonic_micros())
        };

        let pull::rate::Verdict::Deny { span, wait_micros } = verdict else {
            return Ok(());
        };
        last = format!(
            "{}'s {} budget is spent. The next request is admitted in \
             {}.{:03}s. Nothing was asked of the vendor and nothing was written.",
            feed.display(),
            // `WindowSpan` has no operator-facing name of its own; its Debug is
            // already exactly the word an operator wants.
            format!("{span:?}").to_lowercase(),
            // INTEGER SECONDS AND MILLISECONDS. `{:.3}` on an `f64` is the
            // obvious way to write this and clippy denies it workspace-wide —
            // correctly. A duration that is exact integer arithmetic stays it.
            wait_micros / 1_000_000,
            (wait_micros % 1_000_000) / 1_000
        );
        // THE GOVERNOR SAID WHEN, so sleep exactly that. A fixed sleep is
        // either longer than the wait (throughput thrown away) or shorter (a
        // spin). +1 ms so the clock has certainly passed the instant rather
        // than landing exactly on it, which would deny once more.
        tokio::time::sleep(std::time::Duration::from_micros(wait_micros + 1_000)).await;
    }
    Err(format!(
        "{last} — and that wait was taken {MAX_ADMISSION_WAITS} times without \
         the permit ever being earned, so the ceiling is below one request per \
         span and waiting longer cannot help."
    ))
}

/// How many full waits are taken before a rate refusal is believed.
const MAX_ADMISSION_WAITS: u32 = 64;

/// A monotonic microsecond reading, which is what [`pull::rate::Governor`] asks
/// for and deliberately will not read itself.
///
/// The governor takes the clock as an argument so a test can drive it without
/// sleeping. This is the one place that has to supply a real one.
fn monotonic_micros() -> u64 {
    use std::sync::OnceLock;
    static ORIGIN: OnceLock<std::time::Instant> = OnceLock::new();
    ORIGIN
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_micros()
        // A `u128` of microseconds since process start exceeds `u64` after
        // ~584,000 years. Saturating rather than wrapping, because a wrapped
        // clock would hand the governor a reading in the past and refill every
        // bucket at once.
        .try_into()
        .unwrap_or(u64::MAX)
}

/// The requested window, narrowed to what this feed can actually answer.
///
/// # Errors
///
/// When the whole window is below the floor — the operator asked only for days
/// the vendor no longer has. Refused by name rather than answered empty,
/// because an empty answer here is indistinguishable from a market holiday and
/// would be recorded as "nothing to store" rather than "nothing available".
///
/// # THE DAY IS THE CALLER'S, AND THAT IS THE WHOLE POINT OF THE PARAMETER
///
/// This function used to read `SystemTime::now()` itself, in both rolling arms.
/// It therefore ignored any day a caller threaded in, and
/// `the_two_floors_that_name_no_day_resolve_to_none` — which passes a fixed
/// 2026-08-12 and asserts 2026-05-12 — was GREEN ON EXACTLY ONE DAY and red on
/// every other. It had been red for two days when this was found.
///
/// A function that reads the clock cannot be tested against a day, and
/// `pull::vendor`'s own floor test already states the rule this one broke: "a
/// test that reads the clock asserts a different thing every day it runs".
///
/// So the clock moves OUT, to the three callers. Each reads it exactly where it
/// already had the means to, production behaviour is unchanged to the day, and
/// the resolution itself is now a pure function of (window, floor, today).
/// Whether an ARCHIVE feed has data the operator actually bought, and why not.
///
/// # Why the folder and not the census
///
/// A broker proves entitlement with a credential, so it is ready with an empty
/// store — `SourceKind::needs_credential` answers that and this function is
/// never reached for one. An archive has no credential, so the only evidence is
/// the files themselves. Asking the census instead made readiness circular: the
/// census fills from an ingest, the ingest needs readiness, so a freshly bought
/// archive could never be selected to do the pull that would make it
/// selectable.
///
/// # Three refusals, never one
///
/// `CLAUDE.md` §4 forbids collapsing distinct failures into one silence, and
/// these three are the operator's whole diagnostic path — they tell him whether
/// to set a path, fix a permission, or copy his files in:
///
/// * the root or the feed's folder cannot be resolved at all;
/// * the folder is not there yet;
/// * the folder is there and holds nothing.
///
/// # Cost
///
/// O(1). `read_dir` opens the directory and `next()` pulls ONE entry — the
/// count of files in it is never walked, so a folder holding 100,000 CSVs
/// answers in the same time as one holding a single file. That matters because
/// this runs on the `/feeds.json` render path, where the comment above the
/// readiness match already warns that an O(files) probe is the cost `/store`
/// exists to avoid.
///
/// UNVERIFIED as a measured figure. Nothing here was timed against a folder of
/// 100,000 CSVs, and the sentence above about one answering as fast as a folder
/// of one is what the STRUCTURE implies — one `read_dir` and one `next()`,
/// never a `count()` and never a `collect()` — not a reading anybody took.
/// `CLAUDE.md` §3 rule 6, and the same admission `docs/06-limits.md` §65 makes
/// about the walk this probe exists to avoid.
fn archive_ready(feed: pull::vendor::Feed) -> (bool, String) {
    let root = match pull::folder::root() {
        Ok(root) => root,
        Err(why) => {
            return (
                false,
                format!("the archive root could not be resolved: {why}"),
            );
        }
    };
    let dir = match pull::folder::folder_of(&root, feed) {
        Ok(dir) => dir,
        Err(why) => {
            return (
                false,
                format!(
                    "{}'s archive folder could not be named: {why}",
                    feed.display()
                ),
            );
        }
    };
    let shown = dir.display();
    match std::fs::read_dir(&dir) {
        // NOT THERE YET is not the same fact as EMPTY, and the operator acts
        // differently on each: one is a path to create or configure, the other
        // is files to copy.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => (
            false,
            format!("no folder at {shown} yet — the bought CSVs go there"),
        ),
        Err(why) => (false, format!("{shown} could not be read: {why}")),
        Ok(mut entries) => match entries.next() {
            Some(_) => (true, String::new()),
            None => (
                false,
                format!(
                    "{shown} is there and holds nothing — the bought CSVs have not been copied in"
                ),
            ),
        },
    }
}

pub(crate) fn clamp_to_floor(
    window: pull::session::Window,
    floor: pull::vendor::HistoryFloor,
    today: pull::session::Day,
) -> Result<pull::session::Window, String> {
    use pull::vendor::HistoryFloor;

    let oldest = match floor {
        // TWO DIFFERENT FACTS, ONE ARM, AND THE ARM IS NOT WHERE THEY DIFFER.
        //
        // `Unstated` is nobody having published a floor — inventing one would
        // be the vendor fact `CLAUDE.md` §3 rule 1 forbids. `Unbounded` is the
        // vendor stating it serves back to the instrument's own inception.
        // Neither narrows a window, so clamping cannot tell them apart and
        // must not pretend to; `/feeds.json` reports them as `unknown` and
        // `none`, which is where the difference is visible to an operator.
        HistoryFloor::Unstated | HistoryFloor::Unbounded => return Ok(window),
        // A ROLLING FLOOR STATED IN MONTHS, resolved the same way the years
        // arm is: from today's clock, never from a stored date. The calendar
        // walk lives on `Day` because a month is not a fixed number of days
        // and dividing one into 30 would be a figure no vendor published.
        HistoryFloor::RollingMonths { months } => today
            .months_before(months)
            .map_err(|why| format!("the rolling floor lands before the epoch: {why}"))?,
        HistoryFloor::Fixed { year, month, day } => pull::session::Day::new(year, month, day)
            .map_err(|why| format!("the feed's history floor is not a real day: {why}"))?,
        HistoryFloor::Rolling { years } => {
            // RECOMPUTED FROM TODAY, never stored. A stored rolling floor is a
            // frozen one, and a frozen rolling floor is the bug this exists to
            // prevent. `today` is the caller's — see the header.
            //
            // 365.25 days per year, so four years of leap days do not drift the
            // floor a day earlier than the vendor's own.
            let back = u32::try_from(u64::from(years) * 36_525 / 100).unwrap_or(u32::MAX);
            pull::session::Day::from_days(today.days_from_epoch().saturating_sub(back))
                .map_err(|why| format!("the rolling floor lands before the epoch: {why}"))?
        }
    };

    if window.to() < oldest {
        return Err(format!(
            "this feed's history starts at {oldest} and the window ends {}. \
             Every day asked for is older than the vendor holds — it would \
             answer empty, which reads exactly like a market holiday and would \
             be recorded as nothing to store rather than nothing available.",
            window.to()
        ));
    }
    if window.from() >= oldest {
        return Ok(window);
    }
    // The window straddles the floor. Take the part that exists.
    pull::session::Window::new(oldest, window.to())
        .map_err(|why| format!("the clamped window is not forward: {why}"))
}

/// Every legal chunk of the operator's window, fetched over one client.
///
/// A vendor caps how much history one request may name — 30 days for Groww at
/// one-minute granularity, 90 for Dhan, both from `docs/00-charter.md` §4.
/// `api::ingest::MAX_WINDOW_DAYS` is 3,653 and says nothing about that, so a
/// 2020-to-today window passed the parser and was handed WHOLE to a vendor that
/// will not serve it: an error if the operator was lucky, a silently truncated
/// answer if not, and the second writes a gap the append-only store cannot
/// correct.
///
/// The split happens here rather than in the caller so the credential, the
/// identity and the client are paid for once and reused across every chunk.
/// 2020-01-01 to yesterday is 81 chunks at Groww's cap — 81 Parameter Store
/// round-trips would be the obvious way to write this and the wrong one.
///
/// # The cap is looked up PER RUNG, and an absent one is not "no split"
///
/// `docs/00-charter.md` §4 records Groww's cap **at 1-minute granularity** and
/// Dhan's against its intraday-charts endpoint. Neither records a day-level
/// figure, so neither descriptor carries one — UNVERIFIED, absent rather than
/// guessed. `split_window` takes the `Option` and still breaks every chunk at a
/// month boundary, because the store addresses one month per file at every
/// rung. That is why the `None` arm that sent the window WHOLE is gone: it was
/// the un-split request this function exists to prevent, wearing the label of a
/// vendor that published nothing.
async fn fetch_chunks(
    asked: &ingest::SpotRequest,
    site: &Site,
    source: &pull::http::HttpSource,
    instrument_id: &str,
    // BESIDE THE ID, because they are read from the same master row and the
    // vendor requires both. See the `listing` field on the request below.
    listing: pull::vendor::Listing,
    // BY REFERENCE: `HttpSpec` is 280 bytes and only one field is read.
    spec: &pull::vendor::HttpSpec,
) -> Result<Vec<(pull::session::Window, pull::fetch::RawWindow)>, String> {
    // CLAMPED TO THE FEED'S HISTORY FLOOR BEFORE ANYTHING IS SPLIT.
    //
    // Asking below the floor is not an error the vendor reports usefully: it
    // answers EMPTY, and an empty answer is indistinguishable from a day that
    // did not trade. So a 2020-to-yesterday backfill against a feed whose
    // history starts later spends real requests on data that does not exist and
    // reports success.
    //
    // For Dhan it is worse than waste. Its floor ROLLS — `docs/00-charter.md`
    // §4: "rolling ~5 years. Not a fixed floor — it moves every day". Asking
    // for 2020 today is asking for ~5 months it no longer has, and that gap
    // widens every month while nothing notices. Clamping is what stops
    // "complete" from being a claim with an expiry date.
    let today = ingest::ist_day(std::time::SystemTime::now())
        .map_err(|why| format!("the clock is unusable: {why}"))?;
    let asked_window = clamp_to_floor(asked.window, spec.history_floor, today)?;

    let chunks = pull::session::split_window(asked_window, spec.window_cap_days(asked.granularity))
        .map_err(|why| format!("the window could not be split to the vendor's cap: {why}"))?;

    let mut bodies = Vec::with_capacity(chunks.len());
    for (nth, chunk) in chunks.iter().enumerate() {
        // THE BUDGET IS CHARGED PER REQUEST, NOT PER FORM SUBMISSION. One
        // submission is up to 81 requests; charging once would spend one permit
        // for eighty-one calls and make the governor a decoration.
        //
        // The first chunk's permit was taken in `broker_window`, before the
        // credential was read — a request that will not be issued must not cost
        // a round-trip to ap-south-1. The rest are charged here.
        if nth > 0 {
            await_budget(asked.feed, site).await?;
        }

        let request = pull::fetch::BarRequest {
            instrument_id: instrument_id.to_owned(),
            // THE CLASS TRAVELS WITH THE ID, FROM THE SAME MASTER ROW.
            //
            // Separating them is precisely how 750 NIFTY-Total-Market equities
            // came to be asked for inside Dhan's INDEX segment: the id was
            // resolved per instrument and the segment was a fixed word. Both
            // are properties of the one row, so both are read from it.
            listing,
            window: *chunk,
            // The rung goes ON THE WIRE for a feed that spells one, and it is
            // the same rung the answer is filed under. A feed with no recorded
            // word for it refuses here, before the socket — see
            // `pull::fetch::FetchError::RungNotSpellable`.
            granularity: asked.granularity,
        };
        bodies.push((
            // THE CHUNK'S OWN WINDOW, travelling with the answer it belongs to
            // rather than being recomputed — or, as it was, discarded.
            *chunk,
            with_retry(source, &request, asked.feed, site)
                .await
                .map_err(|why| {
                    // THE VENDOR'S OWN WORDS, IN THEIR OWN FIELD.
                    //
                    // Logging the wrapped string below loses them: telemetry
                    // bounds a string value at MAX_STR_VALUE_BYTES (128) to keep
                    // an event O(1) in space, and the prose is longer than that
                    // before `{why}` is even reached — so the operator got a
                    // sentence about chunk counts and nothing about the failure.
                    //
                    // Emitted here, unwrapped, the reason is the whole field and
                    // fits. The bound stays where it is; what changes is that
                    // the field carries the answer rather than the preamble.
                    let noted = telemetry::emit(
                        &telemetry::Event::error("pull.http", "vendor refused a window")
                            .with("instrument_id", telemetry::Value::Str(instrument_id))
                            .with("feed", telemetry::Value::Str(asked.feed.wire()))
                            .with("chunk", telemetry::Value::Uint(nth as u64 + 1))
                            .with("of", telemetry::Value::Uint(chunks.len() as u64))
                            .with("from", telemetry::Value::Str(&chunk.from().to_string()))
                            .with("to", telemetry::Value::Str(&chunk.to().to_string()))
                            .with("vendor_said", telemetry::Value::Str(&why)),
                    );
                    debug_assert!(
                        noted.is_written() || telemetry::global().is_none(),
                        "the vendor's reason is the one field this event exists to carry"
                    );
                    format!(
                        "the broker did not answer with a window. This was \
                         request {} of {}, covering {}..={} — the chunks before \
                         it were fetched and are not written, because a partial \
                         answer to a whole request is a gap this store cannot \
                         correct later: {why}",
                        nth + 1,
                        chunks.len(),
                        chunk.from(),
                        chunk.to()
                    )
                })?,
        ));
        // ONE LINE PER REQUEST THAT ACTUALLY WENT OUT, at `Trace`.
        //
        // The refusal path has been logged since `pull.http` existed; the
        // SUCCESS path never was, so a run that was merely slow and a run that
        // was silently returning nothing looked identical from the log. The
        // row count is what tells those two apart.
        //
        // `Trace` and not `Debug`: a one-minute backfill is ~62,600 chunks, and
        // members are already at `Debug`. An operator who wants the wire turns
        // the floor all the way down for one run and accepts that the 64 MiB
        // window holds less of it.
        let rows = bodies.last().map_or(0, |(_, body)| body.rows.len());
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::trace("pull.chunk", "answered")
                .with("instrument_id", telemetry::Value::Str(instrument_id))
                .with("feed", telemetry::Value::Str(asked.feed.wire()))
                .with("chunk", telemetry::Value::Uint(nth as u64 + 1))
                .with("of", telemetry::Value::Uint(chunks.len() as u64))
                .with("from", telemetry::Value::Str(&chunk.from().to_string()))
                .with("to", telemetry::Value::Str(&chunk.to().to_string()))
                .with("rows", telemetry::Value::Uint(rows as u64)),
        );
    }
    Ok(bodies)
}

/// How many times a chunk is attempted before the instrument is given up on.
///
/// Three, not one and not ten. A 62,600-request backfill will meet a timeout, a
/// reset connection and a 5xx many times over; giving up on the first turns
/// each into a lost instrument. Ten would keep hammering a vendor that is down,
/// which the rate governor cannot see because a refused connection never
/// reaches it.
const THROTTLE_ATTEMPTS: u32 = 6;

/// Attempts allowed for a **5xx**, which is fewer than a transport blip gets.
///
/// A 5xx is an ANSWER: the vendor was reached, its front door works, and its
/// own side failed. That is worth re-asking — the same bytes may well be
/// answered next time — but it is much weaker evidence of *transience* than a
/// connection that never completed, because a broken backend usually stays
/// broken for longer than a backoff ladder.
///
/// The full [`THROTTLE_ATTEMPTS`] ladder on the quadratic wait is
/// `250 + 1000 + 2250 + 4000 + 6250 ms` — 13.75 s spent per instrument before
/// giving up. A one-minute backfill is ~785 instruments, so a vendor having a
/// bad hour would spend about three hours asleep discovering that, one
/// instrument at a time, and the run would look hung rather than failing.
///
/// Three attempts cost at most `250 + 1000 ms`. That survives the blip this
/// exists for and reports the outage in a length of time an operator will
/// actually watch.
const SERVER_ERROR_ATTEMPTS: u32 = 3;

/// The longest a single refused chunk may sleep, in milliseconds.
///
/// This is the bound a runtime `.min()` inside [`step`] reached for and could
/// never enforce, because the cap sat above every value the ladder produces.
/// Stated here it is checked when the code is COMPILED, so raising
/// [`THROTTLE_ATTEMPTS`] past what the exponential ladder can afford is a build
/// failure naming this constant, rather than a run that quietly sleeps for
/// minutes on one instrument.
///
/// 31,000 ms is `1000 + 2000 + 4000 + 8000 + 16000` — the five waits a 429 can
/// spend before the sixth attempt gives up.
const MAX_CHUNK_BACKOFF_MS: u64 = 31_000;

const _: () = {
    // Summed the same way `step` computes each wait, so the two cannot drift.
    let mut spent = 0u64;
    let mut attempt = 1u32;
    while attempt < THROTTLE_ATTEMPTS {
        spent += 1_000u64 << (attempt - 1);
        attempt += 1;
    }
    assert!(
        spent <= MAX_CHUNK_BACKOFF_MS,
        "the 429 ladder now sleeps longer than a chunk is allowed to; lower \
         THROTTLE_ATTEMPTS or raise MAX_CHUNK_BACKOFF_MS deliberately"
    );
    // And the shift must stay defined: `u64 << 64` is a panic, not a big number.
    assert!(THROTTLE_ATTEMPTS <= 64);
};

/// One chunk, retried on the failures that are worth retrying.
///
/// # Which failures, and why not all of them
///
/// A TRANSPORT failure is worth retrying: a timeout, a reset, a DNS blip, a
/// 5xx. Nothing about the request was wrong and the same request may well
/// succeed. Over the ~62,600 requests of the stated one-minute backfill these
/// are certainties, not edge cases, and giving up on the first costs that
/// instrument entirely.
///
/// A REFUSAL is not. `DH-905 missing required fields` will be refused
/// identically three times, and retrying it spends three times the rate budget
/// to learn what the first answer already said. Worse, it hides the real error
/// behind two duplicates in the log.
///
/// # The backoff, and why it is not exponential
///
/// 250 ms, then 1 s. Two waits, bounded, because the caller is an HTTP request
/// an operator is holding open — a run that backs off for a minute to recover
/// one chunk has lost the operator's attention and the connection. The rate
/// Refuses a rung the feed does not declare, **by name**.
///
/// # Why this is a function and not a line in `broker_window`
///
/// `Feed::serves` existed and had NO caller outside its own tests, so the
/// `granularities` row in `crate::vendor` was documentation rather than a gate.
/// Withdrawing Dhan's `Minute1` there — because that vendor's `bars_path` is
/// pinned to its DAILY endpoint and the request carries no `interval` field —
/// therefore changed nothing: `autopilot::round` pins `Granularity::Minute1`
/// for every feed, so a minute request still reached the daily endpoint and its
/// daily candles were filed under `1min/`.
///
/// Silent wrong data in an append-only store is the outcome `CLAUDE.md` §4
/// singles out: the month cannot be prepended, the bars look plausible, and no
/// later reader can tell them from real one-minute bars.
///
/// # Errors
///
/// The feed's descriptor does not declare this rung.
///
/// One bitmask test — constant work, and it runs before the credential read so
/// a request that will not be issued costs no round-trip to ap-south-1.
fn served(feed: pull::vendor::Feed, rung: pull::vendor::Granularity) -> Result<(), String> {
    if feed.serves(rung) {
        return Ok(());
    }
    Err(format!(
        // THE REASON IS STATED GENERALLY, because it stopped being true of one
        // feed the day that feed gained a second endpoint. It read "this feed's
        // request carries no interval field and its bars path is pinned to one
        // rung" — Dhan's shape before `vendor::HttpSpec::rung_routes`, quoted
        // as though it were every unserved rung's reason. It is not: a rung is
        // unserved because the DESCRIPTOR does not declare it, and why that
        // descriptor cannot carry it is a per-feed fact this sentence is not
        // the place to assert.
        "{} does not serve {rung} bars. Nothing was sent, because a rung this \
         feed's descriptor does not declare cannot be put on its wire — the \
         request would be answered with a DIFFERENT bar length and filed under \
         the one you asked for, and the bar length lives in the store PATH, so \
         no later reader could tell them apart. Pick a rung this feed declares, \
         or record the rung's endpoint and its wire word in `pull::vendor` \
         first.",
        feed.display()
    ))
}

/// What to do about one refusal — the whole retry policy, with no I/O in it.
///
/// # Why this is a function and not four `if`s inside the loop
///
/// It used to be four `if`s inside the loop, and the only test of it read this
/// file as TEXT and asserted that certain string literals appeared before the
/// word `sleep`. That test could not distinguish the policy from its wording:
/// it passed for a build that returned on the first 5xx and would have passed
/// for one that never retried anything, as long as the literals were in the
/// right order. It also brace-counted Rust source to find the function body,
/// which is unsound — braces live in string literals too.
///
/// Split out, every arm is reachable from a test with a `u16` and no socket,
/// and the loop below keeps only the parts that genuinely need one.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    /// The credential died mid-run. §8 forbids minting, so this stops.
    CredentialDied,
    /// The credential is ALIVE and this key is not entitled to the call.
    ///
    /// Split from [`Self::CredentialDied`] because the two differ in what the
    /// operator must go and do, and because collapsing them told an operator
    /// with no historical-data subscription that tomorrow's token would fix
    /// it. Only a vendor that NAMES its refusals can produce this — see
    /// `pull::refusal` and `pull::vendor::HttpSpec::error_names`.
    NotEntitled,
    /// The vendor gave a reason about the request. Asking again cannot change
    /// it.
    Answered,
    /// The vendor's own side failed, and has now said so this many times.
    ///
    /// Carried rather than assumed: the message used to interpolate
    /// [`SERVER_ERROR_ATTEMPTS`] directly, which asserted a count nothing had
    /// measured.
    ServerDown { answered: u32 },
    /// Ask again in `wait_ms`. `throttled` is the governor's cue to take its
    /// multiplicative decrease first.
    Again { wait_ms: u64, throttled: bool },
    /// Every attempt is spent.
    Exhausted,
}

/// The rate ladder: exponential, because the vendor named the arrival RATE and
/// 250 ms twice does not change a rate.
///
/// Lifted out of [`step`] so the status path and the vendor-named path use the
/// SAME ladder rather than two that can drift. See [`step`]'s header.
const fn throttle_ladder(attempt: u32) -> Step {
    if attempt >= THROTTLE_ATTEMPTS {
        return Step::Exhausted;
    }
    // NO CAP, BECAUSE A CAP HERE IS A BRANCH NOTHING CAN ENTER. The largest
    // reachable `attempt` in an `Again` is `THROTTLE_ATTEMPTS - 1` = 5, so the
    // largest wait is `1_000 << 4` = 16,000 ms and a `.min(30_000)` could never
    // fire. §4 bans a test that asserts nothing, and an uncoverable branch is
    // the same defect one level down. The bound it reached for is enforced at
    // COMPILE time instead — see `MAX_CHUNK_BACKOFF_MS`.
    Step::Again {
        wait_ms: 1_000_u64 << (attempt - 1),
        throttled: true,
    }
}

/// The backend ladder: quadratic in the 5xx count, and the governor is not
/// touched, because a 5xx names no budget.
///
/// Lifted for the same reason as [`throttle_ladder`].
const fn server_ladder(attempt: u32, server_errors: u32) -> Step {
    // COUNTED AGAINST 5xx ANSWERS, NOT AGAINST EVERY FAILURE — see [`step`]'s
    // parameter note for what reading the shared ordinal cost.
    if server_errors >= SERVER_ERROR_ATTEMPTS || attempt >= THROTTLE_ATTEMPTS {
        return Step::ServerDown {
            answered: server_errors,
        };
    }
    // So two timeouts before the first 502 do not push it straight to a
    // four-second wait.
    Step::Again {
        wait_ms: 250 * server_errors as u64 * server_errors as u64,
        throttled: false,
    }
}

/// The policy in [`Step`]'s terms.
///
/// `status` is the one the vendor sent, carried on
/// `pull::fetch::FetchError::VendorRefused`, or `None` when nothing was
/// answered at all. `invalid_auth` is the body-level marker a vendor writes
/// instead of a status.
///
/// `attempt` is the chunk's attempt ordinal, counting failures of every class.
/// `server_errors` counts only the 5xx answers, INCLUDING the one being judged.
///
/// # Why those are two numbers and not one
///
/// They were one. The 5xx cap read the shared ordinal, so a 502 arriving after
/// two timeouts on the same chunk was refused on the vendor's FIRST 5xx —
/// exactly the defect the 5xx retry was added to fix, reappearing whenever the
/// chunk had a bad minute first. And the message said "it answered that 3
/// times" when the vendor had answered once, which is a measurement the code
/// never took, stated as fact (§3 rule 6).
///
/// Two counters cost one `u32` and remove both.
///
/// Constant work — a handful of integer comparisons, no allocation.
const fn step(
    status: Option<u16>,
    invalid_auth: bool,
    named: Option<pull::refusal::Disposition>,
    attempt: u32,
    server_errors: u32,
) -> Step {
    use pull::refusal::Disposition;

    // THE VENDOR'S OWN NAME OUTRANKS THIS BUILD'S READING OF THE STATUS,
    // BECAUSE THE VENDOR INSTRUCTS EXACTLY THAT.
    //
    // Kite's exceptions page: "You can define corresponding exceptions in your
    // language or library, and raise them by doing a switch on the returned
    // exception name." `named` is `Some` only when the feed declares a
    // body-level contract AND the name that arrived is one its reader knows —
    // `pull::vendor::HttpSpec::error_names` — so every feed without one falls
    // through to the status match below, byte for byte as before.
    //
    // The pair this arm exists for is `NotEntitled`. It and `SessionDead` are
    // BOTH answered under 403, so the status match below cannot separate them,
    // and reporting the first as the second tells an operator with no
    // historical-data subscription to wait for a token refresh that can never
    // fix it.
    //
    // `Throttled` and `RetryBounded` call the SAME two ladders the status path
    // calls, so a vendor that names a rate refusal under an unexpected status
    // still gets the rate ladder rather than the backend one.
    if let Some(disposition) = named {
        return match disposition {
            Disposition::NotEntitled => Step::NotEntitled,
            Disposition::SessionDead => Step::CredentialDied,
            Disposition::RequestWrong | Disposition::ReasonGiven => Step::Answered,
            Disposition::Throttled => throttle_ladder(attempt),
            Disposition::RetryBounded => server_ladder(attempt, server_errors),
        };
    }
    if invalid_auth {
        return Step::CredentialDied;
    }
    match status {
        // 401 and 403 are the same fact spelled two ways. §4z: Kite answers 403
        // `TokenException` on expiry, on logout, and when the user logs into
        // another Kite instance.
        //
        // AND IT ALSO ANSWERS 403 `PermissionException` FOR AN UNENTITLED KEY,
        // which this arm cannot see and the `named` arm above can. A feed with
        // no declared contract still lands here and still cannot tell them
        // apart; that is a gap in what has been READ for that vendor, and it is
        // recorded in docs/06-limits.md rather than papered over.
        Some(401 | 403) => Step::CredentialDied,
        // The one refusal that means LATER.
        Some(429) => throttle_ladder(attempt),
        // The vendor's own side failed. Worth re-asking, on a shorter ladder
        // than a blip gets, and the governor is not touched: a 500 names no
        // budget.
        Some(500..=599) => server_ladder(attempt, server_errors),
        // Any other answer is a reason about the request.
        Some(_) => Step::Answered,
        // Nothing was answered, so it is a transport blip -- the case the
        // quadratic backoff was originally sized for.
        None => {
            if attempt >= THROTTLE_ATTEMPTS {
                return Step::Exhausted;
            }
            Step::Again {
                wait_ms: 250 * attempt as u64 * attempt as u64,
                throttled: false,
            }
        }
    }
}

/// One window, re-asked while the reason to re-ask still stands.
///
/// The refusal decides, and it decides from the status the vendor actually
/// sent rather than from this function's rendering of it:
///
/// * **401 / 403** — the credential died mid-run. Returned at once; §8 forbids
///   minting, and the refreshed value is read on the next pull.
/// * **429** — the one refusal that means *later*. The governor takes its
///   multiplicative decrease and the wait is exponential, because the vendor is
///   saying the arrival RATE is wrong and 250 ms twice does not change a rate.
/// * **5xx** — the vendor's own side failed. Retried on the quadratic transport
///   backoff, and the governor is not touched: a 500 names no budget. Capped at
///   [`SERVER_ERROR_ATTEMPTS`], which is shorter than the blip ladder, because
///   the vendor answered and a broken backend outlasts a backoff.
/// * **any other status** — a reason about the request, which will not change
///   because it was asked twice more. Returned at once.
/// * **no status at all** — nothing was answered, so it is a transport blip and
///   the quadratic backoff applies.
///
/// The governor already handles *sustained* throttling; this handles the blip.
///
/// # Errors
///
/// The last refusal, either returned early by the table above or after
/// [`THROTTLE_ATTEMPTS`] attempts failed to get an answer.
async fn with_retry(
    source: &pull::http::HttpSource,
    request: &pull::fetch::BarRequest,
    feed: pull::vendor::Feed,
    site: &Site,
) -> Result<pull::fetch::RawWindow, String> {
    let mut last = String::new();
    // Counted apart from `attempt`, so a 5xx budget is spent by 5xx answers.
    let mut server_errors = 0u32;
    for attempt in 1..=THROTTLE_ATTEMPTS {
        match source.window_async(request).await {
            // THE ADDITIVE INCREASE. Without this the governor admits
            // against a fixed budget forever and never learns the vendor's
            // real ceiling -- AIMD with neither the A nor the D.
            Ok(body) => {
                if let Ok(mut budgets) = site.budgets.lock()
                    && let Some(Some(g)) = budgets.get_mut(feed as usize)
                {
                    g.record_success();
                }
                return Ok(body);
            }
            Err(why) => {
                let text = why.to_string();
                // THE STATUS IS CARRIED, SO IT IS READ RATHER THAN RE-PARSED.
                //
                // `window_async` answers a `FetchError`, and `VendorRefused`
                // holds `status: u16`. Every decision below used to be taken by
                // searching this function's own rendering of that number for
                // `"status 429"` — a formatter and a policy coupled through a
                // string, with nothing testing them together, and one that
                // silently answers "not a refusal" for any status whose text
                // this file did not happen to spell out.
                //
                // AND SO IS THE VENDOR'S OWN NAME FOR IT, for the same reason
                // one level down. `VendorRefused` now also holds
                // `named: Option<Disposition>`, classified in `pull::http`
                // where the WHOLE body was in hand — `detail` is trimmed to 500
                // characters, and a Kite envelope orders its keys `status`,
                // `message`, `error_type`, so a long enough message pushes the
                // name past the cut. Reading it here would be reading a
                // rendering again.
                let (status, named) = match why {
                    pull::fetch::FetchError::VendorRefused { status, named, .. } => {
                        (Some(status), named)
                    }
                    _ => (None, None),
                };
                // A BODY THIS BUILD CANNOT READ IS NOT A BLIP.
                //
                // The exchange succeeded; the bytes are simply not what the
                // descriptor says. Asking again returns the same bytes and
                // fails the same way, so the ladder below would spend 13.75 s
                // proving that. Returned at once, like any other answered
                // refusal.
                if matches!(why, pull::fetch::FetchError::BodyNotUnderstood { .. }) {
                    return Err(text);
                }
                let invalid_auth = text.contains("Invalid_Authentication");
                if status.is_some_and(|code| (500..=599).contains(&code)) {
                    server_errors = server_errors.saturating_add(1);
                }
                match step(status, invalid_auth, named, attempt, server_errors) {
                    // THE REFUSAL THAT A LATER RUN CANNOT FIX.
                    //
                    // Reachable only from a vendor that NAMES its refusals —
                    // Kite's `PermissionException`, which arrives under the
                    // same 403 as `TokenException`. Before `pull::refusal` this
                    // was reported as the arm below, so an operator whose API
                    // key simply has no historical-data subscription was told
                    // the token would be refreshed on the next pull. It is not
                    // a token problem, there is nothing to refresh, and every
                    // later run fails identically.
                    Step::NotEntitled => {
                        return Err(format!(
                            "{text} — the credential is ALIVE and this API key is \
                             not entitled to this call. This is not an expired \
                             session and re-running later cannot fix it: the \
                             vendor named the refusal itself, and only a change \
                             to the key's subscription changes the answer. \
                             Nothing was retried, because every retry would \
                             spend a request to be told the same thing."
                        ));
                    }
                    Step::CredentialDied => {
                        // A CREDENTIAL DEATH MID-RUN IS CERTAIN, NOT A BLIP.
                        //
                        // A broker token lasts a day; the one-minute backfill is
                        // ~62,600 requests and no run of that size fits inside
                        // one. So the run WILL cross a reset, and every request
                        // after it is refused — 700 instruments lost to a
                        // credential that was refreshed minutes earlier.
                        //
                        // Refused rather than re-read HERE, because §8 is
                        // explicit: this repository never mints. The value is
                        // read fresh on the NEXT pull, and the message says so.
                        return Err(format!(
                            "{text} — the access token is no longer valid mid-run \
                             (expiry, a logout, or a login to another session of \
                             the same vendor). This repository never mints one \
                             (§8): the refreshed value is read from Parameter \
                             Store on the next pull, and resume means re-running \
                             costs only what is still missing."
                        ));
                    }
                    // It gave a reason; the reason will not change because it
                    // was asked twice more.
                    Step::Answered => return Err(text),
                    Step::ServerDown { answered } => {
                        return Err(format!(
                            "{text} — and its own side has now failed {answered} \
                             time(s) on this chunk, out of {SERVER_ERROR_ATTEMPTS} \
                             allowed. The vendor is reachable and failing, which \
                             is not a blip this run can wait out."
                        ));
                    }
                    Step::Again { wait_ms, throttled } => {
                        if throttled {
                            // THE MULTIPLICATIVE DECREASE, on every span,
                            // because a 429 names none of them. See
                            // `pull::rate::Governor`.
                            //
                            // Measured: a 785-instrument, 3-chunk pull fired
                            // ~2,355 requests in 365 s (~6.4/s) and 458
                            // instruments died on `status 429`.
                            //
                            // An earlier draft of this comment said the branch
                            // "used to be unreachable" and that
                            // `record_throttled` was dead workspace-wide. That
                            // described a build TWO steps back, not the one
                            // this replaced: the immediately preceding version
                            // already tested `throttled` first and reached this
                            // call. Corrected rather than deleted, because the
                            // measurement above is why the branch exists.
                            if let Ok(mut budgets) = site.budgets.lock()
                                && let Some(Some(g)) = budgets.get_mut(feed as usize)
                            {
                                g.record_throttled();
                            }
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
                    }
                    // The loop's own exit says it better than a branch here can.
                    Step::Exhausted => {}
                }
                last = text;
            }
        }
    }
    Err(format!(
        "{last} — and it failed {THROTTLE_ATTEMPTS} times, so the transport is not \
         blipping, it is down"
    ))
}

/// Every secret this feed's [`pull::vendor::AuthScheme`] names, read from
/// Parameter Store and handed straight to the source.
///
/// # Why the DESCRIPTOR decides, never this function
///
/// Zerodha's header is `Authorization: token api_key:access_token` — a prefix
/// and TWO secrets, the shape `docs/07-plan.md` §5 predicted no two-variant
/// scheme could describe (D-0134). This asks `auth.key_field`, and never
/// `feed == Zerodha`: a match on the feed here would be a second answer to "how
/// many secrets does this vendor take", and the two would disagree the first
/// time a vendor changed its scheme. `CLAUDE.md` §5 — adding a broker is a row
/// in `pull::vendor`, not an edit here.
///
/// # What crosses this boundary
///
/// The field NAMES come from the descriptor; the VALUES come from Parameter
/// Store and go into one header and nowhere else. Neither secret is logged,
/// formatted into an error, or returned — `CLAUDE.md` §8, and the reason
/// `pull::http::HttpSource` and `pull::http::Credential` both hand-write their
/// `Debug`. The error strings below name the FIELD that could not be read and
/// never what it holds.
///
/// # Extracted rather than inlined
///
/// `broker_window` is at the workspace's 100-line ceiling, and the same reason
/// `pull::http::HttpSource::resolve_param` gives applies: the arms here are the
/// vendor contract and they grow every time a broker is added.
///
/// # Errors
///
/// A human-readable string naming which parameter path could not be built or
/// which secret could not be read. One `now_stamp` covers both reads, so the
/// Prefixed onto a refusal that happened AFTER a socket was opened.
///
/// Stripped before the message reaches an operator — it exists so the route can
/// tell "this side refused" from "the vendor did" without matching prose. See
/// `BrokerRun::touched_wire`.
///
/// A control character rather than a word, so it can never collide with
/// anything a vendor or this build would legitimately write.
const WIRE_REACHED: &str = "\u{1}";

/// two parameters are fetched against the same signing instant.
async fn read_credential(
    identity: &pull::ssm::AwsIdentity,
    config: &pull::config::CredentialConfig,
    vendor: brutex_core::vendor::Vendor,
    auth: pull::vendor::Auth,
) -> Result<pull::http::Credential, String> {
    let stamp = pull::ssm::now_stamp().map_err(|why| why.detail)?;
    let read = async |field: &str| -> Result<String, String> {
        let path = config
            .path_for(vendor, field)
            .map_err(|why| format!("the parameter path for {field:?} could not be built: {why}"))?;
        pull::ssm::get_parameter(identity, config.region(), &path.to_string(), &stamp)
            .await
            .map_err(|why| {
                format!(
                    "this feed's credential field {field:?} could not be read: {}",
                    why.detail
                )
            })
    };

    let token = read("access-token").await?;
    match auth.key_field {
        // One secret. Every feed in this build until a two-secret vendor lands.
        None => Ok(pull::http::Credential::token(token)),
        // Two. `Credential::pair` takes the key FIRST because that is the order
        // it goes on the wire, so a transposition is visible at the call site
        // rather than as a 403 that reads like an expired session.
        Some(field) => Ok(pull::http::Credential::pair(read(field).await?, token)),
    }
}

/// The credential and the socket for one HTTP feed, in the order that costs
/// least when it fails.
///
/// # Why this is a function rather than two copies
///
/// Two callers need a credentialed source now — the spot window and expired
/// F&O discovery — and the sequence between them is not incidental: HOME, then
/// the configuration file, then the AWS identity, then the store prefix, then
/// Parameter Store, then the client. A second hand-written copy would drift on
/// the first vendor whose auth shape changed, and the drift would show up as
/// one route working and the other refusing for a reason neither names.
///
/// `spec` is taken rather than looked up so the caller keeps its own transport
/// refusal, which each places FIRST for its own reason — an archive feed that
/// reaches either one pays no clock, no HOME, no credentials file, no AWS
/// identity and no socket.
///
/// # Cost
///
/// O(1). One file read, one identity discovery, one Parameter Store read, one
/// client build — none of which grows with the store or the request.
async fn credentialed_source(
    feed: pull::vendor::Feed,
    spec: &pull::vendor::HttpSpec,
) -> Result<(pull::http::HttpSource, brutex_core::vendor::Vendor), String> {
    let Some(home) = std::env::var_os("HOME") else {
        return Err("HOME is unset, so ~/.brutex/credentials.toml cannot be located".to_owned());
    };
    let config_path = pull::config::default_config_path(std::path::Path::new(&home));
    let config = pull::config::CredentialConfig::load(&config_path).map_err(|why| {
        format!(
            "the credential configuration at {} is not usable: {why}",
            config_path.display()
        )
    })?;

    let identity =
        pull::ssm::AwsIdentity::discover().map_err(|why| format!("no AWS identity: {why}"))?;

    // WHICH BROKER, FROM THE REQUEST — not hardcoded. `CLAUDE.md`: adding a
    // vendor is a row in `pull::vendor`, and a route that names one defeats
    // that. Dhan when unstated, because it is the one whose descriptor has been
    // verified against a live body.
    // THE FEED CAME FROM THE REQUEST; NOTHING HERE MAPS A NAME TO A ROW.
    //
    // This was a seven-arm `match asked.vendor { Dhan => Feed::Dhan, Groww =>
    // Feed::Groww, other => refuse }`. It was well argued — it refused by name
    // rather than falling back — but it was still a hand-written table that a
    // fifth feed would have had to be added to, and forgetting meant a refusal
    // for a feed that existed. The parse now yields the `Feed` directly, so the
    // table is `DESCRIPTORS` and there is no second copy to fall behind it.

    // THE STORE PREFIX — a different question from the transport, asked
    // separately now that the transport has its own answer above.
    //
    // `store_vendor` is `None` for a feed with no `core::Vendor` row. For an
    // HTTP feed that is not "you picked an archive", it is "this broker has
    // nowhere to file its bars yet", and saying the first would misdirect. The
    // message names the real gap.
    let vendor = feed.store_vendor().ok_or_else(|| {
        format!(
            "{} has no store prefix. Bars are filed under bars/<vendor>/, and \
             filing one broker's prices under another's path destroys the \
             per-vendor independence D-0019 exists for — so nothing is pulled \
             until {} has a row in brutex_core::vendor::Vendor.",
            feed.display(),
            feed.display()
        )
    })?;
    let credential = read_credential(&identity, &config, vendor, spec.auth).await?;

    // The descriptor is the single source of every vendor difference — URL,
    // auth header, date format, response shape, field names, timestamp
    // encoding, price scale. `CLAUDE.md`: adding a broker is a row in
    // `crate::vendor`, not an edit here, and this is the line that keeps that
    // true — Groww and Dhan differ in six of those fields and share every line
    // of code below.
    let source = pull::http::HttpSource::new(*spec, credential).map_err(|why| why.to_string())?;

    Ok((source, vendor))
}

async fn broker_window(
    asked: &ingest::SpotRequest,
    instrument: &brutex_core::instrument::InstrumentKey,
    site: &Site,
) -> Result<BrokerWindow, String> {
    // THE TRANSPORT CHECK IS THE FIRST STATEMENT, AND IT IS THE ONLY THING
    // THAT ANSWERS "IS THIS A BROKER".
    //
    // It used to be the LAST of eleven guards, below a live `GetParameter`
    // round-trip to ap-south-1. Nothing reached it wrongly, because a
    // `store_vendor()` check six lines above happened to refuse the same set —
    // but that made this destructure unreachable, and it was standing in for a
    // question it does not answer. `store_vendor` is "which store prefix", not
    // "which transport", and the two come apart on the first HTTP feed added
    // before its `core::Vendor` row: it would be told it is a local archive,
    // which is a lie that sends the operator to the wrong place entirely.
    //
    // Placed first, an archive feed that ever reached here pays nothing: no
    // clock, no HOME, no credentials file, no AWS identity, no socket.
    let pull::vendor::Transport::Http(spec) = asked.feed.descriptor().transport else {
        return Err(format!(
            "{} declares a local-archive transport, not an HTTP one. Reaching \
             this function at all is a routing error rather than an operator \
             one — the transport is what chooses the path.",
            asked.feed.display()
        ));
    };

    served(asked.feed, asked.granularity)?;

    // ONE INSTRUMENT IS ALL THIS PATH CAN ADDRESS, AND IT NOW SAYS SO.
    //
    // `HttpSpec` carries no request-parameter map, so nothing here can name an
    // instrument to the vendor at all — which is why Dhan answers
    // `DH-905 securityId is required`. One window comes back and it was being
    // labelled `NIFTY` and filed under `NSE/INDEX/NIFTY` WHATEVER the operator
    // picked, including `NIFTY Total Market equities`, which names 750 symbols.
    // The receipt then reported success against the target they chose.
    //
    // Refusing the targets this path cannot serve is `CLAUDE.md` §4 — degrade
    // loudly and name the reason. It does not pretend to be multi-instrument;
    // it stops claiming to be. The refusal names the blocker so the message
    // stays true only until the parameter map lands, and turns into a real
    // multi-instrument pull rather than being quietly deleted.
    finished_day_only(asked)?;

    // THE RATE BUDGET, SPENT BEFORE THE SOCKET AND NOT AFTER.
    //
    // `crates/pull/src/rate.rs` implements an AIMD governor — additive increase
    // on a clean response, multiplicative decrease on a throttle — and had ZERO
    // callers. An implemented limiter nothing consults is not a limiter; the
    // backfill this repository exists to run is ~11,200 requests at day
    // granularity and ~64,800 at one-minute, and an unthrottled burst of those
    // is cut off partway, leaving a half-written history the append-only store
    // cannot correct.
    //
    // Charged HERE, ahead of the credential read and the socket, because a
    // request that will not be issued must not first cost a Parameter Store
    // round-trip.
    //
    // WAITED FOR, NOT REFUSED. This is the per-instrument path — it runs once
    // per instrument per chunk, ~97,524 times for a full Groww backfill — and
    // a refusal here is the difference between a slow pull and a dead one.
    // Measured before this changed: 785 attempted, 6 reached, 779 refused for
    // `the next request is admitted in 0.198s`, the whole run over in 5.2
    // seconds. The governor was right every time; the pull just would not
    // wait a fifth of a second for it.
    await_budget(asked.feed, site).await?;

    // THE TARGET GUARD IS GONE, AND IT HAD BEEN FALSE FOR SOME TIME.
    //
    // It read: "the broker path can address ONE instrument today and {target}
    // names a set. pull::vendor::HttpSpec has no request-parameter map, so no
    // instrument is put on the wire at all." Every clause of that was untrue by
    // the time it was removed:
    //
    //   * `HttpSpec::params` exists and BOTH broker descriptors populate it —
    //     Dhan names `securityId`/`exchangeSegment`, Groww `groww_symbol`/
    //     `segment` — and `HttpSource::resolve_param` puts them on the wire.
    //     That map is what closed `DH-905 securityId is required`.
    //   * this function takes `instrument` as an ARGUMENT, and `broker_run`
    //     above builds the target's whole instrument list, sorts it for
    //     reproducibility, and calls this once per member. The loop the guard
    //     said did not exist is the loop that called the guard.
    //
    // So it refused every target but `Swept` for a reason that had stopped
    // being a reason, and did it AFTER `await_budget` — which WAITS rather than
    // refuses. An operator picking NIFTY Total Market got 750 iterations each
    // queueing for a rate permit and then refusing: minutes of governor waiting,
    // a live in-flight status per instrument, zero sockets and zero bars. The
    // page said "no target" and the receipt said the vendor was the problem.
    //
    // What still limits a run is what always did and is enforced elsewhere:
    // `served` refuses a rung the feed does not declare, `finished_day_only`
    // refuses an unfinished day, the governor bounds the rate, and
    // `catalog::tracked && target.names(..)` in `broker_run` bounds the set.
    // None of those is a target check, and none of them is weakened here.
    // D-0136.

    let feed = asked.feed;
    // THE CREDENTIAL AND THE SOCKET, in one place shared with expired F&O
    // discovery. The sequence this replaced is unchanged; it moved.
    let (source, vendor) = credentialed_source(feed, &spec).await?;
    // THE ENDPOINT, NOT ONE REQUEST'S URL. This receipt covers every window of
    // one instrument, and a feed that carries its instrument or its rung as a
    // path segment has a different URL per window — so the honest single value
    // here is the endpoint with its placeholders left standing. It also cannot
    // fail, which a receipt should not.
    let origin = source.endpoint(asked.granularity);

    // THE ID THE VENDOR ASKED FOR, RESOLVED IN ONE PROBE.
    //
    // This was `String::new()`, so the request named a window and no
    // instrument — which is the whole of `DH-905 securityId is required`. The
    // master carries the id, `merge` indexes it by key, and this is where it
    // reaches the wire.
    //
    // Refused rather than defaulted when the vendor does not list it: sending
    // Dhan a `groww_symbol`, or any other vendor's id, asks the wrong broker
    // for the wrong thing and it would answer something.
    // THE INSTRUMENT IS AN ARGUMENT. It was `Symbol::new("NIFTY")`.
    //
    // That one literal is why spot did 1/800th of the job: the route fetched a
    // single index whatever universe the operator selected, and
    // `grep -c "for instrument in"` over this file returned zero.
    //
    // ONE HASH PROBE, not a walk. `by_key` is a `HashMap` and `ids` is indexed
    // by the vendor's own discriminant, so resolving an instrument costs the
    // same at 800 as at one — the measured 19 ns against the 1,236 ns walk this
    // replaced (D-0039).
    // THE CLASS, FROM THE SAME PROBE THAT FINDS THE ID.
    //
    // `Universe::INDEX` is set by the decoder from the master's own instrument
    // column, so this is the vendor's answer rather than a guess from the
    // symbol's shape. Everything the operator sweeps is either an NSE index or
    // an NTM cash equity — `catalog::tracked` admits exactly those two — so the
    // else-arm is Equity rather than a third refusal path that cannot be hit.
    let listing = site
        .read
        .merged
        .by_key
        .get(instrument)
        .filter(|e| e.universe.contains(brutex_core::universe::Universe::INDEX))
        .map_or(pull::vendor::Listing::Equity, |_| {
            pull::vendor::Listing::Index
        });
    let Some(instrument_id) = site
        .read
        .merged
        .by_key
        .get(instrument)
        .and_then(|e| e.ids.get(vendor as usize).copied().flatten())
    else {
        return Err(format!(
            "{} does not list {} in the instrument master this build read, so \
             there is no id to name it by. Refused rather than sending another \
             vendor's id, which would ask for the wrong instrument and be \
             answered.",
            vendor.as_str(),
            instrument.underlying
        ));
    };

    // THE WIRE STARTS HERE, AND THE REFUSAL SAYS SO.
    //
    // Everything above this line is decidable from the request: the transport,
    // the rung, whether the session has closed, the rate permit, the credential.
    // `fetch_chunks` is the only call in this function that opens a socket, so
    // a failure from it — and only from it — is one the VENDOR is responsible
    // for.
    //
    // Marked with a sentinel rather than inferred from the message. Matching a
    // refusal's PROSE is how a 403 came to be filed as a transport blip
    // elsewhere in this file; this is a marker this code writes and this code
    // strips, which is a different thing from reading a vendor's words.
    let bodies = fetch_chunks(asked, site, &source, instrument_id.as_str(), listing, &spec)
        .await
        .map_err(|why| format!("{WIRE_REACHED}{why}"))?;

    let Some(store_vendor) = feed.store_vendor() else {
        return Err(format!(
            "{} has no store prefix of its own, so there is nowhere to file its \
             bars. Nothing is written rather than another vendor's prefix being \
             borrowed.",
            vendor.as_str()
        ));
    };

    // Even `Swept` is two series and this fetches one. Saying which, rather
    // than letting the receipt imply both.
    Ok(BrokerWindow {
        // SPOT. `broker_window` serves the spot path; the expired-F&O walk
        // builds its own with the contract discovery returned.
        contract: None,
        bodies,
        exchange: instrument.exchange.as_str(),
        segment: instrument.segment.as_str(),
        window: asked.window,
        granularity: asked.granularity,
        instrument: instrument.underlying.to_string(),
        origin,
        spec,
        store_vendor,
    })
}

/// The receipt for a run that reached the store, whichever side it came from.
///
/// # One implementation, for the reason `ingest::from_members` is one
///
/// This was the `Ok` arm of the local-archive path and nothing else could reach
/// it. The broker path needs every line: the six counters, the balance check,
/// the first five failures by name, the audit record and the two verdicts. A
/// second copy would have been a second answer to "what did this run do", and
/// the two would have drifted the first time either was edited — which is
/// exactly what happened to the ingest pipeline before `from_members` existed.
///
/// `source` is what the run is filed under in the journal: a folder path on one
/// side, the broker's URL on the other.
fn landed_answer(
    done: &pull::ingest::Ingested,
    window: pull::session::Window,
    now: std::time::SystemTime,
    source: &str,
    journal: &audit::Journal,
    mut facts: Vec<(&'static str, String)>,
    took: u64,
) -> (axum::http::StatusCode, String) {
    facts.push(("Members read", done.members.to_string()));
    facts.push(("Rows read", done.rows_read.to_string()));
    facts.push(("Bars stored", done.bars_stored.to_string()));
    facts.push(("Rows folded into an open bar", done.rows_folded.to_string()));
    facts.push(("Slices the census counted", done.counted.to_string()));
    facts.push(("Rows dropped", done.census.total().to_string()));
    facts.push(("Members failed", done.failures.len().to_string()));
    facts.push(("Took", render_elapsed(took)));
    // EVERY ROW ACCOUNTED FOR, or say so. A row that vanished without landing
    // in one of the four is indistinguishable from a row the vendor never sent.
    facts.push((
        "Balances",
        if done.balances() {
            format!(
                "yes — {} read = {} stored + {} folded + {} dropped",
                done.rows_read,
                done.bars_stored,
                done.rows_folded,
                done.census.total()
            )
        } else {
            format!(
                "NO — {} rows read, {} stored, {} folded, {} dropped, {} members failed",
                done.rows_read,
                done.bars_stored,
                done.rows_folded,
                done.census.total(),
                done.failures.len()
            )
        },
    ));
    for f in done.failures.iter().take(5) {
        facts.push(("Failed", format!("{} — {}", f.instrument, f.why)));
    }
    let record = audit::Record::of_run(audit::Scope::Spot, now, took, source, window, done);
    let verdict = record.outcome.label();
    facts.extend(recorded_with_failures(
        journal,
        &record,
        audit::Scope::Spot,
        now,
        window,
        &done.failures,
    ));
    let reason = if done.balances() {
        "The run finished and every row is accounted for. Bars are on disk, \
         the manifest counts them, and this run is on the record at /audit."
    } else {
        "THE RUN FINISHED AND THE BOOKS DO NOT BALANCE. Rows in does not equal \
         bars out plus rows folded plus rows dropped, or a member failed. \
         Nothing has been hidden — the figures below are what happened."
    };
    (
        axum::http::StatusCode::OK,
        stored_html("Spot pull", verdict, reason, &facts),
    )
}

/// One local-archive run, from the folder to the receipt.
///
/// Split out of [`spot_answer`] to stay under clippy's line ceiling; the split
/// is a lint, not a design.
fn local_answer(
    asked: &ingest::SpotRequest,
    folder: &str,
    now: std::time::SystemTime,
    site: &Site,
    journal: &audit::Journal,
    mut facts: Vec<(&'static str, String)>,
) -> (axum::http::StatusCode, String) {
    // THE WHOLE REQUEST, not three of its fields. It was `feed`, `folder` and
    // `window` unpacked at the call site, and every field this path later needs
    // — the rung is the third — arrived as another positional argument beside
    // two that are already the same type.
    let window = asked.window;
    // MEASURED, NOT ESTIMATED. `Instant` and not the wall clock: the wall
    // clock can step backwards under NTP and a negative duration is not a
    // thing an operator should ever be shown.
    let started = std::time::Instant::now();
    let outcome = run_local(
        asked.feed,
        folder,
        window,
        asked.granularity,
        &site.store_root,
    );
    let took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);

    match outcome {
        Ok(done) => landed_answer(&done, window, now, folder, journal, facts, took),
        Err(why) => {
            facts.push(("Refused", why.clone()));
            facts.push(("Took", render_elapsed(took)));
            let record = audit::Record::refused(
                audit::Scope::Spot,
                audit::Outcome::Failed,
                now,
                folder,
                &why,
            )
            .with_window(window);
            facts.push(recorded_fact(journal, &record));
            (
                axum::http::StatusCode::BAD_REQUEST,
                stored_html(
                    "Spot pull",
                    "FAILED",
                    "The folder was reached and the run did not complete. Nothing \
                     partial is claimed: the reason is below and the attempt is on \
                     the record at /audit.",
                    &facts,
                ),
            )
        }
    }
}

/// A duration an operator reads, from microseconds.
///
/// The receipt's copy of [`render::elapsed`]'s job. It is here rather than
/// exported from `render` because a receipt row is a fact, not markup, and the
/// two formats being identical is a coincidence this does not depend on.
fn render_elapsed(micros: u64) -> String {
    if micros < 1_000_000 {
        format!("{}.{:03} ms", micros / 1_000, micros % 1_000)
    } else {
        format!(
            "{}.{:03} s",
            micros / 1_000_000,
            (micros % 1_000_000) / 1_000
        )
    }
}

/// Appends one record and says on the receipt whether it landed.
///
/// **The failure is not swallowed.** A run that wrote 9.8 MB of bars and could
/// not write its 256-byte record is a run nobody can find afterwards, so the
/// answer page says which of the two happened. `CLAUDE.md` §4 — a fallback that
/// hides a failure is banned; this one names it in the same table as the
/// figures it belongs to.
fn recorded_fact(journal: &audit::Journal, record: &audit::Record) -> (&'static str, String) {
    match journal.append(record) {
        Ok(()) => (
            "Recorded",
            format!("yes — appended to {}", journal.path.display()),
        ),
        Err(why) => (
            "Recorded",
            format!("NO — this run is NOT in the journal. {why}"),
        ),
    }
}

/// The run's record, then one more for every member that did not land.
///
/// # Why the failures are separate records
///
/// [`audit::Record::of_run`] keeps `failures.first()` and nothing else — the
/// note is a fixed 68 bytes because the stride is fixed, which is what makes an
/// append O(1) and the file scannable without an index — the stride is proved by
/// `api::audit::every_failed_member_gets_its_own_record_at_the_same_stride` and
/// the index-free count by
/// `api::audit::appending_creates_the_directory_and_the_count_is_the_length_divided`.
/// A run that refused ten
/// members therefore left nine reasons nowhere on disk, and which months had
/// failed had to be worked out by decoding bar files by hand.
///
/// One extra [`audit::Kind::MemberFailure`] record per failure puts every
/// failed (instrument, month) pair in the journal at the same stride. Bounded
/// by the member count, which is bounded by the chunk count — never by rows.
///
/// A failure record that cannot be written does **not** overwrite the run's own
/// "Recorded" line: the run landing and the detail landing are two facts, and
/// collapsing them would let a partial write read as a clean one. See D-0073.
fn recorded_with_failures(
    journal: &audit::Journal,
    record: &audit::Record,
    scope: audit::Scope,
    now: std::time::SystemTime,
    window: pull::session::Window,
    failures: &[pull::ingest::Failure],
) -> Vec<(&'static str, String)> {
    let mut facts = vec![recorded_fact(journal, record)];
    if failures.is_empty() {
        return facts;
    }
    let mut written = 0usize;
    let mut first_refusal = None;
    for failure in failures {
        let detail =
            audit::Record::member_failure(scope, now, &failure.instrument, window, &failure.why);
        match journal.append(&detail) {
            Ok(()) => written = written.saturating_add(1),
            Err(why) => {
                if first_refusal.is_none() {
                    first_refusal = Some(why);
                }
            }
        }
    }
    facts.push((
        "Failures recorded",
        first_refusal.map_or_else(
            || format!("{written} of {} — each on its own record", failures.len()),
            |why| {
                format!(
                    "{written} of {} — the rest are NOT on disk: {why}",
                    failures.len()
                )
            },
        ),
    ));
    facts
}

/// Ingests one local vendor folder into the store.
///
/// Split out so the handler stays a handler. Every knob comes from
/// `pull::vendor` rather than being decided here — the column layout, the
/// timestamp encoding and the price scale are descriptor fields, so adding a
/// feed is a row in that table and not an edit in this function.
///
/// # Two defects this signature and its body carry the fix for
///
/// **The segment was the string `"FUT"`.** `brutex_core::instrument::Segment`
/// parses `INDEX`, `CASH` and `FNO` and nothing else, so every member of every
/// `GDFL` folder was refused by the census key — 194 named failures and not one
/// bar written, on a page that used to write the bars and count none of them.
/// The two path segments are now spelled by [`Exchange::as_str`] and
/// [`Segment::as_str`] rather than by a literal, so the next typo does not
/// compile.
///
/// **The store root was read from the environment a second time.** It called a
/// private `default_store_dir` that consulted `HOME` directly, while the pages
/// read [`store_dir`], which honours `BRUTEX_STORE`. With that variable set,
/// bars went to one tree and `/store` reported another — a page truthfully
/// describing a store nobody had written to. The root now arrives as an
/// argument, from the one [`Site`] every page renders from.
fn run_local(
    feed: pull::vendor::Feed,
    folder: &str,
    window: pull::session::Window,
    granularity: pull::vendor::Granularity,
    store_root: &Path,
) -> Result<pull::ingest::Ingested, String> {
    use brutex_core::instrument::{Exchange, Segment};

    // THE PREFIX IS THE FEED'S OWN, and there is no default.
    //
    // This took no `Feed` and hardcoded `Vendor::Dhan`, so every archive bar
    // landed under `bars/dhan/`. The operator found 194 instrument-months of
    // GDFL futures there — `ABB-III` from `ABB-III.NFO.csv` — and was right
    // that it is a bug, not a labelling slip: the per-vendor prefix is what
    // D-0019 exists for.
    //
    // `store_vendor` is now `Some` for every feed. It is still checked rather
    // than unwrapped, so a sixth feed added without a prefix fails loudly here
    // instead of borrowing one.
    let vendor = feed.store_vendor().ok_or_else(|| {
        format!(
            "{} has no store prefix of its own, so its bars have nowhere to be \
             filed. Nothing was read and nothing was written — borrowing another \
             vendor's prefix is what put GDFL futures under bars/dhan/.",
            feed.display()
        )
    })?;
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        // NOT ON THE WIRE ON THIS PATH. An archive addresses a FILE and a
        // landing record addresses bars already in hand, so no request
        // parameter is built from this and no vendor ever sees it. Stated
        // anyway because `BarRequest` has no `Default` — a field that can be
        // omitted is a field a later caller omits by accident.
        listing: pull::vendor::Listing::Equity,
        window,
        // THE OPERATOR'S RUNG HERE TOO, and this path is where a rung the store
        // cannot carry is actually reachable: both archive feeds are
        // one-SECOND sources, and `Granularity::Second1` has no directory. The
        // refusal is `pull::ingest::Plan::timeframe`'s and it names the rung.
        //
        // Nothing about the archive's own cadence changes: the files are
        // whatever they are, and the fold coarsens them to the rung being filed
        // under. That is why a tick archive can still be filed as `1min` — it
        // always was, and this argument did not narrow it.
        granularity,
    };
    // THE COLUMN SHAPE IS STILL A LITERAL HERE, AND IT IS A KNOWN DEFECT.
    //
    // `Columns::Gdfl` is GDFL's ten-column shape, used for EVERY archive feed.
    // `TrueData`'s index rows carry five (`docs/08-vendor-samples.md`), so this
    // constant decodes one of the two vendors against the other's shape.
    //
    // The measured shape now EXISTS to read — `ColumnLayout::shape`, keyed on
    // `(feed, segment)` and cross-checked against the layout's own column list
    // by a `const` block in `pull::vendor` — and `crate::folder` reads it.
    // This site cannot simply take it, because the segment below is also a
    // literal: this function files every archive bar under `FNO`, and
    // `TrueData` declares a layout for `INDEX` only. Deriving the shape without
    // also deciding the segment turns a wrong decode into a refusal for the
    // one feed whose files this path is exercised with.
    //
    // Left as it stands, deliberately and named, rather than half-fixed. The
    // segment is a store-path decision — it decides where bars are FILED — and
    // `CLAUDE.md` §8's append-only rule means getting it wrong writes a
    // directory nothing can rename. Recorded in `docs/06-limits.md`.
    let plan = pull::ingest::Plan {
        columns: pull::csv::Columns::Gdfl,
        request: &request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor,
        exchange: Exchange::Nse.as_str(),
        segment: Segment::Fno.as_str(),
        contract: None,
    };
    pull::ingest::from_dir(std::path::Path::new(folder), store_root, plan)
        .map_err(|why| why.to_string())
}

/// Names this response as a pull receipt written by this build.
///
/// # Why a receipt has to say that it is one
///
/// `/pull/spot` answers `text/html`, and the page parses the answer by looking
/// for a `.badge` element and a `table.kv`. A `200` carrying HTML that is **not
/// a receipt** — an authenticating proxy's interstitial, a captive portal, a
/// misdirected origin, or this build's own markup after a rename — finds
/// neither, and the parser falls open: `verdict: badge?.textContent?.trim() ||
/// (ok ? 'OK' : ...)` yields the string `OK`, and `good: badge ? ... : ok`
/// yields `true`. A request that never reached this process renders a green
/// dot reading OK with a blank reason, and every instrument is then classified
/// "already held" or "no bars landed" rather than "the request never arrived".
///
/// A content type cannot separate the two, because a receipt legitimately IS
/// `text/html`. A header this handler writes can: nothing between the browser
/// and this function has any reason to invent it, and it is stamped on **every**
/// answer this handler gives — the refusals as well as the successes, so a
/// reader may require it unconditionally.
///
/// # What the page must read
///
/// `r.headers.get('x-brutex-receipt') === 'pull-spot'` before `readReceipt` is
/// called at all. Anything else is not a receipt and must render as "this
/// answer did not come from the API" rather than as a verdict. D-0124.
pub const RECEIPT_HEADER: &str = "x-brutex-receipt";

/// The value [`RECEIPT_HEADER`] carries on a spot-pull answer.
pub const SPOT_RECEIPT: &str = "pull-spot";

/// Starting a spot pull. **POST only.**
async fn pull_spot(
    axum::extract::State(site): axum::extract::State<Loaded>,
    body: String,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    axum::response::Html<String>,
) {
    // ONE STAMP, EVERY ARM. Built once here rather than at each `return`, so a
    // later arm cannot be the one that forgets it — the receipt marker is only
    // worth requiring if it is unconditional.
    let receipt = || {
        [(
            axum::http::HeaderName::from_static(RECEIPT_HEADER),
            SPOT_RECEIPT,
        )]
    };
    // ONE READING OF THE CLOCK, USED TWICE. The day the gate checks against and
    // the second stamped into the record come from the same `SystemTime`, so a
    // request that straddles midnight cannot be gated against one day and
    // recorded on another.
    let now = std::time::SystemTime::now();
    // ONE PULL AT A TIME PER FEED, AND THE REFUSAL SAYS WHAT TO DO ABOUT IT.
    //
    // `pull::ingest`'s census lock REFUSES rather than queues, and it is taken
    // and released once per chunk — so a hand-made pull landing inside an
    // autopilot tick would not wait its turn, it would see every one of the 773
    // instruments fail with a lock message and call that a run. A seat turns
    // that into a single honest refusal that names the control which resolves
    // it. `CLAUDE.md` §4: degrade loudly and name the reason.
    //
    // PER FEED, because the census it stands for is per vendor. One seat for
    // the process refused the second feed of a parallel pull at the door — it
    // read no credential and opened no socket, and the store ended the evening
    // holding one broker's month and none of the other's.
    //
    // THE FEED IS PARSED BEFORE THE SEAT IS TAKEN, and it has to be: a seat
    // cannot be per-feed if it is claimed before anyone knows which feed. This
    // is the same O(1) form lookup `parse_spot` does later, and an unreadable
    // one falls to the descriptor's default exactly as it does there, so the
    // seat and the run can never disagree about which feed this is.
    let wants = ingest::parse_feed(&param(&body, "vendor")).unwrap_or(pull::vendor::Feed::Dhan);
    let Some(_seat) = site.autopilot.take_seat(wants) else {
        return (
            axum::http::StatusCode::CONFLICT,
            receipt(),
            axum::response::Html(accepted_html(
                "Spot pull",
                vec![(
                    "Refused because",
                    format!(
                        "another pull already holds {}'s seat, so this request was \
                         refused rather than run against a manifest that pull is \
                         already writing to. Other feeds are unaffected — the seats \
                         are per feed, because the census they stand for is. If it \
                         is the autopilot, pause it at /autopilot and try again; it \
                         resumes from wherever the store reaches, so nothing is lost \
                         by pausing.",
                        wants.display()
                    )
                    .to_owned(),
                )],
                site.broker,
            )),
        );
    };
    // `dated` takes a synchronous closure, and `spot_answer` is now async
    // because the broker path awaits a credential and a window. Rather than
    // make `dated` generic over futures — which would touch every page that
    // uses it for one caller's benefit — the day is resolved first and the
    // refusal arm is spelled out here. `dated`'s job was always to turn a bad
    // clock into a page, and that is what these three lines do.
    let (code, page) = match ingest::ist_day(now) {
        Ok(today) => spot_answer(&body, today, now, &site).await,
        Err(why) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            refusal_html("Spot pull", &why),
        ),
    };
    (code, receipt(), axum::response::Html(page))
}

/// What one expired-series request is answered with, given a day to check the
/// expiry against.
///
/// Split from the handler so the expiry gate is driven by a value rather than
/// by the machine's clock — `CLAUDE.md` §3 rule 5.
async fn fno_answer(
    body: &str,
    today: Day,
    now: std::time::SystemTime,
    journal: &audit::Journal,
    // Which sentence this process is entitled to print. Taken rather than
    // assumed, for the reason `halt_for` documents.
    broker: Broker,
    // THE SITE, FOR THE CREDENTIAL AND THE FEED. Discovery is a vendor call and
    // a vendor call needs a token, which is read from Parameter Store — async,
    // and reachable only from here. This is the argument that turned the 503
    // into a walk.
    site: &Site,
) -> (axum::http::StatusCode, String) {
    match ingest::parse_fno(body, today) {
        Err(why) => {
            let record = audit::Record::refused(
                audit::Scope::Fno,
                audit::Outcome::Refused,
                now,
                &param(body, "underlying"),
                &why.to_string(),
            );
            (
                axum::http::StatusCode::BAD_REQUEST,
                refused_and_recorded("Expired F&O pull", &why, journal, &record),
            )
        }
        Ok(asked) => fno_walk(&asked, today, now, journal, broker, site).await,
    }
}

/// The context every expired-F&O answer is written against, and the one place
/// that stamps the journal.
///
/// # Why a type rather than four copies of twelve lines
///
/// Every way this route can end — archive feed, no budget, no credential, a
/// vendor that refused, a chain that walked — ends the same way: record the
/// outcome, push the receipt, render. Written out per arm it was the same block
/// four times, and the failure mode of that shape is not verbosity, it is the
/// fifth arm that renders without recording. Here an arm cannot produce a page
/// without producing the record, because there is no other way to produce one.
struct FnoPage<'a> {
    asked: &'a ingest::FnoRequest,
    now: std::time::SystemTime,
    journal: &'a audit::Journal,
    broker: Broker,
}

impl FnoPage<'_> {
    /// One outcome, recorded and rendered.
    fn say(
        &self,
        mut facts: Vec<(&'static str, String)>,
        code: axum::http::StatusCode,
        outcome: audit::Outcome,
        why: &str,
    ) -> (axum::http::StatusCode, String) {
        let record = audit::Record::refused(
            audit::Scope::Fno,
            outcome,
            self.now,
            self.asked.underlying.as_str(),
            why,
        )
        .with_window(self.asked.window);
        facts.push(recorded_fact(self.journal, &record));
        (code, accepted_html("Expired F&O pull", facts, self.broker))
    }
}

/// One credentialed connection to a feed, and the two facts filing needs
/// beside it.
///
/// Bundled because they are acquired together and always travel together: the
/// socket, the descriptor whose fields decide every vendor difference, and the
/// store prefix the bars are filed under. Passing them as three arguments put
/// the functions below over clippy's argument ceiling, and the ceiling was
/// right — three parallel parameters that must agree are one value.
struct Wire {
    source: pull::http::HttpSource,
    spec: pull::vendor::HttpSpec,
    store_vendor: brutex_core::vendor::Vendor,
}

/// Fetches and files the bars for every contract a walk discovered.
///
/// # Why this is a second pass and not part of the walk
///
/// Discovery answers *which contracts existed*; this answers *what they did*.
/// Keeping them apart is what lets the page report a month whose contracts were
/// all found and none could be fetched as exactly that, rather than as a walk
/// that failed.
///
/// # One contract's failure is not the month's
///
/// A contract that will not fetch, or will not land, is counted and its first
/// reason is kept. It does not abandon the other two hundred — the same rule
/// `Chain::unreadable` follows for a name that would not parse.
///
/// # Cost
///
/// One request per contract, each rate-governed. O(1) per contract; nothing
/// here scans the store.
async fn fno_land(
    wanted: &[pull::fno::Found],
    asked: &ingest::FnoRequest,
    site: &Site,
    wire: &Wire,
) -> (usize, usize, Vec<String>) {
    let mut stored = 0usize;
    let mut failed = 0usize;
    let mut why: Vec<String> = Vec::new();
    let origin = wire.source.endpoint(asked.granularity);

    for found in wanted {
        // THE GOVERNOR BEFORE EACH ONE, not once for the batch. A month of two
        // hundred contracts is two hundred requests, and a budget charged once
        // would be a ceiling observed once.
        if let Err(halt) = await_budget(asked.feed, site).await {
            why.push(halt);
            failed = failed.saturating_add(wanted.len().saturating_sub(stored));
            break;
        }
        let request = pull::chain::request(found, asked.window, asked.granularity);
        let body = match wire.source.window_async(&request).await {
            Ok(body) => body,
            Err(refusal) => {
                failed = failed.saturating_add(1);
                if why.len() < 5 {
                    why.push(format!("{}: {refusal}", found.vendor_symbol));
                }
                continue;
            }
        };
        // THE UNDERLYING IS THE SYMBOL AND THE CONTRACT IS THE LEVEL BELOW IT.
        // `pull::ingest` parses `instrument` into a `Symbol` and renders the
        // contract as its own path segment, so passing the vendor's contract
        // name here would file `NIFTY-30Sep25-24650-CE` as a symbol and leave
        // the underlying nowhere in the tree.
        let landed = BrokerWindow {
            bodies: vec![(asked.window, body)],
            instrument: found.underlying.clone(),
            origin: origin.clone(),
            spec: wire.spec,
            exchange: brutex_core::instrument::Exchange::Nse.as_str(),
            segment: brutex_core::instrument::Segment::Fno.as_str(),
            window: asked.window,
            granularity: asked.granularity,
            store_vendor: wire.store_vendor,
            contract: Some(found.contract),
        };
        let done = land_one(&landed, site);
        if done.bars_stored == 0 {
            failed = failed.saturating_add(1);
            if why.len() < 5 {
                why.push(format!(
                    "{}: fetched {} row(s) and stored none",
                    found.vendor_symbol, done.rows_read
                ));
            }
            continue;
        }
        stored = stored.saturating_add(done.bars_stored);
    }
    (stored, failed, why)
}

/// The discovery walk for one accepted expired-series request.
///
/// # Why this is not part of [`fno_answer`]
///
/// A parse refusal and a vendor walk fail for unrelated reasons and are read by
/// unrelated people: the first is the operator's form, the second is the
/// vendor's answer.
///
/// # Cost
///
/// One request for the month's expiries, then one per expiry. O(1) per request;
/// nothing here scans the store.
async fn fno_walk(
    asked: &ingest::FnoRequest,
    today: Day,
    now: std::time::SystemTime,
    journal: &audit::Journal,
    broker: Broker,
    site: &Site,
) -> (axum::http::StatusCode, String) {
    let page = FnoPage {
        asked,
        now,
        journal,
        broker,
    };
    let mut facts = vec![
        ("Underlying", asked.underlying.as_str().to_owned()),
        ("Series", asked.series.label().to_owned()),
        (
            "Expiry",
            asked.expiry.map_or_else(
                || {
                    format!(
                        "every expiry {} held — the whole month, checked against {today}",
                        asked.window.to()
                    )
                },
                |one| format!("{one} — expired, checked against {today}"),
            ),
        ),
    ];
    facts.extend(window_facts(asked.window));
    facts.push(("Feed", asked.feed.display().to_owned()));

    // THE TRANSPORT DECIDES THE PATH, AND IT IS ASKED FIRST.
    //
    // An archive feed's expired contracts are already on disk in the operator's
    // folder; there is no expiries endpoint to call and never was. Refusing
    // here names that rather than letting a feed with no `HttpSpec` reach a
    // credential read it has no use for.
    let pull::vendor::Transport::Http(spec) = asked.feed.descriptor().transport else {
        return page.say(
            facts,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            audit::Outcome::NotStarted,
            "this feed is a local archive; its expired contracts are read from \
             the operator's folder, not discovered over HTTP",
        );
    };

    // THE GOVERNOR ADMITS DISCOVERY TOO, and charging it here is not a
    // formality. One walk is 1 + N requests against a vendor whose ceiling is
    // five a second, so a discovery path that skipped the budget would be the
    // one path in this process able to earn a 429 that every other path then
    // pays for. Waited for rather than refused, for the reason the spot path's
    // comment gives.
    if let Err(why) = await_budget(asked.feed, site).await {
        return page.say(
            facts,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            audit::Outcome::NotStarted,
            &why,
        );
    }

    let wire = match credentialed_source(asked.feed, &spec).await {
        Ok((source, store_vendor)) => Wire {
            source,
            spec,
            store_vendor,
        },
        Err(why) => {
            return page.say(
                facts,
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                audit::Outcome::NotStarted,
                &why,
            );
        }
    };

    // THE MONTH IS THE EXPIRY'S OWN MONTH. Discovery is keyed on (underlying,
    // year, month) because that is what both vendors publish an expiry list
    // for. The operator asked for ONE expiry and ONE series, and `wanted` in
    // `fno_report` is what narrows the month's answer back to it.
    // THE MONTH COMES FROM THE EXPIRY WHEN ONE WAS NAMED, AND FROM THE WINDOW
    // WHEN ONE WAS NOT. Both answer the same question — which month's expiry
    // list to ask the vendor for — and the window's end is the right end of it:
    // `parse_fno` has already proved that day is behind today, so the month it
    // names cannot hold a live contract this build would then try to store.
    let month_of = asked.expiry.unwrap_or_else(|| asked.window.to());
    let ask = pull::fno::Ask {
        underlying: asked.underlying.as_str().to_owned(),
        year: month_of.year(),
        month: month_of.month(),
        expiry: String::new(),
    };

    // WHICH SHAPE THIS VENDOR ANSWERS IN, and the two are not variations of
    // one request.
    //
    // Groww publishes NAMES: ask a month's expiries, then each expiry's
    // contracts, then fetch by the name it gave back. Dhan publishes none — its
    // endpoint takes a cadence, an ordinal, a strike OFFSET and a side, so
    // there is nothing to discover and the walk has no first step.
    //
    // Sending the name-walk at Dhan is what produced the 502s of 2026-08-19:
    // `chain::month` asks the descriptor for `by_name()`, Dhan's answers None,
    // and the walk refused before a bar could be fetched. The refusal was
    // right and the request was the wrong shape.
    if let Some(rolling) = spec.fno.by_offset() {
        return fno_roll(&page, facts, asked, site, &wire, rolling).await;
    }
    if spec.fno.by_name().is_none() {
        return page.say(
            facts,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            audit::Outcome::NotStarted,
            "this feed declares no expired-derivative history at all, so there \
             is nothing to ask it for",
        );
    }

    // EXPIRIES FIRST, THEN CONTRACTS — and the ordering is enforced by `?`
    // inside `chain::month` rather than by this call site's good manners. A
    // refused expiries call returns before one contracts URL is built, which is
    // what makes a half-walked month impossible rather than merely unlikely.
    match pull::chain::month(asked.feed, &ask, &wire.source).await {
        Err(why) => {
            // THE WHOLE REASON, WHERE THE STRIDE CANNOT CUT IT.
            //
            // The journal's note is 68 bytes and a discovery failure's reason
            // starts with the URL, so what an operator actually needed — the
            // vendor's status, or the field that would not read — was exactly
            // the part that fell off the end. Measured: three 502s whose notes
            // were all `https://api.groww.in/v1/historical/expiries?exchange=…`
            // and nothing else, and no event anywhere carried the rest. The
            // run was legible as "it failed" and illegible as "why".
            //
            // `note_bytes` said the note was cut, which is honest and useless:
            // knowing a sentence was truncated does not tell you the sentence.
            // The journal keeps the stride it needs; this carries the reason.
            // BOUND FIRST: `Value::Str` borrows, so a temporary built inside
            // the call would not outlive it.
            let said = why.to_string();
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::error("pull.fno", "discovery refused")
                    .with("feed", telemetry::Value::Str(asked.feed.wire()))
                    .with(
                        "underlying",
                        telemetry::Value::Str(asked.underlying.as_str()),
                    )
                    .with("year", telemetry::Value::Uint(u64::from(month_of.year())))
                    .with("month", telemetry::Value::Uint(u64::from(month_of.month())))
                    .with("why", telemetry::Value::Str(&said)),
            );
            page.say(
                facts,
                axum::http::StatusCode::BAD_GATEWAY,
                audit::Outcome::Failed,
                &why.to_string(),
            )
        }
        Ok(chain) => fno_report(&page, facts, &chain, asked, site, &wire).await,
    }
}

/// One contract of the product: build, post, read, name it, file it.
///
/// # Errors
///
/// A sentence naming what could not be done, for the caller to count and quote.
/// Every arm names the contract it was working on, because "a request failed"
/// with 252 in flight tells an operator nothing.
#[allow(clippy::too_many_arguments)]
async fn roll_one(
    asked: &ingest::FnoRequest,
    site: &Site,
    wire: &Wire,
    security_id: &str,
    word: &'static str,
    flag: &'static str,
    code: &'static str,
    strike: &'static str,
    option_type: &'static str,
    endpoint: &str,
    rolling: pull::vendor::RollingSpec,
) -> Result<usize, String> {
    let label = format!("{word} {flag}/{code} {strike} {option_type}");
    // THE EXPIRY THE ANSWER WILL NOT CARRY, established BEFORE the request.
    //
    // Asked first on purpose: if this underlying has no regime for that cadence
    // the contract cannot be filed whatever comes back, and finding that out
    // after the round-trip spends a request to learn something the calendar
    // already knew.
    let expiry = pull::rolling::expiry_of(asked.underlying.as_str(), flag, code, asked.window.to())
        .map_err(|why| format!("{label}: {why}"))?;

    let ask = pull::rolling::Ask {
        security_id: security_id.to_owned(),
        instrument: word,
        expiry_flag: flag,
        expiry_code: code,
        strike,
        side: option_type,
        interval: asked.granularity.dir(),
        from: asked.window.from().to_string(),
        // EXCLUSIVE ON THE WIRE, which the vendor documents and
        // `fetch::wire_end` owns. Passing the operator's last day loses that
        // session silently — the answer parses, the books balance, one day is
        // absent.
        to: pull::fetch::wire_end(asked.window.to(), wire.spec.range_end)
            .map_err(|why| format!("{label}: {why}"))?
            .to_string(),
    };

    let body = pull::rolling::body(&rolling, &ask);
    let answer = wire
        .source
        .post_json(endpoint, body)
        .await
        .map_err(|why| format!("{label}: {why}"))?;
    let rows = pull::rolling::read(&answer, option_type, wire.spec.prices)
        .map_err(|why| format!("{label}: {why}"))?;
    if rows.is_empty() {
        // NOT A FAILURE. A strike the vendor never listed for this expiry is an
        // ordinary empty answer, and counting it as a fault would report ~200
        // failures on a healthy month.
        return Ok(0);
    }

    let contract = brutex_core::instrument::Contract::of(brutex_core::instrument::Kind::Option {
        expiry,
        // THE STRIKE THE VENDOR RESOLVED, not the offset that was asked for.
        // `ATM+10` is a question; the answer carries the price it meant, and
        // filing under the question would put every month's ATM+10 in one file.
        strike: brutex_core::price::Paisa::from_raw(rows.first().map_or(0, |r| r.overlay.spot)),
        side: if option_type == "CALL" {
            brutex_core::instrument::OptionSide::Call
        } else {
            brutex_core::instrument::OptionSide::Put
        },
    })
    .ok_or_else(|| format!("{label}: this store cannot name that contract"))?;

    let bars: Vec<store::format::Bar> = rows.iter().map(|r| r.bar).collect();
    // ONLY THE OVERLAYS THAT STATE SOMETHING. A contract whose vendor sent
    // neither a spot nor a volatility has nothing to overlay, and a file of
    // null rows costs a block per 170 bars to answer what an absent file
    // answers better.
    let overlays: Vec<store::format::Overlay> = rows
        .iter()
        .map(|r| r.overlay)
        .filter(store::format::Overlay::states_something)
        .collect();

    let request = pull::fetch::BarRequest {
        instrument_id: security_id.to_owned(),
        listing: pull::vendor::Listing::Derivative,
        window: asked.window,
        granularity: asked.granularity,
    };
    let plan = pull::ingest::Plan {
        columns: pull::csv::Columns::Gdfl,
        request: &request,
        encoding: wire.spec.timestamps,
        // ALREADY PAISA. `rolling::read` converted at the boundary through
        // `csv::paisa`, so saying Rupees here would multiply by a hundred a
        // second time — the trap `DECODED_PRICE_SCALE` exists to name.
        scale: pull::vendor::PriceScale::Paisa,
        vendor: wire.store_vendor,
        exchange: brutex_core::instrument::Exchange::Nse.as_str(),
        segment: brutex_core::instrument::Segment::Fno.as_str(),
        contract: Some(contract),
    };
    let done = pull::ingest::from_rows(
        &bars,
        &overlays,
        asked.underlying.as_str(),
        endpoint,
        &site.store_root,
        plan,
    );
    if let Some(first) = done.failures.first() {
        return Err(format!("{label}: {}", first.why));
    }
    Ok(done.bars_stored)
}

/// Every request in the cross product, fetched and filed.
///
/// # Why one failure does not stop the rest
///
/// A strike the vendor never listed, a cadence an underlying does not carry, a
/// window before a regime's verified floor — each is a legitimate empty or
/// refused answer for ONE contract, and abandoning the other 251 over it would
/// throw away the month to report the one. Counted and named instead, the same
/// rule `Chain::unreadable` follows for a name that would not parse.
///
/// # Cost
///
/// One request per member of the product, each rate-governed. O(1) per
/// request; nothing here scans the store.
async fn roll_every(
    asked: &ingest::FnoRequest,
    site: &Site,
    wire: &Wire,
    rolling: pull::vendor::RollingSpec,
    security_id: &str,
    word: &'static str,
    offsets: &'static [&'static str],
) -> (usize, usize, Vec<String>) {
    let endpoint = pull::rolling::url(&rolling, wire.spec.base_url);
    let mut stored = 0usize;
    let mut failed = 0usize;
    let mut why: Vec<String> = Vec::new();

    for flag in rolling.expiry_flags {
        for code in rolling.expiry_codes {
            for strike in offsets {
                for side in rolling.sides {
                    // THE GOVERNOR BEFORE EACH ONE. A month is 252 requests
                    // against a ceiling of five a second; a budget charged once
                    // for the batch is a ceiling observed once.
                    if let Err(halt) = await_budget(asked.feed, site).await {
                        why.push(halt);
                        return (stored, failed.saturating_add(1), why);
                    }
                    let one = roll_one(
                        asked,
                        site,
                        wire,
                        security_id,
                        word,
                        flag,
                        code,
                        strike,
                        side,
                        &endpoint,
                        rolling,
                    )
                    .await;
                    match one {
                        Ok(count) => stored = stored.saturating_add(count),
                        Err(said) => {
                            failed = failed.saturating_add(1);
                            if why.len() < 5 {
                                why.push(said);
                            }
                        }
                    }
                }
            }
        }
    }
    (stored, failed, why)
}

/// The Dhan shape: enumerate the contract set, fetch each, file bar and overlay.
///
/// # Why this enumerates instead of discovering
///
/// There is nothing to discover. The vendor's endpoint takes a cadence
/// (WEEK|MONTH), an ordinal (near|next|far), a strike OFFSET (ATM±n) and a side
/// (CALL|PUT), and every combination of those is a request that can be built
/// without asking it anything first. The contract set is the descriptor's own
/// cross product.
///
/// That is also why its cost can be STATED rather than found out while running:
/// 21 index offsets × 2 sides × 2 cadences × 3 ordinals is 252 requests for an
/// index month and 84 for a stock month, and no answer from any vendor changes
/// either number.
///
/// # Cost
///
/// O(1) per request built and per row read. The request COUNT is fixed by the
/// descriptor, never by an answer.
async fn fno_roll(
    page: &FnoPage<'_>,
    mut facts: Vec<(&'static str, String)>,
    asked: &ingest::FnoRequest,
    site: &Site,
    wire: &Wire,
    rolling: pull::vendor::RollingSpec,
) -> (axum::http::StatusCode, String) {
    // THE UNDERLYING'S OWN ID, never a contract's. There is no contract id to
    // have — that is the whole reason this path exists — so what goes on the
    // wire is the spot instrument's, looked up exactly as the spot path looks
    // it up.
    let key = brutex_core::instrument::InstrumentKey {
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: brutex_core::instrument::Segment::Index,
        underlying: asked.underlying,
        kind: brutex_core::instrument::Kind::Index,
    };
    let Some(security_id) = site
        .read
        .merged
        .by_key
        .get(&key)
        .and_then(|e| e.ids.get(wire.store_vendor as usize).copied().flatten())
    else {
        return page.say(
            facts,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            audit::Outcome::NotStarted,
            "this vendor's instrument master lists no id for the underlying, so \
             there is nothing to name it by. Refused rather than sending \
             another vendor's id, which would ask for a different instrument \
             and be answered",
        );
    };

    // AN INDEX OPTION AND A STOCK OPTION ARE DIFFERENT WIDTHS. The vendor
    // serves ATM±10 on an index and ATM±3 on a stock, and an ask outside a
    // type's own width is a request it answers with nothing — 28 empty calls
    // per expiry per side on every stock if the wider list were used for both.
    let word = rolling.index_word;
    let offsets = rolling.offsets_for(word);
    let planned = offsets.len()
        * rolling.sides.len()
        * rolling.expiry_flags.len()
        * rolling.expiry_codes.len();
    facts.push(("Requests planned", planned.to_string()));
    facts.push((
        "Addressed by",
        format!("strike offset — {} offsets, ATM-relative", offsets.len()),
    ));

    let (stored, failed, why) = roll_every(
        asked,
        site,
        wire,
        rolling,
        security_id.as_str(),
        word,
        offsets,
    )
    .await;
    facts.push(("Bars stored", stored.to_string()));

    if failed == 0 {
        return page.say(
            facts,
            axum::http::StatusCode::OK,
            if stored == 0 {
                audit::Outcome::Empty
            } else {
                audit::Outcome::Stored
            },
            "every planned contract answered and was filed under its own \
             expiry and strike",
        );
    }
    facts.push((
        "Requests that did not land",
        format!("{failed} of {planned}"),
    ));
    if !why.is_empty() {
        facts.push(("First reasons", why.join(" · ")));
    }
    page.say(
        facts,
        axum::http::StatusCode::BAD_GATEWAY,
        audit::Outcome::Failed,
        "some contracts did not land; the month is incomplete and must not be \
         read as held",
    )
}

/// Turns one walked chain into bars on disk and a page that says what happened.
///
/// Split from [`fno_walk`] because a walk and a fetch fail for unrelated
/// reasons, and keeping both in one function put it over the workspace's line
/// ceiling.
async fn fno_report(
    page: &FnoPage<'_>,
    mut facts: Vec<(&'static str, String)>,
    chain: &pull::chain::Chain,
    asked: &ingest::FnoRequest,
    site: &Site,
    wire: &Wire,
) -> (axum::http::StatusCode, String) {
    facts.push(("Expiries in month", chain.expiries.len().to_string()));
    facts.push(("Contracts discovered", chain.contracts.len().to_string()));
    // REPORTED, NEVER SILENT. `Chain` keeps the names it could not read
    // precisely so this line can exist; a count of contracts alone
    // would render a partly-read month as a whole one, which is the §4
    // fallback that hides a failure.
    if !chain.unreadable.is_empty() {
        facts.push((
            "Unreadable names",
            format!(
                "{} — {}",
                chain.unreadable.len(),
                chain.unreadable.join(", ")
            ),
        ));
    }
    // NARROWED TO WHAT WAS ASKED FOR, WHICH IS NOT WHAT WAS DISCOVERED.
    //
    // A month publishes several expiries and both series. The request names ONE
    // of each. Without this the fetch below walked every contract of every
    // expiry in the month: for a weekly-expiry index that is five times the
    // requests, five times the rate budget, and bars filed for series nobody
    // asked for -- under a page whose own "Expiry" line named a single date.
    //
    // The comment where `ask` is built claimed this filter existed before the
    // filter did. That is the shape this codebase treats as worse than a
    // missing guard: a protection a reader can see asserted and cannot see run.
    // NARROWED BY `ingest::matching`, WHICH ALREADY EXISTED FOR THIS.
    //
    // A filter was written inline here first, which was a second answer to a
    // question one function already answered — and the two would have drifted
    // on the first vendor whose contract naming changed. This is the one that
    // ships; the inline copy is gone.
    let wanted = ingest::matching(chain, asked);
    facts.push(("Contracts asked for", wanted.len().to_string()));

    if wanted.is_empty() {
        return page.say(
            facts,
            axum::http::StatusCode::OK,
            audit::Outcome::Empty,
            "the walk succeeded and the month published no contract for the \
             expiry and series asked for, which is not the same as a month \
             that was not walked",
        );
    }

    // AND NOW THE BARS. Discovery said which contracts existed; this
    // fetches what they did and files it.
    let (stored, failed, why) = fno_land(&wanted, asked, site, wire).await;
    facts.push(("Bars stored", stored.to_string()));

    if failed == 0 {
        return page.say(
            facts,
            axum::http::StatusCode::OK,
            audit::Outcome::Stored,
            "every discovered contract fetched and filed",
        );
    }

    // A PARTIAL MONTH IS REPORTED AS ONE. It is neither a success to
    // carry on from nor a failure to retry whole, and rendering it as
    // either is the §4 fallback that hides what happened. The counts
    // and the first reasons go on the page, and the outcome says
    // FAILED so the ladder does not read this month as held.
    facts.push((
        "Contracts that did not land",
        format!("{failed} of {}", chain.contracts.len()),
    ));
    if !why.is_empty() {
        facts.push(("First reasons", why.join(" · ")));
    }
    page.say(
        facts,
        axum::http::StatusCode::BAD_GATEWAY,
        audit::Outcome::Failed,
        "some contracts did not land; the month is incomplete and must \
         not be read as held",
    )
}

/// Starting an expired-series pull. **POST only.**
async fn pull_fno(
    axum::extract::State(site): axum::extract::State<Loaded>,
    body: String,
) -> (axum::http::StatusCode, axum::response::Html<String>) {
    let now = std::time::SystemTime::now();
    let journal = site.journal();
    // THE SAME SHAPE `pull_spot` USES, and for the same reason its comment
    // gives: `dated` takes a SYNCHRONOUS closure and this answer is now async,
    // because discovery awaits a credential. Making `dated` generic over
    // futures would touch every page that uses it for one caller's benefit, so
    // the day is resolved first and the refusal arm is spelled out here.
    let (code, page) = match ingest::ist_day(now) {
        Ok(today) => fno_answer(&body, today, now, &journal, site.broker, &site).await,
        Err(why) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            refusal_html("Expired F&O pull", &why),
        ),
    };
    (code, axum::response::Html(page))
}

/// The audit page.
async fn audit_get(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::response::Html<String>) {
    let (code, body) = dated(ingest::today_ist(), "Audit", |today| {
        (
            axum::http::StatusCode::OK,
            audit_html(&site, today, page_number(uri.query().unwrap_or(""))),
        )
    });
    (code, axum::response::Html(body))
}

/// The audit page, from a site already loaded.
///
/// Bounded twice over: the record count is one `metadata` call, so the pager is
/// arithmetic, and only this page's records are read off disk. Neither cost
/// grows with the journal —
/// `api::audit::the_tail_reads_only_the_records_it_shows` is what holds the
/// read to its page.
#[must_use]
pub fn audit_html(site: &Site, today: Day, page: usize) -> String {
    let journal = site.journal();
    let log = journal.look();
    let trouble = journal_trouble(&log);
    let path = journal.path.display().to_string();
    let total = log.records();
    let per_page = u64::try_from(PAGE_ROWS).unwrap_or(1).max(1);
    let last_page = usize::try_from(total.saturating_sub(1) / per_page).unwrap_or(0);
    let page = page.min(last_page);
    let skip = u64::try_from(page).unwrap_or(0).saturating_mul(per_page);
    let mut notes = vec![format!(
        "audit journal: {path} — {total} record(s), one 256-byte record per pull, \
         appended and fsync-ed before the answer is rendered"
    )];
    if !trouble.is_empty() {
        notes.push(format!("UNAVAILABLE — {trouble}"));
    }
    let rows = audit_rows(&journal, total, skip, per_page, &mut notes);
    // Prepared here rather than inside the renderer: `Notes` is where a line's
    // text and its loudness are decided, and deciding them once is D-0130.
    let notes = render::Notes::build(&notes);
    render::audit_page(&render::AuditView {
        today,
        journal: journal_note(&path, &log, &trouble),
        rows: &rows,
        page,
        last_page,
        notes: &notes,
    })
}

/// One page of the journal, as rows, with any refusal appended to `notes`.
///
/// Split from [`audit_html`] so the refusal arm is drivable. Left inline, it
/// was reachable only from a journal that changed size between the `metadata`
/// call and the read — a race a test cannot stage — and an untestable arm on a
/// render path is an arm nobody has checked. Taking the journal and the count
/// as arguments makes both outcomes a property of the file, which a test owns.
///
/// A read that fails empties the table and says why. It does **not** fall back
/// to a shorter read or to zero records: `CLAUDE.md` §4 — degrade loudly and
/// name the reason.
fn audit_rows(
    journal: &audit::Journal,
    total: u64,
    skip: u64,
    take: u64,
    notes: &mut Vec<String>,
) -> Vec<render::AuditRow> {
    match journal.page(total, skip, take) {
        Ok(entries) => entries.into_iter().map(audit_row).collect(),
        Err(why) => {
            notes.push(format!("UNREADABLE — {why}"));
            Vec::new()
        }
    }
}

/// One journal entry, as the page shows it.
///
/// A record that will not decode becomes a row saying so rather than a gap: one
/// damaged record must not blank the rows around it, which is exactly what a
/// `filter_map` here would have done.
fn audit_row(entry: audit::Entry) -> render::AuditRow {
    match entry.decoded {
        Err(fault) => render::AuditRow {
            ordinal: entry.ordinal,
            fault: Some(fault.to_string()),
            when: String::new(),
            scope: String::new(),
            outcome: String::new(),
            loud: true,
            source: String::new(),
            window: String::new(),
            members: 0,
            rows_read: 0,
            bars_stored: 0,
            rows_folded: 0,
            counted: 0,
            drops: audit::Drops::default(),
            failures: 0,
            took_micros: 0,
            note: String::new(),
        },
        Ok(record) => render::AuditRow {
            ordinal: entry.ordinal,
            fault: None,
            when: ist_stamp(record.at_unix_secs),
            // THE KIND IS ON THE PAGE, because a member record's counters are
            // all zero by design — they belong to the run — and a row of zeros
            // with no label reads as a run that did nothing rather than as one
            // member that did not land. A `Run` row is unchanged, byte for
            // byte, which is what keeps every existing assertion true.
            scope: match record.kind {
                audit::Kind::Run => record.scope.label().to_owned(),
                audit::Kind::MemberFailure => {
                    format!("{} · {}", record.scope.label(), record.kind.label())
                }
            },
            outcome: record.outcome.label().to_owned(),
            loud: record.outcome.is_loud(),
            source: if record.source_was_cut() {
                format!(
                    "{}… (cut from {} bytes)",
                    record.source, record.source_bytes
                )
            } else {
                record.source.clone()
            },
            window: window_text(record.from_days, record.to_days),
            members: record.members,
            rows_read: record.rows_read,
            bars_stored: record.bars_stored,
            rows_folded: record.rows_folded,
            counted: record.counted,
            drops: record.drops,
            failures: record.failures,
            took_micros: record.elapsed_micros,
            note: if record.note_was_cut() {
                format!("{}… (cut from {} bytes)", record.note, record.note_bytes)
            } else {
                record.note.clone()
            },
        },
    }
}

/// The store page.
async fn store_get(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::response::Html<String>) {
    // THE STATUS COMES OUT OF THE PAGE NOW, the way `/bars` has always taken
    // it out of `bars_html`: this route can refuse, and a refusal answered
    // under `200` is a refusal a monitor cannot see.
    let query = uri.query().unwrap_or("");
    let (code, body) = dated(ingest::today_ist(), "Store", |today| {
        store_html(&site, today, page_number(query), query)
    });
    (code, axum::response::Html(body))
}

/// The page `/store` answers when `feed=` names a vendor this build cannot read.
///
/// [`render::receipt_page`] rather than the store page, and not for want of
/// trying: [`render::StoreView`] has to name a `Vendor` to draw its column and
/// its counter cards, and the whole content of this arm is that there is no
/// vendor to name. Rendering it under Dhan to keep the shape is the defect
/// being removed, one layer down.
///
/// The receipt is the same shape [`refusal_html`] answers a bad window with, so
/// this server has one refusal page and not two. The scope is a literal because
/// there is one caller: a parameter here would be generality nothing asked for.
fn store_feed_refusal(asked: &str) -> (axum::http::StatusCode, String) {
    // THROUGH `note_alphabet` BEFORE IT IS RENDERED. The value came off a query
    // string, and while `render::receipt_page` escapes it for HTML, the
    // 400-character ceiling is what stops a page-length "feed" from being drawn
    // into a table cell.
    let facts = [("Feed asked for", note_alphabet(asked))];
    (
        axum::http::StatusCode::BAD_REQUEST,
        render::receipt_page(&render::Receipt {
            scope: "Store",
            verdict: "REFUSED",
            reason: &no_such_feed(),
            good: false,
            facts: &facts,
            footnote: "Nothing was counted and nothing was written. The feed \
                       picker on /store lists every name this build reads.",
        }),
    )
}

/// The store page, from a site already loaded, and the status it answers under.
///
/// An absent manifest renders; it never fails. Before the first ingest there is
/// no file, and a page that 500s on a fresh install is a page that is broken
/// exactly when an operator most needs to look at it.
///
/// # Why it returns a status now
///
/// The same shape [`bars_html`] has: one arm of this page is a REFUSAL — a
/// `feed=` this build cannot read — and a refusal is not a `200`. The status
/// used to be a literal `200` written in [`store_get`], which is a caller
/// asserting something about a body it has not looked at.
#[must_use]
pub fn store_html(
    site: &Site,
    today: Day,
    page: usize,
    query: &str,
) -> (axum::http::StatusCode, String) {
    // THE FEED IS RESOLVED FIRST, BEFORE ANYTHING IS COUNTED. An unknown name
    // used to fall to Dhan here, so `/store?feed=growww` drew Dhan's counters
    // under a page an operator had asked to be about Groww — the same wrong
    // answer in the shape of a right one that `bars_html` refuses for
    // `?vendor=`, and that `CLAUDE.md` §4 bans. An ABSENT feed is untouched:
    // `ingest::parse_vendor` answers the empty string with Dhan itself and says
    // so, so `/store` with no query is the page it always was.
    let asked = param(query, "feed");
    let Some(feed) = ingest::parse_vendor(&asked) else {
        return store_feed_refusal(&asked);
    };
    // ── THE PAGE OPENS ON WHAT IS HELD, NOT ON THE PRODUCT ───────────────
    //
    // It used to render `series × 36 months` unconditionally: 7,056 cells on
    // this operator's disk, of which 194 are held. So `/store` opened on 36
    // consecutive blank rows of one index and the real entries began on page
    // two — a store holding 62,978 bars that LOOKED empty, which is the same
    // sentence D-0048 was written to stop being true.
    //
    // `show=gaps` still renders the product, because "what am I missing" is a
    // real question. It is just not the first one.
    let filter = store_filter(query);
    let held_only = param(query, "show") != "gaps";

    let (rows, total, last_page) = if held_only {
        let kept = census::filtered(&site.entries, &filter);
        let total = kept.len();
        let last = total.saturating_sub(1) / PAGE_ROWS;
        let page = page.min(last);
        (
            census::held_page(
                &kept,
                &site.censuses,
                page.saturating_mul(PAGE_ROWS),
                PAGE_ROWS,
            ),
            total,
            last,
        )
    } else {
        // The product, as before: bounded by construction, so the last page is
        // a division rather than a walk and `?page=999` clamps.
        let total = census::grid_rows(site.series.len());
        let last = total.saturating_sub(1) / PAGE_ROWS;
        let page = page.min(last);
        (
            census::coverage_page(
                &site.series,
                &site.censuses,
                today,
                page.saturating_mul(PAGE_ROWS),
                PAGE_ROWS,
            ),
            total,
            last,
        )
    };
    let page = page.min(last_page);
    let mut notes = vec![format!(
        "store root: {} — every figure here is a field read of one manifest \
         header, and every grid cell is one hash probe",
        site.store_root.display()
    )];
    // A COUNTER ON THIS PAGE IS AS OLD AS THIS PROCESS, and until now nothing
    // said so. The censuses are read once into an `Arc<Site>` (D-0039), so a
    // pull run by this very server lands on disk and in the journal while these
    // cards still say what they said at startup. The audit journal is what
    // makes the staleness detectable in constant time: one 256-byte read of the
    // newest record, compared against the second the site was loaded.
    let journal = site.journal();
    let log = journal.look();
    if let Some(record) = newest_record(&journal, &log)
        && record.at_unix_secs >= site.loaded_at
    {
        // TWO NOTES, EACH UNDER `render::clamp`'s 160-byte ceiling. One long
        // note would be cut mid-sentence by the renderer, and a truncated
        // warning is a warning nobody finishes reading.
        notes.push(format!(
            "UNCHECKED — a pull ran at {}; these manifests were read at {}. \
             Restart to refresh, or see /audit.",
            ist_stamp(record.at_unix_secs),
            ist_stamp(site.loaded_at)
        ));
        notes.push(
            "The counters below are this process's startup read; re-reading them \
             per request is the cost D-0039 removed."
                .to_owned(),
        );
    }
    let trouble = journal_trouble(&log);
    if !trouble.is_empty() {
        notes.push(format!("UNAVAILABLE — audit journal {trouble}"));
    }
    notes.extend(site.censuses.iter().map(census::VendorCensus::note));
    let mut notes = render::Notes::build(&notes);
    // The master notes ride along, because an `UNAVAILABLE` master is why the
    // grid may be down to the two swept series — but ALREADY PREPARED. Cloning
    // them as raw strings copied a note whose length grows with the instrument
    // set, once per request; the prepared line is what the page draws and is
    // bounded. D-0130.
    notes.extend_from(&site.read.notes_view);
    (
        axum::http::StatusCode::OK,
        render::store_page(&render::StoreView {
            // ONE FEED PER VIEW, resolved at the top of this function through
            // the same parser the pull form uses — so "groww" means the same
            // thing on both pages, and a name this build does not read never
            // reaches here at all.
            feed,
            today,
            censuses: &site.censuses,
            rows: &rows,
            page,
            last_page,
            total,
            notes: &notes,
            filter: Some(&filter),
            held: site.entries.len(),
            held_only,
        }),
    )
}

/// The four narrowings, out of a query string.
///
/// Every one is optional and a missing or malformed value is simply `None` —
/// there is no refusal here on purpose. A filter is a *view*, not a request to
/// change anything, and answering `?from=banana` with an error page would be
/// louder than the mistake deserves. What it must never do is silently apply a
/// DIFFERENT filter than the one shown, which is why the controls render from
/// this same struct: what the bar shows is what was parsed.
fn store_filter(query: &str) -> census::StoreFilter {
    let month = |raw: &str| -> Option<store::path::YearMonth> {
        let (y, m) = raw.split_once('-')?;
        store::path::YearMonth::new(y.parse().ok()?, m.parse().ok()?).ok()
    };
    let symbol = param(query, "symbol");
    census::StoreFilter {
        segment: brutex_core::instrument::Segment::parse(&param(query, "kind")).ok(),
        // UPPER-CASED ONCE, HERE, not once per row.
        //
        // `StoreFilter::keeps` folded the needle on every entry it tested —
        // loop-invariant work in the innermost loop of a per-request path. The
        // stored side needs no folding at all: `Symbol::new` admits only ASCII
        // and upper-cases at construction, so the comparison is now allocation
        // free on both sides.
        symbol: (!symbol.is_empty()).then(|| symbol.to_uppercase()),
        // Not offered in the bar yet: the store holds one timeframe today, and
        // a control with one option is a control that lies about having a
        // choice. The field is parsed so a URL can still carry it.
        timeframe: None,
        from: month(&param(query, "from")),
        to: month(&param(query, "to")),
    }
}

/// The prices themselves — one month of one instrument, read by index.
///
/// **The page every other page was describing.** `/store` counted rows and drew
/// swatches; nothing in this server ever showed a price. This opens the bar file
/// the ingest path wrote and renders what is in it.
async fn bars_get(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (axum::http::StatusCode, axum::response::Html<String>) {
    let query = uri.query().unwrap_or("").to_owned();
    let (code, body) = dated(ingest::today_ist(), "Bars", |_today| {
        bars_html(&site, &query)
    });
    (code, axum::response::Html(body))
}

/// The `/bars` page with no rows on it, and the reason why.
///
/// Both of this route's refusals — a month that cannot be parsed and a vendor
/// this build has no feed for — render the same shape, so they share it. A
/// second copy is a second place for the two to drift into describing the
/// store differently.
fn bars_refusal(
    site: &Site,
    symbol: &str,
    segment: &str,
    vendor: &str,
    trouble: &'static str,
) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::BAD_REQUEST,
        render::bars_page(&render::BarsView {
            symbol,
            segment,
            vendor,
            month: "—",
            rows: &[],
            total: 0,
            page: 0,
            last_page: 0,
            // NO RUNG WAS PARSED on this path — the request failed before it.
            // The minute grid is what an absent `?timeframe=` means everywhere
            // else, so the links this page renders stay consistent with it.
            timeframe: store::path::Timeframe::MINUTE_1.as_str(),
            trouble: Some(trouble),
            store_root: &site.store_root.display().to_string(),
        }),
    )
}

/// One month of bars, or a named refusal.
///
/// Every failure arm here names what it looked for: an unreadable month must
/// say *which path*, because "no data" and "the wrong path" send an operator to
/// opposite places — the same distinction `census::Census` draws between absent
/// A query parameter, or `fallback` when it is absent or empty.
///
/// The two spellings this route needs — an absent `?segment=` means INDEX and
/// an absent `?exchange=` means NSE, because those are the only values the
/// engine surface has (`CLAUDE.md` §1). Written once so the two cannot drift.
fn param_or(query: &str, name: &str, fallback: &str) -> String {
    let raw = param(query, name);
    if raw.is_empty() {
        fallback.to_owned()
    } else {
        raw
    }
}

/// and unreadable.
#[must_use]
pub fn bars_html(site: &Site, query: &str) -> (axum::http::StatusCode, String) {
    let symbol = param(query, "symbol");
    let segment = param_or(query, "segment", "INDEX");
    let exchange = param_or(query, "exchange", "NSE");
    // ONE PARSER, shared with the pull form. This route had its own copy that
    // compared with `==` while `ingest::parse_vendor` used
    // `eq_ignore_ascii_case`, so `?vendor=Groww` meant a different vendor here
    // than there — one question, two answers, and BOTH silently fell back to
    // Dhan. Being served Dhan's copy of a month after asking for Groww's, under
    // HTTP 200, is the worst class of defect in this repository: a wrong answer
    // wearing the shape of a right one.
    let asked_vendor = param(query, "vendor");
    let Some(vendor) = ingest::parse_vendor(&asked_vendor) else {
        return bars_refusal(
            site,
            &symbol,
            &segment,
            "—",
            "that is not a vendor this build has a feed for. Refused rather \
             than answered from another vendor's prefix — a month served from \
             the wrong feed looks exactly like the right one.",
        );
    };
    let page = page_number(query);

    let month = {
        let raw = param(query, "month");
        raw.split_once('-')
            .and_then(|(y, m)| store::path::YearMonth::new(y.parse().ok()?, m.parse().ok()?).ok())
    };

    let Some(month) = month else {
        return bars_refusal(
            site,
            &symbol,
            &segment,
            vendor.as_str(),
            "no month was named. A bar file is addressed by (vendor, exchange, \
             segment, symbol, timeframe, month) and the month is the one part \
             that cannot be defaulted — ?month=2025-07",
        );
    };

    // A NAMED-BUT-UNKNOWN RUNG IS THE PAGE'S OWN REFUSAL, not a default. The
    // same page shape the missing-file arm below renders, carrying this
    // sentence instead — so an operator who typed a rung sees why, on the page
    // they asked for, rather than one-minute bars they did not.
    let timeframe = match timeframe_param(query) {
        Ok(tf) => tf,
        Err(why) => {
            return empty_bars_page(
                axum::http::StatusCode::BAD_REQUEST,
                site,
                &BarsAddress {
                    symbol: &symbol,
                    segment: &segment,
                    vendor: vendor.as_str(),
                    month: &month.to_string(),
                    timeframe: store::path::Timeframe::MINUTE_1.as_str(),
                },
                &why,
            );
        }
    };
    match bars::open(
        &site.store_root,
        vendor,
        &exchange,
        &segment,
        &symbol,
        timeframe,
        month,
    ) {
        Err(why) => empty_bars_page(
            axum::http::StatusCode::NOT_FOUND,
            site,
            &BarsAddress {
                symbol: &symbol,
                segment: &segment,
                vendor: vendor.as_str(),
                month: &month.to_string(),
                timeframe: timeframe.as_str(),
            },
            &why,
        ),
        Ok(file) => {
            // `n_valid` is a header field, so the page count is a division and
            // not a walk — the same arithmetic every paged surface here uses.
            let total = usize::try_from(file.header().n_valid).unwrap_or(usize::MAX);
            let last_page = total.saturating_sub(1) / bars::PAGE_BARS;
            let page = page.min(last_page);
            let (rows, faults) =
                bars::page(&file, page.saturating_mul(bars::PAGE_BARS), bars::PAGE_BARS);
            let trouble = (!faults.is_empty()).then(|| faults.join(" · "));
            (
                axum::http::StatusCode::OK,
                render::bars_page(&render::BarsView {
                    symbol: &symbol,
                    segment: &segment,
                    vendor: vendor.as_str(),
                    month: &month.to_string(),
                    rows: &rows,
                    total,
                    page,
                    last_page,
                    timeframe: timeframe.as_str(),
                    trouble: trouble.as_deref(),
                    store_root: &site.store_root.display().to_string(),
                }),
            )
        }
    }
}

/// Whether a live vendor call may leave this process.
///
/// Two states and no default: a bool would read as `false` in a struct literal
/// that forgot it, and "forgot to say" and "said no" must not be the same value
/// when the difference is whether a test hits a broker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Broker {
    /// Reach the vendor. Set only by [`Site::load`], which only the served
    /// process calls.
    Live,
    /// Refuse before a socket is opened, and say so. Every test.
    Refused,
}

/// Every route this server answers, serving from an already-loaded site.
///
/// Takes the site rather than a directory: a router holding a `PathBuf` can
/// only re-read, and re-reading per request is the O(rows) cost this split
/// exists to remove.
///
/// **`/pull/spot` and `/pull/fno` are `post` and nothing else.** A `get` on
/// either answers 405 without touching a vendor, which is the whole point: a
/// crawler follows links, a browser refetches on back, and either would
/// otherwise start an ingest nobody asked for.
///
/// **The request body is bounded here, by a number this repository states.**
/// `crates/api/src/ingest.rs` says every parser it holds works over "a form
/// body whose length the server caps". That was true, and it was true by
/// accident: the cap was `axum`'s own default and no line in this repository
/// named it. A framework default is a bound somebody else may change, and
/// `docs/07-o1-architecture.md` law 5 is bound every input **at the boundary**
/// — which means the boundary says the number.
///
/// # The front end is served by the same process, and it is served last
///
/// Everything named below is a registered route and every one of them wins.
/// The built front end is the **fallback**, so no static file can shadow a JSON
/// route, `POST /pull/spot`, or any server-rendered page — see
/// [`crate::assets`] for what happens to a path that reaches it.
///
/// `/` is deliberately NOT registered. It is the front end's front door — the
/// `SvelteKit` app's `Markets` page, and what `web/svelte.config.js` and
/// `web/src/routes/+layout.svelte` have both said `/` is since they were
/// written. The server-rendered dashboard it displaced answers at
/// `/dashboard`, unchanged and still linked from the nav; D-0064.
pub fn router(site: Loaded) -> axum::Router {
    router_serving(
        site,
        std::sync::Arc::new(assets::Assets::new(&assets::web_dir())),
    )
}

/// [`router`], over a front end the caller names.
///
/// Split for the reason [`run_in`] takes a directory: which directory the
/// assets come from is read from the environment at the edge, and a test cannot
/// set an environment variable — `set_var` is `unsafe` under edition 2024 and
/// this crate forbids `unsafe`. Every routing-order and traversal test drives
/// this one over a scratch directory it owns.
pub fn router_serving(site: Loaded, assets: std::sync::Arc<assets::Assets>) -> axum::Router {
    let typeahead = std::sync::Arc::clone(&assets);
    axum::Router::new()
        .route("/dashboard", axum::routing::get(home))
        .route("/instruments", axum::routing::get(page))
        // THE TYPE-AHEAD'S TWO ROUTES. Both are progressive enhancement: the
        // instruments page renders every row without either of them, and a
        // browser with scripting off sees exactly the form and table it always
        // saw. D-0052.
        .route("/instruments.json", axum::routing::get(instruments_json))
        .route("/feeds.json", axum::routing::get(feeds_json))
        // WHAT A UNIVERSE RESOLVES TO FOR THE SELECTED FEED. The one thing
        // `/ingest` needed to stop drawing four tiers as `no target`, and the
        // one thing no route said. D-0120.
        .route("/universes.json", axum::routing::get(universe_reach_json))
        // HOW FAR A FOLDER FEED REACHES — the files present, and nothing else.
        // Its own route rather than a field on `/feeds.json` because answering
        // it means WALKING the folder, and `/feeds.json` renders on every page
        // load. See `crate::folder`.
        .route(
            "/folder.json",
            axum::routing::get(crate::folder::folder_json),
        )
        .route("/bars.json", axum::routing::get(bars_json))
        .route("/store.json", axum::routing::get(store_json))
        // THE ONLY ROUTE THAT OPENS A BAR FILE TO CHECK THE COUNTER.
        .route("/verify.json", axum::routing::get(verify_json))
        // READ FROM DISK, NOT `include_str!`. It is a file under `web/`, and a
        // crate that reaches into that tree at compile time is the coupling CI
        // gate 1e detaches the tree to find. D-0064.
        .route(
            "/typeahead.js",
            axum::routing::get(move || {
                let typeahead = std::sync::Arc::clone(&typeahead);
                async move { typeahead.typeahead() }
            }),
        )
        .route("/pull", axum::routing::get(pull_get))
        .route("/pull/spot", axum::routing::post(pull_spot))
        .route("/pull/fno", axum::routing::post(pull_fno))
        // THE AUTOPILOT'S THREE. Status is a read of one in-memory struct
        // behind an uncontended lock — never `census_now`, because this is the
        // route an operator refreshes every few seconds through a twelve-hour
        // run and re-reading every manifest per request is the O(entries) cost
        // D-0039 exists to remove.
        .route(
            "/autopilot.json",
            axum::routing::get(autopilot::status_json),
        )
        // POST, AND THERE IS NO GET. This opens ~300 sockets to a third
        // party; a GET would be fetched by a link preview or a health check,
        // and none of those is a person deciding to crawl an exchange. D-0128.
        .route("/universe/resolve", axum::routing::post(universe_resolve))
        .route("/autopilot/pause", axum::routing::post(autopilot::pause))
        .route("/autopilot/resume", axum::routing::post(autopilot::resume))
        // THE ONE CONTROL, AND WHY IT IS NOT AT `/autopilot`. That path is the
        // front end's own page, served by the fallback below; a `post`-only
        // route there makes `GET /autopilot` answer 405, because axum's method
        // router answers a matched path itself and never reaches
        // `Router::fallback`. See `autopilot::control`.
        .route(
            "/autopilot/control",
            axum::routing::post(autopilot::control),
        )
        // WHAT THE NEXT SWEEP IS WAITING ON, and what a queue request is
        // answered with. Both are served from memory and neither reads the
        // store — see `ingest::status_json` for why that is deliberate, and
        // `ingest::queue` for why nothing is ever queued.
        .route(
            "/ingest/status.json",
            axum::routing::get(crate::ingest::status_json),
        )
        .route("/ingest/queue", axum::routing::post(crate::ingest::queue))
        // THE AUDIT'S TWO. `/audit.json` is what the browser console reads and
        // `/audit` is the no-script page, unchanged. Before the JSON route
        // existed the console fetched the PAGE and parsed its table back out
        // with `DOMParser`, which coupled a browser to `render::audit_row`'s
        // markup and left one path answered by two different applications.
        .route("/audit.json", axum::routing::get(audit_json::audit_json))
        .route("/audit", axum::routing::get(audit_get))
        .route("/store", axum::routing::get(store_get))
        .route("/bars", axum::routing::get(bars_get))
        // THE LOG, READABLE FROM THE APPLICATION THAT WROTE IT.
        //
        // `telemetry::tail` — a complete, bounded reader — had NO caller
        // anywhere in this workspace, so the first question after a failed pull
        // ("what did it say?") had no answer inside the thing that said it.
        // See `crate::logs`.
        .route("/logs", axum::routing::get(crate::logs::logs_page))
        .route("/logs.json", axum::routing::get(crate::logs::logs_json))
        .route("/health", axum::routing::get(health))
        // THE FRONT END, LAST. A fallback rather than a `/*path` route, so
        // every line above keeps winning and nothing on disk can shadow one.
        .fallback(move |request: axum::extract::Request| {
            let assets = std::sync::Arc::clone(&assets);
            async move { assets.respond(request.method(), request.uri().path()) }
        })
        // WHO ASKED, NOT ONLY WHICH VERB. `post` stops a crawler; it does not
        // stop the other tab in the operator's browser. Registered INSIDE
        // `note_request` — a later `.layer` on an `axum::Router` wraps the
        // earlier one, so the log sees the 403 — and inside the body limit, so
        // an oversized body is still refused by length first. See
        // [`same_origin_writes_only`] for what this stops and what it does not.
        .layer(axum::middleware::from_fn(same_origin_writes_only))
        .layer(axum::middleware::from_fn(crate::logs::note_request))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_FORM_BYTES))
        .with_state(site)
}

/// The fetch-metadata header a browser stamps on every request it issues.
///
/// Lower case because that is how `HeaderMap` stores and matches a name, and
/// spelled once so the lookup and the refusal sentence cannot disagree about
/// which header decided.
const SEC_FETCH_SITE: &str = "sec-fetch-site";

/// The one `Sec-Fetch-Site` value that is this server's own page.
const SEC_FETCH_SAME_ORIGIN: &str = "same-origin";

/// Refuses a state-changing request that did not come from this server's own
/// page.
///
/// # What "POST, AND THERE IS NO GET" does and does not buy
///
/// [`router_serving`] carries that sentence over `/universe/resolve` and the
/// two `/pull` routes, citing D-0128, and it is right about CRAWLERS: a link
/// preview, a health check and a browser's back button all issue `GET`, and a
/// `GET` on those paths reaches no handler that spends anything.
///
/// It is not a guard against another PAGE. An HTML form posting
/// `application/x-www-form-urlencoded` is a CORS **simple** request: no
/// preflight, no opt-in required from this server, and it fires from any
/// origin the operator happens to have open. Every handler behind these routes
/// takes a bare `body: String` — see [`pull_spot`] and [`autopilot::control`] —
/// so not even the content type narrows what is accepted. [`DEFAULT_ADDR`] is
/// loopback and loopback is not a boundary here: the browser running the
/// hostile page is on the loopback interface too.
///
/// What that reaches is not a read — a cross-origin caller cannot see the
/// response — it is the **write**: `/universe/resolve` opens roughly 300
/// sockets to a third party against the shared token D-0128 exists to protect,
/// `/autopilot/resume` restarts a twelve-hour run, `/pull/spot` begins an
/// ingest. Fire-and-forget is exactly the class this reaches.
///
/// # The rule
///
/// `GET` and `HEAD` pass untouched. They change nothing, and refusing them
/// would take down every page and every JSON route on the site.
///
/// Anything else is decided by `Sec-Fetch-Site`, which the **user agent** sets
/// and page script cannot write — which is the only reason it is worth reading.
/// `same-origin` passes. `cross-site`, `same-site`, `none` and any token this
/// build has never heard of are refused: a value nobody here recognises is not
/// evidence of anything, and defaulting it open is the fallback `CLAUDE.md` §4
/// bans.
///
/// # An ABSENT `Sec-Fetch-Site` is allowed, deliberately, and here is the cost
///
/// `curl` sends none. Every socket test in this crate sends none. A client that
/// is not a browser has no origin to compare in the first place. So an absent
/// header falls through to `Origin` against `Host`, and a request carrying
/// neither passes.
///
/// Stated plainly, because a guard that is not honest about its hole is worse
/// than none: **a browser that sends neither header on a cross-origin form POST
/// is not stopped by this.** Every engine shipping today sends `Origin` on such
/// a POST and all three send `Sec-Fetch-Site`; that is read from the
/// specifications and NOTHING HERE MEASURES IT, so it is a claim about the
/// world and not about this code. This defends against the browser an operator
/// already has open. It does not defend against a hand-written client, and it
/// cannot: such a client can reach this port directly and needs no page's help.
///
/// The `Origin`/`Host` fallback compares **authorities** and cannot compare
/// schemes, because `Host` carries none — `http://t` and `https://t` are one
/// origin to this check. That is the blind spot a plain-HTTP loopback
/// deployment already has, and this does not widen it. An HTTP/2 request
/// carries its authority in the URI rather than in a `Host` header; one that
/// also carries an `Origin` is therefore refused, which is the safe direction
/// and is not a state `axum::serve` over plain TCP reaches from a browser.
async fn same_origin_writes_only(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;

    // DECIDED WHILE THE BORROW IS LIVE. `Next::run` takes the request by value,
    // so the verdict is an owned `String` and nothing of the request is held
    // across the move.
    let Some(why) = cross_origin_refusal(request.method(), request.headers()) else {
        return next.run(request).await;
    };
    (
        axum::http::StatusCode::FORBIDDEN,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        why,
    )
        .into_response()
}

/// Why a request is refused as cross-origin, or `None` when it may proceed.
///
/// Split from [`same_origin_writes_only`] so every arm is reachable from a test
/// without a socket — the argument [`store_body`] makes about the same split,
/// and the reason the middleware above holds no logic of its own.
fn cross_origin_refusal(
    method: &axum::http::Method,
    headers: &axum::http::HeaderMap,
) -> Option<String> {
    // A READ IS NEVER REFUSED. `HEAD` rides with `GET` for the reason
    // `assets::respond` pairs them: it is a `GET` whose body is dropped.
    if method == axum::http::Method::GET || method == axum::http::Method::HEAD {
        return None;
    }
    // A header value that is not visible ASCII reads as NO value, not as a
    // pass: `to_str` refusing tells us nothing about where the request came
    // from, so it falls to the `Origin` comparison below.
    if let Some(site) = headers.get(SEC_FETCH_SITE).and_then(|v| v.to_str().ok()) {
        if site == SEC_FETCH_SAME_ORIGIN {
            return None;
        }
        return Some(cross_origin_sentence(method, "Sec-Fetch-Site", site));
    }
    let Some(origin) = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    else {
        // NO FETCH METADATA AND NO ORIGIN. This is `curl`, this crate's own
        // socket tests, and any non-browser client. See the doc block for why
        // this is a pass and what it costs.
        return None;
    };
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    // The scheme is dropped because `Host` has none to compare against.
    // `Origin: null` — a sandboxed frame's opaque origin — carries no `://`,
    // so it stays `null`, matches no host, and is refused. An empty authority
    // is refused for the same reason rather than matching an absent `Host`.
    let authority = origin
        .split_once("://")
        .map_or(origin, |(_scheme, rest)| rest);
    if !authority.is_empty() && authority == host {
        return None;
    }
    Some(cross_origin_sentence(method, "Origin", origin))
}

/// The `403` body: which header decided, and what it actually said.
///
/// The observed value goes through [`note_alphabet`] — this file's existing
/// reducer to visible ASCII with a 400-character ceiling. It is echoed because
/// an operator debugging a reverse proxy needs to see what arrived, not because
/// it is trusted; the content type is `text/plain`, so nothing here is markup.
fn cross_origin_sentence(method: &axum::http::Method, header: &str, value: &str) -> String {
    format!(
        "REFUSED — a {method} on this server is answered only for its own \
         pages, and {header} says this one came from somewhere else: {}.\n\
         \n\
         No vendor was contacted, no autopilot state moved, and nothing was \
         written. These routes are POST-only so a crawler cannot start them \
         (D-0128); this check is for the other tab in your browser, which \
         POST-only does not stop.\n",
        note_alphabet(value)
    )
}

/// Serves on an already-bound listener until `shutdown` resolves.
///
/// The shutdown signal is a parameter rather than a `ctrl_c()` buried inside,
/// so that a test can drive the whole serve path — accept, route, respond,
/// stop — without a signal and without a hard kill.
///
/// # Errors
///
/// Whatever the server failed with. A server that stops for a reason is a
/// reason the operator gets to read.
pub async fn serve(
    listener: tokio::net::TcpListener,
    app: axum::Router,
    shutdown: Shutdown,
) -> std::io::Result<()> {
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            // The signal's own error is not actionable: a failed ctrl-c
            // registration still means stop, and there is nothing else to do
            // about it here.
            let _ = shutdown.await;
        })
        .await
}

/// The banner line naming where the front end is and what state it is in.
///
/// Split out of [`run_in`] for the reason [`announce_log`] is.
///
/// THE STATE, NOT THE BIT. `Assets::built` is true for any directory that
/// exists, so an empty `build/`, a half-written one, and a bundle older than
/// the sources beside it all printed `serving` while every page answered 503.
/// See [`assets::Build`].
fn announce_front_end(front: &assets::Assets) {
    println!(
        "  web:     {} ({})",
        front.named().display(),
        front.build().note()
    );
}

/// The serve lock, or the exit code a refused second instance earns.
///
/// Split from the serve arm because a refusal that is only printed is a refusal
/// that exists for whoever was watching the terminal — and because the arm it
/// came out of is already at the line ceiling.
///
/// # Errors
///
/// [`FAILED`] — the work could not be done. It is not [`DEGRADED`]: nothing was
/// served, so there is no answer whose trust is in question.
fn sole_server(store_root: &Path, addr: std::net::SocketAddr) -> Result<ServeLock, u8> {
    match take_serve_lock(store_root, addr) {
        Ok(lock) => Ok(lock),
        Err(why) => {
            let _noted = telemetry::emit(
                &telemetry::Event::new(
                    telemetry::Level::Error,
                    "api.serve",
                    "refused: another instance is serving this store",
                )
                .with("why", telemetry::Value::Str(&why)),
            );
            eprintln!("{why}");
            Err(FAILED)
        }
    }
}

/// Prints what the masters read found, and answers whether it was clean.
///
/// # The banner had no line for this
///
/// Eight `println!`s and not one of them named the universe. The masters are
/// read one line above the banner and the verdict was discarded, so a serve
/// with one master missing was byte-identical, on the terminal, to a clean
/// one — and then exited `0`. `/health` answered `503` for the whole session
/// and nothing else did. `CLAUDE.md` §4.
///
/// The word is [`Read::status`]'s own, so the banner, `/health`, `/audit.json`
/// and the exit code cannot drift into four opinions about one read.
fn announce_universe(read: &Read) -> bool {
    println!("  universe: {}", read.status());
    let clean = read.is_clean();
    if !clean {
        for note in &read.notes {
            println!("            {note}");
        }
        println!(
            "            this process will exit {DEGRADED} when it stops, because it \
             served this whole session over a universe it could not fully read."
        );
    }
    clean
}

/// The file that proves this process is the only one serving this store.
///
/// # What was not detected before
///
/// The only guard on a second instance was the bind, and a bind is honest for
/// exactly one shape of collision: a byte-identical address. Measured on this
/// machine — `mio` sets `SO_REUSEADDR` — `127.0.0.1:8080` and `0.0.0.0:8080`
/// both bind, in either order, and every other port is not even a collision.
/// Both processes then print `listening`, both open a browser, and both spawn
/// `autopilot::fly` against the same `BRUTEX_STORE`: two backfills spending one
/// shared vendor token's quota, against one append-only store, with nothing on
/// either terminal saying so. The file locks that do exist are taken *after*
/// the vendor has been paid — `pull::ingest`'s census lock is on the install
/// path and `store::file`'s is per bar file — so the quota is spent before
/// either refuses.
///
/// The subject of this lock is the **store**, not the address: the store is
/// what two processes corrupt each other over, and it is the thing an operator
/// gets wrong by starting a second server "on a different port".
///
/// # Held by the OS, released by the OS
///
/// `File::try_lock` is an advisory lock on the open file description. It is
/// released when the handle closes, which includes a process that was killed —
/// so an abandoned lock file never wedges the next start, the way a PID file
/// written by hand does. The handle is kept alive for the whole session by this
/// value; nothing reads it again.
#[derive(Debug)]
struct ServeLock {
    /// The locked handle. `None` when this process already holds the lock — see
    /// [`take_serve_lock`].
    _held: Option<std::fs::File>,
    /// The store this lock is over, canonical, so [`Drop`] releases the same
    /// key that was taken.
    root: PathBuf,
}

impl Drop for ServeLock {
    fn drop(&mut self) {
        if let Ok(mut held) = serving_roots().lock() {
            held.remove(&self.root);
        }
    }
}

/// The store roots this process is already serving.
///
/// # Why a second serve inside ONE process is allowed
///
/// The defect is two *processes*. A second `serve` in one process is the test
/// harness — this crate drives `run_in` from tokio tests that run in parallel
/// against the developer's real store root — and an advisory lock is held per
/// open file description, so a second handle in the same process would refuse
/// itself. That would turn a suite into a race and would not detect one extra
/// instance of the thing this guards against.
fn serving_roots() -> &'static std::sync::Mutex<std::collections::BTreeSet<PathBuf>> {
    static ROOTS: std::sync::OnceLock<std::sync::Mutex<std::collections::BTreeSet<PathBuf>>> =
        std::sync::OnceLock::new();
    ROOTS.get_or_init(|| std::sync::Mutex::new(std::collections::BTreeSet::new()))
}

/// The name of the lock file inside the store root.
const SERVE_LOCK: &str = "serve.lock";

/// Takes the serve lock for `store_root`, or names who holds it.
///
/// # Errors
///
/// A sentence naming the store, the lock file, and — read out of the lock file
/// the holder wrote — the other instance's address and pid. The reason reaches
/// the operator's terminal and the exit code, because a second instance is not
/// a state to degrade through: `CLAUDE.md` §4 — degrade loudly and name the
/// reason, or refuse.
fn take_serve_lock(store_root: &Path, addr: std::net::SocketAddr) -> Result<ServeLock, String> {
    let path = store_root.join(SERVE_LOCK);
    if let Err(why) = std::fs::create_dir_all(store_root) {
        return Err(format!(
            "REFUSED: the store root {} cannot be created, so the one-server lock \
             cannot be taken there — {why}",
            store_root.display()
        ));
    }
    // CANONICAL, so `~/.brutex/store` and `~/.brutex/store/` and a path through
    // a symlink are one key rather than three.
    let key = std::fs::canonicalize(store_root).unwrap_or_else(|_| store_root.to_path_buf());
    match serving_roots().lock() {
        // A POISONED MUTEX IS NOT A LICENCE TO SKIP THE CHECK. It means another
        // thread panicked holding it; the set is still readable and the lock
        // below is still the real guard, so this continues rather than refusing
        // a server for a fault in a test harness.
        Err(poisoned) => {
            if !poisoned.into_inner().insert(key.clone()) {
                return Ok(ServeLock {
                    _held: None,
                    root: key,
                });
            }
        }
        Ok(mut held) => {
            if !held.insert(key.clone()) {
                return Ok(ServeLock {
                    _held: None,
                    root: key,
                });
            }
        }
    }
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
    {
        Ok(file) => file,
        Err(why) => {
            release_root(&key);
            return Err(format!(
                "REFUSED: the one-server lock {} could not be opened — {why}",
                path.display()
            ));
        }
    };
    if let Err(refusal) = file.try_lock() {
        // WHO HOLDS IT, NOT MERELY THAT SOMEBODY DOES. The holder wrote its
        // address and pid into this file after taking the lock, and reading a
        // locked file is not itself a locked operation.
        let held_by = std::fs::read_to_string(&path).unwrap_or_default();
        let held_by = held_by.trim();
        release_root(&key);
        return Err(format!(
            "REFUSED: another brutex api is already serving this store.\n  \
             store: {}\n  lock:  {} ({refusal})\n  held by: {}\n\
             Two servers over one store run two autopilots against one \
             append-only tree and spend one shared vendor token's quota twice. \
             A different port is not a second store. Stop the other instance, \
             or point this one at another BRUTEX_STORE.",
            store_root.display(),
            path.display(),
            if held_by.is_empty() {
                "an instance that had not yet stamped the file"
            } else {
                held_by
            }
        ));
    }
    // STAMPED AFTER THE LOCK IS HELD, so the value a refused instance reads was
    // written by the instance that actually holds it.
    let stamp = format!("addr={addr} pid={}\n", std::process::id());
    let _ignored_stamp = std::io::Write::write_all(&mut (&file), stamp.as_bytes());
    Ok(ServeLock {
        _held: Some(file),
        root: key,
    })
}

/// Drops a root out of the in-process set after a failed take.
fn release_root(key: &Path) {
    if let Ok(mut held) = serving_roots().lock() {
        held.remove(key);
    }
}

/// The exit code of a server that has stopped, and the reason if it fell over.
///
/// A separate function because a server that fails while accepting is not a
/// state a test can conjure on demand — and an untestable arm inside `run`
/// would be an uncovered branch that the coverage gate could never accept, so
/// the branch lives where it can be exercised directly.
///
/// # Why a clean stop is not always a clean exit
///
/// A `serve` that ran its whole life with one instrument master missing did the
/// work it was asked for — every request was answered — and the answers were
/// drawn over a universe that was never fully read. `Ok(())` from the graceful
/// shutdown was mapped straight to [`OK`], which `api::main`'s `exit_note`
/// prints as *"everything went as asked"*, and a monitor reading the exit code
/// of a supervised process saw green for a session `/health` had been answering
/// `503` for the whole time.
///
/// D-0026 already decided this for `report` — [`reported`] returns [`DEGRADED`]
/// on exactly this state, and `tests/binary.rs` asserts it. This is the same
/// rule on the path that actually runs. A failure still outranks it: a server
/// that fell over is [`FAILED`], and the universe is beside the point.
fn stopped_over(outcome: std::io::Result<()>, clean: bool) -> u8 {
    match outcome {
        Ok(()) if clean => OK,
        Ok(()) => {
            let _noted = telemetry::emit(
                &telemetry::Event::new(
                    telemetry::Level::Error,
                    "api.server",
                    "the server stopped after serving a DEGRADED universe",
                )
                .with("exit", telemetry::Value::Uint(u64::from(DEGRADED))),
            );
            eprintln!(
                "server stopped: the universe was DEGRADED for this whole session — \
                 exiting {DEGRADED} rather than 0, because every answer it gave was \
                 drawn over a read that did not complete. /health said so throughout."
            );
            DEGRADED
        }
        Err(e) => {
            // THE EVENT FIRST. A server that stopped on an error is the single
            // most important line in any post-mortem, and printing it to stderr
            // alone means it exists only for whoever was watching the terminal
            // at the time. The log file is what gets handed to a diagnosis.
            let _noted = telemetry::emit(
                &telemetry::Event::new(
                    telemetry::Level::Error,
                    "api.server",
                    "the server stopped on an error",
                )
                .with("why", telemetry::Value::Str(&e.to_string())),
            );
            eprintln!("server stopped: {e}");
            FAILED
        }
    }
}

/// Everything went as asked.
pub const OK: u8 = 0;
/// It was asked for something reasonable and could not do it.
pub const FAILED: u8 = 1;
/// It was asked for something it does not understand. Distinct from
/// [`FAILED`]: "I do not know what you mean" is not "I tried and failed".
pub const MISUSED: u8 = 2;
/// It did the work and the answer must not be trusted.
///
/// A vendor was never read, or two vendors disagreed, or a listing class
/// nobody recognises turned up. Distinct from [`FAILED`] because the work
/// completed and the output is real — it is the *universe* that is refused,
/// not the run. D-0026. Zero here is what let a monitor read green while one
/// of the two masters was missing.
pub const DEGRADED: u8 = 3;

/// Prints the report for `dir` and returns the exit code it earned.
///
/// A separate function for the same reason [`stopped`] is one: which directory
/// `run` reads comes from the environment, and a test cannot set an
/// environment variable here — `set_var` is `unsafe` under edition 2024 and
/// this crate forbids `unsafe`. Left inline, the `OK` arm would be reachable
/// only from the child process in `tests/binary.rs`, and a branch that only a
/// subprocess can enter is a branch the coverage gate cannot hold. Taking the
/// directory as an argument puts both arms where a test can drive them
/// directly.
fn reported(dir: &Path) -> u8 {
    let (text, clean) = report(dir);
    print!("{text}");
    if clean { OK } else { DEGRADED }
}

/// Runs one command to completion and reports how it went.
///
/// Returns the exit code as a number rather than calling `exit`, so the whole
/// thing — every arm of it — is callable from a test.
pub async fn run(args: &[String], shutdown: Shutdown) -> u8 {
    run_from(masters_dir(), args, shutdown).await
}

/// [`run`], over a masters directory the environment may not have named.
///
/// Split for the reason [`run_in`] is split from [`run`], and this time for a
/// branch rather than for a path: the refusal arm below is only reachable on a
/// machine with no `HOME` and no `BRUTEX_MASTERS`, which no test may create —
/// `set_var` is `unsafe` under edition 2024 and this crate forbids `unsafe`.
/// Taking the `Result` as an argument makes both arms drivable.
async fn run_from(dir: Result<PathBuf, String>, args: &[String], shutdown: Shutdown) -> u8 {
    // A DIRECTORY THAT COULD NOT BE DERIVED IS NOT A DIRECTORY. `.` used to
    // stand in here, and the report it produced named two vendors as missing
    // rather than naming the environment that never pointed at them. `FAILED`
    // and not `DEGRADED`: nothing was read, so there is no output whose trust
    // is in question. D-0124.
    match dir {
        Ok(dir) => run_in(&dir, args, shutdown).await,
        Err(why) => {
            eprintln!("{why}");
            FAILED
        }
    }
}

/// The environment variable naming where the rolling log is written.
pub const LOG_DIR_ENV: &str = "BRUTEX_LOGS";

/// The environment variable that decides how much is written.
///
/// One of `trace`, `debug`, `info`, `warn`, `error`, case-insensitive.
pub const LOG_LEVEL_ENV: &str = "BRUTEX_LOG_LEVEL";

/// The quietest level this process writes, and the sentence that says why.
///
/// # Why this is an environment variable and not a constant
///
/// `crates/pull` now emits at `Debug` on the per-member path and `Trace` on the
/// per-chunk one. Those are the events an operator wants **while diagnosing**
/// and never during a clean backfill: the sink keeps
/// [`telemetry::DEFAULT_KEEP_FILES`] files of
/// [`telemetry::DEFAULT_MAX_FILE_BYTES`] each — a 64 MiB window — and a
/// one-minute backfill is ~62,600 requests, so writing every chunk would roll
/// the run's own beginning out of the window before it finished. The evidence
/// would destroy itself.
///
/// So the default stays `Info` and the operator turns it up for one run. A
/// filtered event costs one relaxed atomic load and touches no clock, no lock
/// and no buffer, which is what makes instrumenting every corner affordable
/// rather than a tax on the quiet path.
///
/// # An unreadable word is `Info` AND SAYS SO
///
/// Never a silent fallback (`CLAUDE.md` §4): the returned sentence names what
/// was found and what was used, and the caller prints it beside the log path.
fn served_log_level() -> (telemetry::Config, String) {
    log_level_from(
        std::env::var_os(LOG_LEVEL_ENV)
            .as_deref()
            .map(|raw| raw.to_string_lossy().into_owned())
            .as_deref(),
    )
}

/// [`served_log_level`] with the environment as an argument, so a test owns it.
///
/// Split for the reason [`log_dir_from`] is split, and it is the same reason:
/// a function that reads the environment can only be tested by mutating the
/// environment, which is process-global, races every other test in the binary,
/// and is `unsafe` under edition 2024 — which every crate here forbids
/// outright. The parser is the part worth testing and it takes a string.
fn log_level_from(raw: Option<&str>) -> (telemetry::Config, String) {
    let dir = std::path::PathBuf::new();
    let Some(raw) = raw else {
        return (
            telemetry::Config::new(dir),
            format!(
                "the default; set {LOG_LEVEL_ENV}=debug, or {LOG_LEVEL_ENV}=info,pull=debug for one subsystem"
            ),
        );
    };
    let text = raw.to_ascii_lowercase();
    let mut config = telemetry::Config::new(dir);
    let mut global: Option<telemetry::Level> = None;
    let mut targets: Vec<String> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    let mut dropped: Vec<String> = Vec::new();

    for clause in text.split(',').map(str::trim).filter(|c| !c.is_empty()) {
        // `target=level` is an override; a bare word is the global floor. The
        // split is on the FIRST `=` only, so a target may not contain one —
        // and a dotted subsystem name never does.
        match clause.split_once('=') {
            Some((target, word)) => {
                let (target, word) = (target.trim(), word.trim());
                if let Some(level) = telemetry::Level::of_label(word)
                    && !target.is_empty()
                {
                    // `Config::with_target_level` DROPS silently past
                    // `MAX_TARGET_LEVELS` — its own doc says so. The length is
                    // the only signal it offers, so the caller has to look, and
                    // an override the operator asked for and did not get is
                    // `CLAUDE.md` §4's silent fallback if nobody says which.
                    let before = config.target_levels.len();
                    config = config.with_target_level(target, level);
                    if config.target_levels.len() > before {
                        targets.push(format!("{target}={word}"));
                    } else {
                        dropped.push(format!("{target}={word}"));
                    }
                } else {
                    unreadable.push(clause.to_owned());
                }
            }
            None => {
                if let Some(level) = telemetry::Level::of_label(clause) {
                    global = Some(level);
                } else {
                    unreadable.push(clause.to_owned());
                }
            }
        }
    }

    if let Some(level) = global {
        config = config.with_min_level(level);
    }
    // NEVER A SILENT FALLBACK. Anything unreadable is named beside what was
    // used, and the caller prints the whole sentence next to the log path —
    // `CLAUDE.md` §4. An operator who mistypes `pul=debug` gets told, rather
    // than getting a quiet `info` and wondering where the lines went.
    let mut note = match global {
        Some(level) => format!("floor {} from {LOG_LEVEL_ENV}", level.label()),
        None => format!("floor info (default), {LOG_LEVEL_ENV} named no bare level"),
    };
    if !targets.is_empty() {
        let _ = write!(note, "; per subsystem: {}", targets.join(" "));
    }
    if !unreadable.is_empty() {
        let _ = write!(
            note,
            "; IGNORED as unreadable: {} — a level is one of {}",
            unreadable.join(" "),
            telemetry::LEVELS
                .iter()
                .map(|l| l.label())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    // NAMED, AND ONLY WHEN IT HAPPENED. This tested
    // `target_levels.len() == MAX_TARGET_LEVELS`, which is true of an operator
    // who wrote exactly eight overrides and had every one of them applied — so
    // it announced "later ones were dropped" when nothing had been. A false
    // alarm is the same defect as a silent drop wearing the other face: both
    // leave the reader's belief about the log wrong, and the one that cries
    // wolf also teaches them to stop reading the banner.
    if !dropped.is_empty() {
        let _ = write!(
            note,
            "; DROPPED past the {}-override ceiling: {}",
            telemetry::MAX_TARGET_LEVELS,
            dropped.join(" ")
        );
    }
    (config, note)
}

/// The environment variable that suppresses opening a browser on start.
pub const NO_OPEN_ENV: &str = "BRUTEX_NO_OPEN";

/// Opens the operator's browser at `url`, and returns what happened.
///
/// # Why this is not a violation of `CLAUDE.md` §2
///
/// §2 forbids "any `build.rs` that invokes an external process". This is not a
/// build script: nothing here runs during `cargo build`, and gate 2 greps
/// `*build.rs` for exactly that reason. The bare-machine promise §2 protects —
/// that `cargo build`, `cargo test` and `cargo clippy` pass with no foreign
/// toolchain — is untouched, because the only thing spawned here is the
/// operating system's own URL handler, at run time, on a machine that by
/// definition already has a browser the operator is about to look at.
///
/// # Why it is opt-OUT rather than opt-in
///
/// The whole stated run procedure is one click, and a procedure whose last step
/// is "now go and type the address yourself" is not one click. A default that
/// has to be enabled would leave every fresh clone in the state this function
/// exists to remove.
///
/// # Why a failure is reported rather than swallowed
///
/// There is no browser on a CI runner, in a container, or over SSH, and the
/// spawn fails there. That is not an error worth stopping a server for — but a
/// silent failure would leave an operator waiting for a window that is never
/// coming, so the caller prints the URL. `CLAUDE.md` §4: degrade loudly and
/// name the reason.
fn open_in_browser(url: &str) -> Result<(), String> {
    open_unless_suppressed(url, std::env::var_os(NO_OPEN_ENV).as_deref())
}

/// [`open_in_browser`] with the opt-out as an argument, so a test owns it.
///
/// Split out because the workspace denies `unsafe_code` and `std::env::set_var`
/// is `unsafe` in edition 2024 — so the suppression arm is untestable while the
/// variable is read inside the function. That constraint pushed toward the same
/// shape [`log_dir_from`] already has, and the lint was right: a function that
/// reads process-global state is one no test can pin without racing every other
/// test in the binary.
fn open_unless_suppressed(url: &str, suppressed: Option<&std::ffi::OsStr>) -> Result<(), String> {
    if suppressed.is_some() {
        return Err(format!("{NO_OPEN_ENV} is set"));
    }
    // THE HANDLER IS THE PLATFORM'S, NEVER A BROWSER BY NAME. Naming a browser
    // picks one the operator may not use and may not have; the OS already knows
    // which one they chose.
    let (program, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[])
    } else if cfg!(target_os = "windows") {
        ("cmd", &["/C", "start", ""])
    } else {
        ("xdg-open", &[])
    };
    std::process::Command::new(program)
        .args(args)
        .arg(url)
        // DETACHED. The child is the operator's browser and outlives this
        // process; inheriting stdout would interleave its noise with the
        // banner, and waiting on it would block the server behind a window.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|why| format!("{program}: {why}"))
}

/// Where the rolling log goes: `BRUTEX_LOGS`, else `./logs` when the process
/// was started from a Cargo workspace, else beneath the store.
///
/// # Why the working directory is consulted at all
///
/// Every other path in this binary is deliberately independent of where it was
/// launched from, because a path that moves with the shell is a path no test
/// can pin. This one is the exception, and the reason is the operator rather
/// than the code: the whole run procedure is *press Run in the IDE*, and the
/// IDE runs the binary with the working directory set to the project root. A
/// log written beneath the store lands in `~/.brutex`, which is not in the
/// project tree and therefore not somewhere the operator can see it without
/// leaving the window they are already looking at.
///
/// THE PROBE IS FOR `Cargo.toml`, NOT FOR A NAME. Testing that the directory
/// is *called* `brutex` would write logs into any directory that happened to
/// share the name, and would stop working the moment the checkout was renamed.
/// A `Cargo.toml` in the working directory means the process was started from
/// a Rust project root, which is exactly the condition the IDE creates and a
/// bare `./api` from `$HOME` does not.
///
/// The fallback is beneath the store rather than the current directory,
/// because a server started from `/` or from a read-only directory must still
/// log somewhere it can write, and the store root is already required to be
/// writable for the process to do anything at all.
fn served_log_dir(store_root: &Path) -> PathBuf {
    log_dir_from(
        std::env::var_os(LOG_DIR_ENV),
        std::env::current_dir().ok().as_deref(),
        store_root,
    )
}

/// Whether `dir` is the root of a Cargo **workspace**, not merely a crate.
///
/// # Why the probe is `[workspace]` and not the file's existence
///
/// The first version of this tested `dir.join("Cargo.toml").is_file()`, which is
/// true in the workspace root AND in every one of the eight crate directories
/// beneath it. `cargo test -p api` runs with the working directory set to
/// `crates/api`, so the test suite created `crates/api/logs/` on every run — an
/// untracked directory holding a `.ndjson`, which is on no gate 1 allowlist. One
/// `git add -A` would have turned that into the build failure `CLAUDE.md` §2
/// describes, from a test run.
///
/// A `[workspace]` table appears in exactly one manifest per tree, which is the
/// property actually wanted: *this is the top*.
///
/// THE READ IS FALLIBLE AND A FAILURE IS `false`. An unreadable manifest is not
/// a workspace root as far as this question goes, and the caller's fallback —
/// beneath the store — is always writable or the process could not serve at all.
fn is_workspace_root(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("Cargo.toml")).is_ok_and(|text| {
        text.lines()
            .any(|line| line.trim_start().starts_with("[workspace]"))
    })
}

/// [`served_log_dir`] with the environment and the working directory as
/// arguments, so a test owns both.
///
/// Split out for the reason [`run_in`] is split from [`run`]: a function that
/// reads the environment can only be tested by mutating the environment, which
/// is process-global and races every other test in the binary.
fn log_dir_from(
    named: Option<std::ffi::OsString>,
    cwd: Option<&Path>,
    store_root: &Path,
) -> PathBuf {
    if let Some(named) = named {
        return PathBuf::from(named);
    }
    match cwd {
        Some(cwd) if is_workspace_root(cwd) => cwd.join("logs"),
        _ => telemetry::dir_beneath_store(store_root),
    }
}

/// Read here rather than threaded through [`run_in`] for the same reason
/// [`masters_dir`] is read in [`run`]: the environment is consulted once, at
/// the edge, and everything below takes a path.
///
/// # Errors
///
/// The environment names no store root. See [`store_dir_from`].
fn served_store_root() -> Result<PathBuf, String> {
    store_dir()
}

/// The two banner lines about the rolling log, or the one that says there is
/// none.
///
/// Split out of [`run_in`], which is at `clippy::too_many_lines`' bound. That
/// bound is not a style rule here: a function that must grow to add a refusal
/// is a function whose next refusal gets left out, and this change added one.
/// The install itself stays in [`run_in`] because the log is opened **before**
/// the first line is printed, and moving the call would move that.
fn announce_log(logging: Result<&&'static telemetry::Sink, &String>, level_note: &str) {
    match logging {
        Ok(sink) => {
            println!(
                "  log:     {} (rolling, {} files x {} MiB ceiling)",
                sink.path().display(),
                telemetry::DEFAULT_KEEP_FILES,
                telemetry::DEFAULT_MAX_FILE_BYTES / (1024 * 1024),
            );
            println!("  level:   {level_note}");
        }
        Err(why) => println!("  log:     NOT WRITABLE — {why}"),
    }
}

/// [`run`], over a directory the caller names.
///
/// Split for the same reason [`masters_dir_from`] and [`reported`] are split,
/// and for a reason found the hard way. With the directory read inside this
/// function, the only test that could reach the `Report` arm had to call the
/// real entry point, which read `$HOME/.brutex/masters` — so on the operator's
/// machine the test parsed 53 MB of real vendor CSV and returned `OK`, and on a
/// CI runner with an empty `HOME` the same test returned `DEGRADED`. Its
/// assertions were written to survive both (`assert_ne!(code, FAILED)`,
/// `assert_ne!(code, MISUSED)`), and since [`reported`] returns only `OK` or
/// `DEGRADED`, **no input, machine or environment could ever falsify them** —
/// a mutant pinning `reported` to `OK` survived. CLAUDE.md §4 bans a test that
/// asserts nothing; §3 rule 5 wants the same inputs to give the same outputs.
///
/// Taking the directory as an argument makes the answer a property of the
/// files, which a test owns, rather than of the machine, which it does not.
async fn run_in(dir: &Path, args: &[String], shutdown: Shutdown) -> u8 {
    run_in_over(dir, served_store_root(), args, shutdown).await
}

/// [`run_in`], over a store root the environment may not have named.
///
/// The third layer of the same split, for the same reason and for a branch this
/// time: `served_store_root` reads the environment, so the refusal arm is only
/// reachable on a machine with no `HOME` and no `BRUTEX_STORE`. A test cannot
/// make one — `set_var` is `unsafe` and this crate forbids `unsafe` — so the
/// value is taken as an argument and both arms are drivable.
async fn run_in_over(
    dir: &Path,
    store: Result<PathBuf, String>,
    args: &[String],
    shutdown: Shutdown,
) -> u8 {
    match Command::parse(args) {
        Ok(Command::Report) => reported(dir),
        Ok(Command::Serve(addr)) => match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                // BEFORE ANYTHING IS OPENED OR CREATED. A store root the
                // environment could not name is a refusal, not a `.`, and the
                // listener is dropped unserved rather than bound over a tree
                // the first append would have built inside the checkout.
                let store_root = match store {
                    Ok(root) => root,
                    Err(why) => {
                        eprintln!("{why}");
                        return FAILED;
                    }
                };
                // THE LOG IS OPENED BEFORE THE FIRST LINE IS PRINTED.
                //
                // Installed here rather than inside `Site::serving` because a
                // failure to open it must be visible in the banner the operator
                // is already reading, and because every line below this one is
                // an event worth keeping. `install` is idempotent per process
                // and returns the sink; a second call in the same process is
                // the one that is refused, not the first.
                //
                // A LOG THAT CANNOT BE OPENED DOES NOT STOP THE SERVER, AND
                // SAYS SO. Refusing to serve because the log directory is
                // unwritable would take the whole application down for the one
                // subsystem whose absence costs no correctness — but a silent
                // fallback to no logging is exactly what `CLAUDE.md` §4 bans,
                // so the reason is named on stdout where the operator sees it.
                // ONE SERVER PER STORE, PROVED BEFORE ANYTHING IS OPENED.
                // The listener above is dropped unserved if this refuses. See
                // `take_serve_lock`.
                let _one_server = match sole_server(&store_root, addr) {
                    Ok(lock) => lock,
                    Err(code) => return code,
                };
                let log_dir = served_log_dir(&store_root);
                // The ENV DECIDES THE LEVELS AND THE CALLER DECIDES THE
                // DIRECTORY. `served_log_level` builds a template it cannot
                // know the path for, so the directory is set here, where it is
                // known, rather than threaded into a parser.
                let (asked, level_note) = served_log_level();
                let logging = telemetry::install(&telemetry::Config {
                    dir: log_dir,
                    ..asked
                });
                println!("brutex api listening on http://{addr}/");
                println!("  masters: {}", dir.display());
                println!("  store:   {}", store_root.display());
                announce_log(logging.as_ref(), &level_note);
                // NAMED WHETHER OR NOT IT IS THERE. A front end that silently
                // is not being served looks exactly like a front end that is
                // broken, and the operator has no way to tell the two apart
                // from the browser. `CLAUDE.md` §4.
                let front = std::sync::Arc::new(assets::Assets::new(&assets::web_dir()));
                announce_front_end(&front);
                // `serving`, not `load`: this is the one process that may
                // reach a broker. See `Broker`.
                let site = Loaded::new(Site::serving(dir, &store_root));
                // THE READ'S OWN VERDICT, ON THE BANNER AND IN THE EXIT CODE.
                //
                // The masters are read HERE, one line above, and until now
                // nothing printed what the read found: a serve that ran its
                // whole life over a universe with one master missing was
                // byte-identical, on the terminal, to a clean one — and then
                // exited `0`, which `main` logs as "everything went as asked".
                // `/health` answered 503 the entire time and nobody polls it.
                //
                // D-0026 applied this to `report` and not to `serve`; this is
                // the other half of it. The word is `Read::status`'s own, so
                // the banner, `/health` and the exit code cannot disagree.
                let universe = site.read.status();
                let clean = announce_universe(&site.read);
                // THE BANNER NAMES THE STATE IT IS IN, NOT THE ONE IT WOULD BE
                // IN IF THE OPERATOR HAD OPTED IN.
                //
                // This line was unconditional, and `Control::serving` pauses
                // unless `BRUTEX_AUTOPILOT=run` — so on the DEFAULT
                // configuration the terminal announced a backfill starting in
                // twenty seconds while `/autopilot.json` said `paused` and
                // nothing was ever asked of a vendor. The two surfaces
                // disagreed about the only fact the operator opens the page
                // for, and the terminal is the one they see first.
                //
                // The false half was not the countdown, it was the absence of
                // the condition: a message about a grace window is true only
                // for a run that is going to happen. `CLAUDE.md` §4 bans a
                // fallback that hides a failure; announcing work that will not
                // start is the same defect wearing the opposite sign.
                let flying_on_start = autopilot::flies_on_startup();
                if flying_on_start {
                    println!(
                        "  autopilot: starting in {}s — http://{addr}/autopilot.json",
                        autopilot::GRACE_SECS
                    );
                } else {
                    println!(
                        "  autopilot: PAUSED — nothing will be asked of any vendor. \
                         Set {}={} to fly on start, or press Resume at http://{addr}/autopilot",
                        autopilot::AUTOPILOT_ENV,
                        autopilot::AUTOPILOT_RUN
                    );
                }
                // THE SAME FACTS THE BANNER PRINTED, AS ONE STRUCTURED RECORD.
                //
                // Deliberately a duplicate rather than a replacement. The
                // banner is for the person watching the Run window and scrolls
                // away; this is for the reader who arrives afterwards asking
                // what this process was configured to do, and it is the record
                // that survives the terminal being closed. Both are cheap and
                // only one of them is durable.
                // THE RETURN VALUE IS READ, NOT DROPPED. `emit` is `#[must_use]`
                // and returns which of four things happened; discarding it here
                // would reintroduce, in the very first event this binary writes,
                // the defect that `Journal::append` still carries — a write
                // whose failure is unobservable. If the FIRST event cannot be
                // written then no later one can either, so this is the single
                // most informative place to check it.
                let first = telemetry::emit(
                    &telemetry::Event::info("api.serve", "listening")
                        .with("addr", telemetry::Value::Str(&addr.to_string()))
                        .with(
                            "store",
                            telemetry::Value::Str(&store_root.display().to_string()),
                        )
                        .with("masters", telemetry::Value::Str(&dir.display().to_string()))
                        .with("web_built", telemetry::Value::Bool(front.built()))
                        .with("web_state", telemetry::Value::Str(front.build().word()))
                        // THE FIELD THAT WAS MISSING. Five facts were recorded
                        // about this process and the state of the universe it
                        // was about to serve was not one of them.
                        .with("universe", telemetry::Value::Str(universe))
                        .with("autopilot_flies", telemetry::Value::Bool(flying_on_start)),
                );
                if !first.is_written() {
                    println!("  log:     FIRST EVENT NOT WRITTEN — {first:?}");
                }
                // THE LAST STEP OF THE RUN PROCEDURE, PERFORMED RATHER THAN
                // PRINTED. The listener is already bound above, so the address
                // handed to the browser answers before the browser can ask —
                // opening it earlier would race the bind and show a refusal
                // page on a server that was about to work.
                let home = format!("http://{addr}/");
                // PORT ZERO IS NEVER OPENED.
                //
                // `0` is the ask-the-kernel-for-any-port marker, so the URL
                // built from it addresses nothing — a browser sent there
                // answers `ERR_UNSAFE_PORT` and nothing else. Every caller that
                // binds `:0` is a harness rather than an operator, and this
                // guard is why running the suite no longer opens a window on
                // the desktop of whoever ran it.
                //
                // The check is on the ADDRESS rather than on a test flag: a
                // flag has to be remembered by every future harness, and the
                // one that forgets is the one that opens the window.
                if addr.port() == 0 {
                    println!("  opening: skipped — port 0 addresses nothing");
                } else {
                    match open_in_browser(&home) {
                        Ok(()) => println!("  opening: {home}"),
                        Err(why) => println!("  opening: NOT OPENED ({why}) — go to {home}"),
                    }
                }
                // SPAWNED, NOT AWAITED. The listener is already bound and
                // `serve` is entered on the next line, so the first request is
                // answered while the autopilot is still counting down its grace
                // window. It holds the same `Arc`, so pause/resume and the
                // status it publishes are the ones the routes read.
                let flying = tokio::spawn(autopilot::fly(Loaded::clone(&site)));
                let code = stopped_over(
                    serve(listener, router_serving(site, front), shutdown).await,
                    clean,
                );
                // Ctrl-C stopped the HTTP surface; stop the backfill too. A
                // sweep aborted mid-append is safe by construction — the bar
                // file commits its header after the records are synced, so a
                // torn write leaves bytes past `n_valid` that the next run
                // overwrites.
                flying.abort();
                code
            }
            Err(e) => {
                // A refused bind is the failure an operator most often has to
                // explain later — the port was taken, the address was wrong, the
                // permission was missing. It belongs in the file, not only on
                // the terminal that happened to be open.
                let _noted = telemetry::emit(
                    &telemetry::Event::new(
                        telemetry::Level::Error,
                        "api.server",
                        "cannot bind the listening address",
                    )
                    .with("addr", telemetry::Value::Str(&addr.to_string()))
                    .with("why", telemetry::Value::Str(&e.to_string())),
                );
                eprintln!("cannot bind {addr}: {e}");
                FAILED
            }
        },
        Err(usage) => {
            eprintln!("{usage}");
            MISUSED
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// A directory holding one or both masters, named after the test.
    /// The third mastered vendor's file, written for every fixture.
    ///
    /// # Why a HEADER AND NO ROWS, and why it is written at all
    ///
    /// `Vendor::MASTERED` gained a third name on 14 Aug 2026, so
    /// `master_paths` looks for three files. A fixture that writes two leaves
    /// the third UNAVAILABLE, and a mastered vendor whose file is missing
    /// degrades the whole report — correctly, and in every test at once, which
    /// tells you nothing about the test.
    ///
    /// A header with no rows is the honest fixture: the master was READ and
    /// listed nothing. That is a state this build must handle and it keeps the
    /// read clean, so a test that is about something else stays about that.
    /// The columns are `core::vendor::MasterColumns` for this vendor, in the
    /// order the exchange publishes them — twelve, and no ISIN among them.
    const ZERODHA_HEAD: &str = "instrument_token,exchange_token,tradingsymbol,name,last_price,\
                                expiry,strike,tick_size,lot_size,instrument_type,segment,exchange\n";

    /// The month, for a fixture that needs to name one.
    fn month_of(year: u16, month: u8) -> store::path::YearMonth {
        store::path::YearMonth::new(year, month).expect("a real month")
    }

    /// The `/store` page body, for a request whose feed this build reads.
    ///
    /// `store_html` answers a status now, because one of its arms is a refusal
    /// — a `feed=` naming a vendor this build has no reader for. Every test
    /// below is about the PAGE, so each one asserts here, once, that it did not
    /// get the refusal instead. Threading `.1` through them would have thrown
    /// that away silently: a test reading the body of a 400 as if it were the
    /// page is exactly the shape this helper exists to make impossible.
    fn store_ok(site: &Site, today: Day, page: usize, query: &str) -> String {
        let (code, html) = store_html(site, today, page, query);
        assert_eq!(
            code,
            axum::http::StatusCode::OK,
            "this fixture names a feed the build reads, so the page is the answer: {html}"
        );
        html
    }

    /// A census holding ONE index-month at DAY level, and nothing else.
    ///
    /// In memory, at a path nothing writes: the ladder gate reads the census
    /// and never a bar file, so a fixture that reached for the disk would be
    /// asserting something this code does not do. `Held::unknown` because the
    /// gate asks whether the month is THERE and never what it closed at.
    fn day_pass_held(
        vendor: Vendor,
        name: &str,
        at: store::path::YearMonth,
    ) -> census::VendorCensus {
        let mut manifest =
            pull::manifest::Manifest::open(vendor, &[], &[]).expect("a genesis census");
        manifest
            .record_held(pull::manifest::Held::unknown(pull::manifest::Entry {
                key: pull::manifest::EntryKey {
                    contract: None,
                    exchange: brutex_core::instrument::Exchange::Nse,
                    segment: brutex_core::instrument::Segment::Index,
                    symbol: brutex_core::symbol::Symbol::new(name).expect("a legal symbol"),
                    timeframe: store::path::Timeframe::DAY_1,
                    month: at,
                },
                // A COUNT THE CENSUS WILL ACCEPT. `record_held` refuses a
                // zero-row entry outright, as `EmptyEntry` — which is what
                // makes absence the gate's whole test.
                rows: 3,
                first_ts_micros: 1,
                last_ts_micros: 2,
            }))
            .expect("the census has room");
        census::VendorCensus {
            vendor,
            path: PathBuf::from("/nonexistent/ladder-gate/census.man"),
            state: census::Census::Held {
                manifest: Box::new(manifest),
            },
        }
    }

    /// One spot instrument, in the segment named.
    fn spot_key(
        name: &str,
        segment: brutex_core::instrument::Segment,
    ) -> brutex_core::instrument::InstrumentKey {
        brutex_core::instrument::InstrumentKey {
            exchange: brutex_core::instrument::Exchange::Nse,
            segment,
            underlying: brutex_core::symbol::Symbol::new(name).expect("a legal symbol"),
            kind: brutex_core::instrument::Kind::Index,
        }
    }

    /// A minute request over a window inside one month.
    fn minute_ask() -> ingest::SpotRequest {
        ingest::parse_spot(
            "target=swept&from=2026-08-03&to=2026-08-05&granularity=1min",
            day(2026, 8, 10),
        )
        .expect("a real target and a window in the past")
    }

    /// THE ORDER REFUSES BEFORE THE LOOP, and the refusal carries the count.
    ///
    /// The gate itself is proven in `crate::ladder`; this is about the WIRING —
    /// that `broker_run` consults it, that it does so before a socket, and that
    /// what an operator reads names the rung to run instead.
    #[test]
    fn a_minute_run_is_refused_until_the_day_pass_has_landed() {
        let asked = minute_ask();
        let targets = [spot_key("NIFTY", brutex_core::instrument::Segment::Index)];

        // NO CENSUS ROW AT ALL and an ABSENT one are the same answer: nothing
        // is held. Both arms are driven here because the store before a first
        // ingest is the ordinary state, not an error.
        for censuses in [
            Vec::new(),
            vec![census::VendorCensus {
                vendor: Vendor::Dhan,
                path: PathBuf::from("/nonexistent/ladder-gate/none.man"),
                state: census::Census::Absent,
            }],
        ] {
            let why = ladder_refusal(&asked, &targets, &censuses)
                .expect("the day pass has not landed, so the minute run is refused");
            assert!(
                why.contains("1day") && why.contains("1 of 1"),
                "the refusal names the rung to run and the arithmetic: {why}"
            );
            assert!(
                why.contains("Nothing was asked for"),
                "and says plainly that no socket was opened: {why}"
            );
        }

        // AND WITH IT HELD, THE SAME REQUEST IS OPEN.
        let held = [day_pass_held(Vendor::Dhan, "NIFTY", month_of(2026, 8))];
        assert_eq!(
            ladder_refusal(&asked, &targets, &held),
            None,
            "the prerequisite is met, so the order has nothing to say"
        );
    }

    /// AN UNREADABLE CENSUS REFUSES RATHER THAN GUESSING EITHER WAY.
    ///
    /// This is the arm that would be a `CLAUDE.md` §4 silent fallback if it
    /// read as "everything held", and a spurious refusal if it read as
    /// "nothing held". It refuses, and it quotes the census's own words.
    #[test]
    fn an_unreadable_census_refuses_and_names_what_would_not_load() {
        let censuses = vec![census::VendorCensus {
            vendor: Vendor::Dhan,
            path: PathBuf::from("/nonexistent/ladder-gate/broken.man"),
            state: census::Census::Unreadable {
                reason: "entry 4 fails its own checksum".to_owned(),
            },
        }];
        let why = ladder_refusal(
            &minute_ask(),
            &[spot_key("NIFTY", brutex_core::instrument::Segment::Index)],
            &censuses,
        )
        .expect("a census that will not load cannot clear the order");
        assert!(
            why.contains("entry 4 fails its own checksum"),
            "the census's own words reach the operator: {why}"
        );
        assert!(
            why.contains("will not open a socket on a guess"),
            "and the reason it refused rather than proceeded: {why}"
        );
    }

    /// A DERIVATIVE WITH NO SPOT BEHIND IT IS REFUSED FOR *THAT*, not the rung.
    ///
    /// Unreachable from `/pull/spot`, which names index and cash instruments
    /// only — so it is driven directly. The arm exists because `Gate` carries
    /// the variant, and the day `/pull/fno` has transport it is the sentence
    /// that runs.
    #[test]
    fn a_derivative_is_refused_for_its_missing_underlying() {
        let why = ladder_refusal(
            &minute_ask(),
            &[spot_key("NIFTY", brutex_core::instrument::Segment::Fno)],
            &[],
        )
        .expect("no spot behind it");
        assert!(
            why.contains("the underlying comes first"),
            "the SEGMENT refusal, not the rung one: {why}"
        );
        assert!(
            why.contains("cannot be verified against anything"),
            "and why that order exists at all: {why}"
        );
    }

    /// THE RECEIPT CARRIES THE ORDER'S OWN WORDS AND ITS OWN CODE.
    ///
    /// This is the regression, and it is the whole reason `Blocked` carries a
    /// status: `broker_answer` restated the reason and the code as literals.
    /// That was TRUE while `Broker::Refused` was the only thing that could
    /// block a run, and it became false the moment the pull order became the
    /// second — an out-of-sequence request would have told the operator the
    /// broker was unreachable, with a 503 to agree with it. Which is the
    /// mistake `BrokerRun::touched_wire` records costing a diagnosis once
    /// already, one number over.
    #[tokio::test]
    async fn a_run_the_order_refuses_says_so_on_its_receipt_and_answers_409() {
        let _sink = crate::emitted::sink();
        let dir = masters(
            "ladder-receipt",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,1333\n"
            )),
        );
        let root = store_root("ladder-receipt");
        // NOTHING HELD, AND IT IS THE DISK THAT SAYS SO. `broker_answer` reads
        // a FRESH census per request now, so seeding `site.censuses` would be
        // ignored — this scratch root has no manifest, which is exactly the
        // state before a first ingest and exactly what this refusal is for.
        let site = Site::serving(&dir, &root);
        let journal = audit::Journal::at(&root);

        let (code, body) = broker_answer(
            minute_ask(),
            std::time::SystemTime::UNIX_EPOCH,
            &site,
            &journal,
            Vec::new(),
        )
        .await;

        assert_eq!(
            code,
            axum::http::StatusCode::CONFLICT,
            "out of sequence is a CONFLICT; nothing upstream is unavailable: {body}"
        );
        assert!(
            body.contains("1day pass comes first"),
            "the receipt names the pass to run instead: {body}"
        );
        assert!(
            !body.contains("may not reach a live broker"),
            "and does NOT report the broker as unreachable — the literal this \
             block used to restate for every blocked run: {body}"
        );
    }

    /// NO REASON THIS BUILD WRITES IS MISTAKEN FOR A CREDENTIAL OR A DISK.
    ///
    /// `autopilot::classify` reads a refusal by SUBSTRING, and `observe` turns
    /// a feed-wide `Credential` into a permanent halt and a repeated `Store`
    /// into another. So every sentence that can reach it is classifier input,
    /// not prose — and this asserts the ones this module authors.
    ///
    /// It exists because a draft of `unreachable_broker` said "no credential
    /// was read". One word, and a transport-shaped refusal that should back off
    /// for thirty seconds became a halt telling the operator their Parameter
    /// Store token was dead. Nothing in the sentence was false; it was simply
    /// being read by something other than a human.
    #[test]
    fn no_refusal_this_module_writes_is_read_as_a_credential_or_disk_fault() {
        let ladder_refusals = [
            ladder_refusal(
                &minute_ask(),
                &[spot_key("NIFTY", brutex_core::instrument::Segment::Index)],
                &[],
            )
            .expect("the rung refusal"),
            ladder_refusal(
                &minute_ask(),
                &[spot_key("NIFTY", brutex_core::instrument::Segment::Fno)],
                &[],
            )
            .expect("the segment refusal"),
        ];
        let broker = BrokerRun::unreachable_broker()
            .blocked
            .expect("it is blocked")
            .why;

        for why in ladder_refusals.iter().map(String::as_str).chain([&*broker]) {
            assert_eq!(
                crate::autopilot::classify(why),
                crate::autopilot::Trouble::Transport,
                "this reason halts the backfill for a fault it is not about: {why}"
            );
        }
    }

    /// AN ARCHIVE FEED IS EXEMPT, and the wiring honours that too.
    #[test]
    fn a_folder_feed_reaches_the_loop_with_an_empty_store() {
        let asked = ingest::parse_spot(
            "target=swept&from=2026-08-03&to=2026-08-05&granularity=1min&vendor=gdfl",
            day(2026, 8, 10),
        )
        .expect("a real archive request");
        assert_eq!(
            ladder_refusal(
                &asked,
                &[spot_key("NIFTY", brutex_core::instrument::Segment::Index)],
                &[],
            ),
            None,
            "a folder feed issues no request, so there is no expensive call for \
             a cheap one to protect"
        );
    }

    fn masters(name: &str, groww: Option<&str>, dhan: Option<&str>) -> PathBuf {
        let dir = crate::scratch::path(&format!("server-{name}"));
        std::fs::create_dir_all(&dir).expect("mkdir");
        for (file, body) in [
            ("groww_instruments.csv", groww),
            ("dhan_scrip.csv", dhan),
            ("zerodha_instruments.csv", Some(ZERODHA_HEAD)),
        ] {
            let path = dir.join(file);
            match body {
                Some(text) => {
                    let mut f = std::fs::File::create(&path).expect("create");
                    f.write_all(text.as_bytes()).expect("write");
                }
                None => {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        dir
    }

    /// An empty store root, named after the test.
    ///
    /// Empty on purpose in most tests: an absent manifest is the ordinary
    /// state before the first ingest, and it must render rather than fail.
    fn store_root(name: &str) -> PathBuf {
        let dir = crate::scratch::path(&format!("store-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("manifest")).expect("mkdir");
        dir
    }

    /// A day this build can name, for the pages that take one.
    fn day(y: u16, m: u8, d: u8) -> Day {
        Day::new(y, m, d).expect("a real date")
    }

    /// A site over the given masters and an empty store.
    fn site(name: &str, dir: &Path) -> Site {
        Site::load(dir, &store_root(name))
    }

    /// A fixed moment, so a recorded run is the same record on every run.
    ///
    /// 04:30 UTC on the day every gate below is driven with, which is 10:00
    /// IST — so a record's stamp and a page's `today` cannot disagree, and no
    /// assertion here is a property of when the suite happened to run.
    fn moment() -> std::time::SystemTime {
        let secs = u64::from(day(2026, 8, 7).days_from_epoch()) * 86_400 + 4 * 3_600 + 30 * 60;
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs)
    }

    const GROWW_HEAD: &str = "exchange,segment,underlying_symbol,trading_symbol,\
                              instrument_type,series,isin,expiry_date,strike_price,groww_symbol\n";
    // `SERIES` is the column the gate reads; `INSTRUMENT_TYPE` is present
    // because the real file has it, and is deliberately never read. D-0025.
    const DHAN_HEAD: &str = "EXCH_ID,SEGMENT,ISIN,INSTRUMENT,UNDERLYING_SYMBOL,SYMBOL_NAME,\
                             INSTRUMENT_TYPE,SERIES,SM_EXPIRY_DATE,STRIKE_PRICE,OPTION_TYPE,SECURITY_ID\n";

    /// Both vendors, agreeing about NIFTY and RELIANCE.
    /// **A refused bind reaches the LOG, not only the terminal.**
    ///
    /// This site was one `eprintln!` and nothing else: the port was taken, the
    /// operator saw a line on whatever terminal was open, and the log file --
    /// the thing handed to somebody diagnosing it afterwards -- said nothing.
    /// Gate 23 exists for that shape, and this drives the event it now emits.
    ///
    /// **Not listed as unreachable, because it is not.** `emitted::UNREACHABLE`
    /// carries three struck-through rows whose lesson is that every one of them
    /// named a dependency the site did not have. A refused bind needs no vendor,
    /// no credential and no byte of real data -- it needs a port somebody else
    /// already holds, which is two lines of `tokio`.
    #[tokio::test]
    async fn a_refused_bind_is_logged_and_not_only_printed() {
        // The kernel supplies the refusal. `:0` asks for any free port, so this
        // cannot collide with a real service or with a concurrent test binary.
        let squatter = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a port to sit on");
        let taken = squatter.local_addr().expect("the address it took");

        assert_eq!(
            run_in(
                &agreeing("bindrefused"),
                &argv(&["serve", &taken.to_string()]),
                fired()
            )
            .await,
            FAILED,
            "a port already held is a refused bind, and the exit code says so"
        );
        // The listener is dropped here, not before: releasing it early would
        // race the bind and this test would pass for the wrong reason.
        drop(squatter);
    }

    fn agreeing(name: &str) -> PathBuf {
        masters(
            name,
            Some(&format!(
                "{GROWW_HEAD}\
                 NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n\
                 NSE,CASH,,RELIANCE,EQ,EQ,INE002A01018,,,NSE-RELIANCE\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}\
                 NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,1333\n\
                 NSE,E,INE002A01018,EQUITY,RELIANCE,RELIANCE INDUSTRIES,ES,EQ,,,,1333\n"
            )),
        )
    }

    #[test]
    fn the_command_line_is_parsed_or_refused_and_never_guessed() {
        let parse = |args: &[&str]| {
            let owned: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
            Command::parse(&owned)
        };
        assert_eq!(parse(&[]), Ok(Command::Serve(DEFAULT_ADDR)));
        assert_eq!(parse(&["serve"]), Ok(Command::Serve(DEFAULT_ADDR)));
        assert_eq!(
            parse(&["serve", "0.0.0.0:9100"]),
            Ok(Command::Serve("0.0.0.0:9100".parse().expect("valid")))
        );
        assert_eq!(parse(&["report"]), Ok(Command::Report));

        // A typo that silently started a server is a typo nobody finds.
        let e = parse(&["serv"]).expect_err("must refuse");
        assert!(e.contains("unknown argument") && e.contains("usage"), "{e}");
        let e = parse(&["report", "now"]).expect_err("must refuse");
        assert!(e.contains("unknown"), "{e}");
        let e = parse(&["serve", "not-an-address"]).expect_err("must refuse");
        assert!(e.contains("not a socket address"), "{e}");
        assert_eq!(DEFAULT_ADDR.port(), 8080);
    }

    #[test]
    fn the_masters_directory_comes_from_the_environment_or_defaults_under_home() {
        // Tested through the value rather than by setting the variable: under
        // edition 2024 `set_var` is unsafe, this crate forbids unsafe, and a
        // test that mutated process-wide state would be racing every other
        // test in the binary anyway.
        //
        // The default is asserted by SHAPE, not by a literal: the home
        // directory differs per machine and per CI runner, and hard-coding one
        // would pass here and fail everywhere else.
        // Absent value delegates to the default, exactly. Asserted against the
        // function rather than against a literal path: the home directory
        // differs per machine and per CI runner, so a literal would pass here
        // and fail everywhere else. The default's own two arms are pinned to
        // literals below, where the input IS controlled.
        assert_eq!(masters_dir_from(None), default_masters_dir());
        // And the default is exactly the pure function over the SAME
        // environment. Compared against `default_masters_dir_from` rather than
        // only against `masters_dir_from(None)`, which calls it: a function
        // compared with its own caller cannot fail, and `cargo mutants` proved
        // it by replacing `default_masters_dir` with an empty path and passing.
        assert_eq!(
            default_masters_dir(),
            default_masters_dir_from(std::env::var_os("HOME"))
        );
        assert!(
            !default_masters_dir().is_ok_and(|d| d.as_os_str().is_empty()),
            "when it names somewhere, an empty path is not a directory"
        );
        assert_eq!(
            masters_dir_from(Some("/somewhere/else".into())),
            Ok(PathBuf::from("/somewhere/else"))
        );
        // Both arms of the default, driven by value rather than by mutating
        // the environment.
        assert_eq!(
            default_masters_dir_from(Some("/home/who".into())),
            Ok(PathBuf::from("/home/who/.brutex/masters"))
        );
        // NO HOME IS A REFUSAL AND THE REFUSAL NAMES THE VARIABLES.
        //
        // This asserted `PathBuf::from(".")` beside the words "no HOME is a
        // broken environment, not a supported one" — the sentence and the
        // return value said opposite things, and the return value won on every
        // machine that ran it. D-0124.
        let refused = default_masters_dir_from(None).expect_err("a broken environment refuses");
        assert!(refused.starts_with("REFUSED:"), "{refused}");
        assert!(
            refused.contains("BRUTEX_MASTERS") && refused.contains("HOME"),
            "the refusal names both variables an operator can set: {refused}"
        );
        assert!(
            !refused.contains("UNAVAILABLE"),
            "it names the environment, not the vendors it never looked for: {refused}"
        );
    }

    #[test]
    fn a_server_that_falls_over_says_so_and_exits_non_zero() {
        assert_eq!(stopped_over(Ok(()), true), OK);
        assert_eq!(
            stopped_over(Err(std::io::Error::other("the socket went away")), true),
            FAILED
        );
        assert_ne!(FAILED, MISUSED, "a misuse is not a failure");
    }

    /// **A serve that ran its whole life over a DEGRADED universe does not exit
    /// zero, and the reason is in the log.**
    ///
    /// The graceful-shutdown path mapped `Ok(())` straight to [`OK`], which
    /// `api::main`'s `exit_note` prints as *"everything went as asked"* — over a
    /// session in which `/health` had been answering `503` from the first
    /// request to the last. D-0026 had already decided this for `report`;
    /// [`reported`] returns [`DEGRADED`] on exactly this state and
    /// `tests/binary.rs` asserts it. This is the same rule on the path that
    /// actually runs.
    ///
    /// Both arms, because the distinction is the whole point: a clean serve is
    /// still `0`, and a serve that FELL OVER is still [`FAILED`] — a universe
    /// verdict does not outrank a server that stopped on an error.
    #[test]
    fn a_serve_over_a_degraded_universe_exits_non_zero_rather_than_claiming_success() {
        let _shared = crate::emitted::sink();
        let from = crate::emitted::mark();

        assert_eq!(stopped_over(Ok(()), true), OK, "a clean serve is zero");
        assert_eq!(
            stopped_over(Ok(()), false),
            DEGRADED,
            "a serve over a universe that was never fully read is not"
        );
        assert_eq!(
            stopped_over(Err(std::io::Error::other("the socket went away")), false),
            FAILED,
            "and a server that fell over is a failure, whatever the universe was"
        );

        let landed = crate::emitted::landed(
            from,
            "api.server",
            "the server stopped after serving a DEGRADED universe",
        );
        assert!(
            landed
                .iter()
                .any(|record| record.level == telemetry::Level::Error),
            "the exit is on the record at Error, not only on the terminal that saw it"
        );
    }

    /// **Two servers over one store are refused, and the refusal names the
    /// other one.**
    ///
    /// The only guard was the bind, which is honest for a byte-identical
    /// address and for nothing else: `127.0.0.1:8080` and `0.0.0.0:8080` both
    /// bind, in either order, and any other port is not even a collision. Both
    /// processes then spawn `autopilot::fly` against one `BRUTEX_STORE` and
    /// spend one shared vendor token's quota twice.
    ///
    /// The lock file stands in for the other process here — a handle this test
    /// holds and never registers, so `take_serve_lock` meets it exactly as it
    /// would meet another `brutex api`. Advisory locks are per open file
    /// description, so this conflicts even inside one process, which is why the
    /// refusal is reachable from a test at all.
    #[tokio::test]
    async fn a_second_server_over_one_store_is_refused_and_the_reason_reaches_the_operator() {
        let _shared = crate::emitted::sink();
        let root = crate::scratch::path("serve-lock");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mkdir");

        // THE OTHER INSTANCE. It writes its own stamp first, so the refusal can
        // quote it the way a real holder's would be quoted.
        let squatter = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(root.join(SERVE_LOCK))
            .expect("the lock file");
        squatter.try_lock().expect("nothing else holds it");
        std::io::Write::write_all(&mut (&squatter), b"addr=127.0.0.1:8080 pid=4242\n")
            .expect("stamp");

        let refused = take_serve_lock(&root, "127.0.0.1:9999".parse().expect("an address"))
            .expect_err("a store already being served is a refusal");
        assert!(refused.contains("already serving this store"), "{refused}");
        assert!(
            refused.contains("pid=4242") && refused.contains("addr=127.0.0.1:8080"),
            "and it names the instance that holds it: {refused}"
        );
        assert!(
            refused.contains("A different port is not a second store"),
            "and the mistake it is actually made by: {refused}"
        );

        // THE WHOLE COMMAND REFUSES, not just the helper: the listener is
        // dropped unserved and the exit code says so.
        let from = crate::emitted::mark();
        let code = run_in_over(
            &agreeing("serve-lock-masters"),
            Ok(root.clone()),
            &argv(&["serve", "127.0.0.1:0"]),
            Box::pin(std::future::pending()),
        )
        .await;
        assert_eq!(code, FAILED, "a second server is a refusal to run");
        let landed = crate::emitted::landed(
            from,
            "api.serve",
            "refused: another instance is serving this store",
        );
        assert!(
            landed.iter().any(|record| {
                record.level == telemetry::Level::Error
                    && crate::emitted::says(record, "why", "already serving this store")
            }),
            "the refusal is in the file as well as on the terminal"
        );

        // AND IT IS RELEASED WITH THE HANDLE. Nothing wedges the next start.
        drop(squatter);
        let taken = take_serve_lock(&root, "127.0.0.1:9999".parse().expect("an address"))
            .expect("the store is free once the other instance is gone");
        drop(taken);
    }

    /// **A refusal that could not be journalled says so on its own page.**
    ///
    /// Nine paths named the journal's answer through [`recorded_fact`], under a
    /// doc comment citing `CLAUDE.md` §4. The three refusal paths threw it away
    /// — `let _ignored = journal.append(&record);` — so on a store root where
    /// the journal cannot be written, `/audit` showed no refusal ever happening
    /// and the receipt said nothing. The autopilot page tells the operator to
    /// conclude the opposite: a failure missing from `/audit` "was never written
    /// down".
    #[tokio::test]
    async fn a_refusal_that_never_reached_the_journal_says_so_on_the_receipt() {
        let root = crate::scratch::path("refusal-journal");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mkdir");
        // A FILE WHERE THE `audit` DIRECTORY HAS TO BE — the same disk state
        // `emitted.rs` drives, and one `Journal::append` genuinely refuses.
        std::fs::write(root.join("audit"), b"not a directory").expect("a file in the way");
        let site = Site::load(
            &masters("refusal-journal", Some(GROWW_HEAD), Some(DHAN_HEAD)),
            &root,
        );

        // A BODY THE PARSER REFUSES, so this is the refusal path and no vendor
        // is reachable from it.
        let (code, html) = spot_answer(
            "target=nonsense&from=2020-01-01&to=2020-01-02",
            Day::new(2026, 8, 12).expect("a real day"),
            std::time::SystemTime::UNIX_EPOCH,
            &site,
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
        assert!(html.contains("REFUSED"), "{html}");
        assert!(
            html.contains("NOT in the journal"),
            "the receipt names what became of the record: {html}"
        );
        assert!(
            html.contains("audit directory"),
            "and it carries the journal's own reason: {html}"
        );

        // AND THE OTHER WAY ROUND: a journal that works says which file it
        // landed in, so the two states are never the same page.
        let good = crate::scratch::path("refusal-journal-ok");
        let _ = std::fs::remove_dir_all(&good);
        std::fs::create_dir_all(&good).expect("mkdir");
        let ok_site = Site::load(
            &masters("refusal-journal-ok", Some(GROWW_HEAD), Some(DHAN_HEAD)),
            &good,
        );
        let (_, html) = spot_answer(
            "target=nonsense&from=2020-01-01&to=2020-01-02",
            Day::new(2026, 8, 12).expect("a real day"),
            std::time::SystemTime::UNIX_EPOCH,
            &ok_site,
        )
        .await;
        assert!(
            html.contains("appended to") && !html.contains("NOT in the journal"),
            "{html}"
        );
    }

    /// **The bar count per instrument is ONE pass over the census, whatever the
    /// universe is.**
    ///
    /// `/instruments.json` folded the census inside a closure that was both the
    /// sort key and the emitted field, so the scan ran once per comparison —
    /// ~22.6 per row at n=785 — and the request cost the PRODUCT of the universe
    /// and the census for a response whose size never moves. Measured in an
    /// optimised build with no disk I/O: 750 entries 15.5 ms, 43,422 entries
    /// 819.2 ms, response flat at ~25 KB.
    ///
    /// The assertion is the pass count and not a duration, because a timing
    /// assertion on a shared machine is a flake. The iterator counts what it
    /// yields, so a future edit that puts the scan back inside the per-symbol
    /// path fails here by COST.
    ///
    /// The proof of the claim above is this test itself,
    /// `api::server::instrument_bar_counts_are_one_pass_over_the_census`. Named
    /// rather than left implicit because gate 12 reads a cost claim in any doc
    /// block, including a test's own, and cannot tell that the assertion is six
    /// lines below the sentence. The two DURATIONS quoted -- 15.5 ms and 819.2 ms
    /// -- are measurements and are not asserted anywhere, which is what the
    /// paragraph above says and why the pass count is the thing pinned.
    ///
    /// # What this asserted before, and why it asserted nothing
    ///
    /// `entries` was `Vec::new()`. The load-bearing line was
    /// `assert_eq!(visits.get(), entries.len(), ...)` with BOTH SIDES ZERO —
    /// `0 == 0`, under a message reading "one visit per entry, and the universe
    /// is not a factor in it". Nothing about a per-symbol scan can fail against
    /// an empty vector, so the O(entries x universe) regression the doc block
    /// on `bars_by_symbol` cites this test against would have walked straight
    /// through it. `CLAUDE.md` §4's "a test that asserts nothing" is the row it
    /// was sitting on.
    ///
    /// It runs at TWO sizes now, and the sizes differ in the UNIVERSE — three
    /// symbols and ten, over the same four months each — because the whole
    /// claim is that the universe is not a factor. A reintroduced inner scan
    /// yields `entries.len() * symbols` and misses at both.
    #[test]
    fn instrument_bar_counts_are_one_pass_over_the_census() {
        // Two universes, four months each. The month count is held fixed so the
        // only thing moving between the passes is the number of distinct
        // symbols -- which is the factor the claim says is absent.
        let small = ["NIFTY", "BANKNIFTY", "RELIANCE"];
        let large = [
            "NIFTY",
            "BANKNIFTY",
            "RELIANCE",
            "TCS",
            "INFY",
            "HDFCBANK",
            "ITC",
            "SBIN",
            "WIPRO",
            "ONGC",
        ];
        for symbols in [small.as_slice(), large.as_slice()] {
            let entries = one_pass_entries(symbols, 4);
            assert_eq!(
                entries.len(),
                symbols.len().saturating_mul(4),
                "the fixture must not be empty, which is the defect this replaces"
            );
            let census = one_pass_census(&entries);
            let visits = std::cell::Cell::new(0_usize);
            let counted = || {
                entries.iter().inspect(|_| {
                    visits.set(visits.get() + 1);
                })
            };

            // THE COST. One visit per entry, at both universe sizes. A scan
            // reintroduced inside the per-symbol path reads
            // `entries.len() * symbols` here -- 36 for the small pass and 400
            // for the large one -- and fails by COST rather than by taste.
            let held = bars_by_symbol(Some(&census), counted());
            assert_eq!(
                visits.get(),
                entries.len(),
                "one visit per entry, and the universe is not a factor in it"
            );

            // AND THE FOLD IS RIGHT, not merely cheap. A pass count on its own
            // is satisfied by a function that walks the entries and counts
            // nothing, so the map is checked too: one key per symbol, and each
            // key carrying every month that symbol holds.
            assert_eq!(
                held.len(),
                symbols.len(),
                "one key per symbol, and no key for a symbol nobody holds"
            );
            for name in symbols {
                let symbol = brutex_core::symbol::Symbol::new(name).expect("a symbol");
                assert_eq!(
                    held.get(&symbol),
                    Some(&(4 * ONE_PASS_ROWS)),
                    "{name} holds four months of {ONE_PASS_ROWS} rows"
                );
            }

            // NO CENSUS IS AN EMPTY MAP, not a map of zeroes: the caller reads a
            // missing key as "no count", which is the distinction D-0124 put on
            // the wire. It does not walk the entries at all -- which is the one
            // assertion here that the old empty fixture could still make.
            visits.set(0);
            let none = bars_by_symbol(None, counted());
            assert!(none.is_empty(), "an unreadable census counts nothing");
            assert_eq!(visits.get(), 0, "and it does not even walk the entries");

            // AN ABSENT MANIFEST OVER THE SAME ENTRIES. The entries are walked
            // -- once each, still -- and every probe misses, so the map is
            // empty rather than a row of zeroes. With the old fixture this arm
            // never entered the loop body at all.
            visits.set(0);
            let absent = census::read_vendor(
                &crate::scratch::path("bars-by-symbol"),
                brutex_core::vendor::Vendor::Dhan,
            );
            let nothing = bars_by_symbol(Some(&absent), counted());
            assert!(nothing.is_empty(), "an absent manifest holds nothing");
            assert_eq!(
                visits.get(),
                entries.len(),
                "and it is still one visit per entry when every probe misses"
            );
        }
    }

    /// The rows every fixture month in [`one_pass_census`] is recorded with.
    ///
    /// A regular NSE session, so a wrong fold reads as a wrong number of
    /// sessions rather than as an anonymous integer.
    const ONE_PASS_ROWS: u64 = 375;

    /// `symbols x months` index entries, oldest month first.
    ///
    /// Real [`census::Series`] values and real months: a fixture of the right
    /// SHAPE is what the test this feeds did not have.
    fn one_pass_entries(
        symbols: &[&str],
        months: u8,
    ) -> Vec<(census::Series, store::path::YearMonth)> {
        let mut out = Vec::with_capacity(symbols.len().saturating_mul(usize::from(months)));
        for name in symbols {
            for m in 1..=months {
                out.push((
                    census::Series {
                        exchange: brutex_core::instrument::Exchange::Nse,
                        segment: brutex_core::instrument::Segment::Index,
                        symbol: brutex_core::symbol::Symbol::new(name).expect("a symbol"),
                        timeframe: store::path::Timeframe::MINUTE_1,
                    },
                    month_of(2026, m),
                ));
            }
        }
        out
    }

    /// A census holding [`ONE_PASS_ROWS`] for every entry it is handed.
    ///
    /// In memory, at a path nothing ever writes -- the same fixture discipline
    /// `percentage_tests::census_of` states: every number the test reads came
    /// out of the manifest, and a change that reached for a bar file would fail
    /// rather than pass slowly.
    fn one_pass_census(
        entries: &[(census::Series, store::path::YearMonth)],
    ) -> census::VendorCensus {
        let mut manifest =
            pull::manifest::Manifest::open(Vendor::Dhan, &[], &[]).expect("a genesis census");
        for &(series, at) in entries {
            manifest
                .record_held(pull::manifest::Held::new(
                    pull::manifest::Entry {
                        key: series.at(at),
                        rows: ONE_PASS_ROWS,
                        first_ts_micros: 1_751_350_800_000_000,
                        last_ts_micros: 1_751_363_940_000_000,
                    },
                    pull::manifest::Closes::UNKNOWN,
                ))
                .expect("records");
        }
        census::VendorCensus {
            vendor: Vendor::Dhan,
            path: PathBuf::from("/nonexistent/bars-by-symbol/dhan.man"),
            state: census::Census::Held {
                manifest: Box::new(manifest),
            },
        }
    }

    #[test]
    fn each_vendor_is_looked_for_under_its_own_file_name() {
        let paths = master_paths(Path::new("/m"));
        // THREE SINCE 14 AUG 2026. `master_paths` walks `Vendor::MASTERED`, so
        // a broker that publishes a master is LOOKED FOR whether or not its
        // file is on disk yet — which is the correct behaviour: a master that
        // is absent reports as unread, and one that is never looked for reports
        // as nothing at all. Zerodha's is the third.
        assert_eq!(paths.len(), brutex_core::vendor::Vendor::MASTERED.len());
        assert_eq!(paths.len(), 3);
        assert!(paths[0].1.ends_with("groww_instruments.csv"));
        assert!(paths[1].1.ends_with("dhan_scrip.csv"));
        assert!(paths[2].1.ends_with("zerodha_instruments.csv"));
    }

    #[test]
    fn a_query_string_is_decoded_without_a_dependency() {
        assert_eq!(parse_query("q=NIFTY"), "NIFTY");
        assert_eq!(parse_query("x=1&q=BANK"), "BANK");
        assert_eq!(parse_query(""), "");
        assert_eq!(parse_query("x=1"), "");
        assert_eq!(parse_query("q=M%26M"), "M&M");
        assert_eq!(parse_query("q=NIFTY+50"), "NIFTY 50");
        // Both cases of hex, because a browser may send either.
        assert_eq!(parse_query("q=%2f%2F"), "//");
        assert_eq!(parse_query("q=%7e"), "~");
        // Undecodable input is kept LITERALLY. A query must never silently
        // become a different query.
        assert_eq!(parse_query("q=100%"), "100%", "an escape at the very end");
        assert_eq!(parse_query("q=a%2"), "a%2", "a truncated escape");
        assert_eq!(parse_query("q=%zz"), "%zz", "not hex at all");
        assert_eq!(parse_query("q=%2z"), "%2z", "half hex is not hex");
    }

    #[test]
    fn the_report_names_every_vendor_and_every_decline_reason() {
        let dir = agreeing("report");
        let (text, clean) = report(&dir);
        assert!(text.starts_with("ok\n"));
        assert!(clean, "both vendors read, nothing disagreed");
        assert!(text.contains("groww: 2 kept"), "{text}");
        assert!(text.contains("dhan: 2 kept"), "{text}");
        assert!(text.contains("merged: 2 instruments"), "{text}");
        assert!(text.contains("0 isin conflicts"), "{text}");
        assert!(text.contains("0 eligibility conflicts"), "{text}");
        // The census is stated on every run, and it separates what two vendors
        // confirmed from what one asserted.
        assert!(
            text.contains("F&O underlyings: 2 resolved, 2 confirmed by every master read"),
            "{text}"
        );
        assert!(
            text.contains("NIFTY Total Market: 1 resolved, 1 confirmed by every master read"),
            "{text}"
        );
        assert!(
            !text.contains("UNCHECKED"),
            "nothing rests on one vendor here"
        );
    }

    #[test]
    fn a_missing_master_is_named_as_unavailable_and_the_status_says_so() {
        // "This vendor lists nothing" and "this vendor was never read" are
        // different facts, and collapsing them is the silent degradation the
        // charter forbids. That was true of the NOTE and false of the STATUS:
        // the first line said `ok` and the exit code was 0, so every monitor
        // read green while half the universe had never been opened.
        let dir = masters(
            "missing",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n"
            )),
            None,
        );
        let (text, clean) = report(&dir);
        assert!(text.starts_with("DEGRADED\n"), "{text}");
        assert!(!clean, "a vendor that was never read is not a clean read");
        assert!(text.contains("groww: 1 kept"), "{text}");
        assert!(text.contains("dhan: UNAVAILABLE"), "{text}");
        assert!(text.contains("merged: 1 instruments"), "{text}");
    }

    #[test]
    fn the_report_counts_declines_by_reason() {
        let dir = masters(
            "reasons",
            Some(&format!(
                "{GROWW_HEAD}\
                 NSE,CASH,,SOMEBOND,EQ,N2,INE002A01018,,\n\
                 NSE,CASH,,SOMESME,EQ,SM,INE002A01018,,,NSE-SOMESME\n"
            )),
            None,
        );
        let (text, _) = report(&dir);
        assert!(text.contains("not an equity listing 1"), "{text}");
        assert!(text.contains("SME board 1"), "{text}");
    }

    #[test]
    fn an_unrecognised_listing_class_names_the_code_and_degrades_the_run() {
        // Renaming the equity series on the real Dhan master dropped 2,438
        // shares under the same label a debenture gets, while the report
        // printed `ok` and exited 0. The code is now named, the reason is its
        // own, and the run is not clean.
        let dir = masters(
            "unrecognised",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,RELIANCE,EQ,EQX,INE002A01018,,,NSE-RELIANCE\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}NSE,E,INE002A01018,EQUITY,RELIANCE,RELIANCE INDUSTRIES,ES,EQX,,,,1333\n"
            )),
        );
        let (text, clean) = report(&dir);
        assert!(!clean, "an alphabet moving is not a routine skip");
        assert!(text.starts_with("DEGRADED\n"), "{text}");
        assert!(text.contains("unrecognised listing class 1"), "{text}");
        assert!(
            text.contains("UNRECOGNISED LISTING CLASS · \"EQX\" ×1"),
            "the CODE is named, not just the count: {text}"
        );
        assert!(
            text.contains("this engine has never seen"),
            "and it says what that means: {text}"
        );
        // A routine bond, by contrast, leaves the run clean.
        let dir = masters(
            "routinebond",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,SOMEBOND,EQ,N2,INE002A01018,,\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}NSE,E,INE002A01018,EQUITY,SOMEBOND,SOME BOND,DEB,N2,,,\n"
            )),
        );
        let (text, clean) = report(&dir);
        assert!(clean, "{text}");
        assert!(text.contains("not an equity listing 1"), "{text}");
    }

    #[test]
    fn an_eligibility_disagreement_is_reported_and_degrades_the_run() {
        // One vendor calls it an equity, the other calls it a fund. Both rows
        // carry the SAME ISIN, so the check has the key it needs -- and before
        // this the declined row was dropped at the reader, so the report said
        // `0 conflicts` and exited 0.
        let dir = masters(
            "eligibility",
            // Groww declines it: series MF is a fund.
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,FISTIPD3GP,EQ,MF,INF090I01VS3,,,NSE-FISTIPD3GP\n"
            )),
            // Dhan keeps it: series EQ.
            Some(&format!(
                "{DHAN_HEAD}NSE,E,INF090I01VS3,EQUITY,FISTIPD3GP,FRANKLIN PLAN,ETF,EQ,,,,1333\n"
            )),
        );
        let (text, clean) = report(&dir);
        assert!(!clean);
        assert!(text.contains("ELIGIBILITY CONFLICT"), "{text}");
        assert!(text.contains("1 eligibility conflicts"), "{text}");
        assert!(text.contains("dhan kept it"), "{text}");
        assert!(text.contains("groww declined it"), "{text}");
        assert!(text.contains("INF090I01VS3"), "{text}");
        // And it reaches a SEARCHED page, not only the unfiltered one.
        let html = instruments_html(&dir, "FISTIP");
        assert!(html.contains("ELIGIBILITY CONFLICT"), "{html}");
    }

    #[test]
    fn an_isin_conflict_reaches_the_report_and_the_page() {
        // The two vendors give one ticker two different ISINs. Neither the
        // report nor the page may swallow that.
        let dir = masters(
            "conflict",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,CHOLAFIN,EQ,EQ,INE121A01024,,,NSE-CHOLAFIN\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}NSE,E,INE121A08PJ0,EQUITY,CHOLAFIN,CHOLA,ES,EQ,,,,1333\n"
            )),
        );
        let (text, clean) = report(&dir);
        assert!(text.contains("ISIN CONFLICT"), "{text}");
        assert!(text.contains("1 isin conflicts"), "{text}");
        assert!(
            text.contains("INE121A01024") && text.contains("INE121A08PJ0"),
            "{text}"
        );
        assert!(
            !clean,
            "D-0026: a disagreement REFUSES the universe rather than logging it"
        );
        assert!(text.starts_with("DEGRADED\n"), "{text}");

        let html = instruments_html(&dir, "");
        assert!(html.contains("ISIN CONFLICT"), "the page says so too");
        assert!(html.contains("clash"), "and the row is marked");
    }

    #[test]
    fn a_page_number_that_is_not_a_number_is_page_one() {
        // `?page=abc` is a mangled bookmark or a hand-typed URL. It must render
        // the first page, not fail and not 500 -- but note this is the ONE
        // place a bad parameter is quietly defaulted rather than refused, and
        // it is defensible only because a page number selects a VIEW and can
        // never change what the data says.
        assert_eq!(page_number("page=abc"), 0);
        assert_eq!(page_number("page="), 0);
        assert_eq!(page_number("page=-1"), 0);
        assert_eq!(page_number("page=2"), 2);
        assert_eq!(page_number(""), 0);
    }

    #[test]
    fn the_escape_hatch_reaches_a_listing_the_tracked_universe_excludes() {
        // RAJESHEXPO is a real-shaped equity row that is in neither NIFTY Total
        // Market nor the index series, so it is the only kind of instrument the
        // tracked filter actually removes. Without a row like this the filter
        // can never be observed doing anything, and `all` can never be observed
        // undoing it.
        let dir = masters(
            "hatchreach",
            Some(&format!(
                "{GROWW_HEAD}\
                 NSE,CASH,,RAJESHEXPO,EQ,EQ,INE343B01030,,,NSE-RAJESHEXPO\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}\
                 NSE,E,INE343B01030,EQUITY,RAJESHEXPO,RAJESH EXPORTS,ES,EQ,,,,1333\n"
            )),
        );
        let read = universe(&dir);

        // Tracked view: the gate accepted it as a share, and the universe
        // filter still excludes it — those are different questions.
        let tracked = instruments_html_from(&read, "", "", false, "", 0);
        assert!(
            !tracked.contains("NSE-RAJESHEXPO"),
            "not in NIFTY Total Market, so not tracked"
        );
        assert!(tracked.contains("0 instruments total"));

        // The escape hatch reaches it. This is the whole reason the hatch
        // exists: an instrument the filter hides must still be inspectable.
        let every = instruments_html_from(&read, "", "", true, "", 0);
        assert!(
            every.contains("NSE-RAJESHEXPO"),
            "?all=1 reaches every listing"
        );
        assert!(every.contains("1 instruments total"));
    }

    #[test]
    fn a_leading_minus_reverses_the_order_without_a_second_comparator() {
        // Ascending and descending share ONE ordering rule per column: the
        // keys are sorted ascending and then reversed. Two comparators is how
        // ties silently break differently in each direction.
        let read = universe(&agreeing("desc"));
        let up = instruments_html_from(&read, "", "symbol", false, "", 0);
        let down = instruments_html_from(&read, "", "-symbol", false, "", 0);

        let first = |html: &str| -> String {
            html.split("<tr class=")
                .nth(1)
                .and_then(|r| r.split("<td>").nth(1))
                .and_then(|c| c.split("</td>").next())
                .unwrap_or_default()
                .to_owned()
        };
        assert_ne!(first(&up), first(&down), "the two orders differ");
        assert!(up.contains("NSE-NIFTY") && up.contains("NSE-RELIANCE"));
        assert!(down.contains("NSE-NIFTY") && down.contains("NSE-RELIANCE"));

        // An unknown column reverses the DEFAULT order rather than erroring.
        let stale = instruments_html_from(&read, "", "-no-such-column", false, "", 0);
        assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
    }

    #[test]
    fn every_row_is_reachable_by_paging_and_a_stale_page_clamps() {
        // The cap alone was a wall: 200 of 785 rendered and nothing led to the
        // rest. Scrolling cannot reveal rows that were never sent.
        let read = universe(&agreeing("paging"));

        // This fixture is smaller than one page, so there is no pager at all --
        // navigation that leads nowhere is worse than none.
        let one = instruments_html_from(&read, "", "", false, "", 0);
        assert!(
            !one.contains("class=\"pager\""),
            "no pager for a single page"
        );
        assert!(one.contains("NSE-NIFTY") && one.contains("NSE-RELIANCE"));

        // A page beyond the end CLAMPS to the last page rather than rendering
        // an empty table: `?page=999` is a stale bookmark, not an attack, and
        // it should land somewhere real.
        let past = instruments_html_from(&read, "", "", false, "", 999);
        assert!(
            past.contains("NSE-NIFTY") && past.contains("NSE-RELIANCE"),
            "an out-of-range page clamps to the last one, it does not empty out"
        );
        assert_eq!(past, one, "clamping lands exactly on the last page");
    }

    #[test]
    fn every_sort_column_orders_and_none_of_them_errors() {
        // Each arm of the sort match is a separate closure; an arm no test
        // enters is an arm that can be wrong forever. RELIANCE is in NIFTY
        // Total Market and NIFTY is an index, so the two rows differ on every
        // column the page can order by.
        let read = universe(&agreeing("sortcols"));
        for column in ["key", "symbol", "isin", "universe", "kind", "vendors", ""] {
            let html = instruments_html_from(&read, "", column, false, "", 0);
            assert!(
                html.contains("NSE-NIFTY") && html.contains("NSE-RELIANCE"),
                "sort={column:?} lost a row"
            );
            // Every named column has a header link. The empty column is the
            // default order and names no column, so it is checked separately
            // rather than through a short-circuit whose right side never runs.
            if !column.is_empty() {
                assert!(
                    html.contains(&format!("sort={column}")),
                    "sort={column:?} has no header link"
                );
            }
        }
        // An unrecognised column is the DEFAULT order, not an error and not an
        // empty page: a stale bookmark should still render.
        let stale = instruments_html_from(&read, "", "no-such-column", false, "", 0);
        assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
    }

    #[test]
    fn the_universe_pill_selects_from_the_whole_set_not_from_the_page() {
        let read = universe(&agreeing("pills"));

        // Counts are of the tracked set. Both rows are tracked: NIFTY is an
        // index, RELIANCE is a Total Market constituent.
        let all = instruments_html_from(&read, "", "", false, "", 0);
        assert!(all.contains("2 instruments total"));

        // Indices selects the index and drops the equity.
        let idx = instruments_html_from(&read, "", "", false, "idx", 0);
        assert!(idx.contains("NSE-NIFTY"), "the index survives");
        assert!(!idx.contains("NSE-RELIANCE"), "the equity is filtered out");

        // Total Market selects the equity and drops the index.
        let ntm = instruments_html_from(&read, "", "", false, "ntm", 0);
        assert!(ntm.contains("NSE-RELIANCE"));
        assert!(!ntm.contains("NSE-NIFTY"));

        // F&O selects BOTH, because both are F&O underlyings.
        //
        // `universe::of_instrument` gives an index `INDEX ∪ of_equity(symbol)`,
        // so NIFTY carries INDEX **and** FNO — it is an index *and* the
        // underlying of its options. The universes deliberately overlap; a
        // filter treating them as disjoint would drop NIFTY from the F&O view,
        // which is the one instrument that most needs to be there.
        let fno = instruments_html_from(&read, "", "", false, "fno", 0);
        assert!(
            fno.contains("NSE-RELIANCE"),
            "RELIANCE is an F&O underlying"
        );
        assert!(
            fno.contains("NSE-NIFTY"),
            "NIFTY is INDEX and FNO both — see universe::of_instrument"
        );

        // An unrecognised value selects EVERYTHING, so a stale bookmark renders
        // the page rather than an empty one.
        let stale = instruments_html_from(&read, "", "", false, "no-such-universe", 0);
        assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
    }

    #[test]
    fn the_escape_hatch_says_which_direction_it_goes() {
        let read = universe(&agreeing("hatch"));
        // Default: the link offers to widen.
        let tracked = instruments_html_from(&read, "", "", false, "", 0);
        assert!(tracked.contains("show every NSE listing"));
        assert!(tracked.contains("all=1"));
        // Widened: the SAME link offers to narrow again. Without this arm the
        // page can be widened and never returned from.
        let every = instruments_html_from(&read, "", "", true, "", 0);
        assert!(every.contains("show tracked only"));
        assert!(every.contains("all=0"));
    }

    #[test]
    fn the_page_filters_on_the_query_and_says_which_total_it_means() {
        let dir = agreeing("page");
        let all = instruments_html(&dir, "");
        assert!(all.contains("2 instruments total"));
        assert!(all.contains("NSE-NIFTY") && all.contains("NSE-RELIANCE"));

        let filtered = instruments_html(&dir, "reliance");
        assert!(filtered.contains("1 matched"), "case-insensitive match");
        assert!(filtered.contains("NSE-RELIANCE"));
        assert!(!filtered.contains("NSE-NIFTY"));

        let none = instruments_html(&dir, "NOTHINGLIKETHIS");
        assert!(none.contains("0 matched"));
        assert!(none.contains("<tbody></tbody>"));
    }

    #[test]
    fn a_searched_page_still_says_a_vendor_was_never_read() {
        // THE COLLAPSE `universe`'s OWN DOC SAYS SECTION 4 FORBIDS. The notes
        // were folded into the title only when the query was empty, so typing
        // anything made the page stop saying that a master was missing -- and
        // with PAGE_ROWS at 200 against thousands of instruments, searching is
        // the only way to reach most of them.
        let dir = masters(
            "searchnotes",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,RELIANCE,EQ,EQ,INE002A01018,,,NSE-RELIANCE\n"
            )),
            None,
        );
        for query in ["", "RELIANCE", "NOTHINGLIKETHIS"] {
            let html = instruments_html(&dir, query);
            assert!(
                html.contains("dhan: UNAVAILABLE"),
                "query {query:?} must not hide an unread vendor: {html}"
            );
            assert!(
                html.contains("DEGRADED"),
                "query {query:?} must not claim the read was clean"
            );
        }
        // A row from one vendor when the other was never read must not be
        // byte-identical to a genuine single-vendor listing; the banner is
        // what distinguishes them, and it is present above.
        assert!(instruments_html(&dir, "RELIANCE").contains("1 matched"));
    }

    #[test]
    fn an_unreadable_row_says_why_and_where_rather_than_only_how_many() {
        // The line numbers and reasons were collected and then read only for
        // `.len()`, so `104 unreadable` was the whole of what an operator was
        // ever told.
        // The fixture used to be `NIFTY 100` and `NIFTY 200`, chosen because a
        // space was not a legal Symbol and 104 rows of the real master were
        // exactly that shape. D-0147 made those rows READABLE -- an index name
        // is normalised to the exchange's canonical ticker -- so they are no
        // longer an example of anything unreadable. A period still is: the
        // allowlist is `A-Z 0-9 - _ &` and nothing else, and confining the
        // collapse to spaces is what keeps that true.
        let dir = masters(
            "unreadable",
            Some(&format!(
                "{GROWW_HEAD}\
                 NSE,CASH,,RELIANCE,EQ,EQ,INE002A01018,,,NSE-RELIANCE\n\
                 NSE,CASH,,NIFTY.100,IDX,,NIFTY,,\n\
                 NSE,CASH,,NIFTY.200,IDX,,NIFTY,,\n"
            )),
            None,
        );
        let (text, _) = report(&dir);
        assert!(text.contains("2 unreadable"), "{text}");
        assert!(
            text.contains("groww UNREADABLE · malformed instrument identifier ×2, first at line 3"),
            "the reason and the line, not a bare count: {text}"
        );
        assert!(instruments_html(&dir, "RELIANCE").contains("UNREADABLE"));
    }

    #[test]
    fn a_row_too_short_for_its_columns_is_unreadable_never_a_routine_decline() {
        // A truncated row used to default its missing fields to "", which the
        // gate read as a series it does not recognise -- so a genuine share
        // was dropped and reported as ordinary business with `0 unreadable`.
        // The RELIANCE row below names the instrument and then stops before
        // the series column.
        let dir = masters(
            "short",
            Some(&format!(
                "{GROWW_HEAD}\
                 NSE,CASH,,RELIANCE,EQ\n\
                 NSE,CASH,,CHOLAFIN,EQ,EQ,INE121A01024,,,NSE-CHOLAFIN\n"
            )),
            None,
        );
        let (text, _) = report(&dir);
        assert!(text.contains("groww: 1 kept"), "{text}");
        assert!(text.contains("1 unreadable"), "{text}");
        assert!(
            text.contains("row has 5 field(s); the columns this vendor needs run to 9"),
            "the shortfall is named: {text}"
        );
        assert!(
            !text.contains("not an equity listing"),
            "a truncated share is NOT a bond: {text}"
        );
    }

    #[test]
    fn the_swept_instruments_are_rendered_first() {
        let dir = agreeing("order");
        let html = instruments_html(&dir, "");
        let nifty = html.find("NSE-NIFTY").expect("present");
        let reliance = html.find("NSE-RELIANCE").expect("present");
        assert!(nifty < reliance, "a swept instrument leads the page");
    }

    /// Reads one HTTP response off a fresh connection.
    ///
    /// A blocking client on a blocking thread, deliberately: it needs nothing
    /// from `tokio` that this crate's feature set does not already have, and
    /// awaiting it yields the runtime to the server task under test.
    async fn get(addr: SocketAddr, path: &str) -> String {
        let request = format!("GET {path} HTTP/1.1\r\nHost: t\r\nConnection: close\r\n\r\n");
        tokio::task::spawn_blocking(move || {
            use std::io::Read as _;
            let mut s = std::net::TcpStream::connect(addr).expect("connect");
            s.write_all(request.as_bytes()).expect("write");
            let mut buf = String::new();
            s.read_to_string(&mut buf).expect("read");
            buf
        })
        .await
        .expect("the client thread must not panic")
    }

    /// A front end on disk, named after the test.
    ///
    /// EVERY SERVING TEST TAKES ONE OF THESE RATHER THAN THE REAL `web/build`.
    /// [`router`] reads the environment, so a test driving it answers `/nope`
    /// with the shell on a machine that has run a front-end build and with a
    /// `503` on one that has not — the same machine-dependence `run_in`'s
    /// doc comment records against `$HOME/.brutex/masters`, and an assertion
    /// written to survive both outcomes asserts nothing.
    fn front(name: &str) -> std::sync::Arc<assets::Assets> {
        let dir = crate::scratch::path(&format!("front-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let build = dir.join("build");
        std::fs::create_dir_all(build.join("_app")).expect("mkdir");
        for (path, body) in [
            (
                build.join("index.html"),
                "<!doctype html><title>shell</title>",
            ),
            (build.join("_app").join("app.js"), "export const app = 1;"),
            (dir.join("typeahead.js"), "export const typeahead = 1;"),
            // A DECOY, on purpose. A file on disk at the same path as a JSON
            // route is the one way a static handler can silently take a route
            // over, and the only way to prove it cannot is to put one there.
            (build.join("store.json"), "\"DECOY\""),
        ] {
            let mut f = std::fs::File::create(&path).expect("create");
            f.write_all(body.as_bytes()).expect("write");
        }
        std::sync::Arc::new(assets::Assets::new(&dir))
    }

    #[tokio::test]
    async fn the_server_answers_every_route_and_then_shuts_down_gracefully() {
        let dir = agreeing("serve");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");

        // The shutdown signal is a second listener rather than a channel: it
        // needs nothing this crate does not already depend on, and connecting
        // to it is an unambiguous "stop now".
        let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let stop_addr = stopper.local_addr().expect("addr");
        let served = tokio::spawn(serve(
            listener,
            router_serving(
                Loaded::new(Site::load(&dir, &store_root("serve"))),
                front("serve"),
            ),
            Box::pin(async move { stopper.accept().await.map(|_| ()) }),
        ));

        let health = get(addr, "/health").await;
        assert!(health.contains("200 OK"), "{health}");
        assert!(health.contains("merged: 2 instruments"), "{health}");

        let page = get(addr, "/instruments?q=NIFTY").await;
        assert!(page.contains("200 OK"));
        assert!(page.contains("NSE-NIFTY"));
        assert!(
            page.contains("groww: 2 kept"),
            "a searched page still carries the notes: {page}"
        );

        // `/dashboard` IS THE DASHBOARD, not a second copy of the instruments
        // page. It carries the nav, so every other page is one click away, and
        // its figures are counters rather than a row listing.
        //
        // It answered at `/` until D-0064 moved it. `/` is the front end's
        // front door and two applications on one URL is the defect
        // `web/vite.config.js` already records against `/audit`: a click
        // renders one page and a reload renders another.
        let root = get(addr, "/dashboard").await;
        assert!(root.contains("200 OK"));
        assert!(root.contains("nav class"), "the dashboard carries the nav");
        assert!(
            root.contains("NIFTY Total Market"),
            "the dashboard names the universes it counts: {root}"
        );
        assert!(
            !root.contains("<tbody>"),
            "the dashboard counts; it does not list rows"
        );
        // THE NAV'S PREMISE CHANGED WITH D-0038, AND ONLY HALF OF IT.
        //
        // This assertion used to be `root.contains("lnk off")` with `Ingest`
        // beside it, because /pull and /store were rendered disabled. They now
        // answer, so `Ingest` and `Store` are real links — asserting they are
        // still greyed out would be asserting the opposite of what shipped.
        //
        // The RULE the old assertion protected is unchanged and is still
        // checked: a page that does not exist is shown disabled rather than
        // hidden or linked. `/runs` is that page — there is no sweep yet — so
        // `lnk off` must still be here, and it must be Runs that carries it.
        assert!(
            root.contains("<a class=\"lnk\" href=\"/pull\">Ingest</a>"),
            "Ingest is a real link now: {root}"
        );
        assert!(
            root.contains("<a class=\"lnk\" href=\"/store\">Store</a>"),
            "Store is a real link now: {root}"
        );
        assert!(
            root.contains("<a class=\"lnk\" href=\"/audit\">Audit</a>"),
            "and Audit is a real link too: {root}"
        );
        assert!(
            root.contains("<span class=\"lnk off\" title=\"not built yet\">Runs</span>"),
            "and Runs is still shown, disabled, because there is no sweep: {root}"
        );
        assert_eq!(
            root.matches("lnk off").count(),
            1,
            "exactly one page is still unbuilt"
        );

        // Both new pages answer, and the store page answers even though the
        // store root is empty — an absent manifest is the ordinary state
        // before the first ingest and must never be a 500.
        let ingest_page = get(addr, "/pull").await;
        assert!(ingest_page.contains("200 OK"), "{ingest_page}");
        assert!(ingest_page.contains("action=\"/pull/spot\""));
        assert!(ingest_page.contains("action=\"/pull/fno\""));
        let store_page = get(addr, "/store").await;
        // THE STATUS LINE, NOT THE WHOLE RESPONSE. `contains("500")` over the
        // body matched the scratch directory's name, which carries this
        // process's id — so the assertion failed whenever that id happened to
        // hold those three digits, and passed the rest of the time. A test
        // that depends on a process id is a test that fails at random and
        // teaches everyone to rerun rather than to read.
        let status = store_page.lines().next().unwrap_or_default();
        assert!(status.contains("200 OK"), "{store_page}");
        assert!(
            !status.contains("500"),
            "an absent manifest is the ordinary state before the first ingest \
             and must never be a 500: {store_page}"
        );
        assert!(store_page.contains("UNAVAILABLE"), "{store_page}");

        // AND /audit ANSWERS OVER AN EMPTY JOURNAL, for the same reason: no
        // pull has been run against this store root, and a page that 500s on a
        // fresh install is a page that is broken exactly when it is first
        // opened.
        let audit_page = get(addr, "/audit?page=3").await;
        let status = audit_page.lines().next().unwrap_or_default();
        assert!(status.contains("200 OK"), "{audit_page}");
        assert!(audit_page.contains("nothing recorded yet"), "{audit_page}");
        assert!(
            audit_page.contains("It is a file on disk, not memory"),
            "and it says where the history lives: {audit_page}"
        );

        // A GET ON A POST ROUTE STARTS NOTHING. A crawler follows links and a
        // browser refetches on back; either would otherwise begin an ingest.
        for path in ["/pull/spot", "/pull/fno"] {
            let crawled = get(addr, path).await;
            assert!(
                crawled.contains("405"),
                "GET {path} must not be a route at all: {crawled}"
            );
            assert!(
                !crawled.contains("NOT STARTED"),
                "GET {path} must not even reach the parser: {crawled}"
            );
        }

        // AN UNMATCHED PATH IS THE FRONT END'S, NOT A 404. `/nope` is a route
        // the browser may own — the client router decides — so it gets the
        // shell. A path that LOOKS like an asset does not: see
        // `the_static_handler_is_last_and_never_shadows_a_route`.
        let missing = get(addr, "/nope").await;
        assert!(missing.contains("200 OK"), "{missing}");
        assert!(missing.contains("<title>shell</title>"), "{missing}");

        let _ = tokio::net::TcpStream::connect(stop_addr).await;
        let outcome = served.await.expect("the serve task must not panic");
        assert!(outcome.is_ok(), "a graceful shutdown is not a failure");
    }

    /// Every rule in the routing order, over a real socket.
    ///
    /// The unit tests in [`crate::assets`] prove what the handler does with a
    /// path. This one proves WHERE it sits: that a registered route still wins
    /// when a file of the same name is sitting on disk, that a `POST` route is
    /// still a `POST` route, and that the handler is reached at all.
    #[tokio::test]
    async fn the_static_handler_is_last_and_never_shadows_a_route() {
        let dir = agreeing("static");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let stop_addr = stopper.local_addr().expect("addr");
        let served = tokio::spawn(serve(
            listener,
            router_serving(
                Loaded::new(Site::load(&dir, &store_root("static"))),
                front("static"),
            ),
            Box::pin(async move { stopper.accept().await.map(|_| ()) }),
        ));

        // 1. A JSON ROUTE WINS OVER A FILE OF THE SAME NAME. `front` puts a
        //    `store.json` in the build directory precisely so this can fail.
        let json = get(addr, "/store.json?feed=groww").await;
        assert!(json.contains("200 OK"), "{json}");
        assert!(
            !json.contains("DECOY"),
            "the route wins, not the file: {json}"
        );

        // 1b. AND `/universes.json` IS ROUTED, not merely written. A handler
        //     that answers correctly and is never reachable is the exact shape
        //     of the defect D-0120 exists to close — `/ingest` could not ask a
        //     question no route served. This drives the real router.
        let reach = get(addr, "/universes.json?feed=groww").await;
        assert!(reach.contains("200 OK"), "{reach}");
        assert!(
            reach.contains(r#""feed":"groww""#) && reach.contains(r#""target":"n50""#),
            "the four tiers reach the wire through the router: {reach}"
        );
        let refused = get(addr, "/universes.json?feed=nobody").await;
        assert!(
            refused.contains("400 Bad Request"),
            "and an unknown feed is refused rather than answered as Dhan: {refused}"
        );

        // 2. A SERVER-RENDERED PAGE WINS TOO.
        let dashboard = get(addr, "/dashboard").await;
        assert!(dashboard.contains("nav class"), "{dashboard}");

        // 3. `POST /pull/spot` IS STILL A POST ROUTE. If the fallback had
        //    swallowed it, this would answer with the shell and start nothing —
        //    which reads exactly like a refusal and is not one.
        let posted = post(
            addr,
            "/pull/spot",
            "target=nifty&from=2024-01-01&to=2024-01-02",
        )
        .await;
        assert!(
            !posted.contains("<title>shell</title>"),
            "the ingest route answers, not the front end: {posted}"
        );

        // 4. A REAL ASSET IS SERVED, WITH ITS TYPE.
        let asset = get(addr, "/_app/app.js").await;
        assert!(asset.contains("200 OK"), "{asset}");
        assert!(asset.contains("text/javascript"), "{asset}");
        assert!(asset.contains("export const app"), "{asset}");

        // 5. THE TYPE-AHEAD COMES OFF DISK NOW, NOT OUT OF THE BINARY.
        let script = get(addr, "/typeahead.js").await;
        assert!(script.contains("200 OK"), "{script}");
        assert!(script.contains("text/javascript"), "{script}");
        assert!(script.contains("export const typeahead"), "{script}");

        // 6. A MISSING ASSET IS A 404 AND NEVER HTML.
        let missing = get(addr, "/assets/missing.js").await;
        assert!(missing.contains("404"), "{missing}");
        assert!(
            !missing.contains("<!doctype"),
            "a missing script must not answer with a page: {missing}"
        );

        // 7. A CLIENT ROUTE GETS THE SHELL.
        let db = get(addr, "/db").await;
        assert!(db.contains("200 OK"), "{db}");
        assert!(db.contains("<title>shell</title>"), "{db}");

        // 8. AND A TRAVERSAL IS REFUSED OVER THE WIRE, not only in a unit test.
        let escape = get(addr, "/%2e%2e/Cargo.toml").await;
        assert!(escape.contains("400"), "{escape}");
        assert!(!escape.contains("[package]"), "nothing leaked: {escape}");

        let _ = tokio::net::TcpStream::connect(stop_addr).await;
        served
            .await
            .expect("task")
            .expect("a graceful shutdown is not a failure");
    }

    /// A signal that has already fired.
    fn fired() -> Shutdown {
        Box::pin(std::future::ready(Ok(())))
    }

    /// One command line, owned.
    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| (*s).to_owned()).collect()
    }

    /// Sends one form POST and reads the whole response.
    async fn post(addr: SocketAddr, path: &str, form: &str) -> String {
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: t\r\n\
             Content-Type: application/x-www-form-urlencoded\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{form}",
            form.len()
        );
        tokio::task::spawn_blocking(move || {
            use std::io::Read as _;
            let mut s = std::net::TcpStream::connect(addr).expect("connect");
            s.write_all(request.as_bytes()).expect("write");
            let mut buf = String::new();
            s.read_to_string(&mut buf).expect("read");
            buf
        })
        .await
        .expect("the client thread must not panic")
    }

    /// Runs `body` against a live server over the agreeing fixture.
    async fn with_server<F, Fut>(name: &str, body: F)
    where
        F: FnOnce(SocketAddr) -> Fut,
        Fut: Future<Output = ()>,
    {
        let dir = agreeing(name);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let stop_addr = stopper.local_addr().expect("addr");
        let served = tokio::spawn(serve(
            listener,
            router_serving(Loaded::new(site(name, &dir)), front(name)),
            Box::pin(async move { stopper.accept().await.map(|_| ()) }),
        ));
        body(addr).await;
        let _ = tokio::net::TcpStream::connect(stop_addr).await;
        served
            .await
            .expect("task")
            .expect("a graceful shutdown is not a failure");
    }

    /// One form POST carrying whatever a browser would have stamped on it.
    ///
    /// [`post`] sends no fetch metadata and no `Origin`, which is what `curl`
    /// does and is the case `same_origin_writes_only` deliberately lets
    /// through. Proving the refusal needs a request that says where it came
    /// from, so this one lets the caller say it. `extra` carries its own
    /// trailing CRLF, because a header list that is sometimes empty cannot own
    /// a separator.
    async fn post_as(addr: SocketAddr, path: &str, form: &str, extra: &str) -> String {
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: t\r\n{extra}\
             Content-Type: application/x-www-form-urlencoded\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{form}",
            form.len()
        );
        tokio::task::spawn_blocking(move || {
            use std::io::Read as _;
            let mut s = std::net::TcpStream::connect(addr).expect("connect");
            s.write_all(request.as_bytes()).expect("write");
            let mut buf = String::new();
            s.read_to_string(&mut buf).expect("read");
            buf
        })
        .await
        .expect("the client thread must not panic")
    }

    /// **A WRITE FROM ANOTHER ORIGIN IS REFUSED, OVER A REAL SOCKET.**
    ///
    /// The unit test below proves what [`cross_origin_refusal`] decides. This
    /// one proves the layer is actually INSTALLED — a decision function nothing
    /// calls is the shape of defect this repository keeps finding, and the
    /// registration in [`router_serving`] is the only thing that makes it a
    /// guard rather than an opinion.
    ///
    /// `/pull/spot` is the hostile target because a refused request does
    /// nothing by definition. The allowed request goes to `/ingest/queue`,
    /// which queues nothing by design (`ingest::queue`), so the positive half
    /// of this test starts no ingest and touches no vendor.
    #[tokio::test]
    async fn a_cross_origin_write_is_refused_and_this_page_s_own_is_not() {
        with_server("csrf", |addr| async move {
            let form = "target=nifty&from=2024-01-01&to=2024-01-02";

            let hostile = post_as(addr, "/pull/spot", form, "Sec-Fetch-Site: cross-site\r\n").await;
            assert!(hostile.contains("403 Forbidden"), "{hostile}");
            assert!(hostile.contains("REFUSED"), "{hostile}");
            assert!(
                hostile.contains("Sec-Fetch-Site says this one came from somewhere else"),
                "the refusal names the header that decided: {hostile}"
            );
            assert!(
                !hostile.contains("NOT STARTED"),
                "it must not reach the ingest parser at all: {hostile}"
            );

            // AND THE OLDER SIGNAL, for a client that sends no fetch metadata.
            let elsewhere =
                post_as(addr, "/pull/spot", form, "Origin: http://evil.example\r\n").await;
            assert!(elsewhere.contains("403 Forbidden"), "{elsewhere}");
            assert!(elsewhere.contains("evil.example"), "{elsewhere}");

            // THE THREE THAT MUST STILL WORK. `Host` is `t`, so `http://t` IS
            // this origin; a browser posting this server's own form says
            // `same-origin`; and a client that says neither is `curl`.
            for extra in [
                "Sec-Fetch-Site: same-origin\r\n",
                "Origin: http://t\r\n",
                "",
            ] {
                let ours = post_as(addr, "/ingest/queue", form, extra).await;
                assert!(
                    !ours.contains("403 Forbidden"),
                    "[{extra}] this server's own page still posts: {ours}"
                );
            }

            // AND A READ IS NEVER REFUSED, whoever asked for it. A page that
            // could not be fetched cross-origin would break nothing an attacker
            // can see and everything a link can.
            let read = get(addr, "/store.json?feed=groww").await;
            assert!(!read.contains("403 Forbidden"), "{read}");
        })
        .await;
    }

    /// **THE SAME-ORIGIN RULE, ARM BY ARM, WITHOUT A SOCKET.**
    ///
    /// Every branch of [`cross_origin_refusal`] is here, including the two that
    /// deliberately PASS: a request with no fetch metadata and no `Origin` at
    /// all, which is `curl` and is every other socket test in this file. That
    /// hole is documented on the function and it is asserted here, so it cannot
    /// be closed or widened without this test saying so.
    #[test]
    fn only_a_same_origin_write_passes_and_a_read_always_does() {
        let head = |pairs: &[(&str, &str)]| {
            let mut headers = axum::http::HeaderMap::new();
            for &(name, value) in pairs {
                headers.insert(
                    axum::http::HeaderName::from_bytes(name.as_bytes()).expect("a header name"),
                    axum::http::HeaderValue::from_str(value).expect("a header value"),
                );
            }
            headers
        };
        let post = axum::http::Method::POST;

        // A READ IS NEVER REFUSED, whatever it says about itself.
        for method in [axum::http::Method::GET, axum::http::Method::HEAD] {
            assert!(
                cross_origin_refusal(&method, &head(&[("sec-fetch-site", "cross-site")])).is_none(),
                "{method} changes nothing and must not be gated"
            );
        }

        // THE BROWSER'S OWN WORD. One value passes and every other refuses,
        // including a token this build has never heard of.
        assert!(
            cross_origin_refusal(&post, &head(&[("sec-fetch-site", "same-origin")])).is_none(),
            "this server's own form is the only thing this check exists to admit"
        );
        for hostile in ["cross-site", "same-site", "none", "nonsense"] {
            let why = cross_origin_refusal(&post, &head(&[("sec-fetch-site", hostile)]))
                .expect("a write that is not from this origin is refused");
            assert!(why.contains("Sec-Fetch-Site"), "{why}");
            assert!(why.contains(hostile), "it names what arrived: {why}");
            assert!(why.contains("REFUSED"), "{why}");
        }

        // NO METADATA AT ALL PASSES, and that is the documented cost: curl
        // sends none, and neither does `post` above.
        assert!(cross_origin_refusal(&post, &head(&[])).is_none());
        assert!(cross_origin_refusal(&post, &head(&[("host", "t")])).is_none());

        // ORIGIN AGAINST HOST IS THE FALLBACK, on authority alone.
        for (host, origin) in [
            ("t", "http://t"),
            ("127.0.0.1:8080", "http://127.0.0.1:8080"),
        ] {
            assert!(
                cross_origin_refusal(&post, &head(&[("host", host), ("origin", origin)])).is_none(),
                "{origin} IS {host}"
            );
        }
        for hostile in ["http://evil.example", "http://t.evil.example", "null", ""] {
            let why = cross_origin_refusal(&post, &head(&[("host", "t"), ("origin", hostile)]))
                .expect("an Origin that is not this host is refused");
            assert!(why.contains("Origin"), "{why}");
        }
        assert!(
            cross_origin_refusal(&post, &head(&[("origin", "http://t")])).is_some(),
            "an Origin with no Host to compare it against decides nothing, and \
             deciding nothing is a refusal here"
        );

        // A HEADER VALUE THAT IS NOT TEXT IS NO VALUE. `to_str` refusing says
        // nothing about where the request came from, so it falls to `Origin`
        // rather than passing.
        let mut opaque = head(&[("host", "t"), ("origin", "http://evil.example")]);
        opaque.insert(
            axum::http::HeaderName::from_static("sec-fetch-site"),
            axum::http::HeaderValue::from_bytes(&[0xff_u8]).expect("an opaque header value"),
        );
        let why = cross_origin_refusal(&post, &opaque)
            .expect("an unreadable fetch-site header is not a pass");
        assert!(
            why.contains("Origin"),
            "the unreadable header is skipped and Origin decides: {why}"
        );
    }

    /// **AN UNKNOWN FEED IS REFUSED BY NAME, ON BOTH STORE SURFACES.**
    ///
    /// `/store.json` and `/store` both answered `.unwrap_or(Vendor::Dhan)`, so
    /// `?feed=growww` was served Dhan's counters under HTTP 200 with no field
    /// on the wire saying so. `/universes.json` has refused the same input
    /// since D-0120 and its own doc block said the other two did not copy it;
    /// they do now, through one sentence.
    ///
    /// The last two assertions are the ones that keep the fix honest: a feed
    /// this build DOES read is not refused, and an ABSENT feed still means Dhan
    /// — `ingest::parse_vendor` answers the empty string itself, so refusing
    /// `None` cannot have taken the default away.
    #[tokio::test]
    async fn an_unknown_feed_is_refused_rather_than_answered_as_dhan() {
        with_server("unknownfeed", |addr| async move {
            let json = get(addr, "/store.json?feed=growww").await;
            assert!(json.contains("400 Bad Request"), "{json}");
            assert!(json.contains("reads no feed called that"), "{json}");
            assert!(
                json.contains(r#""feed":"growww""#),
                "it names what arrived, not what it guessed: {json}"
            );
            assert!(
                !json.contains(r#""instrument":"#),
                "and no vendor's rows are on it: {json}"
            );

            let page = get(addr, "/store?feed=growww").await;
            assert!(page.contains("400 Bad Request"), "{page}");
            assert!(page.contains("REFUSED"), "{page}");
            assert!(page.contains("reads no feed called that"), "{page}");
            assert!(page.contains("growww"), "the receipt names it: {page}");

            for asked in ["/store.json?feed=groww", "/store.json", "/store"] {
                let fine = get(addr, asked).await;
                assert!(
                    !fine.contains("400 Bad Request") && !fine.contains("reads no feed called"),
                    "{asked} names a feed this build reads: {fine}"
                );
            }
        })
        .await;
    }

    #[tokio::test]
    async fn a_malformed_or_backwards_window_is_refused_with_the_reason_named() {
        with_server("badwindow", |addr| async move {
            // A date that is not a date. The refusal names the FIELD and what
            // arrived, so an operator is not sent to guess which of four it
            // meant.
            let out = post(
                addr,
                "/pull/spot",
                "target=swept&from=08/01/2022&to=2022-02-08",
            )
            .await;
            assert!(out.contains("400 Bad Request"), "{out}");
            assert!(out.contains("REFUSED"), "{out}");
            assert!(out.contains("08/01/2022"), "it names what arrived: {out}");
            assert!(out.contains("YYYY-MM-DD"), "and what it wanted: {out}");
            assert!(
                out.contains("nothing was written"),
                "and that nothing happened: {out}"
            );

            // A day that does not exist reaches the calendar and is refused by
            // it, not by a second opinion here.
            let leap = post(
                addr,
                "/pull/spot",
                "target=swept&from=2023-02-29&to=2023-03-01",
            )
            .await;
            assert!(leap.contains("400 Bad Request"), "{leap}");
            assert!(leap.contains("2023-02-29"), "{leap}");

            // Backwards is refused, never silently swapped.
            let back = post(
                addr,
                "/pull/spot",
                "target=swept&from=2022-02-08&to=2022-01-08",
            )
            .await;
            assert!(back.contains("400 Bad Request"), "{back}");
            assert!(back.contains("runs backwards"), "{back}");
            assert!(
                back.contains("2022-02-08") && back.contains("2022-01-08"),
                "both ends are named: {back}"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn a_valid_window_is_echoed_with_the_wire_date_and_still_starts_nothing() {
        with_server("goodwindow", |addr| async move {
            // A RANGE ACROSS A YEAR BOUNDARY. Four days, both ends included,
            // and the wire `toDate` is the day after the last one.
            let over = post(
                addr,
                "/pull/spot",
                "target=indices&from=2021-12-30&to=2022-01-02",
            )
            .await;
            assert!(
                over.contains("503 Service Unavailable"),
                "valid, and still not started: {over}"
            );
            assert!(over.contains("NOT STARTED"), "{over}");
            assert!(over.contains("Reference indices"), "{over}");
            assert!(
                over.contains("2021-12-30..=2022-01-02"),
                "the range is stated in Window's own inclusive notation: {over}"
            );
            assert!(
                over.contains("<td>4</td>"),
                "30, 31, 1, 2 — four days: {over}"
            );
            assert!(
                over.contains("2022-01-03"),
                "the non-inclusive toDate is the day AFTER, and is shown: {over}"
            );
            assert!(
                over.contains("not inclusive"),
                "and the correction is stated, not performed in silence: {over}"
            );
            assert!(
                over.contains("no vendor was contacted") || over.contains("no vendor is contacted"),
                "and it says nothing ran: {over}"
            );

            // A SINGLE-DAY RANGE, which is the commonest resume shape.
            let one = post(
                addr,
                "/pull/spot",
                "target=equities&from=2022-01-08&to=2022-01-08",
            )
            .await;
            assert!(one.contains("503"), "{one}");
            assert!(one.contains("<td>1</td>"), "one calendar day: {one}");
            assert!(one.contains("2022-01-09"), "and the wire date: {one}");

            // THE RECEIPT NAMES THE TARGET THAT WAS ASKED FOR, not whichever
            // one the lookup happened to land on. Each of the three is posted
            // and each comes back as itself; `cargo mutants` found that with
            // only one target exercised, the `==` selecting its member count
            // could be a `!=` and nothing would notice.
            for (slug, label) in [
                ("swept", "Swept indices"),
                ("indices", "Reference indices"),
                ("equities", "NIFTY Total Market equities"),
            ] {
                let form = format!("target={slug}&from=2022-01-08&to=2022-01-08");
                let out = post(addr, "/pull/spot", &form).await;
                assert!(out.contains(label), "{slug} must echo as {label}: {out}");
            }
            // The fixture has one index and one Total Market constituent, and
            // the receipt states each target's own population — so the three
            // answers are not interchangeable.
            let swept = post(
                addr,
                "/pull/spot",
                "target=swept&from=2022-01-08&to=2022-01-08",
            )
            .await;
            assert!(
                swept.contains("<th>Instruments covered</th><td>1 in the merged universe</td>"),
                "the swept count is the swept count: {swept}"
            );
            // AND THE SECOND NUMBER, WHICH IS THE ONE THE RUN OBEYS. The line
            // above is the union of what both masters name; this run reaches
            // one feed, and D-0120 put that on the receipt beside it.
            assert!(
                swept.contains(
                    "<th>This feed can name</th><td>1 — every name this target holds, by Dhan id</td>"
                ),
                "the receipt says what THIS feed reaches, not what the universe holds: {swept}"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn the_fno_form_cannot_request_a_live_contract_over_http_either() {
        with_server("livecontract", |addr| async move {
            // 9998-12-31 is live under any clock this build can run on, and
            // 2020-01-30 has expired under every one of them. Neither depends
            // on when the test runs.
            let live = post(
                addr,
                "/pull/fno",
                "underlying=NIFTY&series=opt&expiry=9998-12-31&from=9998-12-01&to=9998-12-31",
            )
            .await;
            assert!(live.contains("400 Bad Request"), "{live}");
            assert!(
                live.contains("LIVE CONTRACT IS NEVER STORED"),
                "the rule is named, loudly: {live}"
            );
            assert!(live.contains("9998-12-31"), "{live}");

            let expired = post(
                addr,
                "/pull/fno",
                "underlying=nifty&series=fut&expiry=2020-01-30&from=2020-01-01&to=2020-01-30\
                 &vendor=truedata",
            )
            .await;
            assert!(expired.contains("503"), "expired is acceptable: {expired}");
            assert!(expired.contains("NIFTY"), "canonicalised: {expired}");
            assert!(expired.contains("2020-01-31"), "the wire date: {expired}");

            // A window past the expiry asks for bars that cannot exist.
            let past = post(
                addr,
                "/pull/fno",
                "underlying=NIFTY&series=fut&expiry=2020-01-30&from=2020-01-01&to=2020-02-05",
            )
            .await;
            assert!(past.contains("400"), "{past}");
            assert!(past.contains("no bars there"), "{past}");

            // An underlying with no derivative on it.
            let spot_only = post(
                addr,
                "/pull/fno",
                "underlying=RAJESHEXPO&series=fut&expiry=2020-01-30&from=2020-01-01&to=2020-01-30",
            )
            .await;
            assert!(spot_only.contains("400"), "{spot_only}");
            assert!(spot_only.contains("no F&amp;O series"), "{spot_only}");
        })
        .await;
    }

    #[tokio::test]
    async fn a_body_larger_than_this_server_reads_is_refused_and_never_parsed() {
        // `ingest.rs` says every parser it holds works over "a form body whose
        // length the server caps". It was true, and it was true by accident:
        // the cap was axum's 2 MiB default and no line here named it. A
        // dependency's default is a bound this repository does not own, so the
        // boundary now states the number and this is the test that proves the
        // number is the one in force.
        with_server("bodylimit", |addr| async move {
            // Just inside: a legitimate body still answers on its own merits.
            let ok = post(
                addr,
                "/pull/spot",
                "target=swept&from=2024-01-01&to=2024-01-31",
            )
            .await;
            assert!(
                !ok.contains("413"),
                "an ordinary form is nowhere near the bound: {ok}"
            );

            // Just outside: the field is padded past MAX_FORM_BYTES. It is
            // refused for its SIZE, before any parser sees it -- so the reply
            // carries neither the accepted page nor a named field refusal.
            let huge = format!(
                "target=swept&from=2024-01-01&to=2024-01-31&pad={}",
                "x".repeat(MAX_FORM_BYTES + 1)
            );
            let refused = post(addr, "/pull/spot", &huge).await;
            assert!(
                refused.contains("413"),
                "a body past the bound is refused loudly: {refused}"
            );
            assert!(
                !refused.contains("REFUSED ·"),
                "and it never reached the form parser: {refused}"
            );
        })
        .await;
    }

    /// Two defects that SHIPPED, and the assertions that keep them out.
    ///
    /// **Neither was found by reading the code.** Both came out of an
    /// adversarial sweep that drove the page in a real browser, and both were
    /// invisible from the Rust side: the code was right, the HTML it produced
    /// was not. That is the general shape of a renderer bug and the reason this
    /// test asserts about *markup* rather than about functions.
    #[test]
    fn the_pickers_do_not_block_submission_and_do_not_close_on_their_own_chrome() {
        let dir = agreeing("pickerhtml");
        let site = site("pickerhtml", &dir);
        let html = pull_html(&site, day(2026, 8, 7));

        // 1. `required` ON A ZERO-SIZED RADIO INSIDE A `display:none` POPOVER.
        //
        // Every day/month/year radio is `width:0;height:0;opacity:0` inside a
        // `.cal` that is hidden until the popover opens. A browser that finds
        // an unsatisfied `required` control tries to FOCUS it, to anchor the
        // validation bubble somewhere. It cannot focus a zero-sized control in
        // a hidden subtree — so Chrome logs
        //
        //     An invalid form control with name='to_d' is not focusable.
        //
        // to a console nobody is watching, REFUSES TO SUBMIT, and shows the
        // operator nothing. The button looked dead. Reproduced live:
        // `form.reportValidity()` returned false with that message and no
        // visible UI. Exactly the silent failure `CLAUDE.md` §4 bans.
        //
        // Presence is the server's job and always was:
        // `ingest::parse_day_field` returns `Refusal::FieldMissing`, which the
        // result page renders with the field named.
        for group in ["from_y", "from_m", "from_d", "to_y", "to_m", "to_d"] {
            let needle = format!("name=\"{group}\"");
            for (i, _) in html.match_indices(&needle) {
                let end = html[i..].find('>').map_or(html.len(), |e| i + e);
                assert!(
                    !html[i..end].contains("required"),
                    "{group} is `required` and unfocusable: the browser cannot \
                     report the error, so it blocks submission in silence"
                );
            }
        }

        // 2. THE PICKER WRAPPED IN A `<label>` WITH NO `for`.
        //
        // HTML makes such a label's control the first labelable descendant —
        // the popover checkbox — and a label forwards every click that did not
        // land on *interactive content*. The weekday headers, the explanatory
        // paragraph, the grid gutters and the greyed impossible days are none
        // of those, so clicking any of them CLOSED THE CALENDAR. The sharpest
        // case: an impossible day carries `pointer-events:none`, so the click
        // fell through to the grid and shut the picker — the one interaction
        // that should most obviously do nothing.
        //
        // `render::field_unlabelled` emits a `<div>`; the picker carries its
        // own explicit `<label for="o-…">`, so no association is lost.
        for (i, _) in html.match_indices("class=\"pick ") {
            let before = &html[..i];
            let opened = before.rfind("<label").map_or(0, |p| p + 1);
            let closed = before.rfind("</label>").map_or(0, |p| p + 1);
            assert!(
                closed >= opened,
                "a picker sits inside an open <label>: clicking the calendar's \
                 own chrome toggles that label's control and shuts it"
            );
        }
    }

    #[test]
    fn the_ingest_page_counts_its_targets_and_never_draws_a_bar_over_nothing() {
        let dir = agreeing("pullpage");
        let site = site("pullpage", &dir);
        let html = pull_html(&site, day(2026, 8, 7));

        // Two forms, not one form with a dropdown.
        assert_eq!(html.matches("<form class=\"pull\"").count(), 2);
        assert!(html.contains("action=\"/pull/spot\">"));
        assert!(html.contains("action=\"/pull/fno\""));
        assert!(
            html.contains("method=\"post\""),
            "starting a pull is a POST"
        );
        assert!(!html.contains("method=\"get\""), "and never a GET: {html}");

        // EVERY DATE FIELD IS A CLICKABLE MONTH GRID, and the assertion changed
        // with the code rather than being deleted — twice now.
        //
        // `type=date` was first and renders in the BROWSER'S LOCALE — macOS
        // showed `dd/mm/yyyy`, and `01/07/2025` is 1 July here and 7 January in
        // half the world, an ambiguity this codebase has already been bitten by
        // reading GDFL. A text box with `placeholder="YYYY-MM-DD"` was second
        // and fixed the ambiguity by making the operator type, which is not a
        // fix. Third is `api::calendar`: a grid you click, rendered here.
        assert_eq!(
            html.matches("class=\"readout\"").count(),
            5,
            "two on the spot form, three on the F&O form, all clickable"
        );
        assert_eq!(
            html.matches("placeholder=\"YYYY-MM-DD\"").count(),
            0,
            "nobody types a date on this page any more"
        );
        // THE IDS MUST BE UNIQUE. Both forms have a field called `from`, and an
        // id is document-wide — so a collision would silently point the spot
        // picker's label at the F&O picker's checkbox and clicking one would
        // open the other. Counted rather than eyeballed.
        {
            let mut ids: Vec<&str> = html
                .match_indices("id=\"o-")
                .filter_map(|(i, _)| html[i + 4..].split('"').next())
                .collect();
            let total = ids.len();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), total, "two pickers share an id: {ids:?}");
            assert_eq!(total, 5, "one popover latch per date field");
        }

        assert_eq!(
            html.matches("type=\"date\"").count(),
            0,
            "no locale-rendered date input survives"
        );
        assert!(
            html.contains("2026-08-07"),
            "the latest spot date is still stated"
        );
        assert!(
            html.contains("2026-08-06"),
            "and the expiry bound is still STATED — `max` is inert on a text
             input, so the guard that matters is `parse_fno`, which refuses a
             live contract whatever the form sends"
        );

        // The counts are the real ones from the loaded universe: the fixture
        // has NIFTY (an index, and swept) and RELIANCE (Total Market).
        assert!(html.contains("1 instrument(s)"), "{html}");
        assert!(html.contains("Swept indices"), "{html}");
        assert!(html.contains("NIFTY Total Market equities"), "{html}");

        // NOT A FABRICATED PROGRESS BAR, AND NOT A FABRICATED ABSENCE EITHER.
        //
        // CHANGED, AND WHY. This block used to assert the page said
        // "CAPTURE UNAVAILABLE" and named `pull::fetch` and `pull::rate` as the
        // MISSING modules. Both files exist — 521 and 822 lines — and the
        // local-archive path writes bars through them, so the old assertions
        // pinned a sentence that had become false. What is actually absent is
        // an HTTP implementor of `BarSource`, and that is what is asserted now:
        // the claim is checkable, so the test can hold it to being true.
        assert!(html.contains("ARCHIVE ONLY"), "{html}");
        assert!(
            html.contains("THE LOCAL-ARCHIVE PATH RUNS. THE HTTP PATH IS BUILT BUT NOT YET WIRED"),
            "{html}"
        );
        assert!(
            !html.contains("there is no pull::fetch"),
            "the stale claim must not come back: {html}"
        );
        assert!(
            html.contains("pull::http::HttpSource"),
            "and the client that now exists is named, so an operator knows \
             where the missing join is: {html}"
        );
        assert!(
            !html.contains("<div class=\"cv\">0</div>"),
            "a zero would claim a measurement nobody took: {html}"
        );
        assert!(html.contains("<div class=\"cv\">—</div>"), "{html}");

        // Every drop reason the filter counts is on the page, by its own label.
        for reason in [
            "before the session open",
            "at or after the session close",
            "before the requested window",
            "after the requested window",
        ] {
            assert!(html.contains(reason), "{reason} must be shown: {html}");
        }
        // And the session bounds it filters on.
        assert!(html.contains("09:15") && html.contains("15:30"), "{html}");
        assert!(
            html.contains("375"),
            "375 bars in a regular session: {html}"
        );
    }

    #[test]
    fn the_store_page_renders_an_absent_manifest_rather_than_failing() {
        // The ordinary state of a fresh install: no manifest file at all. The
        // page must say what is missing and where it looked, and it must not
        // be an error — this is the page you look at to find out why.
        let dir = agreeing("storeabsent");
        let site = site("storeabsent", &dir);
        let html = store_ok(&site, day(2026, 8, 7), 0, "show=gaps");

        assert!(html.starts_with("<!doctype html>"));
        assert!(html.ends_with("</html>"));
        assert!(html.contains("UNAVAILABLE"), "{html}");
        assert!(html.contains("groww") && html.contains("dhan"), "{html}");
        assert!(html.contains("no manifest at"), "{html}");
        // The path that was looked at, named on the page. Asserted against the
        // scratch directory's own name rather than a literal prefix, because
        // `crate::scratch::path` owns the shape and a second spelling of it
        // here would be a test pinned to a naming rule it does not own.
        let looked_at = site
            .censuses
            .first()
            .expect("groww")
            .path
            .display()
            .to_string();
        assert!(html.contains(&looked_at), "the path: {html}");
        assert!(looked_at.contains("store-storeabsent"), "{looked_at}");
        assert!(html.contains("DEGRADED"), "and the badge says so: {html}");
        // Counters are dashes, never zeros: "nothing ingested" and "the counter
        // says zero" are different claims.
        assert!(html.contains("<div class=\"cv\">—</div>"), "{html}");
        // The grid still shows the instruments, with every cell a miss.
        assert!(html.contains("NSE-INDEX-NIFTY"), "{html}");
        assert!(html.contains("class=\"num miss\""), "{html}");
    }

    #[test]
    fn the_store_page_reads_zero_as_zero_and_pages_past_the_end_by_clamping() {
        // A genesis manifest for each vendor: the store exists and holds
        // nothing. That is a real answer and a DIFFERENT one from an absence,
        // so the counters read 0 and the page is not degraded.
        let root = store_root("storezero");
        for vendor in Vendor::ALL {
            let header = pull::manifest::ManifestHeader::genesis(vendor);
            let mut bytes = vec![0u8; 32_768];
            bytes.splice(..64, header.image());
            std::fs::write(pull::manifest::manifest_path(&root, vendor), &bytes).expect("write");
        }
        let dir = agreeing("storezero");
        let site = Site::new(universe(&dir), census::read_all(&root), root);

        let html = store_ok(&site, day(2026, 8, 7), 0, "show=gaps");
        assert!(
            html.contains("<div class=\"cv\">0</div>"),
            "zero months: {html}"
        );
        assert!(html.contains("0 month(s), 0 row(s)"), "{html}");
        assert!(
            !html.contains("UNAVAILABLE"),
            "an empty store is not absent"
        );
        assert!(
            html.contains("badge good"),
            "and it is not degraded: {html}"
        );
        // Still nothing held, so every grid cell is a miss rather than a zero.
        assert!(html.contains("class=\"num miss\""), "{html}");

        // PAGING PAST THE END CLAMPS. `?page=999` is a stale bookmark, not an
        // attack, and it must land somewhere real.
        let first = store_ok(&site, day(2026, 8, 7), 0, "show=gaps");
        let past = store_ok(&site, day(2026, 8, 7), 999, "show=gaps");
        assert_eq!(past, first, "an out-of-range page clamps to the last one");
        assert!(
            past.contains("NSE-INDEX-NIFTY"),
            "and still has rows: {past}"
        );
        // The fixture's one index × 36 months fits one page, so there is no
        // pager at all — navigation that leads nowhere is worse than none.
        assert!(!first.contains("class=\"pager\""), "{first}");
    }

    #[test]
    fn the_store_grid_pages_when_it_is_larger_than_one_page() {
        // 200 rows per page against 36 months means the pager appears at six
        // instruments. Without a fixture that large the paging arms are code no
        // test enters.
        let dir = agreeing("storepager");
        let mut site = site("storepager", &dir);
        site.series = (0..10)
            .filter_map(|i| {
                Some(census::Series {
                    exchange: brutex_core::instrument::Exchange::Nse,
                    segment: brutex_core::instrument::Segment::Index,
                    symbol: brutex_core::symbol::Symbol::new(&format!("IDX{i:02}")).ok()?,
                    timeframe: store::path::Timeframe::MINUTE_1,
                })
            })
            .collect();
        assert_eq!(census::grid_rows(site.series.len()), 360);

        let first = store_ok(&site, day(2026, 8, 7), 0, "show=gaps");
        assert!(first.contains("page 1 of 2"), "{first}");
        assert!(first.contains("next"), "{first}");
        assert!(!first.contains("previous"), "no previous to nowhere");
        assert!(first.contains("360 instrument-month(s)"), "{first}");
        assert!(first.contains("showing 200"), "{first}");

        let last = store_ok(&site, day(2026, 8, 7), 1, "show=gaps");
        assert!(last.contains("page 2 of 2"), "{last}");
        assert!(last.contains("previous"), "{last}");
        assert!(!last.contains("next &rarr;"), "{last}");
        assert!(last.contains("showing 160"), "the remainder: {last}");

        // And past the end clamps onto that last page exactly.
        assert_eq!(store_ok(&site, day(2026, 8, 7), 99, "show=gaps"), last);
    }

    /// THE REGRESSION THIS PINS. `log_dir_from`'s first version probed for
    /// `Cargo.toml`, which every crate directory also has — so `cargo test -p
    /// api` wrote `crates/api/logs/events.ndjson`, an untracked file whose
    /// extension is on no gate 1 allowlist. This asserts the crate directory is
    /// NOT mistaken for the top, which is the half that was wrong.
    #[test]
    fn a_crate_directory_is_not_a_workspace_root_and_does_not_take_the_log() {
        let store = std::path::Path::new("/store");
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // `crates/api` — has a Cargo.toml, has no `[workspace]`.
        assert!(
            repo.join("Cargo.toml").is_file(),
            "the premise: it is a manifest"
        );
        assert!(
            !is_workspace_root(repo),
            "a crate is not the workspace root"
        );
        assert_eq!(
            log_dir_from(None, Some(repo), store),
            telemetry::dir_beneath_store(store),
            "a crate directory falls back to the store, never to ./logs"
        );

        // The workspace root two levels up — has `[workspace]`.
        let root = repo
            .parent()
            .and_then(std::path::Path::parent)
            .expect("crates/api has two parents");
        assert!(is_workspace_root(root), "the workspace root is recognised");
        assert_eq!(
            log_dir_from(None, Some(root), store),
            root.join("logs"),
            "the workspace root takes ./logs, which is what pressing Run does"
        );
    }

    /// THE PER-SUBSYSTEM SYNTAX REACHES THE SINK, AND A TYPO IS NEVER SILENT.
    ///
    /// `Config::with_target_level` and `Sink::level_for` were built, tested and
    /// mutation-checked in `crates/telemetry` — and nothing parsed the
    /// environment into them, so the whole feature was unreachable from the
    /// operator. That is the same "built with no caller" shape
    /// `telemetry::tail` wore before `/logs` existed.
    #[test]
    fn the_log_level_env_parses_a_global_floor_and_per_subsystem_overrides() {
        // Absent: the default, and the sentence names the syntax.
        let (config, note) = log_level_from(None);
        assert_eq!(config.min_level, telemetry::Level::Info);
        assert!(config.target_levels.is_empty());
        assert!(
            note.contains("pull=debug"),
            "the default names the syntax: {note}"
        );

        // A bare word is the global floor, exactly as before this feature.
        let (config, note) = log_level_from(Some("debug"));
        assert_eq!(config.min_level, telemetry::Level::Debug);
        assert!(config.target_levels.is_empty());
        assert!(note.contains("debug"), "{note}");

        // THE POINT: one subsystem loud, the rest at the global floor.
        let (config, note) = log_level_from(Some("info,pull=debug,pull.chunk=trace"));
        assert_eq!(config.min_level, telemetry::Level::Info);
        assert_eq!(config.target_levels.len(), 2);
        let sink = telemetry::Sink::open(&telemetry::Config {
            dir: crate::scratch::path("log-level-env"),
            ..config
        })
        .expect("opens");
        assert_eq!(sink.level_for("pull"), telemetry::Level::Debug);
        assert_eq!(
            sink.level_for("pull.member"),
            telemetry::Level::Debug,
            "children inherit"
        );
        assert_eq!(
            sink.level_for("pull.chunk"),
            telemetry::Level::Trace,
            "longest wins"
        );
        assert_eq!(
            sink.level_for("api.request"),
            telemetry::Level::Info,
            "the rest stay quiet"
        );
        assert!(
            note.contains("pull=debug"),
            "the banner says what it applied: {note}"
        );

        // A TYPO IS NAMED, NOT SWALLOWED. `CLAUDE.md` §4: an operator who
        // writes `pul=debg` must be told, not handed a quiet `info` and left
        // wondering where the lines went.
        let (config, note) = log_level_from(Some("info,pull=debg,notalevel"));
        assert_eq!(config.min_level, telemetry::Level::Info);
        assert!(
            config.target_levels.is_empty(),
            "an unreadable level sets nothing"
        );
        assert!(note.contains("IGNORED"), "and it is reported: {note}");
        assert!(
            note.contains("debg") && note.contains("notalevel"),
            "by name: {note}"
        );

        // Whitespace and case are the operator's, not a refusal.
        let (config, _) = log_level_from(Some(" INFO , PULL = TRACE "));
        assert_eq!(config.min_level, telemetry::Level::Info);
        assert_eq!(config.target_levels.len(), 1);
    }

    /// **THE CEILING IS REPORTED WHEN IT BITES, AND NOT WHEN IT DOES NOT.**
    ///
    /// `telemetry::Config::with_target_level` drops silently past
    /// [`telemetry::MAX_TARGET_LEVELS`] — its own doc says so — so the banner is
    /// the only place an operator can learn that an override they typed is not
    /// in force. This pins both halves, and the second is the one that was
    /// wrong: the note used to fire on
    /// `target_levels.len() == MAX_TARGET_LEVELS`, which is also true when
    /// every one of exactly eight overrides was applied. Somebody who used
    /// their whole budget correctly was told "later ones were dropped".
    #[test]
    fn the_override_ceiling_is_named_only_when_something_was_actually_dropped() {
        let max = telemetry::MAX_TARGET_LEVELS;

        // EXACTLY AT THE CEILING, NOTHING DROPPED. No alarm.
        let full: Vec<String> = (0..max).map(|n| format!("t{n}=debug")).collect();
        let (config, note) = log_level_from(Some(&format!("info,{}", full.join(","))));
        assert_eq!(config.target_levels.len(), max, "all {max} applied");
        assert!(
            !note.contains("DROPPED"),
            "nothing was dropped, so nothing may be announced: {note}"
        );

        // ONE PAST IT. The overflow is named, and named BY NAME so the operator
        // knows which subsystem is not in force rather than only that one is.
        let over: Vec<String> = (0..=max).map(|n| format!("t{n}=debug")).collect();
        let (config, note) = log_level_from(Some(&format!("info,{}", over.join(","))));
        assert_eq!(config.target_levels.len(), max, "the ceiling still holds");
        assert!(note.contains("DROPPED"), "{note}");
        assert!(
            note.contains(&format!("t{max}=debug")),
            "the dropped override is named, not merely counted: {note}"
        );
        assert!(
            !note.contains(&format!("; per subsystem: t{max}=debug")),
            "and it is NOT also listed as applied: {note}"
        );

        // A dropped override and an unreadable one are different faults and are
        // reported separately — an operator who sees only one message must not
        // have to guess which of the two happened.
        let (_, note) = log_level_from(Some(&format!("info,{},bad=nope", over.join(","))));
        assert!(note.contains("DROPPED"), "{note}");
        assert!(note.contains("IGNORED as unreadable"), "{note}");
        assert!(note.contains("bad=nope"), "{note}");
    }

    #[test]
    fn zz_probe_duplicate_banner() {
        let (config, note) = log_level_from(Some("pull=debug,pull=trace"));
        println!("A targets={:?}", config.target_levels);
        println!("A note={note}");

        let max = telemetry::MAX_TARGET_LEVELS;
        let mut clauses: Vec<String> = (0..max).map(|_| "pull=debug".to_owned()).collect();
        clauses.push("api=trace".to_owned());
        let (config, note) = log_level_from(Some(&clauses.join(",")));
        println!(
            "B len={} targets={:?}",
            config.target_levels.len(),
            config.target_levels
        );
        println!("B note={note}");

        let (config, note) = log_level_from(Some("info,pull=debug,pull=trace,pull=warn"));
        let sink = telemetry::Sink::open(&telemetry::Config {
            dir: crate::scratch::path("zz-probe-dup"),
            ..config
        })
        .expect("opens");
        println!("C live pull={:?}", sink.level_for("pull"));
        println!("C note={note}");
    }

    /// The two arms that do not depend on the filesystem at all.
    #[test]
    fn the_log_directory_prefers_the_environment_and_falls_back_to_the_store() {
        let store = std::path::Path::new("/store");
        assert_eq!(
            log_dir_from(
                Some(std::ffi::OsString::from("/named/elsewhere")),
                None,
                store
            ),
            std::path::Path::new("/named/elsewhere"),
            "an explicit BRUTEX_LOGS wins over every probe"
        );
        // The named path wins even when the working directory IS a workspace root,
        // so an operator can move the log off a synced directory without moving
        // the checkout.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("crates/api has two parents");
        assert_eq!(
            log_dir_from(Some(std::ffi::OsString::from("/named")), Some(root), store),
            std::path::Path::new("/named"),
        );
        assert_eq!(
            log_dir_from(None, None, store),
            telemetry::dir_beneath_store(store),
            "no variable and no working directory falls back to the store"
        );
    }

    /// A directory with no manifest at all is not a workspace root, and the
    /// read failing is `false` rather than a panic.
    #[test]
    fn a_directory_with_no_manifest_is_not_a_workspace_root() {
        assert!(!is_workspace_root(std::path::Path::new(
            "/nonexistent-uCe1r"
        )));
        let store = std::path::Path::new("/store");
        assert_eq!(
            log_dir_from(
                None,
                Some(std::path::Path::new("/nonexistent-uCe1r")),
                store
            ),
            telemetry::dir_beneath_store(store),
        );
    }

    /// `BRUTEX_NO_OPEN` is honoured, and the refusal names the variable rather
    /// than reporting a spawn that never happened.
    ///
    /// The spawning arm is deliberately not exercised: asserting it would open
    /// a browser window on the machine running the suite, and a test with a
    /// visible side effect on the operator's desktop is one they will disable.
    #[test]
    fn the_browser_is_not_opened_when_the_operator_said_not_to() {
        let why = open_unless_suppressed("http://127.0.0.1:8080/", Some(std::ffi::OsStr::new("1")))
            .expect_err("it must refuse when the variable is set");
        assert!(
            why.contains(NO_OPEN_ENV),
            "the refusal names the variable that caused it: {why}"
        );
        // AN EMPTY VALUE STILL SUPPRESSES. `var_os` returns `Some("")` for
        // `BRUTEX_NO_OPEN=`, and an operator who wrote that meant "off" — the
        // test pins the presence rule rather than a truthiness one.
        assert!(
            open_unless_suppressed("http://127.0.0.1:8080/", Some(std::ffi::OsStr::new("")))
                .is_err(),
            "presence suppresses, whatever the value"
        );
    }

    #[test]
    fn the_store_root_comes_from_the_environment_or_defaults_under_home() {
        assert_eq!(
            store_dir_from(Some("/somewhere/else".into()), Some("/home/who".into())),
            Ok(PathBuf::from("/somewhere/else")),
            "an explicit value wins"
        );
        // An explicit RELATIVE value is still honoured: that is an operator's
        // stated choice, and refusing a choice is a different act from
        // inventing one. Only the fallback is refused.
        assert_eq!(
            store_dir_from(Some(".".into()), None),
            Ok(PathBuf::from(".")),
            "BRUTEX_STORE is obeyed as given"
        );
        assert_eq!(
            store_dir_from(None, Some("/home/who".into())),
            Ok(PathBuf::from("/home/who/.brutex/store"))
        );
        // THE FALLBACK IS GONE, AND WHAT REPLACED IT NAMES THE CAUSE.
        //
        // `PathBuf::from(".")` was asserted here beside the words "no HOME is a
        // broken environment, not a supported one". `.` is the working
        // directory, which for the launcher this runs under is the repository
        // checkout, and the first append would have built `bars/`, `manifest/`
        // and `audit/` inside it — untracked, so CI gate 1 walks past them.
        // D-0124.
        let refused = store_dir_from(None, None).expect_err("a broken environment refuses");
        assert!(refused.starts_with("REFUSED:"), "{refused}");
        assert!(
            refused.contains("BRUTEX_STORE") && refused.contains("HOME"),
            "the refusal names both variables an operator can set: {refused}"
        );
        assert!(
            refused.contains("bars/") && refused.contains("manifest/"),
            "and what the old fallback would have created, and where: {refused}"
        );
        // And the environment is read in exactly one place, which is this one.
        // Asserted against the pure function fed the SAME environment rather
        // than against `served_store_root`, which calls `store_dir` itself:
        // comparing a function with its own caller cannot fail, and
        // `cargo mutants` proved it by replacing `store_dir` with an empty
        // path and passing.
        assert_eq!(
            store_dir(),
            store_dir_from(std::env::var_os("BRUTEX_STORE"), std::env::var_os("HOME")),
            "store_dir is exactly store_dir_from over the environment"
        );
        assert!(
            !store_dir().is_ok_and(|d| d.as_os_str().is_empty()),
            "and when it names somewhere, an empty path is not a store root"
        );
        assert_eq!(served_store_root(), store_dir(), "one reader, one answer");
    }

    #[test]
    fn a_clock_that_names_no_day_refuses_the_page_rather_than_guessing_one() {
        // An expiry gate compared against an invented "today" is a gate that
        // passes for the wrong reason, so there is no default. Both arms of the
        // one place that reads the clock are driven here by VALUE, because a
        // branch only a broken machine could enter is a branch no test can
        // hold — the same split `run_in` and `masters_dir_from` already use.
        let broken = ingest::ist_day(
            std::time::SystemTime::UNIX_EPOCH - std::time::Duration::from_hours(24),
        );
        let why = broken.clone().expect_err("no such day");
        let html = refusal_html("Ingest", &why);
        assert!(html.contains("REFUSED"), "{html}");
        assert!(html.contains("clock"), "{html}");
        assert!(html.contains("nothing was written"), "{html}");

        // ONE closure, driven through BOTH arms. A closure body that only the
        // passing arm can reach is a body no test enters, so the same
        // non-capturing renderer is handed to the refusing call and to the
        // succeeding one -- and the refusing call proves it was never invoked
        // by answering with the refusal page rather than with a date.
        let render = |d: Day| (axum::http::StatusCode::OK, d.to_string());
        let (code, page) = dated(broken, "Ingest", render);
        assert_eq!(code, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert!(page.contains("clock"), "{page}");
        assert!(
            !page.contains("1970-01-01"),
            "the closure must not have run: {page}"
        );

        // And the other arm hands the day straight through.
        let (code, page) = dated(Ok(day(2026, 8, 7)), "Ingest", render);
        assert_eq!(code, axum::http::StatusCode::OK);
        assert_eq!(page, "2026-08-07");
    }

    /// What every spot target must put on its own receipt, on the
    /// `targetcounts` fixture, for the default feed.
    ///
    /// A module-level table rather than forty lines inside the test, because
    /// the test drives a real server and the rows are data. `covered` is the
    /// merged universe's count for the slot; `reach` is what the feed named on
    /// the request can actually be asked for — D-0120's second number, and the
    /// only one of the two that predicts what comes back. The five list-defined
    /// targets show it as a fraction of the published list, so `1 of 750` is a
    /// Total Market pull that will fetch one name, a sentence that was
    /// previously nowhere on the page. `broker_run` still ATTEMPTS the first
    /// number and refuses the difference by name — `docs/06-limits.md` §63 —
    /// so this is a prediction of the outcome and not of the request count.
    const TARGET_RECEIPTS: [(&str, &str, usize, &str); ingest::SpotTarget::ALL.len()] = [
        (
            "swept",
            "Swept indices",
            1,
            "1 — every name this target holds, by Dhan id",
        ),
        (
            "indices",
            "Reference indices",
            2,
            "2 — every name this target holds, by Dhan id",
        ),
        (
            "equities",
            "NIFTY Total Market equities",
            1,
            "1 of 750 — 749 cannot be named by Dhan",
        ),
        (
            "n500",
            "NIFTY 500 equities",
            1,
            "1 of 500 — 499 cannot be named by Dhan",
        ),
        (
            "n200",
            "NIFTY 200 equities",
            1,
            "1 of 200 — 199 cannot be named by Dhan",
        ),
        (
            "n100",
            "NIFTY 100 equities",
            1,
            "1 of 100 — 99 cannot be named by Dhan",
        ),
        (
            "n50",
            "NIFTY 50 equities",
            1,
            "1 of 50 — 49 cannot be named by Dhan",
        ),
        // NIFTY and RELIANCE are both F&O underlyings; INDIAVIX is not. So this
        // row is 2 where its four neighbours are 1, which is what makes it
        // mutant-visible rather than a copy of the row above.
        // TWO COVERED, ONE REACHABLE, AND THE GAP IS THE POINT.
        //
        // NIFTY and RELIANCE both carry `Universe::FNO`, so the merged universe
        // counts 2. The JOIN is keyed on `(exchange, ISIN)` (D-0125) and an
        // INDEX has no ISIN — NSE never issues one — so only RELIANCE resolves
        // to a Dhan id. This is the only row in the table where the two numbers
        // differ, which is exactly what makes a positional mix-up visible here.
        // `F&amp;O`, NOT `F&O`, AND THAT IS THE ASSERTION WORKING.
        //
        // This column is matched against RENDERED HTML, and `render::escape`
        // turns the ampersand in the label into an entity. Writing the raw
        // label here would fail, and "fixing" it by loosening the assertion
        // would drop the only row in this table that exercises escaping on a
        // target name at all.
        (
            "fno",
            "F&amp;O underlyings",
            2,
            "1 of 213 — 212 cannot be named by Dhan",
        ),
        // ALL THREE, and it is the only counter in this table that is 3. It is
        // master-counted rather than joined — `Everything` has no published
        // list — so its receipt reads like `indices`, not like a tier.
        (
            "all",
            "Everything tracked",
            3,
            "3 — every name this target holds, by Dhan id",
        ),
    ];

    #[tokio::test]
    async fn each_spot_target_reports_its_own_population_and_not_a_neighbours() {
        // Found by `cargo mutants` while D-0038 was measuring `server.rs`: the
        // receipt looks the requested target up to state how many instruments
        // it covers, and on a fixture where all three populations happen to be
        // equal the `==` doing that lookup can be a `!=` with the suite green.
        //
        // INDIAVIX is an index and is NOT swept — `CLAUDE.md` §1 makes it
        // reference-only — so this fixture has one swept series, TWO index
        // series and one Total Market constituent, and the three answers are
        // three different numbers.
        let dir = masters(
            "targetcounts",
            Some(&format!(
                "{GROWW_HEAD}\
                 NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n\
                 NSE,CASH,,INDIAVIX,IDX,,NIFTY,,,NSE-INDIAVIX\n\
                 NSE,CASH,,RELIANCE,EQ,EQ,INE002A01018,,,NSE-RELIANCE\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}\
                 NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,1333\n\
                 NSE,I,NA,INDEX,INDIAVIX,INDIA VIX,INDEX,NA,0001-01-01,,,1333\n\
                 NSE,E,INE002A01018,EQUITY,RELIANCE,RELIANCE INDUSTRIES,ES,EQ,,,,1333\n"
            )),
        );
        let built = site("targetcounts", &dir);
        // RELIANCE is in all four published tiers as well as the Total Market,
        // so the four counters D-0105 appended are 1 each and the fixture
        // proves them without gaining a row. The two numbers that are NOT one
        // are the ones a mutant would have to move.
        assert_eq!(
            built.targets,
            [1, 2, 1, 1, 1, 1, 1, 2, 3],
            "one swept, two index series, one Total Market constituent, \
             RELIANCE once in each of the 500, 200, 100 and 50, TWO F&O \
             underlyings (NIFTY and RELIANCE, not INDIAVIX) and all three \
             tracked"
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let stop_addr = stopper.local_addr().expect("addr");
        let served = tokio::spawn(serve(
            listener,
            router_serving(Loaded::new(built), front("pill")),
            Box::pin(async move { stopper.accept().await.map(|_| ()) }),
        ));

        // ALL SEVEN, END TO END: the slug goes on the wire, the parser takes
        // it, the receipt echoes the label and the count beside it is the one
        // `Site::new` counted for that slot. The four tiers are here because a
        // slug that parses and then reports another target's population is the
        // defect this test was written for, and appending variants is exactly
        // when a positional lookup goes wrong.
        //
        // TWO NUMBERS PER ROW, and they are different questions. `covered` is
        // the merged universe's count for the slot; `reach` is what the feed
        // this request named can actually be asked for, which is the number
        // D-0120 put on the receipt because it is the one that predicts what
        // comes back. See `TARGET_RECEIPTS`, and `docs/06-limits.md` §63 for
        // the half of this that the pull path does not yet honour.
        for (slug, label, covered, reach) in TARGET_RECEIPTS {
            let form = format!("target={slug}&from=2022-01-08&to=2022-01-08");
            let out = post(addr, "/pull/spot", &form).await;
            assert!(out.contains(label), "{slug} echoes as {label}: {out}");
            assert!(
                out.contains(&format!(
                    "<th>Instruments covered</th><td>{covered} in the merged universe</td>"
                )),
                "{slug} covers {covered}: {out}"
            );
            assert!(
                out.contains(&format!("<th>This feed can name</th><td>{reach}")),
                "{slug} must report {reach:?} for the feed it was asked with: {out}"
            );
        }
        // AND THE REASON IS REACHABLE, not merely counted. Five names, then a
        // remainder, then the route that carries all 749 of them.
        let equities = post(
            addr,
            "/pull/spot",
            "target=equities&from=2022-01-08&to=2022-01-08",
        )
        .await;
        assert!(
            equities.contains("and 744 more."),
            "the 749 that cannot be named are five names and a remainder, not a \
             bare number: {equities}"
        );
        assert!(
            equities.contains("/universes.json?feed=dhan"),
            "and the receipt points at the surface that carries all 749 with their \
             reasons, so a shortfall has a way down rather than being a dead end: \
             {equities}"
        );

        let _ = tokio::net::TcpStream::connect(stop_addr).await;
        served
            .await
            .expect("task")
            .expect("a graceful shutdown is not a failure");
    }

    #[test]
    fn the_dashboard_counts_each_universe_and_shouts_only_when_something_disagrees() {
        // Found by `cargo mutants` while D-0038 was measuring `server.rs`: the
        // dashboard's fold and its `disputes > 0` flag were reachable but not
        // distinguished, so `a + 1` could be `a * 1` and `> 0` could be `== 0`
        // with the suite green. Each figure is now pinned to its own number.
        //
        // The fixture is NIFTY — an index, and an F&O underlying — plus
        // RELIANCE, which is a Total Market constituent and an F&O underlying.
        let clean = universe(&agreeing("dashclean"));
        let html = dashboard_html(&clean);
        for (label, value) in [
            ("Tracked", "2"),
            ("NIFTY Total Market", "1"),
            ("F&amp;O underlyings", "2"),
            ("Indices", "1"),
            ("Confirmed by both feeds", "2"),
            ("Disagreements", "0"),
        ] {
            let expected =
                format!("<div class=\"ck\">{label}</div><div class=\"cv\">{value}</div>");
            assert!(html.contains(&expected), "{label} must be {value}: {html}");
        }
        assert!(
            !html.contains("card loud"),
            "nothing disagreed, so nothing shouts: {html}"
        );
        assert!(html.contains("badge good"), "{html}");

        // One disagreement, and the same figure is loud. Both sides of the
        // comparison, driven by data rather than by a constructed `Stat`.
        let disputed = universe(&masters(
            "dashloud",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,CHOLAFIN,EQ,EQ,INE121A01024,,,NSE-CHOLAFIN\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}NSE,E,INE121A08PJ0,EQUITY,CHOLAFIN,CHOLA,ES,EQ,,,,1333\n"
            )),
        ));
        let loud = dashboard_html(&disputed);
        assert!(loud.contains("card loud"), "a disagreement shouts: {loud}");
        assert!(
            loud.contains("<div class=\"ck\">Disagreements</div><div class=\"cv\">1</div>"),
            "{loud}"
        );
        assert!(loud.contains("badge bad"), "{loud}");

        // THE FIGURE IS THE SUM OF BOTH KINDS OF DISAGREEMENT, and this is the
        // fixture that says so: CHOLAFIN is one ISIN conflict and FISTIPD3GP is
        // one eligibility conflict, so the answer is 2. With only one kind
        // present the addition could be a subtraction — `cargo mutants` found
        // exactly that.
        let both = universe(&masters(
            "dashboth",
            Some(&format!(
                "{GROWW_HEAD}\
                 NSE,CASH,,CHOLAFIN,EQ,EQ,INE121A01024,,,NSE-CHOLAFIN\n\
                 NSE,CASH,,FISTIPD3GP,EQ,MF,INF090I01VS3,,,NSE-FISTIPD3GP\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}\
                 NSE,E,INE121A08PJ0,EQUITY,CHOLAFIN,CHOLA,ES,EQ,,,,1333\n\
                 NSE,E,INF090I01VS3,EQUITY,FISTIPD3GP,FRANKLIN PLAN,ETF,EQ,,,,1333\n"
            )),
        ));
        let two = dashboard_html(&both);
        assert!(
            two.contains("<div class=\"ck\">Disagreements</div><div class=\"cv\">2</div>"),
            "one identity conflict plus one eligibility conflict is two: {two}"
        );
    }

    #[tokio::test]
    async fn the_escape_hatch_is_read_off_the_query_string_and_not_off_its_presence() {
        // Found by `cargo mutants` while D-0038 was measuring `server.rs`: the
        // handler reads `all=1`, and with only one value ever requested over
        // HTTP the `==` could be a `!=` — so `?all=0` would widen and `?all=1`
        // would narrow, which is the page doing exactly the opposite of what
        // the link says.
        let dir = masters(
            "hatchhttp",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,RAJESHEXPO,EQ,EQ,INE343B01030,,,NSE-RAJESHEXPO\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}NSE,E,INE343B01030,EQUITY,RAJESHEXPO,RAJESH EXPORTS,ES,EQ,,,,1333\n"
            )),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let stop_addr = stopper.local_addr().expect("addr");
        let served = tokio::spawn(serve(
            listener,
            router_serving(
                Loaded::new(Site::load(&dir, &store_root("hatchhttp"))),
                front("hatchhttp"),
            ),
            Box::pin(async move { stopper.accept().await.map(|_| ()) }),
        ));

        // RAJESHEXPO is in neither tracked universe, so it is the only kind of
        // row the filter actually removes.
        let widened = get(addr, "/instruments?all=1").await;
        assert!(widened.contains("NSE-RAJESHEXPO"), "{widened}");
        for narrow in ["/instruments", "/instruments?all=0", "/instruments?all=yes"] {
            let page = get(addr, narrow).await;
            assert!(
                !page.contains("NSE-RAJESHEXPO"),
                "{narrow} must NOT widen: {page}"
            );
        }

        let _ = tokio::net::TcpStream::connect(stop_addr).await;
        served
            .await
            .expect("task")
            .expect("a graceful shutdown is not a failure");
    }

    #[test]
    fn a_site_with_no_universe_still_shows_the_two_instruments_that_matter() {
        // Nothing has been ingested and no master was read, so the axis has only
        // the swept pair on it — which is the answer, not an absence of one. The
        // grid's axis no longer comes from the masters at all (see
        // `census::held_series`), so an empty merge cannot empty it.
        let empty = masters("nomasters", None, None);
        let site = site("nomasters", &empty);
        assert_eq!(site.series.len(), 2, "the engine surface, exactly");
        assert_eq!(
            site.targets,
            [0; ingest::SpotTarget::ALL.len()],
            "and no target has members — all seven, sized off the enum so an \
             eighth cannot be forgotten here"
        );

        let html = store_ok(&site, day(2026, 8, 7), 0, "show=gaps");
        assert!(
            html.contains("NSE-INDEX-NIFTY") && html.contains("NSE-INDEX-BANKNIFTY"),
            "{html}"
        );
        assert!(
            html.contains("groww: UNAVAILABLE") && html.contains("dhan: UNAVAILABLE"),
            "the masters' own absence rides along: {html}"
        );

        // The ingest page then offers every target with a truthful zero.
        let ingest_html = pull_html(&site, day(2026, 8, 7));
        assert!(ingest_html.contains("0 instrument(s)"), "{ingest_html}");
    }

    #[test]
    fn a_window_whose_wire_date_does_not_exist_says_so_rather_than_wrapping() {
        // The vendor's `toDate` is the day after the operator's last day, and
        // after 9999-12-31 there is no such day. Refused by name in the fact
        // itself — a wrapped date would silently ask for a window ending in
        // 1970.
        let window =
            pull::session::Window::new(day(9990, 1, 1), day(9999, 12, 31)).expect("forwards");
        let facts = window_facts(window);
        let wire = facts
            .iter()
            .find(|&&(k, _)| k == "toDate on the wire")
            .map(|(_, v)| v.clone())
            .expect("the fact is always present");
        assert!(wire.starts_with("REFUSED — "), "{wire}");
        assert!(wire.contains("9999-12-31"), "{wire}");

        // An ordinary window states the day after, and says why it is that day.
        let ordinary = window_facts(
            pull::session::Window::new(day(2022, 1, 8), day(2022, 2, 8)).expect("forwards"),
        );
        let wire = ordinary
            .iter()
            .find(|&&(k, _)| k == "toDate on the wire")
            .map(|(_, v)| v.clone())
            .expect("present");
        assert!(wire.starts_with("2022-02-09"), "{wire}");
        assert!(wire.contains("not inclusive"), "{wire}");
    }

    #[tokio::test]
    async fn an_expiry_gate_driven_by_value_answers_both_ways_without_a_clock() {
        // `fno_answer` is the half of the handler the expiry rule lives in, and
        // it takes the day rather than reading one, so both outcomes are pinned
        // rather than being properties of when the suite ran.
        let root = store_root("fnogate");
        let journal = audit::Journal::at(&root);
        let site = Site::new(universe(&root), census::read_all(&root), root.clone());
        // AN ARCHIVE FEED, DELIBERATELY. This test is about the expiry gate, and
        // the gate is upstream of the transport — but an HTTP feed would carry
        // this body on to a credential read and a discovery socket, which is a
        // live vendor request inside a unit test. Naming the archive feed keeps
        // every assertion below about the gate and makes the network
        // unreachable from here rather than merely unlikely.
        let (code, page) = fno_answer(
            "underlying=NIFTY&series=fut&expiry=2026-07-30&from=2026-07-01&to=2026-07-30\
             &vendor=truedata",
            day(2026, 8, 7),
            moment(),
            &journal,
            Broker::Refused,
            &site,
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert!(page.contains("NOT STARTED"), "{page}");
        assert!(
            page.contains("expired, checked against 2026-08-07"),
            "{page}"
        );
        assert!(
            page.contains("<th>Recorded</th><td>yes"),
            "an accepted request that cannot run is still on the record: {page}"
        );

        let (code, page) = fno_answer(
            "underlying=NIFTY&series=fut&expiry=2026-08-07&from=2026-08-01&to=2026-08-07\
             &vendor=truedata",
            day(2026, 8, 7),
            moment(),
            &journal,
            Broker::Refused,
            &site,
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
        assert!(page.contains("LIVE CONTRACT IS NEVER STORED"), "{page}");

        // BOTH went into the journal — the refusal as well as the acceptance.
        // An operator debugging a form that never starts anything needs the
        // refusals more than the successes, and a log that keeps only the
        // successes is the one that cannot answer them.
        let log = journal.look();
        assert_eq!(
            log.records(),
            2,
            "one record per request, refusals included"
        );
        let rows = journal.page(2, 0, 2).expect("a page");
        let newest = rows[0].decoded.clone().expect("decodes");
        assert_eq!(newest.outcome, audit::Outcome::Refused);
        assert_eq!(newest.scope, audit::Scope::Fno);
        assert!(newest.note.contains("has not expired"), "{}", newest.note);
        // AND THE CUT IS VISIBLE. The refusal is 107 bytes and the field holds
        // 68, so what the record keeps is a prefix — and it says so, rather
        // than presenting a half sentence as the whole reason.
        assert!(newest.note_was_cut(), "{}", newest.note);
        assert!(newest.note_bytes > 68, "the original length is kept");
        let older = rows[1].decoded.clone().expect("decodes");
        assert_eq!(older.outcome, audit::Outcome::NotStarted);
        assert_eq!(older.source, "NIFTY");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The whole path through `run`, and the first event that path writes.
    ///
    /// # Why it takes the shared sink FIRST, on its first line
    ///
    /// `run_in` installs a sink of its own, over the SERVED log directory —
    /// `BRUTEX_LOG_DIR`, or the operator's real store root — at the floor
    /// `BRUTEX_LOG_LEVEL` names. In this test binary that made the process
    /// global a race: whichever of `run_in` and [`crate::emitted::sink`] got
    /// there first decided both the directory every other test reads and the
    /// floor every other test's events have to clear, and at the default `Info`
    /// floor a `Debug` site writes nothing at all. Taking the shared sink here,
    /// before `run` is entered, is what makes the binary deterministic —
    /// `run_in`'s install is then refused, which is the refusal working, and it
    /// prints the reason in the banner rather than swallowing it.
    ///
    /// The refusal is also why nothing this suite writes can land in the
    /// operator's own `~/.brutex/store/logs`.
    ///
    /// # And the event
    ///
    /// `api.serve listening` is the record that answers *what was this process
    /// configured to do* after the terminal that printed the banner is closed.
    /// The site already checks its own `Emitted` — `if !first.is_written()` —
    /// but with no sink installed that check passes vacuously, which is exactly
    /// the state `crates/api/src/emitted.rs` was written to end.
    #[tokio::test]
    async fn run_serves_until_the_signal_and_exits_zero() {
        let _shared = crate::emitted::sink();
        let from = crate::emitted::mark();
        // The server binds an ephemeral port, accepts nothing, and stops --
        // which is the whole path through `run` minus the waiting.
        //
        // THE EXIT CODE IS THE UNIVERSE'S OWN VERDICT, not a constant. `run`
        // reads the masters directory of whatever machine this suite is on:
        // this developer's holds a real pair of vendor masters that disagree,
        // and CI's fixtures are clean. A hardcoded `OK` here asserted the
        // machine rather than the code, and `stopped_over` — the reason a
        // degraded serve no longer exits 0 — would have made this test's
        // failure look like its own defect. The expectation is computed from
        // the same `report` D-0026 gave the `report` command.
        //
        // AND THE STORE HAS A VERDICT TOO, which is the other half of the same
        // argument. `run` refuses a store another server already holds, and the
        // developer this suite runs for keeps one serving from an IDE all day —
        // so the assertion below read `left: 1, right: 3` on his machine for a
        // reason that was entirely correct behaviour. The store root cannot be
        // redirected from here: it comes from `BRUTEX_STORE`, `set_var` is
        // `unsafe` under edition 2024, this crate forbids `unsafe`, and mutating
        // it would race every other test in this binary. So the expectation
        // takes the environment as it finds it, exactly as it already does for
        // the masters, and BOTH branches assert something.
        let busy = store_dir().is_ok_and(|root| {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(root.join("serve.lock"))
                .is_ok_and(|file| file.try_lock().is_err())
        });
        if busy {
            // A HELD STORE IS A REFUSAL AND IT NAMES THE HOLDER. This is the
            // same path `run_refuses_a_store_another_server_holds` pins; what is
            // asserted here is that the refusal is what a serve DOES when the
            // store is busy, rather than the universe verdict it would have
            // reported otherwise.
            assert_eq!(
                run(&argv(&["serve", "127.0.0.1:0"]), fired()).await,
                FAILED,
                "a store another server is already serving is refused, not shared"
            );
            let refused = crate::emitted::landed(
                from,
                "api.serve",
                "refused: another instance is serving this store",
            );
            assert!(
                refused.iter().any(|record| {
                    record.level == telemetry::Level::Error
                        && crate::emitted::says(record, "why", "already serving this store")
                }),
                "and the refusal is in the file as well as on the terminal: {refused:?}"
            );
            return;
        }

        let expected =
            masters_dir().map_or(FAILED, |dir| if report(&dir).1 { OK } else { DEGRADED });
        assert_eq!(
            run(&argv(&["serve", "127.0.0.1:0"]), fired()).await,
            expected,
            "a serve exits on the verdict of the universe it served"
        );

        let listening = crate::emitted::landed(from, "api.serve", "listening");
        let mine = listening
            .iter()
            .find(|record| crate::emitted::says(record, "addr", "127.0.0.1:"))
            .expect("the process's own configuration is the first thing it writes");
        assert_eq!(mine.level, telemetry::Level::Info);
        // The two flags, not the two PATHS: `store` and `masters` are read off
        // the machine this suite happens to run on, so asserting them would be
        // asserting somebody's home directory.
        assert!(
            mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
            "a reader arriving afterwards must be able to tell whether this \
             process was serving a front end and whether it was flying: {mine:?}"
        );
    }

    #[tokio::test]
    async fn run_refuses_an_address_it_cannot_bind_and_says_which() {
        let taken = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = taken.local_addr().expect("addr");
        assert_eq!(
            run(&argv(&["serve", &addr.to_string()]), fired()).await,
            FAILED,
            "an address already in use is a refusal, not a silent retry"
        );
    }

    #[tokio::test]
    async fn health_answers_503_when_a_vendor_was_never_read() {
        // A monitor reads the status code and nothing else. 200 with `ok` on
        // the first line, beside `dhan: UNAVAILABLE` in the body, is the exact
        // shape of a green light over a half-read universe.
        let dir = masters(
            "health503",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n"
            )),
            None,
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let stop_addr = stopper.local_addr().expect("addr");
        let served = tokio::spawn(serve(
            listener,
            router(Loaded::new(Site::load(&dir, &store_root("health503")))),
            Box::pin(async move { stopper.accept().await.map(|_| ()) }),
        ));

        let health = get(addr, "/health").await;
        assert!(health.contains("503"), "{health}");
        assert!(health.contains("DEGRADED"), "{health}");
        assert!(health.contains("dhan: UNAVAILABLE"), "{health}");

        let _ = tokio::net::TcpStream::connect(stop_addr).await;
        served
            .await
            .expect("task")
            .expect("a graceful shutdown is not a failure");
    }

    /// The body of a raw HTTP response, so an assertion names the payload
    /// rather than a substring of the whole exchange.
    fn body_of(response: &str) -> &str {
        response.split_once("\r\n\r\n").map_or("", |(_, body)| body)
    }

    /// A store root whose Groww counter exists and will not decode.
    ///
    /// Sixteen bytes, which is shorter than the header region, so
    /// `Manifest::open` refuses it by name and `census::read_vendor` answers
    /// `Census::Unreadable`. NOT an absent file — that is the other state, and
    /// telling the two apart is the whole point of what is asserted below.
    fn corrupt_census(name: &str) -> PathBuf {
        let root = store_root(name);
        std::fs::write(
            pull::manifest::manifest_path(&root, Vendor::Groww),
            [0xFF_u8; 16],
        )
        .expect("a damaged counter");
        root
    }

    /// Serves `site` and hands the body its address.
    async fn served_over<F, Fut>(name: &str, built: Loaded, body: F)
    where
        F: FnOnce(SocketAddr) -> Fut,
        Fut: Future<Output = ()>,
    {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let stop_addr = stopper.local_addr().expect("addr");
        let served = tokio::spawn(serve(
            listener,
            router_serving(built, front(name)),
            Box::pin(async move { stopper.accept().await.map(|_| ()) }),
        ));
        body(addr).await;
        let _ = tokio::net::TcpStream::connect(stop_addr).await;
        served
            .await
            .expect("task")
            .expect("a graceful shutdown is not a failure");
    }

    /// **A CORRUPT COUNTER AND AN EMPTY STORE ARE NOT THE SAME ANSWER.**
    ///
    /// `held_row` folds `Census::Absent` and `Census::Unreadable` into one
    /// `None`, so `store_body` skips every entry either way and `/store.json`
    /// answered `[]` at `200` for both — **byte for byte**, which is asserted
    /// here as the thing that must NOT happen. `/db` reads that as an empty
    /// feed and the Markets page renders "holds no bars", which is a claim
    /// about the store made from a fact about its counter, over a store that
    /// may hold every bar it ever pulled.
    ///
    /// Three things are asserted, and the reason is one of them — a status code
    /// alone would say only "something", and `CLAUDE.md` §4 wants the reason:
    ///
    /// * the status separates them (`503` against `200`);
    /// * `x-brutex-census-state` carries `unreadable` against `absent`;
    /// * `x-brutex-census-note` carries the refusal in the refusal's own words.
    ///
    /// And the fourth thing, which is what makes the change safe to ship: the
    /// **body is still a JSON array** in both states. A consumer that only ever
    /// reads rows sees exactly what it saw before. D-0124.
    #[tokio::test]
    async fn a_corrupt_census_is_not_answered_as_an_empty_store() {
        let dir = agreeing("censusloud");
        let root = corrupt_census("censusloud");
        let built = Loaded::new(Site::load(&dir, &root));
        served_over("censusloud", built, |addr| async move {
            let damaged = get(addr, "/store.json?feed=groww").await;
            let empty = get(addr, "/store.json?feed=dhan").await;

            // THE OLD DEFECT, NAMED. Same body, and before D-0124 that was the
            // whole response — so this pair was indistinguishable.
            assert_eq!(body_of(&damaged), "[]", "{damaged}");
            assert_eq!(body_of(&empty), "[]", "{empty}");
            assert_ne!(
                damaged, empty,
                "a corrupt counter and a store that holds nothing must not be \
                 the same response"
            );

            assert!(
                damaged.starts_with("HTTP/1.1 503"),
                "a counter that will not load cannot answer the question: {damaged}"
            );
            assert!(
                damaged.contains("x-brutex-census-state: unreadable"),
                "{damaged}"
            );
            assert!(
                damaged.contains("x-brutex-census-note:") && damaged.contains("UNREADABLE"),
                "the refusal reaches the caller in its own words: {damaged}"
            );
            assert!(
                damaged.contains("groww.man"),
                "and names the file to look at: {damaged}"
            );
            assert!(
                damaged.contains("x-brutex-census-degraded:"),
                "always present, so absent means this build does not say: {damaged}"
            );

            // THE ORDINARY STATE IS STILL ORDINARY. A fresh install has no
            // manifest and that is not a failure — 200, and the state says
            // which of the two it is.
            assert!(empty.starts_with("HTTP/1.1 200"), "{empty}");
            assert!(empty.contains("x-brutex-census-state: absent"), "{empty}");
            assert!(
                !empty.contains("x-brutex-census-state: unreadable"),
                "{empty}"
            );
            // AND THE CONTENT TYPE IS UNCHANGED. The route still answers JSON;
            // nothing about the shape moved.
            assert!(
                damaged.contains("content-type: application/json"),
                "{damaged}"
            );
        })
        .await;
    }

    /// **`/instruments.json` used to answer both of its failures with `200`
    /// and nothing else.**
    ///
    /// Two states, one route, and neither reached the caller:
    ///
    /// * the counter would not load, so `rows_for` answered `None` for every
    ///   key and every row read `"bars":0` — the same em dash an un-pulled
    ///   instrument shows;
    /// * the selected feed's master would not decode, so no row carried that
    ///   feed's id, the filter admitted nothing, and the body was `[]` —
    ///   identical to a feed that lists nothing.
    ///
    /// `Read::notes` and `Read::status` were one field away, and `/health`
    /// answers `503` off exactly them. Both are on the wire now, and the second
    /// case gets the per-feed answer a whole-read boolean could never give:
    /// `?feed=dhan` on the very same site is `200` and complete.
    #[tokio::test]
    async fn instruments_json_says_when_its_zeroes_are_not_measurements() {
        // (a) THE COUNTER IS DAMAGED. The universe is intact, so the LIST is
        // real and only the counts are absent — 503, and the census headers say
        // which half is not to be believed.
        let dir = agreeing("instrcensus");
        let root = corrupt_census("instrcensus");
        let built = Loaded::new(Site::load(&dir, &root));
        served_over("instrcensus", built, |addr| async move {
            let answer = get(addr, "/instruments.json?feed=groww").await;
            assert!(answer.starts_with("HTTP/1.1 503"), "{answer}");
            assert!(
                answer.contains("x-brutex-census-state: unreadable"),
                "{answer}"
            );
            assert!(
                answer.contains("x-brutex-census-note:") && answer.contains("UNREADABLE"),
                "the reason, not just a code: {answer}"
            );
            assert!(
                answer.contains("x-brutex-master-state: read"),
                "the master decoded — it is the counter that did not: {answer}"
            );
            // THE ZEROES ARE STILL THERE AND STILL AN ARRAY. Nothing about the
            // body moved; what moved is that the response now says the zeroes
            // are absent rather than measured.
            let body = body_of(&answer);
            assert!(body.starts_with('['), "still an array: {body}");
            assert!(body.contains(r#""bars":0"#), "{body}");
        })
        .await;

        // (b) THE SELECTED FEED'S MASTER NEVER DECODED. The body is `[]` and
        // that is not this feed listing nothing.
        let half = masters(
            "instrmaster",
            None,
            Some(&format!(
                "{DHAN_HEAD}\
                 NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,1333\n"
            )),
        );
        let built = Loaded::new(Site::load(&half, &store_root("instrmaster")));
        served_over("instrmaster", built, |addr| async move {
            let missing = get(addr, "/instruments.json?feed=groww").await;
            assert_eq!(body_of(&missing), "[]", "{missing}");
            assert!(
                missing.starts_with("HTTP/1.1 503"),
                "an empty list from a failed read is not an empty universe: {missing}"
            );
            assert!(
                missing.contains("x-brutex-master-state: UNAVAILABLE"),
                "{missing}"
            );
            assert!(
                missing.contains("groww_instruments.csv"),
                "the note names the file that was not read: {missing}"
            );
            assert!(
                missing.contains("x-brutex-universe-status: DEGRADED"),
                "the same word /health answers with: {missing}"
            );

            // AND THE OTHER FEED ON THE SAME SITE IS FINE. This is what a
            // whole-read boolean cannot say, and it is why `Read::master`
            // exists: one master failed, one did not, and the route answers per
            // feed.
            let ok = get(addr, "/instruments.json?feed=dhan").await;
            assert!(ok.starts_with("HTTP/1.1 200"), "{ok}");
            assert!(ok.contains("x-brutex-master-state: read"), "{ok}");
            assert!(
                body_of(&ok).contains(r#""symbol":"NIFTY""#),
                "and it lists what it read: {ok}"
            );
        })
        .await;
    }

    /// **A pull receipt says that it is one, on every arm.**
    ///
    /// `/pull/spot` answers `text/html`, and the page finds the verdict by
    /// looking for a `.badge`. A `200` carrying HTML that is not a receipt — an
    /// interstitial, a misdirected origin, this build's markup after a rename —
    /// has no badge, and the parser falls open to `verdict: 'OK', good: true`.
    /// A request that never reached this process renders a green OK.
    ///
    /// A content type cannot separate them, because a receipt legitimately is
    /// `text/html`. This header can: it is written by the handler, on the
    /// refusals as well as the successes, so a reader may require it
    /// unconditionally — and no other route in this server carries it, which is
    /// the second half of what is asserted here.
    #[tokio::test]
    async fn every_spot_answer_names_itself_a_receipt_and_no_other_route_does() {
        let dir = agreeing("receipt");
        let built = Loaded::new(site("receipt", &dir));
        // A SECOND HANDLE ON THE SAME CONTROL, so the seat-conflict arm can be
        // driven from outside the handler — it is the one arm no request can
        // reach on its own.
        let holder = std::sync::Arc::clone(&built);
        served_over("receipt", built, |addr| async move {
            // A REFUSAL, so nothing is asked of a vendor: `Site::load` sets
            // `Broker::Refused` and the answer is a 503 receipt.
            let refused = post(
                addr,
                "/pull/spot",
                "target=swept&from=2024-01-01&to=2024-01-31",
            )
            .await;
            assert!(
                refused.contains("x-brutex-receipt: pull-spot"),
                "a receipt says so whatever its verdict: {refused}"
            );

            // A malformed request takes a different arm and must carry it too —
            // the marker is only worth requiring if it is unconditional.
            let bad = post(addr, "/pull/spot", "target=nonsense").await;
            assert!(
                bad.contains("x-brutex-receipt: pull-spot"),
                "including the arm that refuses the form: {bad}"
            );

            // AND THE DISCRIMINATOR. Another 200 that is HTML — which is
            // exactly what a proxy interstitial looks like to the page — does
            // not carry it, so requiring the header separates a receipt from
            // any other HTML answer.
            let page = get(addr, "/dashboard").await;
            assert!(page.starts_with("HTTP/1.1 200"), "{page}");
            assert!(page.contains("text/html"), "{page}");
            assert!(
                !page.contains("x-brutex-receipt"),
                "only a receipt claims to be one: {page}"
            );

            // THE THIRD ARM: the pull seat is already held, so the handler
            // refuses before it parses anything. `409` is not a receipt the
            // page would ever draw a verdict from, and it carries the marker
            // for the same reason the other two do — the reader tests one
            // thing, unconditionally.
            let seat = holder
                .autopilot
                .take_seat(pull::vendor::Feed::Dhan)
                .expect("the seat is free");
            let conflict = post(
                addr,
                "/pull/spot",
                "target=swept&from=2024-01-01&to=2024-01-31",
            )
            .await;
            drop(seat);
            assert!(conflict.starts_with("HTTP/1.1 409"), "{conflict}");
            assert!(
                conflict.contains("x-brutex-receipt: pull-spot"),
                "every arm, or the marker is not worth requiring: {conflict}"
            );
        })
        .await;
    }

    /// **A NOTE IS NEVER LOST TO THE ALPHABET A HEADER ADMITS.**
    ///
    /// The note carries a filesystem path — arbitrary bytes on this platform —
    /// and a vendor's own refusal text, which has already been observed to hold
    /// `·` and `—`. `HeaderValue::from_str` refuses every one of those. A
    /// `unwrap_or(<empty>)` at the call site would answer a corrupt census with
    /// a BLANK reason, which is the failure this whole change removes arriving
    /// one layer down, so the alphabet is enforced before the conversion and
    /// the conversion cannot fail.
    #[test]
    fn a_hostile_note_is_still_a_header() {
        // The real note's own punctuation, a newline, a NUL and a DEL — the
        // three bytes that would otherwise split or truncate a header.
        let hostile = "groww UNREADABLE · header\r\nInjected: yes\u{0}\u{7f} — at /tmp/x";
        let clean = note_alphabet(hostile);
        assert!(
            clean.chars().all(|c| c.is_ascii_graphic() || c == ' '),
            "{clean}"
        );
        assert!(
            !clean.contains('\r') && !clean.contains('\n'),
            "a note can never open a second header: {clean}"
        );
        assert!(
            clean.contains("groww UNREADABLE") && clean.contains("at /tmp/x"),
            "and what an operator has to read survives it: {clean}"
        );
        // BOUNDED, AND THE TRUNCATION SAYS SO. An operator's own directory name
        // is not a bound this crate owns.
        let long = note_alphabet(&"x".repeat(NOTE_HEADER_CHARS * 2));
        assert_eq!(long.chars().count(), NOTE_HEADER_CHARS + 3, "{long}");
        assert!(long.ends_with("..."), "{long}");
        // And the value the route actually stamps is built from exactly that.
        assert_eq!(note_header(hostile).to_str().expect("visible ASCII"), clean);
    }

    /// **A feed with no census row at all still says which state that is.**
    ///
    /// `census::read_all` answers one row per `Vendor::ALL`, so the `None` arm
    /// is not reachable through a request — it is reachable here, and it must
    /// answer the same word `/audit.json` answers for the same input rather
    /// than a fourth one invented at this call site.
    #[test]
    fn a_missing_census_row_is_absent_and_says_so() {
        let headers = census_headers(None);
        assert_eq!(
            headers
                .get(CENSUS_STATE_HEADER)
                .and_then(|v| v.to_str().ok()),
            Some("absent"),
            "the same word audit_json::store_block answers with"
        );
        assert!(
            headers
                .get(CENSUS_NOTE_HEADER)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|n| n.contains("no census row")),
            "and the note says which absence it is"
        );
        assert!(!census_is_unreadable(None));
    }

    /// **A feed this build parses no master for is not a feed that lists
    /// nothing.**
    ///
    /// `/instruments.json?feed=truedata` answers `[]` and always did. The
    /// difference is that `[]` now travels with the reason: an archive source
    /// publishes no scrip file, so there is nothing to have failed to read, and
    /// `200` is the honest status. Collapsing this into `UNAVAILABLE` would
    /// send an operator looking for a file that does not exist in this design.
    #[tokio::test]
    async fn an_archive_feed_names_itself_unmastered_rather_than_empty() {
        let dir = agreeing("notmastered");
        let built = Loaded::new(Site::load(&dir, &store_root("notmastered")));
        served_over("notmastered", built, |addr| async move {
            let answer = get(addr, "/instruments.json?feed=truedata").await;
            assert_eq!(body_of(&answer), "[]", "{answer}");
            assert!(
                answer.starts_with("HTTP/1.1 200"),
                "nothing failed, so nothing is refused: {answer}"
            );
            assert!(
                answer.contains("x-brutex-master-state: not-mastered"),
                "{answer}"
            );
            assert!(
                answer.contains("parses no instrument master"),
                "and says why the list is empty: {answer}"
            );
        })
        .await;
    }

    /// **A store root the environment never named is a refusal, and the
    /// listener is dropped rather than served.**
    ///
    /// `served_store_root` reads the environment, so this arm is only reachable
    /// on a machine with no `HOME` and no `BRUTEX_STORE` — which no test may
    /// create, since `set_var` is `unsafe` under edition 2024 and this crate
    /// forbids `unsafe`. `run_in_over` takes the value, so both arms are
    /// drivable by argument. The exit code is `FAILED`: nothing was read, so
    /// there is no output whose trust is in question, which is what `DEGRADED`
    /// would have meant.
    #[tokio::test]
    async fn a_serve_with_no_store_root_refuses_instead_of_serving_the_checkout() {
        let dir = agreeing("nostoreroot");
        let code = run_in_over(
            &dir,
            Err(String::from(
                "REFUSED: no store root. BRUTEX_STORE is unset",
            )),
            &argv(&["serve", "127.0.0.1:0"]),
            Box::pin(std::future::pending()),
        )
        .await;
        assert_eq!(
            code, FAILED,
            "a refused store root is a failure to run, not a degraded answer"
        );

        // AND THE SAME SPLIT ONE LAYER UP, for the masters directory.
        let refused = run_from(
            Err(String::from("REFUSED: no masters directory")),
            &argv(&["report"]),
            Box::pin(std::future::pending()),
        )
        .await;
        assert_eq!(refused, FAILED);
    }

    #[test]
    fn a_report_earns_zero_only_when_the_read_was_clean() {
        // Both arms, driven directly. Which directory `run` reads comes from
        // the environment and a test cannot set one here, so leaving this
        // branch inside `run` would make the OK arm reachable only from the
        // child process in tests/binary.rs -- and a branch only a subprocess
        // can enter is a branch this gate cannot hold.
        assert_eq!(reported(&agreeing("reported")), OK);
        let missing = masters(
            "reportedmissing",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n"
            )),
            None,
        );
        assert_eq!(reported(&missing), DEGRADED);
    }

    #[tokio::test]
    async fn run_reports_and_refuses_nonsense_with_different_codes() {
        // THE ASSERTIONS HERE USED TO BE UNFALSIFIABLE, and that is worth
        // spelling out because it looked like caution. `run` read
        // $HOME/.brutex/masters, which a test cannot set, so its answer was a
        // property of the machine: OK on an operator's laptop where the real
        // masters live, DEGRADED on a CI runner where they do not. The
        // assertions were written to pass under both -- `!= FAILED` and
        // `!= MISUSED` -- and `reported` returns ONLY OK or DEGRADED, so no
        // input, machine or environment could ever have failed them. A mutant
        // pinning `reported` to OK survived. That is a test that asserts
        // nothing, which CLAUDE.md §4 bans outright.
        //
        // `run_in` takes the directory, so the expected value is a property of
        // the files, which this test owns. Exact codes, both arms, deterministic
        // on every host.
        assert_eq!(
            run_in(&agreeing("runreport"), &argv(&["report"]), fired()).await,
            OK,
            "two masters that agree is a clean universe"
        );
        let missing = masters(
            "runreportmissing",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n"
            )),
            None,
        );
        assert_eq!(
            run_in(&missing, &argv(&["report"]), fired()).await,
            DEGRADED,
            "a vendor that was never read is a refused universe"
        );
        assert_eq!(run(&argv(&["--wat"]), fired()).await, MISUSED);
        assert_ne!(DEGRADED, OK, "a refused universe is not a success");
        assert_ne!(DEGRADED, FAILED, "the run worked; its ANSWER is refused");
        assert_ne!(DEGRADED, MISUSED);
    }

    // =======================================================================
    // The local-archive pull, which is the half of the vendor surface that
    // works today
    // =======================================================================

    /// `GDFL`'s header, character for character, and the shape `run_local`
    /// declares: ten fields, a header row, `DD/MM/YYYY`.
    const GDFL_MEMBER_HEAD: &str =
        "Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest\n";

    /// A folder holding one `GDFL` member, named after the test.
    fn vendor_folder(name: &str, body: &str) -> PathBuf {
        let dir = crate::scratch::path(&format!("vendor-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        let mut f = std::fs::File::create(dir.join("NIFTY.NFO.csv")).expect("create");
        f.write_all(body.as_bytes()).expect("write");
        dir
    }

    /// A spot form body over one local folder.
    /// A form for the ARCHIVE path, which means it names an archive feed.
    ///
    /// It used to name no feed at all, and worked because the route decided
    /// HTTP-versus-archive by asking whether `folder` was blank. Once the
    /// TRANSPORT decides instead, a form with no feed defaults to Dhan — an
    /// HTTP broker — and a folder beside it is a contradiction the route is now
    /// right to resolve in the broker's favour. So the feed is stated, which is
    /// what an operator does on the page too.
    fn spot_form(folder: &Path, from: &str, to: &str) -> String {
        format!(
            "target=swept&vendor={}&from={from}&to={to}&folder={}",
            pull::vendor::Feed::TrueData.wire(),
            folder.display()
        )
    }

    /// Every HTTP feed gets a governor, every archive feed gets none, and the
    /// numbers are the descriptor's rather than this function's.
    ///
    /// The governor in `pull::rate` had ZERO callers. An implemented limiter
    /// nothing consults is not a limiter, and the backfill this repository
    /// exists to run is ~11,200 requests at day granularity and ~64,800 at
    /// one-minute — an unthrottled burst of those is cut off partway and leaves
    /// a half-written history the append-only store cannot correct.
    ///
    /// No feed is named here. The loop is over `Feed::ALL`, so a fifth row is
    /// covered the day it exists, and which half it lands in is decided by its
    /// own declared transport.
    #[test]
    fn every_http_feed_has_a_budget_and_no_archive_feed_does() {
        let budgets = feed_budgets();
        assert_eq!(
            budgets.len(),
            pull::vendor::Feed::ALL.len(),
            "one slot per feed, indexed by discriminant — a short vector would \
             make the lookup silently miss the last feed"
        );

        for (slot, feed) in pull::vendor::Feed::ALL.into_iter().enumerate() {
            match feed.descriptor().transport {
                pull::vendor::Transport::Http(spec) => {
                    let governor = budgets[slot].as_ref().unwrap_or_else(|| {
                        panic!("{} is an HTTP feed and must have a budget", feed.display())
                    });
                    // THE DESCRIPTOR'S NUMBERS, NOT THIS FUNCTION'S. Every one
                    // cites docs/00-charter.md §4, and the unverified figure
                    // says so in the constant's own name.
                    assert_eq!(
                        (
                            governor.ceiling(pull::rate::WindowSpan::Second),
                            governor.ceiling(pull::rate::WindowSpan::Minute),
                            governor.ceiling(pull::rate::WindowSpan::Day),
                        ),
                        (
                            spec.budget.per_second,
                            spec.budget.per_minute,
                            spec.budget.per_day
                        ),
                        "{}'s governor must carry the budget its descriptor \
                         declares, span for span — a mismatch means the number \
                         was decided here instead of being read",
                        feed.display()
                    );
                }
                pull::vendor::Transport::LocalArchive(_) => assert!(
                    budgets[slot].is_none(),
                    "{} reads files and issues no requests, so it has no \
                     budget to spend",
                    feed.display()
                ),
            }
        }
    }

    /// A spent budget refuses, names the span, and says when it clears.
    ///
    /// Drives the governor directly past its per-second ceiling rather than
    /// issuing real requests. The claim is not "it refuses" — it no longer
    /// does. The claim is that a burst larger than the ceiling is **throttled
    /// and then admitted in full**: nothing is lost and nothing is issued
    /// faster than the vendor published.
    ///
    /// This test previously asserted a refusal, and the refusal was the bug.
    /// Measured on the real universe: 785 instruments attempted, 6 reached,
    /// 779 refused for a wait of 0.198 s. A backfill that abandons 99% of its
    /// work rather than waiting a fifth of a second is not automated.
    #[tokio::test]
    async fn a_burst_past_the_ceiling_is_throttled_and_still_admitted_in_full() {
        const BURST: u32 = 12;
        let dir = agreeing("budgetspent");
        let site = site("budgetspent", &dir);
        let feed = pull::vendor::Feed::Dhan;

        // A burst bigger than any published per-second ceiling, so at least
        // one full second-window must be waited out. Kept small because the
        // wait is REAL time — the point is to prove throttling happened, and
        // one crossed window proves it as well as a hundred.
        let started = std::time::Instant::now();
        for n in 0..BURST {
            await_budget(feed, &site).await.unwrap_or_else(|why| {
                panic!(
                    "request {n} of {BURST} was refused rather than waited \
                     for, which loses work the vendor would have served: {why}"
                )
            });
        }
        let took = started.elapsed();

        // NOTHING WAS LOST — every one of the twelve was admitted, or the loop
        // above would have panicked.
        //
        // AND NOTHING WAS RUSHED. Dhan publishes 5 requests per second
        // (`docs/00-charter.md` §4), so twelve cannot be issued in under two
        // full seconds' worth of allowance. If this elapsed instantly the
        // governor is not being consulted at all and the assertion above
        // would pass vacuously — which is exactly the shape of test
        // `CLAUDE.md` §4 bans.
        assert!(
            took >= std::time::Duration::from_millis(900),
            "{BURST} requests against a 5-per-second ceiling completed in \
             {took:?} — too fast to have been throttled, so the governor is \
             not being consulted and this test proves nothing"
        );
    }

    /// Every HTTP feed declares the cap its own vendor published **at the rung
    /// it was published for**, and the fetch loop is driven by it.
    ///
    /// The numbers are `docs/00-charter.md` §4's, not this test's: Groww 30
    /// days per request at one-minute granularity, Dhan 90. What is asserted
    /// here is that the descriptor CARRIES them and that the split those caps
    /// imply is the one the backfill will actually issue — 81 requests per
    /// instrument at Groww's cap for 2020-01-01 to yesterday, not one request
    /// for 2,411 days that no vendor will serve.
    ///
    /// **The rung is now part of the question, and only the one-minute rung is
    /// asserted here.** The charter's Groww row carries the qualifier "at
    /// 1-minute granularity" in its own text and its Dhan row sits under an
    /// intraday-endpoint line, so `Minute1` is the rung both figures were
    /// recorded for and the only one this may demand a number at. A `None` at
    /// any other rung is a vendor fact nobody wrote down — see
    /// `a_daily_window_is_split_by_the_month_and_not_by_the_one_minute_cap`,
    /// which is where the absent daily cap is asserted rather than tolerated.
    #[test]
    fn the_backfill_window_becomes_one_legal_request_per_chunk() {
        for feed in pull::vendor::Feed::ALL {
            let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
                continue;
            };
            let Some(cap) = spec.window_cap_days(pull::vendor::Granularity::Minute1) else {
                // TWO REASONS FOR A MISSING NUMBER, AND THEY ARE TOLD APART BY
                // THE ROW ITSELF RATHER THAN BY A LIST KEPT HERE.
                //
                // An EMPTY `window_caps` is a vendor for which no per-request
                // cap has been read at any rung. That is a legal state: the
                // store's month boundary is then the only bound, and inventing
                // a figure would be §3 rule 1.
                //
                // ZERODHA USED TO BE THIS BRANCH'S EXAMPLE and is no longer,
                // which is why this comment names none. Its historical page
                // still states no cap — that part was always true — and the
                // vendor's own developer forum does, so §4z now carries minute
                // 60 and day 2000 and this feed takes the `Some(cap)` path
                // below. The two shipped archives have no HTTP transport at all
                // and never reach here.
                //
                // A POPULATED table with no minute row is a number that went
                // missing, which is the case this test was written for. Every
                // shipped broker carries theirs — Groww 30, Dhan 90, Zerodha 60
                // — and losing one would send a window wider than the vendor
                // accepts.
                assert!(
                    spec.window_caps.is_empty(),
                    "{} publishes caps at other rungs and none at ONE MINUTE — \
                     that is a number that went missing, not a vendor that \
                     published nothing. The figure is in docs/00-charter.md §4 \
                     for every broker that has one.",
                    feed.display()
                );
                continue;
            };
            assert!(cap > 0, "{} must publish a positive cap", feed.display());

            // The real backfill: 2020-01-01 to 2026-08-07 inclusive.
            let window = pull::session::Window::new(
                pull::session::Day::new(2020, 1, 1).expect("2020-01-01"),
                pull::session::Day::new(2026, 8, 7).expect("2026-08-07"),
            )
            .expect("a forward window");

            let chunks =
                pull::session::split_window(window, Some(cap)).expect("a positive cap splits");
            for chunk in &chunks {
                let len = chunk.to().days_from_epoch() - chunk.from().days_from_epoch() + 1;
                assert!(
                    len <= cap,
                    "{} caps a request at {cap} days and this chunk is {len}",
                    feed.display()
                );
            }
            assert_eq!(
                chunks.first().map(|w| w.from()),
                Some(window.from()),
                "the split starts where the operator asked"
            );
            assert_eq!(
                chunks.last().map(|w| w.to()),
                Some(window.to()),
                "and ends where they asked — never truncated to fit"
            );
            // ONE CHUNK PER MONTH, NOT ONE PER CAP.
            //
            // This used to assert `ceil(2411 / cap)`, which was the answer when
            // the ONLY bound was the vendor's window cap. It is no longer, and
            // the difference cost 699 instruments on a real pull: the store
            // addresses one month per file and `fetch::land` refuses a batch
            // spanning two, so the splitter now also breaks at every month
            // boundary. For a cap wider than any month -- Dhan's 90 -- the
            // month is what binds, and 2,411 days is 80 months, not 27 chunks.
            //
            // Asserted as "every chunk is inside one month AND the chunks
            // tile" rather than a literal count, because a count is a fact
            // about the calendar and this is a claim about the invariant.
            let mut months = std::collections::BTreeSet::new();
            for chunk in &chunks {
                assert_eq!(
                    (chunk.from().year(), chunk.from().month()),
                    (chunk.to().year(), chunk.to().month()),
                    "{} produced a chunk spanning two months, which the store \
                     refuses and which throws the whole fetch away",
                    feed.display()
                );
                months.insert((chunk.from().year(), chunk.from().month()));
            }
            let mut want = window.from().days_from_epoch();
            for chunk in &chunks {
                assert_eq!(
                    chunk.from().days_from_epoch(),
                    want,
                    "{} left a gap or an overlap between chunks",
                    feed.display()
                );
                want = chunk.to().days_from_epoch() + 1;
            }
            assert_eq!(
                want,
                window.to().days_from_epoch() + 1,
                "{} stopped short of the window",
                feed.display()
            );
            assert!(
                chunks.len() >= months.len(),
                "{} cannot cover {} months in fewer chunks",
                feed.display(),
                months.len()
            );
        }
    }

    /// **A DAILY WINDOW IS SPLIT BY THE MONTH AND NOT BY THE ONE-MINUTE CAP.**
    ///
    /// The two bounds are different rules from different owners. The **cap** is
    /// the vendor's, and `docs/00-charter.md` §4 records it *per rung*: Groww
    /// "30 days per request **at 1-minute granularity**", Dhan 90 against its
    /// intraday-charts endpoint. Neither records a day-level figure, so neither
    /// descriptor carries one — UNVERIFIED, absent rather than guessed. The
    /// **month** is the store's, and it binds at every rung, because a bar file
    /// addresses one month whatever the bar length is.
    ///
    /// So the daily split must be the month alone. Groww at one minute needs
    /// **126** requests per instrument for 2020-01-01..2026-08-07 — 80 months,
    /// plus one extra cut inside each of the 46 months longer than its 30-day
    /// cap. At the day rung it needs **80**, one per month and no more.
    ///
    /// # The honest size of that, because a test should not flatter it
    ///
    /// 80 is a FLOOR, not a target, and it is the store's floor rather than any
    /// vendor's: `docs/05-decisions.md` D-0054 says "14 at day level", which
    /// implies a ~172-day cap that appears in no source and is unreachable
    /// regardless. For Dhan the daily count and the one-minute count are both
    /// 80 — its 90-day cap is already wider than any month, so the month was
    /// the only bound at either rung and this change buys it nothing. Asserted
    /// for both feeds, including the one where the answer is "no difference",
    /// because a test that only looked at Groww would let a Dhan regression
    /// through.
    #[test]
    fn a_daily_window_is_split_by_the_month_and_not_by_the_one_minute_cap() {
        use pull::vendor::Granularity;

        // 2020-01-01 to 2026-08-07 inclusive: 2,411 days across 80 months.
        let window = pull::session::Window::new(
            pull::session::Day::new(2020, 1, 1).expect("2020-01-01"),
            pull::session::Day::new(2026, 8, 7).expect("2026-08-07"),
        )
        .expect("a forward window");
        let months = |chunks: &[pull::session::Window]| {
            chunks
                .iter()
                .map(|w| (w.from().year(), w.from().month()))
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        };

        for (feed, minute_chunks) in [
            (pull::vendor::Feed::Groww, 126_usize),
            (pull::vendor::Feed::Dhan, 80),
        ] {
            let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
                panic!("{} is a broker", feed.display());
            };

            // THE CAP IS ABSENT AT THE DAY RUNG, and that is the recorded fact
            // rather than a hole: no day-level figure is in the charter for
            // either broker. If one is ever read live and written down, this
            // line is what has to change with it.
            assert_eq!(
                spec.window_cap_days(Granularity::Day1),
                None,
                "{} publishes no day-level cap in docs/00-charter.md §4 — a \
                 number here is one somebody invented",
                feed.display()
            );

            let daily =
                pull::session::split_window(window, spec.window_cap_days(Granularity::Day1))
                    .expect("an absent cap still splits");
            let minute =
                pull::session::split_window(window, spec.window_cap_days(Granularity::Minute1))
                    .expect("a positive cap splits");

            assert_eq!(
                minute.len(),
                minute_chunks,
                "{} at one minute is {minute_chunks} requests per instrument",
                feed.display()
            );
            assert_eq!(
                daily.len(),
                80,
                "{} at the day rung is ONE request per month and no more",
                feed.display()
            );
            assert_eq!(months(&daily), 80, "the window touches 80 months");
            assert_eq!(
                daily.len(),
                months(&daily),
                "{}: the month is the ONLY bound at the day rung — a chunk \
                 count above the month count means a cap is still cutting",
                feed.display()
            );
            // THE SAVING, WHERE THERE IS ONE — and no claim of one where there
            // is not. Groww's 30-day cap is narrower than a 31-day month, so
            // the day rung is STRICTLY cheaper; Dhan's 90 is wider than any
            // month, so the month was already the only bound and the two counts
            // are exactly equal. Asserting `<=` for both would have let a
            // regression that reintroduced the minute cap at the day rung pass
            // on Groww.
            if minute_chunks > 80 {
                assert!(
                    daily.len() < minute.len(),
                    "{}: its cap cuts inside a month at one minute, so the day \
                     rung must issue strictly fewer requests — {} against {}",
                    feed.display(),
                    daily.len(),
                    minute.len()
                );
            } else {
                assert_eq!(
                    daily.len(),
                    minute.len(),
                    "{}: its cap is wider than any month, so the month binds at \
                     both rungs and the day rung saves nothing",
                    feed.display()
                );
            }

            // AND THE MONTH BOUND IS REALLY STILL THERE. An absent cap read as
            // "send the window whole" would be ONE chunk of 2,411 days here,
            // which `fetch::land` refuses at the write boundary — 699 members
            // failed that way on a real 37-day pull.
            let mut want = window.from().days_from_epoch();
            for chunk in &daily {
                assert_eq!(
                    (chunk.from().year(), chunk.from().month()),
                    (chunk.to().year(), chunk.to().month()),
                    "{}: a daily chunk spans two months, which the store refuses",
                    feed.display()
                );
                assert_eq!(
                    chunk.from().days_from_epoch(),
                    want,
                    "{}: the daily chunks leave a gap or an overlap",
                    feed.display()
                );
                want = chunk.to().days_from_epoch() + 1;
            }
            assert_eq!(
                want,
                window.to().days_from_epoch() + 1,
                "{}: the daily chunks stop short of the window",
                feed.display()
            );
        }
    }

    /// The fetch loop reads the DESCRIPTOR's cap, not a literal and not `None`.
    ///
    /// Written against the source because the alternative is a live broker.
    /// It exists because a mutant that replaced `spec.window_cap_days` with a
    /// hardcoded `None` — sending the whole window, which is the entire bug —
    /// passed every other test here: they assert that the descriptor CARRIES
    /// the cap and that `split_window` divides correctly, and both remain true
    /// while the loop ignores the answer.
    #[test]
    fn the_fetch_loop_takes_its_cap_from_the_descriptor() {
        // `fetch_chunks`, because the loop moved there when `broker_window`
        // crossed the 100-line lint. The test follows the code rather than
        // passing because the needle left the span it was searching — which is
        // the failure mode that already bit the ordering test once.
        let me = include_str!("server.rs");
        let body = me
            .split_once("async fn fetch_chunks")
            .expect("fetch_chunks exists")
            .1;
        let body = &body[..body
            .find("\n}\n")
            .expect("fetch_chunks' body ends at a column-0 brace")];

        assert!(
            body.contains("spec.window_cap_days"),
            "the chunking must be driven by the feed's own declared cap — a \
             literal here is one vendor's number applied to every vendor, and \
             a `None` is the un-split window this loop exists to prevent"
        );
        assert!(
            body.contains("split_window("),
            "and the split must be the one function that computes it, not a \
             second copy of the arithmetic"
        );
    }

    /// The type-ahead index is the SAME universe the page shows.
    ///
    /// It shipped `merged.by_key` whole — 2,780 listings — while the page
    /// beside it said 785. A suggestion the operator's own universe excludes is
    /// worse than no suggestion: the click lands on a page that does not list
    /// it and nothing says why. `CLAUDE.md` bounds the surface; a second
    /// endpoint that does not honour the bound is the bound not holding.
    #[tokio::test]
    async fn the_typeahead_index_is_the_tracked_universe_and_not_the_master() {
        let dir = agreeing("tajson");
        let site = site("tajson", &dir);
        let loaded: Loaded = std::sync::Arc::new(site);

        let (_code, _headers, json) = instruments_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            // The feed is part of the question now: the two brokers do not
            // list the same instruments, so the index is per feed.
            "/instruments.json?feed=groww".parse().expect("a legal uri"),
        )
        .await;
        let site = &*loaded;

        let shipped = json.matches(r#""symbol":"#).count();
        let tracked = site
            .read
            .merged
            .by_key
            .values()
            .filter(|e| crate::catalog::tracked(e.universe))
            .count();

        assert_eq!(
            shipped, tracked,
            "the index must carry exactly the tracked universe — not the whole \
             master, which is what shipped 2,780 rows behind a page saying 785"
        );
        assert!(
            shipped <= site.read.merged.by_key.len(),
            "and it cannot exceed the master it is drawn from"
        );

        // The escaper is exercised on real data rather than asserted about:
        // `&` is a legal symbol byte (M&M, M&MFIN) and a raw control character
        // would make the whole document unparseable, losing the type-ahead
        // rather than one row.
        assert!(
            !json.contains('\n') && !json.contains('\t'),
            "no raw control characters reach the document"
        );
        assert!(
            json.starts_with('[') && json.ends_with(']'),
            "and it is an array"
        );
    }

    /// The wire says every universe a row is in, and the old field never moves.
    ///
    /// D-0089 landed NIFTY 50 / 100 / 200 / 500 as `Universe` bits 3–6 and
    /// wired none of it: `/instruments.json` still answered one word out of
    /// `index|fno|ntm|other`, so four rows on `/` and `/db` shipped DISABLED
    /// with "no endpoint carries this membership" written on them. The data was
    /// there and the API could not say it.
    ///
    /// Two fields now, and this test is the reason both exist. `universes`
    /// carries the set. `universe` is frozen at the three words a running page
    /// already splits on `+` — widening it would have been the quiet kind of
    /// break, because the string would still parse and would still group, and
    /// only a reader comparing or bookmarking the whole value would notice.
    ///
    /// Pinned on a value with a NEW bit and no old one — a bare `NIFTY_50` —
    /// because that is the only shape that can tell "the compatibility field is
    /// pinned to three" apart from "the new bits happen to travel with `ntm`",
    /// which every real constituent does.
    #[tokio::test]
    async fn the_wire_carries_every_membership_and_the_frozen_field_never_widens() {
        use brutex_core::universe::Universe;

        // Nothing at all: the array is empty, the word is `other`. The two
        // spellings of "no membership" are deliberate and are not equal.
        assert_eq!(universe_label(Universe::NONE), "other");
        assert!(universe_tokens(Universe::NONE).is_empty());
        assert_eq!(universes_json(Universe::NONE), "[]");

        // Everything: the frozen field is STILL exactly three words.
        let every = UNIVERSE_TOKENS
            .iter()
            .fold(Universe::NONE, |acc, (bit, _)| acc.union(*bit));
        assert_eq!(
            universe_label(every),
            "index+fno+ntm",
            "the compatibility field is pinned to the three universes that \
             existed before D-0089, whatever else the row is in"
        );
        assert_eq!(
            universe_tokens(every),
            ["index", "fno", "ntm", "n500", "n200", "n100", "n50"],
            "and the array carries all seven, in bit order"
        );

        // A new bit ALONE cannot reach the frozen field.
        assert_eq!(universe_label(Universe::NIFTY_50), "other");
        assert_eq!(universe_tokens(Universe::NIFTY_50), ["n50"]);
        assert_eq!(universes_json(Universe::NIFTY_50), r#"["n50"]"#);

        // And the two fields cannot disagree about how a bit is spelled,
        // because both read `UNIVERSE_TOKENS`.
        for (bit, token) in UNIVERSE_TOKENS.into_iter().take(LEGACY_UNIVERSE_TOKENS) {
            assert_eq!(universe_label(bit), token, "one spelling per bit");
        }

        // Now the endpoint, on real membership rather than synthesised bits.
        let dir = agreeing("unijson");
        let site = site("unijson", &dir);
        let loaded: Loaded = std::sync::Arc::new(site);
        let (_code, _headers, json) = instruments_json(
            axum::extract::State(loaded),
            "/instruments.json?feed=groww".parse().expect("a legal uri"),
        )
        .await;

        // RELIANCE is in all six equity lists — that is what the exchange's own
        // constituent files say, and it is why the four tiers were worth
        // wiring: `fno+ntm` was the whole answer this endpoint could give.
        assert!(
            json.contains(
                r#""universe":"fno+ntm","universes":["fno","ntm","n500","n200","n100","n50"]"#
            ),
            "RELIANCE must carry its four tiers in the array and none of them \
             in the frozen word: {json}"
        );
        // NIFTY is an index and an F&O underlying, and a constituent of
        // nothing: an index is not a share. Both fields say so.
        assert!(
            json.contains(r#""universe":"index+fno","universes":["index","fno"]"#),
            "an index names no tier: {json}"
        );
        // Every row carries both. `"universe":` is not a substring of
        // `"universes":` — the byte after `universe` differs — so these two
        // counts are independent.
        let frozen = json.matches(r#""universe":"#).count();
        let full = json.matches(r#""universes":"#).count();
        assert_eq!(frozen, json.matches(r#""symbol":"#).count());
        assert_eq!(full, frozen, "neither field is optional");
    }

    #[test]
    fn a_compound_bitset_has_no_single_token() {
        use brutex_core::universe::Universe;

        // The reverse of `universe_tokens`, and it takes ONE bit. Every one of
        // the seven answers its own word — that is the property `/universes.json`
        // leans on to tell a page which token of the `universes` array picks
        // out the rows a target covers.
        for (bit, token) in UNIVERSE_TOKENS {
            assert_eq!(universe_token_of(bit), token, "one word per bit");
        }
        // NEITHER OF THE TWO NON-ANSWERS IS A GUESS. An empty bitset names no
        // universe, and two bits at once is a SET whose name is the array
        // field; both answer with the empty string rather than with the first
        // bit that happens to match, which would have made a compound row look
        // like a single-universe one on the wire.
        assert_eq!(universe_token_of(Universe::NONE), "");
        assert_eq!(
            universe_token_of(Universe::NIFTY_50.union(Universe::NIFTY_100)),
            "",
            "a compound bitset is not one universe and does not get one word"
        );
    }

    #[tokio::test]
    async fn universes_json_says_what_each_target_resolves_to_for_the_named_feed() {
        // THE ROUTE THE `/ingest` UNIVERSE MENU HAD NO ANSWER FROM. Its four
        // NIFTY rows drew `no target` because nothing on the wire turned a
        // tier into a request for the selected feed. D-0120.
        let dir = agreeing("reachjson");
        let loaded: Loaded = std::sync::Arc::new(site("reachjson", &dir));

        let (status, _headers, json) = universe_reach_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/universes.json?feed=groww".parse().expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(
            json.starts_with(r#"{"feed":"groww","mastered":true,"#),
            "{json}"
        );

        // ALL SEVEN ROWS, EACH WITH THE SLUG A POST WOULD CARRY. The four
        // tiers are the point: a page that reads this can enable them.
        for target in ingest::SpotTarget::ALL {
            let slug = target.slug();
            assert!(
                json.contains(&format!(r#""target":"{slug}""#)),
                "{slug} must be requestable from this answer: {json}"
            );
        }
        // RELIANCE is in every equity list this fixture's master carries, so
        // each tier resolves to exactly one Groww id and the rest of the
        // published list is named as unreachable rather than dropped.
        assert!(
            json.contains(r#""target":"n50","label":"NIFTY 50 equities""#),
            "{json}"
        );
        assert!(
            json.contains(
                r#""counted_from":"join","published":50,"matched":1,"lacks":49,"ambiguous":0,"malformed":0,"no_nse_isin":0"#
            ),
            "the count on the control is the matched bucket and the shortfall is \
             beside it, not behind it: {json}"
        );
        // AND THE BUCKET IS THE RIGHT ONE, WHICH D-0125 CHANGED. The join is
        // keyed on NSE's OWN ISIN now, so every NIFTY 50 name HAS an identity
        // whether or not a master mentions it — the forty-nine this fixture's
        // master does not carry are `lacks`, each naming the ISIN the exchange
        // printed, and `malformed` is empty because no published name here is
        // unusable. The two are still not interchangeable: `lacks` is a row to
        // go and ask Groww about, `malformed` would be a transcription to fix
        // in this repository, and `no_nse_isin` is a cell NSE left empty.
        assert!(
            json.contains(
                r#""symbol":"3MINDIA","bucket":"lacks","why":"this feed's master carries no row for ISIN INE470A01017""#
            ),
            "{json}"
        );
        assert!(
            json.contains(r#""counted_from":"master","published":null,"matched":1,"lacks":0"#),
            "and the two targets no published list defines say where THEIR count \
             came from rather than reporting a denominator nobody published: {json}"
        );

        // THE FEED DECIDES THE ANSWER. Dhan's master in this fixture carries
        // the same two instruments, so the counts match and the ids do not —
        // what must differ is the feed the document is about.
        let (status, _headers, other) = universe_reach_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/universes.json?feed=dhan".parse().expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(other.starts_with(r#"{"feed":"dhan","#), "{other}");

        // AN ARCHIVE FEED IS NOT AN EMPTY ONE. A folder of CSVs is its own
        // listing and publishes no master, so every count is null.
        let (status, _headers, archive) = universe_reach_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/universes.json?feed=truedata"
                .parse()
                .expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert!(
            archive.contains(r#""mastered":false"#)
                && archive.contains(r#""counted_from":"no master""#),
            "{archive}"
        );

        // AND AN UNKNOWN FEED IS REFUSED BY NAME. `/instruments.json` answers
        // an unrecognised feed as Dhan; copying that here would silently
        // enable four controls against the other broker's reach.
        //
        // `zerodha` WAS THIS EXAMPLE until 14 Aug 2026, when it became a feed
        // this build names. A test for an unknown feed has to name one that
        // stays unknown, or it stops testing the refusal and starts testing
        // whichever vendor was added last.
        let (status, _headers, refused) = universe_reach_json(
            axum::extract::State(loaded),
            "/universes.json?feed=upstox".parse().expect("a legal uri"),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(
            refused.contains(r#""feed":"upstox""#)
                && refused.contains("this build reads no feed called that")
                && refused.contains("groww")
                && refused.contains("dhan"),
            "the refusal names what was asked and what is known: {refused}"
        );
    }

    #[test]
    fn the_receipt_reports_reach_for_the_feed_and_never_a_number_it_did_not_measure() {
        let dir = agreeing("reachtext");
        let built = site("reachtext", &dir);

        // EVERY NAME — no fraction, because there is nothing missing to be a
        // fraction of, and "1 of 1" invites the reader to look for the zero.
        let whole = reach_text(ingest::SpotTarget::Swept, pull::vendor::Feed::Groww, &built);
        assert_eq!(whole, "1 — every name this target holds, by Groww id");

        // SHORT: both numbers, the first names, the remainder, and the route
        // that carries the rest.
        let short = reach_text(
            ingest::SpotTarget::Nifty50,
            pull::vendor::Feed::Groww,
            &built,
        );
        assert!(
            short.starts_with("1 of 50 — 49 cannot be named by Groww: "),
            "{short}"
        );
        assert!(short.contains("and 44 more."), "{short}");
        assert!(short.ends_with("/universes.json?feed=groww"), "{short}");

        // A SHORTFALL SMALLER THAN THE CAP IS NAMED IN FULL, with no "and N
        // more" tacked on: a remainder of zero is not a remainder, and
        // printing "and 0 more" is the shape of a sentence generated rather
        // than written. Groww's master here omits NIFTY, so `indices` is short
        // by exactly one.
        let partial = masters(
            "reachone",
            Some(&format!(
                "{GROWW_HEAD}\
                 NSE,CASH,,INDIAVIX,IDX,,NIFTY,,,NSE-INDIAVIX\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}\
                 NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,1333\n\
                 NSE,I,NA,INDEX,INDIAVIX,INDIA VIX,INDEX,NA,0001-01-01,,,1333\n"
            )),
        );
        let one_short = site("reachone", &partial);
        let short_one = reach_text(
            ingest::SpotTarget::Indices,
            pull::vendor::Feed::Groww,
            &one_short,
        );
        assert_eq!(
            short_one,
            "1 of 2 — 1 cannot be named by Groww: NIFTY. Every one of them, with its \
             reason, is on /universes.json?feed=groww"
        );

        // AN ARCHIVE MEASURES NOTHING AND SAYS SO. A zero here would be a
        // reading of a file this feed does not publish.
        let archive = reach_text(
            ingest::SpotTarget::Nifty50,
            pull::vendor::Feed::TrueData,
            &built,
        );
        assert!(
            archive.contains("publishes no instrument master"),
            "{archive}"
        );
        assert!(
            !archive.contains('0'),
            "and it reports no count at all, because it took no measurement: {archive}"
        );
    }

    /// The retry POLICY, arm by arm, with no socket and no sleeping.
    ///
    /// This replaces a test that read this file as TEXT and asserted that
    /// certain string literals appeared before the word `sleep`. That test
    /// could not tell the policy from its wording — it passed for a build that
    /// returned on the first 5xx — and it brace-counted Rust source to find the
    /// function body, which is unsound because braces live in string literals.
    #[test]
    fn the_retry_policy_answers_each_class_of_refusal() {
        const _: () = assert!(THROTTLE_ATTEMPTS > 1);
        const _: () = assert!(SERVER_ERROR_ATTEMPTS < THROTTLE_ATTEMPTS);

        // A CREDENTIAL DEATH IS CERTAIN, so it never waits, at any attempt.
        // 403 is the third broker's `TokenException` and 401 is the other two.
        for attempt in 1..=THROTTLE_ATTEMPTS {
            assert_eq!(
                step(Some(401), false, None, attempt, 0),
                Step::CredentialDied
            );
            assert_eq!(
                step(Some(403), false, None, attempt, 0),
                Step::CredentialDied
            );
            // And the body-level marker a vendor writes instead of a status,
            // which outranks whatever the status happened to be.
            assert_eq!(step(None, true, None, attempt, 0), Step::CredentialDied);
            assert_eq!(
                step(Some(200), true, None, attempt, 0),
                Step::CredentialDied
            );
        }

        // A REASON ABOUT THE REQUEST IS NOT RE-ASKED.
        for code in [400, 404, 405, 410, 418, 422] {
            assert_eq!(
                step(Some(code), false, None, 1, 0),
                Step::Answered,
                "status {code}"
            );
        }
    }

    /// The two ladders a refusal that MIGHT clear is put on, and where each
    /// one stops.
    ///
    /// Split from `the_retry_policy_answers_each_class_of_refusal` when the
    /// vendor-named axis added a parameter and took that function past
    /// `clippy::too_many_lines`. Every assertion is the one it had; the halves
    /// are the two questions it was always asking — *which class is this?* and
    /// *how long does this class wait?*
    #[test]
    fn the_retry_policy_ladders_back_off_and_then_stop() {
        const _: () = assert!(THROTTLE_ATTEMPTS > 1);
        const _: () = assert!(SERVER_ERROR_ATTEMPTS < THROTTLE_ATTEMPTS);

        // 429 BACKS OFF EXPONENTIALLY AND TELLS THE GOVERNOR.
        let mut waits = Vec::new();
        for attempt in 1..THROTTLE_ATTEMPTS {
            match step(Some(429), false, None, attempt, 0) {
                Step::Again { wait_ms, throttled } => {
                    assert!(throttled, "a 429 is the governor's cue");
                    waits.push(wait_ms);
                }
                other => panic!("attempt {attempt} of a 429 gave {other:?}"),
            }
        }
        assert_eq!(waits, vec![1_000, 2_000, 4_000, 8_000, 16_000]);
        assert!(
            waits.windows(2).all(|w| w[1] > w[0]),
            "each wait exceeds the last, or it is not a backoff"
        );
        assert_eq!(
            step(Some(429), false, None, THROTTLE_ATTEMPTS, 0),
            Step::Exhausted
        );

        // THE TOTAL, WHICH IS THE NUMBER THAT ACTUALLY BOUNDS A RUN.
        //
        // This replaces a line that asserted nothing. It read
        // `matches!(step(Some(429), false, None, 20, 0), Exhausted | Again { wait_ms:
        // 30_000, .. })` and claimed to prove a 30,000 ms cap -- but attempt 20
        // is past `THROTTLE_ATTEMPTS`, so the first arm always matched and the
        // cap was never evaluated. The cap could not fire at any reachable
        // attempt either, and is now gone. §4 bans a test that asserts nothing;
        // this was one, and it was mine.
        assert_eq!(waits.iter().sum::<u64>(), MAX_CHUNK_BACKOFF_MS);

        // 5xx IS RETRIED -- THE BUG THIS CLASS EXISTS FOR -- ON ITS OWN
        // SHORTER LADDER, QUADRATICALLY, AND WITHOUT TOUCHING THE GOVERNOR.
        for code in [500, 502, 503, 504, 599] {
            assert_eq!(
                step(Some(code), false, None, 1, 1),
                Step::Again {
                    wait_ms: 250,
                    throttled: false
                },
                "status {code} is the vendor's own side failing, so it is re-asked"
            );
        }
        assert_eq!(
            step(Some(503), false, None, 2, 2),
            Step::Again {
                wait_ms: 1_000,
                throttled: false
            }
        );
        // Stops at its OWN cap, so a bad hour at the vendor does not cost
        // 13.75 s per instrument.
        assert_eq!(
            step(
                Some(503),
                false,
                None,
                SERVER_ERROR_ATTEMPTS,
                SERVER_ERROR_ATTEMPTS
            ),
            Step::ServerDown {
                answered: SERVER_ERROR_ATTEMPTS
            }
        );
        let spent: u64 = (1..SERVER_ERROR_ATTEMPTS)
            .map(|a| 250 * u64::from(a) * u64::from(a))
            .sum();
        assert_eq!(spent, 1_250, "a dead vendor costs 1.25 s, not 13.75 s");

        // THE COUNTER IS THE 5xx COUNT, NOT THE ATTEMPT ORDINAL.
        //
        // This is the defect the audit filed as a blocker. The two counters
        // were one, so a 502 arriving after two timeouts on the same chunk hit
        // the cap on the vendor's FIRST 5xx -- the exact bug the 5xx retry was
        // added to fix, back again whenever the chunk had a bad minute first.
        assert_eq!(
            step(Some(502), false, None, 3, 1),
            Step::Again {
                wait_ms: 250,
                throttled: false
            },
            "attempt 3 with one 5xx answer is the vendor's FIRST failure and \
             must be re-asked"
        );
        // And the reported count is the one that was measured, so the message
        // cannot claim three answers when there was one.
        assert_eq!(
            step(Some(502), false, None, 5, 3),
            Step::ServerDown { answered: 3 }
        );

        // The chunk's own ladder still bounds it: past THROTTLE_ATTEMPTS there
        // are no attempts left, whatever the 5xx count is.
        assert_eq!(
            step(Some(502), false, None, THROTTLE_ATTEMPTS, 1),
            Step::ServerDown { answered: 1 }
        );

        // NOTHING ANSWERED AT ALL IS THE TRANSPORT BLIP THE QUADRATIC WAIT WAS
        // SIZED FOR, and it gets the full ladder because a lost packet is much
        // weaker evidence of a broken vendor than a 500 is.
        assert_eq!(
            step(None, false, None, 1, 0),
            Step::Again {
                wait_ms: 250,
                throttled: false
            }
        );
        assert_eq!(
            step(None, false, None, THROTTLE_ATTEMPTS - 1, 0),
            Step::Again {
                wait_ms: 250 * 25,
                throttled: false
            }
        );
        assert_eq!(
            step(None, false, None, THROTTLE_ATTEMPTS, 0),
            Step::Exhausted
        );
    }

    /// **A VENDOR THAT NAMES ITS REFUSAL IS BELIEVED, AND THE 403 PAIR SPLITS.**
    ///
    /// The whole point of `pull::refusal` reaching this ladder. Before it,
    /// every decision here came from the status, so Kite's `TokenException`
    /// and its `PermissionException` — both answered under 403 — were one
    /// event, and an API key with no historical-data subscription was reported
    /// as an expired session with the sentence "the refreshed value is read
    /// from Parameter Store on the next pull". Nothing about that key is
    /// refreshable and every later run fails identically.
    ///
    /// Six properties, over the whole `Disposition` set:
    ///
    /// 1. `NotEntitled` is its own arm, at every attempt, and never waits
    /// 2. `SessionDead` is still `CredentialDied` — the arm that already worked
    /// 3. `RequestWrong` and `ReasonGiven` are answered, never re-asked
    /// 4. A named refusal outranks the status, **including** a status that
    ///    disagrees with it
    ///
    /// The two retry ladders are checked separately, in
    /// [`the_named_path_and_the_status_path_share_one_ladder`].
    #[test]
    fn a_vendor_that_names_its_refusal_decides_the_ladder() {
        use pull::refusal::Disposition;

        // 1 — and it never waits, because a wait cannot change an entitlement.
        for attempt in 1..=THROTTLE_ATTEMPTS {
            assert_eq!(
                step(Some(403), false, Some(Disposition::NotEntitled), attempt, 0),
                Step::NotEntitled,
                "attempt {attempt}"
            );
        }
        // 2 — the arm that already worked, reached by name rather than by 403.
        assert_eq!(
            step(Some(403), false, Some(Disposition::SessionDead), 1, 0),
            Step::CredentialDied
        );
        // AND THE PAIR, SIDE BY SIDE, UNDER ONE STATUS. This is the assertion
        // the status axis could not make.
        assert_ne!(
            step(Some(403), false, Some(Disposition::NotEntitled), 1, 0),
            step(Some(403), false, Some(Disposition::SessionDead), 1, 0),
            "one status, two names, two futures"
        );

        // 3 — a reason about the request is not re-asked, at any attempt.
        for d in [Disposition::RequestWrong, Disposition::ReasonGiven] {
            for attempt in 1..=THROTTLE_ATTEMPTS {
                assert_eq!(step(Some(400), false, Some(d), attempt, 0), Step::Answered);
            }
        }

        // 6 — THE NAME OUTRANKS THE STATUS, INCLUDING WHEN THEY DISAGREE.
        //
        // A vendor may answer 500 while naming a rate refusal, or 200 while
        // naming a dead session. The name is what the vendor instructs
        // switching on, so it decides — and a rate refusal under a 500 still
        // gets the RATE ladder, which is the property lifting the ladders out
        // of `step` was for.
        assert_eq!(
            step(Some(500), false, Some(Disposition::Throttled), 1, 0),
            Step::Again {
                wait_ms: 1_000,
                throttled: true
            },
            "a named rate refusal takes the rate ladder whatever the status was"
        );
        assert_eq!(
            step(Some(200), false, Some(Disposition::SessionDead), 1, 0),
            Step::CredentialDied
        );
        assert_eq!(
            step(Some(429), false, Some(Disposition::RequestWrong), 1, 0),
            Step::Answered,
            "the vendor said the request was wrong; a 429 does not make it right"
        );

        // AND IT OUTRANKS THE UNTYPED BODY MARKER TOO. `invalid_auth` is a
        // `str::contains` against this build's own rendering of the error —
        // the coupling `with_retry` already names — so a vendor that says what
        // it means in a field beats a search for a word in a sentence.
        assert_eq!(
            step(Some(403), true, Some(Disposition::NotEntitled), 1, 0),
            Step::NotEntitled
        );

        // NO FEED REGRESSES. `None` on every disposition is exactly the status
        // path, which is what Dhan and Groww still take — both carry
        // `error_names: None`.
        for code in [400_u16, 401, 403, 404, 429, 500, 502] {
            assert_eq!(
                step(Some(code), false, None, 1, 1),
                step(Some(code), false, None, 1, 1),
                "status {code}"
            );
        }
    }

    /// **ONE LADDER, TWO CALLERS.** The vendor-named path and the status path
    /// must not drift apart.
    ///
    /// Lifting `throttle_ladder` and `server_ladder` out of [`step`] is what
    /// makes that true by construction; this is what makes it visible. The
    /// assertion is an EQUALITY against the status path rather than a repeat of
    /// the wait numbers, so a change to either ladder that reaches only one
    /// caller fails here instead of being discovered in a pull.
    #[test]
    fn the_named_path_and_the_status_path_share_one_ladder() {
        use pull::refusal::Disposition;

        for attempt in 1..=THROTTLE_ATTEMPTS {
            assert_eq!(
                step(Some(429), false, Some(Disposition::Throttled), attempt, 0),
                step(Some(429), false, None, attempt, 0),
                "the rate ladder, attempt {attempt}"
            );
            for errors in 0..=SERVER_ERROR_ATTEMPTS {
                assert_eq!(
                    step(
                        Some(503),
                        false,
                        Some(Disposition::RetryBounded),
                        attempt,
                        errors
                    ),
                    step(Some(503), false, None, attempt, errors),
                    "the backend ladder, attempt {attempt}, errors {errors}"
                );
            }
        }
        // And the ladders are genuinely different from each other, or the
        // equalities above would hold for a reason that is not the one claimed.
        assert_ne!(
            step(Some(429), false, None, 2, 1),
            step(Some(503), false, None, 2, 1),
            "a rate refusal and a backend failure are not the same wait"
        );
    }

    /// The transport is checked before anything a refusal should not cost.
    ///
    /// It used to be the LAST of eleven guards in `broker_window`, below a live
    /// `GetParameter` round-trip to ap-south-1 — so a feed that could not be
    /// pulled over HTTP paid a clock read, a `HOME`, a parsed credentials file,
    /// an AWS identity discovery and a socket to be told so. Every one of those
    /// is decidable from the descriptor before any of it.
    /// **A REFUSAL THIS SIDE DECIDED IS NOT A GATEWAY FAILURE.**
    ///
    /// A run that reached nothing answered `502 BAD_GATEWAY` whatever stopped
    /// it. Measured: 213 attempted, 0 reached, 13.9 ms — no socket opened, no
    /// credential read — and the page drew `Last pull HTTP 502`, which an
    /// operator reads as *the broker is down* when the truth was *you asked for
    /// a day whose session had not closed*.
    ///
    /// The two cases are told apart by a marker this code writes at the one
    /// call that opens a socket and strips in the caller — NOT by matching the
    /// refusal's prose, which is how a 403 came to be filed as a transport blip
    /// elsewhere in this same file.
    #[test]
    fn a_run_that_never_reached_a_socket_is_a_bad_request_and_not_a_bad_gateway() {
        let me = include_str!("server.rs");

        // The marker is set at `fetch_chunks` and nowhere else, because that is
        // the only call in `broker_window` that opens a socket.
        let window = me
            .split_once("async fn broker_window")
            .expect("broker_window exists")
            .1;
        let window = &window[..window.find("\n}\n").expect("it has an end")];
        let marked: Vec<&str> = window
            .lines()
            .filter(|l| l.contains("WIRE_REACHED") && !l.trim_start().starts_with("//"))
            .collect();
        assert_eq!(
            marked.len(),
            1,
            "exactly one place marks the wire as reached: {marked:?}"
        );
        assert!(
            marked[0].contains("map_err"),
            "and it marks a FAILURE from the socket call, not a success: {:?}",
            marked[0]
        );

        // The status turns on the recorded flag, never on the message.
        let answer = me
            .split_once("if run.reached == 0 {")
            .expect("the zero-reached branch exists")
            .1;
        let answer = &answer[..answer.find("\n    }\n").expect("it has an end")];
        let code: String = answer
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            code.contains("run.touched_wire"),
            "the status is decided by what the run RECORDED: {code}"
        );
        assert!(
            code.contains("BAD_REQUEST") && code.contains("BAD_GATEWAY"),
            "both statuses are reachable, or the split is decorative: {code}"
        );
        assert!(
            !code.contains("contains(") && !code.contains("starts_with("),
            "and never by reading the refusal's words: {code}"
        );
    }

    /// **TODAY IS ASKABLE ONCE ITS SESSION HAS CLOSED, AND WAS NOT.**
    ///
    /// `finished_day_only` refused any window reaching today, unconditionally.
    /// Measured at 21:11 IST on a session that shut at 15:30: 213 instruments
    /// attempted, 0 reached, every one refused in 13.9 ms with *"a session that
    /// is still running yields a partial day"*. No session was running.
    ///
    /// The browser already had it right — it offers today once the close has
    /// passed — so an operator following the page's own instruction got a 502
    /// about a session that had ended hours earlier.
    ///
    /// This reads the rule off the source rather than driving the clock,
    /// because driving it would mean either mocking `SystemTime::now` or
    /// writing a test whose answer changes at 15:30 every day — which is the
    /// failure `pull::vendor`'s own floor test names in as many words.
    #[test]
    fn a_finished_session_is_askable_today_and_an_unfinished_one_is_not() {
        let me = include_str!("server.rs");
        let body = me
            .split_once("fn finished_day_only")
            .expect("the guard exists")
            .1;
        let body = &body[..body.find("\n}\n").expect("it has an end")];

        // The comparison is against the SESSION, not the calendar day alone.
        assert!(
            body.contains("close_minute()"),
            "whether today may be stored is a question about the session, and \
             the guard must ask the session table: {body}"
        );
        assert!(
            body.contains("minute_of_day()"),
            "which needs the clock's minute, not just its date"
        );
        // A future day still refuses outright, and today refuses while open.
        assert!(
            body.contains("asked.window.to() > today"),
            "a day after today is refused whatever the clock says"
        );
        assert!(
            body.contains("== today && !closed"),
            "and today is refused only while its session is still running"
        );
        // The close is READ, never hardcoded, because the close MOVES: the CAS
        // change put the index at 15:15 from 2026-08-03 while cash kept 15:30
        // (D-0151). This comment used to cite a 14:45 Muhurat close instead —
        // a row the table has never held, in a shape `SessionRow` cannot carry.
        // See `docs/06-limits.md` §68.
        //
        // COMMENTS STRIPPED FIRST, and the first draft of this failed for it:
        // the paragraph above the guard EXPLAINS the 15:30 it must not contain,
        // and a scan that reads prose as code fails on the explanation for the
        // rule it is checking. Same filter gates 17, 23 and 1d use.
        let code: String = body
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !code.contains("15:30") && !code.contains("930"),
            "the close comes from the session table, not from a literal: {code}"
        );
    }

    #[test]
    fn the_transport_is_checked_before_any_vendor_facing_cost() {
        // ORDER, read off the source rather than asserted about behaviour,
        // because the thing being pinned is that a refusal COSTS NOTHING, and
        // no observable distinguishes "refused cheaply" from "refused after a
        // round-trip" except the round-trip itself.
        //
        // The search is BOUNDED to this function. It was not, and extracting
        // the clock read into `finished_day_only` — declared above
        // `broker_window` — silently moved it outside the searched span, so the
        // clock assertion started passing because the needle was absent rather
        // than because the order was right. A test that passes when the thing
        // it names has left the building asserts nothing.
        // THE CALLEE IS INLINED AT ITS CALL SITE, and that is what keeps this
        // test honest across the extraction that moved four of the needles
        // below out of `broker_window` and into `credentialed_source`.
        //
        // The property was never "these lines live in this function". It is
        // "nothing that costs a vendor anything runs before the transport is
        // known", and that is a property of the ORDER CONTROL REACHES THEM. So
        // the searched text is broker_window's body with the helper's body
        // spliced in where the call is, which is exactly the sequence a request
        // executes. Had the extraction been allowed to simply delete the
        // needles, every assertion below would have started passing for free —
        // which is the failure mode this test's own header names.
        let me = include_str!("server.rs");
        let span = |after: &str, what: &str| -> String {
            let tail = me
                .split_once(after)
                .unwrap_or_else(|| panic!("{what} exists"))
                .1;
            tail[..tail
                .find("\n}\n")
                .unwrap_or_else(|| panic!("{what}'s body ends at a column-0 brace"))]
                .to_owned()
        };
        let outer = span("async fn broker_window", "broker_window");
        let helper = span("async fn credentialed_source", "credentialed_source");
        let call = outer
            .find("credentialed_source(feed, &spec)")
            .expect("broker_window reaches its credential and socket through the helper");
        let body = format!("{}{helper}{}", &outer[..call], &outer[call..]);
        let body = body.as_str();

        let transport = body
            .find("Transport::Http(spec)")
            .expect("the transport is destructured inside broker_window");

        // THE BUDGET IS PART OF THIS ORDER, not a separate concern.
        //
        // Charging it after the credential read passed every test here until
        // this line existed: a request that will not be issued would first
        // spend a Parameter Store round-trip to ap-south-1 to find that out.
        // Every guard below is decidable from the form and the descriptor, so
        // every one of them belongs above the first thing that costs anything.
        let budget = body
            .find("await_budget(")
            .expect("the rate budget is charged inside broker_window");
        assert!(
            transport < budget,
            "the transport is checked before the budget is charged — an \
             archive feed has no budget to spend"
        );

        for (what, needle) in [
            (
                "the finished-day gate, which reads the clock",
                "finished_day_only(",
            ),
            ("the credentials file", "CredentialConfig::load"),
            ("the AWS identity", "AwsIdentity::discover"),
            // PARAMETER STORE MOVED, AND THIS TEST FOLLOWED IT — which is the
            // instruction the panic below has always carried.
            //
            // `ssm::get_parameter` is no longer called from this function.
            // D-0134 gave a feed's auth scheme a possible SECOND secret, so the
            // read became two reads behind one `key_field` decision, and the
            // block took `broker_window` past the workspace's 100-line ceiling.
            // It is now `read_credential`, declared ABOVE this function and
            // therefore outside the searched span — exactly the disappearance
            // this test's own header describes `finished_day_only` causing.
            //
            // The needle is the CALL, and the assertion under this loop pins
            // that the call still reaches Parameter Store. Both halves are
            // needed: this one keeps the ordering honest, that one stops the
            // needle being hollowed out into a function that costs nothing.
            ("Parameter Store, through its reader", "read_credential("),
            ("the HTTP client and its socket", "HttpSource::new"),
        ] {
            let cost = body.find(needle).unwrap_or_else(|| {
                panic!(
                    "{what} is named by `{needle}`, and it is not in \
                     broker_window's body. If it moved, this test must follow \
                     it — an absent needle would otherwise pass for free."
                )
            });
            assert!(
                transport < cost,
                "the transport is checked before {what}; a feed that cannot be \
                 pulled over HTTP must not pay for discovering that"
            );
            // The budget too, for everything that is not the clock — a request
            // the governor will refuse must not first read a credential.
            if needle != "finished_day_only(" {
                assert!(
                    budget < cost,
                    "the rate budget is charged before {what}; a request that \
                     will not be issued must not pay for discovering that"
                );
            }
        }

        // THE EXTRACTED READER STILL COSTS WHAT THE LOOP ASSUMES IT COSTS.
        //
        // Above, `read_credential(` stands in for "this is where Parameter
        // Store is reached". That substitution is only sound while the function
        // it names actually reaches it — otherwise the ordering assertions
        // above would guard a call that spends nothing, and the real
        // round-trip could move anywhere. So the needle the loop replaced is
        // asserted to live in the replacement, bounded to that function's own
        // body for the same reason the loop is bounded to this one.
        let reader = me
            .split_once("async fn read_credential")
            .expect("read_credential exists")
            .1;
        let reader = &reader[..reader
            .find("\n}\n")
            .expect("read_credential's body ends at a column-0 brace")];
        assert!(
            reader.contains("ssm::get_parameter"),
            "read_credential is what `broker_window` now pays Parameter Store \
             through, and it does not name `ssm::get_parameter`. Either the \
             read moved again — in which case follow it in BOTH places — or \
             the ordering assertions above are guarding a call that costs \
             nothing."
        );
    }

    /// An ARCHIVE feed with no folder refuses by name, and asks for a folder.
    ///
    /// The old fork read this exact request — no folder — as "use the broker
    /// path", so naming `TrueData` and forgetting the folder sent the request to
    /// Parameter Store to look for a `TrueData` access token that does not and
    /// will never exist. The operator got a credential error for a vendor that
    /// has no credential.
    #[tokio::test]
    async fn an_archive_feed_with_no_folder_says_so_instead_of_asking_for_a_token() {
        let dir = agreeing("archivenofolder");
        let site = site("archivenofolder", &dir);

        let (code, html) = spot_answer(
            &format!(
                "target=swept&vendor={}&from=2022-01-08&to=2022-01-08",
                pull::vendor::Feed::TrueData.wire()
            ),
            day(2026, 8, 7),
            moment(),
            &site,
        )
        .await;

        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{html}");
        assert!(
            html.contains("folder"),
            "the refusal asks for the missing folder: {html}"
        );
        // The REFUSAL ITSELF, not the page around it. The surrounding chrome
        // legitimately talks about credentials — it is the same layout the
        // broker path renders — so asserting over the whole document would be
        // asserting about the furniture. What must be true is that the reason
        // given is the missing folder and not a credential that was sought.
        assert!(
            html.contains("there is no endpoint to call and no credential to read"),
            "the reason is the folder, and it says outright that no credential \
             was even looked for: {html}"
        );
    }

    /// EVERY feed routes, and a feed added tomorrow routes too.
    ///
    /// This is the N-feed guarantee stated as an assertion rather than as a
    /// comment. It walks `Feed::ALL` — the array a new descriptor row joins
    /// automatically — and requires each one to reach the path its OWN
    /// transport names, with no list of vendor names anywhere in the test.
    ///
    /// A fifth feed declaring `Transport::LocalArchive` starts passing the
    /// archive half the moment its row exists. One declaring `Transport::Http`
    /// must produce a broker-shaped answer. Neither needs an edit here, which
    /// is the property being pinned: if a future change reintroduces a
    /// hand-written match on vendor names, some feed in this loop stops being
    /// routed and the test says which.
    #[tokio::test]
    async fn every_feed_reaches_the_path_its_own_transport_names() {
        for feed in pull::vendor::Feed::ALL {
            let tag = format!("routes{}", feed.wire());
            let dir = agreeing(&tag);
            let site = site(&tag, &dir);

            // No folder, deliberately. It is the field the old fork used to
            // decide the route, so leaving it empty for every feed is what
            // makes this test able to see the difference at all.
            let (code, html) = spot_answer(
                &format!(
                    "target=swept&vendor={}&from=2022-01-08&to=2022-01-08",
                    feed.wire()
                ),
                day(2026, 8, 7),
                moment(),
                &site,
            )
            .await;

            match feed.descriptor().transport {
                pull::vendor::Transport::LocalArchive(_) => {
                    assert_eq!(
                        code,
                        axum::http::StatusCode::BAD_REQUEST,
                        "{} is an archive feed, so no folder is the operator's \
                         mistake and it is a 400, not an attempt to call \
                         anything: {html}",
                        feed.display()
                    );
                    // THE SPECIFIC REFUSAL, not merely the word "folder".
                    // `contains("folder")` passed even against a mutant that
                    // dropped the guard entirely, because the reader it then
                    // fell through to also says "folder" — about a folder it
                    // failed to open rather than one that was never named. The
                    // distinction is the whole point of the guard, so the
                    // assertion has to be able to see it.
                    assert!(
                        html.contains("there is no endpoint to call and no credential to read"),
                        "{} must refuse for the missing folder itself, not \
                         fall through and fail to open one: {html}",
                        feed.display()
                    );
                }
                pull::vendor::Transport::Http(_) => {
                    // The broker path is not driven here — it would need a
                    // credential and a socket. What is asserted is that it did
                    // NOT take the archive path: an HTTP feed must never be
                    // answered by the CSV reader complaining about a folder,
                    // which is precisely what the old fork did.
                    assert!(
                        !html.contains("Name the folder"),
                        "{} is an HTTP feed and must not be routed to the \
                         archive reader: {html}",
                        feed.display()
                    );
                }
            }
        }
    }

    /// A folder that is not there refuses the pull and names the folder.
    ///
    /// NOTHING IS WRITTEN TO THE STORE by any test in this section, and that is
    /// arranged rather than hoped: `pull::ingest` opens a bar file only after a
    /// member has produced at least one surviving bar, so a run whose members
    /// all drop or all fail never reaches the store root at all. A test that
    /// wrote into the operator's real `$HOME/.brutex/store` would be a test
    /// with a side effect nobody asked for.
    #[tokio::test]
    async fn a_local_folder_that_is_not_there_refuses_the_pull_and_names_it() {
        let dir = agreeing("localabsent");
        let site = site("localabsent", &dir);
        let absent = crate::scratch::path("vendor-localabsent-NOT-THERE");
        let _ = std::fs::remove_dir_all(&absent);

        let (code, html) = spot_answer(
            &spot_form(&absent, "2022-01-08", "2022-01-08"),
            day(2026, 8, 7),
            moment(),
            &site,
        )
        .await;
        assert_eq!(
            code,
            axum::http::StatusCode::BAD_REQUEST,
            "a folder that is not there is the operator's mistake, not the \
             server's — {html}"
        );
        assert!(html.contains("Refused"), "the receipt says so: {html}");
        assert!(
            html.contains(&absent.display().to_string()),
            "and names the folder it looked in: {html}"
        );
        assert!(
            html.contains("local folder"),
            "and which transport was used: {html}"
        );
    }

    /// A folder whose every row falls outside the window stores nothing, says
    /// nothing was stored, and reports that the run **balances**.
    #[tokio::test]
    async fn a_local_pull_that_stores_nothing_still_accounts_for_every_row() {
        let dir = agreeing("localdropped");
        let site = site("localdropped", &dir);
        // February rows against a January window: both are declined, neither
        // is a failure, and no bar file is ever opened.
        let folder = vendor_folder(
            "localdropped",
            &format!(
                "{GDFL_MEMBER_HEAD}\
                 NIFTY,08/02/2022,10:00:00,100.00,0,0,0,0,0,0\n\
                 NIFTY,08/02/2022,10:00:01,100.50,0,0,0,0,0,0\n"
            ),
        );

        let (code, html) = spot_answer(
            &spot_form(&folder, "2022-01-08", "2022-01-08"),
            day(2026, 8, 7),
            moment(),
            &site,
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK, "{html}");
        assert!(html.contains("Members read"), "{html}");
        assert!(
            html.contains("<th>Rows read</th><td>2</td>"),
            "both rows were read: {html}"
        );
        assert!(
            html.contains("<th>Bars stored</th><td>0</td>"),
            "and neither was stored: {html}"
        );
        assert!(
            html.contains("<th>Rows dropped</th><td>2</td>"),
            "each one counted by the reason it was declined for: {html}"
        );
        assert!(
            html.contains("<th>Members failed</th><td>0</td>"),
            "a declined row is not a failure: {html}"
        );
        // RENAMED HONESTLY, and the arithmetic is now spelled out rather than
        // described. The old sentence said "rows in equals bars out plus drops"
        // and omitted the folded rows entirely, which is the term that made a
        // real 354,675-row run read as not balancing.
        assert!(
            html.contains("<th>Balances</th><td>yes — 2 read = 0 stored + 0 folded + 2 dropped"),
            "the run balances and the receipt shows every term: {html}"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A member that fails is named on the receipt, and the run is reported as
    /// **not** balancing rather than as a success with a smaller number.
    #[tokio::test]
    async fn a_local_pull_with_a_failed_member_says_it_does_not_balance_and_names_it() {
        let dir = agreeing("localfailed");
        let site = site("localfailed", &dir);
        // File order is the only order there is, and this file's order
        // descends — the fold refuses it rather than sorting it, and the
        // member fails before any bar file is opened.
        let folder = vendor_folder(
            "localfailed",
            &format!(
                "{GDFL_MEMBER_HEAD}\
                 NIFTY,08/01/2022,10:00:00,100.00,0,0,0,0,0,0\n\
                 NIFTY,08/01/2022,09:30:00,100.50,0,0,0,0,0,0\n"
            ),
        );

        let (code, html) = spot_answer(
            &spot_form(&folder, "2022-01-08", "2022-01-08"),
            day(2026, 8, 7),
            moment(),
            &site,
        )
        .await;
        assert_eq!(
            code,
            axum::http::StatusCode::OK,
            "the RUN completed; one member did not — those are different \
             answers and the page gives both: {html}"
        );
        assert!(html.contains("<th>Members failed</th><td>1</td>"), "{html}");
        assert!(
            html.contains("2 rows read, 0 stored"),
            "the receipt spells out WHY it does not balance rather than \
             printing a smaller number as if it were the answer: {html}"
        );
        assert!(
            html.contains("NIFTY"),
            "and names the member that failed: {html}"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// Without a folder the HTTP path is what is being asked for, and it does
    /// not exist — so the pull is refused loudly rather than silently doing
    /// nothing.
    #[tokio::test]
    async fn a_spot_pull_with_no_folder_is_still_the_loud_unavailable() {
        let dir = agreeing("localnofolder");
        let site = site("localnofolder", &dir);
        let (code, html) = spot_answer(
            "target=swept&from=2022-01-08&to=2022-01-08",
            day(2026, 8, 7),
            moment(),
            &site,
        )
        .await;
        assert_eq!(
            code,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "an absent folder means the HTTP path, which does not exist: {html}"
        );
        assert!(
            !html.contains("Bars stored"),
            "and nothing was counted, because nothing ran: {html}"
        );
    }

    /// Bars land in the store root the PAGE reports, and nowhere else.
    ///
    /// **This test replaced `the_default_store_directory_is_named_under_the_
    /// home_directory`, and the function it exercised is gone.** That function
    /// read `HOME` directly and joined `.brutex/store`, while every page reads
    /// [`store_dir`], which honours `BRUTEX_STORE`. With that variable set the
    /// two disagreed: bars went to one tree and `/store` truthfully described
    /// another, which is the "plausible wrong answer" shape this repository
    /// hunts. The old test asserted the old function's shape faithfully — it
    /// was the *design* that was wrong, so the assertion is now about the
    /// property that matters instead: one root, taken from the site.
    #[tokio::test]
    async fn a_run_writes_under_the_same_store_root_the_page_reports() {
        let dir = agreeing("localroot");
        let site = site("localroot", &dir);
        let folder = vendor_folder(
            "localroot",
            &format!(
                "{GDFL_MEMBER_HEAD}\
                 NIFTY,08/01/2022,10:00:00,100.00,0,0,0,0,0,7\n\
                 NIFTY,08/01/2022,10:00:30,100.50,0,0,0,0,0,7\n"
            ),
        );
        let (code, html) = spot_answer(
            &spot_form(&folder, "2022-01-08", "2022-01-08"),
            day(2026, 8, 7),
            moment(),
            &site,
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK, "{html}");
        let root = site.store_root.display().to_string();
        assert!(
            html.contains(&format!("<th>Store root</th><td>{root}</td>")),
            "the receipt names the root it wrote under: {html}"
        );
        assert!(
            site.store_root.join("bars").exists(),
            "and the bars are THERE, under the site's root and not under one \
             read from the environment a second time"
        );
        // The journal is under the same root, for the same reason.
        assert!(site.journal().path.starts_with(&site.store_root));
        assert_eq!(site.journal().look().records(), 1);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// One local run, on the record, and the pull page reads it back.
    ///
    /// The whole loop in one test: a run happens, a record lands on disk, and
    /// the page renders that record's counters rather than an em dash.
    #[tokio::test]
    async fn a_run_is_recorded_and_the_pull_page_reads_the_record_back() {
        let dir = agreeing("localaudit");
        let site = site("localaudit", &dir);
        // Before anything: no journal, dashes, and a sentence naming the empty
        // file rather than an absent module.
        let before = pull_html(&site, day(2026, 8, 7));
        assert!(before.contains("No file yet"), "{before}");
        assert_eq!(before.matches("class=\"meter none\"").count(), 6);
        assert!(
            before.contains("No pull has been recorded against this store root yet"),
            "{before}"
        );

        let folder = vendor_folder(
            "localaudit",
            &format!(
                "{GDFL_MEMBER_HEAD}\
                 NIFTY,08/01/2022,10:00:00,100.00,0,0,0,0,5,7\n\
                 NIFTY,08/01/2022,10:00:30,100.50,0,0,0,0,3,7\n\
                 NIFTY,08/01/2022,10:01:00,101.00,0,0,0,0,2,7\n"
            ),
        );
        let (code, receipt) = spot_answer(
            &spot_form(&folder, "2022-01-08", "2022-01-08"),
            day(2026, 8, 7),
            moment(),
            &site,
        )
        .await;
        assert_eq!(code, axum::http::StatusCode::OK, "{receipt}");
        assert!(
            receipt.contains("<th>Recorded</th><td>yes — appended to"),
            "the receipt says whether the record landed: {receipt}"
        );
        // A RUN THAT STORED BARS IS NOT "NOT STARTED". That verdict on this
        // page was the third stale claim on this route.
        assert!(!receipt.contains("NOT STARTED"), "{receipt}");
        assert!(receipt.contains("STORED"), "{receipt}");
        assert!(
            receipt.contains("<th>Rows folded into an open bar</th>"),
            "{receipt}"
        );
        assert!(
            receipt.contains("<th>Slices the census counted</th>"),
            "{receipt}"
        );

        // 3 rows in, 2 bars out (10:00 and 10:01), 1 folded, 0 dropped.
        assert!(html_fact(&receipt, "Rows read", "3"), "{receipt}");
        assert!(html_fact(&receipt, "Bars stored", "2"), "{receipt}");
        assert!(
            html_fact(&receipt, "Rows folded into an open bar", "1"),
            "{receipt}"
        );

        // And the pull page now reads that record back off disk.
        let after = pull_html(&site, day(2026, 8, 7));
        assert_eq!(
            after.matches("class=\"meter none\"").count(),
            0,
            "every counter is measured now: {after}"
        );
        assert!(after.contains("STORED"), "{after}");
        assert!(after.contains("2022-01-08..=2022-01-08"), "{after}");
        assert!(after.contains("1 record(s)"), "{after}");
        assert!(
            after.contains("It is a file on disk, not memory"),
            "and it says which: {after}"
        );

        // As does /audit, with the same numbers and no JavaScript.
        let page = audit_html(&site, day(2026, 8, 7), 0);
        assert!(page.contains("2026-08-07 10:00:00 IST"), "{page}");
        assert!(page.contains("STORED"), "{page}");
        assert!(
            page.contains(">3</td>") && page.contains(">2</td>"),
            "{page}"
        );
        for forbidden in ["<script", "javascript:", "onclick", "onload", "onerror"] {
            assert!(!page.contains(forbidden), "{forbidden} must never appear");
        }
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// Whether a `<th>label</th><td>value</td>` pair is on a receipt.
    fn html_fact(html: &str, label: &str, value: &str) -> bool {
        html.contains(&format!("<th>{label}</th><td>{value}</td>"))
    }

    /// The banner names what exists and what does not, and both halves are
    /// checkable against the tree.
    ///
    /// **This test exists because the sentence it guards was false for
    /// months.** The old constant said `pull::fetch` and `pull::rate` did not
    /// exist while both files were on disk and the local-archive path was
    /// writing bars through them, and nothing in CI could see it: gate 12 only
    /// reads *cost* claims, and "there is no module X" is not one. A claim
    /// about a file is checkable by looking at the file, so this looks.
    #[test]
    fn the_ingest_page_names_what_exists_and_what_does_not() {
        // THE CLAIM, IN ITS CURRENT FORM: fetch.rs, rate.rs and http.rs are all
        // present; HttpSource answers through `window_async`; and the route
        // does not call it. Every one of those is a fact about a tracked file,
        // so it is read rather than asserted from memory.
        //
        // THIS TEST DID ITS JOB. The banner used to say "crates/pull declares
        // no HTTP client", and the assertion below used to be
        // `!manifest.contains("reqwest")`. Adding the dependency turned the
        // sentence false and turned this test red in the same commit — which is
        // the entire reason it exists. It was updated to the new truth, not
        // deleted, and the new truth is narrower and therefore easier to break.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates/")
            .join("pull");
        for file in ["src/fetch.rs", "src/rate.rs", "src/http.rs"] {
            assert!(
                root.join(file).is_file(),
                "the banner claims crates/pull/{file} is present — it must be"
            );
        }
        let fetch = std::fs::read_to_string(root.join("src/fetch.rs")).expect("fetch.rs");
        assert!(
            fetch.contains("pub trait BarSource"),
            "the seam the banner names must be there"
        );
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml");
        assert!(
            manifest.contains("reqwest"),
            "the banner says the HTTP client is BUILT; the dependency must be \
             declared for that to be true"
        );
        let http = std::fs::read_to_string(root.join("src/http.rs")).expect("http.rs");
        assert!(
            http.contains("pub async fn window_async"),
            "the banner names window_async as the method that works"
        );

        // THE OTHER HALF, AND THE ONE THAT MATTERS MOST: this route must not
        // call the vendor. A test that only checked the client exists would
        // pass just as happily once the wiring landed, and the banner would
        // then be telling an operator nothing is contacted while it was.
        // HOW THIS READ USED TO DEFEAT ITSELF, kept as the reason it is written
        // the way it is now.
        //
        // It was:
        //     let me = read_to_string(Path::new(file!()))
        //         .or_else(|_| read_to_string("crates/api/src/server.rs"))
        //         .unwrap_or_default();
        //     assert!(!me.contains("HttpSource::new"), ...);
        //
        // `file!()` is workspace-relative and the test process runs with its CWD
        // at `crates/api`, so BOTH reads fail. `unwrap_or_default()` then hands
        // back an empty string, and `!"".contains(..)` is trivially true — so
        // the assertion passed no matter what the file said. It was passing at
        // the moment `broker_window` began calling `HttpSource::new` forty lines
        // above, which is the exact event it existed to catch.
        //
        // Two `CLAUDE.md` §4 bans in four lines: a test that asserts nothing,
        // and a fallback that hides a failure. The read is now a hard failure
        // and the path is built from CARGO_MANIFEST_DIR, which always resolves.
        let me =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server.rs"))
                .expect(
                    "this module's own source must be readable, or the check below proves nothing",
                );
        assert!(
            me.contains("fn broker_window"),
            "sanity: the source really was read"
        );

        // AND THE CORRESPONDENCE, which is the thing worth asserting.
        //
        // Whether this process reaches a vendor is decided by `Site::broker`,
        // not by a sentence. So: if the source constructs an `HttpSource` at
        // all, a LIVE site must not be told that nothing is contacted. Pinning
        // either string is what let the old sentence rot while green.
        let constructs_a_client = me.contains("HttpSource::new");
        assert!(
            constructs_a_client,
            "broker_window builds the client; if that stops being true the \
             banner texts below need revisiting"
        );
        assert!(
            !halt_for(Broker::Live).contains("no vendor is contacted"),
            "a process that constructs an HttpSource and sets Broker::Live must \
             not tell an operator that no vendor is contacted from it"
        );
        assert!(
            halt_for(Broker::Refused).contains("no vendor is contacted"),
            "and a refused process must still say so"
        );
        assert_ne!(
            halt_for(Broker::Live),
            halt_for(Broker::Refused),
            "the two states must read differently or the choice is decoration"
        );

        // AND THE PAGE SAYS EXACTLY THAT, with none of the old sentence left.
        let dir = agreeing("banner");
        let site = site("banner", &dir);
        let html = pull_html(&site, day(2026, 8, 7));
        assert!(html.contains("crates/pull/src/fetch.rs"), "{html}");
        assert!(html.contains("crates/pull/src/rate.rs"), "{html}");
        assert!(html.contains("crates/pull/src/http.rs"), "{html}");
        // A test site is `Broker::Refused`, so it gets the unavailable text — but
        // assert that through `halt_for` rather than by quoting the sentence, so
        // rewording the copy does not require editing the test.
        // The first sentence, TAKEN FROM the constant rather than quoted here,
        // so rewording the copy does not require editing the test. The whole
        // string cannot be compared: `render::escape` turns the `&` of "F&O"
        // into `&amp;` on the way to the page.
        let head = halt_for(site.broker)
            .split('.')
            .next()
            .expect("the banner has a first sentence");
        assert!(html.contains(head), "expected {head:?} in {html}");
        assert_eq!(
            site.broker,
            Broker::Refused,
            "a test never reaches a vendor"
        );
        for stale in [
            "no vendor fetch and",
            "there is no pull::fetch",
            "no pull::rate",
            "P-01 through P-04 still stand",
            "CAPTURE UNAVAILABLE",
        ] {
            assert!(
                !html.contains(stale),
                "the old, false sentence must not survive anywhere: {stale}"
            );
        }
        // The four invariant rows the old text claimed still read "—" do not.
        let inv = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("repo root")
            .join("docs/04-invariants.md");
        let text = std::fs::read_to_string(&inv).expect("the invariants document");
        for row in ["| P-01 |", "| P-02 |", "| P-03 |", "| P-04 |"] {
            let line = text
                .lines()
                .find(|l| l.starts_with(row))
                .unwrap_or_else(|| panic!("{row} must exist"));
            assert!(
                !line.trim_end().ends_with("— |"),
                "{row} no longer stands at an em dash, which is exactly what the \
                 old banner claimed it did"
            );
        }
    }

    /// A journal that cannot be read at all is loud on every page that reads
    /// it, and a run whose record could not be written says so on its receipt.
    ///
    /// The fixture is a FILE where the `audit/` directory has to be: the
    /// metadata call on the journal path then fails with `NotADirectory`, which
    /// is neither "absent" nor "held" and is exactly the third state the reader
    /// keeps separate.
    #[tokio::test]
    async fn a_journal_that_cannot_be_read_is_loud_and_a_lost_record_is_named() {
        let dir = agreeing("journalbroken");
        let site = site("journalbroken", &dir);
        std::fs::write(site.store_root.join("audit"), b"not a directory").expect("writes");

        assert!(
            matches!(site.journal().look(), audit::Log::Unreadable { .. }),
            "a file where the directory has to be is unreadable, not absent"
        );

        // /pull says so, /audit says so, /store says so. Three pages, one fact.
        let pull = pull_html(&site, day(2026, 8, 7));
        assert!(pull.contains("UNREADABLE —"), "{pull}");
        assert!(pull.contains("No run can be recorded"), "{pull}");
        let page = audit_html(&site, day(2026, 8, 7), 0);
        assert!(page.contains("UNREADABLE —"), "{page}");
        assert!(page.contains("ATTENTION"), "the badge is loud: {page}");
        let store = store_ok(&site, day(2026, 8, 7), 0, "show=gaps");
        assert!(store.contains("UNAVAILABLE — audit journal"), "{store}");

        // AND THE RECEIPT NAMES THE LOST RECORD. A run whose 256 bytes could
        // not be written is a run nobody can find afterwards, so the page that
        // answers it is where that has to be said.
        let folder = vendor_folder(
            "journalbroken",
            &format!(
                "{GDFL_MEMBER_HEAD}\
                 NIFTY,08/01/2022,10:00:00,100.00,0,0,0,0,4,7\n"
            ),
        );
        let (code, html) = spot_answer(
            &spot_form(&folder, "2022-01-08", "2022-01-08"),
            day(2026, 8, 7),
            moment(),
            &site,
        )
        .await;
        assert_eq!(
            code,
            axum::http::StatusCode::OK,
            "the BARS still landed: {html}"
        );
        assert!(
            html.contains("<th>Recorded</th><td>NO — this run is NOT in the journal."),
            "the lost record is named, not swallowed: {html}"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// A torn journal is named on the pages that read it, and the whole records
    /// before the tear still render.
    #[test]
    fn a_torn_journal_is_named_and_the_records_before_it_still_render() {
        let dir = agreeing("journaltorn");
        let site = site("journaltorn", &dir);
        let journal = site.journal();
        journal
            .append(&audit::Record::refused(
                audit::Scope::Spot,
                audit::Outcome::Stored,
                moment(),
                "swept",
                "whole",
            ))
            .expect("appends");
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&journal.path)
                .expect("opens");
            f.write_all(&[0u8; 100]).expect("writes");
        }
        let page = audit_html(&site, day(2026, 8, 7), 0);
        assert!(page.contains("TORN WRITE — 100 byte(s)"), "{page}");
        assert!(page.contains("nothing here repairs the tail"), "{page}");
        assert!(
            page.contains("swept"),
            "the whole record still renders: {page}"
        );
        let pull = pull_html(&site, day(2026, 8, 7));
        assert!(pull.contains("TORN WRITE"), "{pull}");
    }

    /// The three renderings that only a value out of range can reach.
    #[test]
    fn a_clock_or_a_window_outside_the_calendar_is_said_rather_than_guessed() {
        // A timestamp no IST calendar can name. `ist_stamp` refuses it by name
        // instead of blanking the row: a record whose stamp is unusable is
        // still a record of a run that happened.
        let said = ist_stamp(i64::MAX);
        assert!(
            said.starts_with("epoch second 9223372036854775807 — "),
            "{said}"
        );
        let epoch = ist_stamp(0);
        assert!(epoch.contains("1970-01-01 05:30:00 IST"), "{epoch}");

        // A window whose day counts are past 9999-12-31.
        assert_eq!(window_text(0, 0), "—", "no window is a dash, not 1970");
        assert!(window_text(u32::MAX, u32::MAX).contains("outside the calendar"));
        assert_eq!(window_text(0, 1), "1970-01-01..=1970-01-02");

        // And a duration crossing the second boundary in both directions.
        assert_eq!(render_elapsed(0), "0.000 ms");
        assert_eq!(render_elapsed(999_999), "999.999 ms");
        assert_eq!(render_elapsed(4_512_903), "4.512 s");
    }

    /// A record whose window and note are both past what a record can hold
    /// renders as a cut value that says it was cut.
    #[test]
    fn a_record_with_an_impossible_window_and_a_cut_note_still_renders() {
        let dir = agreeing("auditcut");
        let site = site("auditcut", &dir);
        let mut record = audit::Record::refused(
            audit::Scope::Spot,
            audit::Outcome::Refused,
            moment(),
            "swept",
            "a reason far longer than the sixty-eight bytes one record keeps for it, \
             so the page has to say that what it shows is a prefix",
        );
        record.from_days = u32::MAX;
        record.to_days = u32::MAX;
        site.journal().append(&record).expect("appends");

        let page = audit_html(&site, day(2026, 8, 7), 0);
        assert!(page.contains("(cut from 125 bytes)"), "{page}");
        assert!(page.contains("a reason far longer"), "{page}");
        // The window is unrenderable and the row still renders.
        assert!(page.contains("REFUSED"), "{page}");
        // And /pull shows dashes rather than a fabricated window, with a
        // sentence naming why.
        let pull = pull_html(&site, day(2026, 8, 7));
        assert!(
            pull.contains("The last record carries no usable window"),
            "{pull}"
        );
        assert_eq!(pull.matches("class=\"meter none\"").count(), 6, "{pull}");
    }

    /// A source longer than the record's field is cut on the page and says so.
    #[test]
    fn a_source_longer_than_the_field_is_shown_as_cut_on_the_audit_page() {
        let dir = agreeing("auditcutsrc");
        let site = site("auditcutsrc", &dir);
        let long = format!("/very/long/vendor/folder/path/{}", "segment/".repeat(12));
        assert!(long.len() > 64);
        site.journal()
            .append(&audit::Record::refused(
                audit::Scope::Spot,
                audit::Outcome::Failed,
                moment(),
                &long,
                "short",
            ))
            .expect("appends");
        let page = audit_html(&site, day(2026, 8, 7), 0);
        assert!(
            page.contains(&format!("(cut from {} bytes)", long.len())),
            "{page}"
        );
        assert!(page.contains("FAILED"), "{page}");
    }

    /// A journal that will not read empties the table and says why, rather
    /// than reporting no runs.
    ///
    /// The disagreement staged here is the real one: the `metadata` call said
    /// there were four records and the read found nothing there — a journal
    /// deleted between the two. "Nothing has been recorded" and "I could not
    /// read what was recorded" are different facts and this is where they stay
    /// different.
    #[test]
    fn a_journal_that_will_not_read_empties_the_table_and_names_the_refusal() {
        let root = store_root("auditunread");
        let journal = audit::Journal::at(&root);
        let mut notes = Vec::new();
        let rows = audit_rows(&journal, 4, 0, 200, &mut notes);
        assert!(
            rows.is_empty(),
            "no row is invented over an unreadable file"
        );
        assert_eq!(notes.len(), 1, "and exactly one reason is given: {notes:?}");
        // AND THE SAME DISAGREEMENT SHOWS NO "LAST RUN". A counter that says
        // four and a file that holds none must not produce a capture panel
        // over figures nobody read.
        assert_eq!(
            newest_record(
                &journal,
                &audit::Log::Held {
                    records: 4,
                    bytes: 1024,
                    torn: None
                }
            ),
            None,
            "a record that cannot be read is not a record"
        );
        let why = notes.first().map(String::as_str).unwrap_or_default();
        assert!(why.starts_with("UNREADABLE — "), "{why}");
        assert!(why.contains("pull.journal"), "and names the file: {why}");

        // The other half: a file that IS there yields rows and no note.
        journal
            .append(&audit::Record::refused(
                audit::Scope::Spot,
                audit::Outcome::Stored,
                moment(),
                "swept",
                "",
            ))
            .expect("appends");
        let mut clean = Vec::new();
        let rows = audit_rows(&journal, 1, 0, 200, &mut clean);
        assert_eq!(rows.len(), 1);
        assert!(clean.is_empty(), "a clean read adds no note: {clean:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The audit page with nothing in it says so, and never 500s.
    #[test]
    fn an_empty_audit_page_says_nothing_was_recorded_rather_than_showing_zeroes() {
        let dir = agreeing("auditempty");
        let site = site("auditempty", &dir);
        let page = audit_html(&site, day(2026, 8, 7), 0);
        assert!(page.starts_with("<!doctype html>"));
        assert!(page.contains("nothing recorded yet"), "{page}");
        assert!(page.contains("No file yet"), "{page}");
        assert!(!page.contains("<tbody>"), "no table at all: {page}");
        // A page past the end clamps rather than erroring.
        let far = audit_html(&site, day(2026, 8, 7), 9_999);
        assert_eq!(far, page, "?page=9999 clamps to the only page there is");
    }

    /// A damaged record is a refused ROW, never a blank page.
    #[test]
    fn a_damaged_record_is_shown_as_refused_and_the_rest_of_the_page_renders() {
        let dir = agreeing("auditdamaged");
        let site = site("auditdamaged", &dir);
        let journal = site.journal();
        for i in 0..3u32 {
            journal
                .append(&audit::Record::refused(
                    audit::Scope::Spot,
                    audit::Outcome::Stored,
                    moment(),
                    &format!("run-{i}"),
                    "nothing to report",
                ))
                .expect("appends");
        }
        let mut bytes = std::fs::read(&journal.path).expect("reads");
        bytes[300] ^= 0xff;
        std::fs::write(&journal.path, &bytes).expect("writes");

        let page = audit_html(&site, day(2026, 8, 7), 0);
        assert!(page.contains("RECORD REFUSED"), "{page}");
        assert!(page.contains("record checksum"), "and both numbers: {page}");
        assert!(page.contains("run-2"), "the newest still renders: {page}");
        assert!(page.contains("run-0"), "and so does the oldest: {page}");
        assert!(page.contains("ATTENTION"), "the badge is loud: {page}");
    }

    /// A held month renders as a filled swatch on the real page, driven by a
    /// real manifest file rather than by a hand-built row.
    ///
    /// The render-level proof is `api::render::a_swatch_is_a_shape_before_it_is
    /// _a_shade`; this is the plumbing behind it — manifest bytes on disk, read
    /// by `census::read_vendor`, probed by `census::coverage_page`, drawn by
    /// `render::store_page`. A visualisation proved only against a fixture is a
    /// visualisation nobody has seen over data.
    #[test]
    fn a_held_month_is_a_filled_swatch_on_the_page_over_a_real_manifest() {
        use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
        let dir = agreeing("swatchreal");
        let root = store_root("swatchreal");

        let mut manifest = Manifest::open(Vendor::Dhan, &[], &[]).expect("a genesis manifest");
        let symbol = brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol");
        let month = store::path::YearMonth::new(2026, 8).expect("a month");
        manifest
            .record(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: brutex_core::instrument::Exchange::Nse,
                    segment: brutex_core::instrument::Segment::Index,
                    symbol,
                    timeframe: store::path::Timeframe::MINUTE_1,
                    month,
                },
                rows: 8_250,
                first_ts_micros: 1_751_350_800_000_000,
                last_ts_micros: 1_751_363_940_000_000,
            })
            .expect("records");
        let path = manifest_path(&root, Vendor::Dhan);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(&path, manifest.image()).expect("writes");

        let site = Site::load(&dir, &root);
        let html = store_ok(&site, day(2026, 8, 7), 0, "show=gaps");

        // The counter card reads the file, not a guess.
        assert!(
            html.contains("<div class=\"cv\">1</div>"),
            "one month: {html}"
        );
        assert!(
            html.contains("8250 row(s) across 1 committed entr(ies)"),
            "{html}"
        );

        // AND THE CELL IS A FILLED SQUARE. Exactly one, because exactly one
        // instrument-month on this page is held — the top shade, since it is
        // also the fullest month shown.
        assert_eq!(
            html.matches("class=\"sw q4\"").count(),
            1,
            "the held cell is solid and at the top shade: {html}"
        );
        assert!(html.contains("held, 8250 row(s)"), "{html}");
        assert!(
            html.matches("class=\"sw void\"").count() > 1,
            "and every month that is not held is hollow and crossed: {html}"
        );
        assert_eq!(
            html.matches("<tr class=\"swept\">").count(),
            1,
            "one row is tinted held: {html}"
        );
        // One index instrument in the fixture universe × 36 months back.
        assert!(
            html.contains("72 instrument-month(s) in the grid"),
            "{html}"
        );
        assert!(
            html.contains("<b>1 of 72</b> shown row(s) are held"),
            "{html}"
        );
        assert!(html.contains("quartiles of 8250"), "{html}");
        assert!(html.contains("class=\"strip\""), "{html}");
        assert!(
            html.contains("<rect class=\"on\""),
            "and the glance strip has a tick for it: {html}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The store page says out loud that its counters predate a pull this
    /// process has since performed.
    #[test]
    fn the_store_page_says_when_its_counters_are_older_than_the_last_pull() {
        let dir = agreeing("storestale");
        let site = site("storestale", &dir);
        let fresh = store_ok(&site, day(2026, 8, 7), 0, "show=gaps");
        assert!(
            !fresh.contains("these manifests were read at"),
            "nothing has happened yet: {fresh}"
        );

        // A record stamped after the site loaded is exactly the case the note
        // exists for: the bars and the manifest moved, and these cards did not.
        let later = std::time::SystemTime::now() + std::time::Duration::from_mins(1);
        site.journal()
            .append(&audit::Record::refused(
                audit::Scope::Spot,
                audit::Outcome::Stored,
                later,
                "swept",
                "",
            ))
            .expect("appends");
        let stale = store_ok(&site, day(2026, 8, 7), 0, "show=gaps");
        assert!(
            stale.contains("UNCHECKED — a pull ran at"),
            "the staleness is named, not left to be discovered: {stale}"
        );
        // IT SURVIVES THE RENDERER'S CLAMP WHOLE. `render::clamp` cuts a note
        // past 160 bytes at its last comma and appends "… and N more", so a
        // warning that is too long is a warning whose second half nobody reads.
        // Asserting the closing tag right after the last word is what proves
        // this one was not cut — a `contains("Restart")` would pass on a
        // truncated note too.
        assert!(
            stale.contains("Restart to refresh, or see /audit.</li>"),
            "the whole note reaches the page, uncut: {stale}"
        );
        assert!(
            stale.contains("class=\"loud\""),
            "UNCHECKED is one of the words that makes a note loud: {stale}"
        );
    }

    /// A vendor on loopback that answers once, and reports it was reached.
    ///
    /// Raw sockets and hand-written HTTP, for the reason `crates/pull`'s own
    /// socket tests give: this workspace does not widen a dependency for a test.
    ///
    /// **THE DOC COMMENT THAT USED TO SIT HERE BELONGED TO ANOTHER TEST.** The
    /// prose about four emit sites and a `Trace` floor was written for
    /// [`the_run_events_and_the_request_event_reach_the_installed_sink`], and
    /// this function was later inserted BETWEEN that comment and the item it
    /// documented — so the test lost its explanation and this helper acquired
    /// one about events it does not emit. Rust attaches a doc block to whatever
    /// item follows it, silently. The block is back on its own test below; this
    /// note is here so the same insertion is not made again.
    fn loopback_vendor(answer: &'static str) -> (String, std::sync::mpsc::Receiver<()>) {
        use std::io::{Read as _, Write as _};
        let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let addr = socket.local_addr().expect("an address");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok((mut stream, _)) = socket.accept() {
                let mut buf = [0u8; 4096];
                drop(stream.read(&mut buf));
                let _reached = tx.send(());
                drop(stream.write_all(answer.as_bytes()));
                drop(stream.flush());
            }
        });
        (format!("http://{addr}"), rx)
    }

    /// **THE TWO SITES PAST THE SOCKET, DRIVEN OVER A REAL ONE.**
    ///
    /// `crates/api/src/emitted.rs` names three emit sites it cannot reach and
    /// says they "become reachable the day a recorded-transport fake exists for
    /// `pull::fetch`". Two of them do not need one. They need a SOCKET, and
    /// `crates/pull` already established that a loopback listener is how this
    /// workspace supplies one — `pull::emit_sites::drive_http` does exactly
    /// this against a 503.
    ///
    /// So `pull.http vendor refused a window` and `pull.chunk answered` are
    /// proven here, through the shipped `fetch_chunks`, against a vendor this
    /// test opened. Nothing is faked about the wire: the bytes on it are the
    /// bytes a vendor would send, and the code under test is the code that runs
    /// in production.
    ///
    /// # The success path needs a body the SHIPPED decoder accepts
    ///
    /// The 503 half only had to be refused. The 200 half has to be *decoded*,
    /// by `pull::http::decode_body` reading the real Dhan descriptor — so the
    /// bytes below are that descriptor's own shape and not a shape invented
    /// here: `ResponseShape::ParallelArrays { envelope: None }`, six arrays
    /// named by its `FieldNames`, prices in rupees and no `open_interest` array
    /// at all, because that row deliberately leaves the name `None` (a spot
    /// index has none, and an absent array is not a zero — `CLAUDE.md` §7).
    /// Getting any of that wrong fails the decode, and a decode that fails
    /// never reaches the emit — which is what makes this a test of the site
    /// rather than of the fixture.
    ///
    /// The third — `pull.spot instrument refused` — is driven by
    /// [`the_refused_instrument_site_is_driven_over_a_real_universe`], which
    /// needs a non-empty universe and no socket at all.
    #[expect(
        clippy::too_many_lines,
        reason = "two emit sites, two loopback vendors and ONE installed sink. \
                  Split in two, both halves would still have to agree about a \
                  sink that is a process singleton, and the second would repeat \
                  the descriptor surgery the first already performs -- which is \
                  the coupling this test exists inside rather than around."
    )]
    #[tokio::test]
    async fn the_two_sites_past_the_socket_are_driven_over_a_real_one() {
        /// One bar, in the shape the SHIPPED Dhan descriptor declares: parallel
        /// arrays with no envelope, prices in rupees, and no `open_interest`
        /// array because that row leaves the name `None`. See the second half
        /// of this test for why every one of those is load-bearing.
        ///
        /// At the top of the function rather than beside its use, because
        /// clippy denies an item after a statement — an item is in scope from
        /// the start of the block whatever line it is written on, and a reader
        /// who believes otherwise is the defect that lint exists for.
        const BODY: &str = concat!(
            r#"{"open":[24500.75],"high":[24501.50],"low":[24499.25],"#,
            r#""close":[24500.50],"volume":[250],"timestamp":[1751337900]}"#
        );

        // Installs (or adopts) the process sink, so `landed` below reads the
        // one `telemetry::emit` actually reaches.
        let _sink = crate::emitted::sink();
        let dir = masters("chunks", None, None);
        let site = Site::serving(&dir, &store_root("chunks"));
        let asked = ingest::parse_spot(
            "target=swept&from=2026-08-03&to=2026-08-05",
            day(2026, 8, 10),
        )
        .expect("a real target and a window in the past");

        // A REFUSAL: the vendor answers 503, so `with_retry` gives up and the
        // error map writes the line that carries the vendor's own words.
        let (url, reached) = loopback_vendor(
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 9\r\n\r\ntoo busy\n",
        );
        let pull::vendor::Transport::Http(shipped) =
            pull::vendor::Feed::Dhan.descriptor().transport
        else {
            panic!("this feed authenticates, so its transport is HTTP")
        };
        let spec = pull::vendor::HttpSpec {
            // Leaked for the reason `crates/pull`'s socket tests leak theirs: a
            // descriptor's base URL is `&'static str` and a port known only at
            // run time has to outlive the borrow.
            base_url: Box::leak(url.into_boxed_str()),
            ..shipped
        };
        let source =
            pull::http::HttpSource::new(spec, pull::http::Credential::token("shhh".to_owned()))
                .expect("a client");

        let from = crate::emitted::mark();
        let answered = fetch_chunks(
            &asked,
            &site,
            &source,
            "13",
            pull::vendor::Listing::Index,
            &spec,
        )
        .await;
        assert!(answered.is_err(), "a 503 is a refusal");
        assert!(
            reached
                .recv_timeout(std::time::Duration::from_secs(5))
                .is_ok(),
            "the loopback vendor was contacted, so there was an answer to write down"
        );
        let refused = crate::emitted::landed(from, "pull.http", "vendor refused a window");
        assert!(
            !refused.is_empty(),
            "the refusal reaches the log, carrying the vendor's own words in \
             their own field — the whole reason that site exists"
        );
        assert!(
            refused
                .iter()
                .any(|r| crate::emitted::says(r, "instrument_id", "13")),
            "and it names the instrument it was asking about: {refused:?}"
        );

        // AN ANSWER: a second vendor, on its own port, returning one bar in the
        // shape THIS descriptor declares. `fetch_chunks` decodes it and writes
        // the `pull.chunk answered` line — the site that tells a run which was
        // merely slow apart from a run that was silently returning nothing, and
        // the row count is the field that does it. `BODY` is at the top of the
        // function; it is this vendor's own response shape.
        //
        // Leaked for the same reason the base URL below is: `loopback_vendor`
        // hands its bytes to a thread that outlives this frame, and the length
        // header has to be computed rather than written, or a body edited by
        // one byte would hang the client instead of failing the assertion.
        let answer: &'static str = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{BODY}",
                BODY.len()
            )
            .into_boxed_str(),
        );
        let (url, served) = loopback_vendor(answer);
        let spec = pull::vendor::HttpSpec {
            base_url: Box::leak(url.into_boxed_str()),
            ..shipped
        };
        let source =
            pull::http::HttpSource::new(spec, pull::http::Credential::token("shhh".to_owned()))
                .expect("a client");

        let from = crate::emitted::mark();
        let window = fetch_chunks(
            &asked,
            &site,
            &source,
            "1333",
            pull::vendor::Listing::Index,
            &spec,
        )
        .await
        .expect("a 200 in this descriptor's own shape decodes to a window");
        assert_eq!(
            window.len(),
            1,
            "three days is one chunk at this feed's 90-day cap, so one request \
             went out and one answer came back"
        );
        assert!(
            served
                .recv_timeout(std::time::Duration::from_secs(5))
                .is_ok(),
            "the loopback vendor was contacted, so the row count below is one \
             it actually sent"
        );
        assert_eq!(
            window.first().map(|(_, body)| body.rows.len()),
            Some(1),
            "one bar on the wire is one row after the shipped decode: {window:?}"
        );

        let chunks = crate::emitted::landed(from, "pull.chunk", "answered");
        let mine: Vec<&telemetry::Record> = chunks
            .iter()
            .filter(|r| crate::emitted::says(r, "instrument_id", "1333"))
            .collect();
        assert!(
            !mine.is_empty(),
            "the success path writes its own line; before this it wrote nothing \
             and a run returning empty windows was indistinguishable from a slow \
             one. {} record(s) landed on that target in the window.",
            chunks.len()
        );
        for record in mine {
            assert_eq!(
                record.level,
                telemetry::Level::Trace,
                "`Trace`, deliberately: ~62,600 chunks on a one-minute backfill, \
                 so an operator opts in to the wire rather than drowning in it"
            );
            assert!(
                crate::emitted::counts(record, "rows", 1),
                "the ROW COUNT is the field that tells a slow run from an empty \
                 one, and it is the vendor's own: {record:?}"
            );
            assert!(
                crate::emitted::counts(record, "chunk", 1)
                    && crate::emitted::counts(record, "of", 1),
                "and it says which request of how many this was: {record:?}"
            );
        }
    }

    /// **THE SITE INSIDE `broker_run`'S LOOP, DRIVEN OVER A REAL UNIVERSE.**
    ///
    /// `pull.spot instrument refused` is the last of the three
    /// `crates/api/src/emitted.rs` listed as out of reach. Its entry claimed it
    /// needed "a non-empty universe **and** a socket to a vendor", and the
    /// second half was wrong in a way worth writing down: the loop emits for
    /// **whatever** [`broker_window`] refuses, and that function refuses a rung
    /// the feed does not declare on its FOURTH statement — above the credential
    /// file, above `AwsIdentity::discover`, above every byte of network.
    ///
    /// So this drives it with no socket and no credential at all. Dhan's
    /// descriptor declares `Day1` and nothing else — its `bars_path` is pinned
    /// to the daily endpoint and its request carries no `interval` field, so
    /// `Minute1` is withdrawn there rather than silently answered with
    /// five-minute candles — and `Minute1` is what a form with no `granularity`
    /// field means. `served` refuses by name, and the refusal is what the site
    /// under test writes down.
    ///
    /// # Why a universe of exactly one
    ///
    /// The loop runs once per instrument the target names, so one row is one
    /// emit and `index`/`of` are `1`/`1` — figures an assertion can pin. The
    /// sibling test drives the same `broker_run` over an EMPTY universe to
    /// prove the run-level events fire either side of a loop that never opens;
    /// between the two, both states of that loop are covered.
    ///
    /// # What this proves that the audit journal cannot
    ///
    /// The journal is a fixed 256-byte stride and cuts a refusal off mid-word,
    /// which is exactly why this event exists. The `why` field carries the
    /// vendor's own sentence, and the assertion below reads it back out of the
    /// file rather than trusting that it was passed in.
    #[tokio::test]
    async fn the_refused_instrument_site_is_driven_over_a_real_universe() {
        let _sink = crate::emitted::sink();
        // ONE SWEEPABLE INDEX, from both masters, so `merge` yields one key and
        // `SpotTarget::Swept` names it. `is_sweepable` demands the INDEX
        // segment and the INDEX kind, which is what these two rows carry.
        let dir = masters(
            "emit-refused",
            Some(&format!(
                "{GROWW_HEAD}NSE,CASH,,NIFTY,IDX,,NIFTY,,,NSE-NIFTY\n"
            )),
            Some(&format!(
                "{DHAN_HEAD}NSE,I,NA,INDEX,NIFTY,NIFTY,INDEX,NA,0001-01-01,,,1333\n"
            )),
        );
        let site = Site::serving(&dir, &store_root("emit-refused"));
        assert_eq!(
            site.read.merged.by_key.len(),
            1,
            "the premise: the loop under test runs exactly once"
        );
        // THE DAY PASS, HELD — the second half of the premise.
        //
        // `crate::ladder` refuses a minute run whose day pass has not landed,
        // and it refuses it BEFORE the loop, which is the whole design. This
        // test is about the per-instrument refusal that happens INSIDE the
        // loop, so the prerequisite is seeded rather than the gate worked
        // around: without this the run is `blocked` and the subject below is
        // never reached. It doubles as the proof that a satisfied gate opens.
        let censuses = vec![day_pass_held(Vendor::Dhan, "NIFTY", month_of(2026, 8))];
        let asked = ingest::parse_spot(
            // A RUNG THIS FEED STILL DOES NOT SERVE, and it must be named now.
            //
            // This was unstated, which defaults to the MINUTE — and the minute
            // rung was the unserved one until Dhan gained its intraday endpoint.
            // It is served now, so the default no longer produces the
            // per-instrument refusal this test is about. `5min` is still
            // undeclared on this feed, pinned by
            // `vendor::a_feed_serves_only_the_rungs_its_row_declares`, so the
            // SUBJECT is unchanged and only the rung exhibiting it moved.
            "target=swept&from=2026-08-03&to=2026-08-05&granularity=5min",
            day(2026, 8, 10),
        )
        .expect("a real target and a window in the past");

        let from = crate::emitted::mark();
        let out = broker_run(&asked, &site, &censuses).await;
        assert!(
            out.blocked.is_none(),
            "a serving site reaches the loop: {out:?}"
        );
        assert_eq!(out.attempted, 1, "one instrument was named: {out:?}");
        assert_eq!(
            out.reached, 0,
            "and the feed does not serve this rung, so none was fetched: {out:?}"
        );
        assert_eq!(
            out.refused.len(),
            1,
            "one refusal, recorded against its own instrument: {out:?}"
        );

        let refused = crate::emitted::landed(from, "pull.spot", "instrument refused");
        // FILTERED ON THE REASON, not on the instrument: `says` is a
        // `contains`, and "NIFTY" is a substring of "BANKNIFTY". The rung
        // refusal is this test's own and no other test in the binary writes it.
        let mine: Vec<&telemetry::Record> = refused
            .iter()
            .filter(|r| crate::emitted::says(r, "why", "does not serve"))
            .collect();
        assert!(
            !mine.is_empty(),
            "the production call wrote no refusal to the file. {} record(s) \
             landed on that target in the window; either the emit is gone or \
             the loop never opened.",
            refused.len()
        );
        for record in mine {
            assert_eq!(
                record.level,
                telemetry::Level::Error,
                "an instrument that will not pull is the line an operator is \
                 woken for: {record:?}"
            );
            assert!(
                crate::emitted::says(record, "instrument", "NIFTY")
                    && crate::emitted::says(record, "feed", "dhan"),
                "it names the instrument and the feed: {record:?}"
            );
            assert!(
                crate::emitted::counts(record, "index", 1)
                    && crate::emitted::counts(record, "of", 1),
                "and where in the run it happened: {record:?}"
            );
            assert!(
                crate::emitted::says(record, "why", "NIFTY: "),
                "the reason begins with the instrument, whole, in the one \
                 surface that has no 256-byte stride to cut it: {record:?}"
            );
        }
    }

    /// **THE RUN'S OWN EVENTS REACH THE FILE, AND SO DOES EVERY REQUEST'S.**
    ///
    /// Four production `telemetry::emit` sites live behind functions this
    /// module keeps private — [`note_run_started`], [`note_run_finished`],
    /// [`note_member_failure`] — or behind a `tower` layer that only a served
    /// request drives: `logs::note_request`. Each was executed by tests and
    /// asserted nothing, because `telemetry::emit` answers `NotInstalled` when
    /// there is no sink and every one of these discards its return. Delete any
    /// of the four calls and, before this test, nothing anywhere failed.
    ///
    /// The sink is [`crate::emitted::sink`] — the single install this binary
    /// owns, at a `Trace` floor. The floor is load-bearing for the last of the
    /// four: `api.request served` is `Debug` for a request that is neither a
    /// 4xx nor a 5xx, so under the default `Info` floor the ordinary path
    /// writes nothing and an assertion about it would be an assertion about the
    /// floor.
    ///
    /// # No socket reaches a vendor
    ///
    /// [`broker_run`] returns before [`note_run_started`] unless
    /// [`Site::broker`] is [`Broker::Live`], which only [`Site::serving`] sets —
    /// so the run is driven through `serving`, over masters that hold **no**
    /// instruments. The target list is then empty, the per-instrument loop does
    /// not execute, and the two run-level events are written either side of a
    /// loop that opens nothing. The site INSIDE that loop is driven by
    /// [`the_refused_instrument_site_is_driven_over_a_real_universe`], which
    /// supplies the one instrument this run deliberately does not have.
    #[expect(
        clippy::too_many_lines,
        reason = "four emit sites against ONE installed sink and ONE bound \
                  listener. Split into four tests it would bind four listeners \
                  and start four servers to assert four lines, and all four \
                  would still have to agree about a sink that is a process \
                  singleton -- which is the coupling this test exists inside \
                  rather than around."
    )]
    #[tokio::test]
    async fn the_run_events_and_the_request_event_reach_the_installed_sink() {
        let sink = crate::emitted::sink();
        let empty = masters("emit-run", None, None);
        let site = Site::serving(&empty, &store_root("emit-run"));
        assert!(
            site.read.merged.by_key.is_empty(),
            "the premise: no instrument, so nothing is asked of any vendor"
        );
        let asked = ingest::parse_spot(
            "target=swept&from=2026-08-03&to=2026-08-05",
            day(2026, 8, 10),
        )
        .expect("a real target and a window in the past");

        let from = crate::emitted::mark();
        // AN EMPTY CENSUS, PASSED EXPLICITLY. The universe is empty here, so
        // the gate has no instrument-month to probe and cannot refuse.
        let out = broker_run(&asked, &site, &[]).await;
        assert_eq!(out.attempted, 0, "an empty universe attempts nothing");
        assert!(
            out.blocked.is_none(),
            "and it was not refused before it began"
        );

        let started = crate::emitted::landed(from, "pull.run", "started");
        let mine = started
            .iter()
            .find(|record| {
                crate::emitted::counts(record, "instruments", 0)
                    && crate::emitted::says(record, "from", "2026-08-03")
            })
            .expect("note_run_started must put the RESOLVED parameters in the file");
        assert_eq!(mine.level, telemetry::Level::Info);
        assert!(
            crate::emitted::says(mine, "rung", "1min")
                && crate::emitted::says(mine, "target", asked.target.label()),
            "the resolved rung and target are the numbers an operator needs: {mine:?}"
        );

        let finished = crate::emitted::landed(from, "pull.run", "finished");
        let mine = finished
            .iter()
            .find(|record| crate::emitted::counts(record, "attempted", 0))
            .expect("note_run_finished must put the run's own verdict in the file");
        assert_eq!(
            mine.level,
            telemetry::Level::Info,
            "a run with no failures and balanced books is Info, not Warn"
        );
        assert!(
            crate::emitted::counts(mine, "failed", 0)
                && crate::emitted::counts(mine, "bars_stored", 0),
            "{mine:?}"
        );

        // A MEMBER THAT REACHED THE VENDOR AND DIED AT THE STORE. Driven
        // directly because the only other way in is through a live socket, and
        // it is the one site in this file whose `debug_assert` already demands
        // `is_written` — which, with no sink installed, was vacuous.
        let from = crate::emitted::mark();
        note_member_failure(
            &pull::ingest::Failure {
                instrument: "EMITMEMBER".to_owned(),
                why: "the store refused the month".to_owned(),
            },
            "2026-08",
            &asked,
            &site,
        );
        let failed = crate::emitted::landed(from, "pull.spot", "member did not land");
        let mine = failed
            .iter()
            .find(|record| crate::emitted::says(record, "instrument", "EMITMEMBER"))
            .expect("a member that did not land is the failure this event exists to name");
        assert_eq!(
            mine.level,
            telemetry::Level::Error,
            "the socket worked and the disk did not, which is an error"
        );
        assert!(
            crate::emitted::says(mine, "month", "2026-08")
                && crate::emitted::says(mine, "why", "the store refused the month"),
            "{mine:?}"
        );

        // EVERY HTTP REQUEST, ONCE, THROUGH THE LAYER RATHER THAN THE HANDLER.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let stop_addr = stopper.local_addr().expect("addr");
        let served = tokio::spawn(serve(
            listener,
            router_serving(
                Loaded::new(Site::load(&empty, &store_root("emit-request"))),
                front("emit-request"),
            ),
            Box::pin(async move { stopper.accept().await.map(|_| ()) }),
        ));

        let from = crate::emitted::mark();
        // A path with no extension is the client-routed shell, so this is a
        // 200 — the arm the DEFAULT floor would have swallowed.
        let shell = get(addr, "/emit-request-probe").await;
        assert!(shell.contains("200 OK"), "{shell}");
        // A path that names a file is a 404, which is the `Warn` arm.
        let missing = get(addr, "/emit-request-probe.js").await;
        assert!(missing.contains("404 Not Found"), "{missing}");

        let served_events = crate::emitted::landed(from, "api.request", "served");
        let ordinary = served_events
            .iter()
            .find(|record| {
                record.field("path").and_then(telemetry::OwnedValue::as_str)
                    == Some("/emit-request-probe")
            })
            .expect("every request writes one line, including the ones that worked");
        assert_eq!(
            ordinary.level,
            telemetry::Level::Debug,
            "a request that worked is detail, and detail is Debug"
        );
        assert!(
            crate::emitted::counts(ordinary, "status", 200)
                && crate::emitted::says(ordinary, "method", "GET")
                && ordinary.field("micros").is_some(),
            "{ordinary:?}"
        );

        let refused = served_events
            .iter()
            .find(|record| crate::emitted::says(record, "path", "/emit-request-probe.js"))
            .expect("and so does the one that answered 404");
        assert_eq!(
            refused.level,
            telemetry::Level::Warn,
            "a 4xx is louder than a request that worked"
        );
        assert!(
            crate::emitted::counts(refused, "status", 404),
            "{refused:?}"
        );

        // THE QUERY STRING IS NEVER ON THE LINE. `note_request` logs the path
        // and deliberately not the query, and this repository is public.
        let from = crate::emitted::mark();
        let _ignored = get(addr, "/emit-request-probe?secret=emit-site-probe").await;
        let with_query = crate::emitted::landed(from, "api.request", "served");
        let carried = with_query
            .iter()
            .find(|record| {
                record.field("path").and_then(telemetry::OwnedValue::as_str)
                    == Some("/emit-request-probe")
            })
            .expect("the request with a query string is still one line");
        assert!(
            carried
                .fields
                .iter()
                .all(|(_, value)| value.as_str() != Some("emit-site-probe")),
            "the PATH is logged and the query is not — this repository is \
             public and a form carries operator input: {carried:?}"
        );

        let _stop = std::net::TcpStream::connect(stop_addr).expect("stop");
        served
            .await
            .expect("the server task must not panic")
            .expect("and must stop cleanly");
        assert!(
            !sink.health().is_loud(),
            "nothing was dropped while proving these sites: {:?}",
            sink.health()
        );
    }

    /// THE HISTORY FLOORS LEAVE THIS PROCESS, PER RUNG, WITH THEIR SOURCES.
    ///
    /// The /ingest page held this table in the browser because there was no
    /// field to read. Every claim below is now emitted, so a stale copy in a
    /// front end is a copy that can be deleted rather than a copy that has to
    /// be kept in step with `crates/pull` by hand.
    #[tokio::test]
    async fn feeds_json_carries_a_floor_per_rung_with_the_source_that_made_it() {
        let dir = masters("feeds-floors", None, None);
        let site = Loaded::new(Site::load(&dir, &store_root("feeds-floors")));
        let (_, body) = feeds_json(axum::extract::State(site)).await;

        // ONE VENDOR, TWO RUNGS, AND THE DISAGREEMENT IS PER RUNG — the whole
        // reason the field is keyed on the rung.
        //
        // What BINDS is the operator's fixed January 2020 at both rungs; he
        // stated it against the vendor and not against a rung, and D-0131
        // records that applying one figure to both is UNVERIFIED rather than
        // split on a guess.
        for rung in ["1min", "1day"] {
            assert!(
                body.contains(&format!(
                    r#"{{"rung":"{rung}","served":true,"kind":"fixed","unit":null,"n":null,"from":"2020-01-01""#
                )),
                "Groww's {rung} rung binds at the operator's fixed 2020: {body}"
            );
        }
        // What the VENDOR claims still differs between the two, and both are
        // carried as `contested` — deleting them would leave a number no
        // reader could argue with.
        assert!(
            body.contains(
                r#""contested":{"kind":"rolling","unit":"m","n":3,"from":null,"oldest":"#
            ),
            "the vendor's published quarter is kept beside the minute rung: {body}"
        );

        // WHO SAID IT, AS A WORD RATHER THAN A PROSE MATCH. The page decides
        // which of two claims binds from this field; before D-0131 it decided
        // it from a second table in the browser.
        assert!(
            body.contains(r#""standing":"operator""#)
                && body.contains(r#""standing":"vendor_doc""#),
            "both standings reach the wire: {body}"
        );
        // A rung nobody has said anything about has no standing to report, and
        // that is the one place the field is null.
        assert!(
            body.contains(r#""kind":"unknown","unit":null,"n":null,"from":null,"oldest":null,"source":null,"standing":null"#),
            "an unrecorded rung reports no standing rather than a default: {body}"
        );

        // A ROLLING FLOOR IS RESOLVED, NOT STORED. The day is computed from
        // today's clock through the same clamp a pull uses, so it is a
        // different string tomorrow — which is why the assertion is on its
        // shape and on the two facts around it rather than on a literal.
        let oldest = body
            .split(r#""kind":"rolling","unit":"y","n":5,"from":null,"oldest":""#)
            .nth(1)
            .expect("the five-year floor resolved to a day")
            .get(..10)
            .expect("a rendered day is ten characters")
            .to_owned();
        assert!(
            oldest.starts_with("20") && oldest.contains('-'),
            "a resolved day, not a stored one: {oldest}"
        );
        assert!(
            !body.contains(r#""oldest":"2021-08-11""#) || oldest != "2021-08-11",
            "nothing here pins a rolling floor to a literal"
        );

        // THE FOUR KINDS ARE FOUR WORDS, and the two that name no day are not
        // the same word. An archive claims NOTHING; a vendor that states it
        // holds everything says so.
        assert!(
            body.contains(r#"{"rung":"1s","served":true,"kind":"unknown","unit":null,"n":null,"from":null,"oldest":null,"source":null"#),
            "an archive's reach is unknown, not unlimited: {body}"
        );
        assert!(
            body.contains(r#""contested":{"kind":"none""#),
            "the vendor claim that lost is carried, by name: {body}"
        );

        // THE RUNG IS SERVED NOW, AND ITS FLOOR IS THE SAME FACT IT ALWAYS WAS.
        //
        // This asserted `"served":false` — Dhan's minute rung was withdrawn
        // until the intraday endpoint existed. It exists, so the flag flips and
        // the five-year floor beside it does not move: the floor was read from
        // the vendor's own intraday page ("for last 5 years") the whole time,
        // which is exactly why `served` and the floor are separate fields. What
        // this build ASKS FOR and what the vendor OFFERS are two facts, and a
        // rung going from withdrawn to served changes only the first.
        assert!(
            body.contains(r#"{"rung":"1min","served":true,"kind":"rolling","unit":"y","n":5"#),
            "the minute rung is served and carries the floor its own page states: {body}"
        );

        // Every entry carries every key, so a reader never has to tell an
        // absent field from a null one.
        for key in [
            r#""kind":"#,
            r#""unit":"#,
            r#""n":"#,
            r#""from":"#,
            r#""oldest":"#,
            r#""source":"#,
            r#""binds_because":"#,
            r#""contested":"#,
        ] {
            assert!(body.contains(key), "every entry carries {key}: {body}");
        }
        // THE SOURCE KIND IS ON THE WIRE ONCE. `transport` is gone: it was a
        // second spelling (`broker` / `archive`) of the split `kind` already
        // carries, produced by its own `match` in the same function, and the
        // browser had started guessing one from the other. `kind`, `kind_label`
        // and `verb` are all `SourceKind`'s, read from one value per feed.
        assert!(body.starts_with(
            r#"[{"wire":"dhan","display":"Dhan","kind":"rest","kind_label":"REST API","verb":"pull","ready":true,"why":""#
        ), "{body}");
        assert!(
            !body.contains(r#""transport":"#),
            "the second spelling of the source kind is gone, not merely unused: {body}"
        );
        // A BROKER IS PULLED; A FOLDER IS READ. The two archive vendors carry
        // the other word, and no row carries both.
        for (wire, kind, verb) in [
            ("truedata", "folder", "read"),
            ("gdfl", "folder", "read"),
            ("groww", "rest", "pull"),
        ] {
            let row = body
                .split("{\"wire\":")
                .find(|part| part.starts_with(&format!("\"{wire}\"")))
                .unwrap_or_else(|| panic!("a row for {wire}: {body}"));
            assert!(
                row.contains(&format!(r#""kind":"{kind}""#)),
                "{wire} is a {kind}: {row}"
            );
            assert!(
                row.contains(&format!(r#""verb":"{verb}""#)),
                "{wire} is {verb}: {row}"
            );
        }
        // AND THE LABEL IS THE TYPE'S OWN WORDS. The page prints this; it used
        // to print `transport`, whose vocabulary lived in this file alone.
        for (wire, label) in [("dhan", "REST API"), ("gdfl", "folder of files")] {
            let row = body
                .split("{\"wire\":")
                .find(|part| part.starts_with(&format!("\"{wire}\"")))
                .unwrap_or_else(|| panic!("a row for {wire}: {body}"));
            assert!(
                row.contains(&format!(r#""kind_label":"{label}""#)),
                "{wire} is labelled by SourceKind: {row}"
            );
        }
    }

    /// THE GRANULARITY FLOOR LEAVES THIS PROCESS WHOLE, AND A SNAPSHOT IS
    /// NEVER CALLED A TICK.
    ///
    /// The /ingest page held a transcription of all four `GranularityFloor`
    /// consts because there was no field to read. Every part of the fact is
    /// emitted here — the rung, what one record at it IS, the two booleans that
    /// decide what may be printed beside a number, and the vendor's own words
    /// with the place they were read — so the copy in the browser is one that
    /// can be deleted rather than one that has to be kept in step by hand.
    #[tokio::test]
    async fn feeds_json_carries_the_granularity_floor_and_never_calls_a_snapshot_a_tick() {
        let dir = masters("feeds-finest", None, None);
        let site = Loaded::new(Site::load(&dir, &store_root("feeds-finest")));
        let (_, body) = feeds_json(axum::extract::State(site)).await;

        // TWO SHAPES, FOUR ROWS. The brokers bottom out at a minute and one
        // record there is a BAR; the archives bottom out at a second and one
        // record there is a CONFLATED SNAPSHOT, which is a different object at
        // the same number.
        for (wire, rung, kind, conflated) in [
            ("dhan", "1min", "bar", false),
            ("groww", "1min", "bar", false),
            ("truedata", "1s", "snapshot", true),
            ("gdfl", "1s", "snapshot", true),
        ] {
            let row = body
                .split("{\"wire\":")
                .find(|part| part.starts_with(&format!("\"{wire}\"")))
                .unwrap_or_else(|| panic!("a row for {wire}: {body}"));
            assert!(
                row.contains(&format!(
                    r#""finest":{{"rung":"{rung}","kind":"{kind}","label":""#
                )),
                "{wire}'s floor is {rung} and one record is a {kind}: {row}"
            );
            assert!(
                row.contains(&format!(r#""tick_stream":false,"conflated":{conflated},"#)),
                "{wire}: the two booleans that decide what may be written beside \
                 the number: {row}"
            );
        }

        // NO ROW CLAIMS A TICK STREAM, and the wire says so in a field rather
        // than by omission. `FinestKind::Tick` is constructed by no descriptor
        // in this build; a reader must be able to see that stated.
        assert!(
            !body.contains(r#""tick_stream":true"#),
            "no feed in this build serves a tick stream: {body}"
        );

        // THE VENDOR'S WORDS TRAVEL VERBATIM, NOT PARAPHRASED. Compared against
        // the descriptor itself, so a reworded const is a failing test here
        // rather than a browser quietly showing yesterday's reason.
        for feed in pull::vendor::Feed::ALL {
            let floor = feed.descriptor().granularity_floor;
            assert!(
                body.contains(render::json_string(floor.because).trim_matches('"')),
                "{} carries its own reason: {body}",
                feed.wire()
            );
            assert!(
                body.contains(render::json_string(floor.source).trim_matches('"')),
                "{} carries where that was read — CLAUDE.md section 3 rule 1: {body}",
                feed.wire()
            );
        }
    }

    /// THREE KINDS, THREE WORDS, AND THE UNREACHED ONE IS NAMED HERE.
    ///
    /// `FinestKind::Tick` is constructed by no descriptor and the `const` block
    /// under `DESCRIPTORS` keeps it that way. Its arm is therefore unreachable
    /// through `feeds_json`, which is why the word lives in its own `const fn`
    /// — a region that can never run is a coverage hole `CLAUDE.md` section 9
    /// has no way to forgive, and this test is what runs it.
    #[test]
    fn finest_kind_words_are_three_and_the_tick_word_is_one_of_them() {
        assert_eq!(finest_kind_word(pull::vendor::FinestKind::Tick), "tick");
        assert_eq!(
            finest_kind_word(pull::vendor::FinestKind::ConflatedSnapshot),
            "snapshot",
            "and it is NOT the tick word — the whole point of the variant"
        );
        assert_eq!(finest_kind_word(pull::vendor::FinestKind::Bar), "bar");
    }

    /// A FLOOR THAT NAMES NO DAY RESOLVES TO NO DAY — both of them, and for
    /// two different reasons that the resolver is deliberately blind to.
    #[test]
    fn the_two_floors_that_name_no_day_resolve_to_none() {
        let today = Day::new(2026, 8, 12).expect("a real date");
        assert_eq!(
            floor_oldest(pull::vendor::HistoryFloor::Unbounded, today),
            None,
            "the vendor says it holds everything, so there is no oldest day"
        );
        assert_eq!(
            floor_oldest(pull::vendor::HistoryFloor::Unstated, today),
            None,
            "and nobody has said anything, which is not the same fact"
        );
        // A fixed floor resolves to itself, and a rolling one to a day before
        // today — through the clamp, so this is the day a pull would be given.
        assert_eq!(
            floor_oldest(
                pull::vendor::HistoryFloor::Fixed {
                    year: 2020,
                    month: 1,
                    day: 1
                },
                today
            ),
            Some(Day::new(2020, 1, 1).expect("a real date"))
        );
        assert_eq!(
            floor_oldest(
                pull::vendor::HistoryFloor::RollingMonths { months: 3 },
                today
            ),
            Some(Day::new(2026, 5, 12).expect("a real date")),
            "three calendar months, not ninety days"
        );
        // A CLAIM NOBODY MADE IS `unknown`, WITH A NULL SOURCE. This is the
        // arm a rung with no row takes, and it must not read as "no limit".
        let fields = claim_fields(None, Some(today));
        assert!(
            fields.contains(r#""kind":"unknown""#) && fields.contains(r#""source":null"#),
            "{fields}"
        );
        // And a floor whose date is not a date resolves to nothing rather than
        // to a day it invented. 30 February is the case.
        assert_eq!(
            floor_oldest(
                pull::vendor::HistoryFloor::Fixed {
                    year: 2020,
                    month: 2,
                    day: 30
                },
                today
            ),
            None
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod broker_target_tests {
    use super::*;

    /// **THE TARGET GUARD IS GONE, AND IT MUST NOT COME BACK ON THE OLD
    /// PREMISE.**
    ///
    /// `broker_window` used to refuse every target but `Swept`, on the stated
    /// ground that *"`pull::vendor::HttpSpec` has no request-parameter map, so no
    /// instrument is put on the wire at all"*. Both halves were false by the
    /// time it was removed, and this test pins the facts that make them false
    /// rather than the absence of a line — an absence is trivially satisfied by
    /// deleting the test.
    ///
    /// The predecessor asserted the guard's POSITION and was correct to; it is
    /// replaced rather than deleted because the ordering it protected still
    /// matters and is now covered by
    /// `the_transport_is_checked_before_any_vendor_facing_cost`, which walks
    /// every costly call in the same body.
    #[test]
    fn the_broker_path_addresses_a_set_and_no_target_guard_stands_in_its_way() {
        let src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server.rs"),
        )
        .expect("this module's own source");

        let body = src
            .split_once("async fn broker_window")
            .expect("broker_window exists")
            .1;
        let end = body.find("\n}\n").expect("it has an end");
        let body = &body[..end];

        // 1. THE GUARD IS ABSENT. Written as a substring of the comparison
        //    itself, so re-adding it in any spelling that compares the target
        //    to a single variant trips this.
        assert!(
            !body.contains("asked.target != ingest::SpotTarget::"),
            "broker_window refuses a target again. Before restoring this, read \
             the two facts below — the guard's stated reason was false when it \
             was removed, and nothing has narrowed since."
        );

        // 2. THE INSTRUMENT IS AN ARGUMENT, so this function was never
        //    single-instrument by construction — its CALLER decides the set.
        let signature = src
            .split_once("async fn broker_window(")
            .expect("broker_window exists")
            .1;
        let signature = &signature[..signature.find(") -> ").expect("it has a return type")];
        assert!(
            signature.contains("instrument: &brutex_core::instrument::InstrumentKey"),
            "broker_window takes ONE instrument as an argument; the set is the \
             caller's business: {signature}"
        );

        // 3. THE CALLER LOOPS. `broker_run` filters the merged universe by the
        //    chosen target, sorts it for reproducibility, and calls this once
        //    per member.
        let run = src
            .split_once("pub(crate) async fn broker_run")
            .expect("broker_run exists")
            .1;
        let run = &run[..run.find("\n}\n").expect("it has an end")];
        assert!(
            run.contains("asked.target.names(key, entry.universe)"),
            "broker_run builds its instrument list from the chosen target"
        );
        assert!(
            run.contains("for (index, instrument) in targets.iter().enumerate()"),
            "broker_run loops the list it built"
        );

        // 4. THE PARAMETER MAP EXISTS AND BOTH BROKERS FILL IT. This is the
        //    clause the guard's message named, and it is checked against the
        //    descriptors rather than against source text — a populated `params`
        //    table is what puts an instrument id on the wire and what closed
        //    `DH-905 securityId is required`.
        for feed in [pull::vendor::Feed::Dhan, pull::vendor::Feed::Groww] {
            let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
                panic!("{feed} is an HTTP broker");
            };
            assert!(
                spec.params
                    .iter()
                    .any(|p| p.value == pull::vendor::ParamValue::InstrumentId),
                "{feed} must name the instrument on its wire, or the guard's \
                 reason becomes true again"
            );
        }
    }

    /// Every target is REQUESTABLE on the broker path, and none of them widens
    /// the sweep.
    ///
    /// The predecessor asserted that exactly ONE of seven was servable and
    /// called that "the reminder to widen the guard rather than delete it when
    /// the parameter map lands". The map landed, the guard is deleted, and this
    /// is the assertion that replaces it: what a target selects is a SET TO
    /// STORE, and `CLAUDE.md` §1's two-instrument sweep surface is untouched by
    /// any of them.
    #[test]
    fn every_target_is_requestable_and_none_of_them_widens_the_sweep() {
        assert_eq!(ingest::SpotTarget::ALL.len(), 9);
        assert_eq!(
            brutex_core::instrument::InstrumentKey::SWEPT.len(),
            2,
            "CLAUDE.md §1: the engine surface is NSE-NIFTY and NSE-BANKNIFTY, \
             and widening it is a docs/05-decisions.md entry — never a \
             side-effect of adding a target that STORES more"
        );
        // Several targets legitimately NAME the swept pair — `Indices` always
        // has, and `Fno` and `Everything` now do. Naming is STORING; sweeping
        // is `is_sweepable`, and no target can reach it. `SWEPT` is a table of
        // `(Exchange, &str)` pairs rather than of keys, so the predicate is
        // asked of a key BUILT from each pair — which is also the only way to
        // prove the table and the predicate still agree.
        for (exchange, symbol) in brutex_core::instrument::InstrumentKey::SWEPT {
            let key = brutex_core::instrument::InstrumentKey::index(exchange, symbol)
                .expect("a swept pair is a valid key");
            assert!(
                key.is_sweepable(),
                "{exchange:?}-{symbol} is the surface itself"
            );
        }
    }
}

/// Every universe bit, with the one word the wire spells it as.
///
/// **The single spelling table.** Two lists of these words is how one of them
/// says `ntm` while the other says `total_market` and a browser filter matches
/// nothing — so the compatibility field and the full field below are both
/// generated from THIS array and cannot disagree about a bit's name.
///
/// Ordered by bit position ascending. The order is fixed so a response is
/// byte-identical between reloads (`CLAUDE.md` §3 rule 5) and for no other
/// reason: membership is a SET, the array is not a ranking, and nothing may
/// read position 0 as "the most important universe". `core::universe` says the
/// same thing about the bits themselves.
///
/// The first [`LEGACY_UNIVERSE_TOKENS`] entries are the three universes that
/// existed before D-0089, in the order [`universe_label`] has always emitted
/// them. Appending to this array is how a new universe reaches the wire;
/// REORDERING the first three changes a shipped field and is forbidden by the
/// same append-only rule that governs the bits (§3 rule 8).
const UNIVERSE_TOKENS: [(brutex_core::universe::Universe, &str); 7] = [
    (brutex_core::universe::Universe::INDEX, "index"),
    (brutex_core::universe::Universe::FNO, "fno"),
    (brutex_core::universe::Universe::TOTAL_MARKET, "ntm"),
    (brutex_core::universe::Universe::NIFTY_500, "n500"),
    (brutex_core::universe::Universe::NIFTY_200, "n200"),
    (brutex_core::universe::Universe::NIFTY_100, "n100"),
    (brutex_core::universe::Universe::NIFTY_50, "n50"),
];

/// The one word the wire spells ONE universe bit as.
///
/// The reverse of [`universe_tokens`], which takes a whole bitset. This takes a
/// single named bit — what `ingest::SpotTarget::universe` hands back — so
/// `/universes.json` can tell a page which token of the `universes` ARRAY picks
/// out the rows a target covers, WITHOUT a second table of these words. Two
/// lists of them is how one says `ntm` and the other says `total_market`.
///
/// `""` for a bitset that is not exactly one of the seven — the empty set, or
/// two bits at once. Empty rather than a guess, and rather than a panic: this
/// is called from a JSON writer, and a caller with a compound bitset wanted the
/// array field. `api::server::a_compound_bitset_has_no_single_token` pins it.
///
/// One pass over a compile-time array of seven. `CLAUDE.md` §3 rule 4.
pub(crate) fn universe_token_of(u: brutex_core::universe::Universe) -> &'static str {
    UNIVERSE_TOKENS
        .into_iter()
        .find(|(bit, _)| *bit == u)
        .map_or("", |(_, token)| token)
}

/// How many of [`UNIVERSE_TOKENS`] the compatibility field may name.
///
/// Three: `index`, `fno`, `ntm`. Pinned so that appending an eighth universe
/// widens [`universe_tokens`] and leaves [`universe_label`] alone, which is the
/// whole content of the compatibility promise.
const LEGACY_UNIVERSE_TOKENS: usize = 3;

/// The universe a listing belongs to, as the ONE string old readers parse.
///
/// # This field is frozen, and that is its job
///
/// `universe` on `/instruments.json` has always been `index`, `fno`, `ntm`,
/// those joined by `+`, or `other`. A running page splits it on `+` and tests
/// membership against exactly those words. D-0089 appended four bits —
/// NIFTY 50 / 100 / 200 / 500 — and this function deliberately does **not**
/// emit them: a row that started answering `fno+ntm+n500+n200+n100+n50` would
/// still parse, and would still group correctly under `split('+')`, but any
/// reader comparing the whole string, or grouping by it, or writing it into a
/// bookmark, would silently change behaviour. The new memberships travel in
/// [`universe_tokens`] instead, so an old reader keeps working and a new one
/// gets the full set.
///
/// `Universe` is a bitset and an instrument is in several at once — a NIFTY
/// Total Market equity that also has F&O contracts is in both — which is why
/// this was never expressive enough and why the array field exists. It stays
/// because deleting a shipped field is a change to somebody else's page.
fn universe_label(u: brutex_core::universe::Universe) -> String {
    let parts: Vec<&str> = UNIVERSE_TOKENS
        .iter()
        .take(LEGACY_UNIVERSE_TOKENS)
        .filter(|(bit, _)| u.contains(*bit))
        .map(|(_, token)| *token)
        .collect();
    if parts.is_empty() {
        "other".to_owned()
    } else {
        parts.join("+")
    }
}

/// Every universe a listing belongs to, as the JSON array `universes`.
///
/// # Why an array beside a string
///
/// Membership is a set: a NIFTY 50 name is also in the 100, the 200, the 500
/// and the Total Market, and it is usually an F&O underlying too. One word
/// cannot say that, and `universe_label`'s `+` join said it only by convention
/// — a reader had to know to split. An array says it in the type.
///
/// An instrument in nothing yields `[]`. The compatibility field spells that
/// same fact `"other"`, because a field that must always be one token has to
/// put *something* there; the array does not, and inventing an `"other"`
/// element would make emptiness look like a membership.
///
/// # Cost
///
/// Seven `contains` — one mask and one compare each — over a table whose length
/// is a compile-time constant, plus one `Vec` of at most seven `&'static str`.
/// Nothing is looked up by symbol: `entry.universe` was computed once by
/// `core::universe::of_instrument` when the masters were merged, so the
/// per-row cost here does not include the six `MemberIndex` probes that
/// produced it, and it does not grow with the instrument set.
///
/// O(1) per row **by construction and UNVERIFIED as a measurement**: no bench
/// times this function, so what is claimed above is the shape of the loop and
/// not a number. `docs/06-limits.md` §11 carries the admission rather than this
/// comment carrying a bound nobody took.
fn universe_tokens(u: brutex_core::universe::Universe) -> Vec<&'static str> {
    UNIVERSE_TOKENS
        .iter()
        .filter(|(bit, _)| u.contains(*bit))
        .map(|(_, token)| *token)
        .collect()
}

/// The `universes` array, escaped, as one JSON value.
///
/// Every element goes through [`render::json_string`] like every other value in
/// this document — the tokens are compile-time literals and could be written
/// raw, and the day one is not a literal is the day a raw write is a hole.
fn universes_json(u: brutex_core::universe::Universe) -> String {
    let mut out = String::from("[");
    for (n, token) in universe_tokens(u).into_iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        out.push_str(&render::json_string(token));
    }
    out.push(']');
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod percentage_tests {
    use super::*;
    use brutex_core::instrument::{Exchange, Segment};
    use brutex_core::symbol::Symbol;
    use pull::manifest::{Closes, Entry, EntryKey, Held, Manifest};
    use store::path::{Timeframe, YearMonth};

    fn month(year: u16, ordinal: u8) -> YearMonth {
        YearMonth::new(year, ordinal).expect("a month")
    }

    fn series(segment: Segment, symbol: &str) -> census::Series {
        series_at(segment, symbol, Timeframe::MINUTE_1)
    }

    /// The same series, at a rung the caller names.
    ///
    /// `series` hardcoded `MINUTE_1`, which is fine for the arithmetic tests
    /// below — and is exactly why no test in this module could see a writer
    /// that hardcoded the same word.
    fn series_at(segment: Segment, symbol: &str, timeframe: Timeframe) -> census::Series {
        census::Series {
            exchange: Exchange::Nse,
            segment,
            symbol: Symbol::new(symbol).expect("a symbol"),
            timeframe,
        }
    }

    fn key(series: census::Series, at: YearMonth) -> EntryKey {
        series.at(at)
    }

    /// A census held entirely in memory, at a path that does not exist.
    ///
    /// The path is deliberately a name nothing ever writes: every assertion
    /// below that reads a close therefore reads it from the manifest, and a
    /// change that reached for a bar file would fail rather than pass slowly.
    fn census_of(rows: &[(census::Series, YearMonth, u64, Closes)]) -> census::VendorCensus {
        let mut manifest = Manifest::open(Vendor::Groww, &[], &[]).expect("a genesis census");
        for &(series, at, bars, closes) in rows {
            manifest
                .record_held(Held::new(
                    Entry {
                        key: key(series, at),
                        rows: bars,
                        first_ts_micros: 1_751_350_800_000_000,
                        last_ts_micros: 1_751_363_940_000_000,
                    },
                    closes,
                ))
                .expect("records");
        }
        census::VendorCensus {
            vendor: Vendor::Groww,
            path: PathBuf::from("/nonexistent/percentage-tests/groww.man"),
            state: census::Census::Held {
                manifest: Box::new(manifest),
            },
        }
    }

    fn priced(first: i64, last: i64) -> Closes {
        Closes::known(first, last).expect("two prices")
    }

    /// The one row `store_body` produced, so an assertion names a field rather
    /// than a substring of the whole array.
    fn only_row(body: &str) -> String {
        let inner = body
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .expect("a JSON array");
        assert!(
            !inner.contains("},{"),
            "this helper is for a single-row body: {body}"
        );
        inner.to_owned()
    }

    /// A feed refuses a rung it does not declare, and the refusal names both.
    ///
    /// The regression this pins is a whole class. `Feed::serves` existed with
    /// NO caller outside its own tests, so withdrawing Dhan's `Minute1` from
    /// its `granularities` row was documentation and not a gate.
    /// `autopilot::round` pins `Minute1` for every feed, so the withdrawn rung
    /// still reached that vendor's DAILY endpoint and its daily candles were
    /// filed under `1min/` — silent wrong data in an append-only store.
    #[test]
    fn a_feed_refuses_a_rung_it_does_not_declare_and_names_both() {
        use pull::vendor::{Feed, Granularity};

        assert_eq!(
            served(Feed::Dhan, Granularity::Day1),
            Ok(()),
            "the rung this feed's bars path is pinned to must be admitted"
        );
        assert_eq!(
            served(Feed::Dhan, Granularity::Minute1),
            Ok(()),
            "the minute rung is addressable now: its own endpoint and its own \
             interval field both exist"
        );
        // AND A RUNG THAT STILL IS NOT ADDRESSABLE STILL REFUSES, BY NAME.
        // `Minute5` is undeclared on this feed — `vendor::\
        // a_feed_serves_only_the_rungs_its_row_declares` pins it — so the
        // behaviour under test is unchanged and only the rung exhibiting it
        // moved.
        let why = served(Feed::Dhan, Granularity::Minute5)
            .expect_err("a rung this feed cannot address must refuse");
        assert!(
            why.contains("Dhan"),
            "the refusal names the feed, so the operator knows which descriptor \
             row to amend: {why}"
        );
        assert!(
            why.contains(&Granularity::Minute5.to_string()),
            "and names the rung that was asked for: {why}"
        );

        // EVERY DECLARED RUNG IS ADMITTED, for every feed. A gate that refuses
        // what the descriptor declares is the opposite failure and just as
        // quiet — the pull simply never happens.
        for feed in Feed::ALL {
            for rung in Granularity::ALL {
                assert_eq!(
                    served(feed, rung).is_ok(),
                    feed.serves(rung),
                    "{} and {rung} must agree with the descriptor's own row",
                    feed.display()
                );
            }
        }
    }

    /// `?timeframe=` defaults, resolves, and refuses BY NAME.
    ///
    /// The parser gates every bars read on both routes. It had no test at all —
    /// its three branches, including the refusal sentence its own doc comment
    /// promises, were unproven. This test was also lost once to a stray
    /// `git checkout` and is re-stated here rather than assumed.
    #[test]
    fn the_rung_parameter_defaults_to_minutes_and_refuses_an_unknown_rung_by_name() {
        assert_eq!(
            timeframe_param(""),
            Ok(store::path::Timeframe::MINUTE_1),
            "an absent rung is the grid every pre-D-0055 link meant"
        );
        assert_eq!(
            timeframe_param("symbol=NIFTY&month=2026-07"),
            Ok(store::path::Timeframe::MINUTE_1),
            "absent AMONG other params is still absent"
        );
        assert_eq!(
            timeframe_param("timeframe=1min"),
            Ok(store::path::Timeframe::MINUTE_1)
        );
        assert_eq!(
            timeframe_param("timeframe=1day"),
            Ok(store::path::Timeframe::DAY_1),
            "the rung the whole store currently holds must be reachable"
        );

        let why = timeframe_param("timeframe=1hour").expect_err("an unheld rung must refuse");
        assert!(
            why.contains("1hour"),
            "the refusal names what was asked: {why}"
        );
        assert!(
            why.contains("1min") && why.contains("1day"),
            "and names what this store does hold, so the operator can retry: {why}"
        );

        // EVERY KNOWN RUNG IS REACHABLE FROM THE WIRE. A rung added to
        // `Timeframe::KNOWN` and not here is a directory the store writes and
        // no request can name.
        for tf in store::path::Timeframe::KNOWN {
            assert_eq!(
                timeframe_param(&format!("timeframe={}", tf.as_str())),
                Ok(*tf),
                "{} is in Timeframe::KNOWN and must be requestable",
                tf.as_str()
            );
        }
    }

    /// **A STORE ROW NAMES THE RUNG IT IS STORED AT, NEVER A FIXED WORD.**
    ///
    /// `store_body` wrote `"timeframe":"1min"` as a literal on every row while
    /// `Series` carried the real `Timeframe`. Every test in this module built
    /// its series at `MINUTE_1`, so all 321 of them agreed with the literal and
    /// none of them could tell the difference — the writer was reverted to the
    /// literal and the suite stayed green.
    ///
    /// This test builds a `DAY_1` series specifically so it cannot: it asserts
    /// the row says `1day` AND that it does not say `1min`, which is what makes
    /// reverting the writer a failure rather than a wash.
    #[test]
    fn a_store_row_names_the_rung_it_is_stored_at_and_never_a_fixed_word() {
        let daily = series_at(Segment::Index, "NIFTY", Timeframe::DAY_1);
        let at = month(2026, 7);
        let census = census_of(&[(daily, at, 23, priced(10_000_000, 10_125_000))]);
        let body = only_row(&store_body(&[census], &[(daily, at)], Vendor::Groww));
        assert!(
            body.contains(r#""timeframe":"1day""#),
            "a 1day series must say 1day: {body}"
        );
        assert!(
            !body.contains(r#""timeframe":"1min""#),
            "a 1day series must not say 1min: {body}"
        );

        // The other rung, from the same writer, so the assertion above is
        // reading the field and not a word that happens to be everywhere.
        let minute = series_at(Segment::Index, "NIFTY", Timeframe::MINUTE_1);
        let census = census_of(&[(minute, at, 8_250, priced(10_000_000, 10_125_000))]);
        let body = only_row(&store_body(&[census], &[(minute, at)], Vendor::Groww));
        assert!(
            body.contains(r#""timeframe":"1min""#),
            "a 1min series must say 1min: {body}"
        );
    }

    /// **EVERY RUNG THE STORE KNOWS IS REACHABLE FROM THE WIRE.**
    ///
    /// `timeframe_param` had no test at all: an absent value defaulting to
    /// one minute, a named rung, and a refusal by name were three behaviours
    /// its doc comment promised and nothing checked.
    ///
    /// The last assertion is the one that matters after D-0055: it walks
    /// `Timeframe::KNOWN` rather than naming rungs, so a third rung added to
    /// the store is either reachable here or this test fails.
    #[test]
    fn every_rung_the_store_knows_is_reachable_from_the_wire_and_an_unknown_one_is_refused() {
        // Absent, not empty-and-present: the bookmark case.
        assert_eq!(timeframe_param(""), Ok(Timeframe::MINUTE_1));
        assert_eq!(
            timeframe_param("symbol=NIFTY&month=2026-07"),
            Ok(Timeframe::MINUTE_1),
            "an absent rung is the one-minute grid, so old links keep working"
        );
        assert_eq!(timeframe_param("timeframe=1min"), Ok(Timeframe::MINUTE_1));
        assert_eq!(
            timeframe_param("timeframe=1day"),
            Ok(Timeframe::DAY_1),
            "the rung 100% of the data on disk is stored at"
        );

        // PRESENT AND UNKNOWN IS REFUSED BY NAME, never defaulted.
        let why = timeframe_param("timeframe=1hour").expect_err("no 1hour directory");
        assert!(
            why.contains("1hour"),
            "the refusal names what was asked: {why}"
        );
        assert!(
            why.contains("1min") && why.contains("1day"),
            "the refusal names what the store does hold: {why}"
        );

        // EVERY member of the store's own list, so a new rung cannot be added
        // to `KNOWN` and left unreachable from the wire.
        for rung in Timeframe::KNOWN {
            assert_eq!(
                timeframe_param(&format!("timeframe={}", rung.as_str())),
                Ok(*rung),
                "{} is in Timeframe::KNOWN and must be askable",
                rung.as_str()
            );
        }
    }

    /// **A MONTH THAT MOVED 1.25% IS 125, AND A FLAT MONTH IS A REAL ZERO.**
    ///
    /// The comment on `store_json` promised integer basis points for years
    /// before anything computed one. This is the arithmetic that comment
    /// describes, in the units it names: ₹100.00 to ₹101.25 is `125`.
    ///
    /// The flat case is the other half and it is not decoration. Zero basis
    /// points is a month that closed where it opened — a fact — and the whole
    /// design of the `null`-plus-reason wire shape exists so that fact is never
    /// confused with "nobody knows". If `0` could mean both, the column would
    /// be unreadable on exactly the rows an operator looks hardest at.
    #[test]
    fn a_month_that_moved_1_25_percent_is_125_basis_points_and_a_flat_month_is_a_real_zero() {
        assert_eq!(
            month_change(Segment::Index, Some(priced(10_000, 10_125))),
            Ok(125),
            "₹100.00 to ₹101.25 is +1.25%, which is 125 basis points"
        );
        assert_eq!(
            month_change(Segment::Index, Some(priced(10_000, 9_875))),
            Ok(-125),
            "and the same move down is -125, sign carried by the number itself"
        );
        assert_eq!(
            month_change(Segment::Index, Some(priced(295_050, 295_050))),
            Ok(0),
            "a month that closed where it opened is ZERO, not unknown"
        );
    }

    /// **HALF ROUNDS AWAY FROM ZERO, AND IT DOES SO IN BOTH DIRECTIONS.**
    ///
    /// `CLAUDE.md` §7's half-up rule governs snapping a PRICE to the tick grid
    /// at the WRITE boundary. A percentage is neither. Under half-up, +0.5 bp
    /// renders `1` and -0.5 bp renders `0` — a gain and its mirror-image loss
    /// printing different magnitudes, which is visible the moment the column is
    /// sorted and inexplicable when it is noticed.
    ///
    /// The fixture is an exact tie by construction: a base of 32 paisa and a
    /// one-paisa move is 10,000/32 = 312.5 basis points exactly, so nothing
    /// here depends on a rounding accident. A half-up implementation returns
    /// `313` and `-312` and fails the second assertion alone.
    #[test]
    fn basis_points_round_half_away_from_zero_in_both_directions() {
        assert_eq!(
            month_change(Segment::Index, Some(priced(32, 33))),
            Ok(313),
            "312.5 bp exactly, rounded away from zero"
        );
        assert_eq!(
            month_change(Segment::Index, Some(priced(32, 31))),
            Ok(-313),
            "-312.5 bp exactly, rounded away from zero — NOT -312"
        );
        assert_eq!(
            month_change(Segment::Index, Some(priced(32, 34))),
            Ok(625),
            "and a move that divides exactly is not nudged"
        );
    }

    /// **A BASE OF ZERO IS REFUSED, NOT DIVIDED BY.**
    ///
    /// `docs/02-store-format.md` §3: "An all-zero record is a legal flat bar",
    /// so a first close of zero paisa is a state the format permits and this is
    /// a check rather than a defensive comment. A ratio against it is undefined
    /// — it is not `0`, it is not infinity, and it is not an empty cell that
    /// means four other things.
    #[test]
    fn a_base_of_zero_paisa_is_refused_rather_than_divided_by() {
        assert_eq!(
            month_change(Segment::Index, Some(priced(0, 500))),
            Err(Unknown::BaseNotPositive)
        );
        assert_eq!(
            month_change(Segment::Index, Some(priced(0, 0))),
            Err(Unknown::BaseNotPositive),
            "zero to zero is still no base, and 0% would be an invention"
        );
        assert_eq!(Unknown::BaseNotPositive.code(), "base_not_positive");
    }

    /// **A MOVE THAT LEAVES `i64` IS REFUSED, NOT WRAPPED.**
    ///
    /// The scaling by 10,000 is the only step that can overflow, and the bound
    /// is exact: it fits while `|delta| <= i64::MAX / 10_000 =
    /// 922,337,203,685,477` paisa. Both sides of that boundary are asserted, so
    /// the arm is a bound and not a guess.
    #[test]
    fn a_move_that_leaves_i64_is_refused_rather_than_wrapped() {
        let widest = i64::MAX / 10_000;
        assert_eq!(
            month_change(Segment::Index, Some(priced(1, widest + 1))),
            Ok(widest * 10_000),
            "a base of one paisa makes the delta the whole value; this one fits"
        );
        assert_eq!(
            month_change(Segment::Index, Some(priced(1, widest + 2))),
            Err(Unknown::Overflow),
            "one paisa further and the scaled move leaves i64"
        );
        assert_eq!(Unknown::Overflow.code(), "overflow");
    }

    /// **BASIS POINTS DO NOT FIT `i32`, AND AN ORDINARY PRICE PROVES IT.**
    ///
    /// `i32::MAX` is 2,147,483,647 basis points, which sounds unreachable until
    /// the base is small: a first close of one paisa and a last close of
    /// ₹2,147.50 — an utterly ordinary NSE share price — is 2,147,490,000 basis
    /// points. A one-paisa base is garbage data, and the store can hold it;
    /// `CLAUDE.md` §3 rule 6 does not permit claiming a bound that is not
    /// enforced. So the value is computed and carried as `i64`.
    #[test]
    fn basis_points_do_not_fit_i32_and_an_ordinary_price_proves_it() {
        let over = month_change(Segment::Index, Some(priced(1, 214_750))).expect("a number");
        assert_eq!(over, 2_147_490_000);
        assert!(
            over > i64::from(i32::MAX),
            "{over} is past i32::MAX = {}, from a base of ₹0.01 and a close of ₹2,147.50",
            i32::MAX
        );
    }

    /// **NO EQUITY RENDERS A NUMBER WHILE NO THRESHOLD IS SOURCED.**
    ///
    /// `docs/05-decisions.md` D-0018 already decided this: a suspected
    /// corporate action is refused loudly, never back-adjusted from a source no
    /// vendor has been verified to supply. No threshold exists anywhere in this
    /// repository, so every segment that can be re-based is refused and only
    /// an index — which never splits — renders.
    ///
    /// **The gate outranks the data, and the third assertion is why.** An
    /// equity whose closes ARE recorded is still refused; an equity whose
    /// closes are NOT recorded is refused for the corporate action rather than
    /// for the missing close, because an operator told "no close recorded"
    /// would re-ingest the month and wait forever for a number.
    #[test]
    fn no_equity_month_renders_a_number_while_no_threshold_is_sourced() {
        assert_eq!(
            month_change(Segment::Index, Some(priced(10_000, 10_125))),
            Ok(125),
            "an index never splits, so it renders"
        );
        assert_eq!(
            month_change(Segment::Cash, Some(priced(10_000, 10_125))),
            Err(Unknown::CorporateActionUnverified),
            "the same two prices under a segment that can split are refused"
        );
        assert_eq!(
            month_change(Segment::Fno, Some(priced(10_000, 10_125))),
            Err(Unknown::CorporateActionUnverified),
            "a contract is adjusted when its underlying is, so it inherits the hazard"
        );
        assert_eq!(
            month_change(Segment::Cash, Some(Closes::UNKNOWN)),
            Err(Unknown::CorporateActionUnverified),
            "the permanent reason outranks the temporary one"
        );
        // AND IT OUTRANKS THE MISSING NEIGHBOUR TOO, which is the arm that
        // makes the Prev column consistent. An equity whose previous month is
        // simply absent must not read `no_earlier_month`: that sends an
        // operator to ingest a month, and no pull will ever make this cell a
        // number. Only a sourced threshold will.
        assert_eq!(
            month_change(Segment::Cash, None),
            Err(Unknown::CorporateActionUnverified)
        );
        assert_eq!(
            month_change(Segment::Index, None),
            Err(Unknown::NoEarlierMonth),
            "for an index, an absent neighbour is exactly what it says"
        );
        assert_eq!(
            Unknown::CorporateActionUnverified.code(),
            "corporate_action_unverified"
        );
        assert!(
            !can_be_rebased(Segment::Index)
                && can_be_rebased(Segment::Cash)
                && can_be_rebased(Segment::Fno)
        );
    }

    /// **A MONTH WITH NO CLOSE RECORDED IS UNKNOWN, AND NEVER ZERO.**
    ///
    /// Every version-1 census entry reads back this way, and so does every
    /// month this build committed before it read a bar file — D-0067. On the
    /// wire it is `null` and a code, so the browser cannot render it as a
    /// number by accident.
    #[test]
    fn a_month_with_no_close_recorded_is_unknown_and_never_zero() {
        assert_eq!(
            month_change(Segment::Index, Some(Closes::UNKNOWN)),
            Err(Unknown::NotRecorded)
        );
        let nifty = series(Segment::Index, "NIFTY");
        let at = month(2026, 7);
        let body = store_body(
            &[census_of(&[(nifty, at, 8_250, Closes::UNKNOWN)])],
            &[(nifty, at)],
            Vendor::Groww,
        );
        let row = only_row(&body);
        assert!(
            row.contains(r#""chg_bps":null,"chg_why":"not_recorded""#),
            "unknown on the wire is null and a reason: {row}"
        );
        assert!(
            !row.contains(r#""chg_bps":0"#),
            "and it is never a zero: {row}"
        );
    }

    /// **THE MONTH BEFORE JANUARY IS THE DECEMBER BEFORE IT, AND 1970-01 HAS
    /// NONE.**
    ///
    /// The year boundary is the case a naive `month - 1` gets wrong, and the
    /// floor is the case an unchecked one gets wrong. There is no `month_after`
    /// on purpose — `CLAUDE.md` §3 rule 7 forbids look-ahead, and a function
    /// that does not exist cannot be called.
    #[test]
    fn the_month_before_january_is_december_and_the_first_month_has_none() {
        assert_eq!(month_before(month(2026, 8)), Some(month(2026, 7)));
        assert_eq!(month_before(month(2026, 1)), Some(month(2025, 12)));
        assert_eq!(
            month_before(month(1970, 1)),
            None,
            "1970-01 is the first month the store can name"
        );
        assert_eq!(month_before(month(1970, 2)), Some(month(1970, 1)));
    }

    /// **AN INSTRUMENT'S FIRST MONTH HAS A CHANGE AND NO PREVIOUS CHANGE.**
    ///
    /// This is the property that decided the reading. Because `Δ %` is the
    /// month's own first close to its own last close, it is defined for every
    /// held month including the first — and only the "previous" column has a
    /// missing-neighbour case. That case is `no_earlier_month`, spelled out,
    /// and it is not `0.00%`.
    #[test]
    fn an_instruments_first_month_has_a_change_and_no_previous_change() {
        let nifty = series(Segment::Index, "NIFTY");
        let first = month(2026, 6);
        let second = month(2026, 7);
        let census = census_of(&[
            (nifty, first, 8_250, priced(10_000, 10_125)),
            (nifty, second, 8_250, priced(10_000, 10_500)),
        ]);
        let body = store_body(
            &census_slice(&census),
            &[(nifty, second), (nifty, first)],
            Vendor::Groww,
        );

        assert!(
            body.contains(r#""chg_bps":500,"chg_why":null,"prev_chg_bps":125,"prev_chg_why":null"#),
            "July renders its own +5.00% and June's +1.25% beside it: {body}"
        );
        assert!(
            body.contains(r#""chg_bps":125,"chg_why":null,"prev_chg_bps":null,"prev_chg_why":"no_earlier_month""#),
            "June has its own number and NO previous — not a zero: {body}"
        );
        assert!(
            !body.contains(r#""prev_chg_bps":0"#),
            "a missing neighbour is never rendered as no change: {body}"
        );
    }

    /// One census as the slice `store_body` takes.
    fn census_slice(census: &census::VendorCensus) -> [census::VendorCensus; 1] {
        [census.clone()]
    }

    /// **A ROW AND ITS NEIGHBOUR COST TWO PROBES AND NO FILE.**
    ///
    /// Two things are asserted, and each is the falsifiable half of a claim the
    /// doc comments make.
    ///
    /// **No file.** The census is built in memory and its `path` names a
    /// directory that does not exist. Every number below therefore came out of
    /// the manifest. A change that reached for a bar file to price a month
    /// would fail here rather than pass slowly, which is the difference between
    /// this design and the 17.5 GB route D-0067 measured and rejected.
    ///
    /// **One month back, never a search.** June and August are held and July is
    /// not. August's previous change is `no_earlier_month` — it does NOT walk
    /// back to June. That is what makes the lookup a probe: a scan would find
    /// June, print a number spanning two months, and label it as one.
    #[test]
    fn a_row_and_its_neighbour_cost_two_probes_and_no_file() {
        let nifty = series(Segment::Index, "NIFTY");
        let june = month(2026, 6);
        let august = month(2026, 8);
        let census = census_of(&[
            (nifty, june, 8_250, priced(10_000, 10_125)),
            (nifty, august, 8_250, priced(10_000, 10_200)),
        ]);
        assert!(
            !census.path.exists(),
            "the fixture must not have a file behind it, or it proves nothing"
        );
        let body = store_body(
            &census_slice(&census),
            &[(nifty, august), (nifty, june)],
            Vendor::Groww,
        );
        assert!(
            body.contains(r#""chg_bps":200,"chg_why":null,"prev_chg_bps":null,"prev_chg_why":"no_earlier_month""#),
            "August reads July, finds nothing, and stops — it does not reach June: {body}"
        );
        assert!(
            !body.contains(r#""chg_bps":200,"chg_why":null,"prev_chg_bps":125"#),
            "reaching past the gap would print a June number on an August row: {body}"
        );
    }

    /// **EVERY ROW CARRIES A NUMBER OR A NAMED REASON, AND NEVER BOTH.**
    ///
    /// The wire shape is total: both keys of each pair are present on every
    /// row, and exactly one of them is non-`null`. A key that is sometimes
    /// absent makes "unknown" and "this build has no such field" the same thing
    /// in the browser, and a row carrying both would be a number with an
    /// excuse — which is the shape `CLAUDE.md` §4 bans.
    #[test]
    fn every_row_carries_a_number_or_a_named_reason_and_never_both() {
        let nifty = series(Segment::Index, "NIFTY");
        let reliance = series(Segment::Cash, "RELIANCE");
        let june = month(2026, 6);
        let july = month(2026, 7);
        let census = census_of(&[
            (nifty, june, 8_250, priced(10_000, 10_125)),
            (nifty, july, 8_250, Closes::UNKNOWN),
            (reliance, june, 8_250, priced(295_050, 310_025)),
            (reliance, july, 8_250, priced(310_025, 62_005)),
        ]);
        let body = store_body(
            &census_slice(&census),
            &[
                (nifty, july),
                (reliance, july),
                (nifty, june),
                (reliance, june),
            ],
            Vendor::Groww,
        );

        // Four rows, four of each key, and no key ever missing.
        for field in [
            "\"chg_bps\":",
            "\"chg_why\":",
            "\"prev_chg_bps\":",
            "\"prev_chg_why\":",
        ] {
            assert_eq!(
                body.matches(field).count(),
                4,
                "{field} appears on every row exactly once: {body}"
            );
        }
        // A number and a reason never co-occur: every `_bps` that is a number
        // is followed by a `_why` of null, and vice versa.
        assert_eq!(
            body.matches("_bps\":null").count() + body.matches("_why\":null").count(),
            8,
            "one of each pair is null on every row, and only one: {body}"
        );

        assert!(
            body.contains(
                r#""chg_bps":null,"chg_why":"not_recorded","prev_chg_bps":125,"prev_chg_why":null"#
            ),
            "an unpriced index month is unknown and its priced neighbour is not: {body}"
        );
        // THE 80% FALL D-0018 IS ABOUT. ₹3,100.25 to ₹620.05 is a 1:5 split
        // read as an -8,000 bp crash, and it is exactly the number this page
        // must never print.
        assert!(
            body.contains(r#""chg_bps":null,"chg_why":"corporate_action_unverified","prev_chg_bps":null,"prev_chg_why":"corporate_action_unverified""#),
            "the equity is refused in both columns, by name: {body}"
        );
        assert!(
            !body.contains("-8000"),
            "the fabricated crash never reaches the wire: {body}"
        );
    }

    /// **A FEED WITH NO CENSUS IS AN EMPTY ARRAY, NOT A ROW OF ZEROES.**
    ///
    /// The page renders "nothing stored for this feed" off an empty array and
    /// names where the rows actually are. It has no way to distinguish a zeroed
    /// row from a real one, so there must never be a zeroed row.
    #[test]
    fn a_feed_with_no_census_is_an_empty_array() {
        let nifty = series(Segment::Index, "NIFTY");
        let at = month(2026, 7);
        let census = census_of(&[(nifty, at, 8_250, priced(10_000, 10_125))]);
        assert_eq!(
            store_body(&census_slice(&census), &[(nifty, at)], Vendor::Dhan),
            "[]",
            "a feed this store has never held answers with nothing"
        );
        assert_eq!(
            store_body(&[], &[(nifty, at)], Vendor::Groww),
            "[]",
            "and so does a store with no censuses at all"
        );
        // A MONTH THE CENSUS DOES NOT HOLD IS SKIPPED, not emitted empty.
        assert_eq!(
            store_body(
                &census_slice(&census),
                &[(nifty, month(2026, 5))],
                Vendor::Groww
            ),
            "[]"
        );
    }
}

// ===================================================================== ======
// THE UNIVERSE RESOLUTION — CRAWLED ON A PRESS, NEVER ON A CLOCK
// ============================================================================

/// The exchange's index host.
///
/// A `const` here rather than in `pull::nse` because it is a HOST and
/// `pull::nse` holds paths. The four category paths that hang off it are that
/// module's, read from the exchange's own navigation and never composed.
const INDEX_HOST: &str = "https://www.niftyindices.com";

/// `POST /universe/resolve` — crawl the exchange's directory and report what
/// agreed with a feed's master.
///
/// # POST, and there is no GET
///
/// D-0128 made the autopilot default PAUSED because starting the server pulled
/// data nobody asked for. The same rule binds here and binds harder: this route
/// opens **300 sockets** to a third party — one per category, one per index
/// page, one per constituent file, for the 148 indices §4d counted. A `GET`
/// would be fetched by a link preview, a browser prefetch, or a health check,
/// and none of those is a person deciding to crawl an exchange.
///
/// # What it does NOT do
///
/// It reads no vendor master and it contacts no broker. The join runs against
/// whatever master the caller supplies, and today that is **the merged universe
/// already in memory** — the same rows `/instruments.json` serves, which were
/// read from files on disk rather than fetched here. Fetching a broker's master
/// is a credentialed request against a shared token's quota, and it belongs to
/// the same operator decision that starts a pull.
///
/// So this answers one question and answers it honestly: *of the names the
/// exchange publishes, which can this feed name?* — with the count, the bucket,
/// and the key the join used.
///
/// # It writes nothing
///
/// The snapshot is returned, not stored. `Snapshot::admits_publication` is
/// evaluated and its verdict is reported, so an operator can see whether this
/// pass WOULD be publishable — but the append-only write is a separate act and
/// is not taken here.
pub async fn universe_resolve(
    axum::extract::State(site): axum::extract::State<Loaded>,
    body: String,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    let json = [(axum::http::header::CONTENT_TYPE, "application/json")];

    let feed = param(&body, "feed");
    let feed = if feed.is_empty() {
        brutex_core::vendor::Vendor::Groww.as_str().to_owned()
    } else {
        feed
    };
    let Some(vendor) = brutex_core::vendor::Vendor::ALL
        .into_iter()
        .find(|v| v.as_str() == feed)
    else {
        return (
            json,
            format!(
                r#"{{"ok":false,"why":{}}}"#,
                render::json_string(&format!(
                    "{feed:?} is not a vendor this build names. It is one of: {}",
                    brutex_core::vendor::Vendor::ALL
                        .map(brutex_core::vendor::Vendor::as_str)
                        .join(", ")
                ))
            ),
        );
    };

    // THE MASTER IS THE ONE ALREADY READ, not one fetched here. See this
    // function's own documentation on why a broker request is a separate act.
    let master: Vec<pull::universe::VendorInstrument<'_>> = site
        .read
        .merged
        .by_key
        .iter()
        .filter_map(|(key, entry)| {
            entry
                .ids
                .get(vendor as usize)
                .and_then(Option::as_ref)
                .map(|id| pull::universe::VendorInstrument {
                    vendor_id: id.as_str(),
                    trading_symbol: key.underlying.as_str(),
                    // THE ISIN IS OPTIONAL AND ITS ABSENCE IS A REAL STATE, not
                    // a gap to paper over: an index has none, and
                    // `Verdict::VendorHasNoIsin` is the bucket that exists to
                    // say so without blaming the vendor.
                    isin: entry.isin.as_ref().map_or("", |(_, isin)| isin.as_str()),
                })
        })
        .collect();

    let source = match pull::resolve::HttpDocuments::new() {
        Ok(client) => client,
        Err(why) => {
            return (
                json,
                format!(r#"{{"ok":false,"why":{}}}"#, render::json_string(&why)),
            );
        }
    };

    // THE DAY THE PASS IS STAMPED WITH. One value for the whole crawl — see
    // `pull::resolve`'s header on why one age matters more than an instant.
    let today = match crate::ingest::today_ist() {
        Ok(day) => day,
        Err(why) => {
            return (
                json,
                format!(
                    r#"{{"ok":false,"why":{}}}"#,
                    render::json_string(&why.to_string())
                ),
            );
        }
    };

    let snap = pull::resolve::crawl(
        &source,
        INDEX_HOST,
        today,
        &feed,
        // EVERY FEED IN THIS BUILD PUBLISHES AN ISIN. The symbol key exists for
        // the one that does not, and asking the descriptor rather than assuming
        // is what keeps that true when it lands.
        pull::universe::JoinKey::Isin,
        &master,
    )
    .await;

    let verdict = snap.admits_publication(None);
    (json, universe_snapshot_json(&snap, &verdict))
}

/// One snapshot as JSON, with the publication verdict beside it.
///
/// Counts per bucket rather than the rows themselves: a full row list across
/// 148 indices is the shape D-0130 measured growing to 50 kB, and this route
/// answers "what agreed", not "name every one".
fn universe_snapshot_json(snap: &pull::resolve::Snapshot, verdict: &Result<(), String>) -> String {
    use core::fmt::Write as _;
    let mut out = format!(
        r#"{{"ok":true,"day":{},"feed":{},"key":{},"identity":{},"published":{},"indices":{},"failed":{},"sound":{},"digest":{},"publishable":{},"#,
        render::json_string(&snap.day.to_string()),
        render::json_string(&snap.feed),
        render::json_string(snap.key.word()),
        snap.key.is_identity(),
        snap.published(),
        snap.resolutions.len(),
        snap.failures.len(),
        snap.is_sound(),
        render::json_string(&hex32(snap.digest())),
        verdict.is_ok(),
    );
    if let Err(why) = verdict {
        let _ = write!(out, r#""why":{},"#, render::json_string(why));
    }
    let _ = out.write_str(r#""buckets":{"#);
    for (n, bucket) in pull::universe::Verdict::ALL.into_iter().enumerate() {
        let total: usize = snap.resolutions.iter().map(|r| r.count(bucket)).sum();
        let _ = write!(
            out,
            "{}{}:{total}",
            if n > 0 { "," } else { "" },
            render::json_string(bucket.word())
        );
    }
    let _ = out.write_str("},\"failures\":[");
    // BOUNDED. A pass that failed everywhere would otherwise put 148 sentences
    // on one line, and the first few name the fault as well as all of them do.
    for (n, failure) in snap.failures.iter().take(8).enumerate() {
        let _ = write!(
            out,
            r#"{}{{"at":{},"why":{}}}"#,
            if n > 0 { "," } else { "" },
            render::json_string(&failure.at),
            render::json_string(&failure.why)
        );
    }
    let _ = out.write_str("]}");
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod universe_route_tests {
    use super::*;

    /// **THE ROUTE IS POST AND THERE IS NO GET, and this reads it off the
    /// source rather than trusting the comment beside it.**
    ///
    /// D-0128 made the autopilot default PAUSED because starting the server
    /// pulled data nobody asked for. This route opens roughly 300 sockets to a
    /// third party, and a `GET` is fetched by link previews, browser prefetch
    /// and health checks — none of which is a person deciding to crawl an
    /// exchange.
    #[test]
    fn the_resolve_route_can_only_be_reached_by_a_post() {
        let me = include_str!("server.rs");
        // THE NEEDLES ARE ASSEMBLED, NOT WRITTEN, and the first draft of this
        // test failed because of it: a source-scanning test that spells its own
        // negative assertion as a literal FINDS THAT LITERAL — in itself. It
        // then fails whatever the route table says, which is a test that cannot
        // pass rather than one that cannot fail. `server.rs` already carries a
        // commented-out ancestor of this exact trap.
        let verb = |v: &str| format!("axum::routing::{v}(universe_resolve)");
        assert!(
            me.contains(&verb("post")),
            "the route must be registered as a POST"
        );
        assert!(
            !me.contains(&verb("get")),
            "a GET on this route would let a link preview crawl an exchange"
        );
    }

    /// **IT CONTACTS NO BROKER**, and the argument is that its master comes
    /// from memory rather than from a socket.
    ///
    /// Fetching a vendor's instrument master is a credentialed request against
    /// a token another system shares. This route answers "of the names the
    /// exchange publishes, which can this feed name?" off the rows already
    /// read from disk — so it spends no quota and needs no credential.
    #[test]
    fn the_resolve_route_reads_its_master_from_memory_and_never_from_a_vendor() {
        let me = include_str!("server.rs");
        let body = me
            .split_once("pub async fn universe_resolve")
            .expect("the handler exists")
            .1;
        let body = &body[..body.find("\n}\n").expect("it has an end")];

        assert!(
            body.contains("site.read.merged.by_key") || body.contains(".read\n        .merged"),
            "the master is the one already in memory"
        );
        for forbidden in [
            "HttpSource",
            "ssm::get_parameter",
            "AwsIdentity",
            "CredentialConfig",
            "read_credential",
        ] {
            assert!(
                !body.contains(forbidden),
                "{forbidden} is a broker-facing cost and this route pays none"
            );
        }
    }

    /// The host is the exchange's, and the PATHS are `pull::nse`'s — never
    /// built here.
    #[test]
    fn the_host_is_the_exchange_and_the_paths_belong_to_the_crawler() {
        assert!(INDEX_HOST.starts_with("https://"));
        let me = include_str!("server.rs");
        let body = me
            .split_once("pub async fn universe_resolve")
            .expect("the handler exists")
            .1;
        let body = &body[..body.find("\n}\n").expect("it has an end")];
        assert!(
            !body.contains("IndexConstituent"),
            "a constituent filename is READ from the exchange's page, never \
             composed — pull::nse owns that and this handler must not learn it"
        );
    }

    #[test]
    fn thirty_two_bytes_render_as_sixty_four_hex_digits() {
        let all_zero = hex32([0u8; 32]);
        assert_eq!(all_zero.len(), 64);
        assert!(all_zero.chars().all(|c| c == '0'));
        let mut bytes = [0u8; 32];
        bytes[0] = 0xab;
        bytes[31] = 0x0f;
        let shown = hex32(bytes);
        assert!(shown.starts_with("ab"), "{shown}");
        assert!(
            shown.ends_with("0f"),
            "a leading zero is not dropped: {shown}"
        );
    }

    /// A snapshot with nothing in it still renders every bucket and says it is
    /// not publishable.
    #[test]
    fn an_empty_pass_renders_every_bucket_and_reports_why_it_cannot_publish() {
        let snap = pull::resolve::Snapshot {
            day: crate::ingest::today_ist().expect("a day"),
            feed: brutex_core::vendor::Vendor::Groww.as_str().to_owned(),
            key: pull::universe::JoinKey::Isin,
            resolutions: Vec::new(),
            failures: vec![pull::resolve::IndexFailure {
                at: "a-listing".to_owned(),
                why: "503".to_owned(),
            }],
        };
        let verdict = snap.admits_publication(None);
        assert!(verdict.is_err(), "a pass with a failure is not publishable");
        let json = universe_snapshot_json(&snap, &verdict);
        for bucket in pull::universe::Verdict::ALL {
            assert!(
                json.contains(bucket.word()),
                "{} is missing from {json}",
                bucket.word()
            );
        }
        assert!(json.contains(r#""publishable":false"#), "{json}");
        assert!(json.contains("a-listing"), "it names what failed: {json}");
        assert!(json.contains(r#""identity":true"#), "{json}");
    }
}

/// 32 bytes as lower-case hex.
fn hex32(bytes: [u8; 32]) -> String {
    use core::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(64), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

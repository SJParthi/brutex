//! Every `telemetry::emit` in this crate, driven through the function that
//! ships and read back off the disk.
//!
//! # What was unproven, and it was all of it
//!
//! `crates/pull` holds **42** `telemetry::emit` call sites — the largest
//! concentration in the workspace — and **not one of them had a test that drove
//! the production helper and then found the record in a file.**
//!
//! **THE COUNT IS MEASURED AND THE TABLE DOES NOT COVER ALL OF IT.** This header
//! said 21 while the crate held 36, and `SITES` holds 27 rows, so the rest
//! are driven by nothing here. Nothing pins either number -- there is no gate
//! comparing the table to the crate -- so re-measure rather than trusting this
//! sentence: `grep -c "telemetry::emit(" crates/pull/src/*.rs`. The gap is
//! stated because a registry that looks exhaustive and is not is worse than one
//! that says where it stops. The events that
//! looked covered were not: the tests that assert on a `pull.member` line live
//! in `crates/api/src/logs.rs` and build their own [`telemetry::Event`] on a
//! locally-opened sink. They prove the *sink* writes. They never call
//! [`crate::ingest`]'s `note_landed` or `note_not_landed`, so a helper that
//! silently stopped emitting — a wrong level, a `return` above the `emit`, a
//! target renamed on one side of a filter — would have left every one of those
//! tests green.
//!
//! That is the defect this file catches, and it is the defect worth catching:
//! `pull.member` at `Error` is the line an operator reads when a backfill
//! half-fails, and until this test nothing said it reached a file.
//!
//! # Why one test, and why here
//!
//! [`telemetry::emit`] reads a process-wide `OnceLock`, and
//! [`telemetry::install`] **refuses a second call**, naming the path already
//! installed. So there is exactly one install per test binary, which makes the
//! unit of work one table-driven test rather than twenty scattered ones. It
//! lives in `src/` rather than `tests/` because several of these sites sit in
//! private helpers an integration binary cannot name.
//!
//! The installed floor is [`telemetry::Level::Trace`], and that is not a
//! detail: seven of these targets emit below `Info` — `pull.member`,
//! `pull.fold`, `pull.split`, `pull.land`, `pull.csv`, `pull.manifest` and
//! `pull.http` on its answering path — so at the default floor they would be
//! filtered and this file would assert nothing while appearing to pass.
//!
//! # Nothing here reaches a network or Parameter Store
//!
//! `CLAUDE.md` §8: the credential value is read from Parameter Store and
//! nowhere else, and this repository never mints a token. So the credential
//! sites are driven through [`crate::secret::CredentialReader`] over an
//! in-memory [`crate::secret::SecretSource`] double, and the vendor site is
//! driven against a loopback socket this test opens itself. The point of each
//! row is that **the helper writes a line**, not that a vendor answered.
//!
//! # The one site this file does not reach
//!
//! `crates/pull/src/ssm.rs:678` — `pull.ssm`, "credential read". It is not a
//! `note_*` helper: the `emit` is written inline in the body of the `async`
//! function that POSTs to `https://<host>/`, after `send().await` and after the
//! status check. There is no seam to drive and no host to substitute, so
//! reaching it requires a live HTTPS call to AWS with a real signature. Named
//! here rather than left for a coverage report to find, per `CLAUDE.md` §3
//! rule 6. Extracting it into a `note_credential_read` helper — the shape every
//! other module in this crate already uses — is what would make it provable.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use brutex_core::vendor::Vendor;
use telemetry::{Config, Level, MAX_LIMIT, OwnedValue, Query, Record, Sink};

use crate::config::{CredentialConfig, CredentialPath};
use crate::csv::Columns;
use crate::fetch::{BarRequest, RawRow, RawWindow};
use crate::ingest::{Ingested, Plan};
use crate::manifest::Manifest;
use crate::secret::{CredentialReader, Secret, SecretError, SecretSource};
use crate::session::{Day, Window};
use crate::vendor::{Granularity, Listing, PriceScale, TimestampEncoding};

// ===========================================================================
// Scratch
// ===========================================================================

/// Distinguishes two scratch trees taken in the same process.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A temporary directory tree that removes itself.
///
/// One per table row, so no row can pass because of a file a previous row
/// left behind.
struct Scratch {
    /// The tree's root, beneath the system temporary directory.
    root: PathBuf,
}

impl Scratch {
    /// A fresh tree.
    fn new() -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut root = std::env::temp_dir();
        root.push(format!("brutex-pull-emit-{}-{serial}", std::process::id()));
        fs::create_dir_all(&root).expect("a scratch root");
        Self { root }
    }

    /// Where bars and the census go.
    fn store(&self) -> PathBuf {
        let dir = self.root.join("STORE");
        fs::create_dir_all(&dir).expect("a scratch store");
        dir
    }

    /// A folder holding one member per `(name, body)` pair.
    fn archive(&self, members: &[(&str, &str)]) -> PathBuf {
        let dir = self.root.join("ARCHIVE");
        fs::create_dir_all(&dir).expect("a scratch archive");
        for (name, body) in members {
            fs::write(dir.join(format!("{name}.csv")), body).expect("a member");
        }
        dir
    }
}

impl Drop for Scratch {
    /// Best effort: a leaked scratch directory must never fail a test run.
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.root));
    }
}

// ===========================================================================
// Fixtures — every segment in this file is invented, CLAUDE.md §8
// ===========================================================================

/// The instrument every archive row below is filed under.
///
/// An index this repository has tracked since its first commit, and not a
/// parameter path segment.
const INSTRUMENT: &str = "NIFTY";

/// Three rows of `TrueData`'s five-field index layout, in one month.
///
/// Two share a minute, so the fold consumes exactly one of them — which is
/// what makes `folded = 1` a number this test can demand rather than observe.
const BODY: &str = "\
20221003,10:00:00,38445.65,0,0
20221003,10:00:01,38446.00,0,0
20221004,10:00:00,38600.00,0,0
";

/// Two rows either side of a month boundary.
///
/// Both are inside the window and inside the session, so both become bars —
/// and a member whose bars need two files is the refusal `crate::ingest`
/// reports at `Error`.
const SPANNING: &str = "\
20221003,10:00:00,38445.65,0,0
20221101,10:00:00,38600.00,0,0
";

/// One row in the month AFTER [`BODY`]'s, so a second run has an entry to
/// append.
///
/// `install_census` returns early when there is nothing to publish and no
/// repair to make, so a re-run of the same month reaches no write at all — and
/// a fixture that re-ran `BODY` would prove the census never refuses because it
/// was never asked.
const NEXT_MONTH: &str = "\
20221101,10:00:00,38600.00,0,0
";

/// A credential configuration whose four segments are nonsense by
/// construction.
///
/// The same invented words `crate::config`'s own doc example uses: no
/// operator's parameter path reads `orgone/testenv/vendorone`. `ap-south-1` is
/// the one real word, and `CLAUDE.md` §8 states it in the clear.
const CONFIG: &str = "\
org    = \"orgone\"
env    = \"testenv\"
region = \"ap-south-1\"

[vendor.groww]
vendor = \"vendorone\"
fields = [\"fieldone\", \"fieldtwo\"]

[vendor.dhan]
vendor = \"vendortwo\"
fields = [\"fieldone\"]
";

/// The refusal the loopback vendor answers with.
///
/// A status, and nothing else. `crate::http` writes its line from the status
/// before it decides what the status means, so a body would prove nothing that
/// this does not.
const REFUSAL: &str =
    "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// The two days [`BODY`] lands in.
fn window() -> Window {
    Window::new(
        Day::new(2022, 10, 3).expect("a real day"),
        Day::new(2022, 10, 4).expect("a real day"),
    )
    .expect("a forward window")
}

/// A window wide enough to hold both of [`SPANNING`]'s rows.
/// A window holding [`NEXT_MONTH`]'s single row and nothing of [`BODY`]'s.
fn november() -> Window {
    Window::new(
        Day::new(2022, 11, 1).expect("a real day"),
        Day::new(2022, 11, 3).expect("a real day"),
    )
    .expect("a forward window")
}

fn crossing() -> Window {
    Window::new(
        Day::new(2022, 10, 3).expect("a real day"),
        Day::new(2022, 11, 3).expect("a real day"),
    )
    .expect("a forward window")
}

/// One request over `at`.
fn request_over(at: Window) -> BarRequest {
    BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window: at,
        granularity: Granularity::Minute1,
    }
}

/// The plan every ingest below runs under.
fn plan_over(request: &BarRequest) -> Plan<'_> {
    Plan {
        columns: Columns::TrueDataIndex,
        request,
        encoding: TimestampEncoding::EpochSecondsUtc,
        scale: PriceScale::Paisa,
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
    }
}

/// One whole ingest of [`BODY`], from folder to bar file to census.
///
/// The production entry point, so the archive walk, the CSV decode, the land,
/// the fold, the per-member line and the census are all the shipped ones.
fn ingest_body(scratch: &Scratch) -> Ingested {
    let archive = scratch.archive(&[(INSTRUMENT, BODY)]);
    let store = scratch.store();
    let request = request_over(window());
    crate::ingest::from_dir(&archive, &store, plan_over(&request))
        .expect("the folder is readable and the column shape is right")
}

/// A credential source that answers from memory and reaches nothing.
///
/// `CLAUDE.md` §8 puts the value in Parameter Store and nowhere else; this
/// double exists so the two `note_*` helpers beside it can be driven without a
/// path, an account or a socket.
struct Fixed {
    /// What every read returns.
    answer: Result<&'static str, SecretError>,
}

impl SecretSource for Fixed {
    fn read(&self, _path: &CredentialPath<'_>) -> Result<Secret, SecretError> {
        self.answer.and_then(|value| Secret::new(value.to_owned()))
    }
}

/// The invented configuration, parsed.
fn credentials() -> CredentialConfig {
    CredentialConfig::parse(CONFIG).expect("the invented fixture parses")
}

/// A server on loopback that answers once and reports that it was reached.
///
/// Raw sockets and hand-written HTTP, for the reason `crate::http`'s own
/// socket tests give: this crate takes `tokio` without the `net` feature and a
/// test is not a reason to widen a dependency.
fn loopback(answer: &'static str) -> (String, std::sync::mpsc::Receiver<()>) {
    use std::io::{Read as _, Write as _};

    let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let addr = socket.local_addr().expect("an address");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = socket.accept() else {
            return;
        };
        let mut buf = [0u8; 4096];
        drop(stream.read(&mut buf));
        let _reached = tx.send(());
        drop(stream.write_all(answer.as_bytes()));
        drop(stream.flush());
    });
    (format!("http://{addr}"), rx)
}

// ===========================================================================
// The table
// ===========================================================================

/// What one row demands of the record its driver produced.
///
/// One field, chosen so the record cannot be some other test's: either a
/// string this test supplied or a count this test fixed.
#[derive(Debug, Clone, Copy)]
enum Says {
    /// The field is text and holds this, whole or as a substring.
    Holds(&'static str),
    /// The field is an integer and is exactly this.
    ///
    /// Signed, because several of these sites render "this vendor bounds
    /// nothing here" as `-1` precisely so it cannot be read as a zero
    /// allowance — and that distinction is worth asserting rather than
    /// stepping around.
    Signs(i64),
    /// The field is an integer and is **strictly positive**.
    ///
    /// For a count whose exact value is a consequence of a RULE rather than a
    /// constant. `census image built` says how many entries the image holds,
    /// and a member now writes one file per rung derived from the one it
    /// pulled — a set `ingest::derived_from` computes from `Timeframe::KNOWN`.
    /// Pinning that to a literal would make a rung added to the store fail this
    /// site for having done exactly what it was added to do, while pinning it to
    /// zero-or-more would let a census that imaged nothing pass. Positive is the
    /// claim that survives both.
    Positive,
    /// The field is a flag and is exactly this.
    Flags(bool),
}

impl Says {
    /// Whether the value the record carried is the one demanded.
    fn satisfied_by(self, value: &OwnedValue) -> bool {
        match self {
            Self::Holds(want) => value.as_str().is_some_and(|got| got.contains(want)),
            Self::Signs(want) => value.as_i64() == Some(want),
            Self::Positive => value.as_i64().is_some_and(|got| got > 0),
            Self::Flags(want) => value.as_bool() == Some(want),
        }
    }
}

/// One production `telemetry::emit`, and the production call that reaches it.
struct Site {
    /// The `telemetry::emit(` this row proves, as `file:line`.
    at: &'static str,
    /// The subsystem the event names.
    target: &'static str,
    /// The message it carries.
    message: &'static str,
    /// A field key, and what the record must say under it.
    says: (&'static str, Says),
    /// The call that must produce it.
    ///
    /// **A shipped function every time.** Not one row builds a
    /// [`telemetry::Event`]: an event this test wrote would prove the sink
    /// works, which is `crates/telemetry`'s business and was never in doubt.
    drive: fn(&Scratch),
}

/// Every `telemetry::emit` under `crates/pull/src` except the one the module
/// header names, one row each.
static SITES: &[Site] = &[
    Site {
        // THE CORRECTION D-0332 MAKES, PROVEN TO REACH A FILE.
        //
        // Without this row `cargo mutants` replaces `note_volumes_corrected`
        // with `()` and the whole suite stays green — measured, 1 missed of 2 —
        // so gate 18's `--in-diff` run fails on the very commit that adds it.
        // That is exactly the defect this module's own header exists to catch:
        // a helper that silently stopped emitting would leave every other test
        // passing.
        //
        // Asserting `bars` rather than `corrected` on purpose: the denominator
        // is the field that was MISSING from the first draft of the event, and
        // a count with nothing to divide it by cannot separate "a few noisy
        // rows" from "the decoder is reading the wrong column".
        at: "crates/pull/src/http.rs — note_volumes_corrected",
        target: "pull.decode",
        message: "an index carried a negative volume and it was recorded as zero",
        says: ("bars", Says::Signs(1)),
        drive: drive_index_volume_corrected,
    },
    Site {
        at: "crates/pull/src/http.rs:1416",
        target: "pull.decode",
        message: "bars carried a negative volume and were skipped",
        // TWO SENT, ONE KEPT. The denominator is the whole window, so a reader
        // can tell "a vendor sent noise in one minute" from "the decoder is
        // reading the wrong column entirely" — the two want opposite responses.
        says: ("bars", Says::Signs(2)),
        drive: drive_negative_volume_skipped,
    },
    Site {
        at: "crates/pull/src/http.rs:1447",
        target: "pull.decode",
        message: "bars carried an impossible OHLC and were skipped",
        says: ("bars", Says::Signs(2)),
        drive: drive_impossible_bar_skipped,
    },
    Site {
        // THE ONE EVENT PER RUN, AND THE FIELD ASSERTED IS ITS NAME.
        //
        // Measured on a real store: 884 of these in the rolling log, one per
        // instrument-month, each carrying `members / rows / bars / committed /
        // folded / dropped / failures` — and **none of them naming what was
        // ingested**. So 884 records said something came up short and not one
        // could answer WHICH month did, which is the whole reason a log is
        // searchable rather than merely written.
        //
        // Asserting `instrument` rather than `bars` on purpose: the counts were
        // there from the first draft and the name was not. A count with no
        // subject is the failure this row exists to refuse.
        at: "crates/pull/src/ingest.rs — note_run",
        target: "pull.run",
        message: "ingested",
        says: ("instrument", Says::Holds(INSTRUMENT)),
        drive: drive_run_named,
    },
    Site {
        // THE NUMBER THAT WOULD HAVE NAMED D-0070 ON THE DAY IT LANDED.
        //
        // `folded` is how many rows the bucket consumed. A daily pull folding N
        // rows into N bars means the vendor already sent one bar per day and
        // the bucket is only ever re-stamping them, so **`folded = 0` on a
        // `1day` rung is the signature of that whole defect** — and nothing
        // wrote it down. It took decoding bar files by hand to see it.
        //
        // Asserting `folded` rather than `bars`, because `bars` is the number
        // that was never in doubt. `BODY` holds two rows sharing one minute, so
        // the fold consumes exactly one of them and this is a value the fixture
        // can DEMAND rather than observe.
        //
        // `Debug`, and that is why the module installs a `Trace` floor: at the
        // default floor this event is filtered and this row would assert
        // nothing while appearing to pass.
        at: "crates/pull/src/ingest.rs — note_fold",
        target: "pull.fold",
        message: "folded",
        says: ("folded", Says::Signs(1)),
        drive: drive_run_named,
    },
    Site {
        // AN `Error` ON THE STORING PATH, AND IT WAS THE LEAST-PROVEN KIND.
        //
        // Measured across `crates/pull/src/ingest.rs`: ten `note_*` helpers,
        // and before this row **three** were driven here. The seven that were
        // not are the FAILURE paths — four of them at `Error`, which is the
        // level an operator is woken by. A helper that silently stopped
        // emitting would leave every other test in this crate green, which is
        // the exact defect this module's header exists to catch, and the events
        // it was catching it for were the successes.
        //
        // `stage` rather than `instrument`, because `stage` is what makes this
        // line actionable: `address` and `bars` are two different faults
        // wearing one message, and an operator reading "not filed" without it
        // cannot tell a name the store refuses from a write that failed.
        at: "crates/pull/src/ingest.rs — note_not_filed",
        target: "pull.file",
        message: "not filed",
        says: ("stage", Says::Holds("address")),
        drive: drive_not_filed,
    },
    Site {
        // A CENSUS THAT LOADED FROM THE OTHER GENERATION, WHICH IS THE QUIETEST
        // WAY THIS STORE CAN BE DAMAGED.
        //
        // The census is double-buffered. One good header slot beside one that
        // will not validate still LOADS — from the generation that survived —
        // so every count is right, every bar is filed, and the run succeeds. The
        // only signal that a generation was stepped over is this line.
        //
        // Without it an operator learns at the NEXT write, against a file that
        // has been half-readable for however many runs happened in between.
        //
        // Asserting `census` rather than the reason: the reason is a
        // `ManifestError`'s own words and would pin this row to that enum's
        // phrasing, while WHICH FILE is what an operator has to go and look at.
        at: "crates/pull/src/ingest.rs — note_census_degraded",
        target: "pull.census",
        message: "loaded degraded",
        says: ("census", Says::Holds("groww")),
        drive: drive_census_degraded,
    },
    Site {
        // BARS ON DISK AND NOTHING COUNTING THEM — the worst state on the
        // storing path, and the one with no way back.
        //
        // The slices land first and the census is published after, so a census
        // that refuses the write leaves bars that are filed, correct, and
        // invisible. They cannot be un-written: the store is append-only and
        // `Header::advance` refuses a batch beginning at or before what is
        // committed. This line is the only record that the two ever disagreed.
        //
        // `slices` is the asserted field because it is the SIZE of the
        // disagreement — how many entries went uncounted. "The census did not
        // publish" without a number cannot tell one lost slice from a whole
        // run's worth.
        //
        // `Positive` rather than a literal, and the measurement is why: one
        // member answered **8**, because a one-minute member writes the rung it
        // pulled plus the seven `ingest::derived_from` computes from
        // `Timeframe::KNOWN`. Pinning that to 8 would make a rung added to the
        // store fail this row for having done exactly what it was added to do,
        // while zero-or-more would let a run that lost NOTHING pass a site
        // about loss.
        at: "crates/pull/src/ingest.rs — note_census_unpublished",
        target: "pull.census",
        message: "not published",
        says: ("slices", Says::Positive),
        drive: drive_census_unpublished,
    },
    Site {
        // HELD AT ONE TIMEFRAME AND MISSING AT THE REST, which is the shape a
        // count alone cannot show.
        //
        // A one-minute member writes the rung it pulled plus seven derived from
        // it. When one of the seven cannot be written the PULLED rung still
        // lands, so the run stores bars, the census counts them, and the
        // instrument-month looks held — until a reader asks for two-minute bars
        // and finds nothing beside a full one-minute file.
        //
        // Asserting `timeframe`, because WHICH rung is the whole content of
        // this line: "a rung was not derived" without it cannot be acted on.
        at: "crates/pull/src/ingest.rs — note_not_derived",
        target: "pull.derive",
        message: "rung not derived",
        says: ("timeframe", Says::Holds("2min")),
        drive: drive_derived_shortfall,
    },
    Site {
        // THE COUNT THAT STOPS THE RUN REPORTING ITSELF CLEAN.
        //
        // `pull.derive` above says which rung failed, one line per rung. This
        // is the roll-up that reaches `Ingested::failures`, and without it a
        // run that lost six of seven derived rungs still ends `Stored` — every
        // individual refusal on the log and nothing raising the total.
        at: "crates/pull/src/ingest.rs — note_derived_shortfall",
        target: "pull.derived",
        message: "a folded rung did not land",
        says: ("instrument", Says::Holds(INSTRUMENT)),
        drive: drive_derived_shortfall,
    },
    Site {
        // BARS WRITTEN THAT THE CENSUS WILL NOT COUNT — the tenth and last
        // `note_*` helper on the storing path, and the one that closes this
        // table's coverage of it.
        //
        // The census refuses when a count goes DOWN, because a smaller number
        // is either a truncated file or a stale write arriving late and
        // accepting it would make the census forget bars still on disk. That
        // refusal is correct, and it leaves the bars written and uncounted —
        // append-only, so they cannot be withdrawn.
        //
        // `bars` is the asserted field, and `Positive` rather than a literal:
        // it is the entry's own row count, a consequence of what the fixture
        // landed rather than a constant, and zero-or-more would let an event
        // about uncounted bars pass with no bars.
        at: "crates/pull/src/ingest.rs — note_bars_not_counted",
        target: "pull.census",
        message: "bars not counted",
        says: ("bars", Says::Positive),
        drive: drive_bars_not_counted,
    },
    Site {
        at: "crates/pull/src/http.rs:1483",
        target: "pull.decode",
        message: "bars carried a negative open interest and were skipped",
        says: ("bars", Says::Signs(2)),
        drive: drive_negative_interest_skipped,
    },
    Site {
        at: "crates/pull/src/rate.rs:663",
        target: "pull.rate",
        message: "throttled — every span backed off",
        // -1, NEVER 0. A span this vendor publishes no bound for is a
        // recorded fact; a zero there would read as "this vendor allows
        // nothing" and send an operator to the wrong page.
        says: ("per_day", Says::Signs(-1)),
        drive: drive_rate,
    },
    Site {
        at: "crates/pull/src/config.rs:271 (Ok arm)",
        target: "pull.config",
        message: "loaded",
        says: ("file", Says::Holds("CREDENTIALS")),
        drive: drive_config_loaded,
    },
    Site {
        at: "crates/pull/src/config.rs:271 (Err arm)",
        target: "pull.config",
        message: "refused",
        says: ("vendors", Says::Signs(0)),
        drive: drive_config_refused,
    },
    Site {
        at: "crates/pull/src/session.rs:1136",
        target: "pull.split",
        message: "window split",
        says: ("chunks", Says::Signs(3)),
        drive: drive_split,
    },
    Site {
        at: "crates/pull/src/totp.rs:172",
        target: "pull.totp",
        message: "the shared secret will not decode",
        says: ("fault", Says::Holds("too-long")),
        drive: drive_totp,
    },
    Site {
        at: "crates/pull/src/work.rs:193",
        target: "pull.work",
        message: "selection narrowed before it ran",
        says: ("asked", Says::Signs(4)),
        drive: drive_work,
    },
    Site {
        at: "crates/pull/src/manifest.rs:2906",
        target: "pull.manifest",
        message: "census absent",
        says: ("outcome", Says::Holds("absent")),
        drive: drive_census_absent,
    },
    Site {
        at: "crates/pull/src/manifest.rs:2991",
        target: "pull.manifest",
        message: "census load",
        // -1 across every count, because a census that could not be read is
        // not a census that counts nothing.
        says: ("entries", Says::Signs(-1)),
        drive: drive_census_refused,
    },
    Site {
        at: "crates/pull/src/manifest.rs:3036",
        target: "pull.manifest",
        message: "census image built",
        says: ("entries", Says::Positive),
        drive: drive_census_imaged,
    },
    Site {
        at: "crates/pull/src/csv.rs:369",
        target: "pull.csv",
        message: "file decoded",
        says: ("rows_in", Says::Signs(3)),
        drive: drive_csv_decoded,
    },
    Site {
        at: "crates/pull/src/csv.rs:397",
        target: "pull.csv",
        message: "file refused",
        says: ("fields", Says::Signs(5)),
        drive: drive_csv_refused,
    },
    Site {
        at: "crates/pull/src/archive.rs:278",
        target: "pull.archive",
        message: "folder walked",
        says: ("dir", Says::Holds("ARCHIVE")),
        drive: drive_archive_walked,
    },
    Site {
        at: "crates/pull/src/archive.rs:301",
        target: "pull.archive",
        message: "folder refused",
        // Nothing had been accepted when the walk stopped, and the absence is
        // the fact: "the first file of twelve thousand is corrupt" and "the
        // eleventh is" are the same error with different numbers here.
        says: ("members", Says::Signs(0)),
        drive: drive_archive_refused,
    },
    Site {
        at: "crates/pull/src/folder.rs — note_read",
        target: "pull.folder",
        message: "folder reach read",
        // THE VERB, which is the whole of the honest-verb rule: a log read
        // after the fact is where the wrong diagnostic frame does its damage,
        // and `read` beside the path is what stops the next hour going on a
        // token nobody needs.
        says: ("verb", Says::Holds("read")),
        drive: drive_folder_read,
    },
    Site {
        at: "crates/pull/src/folder.rs — note_refused",
        target: "pull.folder",
        message: "folder refused",
        // THE PATH. A folder feed that produced no bars used to say nothing at
        // all, and the one thing an operator needs next is which directory to
        // open.
        says: ("dir", Says::Holds("NO-SUCH-FOLDER")),
        drive: drive_folder_refused,
    },
    Site {
        at: "crates/pull/src/fetch.rs:585",
        target: "pull.land",
        message: "window decoded",
        says: ("before_window", Says::Signs(1)),
        drive: drive_land,
    },
    Site {
        at: "crates/pull/src/ingest.rs:389",
        target: "pull.fold",
        message: "folded",
        says: ("folded", Says::Signs(1)),
        drive: drive_fold,
    },
    Site {
        at: "crates/pull/src/ingest.rs:422",
        target: "pull.member",
        message: "landed",
        says: ("instrument", Says::Holds(INSTRUMENT)),
        drive: drive_member_landed,
    },
    Site {
        at: "crates/pull/src/ingest.rs:443",
        target: "pull.member",
        message: "did not land",
        says: ("rows", Says::Signs(2)),
        drive: drive_member_not_landed,
    },
    Site {
        at: "crates/pull/src/secret.rs:483",
        target: "pull.secret",
        message: "credential read",
        says: ("byte_len", Says::Signs(5)),
        drive: drive_secret_read,
    },
    Site {
        at: "crates/pull/src/secret.rs:494",
        target: "pull.secret",
        message: "credential refused",
        // -1 AND NEVER 0: `SecretError::Empty` is a parameter that exists and
        // holds nothing, which is a different operator action from a read that
        // produced no value at all.
        says: ("byte_len", Says::Signs(-1)),
        drive: drive_secret_refused,
    },
    Site {
        at: "crates/pull/src/secret.rs:535",
        target: "pull.secret",
        message: "re-read returned a different value; the rotation landed",
        says: ("rotated", Says::Flags(true)),
        drive: drive_secret_reread,
    },
    Site {
        at: "crates/pull/src/http.rs:290",
        target: "pull.http",
        message: "vendor answered",
        says: ("status", Says::Signs(503)),
        drive: drive_http,
    },
];

// ===========================================================================
// The drivers — one shipped call each
// ===========================================================================

/// A multiplicative decrease, through the governor's own public method.
/// One index bar whose volume column carries noise, decoded through the shipped
/// path.
///
/// The value is the operator's own: Dhan sent `-125` on BANKNIFTY's 1-minute
/// index bars and it refused the whole 90-day chunk. The listing is what decides
/// — see `http::one_volume` — so this drives `Index`, and the equity half of the
/// rule is asserted by
/// `http::an_index_negative_volume_is_zero_and_an_equitys_is_still_refused`.
fn drive_index_volume_corrected(_scratch: &Scratch) {
    let spec = match crate::vendor::Feed::Dhan.descriptor().transport {
        crate::vendor::Transport::Http(spec) => spec,
        crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
    };
    let body = r#"{"open":[24500.75],"high":[24501.50],"low":[24499.25],
                   "close":[24500.50],"volume":[-125],"timestamp":[1751337900]}"#;
    let window = crate::http::decode_body(body, &spec, crate::vendor::Listing::Index)
        .expect("an index has no volume, so noise in that column is not a refusal");
    // `first()` RATHER THAN `[0]`: this file is production code, not a test
    // module, so `clippy::indexing_slicing` applies to it exactly as it does to
    // the decoder it drives.
    let first = window
        .rows
        .first()
        .expect("one bar was sent, so one decodes");
    assert_eq!(first.volume, 0, "recorded as the zero the column always is");
}

/// A traded listing sends one good minute and one with a negative volume.
///
/// The row goes; the window stays. Before this the whole window was refused,
/// and the measured cost of that was `ADANIENT` losing every intraday rung it
/// had: one bad row killed a 90-day chunk, the chunk's failure ended the
/// backfill at request 1 of 21, and the seven derived rungs are rolled up from
/// the 1-minute one that never landed.
fn drive_negative_volume_skipped(_scratch: &Scratch) {
    let spec = match crate::vendor::Feed::Dhan.descriptor().transport {
        crate::vendor::Transport::Http(spec) => spec,
        crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
    };
    let body = r#"{"open":[24500.75,24501.00],"high":[24501.50,24501.75],
                   "low":[24499.25,24500.50],"close":[24500.50,24501.25],
                   "volume":[9,-125],"timestamp":[1751337900,1751337960]}"#;
    let window = crate::http::decode_body(body, &spec, crate::vendor::Listing::Equity)
        .expect("one impossible row does not refuse the window");
    assert_eq!(
        window.rows.len(),
        1,
        "the negative row is the one that went"
    );
    let first = window
        .rows
        .first()
        .expect("one row survived, so one decodes");
    assert_eq!(first.volume, 9, "and the GOOD row is what survived");
}

/// One minute whose high sits below its low, beside one that is a bar.
///
/// Caught here, where the vendor's own row is still in hand. The store catches
/// it too — `Bar::ohlc_is_sane` is what printed `batch record 7075 has
/// impossible OHLC` — but by then the rows are session-filtered and folded, so
/// that index names no vendor row, no timestamp and no file an operator can
/// open. Measured: six consecutive runs refused on the same two records.
fn drive_impossible_bar_skipped(_scratch: &Scratch) {
    let spec = match crate::vendor::Feed::Dhan.descriptor().transport {
        crate::vendor::Transport::Http(spec) => spec,
        crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
    };
    // The second row's high is BELOW its low, which no bar can be.
    let body = r#"{"open":[24500.75,24501.00],"high":[24501.50,24400.00],
                   "low":[24499.25,24500.50],"close":[24500.50,24450.00],
                   "volume":[9,11],"timestamp":[1751337900,1751337960]}"#;
    let window = crate::http::decode_body(body, &spec, crate::vendor::Listing::Equity)
        .expect("one impossible row does not refuse the window");
    assert_eq!(
        window.rows.len(),
        1,
        "the impossible row is the one that went"
    );
    let first = window
        .rows
        .first()
        .expect("one row survived, so one decodes");
    assert_eq!(first.volume, 9, "and the GOOD row is what survived");
}

/// A derivative window whose second row reports a negative open interest.
///
/// Open interest is contracts outstanding and is never below zero. `i64::MIN`
/// is deliberately NOT used here: §7 spends that value on "the vendor sent
/// none", so it is a sentinel collision that `one_number` refuses by name
/// rather than a bad count this skips — and driving it here would emit nothing.
fn drive_negative_interest_skipped(_scratch: &Scratch) {
    let shipped = match crate::vendor::Feed::Dhan.descriptor().transport {
        crate::vendor::Transport::Http(spec) => spec,
        crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
    };
    let named = crate::vendor::HttpSpec {
        fields: crate::vendor::FieldNames {
            open_interest: Some("open_interest"),
            ..shipped.fields
        },
        ..shipped
    };
    let body = r#"{"open":[24500.75,24501.00],"high":[24501.50,24501.75],
                   "low":[24499.25,24500.50],"close":[24500.50,24501.25],
                   "volume":[9,11],"timestamp":[1751337900,1751337960],
                   "open_interest":[41,-5]}"#;
    let window = crate::http::decode_body(body, &named, Listing::Derivative)
        .expect("one impossible count does not refuse the window");
    assert_eq!(
        window.rows.len(),
        1,
        "the negative row is the one that went"
    );
    let first = window
        .rows
        .first()
        .expect("one row survived, so one decodes");
    assert_eq!(first.open_interest, Some(41), "and the GOOD row survived");
}

fn drive_rate(_scratch: &Scratch) {
    let mut governor = crate::rate::Governor::new(Some(5), None, None)
        .expect("one published ceiling and two spans this vendor bounds nothing on");
    governor.record_throttled();
}

/// A configuration file that is there and parses.
fn drive_config_loaded(scratch: &Scratch) {
    let path = scratch.root.join("CREDENTIALS");
    fs::write(&path, CONFIG).expect("a scratch configuration");
    let config = CredentialConfig::load(&path).expect("the invented fixture loads");
    assert_eq!(
        config.region(),
        "ap-south-1",
        "the one region CLAUDE.md section 8 names"
    );
}

/// A configuration file that is there and does not parse.
///
/// **Not a missing file, deliberately.** `CredentialConfig::load` returns on
/// the read before `note_load` is ever called, so an absent file leaves no line
/// here — the refusal an operator sees for that one is the `Unreadable` error
/// itself. What this row proves is the other half: a file that was read and
/// whose contents were refused, which is the case that used to reach an HTML
/// page and nothing else.
fn drive_config_refused(scratch: &Scratch) {
    let path = scratch.root.join("HALF-A-CONFIGURATION");
    fs::write(&path, "org    = \"orgone\"\n").expect("a scratch configuration");
    assert!(
        CredentialConfig::load(&path).is_err(),
        "a configuration naming no vendor halts, and never defaults"
    );
}

/// A window that has to be broken at two month boundaries.
fn drive_split(_scratch: &Scratch) {
    let span = Window::new(
        Day::new(2031, 3, 7).expect("a real day"),
        Day::new(2031, 5, 9).expect("a real day"),
    )
    .expect("a forward window");
    let chunks = crate::session::split_window(span, None)
        .expect("no published cap is not a refusal, it is a fact about the vendor");
    assert_eq!(
        chunks.len(),
        3,
        "the store addresses one month per file, so March, April and May are three"
    );
}

/// A shared secret one byte past the ceiling.
fn drive_totp(_scratch: &Scratch) {
    let over = "A".repeat(crate::totp::MAX_SECRET_LEN + 1);
    assert!(
        crate::totp::base32_decode(&over).is_err(),
        "one past the bound is refused rather than truncated"
    );
}

/// A selection carrying one blank name and one repeat.
fn drive_work(_scratch: &Scratch) {
    let narrowed = crate::work::Selection::of([INSTRUMENT, INSTRUMENT, "", "BANKNIFTY"]);
    assert_eq!(
        narrowed.len(),
        2,
        "the blank and the duplicate are dropped, and the drop is what earns the line"
    );
}

/// A census file that is not there.
fn drive_census_absent(_scratch: &Scratch) {
    let genesis = Manifest::open(Vendor::Groww, &[], &[]).expect("nothing at all is a genesis");
    assert_eq!(genesis.entries(), 0, "a genesis census counts nothing");
}

/// A census file whose header region cannot be a header region.
fn drive_census_refused(_scratch: &Scratch) {
    assert!(
        Manifest::open(Vendor::Groww, &[1, 2, 3], &[]).is_err(),
        "three bytes are not a header region, and a short one is refused by name"
    );
}

/// A whole-file census image, built from a census holding one month.
fn drive_census_imaged(scratch: &Scratch) {
    let done = ingest_body(scratch);
    assert_eq!(
        done.failures,
        Vec::new(),
        "no member failed, so the census this run images is a real one"
    );
    // EIGHT, NOT ONE, AND THE EIGHT ARE THE POINT. This asserted one — the
    // rung that was pulled — and a one-minute member now writes that rung plus
    // every rung derived from it: `crate::ingest::derived_from` answers the set
    // from `Timeframe::KNOWN` rather than from a list, so it is 2, 3, 5, 10, 15,
    // 30 and 60 minutes today and whatever the store ships tomorrow.
    //
    // Asserted against the RULE rather than against 8, so a rung added to the
    // store moves this test's expectation with it instead of breaking it.
    let derived = crate::ingest::derived_count(store::path::Timeframe::MINUTE_1);
    assert_eq!(
        done.counted,
        1 + derived,
        "the pulled rung and every rung derived from it each reach the counter"
    );
    assert_eq!(
        done.derived_files, derived,
        "and the receipt says how many were built rather than fetched"
    );
}

/// A CSV body of three rows in the shape it was written in.
fn drive_csv_decoded(_scratch: &Scratch) {
    let rows = crate::csv::decode(BODY, Columns::TrueDataIndex).expect("five fields, no header");
    assert_eq!(rows.len(), 3, "three rows in, three rows out");
}

/// A CSV row that is two fields where five are required.
fn drive_csv_refused(_scratch: &Scratch) {
    assert!(
        crate::csv::decode("20221003,10:00:00\n", Columns::TrueDataIndex).is_err(),
        "a malformed line refuses the whole file, never a prefix of it"
    );
}

/// A folder holding exactly one member.
fn drive_archive_walked(scratch: &Scratch) {
    let dir = scratch.archive(&[(INSTRUMENT, BODY)]);
    let members = crate::archive::read_dir(&dir, Columns::TrueDataIndex).expect("one real member");
    assert_eq!(members.len(), 1, "one file in the folder, one member out");
}

/// A folder that is not a folder.
fn drive_archive_refused(scratch: &Scratch) {
    let missing = scratch.root.join("NO-SUCH-FOLDER");
    assert!(
        crate::archive::read_dir(&missing, Columns::TrueDataIndex).is_err(),
        "a walk over nothing refuses rather than returning an empty import"
    );
}

/// A folder whose reach is read off the files in it.
fn drive_folder_read(scratch: &Scratch) {
    let dir = scratch.archive(&[(INSTRUMENT, BODY)]);
    let reach =
        crate::folder::read_reach(&dir, crate::vendor::Feed::TrueData, Columns::TrueDataIndex)
            .expect("the folder is there and its one member decodes");
    assert!(
        !reach.is_empty(),
        "a folder holding a real member reaches the days that member covers"
    );
}

/// A folder that is not there — the halt that names the path.
fn drive_folder_refused(scratch: &Scratch) {
    let missing = scratch.root.join("NO-SUCH-FOLDER");
    assert!(
        crate::folder::read_reach(
            &missing,
            crate::vendor::Feed::TrueData,
            Columns::TrueDataIndex
        )
        .is_err(),
        "a folder that is not there halts rather than reporting an empty reach"
    );
}

/// One row that is outside the operator's window and is declined for it.
fn drive_land(_scratch: &Scratch) {
    let request = request_over(window());
    let raw = RawWindow {
        rows: vec![RawRow {
            timestamp: 0,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 0,
            open_interest: None,
        }],
    };
    let landed = crate::fetch::land(
        &raw,
        &request,
        TimestampEncoding::EpochSecondsUtc,
        PriceScale::Paisa,
    )
    .expect("a row outside the window is declined, not a refusal of the window");
    assert!(
        landed.bars.is_empty(),
        "1970 is before October 2022, so nothing is stored and the census says why"
    );
}

/// A member whose two rows share a minute, so the fold consumes one.
fn drive_fold(scratch: &Scratch) {
    let done = ingest_body(scratch);
    assert_eq!(
        done.rows_folded, 1,
        "10:00:00 and 10:00:01 are one minute, and the second folds into the first"
    );
}

/// A member that lands.
fn drive_member_landed(scratch: &Scratch) {
    let done = ingest_body(scratch);
    assert_eq!(
        done.bars_stored, 2,
        "three rows, one fold, two bars on disk"
    );
}

/// A member whose bars would need two files.
fn drive_member_not_landed(scratch: &Scratch) {
    // A NAME NO `Symbol` CAN HOLD, because the refusal this used to drive is
    // gone. It fed a two-month batch and asserted the store refused it — and
    // `ingest::months_in` now writes one file per month out of exactly that,
    // which is the whole point of D-0320. `identify` is the remaining per-member
    // failure and it is a better driver anyway: a member that cannot be named
    // is a member that cannot be filed, whatever its bars look like.
    //
    // `SYMBOL_CAPACITY` is 24, so twenty-five characters cannot be a symbol.
    let unnameable = "A".repeat(25);
    let archive = scratch.archive(&[(unnameable.as_str(), SPANNING)]);
    let store = scratch.store();
    let request = request_over(crossing());
    let done = crate::ingest::from_dir(&archive, &store, plan_over(&request))
        .expect("the folder is readable; the refusal is per member, not per folder");
    assert_eq!(
        done.failures.len(),
        1,
        "a member whose name is not a legal symbol cannot be filed"
    );
    assert_eq!(
        done.bars_stored, 0,
        "and it stored nothing, so the census has nothing to disagree with"
    );
}

/// A credential the source answers with.
fn drive_secret_read(_scratch: &Scratch) {
    let config = credentials();
    let path = config
        .path_for(Vendor::Groww, "fieldtwo")
        .expect("a field the invented fixture declares");
    let reader = CredentialReader::new(Fixed {
        answer: Ok("fresh"),
    });
    let secret = reader
        .read(Vendor::Groww, &path)
        .expect("the double answered");
    assert_eq!(
        secret.byte_len(),
        5,
        "the length is the one property of a credential that is safe to report"
    );
}

/// A credential the source refuses.
fn drive_secret_refused(_scratch: &Scratch) {
    let config = credentials();
    let path = config
        .path_for(Vendor::Groww, "fieldtwo")
        .expect("a field the invented fixture declares");
    let reader = CredentialReader::new(Fixed {
        answer: Err(SecretError::AccessDenied),
    });
    assert!(
        reader.read(Vendor::Groww, &path).is_err(),
        "a refused credential halts the pull, and never falls through"
    );
}

/// A re-read that returns a different value, which is the rotation landing.
fn drive_secret_reread(_scratch: &Scratch) {
    let config = credentials();
    let path = config
        .path_for(Vendor::Groww, "fieldtwo")
        .expect("a field the invented fixture declares");
    let rejected = Secret::new("stale".to_owned()).expect("a non-empty value");
    let reader = CredentialReader::new(Fixed {
        answer: Ok("fresh"),
    });
    let rotated = reader
        .reread_after_rejection(Vendor::Groww, &path, &rejected)
        .expect("a different value is a rotation, not a dead token");
    assert_ne!(
        rotated.byte_len(),
        0,
        "and nothing here minted it — CLAUDE.md section 8"
    );
}

/// A vendor that answers, over a socket this test opened.
fn drive_http(_scratch: &Scratch) {
    let (url, reached) = loopback(REFUSAL);
    let spec = match crate::vendor::Feed::Dhan.descriptor().transport {
        crate::vendor::Transport::Http(shipped) => crate::vendor::HttpSpec {
            // Leaked for the same reason `crate::http`'s own socket tests leak
            // theirs: a descriptor's base URL is `&'static str`, and a port
            // that is only known at run time has to outlive the borrow.
            base_url: Box::leak(url.into_boxed_str()),
            ..shipped
        },
        crate::vendor::Transport::LocalArchive(_) => {
            panic!("this feed authenticates, so its transport is HTTP")
        }
    };
    let source =
        crate::http::HttpSource::new(spec, crate::http::Credential::token("shhh".to_owned()))
            .expect("an HTTPS client builds");
    let request = BarRequest {
        instrument_id: "13".to_owned(),
        listing: Listing::Index,
        window: window(),
        granularity: Granularity::Minute1,
    };
    let answered = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(source.window_async(&request));
    assert!(
        answered.is_err(),
        "a 503 is a refusal — and the line is written before it is returned"
    );
    assert!(
        reached.recv_timeout(Duration::from_secs(5)).is_ok(),
        "the loopback vendor was contacted, so there was an answer to write down"
    );
}

// ===========================================================================
// The proof
// ===========================================================================

/// Every record of one subsystem written at or after `since`.
fn newer(dir: &Path, sink: &Sink, since: u64, target: &str) -> Vec<Record> {
    telemetry::tail(
        dir,
        sink.keep_files(),
        &Query::last(MAX_LIMIT).from_target(target),
    )
    .records
    .into_iter()
    .filter(|record| record.seq >= since)
    .collect()
}

/// **EVERY PRODUCTION `telemetry::emit` IN THIS CRATE WRITES A LINE.**
///
/// One install, one floor of [`Level::Trace`], one scratch tree per row, and
/// for each row: the sequence number the sink would stamp next, the shipped
/// call, and then a walk of the file for a record of that target carrying that
/// message and that field with a sequence number no earlier than the one taken
/// before the call. The sequence check is what makes a row fail when its own
/// driver writes nothing and some other test in this binary happens to have
/// written the same target.
///
/// # The defect it catches
///
/// A helper that stops emitting. Every one of these sites discards
/// [`telemetry::Emitted`] — correctly, because a `Debug` event is *supposed* to
/// be dropped at the default floor — so nothing at the call site can notice a
/// `return` above the `emit`, a target renamed on one side of a filter, or a
/// level raised past a sink's floor. The only place that shows is the file, and
/// until this test nothing read it.
#[test]
fn every_emit_site_in_this_crate_reaches_a_file() {
    // Declared first so it is dropped LAST, and a [`Scratch`] rather than a
    // bare path so a row that fails still takes its sink's directory with it —
    // a panicking test must not leave the log it was reading behind.
    let sink_home = Scratch::new();
    let home = sink_home.root.as_path();
    let sink = telemetry::install(&Config::new(home).with_min_level(Level::Trace))
        .expect("the only install in this test binary — a second one is refused by name");

    for site in SITES {
        let scratch = Scratch::new();
        let before = sink.health().next_seq;
        (site.drive)(&scratch);

        let (key, says) = site.says;
        let landed = newer(home, sink, before, site.target);
        assert!(
            landed.iter().any(|record| {
                record.message == site.message
                    && record
                        .field(key)
                        .is_some_and(|held| says.satisfied_by(held))
            }),
            "{} wrote no `{}` record saying `{key}` — the whole file since \
             seq {before} holds {:?}",
            site.at,
            site.message,
            landed
                .iter()
                .map(|record| (&record.message, record.field(key)))
                .collect::<Vec<_>>()
        );
    }

    // ABSENCE, WHICH IS HALF OF TWO OF THESE SITES.
    //
    // `work::note_narrowed` returns before it emits when nothing was dropped,
    // and `ingest::install_census` never builds an image when nothing moved.
    // A test that only proved the writing half would pass just as well against
    // a helper that wrote unconditionally — which is the mutant, and it is the
    // one an operator would notice, because a line that fires on every clean
    // run is a line nobody reads.
    let quiet = sink.health().next_seq;
    let clean = crate::work::Selection::of([INSTRUMENT, "BANKNIFTY"]);
    assert_eq!(clean.len(), 2, "nothing was blank and nothing repeated");
    assert_eq!(
        newer(home, sink, quiet, "pull.work"),
        Vec::new(),
        "a selection that dropped nothing must leave no line at all"
    );

    let scratch = Scratch::new();
    ingest_body(&scratch);
    let settled = sink.health().next_seq;
    ingest_body(&scratch);
    assert!(
        newer(home, sink, settled, "pull.manifest")
            .iter()
            .all(|record| record.message != "census image built"),
        "a re-run over the same folder moves nothing, so it images nothing and \
         leaves the census byte for byte as it was"
    );
}

/// One completed run, named.
///
/// The ingest that [`drive_census_imaged`] runs also produces the run line, so
/// this drive is that one call and nothing else — a run whose members all
/// landed is the case where a missing name is hardest to notice, because
/// nothing else in the record is wrong.
fn drive_run_named(scratch: &Scratch) {
    let done = ingest_body(scratch);
    // NOT `pending`, AND THIS ASSERTION IS THE CORRECTION. It first read
    // `done.pending.is_some()`, because that is where the first draft of
    // `note_run` took the name from — and it is `None` on this path and on
    // every path a broker pull takes. The name comes from the plan now, so what
    // is worth asserting here is that the run this row drives really ran.
    assert_eq!(
        done.members, 1,
        "one member was read, so the run has exactly one subject to name"
    );
}

/// One batch that cannot be ADDRESSED, on the rolling path.
///
/// **Not the folder path, and the first draft of this drive took that one.**
/// `note_not_filed` is called from `ingest::from_rows`, which is the rolling
/// contract path; `drive_member_not_landed` runs `from_dir`, and an unnameable
/// member there fails in `from_members_inner` through a different helper
/// entirely. The row asserted against it produced no records at all — an empty
/// file, which is what a drive on the wrong path looks like.
///
/// Two bars either side of a month boundary. The store addresses ONE month per
/// file, so a batch needing two is refused at the `address` stage — which is a
/// real refusal with a real caller, not a fault invented to reach a log line.
fn drive_not_filed(scratch: &Scratch) {
    let store = scratch.store();
    let request = request_over(crossing());
    let at = |secs: i64| store::format::Bar {
        ts_micros: secs * 1_000_000,
        open: 100,
        high: 100,
        low: 100,
        close: 100,
        volume: 1,
        open_interest: i64::MIN,
    };
    // 2022-10-03 and 2022-11-03, both inside `crossing()`, in two months.
    let bars = [at(1_664_775_000), at(1_667_453_400)];
    let done = crate::ingest::from_rows(
        &bars,
        &[],
        INSTRUMENT,
        "emit-sites",
        &store,
        plan_over(&request),
    );
    assert_eq!(
        done.failures.len(),
        1,
        "a batch spanning two months is one member's refusal, and the store \
         addresses one month per file"
    );
    assert_eq!(
        done.bars_stored, 0,
        "nothing was filed, so nothing may be reported as stored"
    );
}

/// A census whose SECOND header slot will not validate.
///
/// # Why a corrupt slot rather than a corrupt file
///
/// The census is double-buffered: two 64-byte headers at
/// [`store::format::SLOT_STRIDE`] apart, and `Manifest::open` validates both.
/// One good slot beside one bad one is the state this event exists for — the
/// census still LOADS, from the generation that survived, and reports
/// `degraded_reason()` so an operator learns that a generation was stepped
/// over rather than discovering it when the next write lands on a file it
/// cannot read.
///
/// Corrupting the whole file would produce a different event entirely: nothing
/// loads, and the run refuses. That is `pull.census` "not published", not this
/// one, and the two must not be driven by one fixture.
///
/// # Scribbling on the slot does NOT work, and the reason is the format's
///
/// The first draft filled the second slot with `0xA5` and the row read back an
/// empty file. That is the format behaving correctly: `is_specific` excludes
/// `ManifestError::NotAManifest`, because **an unwritten slot is a legal state
/// and must never read as damage.** Garbage without a magic is
/// indistinguishable from a slot nothing has reached yet, so it is skipped in
/// silence — which is right, and which no amount of scribbling will get past.
///
/// So the header stays VALID and is put in the WRONG PLACE: slot 0's bytes are
/// copied over slot 1. A header decodes there, its checksum holds, and its
/// `generation % SLOT_COUNT` says slot 0 while it is sitting at index 1 —
/// `SlotPositionMismatch`, which IS specific, sets `fault`, and surfaces as
/// `stepped_over`.
///
/// Nothing here forges a header or recomputes a checksum: every byte written
/// was produced by the writer itself, one slot over.
fn degrade_second_slot(store: &Path) {
    let census = crate::manifest::manifest_path(store, Vendor::Groww);
    let mut image = fs::read(&census).expect("the census this run just wrote");
    let at = usize::try_from(store::format::SLOT_STRIDE).expect("16,384 fits a usize");
    assert!(
        image.len() > at + store::format::SLOT_LEN,
        "the census file reaches its second slot, or this fixture corrupts \
         nothing and the row would pass for the wrong reason"
    );
    let first: Vec<u8> = image
        .get(..store::format::SLOT_LEN)
        .expect("the first slot's header")
        .to_vec();
    image
        .get_mut(at..at.saturating_add(store::format::SLOT_LEN))
        .expect("the second slot's header")
        .copy_from_slice(&first);
    fs::write(&census, &image).expect("the misplaced census header");
}

/// One ingest over a census whose second generation will not read.
fn drive_census_degraded(scratch: &Scratch) {
    // A FIRST RUN, so there is a real census to damage. Writing one by hand
    // would be a second opinion about the format.
    let done = ingest_body(scratch);
    assert_eq!(
        done.failures,
        Vec::new(),
        "the run that BUILDS the census is clean; the damage comes after"
    );
    degrade_second_slot(&scratch.store());

    // AND A SECOND RUN, which opens the damaged census.
    let archive = scratch.archive(&[(INSTRUMENT, BODY)]);
    let store = scratch.store();
    let request = request_over(window());
    let after = crate::ingest::from_dir(&archive, &store, plan_over(&request))
        .expect("the folder is still readable");
    assert!(
        after
            .failures
            .iter()
            .any(|f| f.why.contains("DEGRADED") || f.why.contains("degraded")),
        "a census that stepped over a generation is a named failure, not a \
         silent one: {:?}",
        after.failures
    );
}

/// One run whose slices land and whose census cannot be written back.
///
/// # The state this exists for, and why it is the worst one
///
/// `write_appends` opens the census `read(true).write(true)` and appends in
/// place. Take that write away and the run has already put **bars on disk** —
/// they are filed, they are correct, and nothing counts them. A later reader
/// asks the census what the store holds and is told less than the truth.
///
/// The bars cannot be un-written: the store is append-only and `Header::advance`
/// refuses a batch beginning at or before what is committed. So this line is
/// the only record that the two ever disagreed, and `slices` is the number of
/// entries that went uncounted.
///
/// # Mode `0o444`, following `pull/tests/pipeline.rs`
///
/// Readable so `read_census` still succeeds — the run must get far enough to
/// have something to publish — and unwritable so the install fails. The file is
/// reopened afterwards so `Scratch`'s `Drop` can remove it.
fn drive_census_unpublished(scratch: &Scratch) {
    use std::os::unix::fs::PermissionsExt as _;

    // A FIRST RUN, so a real census exists to be made unwritable.
    let done = ingest_body(scratch);
    assert_eq!(done.failures, Vec::new(), "the run that builds it is clean");

    let store = scratch.store();
    let census = crate::manifest::manifest_path(&store, Vendor::Groww);
    fs::set_permissions(&census, fs::Permissions::from_mode(0o444))
        .expect("close the census to writing");

    // A SECOND RUN over a DIFFERENT month, so it has entries to append rather
    // than a re-run's empty set — `install_census` returns early when there is
    // nothing to publish and no repair to make.
    let archive = scratch.archive(&[(INSTRUMENT, NEXT_MONTH)]);
    let request = request_over(november());
    let after = crate::ingest::from_dir(&archive, &store, plan_over(&request))
        .expect("the folder is readable; the census is what refuses");

    fs::set_permissions(&census, fs::Permissions::from_mode(0o644))
        .expect("reopen it, so Drop can remove the tree");

    assert!(
        after.failures.iter().any(|f| f.why.contains("census")),
        "a census that will not take the write is a named failure: {:?}",
        after.failures
    );
}

/// One run where a DERIVED rung cannot be written and the pulled one can.
///
/// # Why one fixture drives two events
///
/// A one-minute member writes the rung it pulled plus the seven
/// `ingest::derived_from` computes from `Timeframe::KNOWN`. Block exactly one of
/// the seven and two things happen in order: `derive` fails for that rung and
/// reports `pull.derive`, then `derived_shortfall` counts 6 of 7 and reports
/// `pull.derived`. Driving them apart would need two runs to produce one
/// state.
///
/// # The block is a DIRECTORY where a file belongs
///
/// Not a permission bit. A directory at the bar file's own path cannot be
/// opened for writing by any user, including root — so this fixture does not
/// quietly stop testing anything when the suite runs in a container. The
/// two-minute rung is chosen because it is the first one `derived_from` yields
/// and the smallest thing that can be taken away.
///
/// **The pulled rung still lands**, which is the half that makes this state
/// worth an `Error`: the instrument-month reads as held at one timeframe and
/// missing at the rest, and a reader asking for two-minute bars finds nothing
/// while one-minute bars sit beside it.
fn block_the_two_minute_rung(store: &Path) {
    let path = store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: INSTRUMENT,
        contract: None,
        timeframe: store::path::Timeframe::MINUTE_2,
        month: store::path::YearMonth::new(2022, 10).expect("a legal month"),
        file: store::path::FileKind::Bars,
    })
    .expect("a legal path")
    .to_path_buf(store);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the rung's directory");
    }
    fs::create_dir_all(&path).expect("a DIRECTORY where the bar file belongs");
}

/// One ingest whose two-minute rung has nowhere to go.
fn drive_derived_shortfall(scratch: &Scratch) {
    let store = scratch.store();
    block_the_two_minute_rung(&store);
    let archive = scratch.archive(&[(INSTRUMENT, BODY)]);
    let request = request_over(window());
    let done = crate::ingest::from_dir(&archive, &store, plan_over(&request))
        .expect("the folder is readable; the rung is what refuses");
    assert!(
        done.bars_stored > 0,
        "the PULLED rung still lands — a fixture where nothing stored would \
         prove the shortfall event fires when there is no shortfall to report"
    );
    assert!(
        done.failures.iter().any(|f| f.why.contains("derived rung")),
        "6 of 7 derived rungs landed and the run says so: {:?}",
        done.failures
    );
}

/// One run whose bars land and whose census refuses to count them.
///
/// # The state, and why the census is right to refuse
///
/// A census entry already holding 9,999 rows meets a run that lands 2.
/// `Manifest::record_held` answers `RowCountWentBackwards`, because a count that
/// goes DOWN is either a truncated file or a stale write arriving late, and
/// silently accepting the smaller number would make the census forget bars that
/// are still on disk.
///
/// So the refusal is correct — and it leaves the exact state this event exists
/// for: **the bars are written and the census does not count them.** The store
/// is append-only, so they cannot be withdrawn; the only record that the two
/// disagree is this line and the `Failure` beside it.
///
/// The seeded entry is written through `ingest::record_held`, the shipped
/// writer, rather than by hand — a census this test forged would be a second
/// opinion about the format.
fn drive_bars_not_counted(scratch: &Scratch) {
    let store = scratch.store();
    let seeded = crate::manifest::Held::new(
        crate::manifest::Entry {
            key: crate::manifest::EntryKey {
                contract: None,
                exchange: brutex_core::instrument::Exchange::Nse,
                segment: brutex_core::instrument::Segment::Index,
                symbol: brutex_core::symbol::Symbol::new(INSTRUMENT).expect("a legal symbol"),
                timeframe: store::path::Timeframe::MINUTE_1,
                month: store::path::YearMonth::new(2022, 10).expect("a legal month"),
            },
            // MORE THAN THE RUN CAN POSSIBLY LAND, so the direction is not in
            // doubt: `BODY` holds three rows and folds to two bars.
            rows: 9_999,
            first_ts_micros: 1_664_775_000_000_000,
            last_ts_micros: 1_664_775_060_000_000,
        },
        crate::manifest::Closes::UNKNOWN,
    );
    assert_eq!(
        crate::ingest::record_held(&store, Vendor::Groww, std::slice::from_ref(&seeded)),
        None,
        "the seed census is written by the shipped writer, or this fixture is \
         testing a format it invented"
    );

    let archive = scratch.archive(&[(INSTRUMENT, BODY)]);
    let request = request_over(window());
    let done = crate::ingest::from_dir(&archive, &store, plan_over(&request))
        .expect("the folder is readable; the census is what refuses");
    assert!(
        done.bars_stored > 0,
        "the bars LANDED — that is the whole point, and a fixture that stored \
         nothing would report an uncounted-bars event with no uncounted bars"
    );
    assert!(
        done.failures
            .iter()
            .any(|f| f.why.contains("the census does not count")),
        "and the run says which instrument holds bars nothing counts: {:?}",
        done.failures
    );
}

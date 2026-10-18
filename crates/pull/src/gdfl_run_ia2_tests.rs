//! IA-2's tests of `pull::gdfl_run`: the runs ledger (IA-GR-11, -11b, -36,
//! -38), the locks by census layout (IA-GR-02's child half, -03, -17, -46,
//! RA-285), the claim and its sidecar (IA-GR-35, -47, RA-307), the start gate
//! and the stop request (IA-GR-21). No vendor byte anywhere.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::ledger::{self, Entry, LedgerHold, RunsLedgerFault};
use super::locks::{self, Claim, ClaimState, LockFault, LockState, RunLocks, SidecarState};
use super::start::{Gate, StartGate, await_go_within};
use super::stop::{self, StopTarget};
use super::*;

/// A clock inside the calendar's span: 2026-10-01 00:00 UTC.
const NOW: u64 = 1_790_812_800;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-ia2-{tag}-{}", std::process::id()));
    let _fresh = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn v4(root: &Path) -> PathBuf {
    let store = root.join("v4");
    std::fs::create_dir_all(store.join("manifest/v4/gdfl")).unwrap();
    store
}

fn entry(mode: &str, kind: &str, by: &str, started: u64) -> Entry {
    Entry {
        id: 0,
        mode: mode.to_owned(),
        kind: kind.to_owned(),
        by: by.to_owned(),
        pid: 4242,
        started,
        resumes: 0,
    }
}

fn template(kind: &str, by: &str) -> Claim {
    Claim {
        run: 0,
        mode: "import".to_owned(),
        kind: kind.to_owned(),
        batch: String::new(),
        source: "tickstore".to_owned(),
        from: "2024-04-01".to_owned(),
        to: "2024-04-03".to_owned(),
        filter: "*".to_owned(),
        verify: "off".to_owned(),
        restate: false,
        repair: "0".to_owned(),
        workers: 8,
        auto_resume: false,
        pid: 4242,
        host: "-".to_owned(),
        exe: "-".to_owned(),
        by: by.to_owned(),
        started: 0,
        resumes: 0,
        resume_day: "-".to_owned(),
        legacy_ack: false,
        unstamped: false,
        request: "-".to_owned(),
    }
}

fn allocate(store: &Path, at: u64) -> Result<u64, RunsLedgerFault> {
    ledger::allocate(store, &entry("import", "options", "cli", at))
}

// ═══ The runs ledger ════════════════════════════════════════════════════════

/// **IA-GR-11: RUN IDS NEVER REPEAT, WHATEVER THE CLOCK DOES.** Ten runs
/// claimed in one second, then a clock stepped back an hour: every id is
/// distinct and larger than the last; a torn record is written over.
#[test]
fn run_ids_never_repeat_whatever_the_clock_does() {
    let store = scratch("ids");
    let mut ids = Vec::new();
    for _ in 0..10 {
        ids.push(allocate(&store, NOW).unwrap());
    }
    ids.push(allocate(&store, NOW - 3_600).unwrap());
    assert!(ids.windows(2).all(|w| w[0] < w[1]), "{ids:?}");
    assert_eq!(ids[0], NOW, "the first id is the clock");
    // A torn append: 50 bytes past the last whole record.
    let path = store.join(ledger::LEDGER);
    let mut bytes = std::fs::read(&path).unwrap();
    let whole = bytes.len();
    bytes.extend_from_slice(&[b'7'; 50]);
    std::fs::write(&path, &bytes).unwrap();
    let next = allocate(&store, NOW).unwrap();
    assert_eq!(next, ids.last().unwrap() + 1);
    assert_eq!(
        std::fs::read(&path).unwrap().len(),
        whole + 128,
        "written over"
    );
    let all = ledger::read_all(&store).unwrap();
    assert!(all.iter().all(Result::is_ok), "{all:?}");
    assert_eq!(all[0].as_ref().unwrap().mode, "seed");
}

/// **IA-GR-11b (RA-292, RA-326): A CLOCK THAT CANNOT BE RIGHT GIVES NO ID.**
/// Before the calendar's first day (a dead battery's 1970), more than a year
/// past its sources (2040), or more than a day behind the newest run: each
/// refused `clock_implausible`, and the ledger is not written.
#[test]
fn an_implausible_clock_is_refused_and_lifts_no_id() {
    let store = scratch("clock");
    for wrong in [0, 1_000_000_000, 2_208_988_800] {
        let got = allocate(&store, wrong).unwrap_err();
        assert_eq!(got.code(), "clock_implausible", "{wrong}: {got}");
    }
    assert!(!store.join(ledger::LEDGER).exists() || ledger::read_all(&store).unwrap().is_empty());
    let id = allocate(&store, NOW).unwrap();
    let behind = allocate(&store, NOW - 2 * 86_400).unwrap_err();
    assert_eq!(behind.code(), "clock_implausible", "{behind}");
    assert_eq!(
        allocate(&store, NOW - 3_600).unwrap(),
        id + 1,
        "an hour back is a step, not a fault"
    );
}

/// **IA-GR-36: ONE RUNS LEDGER SERVES BOTH FEEDS AND SKIPS A TORN TAIL.** A
/// GDFL import and a Zerodha pull claimed in one second get two ids; a
/// length that is not a multiple of 128 is read from its last whole record.
#[test]
fn one_runs_ledger_serves_both_feeds_and_skips_a_torn_tail() {
    let store = scratch("feeds");
    let gdfl = ledger::allocate(&store, &entry("import", "options", "cli", NOW)).unwrap();
    let zerodha = ledger::allocate(&store, &entry("pull", "zerodha", "api", NOW)).unwrap();
    assert_ne!(gdfl, zerodha);
    let path = store.join(ledger::LEDGER);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.extend_from_slice(b"99999999999 import");
    std::fs::write(&path, &bytes).unwrap();
    let after = ledger::allocate(&store, &entry("verify-merge", "-", "-", NOW)).unwrap();
    assert_eq!(after, zerodha + 1);
    let all = ledger::read_all(&store).unwrap();
    let modes: Vec<&str> = all
        .iter()
        .map(|e| e.as_ref().unwrap().mode.as_str())
        .collect();
    assert_eq!(modes, ["seed", "import", "pull", "verify-merge"]);
}

/// **IA-GR-38: A GARBAGE LEDGER TAIL IS NAMED, NEVER READ AS AN ID.** A whole
/// last record whose CRC fails is written over and its id never used; a seed
/// shorter than a record is seeded again; two bad records refuse
/// `runs_ledger_damaged` with an alert, and no id is given.
#[test]
fn a_garbage_runs_ledger_tail_is_named_never_read_as_an_id() {
    let store = scratch("garbage");
    let first = allocate(&store, NOW).unwrap();
    let path = store.join(ledger::LEDGER);
    // A whole record of a huge id whose CRC fails: an append whose sync
    // never finished.
    let mut forged = Entry {
        id: 9_999_999_999,
        ..entry("import", "options", "cli", NOW)
    }
    .record()
    .unwrap();
    forged[120] ^= 1;
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.extend_from_slice(&forged);
    std::fs::write(&path, &bytes).unwrap();
    let next = allocate(&store, NOW).unwrap();
    assert_eq!(next, first + 1, "the bad record's id is never read");
    assert_eq!(std::fs::read(&path).unwrap().len(), 3 * 128, "written over");

    // A seed torn short: seeded again.
    let short = scratch("garbage-short");
    std::fs::create_dir_all(short.join("audit")).unwrap();
    std::fs::write(short.join(ledger::LEDGER), b"1790 seed - -").unwrap();
    assert_eq!(allocate(&short, NOW).unwrap(), NOW);

    // Two bad records at the tail: refused by name, with an alert.
    let mut bytes = std::fs::read(&path).unwrap();
    let len = bytes.len();
    bytes[len - 10] ^= 1;
    bytes[len - 138] ^= 1;
    std::fs::write(&path, &bytes).unwrap();
    let got = allocate(&store, NOW).unwrap_err();
    assert_eq!(
        got,
        RunsLedgerFault::Damaged {
            offset: u64::try_from(len).unwrap() - 256
        }
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "nothing written");
    let alerts = std::fs::read(store.join("audit/alerts")).unwrap();
    assert!(alerts.len() > 64, "the alert was put");
    let refusal: RunRefusal = got.into();
    assert_eq!(refusal.code(), "runs_ledger_damaged");
}

/// **L-IA-23: THE LEDGER IS SEEDED PAST EVERY ID THE JOURNAL HOLDS.** A store
/// whose journal names run 1,900,000,000 gives its first ledger id past it.
#[test]
fn the_ledger_is_seeded_past_every_journal_run() {
    let store = scratch("seed");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    std::fs::write(
        crate::gdfl_import::journal_path(&store),
        "run 1700000000 source=unrecorded\nbegin indices 2024-04-01 *\nrun 1900000000 source=x\n",
    )
    .unwrap();
    assert_eq!(allocate(&store, NOW).unwrap(), 1_900_000_001);
}

// ═══ Locks ══════════════════════════════════════════════════════════════════

/// **IA-GR-46 (D-4906): A LOCK HELD PAST THE WAIT IS REFUSED BY NAME.** The
/// journal lock and the runs ledger lock, each held by another open file,
/// are refused `lock_stalled` naming the lock, never waited for for ever.
#[test]
fn a_lock_held_past_the_wait_is_refused_by_name() {
    let store = scratch("stall");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    let journal_lock =
        crate::gdfl_import::journal_lock_path(&crate::gdfl_import::journal_path(&store));
    let held = locks::take_path_within(&journal_lock, Duration::from_millis(50)).unwrap();
    let got = locks::take_path_within(&journal_lock, Duration::from_millis(60)).unwrap_err();
    assert!(
        matches!(&got, LockFault::Stalled { lock, waited_ms } if lock == "gdfl.journal.lock" && *waited_ms >= 60),
        "{got:?}"
    );
    drop(held);
    let ledger_held = LedgerHold::take(&store, Duration::from_millis(50)).unwrap();
    let got = LedgerHold::take(&store, Duration::from_millis(60)).unwrap_err();
    assert_eq!(got.code(), "lock_stalled", "{got}");
    drop(ledger_held);
    assert!(LedgerHold::take(&store, Duration::from_millis(60)).is_ok());
}

/// **IA-GR-03: THE CLI AND THE API EXCLUDE EACH OTHER BY THE CENSUS LAYOUT.**
/// On v3 one store lock serialises every kind; on v4 kinds run beside each
/// other and one kind is one run, whoever started it. The busy answer names
/// the holder's claim.
#[test]
fn the_cli_and_the_api_exclude_each_other_by_the_census_layout() {
    let root = scratch("exclude");
    let v3 = root.join("v3");
    std::fs::create_dir_all(&v3).unwrap();
    let cli = claim_with(&v3, template("options", "cli"), 0, None, NOW).unwrap();
    let api = claim_with(&v3, template("indices", "api"), 0, None, NOW).unwrap_err();
    let RunRefusal::RunBusy { lock, holder } = &api else {
        panic!("{api:?}");
    };
    assert_eq!(lock, locks::STORE_LOCK);
    assert!(
        holder.contains(&format!("run={} ", cli.run)) && holder.contains(" by=cli "),
        "{holder}"
    );
    cli.release().unwrap();

    let v4 = v4(&root);
    let cli = claim_with(&v4, template("options", "cli"), 0, None, NOW).unwrap();
    let api = claim_with(&v4, template("indices", "api"), 0, None, NOW).unwrap();
    assert_ne!(cli.run, api.run);
    assert_eq!(cli.locks.claim_name(), "imports/gdfl.options.lock");
    let again = claim_with(&v4, template("options", "api"), 0, None, NOW).unwrap_err();
    assert_eq!(again.code(), "run_busy", "{again}");
    // A census verb takes the store lock exclusive: refused while any run
    // holds it shared.
    let census = RunLocks::take_store(&v4, 0).unwrap_err();
    assert_eq!(census.code(), "run_busy");
    cli.release().unwrap();
    api.release().unwrap();
    let census = RunLocks::take_store(&v4, 0).unwrap();
    census.release().unwrap();
    assert!(
        std::fs::read(v4.join("imports/gdfl.options.lock"))
            .unwrap()
            .is_empty(),
        "a release clears its claim"
    );
}

/// **IA-GR-17 (D-4849): A PROBE NEVER MAKES A PRESS BUSY.** A probe loop
/// beside 1,000 presses, each one claim and one release with the probe
/// retry: never a `run_busy`.
#[test]
fn a_probe_never_makes_a_press_busy() {
    let store = scratch("probe");
    let stop = std::sync::atomic::AtomicBool::new(false);
    let probes = std::sync::atomic::AtomicU64::new(0);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                let _seen = locks::lock_state(&store, 0);
                probes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        });
        for press in 0..1_000 {
            let claimed = claim_with(
                &store,
                template("stocks", "api"),
                locks::PROBE_RETRY,
                None,
                NOW,
            )
            .unwrap_or_else(|why| panic!("press {press}: {why}"));
            claimed.release().unwrap();
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    assert!(probes.load(std::sync::atomic::Ordering::Relaxed) > 0);
}

/// **RA-285: A LOCK FILE REPLACED UNDER A LIVE RUN IS NAMED.** The open file
/// is compared with what the name holds: renamed away and a new file put in
/// its place (a restore, Finder), the run's locks are no longer held, and the
/// release says so.
#[test]
fn a_lock_file_replaced_under_a_live_run_is_named() {
    let store = scratch("replaced");
    let claimed = claim_with(&store, template("indices", "cli"), 0, None, NOW).unwrap();
    assert!(claimed.locks.still_held());
    let lock = store.join(locks::STORE_LOCK);
    std::fs::rename(&lock, store.join("imports/gdfl.lock.moved-aside")).unwrap();
    std::fs::write(&lock, b"").unwrap();
    assert!(!claimed.locks.still_held());
    let file = std::fs::File::open(store.join("imports/gdfl.lock.moved-aside")).unwrap();
    assert!(!locks::same_file(&file, &lock));
    claimed.release().unwrap();
}

/// A link or a second name at a lock's name is refused before a byte is
/// written (the guard's rule, now the authority's).
#[test]
fn a_linked_lock_file_is_refused_and_never_written_through() {
    let store = scratch("linked");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    let victim = store.join("victim");
    std::fs::write(&victim, b"keep me").unwrap();
    std::fs::hard_link(&victim, store.join(locks::STORE_LOCK)).unwrap();
    let got = claim_with(&store, template("indices", "cli"), 0, None, NOW).unwrap_err();
    assert_eq!(got.code(), "run_lock_refused", "{got}");
    assert_eq!(std::fs::read(&victim).unwrap(), b"keep me");
}

// ═══ The claim and the request sidecar ══════════════════════════════════════

/// **IA-GR-35 (RA-307): A CLAIM IS BOUNDED WHATEVER THE FILTER.** 256 names
/// of 24 bytes, a 64-byte host with spaces, brackets and a letter of another
/// script: the claim stays under 1 KiB, round-trips, and names the filter by
/// count and digest; the full request is in the sidecar, which verifies.
#[test]
fn a_claim_is_bounded_whatever_the_filter() {
    let store = scratch("bounded");
    let only: Vec<String> = (0..256)
        .map(|i| format!("{:0>24}", format!("N{i}")))
        .collect();
    let mut claim = template("stocks", "api");
    claim.filter = locks::filter_field(&only);
    claim.host = locks::host_field("Parthi’s MacBook Pro (2) = ☃ x".repeat(4).as_str());
    claim.exe = locks::exe_field();
    claim.auto_resume = true;
    claim.repair = locks::digest16(&[b'r'; 200]);
    let request_line = format!("only={}", only.join(","));
    let claimed = claim_with(&store, claim, 0, Some(&request_line), NOW).unwrap();
    let line = claimed.claim.line();
    assert!(line.len() <= locks::CLAIM_MAX, "{} bytes", line.len());
    assert!(claimed.claim.host.len() <= locks::HOST_MAX);
    assert!(!claimed.claim.host.contains(' ') && !claimed.claim.host.contains('='));
    assert_eq!(Claim::parse(&line).unwrap(), claimed.claim);
    assert!(claimed.claim.filter.starts_with("256:"));
    match locks::read_sidecar(&claimed.locks.sidecar_path(), &claimed.claim.request) {
        SidecarState::Verified(body) => assert_eq!(body, request_line),
        other => panic!("{other:?}"),
    }
    // A probe reads the claim, never the sidecar.
    let seen = locks::lock_state(&store, 0);
    assert!(
        seen.iter().any(|s| matches!(s, LockState::Held { claim: ClaimState::Claim(c), .. } if c.run == claimed.run)),
        "{seen:?}"
    );
    let sidecar = claimed.locks.sidecar_path();
    claimed.release().unwrap();
    assert!(!sidecar.exists(), "removed at release");
}

/// **IA-GR-47 (D-4907): A STALE OR GARBAGE CLAIM OR SIDECAR IS NAMED AND
/// NEVER RESUMED.** A dead holder's claim reads `Interrupted`; garbage in the
/// lock file reads `Unreadable`; a missing, torn, CRC-failing or foreign
/// sidecar reads `Stale`; a claim naming none removes one a dead run left.
#[test]
fn a_stale_or_garbage_claim_or_sidecar_is_named_and_never_resumed() {
    let store = scratch("stale");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    let lock = store.join(locks::STORE_LOCK);
    let mut dead = template("options", "api");
    dead.run = 1_790_000_000;
    dead.batch = "options:1790000000".to_owned();
    dead.started = NOW;
    dead.request = locks::digest16(b"another request\n");
    std::fs::write(&lock, dead.line()).unwrap();
    let seen = locks::lock_state(&store, 0);
    let [
        LockState::Interrupted {
            claim: ClaimState::Claim(found),
            stop_requested: false,
            ..
        },
    ] = seen.as_slice()
    else {
        panic!("{seen:?}");
    };
    assert_eq!(found.run, dead.run);
    let sidecar = store.join(locks::sidecar_name(locks::STORE_LOCK));
    assert!(matches!(
        locks::read_sidecar(&sidecar, &found.request),
        SidecarState::Stale(_)
    ));
    std::fs::write(&sidecar, "only=A crc=00000000\n").unwrap();
    assert!(
        matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("crc"))
    );
    std::fs::write(&sidecar, locks::sidecar_line("only=A")).unwrap();
    assert!(
        matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("digest"))
    );
    std::fs::write(&sidecar, "only=A crc=").unwrap();
    assert!(
        matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("torn"))
    );
    assert_eq!(locks::read_sidecar(&sidecar, "-"), SidecarState::None);

    std::fs::write(&lock, b"\x1b[2Jclaim run=1 mode=rm -rf\n").unwrap();
    let seen = locks::lock_state(&store, 0);
    assert!(
        matches!(seen.as_slice(), [LockState::Interrupted { claim: ClaimState::Unreadable(why), .. }] if why.contains("\\x1b")),
        "{seen:?}"
    );
    // A run whose claim names no sidecar removes the dead one.
    let claimed = claim_with(&store, template("options", "cli"), 0, None, NOW).unwrap();
    assert!(
        claimed.locks.dead().is_some(),
        "the dead claim was seen and said"
    );
    assert!(!sidecar.exists());
    claimed.release().unwrap();
}

// ═══ The start gate (IA-GR-02, the child's half) ════════════════════════════

struct Answer(Gate, Vec<(u64, ImportKind)>);

impl StartGate for Answer {
    fn claimed(&mut self, run: u64, kind: ImportKind) {
        self.1.push((run, kind));
    }

    fn wait(&mut self, _at_most: Duration) -> Gate {
        self.0
    }
}

/// **IA-GR-02 (the child's half): A CLAIMED CHILD RUNS ONLY ON `go`.** On
/// `release` or silence it truncates its claim, removes its sidecar, frees
/// its locks, reads no day, writes no journal line, and its id stays used in
/// the runs ledger; a second press meanwhile was refused naming it.
#[test]
fn a_press_with_one_kind_busy_releases_every_claimed_kind() {
    let store = scratch("gate");
    for heard in [Gate::Release, Gate::Silent] {
        let claimed =
            claim_with(&store, template("options", "api"), 0, Some("only=X"), NOW).unwrap();
        let run = claimed.run;
        let busy = claim_with(&store, template("options", "api"), 0, None, NOW).unwrap_err();
        assert!(
            matches!(&busy, RunRefusal::RunBusy { holder, .. } if holder.contains(&format!("run={run} ")))
        );
        let sidecar = claimed.locks.sidecar_path();
        let mut gate = Answer(heard, Vec::new());
        let released = await_go_within(claimed, &mut gate, Duration::from_millis(1)).unwrap_err();
        assert_eq!(
            (released.run, released.heard, released.fault),
            (run, heard, None)
        );
        assert_eq!(gate.1, vec![(run, ImportKind::Options)]);
        assert!(
            std::fs::read(store.join(locks::STORE_LOCK))
                .unwrap()
                .is_empty()
        );
        assert!(!sidecar.exists());
        assert!(
            !crate::gdfl_import::journal_path(&store).exists(),
            "no journal line"
        );
        let ids: Vec<u64> = ledger::read_all(&store)
            .unwrap()
            .into_iter()
            .map(|e| e.unwrap().id)
            .collect();
        assert!(ids.contains(&run), "the id stays in the ledger");
    }
    let claimed = claim_with(&store, template("options", "api"), 0, None, NOW).unwrap();
    let mut gate = Answer(Gate::Go, Vec::new());
    let running = await_go_within(claimed, &mut gate, Duration::from_millis(1)).unwrap();
    assert!(
        !std::fs::read(store.join(locks::STORE_LOCK))
            .unwrap()
            .is_empty(),
        "go keeps the claim"
    );
    running.release().unwrap();
}

// ═══ The stop request (IA-GR-21) ════════════════════════════════════════════

/// **IA-GR-21: A CLI RUN IS NEVER AUTO-RESUMED AND CAN BE STOPPED BY NAME.**
/// A cli claim says `auto_resume=0` even when asked otherwise (D-4804: a cli run is never auto-resumed);
/// `request_stop` by kind or by run writes the stop file the run's own stop
/// sees at its next file boundary; a second stop keeps the first asker; a
/// stop of a run that is not live is refused by name and writes nothing.
#[test]
fn a_cli_run_is_never_auto_resumed_and_can_be_stopped_by_name() {
    let store = scratch("stop");
    let plan_claim = {
        let mut claim = template("indices", "cli");
        claim.auto_resume = false;
        claim
    };
    let claimed = claim_with(&store, plan_claim, 0, None, NOW).unwrap();
    assert!(claimed.claim.line().contains(" auto_resume=0 "));
    assert!(!claimed.stop.requested());
    let answer = stop::request_stop(&store, StopTarget::Kind("indices"), "cli", NOW + 5).unwrap();
    assert_eq!((answer.run, answer.held_by.as_str()), (claimed.run, "cli"));
    assert!(claimed.stop.requested(), "seen at the next boundary");
    assert_eq!(claimed.stop.request().unwrap().by, "cli");
    let second = stop::request_stop(&store, StopTarget::Run(claimed.run), "api", NOW + 9).unwrap();
    assert_eq!(
        (second.request.by.as_str(), second.request.requested),
        ("cli", NOW + 5),
        "the first asker is kept"
    );
    let stop_file = claimed.locks.stop_path();
    assert!(stop_file.exists());
    claimed.release().unwrap();
    assert!(!stop_file.exists(), "removed at release");
    let none = stop::request_stop(&store, StopTarget::Kind("indices"), "cli", NOW).unwrap_err();
    assert_eq!(none.code(), "run_not_live");
    assert!(!stop_file.exists(), "nothing written");
    let bad = stop::request_stop(&store, StopTarget::Kind("indices"), "someone", NOW).unwrap_err();
    assert_eq!(bad.code(), "stop_by_unknown");

    // The template a plan builds: a cli claim is never auto-resumed.
    let ask = ClaimAsk {
        by: By::Cli,
        pid: 1,
        host: "",
        resumes: 0,
        resume_day: None,
        auto_resume: true,
        legacy_ack: false,
        unstamped: false,
        retry: 0,
    };
    let ts = scratch("stop-ts");
    std::fs::write(ts.join("FORMAT.md"), "invented").unwrap();
    let request = Request::new(
        Kinds::of(&[ImportKind::Indices]).unwrap(),
        SourceWord::TickStore,
        (Day::new(2024, 4, 1).unwrap(), Day::new(2024, 4, 1).unwrap()),
    );
    let log = LogState::Installed(store.clone());
    let process = Process {
        store_root: &store,
        commit: Some("feedc0de"),
        log: &log,
        unstamped: None,
    };
    let env = |var: &str| (var == TICKSTORE_VAR).then(|| ts.clone().into_os_string());
    let planned = plan(request, &process, &env, &mut SourceCache::default()).unwrap();
    // A stopped run's progress record says `stopped`, and a stop by the
    // server's shutdown `shutdown`: never `refused`.
    for (by, state) in [("cli", RunState::Stopped), ("shutdown", RunState::Shutdown)] {
        let progress = ProgressFile::new(
            store.join(format!("imports/p-{by}")),
            &planned,
            &planned.kinds[0],
            3,
        );
        let stopped = crate::gdfl_import::ImportRefusal::Stopped {
            kind: ImportKind::Indices,
            day: None,
            by: by.to_owned(),
            requested: NOW,
            done: Box::default(),
        };
        crate::gdfl_import::Observer::finished(&progress, ImportKind::Indices, Err(&stopped));
        assert_eq!(
            Progress::read(progress.path()).unwrap().state,
            state.code(),
            "{by}"
        );
    }
    let made = claim_template(&planned, &planned.kinds[0], &ask);
    assert!(!made.auto_resume);
    assert_eq!(
        (made.host.as_str(), made.source.as_str()),
        ("-", "tickstore")
    );
}

/// **C-GR-07, C-GR-08 and C-GR-04 timings** (run with `--ignored
/// --nocapture`): the stop check at a file boundary with and without a stop
/// file, a run id allocated from a ledger of 10,000 records (and with a
/// garbage tail), and a journal-lock take uncontended. Printed for
/// `docs/06-limits.md` and the package report, never asserted: a timing
/// on a shared machine is evidence, not a gate.
#[test]
#[ignore = "timings, printed"]
fn ia2_timings() {
    fn pct(mut v: Vec<Duration>) -> String {
        v.sort_unstable();
        let n = v.len();
        format!(
            "n={n} p50={:?} p99={:?} max={:?}",
            v[n / 2],
            v[n * 99 / 100],
            v[n - 1]
        )
    }
    let store = scratch("timings");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    let control = stop::StopControl::new(store.join("imports/gdfl.stop"), 1);
    let mut took = Vec::new();
    for _ in 0..10_000 {
        let began = std::time::Instant::now();
        let _seen = control.requested();
        took.push(began.elapsed());
    }
    println!("C-GR-07 stop check, no stop file: {}", pct(took));
    let other = stop::StopRequest {
        run: 2,
        requested: NOW,
        by: "cli".to_owned(),
    };
    stop::write_stop(&store.join("imports/gdfl.stop"), &other).unwrap();
    let mut took = Vec::new();
    for _ in 0..10_000 {
        let began = std::time::Instant::now();
        let _seen = control.requested();
        took.push(began.elapsed());
    }
    println!("C-GR-07 stop check, another run's stop file: {}", pct(took));
    let mut took = Vec::new();
    for _ in 0..2_000 {
        let began = std::time::Instant::now();
        allocate(&store, NOW).unwrap();
        took.push(began.elapsed());
    }
    println!(
        "C-GR-08 run id allocation (ledger growing to 2,001 records): {}",
        pct(took)
    );
    let path = store.join(ledger::LEDGER);
    let mut took = Vec::new();
    for _ in 0..200 {
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.extend_from_slice(&[b'x'; 77]);
        std::fs::write(&path, &bytes).unwrap();
        let began = std::time::Instant::now();
        allocate(&store, NOW).unwrap();
        took.push(began.elapsed());
    }
    println!("C-GR-08 run id allocation over a torn tail: {}", pct(took));
    let lock = crate::gdfl_import::journal_lock_path(&crate::gdfl_import::journal_path(&store));
    let mut took = Vec::new();
    for _ in 0..2_000 {
        let began = std::time::Instant::now();
        drop(locks::take_path_within(&lock, locks::LOCK_WAIT).unwrap());
        took.push(began.elapsed());
    }
    println!(
        "C-GR-04 journal lock take and release, uncontended: {}",
        pct(took)
    );
    let mut took = Vec::new();
    for _ in 0..500 {
        let began = std::time::Instant::now();
        let claimed = claim_with(
            &store,
            template("options", "cli"),
            locks::PROBE_RETRY,
            None,
            NOW,
        )
        .unwrap();
        claimed.release().unwrap();
        took.push(began.elapsed());
    }
    println!(
        "C-GR-01 claim (locks, id, claim) and release, one kind: {}",
        pct(took)
    );
    let held = claim_with(&store, template("options", "cli"), 0, None, NOW).unwrap();
    let mut took = Vec::new();
    for _ in 0..2_000 {
        let began = std::time::Instant::now();
        let seen = locks::lock_state(&store, 0);
        took.push(began.elapsed());
        assert!(!seen.is_empty());
    }
    println!("lock_state probe, one held lock: {}", pct(took));
    held.release().unwrap();
}

// ═══ The census verbs (IA-GR-14) ════════════════════════════════════════════

/// **IA-GR-14 (FM-058): THE CENSUS VERBS SHARE THE IMPORT LOCKS AND THE ONE
/// VERB TABLE.** `census-recount` and `census-migrate` (and `--seal`) refuse
/// `run_busy` naming the run while a GDFL run holds the store, write nothing,
/// and take their id from the runs ledger once free; a probe's shared hold
/// never makes them refuse (round 4, N-4); the census reader reads the
/// importer's verbs, `run` lines with `ended=` and `stage=run` closes.
#[test]
fn the_census_and_dayflags_verbs_share_the_import_locks_and_verbs() {
    use crate::ingest::census_store;
    let store = scratch("census-verbs");
    let span = census_store::Span {
        from: Day::new(2024, 4, 1).unwrap(),
        to: Some(Day::new(2024, 4, 30).unwrap()),
    };
    let run = claim_with(&store, template("options", "cli"), 0, None, NOW).unwrap();
    let busy = census_store::recount(&store, Vendor::Gdfl, span, None).unwrap_err();
    assert!(
        busy.starts_with("run_busy: ")
            && busy.contains(&format!("run={} ", run.run))
            && busy.ends_with("Nothing was written"),
        "{busy}"
    );
    let busy = census_store::migrate(&store, Vendor::Gdfl).unwrap_err();
    assert!(busy.starts_with("run_busy: "), "{busy}");
    let busy = census_store::seal_legacy(&store, Vendor::Gdfl).unwrap_err();
    assert!(busy.starts_with("run_busy: "), "{busy}");
    run.release().unwrap();

    // A probe's shared hold, released within the retry, never refuses it.
    let prober = std::fs::File::open(store.join(locks::STORE_LOCK)).unwrap();
    let shared =
        store::flock::Flock::try_lock_shared(prober, store.join(locks::STORE_LOCK)).unwrap();
    let released = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(30));
        drop(shared);
    });
    let hold = claim_census(&store, "census-recount").unwrap();
    released.join().unwrap();
    let claim = std::fs::read_to_string(store.join(locks::STORE_LOCK)).unwrap();
    assert!(
        claim.contains(" mode=census-recount kind=census ") && claim.contains(" by=cli "),
        "{claim}"
    );
    hold.release().unwrap();

    // The recount itself: its id is the ledger's.
    let before = ledger::read_all(&store).unwrap().len();
    census_store::recount(&store, Vendor::Gdfl, span, None).unwrap();
    let all = ledger::read_all(&store).unwrap();
    assert_eq!(all.len(), before + 1);
    assert_eq!(all.last().unwrap().as_ref().unwrap().mode, "census-recount");
    assert!(
        std::fs::read(store.join(locks::STORE_LOCK))
            .unwrap()
            .is_empty()
    );

    // One verb table: the census reader reads what the importer writes.
    assert_eq!(
        crate::gdfl_import::VERB_WORDS,
        ["begin", "done", "incomplete", "refused", "run", "recounted"]
    );
}

/// **D-4893 (X-5): EVERY STATE CHANGE IA-2 EMITS IS AUDITED.** A table, not a
/// claim: each event this package adds, at its level and target, is kept in
/// the audit ledger for ever (`telemetry::is_audited`), so who asked a stop,
/// which run claimed or superseded a lock, a release before start, a stalled
/// or replaced lock, a damaged ledger and a wrong clock are searchable after
/// the rolling window has moved on.
#[test]
fn every_event_ia2_adds_is_audited() {
    use telemetry::Level;
    let lifecycle = locks::TARGET;
    for (level, target, message) in [
        (Level::Info, lifecycle, "stop requested"),
        (Level::Warn, lifecycle, "run stopped"),
        (Level::Info, lifecycle, "run claimed"),
        (Level::Warn, lifecycle, "run superseded"),
        (Level::Info, lifecycle, "run released before start"),
        (Level::Error, lifecycle, "lock stalled"),
        (Level::Error, lifecycle, "lock replaced under a live run"),
        (Level::Error, lifecycle, "runs ledger damaged"),
        (Level::Error, lifecycle, "runs ledger alert not put"),
        (Level::Error, lifecycle, "clock implausible"),
        (
            Level::Error,
            crate::gdfl_import::TARGET,
            "run end line not written",
        ),
        (
            Level::Error,
            crate::gdfl_import::TARGET,
            "journal shorter than this run last knew it",
        ),
    ] {
        assert!(
            telemetry::is_audited(level, target, message),
            "{level:?} {target} {message:?}"
        );
    }
    assert_eq!(lifecycle, "gdfl.import.lifecycle");
}

// ═══ Repair round 1 (18 Oct): the verifier's four blocking findings ═════════

/// **N ASKERS AT ONCE: EVERY ONE IS ANSWERED WITH THE ONE STOP LINE, AND THE
/// FIRST IS KEPT** (IA-2 repair 1; RA-336, IA-GR-21). Twenty-four askers of
/// one live run, released together by a barrier, in ten rounds: none is
/// refused, all are answered with the same line, the file holds that line,
/// and no staging or aside name is left. Then with a stop file of another
/// (ended) run already there, the same: it is replaced once, never twice.
/// Before the repair every asker staged under one shared name, so one
/// asker's rename took another's staging file (`stop_unwritten`, ENOENT) and
/// a later rename replaced the first line.
#[test]
fn concurrent_stop_askers_are_all_answered_with_the_first_line() {
    const ASKERS: u64 = 24;
    let store = scratch("stop-race");
    let imports = store.join("imports");
    std::fs::create_dir_all(&imports).unwrap();
    let path = imports.join("gdfl.options.stop");
    for round in 0..10_u64 {
        let _fresh = std::fs::remove_file(&path);
        if round % 2 == 1 {
            let ended = stop::StopRequest {
                run: 7,
                requested: NOW,
                by: "cli".to_owned(),
            };
            stop::write_stop(&path, &ended).unwrap();
        }
        let barrier = std::sync::Barrier::new(usize::try_from(ASKERS).unwrap());
        let answers: Vec<Result<stop::StopRequest, String>> = std::thread::scope(|scope| {
            let askers: Vec<_> = (0..ASKERS)
                .map(|asker| {
                    let (barrier, path) = (&barrier, &path);
                    scope.spawn(move || {
                        let request = stop::StopRequest {
                            run: 1_790_000_900,
                            requested: NOW + asker,
                            by: if asker % 2 == 0 { "cli" } else { "api" }.to_owned(),
                        };
                        barrier.wait();
                        stop::write_stop(path, &request)
                    })
                })
                .collect();
            askers.into_iter().map(|one| one.join().unwrap()).collect()
        });
        let first = answers[0]
            .clone()
            .unwrap_or_else(|why| panic!("round {round}: {why}"));
        for answer in &answers {
            assert_eq!(
                answer.as_ref(),
                Ok(&first),
                "round {round}: every asker, one line"
            );
        }
        assert_eq!(first.run, 1_790_000_900, "round {round}");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            first.line(),
            "round {round}: the file holds the line every asker was told"
        );
        let mut left: Vec<String> = std::fs::read_dir(&imports)
            .unwrap()
            .map(|one| one.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            ["gdfl.journal.lock", "gdfl.options.stop"],
            "round {round}: no staging name is left"
        );
    }
}

/// **A CENSUS VERB BESIDE V4 RUNS NAMES EVERY HOLDER, NEVER "CLAIM BEING
/// WRITTEN"** (IA-2 repair 1; FM-058, IA-GR-14). A v4 run holds the store
/// lock shared and its claim lives in its kind lock, so the store lock's
/// own text is empty: the refusal of `census-recount` and `--seal` (both
/// `claim_census`) reads the kind locks and names each live run's run, kind,
/// pid, by and start. Nothing is allocated from the runs ledger on the
/// refusal. With no kind claim written, the words say no claim names the
/// holder yet, never a guessed one.
#[test]
fn a_census_verb_beside_v4_runs_names_every_holder() {
    let root = scratch("census-v4-holder");
    let v4 = v4(&root);
    let options = claim_with(&v4, template("options", "cli"), 0, None, NOW).unwrap();
    let indices = claim_with(&v4, template("indices", "api"), 0, None, NOW).unwrap();
    let ledger_len = || std::fs::metadata(v4.join(ledger::LEDGER)).unwrap().len();
    let before = ledger_len();
    for mode in ["census-recount", "census-seal"] {
        let refused = claim_census(&v4, mode).unwrap_err();
        assert_eq!(refused.code(), "run_busy", "{refused}");
        let said = refused.to_string();
        assert!(!said.contains("being written"), "{said}");
        for (claimed, kind, by) in [(&options, "options", "cli"), (&indices, "indices", "api")] {
            assert!(
                said.contains(&format!(
                    "imports/gdfl.{kind}.lock: run={} kind={kind} pid=4242 by={by} started={NOW}",
                    claimed.run
                )),
                "{mode}: {said}"
            );
        }
    }
    assert_eq!(ledger_len(), before, "a refused census verb takes no id");
    options.release().unwrap();
    indices.release().unwrap();
    // A holder whose claim is not written: the store lock held shared by a
    // process that has no kind claim (a run between its two locks).
    let file = locks::open_lock_file(&v4, locks::STORE_LOCK).unwrap();
    let shared = store::flock::Flock::try_lock_shared(file, v4.join(locks::STORE_LOCK)).unwrap();
    let unnamed = RunLocks::take_store(&v4, 0).unwrap_err();
    assert!(
        matches!(&unnamed, LockFault::Busy { holder, .. } if holder.is_empty()),
        "{unnamed:?}"
    );
    assert!(
        unnamed.to_string().contains("no claim names it yet"),
        "{unnamed}"
    );
    drop(shared);
    RunLocks::take_store(&v4, 0).unwrap().release().unwrap();
}

/// **A HELD LOCK IS NAMED BEFORE THE SINK'S REFUSAL** (IA-2 repair 1;
/// FM-064). A run logs in the directory its claim lock makes it alone in, so
/// a second run of a held lock finds the sink taken before it tries any
/// lock. The plan probes the lock read-only and refuses `run_busy` naming
/// the holder's claim; with no holder the sink's own `log_unavailable`
/// stands. On v4 the same kind, on v3 any second kind; the probe takes
/// nothing, so the holder keeps its lock and no lock file is made.
#[test]
fn a_held_lock_is_named_before_the_sink_refusal() {
    let root = scratch("held-before-sink");
    let v3 = root.join("v3");
    std::fs::create_dir_all(&v3).unwrap();
    let v4 = v4(&root);
    let ts = root.join("ts");
    std::fs::create_dir_all(&ts).unwrap();
    std::fs::write(ts.join("FORMAT.md"), "invented").unwrap();
    let refused_sink = LogState::Refused("the sink is held by another process".to_owned());
    let plan_of = |store: &Path, kind: ImportKind| {
        let request = Request::new(
            Kinds::of(&[kind]).unwrap(),
            SourceWord::TickStore,
            (Day::new(2024, 4, 1).unwrap(), Day::new(2024, 4, 1).unwrap()),
        );
        let process = Process {
            store_root: store,
            commit: Some("feedc0de"),
            log: &refused_sink,
            unstamped: None,
        };
        let env = |var: &str| (var == TICKSTORE_VAR).then(|| ts.clone().into_os_string());
        plan(request, &process, &env, &mut SourceCache::default())
    };
    for (store, held, asked) in [
        (&v4, "options", ImportKind::Options),
        (&v3, "options", ImportKind::Indices),
    ] {
        let unheld = plan_of(store, asked).unwrap_err();
        assert_eq!(unheld.code(), "log_unavailable", "{unheld}");
        assert!(
            !store.join("imports/gdfl.indices.lock").exists(),
            "a probe makes no lock file"
        );
        let holder = claim_with(store, template(held, "cli"), 0, None, NOW).unwrap();
        let busy = plan_of(store, asked).unwrap_err();
        let RunRefusal::RunBusy {
            lock,
            holder: named,
        } = &busy
        else {
            panic!("{busy:?}");
        };
        assert_eq!(lock, holder.locks.claim_name(), "{busy}");
        assert!(
            named.contains(&format!("run={} ", holder.run))
                && named.contains(" pid=4242 ")
                && named.contains(" by=cli "),
            "{named}"
        );
        assert!(
            holder.locks.still_held(),
            "the probe took nothing from the holder"
        );
        holder.release().unwrap();
        assert_eq!(plan_of(store, asked).unwrap_err().code(), "log_unavailable");
    }
}

/// **A CENSUS VERB IS NEVER PROMISED A STOP** (IA-2 repair 1): a census verb
/// reads no stop file, so `gdfl-stop --run <its id>` is refused
/// `run_not_stoppable` naming it, and no stop file is written; a stop by
/// kind never takes the census verb for a run of that kind.
#[test]
fn a_census_verb_is_never_promised_a_stop() {
    let store = scratch("stop-census");
    let census = claim_census(&store, "census-recount").unwrap();
    let refused =
        stop::request_stop(&store, StopTarget::Run(census.run), "cli", NOW + 1).unwrap_err();
    assert_eq!(refused.code(), "run_not_stoppable", "{refused}");
    assert!(
        refused.to_string().contains(&format!(
            "run {} is a census verb (census-recount)",
            census.run
        )),
        "{refused}"
    );
    let by_kind = stop::request_stop(&store, StopTarget::Kind("options"), "cli", NOW).unwrap_err();
    assert_eq!(by_kind.code(), "run_not_live", "{by_kind}");
    assert!(
        !store.join(locks::stop_name(locks::STORE_LOCK)).exists(),
        "no stop file promised to a verb that reads none"
    );
    census.release().unwrap();
}

/// **THE LAST ID THERE IS IS NEVER HANDED OUT TWICE** (IA-2 repair 1): a
/// journal whose largest run id is `u64::MAX` seeds a ledger with no later
/// id; the allocation is refused by name and writes nothing, where it once
/// answered `u64::MAX` to every run.
#[test]
fn a_saturated_runs_ledger_refuses_by_name() {
    let store = scratch("ledger-saturated");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    std::fs::write(
        store.join("imports/gdfl.journal"),
        format!("run {} source=zip started=1\n", u64::MAX),
    )
    .unwrap();
    for _ in 0..2 {
        let refused = allocate(&store, NOW).unwrap_err();
        assert_eq!(refused.code(), "runs_ledger_entry", "{refused}");
        assert!(
            refused.to_string().contains("the largest an id can be"),
            "{refused}"
        );
    }
    assert_eq!(
        std::fs::metadata(store.join(ledger::LEDGER)).map_or(0, |m| m.len()),
        0,
        "nothing was written"
    );
}

/// **REPAIR 1'S OPERATIONS, TIMED** (IA-2 repair 1): a stop written under
/// the journal lock (stage, sync, rename, directory sync), a stop asked
/// again when the file already names the run (one read), a census verb
/// refused beside three v4 runs (the store lock tried, three kind locks
/// probed), and a plan refused before its sink with its lock held (one
/// probe). Printed for `docs/06-limits.md` and the package report, never
/// asserted: a timing on a shared machine is evidence, not a gate.
#[test]
#[ignore = "timings, printed"]
fn ia2_repair_timings() {
    fn pct(mut v: Vec<Duration>) -> String {
        v.sort_unstable();
        let n = v.len();
        format!(
            "n={n} p50={:?} p99={:?} max={:?}",
            v[n / 2],
            v[n * 99 / 100],
            v[n - 1]
        )
    }
    let root = scratch("repair-timings");
    let v4 = v4(&root);
    std::fs::create_dir_all(v4.join("imports")).unwrap();
    let path = v4.join("imports/gdfl.options.stop");
    let mut took = Vec::new();
    for at in 0..500_u64 {
        let _fresh = std::fs::remove_file(&path);
        let request = stop::StopRequest {
            run: 9,
            requested: NOW + at,
            by: "cli".to_owned(),
        };
        let began = std::time::Instant::now();
        stop::write_stop(&path, &request).unwrap();
        took.push(began.elapsed());
    }
    println!(
        "stop written (lock, stage, sync, rename, dir sync): {}",
        pct(took)
    );
    let again = stop::StopRequest {
        run: 9,
        requested: NOW,
        by: "api".to_owned(),
    };
    let mut took = Vec::new();
    for _ in 0..10_000 {
        let began = std::time::Instant::now();
        let _first = stop::write_stop(&path, &again).unwrap();
        took.push(began.elapsed());
    }
    println!("stop asked again, file names the run: {}", pct(took));
    let _fresh = std::fs::remove_file(&path);
    let held: Vec<_> = ["indices", "stocks", "options"]
        .into_iter()
        .map(|kind| claim_with(&v4, template(kind, "cli"), 0, None, NOW).unwrap())
        .collect();
    let mut took = Vec::new();
    for _ in 0..2_000 {
        let began = std::time::Instant::now();
        let refused = RunLocks::take_store(&v4, 0).unwrap_err();
        took.push(began.elapsed());
        assert!(matches!(refused, LockFault::Busy { .. }));
    }
    println!(
        "census verb refused beside 3 v4 runs, holders named: {}",
        pct(took)
    );
    let refused_sink = LogState::Refused("held".to_owned());
    let mut took = Vec::new();
    for _ in 0..2_000 {
        let request = Request::new(
            Kinds::of(&[ImportKind::Options]).unwrap(),
            SourceWord::TickStore,
            (Day::new(2024, 4, 1).unwrap(), Day::new(2024, 4, 1).unwrap()),
        );
        let process = Process {
            store_root: &v4,
            commit: Some("feedc0de"),
            log: &refused_sink,
            unstamped: None,
        };
        let began = std::time::Instant::now();
        let refused = plan(request, &process, &|_| None, &mut SourceCache::default()).unwrap_err();
        took.push(began.elapsed());
        assert_eq!(refused.code(), "run_busy");
    }
    println!("plan refused before its sink, holder named: {}", pct(took));
    for one in held {
        one.release().unwrap();
    }
}

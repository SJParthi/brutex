//! `/disk.json` names a migrated census that was never sealed (AG-05,
//! assurance finding store-2; DB-B68, D-3381).
//!
//! The operator's GDFL census was migrated on 2026-10-10, before the seal
//! existed: `manifest/gdfl.man` still holds the whole version-3 census the
//! migration froze, with no seal receipt and no `manifest/REQUIRES`, and
//! `/disk.json` labelled it `superseded` and said nothing more. A reader
//! built before version 4 opens exactly that name. Each store here is built
//! by the real writers (`census_store::record`, `migrate`, which seals) and
//! then put into the state under test; every row is invented.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::ingest::census_store;
use pull::manifest::{Closes, Entry, EntryKey, Held, manifest_path, vendor_dir};
use store::path::{Timeframe, YearMonth};

use super::{census_files, census_view, measured};

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = crate::scratch::path(&format!(
            "disk-seal-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(root.join("manifest")).expect("a scratch store");
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn held(n: u32) -> Held {
    Held::new(
        Entry {
            key: EntryKey {
                contract: Contract::parse(&format!("2019-12-26-{}-CE", 1_000_000 + n * 100)),
                exchange: Exchange::Nse,
                segment: Segment::Fno,
                symbol: Symbol::new("NIFTY").expect("a symbol"),
                timeframe: Timeframe::SECOND_1,
                month: YearMonth::new(2019, 11).expect("a month"),
            },
            rows: u64::from(n) + 1,
            first_ts_micros: 10,
            last_ts_micros: 20,
        },
        Closes::UNKNOWN,
    )
}

/// A GDFL census recorded as a single file and migrated, which seals it.
fn sealed(root: &Path) {
    census_store::record(root, Vendor::Gdfl, &[held(0), held(1)]).expect("recorded");
    let done = census_store::migrate(root, Vendor::Gdfl).expect("migrated");
    assert!(done.seal.as_ref().is_some_and(Result::is_ok), "and sealed");
}

/// The operator's store as it is: migrated before the seal existed, the
/// census back at its name, no superseded file, no receipt, no REQUIRES.
/// Returns the census's bytes.
fn as_migrated_before_the_seal(root: &Path) -> Vec<u8> {
    sealed(root);
    let legacy = manifest_path(root, Vendor::Gdfl);
    let kept = census_store::superseded_path(root, Vendor::Gdfl);
    let bytes = std::fs::read(&kept).expect("the superseded census");
    std::fs::remove_file(&legacy).expect("the stub removed");
    std::fs::rename(&kept, &legacy).expect("the census back at its name");
    std::fs::remove_file(receipt(root)).expect("no receipt");
    std::fs::remove_file(census_store::requires_path(root)).expect("no REQUIRES");
    bytes
}

fn receipt(root: &Path) -> PathBuf {
    vendor_dir(root, Vendor::Gdfl).join(census_store::SEAL_RECEIPT)
}

fn answer(root: &Path) -> serde_json::Value {
    serde_json::from_str(&measured(root, None)).expect("JSON")
}

fn role(root: &Path, name: &str) -> Option<(&'static str, u64)> {
    census_files(root, Vendor::Gdfl)
        .iter()
        .find(|f| f.path.file_name().and_then(|n| n.to_str()) == Some(name))
        .map(|f| (f.role, f.bytes))
}

/// **A MIGRATED CENSUS WITH NO SEAL RECEIPT IS `unsealed` AND ITS FAULT IS
/// NAMED** with the step that closes it, and whether `manifest/REQUIRES`
/// exists. Before the fix the file was `superseded` and the answer carried
/// no `faults` at all.
#[test]
fn disk_json_names_a_migrated_unsealed_census() {
    let scratch = Scratch::new("unsealed");
    let root = &scratch.0;
    let census = as_migrated_before_the_seal(root);
    let bytes = u64::try_from(census.len()).expect("len");
    assert_eq!(role(root, "gdfl.man"), Some(("unsealed", bytes)));
    assert_eq!(role(root, "gdfl.man.v3-superseded"), None, "none yet");

    let body = answer(root);
    let feed = &body["feeds"][0];
    assert_eq!(feed["feed"], "gdfl");
    assert_eq!(feed["seal"]["migrated"], true);
    assert_eq!(feed["seal"]["receipt"], "absent");
    assert_eq!(feed["seal"]["requires"], false);
    assert_eq!(feed["seal"]["fault"], "census_unsealed");
    let faults = body["faults"].as_array().expect("faults");
    assert_eq!(faults.len(), 1, "{body}");
    assert_eq!(faults[0]["feed"], "gdfl");
    assert_eq!(faults[0]["fault"], "census_unsealed");
    let why = faults[0]["why"].as_str().expect("why");
    for said in [
        "census gdfl: migrated but not sealed; a pre-version-4 reader would serve the \
         frozen v3 census",
        "run `cli census-migrate gdfl --seal` after a snapshot (docs/36 section 2)",
        "REQUIRES is absent",
        "gdfl.man",
    ] {
        assert!(why.contains(said), "{said:?} in {why}");
    }
    // The frozen census is retention: its bytes are counted as kept.
    let kept = feed["census_kept_bytes"].as_u64().expect("kept");
    assert!(kept >= bytes, "{kept} counts the {bytes} unsealed bytes");

    // A TORN RECEIPT IS NO SEAL: unsealed, and the fault says it is torn.
    std::fs::write(receipt(root), "format=census-se").expect("torn");
    assert_eq!(role(root, "gdfl.man"), Some(("unsealed", bytes)));
    let body = answer(root);
    assert_eq!(body["feeds"][0]["seal"]["receipt"], "torn");
    let why = body["faults"][0]["why"].as_str().expect("why");
    assert!(why.contains("is torn"), "{why}");
    assert!(why.contains("migrated but not sealed"), "{why}");
    std::fs::remove_file(receipt(root)).expect("removed");

    // A 0-BYTE SINGLE FILE, migrated and unsealed: still named.
    std::fs::write(manifest_path(root, Vendor::Gdfl), b"").expect("0 bytes");
    assert_eq!(role(root, "gdfl.man"), Some(("unsealed", 0)));
    assert_eq!(answer(root)["faults"][0]["fault"], "census_unsealed");
    // NO SINGLE FILE AT ALL, migrated and unsealed: named, in other words.
    std::fs::remove_file(manifest_path(root, Vendor::Gdfl)).expect("removed");
    let body = answer(root);
    let why = body["faults"][0]["why"].as_str().expect("why");
    assert!(why.contains("finds no census at all"), "{why}");
}

/// **A SEALED CENSUS READS AS `sealed-stub`, UNCHANGED, WITH NO FAULT**; and a
/// seal interrupted after its stub (no receipt yet) is named as interrupted,
/// not as a census a pre-version-4 reader would serve.
#[test]
fn disk_json_reads_a_sealed_census_as_sealed_stub() {
    let scratch = Scratch::new("sealed");
    let root = &scratch.0;
    sealed(root);
    let stub = u64::try_from(census_store::sealed_stub(Vendor::Gdfl).len()).expect("len");
    assert_eq!(role(root, "gdfl.man"), Some(("sealed-stub", stub)));
    assert!(matches!(
        role(root, "gdfl.man.v3-superseded"),
        Some(("superseded", _))
    ));
    let body = answer(root);
    assert_eq!(body["feeds"][0]["seal"]["receipt"], "whole");
    assert_eq!(body["feeds"][0]["seal"]["requires"], true);
    assert!(body["feeds"][0]["seal"]["fault"].is_null());
    assert_eq!(body["faults"], serde_json::json!([]), "{body}");

    // Interrupted after the stub: the receipt is what is missing.
    std::fs::remove_file(receipt(root)).expect("no receipt");
    assert_eq!(role(root, "gdfl.man"), Some(("unsealed", stub)));
    let why = answer(root)["faults"][0]["why"]
        .as_str()
        .expect("why")
        .to_owned();
    assert!(why.contains("a seal was interrupted"), "{why}");
    assert!(why.contains("gdfl.man.v3-superseded keeps the v3"), "{why}");
    assert!(!why.contains("would serve"), "{why}");
}

/// The fault's sentence of a store's one feed, and that it is `word`.
fn fault_said(root: &Path, word: &str) -> String {
    let body = answer(root);
    assert_eq!(body["feeds"][0]["seal"]["fault"], word, "{body}");
    assert_eq!(body["faults"][0]["fault"], word, "{body}");
    body["faults"][0]["why"].as_str().expect("why").to_owned()
}

/// **A WHOLE SEAL RECEIPT IS NEVER TAKEN AT ITS WORD** (AG-05 repair 1): the
/// single name is `sealed-stub` only when the receipt is whole, the name
/// holds the stub and the superseded census is a regular file. The second
/// agent's state — the census renamed back to `gdfl.man` with `SEALED` and
/// `REQUIRES` left in place — read `sealed-stub` with no fault and dropped
/// the census from `census_kept_bytes`: exactly the store a pre-version-4
/// reader serves, reported as sealed. Every other way the two names can
/// disagree with a whole receipt is named the same way, and the stub put back
/// reads as sealed again.
#[test]
fn a_whole_receipt_without_the_stub_or_the_kept_census_is_seal_inconsistent() {
    let scratch = Scratch::new("inconsistent");
    let root = &scratch.0;
    sealed(root);
    let legacy = manifest_path(root, Vendor::Gdfl);
    let kept = census_store::superseded_path(root, Vendor::Gdfl);
    let aside = root.join("manifest").join("gdfl.man.aside");
    let stub_image = census_store::sealed_stub(Vendor::Gdfl);
    let stub = u64::try_from(stub_image.len()).expect("len");
    let census = u64::try_from(std::fs::read(&kept).expect("the kept census").len()).expect("len");
    let kept_sealed = answer(root)["census_kept_bytes"].as_u64().expect("kept");

    // THE SECOND AGENT'S STATE: the census back at its name, the receipt kept.
    std::fs::remove_file(&legacy).expect("the stub removed");
    std::fs::rename(&kept, &legacy).expect("the census back at its name");
    assert!(receipt(root).is_file() && census_store::requires_path(root).is_file());
    assert_eq!(role(root, "gdfl.man"), Some(("seal-inconsistent", census)));
    let body = answer(root);
    let seal = &body["feeds"][0]["seal"];
    assert_eq!(seal["receipt"], "whole", "{body}");
    assert_eq!(seal["stub"], false, "{body}");
    assert_eq!(seal["superseded"], false, "{body}");
    assert_eq!(seal["requires"], true, "{body}");
    assert_eq!(
        body["census_kept_bytes"].as_u64(),
        Some(kept_sealed),
        "the census at its name is kept bytes, not dropped: {body}"
    );
    let why = fault_said(root, "census_seal_inconsistent");
    for said in [
        "seal receipt",
        "is whole, but",
        &format!("gdfl.man holds {census} bytes, not the seal's stub"),
        "a pre-version-4 reader serves what it holds",
        "gdfl.man.v3-superseded is absent",
        "claims a seal the store does not have",
        "`cli census-migrate gdfl --seal` after a snapshot",
        "REQUIRES exists",
    ] {
        assert!(why.contains(said), "{said:?} in {why}");
    }

    // THE STUB AT THE NAME, THE KEPT CENSUS GONE.
    std::fs::rename(&legacy, &aside).expect("the census aside");
    std::fs::write(&legacy, &stub_image).expect("the stub back");
    assert_eq!(role(root, "gdfl.man"), Some(("seal-inconsistent", stub)));
    let why = fault_said(root, "census_seal_inconsistent");
    assert!(why.contains("holds the seal's stub"), "{why}");
    assert!(why.contains("v3-superseded is absent"), "{why}");
    assert_eq!(answer(root)["feeds"][0]["seal"]["stub"], true);

    // A DIRECTORY WHERE THE KEPT CENSUS BELONGS.
    std::fs::create_dir(&kept).expect("a directory");
    assert_eq!(role(root, "gdfl.man"), Some(("seal-inconsistent", stub)));
    let why = fault_said(root, "census_seal_inconsistent");
    assert!(why.contains("v3-superseded is a directory"), "{why}");
    std::fs::remove_dir(&kept).expect("removed");

    // A FILE OF THE STUB'S LENGTH THAT IS NOT THE STUB.
    std::fs::rename(&aside, &kept).expect("the census kept again");
    std::fs::write(&legacy, vec![0_u8; stub_image.len()]).expect("not a stub");
    assert_eq!(role(root, "gdfl.man"), Some(("seal-inconsistent", stub)));
    let why = fault_said(root, "census_seal_inconsistent");
    assert!(
        why.contains(&format!("holds {stub} bytes, not the seal's stub")),
        "{why}"
    );
    assert!(
        why.contains(&format!("v3-superseded keeps {census} bytes")),
        "{why}"
    );

    // A SYMBOLIC LINK AT THE NAME, EVEN TO THE KEPT CENSUS.
    std::fs::remove_file(&legacy).expect("removed");
    std::os::unix::fs::symlink(&kept, &legacy).expect("a link");
    let why = fault_said(root, "census_seal_inconsistent");
    assert!(
        why.contains("gdfl.man is a symbolic link, not the seal's stub"),
        "{why}"
    );
    std::fs::remove_file(&legacy).expect("removed");

    // NOTHING AT THE NAME.
    assert_eq!(role(root, "gdfl.man"), None);
    let why = fault_said(root, "census_seal_inconsistent");
    assert!(why.contains("gdfl.man is absent"), "{why}");

    // THE SEAL WHOLE AGAIN: the stub at the name and the census kept.
    std::fs::write(&legacy, &stub_image).expect("the stub back");
    assert_eq!(role(root, "gdfl.man"), Some(("sealed-stub", stub)));
    let body = answer(root);
    assert_eq!(body["faults"], serde_json::json!([]), "{body}");
    assert_eq!(
        body["census_kept_bytes"].as_u64(),
        Some(kept_sealed),
        "{body}"
    );
}

/// **WHETHER `manifest/REQUIRES` EXISTS IS SAID**: a whole seal without it is
/// its own fault; with it, none; and an unsealed census's fault says it
/// exists when it does.
#[test]
fn disk_json_says_whether_requires_exists() {
    let scratch = Scratch::new("requires");
    let root = &scratch.0;
    sealed(root);
    let requires = census_store::requires_path(root);
    let held = std::fs::read(&requires).expect("REQUIRES written by the seal");
    std::fs::remove_file(&requires).expect("removed");
    let body = answer(root);
    assert_eq!(body["feeds"][0]["seal"]["requires"], false);
    assert_eq!(body["faults"][0]["fault"], "census_requires_absent");
    let why = body["faults"][0]["why"].as_str().expect("why");
    assert!(why.contains("sealed, but"), "{why}");
    assert!(why.contains("REQUIRES is absent"), "{why}");
    // A directory at its name is not a requirements file, and is said to be
    // a directory, never "absent" (AG-05 repair 1); so is a symbolic link.
    std::fs::create_dir(&requires).expect("a directory");
    assert_eq!(answer(root)["feeds"][0]["seal"]["requires"], false);
    let why = fault_said(root, "census_requires_absent");
    assert!(why.contains("REQUIRES is a directory"), "{why}");
    assert!(!why.contains("is absent"), "{why}");
    std::fs::remove_dir(&requires).expect("removed");
    std::os::unix::fs::symlink(root.join("elsewhere"), &requires).expect("a link");
    assert_eq!(answer(root)["feeds"][0]["seal"]["requires"], false);
    let why = fault_said(root, "census_requires_absent");
    assert!(why.contains("REQUIRES is a symbolic link"), "{why}");
    std::fs::remove_file(&requires).expect("removed");
    std::fs::write(&requires, &held).expect("restored");
    let body = answer(root);
    assert_eq!(body["feeds"][0]["seal"]["requires"], true);
    assert_eq!(body["faults"], serde_json::json!([]));

    // Unsealed, with REQUIRES present: the fault says it exists.
    let legacy = manifest_path(root, Vendor::Gdfl);
    let kept = census_store::superseded_path(root, Vendor::Gdfl);
    std::fs::remove_file(&legacy).expect("stub removed");
    std::fs::rename(&kept, &legacy).expect("census back");
    std::fs::remove_file(receipt(root)).expect("no receipt");
    let why = answer(root)["faults"][0]["why"]
        .as_str()
        .expect("why")
        .to_owned();
    assert!(why.contains("REQUIRES exists"), "{why}");
}

/// **A RECEIPT THIS BUILD CANNOT READ IS `seal-unknown`, NEVER A SEAL** — a
/// later format, a directory and a symbolic link at its name — and a store
/// with no census, or an unmigrated one of 0 bytes, carries no seal fault.
#[test]
fn a_receipt_that_cannot_be_read_and_a_census_with_nothing_to_seal() {
    let scratch = Scratch::new("unknown");
    let root = &scratch.0;
    assert_eq!(answer(root)["feeds"], serde_json::json!([]), "no census");
    assert_eq!(answer(root)["faults"], serde_json::json!([]));
    std::fs::write(manifest_path(root, Vendor::Gdfl), b"").expect("0 bytes");
    assert_eq!(role(root, "gdfl.man"), Some(("single", 0)));
    let body = answer(root);
    assert_eq!(body["feeds"][0]["seal"]["migrated"], false);
    assert!(body["feeds"][0]["seal"]["receipt"].is_null());
    assert_eq!(body["faults"], serde_json::json!([]));
    std::fs::remove_file(manifest_path(root, Vendor::Gdfl)).expect("removed");

    sealed(root);
    std::fs::write(receipt(root), "format=census-seal-v9\n").expect("a later format");
    assert!(matches!(role(root, "gdfl.man"), Some(("seal-unknown", _))));
    let body = answer(root);
    assert_eq!(body["feeds"][0]["seal"]["receipt"], "foreign");
    assert_eq!(body["faults"][0]["fault"], "census_seal_unknown");
    assert!(
        body["faults"][0]["why"]
            .as_str()
            .expect("why")
            .contains("format=census-seal-v9"),
        "{body}"
    );
    std::fs::remove_file(receipt(root)).expect("removed");
    let elsewhere = scratch.0.join("elsewhere");
    std::fs::write(&elsewhere, "format=census-seal-v1\n").expect("a file elsewhere");
    for kind in ["directory", "symlink"] {
        if kind == "directory" {
            std::fs::create_dir(receipt(root)).expect("a directory");
        } else {
            std::os::unix::fs::symlink(&elsewhere, receipt(root)).expect("a link");
        }
        assert!(
            matches!(role(root, "gdfl.man"), Some(("seal-unknown", _))),
            "{kind}"
        );
        let body = answer(root);
        assert_eq!(body["feeds"][0]["seal"]["receipt"], "unreadable", "{kind}");
        assert!(
            body["faults"][0]["why"]
                .as_str()
                .expect("why")
                .contains("cannot be read"),
            "{kind}: {body}"
        );
        if kind == "directory" {
            std::fs::remove_dir(receipt(root)).expect("removed");
        } else {
            std::fs::remove_file(receipt(root)).expect("removed");
        }
    }
}

/// **THE SEAL CHECK COSTS A FIXED NUMBER OF CALLS PER VENDOR, MEASURED**
/// (AG-05's O(1) proof for the cold answer, which the route keeps for
/// `DISK_TTL` so a GET loop measures the cache): n = 200 cold measurements
/// each of one vendor's census view on an unsealed store and on a sealed one,
/// and of the whole uncached answer, and of the view of a whole receipt the
/// store contradicts (repair 1). The view's files and the seal's word are
/// the same on every call. NO WALL-CLOCK BOUND IS ASSERTED (AG-05 repair 1):
/// on this shared host a bound in the default suite measures the load, not
/// the code; the cost is a fixed set of calls per vendor, named on
/// [`census_view`], and the printed p50, p99 and max go to docs/06-limits.md.
#[test]
fn the_seal_check_is_measured_per_vendor_and_the_answer_is_stable() {
    const N: usize = 200;
    let pct = |micros: &mut Vec<u128>| {
        micros.sort_unstable();
        (micros[N / 2], micros[N * 99 / 100], micros[N - 1])
    };
    let unsealed = Scratch::new("bench-unsealed");
    as_migrated_before_the_seal(&unsealed.0);
    let sealed_store = Scratch::new("bench-sealed");
    sealed(&sealed_store.0);
    let contradicted = Scratch::new("bench-contradicted");
    sealed(&contradicted.0);
    std::fs::remove_file(manifest_path(&contradicted.0, Vendor::Gdfl)).expect("stub removed");
    std::fs::rename(
        census_store::superseded_path(&contradicted.0, Vendor::Gdfl),
        manifest_path(&contradicted.0, Vendor::Gdfl),
    )
    .expect("the census back at its name");
    let mut said = Vec::new();
    for (label, root, word) in [
        ("unsealed view", &unsealed.0, "census_unsealed"),
        ("sealed view", &sealed_store.0, ""),
        (
            "contradicted receipt view",
            &contradicted.0,
            "census_seal_inconsistent",
        ),
    ] {
        let first = census_view(root, Vendor::Gdfl);
        let mut micros = Vec::with_capacity(N);
        for _ in 0..N {
            let start = std::time::Instant::now();
            let view = census_view(root, Vendor::Gdfl);
            micros.push(start.elapsed().as_micros());
            assert_eq!(view, first, "{label}: the same answer every call");
        }
        assert_eq!(
            first.1.fault.as_ref().map_or("", |(w, _)| *w),
            word,
            "{label}"
        );
        let (p50, p99, max) = pct(&mut micros);
        said.push(format!("{label}: p50 {p50} us, p99 {p99} us, max {max} us"));
    }
    let mut micros = Vec::with_capacity(N);
    for _ in 0..N {
        let start = std::time::Instant::now();
        let body = measured(&unsealed.0, None);
        micros.push(start.elapsed().as_micros());
        assert!(body.contains(r#""fault":"census_unsealed""#), "{body}");
    }
    let (p50, p99, max) = pct(&mut micros);
    said.push(format!(
        "whole uncached answer: p50 {p50} us, p99 {p99} us, max {max} us"
    ));
    eprintln!(
        "AG-05 /disk.json seal check, n = {N} each: {}",
        said.join("; ")
    );
}

/// `census_kept_bytes` of the GDFL feed and of the whole answer.
fn kept(root: &Path) -> (u64, u64) {
    let body = answer(root);
    (
        body["feeds"][0]["census_kept_bytes"]
            .as_u64()
            .expect("the feed's kept bytes"),
        body["census_kept_bytes"].as_u64().expect("kept bytes"),
    )
}

/// **A CENSUS WHOSE RECEIPT THIS BUILD CANNOT READ IS STILL KEPT** (AG-05
/// repair 2, verify-AG-05-r2 blocking 1): with a receipt in a later format,
/// or a pipe at its name, `gdfl.man` reads `seal-unknown`; when the name does
/// not hold the stub it holds the frozen census, and its bytes are retention.
/// Before the fix `seal-unknown` was outside the kept roles and the figure
/// dropped the whole census (seen on a real-store shadow: 268430208 bytes).
/// A name that holds the stub stays out of the figure: the stub is the name
/// in use, and the census the seal kept is counted at its own name.
#[test]
fn a_census_whose_receipt_cannot_be_read_is_still_kept() {
    let scratch = Scratch::new("unknown-kept");
    let root = &scratch.0;
    let census = as_migrated_before_the_seal(root);
    let bytes = u64::try_from(census.len()).expect("len");
    assert_eq!(kept(root), (bytes, bytes), "unsealed: kept");
    for kind in ["foreign", "fifo"] {
        if kind == "foreign" {
            std::fs::write(receipt(root), "format=census-seal-v9\nreader_since=0\n")
                .expect("a later format");
        } else {
            // `mkfifo` is the external program, run from a test only.
            let made = std::process::Command::new("mkfifo")
                .arg(receipt(root))
                .status()
                .expect("mkfifo ran");
            assert!(made.success());
        }
        assert_eq!(
            role(root, "gdfl.man"),
            Some(("seal-unknown", bytes)),
            "{kind}"
        );
        assert_eq!(
            kept(root),
            (bytes, bytes),
            "{kind}: the frozen census is kept"
        );
        assert_eq!(answer(root)["faults"][0]["fault"], "census_seal_unknown");
        std::fs::remove_file(receipt(root)).expect("removed");
    }

    // THE STUB AT THE NAME IS THE NAME IN USE: not counted.
    let scratch = Scratch::new("unknown-stub");
    let root = &scratch.0;
    sealed(root);
    let superseded = role(root, "gdfl.man.v3-superseded").expect("kept").1;
    std::fs::write(receipt(root), "format=census-seal-v9\n").expect("a later format");
    assert!(matches!(role(root, "gdfl.man"), Some(("seal-unknown", _))));
    assert_eq!(kept(root), (superseded, superseded));
}

/// **A FILE AT TWO CENSUS NAMES IS COUNTED ONCE** (AG-05 repair 2,
/// verify-AG-05-r2 blocking 2): a seal links the census to
/// `gdfl.man.v3-superseded` before it writes the stub, so one killed between
/// the two leaves ONE inode at both names and no receipt. The figure counted
/// it at each name, twice the file; the fault said it was the frozen census
/// alone, never that a seal was interrupted. An older epoch that is a second
/// name of the current one costs nothing kept, and a second name of the
/// frozen census is not counted again.
#[test]
fn a_seal_killed_after_its_link_counts_its_one_file_once() {
    let scratch = Scratch::new("linked");
    let root = &scratch.0;
    let census = as_migrated_before_the_seal(root);
    let bytes = u64::try_from(census.len()).expect("len");
    let legacy = manifest_path(root, Vendor::Gdfl);
    let kept_at = census_store::superseded_path(root, Vendor::Gdfl);
    std::fs::hard_link(&legacy, &kept_at).expect("the seal's link");
    assert_eq!(role(root, "gdfl.man"), Some(("unsealed", bytes)));
    assert_eq!(
        role(root, "gdfl.man.v3-superseded"),
        Some(("superseded", bytes))
    );
    assert_eq!(kept(root), (bytes, bytes), "one file, counted once");
    let body = answer(root);
    let why = body["faults"][0]["why"].as_str().expect("why");
    assert_eq!(body["faults"][0]["fault"], "census_unsealed");
    assert!(
        why.contains("a seal was interrupted after its link and before its stub")
            && why.contains(&format!("one file of {bytes} bytes")),
        "{why}"
    );

    // AN OLDER EPOCH THAT IS A SECOND NAME OF THE CURRENT ONE: nothing kept.
    let current = census_files(root, Vendor::Gdfl)
        .into_iter()
        .find(|file| file.role == "current")
        .expect("a current epoch");
    let epoch = current
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(pull::manifest::parse_epoch_name)
        .expect("an epoch name");
    let later = current.path.with_file_name(format!("{:06}.man", epoch + 1));
    std::fs::hard_link(&current.path, &later).expect("a second name");
    assert_eq!(
        census_files(root, Vendor::Gdfl)
            .iter()
            .filter(|file| file.role == "older-epoch")
            .count(),
        1
    );
    assert_eq!(kept(root), (bytes, bytes), "the older name costs nothing");

    // TWO SEPARATE FILES ARE STILL TWO: the stub-less seal interrupted after
    // its stub keeps both counted.
    std::fs::remove_file(&kept_at).expect("the link removed");
    std::fs::write(&kept_at, &census).expect("a separate copy");
    assert_eq!(kept(root), (2 * bytes, 2 * bytes));
}

/// **AN EMPTY SINGLE FILE IS NOT TOLD AS THE FROZEN CENSUS** (AG-05 repair
/// 2): a migrated, unsealed `gdfl.man` of 0 bytes was said to hold "the
/// frozen v3 census" a pre-version-4 reader would serve.
#[test]
fn an_empty_single_file_is_not_told_as_the_frozen_census() {
    let scratch = Scratch::new("empty");
    let root = &scratch.0;
    as_migrated_before_the_seal(root);
    std::fs::write(manifest_path(root, Vendor::Gdfl), b"").expect("0 bytes");
    let body = answer(root);
    let why = body["faults"][0]["why"].as_str().expect("why");
    assert!(why.contains("is empty (0 bytes)"), "{why}");
    assert!(!why.contains("would serve the frozen v3 census"), "{why}");
}

/// **A KEPT CENSUS CUT SHORT OR REPLACED UNDER A WHOLE RECEIPT IS NOT A
/// SEAL** (AG-05 repair 2, a follow-up of verify-AG-05-r2): the receipt
/// records `superseded_bytes`, the length of the census the seal kept, and a
/// `gdfl.man.v3-superseded` of any other length read `sealed-stub` with no
/// fault. Now the single name reads `seal-inconsistent`, the seal says the
/// superseded census is not there, the fault names both lengths, and both
/// names stay in `census_kept_bytes`. The length put back reads as sealed.
#[test]
fn a_kept_census_cut_short_under_a_whole_receipt_is_seal_inconsistent() {
    let scratch = Scratch::new("cut-short");
    let root = &scratch.0;
    sealed(root);
    let kept_at = census_store::superseded_path(root, Vendor::Gdfl);
    let whole = std::fs::read(&kept_at).expect("the kept census");
    let bytes = u64::try_from(whole.len()).expect("len");
    let stub = u64::try_from(census_store::sealed_stub(Vendor::Gdfl).len()).expect("len");
    assert_eq!(role(root, "gdfl.man"), Some(("sealed-stub", stub)));
    for (kind, len) in [
        ("cut short", bytes - 1),
        ("emptied", 0),
        ("grown", bytes + 7),
    ] {
        std::fs::OpenOptions::new()
            .write(true)
            .open(&kept_at)
            .and_then(|file| file.set_len(len))
            .expect("the length planted");
        assert_eq!(
            role(root, "gdfl.man"),
            Some(("seal-inconsistent", stub)),
            "{kind}"
        );
        assert_eq!(
            answer(root)["feeds"][0]["seal"]["superseded"],
            false,
            "{kind}"
        );
        let why = fault_said(root, "census_seal_inconsistent");
        assert!(
            why.contains(&format!(
                "keeps {len} bytes, not the {bytes} bytes the receipt records"
            )),
            "{kind}: {why}"
        );
        assert_eq!(kept(root), (stub + len, stub + len), "{kind}: both kept");
    }
    std::fs::write(&kept_at, &whole).expect("the recorded length put back");
    assert_eq!(role(root, "gdfl.man"), Some(("sealed-stub", stub)));
    assert_eq!(answer(root)["faults"], serde_json::json!([]));
    assert_eq!(kept(root), (bytes, bytes));
}

/// **A NAME AT `manifest/v4/<vendor>` THAT IS NOT A DIRECTORY IS NAMED**
/// (AG-05 repair 2, a follow-up of verify-AG-05-r2): a symbolic link or a
/// file there read `migrated: false`, the single file `single`, and no fault,
/// exactly as a census nobody had touched. The roles stay as they were; the
/// answer now carries `census_v4_not_a_directory` saying what stands there,
/// and a feed with no single file left is still answered for its fault.
#[test]
fn a_name_at_the_v4_directory_that_is_not_one_is_named() {
    let scratch = Scratch::new("v4-name");
    let root = &scratch.0;
    census_store::record(root, Vendor::Gdfl, &[held(0)]).expect("recorded");
    let single = std::fs::metadata(manifest_path(root, Vendor::Gdfl))
        .expect("the single file")
        .len();
    assert_eq!(role(root, "gdfl.man"), Some(("single", single)));
    assert_eq!(answer(root)["faults"], serde_json::json!([]));

    let dir = vendor_dir(root, Vendor::Gdfl);
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("a directory elsewhere");
    std::fs::create_dir_all(dir.parent().expect("manifest/v4")).expect("manifest/v4");
    std::os::unix::fs::symlink(&elsewhere, &dir).expect("a link at the v4 name");
    assert_eq!(
        role(root, "gdfl.man"),
        Some(("single", single)),
        "listed as it is"
    );
    let why = fault_said(root, "census_v4_not_a_directory");
    assert!(
        why.contains(&format!("{} is a symbolic link", dir.display())),
        "{why}"
    );
    assert_eq!(answer(root)["feeds"][0]["seal"]["migrated"], false);

    std::fs::remove_file(&dir).expect("the link removed");
    std::fs::write(&dir, b"not a directory").expect("a file at the v4 name");
    let why = fault_said(root, "census_v4_not_a_directory");
    assert!(why.contains("is a regular file"), "{why}");

    std::fs::remove_file(manifest_path(root, Vendor::Gdfl)).expect("no single file");
    let body = answer(root);
    assert_eq!(body["feeds"][0]["feed"], "gdfl", "{body}");
    assert_eq!(
        body["faults"][0]["fault"], "census_v4_not_a_directory",
        "{body}"
    );
}

#![cfg(test)]
//! A SEALED SELECTION V6 BLOCK, COMMITTED BY THE PRODUCTION `ledger-v6` VERB
//! OVER A GENERATED STORE, READ BACK THROUGH THE PAGE THAT SERVES IT (L-BCA-5).
//!
//! Generated prices only, never market evidence: NIFTY and BANKNIFTY index
//! minutes that cycle through a fixed pattern around 20,000.00 and 40,000.00,
//! written with the store's own writer (`indexstopqualificationjson`'s
//! generator). The sweep, the strict source load, Statistics V3 through
//! Execution V4 and the Selection V6 commit are all the public verb,
//! `cli::run(["ledger-v6", ...])`; nothing here writes or seals a block by
//! hand. A build with no commit stamp cannot run that verb at all, which is the
//! one thing this test asserts there.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a generated fixture and an exact projection must fail the test by name"
)]

use super::*;
use std::path::PathBuf;

/// Set in the child that runs the verb, over the store it names.
const CHILD: &str = "BRUTEX_SAVED_SELECTION_V6_TEST_CHILD";
const CHILD_PATH: &str = concat!(
    module_path!(),
    "::a_ledger_v6_block_is_served_saved_and_every_damage_to_it_is_refused_by_name"
);

/// The verb's words over `root`: zerodha, May 2025 alone, the largest support
/// the verb takes (999,999 ppm: a pattern must fire on all but one bar in a
/// million) and a 50-point stop ceiling, with `root` both the store it reads
/// and the ledger it writes, as the page requires.
fn ledger_words(root: &Path) -> Vec<String> {
    [
        "ledger-v6",
        "zerodha",
        "2025",
        "5",
        "2025",
        "5",
        "999999",
        "50",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([root.display().to_string()])
    .collect()
}

/// The parent half: a fresh scratch store holding April and May 2025 of both
/// indices at one minute and one day (April is the month the verb reads
/// before its first session), then the child run alone over it.
fn in_a_ledger_child() {
    let root = crate::scratch::path("saved-selection-v6");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    for (index, shift) in [("NIFTY", 0), ("BANKNIFTY", 2_000_000)] {
        crate::indexstopqualificationjson::saved_tests::index_months(
            &root,
            index,
            &[4, 5],
            |month, day, minute, place| {
                let (open, high, low, close) =
                    crate::indexstopqualificationjson::saved_tests::generated_bar(
                        month, day, minute, place,
                    );
                (open + shift, high + shift, low + shift, close + shift)
            },
        );
    }
    let policy = root.join("policy.toml");
    std::fs::write(
        &policy,
        include_str!("../../../config/intraday-research-v1.toml"),
    )
    .expect("the policy file");
    let log = root.join("child.log");
    let output = std::fs::File::create_new(&log).expect("a log");
    let child_test = CHILD_PATH
        .split_once("::")
        .map_or(CHILD_PATH, |(_, path)| path);
    let status = std::process::Command::new(std::env::current_exe().expect("this binary"))
        .args(["--exact", child_test, "--nocapture", "--test-threads=1"])
        .env_clear()
        .envs(std::env::var_os("LLVM_PROFILE_FILE").map(|profile| ("LLVM_PROFILE_FILE", profile)))
        .env(CHILD, &root)
        .env("BRUTEX_STORE", &root)
        .env("BRUTEX_CHECKSUM_RECEIPTS", &root)
        .env("BRUTEX_CHECKSUM_MAX_BYTES", "67108864")
        .env("BRUTEX_CHECKSUM_MAX_RECORDS", "3000000")
        .env("BRUTEX_ADMISSION_POLICY_FILE", &policy)
        .stdout(output.try_clone().expect("a second handle"))
        .stderr(output)
        .status()
        .expect("the child runs");
    let printed = std::fs::read_to_string(&log).expect("the child's log");
    assert!(status.success(), "{printed}");
    assert!(printed.contains("test result: ok. 1 passed;"), "{printed}");
    let _ = std::fs::remove_dir_all(&root);
}

/// One page of `/selection-v6.json` through its handler, as JSON with its
/// status.
fn page(runtime: &tokio::runtime::Runtime, query: &str) -> (StatusCode, Value) {
    let uri: Uri = format!("/selection-v6.json?{query}")
        .parse()
        .expect("a URI");
    let (status, _, body) = runtime.block_on(selection_v6_json(uri));
    (status, serde_json::from_str(&body).expect("JSON"))
}

/// The answer `body` gives for `rung`.
fn rung<'a>(body: &'a Value, rung: &str) -> &'a Value {
    body["rungs"]
        .as_array()
        .expect("rungs")
        .iter()
        .find(|row| row["rung"] == rung)
        .unwrap_or_else(|| panic!("{rung}: {body}"))
}

/// **A SELECTION V6 BLOCK `ledger-v6` COMMITTED IS SERVED SAVED, AND EVERY
/// DAMAGE TO IT IS REFUSED BY NAME** (BASE-COV-API-AS3, D-8664). The public
/// verb over a store that holds both indices at one minute and no coarser
/// rung: the 1min rung sweeps both families (each naturally extinct at a
/// support no generated condition meets), runs the route and commits its
/// sealed block, and the 2min rung, whose files the store does not hold,
/// refuses the run. The block stays, and the page serves it SAVED (the
/// `Records` arm and a saved status) beside seven absent rungs. Run in a
/// child, since the store, the strict bounds and the policy come from the
/// environment.
///
/// Extremes: singles — a store holding only the finest rung, the largest
/// support the verb takes (both families extinct: a sealed block of no
/// winners), a page one block past the end (served empty, never refused),
/// the largest page start there is (`u64::MAX`: served empty, no overflow), the
/// smallest page (limit 1), each family selector, the same run asked again
/// (the block reused, its file unchanged byte for byte), the block twice in
/// its file (refused: duplicate identities), the file one byte short (refused:
/// an incomplete trailing block); combinations — a family selector, a page
/// past the end and the smallest page at once, one rung saved while another
/// is refused in one answer at once, the file cut and restored at once (served
/// again); out of the box — the committed block placed under the 5min
/// directory (a sealed block under a rung it does not record: refused by
/// name), and the 1min file replaced by a link to that copy (the no-follow
/// open refuses it).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one production commit, then every page and every damage in the order a reader meets them"
)]
fn a_ledger_v6_block_is_served_saved_and_every_damage_to_it_is_refused_by_name() {
    let Some(root) = std::env::var_os(CHILD) else {
        return in_a_ledger_child();
    };
    let root = PathBuf::from(root);
    let mut said = String::new();
    let code = cli::run(&ledger_words(&root), &mut said);
    let Some(_stamp) = cli::commit_stamp() else {
        assert_eq!(code, cli::FAILED, "{said}");
        assert!(said.contains("refused: "), "{said}");
        return assert!(!root.join("selection").exists(), "{said}");
    };
    // THE RUN: the 1min rung committed, both families extinct, then refused.
    assert_eq!(code, cli::FAILED, "{said}");
    let committed = said
        .lines()
        .find_map(|line| line.trim().strip_prefix("1min Selection V6 "))
        .unwrap_or_else(|| panic!("no 1min commit: {said}"));
    let (short, rest) = committed.split_once(' ').expect("an identity");
    assert_eq!(
        rest, "written: 0 actual Top-25 rows; 0 actual Top-10 rows",
        "{said}"
    );
    assert!(
        said.contains("NIFTY naturally extinct · BANKNIFTY naturally extinct"),
        "{said}"
    );
    assert!(
        said.trim_end()
            .lines()
            .last()
            .is_some_and(|line| line.starts_with("refused: ")),
        "{said}"
    );
    let selection = root.join("selection");
    assert!(
        !selection.join("2min").exists(),
        "the rung that refused made no directory"
    );
    let file = selection.join("1min").join("global-selection-v6.bin");
    let block = std::fs::read(&file).expect("the committed block");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime");

    // THE SAVED PAGE.
    let (status, first) = page(&runtime, "");
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["status"], "saved", "{first}");
    assert_eq!(first["root"], root.display().to_string(), "{first}");
    let saved = rung(&first, "1min");
    assert_eq!(
        (&saved["status"], &saved["total_blocks"], &saved["from"]),
        (&json!("saved"), &json!("1"), &json!("0")),
        "{first}"
    );
    let records = saved["records"].as_array().expect("records");
    assert_eq!(records.len(), 1, "{first}");
    let record = &records[0];
    assert!(
        record["identity"]
            .as_str()
            .expect("an identity")
            .starts_with(short),
        "the page serves the block the run named: {first}"
    );
    assert_eq!(record["rung_seconds"], "60", "{first}");
    assert_eq!(
        (&record["rows"], &record["winner_count"], &record["winners"]),
        (&json!("0"), &json!(0), &json!([])),
        "{first}"
    );
    assert_eq!(
        record["families"],
        json!([
            {"family":"NIFTY","terminal":"naturally-extinct","candidates":"0","evaluated":"0","decisions":"0"},
            {"family":"BANKNIFTY","terminal":"naturally-extinct","candidates":"0","evaluated":"0","decisions":"0"}
        ]),
        "{first}"
    );
    for name in ["2min", "3min", "5min", "10min", "15min", "30min", "60min"] {
        let absent = rung(&first, name);
        assert_eq!(absent["status"], "absent", "{first}");
        assert!(
            absent["path"]
                .as_str()
                .expect("a path")
                .ends_with(&format!("selection/{name}/global-selection-v6.bin")),
            "{first}"
        );
    }

    // THE SAME RUN ASKED AGAIN reuses the block and appends nothing.
    let mut again = String::new();
    assert_eq!(cli::run(&ledger_words(&root), &mut again), cli::FAILED);
    assert!(
        again.contains(&format!("1min Selection V6 {short} reused: ")),
        "{again}"
    );
    assert_eq!(
        std::fs::read(&file).expect("the block"),
        block,
        "unchanged byte for byte"
    );

    // PAGES: past the end, at the largest start, the smallest, each family.
    for (query, from, shown) in [
        ("from=1", "1", 0),
        ("from=18446744073709551615", "18446744073709551615", 0),
        ("limit=1", "0", 1),
        ("family=NIFTY", "0", 1),
        ("family=BANKNIFTY&from=1&limit=1", "1", 0),
    ] {
        let (status, body) = page(&runtime, query);
        assert_eq!(status, StatusCode::OK, "{query}: {body}");
        assert_eq!(body["status"], "saved", "{query}: {body}");
        let saved = rung(&body, "1min");
        assert_eq!(saved["total_blocks"], "1", "{query}: {body}");
        assert_eq!(saved["from"], from, "{query}: {body}");
        assert_eq!(
            saved["records"].as_array().expect("records").len(),
            shown,
            "{query}: {body}"
        );
    }
    let (_, narrowed) = page(&runtime, "family=BANKNIFTY");
    assert_eq!(narrowed["family"], "BANKNIFTY", "{narrowed}");

    // DAMAGE, EACH REFUSED BY NAME, AND RESTORED.
    let refusal = |query: &str, name: &str| {
        let (status, body) = page(&runtime, query);
        assert_eq!(status, StatusCode::OK, "{body}");
        let answer = rung(&body, name).clone();
        assert_eq!(answer["status"], "refused", "{body}");
        assert_eq!(answer["records"], json!([]), "{body}");
        (
            body,
            answer["refusal"].as_str().expect("a refusal").to_owned(),
        )
    };
    std::fs::write(&file, [block.as_slice(), block.as_slice()].concat()).expect("twice");
    let (body, why) = refusal("", "1min");
    assert!(why.contains("duplicate committed identities"), "{why}");
    assert_eq!(body["status"], "absent", "no rung holds a readable block");
    std::fs::write(&file, &block[..block.len() - 1]).expect("one byte short");
    let (_, why) = refusal("", "1min");
    assert!(why.contains("incomplete trailing block"), "{why}");
    std::fs::write(&file, &block).expect("restored");
    let (_, restored) = page(&runtime, "");
    assert_eq!(
        rung(&restored, "1min")["records"],
        first["rungs"][0]["records"]
    );
    let elsewhere = selection.join("5min");
    std::fs::create_dir_all(&elsewhere).expect("the 5min directory");
    std::fs::write(elsewhere.join("global-selection-v6.bin"), &block).expect("a copy");
    let (body, why) = refusal("", "5min");
    assert!(
        why.contains("a block records rung 60 s under the 5min directory"),
        "{why}"
    );
    assert_eq!(body["status"], "saved", "{body}");
    assert_eq!(rung(&body, "1min")["status"], "saved", "{body}");
    std::fs::remove_file(&file).expect("the 1min file moved aside");
    std::os::unix::fs::symlink(elsewhere.join("global-selection-v6.bin"), &file)
        .expect("a link in its place");
    let (_, why) = refusal("", "1min");
    assert!(why.starts_with(&file.display().to_string()), "{why}");
}

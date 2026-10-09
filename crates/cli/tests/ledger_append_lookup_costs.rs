//! Ledger append, page and lookup costs that lane 1-b found stated wrongly,
//! read where they stand and checked against the source they describe.
//!
//! * W2-cli3-4 and W2-cli3-8 (D-1680): `docs/06-limits.md` §150 said a
//!   Candidate Universe append is proportional to the new block and that the
//!   generation check also hashes the data files. The production append door
//!   opened the ledger twice, and the generation is metadata only.
//! * W2-cli13-0, W2-cli11-2 and W2-cli11-3 (D-1681): a Pre-Admission page and
//!   the Observation and Finalization V2 lookups hash a whole file per call
//!   while §153, §157 and §161 said O(P) or average O(1). The page now checks
//!   metadata only; the lookups keep their hash and the documents say so.
//! * W2-cli12-0, W2-cli12-1 and W2-cli12-2 (D-1682): Statistics V2 filtered
//!   whole vectors per candidate, reserved its audit ceiling on every open, and
//!   its two-open append was stated nowhere.
//! * W2-cli7-2 and W2-cli7-3 (D-1683): the Ledger V6 route loaded NIFTY twice
//!   per rung, and its replay's full-route cost was stated nowhere.
//! * W2-cli3-3 (D-1684): the stored OOS source was rebuilt for every witness
//!   while §169 priced minting by the replay alone.
//!
//! Every file a constant below names is read at compile time, so a rename
//! fails the build rather than skipping the check. A separate test crate, as
//! `crates/cli/tests/limits_doc_drift.rs` is, because a document read into the
//! library's own tests would rebuild them whenever a line of it changed.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

const LIMITS: &str = include_str!("../../../docs/06-limits.md");
const CANDIDATE: &str = include_str!("../src/candidate_universe.rs");
const PRE_ADMISSION: &str = include_str!("../src/pre_admission_data.rs");
const OBSERVATIONS: &str = include_str!("../src/population_observations_v1.rs");
const STATISTICS: &str = include_str!("../src/population_statistics_v2.rs");
const LEDGER_V6: &str = include_str!("../src/ledger_v6.rs");
const STRICT_INPUTS: &str = include_str!("../src/strict_v6_inputs.rs");
const POPULATION_V6: &str = include_str!("../src/population_v6.rs");

/// The body of one `### §N —` section, up to the next `### ` heading.
fn section(number: u32) -> &'static str {
    let heading = format!("### §{number} —");
    let start = LIMITS
        .find(&heading)
        .unwrap_or_else(|| panic!("docs/06-limits.md has no `{heading}` heading"));
    let body = LIMITS.get(start + heading.len()..).expect("a suffix");
    let end = body.find("\n### ").unwrap_or(body.len());
    body.get(..end).expect("the section")
}

/// `text` with every `//!` and `///` dropped and every run of whitespace
/// folded to one space, so a sentence re-wrapped across lines is still found.
fn flat(text: &str) -> String {
    text.replace("//!", " ")
        .replace("///", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The body of the free function opened by `signature` in `source`, up to the
/// next line that closes at the left margin.
fn function<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("no `{signature}` in the source"));
    let body = source.get(start..).expect("a suffix");
    let end = body.find("\n}\n").expect("the function closes");
    body.get(..end).expect("the body")
}

#[test]
fn section_150_states_one_open_per_production_append_and_no_data_hash() {
    // The source first: what the section must describe.
    let door = function(CANDIDATE, "fn append_prepared_and_reverify(");
    assert_eq!(
        door.matches("CandidateUniverseLedgerV1::open").count(),
        1,
        "the production append door opens the ledger more than once again; \
         re-measure §150 before changing this test"
    );
    assert!(door.contains("reverify_committed"));
    let generation = function(CANDIDATE, "fn file_generation(");
    assert!(
        !generation.contains("hash") && !generation.contains("blake3"),
        "the candidate file generation hashes data now; §150 must say so"
    );

    let text = flat(section(150));
    for stale in [
        "and also hashes the data files",
        "Append, hashing, canonical-order validation and durability are proportional to the new block",
    ] {
        assert!(!text.contains(stale), "§150 still says `{stale}`");
    }
    for needed in [
        "one production append is O(R+C) for that open plus O(new rows)",
        "It is metadata only and hashes no data file",
        "16 per run",
    ] {
        assert!(text.contains(needed), "§150 no longer says `{needed}`");
    }
    let header = flat(CANDIDATE.get(..2_000).expect("the module header"));
    assert!(!header.contains("the sealed internal append is O(new rows)"));
    assert!(header.contains("one production append costs O(rows + receipts)"));
}

#[test]
fn a_pre_admission_append_door_opens_once_and_section_153_says_so() {
    let mut doors = 0;
    let mut rest = PRE_ADMISSION;
    while let Some(at) = rest.find("    pub(crate) fn append_and_reopen(") {
        let door = method(rest, "    pub(crate) fn append_and_reopen(");
        assert_eq!(door.matches("::open(root, bounds)?").count(), 1);
        assert!(door.contains("ledger.reverify_committed(&committed)?"));
        assert!(
            !door.contains("open_read"),
            "a Pre-Admission append door opens the ledger twice again; re-measure §153"
        );
        doors += 1;
        rest = rest.get(at + 1..).expect("a suffix");
    }
    assert_eq!(
        doors, 2,
        "Pre-Admission V1 and V2 each have one append door"
    );

    let text = flat(section(153));
    for needed in [
        "One production append door, V1 or V2, runs one full open since D-4781",
        "two full opens per append",
        "One append is still O(file bytes)",
    ] {
        assert!(text.contains(needed), "§153 no longer says `{needed}`");
    }
    let module = flat(PRE_ADMISSION.get(..3_400).expect("the module header"));
    assert!(module.contains("opens the ledger once and then re-reads only its committed pair"));
}

/// The body of the method opened by `signature` in `source`, up to the next
/// line that closes at one indent.
fn method<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("no `{signature}` in the source"));
    let body = source.get(start..).expect("a suffix");
    let end = body.find("\n    }\n").expect("the method closes");
    body.get(..end).expect("the body")
}

/// The limits section after `heading`, up to the next `## ` heading.
fn chapter(heading: &str) -> String {
    let start = LIMITS
        .find(heading)
        .unwrap_or_else(|| panic!("docs/06-limits.md has no `{heading}`"));
    let body = LIMITS.get(start + heading.len()..).expect("a suffix");
    let end = body.find("\n## ").unwrap_or(body.len());
    flat(body.get(..end).expect("the chapter"))
}

const CHAPTER: &str = "## Ledger append, page and lookup costs found by lane 1-b — D-1680 onward";

#[test]
fn a_pre_admission_page_checks_metadata_and_its_lookups_are_priced_by_file_bytes() {
    let page = method(PRE_ADMISSION, "    pub fn page(");
    assert!(
        !page.contains("require_unchanged") && !page.contains("require_generation("),
        "a page content-hashes the ledger again; re-measure §153"
    );
    assert_eq!(page.matches("require_metadata_generation(").count(), 3);
    let metadata = function(PRE_ADMISSION, "fn require_metadata_generation(");
    assert!(!metadata.contains("hash_file") && !metadata.contains("file_generation("));

    let section_153 = flat(section(153));
    assert!(section_153.contains("A page costs O(P) for P returned rows: since D-1681"));
    assert!(section_153.contains("A cached `reopen_audit` lookup still content-hashes both files"));
    let section_157 = flat(section(157));
    assert!(!section_157.contains("hashes its bounded bytes; hash-index lookup is average O(1)"));
    assert!(section_157.contains("so one lookup is O(file bytes)"));
    let section_161 = flat(section(161));
    assert!(!section_161.contains("identity-map lookup is average O(1); allocation"));
    assert!(section_161.contains("reads and hashes the whole bounded file, O(B)"));

    let module = flat(PRE_ADMISSION.get(..3_000).expect("the module header"));
    assert!(!module.contains("A page is proportional to the returned records after that scan"));
    assert!(module.contains("A page checks file generations by metadata only"));
}

#[test]
fn observation_lookups_still_hash_the_whole_file_and_say_so() {
    let mut lookups = 0;
    let mut rest = OBSERVATIONS;
    while let Some(at) = rest.find("    pub fn reopen_audit(") {
        let body = method(rest, "    pub fn reopen_audit(");
        assert!(body.contains("self.require_unchanged()?"));
        let doc_start = rest
            .get(..at)
            .expect("a prefix")
            .rfind("\n\n")
            .expect("a gap");
        let doc = flat(rest.get(doc_start..at).expect("the rustdoc"));
        assert!(doc.contains("so it is O(B) time and O(B) transient memory"));
        lookups += 1;
        rest = rest.get(at + 1..).expect("a suffix");
    }
    assert_eq!(lookups, 2, "Observation V1 and V2 each have one lookup");
    let unchanged = method(OBSERVATIONS, "    fn require_unchanged(&mut self)");
    assert!(unchanged.contains("read_bounded_authority_file"));

    let chapter = chapter(CHAPTER);
    for needed in [
        "Pre-Admission Data V1 `page`** (§153) is now O(P)",
        "Observation V1 and V2 `reopen_audit`** (§157, §161) read the whole bounded authority file",
        "Finalization V2 `reopen_structural_receipt`** re-hashes the bounded data file",
    ] {
        assert!(
            chapter.contains(needed),
            "the chapter no longer says `{needed}`"
        );
    }
}

#[test]
fn section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append() {
    let build = function(STATISTICS, "fn build_raw_candidates(");
    assert!(
        !build.contains(".filter("),
        "a per-candidate filter over the whole vectors is back; re-measure §154"
    );
    assert_eq!(build.matches("candidate_column(").count(), 2);
    let column = function(STATISTICS, "fn candidate_column<");
    assert!(column.contains(".skip(sequence).step_by(width)"));
    let open = method(STATISTICS, "    fn open_inner(");
    assert!(open.contains("bounds.audits.min(stored_records)"));
    assert!(!open.contains("try_reserve(usize_of(bounds.audits,"));
    let append = function(STATISTICS, "pub fn append_population_statistics_v2(");
    assert!(append.contains("PopulationStatisticsV2Ledger::open_writer(root, bounds)?"));
    assert!(append.contains("PopulationStatisticsV2Ledger::open_read(root, bounds)?"));

    let text = flat(section(154));
    for needed in [
        "so the per-candidate summaries cost O(C·(P+S)) in total",
        "never the configured `max_audits` ceiling",
        "One append through `append_population_statistics_v2` runs two full opens",
        "A appends to one root cost O(A²) block validations in total",
    ] {
        assert!(text.contains(needed), "§154 no longer says `{needed}`");
    }
}

#[test]
fn the_ledger_v6_route_and_replay_costs_are_stated() {
    let route = function(LEDGER_V6, "fn run_route(");
    assert!(route.contains("preloaded_for(underlying, &mut sized)"));
    let hand_off = function(LEDGER_V6, "fn preloaded_for<");
    assert!(hand_off.contains("sized.take()"));
    assert!(route.contains("commit_strict_candidate_pre_admission_authority_sized_v1("));
    assert!(
        !route.contains("commit_strict_candidate_pre_admission_authority_v1("),
        "the route calls the door that always reloads NIFTY"
    );
    let sizing = function(STRICT_INPUTS, "pub(crate) fn size_sweeper(");
    assert_eq!(sizing.matches("load(").count(), 1);
    assert!(sizing.contains("Ok((sweeper, inputs, sized))"));
    let replay = function(LEDGER_V6, "fn replay_route(");
    assert!(replay.contains("run_route(request, out)?"));
    let start = LEDGER_V6.find("fn replay_route(").expect("replay_route");
    let doc = flat(
        LEDGER_V6
            .get(..start)
            .and_then(|before| before.rfind("\n\n").and_then(|gap| before.get(gap..)))
            .expect("the rustdoc"),
    );
    assert!(doc.contains("O(full Step-4 route) per call"));

    let chapter = chapter(CHAPTER);
    for needed in [
        "One `run_route` now makes 16 strict loads (8 rungs x 2 families)",
        "where before it made 24",
        "a rerun over fully committed authorities still costs the full Step-4 route",
    ] {
        assert!(
            chapter.contains(needed),
            "the chapter no longer says `{needed}`"
        );
    }
}

#[test]
fn section_169_prices_the_oos_fold_once_per_cohort() {
    let witnesses = method(
        POPULATION_V6,
        "    pub(crate) fn selected_stored_oos_witnesses(",
    );
    assert_eq!(witnesses.matches(".fold_recorded(observer)").count(), 1);
    assert_eq!(witnesses.matches(".mint_witness_recorded(").count(), 1);
    let text = flat(section(169));
    for needed in [
        "Since D-1684 Population V6 builds it once per family cohort",
        "so a witness remains Θ(S + Q + D + E) in hashing",
    ] {
        assert!(text.contains(needed), "§169 no longer says `{needed}`");
    }
    assert!(
        !text.contains("Minting every witness is proportional to the authenticated Runner replay")
    );
}

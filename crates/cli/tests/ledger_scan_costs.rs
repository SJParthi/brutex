//! Four ledger calls that rehash or rescan whole files, read where they stand
//! and held to what their docs and `docs/06-limits.md` say they cost.
//!
//! W2-cli11-1, W2-cli11-0, W2-cli12-3 and W2-cli12-4 found each cost
//! undocumented, or contradicted. The sources are read at compile time, so a
//! renamed file fails the build rather than skipping the check. Each test
//! first counts, in the function bodies, the calls that make the cost what the
//! sentence says, then requires the sentence in the item's rustdoc and in the
//! limits. A separate test crate, because a document read into the library's
//! own tests would rebuild them whenever any line of the limits changed.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail. An integration test file is \
              its own crate root, so the attribute cannot be inherited."
)]

const FINALIZATION_V3: &str = include_str!("../src/population_finalization_v3.rs");
const FINALIZATION_V4: &str = include_str!("../src/population_finalization_v4.rs");
const POPULATION_V5: &str = include_str!("../src/population_v5.rs");
const ORCHESTRATOR: &str = include_str!("../src/step3_orchestrator.rs");
const LIMITS: &str = include_str!("../../../docs/06-limits.md");

/// The heading of the limits section these four costs are stated in.
const SECTION: &str = "## Four ledger calls that rehash or rescan whole files per call";

/// `text` with every `///` and `//!` dropped and every run of whitespace
/// folded to one space, so a sentence compares by its words wherever it is
/// wrapped.
fn words(text: &str) -> String {
    text.replace("//!", " ")
        .replace("///", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The limits section this file checks, folded, up to the next `## ` heading.
fn limits() -> String {
    let from = LIMITS
        .find(SECTION)
        .unwrap_or_else(|| panic!("no `{SECTION}` in docs/06-limits.md"));
    let rest = LIMITS.get(from + SECTION.len()..).expect("a suffix");
    let to = rest.find("\n## ").unwrap_or(rest.len());
    words(rest.get(..to).expect("the section"))
}

/// The `///` lines directly above `item` (attributes between them skipped),
/// folded. `item` is searched from `anchor`, so an item name several impls
/// share is read in the one the test means.
fn doc_above(source: &str, anchor: &str, item: &str) -> String {
    let base = source
        .find(anchor)
        .unwrap_or_else(|| panic!("no `{anchor}` in the source"));
    let at = source
        .get(base..)
        .and_then(|rest| rest.find(item))
        .map_or_else(
            || panic!("no `{item}` after `{anchor}`"),
            |offset| base + offset,
        );
    let head = source.get(..at).expect("a prefix");
    let mut doc = Vec::new();
    for line in head.lines().rev().skip(1) {
        let line = line.trim();
        if line.starts_with("///") {
            doc.push(line);
        } else if !doc.is_empty() {
            // Every line between the item and its rustdoc is attribute text,
            // however many lines a `cfg_attr(not(test), expect(..))` spans
            // (CE-95, D-1956); the first non-doc line ABOVE the doc ends it.
            break;
        }
    }
    doc.reverse();
    words(&doc.join("\n"))
}

/// The body of the first function whose head is `head` after `anchor`, from
/// its first `{` to the brace that closes it.
fn body<'a>(source: &'a str, anchor: &str, head: &str) -> &'a str {
    let base = source
        .find(anchor)
        .unwrap_or_else(|| panic!("no `{anchor}` in the source"));
    let from = source
        .get(base..)
        .and_then(|rest| rest.find(head))
        .map_or_else(
            || panic!("no `{head}` after `{anchor}`"),
            |offset| base + offset,
        );
    let open = source
        .get(from..)
        .and_then(|rest| rest.find('{'))
        .map(|offset| from + offset)
        .expect("an opening brace");
    let mut depth = 0_usize;
    for (offset, byte) in source.bytes().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return source.get(open..=offset).expect("the body");
                }
            }
            _ => {}
        }
    }
    panic!("`{head}` never closes");
}

/// `text` with every whitespace character removed, so a call chain compares
/// the same however rustfmt breaks it across lines.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect()
}

/// How many times `needle` occurs in `haystack`.
fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// The sentence each of the four rustdocs carries so that CI Gate 12, which
/// refuses a cost claim naming no test, finds this file named beside it.
const HELD: &str = "`crates/cli/tests/ledger_scan_costs.rs` counts the calls that make this cost.";

/// Requires `sentence` in `doc`, naming `what` when it is absent.
fn says(doc: &str, what: &str, sentence: &str) {
    assert!(
        doc.contains(sentence),
        "{what} does not say: {sentence}\n--- it reads: {doc}"
    );
}

/// W2-cli11-1. `row_projection` reads one fixed-offset row between two
/// generation checks, each of which hashes all three files whole, twice each.
#[test]
fn a_finalization_v3_row_read_hashes_both_files_four_times_and_its_docs_say_so() {
    let lookup = body(
        FINALIZATION_V3,
        "impl PopulationFinalizationV3Ledger",
        "fn authenticated_row(",
    );
    assert_eq!(count(lookup, "self.require_unchanged()?"), 2, "{lookup}");
    assert_eq!(count(lookup, "read_fixed_at::<"), 1, "{lookup}");
    let check = body(FINALIZATION_V3, "", "fn require_unchanged(");
    assert_eq!(count(check, "file_generation("), 3, "{check}");
    for file in ["&self.lock_file", "&self.row_file", "&self.completion_file"] {
        assert!(
            check.contains(&format!("file_generation({file}"))
                || check.contains(&format!("file_generation(\n            {file}")),
            "{file} is not hashed: {check}"
        );
    }
    let generation = body(
        FINALIZATION_V3,
        "",
        "fn file_generation_with_between_hash_action(",
    );
    assert_eq!(count(generation, "hash_held_prefix("), 2, "{generation}");
    let single = body(FINALIZATION_V3, "", "fn file_generation(");
    assert_eq!(
        count(single, "file_generation_with_between_hash_action("),
        1,
        "`file_generation` must delegate to the twice-hashing body: {single}"
    );
    assert!(!single.contains("hash_held_prefix("), "{single}");
    let delegate = body(
        FINALIZATION_V3,
        "impl PopulationFinalizationV3Authority",
        "fn row_projection(",
    );
    assert_eq!(
        count(
            &squeezed(delegate),
            "self.ledger.authenticated_row(self.receipt,global_sequence)"
        ),
        1,
        "`row_projection` must read through the fixed-offset lookup: {delegate}"
    );
    assert!(!delegate.contains("ordered_row_projections"), "{delegate}");
    assert!(
        FINALIZATION_V3.contains("    #[cfg(test)]\n    pub(crate) fn row_projection("),
        "the one-row read is test-only since D-1845"
    );

    let sentence = "One row read therefore hashes the row file and the Completion file four \
                    times each: O(row-file bytes + Completion-file bytes) per row, not O(1), \
                    and O(R × those bytes) for R rows read one at a time.";
    let doc = doc_above(
        FINALIZATION_V3,
        "impl PopulationFinalizationV3Authority",
        "pub(crate) fn row_projection(",
    );
    says(
        &doc,
        "`row_projection`'s rustdoc",
        "Each call runs the ledger's generation check twice, before and after its one \
         fixed-offset read, and each check hashes the lock, row and Completion files \
         whole, each of them twice.",
    );
    says(&doc, "`row_projection`'s rustdoc", sentence);
    says(&doc, "`row_projection`'s rustdoc", HELD);
    let limits = limits();
    says(&limits, "the limits section", sentence);
    says(
        &limits,
        "the limits section",
        "`row_projection` delegates to the ledger's `authenticated_row`, and every \
         generation goes through `file_generation`.",
    );
    says(
        &limits,
        "the limits section",
        "`crates/cli/tests/ledger_scan_costs.rs` reads the source to require, for each \
         cost, the calls and delegations its bullet names, and the sentences that state it.",
    );
}

/// W2-cli11-0. `append_locked` ends by hashing the data file and, since
/// D-1845, validating only the block it wrote: it no longer rescans.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the code shape, the rustdoc and the limits for one call are read together"
)]
fn a_finalization_v4_append_validates_only_its_block_and_its_docs_say_so() {
    let append = body(FINALIZATION_V4, "", "fn append_locked(");
    let tail = append
        .rfind("file_generation(&self.data_file")
        .expect("the append hashes the data file");
    assert!(
        !append.contains("self.scan()"),
        "the append rescans: {append}"
    );
    let rescan = append
        .rfind("validate_complete_block(&mut self.data_file, block_first, &data)?")
        .expect("the append validates its own block");
    assert!(
        tail < rescan,
        "the hash comes before the validation: {append}"
    );
    assert!(
        append
            .rfind("sync Finalization V4 Completion")
            .expect("a Completion sync")
            < tail,
        "the rescan follows the Completion's sync: {append}"
    );
    let scan = body(FINALIZATION_V4, "", "fn scan(");
    assert!(scan.contains("while first < total"), "{scan}");
    assert_eq!(count(scan, "validate_complete_block("), 1, "{scan}");
    assert!(
        scan.trim_end_matches('}')
            .trim_end()
            .ends_with("self.require_unchanged()"),
        "{scan}"
    );
    let check = body(FINALIZATION_V4, "", "fn require_unchanged(");
    assert!(check.contains("file_generation(&self.data_file"), "{check}");
    let prepared = body(
        FINALIZATION_V4,
        "impl PreparedPopulationFinalizationV4 {",
        "fn validate(&self)",
    );
    let block = body(FINALIZATION_V4, "", "fn validate_complete_block(");
    assert!(block.contains("    }\n    .validate()?;"), "{block}");
    assert!(prepared.contains("bounded_set("), "{prepared}");
    let opening = append
        .get(1..)
        .expect("the body after its brace")
        .trim_start();
    assert!(
        opening.starts_with("self.require_unchanged()?;"),
        "the append does not open with a generation check: {append}"
    );
    assert!(
        squeezed(append).contains("self.require_unchanged()?;returnOk((false,receipt));"),
        "the exact reuse does not check the generation before returning: {append}"
    );
    let locked = body(
        FINALIZATION_V4,
        "impl PopulationFinalizationV4Ledger",
        "    fn append(\n",
    );
    let lock = locked.find(".lock()").expect("the exclusive lock");
    let call = locked
        .find("self.append_locked(")
        .expect("the locked append");
    assert!(lock < call, "{locked}");
    assert_eq!(count(FINALIZATION_V4, "self.append_locked("), 1);

    let sentence = "One append is therefore O(F) in the ledger's file bytes F, not O(D) in \
                    its own decisions, and the appends into one ledger cost quadratically \
                    in its length over its life.";
    let doc = doc_above(FINALIZATION_V4, "", "fn append_locked(");
    says(
        &doc,
        "`append_locked`'s rustdoc",
        "and then reads back and validates only the block it wrote: O(D) in its own \
         decisions. It no longer rescans the ledger (W2-cli11-0, D-1845). The two \
         whole-file hashes keep one append O(F) in the ledger's file bytes F, and that \
         is inherent to the check",
    );
    says(&doc, "`append_locked`'s rustdoc", HELD);
    says(
        &doc,
        "`append_locked`'s rustdoc",
        "Appends one authority under the held exclusive lock.",
    );
    let reused = "The append opens with a generation check that hashes the whole data file, \
                  and an exact reuse runs another before it returns, so a reused append that \
                  writes nothing is O(F) as well.";
    says(&doc, "`append_locked`'s rustdoc", reused);
    says(
        &doc,
        "`append_locked`'s rustdoc",
        "Block validation inserts every decision into hash sets, so the bound is expected, \
         not worst case.",
    );
    let limits = limits();
    says(&limits, "the limits section", sentence);
    says(
        &limits,
        "the limits section",
        "**Rescan removed by D-1845:** the append now reads back and validates only the \
         block it wrote",
    );
    says(&limits, "the limits section", reused);
    says(
        &limits,
        "the limits section",
        "§168's \"Encoding, ordered hashing, exact-prefix comparison and append are O(D).\"",
    );
}

/// W2-cli12-3. `CommittedStoredPopulationV5::authenticated_row` prepares the
/// whole Population twice to compare one row.
#[test]
fn a_population_v5_row_read_derives_the_whole_population_twice_and_its_docs_say_so() {
    let lookup = body(
        POPULATION_V5,
        "impl CommittedStoredPopulationV5",
        "pub(crate) fn authenticated_row(",
    );
    assert_eq!(
        count(lookup, "PreparedPopulationV5::from_authority("),
        2,
        "{lookup}"
    );
    assert_eq!(count(lookup, "prepared_before.rows.get("), 1, "{lookup}");
    assert!(
        POPULATION_V5.contains(
            "    #[cfg(test)]\n    pub(crate) fn authenticated_row(\n        &mut self,\n        global_sequence: u64,"
        ),
        "the one-row read is test-only since D-1845"
    );
    let v5_read = body(
        POPULATION_V5,
        "impl PopulationV5Ledger",
        "fn authenticated_row(",
    );
    assert!(v5_read.contains("self.require_unchanged()?"), "{v5_read}");
    assert_eq!(count(v5_read, "read_fixed_at("), 1, "{v5_read}");
    assert_eq!(
        count(
            &squeezed(lookup),
            "self.v5.authority_mut().authenticated_row(global_sequence)"
        ),
        1,
        "{lookup}"
    );
    let hop = body(
        POPULATION_V5,
        "impl PopulationV5Authority {",
        "fn authenticated_row(",
    );
    assert_eq!(
        count(
            &squeezed(hop),
            "self.ledger.authenticated_row(&self.receipt,global_sequence)"
        ),
        1,
        "the authority must read through the fixed-offset lookup: {hop}"
    );
    assert!(!hop.contains("authenticated_rows("), "{hop}");
    let v5_check = body(
        POPULATION_V5,
        "impl PopulationV5Ledger",
        "fn require_unchanged(",
    );
    assert!(
        v5_check.contains("file_generation(&self.row_file"),
        "{v5_check}"
    );
    let prepare = body(POPULATION_V5, "", "fn from_authority(");
    assert!(prepare.contains(".population_v5_inputs()"), "{prepare}");
    assert!(
        prepare.contains(
            "for input in &inputs {\n            rows.push(PopulationV5RowRecord::from_input("
        ),
        "{prepare}"
    );

    let sentence = "One row read is therefore Θ(C) row derivation twice, plus the upstream \
                    joins and the V5 generation hashing, not O(1).";
    let doc = doc_above(
        POPULATION_V5,
        "impl CommittedStoredPopulationV5",
        "pub(crate) fn authenticated_row(",
    );
    says(
        &doc,
        "`authenticated_row`'s rustdoc",
        "Each call prepares the whole Population from the live upstream source twice, \
         before and after its one row read: each preparation joins every upstream input \
         and derives every one of the C rows, and one of them is compared.",
    );
    says(&doc, "`authenticated_row`'s rustdoc", sentence);
    says(&doc, "`authenticated_row`'s rustdoc", HELD);
    let route = "The one row read goes through `PopulationV5Authority::authenticated_row` \
                 to the ledger's fixed-offset `authenticated_row`, not through the \
                 whole-block `authenticated_rows`.";
    says(&doc, "`authenticated_row`'s rustdoc", route);
    let limits = limits();
    says(&limits, "the limits section", sentence);
    says(&limits, "the limits section", route);
}

/// The call shape behind W2-cli12-4: the commit opens a writer, appends and
/// reopens, each door scans once, and every generation check hashes whole files.
fn a_population_v5_commit_opens_appends_and_reopens_and_hashes_whole_files() {
    let commit = body(POPULATION_V5, "", "pub(crate) fn commit_population_v5(");
    let write = commit
        .find("PopulationV5Ledger::open_write(")
        .expect("a writer");
    let append = commit.find("writer.append(&prepared)?").expect("an append");
    let read = commit
        .find("PopulationV5Ledger::open_read(")
        .expect("a reopen");
    assert!(write < append && append < read, "{commit}");
    let open = body(POPULATION_V5, "impl PopulationV5Ledger", "    fn open(\n");
    assert_eq!(count(open, "ledger.scan()?"), 1, "{open}");
    let finish = body(POPULATION_V5, "", "fn finish_written(");
    assert!(!finish.contains("self.scan()"), "{finish}");
    assert!(
        finish.contains("validate_complete_block(&rows, &completion)?"),
        "{finish}"
    );
    for (head, caller) in [
        ("fn append_locked(", "the fresh append"),
        ("fn complete_trailing(", "the trailing completion"),
    ] {
        let written = body(POPULATION_V5, "impl PopulationV5Ledger", head);
        assert_eq!(
            count(written, "self.finish_written(prepared.population_id)"),
            1,
            "{caller} must finish by validating its own block: {written}"
        );
    }
    assert_eq!(
        count(POPULATION_V5, "finish_written("),
        3,
        "one definition and the two written paths' calls, no other"
    );
    let reuse = body(POPULATION_V5, "", "fn reuse_existing(");
    assert!(!reuse.contains("scan("), "{reuse}");
    assert!(!reuse.contains("finish_written("), "{reuse}");
    for head in ["fn open_read(", "fn open_write("] {
        let door = body(POPULATION_V5, "impl PopulationV5Ledger", head);
        assert_eq!(count(door, "Self::open("), 1, "{door}");
    }
    assert_eq!(
        count(commit, "PreparedPopulationV5::from_authority("),
        2,
        "{commit}"
    );
    let join = body(ORCHESTRATOR, "", "pub(crate) fn population_v5_inputs(");
    assert!(join.contains(".ordered_row_projections()"), "{join}");
    assert!(join.contains(".ordered_successor_projections()"), "{join}");
    let v3_bulk = body(
        FINALIZATION_V3,
        "impl PopulationFinalizationV3Authority",
        "fn ordered_row_projections(",
    );
    assert!(
        v3_bulk.contains("self.ledger.ordered_row_projections(self.receipt)"),
        "{v3_bulk}"
    );
    let v3_ledger_bulk = body(
        FINALIZATION_V3,
        "impl PopulationFinalizationV3Ledger",
        "fn ordered_row_projections(",
    );
    assert!(
        v3_ledger_bulk.contains("self.require_unchanged()?"),
        "{v3_ledger_bulk}"
    );
    let v5_check = body(
        POPULATION_V5,
        "impl PopulationV5Ledger",
        "fn require_unchanged(",
    );
    assert_eq!(count(v5_check, "file_generation("), 3, "{v5_check}");
    for file in ["&self.lock_file", "&self.row_file", "&self.completion_file"] {
        assert!(
            squeezed(v5_check).contains(&format!("file_generation({file},")),
            "{file} is not hashed: {v5_check}"
        );
    }
    let v5_generation = body(POPULATION_V5, "", "fn file_generation(");
    assert_eq!(
        count(v5_generation, "hash_held_file("),
        2,
        "{v5_generation}"
    );
}

/// Every scan decodes, re-encodes and block-validates each row; returns the
/// `decode`, `encode` and `validate_complete_block` bodies for the doc checks.
fn population_v5_rows_are_validated_on_every_scan() -> (&'static str, &'static str, &'static str) {
    let scan = body(POPULATION_V5, "impl PopulationV5Ledger", "fn scan(");
    assert!(scan.contains("self.read_rows("), "{scan}");
    assert_eq!(count(scan, "validate_complete_block("), 1, "{scan}");
    let rows = body(POPULATION_V5, "", "fn read_rows(");
    assert!(
        rows.contains("rows.push(PopulationV5RowRecord::decode(&raw)?)"),
        "{rows}"
    );
    let decode = body(POPULATION_V5, "impl PopulationV5RowRecord", "fn decode(");
    assert!(
        decode.contains("row.validate()?;\n        if row.encode()? != *raw"),
        "{decode}"
    );
    let encode = body(
        POPULATION_V5,
        "impl PopulationV5RowRecord",
        "fn encode(&self) -> Result<[u8; POPULATION_V5_ROW_BYTES]",
    );
    assert!(encode.contains("self.validate()?;"), "{encode}");
    let block = body(POPULATION_V5, "", "fn validate_complete_block(");
    assert!(
        block.contains("for (index, row) in rows.iter().enumerate() {\n        row.validate()?;"),
        "{block}"
    );
    (decode, encode, block)
}

/// W2-cli12-4. `commit_population_v5` opens a writer, appends and reopens,
/// and each of the three scans the whole ledger.
#[test]
fn a_population_v5_commit_scans_the_whole_ledger_twice_and_its_docs_say_so() {
    a_population_v5_commit_opens_appends_and_reopens_and_hashes_whole_files();
    let (decode, encode, block) = population_v5_rows_are_validated_on_every_scan();

    let module = words(
        POPULATION_V5
            // The `//!` block ends at the first `use`: CE-95 (D-1956) removed the
            // module-wide `#![expect(` this used to stop at.
            .get(..POPULATION_V5.find("\nuse ").expect("the module doc"))
            .expect("its text"),
    );
    assert!(decode.contains("hash_parts(ROW_SEAL_DOMAIN,"), "{decode}");
    assert!(encode.contains("hash_parts(ROW_SEAL_DOMAIN,"), "{encode}");
    for former in [
        "Sequential codec, hashing and persistence work is O(C) in Candidate count.",
        "Sequential codec and hashing work is O(C) in Candidate count.",
    ] {
        assert!(
            !module.contains(former),
            "the module doc still calls whole-ledger work O(C): {module}"
        );
    }
    says(
        &module,
        "the module doc",
        "Preparing, encoding and row-seal hashing the C new rows is O(C) in Candidate count. \
         Persistence also rescans the whole ledger, decoding and seal-checking every one of \
         its R rows, which is O(R); see `commit_population_v5`.",
    );
    let sentence = "Each scan decodes every row of every Population already in the ledger, \
                    and decoding a row validates it, re-encodes it (which validates it \
                    again), and `validate_complete_block` validates it once more, so the V5 \
                    ledger work of one commit is O(R) in the ledger's total rows R, not O(C).";
    let upstream = "On top of the ledger work, `PreparedPopulationV5::from_authority` runs \
                    twice, before the write and after the reopen, and each run joins every \
                    upstream input";
    let total = "One commit is therefore O(R) plus two whole upstream joins, not O(R) alone.";
    let doc = doc_above(POPULATION_V5, "", "pub(crate) fn commit_population_v5(");
    for stated in [
        "Sequential work is O(C + bounded source/file bytes) and space is O(C) for C \
         Candidates.",
        "The file bytes above include the whole V5 ledger, and not once. The ledger is \
         scanned, every row decoded, twice per commit, written or reused: `open_write` \
         scans it and the fresh `open_read` scans it again. `finish_written` reads back \
         and validates only the block just written; it rescanned the whole ledger until \
         D-1845 (W2-cli12-4).",
        sentence,
        "Separately, every generation check hashes the lock, row and Completion files \
         whole, each of them twice; this counts scans, not those hashes.",
        upstream,
        ": `population_v5_inputs` calls `ordered_row_projections`, which hashes the \
         Finalization V3 files whole, and `ordered_successor_projections`.",
        total,
        HELD,
    ] {
        says(&doc, "`commit_population_v5`'s rustdoc", stated);
    }
    assert!(
        !doc.contains("read whole three times"),
        "the rustdoc counts whole reads it does not count: {doc}"
    );
    let limits = limits();
    for stated in [
        sentence,
        "A written commit scans the ledger three times and a reused one twice. **Since \
         D-1845 a written commit scans it twice as well:**",
        upstream,
        total,
    ] {
        says(&limits, "the limits section", stated);
    }
    assert!(block.contains("bounded_set("), "{block}");
    says(
        &limits,
        "the limits section",
        "The Finalization V4 and Population V5 bounds above are expected, not worst case: \
         the block validation that each rescan repeats inserts every decision or row into \
         hash sets built by `bounded_set`.",
    );
}

Verdict: 3 findings at 331b05c in the cli3 slice (1 medium, 2 low). None of them overlaps the earlier hunt-conc, errpaths or hunt-cli-a/b reports, or the other pass-1 files.

# conc-pass1 / cli3: crates/cli/src (remaining non-test files) plus crates/cli/build_provenance.rs

Method: I read the source only. I ran no cargo. I grepped all 55 slice files plus build_provenance.rs for rename, sync_all, OpenOptions, Mutex, RwLock, spawn, flock, Atomic, create_new, set_len and remove_file. I then read every hit in the writer paths. Before reporting a module I checked that production code can reach it. gaps-1 says `admission_store`, `global_replay` v1/v2/v3, `institutional_statistics`, `stored_data_completeness` and `admission_join` have no production caller, and I re-checked that at this commit: `AdmissionAuthorityLedger::open` is called only from tests, and `global_replay_v3` is named by no other module.

---

## cli3-1 (medium): a VIX companion interrupted after its directory is created can never be published again, and the index-stop catalog stays refused for good

**Where:** crates/cli/src/index_stop_vix.rs:385-411 (`publish`). It is reached from index_stop.rs:415 (`publish_vix`) inside `produce_catalog_inner`, and that runs for every rung of every index-stop search batch (index_stop_search.rs:449 / 459 → `produce_catalog_with_context`).

```rust
let directory = root.join(NAMESPACE).join(crate::identity_hex(&lookup));
match std::fs::symlink_metadata(&directory) {
    Ok(_) => {
        let saved = Reader::open(root, identity, pin, bounds)?;
        ...
        return Ok(saved);
    }
    Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
    Err(why) => return Err(display(why)),
}
catalog.with_current(|| {
    ...
    let pending = persistence::prepare_in_namespace(root, NAMESPACE, lookup, &body)?;
    pending.verify_body(digest, body.len() as u64)?;
    pending.finish(lookup, digest, body.len() as u64)?;
```

**Why it is wrong:** The shortcut treats "the directory exists" as "the companion is complete". But `prepare_in_namespace` (boolean_candidate_persistence.rs:94-119) creates that directory first. It then creates `owner.lock` and writes and fsyncs `body.bin`. Only after that does `finish` write `complete.bin` (:74). So a process that dies anywhere between `directory(&base, &directory_path)` and the end of `finish` leaves a directory with no `complete.bin`. On every later call, `publish` takes the `Ok(_)` arm and calls `Reader::open`. That reaches `Observation::open` (boolean_observation_file.rs:47), whose `read_held(&directory.join("complete.bin"), 112)` fails with NotFound. The result is `Err("saved VIX reference companion unavailable or invalid; no current-data substitution: ...")`. Nothing ever re-enters `prepare_in_namespace`, even though that function would complete the publication (or match it byte for byte). The shortcut is the only place that sends the retry down the read path.

**Repro (crash and restart):**
1. `index-stop-search` runs batch N. In one rung, `produce_catalog_inner` publishes the catalog (index_stop.rs:401-405) and calls `publish_vix`.
2. `index_stop_vix::publish` gets NotFound, captures and encodes the image, and enters `prepare_in_namespace`. The directory `index-stop-vix-reference-v1/<lookup>` is created and synced. The process is then SIGKILLed or loses power while writing `body.bin`, or anywhere before `finish` writes `complete.bin`.
3. The operator reruns the identical command. `checkpoint::recover` restores the pending frame for batch N, and the same rung recomputes the same catalog identity and completion. `lookup_identity(identity, pin)` is therefore the same. The catalog's own `prepare_in_namespace` is idempotent and passes. Then `publish_vix` finds the directory, calls `Reader::open`, and refuses on the missing `complete.bin`. `produce_catalog_inner` fails, the catalog attempt gets the Refused terminal, `completed_links` reports "Single-stop batch remains pending", and the search cannot get past batch N. Every later rerun repeats this, because the inputs are deterministic. Nothing in the tree removes the directory, and §3 rule 8 says nothing should.

A second, transient case shows up from the same line. If two processes (the api's index-stop worker and a cli run) reach the same catalog, the second one sees the first one's half-built directory and refuses instead of waiting or matching.

**Not the same as conc-pass1/search.md (`write_or_equal`):** that finding is about a torn `body.bin` or `complete.bin` being refused on retry. Here the retry never reaches `write_or_equal` at all. A crash right after the `mkdir`, with no files yet, wedges this namespace too. Fixing `write_or_equal` alone leaves this wedge in place.

**Minimal fix:** Key the shortcut on the completion receipt, not on the directory. Return `Reader::open` only when `directory/complete.bin` exists. In every other case (no directory, or a directory without `complete.bin`), go to `prepare_in_namespace`/`finish`. They already reuse identical bytes, and the owner `try_lock` in `prepare_in_namespace` keeps a live publisher exclusive. Add a test that removes `complete.bin` (and separately leaves only the empty directory), then calls `publish` and expects success.

---

## cli3-2 (low): the build stamp is checked once, then other source files are compiled later; an edit in that window gives a clean-HEAD stamp on a binary built from different bytes

**Where:** crates/cli/build.rs:79-90 and build_provenance.rs:155-188 (`verify` → `worktree_matches` → `Verification { commit: Some(head), .. }`).

```rust
let verified = build_provenance::verify(Path::new(&manifest), explicit.as_deref());
...
if let Some(commit) = verified.commit {
    println!("cargo:rustc-env=BRUTEX_COMMIT={commit}");
```

**Why it is wrong:** `verify` hashes every relevant working-tree file at the moment `cli`'s build script runs (build_provenance.rs:758-766, `fs::read(full)... object_oid("blob", &bytes) == expected.oid`). rustc reads those same files later:
- `crates/cli/src/**` is compiled after the build script finishes.
- `engine`, `runner`, `vocab`, `indicators`, `store`, `pull`, `costs` and `core` are normal dependencies of `cli`. Cargo is free to compile them at the same time as `cli`'s build script or after it, because the script waits only on its build-dependencies.

Nothing re-checks after compilation. `rerun-if-changed` makes the NEXT build re-verify, but the binary already built keeps `BRUTEX_COMMIT=<HEAD>`. That stamp feeds the `commit` term of the run identity (§3 rule 3). So runs get recorded under a commit whose source did not produce them, which is the outcome build.rs:51-55 says it exists to prevent ("would stamp a result with a commit whose source never produced it").

**Repro:** On a clean HEAD, start `cargo build --release -p cli` (or press Run in the IDE, which docs/07-plan.md §0 names as the operator path). After the `cli` build-script step has run, while `runner`/`engine` or `cli`'s own lib are still compiling, save an edit to `crates/runner/src/rank.rs`. An IDE with autosave does this without being asked. rustc reads the edited file. The finished binary prints a verified commit, and `sweep-stored` records ledger rows with commit = HEAD for code that is not HEAD. The next `cargo build` would rebuild and unstamp, but the bad binary has already run.

**Minimal fix:** A build script cannot see reads that happen after it exits, so the fix belongs at a point after compilation:
- Emit a digest of the verified blob set as a second `rustc-env` (for example `BRUTEX_TREE`).
- Have CI and the launcher re-run `build_provenance::verify` after the binary is linked, and refuse to treat the stamp as authoritative if the two disagree.

At minimum, record the limit in docs/06-limits.md and in the build.rs header. The stamp proves the tree as it was when the build script ran, not as rustc read it. §3 rule 6 requires that limit to be stated.

---

## cli3-3 (low, latent: no production caller today): a kill or power loss mid-`write_all` leaves a ragged tail, and every later open of these ledgers refuses it with no repair path; one comment says this cannot happen

**Where:**
- crates/cli/src/admission_store.rs:1019-1061 (`write_decisions`), with refusal at :1638-1643 (`check_header`).
- The same append shape with the same refuse-on-ragged rule at institutional_statistics.rs:1404-1435 (`append_sync_with`), with refusal at :1330.
- stored_data_completeness.rs:870-893 (`commit`).

```rust
/// Writes `decisions` at `at` as ONE buffer, so a kill can no longer stop
/// between records of one call, and rolls a failed write back to `at`.
...
if let Err(why) = self.decision_file.write_all(&buffer) {
    return Err(rollback_message(&self.decision_file, at, ...));
```
```rust
if !len.saturating_sub(HEADER).is_multiple_of(stride) {
    return Err(format!("{} has length {len}, not a {HEADER}-byte header plus whole {stride}-byte records; a torn/ragged tail is never ignored", ...
```

**Why it is wrong:** The rollback (`set_len(at)`) runs only when `write_all` returns an error. A process killed inside the write never reaches it. The doc's premise, that one buffer means "a kill can no longer stop between records", is false on Linux for a multi-page buffer. `generic_perform_write` copies page by page and stops with the bytes already copied when a fatal signal is pending. The file is then left extended to a page boundary that is generally not a multiple of the record stride. Power loss gives the same result through partially persisted extents. The next `open`/`open_read` hits `check_header`'s ragged-tail refusal, and nothing in the module can truncate or complete it. The D-1630 orphan recovery handles only a whole-record prefix. So one interrupted decision block wedges the whole admission ledger for every population, not just the one being written.

**Repro:** `AdmissionAuthorityLedger::open(root)` → `commit(pop, digest, policy, decisions)` with enough decisions that the buffer spans several pages. SIGKILL the process while it is inside `write_all` (:1047). On restart, `AdmissionAuthorityLedger::open(root)` returns `Err("... not a 16-byte header plus whole N-byte records; a torn/ragged tail is never ignored")`, and does so on every later open.

**Severity:** Low because `AdmissionAuthorityLedger::open`, `InstitutionalStatisticsLedgerV1::open` and the stored-data-completeness writer have no production caller at this commit (gaps-1; only tests open them). It becomes medium the day a verb wires any of them in.

**Minimal fix:** On writable open, treat a ragged tail past the last whole record as the trace of an interrupted append. If the whole-record prefix validates, truncate to it (`set_len`), `sync_all`, and record the event loudly (a telemetry event and the refusal text), consistent with the D-1630 orphan rule. Keep read-only opens refusing. Correct the `write_decisions` doc comment either way.

---

## Checked and clean (no finding)

- **search_checkpoint.rs:** Journal and Snapshot locks are released by explicit unlock. Payload, marker and directory fsyncs are in the right order. The torn `complete` marker and the missing `DIRECTORY_LIMIT` check are already KNOWN (hunt-cli-b re-1/re-2), so I did not repeat them.
- **expression.rs:** `EvidenceWriter` uses a create_new pending file under an exclusive flock, verifies through the locked handle, hard-links to publish (it never replaces), removes the pending file, then fsyncs the directory. Each token gets its own attempt directory, and the leaf `create_dir` refuses an existing token.
- **live.rs:** The temp file is `<hex>.<pid>.tmp`, then `sync_data`, then `rename`. The census ignores strays and sorts by identity, so output is deterministic. The write-once staleness behaviour and the strays left by killed writers are documented in the module.
- **checksum_receipts.rs:** `publish` takes an exclusive try_lock, checks for an exact-prefix completion, writes the payload, fsyncs, writes the seal, fsyncs, then fsyncs the parent directory. Concurrent publishers refuse loudly with WouldBlock. `Receipt` keeps a shared lease and rechecks the generation.
- **global_replay_v4_store.rs:** The exclusive lock covers prefix compare plus append. The seal is written last behind its own fsync, then the directory is fsynced. Readers take a blocking shared lock and check exact length and digest.
- **global_replay_v4_lifecycle.rs:** Every begun attempt gets a terminal (Refused on error, overlapping stages refused).
- **and_checkpoint.rs:** `recover` correctly handles an orphan chunk whose boundary was never acknowledged, and the final boundary is re-verified against its acknowledged seal.
- **index_stop_search.rs:** Results are reassembled by rung index, and a failure keeps the batch pending.
- **index_stop.rs, index_consistency_store.rs, index_stop_qualification.rs:** All call `prepare_in_namespace` unconditionally, with no existence shortcut. Their remaining torn-file exposure is the `write_or_equal` finding in search.md.
- **ledger_v6.rs, ledger_all.rs, research_policy.rs, research.rs, fold_audit.rs, stability.rs, strict_range_*, audited_*, *_codec.rs, main.rs, readonly_file.rs:** No shared mutable state, no lock and no durable write outside test code, except `create_dir_all` of output roots.
- **HashMap/HashSet in the slice:** These are used only for membership or duplicate checks on reachable paths. The admission_store refusal-order nondeterminism is already KNOWN as hunt-conc-3.

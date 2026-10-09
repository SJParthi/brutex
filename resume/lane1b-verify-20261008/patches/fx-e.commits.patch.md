From 71dff6f1bb43f730c6240abf22d22bdbaeda40fa Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:14:04 +0000
Subject: [PATCH 1/5] cli: one Statistics V2 scan per append and per step
 (W2-cli12-1, D-4764)

Before: append_population_statistics_v2 opened the root twice (the
writer's open, then a fresh read-only reopen after dropping the writer),
each re-validating every stored block and rerunning its bootstrap, and
the step-3 orchestrator opened the root a third time for its Admission
V3 projection: three full scans per step, O(A^2) block validations with
constant 2 over A appends.

After: one open. reverify_committed re-reads only the committed block
through the writer's handle (generation current, a written block ends
the file, the index holds exactly this audit, validate_complete_block
recomputes it, the whole file still matches its content generation),
and the door hands that handle to the orchestrator, which projects
through it. The removed second open used to catch an older block edited
during the append; the pre-write check now returns the digest of the
bytes the block lands after from the same hash pass, and every
post-write re-measure must reproduce it. No byte, format, identity or
type changes. LBE-09's two tests keep their names and pin one scan.

Fail-before: one_append_runs_one_full_scan_and_rereads_only_its_block
"A = 0: one open, the new block twice" left (2, 2) right (1, 2);
stored_pair_commits_observation_and_statistics_then_reuses_exact_bytes
"one Statistics V2 scan per written step" left 3 right 1.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/population_statistics_v2.rs    | 626 ++++++++++++++++--
 crates/cli/src/step3_orchestrator.rs          |  34 +-
 .../cli/tests/ledger_append_lookup_costs.rs   |  32 +-
 docs/04-invariants.md                         |   4 +
 docs/05-decisions.md                          |  61 ++
 docs/06-limits.md                             |  30 +-
 6 files changed, 698 insertions(+), 89 deletions(-)

diff --git a/crates/cli/src/population_statistics_v2.rs b/crates/cli/src/population_statistics_v2.rs
index 5da3e0dd..bc716090 100644
--- a/crates/cli/src/population_statistics_v2.rs
+++ b/crates/cli/src/population_statistics_v2.rs
@@ -7,8 +7,13 @@
 //! split-derived PBO, White, SPA and candidate-specific Romano--Wolf evidence.
 //!
 //! The public writer accepts only an opaque prepared capability, syncs the raw
-//! block before its Completion, drops the writable handle and freshly reopens
-//! the exact audit read-only.  The production constructor accepts only a sealed
+//! block before its Completion and then re-reads the exact committed block
+//! through the same handle under a current content generation.  In this module
+//! "freshly reopened" means exactly that: re-read and re-validated from disk
+//! under a generation measured after the write, the bytes before the block
+//! proven unchanged since the open's scan validated them (D-4764).  Before
+//! D-4764 the writer dropped its handle and ran a second full read-only open.
+//! The production constructor accepts only a sealed
 //! paired Observation V1 capability, its exact freshly reopened durable audit,
 //! and the two exact Pre-Admission reopen audits.  Caller-authored period rows,
 //! split scores and digests remain unreachable.
@@ -2679,6 +2684,37 @@ impl PopulationStatisticsV2Ledger {
         require_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
         require_generation(self.data_generation, &self.data_file, &self.data_path)
     }
+
+    /// [`Self::require_unchanged`] that also returns the generation-domain
+    /// digest of the data file's first `cut` bytes, from the same pass.
+    fn require_unchanged_with_prefix(
+        &self,
+        cut: u64,
+    ) -> Result<VerifiedPrefixV2, PopulationStatisticsV2Refusal> {
+        require_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
+        require_metadata_generation(self.data_generation, &self.data_file, &self.data_path)?;
+        let (observed, digest) = measure_generation(
+            &self.data_file,
+            &self.data_path,
+            self.data_generation.len,
+            cut,
+        )?;
+        if observed != self.data_generation {
+            return Err(format!(
+                "{} changed after population-statistics open; cached audit refused",
+                self.data_path.display()
+            ));
+        }
+        Ok(VerifiedPrefixV2 { len: cut, digest })
+    }
+}
+
+/// The first `len` bytes of the data file, named by their generation-domain
+/// digest, as last proven equal to bytes a scan validated (D-4764).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+struct VerifiedPrefixV2 {
+    len: u64,
+    digest: [u8; 32],
 }
 
 fn require_admission_projection_source_v3(
@@ -3032,6 +3068,8 @@ fn validate_complete_block(
     first: u64,
     manifest: &PopulationStatisticsManifestV2,
 ) -> Result<PopulationStatisticsV2ReopenAudit, PopulationStatisticsV2Refusal> {
+    #[cfg(test)]
+    BLOCK_VALIDATIONS.with(|count| count.set(count.get().saturating_add(1)));
     let candidates = validate_candidate_records(file, first, manifest)?;
     let returns = validate_period_records(file, first, manifest, &candidates)?;
     validate_split_records(file, first, manifest, &candidates)?;
@@ -3430,9 +3468,10 @@ impl PreparedObservationStatisticsV2 {
         self.link
     }
 
-    /// Appends existing Statistics V2 bytes receipt-last and freshly reopens them.
+    /// Appends existing Statistics V2 bytes receipt-last and re-reads the
+    /// committed block from disk through the writer's handle (D-4764).
     ///
-    /// The reopened audit is checked again against both exact Pre-Admission
+    /// The re-read audit is checked again against both exact Pre-Admission
     /// audits and the detached Observation link before success is returned.
     ///
     /// # Errors
@@ -3444,14 +3483,35 @@ impl PreparedObservationStatisticsV2 {
         root: impl AsRef<Path>,
         bounds: PopulationStatisticsV2Bounds,
     ) -> Result<PopulationStatisticsObservationCommitV2, PopulationStatisticsV2Refusal> {
-        let append = append_population_statistics_v2(root, bounds, &self.prepared)?;
+        self.append_and_retain_reader(root, bounds)
+            .map(|(commit, _reader)| commit)
+    }
+
+    /// [`Self::append_and_reopen`], handing back the ledger whose open and
+    /// re-read produced the commit, so the step-3 Admission V3 projection reads
+    /// through it instead of opening the root again (W2-cli12-1, D-4764).
+    pub(crate) fn append_and_retain_reader(
+        &self,
+        root: impl AsRef<Path>,
+        bounds: PopulationStatisticsV2Bounds,
+    ) -> Result<
+        (
+            PopulationStatisticsObservationCommitV2,
+            PopulationStatisticsV2Ledger,
+        ),
+        PopulationStatisticsV2Refusal,
+    > {
+        let (append, reader) = append_and_retain_ledger(root.as_ref(), bounds, &self.prepared)?;
         let audit = append.audit();
         audit.verify_pre_admission_pair(self.nifty_pre_admission, self.banknifty_pre_admission)?;
         self.link.require_reopened_statistics(audit)?;
-        Ok(PopulationStatisticsObservationCommitV2 {
-            statistics: append,
-            link: self.link,
-        })
+        Ok((
+            PopulationStatisticsObservationCommitV2 {
+                statistics: append,
+                link: self.link,
+            },
+            reader,
+        ))
     }
 }
 
@@ -3667,6 +3727,27 @@ thread_local! {
     static STATISTICS_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
 }
 
+#[cfg(test)]
+thread_local! {
+    /// Test-only count of complete-block validations (each one re-reads a
+    /// block and reruns its bootstrap) on this thread.
+    static BLOCK_VALIDATIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+}
+
+/// Full Statistics V2 ledger scans run on this thread since the last
+/// [`reset_statistics_scans_for_test`]; test-only, for callers outside this
+/// module that must count the opens one step pays (W2-cli12-1).
+#[cfg(test)]
+pub(crate) fn statistics_scans_for_test() -> u64 {
+    STATISTICS_SCANS.with(std::cell::Cell::get)
+}
+
+/// Zeroes this thread's Statistics V2 scan count; test-only.
+#[cfg(test)]
+pub(crate) fn reset_statistics_scans_for_test() {
+    STATISTICS_SCANS.with(|count| count.set(0));
+}
+
 #[cfg(test)]
 thread_local! {
     /// Test-only count of rows [`candidate_column`] visited on this thread.
@@ -4583,7 +4664,8 @@ fn observation_statistics_link(
     Ok(value)
 }
 
-/// Result of an exact receipt-last append followed by a fresh read-only reopen.
+/// Result of an exact receipt-last append whose committed block was then
+/// re-read from disk under a current generation (D-4764).
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum PopulationStatisticsV2Append {
     /// New Data/raw rows and then Completion were durably appended.
@@ -4593,7 +4675,7 @@ pub enum PopulationStatisticsV2Append {
 }
 
 impl PopulationStatisticsV2Append {
-    /// Freshly reopened audit produced by either exact branch.
+    /// Re-read audit produced by either exact branch.
     #[must_use]
     pub const fn audit(self) -> PopulationStatisticsV2ReopenAudit {
         match self {
@@ -4607,7 +4689,16 @@ impl PopulationStatisticsV2Ledger {
         &mut self,
         prepared: &PreparedPopulationStatisticsV2,
     ) -> Result<PopulationStatisticsV2Append, PopulationStatisticsV2Refusal> {
-        self.require_unchanged()?;
+        // The prefix a new block lands after: the whole file, or the bytes
+        // before a trailing orphan this append resumes or discards. Its digest
+        // comes out of the same pass that proves the file is still the one the
+        // open's scan validated, and every re-measure after an owned write
+        // must reproduce it (D-4764).
+        let cut = match self.orphan {
+            Some(orphan) => record_offset(orphan.first_record)?,
+            None => self.data_generation.len,
+        };
+        let verified = self.require_unchanged_with_prefix(cut)?;
         if let Some(existing) = self.audits.get(&prepared.manifest.audit_id).copied() {
             let planned = prepared.records(existing.sequence(), existing.first_record)?;
             compare_planned_records(&mut self.data_file, existing.first_record, &planned)?;
@@ -4615,7 +4706,7 @@ impl PopulationStatisticsV2Ledger {
         }
         if let Some(orphan) = self.orphan {
             if orphan.manifest.audit_id == prepared.manifest.audit_id {
-                return self.resume_orphan(prepared, &orphan);
+                return self.resume_orphan(prepared, &orphan, verified);
             }
             // A FOREIGN RECEIPT-LESS ORPHAN IS SCRATCH (D-1905, pop2-4): no
             // Completion ever acknowledged it, and refusing every other audit
@@ -4627,19 +4718,17 @@ impl PopulationStatisticsV2Ledger {
                 &format!("audit {}", hex32(orphan.manifest.audit_id)),
             )?;
             self.orphan = None;
-            self.data_generation =
-                file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
+            self.remeasure_after_owned_write(verified)?;
         }
         if self.completed_audits >= self.bounds.audits {
             return Err("population-statistics append reached audit bound".to_owned());
         }
         self.bounds.validate_manifest(&prepared.manifest)?;
-        let first = record_count(
-            self.data_file
-                .metadata()
-                .map_err(|why| format!("cannot stat append file: {why}"))?
-                .len(),
-        )?;
+        // The block is planned right after the verified prefix, never after
+        // bytes nothing verified: a file that grew since `verified` was
+        // measured fails the block re-read and the reverify's record count
+        // (D-4764).
+        let first = record_count(verified.len)?;
         let planned = prepared.records(self.completed_audits, first)?;
         let added_bytes = u64_of(planned.len(), "planned records")?
             .checked_mul(POPULATION_STATISTICS_V2_RECORD_STRIDE)
@@ -4698,8 +4787,7 @@ impl PopulationStatisticsV2Ledger {
             .completed_audits
             .checked_add(1)
             .ok_or_else(|| "completed audit count overflowed".to_owned())?;
-        self.data_generation =
-            file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
+        self.remeasure_after_owned_write(verified)?;
         Ok(PopulationStatisticsV2Append::Written(audit))
     }
 
@@ -4707,6 +4795,7 @@ impl PopulationStatisticsV2Ledger {
         &mut self,
         prepared: &PreparedPopulationStatisticsV2,
         orphan: &OrphanV2,
+        verified: VerifiedPrefixV2,
     ) -> Result<PopulationStatisticsV2Append, PopulationStatisticsV2Refusal> {
         if orphan.manifest.audit_id != prepared.manifest.audit_id {
             return Err(format!(
@@ -4786,11 +4875,124 @@ impl PopulationStatisticsV2Ledger {
             .checked_add(1)
             .ok_or_else(|| "completed audit count overflowed".to_owned())?;
         self.orphan = None;
-        self.data_generation =
-            file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?;
+        self.remeasure_after_owned_write(verified)?;
         Ok(PopulationStatisticsV2Append::Written(audit))
     }
 
+    /// Re-measures the data generation after this handle wrote, and refuses
+    /// unless the bytes before the block it wrote still hash to the digest the
+    /// pre-write check measured over the scan-validated file. A blind
+    /// re-measure would adopt any non-cooperating edit made to an older block
+    /// during the append, which the second full open D-4764 removed used to
+    /// catch.
+    fn remeasure_after_owned_write(
+        &mut self,
+        verified: VerifiedPrefixV2,
+    ) -> Result<(), PopulationStatisticsV2Refusal> {
+        let (observed, prefix) = measure_generation(
+            &self.data_file,
+            &self.data_path,
+            self.bounds.file_bytes,
+            verified.len,
+        )?;
+        if prefix != verified.digest {
+            return Err(format!(
+                "{} changed below byte {} during the population-statistics append; refused",
+                self.data_path.display(),
+                verified.len
+            ));
+        }
+        self.data_generation = observed;
+        Ok(())
+    }
+
+    /// Re-reads one block this handle committed and returns its audit, under
+    /// the shared lock: the generation must be current, the file must end where
+    /// a written block ends (or hold a reused one), the index must hold exactly
+    /// this audit, the block is re-read and recomputed from disk by
+    /// [`validate_complete_block`], the same function a full open runs, and the
+    /// whole file must still match the content generation measured after the
+    /// write. That generation was itself measured only after the bytes before
+    /// the block were proven unchanged since the open's scan, so every byte of
+    /// the file is accounted for without a second full open (W2-cli12-1,
+    /// D-4764). Cost: O(this block + bootstrap) plus one content generation of
+    /// the file (two hash passes).
+    ///
+    /// # Errors
+    ///
+    /// Refuses a changed or replaced file, a physical record count other than
+    /// the committed one, an audit this handle did not index, a block that no
+    /// longer recomputes, any re-read difference, lock or I/O failure.
+    pub(crate) fn reverify_committed(
+        &mut self,
+        committed: &PopulationStatisticsV2Append,
+    ) -> Result<PopulationStatisticsV2ReopenAudit, PopulationStatisticsV2Refusal> {
+        self.lock_file
+            .lock_shared()
+            .map_err(|why| format!("cannot take population-statistics reverify lock: {why}"))?;
+        let result = self.reverify_committed_locked(committed);
+        let released = self
+            .lock_file
+            .unlock()
+            .map_err(|why| format!("cannot release population-statistics reverify lock: {why}"));
+        match (result, released) {
+            (Ok(value), Ok(())) => Ok(value),
+            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
+        }
+    }
+
+    fn reverify_committed_locked(
+        &mut self,
+        committed: &PopulationStatisticsV2Append,
+    ) -> Result<PopulationStatisticsV2ReopenAudit, PopulationStatisticsV2Refusal> {
+        let expected = committed.audit();
+        require_metadata_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
+        require_metadata_generation(self.data_generation, &self.data_file, &self.data_path)?;
+        let indexed = self
+            .audits
+            .get(&expected.audit_id())
+            .copied()
+            .ok_or_else(|| {
+                format!(
+                    "population-statistics committed audit {} is absent from this handle's index",
+                    hex32(expected.audit_id())
+                )
+            })?;
+        if indexed != expected {
+            return Err(format!(
+                "population-statistics committed audit {} differs from this handle's index",
+                hex32(expected.audit_id())
+            ));
+        }
+        let block_end = expected
+            .first_record
+            .checked_add(expected.manifest.block_record_count()?)
+            .ok_or_else(|| "population-statistics committed block end overflowed".to_owned())?;
+        let records = record_count(self.data_generation.len)?;
+        let in_place = match committed {
+            PopulationStatisticsV2Append::Written(_) => records == block_end,
+            PopulationStatisticsV2Append::Reused(_) => records >= block_end,
+        };
+        if !in_place {
+            return Err(format!(
+                "population-statistics file holds {records} records; the committed block ends at record {block_end}"
+            ));
+        }
+        let reread = validate_complete_block(
+            &mut self.data_file,
+            expected.first_record,
+            &expected.manifest,
+        )?;
+        if reread != expected {
+            return Err(format!(
+                "population-statistics audit {} did not re-read with exact semantics",
+                hex32(expected.audit_id())
+            ));
+        }
+        self.require_unchanged()?;
+        Ok(reread)
+    }
+
     fn append(
         &mut self,
         prepared: &PreparedPopulationStatisticsV2,
@@ -4810,13 +5012,17 @@ impl PopulationStatisticsV2Ledger {
     }
 }
 
-/// Durably appends one opaque prepared block and freshly reopens it read-only.
+/// Durably appends one opaque prepared block and re-reads it from disk.
 ///
 /// The directory must already exist.  The writer never manufactures a missing
 /// configured root.  Data, candidate, period and split records are synced
 /// before the Completion record is appended and synced.  Success is returned
-/// only after dropping the writable ledger, opening a new read-only ledger and
-/// finding the byte-identical recomputed audit.
+/// only after [`PopulationStatisticsV2Ledger::reverify_committed`] re-read and
+/// recomputed exactly the committed block through the writer's own handle and
+/// the whole file still matched the content generation measured after the
+/// write (W2-cli12-1, D-4764). One call is one full open (O(sum over the A
+/// stored audits of (C·(P+S) + bootstrap))) plus O(the committed block); before
+/// D-4764 a second full read-only open followed.
 ///
 /// This is a durability boundary, not a preparation authority: callers cannot
 /// construct [`PreparedPopulationStatisticsV2`] through the public API.
@@ -4831,30 +5037,32 @@ pub fn append_population_statistics_v2(
     bounds: PopulationStatisticsV2Bounds,
     prepared: &PreparedPopulationStatisticsV2,
 ) -> Result<PopulationStatisticsV2Append, PopulationStatisticsV2Refusal> {
-    let root = root.as_ref();
-    let mut writer = PopulationStatisticsV2Ledger::open_writer(root, bounds)?;
-    let committed = writer.append(prepared)?;
-    let expected = committed.audit();
-    drop(writer);
-
-    let reopened = PopulationStatisticsV2Ledger::open_read(root, bounds)?
-        .reopen_audit(&expected.audit_id())?
-        .ok_or_else(|| {
-            format!(
-                "population-statistics audit {} disappeared after receipt-last append",
-                hex32(expected.audit_id())
-            )
-        })?;
-    if reopened != expected {
-        return Err(format!(
-            "population-statistics audit {} did not freshly reopen with exact semantics",
-            hex32(expected.audit_id())
-        ));
-    }
-    Ok(match committed {
-        PopulationStatisticsV2Append::Written(_) => PopulationStatisticsV2Append::Written(reopened),
-        PopulationStatisticsV2Append::Reused(_) => PopulationStatisticsV2Append::Reused(reopened),
-    })
+    append_and_retain_ledger(root.as_ref(), bounds, prepared).map(|(append, _ledger)| append)
+}
+
+/// The append door's body: ONE open, the append, and a re-read of only the
+/// committed block through the same handle, which is then handed back so a
+/// caller that reads the root next (the step-3 Admission V3 projection) does
+/// not open and re-validate it a third time (W2-cli12-1, D-4764).
+///
+/// The handed-back ledger was opened writable, but its only write path,
+/// `append`, is private to this module, so no caller can write through it.
+fn append_and_retain_ledger(
+    root: &Path,
+    bounds: PopulationStatisticsV2Bounds,
+    prepared: &PreparedPopulationStatisticsV2,
+) -> Result<
+    (PopulationStatisticsV2Append, PopulationStatisticsV2Ledger),
+    PopulationStatisticsV2Refusal,
+> {
+    let mut ledger = PopulationStatisticsV2Ledger::open_writer(root, bounds)?;
+    let committed = ledger.append(prepared)?;
+    let reread = ledger.reverify_committed(&committed)?;
+    let append = match committed {
+        PopulationStatisticsV2Append::Written(_) => PopulationStatisticsV2Append::Written(reread),
+        PopulationStatisticsV2Append::Reused(_) => PopulationStatisticsV2Append::Reused(reread),
+    };
+    Ok((append, ledger))
 }
 
 fn compare_planned_records(
@@ -5699,6 +5907,18 @@ fn file_generation(
     path: &Path,
     max_bytes: u64,
 ) -> Result<FileGenerationV2, PopulationStatisticsV2Refusal> {
+    measure_generation(file, path, max_bytes, 0).map(|(generation, _)| generation)
+}
+
+/// [`file_generation`], also returning the generation-domain digest of the
+/// first `cut` bytes, taken from the first of its two hash passes. A `cut`
+/// past the measured length refuses: the bytes it names are gone.
+fn measure_generation(
+    file: &File,
+    path: &Path,
+    max_bytes: u64,
+    cut: u64,
+) -> Result<(FileGenerationV2, [u8; 32]), PopulationStatisticsV2Refusal> {
     let held_before = file
         .metadata()
         .map_err(|why| format!("cannot stat held {}: {why}", path.display()))?;
@@ -5724,7 +5944,13 @@ fn file_generation(
             path.display()
         ));
     }
-    let content_digest = hash_file(&mut named, path, measured_len)?;
+    if cut > measured_len {
+        return Err(format!(
+            "{} holds {measured_len} bytes, fewer than the {cut} it held when last verified",
+            path.display()
+        ));
+    }
+    let (prefix_digest, content_digest) = hash_file_split(&mut named, path, measured_len, cut)?;
     let repeated_digest = hash_file(&mut named, path, measured_len)?;
     if repeated_digest != content_digest {
         return Err(format!(
@@ -5762,7 +5988,7 @@ fn file_generation(
             path.display()
         ));
     }
-    Ok(generation_of(&held_after, content_digest))
+    Ok((generation_of(&held_after, content_digest), prefix_digest))
 }
 
 #[cfg(unix)]
@@ -5793,12 +6019,44 @@ fn hash_file(
     path: &Path,
     exact_bytes: u64,
 ) -> Result<[u8; 32], PopulationStatisticsV2Refusal> {
+    hash_file_split(file, path, exact_bytes, 0).map(|(_, digest)| digest)
+}
+
+/// One generation-hash read pass over exactly `exact_bytes`, returning the
+/// digest of the first `cut` bytes and of all of them. The first `cut` bytes
+/// feed two hashers from the same buffer, so the file is read once. `cut`
+/// must not exceed `exact_bytes`.
+fn hash_file_split(
+    file: &mut File,
+    path: &Path,
+    exact_bytes: u64,
+    cut: u64,
+) -> Result<([u8; 32], [u8; 32]), PopulationStatisticsV2Refusal> {
     file.seek(SeekFrom::Start(0))
         .map_err(|why| format!("cannot seek {} for generation hash: {why}", path.display()))?;
-    let mut hasher = Hasher::new();
-    hasher.update(GENERATION_DOMAIN);
+    let rest = exact_bytes
+        .checked_sub(cut)
+        .ok_or_else(|| "generation hash cut lies past the measured bytes".to_owned())?;
+    let mut prefix = Hasher::new();
+    prefix.update(GENERATION_DOMAIN);
+    let mut whole = Hasher::new();
+    whole.update(GENERATION_DOMAIN);
+    hash_exact(file, path, &mut [&mut prefix, &mut whole], cut, exact_bytes)?;
+    hash_exact(file, path, &mut [&mut whole], rest, exact_bytes)?;
+    Ok((prefix.finalize(), whole.finalize()))
+}
+
+/// Feeds exactly `bytes` more bytes of `file` into every hasher in `hashers`;
+/// `exact_bytes` only names the generation in the refusal.
+fn hash_exact(
+    file: &mut File,
+    path: &Path,
+    hashers: &mut [&mut Hasher],
+    bytes: u64,
+    exact_bytes: u64,
+) -> Result<(), PopulationStatisticsV2Refusal> {
     let mut buffer = [0_u8; READ_CHUNK_BYTES];
-    let mut remaining = exact_bytes;
+    let mut remaining = bytes;
     while remaining != 0 {
         let requested = usize_of(
             remaining.min(READ_CHUNK_BYTES as u64),
@@ -5817,11 +6075,12 @@ fn hash_file(
                 path.display()
             ));
         }
-        hasher.update(
-            buffer
-                .get(..read)
-                .ok_or_else(|| "generation hash read exceeded buffer".to_owned())?,
-        );
+        let chunk = buffer
+            .get(..read)
+            .ok_or_else(|| "generation hash read exceeded buffer".to_owned())?;
+        for hasher in hashers.iter_mut() {
+            hasher.update(chunk);
+        }
         remaining = remaining
             .checked_sub(
                 u64::try_from(read)
@@ -5829,7 +6088,39 @@ fn hash_file(
             )
             .ok_or_else(|| "generation hash remaining-byte underflowed".to_owned())?;
     }
-    Ok(hasher.finalize())
+    Ok(())
+}
+
+/// The metadata half of [`require_generation`]: the held file and the file the
+/// path names (not followed through a link) must be one inode whose length and
+/// nanosecond modification/change times are the cached ones. It reads no
+/// content.
+fn require_metadata_generation(
+    expected: FileGenerationV2,
+    file: &File,
+    path: &Path,
+) -> Result<(), PopulationStatisticsV2Refusal> {
+    let held = file
+        .metadata()
+        .map_err(|why| format!("cannot stat held {}: {why}", path.display()))?;
+    let named = std::fs::symlink_metadata(path)
+        .map_err(|why| format!("cannot stat named {}: {why}", path.display()))?;
+    #[cfg(unix)]
+    if (held.dev(), held.ino()) != (named.dev(), named.ino()) {
+        return Err(format!("held file no longer names {}", path.display()));
+    }
+    // One comparison of both measurements: on Unix they are one inode.
+    if (
+        generation_of(&held, expected.content_digest),
+        generation_of(&named, expected.content_digest),
+    ) != (expected, expected)
+    {
+        return Err(format!(
+            "{} changed after population-statistics open; cached audit refused",
+            path.display()
+        ));
+    }
+    Ok(())
 }
 
 fn require_generation(
@@ -5837,6 +6128,7 @@ fn require_generation(
     file: &File,
     path: &Path,
 ) -> Result<(), PopulationStatisticsV2Refusal> {
+    require_metadata_generation(expected, file, path)?;
     if file_generation(file, path, expected.len)? != expected {
         return Err(format!(
             "{} changed after population-statistics open; cached audit refused",
@@ -6565,18 +6857,220 @@ mod tests {
         );
     }
 
+    /// Zeroes both test counters and returns a closure reading them.
+    fn counted() -> impl Fn() -> (u64, u64) {
+        STATISTICS_SCANS.with(|count| count.set(0));
+        BLOCK_VALIDATIONS.with(|count| count.set(0));
+        || {
+            (
+                STATISTICS_SCANS.with(std::cell::Cell::get),
+                BLOCK_VALIDATIONS.with(std::cell::Cell::get),
+            )
+        }
+    }
+
+    /// LBE-09 names this test, and invariant rows are append-only, so the name
+    /// stays. What it pinned, two full scans per append, D-4764 removed: it now
+    /// witnesses that one written and one reused append each scan once.
+    /// `one_append_runs_one_full_scan_and_rereads_only_its_block` is the proof.
     #[test]
     fn one_append_runs_two_full_scans_as_section_154_states() {
-        // W2-cli12-1 / D-1682: the cost is documented, not removed.
         let root = TempRoot::new("two-scans");
         STATISTICS_SCANS.with(|count| count.set(0));
         append_population_statistics_v2(root.path(), bounds(), &fixture(5))
             .expect("the first append writes");
-        assert_eq!(STATISTICS_SCANS.with(std::cell::Cell::get), 2);
+        assert_eq!(STATISTICS_SCANS.with(std::cell::Cell::get), 1);
         STATISTICS_SCANS.with(|count| count.set(0));
         append_population_statistics_v2(root.path(), bounds(), &fixture(5))
             .expect("the retry is reused");
-        assert_eq!(STATISTICS_SCANS.with(std::cell::Cell::get), 2);
+        assert_eq!(STATISTICS_SCANS.with(std::cell::Cell::get), 1);
+    }
+
+    #[test]
+    fn one_append_runs_one_full_scan_and_rereads_only_its_block() {
+        // W2-cli12-1: the door opened the ledger twice (the writer's open and
+        // a fresh read-only reopen), so every append re-validated every stored
+        // block twice. It now opens once and re-reads only its own block.
+        // (scans, block validations) for A audits already stored: the open
+        // validates A blocks, a written append validates its new block, and
+        // the reverify re-reads that one block, so A + 2 (A + 1 when reused).
+        let root = TempRoot::new("one-scan");
+        let read = counted();
+        let first = append_population_statistics_v2(root.path(), bounds(), &fixture(5))
+            .expect("the first append writes");
+        assert!(matches!(first, PopulationStatisticsV2Append::Written(_)));
+        assert_eq!(read(), (1, 2), "A = 0: one open, the new block twice");
+
+        let read = counted();
+        let second = append_population_statistics_v2(root.path(), bounds(), &fixture(6))
+            .expect("a second audit writes");
+        assert!(matches!(second, PopulationStatisticsV2Append::Written(_)));
+        assert_eq!(
+            read(),
+            (1, 3),
+            "A = 1: the stored block once, the new block twice"
+        );
+
+        let read = counted();
+        let reused = append_population_statistics_v2(root.path(), bounds(), &fixture(5))
+            .expect("the exact retry is reused");
+        assert!(matches!(reused, PopulationStatisticsV2Append::Reused(_)));
+        assert_eq!(
+            read(),
+            (1, 3),
+            "A = 2 reused: two stored blocks, then the reused block"
+        );
+
+        let orphan_root = TempRoot::new("one-scan-orphan");
+        orphan_fixture(orphan_root.path(), &fixture(2));
+        let read = counted();
+        let resumed = append_population_statistics_v2(orphan_root.path(), bounds(), &fixture(2))
+            .expect("the exact orphan retry completes");
+        assert!(matches!(resumed, PopulationStatisticsV2Append::Written(_)));
+        assert_eq!(read(), (1, 2), "orphan resume: the completed block twice");
+    }
+
+    /// Flips one byte in place at absolute `offset`, without resealing.
+    fn flip_byte(path: &Path, offset: u64) {
+        let mut file = OpenOptions::new()
+            .read(true)
+            .write(true)
+            .open(path)
+            .expect("fixture file opens");
+        let mut byte = [0_u8; 1];
+        file.seek(SeekFrom::Start(offset))
+            .and_then(|_| file.read_exact(&mut byte))
+            .expect("fixture byte reads");
+        byte[0] ^= 0x01;
+        file.seek(SeekFrom::Start(offset))
+            .and_then(|_| file.write_all(&byte))
+            .and_then(|()| file.sync_all())
+            .expect("fixture byte writes");
+    }
+
+    #[test]
+    fn a_post_write_remeasure_adopts_only_a_reproduced_prefix() {
+        // D-4764: the removed second full open caught an older block edited by
+        // a non-cooperating writer while this handle appended. The re-measure
+        // after an owned write now refuses unless the bytes below the cut hash
+        // to the digest measured, in the same pass as the unchanged check,
+        // before the write.
+        let root = TempRoot::new("prefix-window");
+        append_population_statistics_v2(root.path(), bounds(), &fixture(5))
+            .expect("first audit writes");
+        let path = root.path().join(DATA_FILE);
+        let mut ledger =
+            PopulationStatisticsV2Ledger::open_writer(root.path(), bounds()).expect("writer opens");
+        let cut = ledger.data_generation.len;
+        assert_eq!(cut, record_offset(14).expect("offset"));
+        let verified = ledger
+            .require_unchanged_with_prefix(cut)
+            .expect("an unchanged file verifies");
+        assert_eq!(verified.len, cut);
+
+        // The split pass agrees with one-shot hashes of the prefix and the file.
+        let mut named = open_file(&path, false, false).expect("named file opens");
+        assert_eq!(
+            hash_file_split(&mut named, &path, cut, cut / 2).expect("split hashes"),
+            (
+                hash_file(&mut named, &path, cut / 2).expect("prefix hashes"),
+                hash_file(&mut named, &path, cut).expect("file hashes")
+            )
+        );
+        assert_eq!(
+            verified.digest,
+            hash_file(&mut named, &path, cut).expect("file hashes"),
+            "the verified prefix of the whole file is the file's content digest"
+        );
+        assert!(
+            measure_generation(&ledger.data_file, &path, cut, cut + 1)
+                .expect_err("a cut past the file refuses")
+                .contains("fewer than the")
+        );
+
+        // An owned write past the cut is adopted.
+        write_bytes(&path, &[0_u8; RECORD_BYTES]);
+        ledger
+            .remeasure_after_owned_write(verified)
+            .expect("an untouched prefix is adopted");
+        assert_eq!(
+            ledger.data_generation.len,
+            cut + POPULATION_STATISTICS_V2_RECORD_STRIDE
+        );
+        let adopted = ledger.data_generation;
+
+        // An edit below the cut is refused and nothing is adopted.
+        flip_byte(&path, record_offset(3).expect("offset") + 40);
+        let why = ledger
+            .remeasure_after_owned_write(verified)
+            .expect_err("an edited prefix is refused");
+        assert!(why.contains("changed below byte"), "{why}");
+        assert_eq!(ledger.data_generation, adopted);
+    }
+
+    #[test]
+    fn reverify_rereads_only_what_this_handle_committed() {
+        let root = TempRoot::new("reverify");
+        let first = append_population_statistics_v2(root.path(), bounds(), &fixture(5))
+            .expect("first audit writes")
+            .audit();
+        append_population_statistics_v2(root.path(), bounds(), &fixture(6))
+            .expect("second audit writes");
+        let path = root.path().join(DATA_FILE);
+        let mut ledger =
+            PopulationStatisticsV2Ledger::open_read(root.path(), bounds()).expect("reader opens");
+
+        assert_eq!(
+            ledger
+                .reverify_committed(&PopulationStatisticsV2Append::Reused(first))
+                .expect("a reused block before the end re-reads"),
+            first
+        );
+        let why = ledger
+            .reverify_committed(&PopulationStatisticsV2Append::Written(first))
+            .expect_err("a written block must end the file");
+        assert!(
+            why.contains("holds 28 records; the committed block ends at record 14"),
+            "{why}"
+        );
+
+        let mut foreign = first;
+        foreign.manifest.audit_id = [0xEE; 32];
+        let why = ledger
+            .reverify_committed(&PopulationStatisticsV2Append::Reused(foreign))
+            .expect_err("an audit this handle did not index");
+        assert!(why.contains("absent from this handle's index"), "{why}");
+
+        let mut differs = first;
+        differs.completion_record_digest = [0x11; 32];
+        let why = ledger
+            .reverify_committed(&PopulationStatisticsV2Append::Reused(differs))
+            .expect_err("an audit unlike the index");
+        assert!(why.contains("differs from this handle's index"), "{why}");
+
+        ledger.audits.insert(first.audit_id(), differs);
+        let why = ledger
+            .reverify_committed(&PopulationStatisticsV2Append::Reused(differs))
+            .expect_err("the disk disagrees with the index");
+        assert!(
+            why.contains("did not re-read with exact semantics"),
+            "{why}"
+        );
+        ledger.audits.insert(first.audit_id(), first);
+
+        // A same-length edit to ANOTHER block, under metadata re-measured to
+        // match (a rewrite inside one timestamp tick): only the content hash
+        // the re-read still runs can see it.
+        flip_byte(&path, record_offset(20).expect("offset") + 40);
+        let metadata = std::fs::metadata(&path).expect("ledger measures");
+        ledger.data_generation = generation_of(&metadata, ledger.data_generation.content_digest);
+        let why = ledger
+            .reverify_committed(&PopulationStatisticsV2Append::Reused(first))
+            .expect_err("the content generation still refuses");
+        assert!(
+            why.contains("changed after population-statistics open"),
+            "{why}"
+        );
     }
 
     #[test]
diff --git a/crates/cli/src/step3_orchestrator.rs b/crates/cli/src/step3_orchestrator.rs
index 4fb6d042..b7e34dcb 100644
--- a/crates/cli/src/step3_orchestrator.rs
+++ b/crates/cli/src/step3_orchestrator.rs
@@ -2495,8 +2495,11 @@ pub(crate) fn commit_stored_observation_statistics_v2(
         "before Statistics V2 receipt-last append/reopen",
     )?;
     roots.require_same("before Statistics V2 receipt-last append/reopen")?;
-    let statistics = prepared_statistics
-        .append_and_reopen(roots.statistics.path(), statistics_bounds)
+    // The writer's own handle comes back with the commit and is the Admission
+    // V3 reader below: the root is scanned once per step, not three times
+    // (W2-cli12-1, D-4764).
+    let (statistics, statistics_reader) = prepared_statistics
+        .append_and_retain_reader(roots.statistics.path(), statistics_bounds)
         .map_err(|why| format!("Step 3 Statistics V2 commit refused: {why}"))?;
     roots.require_same("after Statistics V2 receipt-last append/reopen")?;
     require_exact_stored_source_root_pair_v2(
@@ -2518,11 +2521,11 @@ pub(crate) fn commit_stored_observation_statistics_v2(
         "after final Observation-to-Statistics projection revalidation",
     )?;
     let (statistics_reader, admission_projection) = prepare_admission_statistics_projection_v3(
+        statistics_reader,
         &roots,
         &projection,
         &nifty_source,
         &banknifty_source,
-        statistics_bounds,
     )?;
 
     Ok(CommittedStoredObservationStatisticsV2 {
@@ -2540,12 +2543,15 @@ pub(crate) fn commit_stored_observation_statistics_v2(
     })
 }
 
+/// Scans the complete committed family through `reader`, the handle whose open
+/// and re-read produced the Statistics commit (D-4764), rather than opening the
+/// root a third time.
 fn prepare_admission_statistics_projection_v3(
+    mut reader: PopulationStatisticsV2Ledger,
     roots: &AdmittedObservationStatisticsRootsV2,
     projection: &PopulationStatisticsV2ProjectionSource,
     nifty_source: &CommittedStoredCandidatePreAdmissionV1,
     banknifty_source: &CommittedStoredCandidatePreAdmissionV1,
-    statistics_bounds: PopulationStatisticsV2Bounds,
 ) -> Result<
     (
         PopulationStatisticsV2Ledger,
@@ -2553,11 +2559,6 @@ fn prepare_admission_statistics_projection_v3(
     ),
     Step3OrchestratorRefusal,
 > {
-    let mut reader =
-        PopulationStatisticsV2Ledger::open_read(roots.statistics.path(), statistics_bounds)
-            .map_err(|why| {
-                format!("Step 3 Admission V3 Statistics read-only reopen refused: {why}")
-            })?;
     let authority = reader
         .prepare_admission_projection_v3(projection)
         .map_err(|why| {
@@ -5533,6 +5534,7 @@ mod tests {
             &long,
             &short,
         )?;
+        crate::population_statistics_v2::reset_statistics_scans_for_test();
         let mut first = commit_stored_observation_statistics_v2(
             first_nifty,
             first_banknifty,
@@ -5542,6 +5544,14 @@ mod tests {
             statistics_bounds,
             procedure,
         )?;
+        // W2-cli12-1: the step scanned the Statistics root three times (the
+        // writer's open, a fresh reopen, and the Admission V3 reader's open).
+        // The writer's handle is now the reader, so the step scans it once.
+        assert_eq!(
+            crate::population_statistics_v2::statistics_scans_for_test(),
+            1,
+            "one Statistics V2 scan per written step"
+        );
 
         assert!(matches!(
             first.observation_commit(),
@@ -6022,6 +6032,7 @@ mod tests {
             &long,
             &short,
         )?;
+        crate::population_statistics_v2::reset_statistics_scans_for_test();
         let mut retry = commit_stored_observation_statistics_v2(
             retry_nifty,
             retry_banknifty,
@@ -6031,6 +6042,11 @@ mod tests {
             statistics_bounds,
             procedure,
         )?;
+        assert_eq!(
+            crate::population_statistics_v2::statistics_scans_for_test(),
+            1,
+            "one Statistics V2 scan per reused step"
+        );
         assert!(matches!(
             retry.observation_commit(),
             ObservationAuthorityCommitV1::Reused(_)
diff --git a/crates/cli/tests/ledger_append_lookup_costs.rs b/crates/cli/tests/ledger_append_lookup_costs.rs
index 9467f410..c7d06e1e 100644
--- a/crates/cli/tests/ledger_append_lookup_costs.rs
+++ b/crates/cli/tests/ledger_append_lookup_costs.rs
@@ -16,6 +16,9 @@
 //!   per rung, and its replay's full-route cost was stated nowhere.
 //! * W2-cli3-3 (D-1684): the stored OOS source was rebuilt for every witness
 //!   while §169 priced minting by the replay alone.
+//! * W2-cli12-1 (D-4764): the two-open Statistics V2 append and the step's
+//!   third open are one open per step; the tests named in LBE-09 keep their
+//!   names and now pin the one open.
 //!
 //! Every file a constant below names is read at compile time, so a rename
 //! fails the build rather than skipping the check. A separate test crate, as
@@ -35,6 +38,7 @@ const CANDIDATE: &str = include_str!("../src/candidate_universe.rs");
 const PRE_ADMISSION: &str = include_str!("../src/pre_admission_data.rs");
 const OBSERVATIONS: &str = include_str!("../src/population_observations_v1.rs");
 const STATISTICS: &str = include_str!("../src/population_statistics_v2.rs");
+const STEP3: &str = include_str!("../src/step3_orchestrator.rs");
 const LEDGER_V6: &str = include_str!("../src/ledger_v6.rs");
 const STRICT_INPUTS: &str = include_str!("../src/strict_v6_inputs.rs");
 const POPULATION_V6: &str = include_str!("../src/population_v6.rs");
@@ -203,19 +207,37 @@ fn section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append()
     let open = method(STATISTICS, "    fn open_inner(");
     assert!(open.contains("bounds.audits.min(stored_records)"));
     assert!(!open.contains("try_reserve(usize_of(bounds.audits,"));
-    let append = function(STATISTICS, "pub fn append_population_statistics_v2(");
-    assert!(append.contains("PopulationStatisticsV2Ledger::open_writer(root, bounds)?"));
-    assert!(append.contains("PopulationStatisticsV2Ledger::open_read(root, bounds)?"));
+    // One open per append and per step since D-4764: the door re-reads its
+    // block through the writer's handle and hands that handle to the step.
+    let door = function(STATISTICS, "pub fn append_population_statistics_v2(");
+    assert!(door.contains("append_and_retain_ledger(") && !door.contains("open_read("));
+    let append = function(STATISTICS, "fn append_and_retain_ledger(");
+    assert_eq!(append.matches("open_writer(").count(), 1);
+    assert!(
+        !append.contains("open_read("),
+        "the append reopens its root again; re-measure §154"
+    );
+    assert!(append.contains("ledger.reverify_committed(&committed)?"));
+    let projection = function(STEP3, "fn prepare_admission_statistics_projection_v3(");
+    assert!(
+        !projection.contains("open_read("),
+        "the step opens its Statistics root a third time; re-measure §154"
+    );
 
     let text = flat(section(154));
     for needed in [
         "so the per-candidate summaries cost O(C·(P+S)) in total",
         "never the configured `max_audits` ceiling",
-        "One append through `append_population_statistics_v2` runs two full opens",
-        "A appends to one root cost O(A²) block validations in total",
+        "One append through `append_population_statistics_v2` runs one full open",
+        "A appends to one root cost O(A²) block validations in total, with constant 1",
+        "a step scans its Statistics root once where it scanned three times",
     ] {
         assert!(text.contains(needed), "§154 no longer says `{needed}`");
     }
+    assert!(
+        !text.contains("runs two full opens"),
+        "§154 still prices two opens"
+    );
 }
 
 #[test]
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 31e467ea..326b86a3 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7052,3 +7052,7 @@ old line regex the same input and watched it pass.
 | G18-api-27 | The seek path's `records unreadable` line names the first file that refused a record, not the first file read (D-2046) | `api::bars::window_tests::the_unreadable_line_names_the_first_damaged_file_not_the_first_file` | ✓ |
 | G18-api-28 | The route test's HTTP exchange is bounded at 30 s per read and write, so a server that admits or answers nothing fails it rather than hanging (D-2047) | `api::ingest::route_tests::the_three_routes_answer_and_none_of_them_shadows_the_front_end` | ✓ |
 | G18-api-29 | A dropped calendar `Landing` marks its flight `Abandoned` (or answered), removes it from the flight table, and wakes every follower (D-2047) | `api::calendar_of::tests::a_calendar_landing_releases_its_flight_and_wakes_its_followers_when_dropped` | ✓ |
+| L1FE-01 | One Statistics V2 append through the door, written, reused or completing an orphan, runs exactly one full scan and validates only its own block beyond the open (A + 2 block validations written, A + 1 reused); the step-3 Statistics commit scans its root once per written and per reused step (D-4764) | `cli::population_statistics_v2::tests::one_append_runs_one_full_scan_and_rereads_only_its_block`, `cli::population_statistics_v2::tests::one_append_runs_two_full_scans_as_section_154_states`, `cli::step3_orchestrator::tests::stored_pair_commits_observation_and_statistics_then_reuses_exact_bytes` | ✓ |
+| L1FE-02 | After an owned write the Statistics V2 generation is adopted only when the bytes below the cut reproduce the digest measured before the write; an edit below the cut refuses and nothing is adopted; the one-pass split hash equals one-shot hashes, and a cut past the file refuses (D-4764) | `cli::population_statistics_v2::tests::a_post_write_remeasure_adopts_only_a_reproduced_prefix` | ✓ |
+| L1FE-03 | `reverify_committed` re-reads a reused block; it refuses a written block that does not end the file, an audit absent from or unlike the index, a disk block unlike the index, and a same-length edit elsewhere under re-measured metadata (D-4764) | `cli::population_statistics_v2::tests::reverify_rereads_only_what_this_handle_committed` | ✓ |
+| L1FE-04 | The Statistics V2 door opens once and never read-only, the step's projection opens nothing, and §154 prices one open per append and one scan per step (D-4764) | `cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 552ce57a..c15bd49d 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65025,3 +65025,64 @@ on a live leader would derive a second time and lose the single-flight
 guarantee D-1443 exists for. **Honest limit:** the `Landing` kill depends on
 test order. A rename that sorted a single-flight test ahead of it would
 restore the timeout, so the ordering is pinned in the test's own doc.
+
+### D-4764 — A Statistics V2 append re-reads its block through the writer's handle, and the step reads through it: one full scan per append and per step — 2026-10-09
+
+**What was wrong.** W2-cli12-1: `append_population_statistics_v2` ran two full
+opens, the writer's and a fresh read-only reopen after the writer was dropped.
+Each re-validated every stored block and reran every stored bootstrap. The
+step-3 orchestrator then opened the root a third time for its Admission V3
+projection. One step paid three full scans, and A appends cost O(A²) block
+validations with constant 2. D-1682 stated the cost and kept the reopen.
+
+**Decided.** One open. After the receipt-last write, `reverify_committed`
+re-reads exactly the committed block through the writer's own handle, under the
+shared lock:
+
+- the generation must be current by metadata;
+- a written block must end the file, and a reused one must lie inside it;
+- the index must hold exactly this audit;
+- `validate_complete_block`, the function an open runs per block, must
+  recompute it to the identical audit;
+- the whole file must still match its content generation.
+
+The door hands that handle back with the commit (`append_and_retain_reader`),
+and the orchestrator's Admission V3 projection reads through it instead of
+opening the root again. "Freshly reopened" in this module now means re-read and
+re-validated from disk under a generation measured after the write, with every
+byte before the block proven unchanged since the open's scan validated it. No
+byte, format, identity or projection type changes.
+
+That last clause closes a hole the removed second open used to cover. A blind
+re-measure after an owned write would adopt a non-cooperating edit made to an
+older block during the append. The pre-write check now returns, from the same
+hash pass, the digest of the bytes the block lands after: the whole file, or
+the bytes before a trailing orphan. Every re-measure after a write must
+reproduce that digest or refuses, and the block is planned at that verified
+length, never after bytes nothing verified.
+
+Cost: one full open, O(sum over the A stored audits of (C·(P+S) +
+bootstrap)), plus two validations of the new block and a constant number of
+whole-file generation hash passes with no recomputation. A appends cost O(A²)
+block validations with constant 1, and a step scans its root once where it
+scanned three times. `docs/06-limits.md` §154 states it.
+
+**Supersedes** D-1682's rejection of the writer-handle re-read and the claim in
+LBE-09 that an append runs two full scans. The two tests LBE-09 names keep their
+names, because invariant rows are append-only, and now pin one scan; L1FE-01 to
+L1FE-04 state what holds.
+
+**Rejected.** Re-measuring the generation after the write without the prefix
+digest: it adopts edits made during the append. A sealed verified-prefix
+sidecar letting an open skip recomputing unchanged blocks: a new authority,
+behind an unkeyed seal an adversary with write access could re-forge.
+
+Tests: `cli::population_statistics_v2::tests::one_append_runs_one_full_scan_and_rereads_only_its_block`
+(failed first on the unfixed door: "A = 0: one open, the new block twice",
+left (2, 2), right (1, 2)),
+`cli::step3_orchestrator::tests::stored_pair_commits_observation_and_statistics_then_reuses_exact_bytes`
+(failed first: "one Statistics V2 scan per written step", left 3, right 1),
+`cli::population_statistics_v2::tests::one_append_runs_two_full_scans_as_section_154_states`,
+`cli::population_statistics_v2::tests::a_post_write_remeasure_adopts_only_a_reproduced_prefix`,
+`cli::population_statistics_v2::tests::reverify_rereads_only_what_this_handle_committed`,
+`cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append`.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 12e9bf09..d1feab54 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -8152,15 +8152,27 @@ audit index for at most the records the file holds, never the configured
 `max_audits` ceiling: before D-1682 every open, empty or not, reserved
 `max_audits` slots (production passes 1<<24) before anything was counted.
 
-One append through `append_population_statistics_v2` runs two full opens: the
-writer's, then a fresh read-only reopen after the writer is dropped. Each full
-open validates every stored block and reruns every stored block's bootstrap
-procedures, so one append costs two passes of O(sum over the A stored audits of
-(C·(P+S) + bootstrap)) plus the new block, and A appends to one root cost
-O(A²) block validations in total. The step-3 orchestrator then opens the root
-once more for its Admission V3 projection. D-1682 keeps the fresh reopen,
-because the Observation link and every projection type name a freshly
-reopened audit as their source; the cost is stated here instead.
+One append through `append_population_statistics_v2` runs one full open, the
+writer's. After the receipt-last write it re-reads only the committed block
+through the same handle (`reverify_committed`, which runs
+`validate_complete_block`, the function an open runs per block), and the whole
+file must still match a content generation measured after the write. That
+generation is adopted only when the bytes the block landed after reproduce the
+digest the pre-write check measured, in the same hash pass, over the file the
+open's scan validated, so a non-cooperating edit to an older block made during
+the append is refused rather than adopted. A full open validates every stored
+block and reruns every stored block's bootstrap procedures, so one append costs
+one pass of O(sum over the A stored audits of (C·(P+S) + bootstrap)), plus two
+validations of the new block and a constant number of whole-file generation
+hash passes with no recomputation, and A appends to one root cost O(A²) block
+validations in total, with constant 1 where it was 2. The step-3 orchestrator
+reads its Admission V3 projection through that same handle, so a step scans its
+Statistics root once where it scanned three times (W2-cli12-1, D-4764). Before
+D-4764 the door dropped the writer and ran a second full read-only open, and
+the orchestrator opened the root a third time; D-1682 had kept that because
+the Observation link and every projection type name a freshly reopened audit.
+D-4764 defines "freshly reopened" as re-read and re-validated from disk under a
+generation measured after the write; no byte, identity or type changes.
 
 The eight focused tests use controlled, test-private source rows. The public
 API can durably append and freshly reopen only an opaque prepared capability;
-- 
2.43.0


From aa814260d7f6f5275601e165b381d9a1b952a1f3 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:20:03 +0000
Subject: [PATCH 2/5] cli: cached Statistics V2 reads hash no file (G4-1,
 D-4765)
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Before: every cached read (reopen_audit, candidate, page_candidates,
projection_candidate, the Admission V3 reads, trailing_prefix_audit) ran
two content-generation checks, four whole-file hashes of the data file
and four of the lock file per call, while docs/06 §154 said a lookup is
average O(1) and a page O(page rows).

After: with_shared_lock compares the generations by metadata only, and
each read re-verifies what it returns: a lookup re-reads the audit's
Data and Completion records, candidate reads and pages re-seal their
rows, family-wide reads fold their rows into the block's ordered
candidate digest, a trailing-prefix read re-reads the orphan's Data.
§154 now holds for the whole call and states the residual: an
equal-metadata same-length rewrite of a record a read does not return,
or of a resealed returned row that still validates.

Fail-before: cached_reads_hash_no_file_and_reread_only_what_they_return
"81 cached reads hash no file" left 648 right 0.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/population_statistics_v2.rs    | 453 ++++++++++++++++--
 .../cli/tests/ledger_append_lookup_costs.rs   |  13 +
 docs/04-invariants.md                         |   1 +
 docs/05-decisions.md                          |  51 ++
 docs/06-limits.md                             |  19 +-
 5 files changed, 507 insertions(+), 30 deletions(-)

diff --git a/crates/cli/src/population_statistics_v2.rs b/crates/cli/src/population_statistics_v2.rs
index bc716090..e0096dd9 100644
--- a/crates/cli/src/population_statistics_v2.rs
+++ b/crates/cli/src/population_statistics_v2.rs
@@ -24,10 +24,14 @@
 //! trailing prefix is recoverable only by a byte-identical retry; corruption,
 //! a ragged/torn record, foreign retry, stale handle or bound breach refuses.
 //!
-//! Opening and generation checking scan bounded file bytes.  Recomputing the
+//! Opening scans and content-hashes bounded file bytes.  Recomputing the
 //! statistics is input-dependent and includes the bootstrap costs recorded in
-//! `docs/06-limits.md` §147.  Only fixed-record offset arithmetic is worst-case
-//! O(1) in record count.  Hash-map lookup is average O(1), and file hashing,
+//! `docs/06-limits.md` §147.  A cached read after the open hashes no file: it
+//! compares the open's lock and data generations by metadata only and
+//! re-verifies the records it returns, so a lookup is average O(1) and a page
+//! is O(page rows) plus a constant number of `stat` calls (G4-1, D-4765).
+//! Only fixed-record offset arithmetic is worst-case O(1) in record count.
+//! Hash-map lookup is average O(1), and file hashing at open and append,
 //! locks, allocation, bootstrap work and `sync_all` are not constant-time.
 //!
 //! **UNVERIFIED as a measured bound.** No bench in this workspace
@@ -1515,9 +1519,10 @@ pub struct PopulationStatisticsV2CandidateProjection {
 /// Preparing this authority is O(C) record reads and O(C) rank-bit space after
 /// the ledger's bounded generation validation, where C is the complete
 /// candidate count.  Reading one candidate from a prepared authority uses one
-/// fixed-stride record read; file locking, generation validation and I/O are
-/// not O(1).  Production callers that need the complete family use the bounded
-/// batch projection so generation hashing is not repeated once per candidate.
+/// fixed-stride record read after metadata-only generation checks (D-4765);
+/// file locking and I/O have no constant bound.  Production callers that need
+/// the complete family use the bounded batch projection, which also folds the
+/// family into its ordered candidate digest.
 ///
 /// **UNVERIFIED as a measured bound.** No bench in this workspace
 /// times this, so the shape above is read from the source rather
@@ -2363,28 +2368,60 @@ impl PopulationStatisticsV2Ledger {
     ) -> Result<Option<PopulationStatisticsV2TrailingPrefixAudit>, PopulationStatisticsV2Refusal>
     {
         self.with_shared_lock(|ledger| {
-            Ok(ledger
-                .orphan
-                .map(|orphan| PopulationStatisticsV2TrailingPrefixAudit {
-                    audit_id: orphan.manifest.audit_id,
-                    logical_sequence: orphan.manifest.sequence,
-                    first_record: orphan.first_record,
-                    present_records: orphan.present_records,
-                    planned_records: orphan.planned_records,
-                }))
+            let Some(orphan) = ledger.orphan else {
+                return Ok(None);
+            };
+            // The one record this read describes is re-read (D-4765).
+            let same = read_record(&mut ledger.data_file, orphan.first_record).and_then(|data| {
+                Ok(data.kind == RecordKindV2::Data
+                    && decode_manifest(&data.payload, data.kind)? == orphan.manifest)
+            });
+            if !matches!(same, Ok(true)) {
+                return Err(format!(
+                    "population-statistics trailing prefix at record {} differs from the one seen at open{}",
+                    orphan.first_record,
+                    same.err().map(|why| format!(": {why}")).unwrap_or_default()
+                ));
+            }
+            Ok(Some(PopulationStatisticsV2TrailingPrefixAudit {
+                audit_id: orphan.manifest.audit_id,
+                logical_sequence: orphan.manifest.sequence,
+                first_record: orphan.first_record,
+                present_records: orphan.present_records,
+                planned_records: orphan.planned_records,
+            }))
         })
     }
 
     /// Looks up one completed audit after stale-generation validation.
     ///
+    /// # Complexity
+    ///
+    /// Average O(1): the lock and data generations are compared by metadata
+    /// only, the index is probed once, and a found audit's Data and Completion
+    /// records, the two records it describes, are re-read and must decode to
+    /// the indexed manifest and Completion digest. No file is hashed (G4-1,
+    /// D-4765). Not seen per lookup: a same-length rewrite, under unchanged
+    /// metadata, of a record inside the block other than those two; the full
+    /// recomputation at the next open refuses it. Invariant L1FE-05, proven by
+    /// `cli::population_statistics_v2::tests::cached_reads_hash_no_file_and_reread_only_what_they_return`;
+    /// stated from the source, not timed.
+    ///
     /// # Errors
     ///
-    /// Refuses a stale/replaced data or lock path and I/O/lock errors.
+    /// Refuses a stale/replaced data or lock path, a Data or Completion record
+    /// that no longer matches the index, and I/O/lock errors.
     pub fn reopen_audit(
         &mut self,
         audit_id: &[u8; 32],
     ) -> Result<Option<PopulationStatisticsV2ReopenAudit>, PopulationStatisticsV2Refusal> {
-        self.with_shared_lock(|ledger| Ok(ledger.audits.get(audit_id).copied()))
+        self.with_shared_lock(|ledger| {
+            let Some(audit) = ledger.audits.get(audit_id).copied() else {
+                return Ok(None);
+            };
+            reverify_block_bounds(&mut ledger.data_file, &audit)?;
+            Ok(Some(audit))
+        })
     }
 
     /// Reads one candidate by validated fixed-record sequence.
@@ -2493,6 +2530,7 @@ impl PopulationStatisticsV2Ledger {
                 })?;
             seen_ranks.resize(usize_of(count, "Admission V3 rank count")?, false);
             let mut familywise = None;
+            let mut ordered = OrderedCandidateCheck::new(count);
             for sequence in 0..count {
                 let candidate = read_candidate_at(
                     &mut ledger.data_file,
@@ -2500,6 +2538,7 @@ impl PopulationStatisticsV2Ledger {
                     &reopened.manifest,
                     sequence,
                 )?;
+                ordered.push(&candidate);
                 let rank = usize_of(candidate.romano_wolf_rank, "Admission V3 Romano-Wolf rank")?;
                 let rank_slot = seen_ranks.get_mut(rank).ok_or_else(|| {
                     "population-statistics Admission V3 Romano-Wolf rank is outside family"
@@ -2525,6 +2564,7 @@ impl PopulationStatisticsV2Ledger {
                         .to_owned(),
                 );
             }
+            ordered.require(&reopened.manifest)?;
             let familywise_romano_wolf_probability = familywise.ok_or_else(|| {
                 "population-statistics Admission V3 rank-zero probability is absent".to_owned()
             })?;
@@ -2571,8 +2611,10 @@ impl PopulationStatisticsV2Ledger {
     /// This is the production path for Admission V3.  It is O(C) fixed-record
     /// reads and O(C) returned output space, not O(1) for the whole family; the
     /// Runner arithmetic performed on each returned candidate remains
-    /// fixed-width.  Unlike repeated single-candidate calls, generation hashing
-    /// is not multiplied by C.
+    /// fixed-width.  No file is hashed: the generations are compared by
+    /// metadata, and the C candidates read are folded into the block's ordered
+    /// candidate digest, which must equal the indexed manifest's, so every
+    /// returned row is the byte content the open validated (D-4765).
     ///
     /// **UNVERIFIED as a measured bound.** No bench in this workspace
     /// times this, so the shape above is read from the source rather
@@ -2595,6 +2637,7 @@ impl PopulationStatisticsV2Ledger {
                 .map_err(|why| {
                     format!("cannot reserve Admission V3 candidate projections: {why}")
                 })?;
+            let mut ordered = OrderedCandidateCheck::new(count);
             for sequence in 0..count {
                 let candidate = read_candidate_at(
                     &mut ledger.data_file,
@@ -2602,8 +2645,10 @@ impl PopulationStatisticsV2Ledger {
                     &reopened.manifest,
                     sequence,
                 )?;
+                ordered.push(&candidate);
                 candidates.push(admission_candidate_projection_v3(authority, &candidate)?);
             }
+            ordered.require(&reopened.manifest)?;
             Ok(candidates)
         })
     }
@@ -2664,10 +2709,14 @@ impl PopulationStatisticsV2Ledger {
         self.lock_file
             .lock_shared()
             .map_err(|why| format!("cannot take population-statistics audit lock: {why}"))?;
+        // Metadata only, before and after (G4-1, D-4765): a content hash here
+        // made every lookup, candidate read and page four whole-file hashes of
+        // the data file and four of the lock file. Each read re-verifies the
+        // records it returns instead.
         let result = (|| {
-            self.require_unchanged()?;
+            self.require_metadata_unchanged()?;
             let value = action(self)?;
-            self.require_unchanged()?;
+            self.require_metadata_unchanged()?;
             Ok(value)
         })();
         let released = self
@@ -2685,6 +2734,13 @@ impl PopulationStatisticsV2Ledger {
         require_generation(self.data_generation, &self.data_file, &self.data_path)
     }
 
+    /// The metadata halves of [`Self::require_unchanged`]: a constant number
+    /// of `stat` calls and no content read.
+    fn require_metadata_unchanged(&self) -> Result<(), PopulationStatisticsV2Refusal> {
+        require_metadata_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
+        require_metadata_generation(self.data_generation, &self.data_file, &self.data_path)
+    }
+
     /// [`Self::require_unchanged`] that also returns the generation-domain
     /// digest of the data file's first `cut` bytes, from the same pass.
     fn require_unchanged_with_prefix(
@@ -3288,6 +3344,91 @@ fn validate_orphan_prefix(
     Ok(())
 }
 
+/// Re-reads the Data and Completion records an indexed audit describes and
+/// requires them to decode to its manifest and its Completion digest: two
+/// fixed-record reads, O(1) in file size (D-4765), proven by
+/// `cli::population_statistics_v2::tests::cached_reads_hash_no_file_and_reread_only_what_they_return`.
+fn reverify_block_bounds(
+    file: &mut File,
+    audit: &PopulationStatisticsV2ReopenAudit,
+) -> Result<(), PopulationStatisticsV2Refusal> {
+    let completion_index = audit
+        .manifest
+        .block_record_count()?
+        .checked_sub(1)
+        .and_then(|ordinal| audit.first_record.checked_add(ordinal))
+        .ok_or_else(|| "population-statistics completion index overflowed".to_owned())?;
+    let data = read_record(file, audit.first_record).and_then(|data| {
+        Ok(data.kind == RecordKindV2::Data
+            && decode_manifest(&data.payload, data.kind)? == audit.manifest)
+    });
+    require_reread("Data", audit, data)?;
+    let completion = read_record(file, completion_index).and_then(|completion| {
+        Ok(completion.kind == RecordKindV2::Completion
+            && decode_manifest(&completion.payload, completion.kind)? == audit.manifest
+            && digest_completion_record(&completion.payload) == audit.completion_record_digest)
+    });
+    require_reread("Completion", audit, completion)
+}
+
+/// Names a re-read record of `audit` that no longer matches, with the decode or
+/// read refusal when there was one.
+fn require_reread(
+    kind: &str,
+    audit: &PopulationStatisticsV2ReopenAudit,
+    same: Result<bool, PopulationStatisticsV2Refusal>,
+) -> Result<(), PopulationStatisticsV2Refusal> {
+    match same {
+        Ok(true) => Ok(()),
+        Ok(false) => Err(format!(
+            "population-statistics {kind} record of audit {} differs from the one indexed at open",
+            hex32(audit.audit_id())
+        )),
+        Err(why) => Err(format!(
+            "population-statistics {kind} record of audit {} differs from the one indexed at open: {why}",
+            hex32(audit.audit_id())
+        )),
+    }
+}
+
+/// The block's ordered candidate digest, folded from candidates in sequence
+/// order: the one authority [`ordered_candidate_digest`] and the family-wide
+/// reads share, so a read of all C candidates re-verifies their content
+/// against the manifest at no extra I/O (D-4765).
+struct OrderedCandidateCheck {
+    hasher: Hasher,
+}
+
+impl OrderedCandidateCheck {
+    fn new(count: u64) -> Self {
+        let mut hasher = Hasher::new();
+        hasher.update(CANDIDATE_ORDER_DOMAIN);
+        hasher.update(&count.to_le_bytes());
+        Self { hasher }
+    }
+
+    fn push(&mut self, candidate: &PopulationStatisticsCandidateV2) {
+        hash_candidate(&mut self.hasher, candidate);
+    }
+
+    fn finish(self) -> [u8; 32] {
+        self.hasher.finalize()
+    }
+
+    fn require(
+        self,
+        manifest: &PopulationStatisticsManifestV2,
+    ) -> Result<(), PopulationStatisticsV2Refusal> {
+        if self.finish() != manifest.ordered_candidate_digest {
+            return Err(
+                "population-statistics candidate records no longer reproduce the block's ordered candidate digest"
+                    .to_owned(),
+            );
+        }
+        Ok(())
+    }
+}
+
 fn read_candidate_at(
     file: &mut File,
     first: u64,
@@ -3727,6 +3868,12 @@ thread_local! {
     static STATISTICS_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
 }
 
+#[cfg(test)]
+thread_local! {
+    /// Test-only count of whole-file generation hash passes on this thread.
+    static STATISTICS_FILE_HASHES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+}
+
 #[cfg(test)]
 thread_local! {
     /// Test-only count of complete-block validations (each one re-reads a
@@ -5451,13 +5598,12 @@ fn candidate_source_hasher(domain: &[u8], sequence: u64) -> Hasher {
 fn ordered_candidate_digest(
     candidates: &[PopulationStatisticsCandidateV2],
 ) -> Result<[u8; 32], PopulationStatisticsV2Refusal> {
-    let mut hasher = Hasher::new();
-    hasher.update(CANDIDATE_ORDER_DOMAIN);
-    hasher.update(&u64_of(candidates.len(), "ordered candidate count")?.to_le_bytes());
+    let mut ordered =
+        OrderedCandidateCheck::new(u64_of(candidates.len(), "ordered candidate count")?);
     for candidate in candidates {
-        hash_candidate(&mut hasher, candidate);
+        ordered.push(candidate);
     }
-    Ok(hasher.finalize())
+    Ok(ordered.finish())
 }
 
 fn ordered_period_digest(
@@ -6032,6 +6178,8 @@ fn hash_file_split(
     exact_bytes: u64,
     cut: u64,
 ) -> Result<([u8; 32], [u8; 32]), PopulationStatisticsV2Refusal> {
+    #[cfg(test)]
+    STATISTICS_FILE_HASHES.with(|count| count.set(count.get().saturating_add(1)));
     file.seek(SeekFrom::Start(0))
         .map_err(|why| format!("cannot seek {} for generation hash: {why}", path.display()))?;
     let rest = exact_bytes
@@ -6094,7 +6242,8 @@ fn hash_exact(
 /// The metadata half of [`require_generation`]: the held file and the file the
 /// path names (not followed through a link) must be one inode whose length and
 /// nanosecond modification/change times are the cached ones. It reads no
-/// content.
+/// content, so it is O(1) in file bytes, proven by
+/// `cli::population_statistics_v2::tests::cached_reads_hash_no_file_and_reread_only_what_they_return`.
 fn require_metadata_generation(
     expected: FileGenerationV2,
     file: &File,
@@ -6886,6 +7035,83 @@ mod tests {
         assert_eq!(STATISTICS_SCANS.with(std::cell::Cell::get), 1);
     }
 
+    #[test]
+    fn cached_reads_hash_no_file_and_reread_only_what_they_return() {
+        // G4-1: every cached read ran `with_shared_lock`, whose two content
+        // checks hashed the data file four times and the lock file four times
+        // per call. An open still hashes; a read after it hashes nothing.
+        let root = TempRoot::new("no-hash-reads");
+        let prepared = fixture(31);
+        let procedure =
+            PopulationStatisticsProcedureV2::new(16, 77, 2).expect("fixture procedure is explicit");
+        let link =
+            observation_statistics_link(observation_facts(131), &prepared.manifest, procedure)
+                .expect("detached Observation link prepares");
+        let statistics = append_population_statistics_v2(root.path(), bounds(), &prepared)
+            .expect("Statistics bytes commit");
+        let audit = statistics.audit();
+        let source = PopulationStatisticsObservationCommitV2 { statistics, link }
+            .projection_source()
+            .expect("exact linked commit produces a source");
+        let hashes = || STATISTICS_FILE_HASHES.with(std::cell::Cell::get);
+        STATISTICS_FILE_HASHES.with(|count| count.set(0));
+        let mut reader =
+            PopulationStatisticsV2Ledger::open_read(root.path(), bounds()).expect("reader opens");
+        assert_eq!(
+            hashes(),
+            4,
+            "an open hashes the lock file and the data file twice each"
+        );
+        STATISTICS_FILE_HASHES.with(|count| count.set(0));
+        let authority = reader
+            .prepare_admission_projection_v3(&source)
+            .expect("complete family projects");
+        for _ in 0..10 {
+            assert_eq!(
+                reader
+                    .reopen_audit(&audit.audit_id())
+                    .expect("lookup reads"),
+                Some(audit)
+            );
+            assert_eq!(
+                reader
+                    .reopen_audit(&[0xAB; 32])
+                    .expect("absent lookup reads"),
+                None
+            );
+            assert_eq!(
+                reader
+                    .candidate(&audit.audit_id(), 1)
+                    .expect("candidate reads")
+                    .sequence(),
+                1
+            );
+            assert_eq!(
+                reader
+                    .page_candidates(&audit.audit_id(), 0, 2)
+                    .expect("page reads")
+                    .rows()
+                    .len(),
+                2
+            );
+            reader
+                .projection_candidate(source, 0)
+                .expect("projection candidate reads");
+            reader
+                .admission_candidate_v3(&authority, 1)
+                .expect("Admission V3 candidate reads");
+            assert_eq!(
+                reader
+                    .admission_candidates_v3(&authority)
+                    .expect("Admission V3 family reads")
+                    .len(),
+                2
+            );
+            assert!(reader.trailing_prefix_audit().expect("no orphan").is_none());
+        }
+        assert_eq!(hashes(), 0, "81 cached reads hash no file");
+    }
+
     #[test]
     fn one_append_runs_one_full_scan_and_rereads_only_its_block() {
         // W2-cli12-1: the door opened the ledger twice (the writer's open and
@@ -7073,6 +7299,174 @@ mod tests {
         );
     }
 
+    /// Pins the file's modification time far from now, so a change is visible
+    /// to a metadata generation whatever the filesystem's timestamp tick.
+    fn pin_mtime(path: &Path) {
+        OpenOptions::new()
+            .write(true)
+            .open(path)
+            .expect("fixture file opens")
+            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1))
+            .expect("fixture mtime sets");
+    }
+
+    /// Re-measures the cached data generation's metadata, keeping its content
+    /// digest: a rewrite made inside one timestamp tick, which a metadata
+    /// generation cannot see.
+    fn same_tick(ledger: &mut PopulationStatisticsV2Ledger) {
+        let metadata = std::fs::metadata(&ledger.data_path).expect("ledger measures");
+        ledger.data_generation = generation_of(&metadata, ledger.data_generation.content_digest);
+    }
+
+    #[test]
+    fn cached_reads_reverify_what_they_return_and_the_limit_is_what_they_do_not() {
+        // G4-1, D-4765: a cached read compares generations by metadata and
+        // re-verifies the records it returns. One audit of fixture(31): Data at
+        // record 0, candidates at 1 and 2, periods 3..=10, splits 11 and 12,
+        // Completion at 13.
+        let root = TempRoot::new("reverified-reads");
+        let prepared = fixture(31);
+        let procedure =
+            PopulationStatisticsProcedureV2::new(16, 77, 2).expect("fixture procedure is explicit");
+        let link =
+            observation_statistics_link(observation_facts(131), &prepared.manifest, procedure)
+                .expect("detached Observation link prepares");
+        let statistics = append_population_statistics_v2(root.path(), bounds(), &prepared)
+            .expect("Statistics bytes commit");
+        let audit = statistics.audit();
+        let id = audit.audit_id();
+        let source = PopulationStatisticsObservationCommitV2 { statistics, link }
+            .projection_source()
+            .expect("exact linked commit produces a source");
+        let path = root.path().join(DATA_FILE);
+        let offset = |record: u64, byte: u64| record_offset(record).expect("offset") + byte;
+
+        // A metadata-visible change refuses every read, even an absent id.
+        let mut reader =
+            PopulationStatisticsV2Ledger::open_read(root.path(), bounds()).expect("reader opens");
+        pin_mtime(&path);
+        for why in [
+            reader.reopen_audit(&[0xAB; 32]).err(),
+            reader.candidate(&id, 0).err(),
+            reader.page_candidates(&id, 0, 1).err(),
+            reader.trailing_prefix_audit().err(),
+        ] {
+            let why = why.unwrap_or_default();
+            assert!(
+                why.contains("changed after population-statistics open"),
+                "{why}"
+            );
+        }
+
+        // Inside one tick, a lookup re-reads its Data and Completion.
+        for (record, needed) in [
+            (0, "Data record of audit"),
+            (13, "Completion record of audit"),
+        ] {
+            let mut reader = PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
+                .expect("reader opens");
+            rewrite_resealed_record(&path, record, |payload| payload[600] ^= 0x01);
+            same_tick(&mut reader);
+            let why = reader
+                .reopen_audit(&id)
+                .expect_err("its own records refuse");
+            assert!(why.contains(needed), "{why}");
+            assert_eq!(
+                reader.reopen_audit(&[0xAB; 32]).expect("an absent id"),
+                None
+            );
+            rewrite_resealed_record(&path, record, |payload| payload[600] ^= 0x01);
+        }
+
+        // An unsealed returned candidate refuses by its seal.
+        let mut reader =
+            PopulationStatisticsV2Ledger::open_read(root.path(), bounds()).expect("reader opens");
+        flip_byte(&path, offset(1, 72));
+        same_tick(&mut reader);
+        assert!(
+            reader.candidate(&id, 0).is_err(),
+            "a single read re-seals its row"
+        );
+        assert!(
+            reader.page_candidates(&id, 0, 2).is_err(),
+            "a page re-seals its rows"
+        );
+        assert!(
+            reader.candidate(&id, 1).is_ok(),
+            "the other row is not read"
+        );
+        flip_byte(&path, offset(1, 72));
+
+        // A RESEALED candidate that still validates: the limit. A single read
+        // returns it as written; the family-wide reads refuse it by the
+        // block's ordered candidate digest, and the next open by recomputation.
+        let mut reader =
+            PopulationStatisticsV2Ledger::open_read(root.path(), bounds()).expect("reader opens");
+        let authority = reader
+            .prepare_admission_projection_v3(&source)
+            .expect("complete family projects");
+        rewrite_resealed_record(&path, 1, |payload| payload[72] ^= 0x01);
+        same_tick(&mut reader);
+        let returned = reader.candidate(&id, 0).expect("not seen by a single read");
+        assert_ne!(
+            returned.candidate_semantic_digest,
+            audit_candidate_digest(&prepared)
+        );
+        for why in [
+            reader.admission_candidates_v3(&authority).err(),
+            reader.prepare_admission_projection_v3(&source).err(),
+        ] {
+            let why = why.unwrap_or_default();
+            assert!(
+                why.contains("no longer reproduce the block's ordered candidate digest"),
+                "{why}"
+            );
+        }
+        assert!(
+            reader
+                .reopen_audit(&id)
+                .expect("a lookup does not read rows")
+                .is_some()
+        );
+        drop(reader);
+        assert!(
+            PopulationStatisticsV2Ledger::open_read(root.path(), bounds()).is_err(),
+            "the next open recomputes the block and refuses it"
+        );
+    }
+
+    /// The first candidate's semantic digest as prepared.
+    fn audit_candidate_digest(prepared: &PreparedPopulationStatisticsV2) -> [u8; 32] {
+        let planned = prepared.records(0, 0).expect("planned bytes build");
+        let raw = planned.get(1).expect("a candidate record");
+        let mut payload = [0_u8; PAYLOAD_BYTES];
+        payload.copy_from_slice(&raw[..PAYLOAD_BYTES]);
+        decode_candidate(&payload)
+            .expect("planned candidate decodes")
+            .candidate_semantic_digest
+    }
+
+    #[test]
+    fn a_trailing_prefix_read_reverifies_the_orphan_data_record() {
+        let root = TempRoot::new("orphan-reverify");
+        orphan_fixture(root.path(), &fixture(2));
+        let path = root.path().join(DATA_FILE);
+        let mut reader = PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
+            .expect("the orphan is recoverable");
+        assert!(
+            reader
+                .trailing_prefix_audit()
+                .expect("orphan reads")
+                .is_some()
+        );
+        rewrite_resealed_record(&path, 0, |payload| payload[600] ^= 0x01);
+        same_tick(&mut reader);
+        let why = reader
+            .trailing_prefix_audit()
+            .expect_err("a changed orphan Data refuses");
+        assert!(why.contains("differs from the one seen at open"), "{why}");
+    }
+
     #[test]
     fn complete_pair_recomputes_reopens_pages_and_exactly_reuses() {
         let root = TempRoot::new("reopen");
@@ -7981,6 +8375,10 @@ mod tests {
             let value = get_u64(payload, 136).expect("trade count reads");
             put_u64(payload, 136, value.saturating_add(1)).expect("fixed candidate field exists");
         });
+        // A cached read compares metadata (D-4765): the pin makes the rewrite
+        // visible to it whatever the timestamp tick. A same-tick rewrite is
+        // `cached_reads_reverify_what_they_return_and_the_limit_is_what_they_do_not`.
+        pin_mtime(&path);
         let why = reader
             .candidate(&audit.audit_id(), 0)
             .expect_err("same-length mutation invalidates cached handle");
@@ -8001,6 +8399,7 @@ mod tests {
                     put_u64(payload, 136, value.saturating_add(1))
                         .expect("fixed candidate field exists");
                 });
+                pin_mtime(&during_path);
                 Ok(())
             })
             .expect_err("post-action generation check catches non-cooperating mutation");
diff --git a/crates/cli/tests/ledger_append_lookup_costs.rs b/crates/cli/tests/ledger_append_lookup_costs.rs
index c7d06e1e..f3dbacc4 100644
--- a/crates/cli/tests/ledger_append_lookup_costs.rs
+++ b/crates/cli/tests/ledger_append_lookup_costs.rs
@@ -19,6 +19,8 @@
 //! * W2-cli12-1 (D-4764): the two-open Statistics V2 append and the step's
 //!   third open are one open per step; the tests named in LBE-09 keep their
 //!   names and now pin the one open.
+//! * G4-1 (D-4765): every cached Statistics V2 read hashed the whole data file
+//!   four times while §154 said a lookup is average O(1) and a page O(rows).
 //!
 //! Every file a constant below names is read at compile time, so a rename
 //! fails the build rather than skipping the check. A separate test crate, as
@@ -238,6 +240,17 @@ fn section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append()
         !text.contains("runs two full opens"),
         "§154 still prices two opens"
     );
+
+    // A cached read checks metadata only since D-4765 (G4-1).
+    let lock = method(STATISTICS, "    fn with_shared_lock<T>(");
+    assert_eq!(
+        lock.matches("self.require_metadata_unchanged()?").count(),
+        2
+    );
+    assert!(!lock.contains("self.require_unchanged()"));
+    let metadata = function(STATISTICS, "fn require_metadata_generation(");
+    assert!(!metadata.contains("hash_file") && !metadata.contains("file_generation("));
+    assert!(text.contains("Since D-4765 that is true of the whole call, not only of the probe"));
 }
 
 #[test]
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 326b86a3..1ebc5dec 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7056,3 +7056,4 @@ old line regex the same input and watched it pass.
 | L1FE-02 | After an owned write the Statistics V2 generation is adopted only when the bytes below the cut reproduce the digest measured before the write; an edit below the cut refuses and nothing is adopted; the one-pass split hash equals one-shot hashes, and a cut past the file refuses (D-4764) | `cli::population_statistics_v2::tests::a_post_write_remeasure_adopts_only_a_reproduced_prefix` | ✓ |
 | L1FE-03 | `reverify_committed` re-reads a reused block; it refuses a written block that does not end the file, an audit absent from or unlike the index, a disk block unlike the index, and a same-length edit elsewhere under re-measured metadata (D-4764) | `cli::population_statistics_v2::tests::reverify_rereads_only_what_this_handle_committed` | ✓ |
 | L1FE-04 | The Statistics V2 door opens once and never read-only, the step's projection opens nothing, and §154 prices one open per append and one scan per step (D-4764) | `cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append` | ✓ |
+| L1FE-05 | A cached Statistics V2 read hashes no file (81 reads after an open hash 0 times; the open hashes 4 times); a metadata-visible change refuses every read; inside one timestamp tick a lookup refuses its own changed Data or Completion record, a candidate read or page refuses an unsealed returned row, and the family-wide reads refuse a resealed row that still validates, which a single read returns (the stated limit) and the next open refuses; a trailing-prefix read refuses a changed orphan Data record (D-4765) | `cli::population_statistics_v2::tests::cached_reads_hash_no_file_and_reread_only_what_they_return`, `cli::population_statistics_v2::tests::cached_reads_reverify_what_they_return_and_the_limit_is_what_they_do_not`, `cli::population_statistics_v2::tests::a_trailing_prefix_read_reverifies_the_orphan_data_record`, `cli::population_statistics_v2::tests::explicit_bounds_and_post_open_same_length_mutation_refuse`, `cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index c15bd49d..0da99d87 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65086,3 +65086,54 @@ left (2, 2), right (1, 2)),
 `cli::population_statistics_v2::tests::a_post_write_remeasure_adopts_only_a_reproduced_prefix`,
 `cli::population_statistics_v2::tests::reverify_rereads_only_what_this_handle_committed`,
 `cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append`.
+
+### D-4765 — A cached Statistics V2 read checks metadata and re-verifies what it returns — 2026-10-09
+
+**What was wrong.** G4-1: §154 says a hash-index lookup is average O(1) and a
+bounded page O(page rows). Every cached read (`reopen_audit`, `candidate`,
+`page_candidates`, `projection_candidate`, the Admission V3 reads and
+`trailing_prefix_audit`) ran `with_shared_lock`, whose content checks before
+and after the read hashed the data file four times and the lock file four
+times. That is O(file bytes) per call. The counting test measured 648 hashes
+for 81 calls.
+
+**Decided.** The fix the finding named as (b), the D-1681 shape:
+
+- `with_shared_lock` compares the open's lock and data generations by metadata
+  only, before and after the read: length, device/inode, and nanosecond
+  modification and change times, with no content read.
+- Each read re-verifies what it returns. A lookup re-reads the audit's Data and
+  Completion records, which must decode to the indexed manifest and Completion
+  digest.
+- A candidate read and a page already re-seal and re-validate every row they
+  return.
+- The family-wide reads fold their C rows into the block's ordered candidate
+  digest, through the same function the writer uses, so every returned row is
+  the content the open validated.
+- A trailing-prefix read re-reads the orphan's Data record.
+
+An open still content-hashes, and so do appends. No byte, format or identity
+changes.
+
+**What is no longer seen per read.** A same-length rewrite that leaves every
+metadata field equal: a rewrite inside one timestamp tick, a raw device write
+or a clock change. It goes unseen when it touches a record the read does not
+return, or a returned candidate row that is resealed and still validates. The
+family-wide reads refuse the second by the ordered digest, and the next open
+refuses both by recomputation. This is the residual D-1681 accepted for the
+Pre-Admission page, and §154 states it.
+
+**Rejected.** Option (a), keeping the hashes and correcting §154 to O(file
+bytes) per call: the finding showed (b) is reachable without a format change.
+Dropping only the second `repeated_digest` pass: it halves the cost and leaves
+it O(file bytes).
+
+Tests: `cli::population_statistics_v2::tests::cached_reads_hash_no_file_and_reread_only_what_they_return`
+(failed first on the unfixed reads: "81 cached reads hash no file", left 648,
+right 0),
+`cli::population_statistics_v2::tests::cached_reads_reverify_what_they_return_and_the_limit_is_what_they_do_not`,
+`cli::population_statistics_v2::tests::a_trailing_prefix_read_reverifies_the_orphan_data_record`,
+`cli::population_statistics_v2::tests::explicit_bounds_and_post_open_same_length_mutation_refuse`
+(its rewrites now pin the modification time, so the metadata refusal does not
+depend on the timestamp tick),
+`cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append`.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index d1feab54..75bbc65e 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -8138,9 +8138,22 @@ it is not made O(1) by storing its receipt.
 
 The 1,024-byte fixed stride makes one already-validated candidate seek
 worst-case O(1) in record count. A hash-index lookup is average O(1), a bounded
-page is O(page rows), and one record has constant encoded width. File open,
-whole-file validation, hashing, allocation, locks, `sync_all`, CSCV family work
-and bootstrap resampling are not constant-time or constant-space operations.
+page is O(page rows), and one record has constant encoded width. Since D-4765
+that is true of the whole call, not only of the probe: a cached read compares
+the open's lock and data generations by metadata only (length, device/inode,
+nanosecond modification and change times), a constant number of `stat` calls,
+and re-verifies the records it returns. A lookup re-reads the audit's Data and
+Completion records, a candidate read or a page re-seals each row it returns,
+and a family-wide read folds its C rows into the block's ordered candidate
+digest. Before D-4765 every such call content-hashed the data file four times
+and the lock file four times, O(file bytes) per call (G4-1). What a cached read
+does not see is a same-length rewrite that leaves every metadata field equal (a
+rewrite inside one timestamp tick, a raw device write or a clock change) of a
+record it does not return, or of a returned candidate row that is resealed and
+still validates. The family-wide reads refuse the second by the ordered digest,
+and the next open refuses both by recomputation. File open, whole-file
+validation, hashing, allocation, locks, `sync_all`, CSCV family work and
+bootstrap resampling are not constant-time or constant-space operations.
 Explicit audit/candidate/period/split/file ceilings are refusal bounds; they do
 not sample rows, cap Apriori depth or turn an admitted input into a smaller one.
 
-- 
2.43.0


From bb0fbd83939445bd87fafb98487d36710928b55e Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:25:52 +0000
Subject: [PATCH 3/5] cli: O(1) Observation V1/V2 lookup and a one-scan append
 door (W2-cli11-3, D-4766)
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Before: each Observation reopen_audit read the whole bounded file into
memory and hashed it, O(B) time and memory per lookup, and each append
door made five whole-file reads and two scans (open, pre-append check,
post-write refresh, a second open, the lookup).

After: require_unchanged is metadata only; an open measures the
generations before its read and re-checks them after. A lookup re-reads
the found Data/Completion pair at HEADER + stride x 2 x seq (512 in V1,
1,024 in V2) and requires it to decode to the cached audit. The door
re-reads only the committed pair through the writer's handle, and the
post-write refresh is one streaming hash pass that must reproduce the
digest of the bytes the handle had verified, so an edit below the new
pair is refused rather than adopted. A door is one open and one scan.
§157, §161 and the lane 1-b chapter state it; the LBE-06 doc-pin test
keeps its name and pins the new shape.

Fail-before: lookups_read_no_whole_file_and_one_append_door_scans_once
"V1 written: one open, one post-write pass" left (5, 2, 0) right (1, 1, 1).

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/population_observations_v1.rs  | 788 ++++++++++++++++--
 .../cli/tests/ledger_append_lookup_costs.rs   |  35 +-
 docs/04-invariants.md                         |   2 +
 docs/05-decisions.md                          |  49 ++
 docs/06-limits.md                             |  51 +-
 5 files changed, 830 insertions(+), 95 deletions(-)

diff --git a/crates/cli/src/population_observations_v1.rs b/crates/cli/src/population_observations_v1.rs
index 30c69e05..8a98e734 100644
--- a/crates/cli/src/population_observations_v1.rs
+++ b/crates/cli/src/population_observations_v1.rs
@@ -1743,9 +1743,10 @@ impl ObservationAuthorityAuditV1 {
 /// Whether a receipt-last observation authority was appended or exactly reused.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum ObservationAuthorityCommitV1 {
-    /// New Data and Completion records were synced, then freshly reopened.
+    /// New Data and Completion records were synced, then re-read through the
+    /// writer's handle (D-4766).
     Written(ObservationAuthorityAuditV1),
-    /// Exact existing semantic bytes were freshly reopened without duplication.
+    /// Exact existing semantic bytes were re-read without duplication.
     Reused(ObservationAuthorityAuditV1),
 }
 
@@ -2106,7 +2107,11 @@ pub struct ObservationAuthorityLedgerV1 {
     audits: std::collections::HashMap<[u8; 32], ObservationAuthorityAuditV1>,
     data_by_id: std::collections::HashMap<[u8; 32], ObservationAuthorityDataV1>,
     orphan: Option<(u64, ObservationAuthorityDataV1)>,
+    /// Domain digest of the file's first `snapshot_len` bytes as this handle
+    /// last verified them: at open from the bytes its scan validated, after
+    /// each owned write by one streaming pass that first reproduces it.
     snapshot_digest: [u8; 32],
+    snapshot_len: u64,
     lock_generation: ObservationFileGenerationV1,
     file_generation: ObservationFileGenerationV1,
     writable: bool,
@@ -2201,11 +2206,18 @@ impl ObservationAuthorityLedgerV1 {
                 &authority_header(),
             )?;
         }
+        // The generations are measured BEFORE the read and re-checked after
+        // it, so the bytes the scan validated are the ones every later
+        // metadata check compares against (D-4766).
+        let lock_generation = observation_file_generation(&lock, &lock_path)?;
+        let file_generation = observation_file_generation(&file, &file_path)?;
         let bytes = read_bounded_authority_file(&mut file, bounds)?;
         let (audits, data_by_id, orphan) = scan_authority_file(&bytes, bounds)?;
         let snapshot_digest = digest_authority_file(&bytes);
-        let lock_generation = observation_file_generation(&lock, &lock_path)?;
-        let file_generation = observation_file_generation(&file, &file_path)?;
+        let snapshot_len = u64::try_from(bytes.len())
+            .map_err(|_| "observation authority length does not fit u64".to_owned())?;
+        require_observation_generation(lock_generation, &lock, &lock_path)?;
+        require_observation_generation(file_generation, &file, &file_path)?;
         Ok(Self {
             lock_path,
             file_path,
@@ -2216,32 +2228,113 @@ impl ObservationAuthorityLedgerV1 {
             data_by_id,
             orphan,
             snapshot_digest,
+            snapshot_len,
             lock_generation,
             file_generation,
             writable,
         })
     }
 
-    /// Returns one cached audit only after detecting any stale same-length edit.
+    /// Returns one cached audit only after re-verifying the bytes it names.
     ///
     /// # Complexity
     ///
-    /// Each lookup reads the whole bounded authority file into memory and
-    /// hashes it, so it is O(B) time and O(B) transient memory in file bytes
-    /// B; only the identity-map probe that follows is average O(1). The
-    /// content hash is kept deliberately: it is what refuses a same-length
-    /// edit a metadata generation cannot see (W2-cli11-3, D-1681). Invariant
-    /// LBE-06; UNVERIFIED as a measured time.
+    /// Average O(1) time and O(1) memory in file bytes: the lock and file
+    /// generations are compared by metadata only (length, device/inode,
+    /// nanosecond modification/change times), the identity map is probed
+    /// once, and a found audit's Data and Completion records, the 1,024-byte
+    /// pair at `HEADER + 512 x 2 x record_sequence`, are re-read and must
+    /// decode to exactly the cached audit and Data. Before D-4766 each lookup
+    /// read and hashed the whole file, O(B) (W2-cli11-3). Not seen per lookup:
+    /// a same-length rewrite of ANOTHER authority's pair that leaves every
+    /// metadata field equal; that pair's own lookup and the next open refuse
+    /// it. Invariant L1FE-06, proven by
+    /// `cli::population_observations_v1::tests::lookups_read_no_whole_file_and_one_append_door_scans_once`;
+    /// stated from the source, not timed.
     ///
     /// # Errors
     ///
-    /// Refuses any file mutation or bounded read failure since open.
+    /// Refuses a changed or replaced lock or file, a pair that no longer
+    /// decodes to the cached audit, or a read failure.
     pub fn reopen_audit(
         &mut self,
         authority_id: &[u8; 32],
     ) -> Result<Option<ObservationAuthorityAuditV1>, String> {
         self.require_unchanged()?;
-        Ok(self.audits.get(authority_id).copied())
+        let Some(audit) = self.audits.get(authority_id).copied() else {
+            return Ok(None);
+        };
+        self.reverify_pair(&audit)?;
+        Ok(Some(audit))
+    }
+
+    /// Re-reads the Data/Completion pair at `audit.record_sequence` and
+    /// requires it to decode, seal-checked, to exactly `audit` and to the Data
+    /// this handle indexed: O(1) reads and hashing (D-4766), proven by
+    /// `cli::population_observations_v1::tests::a_v1_lookup_rereads_its_own_pair_and_not_another`.
+    fn reverify_pair(&mut self, audit: &ObservationAuthorityAuditV1) -> Result<(), String> {
+        let sequence = audit.record_sequence;
+        let reread = (|| {
+            let (data_raw, completion_raw) = read_pair_at::<AUTHORITY_RECORD_BYTES>(
+                &mut self.file,
+                OBSERVATION_AUTHORITY_HEADER_BYTES_V1,
+                sequence,
+            )?;
+            let data = ObservationAuthorityDataV1::decode(&data_raw)?;
+            let completion = ObservationAuthorityCompletionV1::decode(&completion_raw)?;
+            if completion.record_sequence != sequence {
+                return Err("its Completion names another sequence".to_owned());
+            }
+            completion.validate(&data)?;
+            if audit_of(&data, completion) != *audit
+                || self.data_by_id.get(&data.authority_id) != Some(&data)
+            {
+                return Err("its records decode to another authority".to_owned());
+            }
+            Ok(())
+        })();
+        reread.map_err(|why| {
+            format!("observation authority pair {sequence} changed after open: {why}")
+        })
+    }
+
+    /// Re-reads the pair this handle just committed (W2-cli11-3, D-4766): the
+    /// generations must be current, the file must end where a written pair
+    /// ends (or hold a reused one), the index must hold exactly this audit,
+    /// and the pair must decode to it. The bytes before a written pair were
+    /// proven unchanged by the append's own streaming pass, so no second open
+    /// is needed. O(1) beyond that pass, proven by
+    /// `cli::population_observations_v1::tests::lookups_read_no_whole_file_and_one_append_door_scans_once`.
+    fn reverify_committed(
+        &mut self,
+        committed: &ObservationAuthorityCommitV1,
+    ) -> Result<ObservationAuthorityAuditV1, String> {
+        let expected = committed.audit();
+        self.require_unchanged()?;
+        if self.audits.get(&expected.authority_id) != Some(&expected) {
+            return Err(
+                "observation authority committed audit is not the one this handle indexed"
+                    .to_owned(),
+            );
+        }
+        let pair_end = pair_end(
+            OBSERVATION_AUTHORITY_HEADER_BYTES_V1,
+            OBSERVATION_AUTHORITY_RECORD_STRIDE_V1,
+            expected.record_sequence,
+        )?;
+        let len = self.file_generation.len;
+        let in_place = match committed {
+            ObservationAuthorityCommitV1::Written(_) => len == pair_end,
+            ObservationAuthorityCommitV1::Reused(_) => len >= pair_end,
+        };
+        if !in_place {
+            return Err(format!(
+                "observation authority file holds {len} bytes; the committed pair ends at byte {pair_end}"
+            ));
+        }
+        self.reverify_pair(&expected)?;
+        self.require_unchanged()?;
+        Ok(expected)
     }
 
     fn append_data(
@@ -2347,17 +2440,11 @@ impl ObservationAuthorityLedgerV1 {
         Ok(ObservationAuthorityCommitV1::Written(audit))
     }
 
-    fn require_unchanged(&mut self) -> Result<(), String> {
+    /// Metadata only: a constant number of `stat` calls and no content read
+    /// (D-4766). Before D-4766 this read and hashed the whole file.
+    fn require_unchanged(&self) -> Result<(), String> {
         require_observation_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
-        require_observation_generation(self.file_generation, &self.file, &self.file_path)?;
-        let bytes = read_bounded_authority_file(&mut self.file, self.bounds)?;
-        if digest_authority_file(&bytes) != self.snapshot_digest {
-            return Err(format!(
-                "observation authority file {} changed after open",
-                self.file_path.display()
-            ));
-        }
-        Ok(())
+        require_observation_generation(self.file_generation, &self.file, &self.file_path)
     }
 
     /// Appends one record through the shared rollback, then syncs it (D-1854).
@@ -2417,11 +2504,32 @@ impl ObservationAuthorityLedgerV1 {
             })
     }
 
+    /// After an owned write or a rollback: one streaming pass that must
+    /// reproduce the verified digest over the first `snapshot_len` bytes, so a
+    /// non-cooperating edit made below this handle's write is refused rather
+    /// than adopted, and then extends the snapshot to the whole file and
+    /// re-measures both generations (D-4766).
     fn refresh_snapshot(&mut self) -> Result<(), String> {
-        let bytes = read_bounded_authority_file(&mut self.file, self.bounds)?;
-        self.snapshot_digest = digest_authority_file(&bytes);
-        self.lock_generation = observation_file_generation(&self.lock_file, &self.lock_path)?;
-        self.file_generation = observation_file_generation(&self.file, &self.file_path)?;
+        let lock_generation = observation_file_generation(&self.lock_file, &self.lock_path)?;
+        let file_generation = observation_file_generation(&self.file, &self.file_path)?;
+        let (prefix, full) = hash_authority_prefix(
+            &mut self.file,
+            AUTHORITY_FILE_DIGEST_DOMAIN,
+            self.snapshot_len,
+            file_generation.len,
+        )?;
+        if prefix != self.snapshot_digest {
+            return Err(format!(
+                "observation authority file {} changed below byte {} while this handle appended",
+                self.file_path.display(),
+                self.snapshot_len
+            ));
+        }
+        require_observation_generation(file_generation, &self.file, &self.file_path)?;
+        self.snapshot_digest = full;
+        self.snapshot_len = file_generation.len;
+        self.lock_generation = lock_generation;
+        self.file_generation = file_generation;
         Ok(())
     }
 }
@@ -2445,8 +2553,9 @@ impl PairedCandidateObservationsV1 {
         require_authority_audit_for_data(&data, audit)
     }
 
-    /// Persists this opaque capability Data-first/Completion-last and requires
-    /// a fresh read-only reopen before returning success.
+    /// Persists this opaque capability Data-first/Completion-last and re-reads
+    /// the committed pair through the writer's handle before returning success
+    /// (D-4766).
     ///
     /// # Errors
     ///
@@ -2465,6 +2574,9 @@ impl PairedCandidateObservationsV1 {
     }
 }
 
+/// One open, the append, and a re-read of only the committed pair through the
+/// same handle: one scan per door where there were two, and no whole-file read
+/// after the open (W2-cli11-3, D-4766).
 fn append_authority_data_and_reopen(
     root: &Path,
     bounds: ObservationAuthorityBoundsV1,
@@ -2472,15 +2584,7 @@ fn append_authority_data_and_reopen(
 ) -> Result<ObservationAuthorityCommitV1, String> {
     let mut ledger = ObservationAuthorityLedgerV1::open(root, bounds)?;
     let committed = ledger.append_data(data)?;
-    let expected = committed.audit();
-    drop(ledger);
-    let mut reopened = ObservationAuthorityLedgerV1::open_read(root, bounds)?;
-    let audit = reopened
-        .reopen_audit(&expected.authority_id)?
-        .ok_or_else(|| "observation authority disappeared after receipt-last append".to_owned())?;
-    if audit != expected {
-        return Err("observation authority fresh reopen differs from written bytes".to_owned());
-    }
+    let audit = ledger.reverify_committed(&committed)?;
     require_authority_audit_for_data(data, &audit)?;
     Ok(match committed {
         ObservationAuthorityCommitV1::Written(_) => ObservationAuthorityCommitV1::Written(audit),
@@ -2636,6 +2740,8 @@ fn scan_authority_file(
     bytes: &[u8],
     bounds: ObservationAuthorityBoundsV1,
 ) -> Result<ObservationScanV1, String> {
+    #[cfg(test)]
+    OBSERVATION_SCANS.with(|count| count.set(count.get().saturating_add(1)));
     if bytes.get(..AUTHORITY_HEADER_BYTES) != Some(authority_header().as_slice()) {
         return Err("observation authority file header is absent or corrupt".to_owned());
     }
@@ -2831,6 +2937,8 @@ fn read_bounded_authority_file(
     file: &mut File,
     bounds: ObservationAuthorityBoundsV1,
 ) -> Result<Vec<u8>, String> {
+    #[cfg(test)]
+    OBSERVATION_FILE_READS.with(|count| count.set(count.get().saturating_add(1)));
     let len = file
         .metadata()
         .map_err(|why| format!("cannot stat observation authority file: {why}"))?
@@ -2856,6 +2964,15 @@ fn read_bounded_authority_file(
     Ok(bytes)
 }
 
+#[cfg(test)]
+thread_local! {
+    /// Test-only counts on this thread: whole-file bounded reads, file scans
+    /// and streaming hash passes, V1 and V2 together (D-4766).
+    static OBSERVATION_FILE_READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+    static OBSERVATION_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+    static OBSERVATION_HASH_PASSES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+}
+
 fn digest_authority_file(bytes: &[u8]) -> [u8; 32] {
     let mut hasher = Hasher::new();
     hasher.update(AUTHORITY_FILE_DIGEST_DOMAIN);
@@ -2863,6 +2980,95 @@ fn digest_authority_file(bytes: &[u8]) -> [u8; 32] {
     hasher.finalize()
 }
 
+/// The byte one past a Data/Completion pair at `sequence`.
+fn pair_end(header: u64, stride: u64, sequence: u64) -> Result<u64, String> {
+    sequence
+        .checked_add(1)
+        .and_then(|pairs| pairs.checked_mul(2))
+        .and_then(|records| records.checked_mul(stride))
+        .and_then(|bytes| bytes.checked_add(header))
+        .ok_or_else(|| "observation authority pair end overflowed u64".to_owned())
+}
+
+/// Reads the two `N`-byte records of the pair at `sequence`: Data at physical
+/// record `2 x sequence`, Completion right after it (D-4766).
+fn read_pair_at<const N: usize>(
+    file: &mut File,
+    header: u64,
+    sequence: u64,
+) -> Result<([u8; N], [u8; N]), String> {
+    let stride = u64::try_from(N).map_err(|_| "record stride does not fit u64".to_owned())?;
+    let offset = sequence
+        .checked_mul(2)
+        .and_then(|record| record.checked_mul(stride))
+        .and_then(|bytes| bytes.checked_add(header))
+        .ok_or_else(|| "observation authority pair offset overflowed u64".to_owned())?;
+    let mut data = [0_u8; N];
+    let mut completion = [0_u8; N];
+    file.seek(SeekFrom::Start(offset))
+        .and_then(|_| file.read_exact(&mut data))
+        .and_then(|()| file.read_exact(&mut completion))
+        .map_err(|why| format!("cannot read pair {sequence}: {why}"))?;
+    Ok((data, completion))
+}
+
+/// One streaming read pass over exactly `len` bytes under `domain`, returning
+/// the digest of the first `cut` bytes and of all of them (the first `cut`
+/// bytes feed two hashers from one buffer); it equals
+/// `digest_authority_file` / `digest_observation_v2_file` of those bytes, with
+/// O(1) memory rather than a whole-file buffer (D-4766), proven by
+/// `cli::population_observations_v1::tests::a_v1_reverify_and_refresh_adopt_only_what_the_handle_verified`.
+fn hash_authority_prefix(
+    file: &mut File,
+    domain: &[u8],
+    cut: u64,
+    len: u64,
+) -> Result<([u8; 32], [u8; 32]), String> {
+    #[cfg(test)]
+    OBSERVATION_HASH_PASSES.with(|count| count.set(count.get().saturating_add(1)));
+    let rest = len.checked_sub(cut).ok_or_else(|| {
+        format!("observation authority file holds {len} bytes, fewer than the {cut} last verified")
+    })?;
+    file.seek(SeekFrom::Start(0))
+        .map_err(|why| format!("cannot seek observation authority file to hash it: {why}"))?;
+    let mut prefix = Hasher::new();
+    prefix.update(domain);
+    let mut whole = Hasher::new();
+    whole.update(domain);
+    hash_more(file, &mut [&mut prefix, &mut whole], cut)?;
+    hash_more(file, &mut [&mut whole], rest)?;
+    Ok((prefix.finalize(), whole.finalize()))
+}
+
+/// Feeds exactly `bytes` more bytes of `file` into every hasher in `hashers`.
+fn hash_more(file: &mut File, hashers: &mut [&mut Hasher], bytes: u64) -> Result<(), String> {
+    let mut buffer = [0_u8; 16 * 1_024];
+    let mut remaining = bytes;
+    while remaining != 0 {
+        let want = usize::try_from(remaining.min(16 * 1_024))
+            .map_err(|_| "hash chunk does not fit usize".to_owned())?;
+        let chunk = buffer
+            .get_mut(..want)
+            .ok_or_else(|| "hash chunk exceeds its buffer".to_owned())?;
+        let read = file
+            .read(chunk)
+            .map_err(|why| format!("cannot hash observation authority file: {why}"))?;
+        if read == 0 {
+            return Err("observation authority file ended while it was hashed".to_owned());
+        }
+        let chunk = buffer
+            .get(..read)
+            .ok_or_else(|| "hash read exceeds its buffer".to_owned())?;
+        for hasher in hashers.iter_mut() {
+            hasher.update(chunk);
+        }
+        remaining = remaining.saturating_sub(
+            u64::try_from(read).map_err(|_| "hash read does not fit u64".to_owned())?,
+        );
+    }
+    Ok(())
+}
+
 fn digest_sessions(sessions: &[i64]) -> [u8; 32] {
     let mut hasher = Hasher::new();
     hasher.update(b"brutex-candidate-observation-sessions-v1\0");
@@ -3122,9 +3328,10 @@ impl ObservationAuthorityAuditV2 {
 /// Whether an Observation V2 authority was written or exactly reused.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum ObservationAuthorityCommitV2 {
-    /// Data and then Completion were synchronized and freshly reopened.
+    /// Data and then Completion were synchronized and re-read through the
+    /// writer's handle (D-4766).
     Written(ObservationAuthorityAuditV2),
-    /// Exact semantic bytes already existed and were freshly reopened.
+    /// Exact semantic bytes already existed and were re-read.
     Reused(ObservationAuthorityAuditV2),
 }
 
@@ -3344,8 +3551,9 @@ impl ProducedObservationAuthorityV2 {
         })
     }
 
-    /// Persists Data first and Completion last, then requires a fresh read-only
-    /// reopen before returning success.
+    /// Persists Data first and Completion last, then re-reads the committed
+    /// pair through the writer's handle before returning success: one scan
+    /// per door where there were two (W2-cli11-3, D-4766).
     pub(crate) fn append_and_reopen(
         &self,
         root: impl AsRef<Path>,
@@ -3354,14 +3562,9 @@ impl ProducedObservationAuthorityV2 {
         let root = root.as_ref();
         let mut ledger = ObservationAuthorityLedgerV2::open(root, bounds)?;
         let committed = ledger.append_data(&self.value)?;
-        let expected = committed.audit();
-        drop(ledger);
-        let mut reopened = ObservationAuthorityLedgerV2::open_read(root, bounds)?;
-        let audit = reopened
-            .reopen_audit(&self.value.authority_id)?
-            .ok_or_else(|| "Observation V2 authority disappeared after append".to_owned())?;
-        if audit != expected {
-            return Err("Observation V2 fresh reopen differs from written bytes".to_owned());
+        let audit = ledger.reverify_committed(&committed)?;
+        if audit.authority_id != self.value.authority_id {
+            return Err("Observation V2 committed a foreign authority".to_owned());
         }
         Ok(match committed {
             ObservationAuthorityCommitV2::Written(_) => {
@@ -3413,7 +3616,9 @@ pub struct ObservationAuthorityLedgerV2 {
     audits: HashMap<[u8; 32], ObservationAuthorityAuditV2>,
     data_by_id: HashMap<[u8; 32], ObservationAuthorityDataV2>,
     orphan: Option<ObservationAuthorityDataV2>,
+    /// As [`ObservationAuthorityLedgerV1`]'s: the verified prefix digest.
     snapshot_digest: [u8; 32],
+    snapshot_len: u64,
     lock_generation: ObservationFileGenerationV1,
     file_generation: ObservationFileGenerationV1,
     writable: bool,
@@ -3496,11 +3701,16 @@ impl ObservationAuthorityLedgerV2 {
                 &observation_v2_header(),
             )?;
         }
+        // Measured before the read, re-checked after it (D-4766).
+        let lock_generation = observation_file_generation(&lock_file, &lock_path)?;
+        let file_generation = observation_file_generation(&file, &file_path)?;
         let bytes = read_bounded_observation_v2_file(&mut file, bounds)?;
         let (audits, data_by_id, orphan) = scan_observation_v2_file(&bytes, bounds)?;
         let snapshot_digest = digest_observation_v2_file(&bytes);
-        let lock_generation = observation_file_generation(&lock_file, &lock_path)?;
-        let file_generation = observation_file_generation(&file, &file_path)?;
+        let snapshot_len = u64::try_from(bytes.len())
+            .map_err(|_| "Observation V2 length does not fit u64".to_owned())?;
+        require_observation_generation(lock_generation, &lock_file, &lock_path)?;
+        require_observation_generation(file_generation, &file, &file_path)?;
         Ok(Self {
             lock_path,
             file_path,
@@ -3511,32 +3721,108 @@ impl ObservationAuthorityLedgerV2 {
             data_by_id,
             orphan,
             snapshot_digest,
+            snapshot_len,
             lock_generation,
             file_generation,
             writable,
         })
     }
 
-    /// Returns one cached audit only after detecting stale or replaced bytes.
+    /// Returns one cached audit only after re-verifying the bytes it names.
     ///
     /// # Complexity
     ///
-    /// Each lookup reads the whole bounded authority file into memory and
-    /// hashes it, so it is O(B) time and O(B) transient memory in file bytes
-    /// B; only the identity-map probe that follows is average O(1). The
-    /// content hash is kept deliberately: it is what refuses a same-length
-    /// edit a metadata generation cannot see (W2-cli11-3, D-1681). Invariant
-    /// LBE-06; UNVERIFIED as a measured time.
+    /// Average O(1) time and O(1) memory in file bytes: metadata-only
+    /// generation checks, one identity-map probe, and a re-read of the found
+    /// audit's 2,048-byte Data/Completion pair at
+    /// `HEADER + 1,024 x 2 x record_sequence`, whose seals must equal the
+    /// cached digests and whose records must decode to exactly the cached
+    /// Data. Before D-4766 each lookup read and hashed the whole file, O(B)
+    /// (W2-cli11-3). Not seen per lookup: a same-length rewrite of ANOTHER
+    /// authority's pair that leaves every metadata field equal; that pair's
+    /// own lookup and the next open refuse it. Invariant L1FE-06, proven by
+    /// `cli::population_observations_v1::tests::lookups_read_no_whole_file_and_one_append_door_scans_once`;
+    /// stated from the source, not timed.
     ///
     /// # Errors
     ///
-    /// Refuses any lock/data generation change or bounded content mismatch.
+    /// Refuses any lock/data generation change, a pair that no longer decodes
+    /// to the cached audit, or a read failure.
     pub fn reopen_audit(
         &mut self,
         authority_id: &[u8; 32],
     ) -> Result<Option<ObservationAuthorityAuditV2>, String> {
         self.require_unchanged()?;
-        Ok(self.audits.get(authority_id).copied())
+        let Some(audit) = self.audits.get(authority_id).copied() else {
+            return Ok(None);
+        };
+        self.reverify_pair(&audit)?;
+        Ok(Some(audit))
+    }
+
+    /// The V2 pair re-read: both seals must equal the cached digests and both
+    /// records must decode to the indexed Data at this sequence (D-4766).
+    fn reverify_pair(&mut self, audit: &ObservationAuthorityAuditV2) -> Result<(), String> {
+        let sequence = audit.record_sequence;
+        let reread = (|| {
+            let (data_raw, completion_raw) = read_pair_at::<AUTHORITY_V2_RECORD_BYTES>(
+                &mut self.file,
+                OBSERVATION_AUTHORITY_HEADER_BYTES_V2,
+                sequence,
+            )?;
+            let (data_kind, data) = ObservationAuthorityDataV2::decode(&data_raw)?;
+            let (completion_kind, completion) =
+                ObservationAuthorityDataV2::decode(&completion_raw)?;
+            if data_kind != AUTHORITY_V2_DATA_KIND
+                || completion_kind != AUTHORITY_V2_COMPLETION_KIND
+                || data.record_sequence != sequence
+                || completion != data
+            {
+                return Err("its records are not this sequence's Data/Completion pair".to_owned());
+            }
+            if get_fixed::<32>(&data_raw, AUTHORITY_V2_PAYLOAD_BYTES)? != audit.data_record_digest
+                || get_fixed::<32>(&completion_raw, AUTHORITY_V2_PAYLOAD_BYTES)?
+                    != audit.completion_digest
+                || observation_v2_audit(&data)? != *audit
+                || self.data_by_id.get(&data.authority_id) != Some(&data)
+            {
+                return Err("its records decode to another authority".to_owned());
+            }
+            Ok(())
+        })();
+        reread.map_err(|why| format!("Observation V2 pair {sequence} changed after open: {why}"))
+    }
+
+    /// The V2 twin of [`ObservationAuthorityLedgerV1::reverify_committed`].
+    fn reverify_committed(
+        &mut self,
+        committed: &ObservationAuthorityCommitV2,
+    ) -> Result<ObservationAuthorityAuditV2, String> {
+        let expected = committed.audit();
+        self.require_unchanged()?;
+        if self.audits.get(&expected.authority_id) != Some(&expected) {
+            return Err(
+                "Observation V2 committed audit is not the one this handle indexed".to_owned(),
+            );
+        }
+        let pair_end = pair_end(
+            OBSERVATION_AUTHORITY_HEADER_BYTES_V2,
+            OBSERVATION_AUTHORITY_RECORD_STRIDE_V2,
+            expected.record_sequence,
+        )?;
+        let len = self.file_generation.len;
+        let in_place = match committed {
+            ObservationAuthorityCommitV2::Written(_) => len == pair_end,
+            ObservationAuthorityCommitV2::Reused(_) => len >= pair_end,
+        };
+        if !in_place {
+            return Err(format!(
+                "Observation V2 file holds {len} bytes; the committed pair ends at byte {pair_end}"
+            ));
+        }
+        self.reverify_pair(&expected)?;
+        self.require_unchanged()?;
+        Ok(expected)
     }
 
     fn append_data(
@@ -3619,14 +3905,10 @@ impl ObservationAuthorityLedgerV2 {
         Ok(())
     }
 
-    fn require_unchanged(&mut self) -> Result<(), String> {
+    /// Metadata only, as V1's (D-4766).
+    fn require_unchanged(&self) -> Result<(), String> {
         require_observation_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
-        require_observation_generation(self.file_generation, &self.file, &self.file_path)?;
-        let bytes = read_bounded_observation_v2_file(&mut self.file, self.bounds)?;
-        if digest_observation_v2_file(&bytes) != self.snapshot_digest {
-            return Err("Observation V2 file changed after open".to_owned());
-        }
-        Ok(())
+        require_observation_generation(self.file_generation, &self.file, &self.file_path)
     }
 
     /// Appends one record through the shared rollback, then syncs it (D-1854).
@@ -3686,11 +3968,28 @@ impl ObservationAuthorityLedgerV2 {
             })
     }
 
+    /// As V1's: one streaming pass that must reproduce the verified prefix,
+    /// then extends it and re-measures the generations (D-4766).
     fn refresh_snapshot(&mut self) -> Result<(), String> {
-        let bytes = read_bounded_observation_v2_file(&mut self.file, self.bounds)?;
-        self.snapshot_digest = digest_observation_v2_file(&bytes);
-        self.lock_generation = observation_file_generation(&self.lock_file, &self.lock_path)?;
-        self.file_generation = observation_file_generation(&self.file, &self.file_path)?;
+        let lock_generation = observation_file_generation(&self.lock_file, &self.lock_path)?;
+        let file_generation = observation_file_generation(&self.file, &self.file_path)?;
+        let (prefix, full) = hash_authority_prefix(
+            &mut self.file,
+            AUTHORITY_V2_FILE_DOMAIN,
+            self.snapshot_len,
+            file_generation.len,
+        )?;
+        if prefix != self.snapshot_digest {
+            return Err(format!(
+                "Observation V2 file changed below byte {} while this handle appended",
+                self.snapshot_len
+            ));
+        }
+        require_observation_generation(file_generation, &self.file, &self.file_path)?;
+        self.snapshot_digest = full;
+        self.snapshot_len = file_generation.len;
+        self.lock_generation = lock_generation;
+        self.file_generation = file_generation;
         Ok(())
     }
 }
@@ -3771,6 +4070,8 @@ fn scan_observation_v2_file(
     bytes: &[u8],
     bounds: ObservationAuthorityBoundsV2,
 ) -> Result<ObservationV2Scan, String> {
+    #[cfg(test)]
+    OBSERVATION_SCANS.with(|count| count.set(count.get().saturating_add(1)));
     if bytes.get(..AUTHORITY_V2_HEADER_BYTES) != Some(observation_v2_header().as_slice()) {
         return Err("Observation V2 file header is absent or corrupt".to_owned());
     }
@@ -3848,6 +4149,8 @@ fn read_bounded_observation_v2_file(
     file: &mut File,
     bounds: ObservationAuthorityBoundsV2,
 ) -> Result<Vec<u8>, String> {
+    #[cfg(test)]
+    OBSERVATION_FILE_READS.with(|count| count.set(count.get().saturating_add(1)));
     let len = file
         .metadata()
         .map_err(|why| format!("cannot stat Observation V2 file: {why}"))?
@@ -4186,6 +4489,347 @@ mod tests {
         );
     }
 
+    /// Zeroes the three Observation counters; returns a reader of
+    /// (whole-file reads, scans, streaming hash passes).
+    fn observation_counts() -> impl Fn() -> (u64, u64, u64) {
+        OBSERVATION_FILE_READS.with(|count| count.set(0));
+        OBSERVATION_SCANS.with(|count| count.set(0));
+        OBSERVATION_HASH_PASSES.with(|count| count.set(0));
+        || {
+            (
+                OBSERVATION_FILE_READS.with(std::cell::Cell::get),
+                OBSERVATION_SCANS.with(std::cell::Cell::get),
+                OBSERVATION_HASH_PASSES.with(std::cell::Cell::get),
+            )
+        }
+    }
+
+    #[test]
+    fn lookups_read_no_whole_file_and_one_append_door_scans_once() {
+        // W2-cli11-3: each lookup read and hashed the whole file, and each
+        // append door made about five whole-file passes and two scans.
+        let bounds = authority_bounds();
+        let root = test_dir();
+        let data = authority_data_fixture(70);
+        let read = observation_counts();
+        let written =
+            append_authority_data_and_reopen(root.path(), bounds, &data).expect("V1 door writes");
+        assert!(matches!(written, ObservationAuthorityCommitV1::Written(_)));
+        assert_eq!(
+            read(),
+            (1, 1, 1),
+            "V1 written: one open, one post-write pass"
+        );
+        let read = observation_counts();
+        let reused =
+            append_authority_data_and_reopen(root.path(), bounds, &data).expect("V1 door reuses");
+        assert!(matches!(reused, ObservationAuthorityCommitV1::Reused(_)));
+        assert_eq!(read(), (1, 1, 0), "V1 reused: one open and nothing else");
+        let mut ledger =
+            ObservationAuthorityLedgerV1::open_read(root.path(), bounds).expect("V1 reader opens");
+        let read = observation_counts();
+        for _ in 0..10 {
+            assert_eq!(
+                ledger.reopen_audit(&data.authority_id).expect("V1 lookup"),
+                Some(written.audit())
+            );
+            assert_eq!(
+                ledger.reopen_audit(&[0x5A; 32]).expect("V1 absent lookup"),
+                None
+            );
+        }
+        assert_eq!(read(), (0, 0, 0), "twenty V1 lookups read no whole file");
+
+        let bounds_v2 = authority_bounds_v2();
+        let root_v2 = test_dir();
+        let (source, commit) =
+            crate::pre_admission_data::observation_v2_zero_production_fixture(88)
+                .expect("zero source fixture derives");
+        let produced = produce_natural_extinction_observation_v2(&source, &commit)
+            .expect("zero source prepares Observation V2");
+        let read = observation_counts();
+        let written_v2 = produced
+            .append_and_reopen(root_v2.path(), bounds_v2)
+            .expect("V2 door writes");
+        assert!(matches!(
+            written_v2,
+            ObservationAuthorityCommitV2::Written(_)
+        ));
+        assert_eq!(
+            read(),
+            (1, 1, 1),
+            "V2 written: one open, one post-write pass"
+        );
+        let read = observation_counts();
+        assert!(matches!(
+            produced
+                .append_and_reopen(root_v2.path(), bounds_v2)
+                .expect("V2 door reuses"),
+            ObservationAuthorityCommitV2::Reused(_)
+        ));
+        assert_eq!(read(), (1, 1, 0), "V2 reused: one open and nothing else");
+        let mut ledger_v2 = ObservationAuthorityLedgerV2::open_read(root_v2.path(), bounds_v2)
+            .expect("V2 reader opens");
+        let read = observation_counts();
+        for _ in 0..10 {
+            assert_eq!(
+                ledger_v2
+                    .reopen_audit(&produced.value.authority_id)
+                    .expect("V2 lookup"),
+                Some(written_v2.audit())
+            );
+            assert_eq!(
+                ledger_v2
+                    .reopen_audit(&[0x5A; 32])
+                    .expect("V2 absent lookup"),
+                None
+            );
+        }
+        assert_eq!(read(), (0, 0, 0), "twenty V2 lookups read no whole file");
+    }
+
+    /// Flips one byte at `offset` in place, as a non-cooperating writer would.
+    fn flip_at(path: &Path, offset: u64) {
+        let mut file = OpenOptions::new()
+            .read(true)
+            .write(true)
+            .open(path)
+            .expect("fixture file opens");
+        let mut byte = [0_u8; 1];
+        file.seek(SeekFrom::Start(offset))
+            .and_then(|_| file.read_exact(&mut byte))
+            .expect("fixture byte reads");
+        byte[0] ^= 1;
+        file.seek(SeekFrom::Start(offset))
+            .and_then(|_| file.write_all(&byte))
+            .and_then(|()| file.sync_data())
+            .expect("fixture byte writes");
+    }
+
+    /// Pins the file's modification time far from now, so a change is visible
+    /// to a metadata generation whatever the filesystem's timestamp tick.
+    fn pin_mtime(path: &Path) {
+        OpenOptions::new()
+            .write(true)
+            .open(path)
+            .expect("fixture file opens")
+            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1))
+            .expect("fixture mtime sets");
+    }
+
+    /// Byte 40 of the Data (or Completion) record of the pair at `sequence`.
+    fn pair_byte(header: u64, stride: u64, sequence: u64, completion: bool) -> u64 {
+        header + stride * (2 * sequence + u64::from(completion)) + 40
+    }
+
+    #[test]
+    fn a_v1_lookup_rereads_its_own_pair_and_not_another() {
+        // D-4766: metadata generations plus the looked-up pair. A rewrite
+        // inside one timestamp tick is modelled by re-measuring the cached
+        // file generation after it.
+        let bounds = authority_bounds();
+        let root = test_dir();
+        let first = authority_data_fixture(71);
+        let second = authority_data_fixture(72);
+        append_authority_data_and_reopen(root.path(), bounds, &first).expect("first commits");
+        append_authority_data_and_reopen(root.path(), bounds, &second).expect("second commits");
+        let path = root.path().join(AUTHORITY_FILE);
+        let at = |sequence, completion| {
+            pair_byte(
+                OBSERVATION_AUTHORITY_HEADER_BYTES_V1,
+                OBSERVATION_AUTHORITY_RECORD_STRIDE_V1,
+                sequence,
+                completion,
+            )
+        };
+        for completion in [false, true] {
+            let mut ledger =
+                ObservationAuthorityLedgerV1::open_read(root.path(), bounds).expect("V1 opens");
+            flip_at(&path, at(0, completion));
+            ledger.file_generation =
+                observation_file_generation(&ledger.file, &ledger.file_path).expect("re-measures");
+            let why = ledger
+                .reopen_audit(&first.authority_id)
+                .expect_err("its own pair refuses");
+            assert!(why.contains("pair 0 changed after open"), "{why}");
+            assert!(
+                ledger
+                    .reopen_audit(&second.authority_id)
+                    .expect("another pair is not read")
+                    .is_some()
+            );
+            flip_at(&path, at(0, completion));
+        }
+        let mut ledger =
+            ObservationAuthorityLedgerV1::open_read(root.path(), bounds).expect("V1 opens");
+        flip_at(&path, at(1, false));
+        ledger.file_generation =
+            observation_file_generation(&ledger.file, &ledger.file_path).expect("re-measures");
+        assert!(
+            ledger
+                .reopen_audit(&first.authority_id)
+                .expect("the limit: another pair is not seen by this lookup")
+                .is_some()
+        );
+        assert!(
+            ledger
+                .reopen_audit(&second.authority_id)
+                .expect_err("that pair's own lookup refuses")
+                .contains("pair 1 changed after open")
+        );
+        drop(ledger);
+        assert!(
+            ObservationAuthorityLedgerV1::open_read(root.path(), bounds).is_err(),
+            "the next open refuses it"
+        );
+        flip_at(&path, at(1, false));
+        let mut ledger =
+            ObservationAuthorityLedgerV1::open_read(root.path(), bounds).expect("V1 opens");
+        pin_mtime(&path);
+        let why = ledger
+            .reopen_audit(&[0x5A; 32])
+            .expect_err("a metadata-visible change refuses even an absent id");
+        assert!(why.contains("changed since open"), "{why}");
+    }
+
+    #[test]
+    fn a_v1_reverify_and_refresh_adopt_only_what_the_handle_verified() {
+        let bounds = authority_bounds();
+        let root = test_dir();
+        let first = authority_data_fixture(73);
+        let one = append_authority_data_and_reopen(root.path(), bounds, &first)
+            .expect("first commits")
+            .audit();
+        append_authority_data_and_reopen(root.path(), bounds, &authority_data_fixture(74))
+            .expect("second commits");
+        let path = root.path().join(AUTHORITY_FILE);
+        let mut ledger =
+            ObservationAuthorityLedgerV1::open(root.path(), bounds).expect("V1 writer opens");
+        assert_eq!(
+            ledger
+                .reverify_committed(&ObservationAuthorityCommitV1::Reused(one))
+                .expect("a reused pair inside the file re-reads"),
+            one
+        );
+        let why = ledger
+            .reverify_committed(&ObservationAuthorityCommitV1::Written(one))
+            .expect_err("a written pair must end the file");
+        assert!(why.contains("the committed pair ends at byte"), "{why}");
+        let mut foreign = one;
+        foreign.authority_id = [0xEE; 32];
+        let why = ledger
+            .reverify_committed(&ObservationAuthorityCommitV1::Reused(foreign))
+            .expect_err("an audit this handle did not index");
+        assert!(why.contains("not the one this handle indexed"), "{why}");
+
+        flip_at(&path, OBSERVATION_AUTHORITY_HEADER_BYTES_V1 + 40);
+        let why = ledger
+            .refresh_snapshot()
+            .expect_err("an edit below the verified length refuses");
+        assert!(why.contains("changed below byte"), "{why}");
+        flip_at(&path, OBSERVATION_AUTHORITY_HEADER_BYTES_V1 + 40);
+        ledger
+            .refresh_snapshot()
+            .expect("the verified bytes refresh");
+        let bytes = std::fs::read(&path).expect("authority file reads");
+        assert_eq!(
+            ledger.snapshot_len,
+            u64::try_from(bytes.len()).expect("length fits")
+        );
+        assert_eq!(ledger.snapshot_digest, digest_authority_file(&bytes));
+    }
+
+    #[test]
+    fn a_v2_lookup_reverify_and_refresh_reread_only_their_own_pair() {
+        let bounds = authority_bounds_v2();
+        let root = test_dir();
+        let mut produced = Vec::new();
+        for tag in [88, 89] {
+            let (source, commit) =
+                crate::pre_admission_data::observation_v2_zero_production_fixture(tag)
+                    .expect("zero source fixture derives");
+            let value = produce_natural_extinction_observation_v2(&source, &commit)
+                .expect("zero source prepares Observation V2");
+            value
+                .append_and_reopen(root.path(), bounds)
+                .expect("V2 commits");
+            produced.push(value);
+        }
+        let first = produced[0].value.authority_id;
+        let second = produced[1].value.authority_id;
+        let path = root.path().join(AUTHORITY_V2_FILE);
+        let at = |sequence, completion| {
+            pair_byte(
+                OBSERVATION_AUTHORITY_HEADER_BYTES_V2,
+                OBSERVATION_AUTHORITY_RECORD_STRIDE_V2,
+                sequence,
+                completion,
+            )
+        };
+        for completion in [false, true] {
+            let mut ledger =
+                ObservationAuthorityLedgerV2::open_read(root.path(), bounds).expect("V2 opens");
+            flip_at(&path, at(0, completion));
+            ledger.file_generation =
+                observation_file_generation(&ledger.file, &ledger.file_path).expect("re-measures");
+            let why = ledger
+                .reopen_audit(&first)
+                .expect_err("its own pair refuses");
+            assert!(why.contains("pair 0 changed after open"), "{why}");
+            assert!(
+                ledger
+                    .reopen_audit(&second)
+                    .expect("another pair")
+                    .is_some()
+            );
+            flip_at(&path, at(0, completion));
+        }
+        let mut ledger =
+            ObservationAuthorityLedgerV2::open(root.path(), bounds).expect("V2 writer opens");
+        let one = ledger
+            .reopen_audit(&first)
+            .expect("lookup reads")
+            .expect("indexed");
+        assert_eq!(
+            ledger
+                .reverify_committed(&ObservationAuthorityCommitV2::Reused(one))
+                .expect("a reused pair re-reads"),
+            one
+        );
+        let why = ledger
+            .reverify_committed(&ObservationAuthorityCommitV2::Written(one))
+            .expect_err("a written pair must end the file");
+        assert!(why.contains("the committed pair ends at byte"), "{why}");
+        let mut foreign = one;
+        foreign.authority_id = [0xEE; 32];
+        assert!(
+            ledger
+                .reverify_committed(&ObservationAuthorityCommitV2::Reused(foreign))
+                .expect_err("not indexed")
+                .contains("not the one this handle indexed")
+        );
+        flip_at(&path, at(0, false));
+        assert!(
+            ledger
+                .refresh_snapshot()
+                .expect_err("an edit below the verified length refuses")
+                .contains("changed below byte")
+        );
+        flip_at(&path, at(0, false));
+        ledger
+            .refresh_snapshot()
+            .expect("the verified bytes refresh");
+        let bytes = std::fs::read(&path).expect("V2 file reads");
+        assert_eq!(ledger.snapshot_digest, digest_observation_v2_file(&bytes));
+        pin_mtime(&path);
+        assert!(
+            ledger
+                .reopen_audit(&[0x5A; 32])
+                .expect_err("a metadata-visible change refuses")
+                .contains("changed since open")
+        );
+    }
+
     #[test]
     fn authority_is_receipt_last_freshly_reopened_and_exactly_reused() {
         let root = test_dir();
diff --git a/crates/cli/tests/ledger_append_lookup_costs.rs b/crates/cli/tests/ledger_append_lookup_costs.rs
index f3dbacc4..e81040b7 100644
--- a/crates/cli/tests/ledger_append_lookup_costs.rs
+++ b/crates/cli/tests/ledger_append_lookup_costs.rs
@@ -21,6 +21,8 @@
 //!   names and now pin the one open.
 //! * G4-1 (D-4765): every cached Statistics V2 read hashed the whole data file
 //!   four times while §154 said a lookup is average O(1) and a page O(rows).
+//! * W2-cli11-3 (D-4766): the Observation lookups no longer hash the whole
+//!   file; the test LBE-06 names for that keeps its name and pins the new shape.
 //!
 //! Every file a constant below names is read at compile time, so a rename
 //! fails the build rather than skipping the check. A separate test crate, as
@@ -150,18 +152,26 @@ fn a_pre_admission_page_checks_metadata_and_its_lookups_are_priced_by_file_bytes
     let section_153 = flat(section(153));
     assert!(section_153.contains("A page costs O(P) for P returned rows: since D-1681"));
     assert!(section_153.contains("A cached `reopen_audit` lookup still content-hashes both files"));
+    // §157 and §161 price the Observation lookups at average O(1) since
+    // D-4766, which `observation_lookups_still_hash_the_whole_file_and_say_so`
+    // pins against the source.
     let section_157 = flat(section(157));
     assert!(!section_157.contains("hashes its bounded bytes; hash-index lookup is average O(1)"));
-    assert!(section_157.contains("so one lookup is O(file bytes)"));
+    assert!(!section_157.contains("so one lookup is O(file bytes)"));
+    assert!(section_157.contains("A cached `reopen_audit` lookup is average O(1) since D-4766"));
     let section_161 = flat(section(161));
     assert!(!section_161.contains("identity-map lookup is average O(1); allocation"));
-    assert!(section_161.contains("reads and hashes the whole bounded file, O(B)"));
+    assert!(!section_161.contains("lookup reads and hashes the whole bounded file, O(B), before"));
+    assert!(section_161.contains("One cached `reopen_audit` lookup is average O(1)"));
 
     let module = flat(PRE_ADMISSION.get(..3_000).expect("the module header"));
     assert!(!module.contains("A page is proportional to the returned records after that scan"));
     assert!(module.contains("A page checks file generations by metadata only"));
 }
 
+/// LBE-06 names this test, and invariant rows are append-only, so the name
+/// stays. What it pinned, a whole-file read per Observation lookup, D-4766
+/// removed: it now pins the metadata check and the pair re-read (L1FE-06).
 #[test]
 fn observation_lookups_still_hash_the_whole_file_and_say_so() {
     let mut lookups = 0;
@@ -169,24 +179,37 @@ fn observation_lookups_still_hash_the_whole_file_and_say_so() {
     while let Some(at) = rest.find("    pub fn reopen_audit(") {
         let body = method(rest, "    pub fn reopen_audit(");
         assert!(body.contains("self.require_unchanged()?"));
+        assert!(body.contains("self.reverify_pair(&audit)?"));
         let doc_start = rest
             .get(..at)
             .expect("a prefix")
             .rfind("\n\n")
             .expect("a gap");
         let doc = flat(rest.get(doc_start..at).expect("the rustdoc"));
-        assert!(doc.contains("so it is O(B) time and O(B) transient memory"));
+        assert!(!doc.contains("so it is O(B) time and O(B) transient memory"));
+        assert!(doc.contains("Average O(1) time and O(1) memory in file bytes"));
         lookups += 1;
         rest = rest.get(at + 1..).expect("a suffix");
     }
     assert_eq!(lookups, 2, "Observation V1 and V2 each have one lookup");
-    let unchanged = method(OBSERVATIONS, "    fn require_unchanged(&mut self)");
-    assert!(unchanged.contains("read_bounded_authority_file"));
+    let mut checks = 0;
+    let mut rest = OBSERVATIONS;
+    while let Some(at) = rest.find("    fn require_unchanged(&self)") {
+        let unchanged = method(rest, "    fn require_unchanged(&self)");
+        assert!(
+            !unchanged.contains("read_bounded_") && !unchanged.contains("digest_"),
+            "an Observation lookup reads the whole file again; re-measure §157 and §161"
+        );
+        checks += 1;
+        rest = rest.get(at + 1..).expect("a suffix");
+    }
+    assert_eq!(checks, 2, "V1 and V2 each have one metadata check");
+    assert!(!OBSERVATIONS.contains("fn require_unchanged(&mut self)"));
 
     let chapter = chapter(CHAPTER);
     for needed in [
         "Pre-Admission Data V1 `page`** (§153) is now O(P)",
-        "Observation V1 and V2 `reopen_audit`** (§157, §161) read the whole bounded authority file",
+        "Observation V1 and V2 `reopen_audit`** (§157, §161) are average O(1) since D-4766",
         "Finalization V2 `reopen_structural_receipt`** re-hashes the bounded data file",
     ] {
         assert!(
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 1ebc5dec..d3dbb993 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7057,3 +7057,5 @@ old line regex the same input and watched it pass.
 | L1FE-03 | `reverify_committed` re-reads a reused block; it refuses a written block that does not end the file, an audit absent from or unlike the index, a disk block unlike the index, and a same-length edit elsewhere under re-measured metadata (D-4764) | `cli::population_statistics_v2::tests::reverify_rereads_only_what_this_handle_committed` | ✓ |
 | L1FE-04 | The Statistics V2 door opens once and never read-only, the step's projection opens nothing, and §154 prices one open per append and one scan per step (D-4764) | `cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append` | ✓ |
 | L1FE-05 | A cached Statistics V2 read hashes no file (81 reads after an open hash 0 times; the open hashes 4 times); a metadata-visible change refuses every read; inside one timestamp tick a lookup refuses its own changed Data or Completion record, a candidate read or page refuses an unsealed returned row, and the family-wide reads refuse a resealed row that still validates, which a single read returns (the stated limit) and the next open refuses; a trailing-prefix read refuses a changed orphan Data record (D-4765) | `cli::population_statistics_v2::tests::cached_reads_hash_no_file_and_reread_only_what_they_return`, `cli::population_statistics_v2::tests::cached_reads_reverify_what_they_return_and_the_limit_is_what_they_do_not`, `cli::population_statistics_v2::tests::a_trailing_prefix_read_reverifies_the_orphan_data_record`, `cli::population_statistics_v2::tests::explicit_bounds_and_post_open_same_length_mutation_refuse`, `cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append` | ✓ |
+| L1FE-06 | An Observation V1 or V2 lookup reads no whole file (20 lookups: 0 reads, 0 scans, 0 hash passes) and an append door runs one open and one scan, plus one streaming hash pass when it writes; inside one timestamp tick a lookup refuses its own changed Data or Completion and does not see another authority's pair (the stated limit), which that pair's lookup and the next open refuse; a metadata-visible change refuses even an absent id; §157, §161 and both lookups' rustdoc state it (D-4766) | `cli::population_observations_v1::tests::lookups_read_no_whole_file_and_one_append_door_scans_once`, `cli::population_observations_v1::tests::a_v1_lookup_rereads_its_own_pair_and_not_another`, `cli::population_observations_v1::tests::a_v2_lookup_reverify_and_refresh_reread_only_their_own_pair`, `cli::ledger_append_lookup_costs::observation_lookups_still_hash_the_whole_file_and_say_so`, `cli::ledger_append_lookup_costs::a_pre_admission_page_checks_metadata_and_its_lookups_are_priced_by_file_bytes` | ✓ |
+| L1FE-07 | An Observation door's re-read re-reads a reused pair and refuses a written pair that does not end the file or an audit the handle did not index; the post-write refresh refuses an edit below the verified length and adopts the verified bytes, whose streaming digest equals the whole-file digest (D-4766) | `cli::population_observations_v1::tests::a_v1_reverify_and_refresh_adopt_only_what_the_handle_verified`, `cli::population_observations_v1::tests::a_v2_lookup_reverify_and_refresh_reread_only_their_own_pair` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 0da99d87..372827d9 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65137,3 +65137,52 @@ right 0),
 (its rewrites now pin the modification time, so the metadata refusal does not
 depend on the timestamp tick),
 `cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append`.
+
+### D-4766 — An Observation lookup reads its own pair, and an append door opens once — 2026-10-09
+
+**What was wrong.** W2-cli11-3: each Observation V1 and V2 `reopen_audit` read
+the whole bounded authority file into memory and hashed it, O(B) time and O(B)
+memory per lookup. The only production lookup is the append door's own fresh
+reopen. Each door made five whole-file reads and two scans: the writer's open,
+the pre-append check, the post-write refresh, a second open, and the lookup.
+D-1681 stated the cost and kept the hash.
+
+**Decided.** The plan G4 §C named, checked against the layout: an authority is
+a contiguous Data/Completion pair at physical records `2·seq` and `2·seq+1`,
+stride 512 bytes in V1 and 1,024 in V2.
+
+- `require_unchanged` is metadata only, the existing pair of
+  `require_observation_generation` calls on the lock and the file. An open now
+  measures those generations before it reads and re-checks them after, so the
+  bytes its scan validated are the bytes later checks compare against.
+- A lookup re-reads the found pair at its fixed offset. Both seals are checked,
+  the Completion must name the sequence and validate against the Data, and the
+  pair must decode to exactly the cached audit and indexed Data. V2 also
+  requires the raw seals to equal the cached digests.
+- The append door re-reads only the committed pair through the writer's handle
+  (`reverify_committed`, the D-1680 shape). It checks the generation, that a
+  written pair ends the file, and the index entry, then re-reads the pair.
+- The post-write refresh is one streaming hash pass with O(1) memory. It must
+  first reproduce the verified digest of the bytes the handle had already
+  validated, so a non-cooperating edit below the new pair is refused rather
+  than adopted. The removed second open used to catch that edit.
+
+A door is now one open (one read, one scan) plus one hash pass on a write, and
+none on a reuse. No byte, format or identity changes.
+
+**What is no longer seen per lookup.** An equal-metadata, same-length rewrite
+of another authority's pair. That pair's own lookup and the next open refuse
+it, the residual D-1681 accepted for the Pre-Admission page.
+
+**Rejected.** Re-verifying only the previous tail pair before an append, as G4
+§C sketched. The streaming prefix check after the write covers every byte
+below the new pair, so it subsumes that.
+
+Tests: `cli::population_observations_v1::tests::lookups_read_no_whole_file_and_one_append_door_scans_once`
+(failed first on the unfixed door: "V1 written: one open, one post-write pass",
+left (5, 2, 0), right (1, 1, 1)),
+`cli::population_observations_v1::tests::a_v1_lookup_rereads_its_own_pair_and_not_another`,
+`cli::population_observations_v1::tests::a_v1_reverify_and_refresh_adopt_only_what_the_handle_verified`,
+`cli::population_observations_v1::tests::a_v2_lookup_reverify_and_refresh_reread_only_their_own_pair`,
+`cli::ledger_append_lookup_costs::observation_lookups_still_hash_the_whole_file_and_say_so`,
+`cli::ledger_append_lookup_costs::a_pre_admission_page_checks_metadata_and_its_lookups_are_priced_by_file_bytes`.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 75bbc65e..1be88fee 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -8325,10 +8325,17 @@ The companion authority contains only fixed-size identities, counts and
 digests. It does not durably retain the O(C·P + C·K) raw evidence. Crash recovery
 therefore requires the upstream exact Candidate replay to derive the same
 observations again before an orphan Data may receive Completion. Opening the
-ledger scans and hashes its bounded bytes. A cached `reopen_audit` lookup
-reads and hashes the whole bounded file again before its hash-index probe, so
-one lookup is O(file bytes); only the probe itself is average O(1) (D-1681
-corrected an older sentence here that called the lookup average O(1)). One
+ledger reads, scans and hashes its bounded bytes once. A cached `reopen_audit`
+lookup is average O(1) since D-4766: it compares the lock and data generations
+by metadata only, probes the identity map once and re-reads the found
+authority's 1,024-byte Data/Completion pair at its fixed offset, which must
+decode to exactly the cached audit; no file is read whole or hashed. Until
+D-4766 each lookup read and hashed the whole bounded file, O(file bytes), which
+D-1681 had stated here (W2-cli11-3). Not seen per lookup: a same-length rewrite
+of another authority's pair that leaves every metadata field equal; that pair's
+own lookup and the next open refuse it. One append door is one open plus one
+streaming hash pass after the write, which must first reproduce the digest of
+the bytes the open validated, and a re-read of the committed pair. One
 fixed-stride record position is worst-case O(1) in record count after
 admission. Allocation, hashing, locking, sync and device latency are not
 constant-time.
@@ -8442,16 +8449,20 @@ record, so the zero-family outcome retains the full Candidate/source and
 natural-extinction proof without fabricating an observation row or statistic.
 That constant record width does not make production, open or persistence O(1).
 
-For A admitted Observation authorities and B file bytes, open/fresh reopen scan
-and validate O(B) bytes and retain O(A) identity/data indexes. Append validates
+For A admitted Observation authorities and B file bytes, an open scans and
+validates O(B) bytes and retains O(A) identity/data indexes. Append validates
 the embedded Pre-Admission record, hashes fixed records, synchronizes Data then
-Completion, hashes the bounded file and freshly reopens it. One fixed-stride
-record address is worst-case O(1) in record count after admission. One cached
-`reopen_audit` lookup reads and hashes the whole bounded file, O(B), before an
-identity-map probe that is average O(1) (D-1681); allocation, locking, synchronization,
-filesystem traversal, page faults, controller behavior, removable-drive loss
-and latency have no constant bound. The explicit byte/authority ceilings refuse
-excess; they do not truncate history or hide a failure.
+Completion, makes one streaming hash pass over the file that must reproduce the
+digest of the bytes the open validated, and re-reads the committed pair through
+the writer's handle: one open per append door where there were two (D-4766).
+One fixed-stride record address is worst-case O(1) in record count after
+admission. One cached `reopen_audit` lookup is average O(1): metadata-only
+generation checks, an identity-map probe and a re-read of the found authority's
+2,048-byte pair. Until D-4766 it read and hashed the whole bounded file, O(B)
+(D-1681). Allocation, locking, synchronization, filesystem traversal, page
+faults, controller behavior, removable-drive loss and latency have no constant
+bound. The explicit byte/authority ceilings refuse excess; they do not truncate
+history or hide a failure.
 
 The focused Observation V1+V2 suite is **11/11 green** and covers the exact
 opaque Pre-Admission commit join, NIFTY/BANKNIFTY family separation, nonzero
@@ -15412,10 +15423,16 @@ or average O(1).
   equal. A rewrite of a record it returns is refused by that record's seal.
 - **Pre-Admission Data V1 `reopen_audit`** still content-hashes the lock and
   data files, O(file bytes) per lookup, then probes its map in average O(1).
-- **Observation V1 and V2 `reopen_audit`** (§157, §161) read the whole bounded
-  authority file into memory and hash it, O(B) time and O(B) transient memory
-  per lookup, before an average-O(1) map probe. Kept deliberately: they are
-  audit-only lookups, and the content hash is what refuses a same-length edit.
+- **Observation V1 and V2 `reopen_audit`** (§157, §161) are average O(1) since
+  D-4766: metadata-only generation checks, one map probe and a re-read of the
+  found authority's Data/Completion pair (1,024 bytes in V1, 2,048 in V2),
+  which must decode to the cached audit. Until D-4766 they read the whole
+  bounded authority file into memory and hashed it, O(B) time and O(B)
+  transient memory per lookup. Not seen per lookup: an equal-metadata
+  same-length rewrite of another authority's pair; its own lookup and the next
+  open refuse it. Each append door is one open and one scan where it was two,
+  plus one streaming hash pass after the write that must reproduce the bytes
+  the open validated.
 - **Finalization V2 `reopen_structural_receipt`** re-hashes the bounded data
   file through `require_unchanged`, O(file bytes) per lookup, before an
   average-O(1) probe; its rustdoc already said so and this section is its
-- 
2.43.0


From b4b9ff1d661123d5f654296b90240a6f75cfc2a6 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:31:42 +0000
Subject: [PATCH 4/5] cli: O(1) Finalization V2 structural-receipt lookup
 (W2-cli11-2, D-4767)

Before: reopen_structural_receipt ran require_unchanged, which
content-hashes the lock file and the whole data file, O(file bytes) per
lookup.

After: require_platform_unchanged checks the root directory generation
and the length and platform generation of the held and named lock and
data files without reading content, and a found receipt's Data and
Completion records (two fixed 2,048-byte reads at first_physical_record
and first_physical_record + rekey_count + 1) must re-digest to the
receipt's raw-record digests. Independent of file size and Rekey count.
The open keeps its content hashes; the dormant append is unchanged.

Fail-before: lookups_hash_no_file_and_redigest_only_the_two_records_they_name
"twenty lookups hash no file" left 40 right 0.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/population_finalization_v2.rs  | 269 +++++++++++++++++-
 .../cli/tests/ledger_append_lookup_costs.rs   |  32 ++-
 docs/04-invariants.md                         |   1 +
 docs/05-decisions.md                          |  40 +++
 docs/06-limits.md                             |  20 +-
 5 files changed, 339 insertions(+), 23 deletions(-)

diff --git a/crates/cli/src/population_finalization_v2.rs b/crates/cli/src/population_finalization_v2.rs
index 4cb99f71..b72104c3 100644
--- a/crates/cli/src/population_finalization_v2.rs
+++ b/crates/cli/src/population_finalization_v2.rs
@@ -1170,10 +1170,12 @@ struct LedgerGenerationWitnessV2 {
 /// Open bounded Finalization V2 ledger.
 ///
 /// Opening scans and validates at most the configured file ceiling, O(file
-/// bytes + Rekey rows) time and O(completed blocks + largest block) memory.
-/// The in-memory identity probe is average O(1) only after that scan.  Public
-/// lookup first re-hashes the bounded file to reject same-length mutation, so
-/// the complete lookup operation is O(file bytes), not universally O(1).
+/// bytes + Rekey rows) time and O(completed blocks + largest block) memory,
+/// and content-hashes the lock and data files.  Public lookup is average O(1)
+/// after that: it compares the root, lock and data generations by metadata
+/// only and re-digests the two fixed records the found receipt names, so it is
+/// independent of file size and of the block's Rekey count (W2-cli11-2,
+/// D-4767).
 ///
 /// **UNVERIFIED as a measured bound.** No bench in this workspace
 /// times this, so the shape above is read from the source rather
@@ -1423,22 +1425,30 @@ impl PopulationFinalizationV2Ledger {
         self.physical_records = physical_records;
     }
 
-    /// Revalidates the bounded file generation, then performs one average-O(1)
-    /// in-memory identity probe.  Absence is `Ok(None)`.
+    /// Revalidates the root, lock and data generations by metadata, performs
+    /// one average-O(1) in-memory identity probe and re-digests the found
+    /// receipt's Data and Completion records.  Absence is `Ok(None)`.
     ///
     /// # Complexity
     ///
-    /// Content-generation validation is O(file bytes); only the subsequent
-    /// hash-table probe is average O(1).
+    /// Average O(1) in file bytes and in the block's Rekey count: a constant
+    /// number of `stat` calls (and one root canonicalization), one hash-table
+    /// probe, and two fixed 2,048-byte record reads whose raw digests must
+    /// equal the receipt's.  No file is hashed (W2-cli11-2, D-4767); before
+    /// D-4767 each lookup content-hashed the whole data file.  Not seen per
+    /// lookup: a same-length rewrite, under unchanged metadata, of a record
+    /// this lookup does not return (another block, or this block's Rekey rows);
+    /// `authenticate_structural_receipt` and the next open refuse it.
     ///
-    /// **UNVERIFIED as a measured bound.** No bench in this workspace
+    /// **UNVERIFIED as a measured time.** No bench in this workspace
     /// times this, so the shape above is read from the source rather
     /// than measured. `CLAUDE.md` §3 rule 6.
     ///
     /// # Errors
     ///
-    /// Refuses lock failure, same-length mutation, append, path replacement or
-    /// any generation change since open.
+    /// Refuses lock failure, a metadata-visible mutation or append, path
+    /// replacement, a changed Data or Completion record of the found block, or
+    /// a read failure.
     pub fn reopen_structural_receipt(
         &self,
         finalization_id: &[u8; 32],
@@ -1447,9 +1457,13 @@ impl PopulationFinalizationV2Ledger {
         self.lock_file
             .lock_shared()
             .map_err(|why| format!("cannot take shared Finalization V2 lookup lock: {why}"))?;
-        let result = self
-            .require_unchanged()
-            .map(|()| self.receipts.get(finalization_id).copied());
+        let result = self.require_platform_unchanged().and_then(|()| {
+            let Some(receipt) = self.receipts.get(finalization_id).copied() else {
+                return Ok(None);
+            };
+            self.redigest_receipt_records(receipt)?;
+            Ok(Some(receipt))
+        });
         let released = self
             .lock_file
             .unlock()
@@ -1858,6 +1872,46 @@ impl PopulationFinalizationV2Ledger {
         )
     }
 
+    /// [`Self::require_unchanged`] without the content hashes: the root's
+    /// directory generation, then the length and platform generation of the
+    /// held and named lock and data files. O(1) in file bytes (D-4767), proven by
+    /// `cli::population_finalization_v2::tests::lookups_hash_no_file_and_redigest_only_the_two_records_they_name`.
+    fn require_platform_unchanged(&self) -> Result<(), PopulationFinalizationV2Refusal> {
+        require_directory_generation(&self.root_generation, &self.root_file, &self.root_path)?;
+        require_file_metadata(self.lock_generation, &self.lock_file, &self.lock_path)?;
+        require_file_metadata(self.data_generation, &self.data_file, &self.data_path)
+    }
+
+    /// Re-reads the Data record at `first_physical_record` and the Completion
+    /// record `rekey_count + 1` after it, and requires their raw digests to be
+    /// the receipt's: exactly the bytes the open's scan reproduced (D-4767).
+    fn redigest_receipt_records(
+        &self,
+        receipt: PopulationFinalizationV2StructuralReceipt,
+    ) -> Result<(), PopulationFinalizationV2Refusal> {
+        let completion_physical = receipt
+            .rekey_count
+            .checked_add(1)
+            .and_then(|offset| receipt.first_physical_record.checked_add(offset))
+            .ok_or_else(|| "Finalization V2 receipt Completion index overflowed".to_owned())?;
+        let data_raw = read_record_shared(&self.data_file, receipt.first_physical_record)?;
+        if raw_record_digest(&data_raw) != receipt.data_record_digest {
+            return Err(format!(
+                "Finalization V2 Data record {} of {} changed after bounded open",
+                receipt.first_physical_record,
+                hex32(receipt.finalization_id)
+            ));
+        }
+        let completion_raw = read_record_shared(&self.data_file, completion_physical)?;
+        if raw_record_digest(&completion_raw) != receipt.completion_record_digest {
+            return Err(format!(
+                "Finalization V2 Completion record {completion_physical} of {} changed after bounded open",
+                hex32(receipt.finalization_id)
+            ));
+        }
+        Ok(())
+    }
+
     fn require_root_and_lock_unchanged(&self) -> Result<(), PopulationFinalizationV2Refusal> {
         require_directory_generation(&self.root_generation, &self.root_file, &self.root_path)?;
         require_file_generation(self.lock_generation, &self.lock_file, &self.lock_path, 0)
@@ -2640,6 +2694,20 @@ fn read_record_at(
     Ok(raw)
 }
 
+/// [`read_record_at`] through a shared descriptor, for the `&self` lookup.
+fn read_record_shared(
+    file: &File,
+    index: u64,
+) -> Result<[u8; POPULATION_FINALIZATION_V2_RECORD_BYTES], PopulationFinalizationV2Refusal> {
+    let mut raw = [0_u8; POPULATION_FINALIZATION_V2_RECORD_BYTES];
+    let mut reader = file;
+    reader
+        .seek(SeekFrom::Start(record_offset(index)?))
+        .and_then(|_| reader.read_exact(&mut raw))
+        .map_err(|why| format!("cannot read Finalization V2 record {index}: {why}"))?;
+    Ok(raw)
+}
+
 fn read_record_range(
     file: &mut File,
     first: u64,
@@ -2910,6 +2978,54 @@ fn file_generation(
     })
 }
 
+/// The metadata half of [`require_file_generation`]: the held file and the
+/// file the path names, not followed through a link, must carry the cached
+/// length and platform generation (device/inode and nanosecond times on Unix).
+/// Reads no content (D-4767).
+fn require_file_metadata(
+    expected: FileGenerationV2,
+    held: &File,
+    path: &Path,
+) -> Result<(), PopulationFinalizationV2Refusal> {
+    let held_metadata = held.metadata().map_err(|why| {
+        format!(
+            "cannot stat held Finalization V2 file {}: {why}",
+            path.display()
+        )
+    })?;
+    let named_metadata = std::fs::symlink_metadata(path).map_err(|why| {
+        format!(
+            "cannot stat named Finalization V2 file {}: {why}",
+            path.display()
+        )
+    })?;
+    let held_platform = platform_generation(&held_metadata, path)?;
+    let named_platform = platform_generation(&named_metadata, path)?;
+    if !held_platform.same_file_object(named_platform) {
+        return Err(format!(
+            "held Finalization V2 file no longer names {}; path replacement refused",
+            path.display()
+        ));
+    }
+    if (
+        held_metadata.len(),
+        named_metadata.len(),
+        held_platform,
+        named_platform,
+    ) != (
+        expected.len,
+        expected.len,
+        expected.platform,
+        expected.platform,
+    ) {
+        return Err(format!(
+            "Finalization V2 file {} changed after bounded open",
+            path.display()
+        ));
+    }
+    Ok(())
+}
+
 fn require_file_generation(
     expected: FileGenerationV2,
     held: &File,
@@ -2926,11 +3042,19 @@ fn require_file_generation(
     Ok(())
 }
 
+#[cfg(test)]
+thread_local! {
+    /// Test-only count of whole-file generation hashes on this thread.
+    static FINALIZATION_FILE_HASHES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+}
+
 fn hash_file_bounded(
     file: &mut File,
     path: &Path,
     max_bytes: u64,
 ) -> Result<[u8; 32], PopulationFinalizationV2Refusal> {
+    #[cfg(test)]
+    FINALIZATION_FILE_HASHES.with(|count| count.set(count.get().saturating_add(1)));
     file.seek(SeekFrom::Start(0)).map_err(|why| {
         format!(
             "cannot seek Finalization V2 file {} for generation hash: {why}",
@@ -3997,6 +4121,123 @@ mod tests {
         );
     }
 
+    #[test]
+    fn lookups_hash_no_file_and_redigest_only_the_two_records_they_name() {
+        // W2-cli11-2: each lookup content-hashed the lock and the data file.
+        let limits = bounds(32);
+        let root = TestPath::directory("lookup-cost");
+        let value = prepared();
+        persist_population_finalization_v2(root.path(), limits, &value)
+            .expect("seed lookup-cost ledger");
+        let hashes = || FINALIZATION_FILE_HASHES.with(std::cell::Cell::get);
+        FINALIZATION_FILE_HASHES.with(|count| count.set(0));
+        let opened = PopulationFinalizationV2Ledger::open_read(root.path(), limits)
+            .expect("open lookup-cost ledger");
+        assert_eq!(
+            hashes(),
+            4,
+            "an open hashes lock and data when it measures and again when its scan closes"
+        );
+        FINALIZATION_FILE_HASHES.with(|count| count.set(0));
+        for _ in 0..10 {
+            let found = opened
+                .reopen_structural_receipt(&value.finalization_id())
+                .expect("lookup reads")
+                .expect("receipt is indexed");
+            assert_eq!(found.finalization_id(), value.finalization_id());
+            assert!(
+                opened
+                    .reopen_structural_receipt(&digest(9_999))
+                    .expect("absent lookup reads")
+                    .is_none()
+            );
+        }
+        assert_eq!(hashes(), 0, "twenty lookups hash no file");
+    }
+
+    #[test]
+    fn a_lookup_redigests_its_data_and_completion_and_not_the_rekey_rows() {
+        // D-4767: metadata generations plus the two records the receipt names.
+        // A rewrite inside one timestamp tick is modelled by re-measuring the
+        // cached data generation's metadata after it. The fixture's one block
+        // is Data at record 0, three Rekey rows at 1..=3, Completion at 4.
+        let limits = bounds(32);
+        let root = TestPath::directory("lookup-redigest");
+        let value = prepared();
+        persist_population_finalization_v2(root.path(), limits, &value)
+            .expect("seed lookup-redigest ledger");
+        let path = root.path().join(LEDGER_FILE);
+        let flip = |record: u64| {
+            let mut file = OpenOptions::new()
+                .read(true)
+                .write(true)
+                .open(&path)
+                .expect("open mutator");
+            let offset = record * POPULATION_FINALIZATION_V2_RECORD_BYTES as u64 + 80;
+            let mut byte = [0_u8; 1];
+            file.seek(SeekFrom::Start(offset))
+                .and_then(|_| file.read_exact(&mut byte))
+                .expect("read the byte");
+            file.seek(SeekFrom::Start(offset))
+                .and_then(|_| file.write_all(&[byte[0] ^ 1]))
+                .and_then(|()| file.sync_data())
+                .expect("write the byte");
+        };
+        let same_tick = |ledger: &mut PopulationFinalizationV2Ledger| {
+            let metadata = std::fs::metadata(&path).expect("stat the ledger");
+            ledger.data_generation = FileGenerationV2 {
+                len: metadata.len(),
+                content_digest: ledger.data_generation.content_digest,
+                platform: platform_generation(&metadata, &path).expect("platform generation"),
+            };
+        };
+        for (record, needed) in [(0, "Data record 0 of"), (4, "Completion record 4 of")] {
+            let mut opened = PopulationFinalizationV2Ledger::open_read(root.path(), limits)
+                .expect("open lookup-redigest ledger");
+            flip(record);
+            same_tick(&mut opened);
+            assert_refuses(
+                opened.reopen_structural_receipt(&value.finalization_id()),
+                needed,
+            );
+            assert!(
+                opened
+                    .reopen_structural_receipt(&digest(9_999))
+                    .expect("an absent id reads no record")
+                    .is_none()
+            );
+            flip(record);
+        }
+        let mut opened = PopulationFinalizationV2Ledger::open_read(root.path(), limits)
+            .expect("open lookup-redigest ledger");
+        flip(2);
+        same_tick(&mut opened);
+        assert!(
+            opened
+                .reopen_structural_receipt(&value.finalization_id())
+                .expect("the limit: a Rekey row is not a record the lookup returns")
+                .is_some()
+        );
+        drop(opened);
+        assert!(
+            PopulationFinalizationV2Ledger::open_read(root.path(), limits).is_err(),
+            "the next open refuses it"
+        );
+        flip(2);
+        let opened = PopulationFinalizationV2Ledger::open_read(root.path(), limits)
+            .expect("open lookup-redigest ledger");
+        OpenOptions::new()
+            .write(true)
+            .open(&path)
+            .expect("open mtime pin")
+            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1))
+            .expect("pin the modification time");
+        assert_refuses(
+            opened.reopen_structural_receipt(&digest(9_999)),
+            "changed after bounded open",
+        );
+    }
+
     #[test]
     fn checked_arithmetic_and_short_blocks_refuse() {
         assert_refuses(checked_sum(&[u64::MAX, 1], "fixture count"), "overflow u64");
diff --git a/crates/cli/tests/ledger_append_lookup_costs.rs b/crates/cli/tests/ledger_append_lookup_costs.rs
index e81040b7..fe694fcc 100644
--- a/crates/cli/tests/ledger_append_lookup_costs.rs
+++ b/crates/cli/tests/ledger_append_lookup_costs.rs
@@ -23,6 +23,8 @@
 //!   four times while §154 said a lookup is average O(1) and a page O(rows).
 //! * W2-cli11-3 (D-4766): the Observation lookups no longer hash the whole
 //!   file; the test LBE-06 names for that keeps its name and pins the new shape.
+//! * W2-cli11-2 (D-4767): the Finalization V2 lookup no longer hashes the
+//!   data file.
 //!
 //! Every file a constant below names is read at compile time, so a rename
 //! fails the build rather than skipping the check. A separate test crate, as
@@ -42,6 +44,7 @@ const CANDIDATE: &str = include_str!("../src/candidate_universe.rs");
 const PRE_ADMISSION: &str = include_str!("../src/pre_admission_data.rs");
 const OBSERVATIONS: &str = include_str!("../src/population_observations_v1.rs");
 const STATISTICS: &str = include_str!("../src/population_statistics_v2.rs");
+const FINALIZATION: &str = include_str!("../src/population_finalization_v2.rs");
 const STEP3: &str = include_str!("../src/step3_orchestrator.rs");
 const LEDGER_V6: &str = include_str!("../src/ledger_v6.rs");
 const STRICT_INPUTS: &str = include_str!("../src/strict_v6_inputs.rs");
@@ -210,7 +213,7 @@ fn observation_lookups_still_hash_the_whole_file_and_say_so() {
     for needed in [
         "Pre-Admission Data V1 `page`** (§153) is now O(P)",
         "Observation V1 and V2 `reopen_audit`** (§157, §161) are average O(1) since D-4766",
-        "Finalization V2 `reopen_structural_receipt`** re-hashes the bounded data file",
+        "Finalization V2 `reopen_structural_receipt`** is average O(1) since D-4767",
     ] {
         assert!(
             chapter.contains(needed),
@@ -219,6 +222,33 @@ fn observation_lookups_still_hash_the_whole_file_and_say_so() {
     }
 }
 
+#[test]
+fn the_finalization_lookup_reads_two_records_and_says_so() {
+    let lookup = method(FINALIZATION, "    pub fn reopen_structural_receipt(");
+    assert!(lookup.contains("self.require_platform_unchanged()"));
+    assert!(lookup.contains("self.redigest_receipt_records(receipt)?"));
+    assert!(
+        !lookup.contains("self.require_unchanged()"),
+        "the lookup content-hashes the ledger again; re-measure the chapter"
+    );
+    let platform = method(FINALIZATION, "    fn require_platform_unchanged(&self)");
+    assert!(!platform.contains("require_file_generation(") && !platform.contains("hash_file"));
+    let metadata = function(FINALIZATION, "fn require_file_metadata(");
+    assert!(!metadata.contains("hash_file") && !metadata.contains("file_generation("));
+    let redigest = method(FINALIZATION, "    fn redigest_receipt_records(");
+    assert_eq!(redigest.matches("read_record_shared(").count(), 2);
+    let start = FINALIZATION
+        .find("    pub fn reopen_structural_receipt(")
+        .expect("the lookup");
+    let doc_start = FINALIZATION
+        .get(..start)
+        .expect("a prefix")
+        .rfind("\n\n")
+        .expect("a gap");
+    let doc = flat(FINALIZATION.get(doc_start..start).expect("the rustdoc"));
+    assert!(doc.contains("Average O(1) in file bytes and in the block's Rekey count"));
+}
+
 #[test]
 fn section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append() {
     let build = function(STATISTICS, "fn build_raw_candidates(");
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index d3dbb993..c565a258 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7059,3 +7059,4 @@ old line regex the same input and watched it pass.
 | L1FE-05 | A cached Statistics V2 read hashes no file (81 reads after an open hash 0 times; the open hashes 4 times); a metadata-visible change refuses every read; inside one timestamp tick a lookup refuses its own changed Data or Completion record, a candidate read or page refuses an unsealed returned row, and the family-wide reads refuse a resealed row that still validates, which a single read returns (the stated limit) and the next open refuses; a trailing-prefix read refuses a changed orphan Data record (D-4765) | `cli::population_statistics_v2::tests::cached_reads_hash_no_file_and_reread_only_what_they_return`, `cli::population_statistics_v2::tests::cached_reads_reverify_what_they_return_and_the_limit_is_what_they_do_not`, `cli::population_statistics_v2::tests::a_trailing_prefix_read_reverifies_the_orphan_data_record`, `cli::population_statistics_v2::tests::explicit_bounds_and_post_open_same_length_mutation_refuse`, `cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append` | ✓ |
 | L1FE-06 | An Observation V1 or V2 lookup reads no whole file (20 lookups: 0 reads, 0 scans, 0 hash passes) and an append door runs one open and one scan, plus one streaming hash pass when it writes; inside one timestamp tick a lookup refuses its own changed Data or Completion and does not see another authority's pair (the stated limit), which that pair's lookup and the next open refuse; a metadata-visible change refuses even an absent id; §157, §161 and both lookups' rustdoc state it (D-4766) | `cli::population_observations_v1::tests::lookups_read_no_whole_file_and_one_append_door_scans_once`, `cli::population_observations_v1::tests::a_v1_lookup_rereads_its_own_pair_and_not_another`, `cli::population_observations_v1::tests::a_v2_lookup_reverify_and_refresh_reread_only_their_own_pair`, `cli::ledger_append_lookup_costs::observation_lookups_still_hash_the_whole_file_and_say_so`, `cli::ledger_append_lookup_costs::a_pre_admission_page_checks_metadata_and_its_lookups_are_priced_by_file_bytes` | ✓ |
 | L1FE-07 | An Observation door's re-read re-reads a reused pair and refuses a written pair that does not end the file or an audit the handle did not index; the post-write refresh refuses an edit below the verified length and adopts the verified bytes, whose streaming digest equals the whole-file digest (D-4766) | `cli::population_observations_v1::tests::a_v1_reverify_and_refresh_adopt_only_what_the_handle_verified`, `cli::population_observations_v1::tests::a_v2_lookup_reverify_and_refresh_reread_only_their_own_pair` | ✓ |
+| L1FE-08 | A Finalization V2 lookup hashes no file (20 lookups after an open hash 0 times; the open hashes 4 times); inside one timestamp tick it refuses its own changed Data or Completion record and does not see a changed Rekey row (the stated limit), which the next open refuses; an absent id is `None`; a metadata-visible change refuses; the source and rustdoc state the two-record shape (D-4767) | `cli::population_finalization_v2::tests::lookups_hash_no_file_and_redigest_only_the_two_records_they_name`, `cli::population_finalization_v2::tests::a_lookup_redigests_its_data_and_completion_and_not_the_rekey_rows`, `cli::ledger_append_lookup_costs::the_finalization_lookup_reads_two_records_and_says_so` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 372827d9..8f6fe750 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65186,3 +65186,43 @@ left (5, 2, 0), right (1, 1, 1)),
 `cli::population_observations_v1::tests::a_v2_lookup_reverify_and_refresh_reread_only_their_own_pair`,
 `cli::ledger_append_lookup_costs::observation_lookups_still_hash_the_whole_file_and_say_so`,
 `cli::ledger_append_lookup_costs::a_pre_admission_page_checks_metadata_and_its_lookups_are_priced_by_file_bytes`.
+
+### D-4767 — A Finalization V2 lookup re-digests the two records it names — 2026-10-09
+
+**What was wrong.** W2-cli11-2: `reopen_structural_receipt` ran
+`require_unchanged` on every lookup. That checks the root directory generation
+and content-hashes the lock file and the whole data file, O(file bytes) per
+lookup, before an average-O(1) probe. The counting test measured 40 hashes for
+20 lookups. D-1681 stated the cost and kept the hash.
+
+**Decided.** The plan G4 §B named, checked against the code. The ledger has no
+caller outside its module, and its receipt carries `first_physical_record`,
+`rekey_count` and the raw-record digests of its Data and Completion.
+
+- `require_platform_unchanged` checks the root's directory generation. It then
+  compares the length and platform generation (device/inode and nanosecond
+  modification/change times) of the held and the named lock and data files
+  against the cached ones, without reading content.
+- A found receipt's Data record, at `first_physical_record`, and its
+  Completion, at `first_physical_record + rekey_count + 1`, are re-read with
+  two fixed-offset reads. Their raw digests must equal the receipt's: the same
+  `raw_record_digest` the scan used.
+- An absent id is `Ok(None)` after the metadata check.
+- The open keeps its content hashes, and the dormant append is unchanged.
+
+No byte, format or identity changes.
+
+**What is no longer seen per lookup.** An equal-metadata, same-length rewrite
+of another block, or of this block's Rekey rows. `authenticate_structural_receipt`
+and the next open refuse it, the residual D-1681 accepted for the
+Pre-Admission page.
+
+**Rejected.** Re-reading the whole block per lookup: that is O(Rekey rows),
+and the receipt does not return those rows.
+
+Tests: `cli::population_finalization_v2::tests::lookups_hash_no_file_and_redigest_only_the_two_records_they_name`
+(failed first on the unfixed lookup: "twenty lookups hash no file", left 40,
+right 0),
+`cli::population_finalization_v2::tests::a_lookup_redigests_its_data_and_completion_and_not_the_rekey_rows`,
+`cli::population_finalization_v2::tests::stale_same_length_mutation_and_path_replacement_invalidate_lookup`,
+`cli::ledger_append_lookup_costs::the_finalization_lookup_reads_two_records_and_says_so`.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 1be88fee..56cd413e 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15433,14 +15433,18 @@ or average O(1).
   open refuse it. Each append door is one open and one scan where it was two,
   plus one streaming hash pass after the write that must reproduce the bytes
   the open validated.
-- **Finalization V2 `reopen_structural_receipt`** re-hashes the bounded data
-  file through `require_unchanged`, O(file bytes) per lookup, before an
-  average-O(1) probe; its rustdoc already said so and this section is its
-  first statement here. Its append is dormant outside tests
-  (`expect(dead_code)`) and calls `require_unchanged`, a whole-file hash, at
-  several steps, so one append is a constant multiple of O(file bytes). The
-  finding counted at least eight passes; that count is not re-measured here
-  and is UNVERIFIED.
+- **Finalization V2 `reopen_structural_receipt`** is average O(1) since
+  D-4767: the root, lock and data generations are compared by metadata only,
+  and the found receipt's Data and Completion records, two fixed 2,048-byte
+  reads, must re-digest to the receipt's digests. It is independent of the file
+  size and of the block's Rekey count. Until D-4767 it re-hashed the bounded
+  data file through `require_unchanged`, O(file bytes) per lookup. Not seen per
+  lookup: an equal-metadata same-length rewrite of another block or of this
+  block's Rekey rows; `authenticate_structural_receipt` and the next open
+  refuse it. Its append is dormant outside tests (`expect(dead_code)`) and
+  still calls `require_unchanged`, a whole-file hash, at several steps, so one
+  append is a constant multiple of O(file bytes). The finding counted at least
+  eight passes; that count is not re-measured here and is UNVERIFIED.
 
 ### Ledger V6 route: one load per family per rung, and replay recomputes the route — D-1683
 
-- 
2.43.0


From fa3100b948e34287886439a5cf3603c85ead97a7 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:36:30 +0000
Subject: [PATCH 5/5] cli: size the Statistics V2 index by audits, not records
 (W2-cli12-2, D-4768)

Before: an open reserved min(max_audits, stored records) index slots;
one audit spans 2 + C x (1 + P + S) records, so 2 audits in 28 records
held 14 slots.

After: the map starts empty and scan reserves one slot before each
insert (a named refusal on failure); geometric growth makes A inserts
amortised O(1) each, O(A) in total, capacity below 2A plus a constant.
Supersedes D-1682's rejection of growth on demand and D-2026's
stored-record floor; the D-2026 test keeps its name and pins capacity
below the stored record count.

Fail-before: an_open_sizes_its_index_by_audits_not_records "the index
holds 14 slots for 2 audits; it is sized by audits, not by the 28
records".

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/population_statistics_v2.rs    | 64 +++++++++++++------
 .../cli/tests/ledger_append_lookup_costs.rs   | 14 +++-
 docs/04-invariants.md                         |  1 +
 docs/05-decisions.md                          | 29 +++++++++
 docs/06-limits.md                             | 13 ++--
 5 files changed, 96 insertions(+), 25 deletions(-)

diff --git a/crates/cli/src/population_statistics_v2.rs b/crates/cli/src/population_statistics_v2.rs
index e0096dd9..7355822b 100644
--- a/crates/cli/src/population_statistics_v2.rs
+++ b/crates/cli/src/population_statistics_v2.rs
@@ -2234,25 +2234,18 @@ impl PopulationStatisticsV2Ledger {
             }
             let lock_generation = file_generation(&held_lock, &lock_path, LOCK_FILE_MAX_BYTES)?;
             let data_generation = file_generation(&data_file, &data_path, bounds.file_bytes)?;
-            // Every stored audit occupies at least one record, so the records
-            // the file holds bound the audits it can hold. Reserving the
-            // configured ceiling instead allocated O(bounds.audits) slots on
-            // every open, before anything was counted (W2-cli12-2, D-1682).
-            let stored_records = data_generation
-                .len
-                .saturating_sub(POPULATION_STATISTICS_V2_HEADER_BYTES)
-                / POPULATION_STATISTICS_V2_RECORD_STRIDE;
-            let mut audits = HashMap::new();
-            audits
-                .try_reserve(usize_of(bounds.audits.min(stored_records), "audit bound")?)
-                .map_err(|why| format!("cannot reserve population-statistics index: {why}"))?;
+            // The index starts empty and grows one admitted audit at a time in
+            // `scan`, so it holds O(A) slots for A audits. Before D-4768 the
+            // open reserved one slot per stored RECORD, and one audit spans
+            // 2 + C x (1 + P + S) records; before D-1682 it reserved the
+            // configured ceiling (W2-cli12-2).
             let mut ledger = Self {
                 lock_path: lock_path.clone(),
                 data_path,
                 lock_file: held_lock,
                 data_file,
                 bounds,
-                audits,
+                audits: HashMap::new(),
                 completed_audits: 0,
                 orphan: None,
                 lock_generation,
@@ -2333,6 +2326,14 @@ impl PopulationStatisticsV2Ledger {
                 continue;
             }
             let audit = validate_complete_block(&mut self.data_file, cursor, &manifest)?;
+            // Room for this one audit, so a failed allocation is a named
+            // refusal, not an abort. The map grows geometrically: A inserts
+            // cost amortised O(1) each, O(A) in total, and its capacity stays
+            // below twice the audits it holds plus a constant (W2-cli12-2,
+            // D-4768).
+            self.audits
+                .try_reserve(1)
+                .map_err(|why| format!("cannot reserve population-statistics index: {why}"))?;
             if self.audits.insert(manifest.audit_id, audit).is_some() {
                 return Err(format!(
                     "population-statistics audit {} appears more than once",
@@ -6992,17 +6993,18 @@ mod tests {
         assert_eq!(reader.completed_audits(), 1);
         assert!(reader.audits.capacity() >= 1);
         assert!(reader.audits.capacity() < 1_024);
-        // THE STORED RECORDS ARE THE RESERVATION, measured off the file: a
-        // quotient, not a remainder (G18-cli-b-17, D-2026).
+        // The stored records were the reservation until D-4768 (D-2026 pinned
+        // it). The index now grows with the audits the scan admits, so one
+        // audit's 14 records no longer size it.
         let stored = (std::fs::metadata(root.path().join(DATA_FILE))
             .expect("ledger measures")
             .len()
             - POPULATION_STATISTICS_V2_HEADER_BYTES)
             / POPULATION_STATISTICS_V2_RECORD_STRIDE;
-        assert!(stored >= 4, "one audit spans {stored} records");
+        assert_eq!(stored, 14, "one audit spans 2 + C x (1 + P + S) records");
         assert!(
-            reader.audits.capacity() >= usize::try_from(stored).expect("small record count"),
-            "the open reserves for the stored records (D-1682)"
+            reader.audits.capacity() < usize::try_from(stored).expect("small record count"),
+            "the open sizes its index by audits, not by stored records (D-4768)"
         );
     }
 
@@ -7112,6 +7114,32 @@ mod tests {
         assert_eq!(hashes(), 0, "81 cached reads hash no file");
     }
 
+    #[test]
+    fn an_open_sizes_its_index_by_audits_not_records() {
+        // W2-cli12-2: the index was reserved for every stored record, and one
+        // audit spans 2 + C x (1 + P + S) records (14 here).
+        let root = TempRoot::new("audit-index");
+        append_population_statistics_v2(root.path(), bounds(), &fixture(4))
+            .expect("the first audit writes");
+        append_population_statistics_v2(root.path(), bounds(), &fixture(5))
+            .expect("the second audit writes");
+        let reader = PopulationStatisticsV2Ledger::open_read(root.path(), bounds())
+            .expect("two audits reopen");
+        assert_eq!(reader.completed_audits(), 2);
+        let stored = (std::fs::metadata(root.path().join(DATA_FILE))
+            .expect("ledger measures")
+            .len()
+            - POPULATION_STATISTICS_V2_HEADER_BYTES)
+            / POPULATION_STATISTICS_V2_RECORD_STRIDE;
+        assert_eq!(stored, 28, "two audits of 14 records each");
+        assert!(reader.audits.capacity() >= 2);
+        assert!(
+            reader.audits.capacity() < 8,
+            "the index holds {} slots for 2 audits; it is sized by audits, not by the {stored} records",
+            reader.audits.capacity()
+        );
+    }
+
     #[test]
     fn one_append_runs_one_full_scan_and_rereads_only_its_block() {
         // W2-cli12-1: the door opened the ledger twice (the writer's open and
diff --git a/crates/cli/tests/ledger_append_lookup_costs.rs b/crates/cli/tests/ledger_append_lookup_costs.rs
index fe694fcc..4833c58d 100644
--- a/crates/cli/tests/ledger_append_lookup_costs.rs
+++ b/crates/cli/tests/ledger_append_lookup_costs.rs
@@ -25,6 +25,8 @@
 //!   file; the test LBE-06 names for that keeps its name and pins the new shape.
 //! * W2-cli11-2 (D-4767): the Finalization V2 lookup no longer hashes the
 //!   data file.
+//! * W2-cli12-2 (D-4768): the Statistics V2 index was reserved for every
+//!   stored record; it now grows with the audits the scan admits.
 //!
 //! Every file a constant below names is read at compile time, so a rename
 //! fails the build rather than skipping the check. A separate test crate, as
@@ -260,8 +262,13 @@ fn section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append()
     let column = function(STATISTICS, "fn candidate_column<");
     assert!(column.contains(".skip(sequence).step_by(width)"));
     let open = method(STATISTICS, "    fn open_inner(");
-    assert!(open.contains("bounds.audits.min(stored_records)"));
-    assert!(!open.contains("try_reserve(usize_of(bounds.audits,"));
+    assert!(
+        !open.contains("stored_records") && !open.contains("try_reserve("),
+        "the open pre-reserves its index again; re-measure §154"
+    );
+    assert!(open.contains("audits: HashMap::new(),"));
+    let scan = method(STATISTICS, "    fn scan(&mut self)");
+    assert_eq!(scan.matches(".try_reserve(1)").count(), 1);
     // One open per append and per step since D-4764: the door re-reads its
     // block through the writer's handle and hands that handle to the step.
     let door = function(STATISTICS, "pub fn append_population_statistics_v2(");
@@ -282,7 +289,8 @@ fn section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append()
     let text = flat(section(154));
     for needed in [
         "so the per-candidate summaries cost O(C·(P+S)) in total",
-        "never the configured `max_audits` ceiling",
+        "never by the stored records and never the configured `max_audits` ceiling",
+        "so A audits cost amortised O(1) each and O(A) in total",
         "One append through `append_population_statistics_v2` runs one full open",
         "A appends to one root cost O(A²) block validations in total, with constant 1",
         "a step scans its Statistics root once where it scanned three times",
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index c565a258..8abe326f 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7060,3 +7060,4 @@ old line regex the same input and watched it pass.
 | L1FE-06 | An Observation V1 or V2 lookup reads no whole file (20 lookups: 0 reads, 0 scans, 0 hash passes) and an append door runs one open and one scan, plus one streaming hash pass when it writes; inside one timestamp tick a lookup refuses its own changed Data or Completion and does not see another authority's pair (the stated limit), which that pair's lookup and the next open refuse; a metadata-visible change refuses even an absent id; §157, §161 and both lookups' rustdoc state it (D-4766) | `cli::population_observations_v1::tests::lookups_read_no_whole_file_and_one_append_door_scans_once`, `cli::population_observations_v1::tests::a_v1_lookup_rereads_its_own_pair_and_not_another`, `cli::population_observations_v1::tests::a_v2_lookup_reverify_and_refresh_reread_only_their_own_pair`, `cli::ledger_append_lookup_costs::observation_lookups_still_hash_the_whole_file_and_say_so`, `cli::ledger_append_lookup_costs::a_pre_admission_page_checks_metadata_and_its_lookups_are_priced_by_file_bytes` | ✓ |
 | L1FE-07 | An Observation door's re-read re-reads a reused pair and refuses a written pair that does not end the file or an audit the handle did not index; the post-write refresh refuses an edit below the verified length and adopts the verified bytes, whose streaming digest equals the whole-file digest (D-4766) | `cli::population_observations_v1::tests::a_v1_reverify_and_refresh_adopt_only_what_the_handle_verified`, `cli::population_observations_v1::tests::a_v2_lookup_reverify_and_refresh_reread_only_their_own_pair` | ✓ |
 | L1FE-08 | A Finalization V2 lookup hashes no file (20 lookups after an open hash 0 times; the open hashes 4 times); inside one timestamp tick it refuses its own changed Data or Completion record and does not see a changed Rekey row (the stated limit), which the next open refuses; an absent id is `None`; a metadata-visible change refuses; the source and rustdoc state the two-record shape (D-4767) | `cli::population_finalization_v2::tests::lookups_hash_no_file_and_redigest_only_the_two_records_they_name`, `cli::population_finalization_v2::tests::a_lookup_redigests_its_data_and_completion_and_not_the_rekey_rows`, `cli::ledger_append_lookup_costs::the_finalization_lookup_reads_two_records_and_says_so` | ✓ |
+| L1FE-09 | A Statistics V2 open sizes its index by the audits it admits, not the stored records: 2 audits in 28 records hold fewer than 8 slots, one audit in 14 records fewer than 14, under a `u64::MAX` ceiling; the open reserves nothing and the scan reserves one slot per insert, as §154 states (D-4768) | `cli::population_statistics_v2::tests::an_open_sizes_its_index_by_audits_not_records`, `cli::population_statistics_v2::tests::an_open_reserves_for_stored_records_not_the_audit_ceiling`, `cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 8f6fe750..25913877 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65226,3 +65226,32 @@ right 0),
 `cli::population_finalization_v2::tests::a_lookup_redigests_its_data_and_completion_and_not_the_rekey_rows`,
 `cli::population_finalization_v2::tests::stale_same_length_mutation_and_path_replacement_invalidate_lookup`,
 `cli::ledger_append_lookup_costs::the_finalization_lookup_reads_two_records_and_says_so`.
+
+### D-4768 — The Statistics V2 index grows with the audits it admits — 2026-10-09
+
+**What was wrong.** W2-cli12-2: after D-1682 an open reserved one index slot per
+stored record, `min(max_audits, records)`, and D-2026 pinned that. One audit
+spans 2 + C·(1 + P + S) records, so the reservation exceeded the audits by that
+factor: 14 slots for 2 audits in the counting test.
+
+**Decided.** The open starts with an empty map. `scan` makes room for exactly
+one audit before each insert with `try_reserve(1)`, so a failed allocation is
+a named refusal, never an abort. The map grows geometrically. A audits cost
+amortised O(1) each and O(A) in total, and the capacity stays below twice A
+plus a constant. Nothing on disk changes.
+
+**Supersedes** D-1682's rejection of growing on demand ("repeated rehashing
+during the scan"). Geometric growth rehashes O(A) entries in total, the same
+class as the scan itself. It also supersedes D-2026's "the open must reserve at
+least the stored record count". The test D-2026 tightened,
+`an_open_reserves_for_stored_records_not_the_audit_ceiling`, keeps its name for
+LBE-08 and now pins a capacity below the stored record count.
+
+**Rejected.** Reserving by a count of audits read ahead of the scan: that would
+read every block's Data record twice to save an amortised constant.
+
+Tests: `cli::population_statistics_v2::tests::an_open_sizes_its_index_by_audits_not_records`
+(failed first on the record-sized reservation: "the index holds 14 slots for 2
+audits; it is sized by audits, not by the 28 records"),
+`cli::population_statistics_v2::tests::an_open_reserves_for_stored_records_not_the_audit_ceiling`,
+`cli::ledger_append_lookup_costs::section_154_states_index_reads_the_bounded_reserve_and_the_two_open_append`.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 56cd413e..d97efefa 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -8160,10 +8160,15 @@ not sample rows, cap Apriori depth or turn an admitted input into a smaller one.
 Preparing one block reads each candidate's P periods and S splits by index
 arithmetic (`outer x C + candidate`) out of the period-major and split-major
 vectors, so the per-candidate summaries cost O(C·(P+S)) in total. Before D-1682
-each candidate filtered both whole vectors, O(C²·(P+S)). An open reserves its
-audit index for at most the records the file holds, never the configured
-`max_audits` ceiling: before D-1682 every open, empty or not, reserved
-`max_audits` slots (production passes 1<<24) before anything was counted.
+each candidate filtered both whole vectors, O(C²·(P+S)). An open sizes its
+audit index by the audits it admits, never by the stored records and never the
+configured `max_audits` ceiling: the map starts empty and makes room for one
+audit before each insert, a named refusal if that fails, so A audits cost
+amortised O(1) each and O(A) in total, with a capacity below twice A plus a
+constant (W2-cli12-2, D-4768). Before D-4768 the open reserved one slot per
+stored record, and one audit spans 2 + C·(1 + P + S) records; before D-1682
+every open, empty or not, reserved `max_audits` slots (production passes
+1<<24) before anything was counted.
 
 One append through `append_population_statistics_v2` runs one full open, the
 writer's. After the receipt-last write it re-reads only the committed block
-- 
2.43.0


From 55813a9102d7eea3006b756b491a010e804791c6 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:25:04 +0000
Subject: [PATCH 1/4] Pre-Admission V1/V2 append doors open once and re-read
 only their pair (G4-4, D-4781)
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Before: each Pre-Admission production append door, V1 and V2, ran a full
open of the ledger, appended, dropped that handle and then ran a second
full `open_read` to confirm what it had just committed. A full open hashes
every record to build the content generations, so one append paid two
O(file bytes) passes.

After: the door opens once, appends, and calls `reverify_committed` on the
same handle. Under a shared lock it checks the metadata generation, that
the physical record count is exactly twice the completed pairs, that a
written pair is the last stored pair, and that the raw Data and Completion
bytes on disk equal the audit's own encodings. A reused pair is re-read the
same way at its index. One append is still O(file bytes), because the one
open still hashes content generations; docs/06 §153 states that bound.

Tests: v1_reverify_refuses_every_disagreement_with_the_disk and
v2_reverify_refuses_every_disagreement_with_the_disk (count, position,
Data bytes, completion bytes, absent audit), the existing
v1_and_v2_append_doors_scan_once_and_reread_only_their_pair (failed before
with 2 scans against 1), and the integration test
a_pre_admission_append_door_opens_once_and_section_153_says_so.
Invariants L1FF-06, L1FF-07, L1FF-14. No stored byte, format or identity
changes.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/pre_admission_data.rs          | 575 +++++++++++++++++-
 .../cli/tests/ledger_append_lookup_costs.rs   |  32 +
 docs/04-invariants.md                         |   8 +
 docs/05-decisions.md                          |  38 ++
 docs/06-limits.md                             |  11 +
 5 files changed, 639 insertions(+), 25 deletions(-)

diff --git a/crates/cli/src/pre_admission_data.rs b/crates/cli/src/pre_admission_data.rs
index 664e7639..c93876d7 100644
--- a/crates/cli/src/pre_admission_data.rs
+++ b/crates/cli/src/pre_admission_data.rs
@@ -31,8 +31,11 @@
 //! the complete bounded file bytes.  A page checks file generations by
 //! metadata only (length, device/inode and nanosecond modification/change
 //! times) and re-reads and re-seals only its returned records, so it is
-//! proportional to the returned records (D-1681).  Calculating one
-//! validated fixed-record offset is O(1) in record count; no whole-ledger,
+//! proportional to the returned records (D-1681).  A production append door
+//! opens the ledger once and then re-reads only its committed pair through
+//! the same handle, by metadata generation, record count and the pair's exact
+//! bytes (D-4781); before D-4781 it ran a second full `open_read`.  Calculating
+//! one validated fixed-record offset is O(1) in record count; no whole-ledger,
 //! source measurement, allocation, hash, lock, sync or filesystem latency is
 //! described as O(1).
 //!
@@ -1106,6 +1109,8 @@ impl PreAdmissionDataLedgerV1 {
     }
 
     fn scan(&mut self) -> Result<(), PreAdmissionDataRefusal> {
+        #[cfg(test)]
+        PRE_ADMISSION_SCANS.with(|count| count.set(count.get().saturating_add(1)));
         verify_header(&mut self.data_file, &self.data_path)?;
         let file_len = self
             .data_file
@@ -1436,6 +1441,100 @@ impl PreAdmissionDataLedgerV1 {
         Ok(())
     }
 
+    /// Re-reads the one pair a production append just committed, through
+    /// this handle and under the shared lock, in place of a second full open
+    /// (G4-4, D-4781; the D-1680 pattern).
+    ///
+    /// The open that preceded the append validated every older pair, so only
+    /// the committed pair is read again. Both file generations are rechecked
+    /// by metadata, the physical record count must be exactly the completed
+    /// pairs this handle indexed (no orphan), a written pair must be the last
+    /// one, and the Data and Completion records on disk must be byte for byte
+    /// the two records the indexed value encodes. O(1) in ledger size; see
+    /// `cli::pre_admission_data::tests::v1_and_v2_append_doors_scan_once_and_reread_only_their_pair`.
+    ///
+    /// # Errors
+    ///
+    /// Refuses a stale or replaced file, a record count this handle did not
+    /// commit, an absent authority, a written pair that is not the last, a
+    /// pair that differs on disk, or a lock failure.
+    fn reverify_committed(
+        &mut self,
+        committed: &PreAdmissionProductionCommitV1,
+    ) -> Result<PreAdmissionDataReopenAuditV1, PreAdmissionDataRefusal> {
+        self.lock_file
+            .lock_shared()
+            .map_err(|why| format!("cannot take shared pre-admission reverify lock: {why}"))?;
+        let result = self.reverify_committed_locked(committed);
+        let released = self
+            .lock_file
+            .unlock()
+            .map_err(|why| format!("cannot release pre-admission reverify lock: {why}"));
+        match (result, released) {
+            (Ok(audit), Ok(())) => Ok(audit),
+            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
+        }
+    }
+
+    fn reverify_committed_locked(
+        &mut self,
+        committed: &PreAdmissionProductionCommitV1,
+    ) -> Result<PreAdmissionDataReopenAuditV1, PreAdmissionDataRefusal> {
+        require_metadata_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
+        require_metadata_generation(self.data_generation, &self.data_file, &self.data_path)?;
+        let file_len = self
+            .data_file
+            .metadata()
+            .map_err(|why| format!("cannot stat pre-admission reverify file: {why}"))?
+            .len();
+        let physical = record_count(file_len)?;
+        let paired = self
+            .completed_rows
+            .checked_mul(2)
+            .ok_or_else(|| "pre-admission reverify record count overflowed u64".to_owned())?;
+        if physical != paired {
+            return Err(format!(
+                "pre-admission ledger holds {physical} records on disk, not the {paired} this append committed"
+            ));
+        }
+        let authority_id = committed.audit().value.authority_id;
+        let audit = self.audits.get(&authority_id).copied().ok_or_else(|| {
+            format!(
+                "pre-admission authority {} disappeared after receipt-last append",
+                hex32(authority_id)
+            )
+        })?;
+        let completion_index = audit
+            .data_record_index
+            .checked_add(1)
+            .ok_or_else(|| "pre-admission reverify completion index overflowed u64".to_owned())?;
+        if matches!(committed, PreAdmissionProductionCommitV1::Written(_))
+            && completion_index.checked_add(1) != Some(physical)
+        {
+            return Err(format!(
+                "pre-admission authority {} was written but is not the last stored pair",
+                hex32(authority_id)
+            ));
+        }
+        if read_raw_record(&mut self.data_file, audit.data_record_index)?
+            != audit.value.record(RecordKindV1::Data)?
+        {
+            return Err(format!(
+                "pre-admission authority {} Data record differs on disk from the committed value",
+                hex32(authority_id)
+            ));
+        }
+        if read_raw_record(&mut self.data_file, completion_index)?
+            != audit.value.record(RecordKindV1::Completion)?
+        {
+            return Err(format!(
+                "pre-admission authority {} completion differs on disk from the committed value",
+                hex32(authority_id)
+            ));
+        }
+        Ok(audit)
+    }
+
     fn require_unchanged(&self) -> Result<(), PreAdmissionDataRefusal> {
         require_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
         require_generation(self.data_generation, &self.data_file, &self.data_path)
@@ -1751,21 +1850,22 @@ impl ProducedPreAdmissionDataV1 {
         root: impl AsRef<Path>,
         bounds: PreAdmissionDataBoundsV1,
     ) -> Result<PreAdmissionProductionCommitV1, PreAdmissionDataRefusal> {
-        let root = root.as_ref();
+        // ONE OPEN, THEN THE PAIR (G4-4, D-4781): the committed pair is
+        // re-read through the writer's own handle instead of a second full
+        // read-only open, so one append door scans the ledger once.
         let mut ledger = PreAdmissionDataLedgerV1::open(root, bounds)?;
         let committed = ledger.append_complete(&self.value)?;
         let expected = committed.audit();
+        let reopened = ledger.reverify_committed(&committed)?;
         drop(ledger);
-
-        let reopened = PreAdmissionDataLedgerV1::open_read(root, bounds)?
-            .reopen_audit(&self.value.authority_id())?
-            .ok_or_else(|| {
-                format!(
-                    "pre-admission authority {} disappeared after receipt-last append",
-                    hex32(self.value.authority_id())
-                )
-            })?;
-        if reopened != expected || !same_semantics(&reopened.value(), &self.value) {
+        // Two checks, not one `||` (G18-cli-a-05, D-2004).
+        if reopened != expected {
+            return Err(format!(
+                "pre-admission authority {} did not reopen with the exact committed audit",
+                hex32(self.value.authority_id())
+            ));
+        }
+        if !same_semantics(&reopened.value(), &self.value) {
             return Err(format!(
                 "pre-admission authority {} did not reopen with the exact derived semantics",
                 hex32(self.value.authority_id())
@@ -2395,6 +2495,8 @@ impl PreAdmissionDataLedgerV2 {
     }
 
     fn scan(&mut self) -> Result<(), PreAdmissionDataRefusal> {
+        #[cfg(test)]
+        PRE_ADMISSION_SCANS.with(|count| count.set(count.get().saturating_add(1)));
         verify_header_v2(&mut self.data_file, &self.data_path)?;
         let file_len = self
             .data_file
@@ -2652,6 +2754,90 @@ impl PreAdmissionDataLedgerV2 {
         Ok(())
     }
 
+    /// The V2 twin of [`PreAdmissionDataLedgerV1::reverify_committed`]
+    /// (G4-4, D-4781): the committed pair only, through this handle, under
+    /// the shared lock. O(1) in ledger size; see
+    /// `cli::pre_admission_data::tests::v1_and_v2_append_doors_scan_once_and_reread_only_their_pair`.
+    ///
+    /// # Errors
+    ///
+    /// The same refusals as the V1 door's.
+    fn reverify_committed(
+        &mut self,
+        committed: &PreAdmissionProductionCommitV2,
+    ) -> Result<PreAdmissionDataReopenAuditV2, PreAdmissionDataRefusal> {
+        self.lock_file
+            .lock_shared()
+            .map_err(|why| format!("cannot take shared pre-admission V2 reverify lock: {why}"))?;
+        let result = self.reverify_committed_locked(committed);
+        let released = self
+            .lock_file
+            .unlock()
+            .map_err(|why| format!("cannot release pre-admission V2 reverify lock: {why}"));
+        match (result, released) {
+            (Ok(audit), Ok(())) => Ok(audit),
+            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
+        }
+    }
+
+    fn reverify_committed_locked(
+        &mut self,
+        committed: &PreAdmissionProductionCommitV2,
+    ) -> Result<PreAdmissionDataReopenAuditV2, PreAdmissionDataRefusal> {
+        require_metadata_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
+        require_metadata_generation(self.data_generation, &self.data_file, &self.data_path)?;
+        let file_len = self
+            .data_file
+            .metadata()
+            .map_err(|why| format!("cannot stat pre-admission V2 reverify file: {why}"))?
+            .len();
+        let physical = record_count_v2(file_len)?;
+        let paired = self
+            .completed_rows
+            .checked_mul(2)
+            .ok_or_else(|| "pre-admission V2 reverify record count overflowed u64".to_owned())?;
+        if physical != paired {
+            return Err(format!(
+                "pre-admission V2 ledger holds {physical} records on disk, not the {paired} this append committed"
+            ));
+        }
+        let authority_id = committed.audit().value.authority_id();
+        let audit = self.audits.get(&authority_id).copied().ok_or_else(|| {
+            format!(
+                "pre-admission V2 authority {} disappeared after receipt-last append",
+                hex32(authority_id)
+            )
+        })?;
+        let completion_index = audit.data_record_index.checked_add(1).ok_or_else(|| {
+            "pre-admission V2 reverify completion index overflowed u64".to_owned()
+        })?;
+        if matches!(committed, PreAdmissionProductionCommitV2::Written(_))
+            && completion_index.checked_add(1) != Some(physical)
+        {
+            return Err(format!(
+                "pre-admission V2 authority {} was written but is not the last stored pair",
+                hex32(authority_id)
+            ));
+        }
+        if read_raw_record_v2(&mut self.data_file, audit.data_record_index)?
+            != audit.value.record(RecordKindV2::Data)?
+        {
+            return Err(format!(
+                "pre-admission V2 authority {} Data record differs on disk from the committed value",
+                hex32(authority_id)
+            ));
+        }
+        if read_raw_record_v2(&mut self.data_file, completion_index)?
+            != audit.value.record(RecordKindV2::Completion)?
+        {
+            return Err(format!(
+                "pre-admission V2 authority {} completion differs on disk from the committed value",
+                hex32(authority_id)
+            ));
+        }
+        Ok(audit)
+    }
+
     fn require_unchanged(&self) -> Result<(), PreAdmissionDataRefusal> {
         require_generation_v2(self.lock_generation, &self.lock_file, &self.lock_path)?;
         require_generation_v2(self.data_generation, &self.data_file, &self.data_path)
@@ -2730,20 +2916,20 @@ impl ProducedPreAdmissionDataV2 {
         root: impl AsRef<Path>,
         bounds: PreAdmissionDataBoundsV2,
     ) -> Result<PreAdmissionProductionCommitV2, PreAdmissionDataRefusal> {
-        let root = root.as_ref();
+        // ONE OPEN, THEN THE PAIR (G4-4, D-4781), as the V1 door.
         let mut ledger = PreAdmissionDataLedgerV2::open(root, bounds)?;
         let committed = ledger.append_complete(&self.value)?;
         let expected = committed.audit();
+        let reopened = ledger.reverify_committed(&committed)?;
         drop(ledger);
-        let reopened = PreAdmissionDataLedgerV2::open_read(root, bounds)?
-            .reopen_audit(&self.value.authority_id())?
-            .ok_or_else(|| {
-                format!(
-                    "pre-admission V2 authority {} disappeared after receipt-last append",
-                    hex32(self.value.authority_id())
-                )
-            })?;
-        if reopened != expected || !same_semantics_v2(&reopened.value(), &self.value) {
+        // Two checks, not one `||` (G18-cli-a-05, D-2004).
+        if reopened != expected {
+            return Err(format!(
+                "pre-admission V2 authority {} did not reopen with the exact committed audit",
+                hex32(self.value.authority_id())
+            ));
+        }
+        if !same_semantics_v2(&reopened.value(), &self.value) {
             return Err(format!(
                 "pre-admission V2 authority {} did not reopen with exact derived semantics",
                 hex32(self.value.authority_id())
@@ -3600,11 +3786,19 @@ fn read_record(
     file: &mut File,
     index: u64,
 ) -> Result<(RecordKindV1, PreAdmissionDataV1), PreAdmissionDataRefusal> {
+    PreAdmissionDataV1::decode(&read_raw_record(file, index)?)
+}
+
+/// The literal bytes of one V1 record, undecoded.
+fn read_raw_record(
+    file: &mut File,
+    index: u64,
+) -> Result<[u8; RECORD_BYTES], PreAdmissionDataRefusal> {
     let mut raw = [0_u8; RECORD_BYTES];
     file.seek(SeekFrom::Start(record_offset(index)?))
         .and_then(|_| file.read_exact(&mut raw))
         .map_err(|why| format!("cannot read pre-admission record {index}: {why}"))?;
-    PreAdmissionDataV1::decode(&raw)
+    Ok(raw)
 }
 
 /// Appends one fixed record and makes it durable. A short write or a failed
@@ -3701,6 +3895,9 @@ fn generation_of(metadata: &std::fs::Metadata, content_digest: [u8; 32]) -> File
 thread_local! {
     /// Test-only count of whole-file V1 generation hashes on this thread.
     static V1_FILE_HASHES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+    /// Test-only count of full V1 or V2 ledger scans (one per open) on this
+    /// thread (D-4781).
+    static PRE_ADMISSION_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
 }
 
 fn hash_file(file: &mut File, path: &Path) -> Result<[u8; 32], PreAdmissionDataRefusal> {
@@ -3884,11 +4081,19 @@ fn read_record_v2(
     file: &mut File,
     index: u64,
 ) -> Result<(RecordKindV2, PreAdmissionDataV2), PreAdmissionDataRefusal> {
+    PreAdmissionDataV2::decode(&read_raw_record_v2(file, index)?)
+}
+
+/// The literal bytes of one V2 record, undecoded.
+fn read_raw_record_v2(
+    file: &mut File,
+    index: u64,
+) -> Result<[u8; RECORD_BYTES_V2], PreAdmissionDataRefusal> {
     let mut raw = [0_u8; RECORD_BYTES_V2];
     file.seek(SeekFrom::Start(record_offset_v2(index)?))
         .and_then(|_| file.read_exact(&mut raw))
         .map_err(|why| format!("cannot read pre-admission V2 record {index}: {why}"))?;
-    PreAdmissionDataV2::decode(&raw)
+    Ok(raw)
 }
 
 fn file_generation_v2(
@@ -5211,6 +5416,326 @@ mod tests {
         Ok(())
     }
 
+    #[test]
+    fn v1_and_v2_append_doors_scan_once_and_reread_only_their_pair() -> TestResult {
+        // G4-4 / D-4781: each production append door opened the ledger, then
+        // dropped it and ran a second full `open_read`: two whole-file opens
+        // per append, on ledgers at the store root every run shares.
+        let root = test_dir()?;
+        let mut v1_commits = Vec::new();
+        for (tag, written) in [(30, true), (31, true), (30, false)] {
+            let produced = ProducedPreAdmissionDataV1 {
+                value: fixture(tag)?,
+            };
+            PRE_ADMISSION_SCANS.with(|count| count.set(0));
+            let committed = must(
+                produced.append_and_reopen(root.path(), bounds(4)?),
+                "the V1 door commits",
+            )?;
+            assert_eq!(
+                PRE_ADMISSION_SCANS.with(std::cell::Cell::get),
+                1,
+                "one full V1 scan per append door, not two"
+            );
+            assert_eq!(
+                matches!(committed, PreAdmissionProductionCommitV1::Written(_)),
+                written
+            );
+            v1_commits.push((produced.value, committed.audit()));
+        }
+        let mut v2_commits = Vec::new();
+        for (tag, written) in [(190, true), (191, true), (190, false)] {
+            let produced = ProducedPreAdmissionDataV2 {
+                value: zero_fixture_v2(tag)?,
+            };
+            PRE_ADMISSION_SCANS.with(|count| count.set(0));
+            let committed = must(
+                produced.append_and_reopen(root.path(), bounds_v2(4)?),
+                "the V2 door commits",
+            )?;
+            assert_eq!(
+                PRE_ADMISSION_SCANS.with(std::cell::Cell::get),
+                1,
+                "one full V2 scan per append door, not two"
+            );
+            assert_eq!(
+                matches!(committed, PreAdmissionProductionCommitV2::Written(_)),
+                written
+            );
+            v2_commits.push((produced.value, committed.audit()));
+        }
+        // What each door returned is exactly what a fresh reader finds.
+        let v1 = must(
+            PreAdmissionDataLedgerV1::open_read(root.path(), bounds(4)?),
+            "V1 reopens",
+        )?;
+        for (value, audit) in v1_commits {
+            assert_eq!(
+                must(v1.reopen_audit(&value.authority_id()), "V1 lookup")?,
+                Some(audit)
+            );
+        }
+        let v2 = must(
+            PreAdmissionDataLedgerV2::open_read(root.path(), bounds_v2(4)?),
+            "V2 reopens",
+        )?;
+        for (value, audit) in v2_commits {
+            assert_eq!(
+                must(v2.reopen_audit(&value.authority_id()), "V2 lookup")?,
+                Some(audit)
+            );
+        }
+        Ok(())
+    }
+
+    /// Writes `raw` at byte `offset` of `path` and syncs it.
+    fn overwrite_record(path: &Path, offset: u64, raw: &[u8]) -> TestResult {
+        let mut file = must(OpenOptions::new().write(true).open(path), "data reopens")?;
+        must(file.seek(SeekFrom::Start(offset)), "the record seeks")?;
+        must(file.write_all(raw), "the record writes")?;
+        must(file.sync_data(), "the record syncs")
+    }
+
+    #[test]
+    #[expect(
+        clippy::too_many_lines,
+        reason = "one adversarial sequence keeps every V1 re-read refusal in its disk order"
+    )]
+    fn v1_reverify_refuses_every_disagreement_with_the_disk() -> TestResult {
+        let root = test_dir()?;
+        let mut ledger = must(
+            PreAdmissionDataLedgerV1::open(root.path(), bounds(4)?),
+            "the ledger opens",
+        )?;
+        let first = must(
+            ledger.append_complete(&fixture(40)?),
+            "the first pair commits",
+        )?;
+        let second = must(
+            ledger.append_complete(&fixture(41)?),
+            "the second pair commits",
+        )?;
+        assert_eq!(
+            must(ledger.reverify_committed(&second), "the last written pair")?,
+            second.audit()
+        );
+        assert_eq!(
+            must(
+                ledger.reverify_committed(&PreAdmissionProductionCommitV1::Reused(first.audit())),
+                "an older reused pair"
+            )?,
+            first.audit()
+        );
+        assert!(
+            must_refuse(ledger.reverify_committed(&first), "an older written pair")?
+                .contains("was written but is not the last stored pair")
+        );
+        let other_root = test_dir()?;
+        let mut other = must(
+            PreAdmissionDataLedgerV1::open(other_root.path(), bounds(4)?),
+            "another ledger opens",
+        )?;
+        let foreign = must(
+            other.append_complete(&fixture(42)?),
+            "a foreign pair commits",
+        )?;
+        assert!(
+            must_refuse(ledger.reverify_committed(&foreign), "an absent authority")?
+                .contains("disappeared after receipt-last append")
+        );
+
+        let data_path = ledger.data_path.clone();
+        let value = second.audit().value;
+        let data_at = must(
+            record_offset(second.audit().data_record_index),
+            "the Data offset",
+        )?;
+        let completion_at = must(
+            record_offset(second.audit().data_record_index + 1),
+            "the completion offset",
+        )?;
+        let data_record = must(value.record(RecordKindV1::Data), "the Data record encodes")?;
+        let completion_record = must(
+            value.record(RecordKindV1::Completion),
+            "the completion encodes",
+        )?;
+        overwrite_record(&data_path, data_at, &completion_record)?;
+        assert!(
+            must_refuse(ledger.reverify_committed(&second), "a moved generation")?
+                .contains("changed after pre-admission open")
+        );
+        ledger.data_generation = must(file_generation(&ledger.data_file, &data_path), "remeasure")?;
+        assert!(
+            must_refuse(
+                ledger.reverify_committed(&second),
+                "a Data record of another kind"
+            )?
+            .contains("Data record differs on disk")
+        );
+        overwrite_record(&data_path, data_at, &data_record)?;
+        overwrite_record(&data_path, completion_at, &data_record)?;
+        ledger.data_generation = must(file_generation(&ledger.data_file, &data_path), "remeasure")?;
+        assert!(
+            must_refuse(
+                ledger.reverify_committed(&second),
+                "a completion of another kind"
+            )?
+            .contains("completion differs on disk")
+        );
+        overwrite_record(&data_path, completion_at, &completion_record)?;
+        ledger.data_generation = must(file_generation(&ledger.data_file, &data_path), "remeasure")?;
+        assert_eq!(
+            must(ledger.reverify_committed(&second), "the restored pair")?,
+            second.audit()
+        );
+
+        let end = must(std::fs::metadata(&data_path), "the data file measures")?.len();
+        overwrite_record(&data_path, end, &data_record)?;
+        ledger.data_generation = must(file_generation(&ledger.data_file, &data_path), "remeasure")?;
+        assert!(
+            must_refuse(ledger.reverify_committed(&second), "an extra record")?
+                .contains("records on disk, not the 4 this append committed")
+        );
+
+        let lock_path = ledger.lock_path.clone();
+        must(
+            std::fs::rename(&lock_path, root.path().join("displaced.lock")),
+            "the held lock is displaced",
+        )?;
+        must(File::create(&lock_path), "a new lock inode appears")?;
+        assert!(
+            must_refuse(ledger.reverify_committed(&second), "a replaced lock")?
+                .contains("no longer names")
+        );
+        Ok(())
+    }
+
+    #[test]
+    #[expect(
+        clippy::too_many_lines,
+        reason = "one adversarial sequence keeps every V2 re-read refusal in its disk order"
+    )]
+    fn v2_reverify_refuses_every_disagreement_with_the_disk() -> TestResult {
+        let root = test_dir()?;
+        let mut ledger = must(
+            PreAdmissionDataLedgerV2::open(root.path(), bounds_v2(4)?),
+            "the ledger opens",
+        )?;
+        let first = must(
+            ledger.append_complete(&zero_fixture_v2(180)?),
+            "the first pair commits",
+        )?;
+        let second = must(
+            ledger.append_complete(&zero_fixture_v2(181)?),
+            "the second pair commits",
+        )?;
+        assert_eq!(
+            must(ledger.reverify_committed(&second), "the last written pair")?,
+            second.audit()
+        );
+        assert_eq!(
+            must(
+                ledger.reverify_committed(&PreAdmissionProductionCommitV2::Reused(first.audit())),
+                "an older reused pair"
+            )?,
+            first.audit()
+        );
+        assert!(
+            must_refuse(ledger.reverify_committed(&first), "an older written pair")?
+                .contains("was written but is not the last stored pair")
+        );
+        let other_root = test_dir()?;
+        let mut other = must(
+            PreAdmissionDataLedgerV2::open(other_root.path(), bounds_v2(4)?),
+            "another ledger opens",
+        )?;
+        let foreign = must(
+            other.append_complete(&zero_fixture_v2(182)?),
+            "a foreign pair commits",
+        )?;
+        assert!(
+            must_refuse(ledger.reverify_committed(&foreign), "an absent authority")?
+                .contains("disappeared after receipt-last append")
+        );
+
+        let data_path = ledger.data_path.clone();
+        let value = second.audit().value;
+        let data_at = must(
+            record_offset_v2(second.audit().data_record_index),
+            "the Data offset",
+        )?;
+        let completion_at = must(
+            record_offset_v2(second.audit().data_record_index + 1),
+            "the completion offset",
+        )?;
+        let data_record = must(value.record(RecordKindV2::Data), "the Data record encodes")?;
+        let completion_record = must(
+            value.record(RecordKindV2::Completion),
+            "the completion encodes",
+        )?;
+        overwrite_record(&data_path, data_at, &completion_record)?;
+        assert!(
+            must_refuse(ledger.reverify_committed(&second), "a moved generation")?
+                .contains("changed after pre-admission open")
+        );
+        ledger.data_generation = must(
+            file_generation_v2(&ledger.data_file, &data_path),
+            "remeasure",
+        )?;
+        assert!(
+            must_refuse(
+                ledger.reverify_committed(&second),
+                "a Data record of another kind"
+            )?
+            .contains("Data record differs on disk")
+        );
+        overwrite_record(&data_path, data_at, &data_record)?;
+        overwrite_record(&data_path, completion_at, &data_record)?;
+        ledger.data_generation = must(
+            file_generation_v2(&ledger.data_file, &data_path),
+            "remeasure",
+        )?;
+        assert!(
+            must_refuse(
+                ledger.reverify_committed(&second),
+                "a completion of another kind"
+            )?
+            .contains("completion differs on disk")
+        );
+        overwrite_record(&data_path, completion_at, &completion_record)?;
+        ledger.data_generation = must(
+            file_generation_v2(&ledger.data_file, &data_path),
+            "remeasure",
+        )?;
+        assert_eq!(
+            must(ledger.reverify_committed(&second), "the restored pair")?,
+            second.audit()
+        );
+
+        let end = must(std::fs::metadata(&data_path), "the data file measures")?.len();
+        overwrite_record(&data_path, end, &data_record)?;
+        ledger.data_generation = must(
+            file_generation_v2(&ledger.data_file, &data_path),
+            "remeasure",
+        )?;
+        assert!(
+            must_refuse(ledger.reverify_committed(&second), "an extra record")?
+                .contains("records on disk, not the 4 this append committed")
+        );
+
+        let lock_path = ledger.lock_path.clone();
+        must(
+            std::fs::rename(&lock_path, root.path().join("displaced.lock")),
+            "the held lock is displaced",
+        )?;
+        must(File::create(&lock_path), "a new lock inode appears")?;
+        assert!(
+            must_refuse(ledger.reverify_committed(&second), "a replaced lock")?
+                .contains("no longer names")
+        );
+        Ok(())
+    }
+
     #[test]
     fn writer_refuses_missing_or_non_directory_root_without_creating_it() -> TestResult {
         let parent = test_dir()?;
diff --git a/crates/cli/tests/ledger_append_lookup_costs.rs b/crates/cli/tests/ledger_append_lookup_costs.rs
index 9467f410..0d79c8c6 100644
--- a/crates/cli/tests/ledger_append_lookup_costs.rs
+++ b/crates/cli/tests/ledger_append_lookup_costs.rs
@@ -107,6 +107,38 @@ fn section_150_states_one_open_per_production_append_and_no_data_hash() {
     assert!(header.contains("one production append costs O(rows + receipts)"));
 }
 
+#[test]
+fn a_pre_admission_append_door_opens_once_and_section_153_says_so() {
+    let mut doors = 0;
+    let mut rest = PRE_ADMISSION;
+    while let Some(at) = rest.find("    pub(crate) fn append_and_reopen(") {
+        let door = method(rest, "    pub(crate) fn append_and_reopen(");
+        assert_eq!(door.matches("::open(root, bounds)?").count(), 1);
+        assert!(door.contains("ledger.reverify_committed(&committed)?"));
+        assert!(
+            !door.contains("open_read"),
+            "a Pre-Admission append door opens the ledger twice again; re-measure §153"
+        );
+        doors += 1;
+        rest = rest.get(at + 1..).expect("a suffix");
+    }
+    assert_eq!(
+        doors, 2,
+        "Pre-Admission V1 and V2 each have one append door"
+    );
+
+    let text = flat(section(153));
+    for needed in [
+        "One production append door, V1 or V2, runs one full open since D-4781",
+        "two full opens per append",
+        "One append is still O(file bytes)",
+    ] {
+        assert!(text.contains(needed), "§153 no longer says `{needed}`");
+    }
+    let module = flat(PRE_ADMISSION.get(..3_400).expect("the module header"));
+    assert!(module.contains("opens the ledger once and then re-reads only its committed pair"));
+}
+
 /// The body of the method opened by `signature` in `source`, up to the next
 /// line that closes at one indent.
 fn method<'a>(source: &'a str, signature: &str) -> &'a str {
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 31e467ea..7f4bc771 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7052,3 +7052,11 @@ old line regex the same input and watched it pass.
 | G18-api-27 | The seek path's `records unreadable` line names the first file that refused a record, not the first file read (D-2046) | `api::bars::window_tests::the_unreadable_line_names_the_first_damaged_file_not_the_first_file` | ✓ |
 | G18-api-28 | The route test's HTTP exchange is bounded at 30 s per read and write, so a server that admits or answers nothing fails it rather than hanging (D-2047) | `api::ingest::route_tests::the_three_routes_answer_and_none_of_them_shadows_the_front_end` | ✓ |
 | G18-api-29 | A dropped calendar `Landing` marks its flight `Abandoned` (or answered), removes it from the flight table, and wakes every follower (D-2047) | `api::calendar_of::tests::a_calendar_landing_releases_its_flight_and_wakes_its_followers_when_dropped` | ✓ |
+
+### Lane 1-b fixer F: one Candidate writer per run, pair re-reads, hoisted OOS digests, handed-on sizing (D-4780 onward)
+
+| Id | Invariant | Proof | |
+|---|---|---|---|
+| L1FF-06 | Each Pre-Admission V1 and V2 append door, written or reused, scans its ledger once, and what it returns is what a fresh reader finds (D-4781) | `cli::pre_admission_data::tests::v1_and_v2_append_doors_scan_once_and_reread_only_their_pair` | ✓ |
+| L1FF-07 | The V1 and V2 pair re-read accepts the last written pair and an older reused pair, and refuses an older written pair, an absent authority, a moved generation, a Data or Completion record of another kind under a re-measured generation, an extra record and a replaced lock (D-4781) | `cli::pre_admission_data::tests::v1_reverify_refuses_every_disagreement_with_the_disk`, `cli::pre_admission_data::tests::v2_reverify_refuses_every_disagreement_with_the_disk` | ✓ |
+| L1FF-14 | §153 states the Pre-Admission door's one open and pair re-read and that an append still hashes the file; the doors call `reverify_committed` and no `open_read` (D-4781) | `cli::ledger_append_lookup_costs::a_pre_admission_append_door_opens_once_and_section_153_says_so` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 552ce57a..3cd27dc2 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65025,3 +65025,41 @@ on a live leader would derive a second time and lose the single-flight
 guarantee D-1443 exists for. **Honest limit:** the `Landing` kill depends on
 test order. A rename that sorted a single-flight test ahead of it would
 restore the timeout, so the ordering is pinned in the test's own doc.
+
+### D-4781 — The Pre-Admission V1 and V2 append doors re-read only their pair — 2026-10-09
+
+**What was observed.** G4-4 found a sibling of W2-cli3-4 that no document
+stated. Both Pre-Admission append doors ran the same sequence: `open` (a full
+scan plus a whole-file content hash), the append, `drop`, a fresh `open_read`
+(another full scan and content hash) and `reopen_audit` (a third hash). That is
+two full opens per append. The doors run once per family per rung, on ledgers at
+the shared store root.
+
+**Decided.** This is the D-1680 pattern. Each door runs one `open`, the append,
+then `reverify_committed` on the same handle under the shared lock:
+
+- The lock and data generations are compared by metadata.
+- The physical record count must be exactly twice the completed pairs the
+  handle indexed, so no orphan remains.
+- The authority must be in the index, and a written pair must be the last one.
+- The Data and Completion records on disk must be byte for byte the two records
+  the indexed value encodes.
+
+The re-read is O(1) in ledger size. The door then compares the re-read audit
+with the committed one and with the derived semantics, as two separate checks
+(D-2004). One door now scans the ledger once.
+
+**Honest limit.** One append is still O(file bytes). The append's own generation
+checks hash the whole data file: `require_unchanged` before writing, then one
+re-measure after each of the two records. Only the second open and the lookup's
+hash are gone. §153 says so.
+
+**Rejected.** Carrying one Pre-Admission writer per run, as D-4780 does for
+Candidate. The append's content-hash generation checks keep every append at
+O(file bytes) either way, so the class would not change.
+
+Tests:
+- `cli::pre_admission_data::tests::v1_and_v2_append_doors_scan_once_and_reread_only_their_pair`
+  (fails at 2 against 1 scans per door on the two-open code).
+- `cli::pre_admission_data::tests::v1_reverify_refuses_every_disagreement_with_the_disk`.
+- `cli::pre_admission_data::tests::v2_reverify_refuses_every_disagreement_with_the_disk`.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 12e9bf09..a277f53c 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -8108,6 +8108,17 @@ The metadata check is length, device/inode and nanosecond modification/change
 times: a same-length rewrite of a record outside the page that left all of
 those equal is not seen by that page, and a rewrite of a returned record is
 refused by its seal.
+
+One production append door, V1 or V2, runs one full open since D-4781
+(G4-4): the open, the append, then a re-read of only the committed Data and
+Completion pair through the writer's own handle. The re-read compares both
+generations by metadata, requires the physical record count to be exactly the
+completed pairs (no orphan) and a written pair to be the last, and compares the
+pair's bytes with the two records the indexed value encodes, O(1) in ledger
+size. Before D-4781 each door then dropped the handle and ran a second full
+`open_read` plus a hashing `reopen_audit`, two full opens per append. One append
+is still O(file bytes): the append's own generation checks content-hash the
+data file before it writes and after each of its two records.
 Lock acquisition, filesystem cache, `sync_data`, allocation and storage latency
 remain system-dependent.
 
-- 
2.43.0


From c2db65fe51d605284be76a776b0b1d35800a42d8 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:27:11 +0000
Subject: [PATCH 2/4] A stored OOS witness hashes no stream (W2-cli3-3, D-4782)
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Before: after D-1684 built the OOS source once per cohort, every witness
still ran the cohort integrity check twice, each re-deriving the cohort
identity by hashing the signal, minute-context, daily and execution
streams; the source's `mint_witness_recorded` re-hashed its own data
identity; and `ExecutionRunV1::new_with_daily_reference` hashed all four
streams again to seal the run. A witness was Θ(S + Q + D + E) in hashing.

After: the Global Replay OOS source hashes its run digests once at
construction (`ExecutionDigestsV1::of_daily_reference`) and seals each
witness with `ExecutionRunV1::with_digests`, which runner proves equal to
hashing per run. The fold's witness calls the cohort's `require_current`
(strict guards, admitted root, O(1) audit compares) instead of
`require_integrity`; the full re-derivation still runs at construction and
once per fold. Every stream is borrowed immutably for the source's life,
so nothing can change it between those checks, and a changed stored file
is what the held guards refuse. Witness, run and cohort bytes are
unchanged: three witnesses over one fold equal the unfolded mint's.

Tests: strict_v6_a_witness_hashes_no_stream_its_fold_already_hashed
(failed before with 15 hashes against 3; also refuses a tampered cohort
id at the fold and a stale source per witness), and section_169 in the
integration test. docs/06 §169 now prices a witness at O(M) plus the
replay. Invariant L1FF-08.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/candidate_universe.rs          | 56 ++++++++----
 crates/cli/src/stored_post_training_oos.rs    | 50 +++++++++--
 crates/cli/src/strict_v6_tests.rs             | 90 +++++++++++++++++++
 .../cli/tests/ledger_append_lookup_costs.rs   | 11 ++-
 docs/04-invariants.md                         |  1 +
 docs/05-decisions.md                          | 48 ++++++++++
 docs/06-limits.md                             | 28 ++++--
 7 files changed, 251 insertions(+), 33 deletions(-)

diff --git a/crates/cli/src/candidate_universe.rs b/crates/cli/src/candidate_universe.rs
index 2bd12520..4c84365c 100644
--- a/crates/cli/src/candidate_universe.rs
+++ b/crates/cli/src/candidate_universe.rs
@@ -1188,7 +1188,10 @@ pub(crate) struct CandidateGlobalReplayOosSourceV1<'a> {
     daily_reference: DailyReferenceBinding<'a>,
     execution_series: ExecutionSeriesV1<'a>,
     execution_column: Column,
-    data_digest: [u8; 32],
+    /// The three-stream data term and the execution-slice digest, hashed once
+    /// here and sealed into every witness's run (W2-cli3-3, D-4782; D-0990
+    /// hoisted the same pair for Candidate).
+    run_digests: ExecutionDigestsV1,
     signal_load_bound: StoredSpanLoadBoundV1,
     minute_load_bound: StoredSpanLoadBoundV1,
     daily_load_bound: StoredSpanLoadBoundV1,
@@ -1279,9 +1282,11 @@ impl<'a> CandidateGlobalReplayOosSourceV1<'a> {
             rung_seconds,
             &evaluation,
         )?;
-        let data_digest = runner::identity::data_digest_with_daily_reference(
+        note_oos_stream_hash();
+        let run_digests = ExecutionDigestsV1::of_daily_reference(
             signal_bars,
             reference_minute_context,
+            execution_series.bars(),
             daily_reference,
         )
         .map_err(|why| format!("Global Replay OOS daily data identity refused: {why:?}"))?;
@@ -1297,7 +1302,7 @@ impl<'a> CandidateGlobalReplayOosSourceV1<'a> {
             daily_reference,
             execution_series,
             execution_column,
-            data_digest,
+            run_digests,
             signal_load_bound,
             minute_load_bound,
             daily_load_bound,
@@ -1327,10 +1332,12 @@ impl<'a> CandidateGlobalReplayOosSourceV1<'a> {
             &self.execution_column,
             self.execution_series.bars().len(),
         )?;
-        if self.data_digest
-            != runner::identity::data_digest_with_daily_reference(
+        note_oos_stream_hash();
+        if self.run_digests
+            != ExecutionDigestsV1::of_daily_reference(
                 self.signal_bars,
                 self.reference_minute_context,
+                self.execution_series.bars(),
                 self.daily_reference,
             )
             .map_err(|why| format!("Global Replay OOS data identity refused: {why:?}"))?
@@ -1359,6 +1366,16 @@ impl<'a> CandidateGlobalReplayOosSourceV1<'a> {
 
     /// Call the fallible identity publisher immediately before trade replay.
     /// The existing mint entry point preserves its original caller contract.
+    ///
+    /// A witness hashes no stream (W2-cli3-3, D-4782). The source's
+    /// integrity was proved once, at construction, and every stream it holds
+    /// is borrowed immutably for its whole life, so re-hashing them per
+    /// witness re-read memory that cannot change; a stored file that changes
+    /// is refused by the cohort's held strict guards before each witness.
+    /// The run is sealed against the digests construction hashed, through
+    /// Runner's `ExecutionRunV1::with_digests`, which is proved equal to
+    /// hashing per run by
+    /// `runner::exit_grid_policy::sealing_against_hoisted_digests_equals_hashing_per_run`.
     pub(crate) fn mint_witness_recorded(
         &self,
         ladder: engine::Ladder,
@@ -1366,7 +1383,6 @@ impl<'a> CandidateGlobalReplayOosSourceV1<'a> {
         disposition: &ExecutionDispositionV1,
         before_replay: &mut dyn FnMut([u8; 32]) -> Result<(), String>,
     ) -> Result<GlobalReplayWitnessUniverseV1, CandidateUniverseRefusal> {
-        self.require_integrity()?;
         let selected = disposition.selected().ok_or_else(|| {
             "Global Replay OOS winner has no authorized selected-exit capability".to_owned()
         })?;
@@ -1403,18 +1419,12 @@ impl<'a> CandidateGlobalReplayOosSourceV1<'a> {
                 u64::from(self.rung_seconds),
                 u64::from(self.horizon.as_bars()),
             ]),
-            data_digest: self.data_digest,
+            data_digest: self.run_digests.data_digest(),
             commit: self.execution_series.commit(),
             feed: self.execution_series.feed(),
         };
-        let execution_run = ExecutionRunV1::new_with_daily_reference(
-            &run,
-            self.signal_bars,
-            self.reference_minute_context,
-            self.execution_series.bars(),
-            self.daily_reference,
-        )
-        .map_err(|why| format!("Global Replay OOS exact run refused: {why:?}"))?;
+        let execution_run = ExecutionRunV1::with_digests(&run, &self.run_digests)
+            .map_err(|why| format!("Global Replay OOS exact run refused: {why:?}"))?;
         let oos = OosExecutionSeriesV1::new(self.execution_series, 0)
             .map_err(|why| format!("Global Replay OOS boundary refused: {why:?}"))?;
         before_replay(execution_run.run_id().bytes())?;
@@ -1468,7 +1478,7 @@ fn derive_global_replay_oos_source_id(source: &CandidateGlobalReplayOosSourceV1<
     hasher.update(&source.requested_span.canonical_bytes());
     hasher.update(&source.signal_calendar.digest());
     hasher.update(&source.execution_calendar.digest());
-    hasher.update(&source.data_digest);
+    hasher.update(&source.run_digests.data_digest());
     hasher.update(&column_digest_v1(&source.execution_column));
     if let Some(spec) = source.execution_column.evaluation_spec_token() {
         hasher.update(spec.fingerprint_v1().as_bytes());
@@ -4944,6 +4954,20 @@ thread_local! {
     pub(crate) static OOS_SOURCE_BUILDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
 }
 
+#[cfg(test)]
+thread_local! {
+    /// Test-only count of whole-stream hash passes the stored OOS path makes
+    /// on this thread: a cohort identity derivation, a source data identity
+    /// and a run's stream digests each count one (D-4782).
+    pub(crate) static OOS_STREAM_HASHES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+}
+
+/// Counts one whole-stream hash pass of the stored OOS path, in tests only.
+pub(crate) fn note_oos_stream_hash() {
+    #[cfg(test)]
+    OOS_STREAM_HASHES.with(|count| count.set(count.get().saturating_add(1)));
+}
+
 #[cfg(test)]
 thread_local! {
     /// Test-only count of `CandidateUniverseLedgerV1::scan` calls (one per
diff --git a/crates/cli/src/stored_post_training_oos.rs b/crates/cli/src/stored_post_training_oos.rs
index 15172d90..42cf7063 100644
--- a/crates/cli/src/stored_post_training_oos.rs
+++ b/crates/cli/src/stored_post_training_oos.rs
@@ -219,11 +219,18 @@ pub(crate) struct StoredOosFoldV1<'c> {
 }
 
 impl StoredOosFoldV1<'_> {
-    /// Mints one witness over this fold. Per witness: the cohort integrity
-    /// check twice (each re-derives the cohort identity, hashing the signal,
-    /// minute-context, daily and execution streams, Θ(S + Q + D + E)), the
+    /// Mints one witness over this fold. Per witness: the cohort currency
+    /// check twice (the held strict guards, the admitted root and the O(1)
+    /// audit fields; `StoredPostTrainingOosCohortV1::require_current`), the
     /// evaluator check, and the Runner replay over the OOS execution bars.
-    /// The fold's column evaluation and alignment are not repeated.
+    /// The fold's column evaluation, alignment and stream hashing are not
+    /// repeated: before W2-cli3-3's second fix (D-4782) each witness also
+    /// re-derived the cohort identity twice, re-hashed the source's data
+    /// identity and hashed all four streams to seal its run, Θ(S + Q + D + E)
+    /// in hashing. Those streams are owned by the cohort and borrowed
+    /// immutably by the fold, so the re-hash could not see a change; a
+    /// changed stored file is what the held strict guards refuse. Proof:
+    /// `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_a_witness_hashes_no_stream_its_fold_already_hashed`.
     ///
     /// # Errors
     ///
@@ -243,7 +250,7 @@ impl StoredOosFoldV1<'_> {
         mut observer: Option<&mut StoredOosObserverV1<'_>>,
     ) -> Result<StoredPostTrainingOosWitnessV1, StoredPostTrainingOosRefusal> {
         let cohort = self.cohort;
-        cohort.require_integrity()?;
+        cohort.require_current()?;
         if self.specification.as_bytes() != &disposition.evaluation_spec_fingerprint() {
             return Err("stored OOS disposition evaluator differs before fold".to_owned());
         }
@@ -268,7 +275,7 @@ impl StoredOosFoldV1<'_> {
         cohort
             .root
             .require_same("after minting stored post-training OOS witness")?;
-        cohort.require_integrity()?;
+        cohort.require_current()?;
         let run_id = witness.run_id().bytes();
         let selected_exit_digest = witness.selected_exit_digest();
         let universe_digest = witness.universe_digest();
@@ -371,7 +378,24 @@ impl StoredPostTrainingOosCohortV1 {
         self.audit
     }
 
+    /// The full check: [`Self::require_current`], then the cohort identity
+    /// re-derived from every stream, Θ(S + Q + D + E) in hashing. Run at
+    /// construction and once per fold.
     fn require_integrity(&self) -> Result<(), StoredPostTrainingOosRefusal> {
+        self.require_current()?;
+        if self.audit.cohort_id != self.derive_cohort_id()? {
+            return Err("stored post-training OOS cohort identity changed".to_owned());
+        }
+        Ok(())
+    }
+
+    /// The per-witness check (W2-cli3-3, D-4782): the held strict guards
+    /// (O(M) metadata and receipt checks), the admitted root, and the audit
+    /// fields compared in O(1). It hashes no stream: the cohort owns them and
+    /// nothing can change them while it is borrowed, and a changed stored
+    /// file is what the guards refuse. Proof:
+    /// `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_a_witness_hashes_no_stream_its_fold_already_hashed`.
+    fn require_current(&self) -> Result<(), StoredPostTrainingOosRefusal> {
         self.stored.require_current()?;
         self.root
             .require_same("while authenticating stored post-training OOS cohort")?;
@@ -388,7 +412,6 @@ impl StoredPostTrainingOosCohortV1 {
             || last.ts_micros != self.audit.oos_last_ts_micros
             || usize_to_u64(execution.len(), "execution bars")? != self.audit.execution_bars
             || first.ts_micros <= training_last
-            || self.audit.cohort_id != self.derive_cohort_id()?
         {
             return Err("stored post-training OOS cohort identity changed".to_owned());
         }
@@ -533,6 +556,7 @@ impl StoredPostTrainingOosCohortV1 {
     }
 
     fn derive_cohort_id(&self) -> Result<[u8; 32], StoredPostTrainingOosRefusal> {
+        crate::candidate_universe::note_oos_stream_hash();
         let execution = self.execution()?;
         let execution_calendar = crate::stored::calendar_receipt_v2_for_bars(
             execution,
@@ -636,3 +660,15 @@ fn hash_parts(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
     }
     hasher.finalize()
 }
+
+#[cfg(test)]
+impl StoredPostTrainingOosCohortV1 {
+    /// Test-only: flips one byte of the cached cohort identity, so the full
+    /// check refuses while the per-witness check, which never re-derives it,
+    /// cannot see it.
+    pub(crate) fn flip_cohort_id_for_test(&mut self) {
+        if let Some(byte) = self.audit.cohort_id.first_mut() {
+            *byte ^= 1;
+        }
+    }
+}
diff --git a/crates/cli/src/strict_v6_tests.rs b/crates/cli/src/strict_v6_tests.rs
index 709cdff4..485f1fd1 100644
--- a/crates/cli/src/strict_v6_tests.rs
+++ b/crates/cli/src/strict_v6_tests.rs
@@ -400,6 +400,96 @@ mod strict_v6_fixture_tests {
         Ok(())
     }
 
+    #[test]
+    fn strict_v6_a_witness_hashes_no_stream_its_fold_already_hashed() -> Result<(), String> {
+        // W2-cli3-3 / D-4782: after D-1684 built the OOS source once per
+        // cohort, every witness still re-derived the cohort identity twice,
+        // re-hashed the source's data identity and hashed all four streams
+        // again to seal its run: Θ(S + Q + D + E) per witness.
+        let fixture = StoredSuccessFixture::new()?;
+        seed_stored_family_month(
+            &fixture.source,
+            Vendor::Zerodha,
+            "NIFTY",
+            FIXTURE_OOS_MONTH,
+            2_000_000,
+        )?;
+        let config = strict_fixture_config(&fixture)?;
+        let long = exit_policy(Side::Long)?;
+        let short = exit_policy(Side::Short)?;
+        let diagnostic = Sweeper::new(engine::Ladder::with_min_hits(1));
+        let support = maximum_fixture_singleton_support(&fixture_request(
+            &fixture.source,
+            "NIFTY",
+            &diagnostic,
+            &long,
+            &short,
+        )?)?;
+        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(support));
+        let committed = commit_stored_with_inputs_v1(
+            fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?,
+            VerifiedBuildCommitV1(FIXTURE_COMMIT),
+            &|_, _, _| {},
+            Some(&config),
+        )?;
+        let disposition = first_selected_disposition(&committed)?;
+        let cohort = committed.stored_post_training_oos_cohort(fixture_oos_request()?)?;
+        // The unfolded mint is the reference: it builds its own fold.
+        let unfolded = cohort.mint_witness(&disposition)?;
+
+        crate::candidate_universe::OOS_STREAM_HASHES.with(|count| count.set(0));
+        let fold = cohort.fold_recorded(&mut |_| Ok(()))?;
+        let per_fold = crate::candidate_universe::OOS_STREAM_HASHES.with(std::cell::Cell::get);
+        assert!(per_fold > 0, "the fold hashes the cohort's streams");
+        for _ in 0..3 {
+            let witness = fold.mint_witness_recorded(&disposition, &mut |_| Ok(()))?;
+            assert_eq!(witness.cohort_id(), unfolded.cohort_id());
+            assert_eq!(
+                witness.witness_id(),
+                unfolded.witness_id(),
+                "the witness binds the same run, exit and replay universe bytes"
+            );
+            assert_eq!(witness.candidate_count(), unfolded.candidate_count());
+        }
+        assert_eq!(
+            crate::candidate_universe::OOS_STREAM_HASHES.with(std::cell::Cell::get),
+            per_fold,
+            "three witnesses re-hash no stream the fold already hashed"
+        );
+
+        // The full check, run once per fold, still re-derives the cohort
+        // identity and refuses one that no longer matches it.
+        let mut tampered = committed.stored_post_training_oos_cohort(fixture_oos_request()?)?;
+        tampered.flip_cohort_id_for_test();
+        assert!(
+            tampered
+                .fold_recorded(&mut |_| Ok(()))
+                .err()
+                .is_some_and(|why| why.contains("cohort identity changed")),
+            "a fold re-proves the cohort identity"
+        );
+
+        // A source that changes after the fold is still refused per witness,
+        // by the held strict guards rather than by re-hashing owned memory.
+        let key = crate::stored::swept_index("NIFTY")?;
+        let path = StorePath::for_key(
+            Vendor::Zerodha,
+            &key,
+            Timeframe::DAY_1,
+            YearMonth::new(2025, 10).map_err(|why| why.to_string())?,
+            FileKind::Bars,
+        )
+        .map_err(|why| why.to_string())?
+        .to_path_buf(&fixture.source);
+        corrupt_strict_fixture_byte(&path, 700)?;
+        assert!(
+            fold.mint_witness_recorded(&disposition, &mut |_| Ok(()))
+                .is_err(),
+            "a stale cohort refuses over a built fold"
+        );
+        Ok(())
+    }
+
     #[test]
     fn strict_v6_extinct_selection_keeps_both_family_sources_through_final_reauthentication()
     -> Result<(), String> {
diff --git a/crates/cli/tests/ledger_append_lookup_costs.rs b/crates/cli/tests/ledger_append_lookup_costs.rs
index 0d79c8c6..e96300ab 100644
--- a/crates/cli/tests/ledger_append_lookup_costs.rs
+++ b/crates/cli/tests/ledger_append_lookup_costs.rs
@@ -38,6 +38,7 @@ const STATISTICS: &str = include_str!("../src/population_statistics_v2.rs");
 const LEDGER_V6: &str = include_str!("../src/ledger_v6.rs");
 const STRICT_INPUTS: &str = include_str!("../src/strict_v6_inputs.rs");
 const POPULATION_V6: &str = include_str!("../src/population_v6.rs");
+const OOS: &str = include_str!("../src/stored_post_training_oos.rs");
 
 /// The body of one `### §N —` section, up to the next `### ` heading.
 fn section(number: u32) -> &'static str {
@@ -299,10 +300,18 @@ fn section_169_prices_the_oos_fold_once_per_cohort() {
     let text = flat(section(169));
     for needed in [
         "Since D-1684 Population V6 builds it once per family cohort",
-        "so a witness remains Θ(S + Q + D + E) in hashing",
+        "Since D-4782 (W2-cli3-3's second fix) a witness hashes no stream",
+        "So a witness is O(M) plus the authenticated Runner replay",
     ] {
         assert!(text.contains(needed), "§169 no longer says `{needed}`");
     }
+    let mint = method(CANDIDATE, "    pub(crate) fn mint_witness_recorded(");
+    assert!(mint.contains("ExecutionRunV1::with_digests(&run, &self.run_digests)"));
+    assert!(!mint.contains("new_with_daily_reference"));
+    assert!(!mint.contains("self.require_integrity()"));
+    let witness = method(OOS, "    fn mint_inner(");
+    assert_eq!(witness.matches("cohort.require_current()?").count(), 2);
+    assert!(!witness.contains("require_integrity()?;"));
     assert!(
         !text.contains("Minting every witness is proportional to the authenticated Runner replay")
     );
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 7f4bc771..b1fb767a 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7059,4 +7059,5 @@ old line regex the same input and watched it pass.
 |---|---|---|---|
 | L1FF-06 | Each Pre-Admission V1 and V2 append door, written or reused, scans its ledger once, and what it returns is what a fresh reader finds (D-4781) | `cli::pre_admission_data::tests::v1_and_v2_append_doors_scan_once_and_reread_only_their_pair` | ✓ |
 | L1FF-07 | The V1 and V2 pair re-read accepts the last written pair and an older reused pair, and refuses an older written pair, an absent authority, a moved generation, a Data or Completion record of another kind under a re-measured generation, an extra record and a replaced lock (D-4781) | `cli::pre_admission_data::tests::v1_reverify_refuses_every_disagreement_with_the_disk`, `cli::pre_admission_data::tests::v2_reverify_refuses_every_disagreement_with_the_disk` | ✓ |
+| L1FF-08 | Three witnesses over one fold hash no stream the fold hashed, each equals the unfolded mint's cohort id, witness id and candidate count, a fold refuses a tampered cohort identity, and a stored file changed after the fold refuses the next witness (D-4782) | `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_a_witness_hashes_no_stream_its_fold_already_hashed` | ✓ |
 | L1FF-14 | §153 states the Pre-Admission door's one open and pair re-read and that an append still hashes the file; the doors call `reverify_committed` and no `open_read` (D-4781) | `cli::ledger_append_lookup_costs::a_pre_admission_append_door_opens_once_and_section_153_says_so` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 3cd27dc2..67b0a54f 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65063,3 +65063,51 @@ Tests:
   (fails at 2 against 1 scans per door on the two-open code).
 - `cli::pre_admission_data::tests::v1_reverify_refuses_every_disagreement_with_the_disk`.
 - `cli::pre_admission_data::tests::v2_reverify_refuses_every_disagreement_with_the_disk`.
+
+### D-4782 — A stored OOS witness hashes no stream — 2026-10-09
+
+**What was observed.** W2-cli3-3 was PARTIAL after D-1684. The fold was built
+once per cohort, but every witness still hashed all four streams four times:
+
+- it ran the cohort's `require_integrity` twice, and each run re-derived the
+  cohort identity by hashing the signal, minute-context, daily and execution
+  streams;
+- it ran the source's `require_integrity` once, which re-hashed the three-stream
+  data term;
+- it ran `ExecutionRunV1::new_with_daily_reference`, which hashed all four
+  streams again to seal the run.
+
+So a witness stayed Θ(S + Q + D + E) in hashing. With three witnesses a fold
+counted 15 stream-hash passes, where the fold alone needs 3. D-1684 kept the
+per-witness check because it "refuses a source that changed after the fold".
+That reason did not hold. The cohort owns those streams and the fold borrows
+them immutably for its whole life, so re-hashing them could not see a change.
+A changed stored file is caught by the held strict guards.
+
+**Decided.**
+- `CandidateGlobalReplayOosSourceV1` hashes `ExecutionDigestsV1::of_daily_reference`
+  once, at construction, and keeps the result. Its integrity check compares that
+  full digest pair.
+- Each witness seals its run with `ExecutionRunV1::with_digests`, D-0990's
+  Runner door. Its equality with hashing per run is proved by
+  `runner::exit_grid_policy::sealing_against_hoisted_digests_equals_hashing_per_run`.
+- The per-witness cohort check is now `require_current`: the held strict guards
+  (O(M) metadata and receipt checks), the admitted root, and the cached audit
+  fields in O(1).
+- The full `require_integrity`, which re-derives the cohort identity, runs at
+  construction and once per fold.
+
+No witness byte, run id, cohort id or witness id changes: the test compares each
+witness with the cohort's unfolded mint. A witness now costs O(M) plus the
+Runner replay.
+
+**Rejected.** G4's step 4, one more full re-check after the last witness. It
+would re-hash memory that cannot change, so no test could make it refuse: a
+check no input can provoke. The per-witness guards are what detects a change.
+A daily file corrupted after the fold still refuses the next witness.
+
+Tests:
+- `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_a_witness_hashes_no_stream_its_fold_already_hashed`
+  (fails at 15 against 3 passes on the re-hashing code; it also refuses a
+  tampered cohort identity at the fold).
+- `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_one_oos_fold_serves_every_witness_of_its_cohort`.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index a277f53c..3ac2e55c 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -8689,15 +8689,25 @@ source a witness replays over (the anchored signal column, its exact-minute
 overlay, the checked execution column, alignment, calendars and stream
 digests) costs Θ(S + Q + D + E) to build. Since D-1684 Population V6 builds it
 once per family cohort and every witness of that cohort replays over it;
-before D-1684 it was rebuilt for every witness. Each witness still pays two
-cohort integrity checks, and each re-derives the cohort identity by hashing
-the signal, minute-context, daily and execution streams and re-checks the
-strict source guards, so a witness remains Θ(S + Q + D + E) in hashing; what
-D-1684 removes per witness is the column evaluation and alignment, not that
-term. Then the authenticated Runner replay over its OOS bars and exit paths. Until
-D-1636 (W2-cli16-1) this paragraph called minting "proportional to the replay";
-D-1636 stated the per-witness Θ(S + Q + D + E) recomputation, and D-1684 then
-moved the column fold and alignment out of it. Full future V4 preflight/scheduling is at least O(P + C) before
+before D-1684 it was rebuilt for every witness. Since D-4782 (W2-cli3-3's
+second fix) a witness hashes no stream. The fold hashes the cohort identity,
+the source's three-stream data term and execution-slice digest once, and each
+witness seals its run against those digests through Runner's
+`ExecutionRunV1::with_digests`, proved equal to hashing per run by
+`runner::exit_grid_policy::sealing_against_hoisted_digests_equals_hashing_per_run`.
+Each witness still pays two cohort currency checks: the held strict source
+guards (O(M) metadata and receipt checks), the admitted root and the cached
+audit fields in O(1). So a witness is O(M) plus the authenticated Runner replay
+over its OOS bars and exit paths. Before D-4782 each witness re-derived the
+cohort identity twice, re-hashed the source's data identity and hashed all
+four streams again to seal its run, so a witness remained Θ(S + Q + D + E) in
+hashing; those streams are owned by the cohort and borrowed immutably by the
+fold, so that re-hash could not see a change, and a changed stored file is
+what the held guards refuse. The full identity re-derivation still runs once
+per fold. Until D-1636 (W2-cli16-1) this paragraph called minting "proportional
+to the replay"; D-1636 stated the per-witness Θ(S + Q + D + E) recomputation,
+D-1684 moved the column fold and alignment out of it, and D-4782 moved the
+hashing out of it. Full future V4 preflight/scheduling is at least O(P + C) before
 persistence. Explicit record ceilings refuse excess before allocation where
 the store header permits; they do not convert any whole operation into O(1).
 
-- 
2.43.0


From e9f40e30aa079f41c101596a10c45b773a948975 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:29:10 +0000
Subject: [PATCH 3/4] Carry one Candidate writer per run and hand the sizing
 census on (W2-cli3-4, G4-3, G4-2; D-4780, D-4783, D-4784)
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Three fixes that share one transaction path: the carried commit door and
the all-rung coordinator take both the run's Candidate writer and the
rung's sized NIFTY context, and one all-rung test proves both.

W2-cli3-4 (D-4780). Before: every production Candidate append opened the
ledger at the shared store root, walking and re-sealing every block, so a
`ledger-v6` or `ledger-all` run paid 16 x O(R_total + C). After: a run
carries one `CandidateLedgerWriterV1` from `run_route` or the all-rung
coordinator down to `append_and_reopen_with`. Its first append opens the
ledger; each later append runs `catch_up_locked` under the exclusive lock:
three stats when nothing changed, otherwise only the completions appended
since are verified against their seals from the handle's committed cursor,
after re-reading the last completion it indexed. A change that is not a
pure append, a moved last completion, a duplicate completion, or another
root or bounds is refused, and a refusal drops the held handle so the
next append opens afresh (D-1700 is the precedent). Limit, stated in
docs/06 §150: an append together with an in-place rewrite of an older
block is not seen by a carried writer; the next full open refuses it.

G4-3 (D-4783). Before: the sizing census built NIFTY's whole Candidate
signal column to read its swept count and dropped it, and the NIFTY
commit built the identical column again. After: `strict::census` builds it
once into a `PrebuiltSignalColumnV1` carried in `SizedNifty`; the source
consumes it only when evaluation inputs, rung and the three slices'
address and length all match, each refused by name, and derives only the
execution projection. Column digest, source id and universe id are
unchanged. A census of any underlying but NIFTY is refused.

G4-2 (D-4784). Before: `ledger-all` loaded and sized all eight NIFTY rungs
up front, and each NIFTY commit loaded again: 24 context loads per run.
After: each rung is sized at its Candidate phase and its context and
column are handed to that rung's NIFTY commit: 16 loads, one sized context
held at a time. Report text, stored bytes and run identity are unchanged;
the `rung sized` events now interleave with the population stage.

Tests (fail-before in brackets):
a_carried_writer_opens_once_and_verifies_what_others_appended_since
[1 scan against 0], a_carried_writer_refuses_what_it_cannot_catch_up_on,
a_carried_writer_catches_up_from_its_own_committed_cursor,
a_carried_writer_refuses_history_that_moved_and_duplicate_completions,
strict_v6_one_candidate_writer_serves_every_family_and_catches_up_on_others
[10 scans against 1, under its first form],
strict_v6_the_nifty_commit_consumes_the_sizing_column_once [2 builds
against 1], a_prebuilt_signal_column_serves_exactly_the_inputs_it_was_built_from,
all_rung_sizing_hands_each_nifty_context_to_its_commit_and_one_writer_serves_every_append
[24 loads against 16], and section_150 plus the ledger-v6 route test in
ledger_append_lookup_costs. Invariants L1FF-01..05, L1FF-09..11, L1FF-13.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/all_rung_population_v5.rs      | 126 ++-
 crates/cli/src/candidate_universe.rs          | 985 +++++++++++++++++-
 crates/cli/src/ledger_all.rs                  | 194 ++--
 crates/cli/src/ledger_v6.rs                   |   7 +
 crates/cli/src/step3_all_rung_tests.rs        |  99 +-
 crates/cli/src/step3_orchestrator.rs          | 133 ++-
 crates/cli/src/strict_v6_inputs.rs            | 114 +-
 crates/cli/src/strict_v6_tests.rs             | 178 +++-
 .../cli/tests/ledger_append_lookup_costs.rs   |  41 +-
 docs/04-invariants.md                         |   9 +
 docs/05-decisions.md                          | 137 +++
 docs/06-limits.md                             |  43 +-
 12 files changed, 1795 insertions(+), 271 deletions(-)

diff --git a/crates/cli/src/all_rung_population_v5.rs b/crates/cli/src/all_rung_population_v5.rs
index 2adcb535..b4c22be5 100644
--- a/crates/cli/src/all_rung_population_v5.rs
+++ b/crates/cli/src/all_rung_population_v5.rs
@@ -64,9 +64,9 @@ use crate::population_v5::{
 use crate::step3_orchestrator::{
     CommittedStoredCandidatePreAdmissionV1, StoredCandidatePreAdmissionBoundsV1,
     StoredCandidatePreAdmissionRequestV1, VerifiedBuildCommitV1,
-    commit_stored_candidate_pre_admission_authority_v1, commit_stored_observation_statistics_v2,
-    commit_stored_population_admission_v3, commit_stored_population_finalization_v3,
-    commit_stored_search_lineage_v4,
+    commit_stored_candidate_pre_admission_authority_carried_v1,
+    commit_stored_observation_statistics_v2, commit_stored_population_admission_v3,
+    commit_stored_population_finalization_v3, commit_stored_search_lineage_v4, strict::SizedNifty,
 };
 
 const RUNG_COUNT: usize = 8;
@@ -86,11 +86,49 @@ const _: () = {
     assert!(CANDIDATE_SIGNAL_RUNGS_SECONDS_V1[7] == 3_600);
 };
 
+/// Eight caller-built Candidate sweepers, one named field per canonical rung.
+///
+/// The eight named fields prevent positional array substitution.
+#[derive(Clone, Copy)]
+pub(crate) struct NamedAllRungSweepersV1<'a> {
+    /// One-minute Candidate sweeper.
+    pub(crate) one_minute: &'a Sweeper,
+    /// Two-minute Candidate sweeper.
+    pub(crate) two_minute: &'a Sweeper,
+    /// Three-minute Candidate sweeper.
+    pub(crate) three_minute: &'a Sweeper,
+    /// Five-minute Candidate sweeper.
+    pub(crate) five_minute: &'a Sweeper,
+    /// Ten-minute Candidate sweeper.
+    pub(crate) ten_minute: &'a Sweeper,
+    /// Fifteen-minute Candidate sweeper.
+    pub(crate) fifteen_minute: &'a Sweeper,
+    /// Thirty-minute Candidate sweeper.
+    pub(crate) thirty_minute: &'a Sweeper,
+    /// Sixty-minute Candidate sweeper.
+    pub(crate) sixty_minute: &'a Sweeper,
+}
+
+/// Sizes one canonical rung, by ordinal and name, on that rung's own NIFTY
+/// Candidate column: the sweeper and the loaded context and built column the
+/// rung's NIFTY commit consumes (G4-2, D-4784).
+pub(crate) type SizeRungV1<'a> = dyn Fn(usize, &str) -> Result<(Sweeper, SizedNifty), String> + 'a;
+
+/// Where each rung's Candidate sweeper comes from.
+#[derive(Clone, Copy)]
+pub(crate) enum AllRungSweepersV1<'a> {
+    /// Eight sweepers the caller built before the transaction.
+    Named(NamedAllRungSweepersV1<'a>),
+    /// Each rung is sized at the start of its own Candidate phase, and its
+    /// NIFTY commit consumes the sizing context and column instead of loading
+    /// and building them again (G4-2, D-4784).
+    SizedPerRung(&'a SizeRungV1<'a>),
+}
+
 /// Complete caller decisions for one canonical stored eight-rung transaction.
 ///
-/// The eight named Sweeper fields prevent positional array substitution.  The
-/// coordinator itself owns the only rung/family topology and reuses each named
-/// Sweeper for NIFTY then BANKNIFTY at that exact rung.
+/// The coordinator itself owns the only rung/family topology and reuses each
+/// rung's sweeper for NIFTY then BANKNIFTY at that exact rung.
 #[derive(Clone, Copy)]
 pub(crate) struct AllRungStoredPopulationV5Request<'a> {
     /// Existing exact stored-market and Candidate ledger root.
@@ -103,22 +141,8 @@ pub(crate) struct AllRungStoredPopulationV5Request<'a> {
     pub(crate) from: (u16, u8),
     /// Inclusive last requested `(year, month)`.
     pub(crate) to: (u16, u8),
-    /// One-minute Candidate sweeper.
-    pub(crate) one_minute_sweeper: &'a Sweeper,
-    /// Two-minute Candidate sweeper.
-    pub(crate) two_minute_sweeper: &'a Sweeper,
-    /// Three-minute Candidate sweeper.
-    pub(crate) three_minute_sweeper: &'a Sweeper,
-    /// Five-minute Candidate sweeper.
-    pub(crate) five_minute_sweeper: &'a Sweeper,
-    /// Ten-minute Candidate sweeper.
-    pub(crate) ten_minute_sweeper: &'a Sweeper,
-    /// Fifteen-minute Candidate sweeper.
-    pub(crate) fifteen_minute_sweeper: &'a Sweeper,
-    /// Thirty-minute Candidate sweeper.
-    pub(crate) thirty_minute_sweeper: &'a Sweeper,
-    /// Sixty-minute Candidate sweeper.
-    pub(crate) sixty_minute_sweeper: &'a Sweeper,
+    /// Every rung's Candidate sweeper.
+    pub(crate) sweepers: AllRungSweepersV1<'a>,
     /// Explicit forward outcome horizon shared by the one requested cohort.
     pub(crate) horizon: Horizon,
     /// Explicit indicator tolerance widths.
@@ -151,7 +175,7 @@ pub(crate) struct AllRungStoredPopulationV5Request<'a> {
     pub(crate) population_bounds: PopulationV5Bounds,
 }
 
-impl AllRungStoredPopulationV5Request<'_> {
+impl<'a> NamedAllRungSweepersV1<'a> {
     /// The sweeper named by one canonical rung ordinal, or `None` past the eighth.
     ///
     /// # It refuses rather than dying, and the arm it replaces could not be covered
@@ -164,20 +188,20 @@ impl AllRungStoredPopulationV5Request<'_> {
     /// no input could enter, which the 100% coverage floor in §9 cannot close: a
     /// panic no caller can provoke is a line no test can reach.
     ///
-    /// `None` is the honest answer instead. Both call sites turn it into a named
+    /// `None` is the honest answer instead. The call site turns it into a named
     /// refusal carrying the ordinal, so an authoring mistake in the phase-one walk
     /// surfaces as a message rather than an abort mid-transaction — which matters
     /// here more than usual, because an abort would strand a receipt-last append.
-    fn sweeper(&self, index: usize) -> Option<&Sweeper> {
+    fn sweeper(&self, index: usize) -> Option<&'a Sweeper> {
         match index {
-            0 => Some(self.one_minute_sweeper),
-            1 => Some(self.two_minute_sweeper),
-            2 => Some(self.three_minute_sweeper),
-            3 => Some(self.five_minute_sweeper),
-            4 => Some(self.ten_minute_sweeper),
-            5 => Some(self.fifteen_minute_sweeper),
-            6 => Some(self.thirty_minute_sweeper),
-            7 => Some(self.sixty_minute_sweeper),
+            0 => Some(self.one_minute),
+            1 => Some(self.two_minute),
+            2 => Some(self.three_minute),
+            3 => Some(self.five_minute),
+            4 => Some(self.ten_minute),
+            5 => Some(self.fifteen_minute),
+            6 => Some(self.thirty_minute),
+            7 => Some(self.sixty_minute),
             _ => None,
         }
     }
@@ -570,18 +594,32 @@ pub(crate) fn commit_all_rung_with_verified_build_v5(
     // Phase one freezes every shared-root Candidate/Base/Pre-Admission append
     // before any successor ledger reader is retained.
     let mut candidate_pairs = Vec::with_capacity(RUNG_COUNT);
+    // ONE CANDIDATE WRITER FOR ALL SIXTEEN APPENDS (W2-cli3-4, D-4780): the
+    // Candidate ledger is the source root's, shared by every run.
+    let mut candidate_writer = crate::candidate_universe::CandidateLedgerWriterV1::new();
     for (index, rung_name) in CANONICAL_RUNG_NAMES_V1.iter().copied().enumerate() {
-        // NAMED, NOT ASSUMED. `CANONICAL_RUNG_NAMES_V1` is `RUNG_COUNT` long and
-        // `sweeper` answers every ordinal below it, so this refusal is unreachable
-        // on the shipped tables -- but it is a refusal and not a panic, for the
-        // reason [`AllRungStoredPopulationV5Request::sweeper`] records.
-        let sweeper = request.sweeper(index).ok_or_else(|| {
-            format!("all-rung ordinal {index} ({rung_name}) names no canonical sweeper")
-        })?;
+        let sized_sweeper: Sweeper;
+        let (sweeper, sized) = match request.sweepers {
+            // NAMED, NOT ASSUMED. `CANONICAL_RUNG_NAMES_V1` is `RUNG_COUNT` long
+            // and `sweeper` answers every ordinal below it, so this refusal is
+            // unreachable on the shipped tables -- but it is a refusal and not a
+            // panic, for the reason [`NamedAllRungSweepersV1::sweeper`] records.
+            AllRungSweepersV1::Named(named) => (
+                named.sweeper(index).ok_or_else(|| {
+                    format!("all-rung ordinal {index} ({rung_name}) names no canonical sweeper")
+                })?,
+                None,
+            ),
+            AllRungSweepersV1::SizedPerRung(size) => {
+                let (sweeper, sized) = size(index, rung_name)?;
+                sized_sweeper = sweeper;
+                (&sized_sweeper, Some(sized))
+            }
+        };
         roots.require_same(&format!(
             "before {rung_name} NIFTY Candidate/Pre-Admission commit"
         ))?;
-        let nifty = commit_stored_candidate_pre_admission_authority_v1(
+        let nifty = commit_stored_candidate_pre_admission_authority_carried_v1(
             StoredCandidatePreAdmissionRequestV1 {
                 root: roots.source.path(),
                 vendor: request.vendor,
@@ -599,13 +637,15 @@ pub(crate) fn commit_all_rung_with_verified_build_v5(
                 bounds: request.candidate_bounds,
             },
             verified_commit,
+            &mut candidate_writer,
+            sized,
         )
         .map_err(|why| format!("all-rung {rung_name} NIFTY refused: {why}"))?;
 
         roots.require_same(&format!(
             "between {rung_name} NIFTY and BANKNIFTY Candidate/Pre-Admission commits"
         ))?;
-        let banknifty = commit_stored_candidate_pre_admission_authority_v1(
+        let banknifty = commit_stored_candidate_pre_admission_authority_carried_v1(
             StoredCandidatePreAdmissionRequestV1 {
                 root: roots.source.path(),
                 vendor: request.vendor,
@@ -623,6 +663,8 @@ pub(crate) fn commit_all_rung_with_verified_build_v5(
                 bounds: request.candidate_bounds,
             },
             verified_commit,
+            &mut candidate_writer,
+            None,
         )
         .map_err(|why| format!("all-rung {rung_name} BANKNIFTY refused: {why}"))?;
         roots.require_same(&format!(
diff --git a/crates/cli/src/candidate_universe.rs b/crates/cli/src/candidate_universe.rs
index 4c84365c..4929c130 100644
--- a/crates/cli/src/candidate_universe.rs
+++ b/crates/cli/src/candidate_universe.rs
@@ -13,10 +13,12 @@
 //! audit lookup and one row seek are O(1) in record count (plus bounded,
 //! metadata-only file generation checks); a page is O(page length), and an
 //! append on an already-open handle is O(new rows). The production append door
-//! opens the ledger once per call, so one production append costs
-//! O(rows + receipts) for that open plus O(new rows) to write the block and
-//! re-read it from disk (D-1680). No whole-ledger operation is described as
-//! O(1).
+//! appends through one writer a run carries across all its appends: its first
+//! append opens the ledger, so it costs O(rows + receipts) for that open plus
+//! O(new rows) to write the block and re-read it from disk (D-1680), and every
+//! later append of the run catches the held handle up on what other writers
+//! appended since, O(new rows + foreign new rows), instead of opening again
+//! (W2-cli3-4, D-4780). No whole-ledger operation is described as O(1).
 //!
 //! Crate-internal production preparation is available only through
 //! [`CandidateUniverseProductionSourceV1`]. That opaque source bundle rebuilds
@@ -852,6 +854,7 @@ impl<'a> CandidateUniverseProductionSourceV1<'a> {
         signal_load_bound: StoredSpanLoadBoundV1,
         minute_load_bound: StoredSpanLoadBoundV1,
         daily_load_bound: StoredSpanLoadBoundV1,
+        prebuilt_signal: Option<PrebuiltSignalColumnV1>,
     ) -> Result<Self, CandidateUniverseRefusal> {
         require_rung(rung_seconds)?;
         require_canonical_daily_reference(daily_references, daily_reference)?;
@@ -908,14 +911,31 @@ impl<'a> CandidateUniverseProductionSourceV1<'a> {
             availability,
             thresholds,
         };
-        let (signal_column, execution_column) = build_candidate_columns(
-            signal_bars,
-            daily_references,
-            reference_minute_context,
-            execution_series.bars(),
-            rung_seconds,
-            &evaluation,
-        )?;
+        let (signal_column, execution_column) = match prebuilt_signal {
+            // CONSUMED, NOT REBUILT (G4-3, D-4783): the sizing census built
+            // this exact column from these exact slices; only the execution
+            // projection is derived here.
+            Some(prebuilt) => project_candidate_execution_column(
+                signal_bars,
+                prebuilt.into_column_for(
+                    signal_bars,
+                    daily_references,
+                    reference_minute_context,
+                    rung_seconds,
+                    &evaluation,
+                )?,
+                execution_series.bars(),
+                rung_seconds,
+            )?,
+            None => build_candidate_columns(
+                signal_bars,
+                daily_references,
+                reference_minute_context,
+                execution_series.bars(),
+                rung_seconds,
+                &evaluation,
+            )?,
+        };
 
         let data_digest = runner::identity::data_digest_with_daily_reference(
             signal_bars,
@@ -1498,7 +1518,7 @@ fn derive_global_replay_oos_source_id(source: &CandidateGlobalReplayOosSourceV1<
     hasher.finalize()
 }
 
-#[derive(Clone, Copy, Debug)]
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub(crate) struct CandidateEvaluationInputsV1 {
     pub(crate) widths: Widths,
     pub(crate) availability: Availability,
@@ -1626,13 +1646,24 @@ pub(crate) fn build_candidate_columns(
     rung_seconds: u32,
     evaluation: &CandidateEvaluationInputsV1,
 ) -> Result<(Column, Column), CandidateUniverseRefusal> {
-    let signal_column = build_candidate_signal_column(
+    let signal_column = build_full_candidate_signal_column(
         signal_bars,
         daily_references,
         reference_minute_context,
         rung_seconds,
         evaluation,
     )?;
+    project_candidate_execution_column(signal_bars, signal_column, execution_bars, rung_seconds)
+}
+
+/// The checked one-minute execution projection of one complete signal
+/// column, returned beside it. O(signal + execution).
+fn project_candidate_execution_column(
+    signal_bars: &[Candle],
+    signal_column: Column,
+    execution_bars: &[Candle],
+    rung_seconds: u32,
+) -> Result<(Column, Column), CandidateUniverseRefusal> {
     let signal_length_micros = signal_length_micros(rung_seconds)?;
     let alignment = runner::align::onto_execution(
         signal_bars,
@@ -1656,24 +1687,27 @@ pub(crate) fn build_candidate_columns(
     Ok((signal_column, execution_column))
 }
 
-/// The rows Candidate production's signal column sweeps over these inputs,
-/// read from that column's own census, warm-up excluded (D-2103).
-///
-/// This runs the one builder below, so a ledger sizing its support on it asks
-/// exactly the fold its Candidate commit will sweep; there is no second count.
-/// O(signal + daily + minute), one column build. **UNVERIFIED as a measured
-/// bound**; read off the source.
-///
-/// # Errors
-///
-/// Every refusal of that build.
-pub(crate) fn candidate_signal_swept_v1(
+#[cfg(test)]
+thread_local! {
+    /// Test-only count of complete Candidate signal-column builds on this
+    /// thread: sizing censuses and source constructions, not Search V4's
+    /// causal prefixes (D-4783).
+    pub(crate) static FULL_SIGNAL_COLUMN_BUILDS: std::cell::Cell<u64> =
+        const { std::cell::Cell::new(0) };
+}
+
+/// [`build_candidate_signal_column`] over a complete span: the one build a
+/// sizing census and a source construction share. O(signal + daily +
+/// minute). **UNVERIFIED as a measured bound**; read off the source.
+fn build_full_candidate_signal_column(
     signal_bars: &[Candle],
     daily_references: &[DailyReference],
     reference_minute_context: &[Candle],
     rung_seconds: u32,
     evaluation: &CandidateEvaluationInputsV1,
-) -> Result<u64, CandidateUniverseRefusal> {
+) -> Result<Column, CandidateUniverseRefusal> {
+    #[cfg(test)]
+    FULL_SIGNAL_COLUMN_BUILDS.with(|count| count.set(count.get().saturating_add(1)));
     build_candidate_signal_column(
         signal_bars,
         daily_references,
@@ -1681,7 +1715,121 @@ pub(crate) fn candidate_signal_swept_v1(
         rung_seconds,
         evaluation,
     )
-    .map(|column| column.census().swept)
+}
+
+/// A complete Candidate signal column a ledger's sizing census built, carried
+/// to the same rung's NIFTY commit so that commit does not build it again
+/// (G4-3, D-4783).
+///
+/// It remembers the evaluation inputs and rung it was built under and the
+/// address and length of the three slices it was built from. A source
+/// construction accepts it only for exactly those: the context that owns the
+/// slices is moved, never copied, between the census and the commit, so its
+/// buffers keep their addresses, and nothing can mutate them in between. Any
+/// difference refuses by name; nothing falls back to a rebuild.
+#[derive(Debug)]
+pub(crate) struct PrebuiltSignalColumnV1 {
+    column: Column,
+    evaluation: CandidateEvaluationInputsV1,
+    rung_seconds: u32,
+    inputs: [(usize, usize); 3],
+}
+
+impl PrebuiltSignalColumnV1 {
+    /// Builds the column once. O(signal + daily + minute), the same build a
+    /// source construction runs.
+    ///
+    /// # Errors
+    ///
+    /// Every refusal of that build.
+    pub(crate) fn build(
+        signal_bars: &[Candle],
+        daily_references: &[DailyReference],
+        reference_minute_context: &[Candle],
+        rung_seconds: u32,
+        evaluation: CandidateEvaluationInputsV1,
+    ) -> Result<Self, CandidateUniverseRefusal> {
+        let column = build_full_candidate_signal_column(
+            signal_bars,
+            daily_references,
+            reference_minute_context,
+            rung_seconds,
+            &evaluation,
+        )?;
+        Ok(Self {
+            column,
+            evaluation,
+            rung_seconds,
+            inputs: slice_identities(signal_bars, daily_references, reference_minute_context),
+        })
+    }
+
+    /// The rows the column sweeps, warm-up excluded (D-2103), read from the
+    /// column's own census.
+    pub(crate) fn swept(&self) -> u64 {
+        self.column.census().swept
+    }
+
+    /// The evaluation inputs the column was built under.
+    pub(crate) const fn evaluation(&self) -> CandidateEvaluationInputsV1 {
+        self.evaluation
+    }
+
+    /// The column, for exactly the inputs it was built from. O(1): three
+    /// separate comparisons, each refusing by name, and no bar is read; see
+    /// `cli::candidate_universe::tests::a_prebuilt_signal_column_serves_exactly_the_inputs_it_was_built_from`.
+    ///
+    /// # Errors
+    ///
+    /// Other evaluation inputs, another rung, or slices at another address or
+    /// of another length than the ones the column was built from.
+    fn into_column_for(
+        self,
+        signal_bars: &[Candle],
+        daily_references: &[DailyReference],
+        reference_minute_context: &[Candle],
+        rung_seconds: u32,
+        evaluation: &CandidateEvaluationInputsV1,
+    ) -> Result<Column, CandidateUniverseRefusal> {
+        if self.evaluation != *evaluation {
+            return Err(
+                "the sizing-built Candidate signal column was built under other evaluation inputs"
+                    .to_owned(),
+            );
+        }
+        if self.rung_seconds != rung_seconds {
+            return Err(format!(
+                "the sizing-built Candidate signal column was built for rung {}s, not {rung_seconds}s",
+                self.rung_seconds
+            ));
+        }
+        if self.inputs != slice_identities(signal_bars, daily_references, reference_minute_context)
+        {
+            return Err(
+                "the sizing-built Candidate signal column was built from other bars than this source's"
+                    .to_owned(),
+            );
+        }
+        Ok(self.column)
+    }
+}
+
+/// The address and length of each slice, compared in O(1) without reading a
+/// bar; see
+/// `cli::candidate_universe::tests::a_prebuilt_signal_column_serves_exactly_the_inputs_it_was_built_from`.
+fn slice_identities(
+    signal_bars: &[Candle],
+    daily_references: &[DailyReference],
+    reference_minute_context: &[Candle],
+) -> [(usize, usize); 3] {
+    [
+        (signal_bars.as_ptr().addr(), signal_bars.len()),
+        (daily_references.as_ptr().addr(), daily_references.len()),
+        (
+            reference_minute_context.as_ptr().addr(),
+            reference_minute_context.len(),
+        ),
+    ]
 }
 
 /// Builds one exact anchored signal column from the typed daily reference and
@@ -2708,15 +2856,36 @@ impl<'a> ProducedCandidateUniverseV1<'a> {
     ///
     /// Returns any receipt-last append, exact-retry, corruption, stale-path,
     /// bounds, I/O or reopen mismatch refusal. No partial audit is returned.
+    #[cfg(test)]
     pub(crate) fn append_and_reopen(
         &self,
         root: impl AsRef<Path>,
         bounds: CandidateUniverseBoundsV1,
+    ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
+        self.append_and_reopen_with(&mut CandidateLedgerWriterV1::new(), root.as_ref(), bounds)
+    }
+
+    /// Commits the complete block through a writer the caller carries across
+    /// a run, then re-reads and re-seals that block and its receipt from disk
+    /// through the same handle, before returning success. The ledger is
+    /// opened once per run rather than once per append (W2-cli3-4, D-4780).
+    ///
+    /// # Errors
+    ///
+    /// Returns any receipt-last append, exact-retry, catch-up, corruption,
+    /// stale-path, bounds, I/O or reopen mismatch refusal, and a carried
+    /// writer that was opened for another root or other bounds. No partial
+    /// audit is returned.
+    pub(crate) fn append_and_reopen_with(
+        &self,
+        writer: &mut CandidateLedgerWriterV1,
+        root: &Path,
+        bounds: CandidateUniverseBoundsV1,
     ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
         self.base_evidence
             .validate_candidate_receipt(&self.prepared.receipt)
             .map_err(|why| why.to_string())?;
-        append_produced_candidate_universe_v1(root, bounds, self)
+        append_produced_candidate_universe_v1(writer, root, bounds, self)
     }
 
     /// Persists the same-pass Base records only after the exact Candidate
@@ -3201,6 +3370,8 @@ pub(crate) fn verify_population_v5_canonical_record(
     verify_population_candidate_canonical_record_v1(canonical_record)
 }
 
+#[cfg(test)]
+pub(crate) use tests::append_foreign_fixture_universe;
 #[cfg(test)]
 pub(crate) use tests::population_v5_test_canonical_candidate_record_for_identity;
 
@@ -3263,6 +3434,9 @@ pub struct CandidateUniverseLedgerV1 {
     audits: HashMap<[u8; 32], CandidateUniverseReopenAuditV1>,
     orphan: Option<OrphanBlockV1>,
     total_rows: u64,
+    /// The last completion this handle indexed, re-read before a catch-up
+    /// trusts the history below it (W2-cli3-4, D-4780).
+    last_receipt: Option<CandidateUniverseReceiptV1>,
     lock_generation: FileGenerationV1,
     row_generation: FileGenerationV1,
     receipt_generation: FileGenerationV1,
@@ -3304,6 +3478,10 @@ impl CandidateUniverseLedgerV1 {
         writable: bool,
     ) -> Result<Self, CandidateUniverseRefusal> {
         let (row_path, receipt_path, lock_path) = candidate_ledger_paths(root)?;
+        #[cfg(test)]
+        if writable {
+            LEDGER_WRITER_OPENS.with(|count| count.set(count.get().saturating_add(1)));
+        }
         let writer_lock = open_file(&lock_path, writable, writable)?;
         // The open lock is released by name on success and by the guard's
         // explicit unlock on every refusal, never by closing a descriptor: the
@@ -3378,6 +3556,7 @@ impl CandidateUniverseLedgerV1 {
                 audits: HashMap::new(),
                 orphan: None,
                 total_rows: 0,
+                last_receipt: None,
                 lock_generation,
                 row_generation,
                 receipt_generation,
@@ -3433,6 +3612,7 @@ impl CandidateUniverseLedgerV1 {
             .try_reserve(capacity)
             .map_err(|why| format!("cannot reserve candidate completion audit index: {why}"))?;
         let mut committed = 0_u64;
+        let mut last = None;
         for index in 0..receipt_count {
             let receipt = read_receipt(&mut self.receipt_file, index)?;
             let end = committed
@@ -3455,10 +3635,12 @@ impl CandidateUniverseLedgerV1 {
                     hex32(receipt.universe_id())
                 ));
             }
+            last = Some(receipt);
             committed = end;
         }
         self.orphan = scan_orphan(&mut self.row_file, committed, total_rows)?;
         self.total_rows = total_rows;
+        self.last_receipt = last;
         self.row_generation = file_generation(&self.row_file, &self.row_path)?;
         self.receipt_generation = file_generation(&self.receipt_file, &self.receipt_path)?;
         Ok(())
@@ -3762,7 +3944,10 @@ impl CandidateUniverseLedgerV1 {
         &mut self,
         prepared: &PreparedCandidateUniverseV1,
     ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
-        self.require_unchanged()?;
+        // CAUGHT UP, NOT REOPENED (W2-cli3-4, D-4780): completions another
+        // writer appended since this handle last measured the files are
+        // verified here, block by block; nothing older is read again.
+        self.catch_up_locked()?;
         let receipt = prepared.receipt;
         receipt.validate()?;
         if receipt.row_count > self.bounds.max_rows {
@@ -3847,6 +4032,7 @@ impl CandidateUniverseLedgerV1 {
         )?;
         let audit = CandidateUniverseReopenAuditV1 { first_row, receipt };
         self.audits.insert(receipt.universe_id(), audit);
+        self.last_receipt = Some(receipt);
         self.orphan = None;
         self.receipt_generation = file_generation(&self.receipt_file, &self.receipt_path)?;
         Ok(CandidateUniverseProductionCommitV1::Written(audit))
@@ -3937,6 +4123,156 @@ impl CandidateUniverseLedgerV1 {
         Ok(audit)
     }
 
+    /// Brings this handle up to the files as they stand, under the exclusive
+    /// writer lock, before an append (W2-cli3-4, D-4780; D-1700 is the
+    /// precedent).
+    ///
+    /// Unchanged generations cost three stats. A ledger another writer has
+    /// appended to since this handle last measured it is caught up by
+    /// [`Self::absorb_foreign_completions`], which reads only the completions
+    /// past the indexed count and the blocks they name. Every other change
+    /// refuses: a replaced path, a change that added no completion, or a last
+    /// indexed completion that is no longer the one this handle indexed.
+    fn catch_up_locked(&mut self) -> Result<(), CandidateUniverseRefusal> {
+        require_generation(self.lock_generation, &self.writer_lock, &self.lock_path)?;
+        let rows_now = file_generation(&self.row_file, &self.row_path)?;
+        let receipts_now = file_generation(&self.receipt_file, &self.receipt_path)?;
+        if rows_now == self.row_generation && receipts_now == self.receipt_generation {
+            return Ok(());
+        }
+        self.absorb_foreign_completions(rows_now, receipts_now)
+    }
+
+    /// Indexes the completions another writer appended since this handle
+    /// last measured the files, verifying each one's block against its seal,
+    /// then re-scans the orphan tail after them. O(new completions + new
+    /// rows); the history below the indexed count is not read again.
+    ///
+    /// That is the honest limit (D-4780): a metadata generation cannot tell
+    /// an append from an append made together with an in-place rewrite of an
+    /// older block, so such a rewrite is not seen here. The last indexed
+    /// completion is re-read as a witness, and the next full open of the
+    /// ledger re-validates every block and refuses the rewrite.
+    ///
+    /// Nothing is indexed until every new completion has verified, so a
+    /// refusal leaves the handle as it was; the carried writer then discards
+    /// it all the same.
+    fn absorb_foreign_completions(
+        &mut self,
+        rows_now: FileGenerationV1,
+        receipts_now: FileGenerationV1,
+    ) -> Result<(), CandidateUniverseRefusal> {
+        let total_rows = record_count(
+            &self.row_file,
+            CANDIDATE_ROW_STRIDE_V1,
+            self.bounds.max_rows,
+            "candidate rows",
+        )?;
+        let receipt_count = record_count(
+            &self.receipt_file,
+            CANDIDATE_RECEIPT_STRIDE_V1,
+            self.bounds.max_universes,
+            "candidate completions",
+        )?;
+        let indexed = u64::try_from(self.audits.len())
+            .map_err(|_| "candidate index size does not fit u64".to_owned())?;
+        if receipt_count <= indexed {
+            return Err(format!(
+                "candidate ledger changed since the carried Candidate writer indexed it, but holds {receipt_count} completions against the {indexed} indexed: only appended completions are caught up, so this refuses"
+            ));
+        }
+        if let Some(last) = indexed.checked_sub(1)
+            && Some(read_receipt(&mut self.receipt_file, last)?) != self.last_receipt
+        {
+            return Err(format!(
+                "candidate completion {last} is no longer the one the carried Candidate writer indexed; the history below a catch-up changed, so this refuses"
+            ));
+        }
+        let mut committed = self
+            .orphan
+            .map_or(self.total_rows, |orphan| orphan.first_row);
+        let mut fresh = HashMap::new();
+        fresh
+            .try_reserve(
+                usize::try_from(receipt_count - indexed)
+                    .map_err(|_| "candidate catch-up count does not fit usize".to_owned())?,
+            )
+            .map_err(|why| format!("cannot reserve candidate catch-up index: {why}"))?;
+        let mut last = None;
+        for index in indexed..receipt_count {
+            let receipt = read_receipt(&mut self.receipt_file, index)?;
+            let end = committed
+                .checked_add(receipt.row_count)
+                .ok_or_else(|| "candidate catch-up row cursor overflowed u64".to_owned())?;
+            if end > total_rows {
+                return Err(format!(
+                    "candidate receipt {} commits rows {committed}..{end}, beyond physical total {total_rows}",
+                    hex32(receipt.universe_id())
+                ));
+            }
+            validate_file_block(&mut self.row_file, committed, &receipt)?;
+            if self.audits.contains_key(&receipt.universe_id()) {
+                return Err(format!(
+                    "candidate universe {} has more than one completion receipt",
+                    hex32(receipt.universe_id())
+                ));
+            }
+            let audit = CandidateUniverseReopenAuditV1 {
+                first_row: committed,
+                receipt,
+            };
+            if fresh.insert(receipt.universe_id(), audit).is_some() {
+                return Err(format!(
+                    "candidate universe {} has more than one completion receipt",
+                    hex32(receipt.universe_id())
+                ));
+            }
+            last = Some(receipt);
+            committed = end;
+        }
+        let orphan = scan_orphan(&mut self.row_file, committed, total_rows)?;
+        #[cfg(test)]
+        LEDGER_CATCH_UP_RECEIPTS.with(|count| {
+            count.set(
+                count
+                    .get()
+                    .saturating_add(u64::try_from(fresh.len()).unwrap_or(u64::MAX)),
+            );
+        });
+        self.audits.extend(fresh);
+        self.last_receipt = last;
+        self.orphan = orphan;
+        self.total_rows = total_rows;
+        self.row_generation = rows_now;
+        self.receipt_generation = receipts_now;
+        Ok(())
+    }
+
+    /// Refuses an append for any root or bounds other than the ones this
+    /// handle opened, so a carried writer cannot write one run's universe
+    /// into another ledger (W2-cli3-4, D-4780).
+    fn require_serves(
+        &self,
+        root: &Path,
+        bounds: CandidateUniverseBoundsV1,
+    ) -> Result<(), CandidateUniverseRefusal> {
+        let (row_path, _, _) = candidate_ledger_paths(root)?;
+        if row_path != self.row_path {
+            return Err(format!(
+                "the carried Candidate writer holds {}, not {}",
+                self.row_path.display(),
+                row_path.display()
+            ));
+        }
+        if bounds != self.bounds {
+            return Err(format!(
+                "the carried Candidate writer was opened with bounds {:?}, not this append's {bounds:?}",
+                self.bounds
+            ));
+        }
+        Ok(())
+    }
+
     fn require_unchanged(&self) -> Result<(), CandidateUniverseRefusal> {
         require_generation(self.lock_generation, &self.writer_lock, &self.lock_path)?;
         require_generation(self.row_generation, &self.row_file, &self.row_path)?;
@@ -3977,27 +4313,89 @@ fn candidate_ledger_paths(
 }
 
 fn append_produced_candidate_universe_v1(
-    root: impl AsRef<Path>,
+    writer: &mut CandidateLedgerWriterV1,
+    root: &Path,
     bounds: CandidateUniverseBoundsV1,
     produced: &ProducedCandidateUniverseV1<'_>,
 ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
-    append_prepared_and_reverify(root.as_ref(), bounds, &produced.prepared)
+    writer.append_and_reverify(root, bounds, &produced.prepared)
+}
+
+/// One Candidate ledger writer carried across every append of one run
+/// (W2-cli3-4, D-4780).
+///
+/// The first append opens the ledger, which validates every stored block
+/// once: O(R + C). Every later append catches the held handle up instead,
+/// under the exclusive lock, verifying only the completions another writer
+/// appended since (`CandidateUniverseLedgerV1::absorb_foreign_completions`),
+/// then appends and re-reads its own block. One run therefore pays one
+/// O(R + C) open plus O(new rows) per append, where it paid one open per
+/// append. Any refusal discards the held handle, so the next append opens
+/// the ledger again rather than trusting a handle that refused.
+#[derive(Debug, Default)]
+pub(crate) struct CandidateLedgerWriterV1 {
+    ledger: Option<CandidateUniverseLedgerV1>,
+}
+
+impl CandidateLedgerWriterV1 {
+    /// A writer that has not opened its ledger yet.
+    #[must_use]
+    pub(crate) const fn new() -> Self {
+        Self { ledger: None }
+    }
+
+    fn append_and_reverify(
+        &mut self,
+        root: &Path,
+        bounds: CandidateUniverseBoundsV1,
+        prepared: &PreparedCandidateUniverseV1,
+    ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
+        let result = self.append_with_held_or_opened(root, bounds, prepared);
+        if result.is_err() {
+            self.ledger = None;
+        }
+        result
+    }
+
+    fn append_with_held_or_opened(
+        &mut self,
+        root: &Path,
+        bounds: CandidateUniverseBoundsV1,
+        prepared: &PreparedCandidateUniverseV1,
+    ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
+        if let Some(ledger) = self.ledger.as_mut() {
+            ledger.require_serves(root, bounds)?;
+            return commit_and_reverify(ledger, prepared);
+        }
+        let ledger = self
+            .ledger
+            .insert(CandidateUniverseLedgerV1::open(root, bounds)?);
+        commit_and_reverify(ledger, prepared)
+    }
 }
 
-/// One production append: one full open, the append, then a re-read of only
-/// the committed block through the same handle. O(R + C) for the open plus
-/// O(new rows); before D-1680 a second full `open_read` made it
-/// 2 x O(R + C) + O(new rows).
+/// One stand-alone production append: one full open, then
+/// [`commit_and_reverify`]. O(R + C) for the open plus O(new rows); before
+/// D-1680 a second full `open_read` made it 2 x O(R + C) + O(new rows).
+#[cfg(test)]
 fn append_prepared_and_reverify(
     root: &Path,
     bounds: CandidateUniverseBoundsV1,
     prepared: &PreparedCandidateUniverseV1,
 ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
     let mut ledger = CandidateUniverseLedgerV1::open(root, bounds)?;
+    commit_and_reverify(&mut ledger, prepared)
+}
+
+/// The append, then a re-read of only the committed block through the same
+/// handle (D-1680), then the exact comparison with the prepared receipt.
+fn commit_and_reverify(
+    ledger: &mut CandidateUniverseLedgerV1,
+    prepared: &PreparedCandidateUniverseV1,
+) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
     let committed = ledger.append_complete(prepared)?;
     let expected = committed.audit();
     let reopened = ledger.reverify_committed(&committed)?;
-    drop(ledger);
     // TWO CHECKS, NOT ONE `||` (G18-cli-a-05, D-2004). Either inequality alone
     // refuses; a joined guard let a mutant require both, and no honest fixture
     // can make the committed block reopen differently from its own audit. Each
@@ -4972,7 +5370,24 @@ pub(crate) fn note_oos_stream_hash() {
 thread_local! {
     /// Test-only count of `CandidateUniverseLedgerV1::scan` calls (one per
     /// full open) on this thread.
-    static LEDGER_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+    pub(crate) static LEDGER_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
+}
+
+#[cfg(test)]
+thread_local! {
+    /// Test-only count of writable Candidate ledger opens on this thread: the
+    /// opens a production append pays, not the read-only reopens successor
+    /// authentication makes (D-4780).
+    pub(crate) static LEDGER_WRITER_OPENS: std::cell::Cell<u64> =
+        const { std::cell::Cell::new(0) };
+}
+
+#[cfg(test)]
+thread_local! {
+    /// Test-only count of foreign completions a carried Candidate writer
+    /// verified while catching up before an append, on this thread (D-4780).
+    pub(crate) static LEDGER_CATCH_UP_RECEIPTS: std::cell::Cell<u64> =
+        const { std::cell::Cell::new(0) };
 }
 
 #[cfg(test)]
@@ -6902,6 +7317,15 @@ mod tests {
         );
     }
 
+    /// Appends one fixture universe through its own full open, as another
+    /// process's writer would (D-4780).
+    pub(crate) fn append_foreign_fixture_universe(
+        root: &Path,
+        bounds: CandidateUniverseBoundsV1,
+    ) -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> {
+        append_prepared_and_reverify(root, bounds, &prepared(97))
+    }
+
     pub(crate) fn population_v5_test_canonical_candidate_record() -> [u8; ROW_STRIDE_BYTES] {
         let raw = prepared(32).rows[0]
             .record()
@@ -7627,6 +8051,14 @@ mod tests {
         }
 
         fn source(&self) -> CandidateUniverseProductionSourceV1<'_> {
+            self.source_with(None)
+                .expect("all production fixture sources agree")
+        }
+
+        fn source_with(
+            &self,
+            prebuilt: Option<PrebuiltSignalColumnV1>,
+        ) -> Result<CandidateUniverseProductionSourceV1<'_>, CandidateUniverseRefusal> {
             CandidateUniverseProductionSourceV1::new(
                 InstrumentFamilyV1::Nifty,
                 60,
@@ -7656,11 +8088,100 @@ mod tests {
                     u64::try_from(self.daily_bars.len()).expect("daily length fits u64"),
                 )
                 .expect("daily load ceiling is nonzero"),
+                prebuilt,
+            )
+        }
+
+        fn evaluation() -> CandidateEvaluationInputsV1 {
+            CandidateEvaluationInputsV1 {
+                widths: Widths::pinned().expect("fixture uses measured widths"),
+                availability: Availability::Absent,
+                thresholds: Thresholds::CLASSICAL,
+            }
+        }
+
+        fn prebuilt(&self) -> PrebuiltSignalColumnV1 {
+            PrebuiltSignalColumnV1::build(
+                self.execution(),
+                &self.daily_references,
+                &self.context,
+                60,
+                Self::evaluation(),
             )
-            .expect("all production fixture sources agree")
+            .expect("the fixture column builds")
         }
     }
 
+    #[test]
+    fn a_prebuilt_signal_column_serves_exactly_the_inputs_it_was_built_from() {
+        // G4-3 / D-4783: the NIFTY commit built the whole Candidate signal
+        // column the sizing census had just built and dropped.
+        let fixture = ProductionFixture::new();
+        let reference = fixture.source();
+        let prebuilt = fixture.prebuilt();
+        FULL_SIGNAL_COLUMN_BUILDS.with(|count| count.set(0));
+        let sourced = fixture
+            .source_with(Some(prebuilt))
+            .expect("a column serves the exact inputs it was built from");
+        assert_eq!(
+            FULL_SIGNAL_COLUMN_BUILDS.with(std::cell::Cell::get),
+            0,
+            "the source consumed the prebuilt column and built none"
+        );
+        assert_eq!(sourced.source_id, reference.source_id);
+        assert_eq!(sourced.signal_column_digest, reference.signal_column_digest);
+        assert_eq!(
+            sourced.execution_column_digest,
+            reference.execution_column_digest
+        );
+
+        let evaluation = ProductionFixture::evaluation();
+        let other = CandidateEvaluationInputsV1 {
+            availability: Availability::Present,
+            ..evaluation
+        };
+        assert!(
+            fixture
+                .prebuilt()
+                .into_column_for(
+                    fixture.execution(),
+                    &fixture.daily_references,
+                    &fixture.context,
+                    60,
+                    &other,
+                )
+                .expect_err("other evaluation inputs must refuse")
+                .contains("other evaluation inputs")
+        );
+        assert!(
+            fixture
+                .prebuilt()
+                .into_column_for(
+                    fixture.execution(),
+                    &fixture.daily_references,
+                    &fixture.context,
+                    120,
+                    &evaluation,
+                )
+                .expect_err("another rung must refuse")
+                .contains("not 120s")
+        );
+        let copied = fixture.execution().to_vec();
+        assert!(
+            fixture
+                .prebuilt()
+                .into_column_for(
+                    &copied,
+                    &fixture.daily_references,
+                    &fixture.context,
+                    60,
+                    &evaluation,
+                )
+                .expect_err("equal bars at another address must refuse")
+                .contains("other bars")
+        );
+    }
+
     fn minute_bars(first_day: i64, last_day: i64) -> Vec<Candle> {
         let mut bars = Vec::new();
         let mut ordinal = 0_i64;
@@ -9354,6 +9875,392 @@ mod tests {
         );
     }
 
+    /// Flips one fact byte of physical row `row` and syncs it.
+    fn flip_candidate_row_fact(root: &Path, row: u64) {
+        let path = root.join(ROW_FILE);
+        let mut file = open_file(&path, true, false).expect("row file reopens");
+        let offset = HEADER_BYTES_V1 + row * CANDIDATE_ROW_STRIDE_V1 + 248;
+        let mut byte = [0_u8; 1];
+        file.seek(SeekFrom::Start(offset))
+            .and_then(|_| file.read_exact(&mut byte))
+            .expect("fact byte reads");
+        byte[0] ^= 1;
+        file.seek(SeekFrom::Start(offset))
+            .and_then(|_| file.write_all(&byte))
+            .and_then(|()| file.sync_data())
+            .expect("fact mutation persists");
+    }
+
+    #[test]
+    fn a_carried_writer_opens_once_and_verifies_what_others_appended_since() {
+        // W2-cli3-4 / D-4780: before, every production append opened the whole
+        // store-root ledger again, so one run paid 16 x O(R_total + C).
+        let bounds = CandidateUniverseBoundsV1::new(256, 16).expect("fixture bounds are nonzero");
+        let root = test_dir();
+        let (first, foreign, third) = (prepared(80), prepared(81), prepared(82));
+        let mut writer = CandidateLedgerWriterV1::new();
+        LEDGER_SCANS.with(|count| count.set(0));
+        LEDGER_CATCH_UP_RECEIPTS.with(|count| count.set(0));
+        let written = writer
+            .append_and_reverify(root.path(), bounds, &first)
+            .expect("the first append opens and writes");
+        assert!(matches!(
+            written,
+            CandidateUniverseProductionCommitV1::Written(_)
+        ));
+        assert_eq!(LEDGER_SCANS.with(std::cell::Cell::get), 1);
+
+        // Another writer appends between two of this run's appends.
+        let other = append_prepared_and_reverify(root.path(), bounds, &foreign)
+            .expect("a foreign writer appends");
+        LEDGER_SCANS.with(|count| count.set(0));
+        let appended = writer
+            .append_and_reverify(root.path(), bounds, &third)
+            .expect("the carried writer catches up and appends");
+        assert_eq!(
+            LEDGER_SCANS.with(std::cell::Cell::get),
+            0,
+            "the carried writer caught up instead of opening the ledger again"
+        );
+        assert_eq!(
+            LEDGER_CATCH_UP_RECEIPTS.with(std::cell::Cell::get),
+            1,
+            "the foreign completion was verified, not missed"
+        );
+        assert_eq!(
+            appended.audit().first_row(),
+            first.receipt().row_count() + foreign.receipt().row_count(),
+            "the new block starts after the foreign one"
+        );
+
+        // The foreign universe is in the carried index, so its exact retry is
+        // reused, and an unchanged ledger is not read again.
+        let reused = writer
+            .append_and_reverify(root.path(), bounds, &foreign)
+            .expect("the foreign universe's exact retry is reused");
+        assert!(matches!(
+            reused,
+            CandidateUniverseProductionCommitV1::Reused(_)
+        ));
+        assert_eq!(reused.audit(), other.audit());
+        let again = writer
+            .append_and_reverify(root.path(), bounds, &first)
+            .expect("this run's own first universe is reused");
+        assert_eq!(again.audit(), written.audit());
+        assert_eq!(LEDGER_SCANS.with(std::cell::Cell::get), 0);
+        assert_eq!(LEDGER_CATCH_UP_RECEIPTS.with(std::cell::Cell::get), 1);
+
+        let fresh = CandidateUniverseLedgerV1::open_read(root.path(), bounds)
+            .expect("a fresh reader opens");
+        for (offered, commit) in [(&first, written), (&foreign, other), (&third, appended)] {
+            assert_eq!(
+                fresh
+                    .reopen_audit(&offered.receipt().universe_id())
+                    .expect("generations hold"),
+                Some(commit.audit()),
+                "a fresh reader agrees with every audit the carried writer returned"
+            );
+        }
+    }
+
+    #[test]
+    #[expect(
+        clippy::too_many_lines,
+        reason = "one adversarial sequence keeps each catch-up refusal beside the reopen that follows it"
+    )]
+    fn a_carried_writer_refuses_what_it_cannot_catch_up_on() {
+        let bounds = CandidateUniverseBoundsV1::new(256, 16).expect("fixture bounds are nonzero");
+
+        // A foreign block broken after it was written is refused by its seal
+        // while the carried writer catches up, and a refused writer reopens.
+        let root = test_dir();
+        let mut writer = CandidateLedgerWriterV1::new();
+        writer
+            .append_and_reverify(root.path(), bounds, &prepared(83))
+            .expect("the first append writes");
+        append_prepared_and_reverify(root.path(), bounds, &prepared(84))
+            .expect("a foreign writer appends");
+        flip_candidate_row_fact(root.path(), 4);
+        assert!(
+            writer
+                .append_and_reverify(root.path(), bounds, &prepared(85))
+                .expect_err("a corrupt foreign block must refuse")
+                .contains("seal")
+        );
+        flip_candidate_row_fact(root.path(), 4);
+        LEDGER_SCANS.with(|count| count.set(0));
+        writer
+            .append_and_reverify(root.path(), bounds, &prepared(85))
+            .expect("the restored ledger accepts the append");
+        assert_eq!(
+            LEDGER_SCANS.with(std::cell::Cell::get),
+            1,
+            "a writer that refused once is never trusted again: it reopens"
+        );
+
+        // A rewrite of committed bytes that added no completion is not an
+        // append, so it is refused rather than caught up.
+        flip_candidate_row_fact(root.path(), 0);
+        LEDGER_SCANS.with(|count| count.set(0));
+        assert!(
+            writer
+                .append_and_reverify(root.path(), bounds, &prepared(86))
+                .expect_err("an in-place rewrite must refuse")
+                .contains("only appended completions are caught up")
+        );
+        assert_eq!(LEDGER_SCANS.with(std::cell::Cell::get), 0);
+        flip_candidate_row_fact(root.path(), 0);
+
+        // The honest limit (D-4780): an in-place rewrite of an older block
+        // that arrives together with a foreign append is not re-read by the
+        // catch-up, which verifies only the new block; the next open refuses it.
+        let limit_root = test_dir();
+        let mut limited = CandidateLedgerWriterV1::new();
+        limited
+            .append_and_reverify(limit_root.path(), bounds, &prepared(87))
+            .expect("the first append writes");
+        append_prepared_and_reverify(limit_root.path(), bounds, &prepared(88))
+            .expect("a foreign writer appends");
+        flip_candidate_row_fact(limit_root.path(), 0);
+        limited
+            .append_and_reverify(limit_root.path(), bounds, &prepared(89))
+            .expect("the catch-up verifies the foreign block only");
+        assert!(
+            CandidateUniverseLedgerV1::open_read(limit_root.path(), bounds)
+                .expect_err("the next open re-validates every block")
+                .contains("seal")
+        );
+
+        // A replaced row file refuses by name.
+        let replaced_root = test_dir();
+        let mut replaced = CandidateLedgerWriterV1::new();
+        replaced
+            .append_and_reverify(replaced_root.path(), bounds, &prepared(90))
+            .expect("the first append writes");
+        let path = replaced_root.path().join(ROW_FILE);
+        let displaced = replaced_root.path().join("candidate-rows.displaced");
+        std::fs::rename(&path, &displaced).expect("the held row file is displaced");
+        std::fs::copy(&displaced, &path).expect("the same bytes appear at a new inode");
+        assert!(
+            replaced
+                .append_and_reverify(replaced_root.path(), bounds, &prepared(91))
+                .expect_err("a replaced row file must refuse")
+                .contains("no longer names")
+        );
+
+        // A replaced lock file refuses by name before anything is caught up.
+        let lock_root = test_dir();
+        let mut locked = CandidateLedgerWriterV1::new();
+        locked
+            .append_and_reverify(lock_root.path(), bounds, &prepared(98))
+            .expect("the first append writes");
+        let lock_path = lock_root.path().join(LOCK_FILE);
+        std::fs::rename(
+            &lock_path,
+            lock_root.path().join("candidate.lock.displaced"),
+        )
+        .expect("the held lock is displaced");
+        File::create(&lock_path).expect("a new lock inode appears");
+        assert!(
+            locked
+                .append_and_reverify(lock_root.path(), bounds, &prepared(99))
+                .expect_err("a replaced lock must refuse")
+                .contains("no longer names")
+        );
+
+        // A carried writer serves exactly the root and bounds it opened.
+        let other_root = test_dir();
+        let mut carried = CandidateLedgerWriterV1::new();
+        carried
+            .append_and_reverify(root.path(), bounds, &prepared(92))
+            .expect("the first append writes");
+        assert!(
+            carried
+                .append_and_reverify(other_root.path(), bounds, &prepared(93))
+                .expect_err("another root must refuse")
+                .contains("carried Candidate writer")
+        );
+        let mut carried = CandidateLedgerWriterV1::new();
+        carried
+            .append_and_reverify(root.path(), bounds, &prepared(92))
+            .expect("the exact retry is reused");
+        let wider = CandidateUniverseBoundsV1::new(512, 16).expect("fixture bounds are nonzero");
+        assert!(
+            carried
+                .append_and_reverify(root.path(), wider, &prepared(93))
+                .expect_err("other bounds must refuse")
+                .contains("carried Candidate writer")
+        );
+    }
+
+    /// Appends a raw copy of the four-row block at `first_row` and of
+    /// completion `receipt_index`, as a writer that ignored every rule would.
+    fn append_raw_copy_of_block(root: &Path, first_row: u64, receipt_index: u64) {
+        let rows_path = root.join(ROW_FILE);
+        let receipts_path = root.join(RECEIPT_FILE);
+        let rows = std::fs::read(&rows_path).expect("rows read");
+        let receipts = std::fs::read(&receipts_path).expect("receipts read");
+        let row_stride = usize::try_from(CANDIDATE_ROW_STRIDE_V1).expect("stride fits");
+        let receipt_stride = usize::try_from(CANDIDATE_RECEIPT_STRIDE_V1).expect("stride fits");
+        let header = usize::try_from(HEADER_BYTES_V1).expect("header fits");
+        let row_start = header + usize::try_from(first_row).expect("row fits") * row_stride;
+        let receipt_start =
+            header + usize::try_from(receipt_index).expect("index fits") * receipt_stride;
+        let block = rows[row_start..row_start + 4 * row_stride].to_vec();
+        let receipt = receipts[receipt_start..receipt_start + receipt_stride].to_vec();
+        let mut file = OpenOptions::new()
+            .append(true)
+            .open(&rows_path)
+            .expect("rows open");
+        file.write_all(&block)
+            .and_then(|()| file.sync_data())
+            .expect("rows copy");
+        let mut file = OpenOptions::new()
+            .append(true)
+            .open(&receipts_path)
+            .expect("receipts open");
+        file.write_all(&receipt)
+            .and_then(|()| file.sync_data())
+            .expect("receipt copy");
+    }
+
+    /// Removes the last stored completion, leaving its rows as an orphan.
+    fn drop_last_candidate_receipt(root: &Path) {
+        let path = root.join(RECEIPT_FILE);
+        let len = std::fs::metadata(&path).expect("receipts measure").len();
+        let file = open_file(&path, true, false).expect("receipts reopen");
+        file.set_len(len - CANDIDATE_RECEIPT_STRIDE_V1)
+            .and_then(|()| file.sync_all())
+            .expect("the last completion is cut");
+    }
+
+    #[test]
+    fn a_carried_writer_catches_up_from_its_own_committed_cursor() {
+        // The carried handle holds an orphan (its append was an exact retry,
+        // reused), another writer then completes that orphan: the catch-up
+        // verifies the orphan's block where it lies, below the physical end.
+        let bounds = CandidateUniverseBoundsV1::new(256, 16).expect("fixture bounds are nonzero");
+        let root = test_dir();
+        let (first, orphaned, later) = (prepared(70), prepared(71), prepared(72));
+        append_prepared_and_reverify(root.path(), bounds, &first).expect("the first writes");
+        append_prepared_and_reverify(root.path(), bounds, &orphaned).expect("the second writes");
+        drop_last_candidate_receipt(root.path());
+        let mut writer = CandidateLedgerWriterV1::new();
+        let reused = writer
+            .append_and_reverify(root.path(), bounds, &first)
+            .expect("an exact retry under an orphan is reused");
+        assert!(matches!(
+            reused,
+            CandidateUniverseProductionCommitV1::Reused(_)
+        ));
+        let completed = append_prepared_and_reverify(root.path(), bounds, &orphaned)
+            .expect("another writer completes the orphan");
+        LEDGER_SCANS.with(|count| count.set(0));
+        LEDGER_CATCH_UP_RECEIPTS.with(|count| count.set(0));
+        let appended = writer
+            .append_and_reverify(root.path(), bounds, &later)
+            .expect("the carried writer catches up on the completed orphan");
+        assert_eq!(appended.audit().first_row(), 8);
+        assert_eq!(LEDGER_SCANS.with(std::cell::Cell::get), 0);
+        assert_eq!(LEDGER_CATCH_UP_RECEIPTS.with(std::cell::Cell::get), 1);
+        assert_eq!(
+            writer
+                .append_and_reverify(root.path(), bounds, &orphaned)
+                .expect("the caught-up universe is reused")
+                .audit(),
+            completed.audit()
+        );
+
+        // A reuse right after a catch-up rechecks the generations the
+        // catch-up measured, and they hold.
+        append_prepared_and_reverify(root.path(), bounds, &prepared(73))
+            .expect("another writer appends");
+        assert_eq!(
+            writer
+                .append_and_reverify(root.path(), bounds, &first)
+                .expect("a reuse after a catch-up reverifies")
+                .audit(),
+            reused.audit()
+        );
+
+        // An orphan another writer left past its completions is seen by the
+        // catch-up, so a different universe is refused rather than written
+        // after it.
+        append_prepared_and_reverify(root.path(), bounds, &prepared(74))
+            .expect("another writer appends");
+        append_prepared_and_reverify(root.path(), bounds, &prepared(75))
+            .expect("another writer appends");
+        drop_last_candidate_receipt(root.path());
+        assert!(
+            writer
+                .append_and_reverify(root.path(), bounds, &prepared(76))
+                .expect_err("a foreign orphan must refuse another universe")
+                .contains("no fallback may hide it")
+        );
+    }
+
+    #[test]
+    fn a_carried_writer_refuses_history_that_moved_and_duplicate_completions() {
+        let bounds = CandidateUniverseBoundsV1::new(256, 16).expect("fixture bounds are nonzero");
+
+        // The ledger is wiped and regrown longer than the handle measured it:
+        // the last indexed completion is not where it was, so the catch-up
+        // refuses rather than trusting the rows below it.
+        let root = test_dir();
+        let mut writer = CandidateLedgerWriterV1::new();
+        let first = prepared(77);
+        writer
+            .append_and_reverify(root.path(), bounds, &first)
+            .expect("the first append writes");
+        for name in [ROW_FILE, RECEIPT_FILE] {
+            let file = open_file(&root.path().join(name), true, false).expect("file reopens");
+            file.set_len(HEADER_BYTES_V1)
+                .and_then(|()| file.sync_all())
+                .expect("the file is wiped to its header");
+        }
+        append_prepared_and_reverify(root.path(), bounds, &prepared(78))
+            .expect("another writer appends to the wiped ledger");
+        append_prepared_and_reverify(root.path(), bounds, &prepared(79))
+            .expect("another writer appends again");
+        assert!(
+            writer
+                .append_and_reverify(root.path(), bounds, &first)
+                .expect_err("moved history must refuse")
+                .contains("is no longer the one the carried Candidate writer indexed")
+        );
+
+        // A raw copy of an indexed block and its completion seals, and is
+        // still refused as a second completion of one universe.
+        let duplicate_root = test_dir();
+        let mut duplicated = CandidateLedgerWriterV1::new();
+        duplicated
+            .append_and_reverify(duplicate_root.path(), bounds, &prepared(100))
+            .expect("the first append writes");
+        append_raw_copy_of_block(duplicate_root.path(), 0, 0);
+        assert!(
+            duplicated
+                .append_and_reverify(duplicate_root.path(), bounds, &prepared(101))
+                .expect_err("a duplicate of an indexed universe must refuse")
+                .contains("more than one completion receipt")
+        );
+
+        // Two copies among the new completions refuse the same way.
+        let twice_root = test_dir();
+        let mut twice = CandidateLedgerWriterV1::new();
+        twice
+            .append_and_reverify(twice_root.path(), bounds, &prepared(102))
+            .expect("the first append writes");
+        append_prepared_and_reverify(twice_root.path(), bounds, &prepared(103))
+            .expect("another writer appends");
+        append_raw_copy_of_block(twice_root.path(), 4, 1);
+        assert!(
+            twice
+                .append_and_reverify(twice_root.path(), bounds, &prepared(104))
+                .expect_err("a duplicate among new completions must refuse")
+                .contains("more than one completion receipt")
+        );
+    }
+
     #[test]
     fn ragged_corrupt_and_stale_files_fail_closed() {
         let bounds = CandidateUniverseBoundsV1::new(32, 4).expect("fixture bounds are nonzero");
diff --git a/crates/cli/src/ledger_all.rs b/crates/cli/src/ledger_all.rs
index 10a7ac6a..f3714a8d 100644
--- a/crates/cli/src/ledger_all.rs
+++ b/crates/cli/src/ledger_all.rs
@@ -874,18 +874,31 @@ fn run_chain(request: &LedgerAllRequest<'_>, out: &mut String) -> Result<usize,
     let source_root = crate::store_root().map_err(|why| format!("stored source root: {why}"))?;
     let tree = LedgerTree::create(request.root)?;
 
-    let sweepers = build_sweepers(&source_root, vendor, request, LEDGER_ALL_VERB)?;
-
     let ranking = RankingPolicyV1::new(Weights::equal())
         .map_err(|why| format!("ranking policy refused: {why:?}"))?;
 
+    // SIZED PER RUNG, AT THAT RUNG'S CANDIDATE PHASE (G4-2, D-4784). The
+    // census loads NIFTY's context and builds its column, and that rung's NIFTY
+    // commit consumes both rather than loading and building them again.
+    let sizing = SizingInputs::new()?;
+    let size = |_: usize, rung: &str| {
+        size_rung(
+            &source_root,
+            vendor,
+            request,
+            LEDGER_ALL_VERB,
+            rung,
+            &sizing,
+        )
+    };
+
     crate::note(&stage_started_event(LEDGER_ALL_VERB, POPULATION_STAGE));
     let population = commit_population(&Stage1 {
         source_root: source_root.as_path(),
         tree: &tree,
         vendor,
         request,
-        sweepers: &sweepers,
+        size: &size,
         admission: &admission,
     })
     .inspect_err(|why| {
@@ -1081,7 +1094,7 @@ struct Stage1<'a> {
     tree: &'a LedgerTree,
     vendor: Vendor,
     request: &'a LedgerAllRequest<'a>,
-    sweepers: &'a [Sweeper; 8],
+    size: &'a crate::all_rung_population_v5::SizeRungV1<'a>,
     admission: &'a AdmissionPolicyV1,
 }
 
@@ -1101,16 +1114,6 @@ fn commit_population(
     let request = stage.request;
     let long_exit = exit_policy(Side::Long)?;
     let short_exit = exit_policy(Side::Short)?;
-    let [
-        one_minute_sweeper,
-        two_minute_sweeper,
-        three_minute_sweeper,
-        five_minute_sweeper,
-        ten_minute_sweeper,
-        fifteen_minute_sweeper,
-        thirty_minute_sweeper,
-        sixty_minute_sweeper,
-    ] = stage.sweepers;
 
     commit_all_rung_stored_population_v5(&AllRungStoredPopulationV5Request {
         source_root: stage.source_root,
@@ -1118,14 +1121,7 @@ fn commit_population(
         vendor: stage.vendor,
         from: request.from,
         to: request.to,
-        one_minute_sweeper,
-        two_minute_sweeper,
-        three_minute_sweeper,
-        five_minute_sweeper,
-        ten_minute_sweeper,
-        fifteen_minute_sweeper,
-        thirty_minute_sweeper,
-        sixty_minute_sweeper,
+        sweepers: crate::all_rung_population_v5::AllRungSweepersV1::SizedPerRung(stage.size),
         horizon: Horizon::DEFAULT,
         widths: Widths::pinned().map_err(|why| format!("pinned tolerances: {why}"))?,
         // ABSENT, and this caller may not derive it: `vwap::availability_of`
@@ -1378,7 +1374,33 @@ fn execution_bounds() -> Result<ExecutionV3Bounds, String> {
     .map_err(|why| format!("execution bounds: {why:?}"))
 }
 
-/// One sweeper per rung, each with a support threshold from its OWN bar count.
+/// What every rung's sizing census shares: the load ceilings and the
+/// evaluation inputs the Candidate commit will use.
+pub(crate) struct SizingInputs {
+    bounds: crate::step3_orchestrator::StoredCandidatePreAdmissionBoundsV1,
+    evaluation: crate::candidate_universe::CandidateEvaluationInputsV1,
+}
+
+impl SizingInputs {
+    /// The production ceilings and evaluation inputs `ledger-all` commits with.
+    ///
+    /// # Errors
+    ///
+    /// A refused ceiling or pinned tolerance.
+    pub(crate) fn new() -> Result<Self, String> {
+        Ok(Self {
+            bounds: candidate_bounds()?,
+            evaluation: crate::candidate_universe::CandidateEvaluationInputsV1 {
+                widths: Widths::pinned().map_err(|why| format!("pinned tolerances: {why}"))?,
+                availability: Availability::Absent,
+                thresholds: Thresholds::CLASSICAL,
+            },
+        })
+    }
+}
+
+/// One rung's sweeper, with a support threshold from that rung's OWN bar
+/// count, and the sized NIFTY context and column its NIFTY commit consumes.
 ///
 /// # Why not one shared threshold
 ///
@@ -1389,79 +1411,77 @@ fn execution_bounds() -> Result<ExecutionV3Bounds, String> {
 /// operator's support in ppm and resolving it against each rung's own bars asks
 /// the same question eight times.
 ///
-/// # What it logs, and why the loop may
+/// # Why per rung, and why the context is handed on (G4-2, D-4784)
+///
+/// D-2103 made this census load NIFTY's whole signal, one-minute and daily
+/// context and build its Candidate column, and the eight censuses ran before
+/// the first commit, each dropping its context, so every rung's NIFTY commit
+/// loaded and built the same thing again. The all-rung coordinator now calls
+/// this at the start of each rung's Candidate phase and hands the returned
+/// context and column to that rung's NIFTY commit: one load and one column
+/// build per (family, rung), and no more contexts held at once than before.
+/// A rung whose span is missing is still named, not skipped; it now refuses
+/// when its own rung is reached, after the earlier rungs' receipt-last
+/// appends, which an exact retry reuses.
+///
+/// # What it logs, and why it may
 ///
 /// One event per rung, eight for the whole run, each at the moment that rung's
-/// span has loaded and its threshold is known. The loop is over
-/// [`LEDGER_RUNGS`] -- a fixed eight -- and not over the bars it just counted,
-/// so the event count does not move with the operator's span. Gate 17's rule is
-/// about the loop over bars, and there is none here.
+/// span has loaded and its threshold is known. The coordinator's loop is over
+/// its eight canonical rungs and not over the bars just counted, so the event
+/// count does not move with the operator's span. Gate 17's rule is about the
+/// loop over bars, and there is none here.
 ///
 /// # Errors
 ///
-/// Refuses if a rung's span cannot be loaded or its ladder rejected. A rung
-/// with no stored bars is named, not skipped: a silent skip would produce a
-/// seven-rung answer in an eight-rung report.
-pub(crate) fn build_sweepers(
+/// Refuses if the rung's span cannot be loaded or its ladder rejected.
+pub(crate) fn size_rung(
     root: &Path,
     vendor: Vendor,
     request: &LedgerAllRequest<'_>,
     verb: &str,
-) -> Result<[Sweeper; 8], String> {
-    let mut sweepers = Vec::with_capacity(LEDGER_RUNGS.len());
-    let bounds = candidate_bounds()?;
-    let evaluation = crate::candidate_universe::CandidateEvaluationInputsV1 {
-        widths: Widths::pinned().map_err(|why| format!("pinned tolerances: {why}"))?,
-        availability: Availability::Absent,
-        thresholds: Thresholds::CLASSICAL,
-    };
-    for rung in LEDGER_RUNGS {
-        // SIZED ON NIFTY'S BARS, and the pair is why that is not a narrowing.
-        // Both families are swept at the same threshold, and the two share a
-        // calendar and a session length, so either one answers "how many bars
-        // does this rung hold over this span". NIFTY is the one the store
-        // actually has.
-        //
-        // SWEPT BARS, NOT RETAINED ONES (D-2103). The support denominator is
-        // the rows NIFTY's Candidate column sweeps, read from the very build
-        // its commit runs, so the warm-up rows no combination can hit do not
-        // raise the threshold.
-        let swept = crate::step3_orchestrator::stored_candidate_swept_v1(
-            root,
-            vendor,
-            ("NIFTY", rung),
-            (request.from, request.to),
-            bounds,
-            &evaluation,
-        )
-        .map_err(|why| {
-            let first = why.lines().next().unwrap_or("").to_owned();
-            let refusal = format!("{rung} span refused: {first}");
-            crate::note(&rung_refused_event(verb, SIZING_STAGE, rung, &refusal));
-            refusal
-        })?;
-        let min_hits = crate::min_hits_for_swept(swept, request.support_ppm);
-        let ladder = crate::ladder_for(min_hits).map_err(|why| {
-            let refusal = format!("{rung} ladder: {why}");
-            crate::note(&rung_refused_event(verb, SIZING_STAGE, rung, &refusal));
-            refusal
-        })?;
-        crate::note(&rung_sized_event(
-            verb,
-            rung,
-            swept,
-            min_hits,
-            request.support_ppm,
-        ));
-        sweepers.push(Sweeper::new(ladder));
-    }
-    // AN ARRAY, so the caller destructures instead of indexing. Eight named
-    // sweepers reached by `sweepers[0]`..`sweepers[7]` is eight chances to
-    // hand the sixty-minute ladder to the one-minute rung, and neither the
-    // compiler nor a reader would catch it.
-    sweepers
-        .try_into()
-        .map_err(|_| "the sweeper phase did not build eight ladders".to_owned())
+    rung: &str,
+    sizing: &SizingInputs,
+) -> Result<(Sweeper, crate::step3_orchestrator::strict::SizedNifty), String> {
+    // SIZED ON NIFTY'S BARS, and the pair is why that is not a narrowing.
+    // Both families are swept at the same threshold, and the two share a
+    // calendar and a session length, so either one answers "how many bars
+    // does this rung hold over this span". NIFTY is the one the store
+    // actually has.
+    //
+    // SWEPT BARS, NOT RETAINED ONES (D-2103). The support denominator is
+    // the rows NIFTY's Candidate column sweeps, read from the very build
+    // its commit consumes, so the warm-up rows no combination can hit do not
+    // raise the threshold.
+    let sized = crate::step3_orchestrator::stored_candidate_swept_v1(
+        root,
+        vendor,
+        ("NIFTY", rung),
+        (request.from, request.to),
+        sizing.bounds,
+        &sizing.evaluation,
+    )
+    .map_err(|why| {
+        let first = why.lines().next().unwrap_or("").to_owned();
+        let refusal = format!("{rung} span refused: {first}");
+        crate::note(&rung_refused_event(verb, SIZING_STAGE, rung, &refusal));
+        refusal
+    })?;
+    let swept = sized.swept();
+    let min_hits = crate::min_hits_for_swept(swept, request.support_ppm);
+    let ladder = crate::ladder_for(min_hits).map_err(|why| {
+        let refusal = format!("{rung} ladder: {why}");
+        crate::note(&rung_refused_event(verb, SIZING_STAGE, rung, &refusal));
+        refusal
+    })?;
+    crate::note(&rung_sized_event(
+        verb,
+        rung,
+        swept,
+        min_hits,
+        request.support_ppm,
+    ));
+    Ok((Sweeper::new(ladder), sized))
 }
 
 #[cfg(test)]
diff --git a/crates/cli/src/ledger_v6.rs b/crates/cli/src/ledger_v6.rs
index 7b9ff80c..58dbe515 100644
--- a/crates/cli/src/ledger_v6.rs
+++ b/crates/cli/src/ledger_v6.rs
@@ -322,6 +322,12 @@ fn run_route(
     let bounds = candidate_bounds()?;
 
     let mut committed = Vec::with_capacity(8);
+    // ONE CANDIDATE WRITER FOR THE WHOLE RUN (W2-cli3-4, D-4780). The
+    // Candidate ledger sits at the store root and every run, rung and family
+    // shares it; opening it per append cost O(R_total + C) sixteen times a
+    // run. The writer opens it at the first append and, before each later one,
+    // verifies only what other writers appended since.
+    let mut candidate_writer = crate::candidate_universe::CandidateLedgerWriterV1::new();
     for (index, rung) in LEDGER_RUNGS.into_iter().enumerate() {
         let (sweeper, sizing_inputs, sized) = crate::step3_orchestrator::strict::size_sweeper(
             &source_root,
@@ -374,6 +380,7 @@ fn run_route(
                 },
                 &strict,
                 preloaded,
+                &mut candidate_writer,
             )
             .map_err(|why| {
                 let refusal = format!("v6 {rung} {underlying} refused: {why}");
diff --git a/crates/cli/src/step3_all_rung_tests.rs b/crates/cli/src/step3_all_rung_tests.rs
index e0cb4bdf..bfe5af1b 100644
--- a/crates/cli/src/step3_all_rung_tests.rs
+++ b/crates/cli/src/step3_all_rung_tests.rs
@@ -8,8 +8,9 @@ use super::tests::{
 };
 use super::*;
 use crate::all_rung_population_v5::{
-    AllRungStoredExecutionV3Request, AllRungStoredPopulationV5Request,
-    commit_all_rung_stored_execution_v3, commit_all_rung_with_verified_build_v5,
+    AllRungStoredExecutionV3Request, AllRungStoredPopulationV5Request, AllRungSweepersV1,
+    NamedAllRungSweepersV1, commit_all_rung_stored_execution_v3,
+    commit_all_rung_with_verified_build_v5,
 };
 use crate::all_rung_selection_v5::{
     AllRungSelectionV5Request, commit_all_rung_stored_selection_v5,
@@ -122,14 +123,16 @@ impl Policies {
             vendor: Vendor::Zerodha,
             from: FIRST_MONTH,
             to: FIXTURE_TO,
-            one_minute_sweeper: &self.sweepers[0],
-            two_minute_sweeper: &self.sweepers[1],
-            three_minute_sweeper: &self.sweepers[2],
-            five_minute_sweeper: &self.sweepers[3],
-            ten_minute_sweeper: &self.sweepers[4],
-            fifteen_minute_sweeper: &self.sweepers[5],
-            thirty_minute_sweeper: &self.sweepers[6],
-            sixty_minute_sweeper: &self.sweepers[7],
+            sweepers: AllRungSweepersV1::Named(NamedAllRungSweepersV1 {
+                one_minute: &self.sweepers[0],
+                two_minute: &self.sweepers[1],
+                three_minute: &self.sweepers[2],
+                five_minute: &self.sweepers[3],
+                ten_minute: &self.sweepers[4],
+                fifteen_minute: &self.sweepers[5],
+                thirty_minute: &self.sweepers[6],
+                sixty_minute: &self.sweepers[7],
+            }),
             horizon: Horizon::DEFAULT,
             widths: Widths::pinned().map_err(|why| format!("{why:?}"))?,
             availability: Availability::Absent,
@@ -400,3 +403,79 @@ fn all_eight_stored_rungs_publish_exact_selection_chains_and_reuse_every_byte()
     assert_eq!(published(&all_paths)?, original.expect("first publication"));
     Ok(())
 }
+
+#[test]
+fn all_rung_sizing_hands_each_nifty_context_to_its_commit_and_one_writer_serves_every_append()
+-> Result<(), String> {
+    // G4-2 / D-4784: `ledger-all` sized every rung on a full NIFTY context and
+    // column, dropped them, and each NIFTY commit loaded and built them again.
+    // W2-cli3-4 / D-4780: every one of the sixteen Candidate appends opened the
+    // whole source-root ledger.
+    let fixture = StoredSuccessFixture::with_family_prices([2_000_000; 2])?;
+    for symbol in ["NIFTY", "BANKNIFTY"] {
+        for month in &MONTHS[..4] {
+            seed_stored_family_month(&fixture.source, Vendor::Zerodha, symbol, *month, 2_000_000)?;
+        }
+    }
+    seed_derived(&fixture.source)?;
+    let policies = Policies::new(&fixture.source)?;
+    let authority = fixture.base.join("all-rung-population");
+    roots(&authority)?;
+    let named = policies.request(&fixture.source, &authority)?;
+    let evaluation = crate::candidate_universe::CandidateEvaluationInputsV1 {
+        widths: named.widths,
+        availability: named.availability,
+        thresholds: named.thresholds,
+    };
+    let size = |index: usize, rung: &str| {
+        let sized = crate::step3_orchestrator::stored_candidate_swept_v1(
+            &fixture.source,
+            Vendor::Zerodha,
+            ("NIFTY", rung),
+            (named.from, named.to),
+            named.candidate_bounds,
+            &evaluation,
+        )?;
+        Ok((Sweeper::new(policies.sweepers[index].ladder()), sized))
+    };
+    let mut sized = named;
+    sized.sweepers = AllRungSweepersV1::SizedPerRung(&size);
+
+    let mut counts = Vec::new();
+    for request in [&named, &sized] {
+        STORED_CONTEXT_LOADS.with(|count| count.set(0));
+        crate::candidate_universe::FULL_SIGNAL_COLUMN_BUILDS.with(|count| count.set(0));
+        crate::candidate_universe::LEDGER_WRITER_OPENS.with(|count| count.set(0));
+        drop(commit_all_rung_with_verified_build_v5(
+            request,
+            VerifiedBuildCommitV1(FIXTURE_COMMIT),
+        )?);
+        counts.push((
+            STORED_CONTEXT_LOADS.with(std::cell::Cell::get),
+            crate::candidate_universe::FULL_SIGNAL_COLUMN_BUILDS.with(std::cell::Cell::get),
+            crate::candidate_universe::LEDGER_WRITER_OPENS.with(std::cell::Cell::get),
+        ));
+    }
+    let [
+        (named_loads, named_builds, named_scans),
+        (sized_loads, sized_builds, sized_scans),
+    ]: [_; 2] = counts.try_into().map_err(|_| "two measured runs")?;
+    assert_eq!(
+        named_loads, 16,
+        "two families by eight rungs, one load each"
+    );
+    assert_eq!(
+        sized_loads, 16,
+        "the eight sizing loads are the NIFTY commits' loads, not extra ones"
+    );
+    assert_eq!(
+        sized_builds, named_builds,
+        "the eight sizing column builds are the NIFTY commits' builds, not extra ones"
+    );
+    assert_eq!(
+        (named_scans, sized_scans),
+        (1, 1),
+        "one writable Candidate ledger open serves all sixteen appends of a run"
+    );
+    Ok(())
+}
diff --git a/crates/cli/src/step3_orchestrator.rs b/crates/cli/src/step3_orchestrator.rs
index 4fb6d042..4e950501 100644
--- a/crates/cli/src/step3_orchestrator.rs
+++ b/crates/cli/src/step3_orchestrator.rs
@@ -44,8 +44,8 @@ use crate::anchored_search_lineage_v4::{
 };
 use crate::candidate_universe::{
     AuthenticatedCandidatePopulationRowV1, BaseEvidenceLedgerBoundsV2, BaseEvidenceLedgerReaderV2,
-    BaseEvidenceReopenAuditV2, CandidateExecutionReplayAuthorityV1, CandidateUniverseBoundsV1,
-    CandidateUniverseLedgerV1, CandidateUniverseProductionCommitV1,
+    BaseEvidenceReopenAuditV2, CandidateExecutionReplayAuthorityV1, CandidateLedgerWriterV1,
+    CandidateUniverseBoundsV1, CandidateUniverseLedgerV1, CandidateUniverseProductionCommitV1,
     CandidateUniverseProductionSourceV1, CandidateUniverseReceiptV1,
     CandidateUniverseReopenAuditV1, PairedBaseEvidenceAuthorityV2, PairedBaseEvidenceReaderV2,
     PairedBaseEvidenceRecordProjectionV2, ProducedCandidateUniverseV1,
@@ -497,30 +497,10 @@ pub(crate) struct BoundedStoredContextV1 {
     pub(crate) strict: Option<Arc<strict::Inputs>>,
 }
 
-impl BoundedStoredContextV1 {
-    /// Rows this context's Candidate signal column sweeps: the support
-    /// denominator a ledger sizes its threshold on (D-2103).
-    ///
-    /// # Errors
-    ///
-    /// Every refusal of the Candidate column build.
-    pub(crate) fn candidate_swept_v1(
-        &self,
-        evaluation: &crate::candidate_universe::CandidateEvaluationInputsV1,
-    ) -> Result<u64, Step3OrchestratorRefusal> {
-        crate::candidate_universe::candidate_signal_swept_v1(
-            &self.signal.bars,
-            &self.daily.references,
-            &self.minute.bars,
-            self.rung_seconds,
-            evaluation,
-        )
-    }
-}
-
-/// Load one stored context exactly as a Candidate commit does and count the
-/// rows its signal column sweeps (D-2103). `ledger-all` sizes each rung's
-/// threshold on this for its sizing underlying.
+/// Load one stored context exactly as a Candidate commit does and build its
+/// signal column once (D-2103). `ledger-all` sizes each rung's threshold on
+/// [`strict::SizedNifty::swept`] and hands the returned context and column to
+/// that rung's NIFTY commit, so neither is built twice (G4-2, D-4784).
 ///
 /// # Errors
 ///
@@ -532,9 +512,10 @@ pub(crate) fn stored_candidate_swept_v1(
     (from, to): ((u16, u8), (u16, u8)),
     bounds: StoredCandidatePreAdmissionBoundsV1,
     evaluation: &crate::candidate_universe::CandidateEvaluationInputsV1,
-) -> Result<u64, Step3OrchestratorRefusal> {
+) -> Result<strict::SizedNifty, Step3OrchestratorRefusal> {
     let root = AdmittedRootV1::admit(root)?;
-    load_bounded_stored_context_from_spec_v1(
+    strict::census(
+        &root,
         StoredContextLoadSpecV1 {
             vendor,
             underlying,
@@ -545,10 +526,9 @@ pub(crate) fn stored_candidate_swept_v1(
             minute_bound: bounds.minute_records,
             daily_bound: bounds.daily_records,
         },
-        &root,
         None,
-    )?
-    .candidate_swept_v1(evaluation)
+        evaluation,
+    )
 }
 
 #[path = "stored_family_v6.rs"]
@@ -2944,6 +2924,7 @@ impl RetainedStoredExecutionContextV1 {
             signal_bound,
             minute_bound,
             daily_bound,
+            None,
         )
         .map_err(|why| format!("Step 3 Execution V3 Candidate source refused: {why}"))?;
         if source.search_splits() != self.search_splits {
@@ -3073,6 +3054,8 @@ pub fn commit_stored_candidate_pre_admission_v1(
 /// minted only from the clean process-free stamp. An all-rung transaction can
 /// retain one proof across its families. It owns a no-op progress observer;
 /// no caller-authored commit, bars, digest, calendar or result is admitted.
+/// Production now reaches it only through the carried door below (D-4780).
+#[cfg(test)]
 pub(crate) fn commit_stored_candidate_pre_admission_authority_v1(
     request: StoredCandidatePreAdmissionRequestV1<'_>,
     verified_commit: VerifiedBuildCommitV1<'_>,
@@ -3081,6 +3064,38 @@ pub(crate) fn commit_stored_candidate_pre_admission_authority_v1(
     commit_stored_with_verified_build_v1(request, verified_commit, &no_progress_observer)
 }
 
+/// Commit the same public stored request while retaining its opaque
+/// sources, for a transaction that carries one Candidate writer across all
+/// its appends (W2-cli3-4, D-4780) and may hand the NIFTY commit the context
+/// and column its sizing census already built (G4-2, D-4784). It owns a
+/// no-op progress observer, and no caller-authored commit, bars, digest,
+/// calendar or result is admitted.
+///
+/// # Errors
+///
+/// Every stored-request, build-proof, Candidate and Pre-Admission refusal,
+/// an extinct family, a carried writer opened for another root or bounds,
+/// or a sized context that names another request or whose sources changed.
+pub(crate) fn commit_stored_candidate_pre_admission_authority_carried_v1(
+    request: StoredCandidatePreAdmissionRequestV1<'_>,
+    verified_commit: VerifiedBuildCommitV1<'_>,
+    candidate_writer: &mut CandidateLedgerWriterV1,
+    sized: Option<strict::SizedNifty>,
+) -> Result<CommittedStoredCandidatePreAdmissionV1, Step3OrchestratorRefusal> {
+    commit_family_from_v6(
+        request,
+        verified_commit,
+        &|_, _, _| {},
+        None,
+        false,
+        sized,
+        candidate_writer,
+    )?
+    .into_parts()
+    .0
+    .ok_or_else(|| "ordinary Pre-Admission V1 cannot represent an extinct family".to_owned())
+}
+
 /// Strict institutional door; ordinary callers retain the historical contract.
 ///
 /// It consumes the strict context the rung's sizing census already loaded
@@ -3097,9 +3112,18 @@ pub(crate) fn commit_strict_candidate_pre_admission_authority_sized_v1(
     request: StoredCandidatePreAdmissionRequestV1<'_>,
     config: &crate::audited_range_command::StrictConfig,
     sized: Option<strict::SizedNifty>,
+    candidate_writer: &mut CandidateLedgerWriterV1,
 ) -> Result<family_v6::StoredFamilyV6, String> {
     let commit = VerifiedBuildCommitV1::current()?;
-    commit_family_from_v6(request, commit, &|_, _, _| {}, Some(config), true, sized)
+    commit_family_from_v6(
+        request,
+        commit,
+        &|_, _, _| {},
+        Some(config),
+        true,
+        sized,
+        candidate_writer,
+    )
 }
 
 fn commit_stored_with_verified_build_v1(
@@ -3137,6 +3161,7 @@ fn commit_family_with_inputs_v6(
         config,
         allow_extinct,
         None,
+        &mut CandidateLedgerWriterV1::new(),
     )
 }
 
@@ -3147,11 +3172,18 @@ fn commit_family_from_v6(
     config: Option<&crate::audited_range_command::StrictConfig>,
     allow_extinct: bool,
     sized: Option<strict::SizedNifty>,
+    candidate_writer: &mut CandidateLedgerWriterV1,
 ) -> Result<family_v6::StoredFamilyV6, String> {
     let mut root = AdmittedRootV1::admit(request.root)?;
-    let context = match sized {
-        Some(sized) => sized.into_context_for(&request, &root, config)?,
-        None => load_bounded_stored_context_v1(&request, &root, config)?,
+    let (context, prebuilt_signal) = match sized {
+        Some(sized) => {
+            let (context, column) = sized.into_context_for(&request, &root, config)?;
+            (context, Some(column))
+        }
+        None => (
+            load_bounded_stored_context_v1(&request, &root, config)?,
+            None,
+        ),
     };
     root.strict.clone_from(&context.strict);
     let attempt = strict::begin(&context, &request, verified_commit.0)?;
@@ -3160,8 +3192,9 @@ fn commit_family_from_v6(
         verified_commit,
         on_level,
         root,
-        context,
+        (context, prebuilt_signal),
         allow_extinct,
+        candidate_writer,
     );
     strict::finish(attempt, result)
 }
@@ -3171,8 +3204,12 @@ fn commit_loaded_stored_v1(
     verified_commit: VerifiedBuildCommitV1<'_>,
     on_level: &dyn Fn(&engine::Frontier, usize, u64),
     root: AdmittedRootV1,
-    context: BoundedStoredContextV1,
+    (context, prebuilt_signal): (
+        BoundedStoredContextV1,
+        Option<crate::candidate_universe::PrebuiltSignalColumnV1>,
+    ),
     allow_extinct: bool,
+    candidate_writer: &mut CandidateLedgerWriterV1,
 ) -> Result<family_v6::StoredFamilyV6, String> {
     let resolved = resolve_stored_execution_v1(
         &context,
@@ -3211,6 +3248,7 @@ fn commit_loaded_stored_v1(
         request.bounds.signal_records,
         request.bounds.minute_records,
         request.bounds.daily_records,
+        prebuilt_signal,
     )
     .map_err(|why| format!("Step 3 stored Candidate source refused: {why}"))?;
 
@@ -3228,6 +3266,7 @@ fn commit_loaded_stored_v1(
         request.bounds.pre_admission,
         on_level,
         allow_extinct,
+        candidate_writer,
     )?;
     let search =
         StoredSearchMemberV4::bind_candidate(&committed.candidate_audit(), search_validation)?;
@@ -3294,11 +3333,21 @@ struct StoredContextLoadSpecV1<'a> {
     daily_bound: StoredSpanLoadBoundV1,
 }
 
+#[cfg(test)]
+thread_local! {
+    /// Test-only count of stored Candidate context loads, strict or not, on
+    /// this thread (D-4784).
+    pub(crate) static STORED_CONTEXT_LOADS: std::cell::Cell<u64> =
+        const { std::cell::Cell::new(0) };
+}
+
 fn load_bounded_stored_context_from_spec_v1(
     spec: StoredContextLoadSpecV1<'_>,
     root: &AdmittedRootV1,
     config: Option<&crate::audited_range_command::StrictConfig>,
 ) -> Result<BoundedStoredContextV1, Step3OrchestratorRefusal> {
+    #[cfg(test)]
+    STORED_CONTEXT_LOADS.with(|count| count.set(count.get().saturating_add(1)));
     if let Some(config) = config {
         return strict::load(spec, root, config);
     }
@@ -4095,6 +4144,7 @@ fn commit_candidate_pre_admission_authority_guarded_v1<'a>(
         pre_admission_bounds,
         on_level,
         false,
+        &mut CandidateLedgerWriterV1::new(),
     )? {
         family_v6::CandidateCommitV6::Evaluated(committed) => Ok(*committed),
         family_v6::CandidateCommitV6::Extinct(_) => {
@@ -4105,7 +4155,7 @@ fn commit_candidate_pre_admission_authority_guarded_v1<'a>(
 
 #[expect(
     clippy::too_many_arguments,
-    reason = "one typed source and its existing independent publication bounds plus explicit V2 extinction permission"
+    reason = "one typed source and its existing independent publication bounds plus explicit V2 extinction permission and the run's carried Candidate writer"
 )]
 fn commit_candidate_family_guarded_v6<'a>(
     root: &Path,
@@ -4116,6 +4166,7 @@ fn commit_candidate_family_guarded_v6<'a>(
     pre_admission_bounds: PreAdmissionDataBoundsV1,
     on_level: &dyn Fn(&engine::Frontier, usize, u64),
     allow_extinct: bool,
+    candidate_writer: &mut CandidateLedgerWriterV1,
 ) -> Result<family_v6::CandidateCommitV6, String> {
     let candidate = produce_candidate_universe_v1(sweeper, source, candidate_bounds, on_level)
         .map_err(|why| format!("Step 3 Candidate production refused: {why}"))?;
@@ -4130,8 +4181,12 @@ fn commit_candidate_family_guarded_v6<'a>(
     let prepared_receipt = candidate.receipt();
     let observations = candidate.observations().clone();
     require_admitted_root_v1(admitted_root, "before Candidate receipt-last append/reopen")?;
+    // ONE WRITER PER RUN (W2-cli3-4, D-4780): the ledger sits at the store
+    // root every run shares, and opening it here cost O(R_total + C) per
+    // append, 16 times a run. The carried writer opens it once and catches up
+    // on what other writers appended since, verifying only those records.
     let candidate_commit = candidate
-        .append_and_reopen(root, candidate_bounds)
+        .append_and_reopen_with(candidate_writer, root, candidate_bounds)
         .map_err(|why| format!("Step 3 Candidate receipt-last commit refused: {why}"))?;
     require_admitted_root_v1(admitted_root, "after Candidate receipt-last append/reopen")?;
     let candidate_audit = candidate_commit.audit();
diff --git a/crates/cli/src/strict_v6_inputs.rs b/crates/cli/src/strict_v6_inputs.rs
index 771946d4..e921e937 100644
--- a/crates/cli/src/strict_v6_inputs.rs
+++ b/crates/cli/src/strict_v6_inputs.rs
@@ -52,11 +52,15 @@ impl Inputs {
 /// The family whose signal bar count sizes every rung's support threshold.
 pub(crate) const SIZING_UNDERLYING: &str = "NIFTY";
 
-/// The strict NIFTY context the sizing census loaded, held for the NIFTY
+/// The NIFTY context a sizing census loaded, and the Candidate signal column
+/// it built from that context to count the swept rows, held for the NIFTY
 /// family commit of the same rung so that span is loaded once, not twice
-/// (W2-cli7-3, D-1683). It can be consumed only by a request whose root,
-/// vendor, family, rung, span, load bounds and strict configuration are the
-/// ones it was loaded under; anything else refuses by name.
+/// (W2-cli7-3, D-1683), and its column is built once, not twice (G4-3,
+/// D-4783). `ledger-v6` holds a strict context; `ledger-all` holds an
+/// ordinary one (G4-2, D-4784). It can be consumed only by a request whose
+/// root, vendor, family, rung, span, load bounds, evaluation inputs and
+/// strict configuration are the ones it was built under; anything else
+/// refuses by name.
 pub(crate) struct SizedNifty {
     root: std::path::PathBuf,
     vendor: brutex_core::vendor::Vendor,
@@ -64,13 +68,20 @@ pub(crate) struct SizedNifty {
     from: (u16, u8),
     to: (u16, u8),
     bounds: [crate::stored::StoredSpanLoadBoundV1; 3],
-    config: StrictConfig,
+    config: Option<StrictConfig>,
     context: BoundedStoredContextV1,
+    column: crate::candidate_universe::PrebuiltSignalColumnV1,
 }
 
 impl SizedNifty {
-    /// The held context, when `request` and `config` name exactly what it
-    /// was loaded under and its sources are still current.
+    /// The rows the held Candidate signal column sweeps, warm-up excluded
+    /// (D-2103): the support denominator.
+    pub(crate) fn swept(&self) -> u64 {
+        self.column.swept()
+    }
+
+    /// The held context and column, when `request` and `config` name exactly
+    /// what they were built under and the sources are still current.
     ///
     /// # Errors
     ///
@@ -80,7 +91,13 @@ impl SizedNifty {
         request: &super::StoredCandidatePreAdmissionRequestV1<'_>,
         root: &AdmittedRootV1,
         config: Option<&StrictConfig>,
-    ) -> Result<BoundedStoredContextV1, String> {
+    ) -> Result<
+        (
+            BoundedStoredContextV1,
+            crate::candidate_universe::PrebuiltSignalColumnV1,
+        ),
+        String,
+    > {
         if let Some(term) = self.differing_term(request, root.path(), config) {
             return Err(format!(
                 "the sized {SIZING_UNDERLYING} context cannot serve a request with another {term}"
@@ -88,7 +105,7 @@ impl SizedNifty {
         }
         self.context.require_current()?;
         root.require_same("before reusing the sized strict context")?;
-        Ok(self.context)
+        Ok((self.context, self.column))
     }
 
     /// The first request term that differs from what this context was loaded
@@ -116,7 +133,14 @@ impl SizedNifty {
         ] != self.bounds
         {
             Some("load bounds")
-        } else if config != Some(&self.config) {
+        } else if (crate::candidate_universe::CandidateEvaluationInputsV1 {
+            widths: request.widths,
+            availability: request.availability,
+            thresholds: request.thresholds,
+        }) != self.column.evaluation()
+        {
+            Some("evaluation")
+        } else if config != self.config.as_ref() {
             Some("strict configuration")
         } else {
             None
@@ -124,6 +148,47 @@ impl SizedNifty {
     }
 }
 
+/// Loads one sizing context for [`SIZING_UNDERLYING`] and builds its Candidate
+/// signal column once, for the swept count and for the same rung's NIFTY
+/// commit to consume (D-2103, D-4783, D-4784). `config` is `Some` for the
+/// strict `ledger-v6` route and `None` for `ledger-all`.
+///
+/// # Errors
+///
+/// Another underlying, and every load and column-build refusal.
+pub(super) fn census(
+    root: &AdmittedRootV1,
+    spec: StoredContextLoadSpecV1<'_>,
+    config: Option<&StrictConfig>,
+    evaluation: &crate::candidate_universe::CandidateEvaluationInputsV1,
+) -> Result<SizedNifty, String> {
+    if spec.underlying != SIZING_UNDERLYING {
+        return Err(format!(
+            "a sizing census is taken on {SIZING_UNDERLYING}, not {}",
+            spec.underlying
+        ));
+    }
+    let context = super::load_bounded_stored_context_from_spec_v1(spec, root, config)?;
+    let column = crate::candidate_universe::PrebuiltSignalColumnV1::build(
+        &context.signal.bars,
+        &context.daily.references,
+        &context.minute.bars,
+        context.rung_seconds,
+        *evaluation,
+    )?;
+    Ok(SizedNifty {
+        root: root.path().to_path_buf(),
+        vendor: spec.vendor,
+        rung: spec.rung_name.to_owned(),
+        from: spec.from,
+        to: spec.to,
+        bounds: [spec.signal_bound, spec.minute_bound, spec.daily_bound],
+        config: config.cloned(),
+        context,
+        column,
+    })
+}
+
 #[cfg(test)]
 std::thread_local! {
     /// Test-only count of strict stored-context loads on this thread.
@@ -147,7 +212,8 @@ pub(crate) fn size_sweeper(
     evaluation: &crate::candidate_universe::CandidateEvaluationInputsV1,
 ) -> Result<(runner::Sweeper, Arc<Inputs>, SizedNifty), String> {
     let root = AdmittedRootV1::admit(root)?;
-    let context = load(
+    let sized = census(
+        &root,
         StoredContextLoadSpecV1 {
             vendor,
             underlying: SIZING_UNDERLYING,
@@ -158,13 +224,15 @@ pub(crate) fn size_sweeper(
             minute_bound: bounds.minute_records,
             daily_bound: bounds.daily_records,
         },
-        &root,
-        config,
+        Some(config),
+        evaluation,
     )?;
-    // The rows the Candidate column sweeps, warm-up excluded (D-2103).
-    let count = context.candidate_swept_v1(evaluation)?;
+    // The rows the Candidate column sweeps, warm-up excluded (D-2103). The
+    // column itself is kept for the NIFTY commit (G4-3, D-4783).
+    let count = sized.swept();
     let inputs = Arc::clone(
-        context
+        sized
+            .context
             .strict
             .as_ref()
             .ok_or("strict institutional sizing lost its input authority")?,
@@ -179,20 +247,6 @@ pub(crate) fn size_sweeper(
         min_hits,
         request.support_ppm,
     ));
-    let sized = SizedNifty {
-        root: root.path().to_path_buf(),
-        vendor,
-        rung: rung.to_owned(),
-        from: request.from,
-        to: request.to,
-        bounds: [
-            bounds.signal_records,
-            bounds.minute_records,
-            bounds.daily_records,
-        ],
-        config: config.clone(),
-        context,
-    };
     Ok((sweeper, inputs, sized))
 }
 
diff --git a/crates/cli/src/strict_v6_tests.rs b/crates/cli/src/strict_v6_tests.rs
index 485f1fd1..7cf3608e 100644
--- a/crates/cli/src/strict_v6_tests.rs
+++ b/crates/cli/src/strict_v6_tests.rs
@@ -490,6 +490,159 @@ mod strict_v6_fixture_tests {
         Ok(())
     }
 
+    #[test]
+    fn strict_v6_one_candidate_writer_serves_every_family_and_catches_up_on_others()
+    -> Result<(), String> {
+        // W2-cli3-4 / D-4780: every family commit opened the whole store-root
+        // Candidate ledger, O(R_total + C), sixteen times a `ledger-v6` run.
+        let fixture = StoredSuccessFixture::new()?;
+        let config = strict_fixture_config(&fixture)?;
+        let long = exit_policy(Side::Long)?;
+        let short = exit_policy(Side::Short)?;
+        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(1_000_000));
+        let mut writer = crate::candidate_universe::CandidateLedgerWriterV1::new();
+        crate::candidate_universe::LEDGER_WRITER_OPENS.with(|count| count.set(0));
+        crate::candidate_universe::LEDGER_CATCH_UP_RECEIPTS.with(|count| count.set(0));
+        let mut commit = |family: &str| {
+            commit_family_from_v6(
+                fixture_request(&fixture.source, family, &sweeper, &long, &short)?,
+                VerifiedBuildCommitV1(FIXTURE_COMMIT),
+                &|_, _, _| {},
+                Some(&config),
+                true,
+                None,
+                &mut writer,
+            )
+        };
+        let nifty = commit("NIFTY")?;
+        // Another writer appends to the same ledger between two families.
+        crate::candidate_universe::append_foreign_fixture_universe(
+            &fixture.source,
+            fixture_bounds()?.candidate,
+        )?;
+        let banknifty = commit("BANKNIFTY")?;
+        let rerun = commit("NIFTY")?;
+        assert_eq!(rerun.candidate_audit(), nifty.candidate_audit());
+        assert_eq!(
+            crate::candidate_universe::LEDGER_WRITER_OPENS.with(std::cell::Cell::get),
+            2,
+            "one writer open serves every family's append; the other is the foreign writer's"
+        );
+        assert_eq!(
+            crate::candidate_universe::LEDGER_CATCH_UP_RECEIPTS.with(std::cell::Cell::get),
+            1,
+            "the foreign completion was verified before the next append, not missed"
+        );
+        let fresh = crate::candidate_universe::CandidateUniverseLedgerV1::open_read(
+            &fixture.source,
+            fixture_bounds()?.candidate,
+        )?;
+        for family in [&nifty, &banknifty] {
+            assert_eq!(
+                fresh.reopen_audit(&family.candidate_audit().universe_id())?,
+                Some(family.candidate_audit())
+            );
+        }
+        Ok(())
+    }
+
+    #[test]
+    fn strict_v6_the_nifty_commit_consumes_the_sizing_column_once() -> Result<(), String> {
+        // G4-3 / D-4783: sizing built NIFTY's whole Candidate signal column to
+        // read its swept count and dropped it; the NIFTY commit built it again.
+        let fixture = StoredSuccessFixture::new()?;
+        let config = strict_fixture_config(&fixture)?;
+        let long = exit_policy(Side::Long)?;
+        let short = exit_policy(Side::Short)?;
+        let ledger_request = crate::ledger_all::LedgerAllRequest {
+            vendor: "zerodha",
+            from: FIXTURE_FROM,
+            to: FIXTURE_TO,
+            support_ppm: 1_000_000,
+            max_points: 1,
+            root: &fixture.base,
+        };
+        crate::candidate_universe::FULL_SIGNAL_COLUMN_BUILDS.with(|count| count.set(0));
+        let (sweeper, _, sized) = strict::size_sweeper(
+            &fixture.source,
+            Vendor::Zerodha,
+            &ledger_request,
+            "1min",
+            fixture_bounds()?,
+            &config,
+            &sizing_evaluation()?,
+        )?;
+        assert_eq!(
+            crate::candidate_universe::FULL_SIGNAL_COLUMN_BUILDS.with(std::cell::Cell::get),
+            1
+        );
+        let mut writer = crate::candidate_universe::CandidateLedgerWriterV1::new();
+        let nifty = commit_family_from_v6(
+            fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?,
+            VerifiedBuildCommitV1(FIXTURE_COMMIT),
+            &|_, _, _| {},
+            Some(&config),
+            true,
+            Some(sized),
+            &mut writer,
+        )?;
+        assert_eq!(
+            crate::candidate_universe::FULL_SIGNAL_COLUMN_BUILDS.with(std::cell::Cell::get),
+            1,
+            "the NIFTY commit consumed the sizing column and built none"
+        );
+        // The receipt is the unsized path's, byte for byte: an unsized commit
+        // of the same request reuses it.
+        let unsized_commit = commit_family_from_v6(
+            fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?,
+            VerifiedBuildCommitV1(FIXTURE_COMMIT),
+            &|_, _, _| {},
+            Some(&config),
+            true,
+            None,
+            &mut writer,
+        )?;
+        assert_eq!(unsized_commit.candidate_audit(), nifty.candidate_audit());
+        assert_eq!(
+            crate::candidate_universe::FULL_SIGNAL_COLUMN_BUILDS.with(std::cell::Cell::get),
+            2,
+            "an unsized commit builds its own column"
+        );
+
+        // A column built under other evaluation inputs serves no request.
+        let (_, _, sized) = strict::size_sweeper(
+            &fixture.source,
+            Vendor::Zerodha,
+            &ledger_request,
+            "1min",
+            fixture_bounds()?,
+            &config,
+            &sizing_evaluation()?,
+        )?;
+        let mut other = fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?;
+        other.availability = Availability::Present;
+        let source = AdmittedRootV1::admit(&fixture.source)?;
+        assert_eq!(
+            sized.differing_term(&other, source.path(), Some(&config)),
+            Some("evaluation")
+        );
+        let refused = commit_family_from_v6(
+            other,
+            VerifiedBuildCommitV1(FIXTURE_COMMIT),
+            &|_, _, _| {},
+            Some(&config),
+            true,
+            Some(sized),
+            &mut writer,
+        );
+        assert!(
+            matches!(&refused, Err(why) if why.contains("cannot serve a request with another evaluation")),
+            "{refused:?}",
+            refused = refused.as_ref().err()
+        );
+        Ok(())
+    }
+
     #[test]
     fn strict_v6_extinct_selection_keeps_both_family_sources_through_final_reauthentication()
     -> Result<(), String> {
@@ -586,6 +739,7 @@ mod strict_v6_fixture_tests {
             Some(&config),
             true,
             Some(sized),
+            &mut crate::candidate_universe::CandidateLedgerWriterV1::new(),
         )?;
         nifty.require_current()?;
         assert_eq!(
@@ -600,6 +754,7 @@ mod strict_v6_fixture_tests {
             Some(&config),
             true,
             None,
+            &mut crate::candidate_universe::CandidateLedgerWriterV1::new(),
         )?;
         banknifty.require_current()?;
         assert_eq!(
@@ -626,7 +781,26 @@ mod strict_v6_fixture_tests {
             (FIXTURE_FROM, FIXTURE_TO),
             fixture_bounds()?,
             &sizing_evaluation()?,
-        )?;
+        )?
+        .swept();
+        // D-4783: the census is NIFTY's alone, so another underlying is
+        // refused by name before any load rather than sized as if it were.
+        let other = crate::step3_orchestrator::stored_candidate_swept_v1(
+            &fixture.source,
+            Vendor::Zerodha,
+            ("BANKNIFTY", "1min"),
+            (FIXTURE_FROM, FIXTURE_TO),
+            fixture_bounds()?,
+            &sizing_evaluation()?,
+        );
+        assert!(
+            other
+                .as_ref()
+                .err()
+                .is_some_and(|why| why == "a sizing census is taken on NIFTY, not BANKNIFTY"),
+            "{:?}",
+            other.map(|sized| sized.swept())
+        );
         let retained = crate::stored::load_span(
             &fixture.source,
             Vendor::Zerodha,
@@ -747,6 +921,7 @@ mod strict_v6_fixture_tests {
             Some(&config),
             true,
             Some(sized),
+            &mut crate::candidate_universe::CandidateLedgerWriterV1::new(),
         );
         assert!(
             matches!(&refused, Err(why) if why.contains("cannot serve a request with another family")),
@@ -801,6 +976,7 @@ mod strict_v6_fixture_tests {
                 Some(&config),
                 true,
                 Some(sized),
+                &mut crate::candidate_universe::CandidateLedgerWriterV1::new(),
             )
             .is_err(),
             "a sized context whose source changed cannot be consumed"
diff --git a/crates/cli/tests/ledger_append_lookup_costs.rs b/crates/cli/tests/ledger_append_lookup_costs.rs
index e96300ab..ac849011 100644
--- a/crates/cli/tests/ledger_append_lookup_costs.rs
+++ b/crates/cli/tests/ledger_append_lookup_costs.rs
@@ -74,15 +74,22 @@ fn function<'a>(source: &'a str, signature: &str) -> &'a str {
 
 #[test]
 fn section_150_states_one_open_per_production_append_and_no_data_hash() {
-    // The source first: what the section must describe.
-    let door = function(CANDIDATE, "fn append_prepared_and_reverify(");
+    // The source first: what the section must describe. The carried writer
+    // opens the ledger once (D-4780), and the per-append commit opens nothing
+    // and re-reads only its block (D-1680).
+    let writer = method(CANDIDATE, "    fn append_with_held_or_opened(");
     assert_eq!(
-        door.matches("CandidateUniverseLedgerV1::open").count(),
+        writer.matches("CandidateUniverseLedgerV1::open").count(),
         1,
-        "the production append door opens the ledger more than once again; \
+        "the carried Candidate writer opens the ledger more than once again; \
          re-measure §150 before changing this test"
     );
-    assert!(door.contains("reverify_committed"));
+    let commit = function(CANDIDATE, "fn commit_and_reverify(");
+    assert!(!commit.contains("CandidateUniverseLedgerV1::open"));
+    assert!(commit.contains("reverify_committed"));
+    let append = method(CANDIDATE, "    fn append_complete_locked(");
+    assert!(append.contains("self.catch_up_locked()?"));
+    assert!(!append.contains("self.require_unchanged()?"));
     let generation = function(CANDIDATE, "fn file_generation(");
     assert!(
         !generation.contains("hash") && !generation.contains("blake3"),
@@ -93,19 +100,24 @@ fn section_150_states_one_open_per_production_append_and_no_data_hash() {
     for stale in [
         "and also hashes the data files",
         "Append, hashing, canonical-order validation and durability are proportional to the new block",
+        "opens the ledger on every call",
     ] {
         assert!(!text.contains(stale), "§150 still says `{stale}`");
     }
     for needed in [
-        "one production append is O(R+C) for that open plus O(new rows)",
+        "The run's first append opens the ledger, so it is O(R+C) for that open plus O(new rows)",
+        "so a later append is O(new rows + foreign new rows)",
+        "16 per run against one root, so a run's Candidate appends now cost one O(R+C) open",
+        "a carried writer does not see such a rewrite",
         "It is metadata only and hashes no data file",
-        "16 per run",
     ] {
         assert!(text.contains(needed), "§150 no longer says `{needed}`");
     }
-    let header = flat(CANDIDATE.get(..2_000).expect("the module header"));
+    let header = flat(CANDIDATE.get(..2_400).expect("the module header"));
     assert!(!header.contains("the sealed internal append is O(new rows)"));
-    assert!(header.contains("one production append costs O(rows + receipts)"));
+    assert!(!header.contains("The production append door opens the ledger once per call"));
+    assert!(header.contains("its first append opens the ledger, so it costs O(rows + receipts)"));
+    assert!(header.contains("instead of opening again (W2-cli3-4, D-4780)"));
 }
 
 #[test]
@@ -263,8 +275,17 @@ fn the_ledger_v6_route_and_replay_costs_are_stated() {
         "the route calls the door that always reloads NIFTY"
     );
     let sizing = function(STRICT_INPUTS, "pub(crate) fn size_sweeper(");
-    assert_eq!(sizing.matches("load(").count(), 1);
+    assert_eq!(sizing.matches("census(").count(), 1);
+    assert!(!sizing.contains("load("));
     assert!(sizing.contains("Ok((sweeper, inputs, sized))"));
+    let census = function(STRICT_INPUTS, "pub(super) fn census(");
+    assert_eq!(
+        census
+            .matches("load_bounded_stored_context_from_spec_v1(")
+            .count(),
+        1
+    );
+    assert_eq!(census.matches("PrebuiltSignalColumnV1::build(").count(), 1);
     let replay = function(LEDGER_V6, "fn replay_route(");
     assert!(replay.contains("run_route(request, out)?"));
     let start = LEDGER_V6.find("fn replay_route(").expect("replay_route");
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index b1fb767a..b398e049 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7057,7 +7057,16 @@ old line regex the same input and watched it pass.
 
 | Id | Invariant | Proof | |
 |---|---|---|---|
+| L1FF-01 | A carried Candidate writer opens its ledger once: a later append after another writer's append scans nothing, verifies exactly the one foreign completion, writes after the foreign block, reuses the foreign universe and its own first universe, and a fresh reader agrees with every audit it returned (D-4780) | `cli::candidate_universe::tests::a_carried_writer_opens_once_and_verifies_what_others_appended_since` | ✓ |
+| L1FF-02 | A carried writer refuses a corrupt foreign block by its seal and then reopens; refuses a rewrite that added no completion without rescanning; does not see an older-block rewrite that came with an append, which the next open refuses by its seal (the documented limit); refuses a replaced row or lock file by name; and refuses another root or other bounds (D-4780) | `cli::candidate_universe::tests::a_carried_writer_refuses_what_it_cannot_catch_up_on` | ✓ |
+| L1FF-03 | A catch-up starts at the handle's own committed cursor (an orphan's first row), a reuse right after a catch-up reverifies against the generations the catch-up measured, and a foreign orphan past the new completions refuses another universe (D-4780) | `cli::candidate_universe::tests::a_carried_writer_catches_up_from_its_own_committed_cursor` | ✓ |
+| L1FF-04 | A carried writer refuses a ledger whose last indexed completion moved (wiped and regrown), and a raw duplicate completion either of an indexed universe or among the new ones (D-4780) | `cli::candidate_universe::tests::a_carried_writer_refuses_history_that_moved_and_duplicate_completions` | ✓ |
+| L1FF-05 | One strict writer serves NIFTY, BANKNIFTY and a NIFTY rerun with one writable open (the other is the foreign writer's), catches up on the foreign completion once, and a fresh reader agrees (D-4780) | `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_one_candidate_writer_serves_every_family_and_catches_up_on_others` | ✓ |
 | L1FF-06 | Each Pre-Admission V1 and V2 append door, written or reused, scans its ledger once, and what it returns is what a fresh reader finds (D-4781) | `cli::pre_admission_data::tests::v1_and_v2_append_doors_scan_once_and_reread_only_their_pair` | ✓ |
 | L1FF-07 | The V1 and V2 pair re-read accepts the last written pair and an older reused pair, and refuses an older written pair, an absent authority, a moved generation, a Data or Completion record of another kind under a re-measured generation, an extra record and a replaced lock (D-4781) | `cli::pre_admission_data::tests::v1_reverify_refuses_every_disagreement_with_the_disk`, `cli::pre_admission_data::tests::v2_reverify_refuses_every_disagreement_with_the_disk` | ✓ |
 | L1FF-08 | Three witnesses over one fold hash no stream the fold hashed, each equals the unfolded mint's cohort id, witness id and candidate count, a fold refuses a tampered cohort identity, and a stored file changed after the fold refuses the next witness (D-4782) | `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_a_witness_hashes_no_stream_its_fold_already_hashed` | ✓ |
+| L1FF-09 | The NIFTY commit consumes its sizing census's signal column and builds none, its receipt equals the unsized commit's, and a request with other evaluation inputs is refused by name (D-4783) | `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_the_nifty_commit_consumes_the_sizing_column_once` | ✓ |
+| L1FF-10 | A prebuilt signal column gives a source byte-identical to the unsized build with no column build, and is refused for other evaluation inputs, another rung, or equal bars at another address (D-4783) | `cli::candidate_universe::tests::a_prebuilt_signal_column_serves_exactly_the_inputs_it_was_built_from` | ✓ |
+| L1FF-11 | `ledger-all`'s per-rung sizing makes 16 context loads per all-rung run, as many column builds as the named path, and one writable Candidate open serves all sixteen appends (D-4784, D-4780) | `cli::step3_orchestrator::all_rung_tests::all_rung_sizing_hands_each_nifty_context_to_its_commit_and_one_writer_serves_every_append` | ✓ |
+| L1FF-13 | Supersedes LBE-03's statement: §150 and the Candidate module header price a run's appends at one O(R+C) open plus caught-up new rows, keep the catch-up's limit, call the generation metadata only, and the writer opens once while `commit_and_reverify` opens nothing and re-reads its block (D-4780) | `cli::ledger_append_lookup_costs::section_150_states_one_open_per_production_append_and_no_data_hash` | ✓ |
 | L1FF-14 | §153 states the Pre-Admission door's one open and pair re-read and that an append still hashes the file; the doors call `reverify_committed` and no `open_read` (D-4781) | `cli::ledger_append_lookup_costs::a_pre_admission_append_door_opens_once_and_section_153_says_so` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 67b0a54f..81668e74 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65026,6 +65026,69 @@ guarantee D-1443 exists for. **Honest limit:** the `Landing` kill depends on
 test order. A rename that sorted a single-flight test ahead of it would
 restore the timeout, so the ordering is pinned in the test's own doc.
 
+### D-4780 — One Candidate ledger writer per run, caught up before each append — 2026-10-09
+
+**What was observed.** W2-cli3-4 was PARTIAL after D-1680. One production
+append no longer opened the Candidate ledger twice, but it still opened it once,
+and an open walks and re-seals every stored block: O(R + C). That ledger sits at
+the store root, and every run, rung and family shares it. `ledger-v6` and
+`ledger-all` each make 16 appends per run, so one run paid 16 x O(R_total + C),
+which grows with the ledger's history and not with the run's own work. D-1680
+rejected carrying the writer only because it changed three callers' ownership.
+
+**Decided.** A run carries one `CandidateLedgerWriterV1` through the whole
+commit chain. `ledger_v6::run_route` and the all-rung coordinator's phase one
+each create one before their rung loop. It passes through
+`commit_strict_candidate_pre_admission_authority_sized_v1` or
+`commit_stored_candidate_pre_admission_authority_carried_v1`, then
+`commit_family_from_v6`, `commit_loaded_stored_v1` and
+`commit_candidate_family_guarded_v6`, and reaches
+`ProducedCandidateUniverseV1::append_and_reopen_with`. The writer's first append
+opens the ledger. Every later append runs `append_complete` on the held handle.
+Under the exclusive writer lock, `catch_up_locked` then rechecks the three
+generations:
+
+- Unchanged generations cost three stats.
+- Otherwise `absorb_foreign_completions` catches up. The ledger must hold more
+  completions than the handle indexed. The last completion it indexed must
+  still be the one stored there. Each new completion's block is validated
+  against its seal from the handle's own committed cursor: the orphan's first
+  row when it holds one, the row total when not. A universe completed twice
+  refuses, and the orphan tail after the new blocks is re-scanned. Nothing is
+  indexed until every new completion has verified.
+- Any other change refuses: a replaced path, a change that added no completion,
+  or a moved last completion.
+
+The writer serves only the root and bounds it opened, as two separate checks,
+and any refusal discards the held handle, so the next append opens again rather
+than trusting a handle that refused. D-1700's ledger catch-up is the precedent.
+The single-family doors keep a fresh writer per call, so they still pay one open
+per call, as before. The read-only reopens that successor authentication makes
+are unchanged.
+
+A run now pays one O(R + C) open plus O(new rows + foreign new rows) per append.
+
+**Honest limit.** A metadata generation cannot tell an append from an append
+made together with an in-place rewrite of an older block. A carried writer
+therefore does not see such a rewrite: it verifies only the new blocks, and its
+witness is the last completion it indexed. The next full open re-validates every
+block and refuses the rewrite.
+`cli::candidate_universe::tests::a_carried_writer_refuses_what_it_cannot_catch_up_on`
+drives that case to the refusal. A rewrite that grows nothing is refused at once.
+
+**Rejected.** Re-scanning the whole ledger on any change that is not a pure
+append. A carried handle would then silently adopt a history rewritten under it.
+Refusing is the loud answer, and a rerun opens afresh.
+
+Tests:
+- `cli::candidate_universe::tests::a_carried_writer_opens_once_and_verifies_what_others_appended_since`
+  (fails at 1 against 0 scans on the per-append-open code).
+- `cli::candidate_universe::tests::a_carried_writer_refuses_what_it_cannot_catch_up_on`.
+- `cli::candidate_universe::tests::a_carried_writer_catches_up_from_its_own_committed_cursor`.
+- `cli::candidate_universe::tests::a_carried_writer_refuses_history_that_moved_and_duplicate_completions`.
+- `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_one_candidate_writer_serves_every_family_and_catches_up_on_others`.
+- `cli::step3_orchestrator::all_rung_tests::all_rung_sizing_hands_each_nifty_context_to_its_commit_and_one_writer_serves_every_append`.
+
 ### D-4781 — The Pre-Admission V1 and V2 append doors re-read only their pair — 2026-10-09
 
 **What was observed.** G4-4 found a sibling of W2-cli3-4 that no document
@@ -65111,3 +65174,77 @@ Tests:
   (fails at 15 against 3 passes on the re-hashing code; it also refuses a
   tampered cohort identity at the fold).
 - `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_one_oos_fold_serves_every_witness_of_its_cohort`.
+
+### D-4783 — The NIFTY commit consumes the signal column its sizing census built — 2026-10-09
+
+**What was observed.** G4-3 found that D-2103 had re-created W2-cli7-3's shape.
+`size_sweeper` built NIFTY's whole anchored Candidate signal column, which is
+Θ(S·W + Q + D) with the evaluator fold and the exact-minute overlay, only to
+read `census().swept`, and then dropped it. The NIFTY commit consumed the same
+`SizedNifty` context and built the identical column again, eight times per
+`ledger-v6` run.
+
+**Decided.** `SizedNifty` now holds a `PrebuiltSignalColumnV1`: the column, the
+evaluation inputs and rung it was built under, and the address and length of the
+three slices it was built from. `differing_term` adds `evaluation`, so a request
+with other widths, availability or thresholds is refused by name.
+`CandidateUniverseProductionSourceV1::new` takes an
+`Option<PrebuiltSignalColumnV1>`. When one is given, `into_column_for` makes
+three separate checks and refuses other evaluation inputs, another rung, or
+slices at another address or of another length. It then derives only the
+execution projection (`project_candidate_execution_column`). The column digest,
+source id and universe id are unchanged: the unit test compares them with the
+unsized build.
+
+The address check is sound because the context that owns the slices is moved,
+never copied, between the census and the commit, so its buffers keep their
+addresses. Equal bytes at another address refuse, and nothing falls back to a
+rebuild.
+
+**Rejected.** Hashing the slices to prove they are the same. That costs
+Θ(S + Q + D), which defeats the purpose. Relying on `SizedNifty`'s term checks
+alone was also rejected: they do not bind the slices.
+
+Tests:
+- `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_the_nifty_commit_consumes_the_sizing_column_once`
+  (fails at 2 against 1 builds on the rebuilding code).
+- `cli::candidate_universe::tests::a_prebuilt_signal_column_serves_exactly_the_inputs_it_was_built_from`.
+
+### D-4784 — `ledger-all` sizes each rung at its Candidate phase and hands that rung's NIFTY commit the context and column — 2026-10-09
+
+**What was observed.** G4-2 found the same problem in `ledger-all`, and worse.
+After D-2103, `build_sweepers` loaded NIFTY's full signal, one-minute and daily
+context and built its column for all eight rungs before the first commit, only
+to read the swept count. Each NIFTY commit then loaded and built them again: 24
+context loads per run where 16 suffice.
+
+**Decided.** The all-rung request now carries `AllRungSweepersV1`:
+
+- `Named` holds the eight caller-built sweepers, as before.
+- `SizedPerRung` is a sizing closure.
+
+`ledger-all` passes `SizedPerRung(size_rung ...)`. The coordinator calls it at
+the start of each rung's Candidate phase and uses the returned sweeper for both
+families. It hands the `SizedNifty` to that rung's NIFTY commit through
+`commit_stored_candidate_pre_admission_authority_carried_v1`. `size_rung` builds
+an ordinary (non-strict) census through the same
+`strict_v6_inputs::census` as `ledger-v6`, so the count and the commit use the
+same bytes. A run now makes 16 context loads (the test measured 24 before), with
+as many column builds as the named path. Peak memory holds one sized context at
+a time.
+
+**Consequences.**
+- A rung whose span cannot be sized now refuses when its own rung is reached.
+  That is after the earlier rungs' receipt-last appends, which an exact retry
+  reuses; it is the coordinator's documented recoverable-prefix contract.
+- The eight `rung sized` events now come after the population stage's start
+  event and interleave with its work, where before all eight came first.
+- A sizing refusal now also emits the stage's refusal event.
+- The report text is unchanged, and no stored byte or run identity changes.
+
+**Rejected.** Building all eight sizings first and holding them for the commits.
+That would raise peak memory eightfold.
+
+Tests:
+- `cli::step3_orchestrator::all_rung_tests::all_rung_sizing_hands_each_nifty_context_to_its_commit_and_one_writer_serves_every_append`
+  (fails at 24 against 16 loads on the up-front sizing code).
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 3ac2e55c..77d8c6a0 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -8018,15 +8018,27 @@ and worst-case O(1) in record count; a hash lookup is average O(1), not a
 worst-case collision guarantee, and a page costs O(P) for P returned rows.
 On an already-open handle, append, hashing, canonical-order validation and
 durability are proportional to the new block plus filesystem costs. The one
-production door, `append_produced_candidate_universe_v1`, opens the ledger on
-every call, so one production append is O(R+C) for that open plus O(new rows)
-to write the block and re-read it through the same handle. Before D-1680 it
-then dropped the handle and ran a second full `open_read`, so it cost two
-O(R+C) passes. `ledger-v6` makes one such append per rung per family, 16 per
-run against one root, so a run's Candidate appends cost O(16 x (R+C)) plus the
-rows written: they grow with the ledger's history, not only the new block.
-Universe construction would additionally walk the naturally extinct frontier
-and both dynamic grids. It is not O(1).
+production door, `append_produced_candidate_universe_v1`, appends through a
+`CandidateLedgerWriterV1` that one run carries across all its appends
+(W2-cli3-4, D-4780). The run's first append opens the ledger, so it is
+O(R+C) for that open plus O(new rows) to write the block and re-read it
+through the same handle. Every later append catches the held handle up under
+the exclusive writer lock instead of opening again: unchanged generations cost
+three stats, and a ledger another writer appended to is caught up by
+validating only the completions past the indexed count and the blocks they
+name, so a later append is O(new rows + foreign new rows). `ledger-v6` and
+`ledger-all` each make one append per rung per family, 16 per run against one
+root, so a run's Candidate appends now cost one O(R+C) open plus the rows
+written and caught up on. Before D-4780 every append opened the ledger, so
+the 16 cost O(16 x (R+C)) and grew with the ledger's history; before D-1680
+each also dropped the handle and ran a second full `open_read`. The catch-up
+has an honest limit: a metadata generation cannot tell an append from an
+append made together with an in-place rewrite of an older block, so a carried
+writer does not see such a rewrite. It re-reads the last completion it indexed
+as a witness, refuses a change that added no completion, and the next full
+open re-validates every block and refuses the rewrite. Universe construction
+would additionally walk the naturally extinct frontier and both dynamic grids.
+It is not O(1).
 
 On Unix, cached-generation refusal binds the held lock, row and receipt paths by
 device/inode, length and nanosecond modification/change times. It is metadata
@@ -15661,10 +15673,15 @@ per-candidate primitive from `CLAUDE.md` §3 rule 4.
 - **`ledger-all` sizing now builds NIFTY's Candidate column for each rung.**
   It used to read one signal span per rung for its length. It now loads the
   signal, one-minute and daily context exactly as the Candidate commit does,
-  and builds that column once to read its swept count. Per command, that is
-  eight extra context loads and column builds. The commit then loads its own
-  copy again. `ledger-v6` already held that context, so it only adds the
-  column build. Not measured; read off the source.
+  and builds that column once to read its swept count. Until D-4784 that was
+  eight extra context loads and column builds per command, because the commit
+  then loaded its own copy again, and until D-4783 `ledger-v6`, which already
+  held that context, built the column twice. Since D-4783 and D-4784 both
+  verbs hand the sizing context and column to the same rung's NIFTY commit,
+  which consumes them, so sizing adds no load and no column build;
+  `ledger-all` sizes each rung at that rung's Candidate phase and holds one
+  sized context at a time. Not measured; read off the source and counted by
+  the tests D-4783 and D-4784 name.
 
 - **The Zerodha day check (D-3001).** `pull::daycheck::compare` folds one
   instrument-month of minute bars to days, O(minutes), and merges two
-- 
2.43.0


From 3d1d755275ab9b60d73608c7d932f41113011823 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:31:07 +0000
Subject: [PATCH 4/4] Ledger V6 replay stays a full re-proof; its duplicated
 validation is removed (W2-cli7-2, D-4785)
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Before: `ledger-v6-replay` re-ran the complete route, and inside that
re-proof a production source re-ran its whole `validate()` (every stream,
both columns and the data term) in `anchored_search_v4` and again in
`produce_candidate_universe_v1`, and the Execution V3 replay ran it twice
more around its own replay. The source has no `&mut` path and borrows
immutable memory, so those passes re-hashed bytes construction had already
proved and nothing could have changed.

After: a production source is validated once, at construction; the four
repeated calls are gone (the Execution V3 receipt is still validated).
The replay itself stays a full re-proof by decision: a replay that trusted
its own sealed output would prove nothing that output does not already
claim. D-4785 and docs/06 state its exact per-run cost (16 strict loads,
16 column builds and projections, 16 Apriori sweeps, 16 Search V4 runs,
16 grid expansions, 16 Execution V3 column rebuilds memoized per family,
8 Statistics bootstraps, the successor recomputations, one Candidate open
plus catch-ups, one Pre-Admission open per family, and the OOS witness
work) and name G4's §A route manifest as the documented cheaper
alternative, which is a new authority and format and an owner decision.

Tests: production_and_its_replay_validate_each_source_once_at_construction
[2 validations against 1 before], strict_v6_a_family_commit_validates_its_source_once
[3 against 1 before], and the ledger-v6 route test in
ledger_append_lookup_costs. Invariants L1FF-12, L1FF-15. No output byte changes.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/candidate_universe.rs          | 66 ++++++++++++++++--
 crates/cli/src/ledger_v6.rs                   |  5 ++
 crates/cli/src/strict_v6_tests.rs             | 28 ++++++++
 .../cli/tests/ledger_append_lookup_costs.rs   | 22 ++++++
 docs/04-invariants.md                         |  2 +
 docs/05-decisions.md                          | 69 +++++++++++++++++++
 docs/06-limits.md                             | 32 +++++++--
 7 files changed, 213 insertions(+), 11 deletions(-)

diff --git a/crates/cli/src/candidate_universe.rs b/crates/cli/src/candidate_universe.rs
index 4929c130..f7986da3 100644
--- a/crates/cli/src/candidate_universe.rs
+++ b/crates/cli/src/candidate_universe.rs
@@ -985,6 +985,8 @@ impl<'a> CandidateUniverseProductionSourceV1<'a> {
         reason = "validation compares every retained Candidate and Search V4 source term in one fail-closed boundary"
     )]
     fn validate(&self) -> Result<(), CandidateUniverseRefusal> {
+        #[cfg(test)]
+        SOURCE_VALIDATIONS.with(|count| count.set(count.get().saturating_add(1)));
         require_rung(self.rung_seconds)?;
         if self.horizon.as_bars() == 0 {
             return Err("candidate production horizon is zero".to_owned());
@@ -1105,7 +1107,9 @@ impl<'a> CandidateUniverseProductionSourceV1<'a> {
         &self,
         sweeper: &Sweeper,
     ) -> Result<AnchoredSearchValidationV4, CandidateUniverseRefusal> {
-        self.validate()?;
+        // VALIDATED ONCE, AT CONSTRUCTION (W2-cli7-2, D-4785): this source has
+        // no `&mut` path, so its streams and columns are what construction
+        // proved, and re-hashing them here re-read memory that cannot change.
         let signal_length_micros = signal_length_micros(self.rung_seconds)?;
         let mut builder = CandidateSearchColumnBuilderV1 {
             full_signal: self.signal_bars,
@@ -2966,7 +2970,7 @@ pub(crate) fn produce_candidate_universe_v1<'a>(
     bounds: CandidateUniverseBoundsV1,
     on_level: &dyn Fn(&engine::Frontier, usize, u64),
 ) -> Result<ProducedCandidateUniverseV1<'a>, CandidateUniverseRefusal> {
-    source.validate()?;
+    // Validated once, at construction (W2-cli7-2, D-4785).
     let identities = production_identities(&source, sweeper)?;
     let descriptor = CandidateUniverseDescriptorV1::new(
         source.family,
@@ -4912,7 +4916,8 @@ fn build_execution_v3_replay_authority(
     receipt: CandidateUniverseReceiptV1,
     authenticated: &[AuthenticatedCandidatePopulationRowV1],
 ) -> Result<CandidateExecutionReplayAuthorityV1, CandidateUniverseRefusal> {
-    source.validate()?;
+    // The source was validated at its construction and cannot change, so it is
+    // not re-hashed before or after this replay (W2-cli7-2, D-4785).
     receipt.validate()?;
     let identities = production_identities_with_ladder(source, ladder)?;
     let descriptor = CandidateUniverseDescriptorV1::new(
@@ -5071,7 +5076,6 @@ fn build_execution_v3_replay_authority(
             "Candidate Execution V3 terminal disposition count changed during replay".to_owned(),
         );
     }
-    source.validate()?;
     Ok(CandidateExecutionReplayAuthorityV1 {
         receipt,
         parameters: [long_parameter, short_parameter],
@@ -5373,6 +5377,15 @@ thread_local! {
     pub(crate) static LEDGER_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
 }
 
+#[cfg(test)]
+thread_local! {
+    /// Test-only count of full production-source validations on this thread:
+    /// each one re-hashes every stream, both columns and the data term
+    /// (W2-cli7-2, D-4785).
+    pub(crate) static SOURCE_VALIDATIONS: std::cell::Cell<u64> =
+        const { std::cell::Cell::new(0) };
+}
+
 #[cfg(test)]
 thread_local! {
     /// Test-only count of writable Candidate ledger opens on this thread: the
@@ -10492,6 +10505,51 @@ mod tests {
         assert!(refused.contains("could not reserve"), "{refused}");
     }
 
+    #[test]
+    fn production_and_its_replay_validate_each_source_once_at_construction() {
+        // W2-cli7-2 / D-4785: inside the full re-proof, production re-ran the
+        // source's whole validation (every stream, both columns and the data
+        // term re-hashed) after construction had just proved it, and the
+        // Execution V3 replay ran it twice more around its own replay.
+        let fixture = ProductionFixture::new();
+        SOURCE_VALIDATIONS.with(|count| count.set(0));
+        let source = fixture.source();
+        assert_eq!(SOURCE_VALIDATIONS.with(std::cell::Cell::get), 1);
+        let (_, max_singleton_support, _) =
+            maximum_nontrivial_live_singleton_support(&source.signal_column);
+        let ladder = engine::Ladder::with_min_hits(max_singleton_support);
+        let sweeper = Sweeper::new(ladder);
+        let bounds = CandidateUniverseBoundsV1::new(1_000_000, 4)
+            .expect("production fixture bounds are explicit");
+        let produced = produce_candidate_universe_v1(&sweeper, source, bounds, &|_, _, _| {})
+            .expect("the fixture production completes");
+        assert_eq!(
+            SOURCE_VALIDATIONS.with(std::cell::Cell::get),
+            1,
+            "production re-validates nothing its source's construction proved"
+        );
+        let root = test_dir();
+        let written = produced
+            .append_and_reopen(root.path(), bounds)
+            .expect("the production commits");
+        let ledger =
+            CandidateUniverseLedgerV1::open_read(root.path(), bounds).expect("the ledger reopens");
+        let authenticated = ledger
+            .complete_population_rows(&written.audit())
+            .expect("the rows authenticate");
+        SOURCE_VALIDATIONS.with(|count| count.set(0));
+        let replay = fixture
+            .source()
+            .execution_v3_replay_authority(ladder, written.audit().receipt(), &authenticated)
+            .expect("the replay reproduces the dispositions");
+        assert_eq!(replay.receipt(), written.audit().receipt());
+        assert_eq!(
+            SOURCE_VALIDATIONS.with(std::cell::Cell::get),
+            1,
+            "the replay's source is validated at its construction only"
+        );
+    }
+
     /// D-0990: the series-invariant execution authority is sealed exactly once
     /// per Candidate block, however many closed masks are expanded through it,
     /// for production and for Execution V3 replay; a rerun is byte-identical.
diff --git a/crates/cli/src/ledger_v6.rs b/crates/cli/src/ledger_v6.rs
index 58dbe515..3260a72c 100644
--- a/crates/cli/src/ledger_v6.rs
+++ b/crates/cli/src/ledger_v6.rs
@@ -712,6 +712,11 @@ pub(crate) fn ledger_v6_replay(
 /// candidates; `docs/06-limits.md` states it (W2-cli7-2, D-1683). Invariant
 /// LBE-11 pins the statement; the route's per-rung load count is
 /// `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_the_nifty_commit_consumes_the_sizing_load_once`.
+///
+/// It stays a full re-proof by decision (D-4785): a replay that trusted its
+/// own sealed output would prove nothing that output does not already claim.
+/// The limits state its exact cost bound and the documented cheaper
+/// alternative, which needs an owner decision.
 fn replay_route(
     request: &LedgerAllRequest<'_>,
     from: (u16, u8),
diff --git a/crates/cli/src/strict_v6_tests.rs b/crates/cli/src/strict_v6_tests.rs
index 7cf3608e..fdd33099 100644
--- a/crates/cli/src/strict_v6_tests.rs
+++ b/crates/cli/src/strict_v6_tests.rs
@@ -546,6 +546,34 @@ mod strict_v6_fixture_tests {
         Ok(())
     }
 
+    #[test]
+    fn strict_v6_a_family_commit_validates_its_source_once() -> Result<(), String> {
+        // W2-cli7-2 / D-4785: the family commit validated its Candidate source
+        // at construction, again before Search V4 and again before production,
+        // each time re-hashing every stream, both columns and the data term.
+        let fixture = StoredSuccessFixture::new()?;
+        let config = strict_fixture_config(&fixture)?;
+        let long = exit_policy(Side::Long)?;
+        let short = exit_policy(Side::Short)?;
+        let sweeper = Sweeper::new(engine::Ladder::with_min_hits(1_000_000));
+        crate::candidate_universe::SOURCE_VALIDATIONS.with(|count| count.set(0));
+        drop(commit_family_from_v6(
+            fixture_request(&fixture.source, "NIFTY", &sweeper, &long, &short)?,
+            VerifiedBuildCommitV1(FIXTURE_COMMIT),
+            &|_, _, _| {},
+            Some(&config),
+            true,
+            None,
+            &mut crate::candidate_universe::CandidateLedgerWriterV1::new(),
+        )?);
+        assert_eq!(
+            crate::candidate_universe::SOURCE_VALIDATIONS.with(std::cell::Cell::get),
+            1,
+            "Search V4 and production re-validate nothing construction proved"
+        );
+        Ok(())
+    }
+
     #[test]
     fn strict_v6_the_nifty_commit_consumes_the_sizing_column_once() -> Result<(), String> {
         // G4-3 / D-4783: sizing built NIFTY's whole Candidate signal column to
diff --git a/crates/cli/tests/ledger_append_lookup_costs.rs b/crates/cli/tests/ledger_append_lookup_costs.rs
index ac849011..4a563396 100644
--- a/crates/cli/tests/ledger_append_lookup_costs.rs
+++ b/crates/cli/tests/ledger_append_lookup_costs.rs
@@ -296,12 +296,34 @@ fn the_ledger_v6_route_and_replay_costs_are_stated() {
             .expect("the rustdoc"),
     );
     assert!(doc.contains("O(full Step-4 route) per call"));
+    assert!(doc.contains("It stays a full re-proof by decision (D-4785)"));
+    for (source, signature) in [
+        (CANDIDATE, "    pub(crate) fn anchored_search_v4("),
+        (CANDIDATE, "fn build_execution_v3_replay_authority("),
+    ] {
+        let body = if signature.starts_with("    ") {
+            method(source, signature)
+        } else {
+            function(source, signature)
+        };
+        assert!(
+            !body.contains("source.validate()?") && !body.contains("self.validate()?"),
+            "`{signature}` re-validates its source again; re-measure the W2-cli7-2 bound"
+        );
+    }
+    let production = function(CANDIDATE, "pub(crate) fn produce_candidate_universe_v1<");
+    assert!(!production.contains("source.validate()?"));
 
     let chapter = chapter(CHAPTER);
     for needed in [
         "One `run_route` now makes 16 strict loads (8 rungs x 2 families)",
         "where before it made 24",
         "a rerun over fully committed authorities still costs the full Step-4 route",
+        "a replay that trusted its own sealed output would prove nothing",
+        "16 Apriori sweeps; 16 Search V4 walk-forward runs; 16 complete two-sided grid expansions",
+        "8 Statistics bootstraps of B draws each",
+        "a production source is validated once, at construction",
+        "The documented alternative is G4's §A",
     ] {
         assert!(
             chapter.contains(needed),
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index b398e049..f8978afa 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7068,5 +7068,7 @@ old line regex the same input and watched it pass.
 | L1FF-09 | The NIFTY commit consumes its sizing census's signal column and builds none, its receipt equals the unsized commit's, and a request with other evaluation inputs is refused by name (D-4783) | `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_the_nifty_commit_consumes_the_sizing_column_once` | ✓ |
 | L1FF-10 | A prebuilt signal column gives a source byte-identical to the unsized build with no column build, and is refused for other evaluation inputs, another rung, or equal bars at another address (D-4783) | `cli::candidate_universe::tests::a_prebuilt_signal_column_serves_exactly_the_inputs_it_was_built_from` | ✓ |
 | L1FF-11 | `ledger-all`'s per-rung sizing makes 16 context loads per all-rung run, as many column builds as the named path, and one writable Candidate open serves all sixteen appends (D-4784, D-4780) | `cli::step3_orchestrator::all_rung_tests::all_rung_sizing_hands_each_nifty_context_to_its_commit_and_one_writer_serves_every_append` | ✓ |
+| L1FF-12 | A production source is validated once, at construction: production and its Execution V3 replay re-validate nothing, and a strict family commit validates its source once (D-4785) | `cli::candidate_universe::tests::production_and_its_replay_validate_each_source_once_at_construction`, `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_a_family_commit_validates_its_source_once` | ✓ |
 | L1FF-13 | Supersedes LBE-03's statement: §150 and the Candidate module header price a run's appends at one O(R+C) open plus caught-up new rows, keep the catch-up's limit, call the generation metadata only, and the writer opens once while `commit_and_reverify` opens nothing and re-reads its block (D-4780) | `cli::ledger_append_lookup_costs::section_150_states_one_open_per_production_append_and_no_data_hash` | ✓ |
 | L1FF-14 | §153 states the Pre-Admission door's one open and pair re-read and that an append still hashes the file; the doors call `reverify_committed` and no `open_read` (D-4781) | `cli::ledger_append_lookup_costs::a_pre_admission_append_door_opens_once_and_section_153_says_so` | ✓ |
+| L1FF-15 | §169 prices a witness at O(M) plus the replay and states D-4782; the source seals with `with_digests` and the fold's witness calls `require_current`; the limits state the replay's full cost bound and the §A alternative (D-4782, D-4785) | `cli::ledger_append_lookup_costs::section_169_prices_the_oos_fold_once_per_cohort`, `cli::ledger_append_lookup_costs::the_ledger_v6_route_and_replay_costs_are_stated` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 81668e74..96af27aa 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65248,3 +65248,72 @@ That would raise peak memory eightfold.
 Tests:
 - `cli::step3_orchestrator::all_rung_tests::all_rung_sizing_hands_each_nifty_context_to_its_commit_and_one_writer_serves_every_append`
   (fails at 24 against 16 loads on the up-front sizing code).
+
+### D-4785 — Ledger V6 replay stays a full re-proof, and the duplicated validation inside it is removed — 2026-10-09
+
+**What was observed.** W2-cli7-2 was DOCUMENTED-ONLY. `replay_route` calls the
+complete `run_route`, and G4's §A sketches a cheaper replay that would trust a
+sealed route manifest. Before choosing, the route was searched for work
+duplicated inside the full re-proof that could go without trusting any sealed
+output. Five cases were found:
+
+- the NIFTY column built twice (D-4783);
+- `ledger-all`'s extra loads (D-4784);
+- one Candidate ledger open per append (D-4780);
+- the Pre-Admission doors' second open (D-4781);
+- a fifth, decided here. `CandidateUniverseProductionSourceV1::validate`
+  re-hashes every stream, both columns and the data term:
+  Θ(S + Q + D + E + columns). It ran at construction, then again before
+  Search V4 and again before production, and twice more around the Execution V3
+  replay on that replay's own freshly constructed source.
+
+**Decided.**
+- **Validate once, at construction.** The calls in `anchored_search_v4`,
+  `produce_candidate_universe_v1` and `build_execution_v3_replay_authority` are
+  removed. The source has no `&mut` path, its slices are borrowed immutably, its
+  columns are owned, and construction validated all of them, so the later calls
+  re-read memory that cannot change. A family commit now validates its source
+  once where it validated it three times, and the replay once where it validated
+  it three times.
+- **The replay stays a full re-proof.** A replay that trusted its own sealed
+  output would prove nothing that output does not already claim. Its value is
+  that every authority is re-derived from the bytes on disk under the running
+  binary.
+
+**The exact cost of one replay, which is one `ledger-v6` run.**
+- 16 strict loads, 8 rungs x 2 families, each O(M + B + source bytes).
+- 16 Candidate column builds. The NIFTY build is the sizing census's.
+- 16 execution projections.
+- 16 Apriori sweeps.
+- 16 Search V4 walk-forward runs.
+- 16 complete two-sided grid expansions.
+- 16 Execution V3 replay column rebuilds, memoized to one per family (D-0994).
+- 8 Statistics bootstraps of B draws each.
+- 8 recomputations each of Admission V4, Finalization V4, Population V6,
+  Execution V4 and Selection V6.
+- One writable Candidate ledger open plus its catch-ups.
+- One Pre-Admission door open per family.
+- Every successor ledger's open and reopen.
+- The OOS witness work: one fold per family cohort and O(M) plus the Runner
+  replay per witness (D-4782).
+
+**Documented alternative: G4's §A.** It adds a route manifest keyed by both
+universe ids and every policy term, and a rehydrated selection whose winners
+are re-derived and checked against the sealed digests. It would remove the
+sweeps, the Search V4 runs, the grid expansions, the bootstraps and the
+downstream recomputation. That would leave 16 loads, 16 column builds and at
+most 200 winner grid evaluations. It needs a new authority and format, and a
+contract change: Global Replay V4 would accept a sealed record plus rebuilt
+winners where it now accepts only a live, re-proved chain. That is an owner
+decision, and it is not taken here.
+
+**Rejected.** Handing the production signal column to the Execution V3 replay,
+which would remove those 16 rebuilds. It would keep a column copy alive per
+family until that family's replay. `ledger-all` commits all 16 families in
+phase one before any successor runs, so peak memory would rise by up to 16
+columns.
+
+Tests:
+- `cli::candidate_universe::tests::production_and_its_replay_validate_each_source_once_at_construction`.
+- `cli::step3_orchestrator::tests::strict_v6_fixture_tests::strict_v6_a_family_commit_validates_its_source_once`.
+- `cli::ledger_append_lookup_costs::the_ledger_v6_route_and_replay_costs_are_stated`.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 77d8c6a0..6fb00194 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15447,14 +15447,32 @@ Holding the context until the NIFTY commit does not raise the peak: the old
 route held one context at a time and so does this one.
 
 W2-cli7-2: `ledger-v6-replay` (and every `ledger-v6` rerun) runs the complete
-route before anything decides reuse. Reuse is keyed by data digest, the data
+route before anything decides reuse, and since D-4785 that is a decision, not
+an omission: a replay that trusted its own sealed output would prove nothing
+that output does not already claim. Reuse is keyed by data digest, the data
 digest needs the strict load, and the load is the dominant term, so a rerun
-over fully committed authorities still costs the full Step-4 route: 16 strict
-loads, eight Search V4 sweeps per family and every commit's reopen. That is
-not O(1) and not proportional to new work. Making it so would need a durable
-request-keyed index of committed routes that a replay could consult before
-loading, which is a new authority and a new format; D-1683 does not add one and
-states the cost here instead.
+over fully committed authorities still costs the full Step-4 route. One replay,
+which is one `ledger-v6` run, costs exactly: 16 strict loads (8 rungs x 2
+families), each O(M + B + source bytes); 16 Candidate column builds (NIFTY's is
+the sizing census's, D-4783) and 16 execution projections; 16 Apriori sweeps;
+16 Search V4 walk-forward runs; 16 complete two-sided grid expansions; 16
+Execution V3 replay column rebuilds, memoized to one per family (D-0994); 8
+Statistics bootstraps of B draws each; 8 recomputations each of Admission V4,
+Finalization V4, Population V6, Execution V4 and Selection V6; one writable
+Candidate ledger open plus its catch-ups (D-4780); one Pre-Admission door open
+per family (D-4781); every successor ledger's open and reopen; and the OOS
+witness work, one fold per family cohort and O(M) plus the Runner replay per
+witness (D-4782). That is not O(1) and not proportional to new work. D-4785
+removed the work this re-proof duplicated without trusting anything sealed: a
+production source is validated once, at construction, where it was validated
+three times per family commit and three times per Execution V3 replay, each
+re-hashing every stream, both columns and the data term. The documented
+alternative is G4's §A: a route manifest keyed by both universe ids and every
+policy term, plus a rehydrated selection whose winners are re-derived and
+checked against the sealed digests, would leave 16 loads, 16 column builds and
+at most 200 winner grid evaluations. It is a new authority, a new format and a
+Global Replay V4 contract change, which is an owner decision; D-4785 does not
+take it.
 
 ## A sweep-evidence ranking is buffered whole before its one write — D-1741, 3 October 2026
 
-- 
2.43.0


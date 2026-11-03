# red-BASE-MSUM-BACKFILL-MS1.txt.md

Every new test of BASE-MSUM-BACKFILL-MS1, each shown red under a hand change at `ceb28bb90b017814c70790afdc5e0a46a749a0b4` (CLOUD.md item 7). Four batches; in each, a test is sensitive to one hand change of its batch only, and its failure line names what that change broke. Built and run in the Route B environment (pinned nightly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, opt-level 0, instrumented; root host, the permission test re-running itself as uid 65534). After each batch the hand changes were reverted with `git checkout -- crates` and the tree checked clean; none was ever committed. The store-steps test (`the_msum_store_half_is_a_pending_step_for_each_feed`) is red at the tip itself, by design, until the Mac's rows land. One fenced block holds the record.

`````text
=== BATCH A
== batch A at ceb28bb90b01 2026-11-03 17:41:15 IST
hand changes:  2 files changed, 8 insertions(+), 10 deletions(-)
build rc=0
restored: tree clean at ceb28bb90b01
--- the hand changes (git diff, reverted after the run):
diff --git a/crates/store/src/month_summary_writer.rs b/crates/store/src/month_summary_writer.rs
index 34ce02ad..8ad3f8c3 100644
--- a/crates/store/src/month_summary_writer.rs
+++ b/crates/store/src/month_summary_writer.rs
@@ -278,11 +278,11 @@ impl BarFile {
         path: StorePath<'_>,
         symbol_id: u32,
     ) -> Result<Summarized, StoreError> {
-        let (mut file, _served) = Self::open_held(root, path, symbol_id, Hold::Shared)?;
+        let (mut file, _served) = Self::open_held(root, path, symbol_id, Hold::Exclusive)?;
         let sidecar = SummarySidecar {
             path: path_beside(&file.bars_path),
         };
-        if file.header.n_valid == 0 {
+        if file.header.n_valid == u64::MAX {
             return Ok(Summarized::Empty);
         }
         let because = match crate::month_summary::read_beside(
@@ -300,8 +300,7 @@ impl BarFile {
         };
         // ONE DOOR AT A TIME, locked on the handle the door already opened:
         // no second open of the bar file, so no second way for it to fail.
-        let _one_door = Flock::try_lock(&file.bars, file.bars_path.as_path())
-            .map_err(|refusal| lock_fault(&file.bars_path, refusal))?;
+        let _one_door = ();
         file.msum = Some(sidecar.clone());
         let summary = file.whole_summary()?;
         write_image(&sidecar, &summary.file_image(), Durable::Synced)?;
@@ -335,7 +334,7 @@ fn write_image(sidecar: &SummarySidecar, image: &[u8], durable: Durable) -> Resu
     // step refused BEFORE a staging file is made: a rename over it would fail
     // after the write and leave the staging name behind.
     if let Ok(meta) = fs::symlink_metadata(&sidecar.path)
-        && !meta.is_file()
+        && meta.len() == u64::MAX
     {
         return Err(StoreError::Io {
             path: sidecar.path.clone(),
@@ -346,7 +345,7 @@ fn write_image(sidecar: &SummarySidecar, image: &[u8], durable: Durable) -> Resu
     }
     let staged = crate::staging::create(&staging).map_err(|why| match why {
         crate::staging::StagingError::Io { source, .. } => {
-            classify(&staging, Action::Open, &source)
+            classify(&sidecar.path, Action::Open, &source)
         }
         // A NAME HELD BY SOMETHING THAT IS NOT A DEAD WRITER'S LEFTOVER: a
         // second name of a file, a link, a directory, a live writer. Nothing
@@ -358,9 +357,9 @@ fn write_image(sidecar: &SummarySidecar, image: &[u8], durable: Durable) -> Resu
             code: None,
         },
     })?;
-    write_fully(&staged.file, &staging, 0, image)?;
+    let _ = write_fully(&staged.file, &staging, 0, image);
     if matches!(durable, Durable::Synced) {
-        fault(sync_hooked(&staged.file), &staging, Action::Sync)?;
+        
     }
     drop(staged);
     fault(
diff --git a/crates/store/src/sort_index_writer.rs b/crates/store/src/sort_index_writer.rs
index a9e6dae1..27dfed77 100644
--- a/crates/store/src/sort_index_writer.rs
+++ b/crates/store/src/sort_index_writer.rs
@@ -382,7 +382,7 @@ impl BarFile {
             return Err(StoreError::NotABarPath { found: path.file() });
         }
         let served = crate::generation::served(store_root, path)?;
-        let served_root = served.root(store_root);
+        let served_root = store_root.to_path_buf();
         let root = served_root.as_path();
         let bars_path = path.to_path_buf(root);
         let lock_path = path.with_file(FileKind::Lock).to_path_buf(root);
@@ -399,7 +399,6 @@ impl BarFile {
         // A MONTH FILED UNDER ANOTHER NAME GETS NO SIDECAR (DB-B65): every
         // reader refuses it, so an index or a summary written beside it is a
         // write no reader reads, under a name its own files deny (D-7430).
-        refuse_misfiled(root, path, &bars_path)?;
         let file = Self::validated(
             bars,
             bars_path,
--- each test under them (crate, test, result, first failure lines):
RED-CHECK store month_summary::tests::the_door_refuses_a_writer_and_never_a_reader :: FAILED :: called `Result::unwrap()` on an `Err` value: Locked { path: "/dev/shm/msum-tmp/brutex-msum-door-readers-9728-0/bars/gdfl/NSE/INDEX/NIFTY/1s/2031-05.lock" }    0: __rustc::rust_begin_unwind 
RED-CHECK store month_summary::tests::a_second_door_is_refused_by_the_bar_files_own_lock :: FAILED :: assertion `left == right` failed   left: Ok(Built("absent: no month summary is kept beside the month"))  right: Err(Locked { path: "/dev/shm/msum-tmp/brutex-msum-door-doors-9745-0/bars/gdfl/NSE/INDEX/NIFTY/1s/2031-05.bin" }) 
RED-CHECK store file::file_fault_tests::a_full_disk_leaves_no_partial_summary_under_its_name :: FAILED :: called `Result::unwrap_err()` on an `Ok` value: Built("absent: no month summary is kept beside the month")    0: __rustc::rust_begin_unwind 
RED-CHECK store file::file_fault_tests::a_refused_sync_leaves_the_name_free :: FAILED :: called `Result::unwrap_err()` on an `Ok` value: Built("absent: no month summary is kept beside the month")    0: __rustc::rust_begin_unwind 
RED-CHECK store month_summary::tests::the_door_names_an_empty_and_a_small_month_and_writes_nothing :: FAILED :: assertion `left == right` failed   left: Ok(BelowThreshold)  right: Ok(Empty) 
RED-CHECK store month_summary::tests::a_name_held_by_a_directory_is_refused_before_a_byte_is_written :: FAILED :: assertion `left == right` failed   left: Err(IsADirectory { path: "/dev/shm/msum-tmp/brutex-msum-door-names-9813-0/bars/gdfl/NSE/INDEX/NIFTY/1s/2031-05.sum", action: Write })  right: Err(Io { path: "/dev/shm/msum-tmp/brutex-msum-door-names-9813-0/bars/gdfl/NSE/INDEX/NIFTY/1s/2031-05.sum", action: Wr
RED-CHECK store month_summary::tests::a_staging_name_the_host_refuses_is_the_doors_refusal :: FAILED :: child month_summary::tests::a_staging_name_the_host_refuses_is_the_doors_refusal (uid 65534) failed or ran no test:  running 1 test 
RED-CHECK store month_summary::tests::a_month_filed_under_another_name_is_refused_by_the_door :: FAILED :: assertion `left == right` failed   left: Ok(Built("absent: no month summary is kept beside the month"))  right: Err(Misfiled { path: "/dev/shm/msum-tmp/brutex-msum-door-misfiled-9849-0/bars/zerodha/NSE/INDEX/NIFTY/1s/2031-05.bin", sidecar: "/dev/shm/msum-tmp/brutex-msum-door-misfiled-9849-0/bars/zer
RED-CHECK cli upkeep_command::tests::a_run_builds_beside_a_reader_and_refuses_a_writers_month_by_name :: FAILED :: UPKEEP RUN — msum for gdfl under /dev/shm/msum-tmp/brutex-cli-upkeep-9867-0   run 1000: 1 worker(s), 3 item(s) given, 0 skipped as closed clean by the run(s) resumed   items               3 of the 3 month file(s) in scope (--all: every one); 0 link(s) never followed, 0 stray name(s) left alone; 0 
RED-CHECK cli upkeep_command::tests::a_month_repaired_mid_run_keeps_its_own_summary :: FAILED :: assertion `left == right` failed: UPKEEP RUN — msum for gdfl under /dev/shm/msum-tmp/brutex-cli-upkeep-9885-0   run 1000: 1 worker(s), 2 item(s) given, 0 skipped as closed clean by the run(s) resumed   items               2 of the 2 month file(s) in scope that have no file of this kind; 0 link(s) 

=== BATCH B
== batch B at ceb28bb90b01 2026-11-03 17:45:26 IST
hand changes:  2 files changed, 5 insertions(+), 5 deletions(-)
build rc=0
restored: tree clean at ceb28bb90b01
--- the hand changes (git diff, reverted after the run):
diff --git a/crates/pull/src/upkeep.rs b/crates/pull/src/upkeep.rs
index f6d37de3..61c63661 100644
--- a/crates/pull/src/upkeep.rs
+++ b/crates/pull/src/upkeep.rs
@@ -1243,7 +1243,7 @@ fn drive<T: Sync>(
     let todo: Vec<(usize, &(String, T))> = plan
         .iter()
         .enumerate()
-        .filter(|(_, (key, _))| done.as_ref().is_none_or(|done| !done.contains(key)))
+        .filter(|_| done.is_some() || done.is_none())
         .collect();
     let skipped = plan.len() - todo.len();
     let workers = ask.kind.workers(ask.workers).min(todo.len().max(1));
@@ -1430,7 +1430,7 @@ impl Lacks {
     /// This set without `kind`.
     #[must_use]
     pub const fn without(self, kind: Kind) -> Self {
-        Self(self.0 & !Self::bit(kind))
+        Self(self.0 & Self::bit(kind))
     }
 
     /// The kinds lacked, in [`Kind::ALL`] order.
@@ -1738,7 +1738,7 @@ fn auction_sidecar(name: &str) -> bool {
 /// look owed; the run then says `small` for it.
 fn summary_owed_by_len(len: u64) -> bool {
     len.checked_sub(store::format::HEADER_LEN)
-        .is_none_or(|rows| !store::time_index::below_threshold(rows / store::format::RECORD_STRIDE))
+        .is_some_and(|rows| !store::time_index::below_threshold(rows / store::format::RECORD_STRIDE))
 }
 
 /// Every month file in one rung directory.
@@ -1797,7 +1797,7 @@ fn walk_rung(
         // ONE MORE `lstat`, ONLY FOR A MONTH WITH NO `.sum`: a step's plan must
         // be able to read zero, so a month too short to owe a summary is not
         // listed as lacking one (D-7430).
-        if lacks.has(Kind::MonthSummary)
+        if lacks.has(Kind::Recount)
             && fs::symlink_metadata(store_path.to_path_buf(&served))
                 .is_ok_and(|bars| !summary_owed_by_len(bars.len()))
         {
diff --git a/crates/store/src/layout.rs b/crates/store/src/layout.rs
index e9fbaf6c..abc03c50 100644
--- a/crates/store/src/layout.rs
+++ b/crates/store/src/layout.rs
@@ -189,7 +189,7 @@ impl Layout {
     /// sidecar later. `Header::decode_parts` resolves a version while decoding.
     /// Whether a geometry may be handed to a BAR reader is decided by
     /// `crate::file::table_of`, from the file kind, and never by this list.
-    pub const KNOWN: &'static [Self] = &[Self::V2, Self::V3, Self::OVERLAY, Self::GREEKS];
+    pub const KNOWN: &'static [Self] = &[Self::V2, Self::V3, Self::OVERLAY, Self::GREEKS, Self::declared(4, *b"BRUTEXB4", SLOT_COUNT, 48, RECORDS_PER_BLOCK)];
 
     /// The version this build writes. Older versions are read, never written.
     pub const CURRENT: Self = Self::V3;
--- each test under them (crate, test, result, first failure lines):
RED-CHECK pull upkeep::tests::the_length_of_a_bar_file_says_when_no_summary_is_owed :: FAILED :: an empty file proves nothing    0: __rustc::rust_begin_unwind 
RED-CHECK pull upkeep::tests::a_set_of_lacks_without_a_kind_keeps_the_rest :: FAILED :: assertion `left == right` failed   left: [MonthSummary]  right: [DayIndex, SortIndex] 
RED-CHECK pull upkeep::tests::the_walk_counts_a_month_too_short_to_owe_a_summary_apart :: FAILED :: assertion `left == right` failed   left: [("bars/gdfl/NSE/INDEX/NIFTY/1s/2021-01.bin", [DayIndex, SortIndex, MonthSummary]), ("bars/gdfl/NSE/INDEX/NIFTY/1s/2021-02.bin", [MonthSummary]), ("bars/gdfl/NSE/INDEX/NIFTY/1s/2021-03.bin", [DayIndex, SortIndex, MonthSummary])]  right: [("bars/gdfl/NSE/INDEX
RED-CHECK pull upkeep::tests::the_length_rule_holds_for_every_bar_layout :: FAILED :: assertion `left == right` failed: Layout { version: 4, magic: [66, 82, 85, 84, 69, 88, 66, 52], slot_count: 2, record_stride: 48, records_per_block: 73 }   left: (32768, 48)  right: (32768, 56) 
RED-CHECK pull upkeep::tests::the_walk_lists_every_month_and_what_it_lacks :: FAILED :: assertion `left == right` failed   left: [("bars/gdfl/NSE/FNO/NIFTY/2019-03-28-1150000-CE/1s/2019-03.bin", [DayIndex, SortIndex, MonthSummary]), ("bars/gdfl/NSE/INDEX/NIFTY/1min/2019-04.bin", [DayIndex, SortIndex, MonthSummary]), ("bars/gdfl/NSE/INDEX/NIFTY/1s/2019-03.bin", [SortIndex]), ("bars/gdfl
RED-CHECK cli upkeep_command::tests::a_plan_names_the_lacking_months_and_none_after_the_step :: FAILED :: assertion `left == right` failed: UPKEEP PLAN — gdfl (the whole feed) under /dev/shm/msum-tmp/brutex-cli-upkeep-10374-0: 4 month file(s) (nothing is written)   NSE/INDEX/NIFTY 1s                             4 month(s); lacks .tix      4, .srt      4, .sum      4   TOTAL lacks .tix 4, .srt 4, .sum 
RED-CHECK cli upkeep_command::tests::an_empty_and_a_zero_length_month_are_told_apart :: FAILED :: assertion `left == right` failed: UPKEEP PLAN — gdfl (the whole feed) under /dev/shm/msum-tmp/brutex-cli-upkeep-10391-0: 2 month file(s) (nothing is written)   NSE/INDEX/NIFTY 1s                             2 month(s); lacks .tix      2, .srt      2, .sum      2   TOTAL lacks .tix 2, .srt 2, .sum 
RED-CHECK cli upkeep_command::tests::a_whole_feed_gets_its_sort_index_in_one_run :: FAILED :: UPKEEP PLAN — gdfl (the whole feed) under /dev/shm/msum-tmp/brutex-cli-upkeep-10408-0: 7 month file(s) (nothing is written)   NSE/FNO/NIFTY/* 1s                             1 month(s); lacks .tix      1, .srt      1, .sum      1   NSE/INDEX/BANKNIFTY 1s                         1 month(s); lacks .t
RED-CHECK cli upkeep_command::tests::a_refused_month_then_a_resume_at_another_worker_count_builds_only_it :: FAILED :: UPKEEP RESUME — msum for gdfl under /dev/shm/msum-tmp/brutex-cli-upkeep-10425-1   run 1001 (resumes run 1000): 3 worker(s), 3 item(s) given, 0 skipped as closed clean by the run(s) resumed   items               3 of the 3 month file(s) in scope (--all: every one); 0 link(s) never followed, 0 stray

=== BATCH C
== batch C at ceb28bb90b01 2026-11-03 17:48:29 IST
hand changes:  4 files changed, 9 insertions(+), 15 deletions(-)
build rc=0
restored: tree clean at ceb28bb90b01
--- the hand changes (git diff, reverted after the run):
diff --git a/crates/cli/src/msum_builder.rs b/crates/cli/src/msum_builder.rs
index b8c6cce6..72182b23 100644
--- a/crates/cli/src/msum_builder.rs
+++ b/crates/cli/src/msum_builder.rs
@@ -175,11 +175,7 @@ fn checked(door: Result<Summarized, StoreError>, read_back: &dyn Fn() -> ReaderV
         Summarized::Empty => return Line::Empty,
         Summarized::BelowThreshold => match read_back() {
             Ok((n_valid, _, _)) => return Line::Small { n_valid },
-            Err(why) => {
-                return Line::Refused {
-                    why: format!("the step finished and the read-back was refused: {why}"),
-                };
-            }
+            Err(_) => return Line::Small { n_valid: 0 },
         },
         other => {
             return Line::Refused {
@@ -192,10 +188,7 @@ fn checked(door: Result<Summarized, StoreError>, read_back: &dyn Fn() -> ReaderV
             n_valid,
             because,
         }),
-        Ok((n_valid, _, Err(reader))) => Line::NotSummarized {
-            n_valid,
-            reader: reader.to_string(),
-        },
+        Ok((n_valid, _, Err(_))) => Line::Kept { n_valid },
         Err(why) => Line::Refused {
             why: format!("the step finished and the read-back was refused: {why}"),
         },
diff --git a/crates/cli/src/upkeep_command.rs b/crates/cli/src/upkeep_command.rs
index 7ab1804b..9c74772b 100644
--- a/crates/cli/src/upkeep_command.rs
+++ b/crates/cli/src/upkeep_command.rs
@@ -485,7 +485,7 @@ fn items(
             );
             // THE SMALL MONTHS ARE SAID, never dropped silently (D-7430).
             let said = if kind == Kind::MonthSummary {
-                format!("{said}; {} under 1,024 bars owe none", walked.sum_small)
+                format!("{said}; {} small", walked.sum_small)
             } else {
                 said
             };
@@ -844,7 +844,7 @@ fn plan(out: &mut String, root: &Path, vendor: Vendor, scope: &Scope) -> u8 {
     );
     let _ = writeln!(
         out,
-        "  SMALL: {} month(s) with no .sum are under 1,024 bars by their length and owe none \
+        "  {} month(s) with no .sum are under 1,024 bars by their length and owe none \
          (D-4933), so they are not among those lacking one; a run takes them only under --all",
         walked.sum_small
     );
@@ -1037,7 +1037,7 @@ fn run(
     if kind == Kind::MonthSummary {
         let _dropped_when_filtered = telemetry::emit(&msum_given(vendor, &plan, &said));
     }
-    let busy = || live_run(root, vendor);
+    let busy = || -> Option<String> { None };
     let ask = Ask {
         vendor,
         kind,
@@ -1083,7 +1083,7 @@ fn msum_given<'a>(vendor: Vendor, plan: &[(String, Item)], said: &'a str) -> tel
     telemetry::Event::info("store.sum", "months lacking a summary, at a run's start")
         .with("vendor", vendor.as_str())
         .with("items", plan.len())
-        .with("lacking", lacking)
+        .with("lacking", plan.len())
         .with("scope", said)
 }
 
diff --git a/crates/pull/src/upkeep.rs b/crates/pull/src/upkeep.rs
index f6d37de3..eb603b15 100644
--- a/crates/pull/src/upkeep.rs
+++ b/crates/pull/src/upkeep.rs
@@ -1213,7 +1213,7 @@ pub fn run<T: Sync>(
         Some(resumed) => Some(done_by_chain(&journal.entries()?, resumed)?),
         None => None,
     };
-    let id = (ask.now)().max(journal.newest_id()?.saturating_add(1));
+    let id = (ask.now)();
     let begun = Begun {
         id,
         plan,
@@ -1788,6 +1788,7 @@ fn walk_rung(
         let served = store::generation::served(at.root, store_path)
             .map_err(|why| format!("{}: {why}", path.display()))?
             .root(at.root);
+        if fs::symlink_metadata(store_path.with_file(FileKind::SealedBars).to_path_buf(&served)).is_ok() { continue; }
         let mut lacks = Lacks::of(Kind::ALL.into_iter().filter(|kind| {
             kind.sidecar().is_some_and(|file| {
                 fs::symlink_metadata(store_path.with_file(file).to_path_buf(&served))
diff --git a/crates/store/src/month_summary_writer.rs b/crates/store/src/month_summary_writer.rs
index 34ce02ad..1b2550f9 100644
--- a/crates/store/src/month_summary_writer.rs
+++ b/crates/store/src/month_summary_writer.rs
@@ -312,7 +312,7 @@ impl BarFile {
 
 /// Writes `summary` as the month's `.sum`, by staging and rename.
 fn write_summary(sidecar: &SummarySidecar, summary: &MonthSummary) -> Result<(), String> {
-    write_image(sidecar, &summary.file_image(), Durable::Unsynced).map_err(|why| why.to_string())
+    { let _ = (sidecar, summary, Durable::Unsynced); Ok(()) }
 }
 
 /// Whether a summary is on the disk before its name appears.
--- each test under them (crate, test, result, first failure lines):
RED-CHECK cli upkeep_command::tests::a_writer_commit_leaves_no_month_without_its_summary :: FAILED :: assertion failed: summarised(&root.0, at, "NIFTY")    0: __rustc::rust_begin_unwind 
RED-CHECK cli upkeep_command::tests::a_sealed_month_beside_its_raw_form_is_summarised_and_left_as_it_was :: FAILED :: assertion `left == right` failed: UPKEEP RUN — msum for gdfl REFUSED: the whole feed holds no month file under bars/gdfl (0 link(s) never followed, 0 stray name(s) left alone): a run of no item is refused, never read as nothing pending. Nothing was written.    left: 1 
RED-CHECK cli upkeep_command::tests::an_import_during_a_run_stops_it_and_a_resume_finishes_the_same_bytes :: FAILED :: assertion `left == right` failed: UPKEEP RUN — msum for gdfl under /dev/shm/msum-tmp/brutex-cli-upkeep-10755-1   run 1000: 1 worker(s), 4 item(s) given, 0 skipped as closed clean by the run(s) resumed   items               4 of the 4 month file(s) in scope that have no file of this kind; 0 link(s)
RED-CHECK cli upkeep_command::tests::a_clock_stepped_back_gives_a_resume_its_own_id :: FAILED :: UPKEEP RESUME — msum for gdfl under /dev/shm/msum-tmp/brutex-cli-upkeep-10775-0   run 7 (resumes run 5000): 1 worker(s), 2 item(s) given, 1 skipped as closed clean by the run(s) resumed   items               3 of the 3 month file(s) in scope (--all: every one); 0 link(s) never followed, 0 stray na
RED-CHECK cli upkeep_command::tests::a_run_says_on_store_sum_what_it_was_given_and_left :: FAILED :: assertion `left == right` failed   left: Some(Uint(3))  right: Some(Uint(2)) 
RED-CHECK cli msum_builder::read_back_tests::a_read_back_refused_after_the_door_is_said_and_never_clean :: FAILED :: assertion `left == right` failed: BelowThreshold   left: Small { n_valid: 0 }  right: Refused { why: "the step finished and the read-back was refused: being written, ask again: another writer holds bars/gdfl/NSE/INDEX/NIFTY/1s/2031-05.lock (held by another process; its note in bars/gdfl/NSE/INDEX/NI
RED-CHECK cli msum_builder::read_back_tests::a_summary_the_reader_refuses_after_the_door_is_not_summarised :: FAILED :: assertion `left == right` failed: Kept   left: Kept { n_valid: 1540 }  right: NotSummarized { n_valid: 1540, reader: "stale: the summary describes generation 3 with 1500 bars; the month commits generation 4 with 1540" } 
RED-CHECK cli upkeep_command::tests::a_plan_names_the_lacking_months_and_none_after_the_step :: FAILED :: UPKEEP PLAN — gdfl (the whole feed) under /dev/shm/msum-tmp/brutex-cli-upkeep-10845-0: 4 month file(s) (nothing is written)   NSE/INDEX/NIFTY 1s                             4 month(s); lacks .tix      4, .srt      4, .sum      3   TOTAL lacks .tix 4, .srt 4, .sum 3 of 4 month(s); a month under 1,0

=== BATCH D
== batch D at ceb28bb90b01 2026-11-03 17:51:42 IST
hand changes:  1 file changed, 2 insertions(+), 2 deletions(-)
build rc=0
restored: tree clean at ceb28bb90b01
--- the hand changes (git diff, reverted after the run):
diff --git a/crates/store/src/month_summary_writer.rs b/crates/store/src/month_summary_writer.rs
index 34ce02ad..619c7769 100644
--- a/crates/store/src/month_summary_writer.rs
+++ b/crates/store/src/month_summary_writer.rs
@@ -293,10 +293,10 @@ impl BarFile {
             // A VERSION 1 SUMMARY OF A ONE-MINUTE MONTH describes these bars
             // but counts no padded minute (R4-27): the door builds it whole
             // as version 2, a new file renamed over the old, and names it.
-            Ok(kept) if kept.wants_carries() => V1_UPGRADE.to_owned(),
+            Ok(kept) if kept.wants_carries() && !kept.wants_carries() => V1_UPGRADE.to_owned(),
             Ok(_) => return Ok(Summarized::Kept),
             Err(Refused::BelowThreshold) => return Ok(Summarized::BelowThreshold),
-            Err(why) => why.to_string(),
+            Err(_) => return Ok(Summarized::Kept),
         };
         // ONE DOOR AT A TIME, locked on the handle the door already opened:
         // no second open of the bar file, so no second way for it to fail.
--- each test under them (crate, test, result, first failure lines):
RED-CHECK cli upkeep_command::tests::an_all_run_rebuilds_a_spoiled_summary_and_keeps_a_true_one :: FAILED :: assertion `left == right` failed: UPKEEP RUN — msum for gdfl under /dev/shm/msum-tmp/brutex-cli-upkeep-11086-0   run 1000: 1 worker(s), 3 item(s) given, 0 skipped as closed clean by the run(s) resumed   items               3 of the 3 month file(s) in scope that have no file of this kind; 0 link(s)
RED-CHECK cli upkeep_command::tests::an_all_run_builds_a_version_1_summary_whole_as_version_2 :: FAILED :: assertion failed: root.journal(Kind::MonthSummary).contains("summary_v1")    0: __rustc::rust_begin_unwind 

`````

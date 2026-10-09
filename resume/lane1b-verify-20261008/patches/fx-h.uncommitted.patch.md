```diff
diff --git a/crates/cli/src/anchored_search_lineage_v2.rs b/crates/cli/src/anchored_search_lineage_v2.rs
index c279b2f2..a377057d 100644
--- a/crates/cli/src/anchored_search_lineage_v2.rs
+++ b/crates/cli/src/anchored_search_lineage_v2.rs
@@ -1603,6 +1603,7 @@ fn file_generation(
     path: &Path,
     maximum: u64,
 ) -> Result<FileGeneration, AnchoredSearchLineageV2Refusal> {
+    #[cfg(test)] crate::hash_meter::add("anchored_search_lineage_v2", file.metadata().map_or(0, |m| m.len()));
     let before = file
         .metadata()
         .map_err(|why| format!("cannot stat held search-lineage file: {why}"))?;
diff --git a/crates/cli/src/anchored_search_lineage_v3.rs b/crates/cli/src/anchored_search_lineage_v3.rs
index d6ceadf4..c1be0dd2 100644
--- a/crates/cli/src/anchored_search_lineage_v3.rs
+++ b/crates/cli/src/anchored_search_lineage_v3.rs
@@ -1675,6 +1675,7 @@ fn file_generation_with_between_hash_action(
     maximum: u64,
     between_hashes: impl FnOnce() -> Result<(), AnchoredSearchLineageV3Refusal>,
 ) -> Result<FileGeneration, AnchoredSearchLineageV3Refusal> {
+    #[cfg(test)] crate::hash_meter::add("anchored_search_lineage_v3", file.metadata().map_or(0, |m| m.len()));
     let before_metadata = file
         .metadata()
         .map_err(|why| format!("cannot stat held search-lineage file: {why}"))?;
diff --git a/crates/cli/src/anchored_search_lineage_v4.rs b/crates/cli/src/anchored_search_lineage_v4.rs
index 018ca196..5f2e363e 100644
--- a/crates/cli/src/anchored_search_lineage_v4.rs
+++ b/crates/cli/src/anchored_search_lineage_v4.rs
@@ -1849,6 +1849,7 @@ fn hash_held_prefix(
     path: &Path,
     length: u64,
 ) -> Result<[u8; 32], AnchoredSearchLineageV4Refusal> {
+    #[cfg(test)] crate::hash_meter::add("anchored_search_lineage_v4", file.metadata().map_or(0, |m| m.len()));
     let mut reader = file
         .try_clone()
         .map_err(|why| format!("cannot clone search-lineage V4 file for hashing: {why}"))?;
diff --git a/crates/cli/src/execution_v3.rs b/crates/cli/src/execution_v3.rs
index 3ba3f41d..46cedf18 100644
--- a/crates/cli/src/execution_v3.rs
+++ b/crates/cli/src/execution_v3.rs
@@ -3987,6 +3987,7 @@ fn file_generation(
 }
 
 fn hash_held_file(file: &File, path: &Path, len: u64) -> Result<[u8; 32], ExecutionV3Refusal> {
+    #[cfg(test)] crate::hash_meter::add("execution_v3", len);
     let mut reader = file
         .try_clone()
         .map_err(|why| format!("cannot clone Execution V3 file {}: {why}", path.display()))?;
diff --git a/crates/cli/src/execution_v4.rs b/crates/cli/src/execution_v4.rs
index 56c49300..ef3654c0 100644
--- a/crates/cli/src/execution_v4.rs
+++ b/crates/cli/src/execution_v4.rs
@@ -4889,6 +4889,7 @@ fn file_generation(
 }
 
 fn hash_held_file(file: &File, path: &Path, len: u64) -> Result<[u8; 32], ExecutionV4Refusal> {
+    #[cfg(test)] crate::hash_meter::add("execution_v4", len);
     let mut reader = file
         .try_clone()
         .map_err(|why| format!("cannot clone Execution V4 file {}: {why}", path.display()))?;
diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index ca01a5d3..e94f770c 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -32088,3 +32088,51 @@ mod derived_floor_tests {
         );
     }
 }
+
+#[cfg(test)]
+pub(crate) mod hash_meter {
+    use std::collections::BTreeMap;
+    use std::sync::Mutex;
+    pub(crate) static METER: Mutex<BTreeMap<String, (u64, u64)>> = Mutex::new(BTreeMap::new());
+    pub(crate) static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
+    const SKIP: [&str; 14] = [
+        "hash_meter", "file_generation", "hash_held_file", "hash_file", "require_unchanged",
+        "require_generation", "refresh", "hash_held_prefix", "hash_file_split", "measure_generation",
+        "require_file_generation", "hash_file_bounded", "{{closure}}", "with_shared_lock",
+    ];
+    pub(crate) fn add(module: &'static str, bytes: u64) {
+        if !ON.load(std::sync::atomic::Ordering::Relaxed) {
+            return;
+        }
+        let trace = std::backtrace::Backtrace::force_capture().to_string();
+        let mut caller = String::from("?");
+        for line in trace.lines() {
+            let line = line.trim();
+            let Some(name) = line.split_once(": ").map(|(_, n)| n) else { continue };
+            if !name.starts_with("cli::") { continue; }
+            let last = name.rsplit("::").find(|s| !s.starts_with('h') || s.len() != 17).unwrap_or("");
+            if SKIP.iter().any(|s| last == *s) { continue; }
+            caller = name.to_owned();
+            break;
+        }
+        let key = format!("{module} <- {caller}");
+        let mut map = METER.lock().unwrap();
+        let e = map.entry(key).or_insert((0, 0));
+        e.0 += 1;
+        e.1 += bytes;
+    }
+    pub(crate) fn dump() -> String {
+        let map = METER.lock().unwrap();
+        let mut out = String::new();
+        let mut per: BTreeMap<String, (u64, u64)> = BTreeMap::new();
+        for (k, (c, b)) in map.iter() {
+            out.push_str(&format!("{c:>6} passes {b:>12} bytes  {k}\n"));
+            let m = k.split(" <- ").next().unwrap().to_owned();
+            let e = per.entry(m).or_insert((0, 0));
+            e.0 += c; e.1 += b;
+        }
+        out.push_str("--- per module\n");
+        for (k, (c, b)) in per { out.push_str(&format!("{c:>6} passes {b:>12} bytes  {k}\n")); }
+        out
+    }
+}
diff --git a/crates/cli/src/population_admission_v3.rs b/crates/cli/src/population_admission_v3.rs
index 9b13fd26..fe57e032 100644
--- a/crates/cli/src/population_admission_v3.rs
+++ b/crates/cli/src/population_admission_v3.rs
@@ -4816,6 +4816,7 @@ fn file_generation_with_between_hash_action(
     max_bytes: u64,
     between_hashes: impl FnOnce() -> Result<(), PopulationAdmissionV3Refusal>,
 ) -> Result<FileGeneration, PopulationAdmissionV3Refusal> {
+    #[cfg(test)] crate::hash_meter::add("population_admission_v3", file.metadata().map_or(0, |m| m.len()));
     let before_metadata = file.metadata().map_err(|why| {
         format!(
             "cannot stat held Admission V3 file {}: {why}",
diff --git a/crates/cli/src/population_admission_v4.rs b/crates/cli/src/population_admission_v4.rs
index 820a4253..e5e9bd5d 100644
--- a/crates/cli/src/population_admission_v4.rs
+++ b/crates/cli/src/population_admission_v4.rs
@@ -3153,6 +3153,7 @@ fn file_generation(
     path: &Path,
     maximum: u64,
 ) -> Result<FileGeneration, PopulationAdmissionV4Refusal> {
+    #[cfg(test)] crate::hash_meter::add("population_admission_v4", file.metadata().map_or(0, |m| m.len()));
     let metadata = file
         .metadata()
         .map_err(|why| format!("cannot stat Admission V4 file: {why}"))?;
diff --git a/crates/cli/src/population_finalization_v2.rs b/crates/cli/src/population_finalization_v2.rs
index 1de5b4e9..e80f8ffd 100644
--- a/crates/cli/src/population_finalization_v2.rs
+++ b/crates/cli/src/population_finalization_v2.rs
@@ -3053,6 +3053,7 @@ fn hash_file_bounded(
     path: &Path,
     max_bytes: u64,
 ) -> Result<[u8; 32], PopulationFinalizationV2Refusal> {
+    #[cfg(test)] crate::hash_meter::add("population_finalization_v2", file.metadata().map_or(0, |m| m.len()));
     #[cfg(test)]
     FINALIZATION_FILE_HASHES.with(|count| count.set(count.get().saturating_add(1)));
     file.seek(SeekFrom::Start(0)).map_err(|why| {
diff --git a/crates/cli/src/population_finalization_v3.rs b/crates/cli/src/population_finalization_v3.rs
index 0a4412b7..45ba591e 100644
--- a/crates/cli/src/population_finalization_v3.rs
+++ b/crates/cli/src/population_finalization_v3.rs
@@ -3073,6 +3073,7 @@ fn file_generation_with_between_hash_action(
     max_bytes: u64,
     between_hashes: impl FnOnce() -> Result<(), PopulationFinalizationV3Refusal>,
 ) -> Result<FileGeneration, PopulationFinalizationV3Refusal> {
+    #[cfg(test)] crate::hash_meter::add("population_finalization_v3", file.metadata().map_or(0, |m| m.len()));
     let before_metadata = file.metadata().map_err(|why| {
         format!(
             "cannot stat held Finalization V3 file {}: {why}",
diff --git a/crates/cli/src/population_finalization_v4.rs b/crates/cli/src/population_finalization_v4.rs
index a70455a0..2f281417 100644
--- a/crates/cli/src/population_finalization_v4.rs
+++ b/crates/cli/src/population_finalization_v4.rs
@@ -2916,6 +2916,7 @@ fn file_generation(
     path: &Path,
     maximum: u64,
 ) -> Result<FileGeneration, PopulationFinalizationV4Refusal> {
+    #[cfg(test)] crate::hash_meter::add("population_finalization_v4", file.metadata().map_or(0, |m| m.len()));
     let metadata = file
         .metadata()
         .map_err(|why| format!("cannot stat Finalization V4 file: {why}"))?;
diff --git a/crates/cli/src/population_statistics_v2.rs b/crates/cli/src/population_statistics_v2.rs
index f7d5c13d..2fff3312 100644
--- a/crates/cli/src/population_statistics_v2.rs
+++ b/crates/cli/src/population_statistics_v2.rs
@@ -6179,6 +6179,7 @@ fn hash_file_split(
     exact_bytes: u64,
     cut: u64,
 ) -> Result<([u8; 32], [u8; 32]), PopulationStatisticsV2Refusal> {
+    #[cfg(test)] crate::hash_meter::add("population_statistics_v2", exact_bytes);
     #[cfg(test)]
     STATISTICS_FILE_HASHES.with(|count| count.set(count.get().saturating_add(1)));
     file.seek(SeekFrom::Start(0))
diff --git a/crates/cli/src/population_statistics_v3.rs b/crates/cli/src/population_statistics_v3.rs
index acd343a4..c043ab31 100644
--- a/crates/cli/src/population_statistics_v3.rs
+++ b/crates/cli/src/population_statistics_v3.rs
@@ -3121,6 +3121,7 @@ fn hash_file(
     path: &Path,
     exact_bytes: u64,
 ) -> Result<[u8; 32], PopulationStatisticsV3Refusal> {
+    #[cfg(test)] crate::hash_meter::add("population_statistics_v3", exact_bytes);
     file.seek(SeekFrom::Start(0))
         .map_err(|why| format!("cannot seek {} for generation hash: {why}", path.display()))?;
     let mut hasher = Hasher::new();
diff --git a/crates/cli/src/population_v5.rs b/crates/cli/src/population_v5.rs
index 2a33eb80..ad58fd0e 100644
--- a/crates/cli/src/population_v5.rs
+++ b/crates/cli/src/population_v5.rs
@@ -3731,6 +3731,7 @@ fn file_generation(
 }
 
 fn hash_held_file(file: &File, path: &Path, len: u64) -> Result<[u8; 32], PopulationV5Refusal> {
+    #[cfg(test)] crate::hash_meter::add("population_v5", len);
     let mut reader = file
         .try_clone()
         .map_err(|why| format!("cannot clone Population V5 file {}: {why}", path.display()))?;
diff --git a/crates/cli/src/population_v6.rs b/crates/cli/src/population_v6.rs
index d635d33a..ec145c10 100644
--- a/crates/cli/src/population_v6.rs
+++ b/crates/cli/src/population_v6.rs
@@ -2530,6 +2530,7 @@ fn file_generation(
     path: &Path,
     maximum: u64,
 ) -> Result<FileGeneration, PopulationV6Refusal> {
+    #[cfg(test)] crate::hash_meter::add("population_v6", file.metadata().map_or(0, |m| m.len()));
     let metadata = file
         .metadata()
         .map_err(|why| format!("cannot stat Population V6 file: {why}"))?;
diff --git a/crates/cli/src/pre_admission_data.rs b/crates/cli/src/pre_admission_data.rs
index c93876d7..66c2625d 100644
--- a/crates/cli/src/pre_admission_data.rs
+++ b/crates/cli/src/pre_admission_data.rs
@@ -3901,6 +3901,7 @@ thread_local! {
 }
 
 fn hash_file(file: &mut File, path: &Path) -> Result<[u8; 32], PreAdmissionDataRefusal> {
+    #[cfg(test)] crate::hash_meter::add("pre_admission_data", file.metadata().map_or(0, |m| m.len()));
     #[cfg(test)]
     V1_FILE_HASHES.with(|count| count.set(count.get().saturating_add(1)));
     file.seek(SeekFrom::Start(0))
@@ -4145,6 +4146,7 @@ fn file_generation_v2(
 }
 
 fn hash_file_v2(file: &mut File, path: &Path) -> Result<[u8; 32], PreAdmissionDataRefusal> {
+    #[cfg(test)] crate::hash_meter::add("pre_admission_v2", file.metadata().map_or(0, |m| m.len()));
     file.seek(SeekFrom::Start(0)).map_err(|why| {
         format!(
             "cannot seek {} for V2 generation hash: {why}",
diff --git a/crates/cli/src/selection_v5.rs b/crates/cli/src/selection_v5.rs
index f8ca1d06..90e03fd1 100644
--- a/crates/cli/src/selection_v5.rs
+++ b/crates/cli/src/selection_v5.rs
@@ -2836,6 +2836,7 @@ fn measured_generation(
     max_bytes: u64,
     label: &str,
 ) -> Result<FileGeneration, SelectionV5Refusal> {
+    #[cfg(test)] crate::hash_meter::add("selection_v5", file.metadata().map_or(0, |m| m.len()));
     require_regular_unique_file(file, path)?;
     let held = file
         .metadata()
diff --git a/crates/cli/src/step3_all_rung_tests.rs b/crates/cli/src/step3_all_rung_tests.rs
index bfe5af1b..e69010e4 100644
--- a/crates/cli/src/step3_all_rung_tests.rs
+++ b/crates/cli/src/step3_all_rung_tests.rs
@@ -343,6 +343,7 @@ fn all_eight_stored_rungs_publish_exact_selection_chains_and_reuse_every_byte()
         .chain(selection_paths.iter().cloned())
         .collect();
     let mut original = None;
+    crate::hash_meter::ON.store(true, std::sync::atomic::Ordering::Relaxed);
     for written in [8, 0] {
         let population = commit_all_rung_with_verified_build_v5(
             &request,
@@ -352,6 +353,8 @@ fn all_eight_stored_rungs_publish_exact_selection_chains_and_reuse_every_byte()
         let execution = commit_all_rung_stored_execution_v3(population, &execution_request)?;
         assert_eq!(execution.written_rung_count(), written);
         let mut selection = commit_all_rung_stored_selection_v5(execution, &selection_request)?;
+        eprintln!("=== V5 ROUTE written={written}\n{}", crate::hash_meter::dump());
+        crate::hash_meter::METER.lock().unwrap().clear();
         assert_eq!(selection.written_rung_count(), written);
         // Each producer reauthenticates before returning; the successor doors
         // also reauthenticate their retained inputs. Do not duplicate those
diff --git a/crates/cli/src/strict_v6_tests.rs b/crates/cli/src/strict_v6_tests.rs
index fdd33099..f963a7bf 100644
--- a/crates/cli/src/strict_v6_tests.rs
+++ b/crates/cli/src/strict_v6_tests.rs
@@ -159,6 +159,26 @@ mod strict_v6_fixture_tests {
         Ok((fixture, high.1))
     }
 
+    #[test]
+    fn tmp_hash_meter_route() -> Result<(), String> {
+        let (fixture, support) = strict_fixture_shape(true, true)?;
+        let nifty = strict_fixture_family_at(&fixture, "NIFTY", support, true)?;
+        let banknifty = strict_fixture_family_at(&fixture, "BANKNIFTY", support, true)?;
+        crate::hash_meter::ON.store(true, std::sync::atomic::Ordering::Relaxed);
+        let mut selection = crate::ledger_v6::strict_fixture_selection(
+            &fixture.base,
+            nifty,
+            banknifty,
+            &population_admission_policy()?,
+        )?;
+        eprintln!("=== ROUTE\n{}", crate::hash_meter::dump());
+        crate::hash_meter::METER.lock().unwrap().clear();
+        let _ = selection.snapshot()?;
+        eprintln!("=== ONE SNAPSHOT\n{}", crate::hash_meter::dump());
+        crate::hash_meter::ON.store(false, std::sync::atomic::Ordering::Relaxed);
+        Ok(())
+    }
+
     #[test]
     fn strict_v6_both_evaluated_and_both_mixed_shapes_reach_real_selection() -> Result<(), String> {
         for (nifty_evaluated, banknifty_evaluated) in [(true, true), (true, false), (false, true)] {
```

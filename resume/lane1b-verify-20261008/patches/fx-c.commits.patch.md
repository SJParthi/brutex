From c222993a440aeb11a56248a338d8edc9d451046a Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:03:51 +0000
Subject: [PATCH 1/9] cli, api: route every open that could wait on a FIFO peer
 through a non-blocking door (G5-2)

Before: D-1743 gave five cli ledger opens a door that adds O_NONBLOCK and
admits only a regular file, under a heading that claimed every ledger.
frontier.rs, trades.rs and four sweep_evidence.rs readers kept File::open,
and a census of cli and api release code found 170 opens with no
O_NONBLOCK: File::open, File::create, OpenOptions chains and directory
opens used as durability barriers. A FIFO planted at one of those paths
parked the caller in open(2) until a peer arrived. The extended FIFO test
failed with "opening .../results/frontier.bin waited for a FIFO peer".

After: readonly_file is public and has three doors. regular is the D-1743
door. read opens a read-only regular file. directory is a non-blocking
read-only open that admits only a directory. Every such open in cli and
api goes through one of them. A read-write open needs none, because
open(2) never parks O_RDWR on a FIFO; a_read_write_open_of_a_fifo_never_waits
pins that, and api's serve lock stays a plain read-write open so its
/dev/full stamp-failure test still reaches the stamp. Sixteen
O_NOFOLLOW handles now use O_NOFOLLOW_NONBLOCK. The FIFO test covers
Frontier, Trades and five sweep_evidence readers, each within a bounded
wait. api::backtest::read has its own 5 s FIFO test.
no_cli_or_api_open_can_wait_for_a_fifo_peer scans every release source
in both crates, with test items removed, and refuses a bare File::open or
File::create, or any OpenOptions that does not reach regular, open
read-write, or name a NONBLOCK flag. D-4732, L1FC-01..03 and L1FC-14.
The scan's limits are in docs/06-limits.md.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/api/src/audit.rs                       |  18 +-
 crates/api/src/autopilot.rs                   |   8 +-
 crates/api/src/backtest.rs                    |  49 +-
 crates/api/src/indexmap.rs                    |   3 +-
 crates/api/src/recovery.rs                    |   6 +-
 crates/api/src/recovery_control.rs            |   4 +-
 crates/api/src/recovery_journal.rs            |  25 +-
 crates/api/src/server.rs                      |   3 +
 crates/cli/src/admission_join.rs              |   2 +-
 crates/cli/src/admission_store.rs             |  24 +-
 crates/cli/src/all_rung_population_v5.rs      |   2 +-
 crates/cli/src/anchored_search_lineage_v2.rs  |   6 +-
 crates/cli/src/anchored_search_lineage_v3.rs  |   6 +-
 crates/cli/src/anchored_search_lineage_v4.rs  |   6 +-
 .../cli/src/boolean_candidate_persistence.rs  |  19 +-
 crates/cli/src/candidate_trades.rs            |  12 +-
 crates/cli/src/candidate_universe.rs          |  11 +-
 crates/cli/src/checksum_receipts.rs           |   4 +-
 crates/cli/src/execution_capability.rs        |  28 +-
 crates/cli/src/execution_disposition_v2.rs    |  28 +-
 crates/cli/src/execution_v3.rs                |   8 +-
 crates/cli/src/execution_v4.rs                |   8 +-
 crates/cli/src/expression.rs                  |  18 +-
 crates/cli/src/frontier.rs                    |  18 +-
 crates/cli/src/global_replay.rs               |  26 +-
 crates/cli/src/global_replay_v2.rs            |  21 +-
 crates/cli/src/global_replay_v3.rs            |  62 +--
 crates/cli/src/global_replay_v4_store.rs      |   4 +-
 crates/cli/src/institutional_statistics.rs    |  21 +-
 crates/cli/src/lib.rs                         |  20 +-
 crates/cli/src/live.rs                        |  11 +-
 crates/cli/src/operation_audit.rs             |   6 +-
 crates/cli/src/pool_oos.rs                    |  10 +-
 crates/cli/src/population.rs                  |  44 +-
 crates/cli/src/population_admission_v2.rs     |  10 +-
 crates/cli/src/population_admission_v3.rs     |  10 +-
 crates/cli/src/population_admission_v4.rs     |   8 +-
 .../src/population_base_evidence_ledger_v2.rs |  15 +-
 crates/cli/src/population_finalization_v2.rs  |  10 +-
 crates/cli/src/population_finalization_v3.rs  |  10 +-
 crates/cli/src/population_finalization_v4.rs  |   8 +-
 crates/cli/src/population_observations_v1.rs  |  82 ++--
 crates/cli/src/population_statistics_v2.rs    |   6 +-
 crates/cli/src/population_statistics_v3.rs    |   6 +-
 crates/cli/src/population_v5.rs               |   8 +-
 crates/cli/src/population_v6.rs               |   8 +-
 crates/cli/src/readonly_file.rs               | 462 +++++++++++++++++-
 crates/cli/src/search_checkpoint.rs           |  26 +-
 crates/cli/src/selection.rs                   |  40 +-
 crates/cli/src/selection_v3.rs                |  20 +-
 crates/cli/src/selection_v4.rs                |  20 +-
 crates/cli/src/selection_v4_authority.rs      |   4 +-
 crates/cli/src/selection_v5.rs                |   6 +-
 crates/cli/src/selection_v6.rs                |   6 +-
 crates/cli/src/step3_orchestrator.rs          |   2 +-
 crates/cli/src/stored_data_completeness.rs    |  18 +-
 crates/cli/src/sweep_evidence.rs              |  49 +-
 crates/cli/src/trades.rs                      |  18 +-
 docs/04-invariants.md                         |   4 +
 docs/05-decisions.md                          |  50 ++
 docs/06-limits.md                             |  28 ++
 61 files changed, 1053 insertions(+), 422 deletions(-)

diff --git a/crates/api/src/audit.rs b/crates/api/src/audit.rs
index 4783e7fd..42984875 100644
--- a/crates/api/src/audit.rs
+++ b/crates/api/src/audit.rs
@@ -1316,12 +1316,14 @@ impl Journal {
         // leave the lock alive on any duplicate a child spawned by another
         // thread still holds (D-0693).
         let mut file = Flock::try_lock(
-            std::fs::OpenOptions::new()
-                .read(true)
-                .append(true)
-                .create(true)
-                .open(&self.path)
-                .map_err(|e| named("cannot open the journal", &e))?,
+            cli::readonly_file::regular(
+                std::fs::OpenOptions::new()
+                    .read(true)
+                    .append(true)
+                    .create(true),
+                &self.path,
+            )
+            .map_err(|e| named("cannot open the journal", &e))?,
             self.path.as_path(),
         )
         .map_err(|e| {
@@ -1416,8 +1418,8 @@ impl Journal {
         // `usize` or wider. The arm is a backstop and no test drives it.
         let span = usize::try_from(wanted.saturating_mul(RECORD_LEN_U64))
             .map_err(|e| format!("{wanted} records do not fit this machine's memory: {e}"))?;
-        let mut file =
-            std::fs::File::open(&self.path).map_err(|e| format!("{}: {e}", self.path.display()))?;
+        let mut file = cli::readonly_file::read(&self.path)
+            .map_err(|e| format!("{}: {e}", self.path.display()))?;
         file.seek(std::io::SeekFrom::Start(
             start.saturating_mul(RECORD_LEN_U64),
         ))
diff --git a/crates/api/src/autopilot.rs b/crates/api/src/autopilot.rs
index 626768ce..fc70f9b4 100644
--- a/crates/api/src/autopilot.rs
+++ b/crates/api/src/autopilot.rs
@@ -626,7 +626,13 @@ fn probe_io(root: &std::path::Path, path: &std::path::Path) -> std::io::Result<(
             ),
         ));
     }
-    let mut file = std::fs::File::create(path)?;
+    let mut file = cli::readonly_file::regular(
+        std::fs::OpenOptions::new()
+            .write(true)
+            .create(true)
+            .truncate(true),
+        path,
+    )?;
     file.write_all(b"brutex write probe\n")?;
     // THE BYTES HAVE TO REACH THE DEVICE. A write that only reached the page
     // cache answers "the disk is fine" on a disk that is full, which is the one
diff --git a/crates/api/src/backtest.rs b/crates/api/src/backtest.rs
index 755106b5..7a4dce4e 100644
--- a/crates/api/src/backtest.rs
+++ b/crates/api/src/backtest.rs
@@ -24,9 +24,10 @@
 //! statements of itself is a format that can diverge. Three things hold it
 //! together:
 //!
-//! 1. **Nothing here writes.** [`File::open`] is read-only and there is no
-//!    append path in this module. A reader that drifts renders wrong; a writer
-//!    that drifts corrupts. Only one crate may write, and it is `cli`.
+//! 1. **Nothing here writes.** [`cli::readonly_file::read`] is read-only and
+//!    there is no append path in this module. A reader that drifts renders
+//!    wrong; a writer that drifts corrupts. Only one crate may write, and it is
+//!    `cli`.
 //! 2. **The stride is asserted against the field sum at compile time** —
 //!    [`FIELD_SUM`] below — which is the same check
 //!    `the_stride_is_exactly_what_the_writer_writes` makes on the writer's
@@ -73,7 +74,6 @@
 //! measurement, however sound it is.
 
 use std::fmt::Write as _;
-use std::fs::File;
 use std::io::SeekFrom;
 use std::path::{Path, PathBuf};
 
@@ -1023,7 +1023,7 @@ pub fn path_in(root: &Path) -> PathBuf {
 #[must_use]
 pub fn read(root: &Path, limit: usize) -> Ledger {
     let path = path_in(root);
-    match File::open(&path) {
+    match cli::readonly_file::read(&path) {
         Ok(mut file) => read_from(path, &mut file, limit),
         // NOT AN ERROR, AND THE SENTENCE SAYS SO. A store that has never been
         // swept has no ledger, and rendering that as a failure would teach an
@@ -1970,6 +1970,45 @@ mod tests {
         let _ = std::fs::remove_dir_all(&root);
     }
 
+    /// G5-2 (D-4732): a FIFO where the ledger belongs is refused at once. The
+    /// read was `File::open`, which waits in `open(2)` for a writer, so a page
+    /// refresh parked its worker until one arrived. The bound is 5 s.
+    #[test]
+    fn a_fifo_at_the_ledger_path_is_refused_without_waiting() {
+        use std::os::unix::fs::OpenOptionsExt as _;
+        let root = std::env::temp_dir().join(format!(
+            "brutex-backtest-fifo-{}-{:?}",
+            std::process::id(),
+            std::thread::current().id()
+        ));
+        let _ = std::fs::remove_dir_all(&root);
+        std::fs::create_dir_all(root.join("results")).expect("a temp root");
+        let made = std::process::Command::new("mkfifo")
+            .arg(path_in(&root))
+            .status()
+            .expect("mkfifo runs");
+        assert!(made.success(), "premise: the FIFO exists");
+        let (sent, got) = std::sync::mpsc::channel();
+        let reader = root.clone();
+        std::thread::spawn(move || {
+            let _ = sent.send(read(&reader, 10));
+        });
+        let answer = got.recv_timeout(std::time::Duration::from_secs(5));
+        if answer.is_err() {
+            // A reader parked in `open(2)` counts as the FIFO's reader, so a
+            // non-blocking writer opens and releases it.
+            let _ = std::fs::OpenOptions::new()
+                .write(true)
+                .custom_flags(store::open_flags::O_NONBLOCK)
+                .open(path_in(&root));
+        }
+        let _ = std::fs::remove_dir_all(&root);
+        let ledger = answer.expect("the read answered within 5 s instead of waiting for a writer");
+        let why = ledger.refusal.expect("a FIFO is refused, not read");
+        assert!(why.contains("not a regular file"), "{why}");
+        assert_eq!((ledger.total, ledger.runs.len()), (0, 0));
+    }
+
     #[test]
     fn a_real_file_on_disk_reads_the_same_as_the_cursor() {
         let root = std::env::temp_dir().join(format!(
diff --git a/crates/api/src/indexmap.rs b/crates/api/src/indexmap.rs
index ceb7801b..53bea013 100644
--- a/crates/api/src/indexmap.rs
+++ b/crates/api/src/indexmap.rs
@@ -52,7 +52,8 @@ impl Published {
     /// a size check and the read cannot slip past (UC-19, D-1502).
     pub fn read(path: &Path) -> Result<Self, String> {
         use std::io::Read as _;
-        let file = std::fs::File::open(path).map_err(|why| format!("{}: {why}", path.display()))?;
+        let file =
+            cli::readonly_file::read(path).map_err(|why| format!("{}: {why}", path.display()))?;
         let mut text = String::new();
         file.take(MAX_CATALOGUE_BYTES.saturating_add(1))
             .read_to_string(&mut text)
diff --git a/crates/api/src/recovery.rs b/crates/api/src/recovery.rs
index d793942f..0f37e3e5 100644
--- a/crates/api/src/recovery.rs
+++ b/crates/api/src/recovery.rs
@@ -831,10 +831,10 @@ fn seeded(site: &Site, id: [u8; 32], units: Option<Vec<Record>>) -> Result<Journ
         std::fs::hard_link(&staging, &active_file).map_err(failure)?;
         std::fs::remove_file(&staging).map_err(failure)?;
     }
-    std::fs::File::open(root(site))
+    cli::readonly_file::directory(root(site))
         .and_then(|file| file.sync_all())
         .map_err(failure)?;
-    std::fs::File::open(site.store_root.join("audit"))
+    cli::readonly_file::directory(site.store_root.join("audit"))
         .and_then(|file| file.sync_all())
         .map_err(failure)?;
     Ok(journal)
@@ -848,7 +848,7 @@ async fn drive(site: Loaded, id: [u8; 32], prepared: Result<Journal, String>, ex
         let mut journal = prepared?;
         let mut attempts =
             Journal::open_existing(&root(&worker_site).join("attempts.bin")).map_err(failure)?;
-        std::fs::File::open(root(&worker_site))
+        cli::readonly_file::directory(root(&worker_site))
             .and_then(|file| file.sync_all())
             .map_err(failure)?;
         let answer = execute(&worker_site, &mut journal, &mut attempts, explicit).await;
diff --git a/crates/api/src/recovery_control.rs b/crates/api/src/recovery_control.rs
index 7e5d2c0a..042a5f83 100644
--- a/crates/api/src/recovery_control.rs
+++ b/crates/api/src/recovery_control.rs
@@ -160,10 +160,10 @@ fn persist(site: &Site, id: [u8; 32], status: Status) -> Result<(), String> {
     }
     // Journal::open synced its own directory entry. These parents may have
     // been created above; a durable file in a lost directory is not a STOP.
-    fs::File::open(site.store_root.join("audit"))
+    cli::readonly_file::directory(site.store_root.join("audit"))
         .and_then(|file| file.sync_all())
         .map_err(|error| error.to_string())?;
-    fs::File::open(&site.store_root)
+    cli::readonly_file::directory(&site.store_root)
         .and_then(|file| file.sync_all())
         .map_err(|error| error.to_string())
 }
diff --git a/crates/api/src/recovery_journal.rs b/crates/api/src/recovery_journal.rs
index fde0baa8..c0561a9c 100644
--- a/crates/api/src/recovery_journal.rs
+++ b/crates/api/src/recovery_journal.rs
@@ -305,12 +305,14 @@ impl Journal {
     }
 
     fn open_at(path: &Path, create: bool, create_new: bool) -> io::Result<Self> {
-        let mut file = OpenOptions::new()
-            .read(true)
-            .append(true)
-            .create(create)
-            .create_new(create_new)
-            .open(path)?;
+        let mut file = cli::readonly_file::regular(
+            OpenOptions::new()
+                .read(true)
+                .append(true)
+                .create(create)
+                .create_new(create_new),
+            path,
+        )?;
         // A guard over a duplicate, so `file` stays free to be read through
         // `&mut` and then moved into `io`. Every `?` below releases the lock
         // through the guard's explicit unlock.
@@ -350,7 +352,7 @@ impl Journal {
             .parent()
             .filter(|parent| !parent.as_os_str().is_empty())
             .unwrap_or_else(|| Path::new("."));
-        File::open(parent)?.sync_all()?;
+        cli::readonly_file::directory(parent)?.sync_all()?;
         Ok(Self {
             latest: index.latest,
             order: index.order,
@@ -549,7 +551,7 @@ pub(crate) fn tail(path: &Path, limit: usize) -> io::Result<Vec<Record>> {
             "recovery tail limit exceeds {MAX_TAIL_RECORDS} events"
         )));
     }
-    let mut file = File::open(path)?;
+    let mut file = cli::readonly_file::read(path)?;
     let metadata = file.metadata()?;
     if !metadata.is_file() {
         return Err(invalid_data("recovery journal is not a regular file"));
@@ -661,14 +663,15 @@ pub(crate) struct Index {
 /// A live writer refuses the snapshot; callers must not infer an empty inventory.
 /// Cost is O(records) time and O(distinct work units) memory.
 pub(crate) fn snapshot(path: &Path) -> io::Result<Index> {
-    let mut file =
-        Flock::try_lock_shared(File::open(path)?, path).map_err(|error| match error {
+    let mut file = Flock::try_lock_shared(cli::readonly_file::read(path)?, path).map_err(
+        |error| match error {
             TryLockError::WouldBlock => io::Error::new(
                 io::ErrorKind::WouldBlock,
                 "recovery journal has a writer; read-only inventory is unavailable",
             ),
             TryLockError::Error(error) => error,
-        })?;
+        },
+    )?;
     let metadata = file.metadata()?;
     if !metadata.is_file() {
         return Err(invalid_data("recovery journal is not a regular file"));
diff --git a/crates/api/src/server.rs b/crates/api/src/server.rs
index 6c513caf..1927d27b 100644
--- a/crates/api/src/server.rs
+++ b/crates/api/src/server.rs
@@ -18987,6 +18987,9 @@ fn take_serve_lock(store_root: &Path, addr: std::net::SocketAddr) -> Result<Serv
             }
         }
     }
+    // Read-write, so `open(2)` never waits on a FIFO here (D-4732), and not
+    // through `cli::readonly_file::regular`: a lock name that reaches a device
+    // must still reach the stamp and be refused there.
     let file = match std::fs::OpenOptions::new()
         .read(true)
         .write(true)
diff --git a/crates/cli/src/admission_join.rs b/crates/cli/src/admission_join.rs
index bdc6a7f5..4158abb0 100644
--- a/crates/cli/src/admission_join.rs
+++ b/crates/cli/src/admission_join.rs
@@ -141,7 +141,7 @@ impl AdmissionAuthoritativeLedger {
         max_bytes: Option<u64>,
     ) -> Result<Self, AdmissionJoinRefusal> {
         let lock_path = root.join("results").join("population-write.lock");
-        let outer_lock = File::open(&lock_path)
+        let outer_lock = crate::readonly_file::read(&lock_path)
             .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
         outer_lock.lock_shared().map_err(|why| {
             format!(
diff --git a/crates/cli/src/admission_store.rs b/crates/cli/src/admission_store.rs
index 8ef27f04..1605009d 100644
--- a/crates/cli/src/admission_store.rs
+++ b/crates/cli/src/admission_store.rs
@@ -664,7 +664,7 @@ impl AdmissionAuthorityLedger {
 
     fn open_existing(root: &Path, max_bytes: Option<u64>) -> Result<Self, AdmissionStoreRefusal> {
         let lock_path = Self::lock_path(root);
-        let writer_lock = File::open(&lock_path)
+        let writer_lock = crate::readonly_file::read(&lock_path)
             .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
         writer_lock
             .lock_shared()
@@ -672,9 +672,9 @@ impl AdmissionAuthorityLedger {
         let opened = (|| {
             let decision_path = Self::decision_path(root);
             let completion_path = Self::completion_path(root);
-            let decision_file = File::open(&decision_path)
+            let decision_file = crate::readonly_file::read(&decision_path)
                 .map_err(|why| format!("{} could not be opened: {why}", decision_path.display()))?;
-            let completion_file = File::open(&completion_path).map_err(|why| {
+            let completion_file = crate::readonly_file::read(&completion_path).map_err(|why| {
                 format!("{} could not be opened: {why}", completion_path.display())
             })?;
             Self::from_files(
@@ -1589,13 +1589,15 @@ fn read_completion_at(
 }
 
 fn open_or_create(path: &Path) -> Result<File, AdmissionStoreRefusal> {
-    OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create(true)
-        .truncate(false)
-        .open(path)
-        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
+    crate::readonly_file::regular(
+        OpenOptions::new()
+            .read(true)
+            .write(true)
+            .create(true)
+            .truncate(false),
+        path,
+    )
+    .map_err(|why| format!("{} could not be opened: {why}", path.display()))
 }
 
 fn ensure_header(
@@ -1853,7 +1855,7 @@ fn require_generation_unchanged(
 }
 
 fn sync_directory(path: &Path) -> Result<(), AdmissionStoreRefusal> {
-    File::open(path)
+    crate::readonly_file::directory(path)
         .and_then(|directory| directory.sync_all())
         .map_err(|why| {
             format!(
diff --git a/crates/cli/src/all_rung_population_v5.rs b/crates/cli/src/all_rung_population_v5.rs
index 2adcb535..4dd95c86 100644
--- a/crates/cli/src/all_rung_population_v5.rs
+++ b/crates/cli/src/all_rung_population_v5.rs
@@ -1266,7 +1266,7 @@ impl AdmittedDirectoryV1 {
             ));
         }
         let identity = directory_identity_v1(&spelling)?;
-        let directory = File::open(&canonical).map_err(|why| {
+        let directory = crate::readonly_file::directory(&canonical).map_err(|why| {
             format!(
                 "all-rung root admission could not hold {label} {}: {why}",
                 canonical.display()
diff --git a/crates/cli/src/anchored_search_lineage_v2.rs b/crates/cli/src/anchored_search_lineage_v2.rs
index 223ce987..c279b2f2 100644
--- a/crates/cli/src/anchored_search_lineage_v2.rs
+++ b/crates/cli/src/anchored_search_lineage_v2.rs
@@ -67,7 +67,7 @@ const LOCK_FILE: &str = "anchored-search-lineage-v2.lock";
 const READ_CHUNK_BYTES: usize = 16 * 1_024;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(MEMBER_PAYLOAD_BYTES + SEAL_BYTES == ANCHORED_SEARCH_LINEAGE_V2_MEMBER_BYTES);
 const _: () =
@@ -1528,7 +1528,7 @@ fn open_root(
     }
     let canonical = std::fs::canonicalize(root)
         .map_err(|why| format!("cannot canonicalize search-lineage root: {why}"))?;
-    let file = File::open(&canonical)
+    let file = crate::readonly_file::directory(&canonical)
         .map_err(|why| format!("cannot open search-lineage root directory: {why}"))?;
     let held = file
         .metadata()
@@ -1581,7 +1581,7 @@ fn open_child(path: &Path, writable: bool) -> Result<(File, bool), AnchoredSearc
     options.read(true).write(writable).create(writable);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
     {
-        options.mode(0o600).custom_flags(O_NOFOLLOW_FLAG);
+        options.mode(0o600).custom_flags(NOFOLLOW_NONBLOCK);
     }
     let file = options
         .open(path)
diff --git a/crates/cli/src/anchored_search_lineage_v3.rs b/crates/cli/src/anchored_search_lineage_v3.rs
index 7455d076..d6ceadf4 100644
--- a/crates/cli/src/anchored_search_lineage_v3.rs
+++ b/crates/cli/src/anchored_search_lineage_v3.rs
@@ -68,7 +68,7 @@ const LOCK_FILE_MAX_BYTES: u64 = 0;
 const READ_CHUNK_BYTES: usize = 16 * 1_024;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(MEMBER_PAYLOAD_BYTES + SEAL_BYTES == ANCHORED_SEARCH_LINEAGE_V3_MEMBER_BYTES);
 const _: () =
@@ -1591,7 +1591,7 @@ fn open_root(
     }
     let canonical = std::fs::canonicalize(root)
         .map_err(|why| format!("cannot canonicalize search-lineage root: {why}"))?;
-    let file = File::open(&canonical)
+    let file = crate::readonly_file::directory(&canonical)
         .map_err(|why| format!("cannot open search-lineage root directory: {why}"))?;
     let held = file
         .metadata()
@@ -1644,7 +1644,7 @@ fn open_child(path: &Path, writable: bool) -> Result<(File, bool), AnchoredSearc
     options.read(true).write(writable).create(writable);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
     {
-        options.mode(0o600).custom_flags(O_NOFOLLOW_FLAG);
+        options.mode(0o600).custom_flags(NOFOLLOW_NONBLOCK);
     }
     let file = options
         .open(path)
diff --git a/crates/cli/src/anchored_search_lineage_v4.rs b/crates/cli/src/anchored_search_lineage_v4.rs
index 1a3ec4c7..018ca196 100644
--- a/crates/cli/src/anchored_search_lineage_v4.rs
+++ b/crates/cli/src/anchored_search_lineage_v4.rs
@@ -66,7 +66,7 @@ const LOCK_FILE_MAX_BYTES: u64 = 0;
 const READ_CHUNK_BYTES: usize = 16 * 1_024;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(MEMBER_PAYLOAD_BYTES + SEAL_BYTES == ANCHORED_SEARCH_LINEAGE_V4_MEMBER_BYTES);
 const _: () =
@@ -1702,7 +1702,7 @@ fn open_root(
     }
     let canonical = std::fs::canonicalize(root)
         .map_err(|why| format!("cannot canonicalize search-lineage V4 root: {why}"))?;
-    let file = File::open(&canonical)
+    let file = crate::readonly_file::directory(&canonical)
         .map_err(|why| format!("cannot open search-lineage V4 root directory: {why}"))?;
     let held = file
         .metadata()
@@ -1758,7 +1758,7 @@ fn open_child(path: &Path, writable: bool) -> Result<(File, bool), AnchoredSearc
     options.read(true).write(writable).create(writable);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
     {
-        options.mode(0o600).custom_flags(O_NOFOLLOW_FLAG);
+        options.mode(0o600).custom_flags(NOFOLLOW_NONBLOCK);
     }
     let file = options.open(path).map_err(|why| {
         format!(
diff --git a/crates/cli/src/boolean_candidate_persistence.rs b/crates/cli/src/boolean_candidate_persistence.rs
index 7617181a..1a343bd5 100644
--- a/crates/cli/src/boolean_candidate_persistence.rs
+++ b/crates/cli/src/boolean_candidate_persistence.rs
@@ -72,7 +72,7 @@ impl Pending {
         self.require_owner()?;
         let encoded = receipt(identity, payload, bytes);
         write_or_equal(&self.directory.join("complete.bin"), &encoded)?;
-        File::open(&self.directory)
+        crate::readonly_file::directory(&self.directory)
             .map_err(display)?
             .sync_all()
             .map_err(display)?;
@@ -96,11 +96,10 @@ pub(crate) fn prepare_in_namespace(
     let directory_path = base.join(crate::identity_hex(&identity));
     directory(&base, &directory_path)?;
     let owner_path = directory_path.join("owner.lock");
-    match OpenOptions::new()
-        .write(true)
-        .create_new(true)
-        .open(&owner_path)
-    {
+    match crate::readonly_file::regular(
+        OpenOptions::new().write(true).create_new(true),
+        &owner_path,
+    ) {
         Ok(file) => {
             file.sync_all().map_err(display)?;
         }
@@ -130,7 +129,7 @@ pub(crate) fn prepare_in_namespace(
         discard(&body_path)?;
         write_or_equal(&body_path, body)?;
     }
-    File::open(&directory_path)
+    crate::readonly_file::directory(&directory_path)
         .map_err(display)?
         .sync_all()
         .map_err(display)?;
@@ -227,7 +226,7 @@ fn directory(parent: &Path, path: &Path) -> Result<(), String> {
     {
         return Err("Boolean candidate namespace is not a real directory".to_owned());
     }
-    File::open(parent)
+    crate::readonly_file::directory(parent)
         .map_err(display)?
         .sync_all()
         .map_err(display)
@@ -238,7 +237,7 @@ fn directory(parent: &Path, path: &Path) -> Result<(), String> {
 /// scratch from an earlier attempt has been discarded, so any difference
 /// here is a different publication and is refused.
 fn write_or_equal(path: &Path, body: &[u8]) -> Result<(), String> {
-    match OpenOptions::new().write(true).create_new(true).open(path) {
+    match crate::readonly_file::regular(OpenOptions::new().write(true).create_new(true), path) {
         Ok(mut file) => {
             // WITHDRAWN, NOT LEFT (ledgers-2, D-1915). A file this call created
             // whose write or barrier failed stayed whole-length in the page
@@ -266,7 +265,7 @@ fn write_or_equal(path: &Path, body: &[u8]) -> Result<(), String> {
             }
             // The reused file is synced too, so a reuse never reports success
             // over bytes no barrier of this run reached (ledgers-2, D-1915).
-            File::open(path)
+            crate::readonly_file::read(path)
                 .and_then(|file| file.sync_all())
                 .map_err(display)?;
             Ok(())
diff --git a/crates/cli/src/candidate_trades.rs b/crates/cli/src/candidate_trades.rs
index 3d824eef..7a55510d 100644
--- a/crates/cli/src/candidate_trades.rs
+++ b/crates/cli/src/candidate_trades.rs
@@ -1310,12 +1310,10 @@ fn write_exact(path: &Path, magic: [u8; 8], payload: &[u8]) -> Result<[u8; 32],
     hash.update(&header);
     hash.update(payload);
     let digest = hash.finalize();
-    match OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create_new(true)
-        .open(path)
-    {
+    match crate::readonly_file::regular(
+        OpenOptions::new().read(true).write(true).create_new(true),
+        path,
+    ) {
         Ok(file) => {
             // A write or sync failure releases the lock through the guard's
             // explicit unlock, never by close (D-0693).
@@ -1339,7 +1337,7 @@ fn write_exact(path: &Path, magic: [u8; 8], payload: &[u8]) -> Result<[u8; 32],
     if observed != digest || actual != payload {
         return Err("immutable candidate detail already contains different bytes".to_owned());
     }
-    File::open(path.parent().ok_or("candidate detail path has no parent")?)
+    crate::readonly_file::directory(path.parent().ok_or("candidate detail path has no parent")?)
         .and_then(|file| file.sync_all())
         .map_err(io_error)?;
     #[cfg(test)]
diff --git a/crates/cli/src/candidate_universe.rs b/crates/cli/src/candidate_universe.rs
index 2bd12520..c5a72e3c 100644
--- a/crates/cli/src/candidate_universe.rs
+++ b/crates/cli/src/candidate_universe.rs
@@ -6563,12 +6563,11 @@ fn scan_orphan(
 }
 
 fn open_file(path: &Path, writable: bool, create: bool) -> Result<File, CandidateUniverseRefusal> {
-    OpenOptions::new()
-        .read(true)
-        .write(writable)
-        .create(create)
-        .open(path)
-        .map_err(|why| format!("cannot open candidate file {}: {why}", path.display()))
+    crate::readonly_file::regular(
+        OpenOptions::new().read(true).write(writable).create(create),
+        path,
+    )
+    .map_err(|why| format!("cannot open candidate file {}: {why}", path.display()))
 }
 
 fn file_generation(file: &File, path: &Path) -> Result<FileGenerationV1, CandidateUniverseRefusal> {
diff --git a/crates/cli/src/checksum_receipts.rs b/crates/cli/src/checksum_receipts.rs
index 9f312abf..e0c14483 100644
--- a/crates/cli/src/checksum_receipts.rs
+++ b/crates/cli/src/checksum_receipts.rs
@@ -352,7 +352,7 @@ fn publish(path: &Path, expected: &[u8; BYTES]) -> Result<(), String> {
             .map_err(error)?;
         }
         file.sync_all().map_err(error)?;
-        File::open(path.parent().ok_or("receipt parent absent")?)
+        crate::readonly_file::directory(path.parent().ok_or("receipt parent absent")?)
             .and_then(|dir| dir.sync_all())
             .map_err(error)?;
         regular_generation(&file, path)?;
@@ -374,7 +374,7 @@ fn namespace_directory(root: &Path, namespace: &str, create: bool) -> Result<Pat
     let base = root.join(namespace);
     if create {
         match fs::create_dir(&base) {
-            Ok(()) => File::open(&root)
+            Ok(()) => crate::readonly_file::directory(&root)
                 .and_then(|dir| dir.sync_all())
                 .map_err(error)?,
             Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
diff --git a/crates/cli/src/execution_capability.rs b/crates/cli/src/execution_capability.rs
index 4d599231..ddefdb38 100644
--- a/crates/cli/src/execution_capability.rs
+++ b/crates/cli/src/execution_capability.rs
@@ -1490,7 +1490,7 @@ impl ExecutionCapabilityLedger {
     /// [`Self::open`], plus any missing path.
     pub fn open_read(root: &Path) -> Result<Self, ExecutionCapabilityRefusal> {
         let lock_path = Self::lock_path(root);
-        let writer_lock = File::open(&lock_path)
+        let writer_lock = crate::readonly_file::read(&lock_path)
             .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
         writer_lock
             .lock_shared()
@@ -1498,16 +1498,16 @@ impl ExecutionCapabilityLedger {
         let opened = (|| {
             let paths = LedgerPaths::of(root);
             let files = LedgerFiles {
-                parameter: File::open(&paths.parameter).map_err(|why| {
+                parameter: crate::readonly_file::read(&paths.parameter).map_err(|why| {
                     format!("{} could not be opened: {why}", paths.parameter.display())
                 })?,
-                percentile: File::open(&paths.percentile).map_err(|why| {
+                percentile: crate::readonly_file::read(&paths.percentile).map_err(|why| {
                     format!("{} could not be opened: {why}", paths.percentile.display())
                 })?,
-                capability: File::open(&paths.capability).map_err(|why| {
+                capability: crate::readonly_file::read(&paths.capability).map_err(|why| {
                     format!("{} could not be opened: {why}", paths.capability.display())
                 })?,
-                completion: File::open(&paths.completion).map_err(|why| {
+                completion: crate::readonly_file::read(&paths.completion).map_err(|why| {
                     format!("{} could not be opened: {why}", paths.completion.display())
                 })?,
             };
@@ -2292,13 +2292,15 @@ fn append_encoded<const STRIDE: usize>(
 }
 
 fn open_or_create(path: &Path) -> Result<File, ExecutionCapabilityRefusal> {
-    OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create(true)
-        .truncate(false)
-        .open(path)
-        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
+    crate::readonly_file::regular(
+        OpenOptions::new()
+            .read(true)
+            .write(true)
+            .create(true)
+            .truncate(false),
+        path,
+    )
+    .map_err(|why| format!("{} could not be opened: {why}", path.display()))
 }
 
 fn ensure_record_header(
@@ -2511,7 +2513,7 @@ fn platform_generation(
 }
 
 fn sync_directory(path: &Path) -> Result<(), ExecutionCapabilityRefusal> {
-    File::open(path)
+    crate::readonly_file::directory(path)
         .and_then(|directory| directory.sync_all())
         .map_err(|why| {
             format!(
diff --git a/crates/cli/src/execution_disposition_v2.rs b/crates/cli/src/execution_disposition_v2.rs
index 0eb7726b..668278a2 100644
--- a/crates/cli/src/execution_disposition_v2.rs
+++ b/crates/cli/src/execution_disposition_v2.rs
@@ -2074,7 +2074,7 @@ impl ExecutionDispositionLedgerV2 {
     /// refuses any missing file.
     pub fn open_read(root: &Path) -> Result<Self, ExecutionDispositionRefusalV2> {
         let lock_path = Self::lock_path(root);
-        let writer_lock = File::open(&lock_path)
+        let writer_lock = crate::readonly_file::read(&lock_path)
             .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
         writer_lock
             .lock_shared()
@@ -2084,15 +2084,15 @@ impl ExecutionDispositionLedgerV2 {
             let percentile_path = Self::percentile_path(root);
             let row_path = Self::row_path(root);
             let completion_path = Self::completion_path(root);
-            let parameter_file = File::open(&parameter_path).map_err(|why| {
+            let parameter_file = crate::readonly_file::read(&parameter_path).map_err(|why| {
                 format!("{} could not be opened: {why}", parameter_path.display())
             })?;
-            let percentile_file = File::open(&percentile_path).map_err(|why| {
+            let percentile_file = crate::readonly_file::read(&percentile_path).map_err(|why| {
                 format!("{} could not be opened: {why}", percentile_path.display())
             })?;
-            let row_file = File::open(&row_path)
+            let row_file = crate::readonly_file::read(&row_path)
                 .map_err(|why| format!("{} could not be opened: {why}", row_path.display()))?;
-            let completion_file = File::open(&completion_path).map_err(|why| {
+            let completion_file = crate::readonly_file::read(&completion_path).map_err(|why| {
                 format!("{} could not be opened: {why}", completion_path.display())
             })?;
             Self::from_files(
@@ -2900,13 +2900,15 @@ fn block_slice<'a, T>(
 }
 
 fn open_or_create(path: &Path) -> Result<File, ExecutionDispositionRefusalV2> {
-    OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create(true)
-        .truncate(false)
-        .open(path)
-        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
+    crate::readonly_file::regular(
+        OpenOptions::new()
+            .read(true)
+            .write(true)
+            .create(true)
+            .truncate(false),
+        path,
+    )
+    .map_err(|why| format!("{} could not be opened: {why}", path.display()))
 }
 
 fn ensure_record_header(
@@ -3080,7 +3082,7 @@ fn sync_record_file(file: &File, path: &Path) -> Result<(), ExecutionDisposition
 }
 
 fn sync_directory(path: &Path) -> Result<(), ExecutionDispositionRefusalV2> {
-    File::open(path)
+    crate::readonly_file::directory(path)
         .and_then(|directory| directory.sync_all())
         .map_err(|why| {
             format!(
diff --git a/crates/cli/src/execution_v3.rs b/crates/cli/src/execution_v3.rs
index 67876928..3ba3f41d 100644
--- a/crates/cli/src/execution_v3.rs
+++ b/crates/cli/src/execution_v3.rs
@@ -129,7 +129,7 @@ const FORCED_STOP_INCLUDE_TAG: u8 = 1;
 const FORCED_STOP_REQUIRE_TAG: u8 = 2;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(PARAMETER_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V3_PARAMETER_BYTES);
 const _: () = assert!(PERCENTILE_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V3_PERCENTILE_BYTES);
@@ -3778,7 +3778,7 @@ fn open_root_directory(
         )
     })?;
     require_not_symlink(&canonical, false)?;
-    let file = File::open(&canonical).map_err(|why| {
+    let file = crate::readonly_file::directory(&canonical).map_err(|why| {
         format!(
             "cannot hold Execution V3 root {}: {why}",
             canonical.display()
@@ -3824,7 +3824,7 @@ fn open_child(
         let mut options = OpenOptions::new();
         options.read(true).write(true).create_new(true);
         #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-        options.custom_flags(O_NOFOLLOW_FLAG);
+        options.custom_flags(NOFOLLOW_NONBLOCK);
         match options.open(path) {
             Ok(file) => {
                 require_regular_file(&file, path)?;
@@ -3843,7 +3843,7 @@ fn open_child(
     let mut options = OpenOptions::new();
     options.read(true).write(writable).truncate(false);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open Execution V3 file {}: {why}", path.display()))?;
diff --git a/crates/cli/src/execution_v4.rs b/crates/cli/src/execution_v4.rs
index d9aa9963..56c49300 100644
--- a/crates/cli/src/execution_v4.rs
+++ b/crates/cli/src/execution_v4.rs
@@ -137,7 +137,7 @@ const FORCED_STOP_INCLUDE_TAG: u8 = 1;
 const FORCED_STOP_REQUIRE_TAG: u8 = 2;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(PARAMETER_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V4_PARAMETER_BYTES);
 const _: () = assert!(PERCENTILE_PAYLOAD_BYTES + SEAL_BYTES == EXECUTION_V4_PERCENTILE_BYTES);
@@ -4680,7 +4680,7 @@ fn open_root_directory(
         )
     })?;
     require_not_symlink(&canonical, false)?;
-    let file = File::open(&canonical).map_err(|why| {
+    let file = crate::readonly_file::directory(&canonical).map_err(|why| {
         format!(
             "cannot hold Execution V4 root {}: {why}",
             canonical.display()
@@ -4726,7 +4726,7 @@ fn open_child(
         let mut options = OpenOptions::new();
         options.read(true).write(true).create_new(true);
         #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-        options.custom_flags(O_NOFOLLOW_FLAG);
+        options.custom_flags(NOFOLLOW_NONBLOCK);
         match options.open(path) {
             Ok(file) => {
                 require_regular_file(&file, path)?;
@@ -4745,7 +4745,7 @@ fn open_child(
     let mut options = OpenOptions::new();
     options.read(true).write(writable).truncate(false);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open Execution V4 file {}: {why}", path.display()))?;
diff --git a/crates/cli/src/expression.rs b/crates/cli/src/expression.rs
index 2ce4fdd0..9125147f 100644
--- a/crates/cli/src/expression.rs
+++ b/crates/cli/src/expression.rs
@@ -222,7 +222,7 @@ pub(crate) fn attempt_directory(
         }
         // Sync each parent entry, including the format and identity directories.
         // Syncing only the leaf would not persist newly created ancestors.
-        File::open(&parent)?.sync_all()?;
+        crate::readonly_file::directory(&parent)?.sync_all()?;
     }
     Ok(directory)
 }
@@ -276,11 +276,13 @@ impl EvidenceWriter {
         expression: &Expression,
     ) -> std::io::Result<Self> {
         let pending = directory.join("expression-v1.pending");
-        let opened = fs::OpenOptions::new()
-            .read(true)
-            .write(true)
-            .create_new(true)
-            .open(&pending)?;
+        let opened = crate::readonly_file::regular(
+            fs::OpenOptions::new()
+                .read(true)
+                .write(true)
+                .create_new(true),
+            &pending,
+        )?;
         let lock = Flock::lock(opened.try_clone()?, pending.clone())?;
         let mut file = BufWriter::new(opened);
         let mut hash = brutex_core::blake3::Hasher::new();
@@ -289,7 +291,7 @@ impl EvidenceWriter {
         hash.update(&header);
         file.flush()?;
         file.get_ref().sync_all()?;
-        File::open(directory)?.sync_all()?;
+        crate::readonly_file::directory(directory)?.sync_all()?;
         let header_seal = hash.finalize();
         Ok(Self {
             pending,
@@ -389,7 +391,7 @@ impl EvidenceWriter {
             .map_err(std::io::Error::other)?;
         fs::remove_file(&self.pending)?;
         if let Some(directory) = self.published.parent() {
-            File::open(directory)?.sync_all()?;
+            crate::readonly_file::directory(directory)?.sync_all()?;
         }
         let published = self.published;
         self.lock.release()?;
diff --git a/crates/cli/src/frontier.rs b/crates/cli/src/frontier.rs
index 4fb513aa..685ecbf8 100644
--- a/crates/cli/src/frontier.rs
+++ b/crates/cli/src/frontier.rs
@@ -835,7 +835,7 @@ impl Frontier {
     /// length when it exceeds `max_bytes`. No row is indexed in that case.
     pub fn open_read_bounded(root: &Path, max_bytes: u64) -> Result<Self, Refusal> {
         let path = Self::path(root);
-        let mut file = File::open(&path).map_err(|why| {
+        let mut file = crate::readonly_file::read(&path).map_err(|why| {
             if why.kind() == std::io::ErrorKind::NotFound {
                 format!(
                     "{} does not exist yet. No run has recorded a frontier — \
@@ -908,13 +908,15 @@ impl Frontier {
         std::fs::create_dir_all(&dir)
             .map_err(|why| format!("the results directory could not be made: {why}"))?;
         let path = Self::path(root);
-        let file = OpenOptions::new()
-            .read(true)
-            .write(true)
-            .create(true)
-            .truncate(false)
-            .open(&path)
-            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
+        let file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .read(true)
+                .write(true)
+                .create(true)
+                .truncate(false),
+            &path,
+        )
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
         // THE WRITER'S OPEN HOLDS THE EXCLUSIVE LOCK while it measures, cuts
         // a torn tail and indexes (D-1901, sweep-2). Unlocked, the length could
         // land inside another writer's live append, and cutting THAT would
diff --git a/crates/cli/src/global_replay.rs b/crates/cli/src/global_replay.rs
index c1ad98c9..e1726358 100644
--- a/crates/cli/src/global_replay.rs
+++ b/crates/cli/src/global_replay.rs
@@ -2671,23 +2671,23 @@ impl GlobalReplayLedger {
     pub fn open_read(root: &Path, max_completions: usize) -> Result<Self, GlobalReplayRefusal> {
         validate_completion_bound(max_completions)?;
         let paths = ReplayPaths::of(root);
-        let writer_lock = File::open(&paths.lock)
+        let writer_lock = crate::readonly_file::read(&paths.lock)
             .map_err(|why| format!("{} could not be opened: {why}", paths.lock.display()))?;
         writer_lock
             .lock_shared()
             .map_err(|why| format!("{} could not be shared-locked: {why}", paths.lock.display()))?;
         let opened = (|| {
             let files = ReplayFiles {
-                stream: File::open(&paths.stream).map_err(|why| {
+                stream: crate::readonly_file::read(&paths.stream).map_err(|why| {
                     format!("{} could not be opened: {why}", paths.stream.display())
                 })?,
-                decision: File::open(&paths.decision).map_err(|why| {
+                decision: crate::readonly_file::read(&paths.decision).map_err(|why| {
                     format!("{} could not be opened: {why}", paths.decision.display())
                 })?,
-                trade: File::open(&paths.trade).map_err(|why| {
+                trade: crate::readonly_file::read(&paths.trade).map_err(|why| {
                     format!("{} could not be opened: {why}", paths.trade.display())
                 })?,
-                completion: File::open(&paths.completion).map_err(|why| {
+                completion: crate::readonly_file::read(&paths.completion).map_err(|why| {
                     format!("{} could not be opened: {why}", paths.completion.display())
                 })?,
             };
@@ -3180,13 +3180,15 @@ fn validate_completion_bound(max_completions: usize) -> Result<(), GlobalReplayR
 }
 
 fn open_or_create(path: &Path) -> Result<File, GlobalReplayRefusal> {
-    OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create(true)
-        .truncate(false)
-        .open(path)
-        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
+    crate::readonly_file::regular(
+        OpenOptions::new()
+            .read(true)
+            .write(true)
+            .create(true)
+            .truncate(false),
+        path,
+    )
+    .map_err(|why| format!("{} could not be opened: {why}", path.display()))
 }
 
 fn ensure_header(
diff --git a/crates/cli/src/global_replay_v2.rs b/crates/cli/src/global_replay_v2.rs
index c4435618..c170360a 100644
--- a/crates/cli/src/global_replay_v2.rs
+++ b/crates/cli/src/global_replay_v2.rs
@@ -2089,7 +2089,7 @@ impl GlobalReplayLedgerV2 {
     pub fn open_read(root: &Path, max_completions: usize) -> Result<Self, GlobalReplayRefusalV2> {
         validate_completion_bound_v2(max_completions)?;
         let paths = ReplayPathsV2::of(root);
-        let writer_lock = File::open(&paths.lock)
+        let writer_lock = crate::readonly_file::read(&paths.lock)
             .map_err(|why| format!("{} could not be opened: {why}", paths.lock.display()))?;
         writer_lock
             .lock_shared()
@@ -2735,17 +2735,20 @@ fn validate_completion_bound_v2(max_completions: usize) -> Result<(), GlobalRepl
 }
 
 fn open_or_create_v2(path: &Path) -> Result<File, GlobalReplayRefusalV2> {
-    OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create(true)
-        .truncate(false)
-        .open(path)
-        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
+    crate::readonly_file::regular(
+        OpenOptions::new()
+            .read(true)
+            .write(true)
+            .create(true)
+            .truncate(false),
+        path,
+    )
+    .map_err(|why| format!("{} could not be opened: {why}", path.display()))
 }
 
 fn open_read_v2(path: &Path) -> Result<File, GlobalReplayRefusalV2> {
-    File::open(path).map_err(|why| format!("{} could not be opened: {why}", path.display()))
+    crate::readonly_file::read(path)
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
 }
 
 fn ensure_header_v2(
diff --git a/crates/cli/src/global_replay_v3.rs b/crates/cli/src/global_replay_v3.rs
index d510d7a5..81247cd6 100644
--- a/crates/cli/src/global_replay_v3.rs
+++ b/crates/cli/src/global_replay_v3.rs
@@ -2535,30 +2535,31 @@ impl GlobalReplayLedgerV3 {
             &paths.money,
             &paths.completion,
         ] {
-            OpenOptions::new()
-                .create(true)
-                .append(true)
-                .read(true)
-                .open(path)
-                .map_err(|why| {
-                    format!(
-                        "Global Replay V3 authority file {} could not be opened: {why}",
-                        path.display()
-                    )
-                })?;
-        }
-        let writer_lock = OpenOptions::new()
-            .create(true)
-            .truncate(false)
-            .read(true)
-            .write(true)
-            .open(&paths.lock)
+            crate::readonly_file::regular(
+                OpenOptions::new().create(true).append(true).read(true),
+                path,
+            )
             .map_err(|why| {
                 format!(
-                    "Global Replay V3 writer lock {} could not be opened: {why}",
-                    paths.lock.display()
+                    "Global Replay V3 authority file {} could not be opened: {why}",
+                    path.display()
                 )
             })?;
+        }
+        let writer_lock = crate::readonly_file::regular(
+            OpenOptions::new()
+                .create(true)
+                .truncate(false)
+                .read(true)
+                .write(true),
+            &paths.lock,
+        )
+        .map_err(|why| {
+            format!(
+                "Global Replay V3 writer lock {} could not be opened: {why}",
+                paths.lock.display()
+            )
+        })?;
         let ledger = Self {
             paths,
             bounds,
@@ -2906,16 +2907,15 @@ fn append_records<T, const N: usize>(
 where
     T: Copy,
 {
-    let mut file = OpenOptions::new()
-        .create(true)
-        .append(true)
-        .open(path)
-        .map_err(|why| {
-            format!(
-                "Global Replay V3 append file {} could not be opened: {why}",
-                path.display()
-            )
-        })?;
+    let mut file =
+        crate::readonly_file::regular(OpenOptions::new().create(true).append(true), path).map_err(
+            |why| {
+                format!(
+                    "Global Replay V3 append file {} could not be opened: {why}",
+                    path.display()
+                )
+            },
+        )?;
     // The same rollback law as V1 and V2: a refused append leaves this file
     // exactly as long as it was (D-0926).
     crate::global_replay::append_encoded_with(&mut file, path, records.iter().copied().map(encode))
@@ -2937,7 +2937,7 @@ fn read_records<T, const N: usize>(
     if stride != N || N == 0 {
         return Err("Global Replay V3 fixed-record stride contract is invalid".to_owned());
     }
-    let mut file = OpenOptions::new().read(true).open(path).map_err(|why| {
+    let mut file = crate::readonly_file::read(path).map_err(|why| {
         format!(
             "Global Replay V3 authority file {} could not be read: {why}",
             path.display()
diff --git a/crates/cli/src/global_replay_v4_store.rs b/crates/cli/src/global_replay_v4_store.rs
index c194b5ab..1aef7eea 100644
--- a/crates/cli/src/global_replay_v4_store.rs
+++ b/crates/cli/src/global_replay_v4_store.rs
@@ -69,7 +69,7 @@ pub(super) fn persist(
             expected_bytes,
         )?;
         file.sync_all().map_err(|why| why.to_string())?;
-        File::open(&root)
+        crate::readonly_file::directory(&root)
             .and_then(|directory| directory.sync_all())
             .map_err(|why| why.to_string())?;
         crate::result_set::file_generation(&file, &path)?;
@@ -179,7 +179,7 @@ fn open(path: &Path, writable: bool) -> Result<File, String> {
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
     {
         use std::os::unix::fs::OpenOptionsExt as _;
-        options.custom_flags(store::open_flags::O_NOFOLLOW);
+        options.custom_flags(store::open_flags::O_NOFOLLOW_NONBLOCK);
     }
     let file = options.open(path).map_err(|why| why.to_string())?;
     let metadata = file.metadata().map_err(|why| why.to_string())?;
diff --git a/crates/cli/src/institutional_statistics.rs b/crates/cli/src/institutional_statistics.rs
index 8a90e80d..91aa65d0 100644
--- a/crates/cli/src/institutional_statistics.rs
+++ b/crates/cli/src/institutional_statistics.rs
@@ -1036,7 +1036,7 @@ impl InstitutionalStatisticsLedgerV1 {
     pub fn open_read(root: &Path, max_records: usize) -> Result<Self, String> {
         validate_bound(max_records)?;
         let paths = StatisticsPathsV1::of(root);
-        let writer_lock = File::open(&paths.lock)
+        let writer_lock = crate::readonly_file::read(&paths.lock)
             .map_err(|why| format!("{} could not be opened: {why}", paths.lock.display()))?;
         writer_lock
             .lock_shared()
@@ -1414,17 +1414,20 @@ fn scan_records<const STRIDE: usize, T>(
 }
 
 fn open_or_create(path: &Path) -> Result<File, String> {
-    OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create(true)
-        .truncate(false)
-        .open(path)
-        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
+    crate::readonly_file::regular(
+        OpenOptions::new()
+            .read(true)
+            .write(true)
+            .create(true)
+            .truncate(false),
+        path,
+    )
+    .map_err(|why| format!("{} could not be opened: {why}", path.display()))
 }
 
 fn open_read(path: &Path) -> Result<File, String> {
-    File::open(path).map_err(|why| format!("{} could not be opened: {why}", path.display()))
+    crate::readonly_file::read(path)
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
 }
 
 fn append_sync(file: &mut File, path: &Path, bytes: &[u8]) -> Result<(), String> {
diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index 4ae07321..c9ead1e6 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -92,7 +92,7 @@ mod fixed_tail;
 mod g18_cli_a_tests;
 #[cfg(test)]
 mod operator_boundary_tests;
-mod readonly_file;
+pub mod readonly_file;
 #[cfg(test)]
 mod results_report_tests;
 #[cfg(test)]
@@ -19898,7 +19898,7 @@ fn canonical_underlying(word: &str) -> String {
 /// the ledger's own name is confirmed before success is returned.
 fn confirm_result_directory(root: &std::path::Path) -> Result<(), String> {
     let path = root.join("results");
-    let directory = std::fs::File::open(&path).map_err(|why| {
+    let directory = crate::readonly_file::directory(&path).map_err(|why| {
         format!(
             "{} could not be opened for a durability barrier: {why}",
             path.display()
@@ -22854,13 +22854,15 @@ impl ResultSetLock {
         std::fs::create_dir_all(&dir)
             .map_err(|why| format!("the results directory could not be made: {why}"))?;
         let path = dir.join("write.lock");
-        let file = std::fs::OpenOptions::new()
-            .read(true)
-            .write(true)
-            .create(true)
-            .truncate(false)
-            .open(&path)
-            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
+        let file = crate::readonly_file::regular(
+            std::fs::OpenOptions::new()
+                .read(true)
+                .write(true)
+                .create(true)
+                .truncate(false),
+            &path,
+        )
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
         let held = store::flock::Flock::lock(file, path.clone())
             .map_err(|why| format!("{} could not be locked: {why}", path.display()))?;
         Ok(Self(held))
diff --git a/crates/cli/src/live.rs b/crates/cli/src/live.rs
index 589b6ab9..073fea60 100644
--- a/crates/cli/src/live.rs
+++ b/crates/cli/src/live.rs
@@ -107,7 +107,6 @@
 //! lying. Reusing the row means the page renders both with one decoder, and a
 //! field added to the frontier appears here without a second edit.
 
-use std::fs::File;
 use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
 use std::path::{Path, PathBuf};
 
@@ -394,7 +393,13 @@ impl Live {
             .path
             .with_extension(format!("{}.tmp", std::process::id()));
         let write = || -> std::io::Result<()> {
-            let mut file = File::create(&temp)?;
+            let mut file = crate::readonly_file::regular(
+                std::fs::OpenOptions::new()
+                    .write(true)
+                    .create(true)
+                    .truncate(true),
+                &temp,
+            )?;
             file.write_all(&body)?;
             // `sync_data` and not `sync_all`: this file is not history, and a
             // torn live view costs a poll rather than a run. The ledger's own
@@ -740,7 +745,7 @@ fn read_one_with_limit(
     now: std::time::SystemTime,
     row_limit: Option<usize>,
 ) -> Option<Run> {
-    let mut file = File::open(path).ok()?;
+    let mut file = crate::readonly_file::read(path).ok()?;
     let mut header = [0_u8; HEADER_BYTES];
     file.read_exact(&mut header).ok()?;
     if header.get(0..8)? != MAGIC {
diff --git a/crates/cli/src/operation_audit.rs b/crates/cli/src/operation_audit.rs
index 7bc3019d..7b049eed 100644
--- a/crates/cli/src/operation_audit.rs
+++ b/crates/cli/src/operation_audit.rs
@@ -282,11 +282,11 @@ fn directory(path: &Path) -> Result<(), String> {
     {
         return Err(error("directory symlinks are refused"));
     }
-    File::open(path)
+    crate::readonly_file::directory(path)
         .and_then(|file| file.sync_all())
         .map_err(error)?;
     if let Some(parent) = path.parent() {
-        File::open(parent)
+        crate::readonly_file::directory(parent)
             .and_then(|file| file.sync_all())
             .map_err(error)?;
     }
@@ -586,7 +586,7 @@ pub fn begin(root: &Path, origin: Origin, label: &str) -> Result<Attempt, String
         .open(own(&base, id))
         .map_err(error)?;
     write_synced(&mut file, &image)?;
-    File::open(&base)
+    crate::readonly_file::directory(&base)
         .and_then(|file| file.sync_all())
         .map_err(error)?;
     Ok(Attempt {
diff --git a/crates/cli/src/pool_oos.rs b/crates/cli/src/pool_oos.rs
index 123fc54f..2d9feb48 100644
--- a/crates/cli/src/pool_oos.rs
+++ b/crates/cli/src/pool_oos.rs
@@ -463,11 +463,11 @@ pub(crate) fn write_catalog(
     }
     {
         use std::io::Write as _;
-        let mut file = std::fs::OpenOptions::new()
-            .write(true)
-            .create_new(true)
-            .open(path)
-            .map_err(|why| format!("cannot create catalog {}: {why}", path.display()))?;
+        let mut file = crate::readonly_file::regular(
+            std::fs::OpenOptions::new().write(true).create_new(true),
+            path,
+        )
+        .map_err(|why| format!("cannot create catalog {}: {why}", path.display()))?;
         file.write_all(text.as_bytes())
             .and_then(|()| file.sync_all())
             .map_err(|why| format!("catalog {} was not written whole: {why}", path.display()))?;
diff --git a/crates/cli/src/population.rs b/crates/cli/src/population.rs
index f5e126ac..00c1b7ab 100644
--- a/crates/cli/src/population.rs
+++ b/crates/cli/src/population.rs
@@ -2617,17 +2617,17 @@ impl PopulationLedger {
         let receipt_v3_path = Self::receipt_v3_path(root);
         let receipt_v4_path = Self::receipt_v4_path(root);
         let lock_path = Self::lock_path(root);
-        let writer_lock = File::open(&lock_path)
+        let writer_lock = crate::readonly_file::read(&lock_path)
             .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
         writer_lock
             .lock_shared()
             .map_err(|why| format!("{} could not be shared-locked: {why}", lock_path.display()))?;
         let opened = (|| {
-            let row_file = File::open(&row_path)
+            let row_file = crate::readonly_file::read(&row_path)
                 .map_err(|why| format!("{} could not be opened: {why}", row_path.display()))?;
-            let receipt_file = File::open(&receipt_path)
+            let receipt_file = crate::readonly_file::read(&receipt_path)
                 .map_err(|why| format!("{} could not be opened: {why}", receipt_path.display()))?;
-            let receipt_v3_file = match File::open(&receipt_v3_path) {
+            let receipt_v3_file = match crate::readonly_file::read(&receipt_v3_path) {
                 Ok(file) => Some(file),
                 Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
                 Err(why) => {
@@ -2637,7 +2637,7 @@ impl PopulationLedger {
                     ));
                 }
             };
-            let receipt_v4_file = match File::open(&receipt_v4_path) {
+            let receipt_v4_file = match crate::readonly_file::read(&receipt_v4_path) {
                 Ok(file) => Some(file),
                 Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
                 Err(why) => {
@@ -2690,13 +2690,15 @@ impl PopulationLedger {
         std::fs::create_dir_all(&dir)
             .map_err(|why| format!("the results directory could not be made: {why}"))?;
         let lock_path = Self::lock_path(root);
-        let writer_lock = OpenOptions::new()
-            .read(true)
-            .write(true)
-            .create(true)
-            .truncate(false)
-            .open(&lock_path)
-            .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
+        let writer_lock = crate::readonly_file::regular(
+            OpenOptions::new()
+                .read(true)
+                .write(true)
+                .create(true)
+                .truncate(false),
+            &lock_path,
+        )
+        .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
         writer_lock
             .lock()
             .map_err(|why| format!("{} could not be locked: {why}", lock_path.display()))?;
@@ -4925,13 +4927,15 @@ fn read_receipt_v4_at(
 }
 
 fn open_or_create(path: &Path) -> Result<File, PopulationRefusal> {
-    OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create(true)
-        .truncate(false)
-        .open(path)
-        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
+    crate::readonly_file::regular(
+        OpenOptions::new()
+            .read(true)
+            .write(true)
+            .create(true)
+            .truncate(false),
+        path,
+    )
+    .map_err(|why| format!("{} could not be opened: {why}", path.display()))
 }
 
 fn ensure_header(
@@ -5225,7 +5229,7 @@ fn require_optional_generation_unchanged(
 }
 
 fn sync_directory(path: &Path) -> Result<(), PopulationRefusal> {
-    File::open(path)
+    crate::readonly_file::directory(path)
         .and_then(|directory| directory.sync_all())
         .map_err(|why| {
             format!(
diff --git a/crates/cli/src/population_admission_v2.rs b/crates/cli/src/population_admission_v2.rs
index 4321b4cd..a2bb2700 100644
--- a/crates/cli/src/population_admission_v2.rs
+++ b/crates/cli/src/population_admission_v2.rs
@@ -77,7 +77,7 @@ const LOCK_FILE: &str = "population-admission-v2.lock";
 const READ_CHUNK_BYTES: usize = 16 * 1_024;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () =
     assert!(DECISION_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_ADMISSION_V2_DECISION_BYTES);
@@ -2765,7 +2765,7 @@ fn open_root_directory(
         )
     })?;
     require_not_symlink(&canonical, false)?;
-    let file = File::open(&canonical).map_err(|why| {
+    let file = crate::readonly_file::directory(&canonical).map_err(|why| {
         format!(
             "cannot hold Admission V2 root {}: {why}",
             canonical.display()
@@ -2788,7 +2788,7 @@ fn open_root_directory(
 
 fn named_root_identity(root: &Path) -> Result<PlatformIdentity, PopulationAdmissionV2Refusal> {
     require_not_symlink(root, false)?;
-    let file = File::open(root).map_err(|why| {
+    let file = crate::readonly_file::directory(root).map_err(|why| {
         format!(
             "cannot reopen named Admission V2 root {}: {why}",
             root.display()
@@ -2819,7 +2819,7 @@ fn open_child(
         let mut create_options = OpenOptions::new();
         create_options.read(true).write(true).create_new(true);
         #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-        create_options.custom_flags(O_NOFOLLOW_FLAG);
+        create_options.custom_flags(NOFOLLOW_NONBLOCK);
         match create_options.open(path) {
             Ok(file) => {
                 require_regular_file(&file, path)?;
@@ -2838,7 +2838,7 @@ fn open_child(
     let mut options = OpenOptions::new();
     options.read(true).write(writable).truncate(false);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open Admission V2 file {}: {why}", path.display()))?;
diff --git a/crates/cli/src/population_admission_v3.rs b/crates/cli/src/population_admission_v3.rs
index 284dd59e..9b13fd26 100644
--- a/crates/cli/src/population_admission_v3.rs
+++ b/crates/cli/src/population_admission_v3.rs
@@ -109,7 +109,7 @@ const EVIDENCE_STATISTICS_OFFSET: usize = RUNNER_HEADER_BYTES + EVIDENCE_BASE_VA
 const EVIDENCE_WALK_OFFSET: usize = 800;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () =
     assert!(DECISION_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_ADMISSION_V3_DECISION_BYTES);
@@ -4679,7 +4679,7 @@ fn open_root_directory(
         )
     })?;
     require_not_symlink(&canonical, false)?;
-    let file = File::open(&canonical).map_err(|why| {
+    let file = crate::readonly_file::directory(&canonical).map_err(|why| {
         format!(
             "cannot hold Admission V3 root {}: {why}",
             canonical.display()
@@ -4702,7 +4702,7 @@ fn open_root_directory(
 
 fn named_root_identity(root: &Path) -> Result<PlatformIdentity, PopulationAdmissionV3Refusal> {
     require_not_symlink(root, false)?;
-    let file = File::open(root).map_err(|why| {
+    let file = crate::readonly_file::directory(root).map_err(|why| {
         format!(
             "cannot reopen named Admission V3 root {}: {why}",
             root.display()
@@ -4733,7 +4733,7 @@ fn open_child(
         let mut create_options = OpenOptions::new();
         create_options.read(true).write(true).create_new(true);
         #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-        create_options.custom_flags(O_NOFOLLOW_FLAG);
+        create_options.custom_flags(NOFOLLOW_NONBLOCK);
         match create_options.open(path) {
             Ok(file) => {
                 require_regular_file(&file, path)?;
@@ -4752,7 +4752,7 @@ fn open_child(
     let mut options = OpenOptions::new();
     options.read(true).write(writable).truncate(false);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open Admission V3 file {}: {why}", path.display()))?;
diff --git a/crates/cli/src/population_admission_v4.rs b/crates/cli/src/population_admission_v4.rs
index f8723356..820a4253 100644
--- a/crates/cli/src/population_admission_v4.rs
+++ b/crates/cli/src/population_admission_v4.rs
@@ -95,7 +95,7 @@ const RUNNER_VERDICT_OFFSET: usize = RUNNER_EVIDENCE_OFFSET + RUNNER_EVIDENCE_BY
 const READ_CHUNK_BYTES: usize = 16 * 1_024;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(RUNNER_VERDICT_OFFSET + RUNNER_VERDICT_BYTES == RUNNER_DECISION_BYTES);
 const _: () = assert!(PAYLOAD_BYTES + 32 == RECORD_BYTES);
@@ -3103,8 +3103,8 @@ fn open_root(
     }
     let canonical = std::fs::canonicalize(root)
         .map_err(|why| format!("cannot canonicalize Admission V4 root: {why}"))?;
-    let file =
-        File::open(&canonical).map_err(|why| format!("cannot open Admission V4 root: {why}"))?;
+    let file = crate::readonly_file::directory(&canonical)
+        .map_err(|why| format!("cannot open Admission V4 root: {why}"))?;
     let identity = PlatformIdentity::of(
         &file
             .metadata()
@@ -3125,7 +3125,7 @@ fn open_child(path: &Path, writable: bool) -> Result<(File, bool), PopulationAdm
     let mut options = OpenOptions::new();
     options.read(true).write(writable).create(writable);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let existed = path.exists();
     let file = options
         .open(path)
diff --git a/crates/cli/src/population_base_evidence_ledger_v2.rs b/crates/cli/src/population_base_evidence_ledger_v2.rs
index ab16b5c5..092f7c0e 100644
--- a/crates/cli/src/population_base_evidence_ledger_v2.rs
+++ b/crates/cli/src/population_base_evidence_ledger_v2.rs
@@ -890,8 +890,8 @@ impl LedgerV2 {
     ) -> Result<Self, BaseEvidenceLedgerRefusalV2> {
         let root = admit_root(root)?;
         refuse_retired_v2_ledger(&root)?;
-        let root_file =
-            File::open(&root).map_err(|why| io_error("hold Base ledger root", &root, &why))?;
+        let root_file = crate::readonly_file::directory(&root)
+            .map_err(|why| io_error("hold Base ledger root", &root, &why))?;
         if !root_file
             .metadata()
             .map_err(|why| io_error("stat held Base ledger root", &root, &why))?
@@ -1780,12 +1780,11 @@ fn open_file(
     writable: bool,
     create: bool,
 ) -> Result<File, BaseEvidenceLedgerRefusalV2> {
-    OpenOptions::new()
-        .read(true)
-        .write(writable)
-        .create(create)
-        .open(path)
-        .map_err(|why| io_error("open", path, &why))
+    crate::readonly_file::regular(
+        OpenOptions::new().read(true).write(writable).create(create),
+        path,
+    )
+    .map_err(|why| io_error("open", path, &why))
 }
 
 fn sync_directory(root_file: &File, root: &Path) -> Result<(), BaseEvidenceLedgerRefusalV2> {
diff --git a/crates/cli/src/population_finalization_v2.rs b/crates/cli/src/population_finalization_v2.rs
index 4cb99f71..f472aca2 100644
--- a/crates/cli/src/population_finalization_v2.rs
+++ b/crates/cli/src/population_finalization_v2.rs
@@ -116,7 +116,7 @@ const LOCK_FILE: &str = "population-finalization-v2.lock";
 const READ_CHUNK_BYTES: usize = 16 * 1_024;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(PAYLOAD_BYTES + SEAL_BYTES == POPULATION_FINALIZATION_V2_RECORD_BYTES);
 
@@ -2481,7 +2481,7 @@ fn open_root_directory(root: &Path) -> Result<File, PopulationFinalizationV2Refu
         ));
     }
     require_not_symlink(root, false)?;
-    let root_file = File::open(root).map_err(|why| {
+    let root_file = crate::readonly_file::directory(root).map_err(|why| {
         format!(
             "Finalization V2 root {} must already exist: {why}",
             root.display()
@@ -2560,7 +2560,7 @@ fn open_ledger_file(
         .create(create)
         .truncate(false);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open Finalization V2 file {}: {why}", path.display()))?;
@@ -2787,7 +2787,7 @@ fn directory_generation(
             path.display()
         )
     })?;
-    let named = File::open(path).map_err(|why| {
+    let named = crate::readonly_file::directory(path).map_err(|why| {
         format!(
             "cannot reopen named Finalization V2 root {}: {why}",
             path.display()
@@ -2859,7 +2859,7 @@ fn file_generation(
             held_before.len()
         ));
     }
-    let mut named = File::open(path).map_err(|why| {
+    let mut named = crate::readonly_file::read(path).map_err(|why| {
         format!(
             "cannot reopen named Finalization V2 file {}: {why}",
             path.display()
diff --git a/crates/cli/src/population_finalization_v3.rs b/crates/cli/src/population_finalization_v3.rs
index ed7575b5..0a4412b7 100644
--- a/crates/cli/src/population_finalization_v3.rs
+++ b/crates/cli/src/population_finalization_v3.rs
@@ -86,7 +86,7 @@ const LOCK_MAX_BYTES: u64 = 0;
 const READ_CHUNK_BYTES: usize = 16 * 1_024;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(ROW_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_FINALIZATION_V3_ROW_BYTES);
 const _: () =
@@ -2936,7 +2936,7 @@ fn open_root_directory(
         )
     })?;
     require_not_symlink(&canonical, false)?;
-    let file = File::open(&canonical).map_err(|why| {
+    let file = crate::readonly_file::directory(&canonical).map_err(|why| {
         format!(
             "cannot hold Finalization V3 root {}: {why}",
             canonical.display()
@@ -2959,7 +2959,7 @@ fn open_root_directory(
 
 fn named_root_identity(root: &Path) -> Result<PlatformIdentity, PopulationFinalizationV3Refusal> {
     require_not_symlink(root, false)?;
-    let file = File::open(root).map_err(|why| {
+    let file = crate::readonly_file::directory(root).map_err(|why| {
         format!(
             "cannot reopen named Finalization V3 root {}: {why}",
             root.display()
@@ -2990,7 +2990,7 @@ fn open_child(
         let mut create_options = OpenOptions::new();
         create_options.read(true).write(true).create_new(true);
         #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-        create_options.custom_flags(O_NOFOLLOW_FLAG);
+        create_options.custom_flags(NOFOLLOW_NONBLOCK);
         match create_options.open(path) {
             Ok(file) => {
                 require_regular_file(&file, path)?;
@@ -3009,7 +3009,7 @@ fn open_child(
     let mut options = OpenOptions::new();
     options.read(true).write(writable).truncate(false);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open Finalization V3 file {}: {why}", path.display()))?;
diff --git a/crates/cli/src/population_finalization_v4.rs b/crates/cli/src/population_finalization_v4.rs
index 252a44d9..a70455a0 100644
--- a/crates/cli/src/population_finalization_v4.rs
+++ b/crates/cli/src/population_finalization_v4.rs
@@ -85,7 +85,7 @@ const RUNNER_POLICY_END: usize = RUNNER_HEADER_BYTES + RUNNER_POLICY_BYTES;
 const READ_CHUNK_BYTES: usize = 16 * 1_024;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(PAYLOAD_BYTES + 32 == RECORD_BYTES);
 
@@ -2863,8 +2863,8 @@ fn open_root(
     }
     let canonical = std::fs::canonicalize(root)
         .map_err(|why| format!("cannot canonicalize Finalization V4 root: {why}"))?;
-    let file =
-        File::open(&canonical).map_err(|why| format!("cannot open Finalization V4 root: {why}"))?;
+    let file = crate::readonly_file::directory(&canonical)
+        .map_err(|why| format!("cannot open Finalization V4 root: {why}"))?;
     let identity = PlatformIdentity::of(
         &file
             .metadata()
@@ -2888,7 +2888,7 @@ fn open_child(
     let mut options = OpenOptions::new();
     options.read(true).write(writable).create(writable);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let existed = path.exists();
     let file = options
         .open(path)
diff --git a/crates/cli/src/population_observations_v1.rs b/crates/cli/src/population_observations_v1.rs
index 30c69e05..83d381ce 100644
--- a/crates/cli/src/population_observations_v1.rs
+++ b/crates/cli/src/population_observations_v1.rs
@@ -2138,17 +2138,19 @@ impl ObservationAuthorityLedgerV1 {
         let admitted_root = admit_existing_observation_root(root)?;
         let file_path = admitted_root.join(AUTHORITY_FILE);
         let lock_path = admitted_root.join(AUTHORITY_LOCK_FILE);
-        let lock = OpenOptions::new()
-            .read(true)
-            .write(writable)
-            .create(writable)
-            .open(&lock_path)
-            .map_err(|why| {
-                format!(
-                    "cannot open observation authority lock {}: {why}",
-                    lock_path.display()
-                )
-            })?;
+        let lock = crate::readonly_file::regular(
+            OpenOptions::new()
+                .read(true)
+                .write(writable)
+                .create(writable),
+            &lock_path,
+        )
+        .map_err(|why| {
+            format!(
+                "cannot open observation authority lock {}: {why}",
+                lock_path.display()
+            )
+        })?;
         let lock = if writable {
             Flock::lock(lock, lock_path.clone()).map_err(|why| {
                 format!(
@@ -2164,17 +2166,19 @@ impl ObservationAuthorityLedgerV1 {
                 )
             })?
         };
-        let mut file = OpenOptions::new()
-            .read(true)
-            .write(writable)
-            .create(writable)
-            .open(&file_path)
-            .map_err(|why| {
-                format!(
-                    "cannot open observation authority file {}: {why}",
-                    file_path.display()
-                )
-            })?;
+        let mut file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .read(true)
+                .write(writable)
+                .create(writable),
+            &file_path,
+        )
+        .map_err(|why| {
+            format!(
+                "cannot open observation authority file {}: {why}",
+                file_path.display()
+            )
+        })?;
         if writable && file.metadata().map_err(|why| why.to_string())?.len() == 0 {
             // A short header write is truncated back to zero bytes, so the next
             // open initializes again instead of refusing a torn header (D-1854).
@@ -2723,7 +2727,7 @@ fn scan_authority_file(
 
 /// Makes newly created names in an observation root durable.
 fn sync_observation_root(root: &Path) -> Result<(), String> {
-    File::open(root)
+    crate::readonly_file::directory(root)
         .and_then(|directory| directory.sync_all())
         .map_err(|why| {
             format!(
@@ -3446,13 +3450,15 @@ impl ObservationAuthorityLedgerV2 {
         let admitted_root = admit_existing_observation_root(root)?;
         let file_path = admitted_root.join(AUTHORITY_V2_FILE);
         let lock_path = admitted_root.join(AUTHORITY_V2_LOCK_FILE);
-        let lock_file = OpenOptions::new()
-            .read(true)
-            .write(writable)
-            .create(writable)
-            .truncate(false)
-            .open(&lock_path)
-            .map_err(|why| format!("cannot open Observation V2 lock: {why}"))?;
+        let lock_file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .read(true)
+                .write(writable)
+                .create(writable)
+                .truncate(false),
+            &lock_path,
+        )
+        .map_err(|why| format!("cannot open Observation V2 lock: {why}"))?;
         let lock_file = if writable {
             Flock::lock(lock_file, lock_path.clone())
                 .map_err(|why| format!("cannot lock Observation V2 writer: {why}"))?
@@ -3460,13 +3466,15 @@ impl ObservationAuthorityLedgerV2 {
             Flock::lock_shared(lock_file, lock_path.clone())
                 .map_err(|why| format!("cannot take shared Observation V2 lock: {why}"))?
         };
-        let mut file = OpenOptions::new()
-            .read(true)
-            .write(writable)
-            .create(writable)
-            .truncate(false)
-            .open(&file_path)
-            .map_err(|why| format!("cannot open Observation V2 file: {why}"))?;
+        let mut file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .read(true)
+                .write(writable)
+                .create(writable)
+                .truncate(false),
+            &file_path,
+        )
+        .map_err(|why| format!("cannot open Observation V2 file: {why}"))?;
         if writable
             && file
                 .metadata()
diff --git a/crates/cli/src/population_statistics_v2.rs b/crates/cli/src/population_statistics_v2.rs
index 5da3e0dd..bc1caccd 100644
--- a/crates/cli/src/population_statistics_v2.rs
+++ b/crates/cli/src/population_statistics_v2.rs
@@ -112,7 +112,7 @@ const READ_CHUNK_BYTES: usize = 16 * 1_024;
 const LOCK_FILE_MAX_BYTES: u64 = 0;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(PAYLOAD_BYTES + SEAL_BYTES == RECORD_BYTES);
 
@@ -5549,7 +5549,7 @@ fn sync_parent(path: &Path) -> Result<(), PopulationStatisticsV2Refusal> {
     let parent = path
         .parent()
         .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
-    File::open(parent)
+    crate::readonly_file::directory(parent)
         .and_then(|directory| directory.sync_all())
         .map_err(|why| format!("cannot sync {}: {why}", parent.display()))
 }
@@ -5648,7 +5648,7 @@ fn open_file(
         .create(create)
         .truncate(false);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open {}: {why}", path.display()))?;
diff --git a/crates/cli/src/population_statistics_v3.rs b/crates/cli/src/population_statistics_v3.rs
index 76e99aa9..acd343a4 100644
--- a/crates/cli/src/population_statistics_v3.rs
+++ b/crates/cli/src/population_statistics_v3.rs
@@ -91,7 +91,7 @@ const READ_CHUNK_BYTES: usize = 16 * 1_024;
 const LOCK_FILE_MAX_BYTES: u64 = 0;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(PAYLOAD_BYTES + 32 == RECORD_BYTES);
 
@@ -2888,7 +2888,7 @@ fn sync_parent(path: &Path) -> Result<(), PopulationStatisticsV3Refusal> {
     let parent = path
         .parent()
         .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
-    File::open(parent)
+    crate::readonly_file::directory(parent)
         .and_then(|directory| directory.sync_all())
         .map_err(|why| format!("cannot sync {}: {why}", parent.display()))
 }
@@ -2984,7 +2984,7 @@ fn open_file(
         .create(create)
         .truncate(false);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open {}: {why}", path.display()))?;
diff --git a/crates/cli/src/population_v5.rs b/crates/cli/src/population_v5.rs
index 6ac7ff85..2a33eb80 100644
--- a/crates/cli/src/population_v5.rs
+++ b/crates/cli/src/population_v5.rs
@@ -87,7 +87,7 @@ const LOCK_FILE: &str = "population-v5.lock";
 const LOCK_MAX_BYTES: u64 = 0;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(ROW_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_V5_ROW_BYTES);
 const _: () = assert!(COMPLETION_PAYLOAD_BYTES + SEAL_BYTES == POPULATION_V5_COMPLETION_BYTES);
@@ -3524,7 +3524,7 @@ fn open_root_directory(
         )
     })?;
     require_not_symlink(&canonical, false)?;
-    let file = File::open(&canonical).map_err(|why| {
+    let file = crate::readonly_file::directory(&canonical).map_err(|why| {
         format!(
             "cannot hold Population V5 root {}: {why}",
             canonical.display()
@@ -3566,7 +3566,7 @@ fn open_child(
         let mut options = OpenOptions::new();
         options.read(true).write(true).create_new(true);
         #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-        options.custom_flags(O_NOFOLLOW_FLAG);
+        options.custom_flags(NOFOLLOW_NONBLOCK);
         match options.open(path) {
             Ok(file) => {
                 require_regular_file(&file, path)?;
@@ -3585,7 +3585,7 @@ fn open_child(
     let mut options = OpenOptions::new();
     options.read(true).write(writable).truncate(false);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open Population V5 file {}: {why}", path.display()))?;
diff --git a/crates/cli/src/population_v6.rs b/crates/cli/src/population_v6.rs
index d247a1dd..d635d33a 100644
--- a/crates/cli/src/population_v6.rs
+++ b/crates/cli/src/population_v6.rs
@@ -111,7 +111,7 @@ const GENERATION_DOMAIN: &[u8] = b"brutex-population-v6-generation\0";
 const READ_CHUNK_BYTES: usize = 16 * 1_024;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(PAYLOAD_BYTES + 32 == RECORD_BYTES);
 
@@ -2460,8 +2460,8 @@ fn open_root(root: &Path) -> Result<(PathBuf, File, PlatformIdentity), Populatio
     }
     let canonical = std::fs::canonicalize(root)
         .map_err(|why| format!("cannot canonicalize Population V6 root: {why}"))?;
-    let file =
-        File::open(&canonical).map_err(|why| format!("cannot open Population V6 root: {why}"))?;
+    let file = crate::readonly_file::directory(&canonical)
+        .map_err(|why| format!("cannot open Population V6 root: {why}"))?;
     let identity = PlatformIdentity::of(
         &file
             .metadata()
@@ -2502,7 +2502,7 @@ fn open_child(path: &Path, writable: bool) -> Result<(File, bool), PopulationV6R
     let mut options = OpenOptions::new();
     options.read(true).write(writable).create(writable);
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let existed = path.exists();
     let file = options
         .open(path)
diff --git a/crates/cli/src/readonly_file.rs b/crates/cli/src/readonly_file.rs
index fa559cee..82ca6060 100644
--- a/crates/cli/src/readonly_file.rs
+++ b/crates/cli/src/readonly_file.rs
@@ -1,5 +1,8 @@
 //! Read-only immutable-evidence opens that cannot follow a final symlink or
-//! wait for a FIFO peer, and the cli ledger door that cannot wait either.
+//! wait for a FIFO peer, and the ledger and directory doors that cannot wait
+//! either. Every `cli` and `api` open goes through one of them or carries
+//! `O_NONBLOCK` itself; `no_cli_or_api_open_can_wait_for_a_fifo_peer` walks
+//! both crates and refuses any other (G5-2, D-4732).
 //! Intermediate directories and device/filesystem I/O still require a trusted
 //! store root; this is not an `openat` path sandbox.
 
@@ -55,7 +58,15 @@ pub(crate) fn open(path: &Path) -> std::io::Result<File> {
 /// cfg'd tails, not two functions: a compiled-out body is still a body
 /// cargo-mutants mutates, and no build of this target can compile or kill the
 /// mutant (G18-cli-b-21, D-2028).
-pub(crate) fn regular(options: &mut OpenOptions, path: &Path) -> std::io::Result<File> {
+///
+/// Public since G5-2 (D-4732), so `api` opens its own journals and the ledger
+/// files it reads through this one door rather than a second copy of it.
+///
+/// # Errors
+///
+/// The host's open error, or a refusal naming `path` when the opened handle
+/// is not a regular file.
+pub fn regular(options: &mut OpenOptions, path: &Path) -> std::io::Result<File> {
     #[cfg(any(
         target_os = "macos",
         all(
@@ -93,6 +104,76 @@ pub(crate) fn regular(options: &mut OpenOptions, path: &Path) -> std::io::Result
     }
 }
 
+/// [`regular`] opened read-only: the drop-in for `File::open` on any ledger,
+/// result, journal or lock file `cli` or `api` reads (G5-2, D-4732). It keeps
+/// `File::open`'s error kinds, so a caller's `NotFound` arm is unchanged, and
+/// it can never wait for a FIFO or socket peer.
+///
+/// # Errors
+///
+/// The host's open error, or a refusal naming `path` when the opened handle is
+/// not a regular file.
+pub fn read(path: impl AsRef<Path>) -> std::io::Result<File> {
+    regular(OpenOptions::new().read(true), path.as_ref())
+}
+
+/// A directory handle that cannot wait for a FIFO or socket peer: the target of
+/// a durability barrier's `fsync`, or a root a ledger holds open (G5-2,
+/// D-4732). `File::open` on a directory path set no `O_NONBLOCK`, so a FIFO
+/// planted there held the opener forever; this opens read-only with
+/// `O_NONBLOCK` and refuses any handle whose `fstat` is not a directory,
+/// naming the path. A final symlink is followed, as `File::open` followed it,
+/// and an absent path keeps its `NotFound` kind.
+///
+/// On any other target it refuses with `Unsupported`, for the reason
+/// [`regular`] gives.
+///
+/// # Errors
+///
+/// The host's open error, or `NotADirectory` naming `path`.
+pub fn directory(path: impl AsRef<Path>) -> std::io::Result<File> {
+    let path = path.as_ref();
+    #[cfg(any(
+        target_os = "macos",
+        all(
+            target_os = "linux",
+            any(target_arch = "x86_64", target_arch = "aarch64")
+        )
+    ))]
+    {
+        use std::os::unix::fs::OpenOptionsExt as _;
+        let file = OpenOptions::new()
+            .read(true)
+            .custom_flags(store::open_flags::O_NONBLOCK)
+            .open(path)?;
+        if file.metadata()?.is_dir() {
+            Ok(file)
+        } else {
+            Err(std::io::Error::new(
+                std::io::ErrorKind::NotADirectory,
+                format!(
+                    "{} is not a directory; a durability barrier or a held root is never anything else",
+                    path.display()
+                ),
+            ))
+        }
+    }
+    #[cfg(not(any(
+        target_os = "macos",
+        all(
+            target_os = "linux",
+            any(target_arch = "x86_64", target_arch = "aarch64")
+        )
+    )))]
+    {
+        let _ = path;
+        Err(std::io::Error::new(
+            std::io::ErrorKind::Unsupported,
+            "directory opens require verified macOS or Linux x86_64/aarch64 flags",
+        ))
+    }
+}
+
 #[cfg(not(any(
     target_os = "macos",
     all(
@@ -115,7 +196,7 @@ pub(crate) fn open(_path: &Path) -> std::io::Result<File> {
         any(target_arch = "x86_64", target_arch = "aarch64")
     )
 ))]
-mod tests {
+pub(crate) mod tests {
     use super::*;
 
     #[test]
@@ -311,6 +392,102 @@ mod tests {
                 assert!(not_regular(&why), "{fifo_at}: {why}");
             }
         }
+
+        // G5-2 (D-4732): the HTTP readers D-1743's heading claimed and its
+        // body never reached -- `/top.json` and `/frontier.json` through
+        // `Frontier`, `/trades.json` through `Trades`.
+        let frontier = crate::frontier::Frontier::path(&root);
+        mkfifo(&frontier)?;
+        for bounded in [false, true] {
+            let at = root.clone();
+            let why = refuses_promptly(&frontier, move || {
+                if bounded {
+                    crate::frontier::Frontier::open_read_bounded(&at, 1 << 20).err()
+                } else {
+                    crate::frontier::Frontier::open_read(&at).err()
+                }
+            })?;
+            assert!(not_regular(&why), "the frontier reader: {why}");
+        }
+        std::fs::remove_file(&frontier)?;
+        let trades = crate::trades::Trades::path(&root);
+        mkfifo(&trades)?;
+        for bounded in [false, true] {
+            let at = root.clone();
+            let why = refuses_promptly(&trades, move || {
+                if bounded {
+                    crate::trades::Trades::open_read_bounded(&at, 1 << 20).err()
+                } else {
+                    crate::trades::Trades::open_read(&at).err()
+                }
+            })?;
+            assert!(not_regular(&why), "the trades reader: {why}");
+        }
+        std::fs::remove_file(&trades)?;
+
+        // `/sweep-evidence.json` and `/candidate.json`: every sweep-evidence
+        // file a reader opens -- the global journal, the identity's start
+        // index, one attempt's lifecycle and reservation, and a child detail
+        // file both counted and paged.
+        sweep_evidence_fifos_refuse(&not_regular)?;
+        Ok(())
+    }
+
+    /// Each sweep-evidence reader door, a FIFO planted in place of the file it
+    /// opens. The attempt is finished before any FIFO exists, so no writer of
+    /// this test can reach one.
+    fn sweep_evidence_fifos_refuse(
+        not_regular: &dyn Fn(&str) -> bool,
+    ) -> Result<(), Box<dyn std::error::Error>> {
+        use crate::sweep_evidence::{self as evidence, Completion, Operation};
+        const BOUND: u64 = 1 << 20;
+        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
+        let root = scratch.0.clone();
+        let identity = [0x5a_u8; 32];
+        let attempt = evidence::begin(&root, identity, Operation::Sweep)?;
+        let token = attempt.token();
+        attempt.finish(Completion::Completed)?;
+        let read = evidence::read(&root, identity, BOUND)?.ok_or("the attempt reads back")?;
+        let base = root.join("results").join("sweep-evidence-v1");
+        let own = base.join("5a".repeat(32));
+
+        // A child detail file the evidence declares empty: counted by every
+        // read, opened by every page.
+        let levels = own.join(format!("{token}-levels.bin"));
+        mkfifo(&levels)?;
+        let at = root.clone();
+        let why = refuses_promptly(&levels, move || evidence::read(&at, identity, BOUND).err())?;
+        assert!(not_regular(&why), "the counted child: {why}");
+        let at = root.clone();
+        let why = refuses_promptly(&levels, move || {
+            evidence::depth_page(&at, &read, 0, 1, BOUND).err()
+        })?;
+        assert!(not_regular(&why), "the paged child: {why}");
+        std::fs::remove_file(&levels)?;
+
+        for (file, door) in [
+            (base.join("attempts.bin"), 0),
+            (own.join("starts.bin"), 1),
+            (own.join(format!("{token}-lifecycle.bin")), 2),
+            (base.join(format!("{token}-start.bin")), 1),
+        ] {
+            let aside = file.with_extension("aside");
+            std::fs::rename(&file, &aside)?;
+            mkfifo(&file)?;
+            let at = root.clone();
+            let why = refuses_promptly(&file, move || match door {
+                0 => evidence::latest(&at, BOUND).err(),
+                1 => evidence::read(&at, identity, BOUND).err(),
+                _ => evidence::read_attempt(&at, identity, token, BOUND).err(),
+            })?;
+            assert!(not_regular(&why), "{}: {why}", file.display());
+            std::fs::remove_file(&file)?;
+            std::fs::rename(&aside, &file)?;
+        }
+        assert!(
+            evidence::read(&root, identity, BOUND)?.is_some(),
+            "every displaced file was restored and reads again"
+        );
         Ok(())
     }
     /// The ledger door: a socket and a directory refuse by name, a symlink to
@@ -360,4 +537,283 @@ mod tests {
         );
         Ok(())
     }
+
+    /// The premise the scan below takes for a read-write `OpenOptions`: with
+    /// no `O_NONBLOCK`, `open(2)` admits a FIFO for `O_RDWR` at once (fifo(7)),
+    /// where `O_RDONLY` waits for a writer. So a read-write open needs no door
+    /// to be bounded, and the serve lock's keeps reaching a device that refuses
+    /// its stamp (G5-2, D-4732).
+    #[test]
+    fn a_read_write_open_of_a_fifo_never_waits() -> Result<(), Box<dyn std::error::Error>> {
+        let scratch = crate::search_checkpoint::tests::Scratch::new()?;
+        let fifo = scratch.0.join("fifo");
+        mkfifo(&fifo)?;
+        let path = fifo.clone();
+        let opened = refuses_promptly(&fifo, move || {
+            std::fs::OpenOptions::new()
+                .read(true)
+                .write(true)
+                .open(&path)
+                .ok()
+                .map(|file| format!("opened {}", file.metadata().is_ok()))
+        })?;
+        assert_eq!(opened, "opened true");
+        let path = fifo.clone();
+        let waited = refuses_promptly(&fifo, move || {
+            std::fs::File::open(&path).ok().map(|_| "opened".to_owned())
+        })
+        .err()
+        .ok_or("premise: a read-only open of a FIFO waits for a writer")?;
+        assert!(
+            waited.to_string().contains("waited for a FIFO peer"),
+            "{waited}"
+        );
+        Ok(())
+    }
+
+    /// `source` with every `#[cfg(test)]` item removed, so a scan reads only
+    /// what a release build compiles, each removed line left empty so line numbers
+    /// still name the file's own lines, and the file names of the test-only
+    /// `mod NAME;` declarations it removed (`#[path]` honoured). The item an
+    /// attribute run covers ends at its own `;`, or at the first later line
+    /// that closes its brace at the same indentation, which `cargo fmt
+    /// --check` makes exact.
+    fn split_release(source: &str) -> (String, Vec<(String, bool)>) {
+        let lines: Vec<&str> = source.lines().collect();
+        let mut kept = String::with_capacity(source.len());
+        let mut gated = Vec::new();
+        let mut at = 0_usize;
+        while let Some(line) = lines.get(at) {
+            let trimmed = line.trim_start();
+            if !(trimmed.starts_with("#[cfg(test") || trimmed.starts_with("#[cfg(all(test")) {
+                kept.push_str(line);
+                kept.push('\n');
+                at += 1;
+                continue;
+            }
+            // Further attributes and comments between the `cfg` and its item.
+            let mut item = at + 1;
+            let mut path_attribute = None;
+            while let Some(next) = lines.get(item) {
+                let next = next.trim_start();
+                if next.starts_with("#[") {
+                    if let Some(named) = next.strip_prefix("#[path = \"") {
+                        path_attribute = named.split('"').next().map(str::to_owned);
+                    }
+                    while lines
+                        .get(item)
+                        .is_some_and(|l| !l.trim_end().ends_with(']'))
+                    {
+                        item += 1;
+                    }
+                    item += 1;
+                } else if next.starts_with("//") {
+                    item += 1;
+                } else {
+                    break;
+                }
+            }
+            let start = at;
+            let Some(head) = lines.get(item) else { break };
+            at = item + 1;
+            let declared = head
+                .trim()
+                .trim_start_matches("pub(crate) ")
+                .trim_start_matches("pub(super) ")
+                .trim_start_matches("pub ")
+                .strip_prefix("mod ")
+                .and_then(|rest| rest.strip_suffix(';'));
+            if let Some(name) = declared {
+                gated.push(match path_attribute {
+                    Some(named) => (named, true),
+                    None => (format!("{name}.rs"), false),
+                });
+            }
+            if head.trim_end().ends_with('{') {
+                let indent = &head[..head.len() - head.trim_start().len()];
+                let close = format!("{indent}}}");
+                while let Some(body) = lines.get(at) {
+                    at += 1;
+                    let body = body.trim_end();
+                    if body == close || body == format!("{close};") {
+                        break;
+                    }
+                }
+            }
+            // One empty line per removed line, so a line number in the release
+            // text is the same line in the file.
+            for _ in start..at {
+                kept.push('\n');
+            }
+        }
+        (kept, gated)
+    }
+
+    /// Every `.rs` file under `dir`, recursively, in path order.
+    fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
+        let Ok(entries) = std::fs::read_dir(dir) else {
+            return;
+        };
+        let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
+        paths.sort();
+        for path in paths {
+            if path.is_dir() {
+                rust_files(&path, out);
+            } else if path.extension().is_some_and(|e| e == "rs") {
+                out.push(path);
+            }
+        }
+    }
+
+    /// Every `.rs` file under `dir` that a release build compiles, with its
+    /// `#[cfg(test)]` items removed: not a `*_tests.rs` or `tests.rs` file,
+    /// not one opening `#![cfg(test)]`, not one a parent declares only under
+    /// `#[cfg(test)]`, and nothing beneath such a file's own module directory.
+    pub(crate) fn release_sources(dir: &Path, out: &mut Vec<(std::path::PathBuf, String)>) {
+        let mut files = Vec::new();
+        rust_files(dir, &mut files);
+        let mut test_only: Vec<std::path::PathBuf> = Vec::new();
+        let mut kept = Vec::new();
+        for path in files {
+            let Ok(text) = std::fs::read_to_string(&path) else {
+                continue;
+            };
+            let (release, gated) = split_release(&text);
+            let parent = path.parent().unwrap_or(dir);
+            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
+            let children = if matches!(name, "lib.rs" | "main.rs" | "mod.rs") {
+                parent.to_path_buf()
+            } else {
+                path.with_extension("")
+            };
+            for (module, attributed) in gated {
+                if attributed {
+                    // `#[path]` is relative to the declaring file's directory.
+                    test_only.push(parent.join(module));
+                } else {
+                    test_only.push(children.join(&module));
+                    test_only.push(children.join(module.trim_end_matches(".rs")).join("mod.rs"));
+                }
+            }
+            let whole_file_test = name.ends_with("_tests.rs")
+                || name == "tests.rs"
+                || text.lines().take(40).any(|l| l.trim() == "#![cfg(test)]");
+            if !whole_file_test {
+                kept.push((path, release));
+            }
+        }
+        for (path, release) in kept {
+            let gated = test_only.iter().any(|test| {
+                *test == path
+                    || path.starts_with(test.with_extension(""))
+                    || (test.ends_with("mod.rs")
+                        && test.parent().is_some_and(|module| path.starts_with(module)))
+            });
+            if !gated {
+                out.push((path, release));
+            }
+        }
+    }
+
+    /// Whether the `OpenOptions` built at `at` cannot wait in `open(2)`: handed to
+    /// `regular`, opened read-write (`.read(true)` with `.write(true)` or
+    /// `.append(true)`, which `a_read_write_open_of_a_fifo_never_waits` shows a
+    /// FIFO admits at once), or given `custom_flags` naming a non-blocking flag
+    /// -- directly, or through a `const` of this file whose value names one --
+    /// before the first `.open(` after it.
+    fn waits_for_nothing(text: &str, at: usize) -> bool {
+        let before = text
+            .get(..at)
+            .unwrap_or("")
+            .trim_end_matches(|c: char| c.is_alphanumeric() || c == '_' || c == ':')
+            .trim_end();
+        if before.ends_with("regular(") {
+            return true;
+        }
+        let rest = text.get(at..).unwrap_or("");
+        let end = rest
+            .find(".open(")
+            .map_or(rest.len(), |open| open + ".open(".len());
+        let window = rest.get(..end.min(2_000)).unwrap_or(rest);
+        if window.contains("readonly_file::regular(") {
+            return true;
+        }
+        if window.contains(".read(true)")
+            && (window.contains(".write(true)") || window.contains(".append(true)"))
+        {
+            return true;
+        }
+        let Some(flags) = window.find("custom_flags(") else {
+            return false;
+        };
+        if window.contains("NONBLOCK") {
+            return true;
+        }
+        let argument = window
+            .get(flags + "custom_flags(".len()..)
+            .and_then(|tail| tail.split(')').next())
+            .unwrap_or("")
+            .trim();
+        text.split(&format!("const {argument}: i32 ="))
+            .nth(1)
+            .and_then(|value| value.split(';').next())
+            .is_some_and(|value| value.contains("NONBLOCK"))
+    }
+
+    /// G5-2 (D-4732): no read, write or directory open a `cli` or `api`
+    /// release build compiles can wait on a FIFO or socket peer. `File::open`
+    /// and `File::create` set no `O_NONBLOCK` and are refused outright; every
+    /// `OpenOptions` must reach `readonly_file::regular` or carry a
+    /// non-blocking custom flag. D-1743 fixed five doors under a heading that
+    /// claimed every ledger, and six HTTP readers kept the blocking open; this
+    /// walks every source file, so a new door cannot slip past by not being
+    /// listed.
+    #[test]
+    fn no_cli_or_api_open_can_wait_for_a_fifo_peer() {
+        let cli = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
+        let api = Path::new(env!("CARGO_MANIFEST_DIR")).join("../api/src");
+        let mut sources = Vec::new();
+        release_sources(&cli, &mut sources);
+        let cli_files = sources.len();
+        release_sources(&api, &mut sources);
+        assert!(
+            cli_files > 100 && sources.len() > cli_files + 40,
+            "premise: both crates were walked ({cli_files} cli, {} total)",
+            sources.len()
+        );
+        let lib = sources
+            .iter()
+            .find(|(path, _)| path.ends_with("cli/src/lib.rs"))
+            .map(|(_, text)| text.lines().filter(|line| !line.trim().is_empty()).count());
+        assert!(
+            lib.is_some_and(|lines| lines > 10_000),
+            "premise: lib.rs is read past its first test module ({lib:?} lines)"
+        );
+        let mut waiting = Vec::new();
+        for (path, text) in &sources {
+            for pattern in ["File::open(", "File::create("] {
+                for (at, _) in text.match_indices(pattern) {
+                    let named = text
+                        .get(..at)
+                        .and_then(|head| head.chars().next_back())
+                        .is_some_and(|c| c.is_alphanumeric() || c == '_');
+                    if !named {
+                        waiting.push(format!("{}: {pattern}", path.display()));
+                    }
+                }
+            }
+            for (at, _) in text.match_indices("OpenOptions::new()") {
+                if !waits_for_nothing(text, at) {
+                    let line = text.get(..at).map_or(0, |head| head.lines().count());
+                    waiting.push(format!("{}:{line}: OpenOptions", path.display()));
+                }
+            }
+        }
+        assert!(
+            waiting.is_empty(),
+            "{} open(s) can wait for a FIFO peer:\n{}",
+            waiting.len(),
+            waiting.join("\n")
+        );
+    }
 }
diff --git a/crates/cli/src/search_checkpoint.rs b/crates/cli/src/search_checkpoint.rs
index 5f5b4caf..68eb75c4 100644
--- a/crates/cli/src/search_checkpoint.rs
+++ b/crates/cli/src/search_checkpoint.rs
@@ -170,7 +170,7 @@ impl Journal {
             Flock::try_lock(open_owner(&owner_path)?, owner_path.clone()).map_err(|why| {
                 format!("this exact search is already owned or cannot be locked: {why}")
             })?;
-        File::open(&directory)
+        crate::readonly_file::directory(&directory)
             .map_err(error)?
             .sync_all()
             .map_err(error)?;
@@ -254,17 +254,16 @@ impl Journal {
         let directory = self.directory.join(format!("{sequence:016x}"));
         fs::create_dir(&directory).map_err(error)?;
         self.entries = entries;
-        File::open(&self.directory)
+        crate::readonly_file::directory(&self.directory)
             .map_err(error)?
             .sync_all()
             .map_err(error)?;
         let path = directory.join("payload");
-        let mut raw = OpenOptions::new()
-            .read(true)
-            .write(true)
-            .create_new(true)
-            .open(&path)
-            .map_err(error)?;
+        let mut raw = crate::readonly_file::regular(
+            OpenOptions::new().read(true).write(true).create_new(true),
+            &path,
+        )
+        .map_err(error)?;
         let mut file = Flock::lock(&mut raw, path.as_path()).map_err(error)?;
         #[cfg(test)]
         tests::payload_locked(&file);
@@ -279,7 +278,7 @@ impl Journal {
             .and_then(|()| file.sync_all())
             .map_err(error)?;
         verify_acknowledged(&mut file, &path, &header, payload, seal)?;
-        File::open(&directory)
+        crate::readonly_file::directory(&directory)
             .map_err(error)?
             .sync_all()
             .map_err(error)?;
@@ -337,7 +336,7 @@ fn publish_marker(directory: &Path, seal: [u8; 32]) -> Result<(), String> {
             ),
         });
     }
-    File::open(directory)
+    crate::readonly_file::directory(directory)
         .map_err(error)?
         .sync_all()
         .map_err(error)
@@ -385,7 +384,7 @@ fn open_owner(path: &Path) -> Result<File, String> {
     #[cfg(unix)]
     {
         use std::os::unix::fs::OpenOptionsExt as _;
-        options.custom_flags(store::open_flags::O_NOFOLLOW);
+        options.custom_flags(store::open_flags::O_NOFOLLOW_NONBLOCK);
     }
     // create_new never follows an existing symlink, including a dangling one.
     // The existing-file door deliberately has no create flag, so refusal can
@@ -430,7 +429,10 @@ fn regular_bytes(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
 
 fn durable_directory(parent: &Path, path: &Path) -> Result<(), String> {
     match fs::create_dir(path) {
-        Ok(()) => File::open(parent).map_err(error)?.sync_all().map_err(error),
+        Ok(()) => crate::readonly_file::directory(parent)
+            .map_err(error)?
+            .sync_all()
+            .map_err(error),
         Err(why)
             if why.kind() == std::io::ErrorKind::AlreadyExists
                 && fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir()) =>
diff --git a/crates/cli/src/selection.rs b/crates/cli/src/selection.rs
index 13bd7c71..52d5aff6 100644
--- a/crates/cli/src/selection.rs
+++ b/crates/cli/src/selection.rs
@@ -1779,13 +1779,15 @@ impl SelectionLedger {
                 parent.display()
             )
         })?;
-        let file = OpenOptions::new()
-            .create(true)
-            .read(true)
-            .write(true)
-            .truncate(false)
-            .open(&path)
-            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
+        let file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .create(true)
+                .read(true)
+                .write(true)
+                .truncate(false),
+            &path,
+        )
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
         Self::open_file(file, path, true, max_receipts)
     }
 
@@ -1799,9 +1801,7 @@ impl SelectionLedger {
     pub fn open_read(root: &Path, max_receipts: usize) -> Result<Self, SelectionRefusal> {
         validate_receipt_limit(max_receipts)?;
         let path = Self::path(root);
-        let file = OpenOptions::new()
-            .read(true)
-            .open(&path)
+        let file = crate::readonly_file::regular(OpenOptions::new().read(true), &path)
             .map_err(|why| format!("{} could not be opened read-only: {why}", path.display()))?;
         Self::open_file(file, path, false, max_receipts)
     }
@@ -2123,13 +2123,15 @@ impl SelectionLedgerV2 {
                 parent.display()
             )
         })?;
-        let file = OpenOptions::new()
-            .create(true)
-            .read(true)
-            .write(true)
-            .truncate(false)
-            .open(&path)
-            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
+        let file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .create(true)
+                .read(true)
+                .write(true)
+                .truncate(false),
+            &path,
+        )
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
         Self::open_file(file, path, true, max_receipts)
     }
 
@@ -2141,9 +2143,7 @@ impl SelectionLedgerV2 {
     pub fn open_read(root: &Path, max_receipts: usize) -> Result<Self, SelectionRefusal> {
         validate_receipt_limit(max_receipts)?;
         let path = Self::path(root);
-        let file = OpenOptions::new()
-            .read(true)
-            .open(&path)
+        let file = crate::readonly_file::regular(OpenOptions::new().read(true), &path)
             .map_err(|why| format!("{} could not be opened read-only: {why}", path.display()))?;
         Self::open_file(file, path, false, max_receipts)
     }
diff --git a/crates/cli/src/selection_v3.rs b/crates/cli/src/selection_v3.rs
index c811c0de..f7bf34d3 100644
--- a/crates/cli/src/selection_v3.rs
+++ b/crates/cli/src/selection_v3.rs
@@ -1126,13 +1126,15 @@ impl SelectionLedgerV3 {
                 parent.display()
             )
         })?;
-        let file = OpenOptions::new()
-            .create(true)
-            .read(true)
-            .write(true)
-            .truncate(false)
-            .open(&path)
-            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
+        let file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .create(true)
+                .read(true)
+                .write(true)
+                .truncate(false),
+            &path,
+        )
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
         Self::open_file(file, path, true, max_receipts)
     }
 
@@ -1145,9 +1147,7 @@ impl SelectionLedgerV3 {
     pub fn open_read(root: &Path, max_receipts: usize) -> Result<Self, SelectionV3Refusal> {
         validate_receipt_limit(max_receipts)?;
         let path = Self::path(root);
-        let file = OpenOptions::new()
-            .read(true)
-            .open(&path)
+        let file = crate::readonly_file::regular(OpenOptions::new().read(true), &path)
             .map_err(|why| format!("{} could not be opened read-only: {why}", path.display()))?;
         Self::open_file(file, path, false, max_receipts)
     }
diff --git a/crates/cli/src/selection_v4.rs b/crates/cli/src/selection_v4.rs
index dcd0f054..a6f26110 100644
--- a/crates/cli/src/selection_v4.rs
+++ b/crates/cli/src/selection_v4.rs
@@ -1699,13 +1699,15 @@ impl SelectionLedgerV4 {
         validate_receipt_limit(max_receipts)?;
         ensure_results_directory(root)?;
         let path = Self::path(root);
-        let file = OpenOptions::new()
-            .create(true)
-            .read(true)
-            .write(true)
-            .truncate(false)
-            .open(&path)
-            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
+        let file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .create(true)
+                .read(true)
+                .write(true)
+                .truncate(false),
+            &path,
+        )
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
         Self::open_file(file, path, true, max_receipts)
     }
 
@@ -1720,9 +1722,7 @@ impl SelectionLedgerV4 {
     pub fn open_read(root: &Path, max_receipts: usize) -> Result<Self, SelectionV4Refusal> {
         validate_receipt_limit(max_receipts)?;
         let path = Self::path(root);
-        let file = OpenOptions::new()
-            .read(true)
-            .open(&path)
+        let file = crate::readonly_file::regular(OpenOptions::new().read(true), &path)
             .map_err(|why| format!("{} could not be opened read-only: {why}", path.display()))?;
         Self::open_file(file, path, false, max_receipts)
     }
diff --git a/crates/cli/src/selection_v4_authority.rs b/crates/cli/src/selection_v4_authority.rs
index 26d21542..5ebd57a4 100644
--- a/crates/cli/src/selection_v4_authority.rs
+++ b/crates/cli/src/selection_v4_authority.rs
@@ -87,7 +87,7 @@ impl SelectionAuthorityLedgerV4 {
             );
         }
         let lock_path = root.join("results").join("population-write.lock");
-        let outer_lock = File::open(&lock_path)
+        let outer_lock = crate::readonly_file::read(&lock_path)
             .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
         outer_lock.lock_shared().map_err(|why| {
             format!(
@@ -583,7 +583,7 @@ fn release_outer<T>(
 }
 
 fn independent_lock_handle(path: &Path) -> Result<File, SelectionV4Refusal> {
-    File::open(path).map_err(|why| {
+    crate::readonly_file::read(path).map_err(|why| {
         format!(
             "{} could not be independently opened for a Selection V4 view: {why}",
             path.display()
diff --git a/crates/cli/src/selection_v5.rs b/crates/cli/src/selection_v5.rs
index cf024790..f8ca1d06 100644
--- a/crates/cli/src/selection_v5.rs
+++ b/crates/cli/src/selection_v5.rs
@@ -100,7 +100,7 @@ const LOCK_FILE: &str = "global-selection-v5.lock";
 const LOCK_MAX_BYTES: u64 = 0;
 
 #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-const O_NOFOLLOW_FLAG: i32 = store::open_flags::O_NOFOLLOW;
+const NOFOLLOW_NONBLOCK: i32 = store::open_flags::O_NOFOLLOW_NONBLOCK;
 
 const _: () = assert!(MAX_TOP == REQUESTED_TOP);
 const _: () = assert!(ROW_PAYLOAD_BYTES + SEAL_BYTES == SELECTION_V5_ROW_BYTES);
@@ -2907,7 +2907,7 @@ fn open_root_directory(
             root.display()
         )
     })?;
-    let file = File::open(&canonical).map_err(|why| {
+    let file = crate::readonly_file::directory(&canonical).map_err(|why| {
         format!(
             "cannot hold Selection V5 root {}: {why}",
             canonical.display()
@@ -2941,7 +2941,7 @@ fn open_child(path: &Path, writable: bool, created: &mut bool) -> Result<File, S
         options.write(true).create(true).truncate(false);
     }
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
-    options.custom_flags(O_NOFOLLOW_FLAG);
+    options.custom_flags(NOFOLLOW_NONBLOCK);
     let file = options
         .open(path)
         .map_err(|why| format!("cannot open Selection V5 child {}: {why}", path.display()))?;
diff --git a/crates/cli/src/selection_v6.rs b/crates/cli/src/selection_v6.rs
index d7341f32..dc58999f 100644
--- a/crates/cli/src/selection_v6.rs
+++ b/crates/cli/src/selection_v6.rs
@@ -234,7 +234,7 @@ fn open(root: &Path, writable: bool) -> Result<(File, PathBuf), String> {
     #[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
     {
         use std::os::unix::fs::OpenOptionsExt as _;
-        options.custom_flags(store::open_flags::O_NOFOLLOW);
+        options.custom_flags(store::open_flags::O_NOFOLLOW_NONBLOCK);
     }
     let file = options
         .open(&path)
@@ -428,7 +428,7 @@ fn set_aside_abandoned_tail(
         .map_err(|why| why.to_string())?;
     file.read_exact(&mut tail).map_err(|why| why.to_string())?;
     let aside = root.join(format!("{FILE_NAME}.abandoned-{committed}"));
-    match OpenOptions::new().write(true).create_new(true).open(&aside) {
+    match crate::readonly_file::regular(OpenOptions::new().write(true).create_new(true), &aside) {
         Ok(mut out) => {
             out.write_all(&tail)
                 .and_then(|()| out.sync_all())
@@ -458,7 +458,7 @@ fn set_aside_abandoned_tail(
 }
 
 fn sync_directory(root: &Path) -> Result<(), String> {
-    File::open(root)
+    crate::readonly_file::directory(root)
         .and_then(|directory| directory.sync_all())
         .map_err(|why| why.to_string())
 }
diff --git a/crates/cli/src/step3_orchestrator.rs b/crates/cli/src/step3_orchestrator.rs
index 4fb6d042..2743471b 100644
--- a/crates/cli/src/step3_orchestrator.rs
+++ b/crates/cli/src/step3_orchestrator.rs
@@ -393,7 +393,7 @@ impl AdmittedRootV1 {
             ));
         }
         let generation = directory_identity_v1(&path_metadata)?;
-        let directory = File::open(&canonical).map_err(|why| {
+        let directory = crate::readonly_file::directory(&canonical).map_err(|why| {
             format!(
                 "Step 3 root admission could not hold directory capability {}: {why}",
                 canonical.display()
diff --git a/crates/cli/src/stored_data_completeness.rs b/crates/cli/src/stored_data_completeness.rs
index 3b454ef8..75244d4f 100644
--- a/crates/cli/src/stored_data_completeness.rs
+++ b/crates/cli/src/stored_data_completeness.rs
@@ -721,13 +721,15 @@ impl StoredDataCompletenessLedgerV1 {
                 parent.display()
             )
         })?;
-        let file = OpenOptions::new()
-            .create(true)
-            .read(true)
-            .write(true)
-            .truncate(false)
-            .open(&path)
-            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
+        let file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .create(true)
+                .read(true)
+                .write(true)
+                .truncate(false),
+            &path,
+        )
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
         Self::open_file(file, path, max_receipts, true)
     }
 
@@ -742,7 +744,7 @@ impl StoredDataCompletenessLedgerV1 {
     ) -> Result<Self, StoredDataCompletenessRefusal> {
         validate_max(max_receipts)?;
         let path = Self::path(root);
-        let file = File::open(&path)
+        let file = crate::readonly_file::read(&path)
             .map_err(|why| format!("{} could not be opened read-only: {why}", path.display()))?;
         Self::open_file(file, path, max_receipts, false)
     }
diff --git a/crates/cli/src/sweep_evidence.rs b/crates/cli/src/sweep_evidence.rs
index e13e388e..1ffddd4b 100644
--- a/crates/cli/src/sweep_evidence.rs
+++ b/crates/cli/src/sweep_evidence.rs
@@ -899,7 +899,7 @@ fn verify_terminal_detail<const N: usize>(
     expected: u64,
     expected_digest: [u8; 32],
 ) -> Result<(), String> {
-    let mut file = File::open(path).map_err(io_error)?;
+    let mut file = crate::readonly_file::read(path).map_err(io_error)?;
     file.lock_shared().map_err(io_error)?;
     let result = (|| {
         let before = crate::result_set::file_generation(&file, path)?;
@@ -1097,18 +1097,17 @@ fn append_identity_start(path: &Path, evidence: Evidence) -> Result<(), String>
 /// before any lifecycle row can refer to the token.
 fn reserve_start(root: &Path, evidence: Evidence) -> Result<(), String> {
     let path = start_path(root, evidence.attempt);
-    let mut file = OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create_new(true)
-        .open(&path)
-        .map_err(|why| {
-            forget_flushed();
-            format!(
-                "sweep attempt reservation refused at {}: {why}; existing history was not replaced",
-                path.display()
-            )
-        })?;
+    let mut file = crate::readonly_file::regular(
+        OpenOptions::new().read(true).write(true).create_new(true),
+        &path,
+    )
+    .map_err(|why| {
+        forget_flushed();
+        format!(
+            "sweep attempt reservation refused at {}: {why}; existing history was not replaced",
+            path.display()
+        )
+    })?;
     let mut raw = [0_u8; 16 + EVENT_BYTES];
     raw[..16].copy_from_slice(&EVENT_HEADER);
     raw[16..].copy_from_slice(&event_bytes(evidence)?);
@@ -1191,7 +1190,7 @@ fn durable_directory(path: &Path) -> Result<(), String> {
     Ok(())
 }
 fn flush_directory(path: &Path) -> Result<(), String> {
-    File::open(path)
+    crate::readonly_file::directory(path)
         .and_then(|directory| barrier(&directory, path))
         .map_err(io_error)
 }
@@ -1206,13 +1205,15 @@ fn forget_flushed() {
     remembered.clear();
 }
 fn open_append(path: &Path) -> Result<File, String> {
-    OpenOptions::new()
-        .read(true)
-        .write(true)
-        .create(true)
-        .truncate(false)
-        .open(path)
-        .map_err(io_error)
+    crate::readonly_file::regular(
+        OpenOptions::new()
+            .read(true)
+            .write(true)
+            .create(true)
+            .truncate(false),
+        path,
+    )
+    .map_err(io_error)
 }
 fn emit(e: Evidence) {
     crate::note(
@@ -1513,7 +1514,7 @@ fn allocate(root: &Path, starts: &mut [Evidence]) -> Result<(), String> {
     .map(|_| ())
 }
 fn last_event(path: &Path, max_bytes: u64) -> Result<Option<Evidence>, String> {
-    let mut file = match File::open(path) {
+    let mut file = match crate::readonly_file::read(path) {
         Ok(file) => file,
         Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
         Err(why) => return Err(io_error(why)),
@@ -1551,7 +1552,7 @@ fn count_optional<const N: usize>(
     magic: [u8; 8],
     max_bytes: u64,
 ) -> Result<u64, String> {
-    let mut file = match File::open(path) {
+    let mut file = match crate::readonly_file::read(path) {
         Ok(file) => file,
         Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(0),
         Err(why) => return Err(io_error(why)),
@@ -1586,7 +1587,7 @@ fn page<const N: usize>(
         e.ranked_rows
     };
     let path = detail_path(root, e, kind);
-    let file = match File::open(&path) {
+    let file = match crate::readonly_file::read(&path) {
         Ok(file) => file,
         Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
             return if expected > 0 || (kind == "ranked" && e.ranked_available) {
diff --git a/crates/cli/src/trades.rs b/crates/cli/src/trades.rs
index 1c02e987..a8ece278 100644
--- a/crates/cli/src/trades.rs
+++ b/crates/cli/src/trades.rs
@@ -426,7 +426,7 @@ impl Trades {
     /// length when it exceeds `max_bytes`. No row is indexed in that case.
     pub fn open_read_bounded(root: &Path, max_bytes: u64) -> Result<Self, Refusal> {
         let path = Self::path(root);
-        let mut file = match OpenOptions::new().read(true).open(&path) {
+        let mut file = match crate::readonly_file::read(&path) {
             Ok(file) => file,
             Err(why)
                 if why.kind() == std::io::ErrorKind::NotFound
@@ -482,13 +482,15 @@ impl Trades {
         std::fs::create_dir_all(&dir)
             .map_err(|why| format!("the results directory could not be made: {why}"))?;
         let path = Self::path(root);
-        let file = OpenOptions::new()
-            .read(true)
-            .write(true)
-            .create(true)
-            .truncate(false)
-            .open(&path)
-            .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
+        let file = crate::readonly_file::regular(
+            OpenOptions::new()
+                .read(true)
+                .write(true)
+                .create(true)
+                .truncate(false),
+            &path,
+        )
+        .map_err(|why| format!("{} could not be opened: {why}", path.display()))?;
         // THE WRITER'S OPEN HOLDS THE EXCLUSIVE LOCK while it measures, cuts
         // a torn tail and indexes (D-1901, sweep-2): an unlocked length could
         // land inside another writer's live append.
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 31e467ea..91017ca5 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7052,3 +7052,7 @@ old line regex the same input and watched it pass.
 | G18-api-27 | The seek path's `records unreadable` line names the first file that refused a record, not the first file read (D-2046) | `api::bars::window_tests::the_unreadable_line_names_the_first_damaged_file_not_the_first_file` | ✓ |
 | G18-api-28 | The route test's HTTP exchange is bounded at 30 s per read and write, so a server that admits or answers nothing fails it rather than hanging (D-2047) | `api::ingest::route_tests::the_three_routes_answer_and_none_of_them_shadows_the_front_end` | ✓ |
 | G18-api-29 | A dropped calendar `Landing` marks its flight `Abandoned` (or answered), removes it from the flight table, and wakes every follower (D-2047) | `api::calendar_of::tests::a_calendar_landing_releases_its_flight_and_wakes_its_followers_when_dropped` | ✓ |
+| L1FC-01 | A FIFO at the frontier, trades, or any of five `sweep_evidence` ledger paths, among every other `cli` ledger door, is refused within the test's bounded wait instead of waiting in `open(2)` for a peer (G5-2, D-4732) | `cli::readonly_file::tests::a_fifo_at_any_cli_ledger_path_refuses_without_waiting` | ✓ |
+| L1FC-02 | No `cli` or `api` release source calls an unqualified `File::open` or `File::create`, or builds an `OpenOptions` that does not reach `readonly_file::regular`, open read-write, or carry a `NONBLOCK` custom flag (G5-2, D-4732) | `cli::readonly_file::tests::no_cli_or_api_open_can_wait_for_a_fifo_peer` | ✓ |
+| L1FC-03 | `api::backtest::read` refuses a FIFO at the results ledger path as not a regular file within 5 s (G5-2, D-4732) | `api::backtest::tests::a_fifo_at_the_ledger_path_is_refused_without_waiting` | ✓ |
+| L1FC-14 | With no `O_NONBLOCK`, a read-write open of a FIFO returns at once while a read-only open waits for a writer, which is the premise under which the FIFO scan admits a read-write `OpenOptions` (G5-2, D-4732) | `cli::readonly_file::tests::a_read_write_open_of_a_fifo_never_waits` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 552ce57a..c9bc3cd0 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65025,3 +65025,53 @@ on a live leader would derive a second time and lose the single-flight
 guarantee D-1443 exists for. **Honest limit:** the `Landing` kill depends on
 test order. A rename that sorted a single-flight test ahead of it would
 restore the timeout, so the ordering is pinned in the test's own doc.
+
+### D-4732 — Every `cli` and `api` open that could wait on a FIFO peer goes through a non-blocking door, and a scan holds it — 2026-10-09
+
+**Finding.** G5-2 and W2-cli13-4. D-1743 is headed "Every cli ledger open sets
+`O_NONBLOCK` and admits only a regular file". It fixed the opens it listed.
+`frontier.rs`, `trades.rs` and four `sweep_evidence.rs` readers still used
+`File::open`. A census of `crates/cli/src` and `crates/api/src` release code
+counted 170 opens that set no `O_NONBLOCK`: plain `File::open` and
+`File::create` calls, `OpenOptions` chains, and directory opens used as
+durability barriers. A FIFO planted at one of those paths made `open(2)` wait
+for a peer that never came. `a_fifo_at_any_cli_ledger_path_refuses_without_waiting`
+failed on the first new door it reached: "opening …/results/frontier.bin
+waited for a FIFO peer".
+
+**The decision.**
+- `readonly_file` is `pub`, with three doors:
+  - `regular`, the D-1743 door, now public;
+  - `read`, for a read-only regular file;
+  - `directory`, a read-only open with `O_NONBLOCK` that refuses anything but
+    a directory.
+- Every `File::open` and `File::create` in `cli` and `api` release code now
+  goes through one of these doors, and so does every `OpenOptions` chain that
+  set no non-blocking flag and did not open read-write.
+- A read-write open never waits on a FIFO (fifo(7)), and
+  `a_read_write_open_of_a_fifo_never_waits` pins that. `api`'s serve lock
+  stays a plain read-write open, so a lock name that reaches a device still
+  reaches its stamp. `a_serve_lock_stamp_that_fails_is_cleared_or_refused_never_left_stale`
+  drives that path through `/dev/full`, and routing the lock through
+  `regular` broke it.
+- Sixteen modules opened their handle with `O_NOFOLLOW` alone. They now use
+  `O_NOFOLLOW_NONBLOCK` and keep their own type check.
+- The FIFO test now also covers `Frontier`, `Trades` and five `sweep_evidence`
+  readers. Each is planted and refused within its bounded wait.
+- `api::backtest::read` gets its own FIFO test with a 5 s bound.
+- `no_cli_or_api_open_can_wait_for_a_fifo_peer` walks every release source in
+  both crates, with `#[cfg(test)]` items removed. It refuses an unqualified
+  `File::open(` or `File::create(`, and any `OpenOptions::new()` that does not
+  reach `regular`, open read-write, or carry a `NONBLOCK` custom flag.
+
+D-1743's heading claimed every ledger open. That claim is true only from this
+entry on.
+
+**Honest limits.** The scan does not see `std::fs::read`, `fs::write`,
+`read_to_string` or `File::create_new`. `create_new` cannot wait, because
+`O_EXCL` fails on any existing name. The other three are whole-file helpers in
+`api` asset and config paths, and they are listed in `docs/06-limits.md`.
+The scan works on source text, not syntax: an open built through a helper
+that hides `OpenOptions::new()` is not seen.
+
+Invariants L1FC-01, L1FC-02, L1FC-03, L1FC-14.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 12e9bf09..8b2c0d86 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15967,3 +15967,31 @@ on every request. D-0904 names the same directory walk for three other
 routes; this one was named only in D-1445's list of audited routes. The second
 open is the freshness check, and it is kept. **No timing was taken. The cost
 is UNVERIFIED as a measurement.**
+
+### The FIFO-peer scan reads source text and sees three spellings (D-4732)
+
+`readonly_file::regular`, `read` and `directory` add `O_NONBLOCK` and one
+`fstat` to each open they replace. That is O(1) per open, and no bar or
+candidate loop opens a file. `no_cli_or_api_open_can_wait_for_a_fifo_peer`
+proves only what it reads. It sees release-build source text in `crates/cli/src`
+and `crates/api/src`, with `#[cfg(test)]` items removed by brace and
+indentation, so it depends on `cargo fmt --check`. It refuses:
+- an unqualified `File::open(`;
+- an unqualified `File::create(`;
+- an `OpenOptions::new()` that does not reach `regular`, open read-write, or
+  name a `NONBLOCK` custom flag before its `.open(`.
+
+A read-write open is admitted because `open(2)` never waits on a FIFO for
+`O_RDWR`, which `a_read_write_open_of_a_fifo_never_waits` measures. A read or
+write on such a handle can still wait, and the scan does not see it.
+
+It does not see these:
+- `std::fs::read`, `fs::read_to_string` and `fs::write`. These remain in `api`
+  asset, config and probe paths, and each can wait on a FIFO planted at its
+  path.
+- `File::create_new`, which cannot wait, because `O_EXCL` fails on any
+  existing name.
+- An open built behind a helper that hides `OpenOptions::new()`.
+
+**UNVERIFIED**: how many such whole-file helpers can be reached from an
+operator-writable path. They were not counted.
-- 
2.43.0


From dc897b756b83ab13ebc755dbe8fd98b301faccf9 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:03:51 +0000
Subject: [PATCH 2/9] cli: check the search-checkpoint staged marker by the
 name it is written under (G5-1)

Before: a_torn_completion_marker_is_an_interrupted_reservation asserted
that complete.writing did not exist after a publication. No code writes
that name; publish_marker stages complete.tmp. The check held for any
publisher, including one that copied the seal and left the staged file
behind. D-1770 named the same nonexistent file.

After: STAGED_MARKER is the one name. The test observes the reservation
while staging (exactly payload and STAGED_MARKER), requires the staged
file gone and a whole 32-byte complete after the acknowledgement, and
plants a kill between the synced seal and the rename: the reopen counts
it interrupted, keeps the older checkpoint latest, publishes the next
sequence and leaves the staged file untouched. A copy-instead-of-rename
mutant now fails at search_checkpoint_tests.rs:354. D-4733 corrects
D-1770. L1FC-04.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/search_checkpoint.rs       | 13 +++--
 crates/cli/src/search_checkpoint_tests.rs | 66 ++++++++++++++++++++++-
 docs/04-invariants.md                     |  1 +
 docs/05-decisions.md                      | 27 ++++++++++
 4 files changed, 102 insertions(+), 5 deletions(-)

diff --git a/crates/cli/src/search_checkpoint.rs b/crates/cli/src/search_checkpoint.rs
index 68eb75c4..5de1b11a 100644
--- a/crates/cli/src/search_checkpoint.rs
+++ b/crates/cli/src/search_checkpoint.rs
@@ -307,14 +307,19 @@ impl Journal {
     }
 }
 
+/// The name a completion seal is staged under before its rename (D-1740).
+/// Discovery never reads it. One constant, so the test that requires it gone
+/// after an acknowledgement checks the name this file writes (G5-1, D-4733).
+const STAGED_MARKER: &str = "complete.tmp";
+
 /// Make `complete` appear whole or not at all (D-1740). The seal is written
-/// and synced under a temporary name and only then renamed into place, then
+/// and synced under [`STAGED_MARKER`] and only then renamed into place, then
 /// the reservation directory is synced. A kill before the rename leaves at
-/// most `complete.tmp`, which discovery never reads, so the reservation is an
-/// interrupted one and the previous checkpoint stays the resume point. A
+/// most that staged file, which discovery never reads, so the reservation is
+/// an interrupted one and the previous checkpoint stays the resume point. A
 /// failure this process sees removes the temporary file and refuses.
 fn publish_marker(directory: &Path, seal: [u8; 32]) -> Result<(), String> {
-    let temporary = directory.join("complete.tmp");
+    let temporary = directory.join(STAGED_MARKER);
     let written = (|| {
         let mut marker = File::create_new(&temporary)?;
         #[cfg(test)]
diff --git a/crates/cli/src/search_checkpoint_tests.rs b/crates/cli/src/search_checkpoint_tests.rs
index 78ca7b97..6d822038 100644
--- a/crates/cli/src/search_checkpoint_tests.rs
+++ b/crates/cli/src/search_checkpoint_tests.rs
@@ -309,6 +309,13 @@ fn os_litter_is_passed_over_and_a_stranger_is_named() -> Result<(), String> {
 /// CE-3, D-1909: a completion marker cut short by a crash is an interrupted
 /// reservation. The newest whole checkpoint is `latest`, and the resume
 /// publishes the next sequence. A publication leaves no scratch marker.
+///
+/// G5-1 (D-4733): the scratch-marker check named `complete.writing`, a file
+/// no code writes, so it held whatever the publisher left behind. The name is
+/// now observed while the publisher is staging -- the reservation holds the
+/// payload and exactly that staged file -- then required gone once the
+/// publication is acknowledged, and a staged marker a kill left before its
+/// rename is required present and ignored by the reopen.
 #[test]
 fn a_torn_completion_marker_is_an_interrupted_reservation() -> Result<(), String> {
     // 0 only: a short NON-EMPTY marker is refused, not passed over
@@ -318,9 +325,41 @@ fn a_torn_completion_marker_is_an_interrupted_reservation() -> Result<(), String
         let scratch = Scratch::new().map_err(error)?;
         let mut journal = Journal::open(&scratch.0, "expression-search-v1", [12; 32])?;
         journal.publish(b"valid old", 1024)?;
+        let staging: Rc<RefCell<Option<Vec<String>>>> = Rc::new(RefCell::new(None));
+        let slot = Rc::clone(&staging);
+        MARKER_CREATED.with(|hook| {
+            *hook.borrow_mut() = Some(Box::new(move |namespace: &Path| {
+                let mut names: Vec<String> = fs::read_dir(namespace.join("0000000000000002"))
+                    .map(|entries| {
+                        entries
+                            .filter_map(Result::ok)
+                            .map(|entry| entry.file_name().to_string_lossy().into_owned())
+                            .collect()
+                    })
+                    .unwrap_or_default();
+                names.sort();
+                *slot.borrow_mut() = Some(names);
+            }));
+        });
         journal.publish(b"torn", 1024)?;
         let newest = journal.directory.join("0000000000000002");
-        assert!(!newest.join("complete.writing").exists());
+        assert_eq!(
+            staging
+                .borrow_mut()
+                .take()
+                .ok_or("the staging hook did not run")?,
+            vec![STAGED_MARKER.to_owned(), "payload".to_owned()],
+            "the publisher stages its seal under exactly the name checked below"
+        );
+        assert!(
+            !newest.join(STAGED_MARKER).exists(),
+            "an acknowledged publication leaves no staged marker"
+        );
+        assert_eq!(
+            fs::read(newest.join("complete")).map_err(error)?.len(),
+            32,
+            "the staged seal was renamed into place whole"
+        );
         OpenOptions::new()
             .write(true)
             .open(newest.join("complete"))
@@ -337,6 +376,31 @@ fn a_torn_completion_marker_is_an_interrupted_reservation() -> Result<(), String
         assert_eq!(sequence, 3);
         let resumed = reopened.latest(1024)?.ok_or("the resume is latest")?;
         assert_eq!(resumed.payload, b"resumed");
+
+        // A kill after the staged seal was synced and before its rename: the
+        // reservation holds the whole seal under the staged name and no
+        // `complete`. The reopen passes it over as interrupted and leaves it.
+        let killed = reopened.directory.join("0000000000000003");
+        let seal = fs::read(killed.join("complete")).map_err(error)?;
+        fs::remove_file(killed.join("complete")).map_err(error)?;
+        fs::write(killed.join(STAGED_MARKER), &seal).map_err(error)?;
+        drop(reopened);
+        let mut after_kill = Journal::open(&scratch.0, "expression-search-v1", [12; 32])?;
+        assert_eq!(
+            (after_kill.interrupted(), after_kill.acknowledged()),
+            (2, 1),
+            "the torn and the staged reservations are both interrupted"
+        );
+        assert_eq!(
+            after_kill.latest(1024)?.ok_or("latest")?.payload,
+            b"valid old"
+        );
+        assert_eq!(after_kill.publish(b"after the kill", 1024)?.0, 4);
+        assert_eq!(
+            fs::read(killed.join(STAGED_MARKER)).map_err(error)?,
+            seal,
+            "the staged marker is ignored, not read, renamed or removed"
+        );
     }
     Ok(())
 }
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 91017ca5..1e43d666 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7056,3 +7056,4 @@ old line regex the same input and watched it pass.
 | L1FC-02 | No `cli` or `api` release source calls an unqualified `File::open` or `File::create`, or builds an `OpenOptions` that does not reach `readonly_file::regular`, open read-write, or carry a `NONBLOCK` custom flag (G5-2, D-4732) | `cli::readonly_file::tests::no_cli_or_api_open_can_wait_for_a_fifo_peer` | ✓ |
 | L1FC-03 | `api::backtest::read` refuses a FIFO at the results ledger path as not a regular file within 5 s (G5-2, D-4732) | `api::backtest::tests::a_fifo_at_the_ledger_path_is_refused_without_waiting` | ✓ |
 | L1FC-14 | With no `O_NONBLOCK`, a read-write open of a FIFO returns at once while a read-only open waits for a writer, which is the premise under which the FIFO scan admits a read-write `OpenOptions` (G5-2, D-4732) | `cli::readonly_file::tests::a_read_write_open_of_a_fifo_never_waits` | ✓ |
+| L1FC-04 | A checkpoint publication stages its seal as exactly `STAGED_MARKER` beside the payload and leaves no staged marker once acknowledged; a staged marker that a kill left before the rename is interrupted, ignored and left untouched by the reopen (G5-1, D-4733) | `cli::search_checkpoint::tests::a_torn_completion_marker_is_an_interrupted_reservation` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index c9bc3cd0..cdbae60e 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65075,3 +65075,30 @@ The scan works on source text, not syntax: an open built through a helper
 that hides `OpenOptions::new()` is not seen.
 
 Invariants L1FC-01, L1FC-02, L1FC-03, L1FC-14.
+
+### D-4733 — The search-checkpoint staged marker is checked by its real name; corrects D-1770 — 2026-10-09
+
+**Finding.** G5-1. `a_torn_completion_marker_is_an_interrupted_reservation`
+asserted `!newest.join("complete.writing").exists()`. No code writes that
+name: at fbdabaec the assertion is the only place in `crates/` it appears.
+`publish_marker` stages its seal as `complete.tmp`. The check therefore held
+whatever the publisher left behind. D-1770's bullet ("Staging's
+`complete.writing` is kept") named the same file that does not exist.
+
+**The decision.** The name is one constant, `STAGED_MARKER = "complete.tmp"`.
+The test watches the publisher from the `marker_created` hook. While staging,
+the reservation must hold exactly `payload` and `STAGED_MARKER`. After the
+publication is acknowledged, `STAGED_MARKER` must be gone and `complete` must
+hold all 32 bytes. A kill between the synced seal and the rename leaves
+`STAGED_MARKER` and no `complete`. The reopen must count that reservation as
+interrupted, keep the older checkpoint as latest, publish the next sequence,
+and leave the staged file byte for byte as it was.
+
+A mutant publisher that copies instead of renaming fails the new check at
+`search_checkpoint_tests.rs:354` ("an acknowledged publication leaves no staged
+marker"). The old check passed it.
+
+**Corrects D-1770.** The kept staging name is `complete.tmp`, not
+`complete.writing`. D-1770 stays as written; this entry is the correction.
+
+Invariant L1FC-04.
-- 
2.43.0


From 0b587b7ecc6893e4144ee0c4be2f6194d2725591 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:03:52 +0000
Subject: [PATCH 3/9] pull, vocab: stop pull saying NSE trades only Monday to
 Friday, and scan every crate (G5-3)

Before: D-1667 removed the weekday-only sentence from indicators and
vocab and pinned only those two files. pull::fold and pull's unit test
still said "an exchange that trades Monday to Friday", against the six
weekend sessions docs/00-charter.md section 3 records. The fold.rs copy
was wrapped across two // lines, so whitespace flattening alone could
not have seen it.

After: both say "ordinarily trades Monday to Friday" and cite the
charter. The stale-claims test walks every .rs file under crates/ and
reads it as prose with comment markers removed. With the pull edits
reverted it failed: "crates/pull/src/fold.rs still says "an exchange that
trades Monday to Friday"". Only comments changed in pull, so no new
string literal reached gates 1c and 1d. D-4734, L1FC-05.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/pull/src/fold.rs            |  5 ++--
 crates/pull/tests/unit.rs          |  3 +-
 crates/vocab/tests/stale_claims.rs | 48 ++++++++++++++++++++++++++++--
 docs/04-invariants.md              |  1 +
 docs/05-decisions.md               | 18 +++++++++++
 5 files changed, 69 insertions(+), 6 deletions(-)

diff --git a/crates/pull/src/fold.rs b/crates/pull/src/fold.rs
index 9f2fbcac..368769b3 100644
--- a/crates/pull/src/fold.rs
+++ b/crates/pull/src/fold.rs
@@ -314,8 +314,9 @@ pub fn fold(snapshots: &[Bar], bucket: Bucket) -> Result<Vec<Bar>, FoldError> {
         // day BEFORE. Floored to the UTC grid it was re-stamped at 00:00 UTC of
         // that earlier day, so EVERY DAILY BAR MOVED BACK ONE CALENDAR DAY.
         // Measured on the store this produced: 86 records, 20 of them stamped
-        // on a SUNDAY and none on a Friday, on an exchange that trades Monday
-        // to Friday. `crate::ingest`'s month guard caught it only where the
+        // on a SUNDAY and none on a Friday, on an exchange that ordinarily trades
+        // Monday to Friday (`docs/00-charter.md` §3 records six weekend
+        // sessions). `crate::ingest`'s month guard caught it only where the
         // shift crossed a month boundary — "bars span 2025-12 to 2026-01" — and
         // stored a wrong answer SILENTLY everywhere else, which is exactly the
         // W1 class `crate::fetch` names and says is undetectable once written.
diff --git a/crates/pull/tests/unit.rs b/crates/pull/tests/unit.rs
index 21886dd2..17cb38c6 100644
--- a/crates/pull/tests/unit.rs
+++ b/crates/pull/tests/unit.rs
@@ -6052,7 +6052,8 @@ fn snapshots_sharing_a_second_fold_into_one_bar() {
 ///
 /// A live 1day pull refused 10 of 16 members with "bars span 2025-12 to
 /// 2026-01" and stored 86 bars, 20 of them stamped on a **Sunday** and none on
-/// a Friday, on an exchange that trades Monday to Friday.
+/// a Friday, on an exchange that ordinarily trades Monday to Friday
+/// (`docs/00-charter.md` §3 records six weekend sessions).
 ///
 /// The cause was the bucket's origin. `fold` floored to a grid anchored at the
 /// Unix epoch — UTC midnight — while `crate::ingest` derives the month in IST.
diff --git a/crates/vocab/tests/stale_claims.rs b/crates/vocab/tests/stale_claims.rs
index eaebeb54..26fde29b 100644
--- a/crates/vocab/tests/stale_claims.rs
+++ b/crates/vocab/tests/stale_claims.rs
@@ -51,6 +51,24 @@ fn flat(text: &str) -> String {
     text.split_whitespace().collect::<Vec<_>>().join(" ")
 }
 
+/// `text` with each line's leading comment marker (`//`, `///` or `//!`) dropped
+/// and its whitespace collapsed, so a sentence a comment wraps across lines
+/// reads as the one sentence it is. `pull::fold` wrapped "trades Monday // to
+/// Friday", and collapsing whitespace alone could not see it (G5-3, D-4734).
+fn prose(text: &str) -> String {
+    let lines: Vec<&str> = text
+        .lines()
+        .map(|line| {
+            let line = line.trim_start();
+            line.strip_prefix("///")
+                .or_else(|| line.strip_prefix("//!"))
+                .or_else(|| line.strip_prefix("//"))
+                .unwrap_or(line)
+        })
+        .collect();
+    flat(&lines.join("\n"))
+}
+
 /// The text from the line starting `start` up to the next line starting
 /// `end`, or to the end of the document.
 fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
@@ -410,10 +428,29 @@ fn the_corrected_sentences_do_not_return() {
 /// record six weekend sessions and 1,710 bars. The behaviour (a weekend bar sets
 /// no weekday bit) is right and stays; the sentences that called the exchange
 /// weekday-only are refused here so they cannot return. D-1667.
+///
+/// G5-3 (D-4734): the list held only the two files the finding named, and
+/// `pull::fold` and its unit test still said "an exchange that trades Monday to
+/// Friday". Every `.rs` file under `crates/` is read now, so a sibling cannot
+/// keep the sentence by not being listed.
 #[test]
 fn no_weekday_comment_says_nse_never_trades_on_a_weekend() {
-    for file in ["crates/indicators/src/lib.rs", "crates/vocab/src/table.rs"] {
-        let text = flat(&read(file));
+    let mut files = Vec::new();
+    rust_sources(&root().join("crates"), &mut files);
+    assert!(
+        files
+            .iter()
+            .any(|path| path.ends_with("crates/pull/src/fold.rs"))
+            && files.len() > 300,
+        "premise: the walk reached every crate ({} files)",
+        files.len()
+    );
+    let this_file = root().join("crates/vocab/tests/stale_claims.rs");
+    for path in files.iter().filter(|path| **path != this_file) {
+        let text = prose(
+            &std::fs::read_to_string(path)
+                .unwrap_or_else(|why| panic!("{} must be readable: {why}", path.display())),
+        );
         for stale in [
             "NSE trades Monday to Friday",
             "NSE does not trade them",
@@ -421,7 +458,11 @@ fn no_weekday_comment_says_nse_never_trades_on_a_weekend() {
             "an exchange that trades Monday to Friday",
             "A weekend bar in an equity series is a store defect",
         ] {
-            assert!(!text.contains(stale), "{file} still says {stale:?}");
+            assert!(
+                !text.contains(stale),
+                "{} still says {stale:?}",
+                path.display()
+            );
         }
     }
     let lib = flat(&read("crates/indicators/src/lib.rs"));
@@ -581,3 +622,4 @@ fn every_count_and_kind_the_documents_state_is_the_tables() {
     }
     assert_eq!(next, COUNT, "the group table stops at {next} of {COUNT}");
 }
+
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 1e43d666..4aadae35 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7057,3 +7057,4 @@ old line regex the same input and watched it pass.
 | L1FC-03 | `api::backtest::read` refuses a FIFO at the results ledger path as not a regular file within 5 s (G5-2, D-4732) | `api::backtest::tests::a_fifo_at_the_ledger_path_is_refused_without_waiting` | ✓ |
 | L1FC-14 | With no `O_NONBLOCK`, a read-write open of a FIFO returns at once while a read-only open waits for a writer, which is the premise under which the FIFO scan admits a read-write `OpenOptions` (G5-2, D-4732) | `cli::readonly_file::tests::a_read_write_open_of_a_fifo_never_waits` | ✓ |
 | L1FC-04 | A checkpoint publication stages its seal as exactly `STAGED_MARKER` beside the payload and leaves no staged marker once acknowledged; a staged marker that a kill left before the rename is interrupted, ignored and left untouched by the reopen (G5-1, D-4733) | `cli::search_checkpoint::tests::a_torn_completion_marker_is_an_interrupted_reservation` | ✓ |
+| L1FC-05 | No `.rs` file under `crates/`, read as prose with comment markers removed, says NSE trades only Monday to Friday (G5-3, D-4734) | `no_weekday_comment_says_nse_never_trades_on_a_weekend` in `crates/vocab/tests/stale_claims.rs` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index cdbae60e..23392a82 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65102,3 +65102,21 @@ marker"). The old check passed it.
 `complete.writing`. D-1770 stays as written; this entry is the correction.
 
 Invariant L1FC-04.
+
+### D-4734 — No `.rs` file in `crates/` says NSE trades only Monday to Friday — 2026-10-09
+
+**Finding.** G5-3. D-1667 removed the weekday-only sentence from two files and
+pinned those two. `pull::fold` and `pull`'s unit test still said "an exchange
+that trades Monday to Friday", and `docs/00-charter.md` §3 records six weekend
+sessions.
+
+**The decision.** Both now read "ordinarily trades Monday to Friday", with the
+charter's count. `no_weekday_comment_says_nse_never_trades_on_a_weekend` now
+walks every `.rs` file under `crates/` and reads the files as prose, with
+comment markers removed. The `fold.rs` sentence was wrapped across two `//`
+lines, and whitespace collapsing alone could not see it. With the `pull`
+edits reverted, the test failed: "…/crates/pull/src/fold.rs still says "an
+exchange that trades Monday to Friday"". Only comments changed in `pull`, and
+no string literal was added there (gates 1c and 1d).
+
+Invariant L1FC-05.
-- 
2.43.0


From 3dd648a16d345bb723218f486d082f4dba067a1a Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:03:52 +0000
Subject: [PATCH 4/9] cli: delete minute_gaps::withhold_holed_days and state
 the census bound as O(d) (G5-4)

Before: withhold_holed_days was public, had no caller, and took its day
set from the interior census alone. That is the census D-1662 replaced,
because it cannot see a session that stops early. A new test counted two
release callers of days_with_interior_gaps where there should be one.
docs/06-limits.md priced days_with_minute_holes at d log d, although the
sort had been replaced by an O(d) merge.

After: the function is gone. Its tests drive days_with_minute_holes,
withhold and GapExclusion::signal_only or one_series, as the doors do. A
new case shows that an early-stopped session is withheld although the
interior census finds no hole. The limits entry says
O(signal + minutes + d). D-4735, L1FC-06, L1FC-07.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/minute_gaps.rs | 142 +++++++++++++++++++++++-----------
 docs/04-invariants.md         |   2 +
 docs/05-decisions.md          |  20 +++++
 docs/06-limits.md             |   6 +-
 4 files changed, 121 insertions(+), 49 deletions(-)

diff --git a/crates/cli/src/minute_gaps.rs b/crates/cli/src/minute_gaps.rs
index 68b1bce6..18528c5c 100644
--- a/crates/cli/src/minute_gaps.rs
+++ b/crates/cli/src/minute_gaps.rs
@@ -431,38 +431,49 @@ pub fn withhold(bars: &[Candle], days: &[i64]) -> (Vec<Candle>, u64) {
     (kept, removed)
 }
 
-/// Withhold the holed days from a signal series and its exact-minute stream.
-///
-/// **Both slices are filtered by ONE measured day set, and that is the whole
-/// correctness argument.** The set is derived from the minute stream, because
-/// the minute stream is what the overlay indexes; filtering the signal by a
-/// different set would leave a signal bar whose closing minute had been removed,
-/// which is the refusal this exists to prevent, arriving by a new route.
-#[must_use]
-pub fn withhold_holed_days(
-    signal: &[Candle],
-    minutes: &[Candle],
-) -> (Vec<Candle>, Vec<Candle>, GapExclusion) {
-    let days = days_with_interior_gaps(minutes);
-    if days.is_empty() {
-        return (signal.to_vec(), minutes.to_vec(), GapExclusion::none());
-    }
-    let (kept_signal, signal_bars) = withhold(signal, &days);
-    let (kept_minutes, minute_bars) = withhold(minutes, &days);
-    (
-        kept_signal,
-        kept_minutes,
-        GapExclusion {
-            days,
-            signal_bars,
-            minute_bars,
-        },
-    )
-}
-
 #[cfg(test)]
 mod tests {
 
+    /// G5-4 (D-4735): no release-build function withholds by the edge-blind
+    /// interior census alone. `withhold_holed_days` derived its day set from
+    /// [`days_with_interior_gaps`] only -- the census W2-cli9-3 (D-1662)
+    /// replaced because it cannot see a session that stops early -- and stayed
+    /// `pub` with no caller, so the first door to reach for it would have
+    /// brought that defect back. The one release call of the interior census
+    /// must be the one inside [`days_with_minute_holes`].
+    #[test]
+    fn only_the_overlay_census_calls_the_interior_census() {
+        let mut sources = Vec::new();
+        crate::readonly_file::tests::release_sources(
+            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
+            &mut sources,
+        );
+        assert!(sources.len() > 100, "premise: crates/cli/src was walked");
+        let mut callers = Vec::new();
+        for (path, text) in &sources {
+            for (at, _) in text.match_indices("days_with_interior_gaps(") {
+                let head = text.get(..at).unwrap_or("");
+                let line = head.rsplit('\n').next().unwrap_or("");
+                if line.trim_start().starts_with("//") || head.ends_with("fn ") {
+                    continue;
+                }
+                let caller = head
+                    .rfind("fn ")
+                    .and_then(|fn_at| text.get(fn_at + 3..))
+                    .and_then(|name| name.split(['(', '<']).next())
+                    .unwrap_or("?");
+                callers.push(format!("{}: {caller}", path.display()));
+            }
+        }
+        assert_eq!(callers.len(), 1, "{callers:#?}");
+        assert!(
+            callers
+                .iter()
+                .all(|c| c.ends_with("minute_gaps.rs: days_with_minute_holes")),
+            "{callers:#?}"
+        );
+    }
+
     /// `days_with_minute_holes` unions its two ascending lists by a merge, not
     /// a sort (gate 11 rule 4, D-1662): ascending, each day once, nothing
     /// dropped, on every interleaving including empty sides and full overlap.
@@ -560,7 +571,17 @@ mod tests {
         assert_eq!(days_with_interior_gaps(&bars), vec![18_501, 19_058]);
     }
 
-    /// Both slices lose the SAME days, which is the property the overlay needs.
+    /// Five minutes, the signal length the tests below use.
+    const FIVE_MINUTES: i64 = 5 * MINUTE_MICROS;
+
+    /// The census every door uses, with no session close known, so each signal
+    /// bar demands its own last minute.
+    fn holes(signal: &[Candle], minutes: &[Candle]) -> Vec<i64> {
+        days_with_minute_holes(signal, minutes, FIVE_MINUTES, |_| None)
+    }
+
+    /// Both slices lose the SAME days, which is the property the overlay needs:
+    /// one census, then [`withhold`] on each slice with that one day set.
     #[test]
     fn the_signal_and_the_minute_stream_lose_the_same_days() {
         let mut minutes: Vec<Candle> = Vec::new();
@@ -574,17 +595,12 @@ mod tests {
             signal.extend((0..4).map(|b| bar_on(day, b * 5)));
         }
 
-        let (kept_signal, kept_minutes, excluded) = withhold_holed_days(&signal, &minutes);
-        assert_eq!(excluded.day_numbers(), &[19_001]);
-        assert_eq!(excluded.days(), 1);
-        assert_eq!(excluded.signal_bars(), 4, "the holed day's signal bars go");
-        assert_eq!(
-            excluded.minute_bars(),
-            4,
-            "and its minute bars go with them"
-        );
-        assert!(!excluded.is_empty());
-
+        let days = holes(&signal, &minutes);
+        assert_eq!(days, vec![19_001]);
+        let (kept_signal, signal_bars) = withhold(&signal, &days);
+        let (kept_minutes, minute_bars) = withhold(&minutes, &days);
+        assert_eq!(signal_bars, 4, "the holed day's signal bars go");
+        assert_eq!(minute_bars, 4, "and its minute bars go with them");
         for bar in kept_signal.iter().chain(kept_minutes.iter()) {
             assert_ne!(
                 indicators::ist_day(bar.ts_micros),
@@ -592,6 +608,31 @@ mod tests {
                 "no bar of a withheld day may survive in either slice"
             );
         }
+        let excluded = GapExclusion::signal_only(days, signal_bars);
+        assert_eq!(excluded.day_numbers(), &[19_001]);
+        assert_eq!(excluded.days(), 1);
+        assert_eq!(excluded.signal_bars(), 4);
+        assert_eq!(excluded.minute_bars(), 0, "a coarse door keeps its minutes");
+        assert!(!excluded.is_empty());
+    }
+
+    /// G5-4 (D-4735): a session that stops early is withheld. The interior
+    /// census alone -- all the deleted `withhold_holed_days` asked -- sees no
+    /// hole, because the step from the last minute crosses the night; the
+    /// overlay census sees the closing minute the last signal bar demands.
+    #[test]
+    fn a_session_that_stops_early_is_withheld() {
+        let minutes: Vec<Candle> = (0..19).map(|m| bar_on(19_000, m)).collect();
+        let signal: Vec<Candle> = (0..4).map(|b| bar_on(19_000, b * 5)).collect();
+        assert!(
+            days_with_interior_gaps(&minutes).is_empty(),
+            "premise: the interior census cannot see an early stop"
+        );
+        let days = holes(&signal, &minutes);
+        assert_eq!(days, vec![19_000], "the 09:34 closing minute is missing");
+        let (kept, removed) = withhold(&signal, &days);
+        assert!(kept.is_empty());
+        assert_eq!(removed, 4);
     }
 
     /// A gap-free span withholds nothing and copies both slices unchanged.
@@ -599,14 +640,18 @@ mod tests {
     fn a_gap_free_span_withholds_nothing() {
         let minutes: Vec<Candle> = (0..40).map(|m| bar_on(19_000, m)).collect();
         let signal: Vec<Candle> = (0..8).map(|b| bar_on(19_000, b * 5)).collect();
-        let (kept_signal, kept_minutes, excluded) = withhold_holed_days(&signal, &minutes);
-        assert!(excluded.is_empty());
-        assert_eq!(excluded.days(), 0);
-        assert_eq!(excluded.signal_bars(), 0);
-        assert_eq!(excluded.minute_bars(), 0);
+        let days = holes(&signal, &minutes);
+        assert!(days.is_empty());
+        let (kept_signal, signal_bars) = withhold(&signal, &days);
+        let (kept_minutes, minute_bars) = withhold(&minutes, &days);
+        assert_eq!((signal_bars, minute_bars), (0, 0));
         assert_eq!(kept_signal, signal);
         assert_eq!(kept_minutes, minutes);
+        let excluded = GapExclusion::signal_only(days, signal_bars);
+        assert!(excluded.is_empty());
+        assert_eq!(excluded.days(), 0);
         assert!(excluded.day_names().is_empty());
+        assert_eq!(excluded, GapExclusion::none());
     }
 
     /// The days are named as dates, which is what an operator can check.
@@ -616,7 +661,10 @@ mod tests {
             .iter()
             .map(|m| bar_on(19_522, *m))
             .collect();
-        let (_, _, excluded) = withhold_holed_days(&[], &minutes);
+        let days = holes(&[], &minutes);
+        let (_, removed) = withhold(&minutes, &days);
+        let excluded = GapExclusion::one_series(days, removed);
         assert_eq!(excluded.day_names(), vec!["2023-06-14".to_owned()]);
+        assert_eq!((excluded.signal_bars(), excluded.minute_bars()), (4, 4));
     }
 }
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 4aadae35..89a3b3bd 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7058,3 +7058,5 @@ old line regex the same input and watched it pass.
 | L1FC-14 | With no `O_NONBLOCK`, a read-write open of a FIFO returns at once while a read-only open waits for a writer, which is the premise under which the FIFO scan admits a read-write `OpenOptions` (G5-2, D-4732) | `cli::readonly_file::tests::a_read_write_open_of_a_fifo_never_waits` | ✓ |
 | L1FC-04 | A checkpoint publication stages its seal as exactly `STAGED_MARKER` beside the payload and leaves no staged marker once acknowledged; a staged marker that a kill left before the rename is interrupted, ignored and left untouched by the reopen (G5-1, D-4733) | `cli::search_checkpoint::tests::a_torn_completion_marker_is_an_interrupted_reservation` | ✓ |
 | L1FC-05 | No `.rs` file under `crates/`, read as prose with comment markers removed, says NSE trades only Monday to Friday (G5-3, D-4734) | `no_weekday_comment_says_nse_never_trades_on_a_weekend` in `crates/vocab/tests/stale_claims.rs` | ✓ |
+| L1FC-06 | The edge-blind interior minute census has exactly one release caller, `days_with_minute_holes` (G5-4, D-4735) | `cli::minute_gaps::tests::only_the_overlay_census_calls_the_interior_census` | ✓ |
+| L1FC-07 | A session that stops before its last demanded closing minute is withheld by the overlay census, although the interior census finds no hole (G5-4, D-4735) | `cli::minute_gaps::tests::a_session_that_stops_early_is_withheld` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 23392a82..b7786567 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65120,3 +65120,23 @@ exchange that trades Monday to Friday"". Only comments changed in `pull`, and
 no string literal was added there (gates 1c and 1d).
 
 Invariant L1FC-05.
+
+### D-4735 — `withhold_holed_days` is deleted; the census bound is O(d), not d log d — 2026-10-09
+
+**Finding.** G5-4. `minute_gaps::withhold_holed_days` took its day set from
+`days_with_interior_gaps` alone. W2-cli9-3 (D-1662) replaced that census
+because it cannot see a session that stops early. The function was still
+`pub`, and nothing called it. A new test found two release callers of the
+interior census, `days_with_minute_holes` and `withhold_holed_days`, where
+there should be one. `docs/06-limits.md` priced the census at "O(signal +
+minutes + d log d)", but D-1662 replaced the sort with an O(d) merge.
+
+**The decision.** The function is deleted, so the edge-blind census has one
+caller, inside `days_with_minute_holes`, and a test pins that. Its three tests
+now drive the doors' real path: `days_with_minute_holes`, `withhold` on each
+slice, and `GapExclusion::signal_only` or `one_series`. A new case,
+`a_session_that_stops_early_is_withheld`, shows that the interior census finds
+nothing where the overlay census withholds the day. `docs/06-limits.md` now
+says O(signal + minutes + d).
+
+Invariants L1FC-06, L1FC-07.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 8b2c0d86..ff7a34df 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15487,8 +15487,10 @@ measured bound**: read off the source, no bench times it. With
 `minute_gaps::days_with_minute_holes` (W2-cli9-3) withholds, before any column
 is built, every day with an interior minute gap and every day holding a
 signal bar whose demanded closing minute is absent. **O(signal + minutes +
-d log d)** for d flagged days, one `kind_of` lookup per signal bar, off every
-per-bar sweep path. **UNVERIFIED as a measured bound**: read off the source.
+d)** for d flagged days: the two ascending day lists are merged in O(d).
+This said `d log d` until G5-4 (D-4735), after D-1662 had removed the sort.
+One `kind_of` lookup per signal bar, off every per-bar sweep path.
+**UNVERIFIED as a measured bound**: read off the source.
 
 `column_withholding_at_build` and `exact_minute_withholding_unsourceable_days`
 keep their 64-pass loops (W2-cli8-6). Each pass still reloads the daily and
-- 
2.43.0


From 08185c40d32a715fa6834ef031676caa805ed693 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:03:53 +0000
Subject: [PATCH 5/9] cli, runner: correct comments that denied cli an arrow it
 has, and scan all crates for them (G1-4)

Before: cli::append_condition_names said "CLAUDE.md section 5 does not
give cli a vocab arrow". runner::report::names_from_words said vocab "is
not among" cli's arrows. crates/cli/Cargo.toml opened with "ONE
DEPENDENCY" and "NO ARROW TO store". cli declares all nine arrows (D-0683,
D-0208). D-1706's test read only the files directly under crates/cli/src,
and only six exact sentences.

After: each sentence is corrected; the manifest head is now D-0169
history. crate_graph_claims.rs walks every .rs file and Cargo.toml under
crates/ and takes cli's arrows from cli's own manifest (nine, by premise).
It flags a comment clause that pairs a denial word with one of those
arrows and either names cli or, inside crates/cli, names no crate before
the denial. Quoted citations are skipped. A detector test pins five false
sentences and six true ones. Against the fbdabaec tree it named exactly
the three sentences above. D-4736, L1FC-08, L1FC-09.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/Cargo.toml                  |  29 +--
 crates/cli/src/lib.rs                  |   6 +-
 crates/cli/tests/crate_graph_claims.rs | 319 +++++++++++++++++++++++--
 crates/runner/src/report.rs            |  17 +-
 docs/04-invariants.md                  |   2 +
 docs/05-decisions.md                   |  31 +++
 6 files changed, 350 insertions(+), 54 deletions(-)

diff --git a/crates/cli/Cargo.toml b/crates/cli/Cargo.toml
index 7c1cdec7..3abacc69 100644
--- a/crates/cli/Cargo.toml
+++ b/crates/cli/Cargo.toml
@@ -18,28 +18,23 @@ description  = "The operator entry point for the sweep: bars in, ladder walked,
 # had no caller and no route. `CLAUDE.md` §5 named that absence as the live gap
 # and `docs/06-limits.md` §78 recorded it. This closes it.
 #
-# ONE DEPENDENCY, AND IT IS THE JOIN CRATE. `runner` already carries `core`,
-# `costs`, `engine`, `indicators` and `vocab`, so naming it once is the whole
-# graph. Adding any of the five directly would be a second spelling of an arrow
-# that already exists.
-#
-# NO ARROW TO `store`, AND THAT IS THE POINT. The operator's standing rule is
-# that neither a vendor pull nor the bars already on disk may be used at all, so
-# the only honest input is `runner::synthetic`, generated in-process. A binary
-# that could open the store would make the rule a matter of discipline again,
-# which is what gate 22 exists to prevent for the swept crates. This crate
-# declines the capability rather than declining to use it.
+# THE FIRST MANIFEST NAMED ONE DEPENDENCY, `runner`, AND DECLINED `store`
+# (D-0169). Both halves are history. The table below declares nine crates, the
+# nine arrows `CLAUDE.md` §5 draws for `cli` (D-0683), and each one's comment
+# says why it was added. `store` is the arrow D-0169 declined and D-0208 took;
+# the reasoning it replaced is quoted at that entry below (G1-4, D-4736).
 #
 # NOT ON GATE 22'S LIST, and it must not be added to one: clause A pins
 # `vocab`, `indicators` and `engine` to `vocab` alone. This crate is the caller,
 # not a swept crate, exactly as `runner` is.
 #
-# THREE ARROWS, NOT ONE, BECAUSE `runner` RE-EXPORTS NOTHING. `Sweeper::run`
-# takes an `indicators::Evaluator` and `Sweeper::new` takes an `engine::Ladder`,
-# so a caller cannot construct either without naming those crates itself. The
-# alternative -- adding `pub use` re-exports to `runner` -- would put a second
-# spelling of two public types into the crate that is currently the only join,
-# and a second spelling is a second thing to keep in agreement.
+# `engine` AND `indicators` BESIDE `runner`, BECAUSE `runner` RE-EXPORTS
+# NOTHING. `Sweeper::run` takes an `indicators::Evaluator` and `Sweeper::new`
+# takes an `engine::Ladder`, so a caller cannot construct either without naming
+# those crates itself. The alternative -- adding `pub use` re-exports to
+# `runner` -- would put a second spelling of two public types into the crate
+# that is currently the only join, and a second spelling is a second thing to
+# keep in agreement.
 [dependencies]
 runner     = { path = "../runner", version = "0.1.0" }
 engine     = { path = "../engine", version = "0.1.0" }
diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index c9ead1e6..bd75eb58 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -8219,8 +8219,10 @@ fn append_condition_names(out: &mut String, record: &crate::results::Record) {
     // was made and never what made it. Version 3 of the ledger carries the six
     // mask words for exactly this line.
     //
-    // `runner::report::names_from_words` and not `vocab` directly: see its own
-    // comment -- `CLAUDE.md` §5 does not give `cli` a `vocab` arrow.
+    // `runner::report::names_from_words` and not `vocab` directly, so this line
+    // and `render_top_record` decode the six stored words one way. `cli` does
+    // hold the `vocab` arrow (D-0683); the call is not a workaround for a
+    // missing one (G1-4, D-4736).
     let names = runner::report::names_from_words(record.mask_words);
     if names.is_empty() {
         // Distinguishable from "the names are missing". An all-zero mask means
diff --git a/crates/cli/tests/crate_graph_claims.rs b/crates/cli/tests/crate_graph_claims.rs
index 50099701..da9e2a86 100644
--- a/crates/cli/tests/crate_graph_claims.rs
+++ b/crates/cli/tests/crate_graph_claims.rs
@@ -1,5 +1,6 @@
-//! No source comment or lint reason in `cli` may deny an arrow `CLAUDE.md` §5
-//! draws (R9-cli-law-1, D-1706).
+//! No source comment or lint reason in the workspace may deny an arrow
+//! `CLAUDE.md` §5 draws for `cli` (R9-cli-law-1, D-1706; widened by G1-4,
+//! D-4736).
 //!
 //! `cli` depends on `vocab` directly: `crates/cli/Cargo.toml` declares it and
 //! §5 draws `... telemetry vocab <-- cli` (D-0683). Fifteen mask literals still
@@ -9,8 +10,20 @@
 //! type "belongs to runner's private dependency graph". Each was false, and a
 //! lint suppression kept for a false reason is the drift §5 warns about.
 //!
-//! This walks every `.rs` file under `crates/cli/src` at test time, so a new
-//! file is covered without being listed, and refuses each false sentence.
+//! D-1706 refused those exact sentences in the files directly under
+//! `crates/cli/src`, and others said the same thing in other words: a comment
+//! in `cli::append_condition_names` ("§5 does not give `cli` a `vocab` arrow"),
+//! the doc of `runner::report::names_from_words` ("`vocab` is not among
+//! them"), and the head of `crates/cli/Cargo.toml` ("NO ARROW TO `store`").
+//! So this walks every `.rs` file and every `Cargo.toml` under `crates/`,
+//! derives `cli`'s nine arrows from its own manifest, and refuses any comment
+//! clause that names one of them beside a denial and beside `cli` (or, inside
+//! `crates/cli`, with no crate named before the denial).
+//!
+//! Limits, stated: a denial written inside double quotes is read as a citation
+//! of old text and skipped, and only whole-line `//` and `#` comment prose is
+//! read by the general detector; the exact sentences are refused anywhere in a
+//! file, string literals included.
 
 #![allow(
     clippy::expect_used,
@@ -20,7 +33,7 @@
               its own crate root, so the attribute cannot be inherited."
 )]
 
-use std::path::Path;
+use std::path::{Path, PathBuf};
 
 /// The false sentences, each as it was written at 1087e544.
 const FALSE_CLAIMS: [&str; 6] = [
@@ -32,38 +45,292 @@ const FALSE_CLAIMS: [&str; 6] = [
     "clippy::default_trait_access",
 ];
 
-/// Every `.rs` file directly under `crates/cli/src`, with its text.
-fn sources() -> Vec<(String, String)> {
-    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
-    let mut out = Vec::new();
-    for entry in std::fs::read_dir(&dir).expect("crates/cli/src is readable") {
-        let path = entry.expect("a directory entry").path();
-        if path.extension().is_some_and(|e| e == "rs") {
-            let text = std::fs::read_to_string(&path).expect("a source file is UTF-8");
-            out.push((path.display().to_string(), text));
+/// Words that deny an arrow, compared lowercase.
+const DENIALS: [&str; 14] = [
+    "not among",
+    "no arrow",
+    "does not give",
+    "does not draw",
+    "does not depend",
+    "doesn't depend",
+    "never depends",
+    "has no `",
+    "lacks",
+    "is not a dependency",
+    "not one of its",
+    "no dependency on",
+    "would add a dependency arrow",
+    "would add an arrow",
+];
+
+/// What ends one clause and starts the next.
+const BOUNDARIES: [&str; 11] = [
+    ". ", "; ", "? ", "! ", ": ", " -- ", " — ", ", so ", ", but ", " (", ") ",
+];
+
+/// The repository root: `crates/cli` is two levels below it.
+fn repo() -> PathBuf {
+    Path::new(env!("CARGO_MANIFEST_DIR"))
+        .parent()
+        .and_then(Path::parent)
+        .expect("crates/cli is two levels below the repository root")
+        .to_owned()
+}
+
+/// Every `.rs` file and every `Cargo.toml` under `dir`, recursively.
+fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
+    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
+        .unwrap_or_else(|why| panic!("{}: {why}", dir.display()))
+        .map(|entry| entry.expect("a directory entry").path())
+        .collect();
+    paths.sort();
+    for path in paths {
+        if path.is_dir() {
+            sources(&path, out);
+        } else if path.extension().is_some_and(|e| e == "rs")
+            || path.file_name().is_some_and(|n| n == "Cargo.toml")
+        {
+            out.push(path);
+        }
+    }
+}
+
+/// The crates `cli` depends on, as the `path = "../X"` keys of its
+/// `[dependencies]` table name their directories.
+fn cli_arrows() -> Vec<String> {
+    let manifest = include_str!("../Cargo.toml");
+    let deps = manifest
+        .split("\n[dependencies]")
+        .nth(1)
+        .and_then(|rest| rest.split("\n[").next())
+        .expect("a [dependencies] table");
+    deps.lines()
+        .filter(|line| !line.trim_start().starts_with('#'))
+        .filter_map(|line| line.split("path = \"../").nth(1))
+        .filter_map(|rest| rest.split('"').next())
+        .map(str::to_owned)
+        .collect()
+}
+
+/// The comment prose of a file: each whole-line comment's marker removed,
+/// consecutive comment lines joined, any other line ending the block, `**`
+/// dropped and every double-quoted span removed as a citation.
+fn comment_prose(text: &str, toml: bool) -> String {
+    let mut prose = String::with_capacity(text.len() / 2);
+    for line in text.lines() {
+        let line = line.trim_start();
+        let comment = if toml {
+            line.strip_prefix('#')
+        } else {
+            line.strip_prefix("//")
+                .map(|rest| rest.trim_start_matches(['/', '!']))
+        };
+        if let Some(words) = comment {
+            prose.push(' ');
+            prose.push_str(words.trim());
+        } else {
+            prose.push_str(" . ");
+        }
+    }
+    let mut kept = String::with_capacity(prose.len());
+    let mut quoted = false;
+    for c in prose.replace("**", "").chars() {
+        if c == '"' {
+            quoted = !quoted;
+        } else if !quoted {
+            kept.push(c);
+        }
+    }
+    kept.split_whitespace().collect::<Vec<_>>().join(" ")
+}
+
+/// Where `clause` first names crate `name`, in any spelling comments use.
+fn first_mention(clause: &str, name: &str) -> Option<usize> {
+    let mut forms = vec![
+        format!("`{name}`"),
+        format!("`{name}::"),
+        format!("`crates/{name}"),
+    ];
+    if name == "core" {
+        forms.push("`brutex_core".to_owned());
+    }
+    forms
+        .iter()
+        .filter_map(|form| clause.find(form.as_str()))
+        .min()
+}
+
+/// Each clause of `prose` that denies `cli` one of `arrows`. `crates` names
+/// every workspace crate. `in_cli` says the file is `cli`'s own, where a
+/// denial with no crate named before it has `cli` as its subject.
+fn denials(prose: &str, in_cli: bool, arrows: &[String], crates: &[String]) -> Vec<String> {
+    let mut clauses = vec![prose.to_owned()];
+    for boundary in BOUNDARIES {
+        clauses = clauses
+            .iter()
+            .flat_map(|clause| clause.split(boundary).map(str::to_owned))
+            .collect();
+    }
+    let mut found = Vec::new();
+    for clause in clauses {
+        let lower = clause.to_lowercase();
+        let Some(denial) = DENIALS.iter().filter_map(|d| lower.find(d)).min() else {
+            continue;
+        };
+        if !arrows
+            .iter()
+            .any(|arrow| first_mention(&clause, arrow).is_some())
+        {
+            continue;
         }
+        let before: Vec<&String> = crates
+            .iter()
+            .filter(|name| first_mention(&clause, name).is_some_and(|at| at < denial))
+            .collect();
+        let about_cli = if first_mention(&clause, "cli").is_some() {
+            before
+                .iter()
+                .all(|name| *name == "cli" || arrows.contains(name))
+        } else {
+            in_cli && before.is_empty()
+        };
+        if about_cli {
+            found.push(clause);
+        }
+    }
+    found
+}
+
+/// The thirteen workspace crates, as `crates/` names their directories.
+fn workspace_crates() -> Vec<String> {
+    let mut crates: Vec<String> = std::fs::read_dir(repo().join("crates"))
+        .expect("crates/ is readable")
+        .map(|entry| entry.expect("a directory entry").file_name())
+        .filter_map(|name| name.to_str().map(str::to_owned))
+        .collect();
+    crates.sort();
+    crates
+}
+
+/// The detector flags the sentences that were false and passes the ones that
+/// are true, so the walk below cannot pass by detecting nothing.
+#[test]
+fn the_arrow_denial_detector_tells_a_false_sentence_from_a_true_one() {
+    let arrows = cli_arrows();
+    let crates = workspace_crates();
+    let flagged = |text: &str, in_cli: bool, toml: bool| {
+        denials(&comment_prose(text, toml), in_cli, &arrows, &crates).len()
+    };
+    for false_sentence in [
+        "// see its own\n// comment -- `CLAUDE.md` §5 does not give `cli` a `vocab` arrow.\n",
+        "/// `CLAUDE.md` §5\n/// lists `cli`'s arrows and `vocab` is **not among them** -- adding\n",
+        "//! `cli` does not depend on `pull`.\n",
+        "// `telemetry` is not a dependency of `cli`.\n",
+        "/// `cli` has no `costs` arrow.\n",
+        "/// `crates/cli` never depends on `brutex_core` itself.\n",
+    ] {
+        assert_eq!(flagged(false_sentence, false, false), 1, "{false_sentence}");
+    }
+    assert_eq!(
+        flagged(
+            "# NO ARROW TO `store`, AND THAT IS THE POINT. The rule\n",
+            true,
+            true
+        ),
+        1,
+        "inside cli's own manifest the subject is cli"
+    );
+    assert_eq!(
+        flagged(
+            "# NO ARROW TO `store`, AND THAT IS THE POINT. The rule\n",
+            false,
+            true
+        ),
+        0,
+        "outside cli's own files a subjectless denial is about some other crate"
+    );
+    for true_sentence in [
+        "// `CLAUDE.md` §5 says `cli` is what makes the sweep reachable at all — `api` does \
+         not depend on `runner`, `engine` or `indicators`, so the binary\n",
+        "/// Every crate that takes a lock — `store`, `pull`, `api` and `cli` — already \
+         depends on this one, so the guard adds no arrow to the crate graph.\n",
+        "// `cli` does hold the `vocab` arrow (D-0683); the call is not a workaround.\n",
+        "// `runner` does not depend on `store`.\n",
+        "// `cli` does not depend on `greeks`.\n",
+    ] {
+        assert_eq!(flagged(true_sentence, false, false), 0, "{true_sentence}");
+    }
+    for in_cli_history in [
+        "# `crates/api` has `store` and no `runner`; `crates/cli` had `runner` and no\n# `store`.\n",
+        "# That entry read: \"NO ARROW TO `store`: the operator's standing rule\n# forbids\"\n",
+    ] {
+        assert_eq!(flagged(in_cli_history, true, true), 0, "{in_cli_history}");
     }
-    out
+    assert_eq!(
+        flagged("let x = 1; // `cli` lacks `vocab`\n", false, false),
+        0,
+        "a trailing comment on a code line is not read"
+    );
 }
 
-/// No `cli` source repeats a sentence denying the `vocab` arrow, and no mask
-/// literal keeps the lint suppression that sentence justified.
+/// No workspace source repeats a sentence denying an arrow `cli` has, and no
+/// mask literal keeps the lint suppression the oldest of them justified.
 #[test]
 fn no_cli_source_denies_the_vocab_arrow_section_5_draws() {
-    let files = sources();
-    // A walk that found nothing would pass vacuously; lib.rs must be there.
+    let repo = repo();
+    let arrows = cli_arrows();
+    assert_eq!(
+        arrows.len(),
+        9,
+        "premise: CLAUDE.md §5 draws nine arrows for cli: {arrows:?}"
+    );
+    assert!(arrows.iter().any(|a| a == "vocab") && arrows.iter().any(|a| a == "store"));
+    let crates = workspace_crates();
+    assert_eq!(crates.len(), 13, "premise: thirteen crates: {crates:?}");
+    let mut files = Vec::new();
+    sources(&repo.join("crates"), &mut files);
     assert!(
-        files.iter().any(|(path, _)| path.ends_with("lib.rs")),
-        "the walk did not reach crates/cli/src/lib.rs"
+        files.len() > 300
+            && files
+                .iter()
+                .any(|p| p.ends_with("crates/runner/src/report.rs"))
+            && files.iter().any(|p| p.ends_with("crates/cli/Cargo.toml"))
+            && files.iter().any(|p| p.ends_with("crates/cli/src/lib.rs")),
+        "premise: the walk reached every crate ({} files)",
+        files.len()
     );
-    for (path, text) in &files {
+    let this_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/crate_graph_claims.rs");
+    let cli_dir = repo.join("crates/cli");
+    let mut false_claims = Vec::new();
+    for path in files.iter().filter(|path| **path != this_file) {
+        let text = std::fs::read_to_string(path)
+            .unwrap_or_else(|why| panic!("{} is UTF-8: {why}", path.display()));
         for claim in FALSE_CLAIMS {
-            assert!(
-                !text.contains(claim),
-                "{path} still says {claim:?}; cli depends on vocab directly (D-0683, D-1706)"
-            );
+            // The lint is refused only in `cli`, whose mask literals it covered:
+            // `api` truly has no `indicators` or `runner` arrow, so its reasons
+            // for the same suppression are true.
+            let scoped = claim == "clippy::default_trait_access" && !path.starts_with(&cli_dir);
+            if !scoped && text.contains(claim) {
+                false_claims.push(format!("{}: {claim:?}", path.display()));
+            }
+        }
+        let toml = path.extension().is_some_and(|e| e == "toml");
+        for clause in denials(
+            &comment_prose(&text, toml),
+            path.starts_with(&cli_dir),
+            &arrows,
+            &crates,
+        ) {
+            false_claims.push(format!("{}: {clause:?}", path.display()));
         }
     }
+    assert!(
+        false_claims.is_empty(),
+        "{} sentence(s) deny an arrow cli has; cli depends on {arrows:?} (D-0683, \
+         D-1706, D-4736):\n{}",
+        false_claims.len(),
+        false_claims.join("\n")
+    );
 }
 
 /// The manifest fact the sentences denied, checked rather than asserted.
diff --git a/crates/runner/src/report.rs b/crates/runner/src/report.rs
index 7c0afa98..e339ab92 100644
--- a/crates/runner/src/report.rs
+++ b/crates/runner/src/report.rs
@@ -1039,17 +1039,16 @@ pub fn render_auto(auto: &Auto, id: Option<&RunId>) -> String {
 
 /// The same names, from the six raw words a stored row carries.
 ///
-/// # Why `cli` cannot call `condition_names` directly
+/// # Why a caller passes words rather than a `ConditionMask`
 ///
-/// It takes a `&ConditionMask`, and that type lives in `vocab`. `CLAUDE.md` §5
-/// lists `cli`'s arrows and `vocab` is **not among them** -- adding one so a
-/// listing could print a name would be the silent scope change §3 rule 2
-/// forbids, and it would be invisible in review because `Cargo.toml` is the
-/// only file that changes.
+/// [`condition_names`] takes a `&ConditionMask`, and a stored row carries the
+/// six raw words. This makes the mask, so every listing that calls it decodes
+/// a stored row the same way, and `cli` passes the `[u64; WORDS]` it read off
+/// disk.
 ///
-/// So the conversion lives here, in a crate that already holds the arrow
-/// legitimately, and `cli` passes the `[u64; WORDS]` it read off disk. The
-/// caller needs no vocabulary type at all.
+/// This section used to say `cli` had no `vocab` arrow and could not call
+/// [`condition_names`] itself. `cli` has declared `vocab` since D-0683 and
+/// `CLAUDE.md` §5 draws it; the old reason is retired by G1-4 (D-4736).
 ///
 /// # Why the ledger stores WORDS and not NAMES
 ///
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 89a3b3bd..840f4979 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7060,3 +7060,5 @@ old line regex the same input and watched it pass.
 | L1FC-05 | No `.rs` file under `crates/`, read as prose with comment markers removed, says NSE trades only Monday to Friday (G5-3, D-4734) | `no_weekday_comment_says_nse_never_trades_on_a_weekend` in `crates/vocab/tests/stale_claims.rs` | ✓ |
 | L1FC-06 | The edge-blind interior minute census has exactly one release caller, `days_with_minute_holes` (G5-4, D-4735) | `cli::minute_gaps::tests::only_the_overlay_census_calls_the_interior_census` | ✓ |
 | L1FC-07 | A session that stops before its last demanded closing minute is withheld by the overlay census, although the interior census finds no hole (G5-4, D-4735) | `cli::minute_gaps::tests::a_session_that_stops_early_is_withheld` | ✓ |
+| L1FC-08 | The arrow-denial detector flags each false denial of a `cli` arrow and passes true sentences about other crates, past tense history, quoted citations and trailing code comments (G1-4, D-4736) | `the_arrow_denial_detector_tells_a_false_sentence_from_a_true_one` in `crates/cli/tests/crate_graph_claims.rs` | ✓ |
+| L1FC-09 | No `.rs` file or `Cargo.toml` under `crates/` denies `cli` one of the nine arrows its manifest declares (G1-4, D-4736) | `no_cli_source_denies_the_vocab_arrow_section_5_draws` in `crates/cli/tests/crate_graph_claims.rs` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index b7786567..3239ed82 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65140,3 +65140,34 @@ nothing where the overlay census withholds the day. `docs/06-limits.md` now
 says O(signal + minutes + d).
 
 Invariants L1FC-06, L1FC-07.
+
+### D-4736 — No comment in `crates/` may deny `cli` one of its nine arrows — 2026-10-09
+
+**Finding.** G1-4. These comments denied arrows that `cli` has:
+- a comment in `cli::append_condition_names`: "`CLAUDE.md` §5 does not give
+  `cli` a `vocab` arrow";
+- the doc of `runner::report::names_from_words`: "`vocab` is **not among
+  them**";
+- the head of `crates/cli/Cargo.toml`: "ONE DEPENDENCY, AND IT IS THE JOIN
+  CRATE" and "NO ARROW TO `store`".
+
+`cli` declares all nine arrows (D-0683, D-0208). D-1706's test read only the
+files directly under `crates/cli/src`, and only six exact sentences.
+
+**The decision.** Each sentence is corrected. The manifest's head is now
+history that cites D-0169 and D-0208. `crate_graph_claims.rs` now walks every
+`.rs` file and `Cargo.toml` under `crates/`. It reads `cli`'s arrows from the
+`path = "../X"` keys of `cli`'s own manifest, and the premise requires nine.
+It flags a comment clause when the clause has:
+- a denial word;
+- one of those arrows;
+- either `cli` with no other crate named before the denial, or, inside
+  `crates/cli`, no crate named before the denial at all.
+
+Quoted text is a citation and is skipped. A separate test pins the detector
+against five false sentences and six true ones. The exact lint-name check
+stays limited to `cli`, because `api` has no `indicators` or `runner` arrow
+and so its reasons for the same suppression are true. Run against the
+fbdabaec tree, the widened test named exactly the three sentences above.
+
+Invariants L1FC-08, L1FC-09.
-- 
2.43.0


From e7edaf5f209b4f90cdf547978d2710eeecba1c14 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:03:53 +0000
Subject: [PATCH 6/9] runner: read caller files past their first test module,
 and name the bottom-half rate's caller (G3-3)

Before: unwired_validation_record.rs cut every caller file at its first
#[cfg(test)]. cli/src/lib.rs declares its first test module on line 57,
so the scan read 56 lines of it. pbo.rs still said "No production caller
(D-1544)" for anchored_walk_forward_bottom_half_rate_v1, which D-1724
wired into cli::overfitting_of. With the whole file read, the test failed:
"crates/cli/src/lib.rs names anchored_walk_forward_bottom_half_rate_v1:
it is wired now".

After: the scan removes only the #[cfg(test)] items, and its premise
requires more than 10,000 lines of lib.rs. The pbo doc names its caller.
The test asserts that the doc says so and that overfitting_of's release
body makes the call. D-4737, L1FC-10.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/runner/src/pbo.rs                      |   9 +-
 .../runner/tests/unwired_validation_record.rs | 112 +++++++++++++++---
 docs/04-invariants.md                         |   1 +
 docs/05-decisions.md                          |  18 +++
 4 files changed, 121 insertions(+), 19 deletions(-)

diff --git a/crates/runner/src/pbo.rs b/crates/runner/src/pbo.rs
index d91475b9..9770985a 100644
--- a/crates/runner/src/pbo.rs
+++ b/crates/runner/src/pbo.rs
@@ -157,9 +157,12 @@ impl AnchoredWalkForwardBottomHalfRateV1 {
 /// One pass plus one sort of display projections: `O(folds log folds)` time
 /// and `O(folds)` temporary space. No bench row measures it yet.
 ///
-/// **No production caller (D-1544).** No `cli` verb or `api` route reaches it;
-/// only this module's tests call it. `docs/07-plan.md` names no surface for it,
-/// so wiring it would be a design this crate does not have.
+/// **Production caller: `cli`'s `overfitting_of` (D-1724).** The stored audit's
+/// overfitting row is this aggregate over each walk-forward fold's exact
+/// [`place_v1`] placement; it replaced the legacy [`place`] adapter there,
+/// which rounded an exact half-rank toward the better half. D-1544's "no
+/// production caller" held until D-1724 wired it, and is retired by G3-3
+/// (D-4737).
 #[must_use]
 pub fn anchored_walk_forward_bottom_half_rate_v1(
     placements: &[PlacementV1],
diff --git a/crates/runner/tests/unwired_validation_record.rs b/crates/runner/tests/unwired_validation_record.rs
index 4a7ebfb5..6d7488a1 100644
--- a/crates/runner/tests/unwired_validation_record.rs
+++ b/crates/runner/tests/unwired_validation_record.rs
@@ -1,9 +1,12 @@
-//! D-1544's record of five validation primitives with no production caller,
+//! D-1544's record of validation primitives with no production caller,
 //! checked against the source it describes (audit-20261003 gaps-3).
 //!
-//! Benjamini-Hochberg, the anchored walk-forward bottom-half rate, the V3
-//! projected walk-forward door and the two admission projections are built
-//! and tested, and no `cli` verb or `api` route reaches them. A reader of the
+//! Benjamini-Hochberg, the V3 projected walk-forward door and the two
+//! admission projections are built and tested, and no `cli` verb or `api`
+//! route reaches them. The anchored walk-forward bottom-half rate was the
+//! fifth until D-1724 wired it into the stored audit; this scan could not see
+//! that, because it read each file only up to its first `#[cfg(test)]`, and
+//! now asserts the wiring its doc names instead (G3-3, D-4737). A reader of the
 //! runner API would otherwise take FDR control and those doors to be
 //! available. Each one's own documentation now says so, and this test fails
 //! the moment either half moves: the sentence goes missing, or a `cli` or
@@ -68,6 +71,57 @@ fn doc_above(file: &str, signature: &str) -> String {
     lines.join("\n")
 }
 
+/// `source` with every `#[cfg(test)]` item removed: the attribute run, then
+/// the item it covers, which ends at its own `;` or at the first later line
+/// closing its brace at the same indentation (`cargo fmt --check` makes that
+/// exact). Everything else is kept, so a test module declared near the top of a
+/// file does not hide the production code below it (G3-3, D-4737).
+fn release_text(source: &str) -> String {
+    let lines: Vec<&str> = source.lines().collect();
+    let mut kept = String::with_capacity(source.len());
+    let mut at = 0_usize;
+    while let Some(line) = lines.get(at) {
+        let trimmed = line.trim_start();
+        if !(trimmed.starts_with("#[cfg(test") || trimmed.starts_with("#[cfg(all(test")) {
+            kept.push_str(line);
+            kept.push('\n');
+            at += 1;
+            continue;
+        }
+        let mut item = at + 1;
+        while let Some(next) = lines.get(item) {
+            let next = next.trim_start();
+            if next.starts_with("#[") {
+                while lines
+                    .get(item)
+                    .is_some_and(|line| !line.trim_end().ends_with(']'))
+                {
+                    item += 1;
+                }
+                item += 1;
+            } else if next.starts_with("//") {
+                item += 1;
+            } else {
+                break;
+            }
+        }
+        let Some(head) = lines.get(item) else { break };
+        at = item + 1;
+        if head.trim_end().ends_with('{') {
+            let indent = &head[..head.len() - head.trim_start().len()];
+            let close = format!("{indent}}}");
+            while let Some(body) = lines.get(at) {
+                at += 1;
+                let body = body.trim_end();
+                if body == close || body == format!("{close};") {
+                    break;
+                }
+            }
+        }
+    }
+    kept
+}
+
 #[test]
 fn every_unwired_validation_primitive_says_so_and_has_no_cli_or_api_caller() {
     let unwired = [
@@ -76,11 +130,6 @@ fn every_unwired_validation_primitive_says_so_and_has_no_cli_or_api_caller() {
             "pub fn benjamini_hochberg(",
             "benjamini_hochberg",
         ),
-        (
-            "crates/runner/src/pbo.rs",
-            "pub fn anchored_walk_forward_bottom_half_rate_v1(",
-            "anchored_walk_forward_bottom_half_rate_v1",
-        ),
         (
             "crates/runner/src/validate.rs",
             "pub fn walk_forward_projected_prepared_anchored_search_v3(",
@@ -105,16 +154,22 @@ fn every_unwired_validation_primitive_says_so_and_has_no_cli_or_api_caller() {
     let callers: Vec<(PathBuf, String)> = callers
         .into_iter()
         .map(|path| {
-            // Everything from the first test-only item on is a test, by this
-            // workspace's convention of a trailing `#[cfg(test)] mod tests`.
-            let text = read(&path);
-            let production = text
-                .split_once("\n#[cfg(test)]")
-                .map_or(text.as_str(), |(head, _)| head)
-                .to_owned();
+            // Only the test items are dropped. Cutting at the first
+            // `#[cfg(test)]` kept 56 lines of `cli/src/lib.rs`, whose first
+            // test module is declared on line 57, so the scan saw almost
+            // nothing of the file that calls the most (G3-3, D-4737).
+            let production = release_text(&read(&path));
             (path, production)
         })
         .collect();
+    let lib = callers
+        .iter()
+        .find(|(path, _)| path.ends_with("crates/cli/src/lib.rs"))
+        .map(|(_, text)| text.lines().count());
+    assert!(
+        lib.is_some_and(|lines| lines > 10_000),
+        "premise: cli/src/lib.rs is read past its first test module ({lib:?} lines)"
+    );
     for (home, signature, name) in unwired {
         let doc = doc_above(&read(&repo().join(home)), signature);
         assert!(
@@ -130,4 +185,29 @@ fn every_unwired_validation_primitive_says_so_and_has_no_cli_or_api_caller() {
             );
         }
     }
+
+    // The one D-1724 wired: its doc names the caller, and the caller exists in
+    // the release text and makes the call.
+    let doc = doc_above(
+        &read(&repo().join("crates/runner/src/pbo.rs")),
+        "pub fn anchored_walk_forward_bottom_half_rate_v1(",
+    );
+    assert!(
+        doc.contains("**Production caller: `cli`'s `overfitting_of` (D-1724).**")
+            && !doc.contains("No production caller"),
+        "the wired bottom-half rate's doc must name its caller:\n{doc}"
+    );
+    let lib = callers
+        .iter()
+        .find(|(path, _)| path.ends_with("crates/cli/src/lib.rs"))
+        .map(|(_, text)| text.as_str())
+        .unwrap_or_default();
+    let body = lib
+        .split_once("fn overfitting_of(")
+        .map(|(_, rest)| rest.split_once("\nfn ").map_or(rest, |(body, _)| body))
+        .expect("cli::overfitting_of exists in release code");
+    assert!(
+        body.contains("runner::pbo::anchored_walk_forward_bottom_half_rate_v1("),
+        "overfitting_of no longer calls the exact bottom-half rate"
+    );
 }
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 840f4979..25c06a9e 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7062,3 +7062,4 @@ old line regex the same input and watched it pass.
 | L1FC-07 | A session that stops before its last demanded closing minute is withheld by the overlay census, although the interior census finds no hole (G5-4, D-4735) | `cli::minute_gaps::tests::a_session_that_stops_early_is_withheld` | ✓ |
 | L1FC-08 | The arrow-denial detector flags each false denial of a `cli` arrow and passes true sentences about other crates, past tense history, quoted citations and trailing code comments (G1-4, D-4736) | `the_arrow_denial_detector_tells_a_false_sentence_from_a_true_one` in `crates/cli/tests/crate_graph_claims.rs` | ✓ |
 | L1FC-09 | No `.rs` file or `Cargo.toml` under `crates/` denies `cli` one of the nine arrows its manifest declares (G1-4, D-4736) | `no_cli_source_denies_the_vocab_arrow_section_5_draws` in `crates/cli/tests/crate_graph_claims.rs` | ✓ |
+| L1FC-10 | The unwired-primitive record reads each `cli` and `api` caller file with only its test items removed, and the wired bottom-half rate's doc names `cli::overfitting_of`, whose release body calls it (G3-3, D-4737) | `runner::unwired_validation_record::every_unwired_validation_primitive_says_so_and_has_no_cli_or_api_caller` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 3239ed82..b538a000 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65171,3 +65171,21 @@ and so its reasons for the same suppression are true. Run against the
 fbdabaec tree, the widened test named exactly the three sentences above.
 
 Invariants L1FC-08, L1FC-09.
+
+### D-4737 — The unwired-primitive record reads past test modules; the bottom-half rate's doc names its caller — 2026-10-09
+
+**Finding.** G3-3. `unwired_validation_record.rs` cut each caller file at its
+first `#[cfg(test)]`. `cli/src/lib.rs` declares its first test module on line
+57, so the scan read 56 lines of the file that makes the most calls. `pbo.rs`
+still said "**No production caller (D-1544).**" for
+`anchored_walk_forward_bottom_half_rate_v1`, which D-1724 wired into
+`cli::overfitting_of`. Once the scan read the whole file it failed: "crates/cli/src/lib.rs
+names anchored_walk_forward_bottom_half_rate_v1: it is wired now".
+
+**The decision.** The scan now removes only the `#[cfg(test)]` items, by
+brace and indentation, so the release text is complete. Its premise requires
+more than 10,000 lines of `lib.rs`. The doc names its caller, and the test
+asserts both halves: the doc says so, and `overfitting_of`'s release body
+makes the call. D-1544's sentence for the other four primitives still holds.
+
+Invariant L1FC-10.
-- 
2.43.0


From e5d77cb9b74cb6b77b6da5097491fd0e70894657 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:03:54 +0000
Subject: [PATCH 7/9] cli: make the GAP4-48 assertion message say what the
 daily filter does (G3-8)

Before: the assertion in daily_context_is_strictly_prior_parallel_and_explicitly_unverified
said "same-day and future daily bytes cannot enter the offered stream".
The next assertion proves that Tuesday's same-day record is offered:
D-1664's filter drops only the last signal day and later.

After: the message says that records on or after the last signal day are
not offered and that an earlier signal day's same-day record is. A test
refuses the old phrase anywhere in stored.rs. It failed before the edit
with "stored.rs says again that same-day daily records are not offered".
D-4738, L1FC-11.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/stored.rs | 22 +++++++++++++++++++++-
 docs/04-invariants.md    |  1 +
 docs/05-decisions.md     | 15 +++++++++++++++
 3 files changed, 37 insertions(+), 1 deletion(-)

diff --git a/crates/cli/src/stored.rs b/crates/cli/src/stored.rs
index 06143830..2d60ef81 100644
--- a/crates/cli/src/stored.rs
+++ b/crates/cli/src/stored.rs
@@ -4980,7 +4980,8 @@ mod tests {
             got.references
                 .iter()
                 .all(|reference| reference.ist_day() < OPEN_WEDNESDAY_2026_08_05),
-            "same-day and future daily bytes cannot enter the offered stream"
+            "daily records on or after the last signal day are not offered; an earlier \
+             signal day's same-day record is (GAP4-48, G3-8)"
         );
         assert_eq!(
             got.references.last().map(DailyReference::ist_day),
@@ -5001,6 +5002,25 @@ mod tests {
         assert_eq!(CHARTER_NON_REGULAR_IST_DAYS.len(), 9);
     }
 
+    /// G3-8 (D-4738): GAP4-48's false claim -- that the filter keeps a
+    /// same-day daily record out of the offered stream -- survived as the
+    /// message of an assertion whose next line proves Tuesday's same-day record
+    /// IS offered. No text in this file may say it again; the filter drops only
+    /// the last signal day and later, and per-row causality is
+    /// `AnchoredEvaluator::advance_before`'s. The phrase is assembled from two
+    /// literals so this test's own source does not match it.
+    #[test]
+    fn no_message_says_the_filter_withholds_same_day_daily_records() {
+        let stale = concat!(
+            "same-day and future daily bytes cannot ",
+            "enter the offered stream"
+        );
+        assert!(
+            !include_str!("stored.rs").contains(stale),
+            "stored.rs says again that same-day daily records are not offered"
+        );
+    }
+
     /// GAP4-48, D-1664: the offered daily stream is exactly what the signal
     /// days consume. Over a three-day daily span and two signal days, an
     /// earlier day's same-day record IS offered (the filter drops only the last
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 25c06a9e..0d3b61ff 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7063,3 +7063,4 @@ old line regex the same input and watched it pass.
 | L1FC-08 | The arrow-denial detector flags each false denial of a `cli` arrow and passes true sentences about other crates, past tense history, quoted citations and trailing code comments (G1-4, D-4736) | `the_arrow_denial_detector_tells_a_false_sentence_from_a_true_one` in `crates/cli/tests/crate_graph_claims.rs` | ✓ |
 | L1FC-09 | No `.rs` file or `Cargo.toml` under `crates/` denies `cli` one of the nine arrows its manifest declares (G1-4, D-4736) | `no_cli_source_denies_the_vocab_arrow_section_5_draws` in `crates/cli/tests/crate_graph_claims.rs` | ✓ |
 | L1FC-10 | The unwired-primitive record reads each `cli` and `api` caller file with only its test items removed, and the wired bottom-half rate's doc names `cli::overfitting_of`, whose release body calls it (G3-3, D-4737) | `runner::unwired_validation_record::every_unwired_validation_primitive_says_so_and_has_no_cli_or_api_caller` | ✓ |
+| L1FC-11 | No message in `stored.rs` says the daily filter withholds an earlier signal day's same-day record (G3-8, D-4738) | `cli::stored::tests::no_message_says_the_filter_withholds_same_day_daily_records` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index b538a000..b2b58d2f 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65189,3 +65189,18 @@ asserts both halves: the doc says so, and `overfitting_of`'s release body
 makes the call. D-1544's sentence for the other four primitives still holds.
 
 Invariant L1FC-10.
+
+### D-4738 — The GAP4-48 assertion's message says what the filter does — 2026-10-09
+
+**Finding.** G3-8. In `daily_context_is_strictly_prior_parallel_and_explicitly_unverified`,
+the assertion message read "same-day and future daily bytes cannot enter the
+offered stream". The assertion's next line proves that Tuesday's same-day
+record is offered. D-1664 says the filter limits the offered set and drops
+only the last signal day and later.
+
+**The decision.** The message now reads "daily records on or after the last
+signal day are not offered; an earlier signal day's same-day record is". A
+test refuses the old phrase anywhere in `stored.rs`. It failed before the edit
+with "stored.rs says again that same-day daily records are not offered".
+
+Invariant L1FC-11.
-- 
2.43.0


From f15f17bf8fa62c5a6a55178568f6e36385489b16 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:03:54 +0000
Subject: [PATCH 8/9] docs: name the per-fold rung resolver
 cli::walk_forward_rungs, correcting D-1660 (G3-5)

Before: D-1660 and its docs/06-limits.md section named cli::fold_rungs.
No cli function has that name. The resolver is cli::walk_forward_rungs,
and runner::validate::fold_rungs is the unrelated BRUTEX_GRID_RUNGS
reader.

After: the limits section names cli::walk_forward_rungs. A vocab
stale-claims test refuses the old name in that document and requires
both the real name there and fn walk_forward_rungs in cli/src/lib.rs. It
failed first with "docs/06-limits.md names cli::fold_rungs, which does
not exist". D-4739 corrects D-1660. L1FC-12.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/vocab/tests/stale_claims.rs | 21 +++++++++++++++++++++
 docs/04-invariants.md              |  1 +
 docs/05-decisions.md               | 15 +++++++++++++++
 docs/06-limits.md                  |  5 +++--
 4 files changed, 40 insertions(+), 2 deletions(-)

diff --git a/crates/vocab/tests/stale_claims.rs b/crates/vocab/tests/stale_claims.rs
index 26fde29b..f88a98f5 100644
--- a/crates/vocab/tests/stale_claims.rs
+++ b/crates/vocab/tests/stale_claims.rs
@@ -623,3 +623,24 @@ fn every_count_and_kind_the_documents_state_is_the_tables() {
     assert_eq!(next, COUNT, "the group table stops at {next} of {COUNT}");
 }
 
+/// G3-5 (D-4739): D-1660's limits section named `cli::fold_rungs`, a `cli`
+/// function that does not exist; the per-fold resolver is
+/// `cli::walk_forward_rungs`, and `runner::validate::fold_rungs` is the
+/// unrelated `BRUTEX_GRID_RUNGS` reader. The limits document names the real
+/// function, and the function is there under that name.
+#[test]
+fn the_limits_document_names_the_per_fold_rung_resolver_that_exists() {
+    let limits = read("docs/06-limits.md");
+    assert!(
+        !limits.contains("`cli::fold_rungs`"),
+        "docs/06-limits.md names cli::fold_rungs, which does not exist"
+    );
+    assert!(
+        limits.contains("`cli::walk_forward_rungs` hands each walk-forward fold"),
+        "docs/06-limits.md no longer names the per-fold resolver"
+    );
+    assert!(
+        read("crates/cli/src/lib.rs").contains("\nfn walk_forward_rungs() -> "),
+        "cli::walk_forward_rungs is gone; the limits document must follow it"
+    );
+}
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 0d3b61ff..78d77c37 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7064,3 +7064,4 @@ old line regex the same input and watched it pass.
 | L1FC-09 | No `.rs` file or `Cargo.toml` under `crates/` denies `cli` one of the nine arrows its manifest declares (G1-4, D-4736) | `no_cli_source_denies_the_vocab_arrow_section_5_draws` in `crates/cli/tests/crate_graph_claims.rs` | ✓ |
 | L1FC-10 | The unwired-primitive record reads each `cli` and `api` caller file with only its test items removed, and the wired bottom-half rate's doc names `cli::overfitting_of`, whose release body calls it (G3-3, D-4737) | `runner::unwired_validation_record::every_unwired_validation_primitive_says_so_and_has_no_cli_or_api_caller` | ✓ |
 | L1FC-11 | No message in `stored.rs` says the daily filter withholds an earlier signal day's same-day record (G3-8, D-4738) | `cli::stored::tests::no_message_says_the_filter_withholds_same_day_daily_records` | ✓ |
+| L1FC-12 | `docs/06-limits.md` names the per-fold rung resolver `cli::walk_forward_rungs`, which exists, and not the name D-1660 gave it, which no `cli` function has (G3-5, D-4739) | `the_limits_document_names_the_per_fold_rung_resolver_that_exists` in `crates/vocab/tests/stale_claims.rs` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index b2b58d2f..ed3d1ffb 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65204,3 +65204,18 @@ test refuses the old phrase anywhere in `stored.rs`. It failed before the edit
 with "stored.rs says again that same-day daily records are not offered".
 
 Invariant L1FC-11.
+
+### D-4739 — The per-fold rung resolver is `cli::walk_forward_rungs`; corrects D-1660 — 2026-10-09
+
+**Finding.** G3-5. D-1660 and the matching `docs/06-limits.md` section name
+`cli::fold_rungs`. No such `cli` function exists. The resolver is
+`cli::walk_forward_rungs`, and `runner::validate::fold_rungs` is the unrelated
+`BRUTEX_GRID_RUNGS` reader.
+
+**The decision.** `docs/06-limits.md` names `cli::walk_forward_rungs`. A
+`vocab` stale-claims test refuses `cli::fold_rungs` in that document and
+requires both the real name there and `fn walk_forward_rungs(` in
+`cli/src/lib.rs`. D-1660 stays as written; this entry corrects its function
+name.
+
+Invariant L1FC-12.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index ff7a34df..124ebb0f 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15472,8 +15472,9 @@ The rollback on a failed append is one `seek`, one `set_len` and one
 
 ## Walk-forward fold rung counts are derived per training window — D-1660, 3 October 2026
 
-`cli::fold_rungs` hands each walk-forward fold `grid_rungs` over its own
-training signal slice (GAP4-46). That is one `reference_price`, one
+`cli::walk_forward_rungs` hands each walk-forward fold `grid_rungs` over its
+own training signal slice (GAP4-46); this section once gave it a name no `cli`
+function has (G3-5, D-4739). That is one `reference_price`, one
 `grid_step_ppm` and one `max_stop_points` pass per fold, so **O(training
 bars) per fold and O(folds x span) per walk-forward shape**, beside the
 per-fold column build that already costs O(training bars). It runs on the
-- 
2.43.0


From 1010370ebed1dccaf091ae7e90f40af8d9250e17 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 05:03:55 +0000
Subject: [PATCH 9/9] web: accept a zero stop ceiling on the backtest descent
 as no ceiling (D-1732 follow-up)

Before: wholeNumber (n > 0) read the descent's stop ceiling, so the page
refused 0. Since D-1732 the route and cli read 0 as no ceiling. The
function's comment still quoted the server refusal D-1732 removed.

After: stopCeiling reads the ceiling. It matches digits first, as
wholeNumber does, but accepts 0. It feeds startDescent (max_points: 0)
and the derived control state. The list length keeps wholeNumber, so 0 is
still refused there. The summary says "no stop ceiling" for 0, and the
hint and refusal texts say that 0 means no ceiling.
web/tests/descent-ceiling.test.js evaluates both functions from the page
source and pins the call sites. It failed first with "the page defines
stopCeiling". D-4740, L1FC-13.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 docs/04-invariants.md                |  1 +
 docs/05-decisions.md                 | 18 +++++++
 web/src/routes/backtest/+page.svelte | 64 ++++++++++++++++-------
 web/tests/descent-ceiling.test.js    | 78 ++++++++++++++++++++++++++++
 4 files changed, 141 insertions(+), 20 deletions(-)
 create mode 100644 web/tests/descent-ceiling.test.js

diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 78d77c37..f36032ab 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7065,3 +7065,4 @@ old line regex the same input and watched it pass.
 | L1FC-10 | The unwired-primitive record reads each `cli` and `api` caller file with only its test items removed, and the wired bottom-half rate's doc names `cli::overfitting_of`, whose release body calls it (G3-3, D-4737) | `runner::unwired_validation_record::every_unwired_validation_primitive_says_so_and_has_no_cli_or_api_caller` | ✓ |
 | L1FC-11 | No message in `stored.rs` says the daily filter withholds an earlier signal day's same-day record (G3-8, D-4738) | `cli::stored::tests::no_message_says_the_filter_withholds_same_day_daily_records` | ✓ |
 | L1FC-12 | `docs/06-limits.md` names the per-fold rung resolver `cli::walk_forward_rungs`, which exists, and not the name D-1660 gave it, which no `cli` function has (G3-5, D-4739) | `the_limits_document_names_the_per_fold_rung_resolver_that_exists` in `crates/vocab/tests/stale_claims.rs` | ✓ |
+| L1FC-13 | The backtest page reads a stop ceiling of `0` as no ceiling and sends `max_points: 0`, still refuses a sign, decimal or exponent, and still refuses a list length of `0` (D-1732, D-4740) | `web/tests/descent-ceiling.test.js` · *a stop ceiling of 0 is read as no ceiling, and nonsense is still refused* | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index ed3d1ffb..bff1fa42 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65219,3 +65219,21 @@ requires both the real name there and `fn walk_forward_rungs(` in
 name.
 
 Invariant L1FC-12.
+
+### D-4740 — The browser reads a zero stop ceiling as no ceiling, as the route does — 2026-10-09
+
+**Finding.** D-1732, left open there. On `web/src/routes/backtest/+page.svelte`,
+`wholeNumber` (n > 0) read the descent's stop ceiling, so the page refused
+`0`. The route and `cli` read `0` as no ceiling. The function's comment also
+still quoted the server refusal that D-1732 removed.
+
+**The decision.** The ceiling has its own reading, `stopCeiling`. Like
+`wholeNumber`, it matches the digits first, but it accepts `0`. It feeds both
+`startDescent` (`max_points: 0`) and the derived control state. The list
+length keeps `wholeNumber`, so `0` is still refused there, as the route does.
+The summary line says "no stop ceiling" for `0`. The refusal and hint texts
+say that 0 means no ceiling. `web/tests/descent-ceiling.test.js` evaluates
+both functions from the page source and pins the call sites. It failed first
+with "the page defines stopCeiling".
+
+Invariant L1FC-13.
diff --git a/web/src/routes/backtest/+page.svelte b/web/src/routes/backtest/+page.svelte
index 2e618fd7..5f2fb483 100644
--- a/web/src/routes/backtest/+page.svelte
+++ b/web/src/routes/backtest/+page.svelte
@@ -2741,16 +2741,17 @@
    * text the operator did not type a number into, and the third turns a
    * fat-fingered ceiling into a plausible one.
    *
-   * ZERO IS REFUSED HERE AS WELL AS ON THE SERVER, and the duplication is
-   * deliberate rather than a second copy of a rule. `descent_from` is the
-   * authority — it refuses `max_points` at zero because "a ceiling of zero
-   * admits no trade and a negative one is not a distance", and `top` at zero
-   * because "a listing of no rows is not a shorter answer, it is no answer" —
-   * and those refusals still arrive if this is wrong. This exists so the
-   * control can name WHICH field is empty while the operator is looking at it,
-   * instead of after a round trip. It never widens what the server accepts and
-   * it never narrows it silently: anything this lets through is still judged
-   * there.
+   * ZERO IS REFUSED HERE AS WELL AS ON THE SERVER, for the one field that
+   * reads it: the list length. `descent_from` is the authority — it refuses
+   * `top` at zero because "a listing of no rows is not a shorter answer, it is
+   * no answer" — and that refusal still arrives if this is wrong. This exists
+   * so the control can name WHICH field is empty while the operator is looking
+   * at it, instead of after a round trip.
+   *
+   * THE STOP CEILING IS NOT READ HERE. D-1732 made the route read a zero
+   * ceiling as no ceiling, as `cli` does, and refuse only a negative one; this
+   * function refused zero for the ceiling as well, so the page could not ask
+   * for a run the route accepts. [`stopCeiling`] is its reading (D-4740).
    *
    * @param {string} text
    * @returns {number | null}
@@ -2762,6 +2763,26 @@
     return Number.isSafeInteger(n) && n > 0 ? n : null;
   }
 
+  /**
+   * A stop ceiling in whole index points, `0` meaning no ceiling, or `null`
+   * for anything else.
+   *
+   * ZERO IS A CEILING THE ROUTE ACCEPTS. `descent_from_wire` refuses only a
+   * negative `max_points` and passes zero to `cli`, where `Rules::admits` reads
+   * a zero ceiling as none (D-1732). The digits are matched first for the
+   * reason [`wholeNumber`] gives, so a sign, a decimal point or an exponent is
+   * still refused here and never reaches the run's identity (D-4740).
+   *
+   * @param {string} text
+   * @returns {number | null}
+   */
+  function stopCeiling(text) {
+    const digits = (text ?? '').trim();
+    if (!/^\d+$/.test(digits)) return null;
+    const n = Number(digits);
+    return Number.isSafeInteger(n) ? n : null;
+  }
+
   /**
    * What a descent sends over and above the feed, instrument and span the bar
    * above already holds.
@@ -2775,7 +2796,7 @@
    * deleted rather than given a sensible value.
    *
    * The three are STRINGS because they are what was typed. Parsing happens in
-   * `wholeNumber`, once, where the refusal can be named.
+   * `stopCeiling` and `wholeNumber`, once each, where the refusal can be named.
    */
   let descent = $state({ rung: '', points: '', top: '' });
 
@@ -2878,14 +2899,14 @@
       };
       return;
     }
-    const ceiling = wholeNumber(descent.points);
+    const ceiling = stopCeiling(descent.points);
     if (ceiling === null) {
       sweep = {
         phase: 'failed',
         run: null,
         why:
-          'The stop ceiling must be a whole number of INDEX POINTS above zero — the widest ' +
-          'adverse excursion this run may accept. It is never a ppm: the engine converts it ' +
+          'The stop ceiling must be a whole number of INDEX POINTS, 0 for no ceiling — the ' +
+          'widest adverse excursion this run may accept. It is never a ppm: the engine converts it ' +
           'against the midpoint of the span’s own bars, and that conversion is the reason ' +
           'this command exists.'
       };
@@ -3713,7 +3734,7 @@
    * missing BEFORE the press — the same reason `blocked` is read at page load
    * rather than discovered after a five-hour sweep refuses its append.
    */
-  const descentPoints = $derived(wholeNumber(descent.points));
+  const descentPoints = $derived(stopCeiling(descent.points));
   const descentRows = $derived(wholeNumber(descent.top));
 
   /**
@@ -7026,7 +7047,7 @@
           placeholder="whole number"
           bind:value={descent.points}
           aria-label="stop ceiling, in whole index points"
-          title="The widest adverse excursion this run may accept, in INDEX POINTS. Never a ppm and never paisa — the engine converts it against the midpoint of this span's own bars."
+          title="The widest adverse excursion this run may accept, in INDEX POINTS; 0 means no ceiling. Never a ppm and never paisa — the engine converts it against the midpoint of this span's own bars."
         />
         <span class="dnum-u">pts</span>
       </label>
@@ -7091,14 +7112,16 @@
         floor.
       {:else if descentStop === 'points'}
         <b class="warnish">The stop ceiling is not a whole number of points.</b> It is the widest
-        adverse excursion this run may accept, in <b>index points</b> — not a ppm and not paisa.
+        adverse excursion this run may accept, in <b>index points</b> — not a ppm and not paisa;
+        0 means no ceiling.
       {:else if descentStop === 'rows'}
         <b class="warnish">The list length is not a whole number of rows.</b> Zero is refused by the
         route: a listing of no rows is no answer.
       {:else}
         this press descends <b>{sweepSymbol}</b> on <b>{activeFeed}</b> at
-        <b>{descent.rung}</b>, walking the support threshold <b>down</b> from a ceiling of
-        <b>{exact(descentPoints ?? 0)} points</b> and listing the top
+        <b>{descent.rung}</b>, walking the support threshold <b>down</b>
+        {#if descentPoints === 0}with <b>no stop ceiling</b>{:else}from a ceiling of
+          <b>{exact(descentPoints ?? 0)} points</b>{/if} and listing the top
         <b>{exact(descentRows ?? 0)}</b>.
         <b>The Engine settings above are not sent with it</b> — `conduct_descent` applies no knobs,
         so spreading them here would let that panel imply it had configured a run it never touched.
@@ -14166,7 +14189,8 @@
      NOT `type="number"`. Its spinner adds a control nobody asked for, its
      silent coercion accepts `1e3` and `12.5`, and `valueAsNumber` on an
      unparseable value is `NaN` — three ways for a figure the operator did not
-     type to reach the run's identity. `wholeNumber` is the one reading. ---- */
+     type to reach the run's identity. `stopCeiling` and `wholeNumber` are the
+     two readings. ---- */
   .dnum {
     display: flex;
     align-items: center;
diff --git a/web/tests/descent-ceiling.test.js b/web/tests/descent-ceiling.test.js
new file mode 100644
index 00000000..75b5fafb
--- /dev/null
+++ b/web/tests/descent-ceiling.test.js
@@ -0,0 +1,78 @@
+import { readFileSync } from 'node:fs';
+import { test } from 'node:test';
+import assert from 'node:assert/strict';
+
+// D-4740. D-1732 made both api doors read a zero stop ceiling as no ceiling,
+// as `cli` does, and left the browser refusing it: `wholeNumber` (n > 0) read
+// the ceiling box, so the page could not ask for a run the route accepts, and
+// its comment still quoted the refusal D-1732 removed. The ceiling now has its
+// own reading, which accepts 0, while the row count keeps refusing it.
+
+const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
+
+/** The source of `function name(...) { ... }` in the page, by brace depth. */
+const functionSource = (name) => {
+  const start = page.indexOf(`function ${name}(`);
+  assert.notEqual(start, -1, `the page defines ${name}`);
+  let depth = 0;
+  for (let at = page.indexOf('{', start); at < page.length; at += 1) {
+    if (page[at] === '{') depth += 1;
+    if (page[at] === '}') {
+      depth -= 1;
+      if (depth === 0) return page.slice(start, at + 1);
+    }
+  }
+  assert.fail(`${name} never closes`);
+};
+
+/** The page's own function, evaluated as written. */
+const load = (name) => new Function(`${functionSource(name)}; return ${name};`)();
+
+/** The slice of `source` from `from` up to the next `to`. */
+const between = (source, from, to) => {
+  const start = source.indexOf(from);
+  assert.notEqual(start, -1, `missing start marker: ${from}`);
+  const end = source.indexOf(to, start);
+  assert.notEqual(end, -1, `missing end marker: ${to}`);
+  return source.slice(start, end);
+};
+
+test('a stop ceiling of 0 is read as no ceiling, and nonsense is still refused', () => {
+  const stopCeiling = load('stopCeiling');
+  assert.equal(stopCeiling('0'), 0);
+  assert.equal(stopCeiling(' 0 '), 0);
+  assert.equal(stopCeiling('120'), 120);
+  for (const wrong of ['', ' ', '-1', '-0', '1.5', '1e3', 'abc', '12abc', '9007199254740993']) {
+    assert.equal(stopCeiling(wrong), null, `${JSON.stringify(wrong)} is not a ceiling`);
+  }
+  assert.equal(stopCeiling(undefined), null);
+});
+
+test('the row count still refuses 0', () => {
+  const wholeNumber = load('wholeNumber');
+  assert.equal(wholeNumber('0'), null, 'a listing of no rows is no answer');
+  assert.equal(wholeNumber('25'), 25);
+  assert.equal(wholeNumber(''), null);
+  assert.equal(wholeNumber('-3'), null);
+});
+
+test('the descent reads the ceiling with stopCeiling and the rows with wholeNumber', () => {
+  const start = functionSource('startDescent');
+  assert.match(start, /const ceiling = stopCeiling\(descent\.points\);/);
+  assert.match(start, /const listRows = wholeNumber\(descent\.top\);/);
+  assert.match(start, /max_points: ceiling,/);
+  assert.doesNotMatch(start, /wholeNumber\(descent\.points\)/);
+  assert.match(page, /const descentPoints = \$derived\(stopCeiling\(descent\.points\)\);/);
+  assert.match(page, /const descentRows = \$derived\(wholeNumber\(descent\.top\)\);/);
+  assert.doesNotMatch(page, /wholeNumber\(descent\.points\)/);
+});
+
+test('no text on the page repeats the refusal D-1732 removed', () => {
+  assert.doesNotMatch(page, /a ceiling of zero\s+admits no trade/);
+  assert.doesNotMatch(page, /INDEX POINTS above zero/);
+  const doc = between(page, 'A whole number above zero', 'function wholeNumber(');
+  assert.match(doc, /D-1732/);
+  const summary = between(page, "{:else if descentStop === 'points'}", '{/if}');
+  assert.match(summary, /0 means no ceiling/);
+  assert.match(summary, /descentPoints === 0[\s\S]*?no stop ceiling/);
+});
-- 
2.43.0


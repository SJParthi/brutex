# conc-pass11: state under filesystem failure, at 1f4de71

**Verdict (head 1f4de71, final/all-fixes-zero, read-only checkout /home/claude/wt/zero3):** 3 new findings, all low: conc11-1, conc11-2 and conc11-3.

The durable writers hold up against this pass's failure list:
- Every positional write goes through `write_all`, `read_exact` or a loop that retries `Interrupted`, so EINTR and short writes are handled. No crate calls `libc`, `pwrite` or `write_at` in production.
- Every production `rename` is between siblings in one directory, so EXDEV cannot arise.
- Every `TryLockError::Error` (ENOLCK or ENOTSUP from a host without flock) is now named as the host's refusal rather than "another writer". The serve lock (D-1911), census lock (D-0955), store (`lock_fault`), recovery journal and candidate-trades `busy` all do this.
- The store's in-process `FAILED_BARRIERS` stops a failed fsync being confirmed by a second one on the bar path.

The defects found sit at the edges:
- **conc11-1:** two store-root ledgers never sync their directory after creating files. This is D-1903's rule, left unapplied.
- **conc11-2:** the autopilot files a disk I/O error, a quota or size limit, and the store's own `BarrierFailed` as a network blip.
- **conc11-3:** the api's accept loop goes silent on EMFILE.

**Method.** I read the source. No cargo was run on the workspace. I ran one standalone `rustc 1.97.0` program in my scratchpad to confirm how `std::io::ErrorKind` prints for each errno conc11-2 depends on.

I listed every production `OpenOptions`/`create`/`create_new`/`File::create`, `rename`, `hard_link`, `set_len`, `sync_all`/`sync_data`, directory sync helper, `try_lock`/`lock` and `custom_flags(O_NOFOLLOW)` site in `store`, `lake`, `pull`, `telemetry`, `cli` and `api`. I checked the list against the pass-3 full writer table (conc-pass3/ledgers.md), the pass-4 and pass-5 coverage tables, and triage-20261004.tsv.

Kernel facts K1 and K2 are used as in pass 3. Not re-reported: conc4-*, conc5-*, conc6-*, conc7-*, conc8-*, conc9-*, conc10-*, nor any pass-1/2/3 id.

## Coverage, by failure

| failure | sites examined | result |
|---|---|---|
| ENOSPC part-way through a write | store month (record region past `n_valid`, header slot, 4-byte positional sidecar), `fixed_tail` users, api pull journal (256 B records) and recovery journal (1 KiB records), telemetry sink, census append, cash-session, capture, Selection V6 quarantine | Fixed-stride records that divide 4 KiB cannot tear on ENOSPC, because block allocation is per page. Variable-length writes either roll back or newline-terminate (telemetry D-1539). Open ones are already known: cli2-1, conc4-1, pull1-3, pull2-3 |
| ENOSPC or EIO at a rename | census `publish`, masters `replace_locked`, `live::publish`, search checkpoint, telemetry roll | the temp file is removed, or the writer is poisoned, or `rotation_broken` is set. A post-rename directory-sync failure is `Landed::Uncertain` (masters). For the census it is xcut-2 (known) |
| EIO from fsync, retried as safe (fsyncgate) | store `barrier`, `fixed_tail::sync_or_roll_back`, api journals, census slots, telemetry `sync` | covered by known rows: conc4-2, conc5-2, ledgers-2, store1-2 and replay-3. Census is self-healing, because the next slot generation supersedes a lost one and its entry was synced first. **New: the autopilot's reaction to such a failure (conc11-2)** |
| read-only FS or EACCES at open | store `classify` (Denied/ReadOnly), autopilot probe, cli `install_log` (printed, D-documented), api journal | loud everywhere. Readers open read-only and never create (store `open_existing`, Candidate `open_read`) |
| EMFILE | `CensusLock::take` (pull2-1, known), resources-2 (known), **axum accept (conc11-3)** | |
| EINTR or short write | all `write_all`/`read_exact`. The store `read_fully` loop retries `Interrupted` (file.rs:3566). Blocking `flock` is restarted (no handler, or tokio's SA_RESTART) | clean |
| EXDEV | none: every temp is a sibling of its target (`.<file>.partial`, `<vendor>.man.writing`, `<hex>.<pid>.tmp`, `complete.writing`, `events.N.ndjson`) | clean |
| directory not fsynced after a create | store (`missing_below` plus `fsync_dir`, D-1522); Base V2, Population V5/V6, Admission V2-V4, Finalization V2-V4, Statistics V2/V3, Observation (D-1903), Execution V3/V4, Selection V5/V6, masters, cash-session, expression, recovery | **Candidate Universe V1 and Pre-Admission V1/V2: none (conc11-1).** Known gaps: press-2, ledgerv6-3, xcut-3, replay-4 |
| symlink or hard link planted at the target | Population, Admission, Finalization, Execution and lineage ledgers, `execution_lease`, checksum receipts and audit, and search checkpoint all use `O_NOFOLLOW` (and `nlink`). The store bar writer, census, masters `.partial`, `live` temp and autopilot probe follow links | The store states this limit (file.rs:67-72; also :6537). The others need write access to the store tree. Not counted |
| flock missing or a no-op (NFS, FUSE, exFAT) | every lock site | ENOLCK/ENOTSUP refuse by name. A cross-host no-op is a stated limit (file.rs:64-66, 02-store-format §9; removable root in 06-limits §155). Linux kernel exFAT and FUSE without `FUSE_FLOCK_LOCKS` give host-local flock, which is enough for one host. Hard links (expression publish) refuse loudly on exFAT (EPERM) |

## New findings

### conc11-1 (low): the store-root Candidate Universe V1 and Pre-Admission V1/V2 ledgers create their files and never sync the store-root directory. D-1903 made exactly this a rule for the Step-3 ledgers, and these two were left out

**Sites:**
- `crates/cli/src/candidate_universe.rs:3259` and `:3287-3288` create the lock, row and receipt files through `open_file`:
  ```rust
  let writer_lock = open_file(&lock_path, writable, writable)?;
  ...
  let mut row_file = open_file(&row_path, writable, writable)?;
  let mut receipt_file = open_file(&receipt_path, writable, writable)?;
  ```
  `open_file` (:6413-6420) is `OpenOptions::new().read(true).write(writable).create(create)`. `ensure_header` (:6132-6137) writes and `sync_data`s the header. Appends `sync_data` the files (:3777, :3796). The module has no directory sync at all.
- `crates/cli/src/pre_admission_data.rs` creates the same way: V1 at :1043 and :1070, V2 at :2328 and :2355, with `ensure_header` at :3517-3531 (V1) and :3760-3766 (V2). `grep -c sync_all` is 0 for the file.
- For contrast, Base Evidence V2, which sits beside them in the same store root, syncs it: `sync_directory(&self.root_file, &self.root)` at population_base_evidence_ledger_v2.rs:1189 and :1291.

**Why it is wrong:**
- D-1903 ("Files created on the first append get a directory barrier", 05-decisions) fixed the identical defect in Statistics V2/V3 and Observation V1/V2 (pop2-5, slice24-F2), and pinned it with ZL-09. Its reasoning applies word for word: "a power loss could lose the names of files whose contents were durable".
- POSIX `fsync`/`fdatasync` on a file does not make the new name in its parent durable.
- These are the store-root ledgers every `ledger-all` and `ledger-v6` run shares (conc10-1).
- If a run's universe is reported committed and the directory entry is then lost, the successor ledgers in ROOT bind a Candidate identity whose file no longer exists. Every later reopen fails "cannot open candidate file", including Population V6's fresh `open_read` re-authentication (step3_orchestrator.rs:693-715). A new run then starts a fresh ledger that does not hold the bound universe.
- Honest limit: conc-pass1/search.md declined this, because on ext4, XFS and btrfs an `fdatasync` of a new file usually commits its creating transaction too. The loss needs a filesystem without that property. The defect is the unapplied rule, not a measured loss.

**Repro (not run):** none short of a power cut, which is also why D-1903 pins its fix with a source-shape test. Evidence: `grep -n 'sync_directory\|File::open(.*root' crates/cli/src/candidate_universe.rs crates/cli/src/pre_admission_data.rs` finds nothing.

**Minimal fix:** open `root_file` in `candidate_ledger_paths` and in the Pre-Admission root check, as Base V2 does. Call `root_file.sync_all()` after `ensure_header` creates a header (len was 0) and on the receipt-last commit. Add both modules to ZL-09 with a source-shape test.

### conc11-2 (low): the autopilot files a disk I/O error, a quota or size limit, and the store's own `BarrierFailed` as a transport blip. A failing disk never halts the feed: the backfill moves on to later months on the same device, and a month refused by `BarrierFailed` is retried until its allowance is spent, although nothing can succeed before a restart

**Site:** `crates/api/src/autopilot.rs:353-394`, `classify`:
```rust
const STORE: [&str; 5] = [
    "disk full", "no space", "short write", "permission denied", "read-only filesystem",
];
...
if STORE.iter().any(|m| lower.contains(m)) { return Trouble::Store; }
Trouble::Transport
```
It is fed the store's `Display` verbatim:
- `pull/src/ingest.rs:2656`, `:2728` and `:2828`: `file.append(..).map_err(|why| why.to_string())`.
- That string reaches `outcome_of` (autopilot.rs:3608-3618, `f.why`) and then `observe` (:1401-1428).

The store renders an `io::Error` it does not name as `"{action} {path} failed: {kind:?} (errno {code:?})"` (store/src/file.rs `write_io`).

**Why it is wrong:**
- **ran:** a standalone rustc 1.97 program printing `io::Error::from_raw_os_error(n).kind()` gives these kinds, and none contains a STORE word:

  | errno | meaning | `ErrorKind` |
  |---|---|---|
  | 5 | EIO | `Uncategorized` |
  | 24 | EMFILE | `Uncategorized` |
  | 122 | EDQUOT | `QuotaExceeded` |
  | 27 | EFBIG | `FileTooLarge` |
  | 116 | ESTALE | `StaleNetworkFileHandle` |

  An fsync EIO on a month therefore reads `syncing /…/<month>.bin failed: Uncategorized (errno Some(5))` and is classed `Trouble::Transport`, "a timeout, a reset, a 5xx" (:335-337). EMFILE as transport is defensible. EIO, EDQUOT and EFBIG are not.
- Every later append to that month in this process is refused by `refuse_after_failed_barrier` with "a durability barrier on … already failed in this process; a second barrier cannot prove its bytes reached the device, so it takes no further append" (file.rs:3610-3617). That is also Transport.
- So the "same store refusal twice" halt never fires. The month takes `MAX_MONTH_ATTEMPTS` (3) backoff retries, each a vendor tick over the whole tracked set, and then `Next::Stall`. The feed then advances and keeps writing new months to the device that just returned EIO. The `Halt::Store` path and its write probe ("nothing further is FETCHED until the disk is dealt with", :1405-1410) exist for exactly this and are bypassed.
- The stall-retry doc (:1446-1450) reasons "Every stall this build can record is transport-class by construction, which is exactly the class where a later attempt is justified". For a `BarrierFailed` stall no later attempt in this process can succeed, because `FAILED_BARRIERS` is process-wide and never cleared. Every idle re-attempt spends vendor budget on a guaranteed refusal.
- The same shape breaks the halt for ENOSPC surfaced at fsync (thin-provisioned volumes, NFS, delayed allocation). The first refusal is `disk full syncing …`, which is Store, so `store_refused` is set. The retry gets `BarrierFailed`, which is Transport, so `store_refused` resets (:1414) and the two-in-a-row condition can never be met.
- The reason is shown verbatim, so the failure is not silent. The wrong *class* drives the wrong action. §4: "Degrade loudly and name the reason, or refuse". The reason is named, but the action is the retry ladder meant for a network fault.

**Repro (not run):**
1. Store root on dm-flakey, `api serve`, autopilot running.
2. Make writes fail during month M's append barrier. The tick's member failure reads `syncing …/M.bin failed: Uncategorized (errno Some(5))`, and the status shows a transport backoff, not a store halt.
3. Restore the device. The next ticks of M fail with the `BarrierFailed` sentence. After 3 attempts M is listed as stalled, the feed advances to M+1, and the idle path later re-asks the vendor for M although the process still refuses its barrier.

**Minimal fix:**
- Stop classifying store failures from prose. Carry a typed class from `StoreError` through the member failure: `DiskFull`/`Denied`/`ReadOnly`/`Io{kind: StorageFull|QuotaExceeded|FileTooLarge|Uncategorized}` map to Store.
- Make `BarrierFailed` its own restart-only halt, like `Halt::Credential`. A write probe must not revive it, because a fresh file syncing OK says nothing about the month this process can no longer confirm, and reviving it would loop halt, probe "CLEARED", refuse.
- At minimum, add `"durability barrier"`, `"(errno some(5))"`, `"quotaexceeded"` and `"filetoolarge"` to `STORE`, and let the store-halt probe skip revival when the reason is `BarrierFailed`.

### conc11-3 (low): when the api runs out of file descriptors, its accept loop goes silent. axum logs `accept error` through `tracing`, which this workspace never subscribes, so nothing reaches `/logs` or stderr while the server stops answering

**Site:** `crates/api/src/server.rs:17246-17259`, `LimitedListener::accept`, delegates to axum's own:
```rust
let (io, addr) = axum::serve::Listener::accept(&mut self.inner).await;
```
axum 0.8.9 (`Cargo.lock`, with default features, so `tracing` is on), `src/serve/listener.rs:140-158`:
```rust
async fn handle_accept_error(e: io::Error) {
    if is_connection_error(&e) { return; }
    error!("accept error: {e}");                       // tracing::error!
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

**Why it is wrong:**
- The workspace deliberately has no `tracing-subscriber` (D-0086, D-0087). `grep -rn tracing crates/api/src crates/cli/src crates/telemetry/src` finds nothing, so this event is discarded.
- On EMFILE or ENFILE (and ENOBUFS or ENOMEM), every accept fails. The listener sleeps one second per failure, for as long as the condition lasts, and no request is served. Not one line records why.
- The telemetry sink keeps its own descriptor open, so it *could* record the fault. The api's own `/logs` banner promise that a loss is visible on a page does not hold for the one failure that blanks every page.
- Concrete route: resources-2 (known: `/bars/window.json` holds 3 descriptors per month for up to 240 months). That is enough on its own to reach a 256 or 1024 soft limit, and then the server goes dark with no trace.
- This is distinct from resources-2 and pull2-1, which are about who consumes descriptors and the census lock's EMFILE arm. This finding is about the server's own reaction being invisible.

**Repro (not run):**
1. `ulimit -n 128; cli`-built `api serve` on a store with a long held series.
2. Issue a few concurrent `/bars/window.json` requests spanning 240 months, then load `/`.
3. The browser hangs until the window requests finish. Afterwards `/logs` shows no accept refusal and stderr is empty.

**Minimal fix:** in `LimitedListener::accept`, call `self.inner.accept()` (tokio) directly. On `Err`, emit one rate-limited telemetry `Error` event, for example `api.accept` / "accept refused" with errno, once per errno per minute. Then sleep and retry as axum does. Stop delegating to axum's tracing-only handler.

## Verification of known rows this theme touches (state at 1f4de71)

| id | verdict | evidence |
|---|---|---|
| pull2-1 | NOT FIXED | pull/src/ingest.rs:3129 `_ => return Ok(Self { _held: None })` for a regular-file lock on any other open error (EMFILE, ENOSPC) |
| store1-1 | NOT FIXED | store/src/file.rs:1592-1597: an absent `.lock` reads with `lock = None` |
| store1-2 | PARTIAL | file.rs:3620-3655: `FAILED_BARRIERS` is in-process only, so a later process trusts the slot |
| xcut-2 | NOT FIXED | ingest.rs:3356-3366: `rename`, then `File::open(dir)?.sync_all()` returns one `io::Error` after publish |
| xcut-3 | NOT FIXED | cli/src/candidate_trades.rs:322 `fs::create_dir_all(&directory)`, ancestors unsynced |
| ledgerv6-3 | NOT FIXED | cli/src/ledger_v6.rs:204 and :693 `create_dir_all`, no parent fsync |
| press-2 | NOT FIXED | api/src/audit.rs:1173-1176: `create_dir(audit/)` with no root fsync before the first record |
| replay-3 | NOT FIXED | cli/src/checksum_receipts.rs:306-311: full-length fast path, no sync |
| replay-4 | NOT FIXED | checksum_receipts.rs:376-381: root fsync only in the `Ok(create)` arm |
| pull1-3 | NOT FIXED | pull/src/capture.rs:247-256: final name, `create_new`, written in place |
| pull2-3 | NOT FIXED | pull/src/cash_session_cache.rs:500-507 and `write_new` :512-531 (receipt kept on a failed sync) |
| conc4-1 | NOT FIXED | cli/src/selection_v6.rs:392-410: offset-only quarantine name, `?` leaves a partial file |
| conc4-2 | NOT FIXED | cli/src/population_v5.rs has no `fixed_tail` call |
| conc5-1 | NOT FIXED | candidate_universe.rs:6129-6137 and pre_admission_data.rs:3523-3531: only `len == 0` re-initialises; no zero-header heal |
| conc5-2 | NOT FIXED | cli/src/selection_v5.rs:2106-2110: rows `sync_data` failure leaves rows, no `fixed_tail` |
| locks-1 (serve lock ENOLCK) | FIXED | api/src/server.rs:18341-18348: the `TryLockError::Error` arm names the host refusal (D-1911) |
| cli2-1 | cited, not re-verified | |

Not counted (stated limits, or no production caller):
- The store bar writer follows symlinks (file.rs:67-72).
- flock is a no-op across NFS hosts (file.rs:64-66).
- A same-name volume can be substituted (06-limits §155).
- `lake::LakeFile::open` reads the whole file with `fs::read`, and its I/O-error arm returns before the `lake.file` Error event. The `lake` crate has no dependent in the workspace, so it is not reported.

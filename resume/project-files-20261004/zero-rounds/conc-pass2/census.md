# conc-pass2 / census: the census writer, the api census reader and cache, store catalog, checksum audit and repair publish, at 331b05c

**Verdict: 1 new finding (0 high, 1 medium, 0 low). Every pass-1 finding in this slice is confirmed. One pass-1 "checked and clean" claim is refuted by census-1.**

Slice: `crates/pull/src/ingest.rs` census section (`from_members_inner` 768-950, `record_all` 1202-1226, `read_census` 2909-2950, `CensusLock` 2990-3200, `install_census`/`write_appends`/`install_locked`/`publish` 3272-3366), `crates/pull/src/manifest.rs` load path (`open`, `open_image`, `walk_generations`, `walk`, `validate`), `crates/api/src/census.rs` (`read_vendor`, `sized`, `read_all`), `crates/api/src/server.rs` census cache (`census_now*`, `manifest_stamps`, `read_as_stamped`, `stamp_could_read`, `refuse_contradicted_absences`, 3528-4110), the cache's consumers in `api/src/pullrun.rs` (`rows_now` and the ticker), `crates/store/src/{catalog,checksum_audit,repair}.rs`, and `store/src/file.rs` where those call into it (`open_or_create`, `open_existing`, `open_existing_audited`, `checksum_inputs`). Source reading only. No cargo was run.

---

## census-1 (medium): an in-place census append moves the manifest's mtime BEFORE its bytes land, so `census_now` can cache the pre-append census (or a torn, "degraded" one) under the post-append stamp and serve it until the next manifest write

**Where**
- Writer: `crates/pull/src/ingest.rs:3313-3331` (`write_appends`), the slot write at 3327:
  ```rust
  file.seek(SeekFrom::Start(append.commit.offset))?;
  file.write_all(&append.commit.bytes)?;
  file.sync_all()?;
  ```
- Cache: `crates/api/src/server.rs:3630` (`let stamps = stamp(&site.store_root);`), 3662 (`let mut censuses = read(&site.store_root);`), 3687-3707 (keep under `stamps`). The key is `manifest_stamps` (3774-3800): `ManifestStamp::At { modified, changed }` from one `stat`, with no size and no inode.
- The invariant the cache relies on, stated at 3656-3660: *"THE STAMPS ABOVE ARE OLDER THAN THIS READ, AND MUST BE. A manifest installed between the two keys a newer census under an older stamp, which the next request's stamp no longer matches, so it reads again: stale for at most that one request."*
- Reader: `crates/api/src/census.rs:419-455` (`sized`): one lock-free `read_to_end` of the whole manifest. It takes no `.man.lock`.

**Why it is wrong**

D-0695's ordering argument holds for the whole-image install: `publish` writes and syncs a temp, then `rename`s it (ingest.rs:3356-3366), so any stamp that shows the new inode's times was taken after that inode's bytes were complete. It does not hold for the incremental path, which has been the normal path since the 424 GB fix (`install_census` takes `append_locked` unless repairing, virgin or upgrading). `write_appends` rewrites a 64-byte header slot in place. On Linux the buffered-write path updates the inode's timestamps **before** it copies the data into the page cache: ext4 `ext4_buffered_write_iter` → `ext4_write_checks` → `file_modified()`, then `generic_perform_write()`, both under the inode lock. The generic `__generic_file_write_iter` has the same order. A buffered reader (`filemap_read`) on ext4, btrfs or tmpfs takes neither the inode lock nor the folio lock to copy an up-to-date page. So for the duration of the copy, `stat` already reports the final mtime and ctime while `read` still returns the old slot bytes, or a half-copied slot.

Nothing moves the times after that. `sync_all` does not, and the slot write is the last write of the install. So the cache keeps a census that is older than the stamp it is filed under, and `*at == stamps` matches on every later request. This is not the documented "two writes inside one timestamp tick" gap (docs/06-limits.md, D-0686/D-0695). It does not depend on timestamp resolution at all, and multigrain timestamps do not close it.

Two outcomes, both cached:
- The read copies the header page before the slot copy. The census is clean at generation g and is missing the month or rows the append just counted.
- The read overlaps the copy. The newest slot fails its CRC-32C, `walk_generations` steps over it, and the census is `Held` with `degraded = Some(..)`. `stamp_could_read(Held, At)` is `true` (server.rs:3880-3884), so the "degraded / needs attention" census is **kept**. pull2-5 said this alarm lasted "that one response". Through the cache it lasts until the manifest is next written.

The persistence matters because of who reads the cache:
- `/store.json`, `/instruments.json`, `/verify.json`, `/audit.json` and the store page.
- `calendar_of::cached`, keyed by `CensusStamps::modified` from the same stamps. It derives and keeps a calendar from the stale months, which feeds `/calendar.json` and the `/gaps.json` peer vote.
- `pullrun::rows_now` (pullrun.rs:509). The press's 5-second ticker (pullrun.rs:947-955) is the reader most likely to land in the window, because it polls **while the legs are appending**. The same cached value is then read for `after` (968) and for the run's final `current_rows` (1006-1012). A pass that added rows can read `after == before` and count as a clean empty pass (`CLEAN_EMPTY_PASSES = 3`), and the finished summary's `current_rows - started_rows` undercounts.

`from_window` installs once per window fetched (one append, one slot write, per vendor request), so a backfill makes thousands of these windows per run.

XFS takes the I/O lock shared for buffered reads and exclusive for buffered writes, so there the read waits for the copy and gets the new bytes. The defect is filesystem-dependent: present on ext4, btrfs and tmpfs, absent on XFS. The operator's filesystem is UNVERIFIED.

**Repro (one api process; ext4 store; one Dhan leg of a press)**
1. Ingest thread W holds `dhan.man.lock` and is in `write_appends` for its last append. Its `pwrite` of the 64-byte slot has run `file_modified()`, so the inode's mtime and ctime are now T, and it has not yet copied the bytes.
2. The pullrun ticker (or any `/store.json` poll) R calls `census_now_stamping`. `manifest_stamps` stats `dhan.man` and gets `At{T, ..}`, the cache key misses, and `read_all` → `sized` → `read_to_end` copies the header page before W's copy. The census is generation g, or degraded if the copy overlapped.
3. W finishes the copy and `sync_all`, then the run ends. No further write touches `dhan.man`.
4. R: `read_as_stamped` returns true, and the cache stores `(stamps{T}, census@g)`.
5. Every later request stamps `At{T}`, hits, and serves generation g (or "degraded"). This includes the press's final `rows_now`. It stays wrong until any vendor's manifest is written again, which can be the next day's pull.

**Minimal fix**

Make a time move **after** the bytes are in the page cache. In `write_appends`, after the final slot `sync_all`, call `file.set_modified(std::time::SystemTime::now())` (or `File::set_times`). This `setattr` moves both mtime and ctime after the copy, so any stamp taken inside the window differs from the final one, and the next request re-reads. The alternative is to have `census::sized` take `.man.lock` shared with `try_lock_shared`, and on `WouldBlock` serve the read uncached. That also closes pull2-5's torn read. But the ingest holds the lock for a whole run, so every request during a run would go uncached.

---

## Pass-1 verification (findings that touch this slice)

| Pass-1 item | Verdict | Reason, from the code at 331b05c |
|---|---|---|
| **pull2-1** (medium): `CensusLock::take` runs unlocked on transient or space errors | CONFIRMED | ingest.rs:3118-3130 is unchanged. Only `PermissionDenied` and `IsADirectory` refuse. Every other open error on a lock path whose `symlink_metadata` is a regular file (EMFILE, ENFILE, ENOMEM), or is `NotFound` (ENOSPC on inode creation while the census exists), reaches `_ => return Ok(Self { _held: None })` at 3129. Both `read_census` and `install_census` then run unserialised, and `install_locked`'s fixed temp `<vendor>.man.writing` (3345) is shared by `File::create` in two runs. |
| **pull2-2** (low): `record_all` drops a whole batch on one refused row | CONFIRMED | ingest.rs:1215-1219: `Err(why) => return Some(why)` returns before `install_census`, which discards every append already collected. The premise holds: `from_rows` writes bars outside the census lock, and the api's rolling path collects every group's `pending` and calls `record_held` once (server.rs:12819). |
| **pull2-5** (low): an in-place slot write lets a lock-free reader see a torn slot and report "degraded" | CONFIRMED, and understated | The write is at ingest.rs:3326-3328 and the reader at census.rs:419-455 takes no lock. The finding said the false alarm lasts one response. census-1 shows the cache keeps that degraded census under the post-write stamp, so it persists until the next manifest write. |
| **xcut-2** (low): census install says "not published" after the rename published | CONFIRMED, plus one unreported case | ingest.rs:3364-3365: an error after `fs::rename` still becomes "could not be published", and the caller (941-946) words it as "the census that counts them was not published". **Also:** `write_appends` loops over a batch and commits each append (slot write plus `sync_all`) before the next. If append *k* fails (for example ENOSPC growing the file for its entry), appends 1..k-1 are durable and published, yet `append_locked` reports "could not be appended to after {n} entry write(s)" and `from_members_inner` reports all `done.counted` slices as unpublished. The next run self-heals: `count` sees the published rows and re-appends only the rest. So the cost is a false receipt and an Error event, not lost data. |
| **store2-1** (low, latent): a concurrent identical `repair::publish` loses with a misleading refusal | CONFIRMED | repair.rs:261 `if exists(&reservation)?` is a check-then-act against the `create_new` at 353-361, which maps `AlreadyExists` to `Io{.., publication_may_be_visible: false}`. The Reused branch's `RevisionReader::open` reads the receipt (line 162) before it takes any lock, so a live publication reads as `Incomplete`. Still latent: grep finds no caller of `repair::publish` or `RevisionReader` outside `store/src/repair_tests.rs`, `store/tests/` and `flock_tests.rs`. |
| **store1-1** (low): `open_existing` reads with no lock when `.lock` is absent | CONFIRMED (not in this slice; checked only where the slice calls it) | file.rs:1398-1406 still answers `Err(why) if why.is_absent() => None`. The checksum-audit door is **not** exposed: `open_existing_audited` (file.rs:1433-1449) opens the lock with `open_regular` and has no absent-lock arm, and `checksum_inputs` refuses a file with no held lock. |
| apicache "checked clean": `census_now_stamping` "never serves a stale answer" | REFUTED | See census-1. The argument assumes a write moves the stamp after its bytes are readable. That is true for the rename install and false for `write_appends`. |
| apicache "checked clean": `census::sized` never sees a half-written manifest because the manifest is installed by temp+rename | STALE (already covered by pull2-5) | Since the incremental install, the common path is an in-place slot write, not a rename. |

---

## Checked and found sound (not findings)

- **Census RMW serialisation.** In both `from_members_inner` (805) and `record_all` (1204), the lock is taken before `read_census` and declared before the census so it drops last. `install_locked` and `append_locked` take `&CensusLock`, so neither can be reached without the guard. Intra-process exclusion also holds: every `take` is a separate `open`, and therefore a separate open file description, so two threads conflict on `flock`.
- **Lock ordering.** The spot path takes the census lock, then each month's bar lock. The rolling F&O path takes the bar lock, releases it, then takes the census lock. Every acquisition is `try_lock`, so there is no wait cycle. A collision is a named refusal (`lock_refusal`).
- **`write_appends` crash points.** Each append runs entry write → `sync_data` → slot write → `sync_all`. Dying before the slot leaves bytes past `n_valid`, which `walk` ignores (`zip(0..n_valid)` over `chunks_exact`) and the next append overwrites. Dying mid-slot leaves the other slot valid, so the next load comes back degraded and the next ingest repairs it by a whole-image install. An entry `sync_data` EIO leaves the counter unpublished, and the next run rewrites the same offset.
- **Lock-free reader versus append ordering.** `read_to_end` reads from offset 0, so the header is copied before the entries. Each entry is written and synced before the slot that counts it, so a reader that sees slot g+1 also sees entry N+1. The only tear is inside the slot itself (pull2-5 / census-1).
- **Whole-image `publish`.** It runs write → `sync_all` → `rename` → directory `sync_all`, with the fixed temp name serialised by the lock (except pull2-1). A temp left by a crash is truncated by the next `File::create` under the lock, and nothing lists `manifest/`.
- **`read_census`.** Only `NotFound`/`NotADirectory` give genesis. Other errors refuse. A file shorter than the header is refused, and only an empty file is genesis. No writer creates an empty `<vendor>.man` (only `rename` of a synced temp), so a crash cannot produce one.
- **Census cache install race.** A slow older reader overwriting a newer entry is re-read on the next request (stamps differ). The double-checked install keeps the canonical `Arc`. The mutex is read through poison, and nothing is held across the read.
- **`store::catalog::walk`.** It lists directories only and opens no file. Output is `sort_unstable` + `dedup`. A month created mid-walk is caught by `open_or_create`'s order: the lock is created and `flock`ed **before** the `.bin` is opened, so a reader of a freshly listed month gets `Locked`, not a zero-length header. `bar-revisions-v1/` sits outside `bars/`, so repair artefacts are never catalogued.
- **`checksum_audit`.** The shared month lock is held for the `AuditedBarFile`'s lifetime. Generations (dev, ino, len, mtime/ctime ns, nlink=1) for data, sidecar and lock are compared fd against path before and after the full scan and around every `read_record`. A cooperating writer's `open_or_create` opens the lock with `create(true)` and no truncate, which moves no time, and then fails `try_lock`.
- **`repair::publish` versus ingest.** The source is shared-locked through the whole revision write, so `open_or_create`'s exclusive `try_lock` refuses, and the reverse order refuses `Locked`. The revision directories are synced to the root before and after the receipt.

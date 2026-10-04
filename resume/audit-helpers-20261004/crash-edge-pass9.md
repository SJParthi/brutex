# Crash and edge-input audit, pass 9: on-disk bytes

Head: `/home/claude/wt/zero3` @ **1f4de71** (origin/final/all-fixes-zero). Audit only; nothing in any checkout was edited.
Theme: every reader of store and lake files, given corrupt or adversarial bytes, checked against `docs/02-store-format.md`.

## Verdict

**NOT ZERO. 1 medium, 2 low, plus 2 latent notes (no production caller).**
- The store header parser itself holds. Wrong magic, past and future versions, stride, flags, reserved bytes, CRC, slot position, a short header region, a counter ahead of the file and a counter behind a published slot are all refused by name.
- The gap is one level up. `n_valid` is bounded only by the **file length**, never by how many records the month and timeframe can hold. So a sparse file plus a CRC-valid header passes `open_existing`, and the api then sizes an allocation from that number and aborts.

## New findings

### CE-61 (medium): a sparse month with a CRC-valid huge counter aborts the api server

- **Where:** `crates/store/src/file.rs:1673-1834` (`BarFile::validated`), `crates/store/src/header.rs:422` (`Header::validate`).
  - The only bound on the counter is `if self.n_valid > layout.capacity_for(file_len) { return Err(FormatError::CounterExceedsFile) }`.
  - Nothing compares `n_valid` with the number of grid slots in the month. `Admission` (file.rs:3025) already holds the month bounds `from/until` and the slot width `width`.
  - The writer can never commit more than `(until-from)/width` records: 44,640 a month at 1 minute. The reader accepts up to `(len-32768)/56`.
- **Sinks that size or loop from the unchecked count:**
  - `crates/api/src/bars.rs:967`: `Vec::with_capacity(usize::try_from(total).unwrap_or(0))`. `total` is the sum of `n_valid` over the window (bars.rs:1098-1101). This runs for GET `/bars/window.json` with any non-ts sort or `extremes=1`.
  - `crates/api/src/server.rs:13275` (`read_month_bars`, reached from `price_chain_month` through `spot_book_for`): `Vec::with_capacity(usize::try_from(n).unwrap_or(0))`.
  - `crates/api/src/server.rs:3194`: `bars::page(&file, 0, n)` loops `n` times. A sealed file pushes one fault `String` per record and an unsealed v2 file pushes one zero `Bar` per record, so either way it grows until out of memory.
  - The release profile has `panic = "abort"`, and allocation failure aborts in any profile. The whole server dies.
- **Repro (ran)** in a scratch worktree at 1f4de71, with a throwaway test in `api::bars` window_tests:
  1. `write_month` 10 bars.
  2. Decode the newest slot and set `generation += 1` and `n_valid = (2^40-32768)/56`.
  3. `Header::commit()`, then pwrite the slot.
  4. `set_len(2^40)` (sparse).
  - `BarFile::open_existing` returned **Ok**. The test's `expect("STORE ACCEPTS")` passed.
  - `window(.., SortKey::Close, ..)` then died with **SIGABRT** in `alloc::alloc::handle_alloc_error ← raw_vec::handle_error ← api::bars::window`. The box has 15 GB of RAM, no swap, overcommit 0.
  - Worktree, target directory and sparse file removed afterwards.
- **Why it is wrong:**
  - The rule says allocation sized from a header must be validated.
  - CLAUDE.md §4 says refuse loudly, never crash.
  - The bound is free: O(1) arithmetic the store already has in `Admission`.
- **Threat:** it needs a CRC-valid slot, which a deliberate rewrite can produce. `docs/02` §2.1 already accepts that such an actor exists, but only as a downgrade risk, not as a process-abort risk.
- **Minimal fix:**
  - In `validated`, refuse `header.n_valid > admission.slots()` as a new `FormatError::CounterExceedsMonth`, where `slots = (until-from)/width` (daily: days in the month).
  - Optionally also refuse `first_ts/last_ts` outside `[from, until)` when `n_valid > 0`.
  - Then `bars.rs:967` and `server.rs:13275` should use `try_reserve_exact`, as `cli/src/stored.rs:2390` already does.
  - Add an invariant row next to S-07.

### CE-62 (low): the bar writer follows symlinks, while `docs/02-store-format.md:453` says it must not

- **Where:** `crates/store/src/file.rs:3443` `open_rw`: `File::options().read(true).write(true).create(true).truncate(false).open(path)`. It has no `O_NOFOLLOW` and no per-component `openat`. It is used by `open_or_create` for `.lock` and `.bin`. The sidecar is opened the same way at file.rs:~1940.
- **What the doc says:** §9 says *"The writer must open with `openat` + `O_NOFOLLOW` per component and halt naming the linked component."*
- **What the code does:** a symlinked `<month>.bin`, `.crc` or directory component is followed. An append writes through the link to wherever it points. Only the read door and the checksum audit use the flags in `store::open_flags`.
- **Repro:** not run. The reasoning is from source: there are no custom flags on `open_rw`, and `std` follows links.
- **Fix:** either open the three writer files with `crate::open_flags::O_NOFOLLOW_NONBLOCK` plus an `fstat`/`is_file` check, as `checksum_audit::open_regular` does, or rewrite the §9 row to state that the writer follows links and record the decision.

### CE-63 (low): a header slot's timestamps are never checked against the month on read

- **Where:** `crates/store/src/header.rs:422-438`. `validate` only checks `last_ts >= first_ts`.
- **What goes wrong:** a CRC-valid slot whose `last_ts_micros` lies years past the month opens fine. The writer then routes every in-month batch to `diagnose_overlap` (file.rs:2821/2895) and reports `OverlapDisagrees`/`NotHeld`. That message names the batch, not the header, so the operator is sent after the wrong cause. It is loud, but it names the wrong fault.
- **Repro:** not run.
- **Fix:** the same O(1) bounds check as CE-61's optional half, refused as a header fault.

### Latent (no production caller; not counted)

- `crates/lake/src/reader.rs:88` `LakeFile::open` = `fs::read(path)`:
  - A FIFO blocks forever.
  - A sparse or huge file is read whole, with no byte ceiling.
  - `grep` finds no crate outside `lake` that names `lake::`.
- `crates/store/src/repair.rs:431` (`read_receipt`) and `:446` (`shared_lock`) use a blocking `File::open`, so a FIFO hangs. That is the class D-1432 fixed for `open_existing`. `RevisionReader::open` is called only from `repair::publish`, and nothing outside `store` calls that.

## Sites checked and why each holds

| Input | Site | Result |
|---|---|---|
| file size 0 / 1 / 63 / 32767 | `read_header` file.rs:3275 reads `min(len, 32768)`; `best_candidate`/`committed` header.rs:821/642 | `NoValidHeader` or `HeaderRegionTooShort`, both logged by `note_header_unreadable` |
| wrong magic, family-prefix mismatch | `decode_parts` header.rs:469 | `NotABarFile`, `MagicVersionMismatch` |
| version 1, unknown or future (u16::MAX) | `Layout::for_version` layout.rs:303 | `RetiredVersion` / `UnknownVersion`, reported preferentially (`informativeness`) |
| stride ≠ 56, unknown flag bits, reserved ≠ 0 | decode_parts | `StrideMismatch`, `UnknownFlags`, `ReservedNotZero` |
| v3 with checksum flag cleared | decode_parts | `ChecksumsRequired(3)`; v2 downgrade is a documented limit (§2.1) |
| slot CRC wrong | decode_parts | `SlotChecksum`; falls back to the older slot, logged as a warning |
| slot at the wrong position | best_candidate | `SlotPositionMismatch` |
| counter past EOF | `Header::validate` | `CounterExceedsFile` |
| counter behind a published slot (truncation) | file.rs:1834 `claimed > header.n_valid` | `CounterExceedsFile` |
| bytes past the commit | `bytes_past_the_counter`, saturating | opens, `store.open` warn with the byte count (§7) |
| generation = u64::MAX | `advance` header.rs:312 | `GenerationExhausted` on the next append, no wrap |
| n_valid × 56 overflow | `capacity_for` bounds n_valid first; `offset_of` checked | no overflow reachable |
| symbol or timeframe mismatch | validated | `SymbolMismatch` / `TimeframeMismatch` |
| checksum wrong, sidecar short or missing | `verify_block_of` file.rs:2662 | `BlockChecksum` with block number / `ShortRead` / `ChecksumsMissing` (refused at read, not open) |
| tail sealed past the commit | `past_the_commit` bounded to one block (`MAX_BLOCK_LEN` stack buffer) | k+1 candidates, at most 72; documented |
| file truncated after open | `read_fully` | named short read |
| directory, FIFO or device at `.bin`/`.crc`/`.lock` (read) | `open_read` file.rs:6546 (O_NONBLOCK + fstat) | `IsADirectory` / `NotARegularFile` |
| symlink loop (read) | `open_read` follows → ELOOP | `classify` names the path |
| catalog: symlinks, loops, FIFOs, non-UTF-8 | catalog.rs `admit` (`file_type`, never followed) | counted in census buckets; reconciles |
| strict audit door | checksum_audit.rs: O_NOFOLLOW, nlink==1, exact data and sidecar extents, `max_bytes`, fixed 8 KiB buffer, `try_reserve_exact` | holds |
| cli stored loader | cli/src/stored.rs:2357 span ceiling + `try_reserve_exact` (2390) | named refusal, not an abort (on a 15 GB box) |
| api census manifest | census.rs `sized`/`open_without_waiting`, MAX_ENTRIES ceiling | holds |
| cli explicit catalog | boolean_catalog_command.rs:116 `take(max+1)` | holds |
| OHLC order, OI sentinel, timestamp order within a file | not checked on read; `docs/02` §9 puts value validation at ingest, and v3 checksums cover writer-sealed bytes | documented limit; not a finding |
| writable memory mapping (CLAUDE.md §4) | none: `forbid(unsafe_code)` in store/lake, no memmap dependency, `store/tests/docs.rs:75` gate | holds |
| lake footer: list lengths, depth, row counts | footer.rs `check` (CE-12/13 fix), reader.rs row bounds | holds (latent crate) |

Counts: new 3 (CE-61 medium, ran; CE-62 low; CE-63 low), latent notes 2. No verification rows were requested this pass.

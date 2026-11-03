# gaps-LS1.txt.md

BASE-COV-LIBS-LS1 (crate `store`): Route B figures and every uncovered line and branch side, written by the session from its own measurement. One fenced block.

`````text
FINAL: Route B (d3-route-b-cloud.sh, sha256 f032b525...) on tip fc9b489f0e3ca9207a3f1fe045d913adfa1429fa (the code tip), set 'reading whole:store' (113 .rs files: the 104 of START and 9 new test files), pinned rustc 1.99.0-nightly (1a98b1e13 2026-08-07) installed as nightly-2026-08-08, LLVM 23.1.0-rust-1.99.0-nightly, Linux x86_64 root host, profiles under /home/user (mode 755) so the nobody re-runs (D-0995) write theirs. Run 2026-11-03 17:06:02-17:34:14 IST (tests 25 min). BASELINE for comparison: tip c7ccb0f7f7e2 (START plus the seven root re-runs), 15:47:42-16:15:48 IST: lines 50,033/51,607 (1,574 uncovered), branches 3,291/3,868 (577 uncovered).
Classes: F? = a failing arm inside test code by pattern (panic!/unreachable!/assert macro condition or message) -- an (F) candidate of recipe step 4, each to be confirmed by hand (fu-LS1.txt.md holds the confirmed (F) and the proved (U) items); GAP = every other uncovered line or side.

ROUTE B LOG (result lines)
  ROUTE-B START 2026-11-03 17:06:02 IST tree /home/user/ls1-deep tip fc9b489f0e3ca9207a3f1fe045d913adfa1429fa packages store K /home/user/brutex/target P /home/user/ls1-prof/fc9b489f0e3c-1-B (entries before: 0)
  versions: rustc 1.99.0-nightly (1a98b1e13 2026-08-07) | llvm-cov  LLVM version 23.1.0-rust-1.99.0-nightly | llvm-profdata  LLVM version 23.1.0-rust-1.99.0-nightly
  == build START 2026-11-03 17:06:02 IST: cargo test -p store --locked --no-run --message-format=json (RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch', opt-level 0, CARGO_TARGET_DIR=/home/user/brutex/target/cov)
  == build rc=0 END 2026-11-03 17:06:40 IST (0 min); 45 artifacts, 0 reused
  objects: 42 (/home/user/ls1-prof/fc9b489f0e3c-1-B/objects.txt, from this build's own report)
  == test store START 2026-11-03 17:06:40 IST
  == test store rc=0 END 2026-11-03 17:32:38 IST (25 min); tests passed 1028 failed 0 ignored 8
  profraw files: 77
  merged /home/user/ls1-prof/fc9b489f0e3c-1-B/store/merged.profdata
  ROUTE-B NOT PROVED: crates/store/benches/ratio.rs: not read (<scratchpad>/routeb-logs/fc9b489f0e3c-cov/show-store-crates_store_benches_ratio.rs.err)
  ROUTE-B NOT PROVED: crates/store/benches/ratio/months.rs: not read (<scratchpad>/routeb-logs/fc9b489f0e3c-cov/show-store-crates_store_benches_ratio_months.rs.err)
  ROUTE-B NOT PROVED: crates/store/benches/ratio/quarantine.rs: not read (<scratchpad>/routeb-logs/fc9b489f0e3c-cov/show-store-crates_store_benches_ratio_quarantine.rs.err)
  ROUTE-B NOT PROVED: crates/store/benches/ratio/safe_files.rs: not read (<scratchpad>/routeb-logs/fc9b489f0e3c-cov/show-store-crates_store_benches_ratio_safe_files.rs.err)
  ROUTE-B NOT PROVED: crates/store/src/lib.rs: not read (<scratchpad>/routeb-logs/fc9b489f0e3c-cov/show-store-crates_store_src_lib.rs.err)
  ROUTE-B DONE 2026-11-03 17:34:14 IST WITH NOT PROVED lines above

TOTALS AND PER FILE (lines covered/counted, branches covered/counted; GAP and F? row counts)
  TOTAL store: lines 51094/52629 (uncovered 1535), branches 3351/3916 (uncovered 565), files 108
  rows: 2100 (F? 653: lines 370, sides 283; GAP 1447: lines 1165, sides 282; GAP in test code 1208)
  crates/store/src/block.rs lines 115/115 branches 10/10
  crates/store/src/catalog.rs lines 189/189 branches 50/50
  crates/store/src/catalog_cover_tests.rs lines 49/49 branches 0/0
  crates/store/src/catalog_tests.rs lines 338/341 branches 9/18  GAP 2 F? 10
  crates/store/src/checksum_audit.rs lines 235/235 branches 27/30  GAP 3 F? 0
  crates/store/src/checksum_audit_tests.rs lines 400/403 branches 11/12  GAP 4 F? 0
  crates/store/src/crc.rs lines 90/90 branches 8/8
  crates/store/src/emits.rs lines 300/306 branches 3/4  GAP 1 F? 6
  crates/store/src/file.rs lines 4564/4662 branches 362/382  GAP 95 F? 23
  crates/store/src/file_cover_tests.rs lines 64/64 branches 6/10  GAP 0 F? 4
  crates/store/src/file_fault_tests.rs lines 571/575 branches 25/40  GAP 2 F? 17
  crates/store/src/file_sealed.rs lines 255/271 branches 46/58  GAP 28 F? 0
  crates/store/src/file_sealed_cover_tests.rs lines 59/59 branches 10/20  GAP 0 F? 10
  crates/store/src/file_sealed_tests.rs lines 584/617 branches 25/36  GAP 25 F? 19
  crates/store/src/file_write_door_tests.rs lines 726/744 branches 24/30  GAP 7 F? 17
  crates/store/src/flock.rs lines 446/454 branches 40/48  GAP 16 F? 0
  crates/store/src/flock_patient_tests.rs lines 495/506 branches 23/36  GAP 10 F? 14
  crates/store/src/flock_tests.rs lines 563/570 branches 18/26  GAP 1 F? 14
  crates/store/src/format.rs lines 278/278 branches 42/42
  crates/store/src/generation.rs lines 315/325 branches 59/60  GAP 11 F? 0
  crates/store/src/generation_cover_tests.rs lines 118/118 branches 3/6  GAP 0 F? 3
  crates/store/src/generation_door_cover_tests.rs lines 92/94 branches 2/4  GAP 1 F? 3
  crates/store/src/generation_tests.rs lines 37/37 branches 0/0
  crates/store/src/header.rs lines 524/524 branches 76/76
  crates/store/src/header_audit_tests.rs lines 49/49 branches 0/0
  crates/store/src/layout.rs lines 186/186 branches 40/40
  crates/store/src/month_summary.rs lines 682/683 branches 189/194  GAP 6 F? 0
  crates/store/src/month_summary_cover_tests.rs lines 147/147 branches 0/0
  crates/store/src/month_summary_decode_cover_tests.rs lines 93/94 branches 0/0  GAP 0 F? 1
  crates/store/src/month_summary_tests.rs lines 559/562 branches 32/52  GAP 2 F? 21
  crates/store/src/month_summary_writer.rs lines 165/165 branches 34/34
  crates/store/src/month_summary_writer_cover_tests.rs lines 175/175 branches 2/4  GAP 0 F? 2
  crates/store/src/open_flags.rs lines 6/6 branches 0/0
  crates/store/src/path.rs lines 206/206 branches 30/30
  crates/store/src/provenance.rs lines 756/765 branches 150/152  GAP 11 F? 0
  crates/store/src/provenance_cover_tests.rs lines 181/187 branches 2/4  GAP 2 F? 6
  crates/store/src/quarantine.rs lines 1460/1483 branches 185/188  GAP 26 F? 0
  crates/store/src/quarantine_cover_tests.rs lines 516/520 branches 1/2  GAP 0 F? 5
  crates/store/src/quarantine_tests.rs lines 1548/1557 branches 50/82  GAP 4 F? 37
  crates/store/src/repair.rs lines 727/741 branches 128/132  GAP 18 F? 0
  crates/store/src/repair_cover_tests.rs lines 67/67 branches 3/4  GAP 0 F? 1
  crates/store/src/repair_door_cover_tests.rs lines 190/195 branches 2/2  GAP 1 F? 4
  crates/store/src/repair_tests.rs lines 639/643 branches 15/18  GAP 3 F? 4
  crates/store/src/safe_open.rs lines 169/177 branches 20/22  GAP 10 F? 0
  crates/store/src/safe_open_tests.rs lines 283/298 branches 9/18  GAP 12 F? 12
  crates/store/src/sealed.rs lines 1434/1434 branches 270/274  GAP 4 F? 0
  crates/store/src/sealed_codec.rs lines 646/646 branches 90/90
  crates/store/src/sealed_codec_tests.rs lines 996/1000 branches 30/34  GAP 0 F? 8
  crates/store/src/sealed_tests.rs lines 1719/1740 branches 34/40  GAP 5 F? 22
  crates/store/src/settings.rs lines 724/743 branches 144/150  GAP 25 F? 0
  crates/store/src/settings_cover_tests.rs lines 187/187 branches 1/2  GAP 0 F? 1
  crates/store/src/settings_door_cover_tests.rs lines 99/100 branches 2/4  GAP 0 F? 3
  crates/store/src/settings_tests.rs lines 832/844 branches 16/28  GAP 10 F? 14
  crates/store/src/sidecar_checksum_tests.rs lines 391/391 branches 10/10
  crates/store/src/small_month_tests.rs lines 106/106 branches 1/2  GAP 0 F? 1
  crates/store/src/sort_index.rs lines 794/794 branches 126/126
  crates/store/src/sort_index_cover_tests.rs lines 230/230 branches 5/10  GAP 0 F? 5
  crates/store/src/sort_index_tests.rs lines 385/385 branches 9/10  GAP 0 F? 1
  crates/store/src/sort_index_writer.rs lines 388/388 branches 62/62
  crates/store/src/sort_index_writer_tests.rs lines 114/114 branches 0/0
  crates/store/src/staging.rs lines 109/109 branches 14/14
  crates/store/src/staging_cover_tests.rs lines 26/28 branches 0/0  GAP 0 F? 2
  crates/store/src/staging_tests.rs lines 222/223 branches 8/16  GAP 4 F? 5
  crates/store/src/time_index.rs lines 744/744 branches 71/80  GAP 0 F? 9
  crates/store/src/verdict.rs lines 2679/2685 branches 296/310  GAP 18 F? 2
  crates/store/src/verdict_tests.rs lines 594/601 branches 24/26  GAP 3 F? 6
  crates/store/tests/admission.rs lines 182/182 branches 0/0
  crates/store/tests/base_cov_words.rs lines 46/46 branches 0/0
  crates/store/tests/bisect_cost.rs lines 80/80 branches 0/0
  crates/store/tests/catalog.rs lines 549/568 branches 5/12  GAP 15 F? 11
  crates/store/tests/cited_commits.rs lines 508/532 branches 72/90  GAP 35 F? 7
  crates/store/tests/cloud_mutants_store.rs lines 2190/2203 branches 62/96  GAP 25 F? 22
  crates/store/tests/cold_read.rs lines 199/199 branches 4/4
  crates/store/tests/docs.rs lines 229/242 branches 5/14  GAP 8 F? 14
  crates/store/tests/durability.rs lines 194/195 branches 0/0  GAP 0 F? 1
  crates/store/tests/fault.rs lines 471/472 branches 10/12  GAP 0 F? 3
  crates/store/tests/fifo.rs lines 194/201 branches 7/12  GAP 6 F? 6
  crates/store/tests/generation.rs lines 483/492 branches 8/18  GAP 4 F? 15
  crates/store/tests/geometry.rs lines 433/433 branches 9/12  GAP 1 F? 2
  crates/store/tests/libm_key.rs lines 778/780 branches 8/12  GAP 1 F? 5
  crates/store/tests/misfiled.rs lines 205/207 branches 9/14  GAP 2 F? 5
  crates/store/tests/month_repair.rs lines 605/613 branches 13/18  GAP 4 F? 9
  crates/store/tests/month_summary_log.rs lines 94/96 branches 0/0  GAP 2 F? 0
  crates/store/tests/open_flags.rs lines 97/98 branches 17/18  GAP 2 F? 0
  crates/store/tests/provenance.rs lines 1222/1238 branches 0/0  GAP 0 F? 16
  crates/store/tests/repair.rs lines 384/384 branches 4/6  GAP 0 F? 2
  crates/store/tests/roundtrip.rs lines 111/111 branches 0/0
  crates/store/tests/seal_buffer.rs lines 100/100 branches 0/0
  crates/store/tests/sealed_codec_alloc.rs lines 230/232 branches 15/28  GAP 1 F? 14
  crates/store/tests/sealed_codec_samples.rs lines 0/366 branches 0/44  GAP 378 F? 32
  crates/store/tests/sealed_door.rs lines 126/211 branches 6/20  GAP 74 F? 25
  crates/store/tests/sealed_samples.rs lines 0/174 branches 0/12  GAP 163 F? 23
  crates/store/tests/sealed_write_door.rs lines 0/165 branches 0/24  GAP 157 F? 32
  crates/store/tests/slot_audit.rs lines 348/354 branches 8/10  GAP 2 F? 6
  crates/store/tests/slot_audit_sealed.rs lines 88/96 branches 0/2  GAP 5 F? 5
  crates/store/tests/slot_damaged_log.rs lines 137/145 branches 0/0  GAP 1 F? 7
  crates/store/tests/sort_index.rs lines 599/602 branches 14/22  GAP 0 F? 11
  crates/store/tests/sort_index_log.rs lines 128/130 branches 0/0  GAP 2 F? 0
  crates/store/tests/support/mod.rs lines 32/32 branches 3/4  GAP 1 F? 0
  crates/store/tests/tail_proof.rs lines 175/175 branches 3/6  GAP 0 F? 3
  crates/store/tests/tix.rs lines 406/407 branches 10/12  GAP 2 F? 1
  crates/store/tests/tix_latency.rs lines 0/164 branches 0/14  GAP 177 F? 1
  crates/store/tests/tix_rebuilder.rs lines 336/336 branches 6/8  GAP 2 F? 0
  crates/store/tests/tix_small_month.rs lines 394/396 branches 6/8  GAP 1 F? 3
  crates/store/tests/tix_warn_once.rs lines 212/217 branches 0/0  GAP 2 F? 3
  crates/store/tests/unit.rs lines 1711/1721 branches 30/40  GAP 1 F? 19
  crates/store/tests/verdict.rs lines 1989/2007 branches 28/44  GAP 3 F? 31
  crates/store/tests/write.rs lines 1199/1203 branches 5/8  GAP 0 F? 7

ROWS (2100)
  F? LINE   crates/store/src/catalog_tests.rs:268 [panic] (test): panic!("a dangling `bars` link is refused");
  F? LINE   crates/store/src/catalog_tests.rs:282 [panic] (test): panic!("an unsearchable root is refused, not empty: {locked:?}");
  F? LINE   crates/store/src/catalog_tests.rs:460 [assert] (test): path.display()
  GAP BRANCH crates/store/src/catalog_tests.rs:267:9 False (test): let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {
  GAP BRANCH crates/store/src/catalog_tests.rs:281:9 False (test): let Err(CatalogError::BarsUnreadable { because }) = locked else {
  F? BRANCH crates/store/src/catalog_tests.rs:314:13 False [assert] (test): assert!(stack.is_empty() && out.held.is_empty());
  F? BRANCH crates/store/src/catalog_tests.rs:314:33 False [assert] (test): assert!(stack.is_empty() && out.held.is_empty());
  F? BRANCH crates/store/src/catalog_tests.rs:320:13 False [assert] (test): assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  F? BRANCH crates/store/src/catalog_tests.rs:320:32 False [assert] (test): assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  F? BRANCH crates/store/src/catalog_tests.rs:320:50 False [assert] (test): assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  F? BRANCH crates/store/src/catalog_tests.rs:336:13 False [assert] (test): assert!(stack.is_empty() && out.held.is_empty());
  F? BRANCH crates/store/src/catalog_tests.rs:336:33 False [assert] (test): assert!(stack.is_empty() && out.held.is_empty());
  GAP BRANCH crates/store/src/checksum_audit.rs:286:8 True: if !header.checksums_present()
  GAP BRANCH crates/store/src/checksum_audit.rs:288:12 True: || layout.header_len() != HEADER_BYTES as u64
  GAP BRANCH crates/store/src/checksum_audit.rs:289:12 True: || layout.block_len() > BUFFER_BYTES as u64
  GAP LINE   crates/store/src/checksum_audit_tests.rs:153 (test): child.kill().expect("bounded timeout");
  GAP LINE   crates/store/src/checksum_audit_tests.rs:154 (test): child.wait().expect("reap child");
  GAP LINE   crates/store/src/checksum_audit_tests.rs:155 (test): return Err("strict checksum source open waited for a FIFO peer".into());
  GAP BRANCH crates/store/src/checksum_audit_tests.rs:152:16 True (test): if started.elapsed() > std::time::Duration::from_secs(2) {
  F? LINE   crates/store/src/emits.rs:835 [assert_eq] (test): found
  F? LINE   crates/store/src/emits.rs:836 [assert_eq] (test): .records
  F? LINE   crates/store/src/emits.rs:837 [assert_eq] (test): .iter()
  F? LINE   crates/store/src/emits.rs:838 [assert_eq] (test): .map(|line| (line.target.clone(), line.message.clone()))
  F? LINE   crates/store/src/emits.rs:839 [assert_eq] (test): .collect::<Vec<_>>()
  F? LINE   crates/store/src/emits.rs:848 [panic] (test): panic!(
  GAP BRANCH crates/store/src/emits.rs:311:12 False (test): if let Some(byte) = region.get_mut(16_384 + 56) {
  GAP LINE   crates/store/src/file.rs:2885: )?;
  GAP LINE   crates/store/src/file.rs:2891: )?;
  GAP LINE   crates/store/src/file.rs:3167: Some(other) => return Err(other),
  GAP LINE   crates/store/src/file.rs:3172: return Err(StoreError::Format {
  GAP LINE   crates/store/src/file.rs:3173: path: self.bars_path.clone(),
  GAP LINE   crates/store/src/file.rs:3174: source,
  GAP LINE   crates/store/src/file.rs:3175: });
  GAP LINE   crates/store/src/file.rs:3182: Err(source) => {
  GAP LINE   crates/store/src/file.rs:3183: return Err(StoreError::Format {
  GAP LINE   crates/store/src/file.rs:3184: path: self.bars_path.clone(),
  GAP LINE   crates/store/src/file.rs:3185: source,
  GAP LINE   crates/store/src/file.rs:3186: });
  GAP LINE   crates/store/src/file.rs:3856: return Ok(None);
  GAP LINE   crates/store/src/file.rs:3870: return Ok(None);
  GAP LINE   crates/store/src/file.rs:3881: Err(Why::OffGrid { index, ts_micros }) => Err(StoreError::OffGrid {
  GAP LINE   crates/store/src/file.rs:3882: at: index.saturating_sub(held.n_valid),
  GAP LINE   crates/store/src/file.rs:3883: ts_micros,
  GAP LINE   crates/store/src/file.rs:3884: timeframe_secs: self.header.timeframe_secs,
  GAP LINE   crates/store/src/file.rs:3885: }),
  GAP LINE   crates/store/src/file.rs:3886: Err(Why::Outside { index, ts_micros }) => Err(StoreError::OutsideMonth {
  GAP LINE   crates/store/src/file.rs:3887: at: index.saturating_sub(held.n_valid),
  GAP LINE   crates/store/src/file.rs:3888: ts_micros,
  GAP LINE   crates/store/src/file.rs:3889: month: self.admission.month,
  GAP LINE   crates/store/src/file.rs:3890: }),
  GAP LINE   crates/store/src/file.rs:3891: Err(other) => Err(index_refused(&tix.path, &other)),
  GAP LINE   crates/store/src/file.rs:3974: return Ok(None);
  GAP LINE   crates/store/src/file.rs:3994: Err(why @ Why::SharedSlot { .. }) => {
  GAP LINE   crates/store/src/file.rs:3995: let path = tix.path.clone();
  GAP LINE   crates/store/src/file.rs:3996: self.set_tix_state(bisecting(&path, why));
  GAP LINE   crates/store/src/file.rs:3997: return Ok(None);
  GAP LINE   crates/store/src/file.rs:3999: Err(Why::OffGrid { index, ts_micros }) => {
  GAP LINE   crates/store/src/file.rs:4000: return Err(StoreError::OffGrid {
  GAP LINE   crates/store/src/file.rs:4001: at: index.saturating_sub(held.n_valid),
  GAP LINE   crates/store/src/file.rs:4002: ts_micros,
  GAP LINE   crates/store/src/file.rs:4003: timeframe_secs: self.header.timeframe_secs,
  GAP LINE   crates/store/src/file.rs:4004: });
  GAP LINE   crates/store/src/file.rs:4006: Err(Why::Outside { index, ts_micros }) => {
  GAP LINE   crates/store/src/file.rs:4007: return Err(StoreError::OutsideMonth {
  GAP LINE   crates/store/src/file.rs:4008: at: index.saturating_sub(held.n_valid),
  GAP LINE   crates/store/src/file.rs:4009: ts_micros,
  GAP LINE   crates/store/src/file.rs:4010: month: self.admission.month,
  GAP LINE   crates/store/src/file.rs:4011: });
  GAP LINE   crates/store/src/file.rs:4013: Err(other) => return Err(index_refused(&tix.path, &other)),
  GAP LINE   crates/store/src/file.rs:4053: return Ok(());
  GAP LINE   crates/store/src/file.rs:4064: return Ok(());
  GAP LINE   crates/store/src/file.rs:4081: return Ok(());
  GAP LINE   crates/store/src/file.rs:4084: return Ok(());
  GAP LINE   crates/store/src/file.rs:4306: )?;
  GAP LINE   crates/store/src/file.rs:4316: path: self.bars_path.clone(),
  GAP LINE   crates/store/src/file.rs:4317: source: FormatError::OffsetOverflow,
  GAP LINE   crates/store/src/file.rs:4318: })?;
  GAP LINE   crates/store/src/file.rs:4452: return Some(conflict(at, row.stamp(), Conflict::NotHeld));
  GAP LINE   crates/store/src/file.rs:4504: return Ok(None);
  GAP LINE   crates/store/src/file.rs:4934: other => StoreError::TimeIndex {
  GAP LINE   crates/store/src/file.rs:4935: path: path.to_path_buf(),
  GAP LINE   crates/store/src/file.rs:4936: reason: other.to_string(),
  GAP LINE   crates/store/src/file.rs:4937: },
  F? LINE   crates/store/src/file.rs:6732 [assert] (test): parent.display()
  F? LINE   crates/store/src/file.rs:6936 [assert] (test): matches!(
  F? LINE   crates/store/src/file.rs:6956 [assert_eq] (test): level.display()
  F? LINE   crates/store/src/file.rs:7215 [assert] (test): matches!(
  F? LINE   crates/store/src/file.rs:7269 [panic] (test): other => panic!("the flush was refused by path: {other:?}"),
  F? LINE   crates/store/src/file.rs:7589 [assert] (test): matches!(
  GAP LINE   crates/store/src/file.rs:7835 (test): fn the_kernels_own_enospc_on_a_full_volume_is_disk_full_by_name() {
  GAP LINE   crates/store/src/file.rs:7836 (test): let dir = std::env::var_os("BRUTEX_FULL_VOLUME")
  GAP LINE   crates/store/src/file.rs:7837 (test): .map(PathBuf::from)
  GAP LINE   crates/store/src/file.rs:7838 (test): .expect("BRUTEX_FULL_VOLUME names a directory on a small volume");
  GAP LINE   crates/store/src/file.rs:7839 (test): let at = dir.join("brutex-enospc-probe.bin");
  GAP LINE   crates/store/src/file.rs:7840 (test): let file = std::fs::OpenOptions::new()
  GAP LINE   crates/store/src/file.rs:7841 (test): .write(true)
  GAP LINE   crates/store/src/file.rs:7842 (test): .create_new(true)
  GAP LINE   crates/store/src/file.rs:7843 (test): .open(&at)
  GAP LINE   crates/store/src/file.rs:7844 (test): .expect("a new file on the small volume");
  GAP LINE   crates/store/src/file.rs:7845 (test): let chunk = vec![0xAB_u8; 1 << 16];
  GAP LINE   crates/store/src/file.rs:7846 (test): let mut offset = 0_u64;
  GAP LINE   crates/store/src/file.rs:7847 (test): let refused = loop {
  F? LINE   crates/store/src/file.rs:7848 [assert] (test): assert!(offset < 1 << 30, "the volume took 1 GiB without refusing");
  GAP LINE   crates/store/src/file.rs:7849 (test): match write_fully(&file, &at, offset, &chunk) {
  GAP LINE   crates/store/src/file.rs:7850 (test): Ok(()) => offset = offset.saturating_add(1 << 16),
  GAP LINE   crates/store/src/file.rs:7851 (test): Err(refused) => break refused,
  F? LINE   crates/store/src/file.rs:7854 [assert_eq] (test): assert_eq!(
  F? LINE   crates/store/src/file.rs:7856 [assert_eq] (test): StoreError::DiskFull {
  GAP LINE   crates/store/src/file.rs:7857 (test): path: at.clone(),
  GAP LINE   crates/store/src/file.rs:7858 (test): action: Action::Write,
  GAP LINE   crates/store/src/file.rs:7859 (test): },
  GAP LINE   crates/store/src/file.rs:7863 (test): eprintln!("ENOSPC: DiskFull by name after {offset} bytes accepted");
  GAP LINE   crates/store/src/file.rs:7864 (test): }
  F? LINE   crates/store/src/file.rs:8353 [assert] (test): reads.get()
  GAP LINE   crates/store/src/file.rs:8676 (test): continue;
  F? LINE   crates/store/src/file.rs:8692 [panic] (test): other => panic!("a version-3 month without its flag must be refused, got {other:?}"),
  F? LINE   crates/store/src/file.rs:8910 [assert] (test): matches!(
  F? LINE   crates/store/src/file.rs:8919 [assert] (test): matches!(
  F? LINE   crates/store/src/file.rs:8929 [assert] (test): matches!(
  F? LINE   crates/store/src/file.rs:9031 [assert] (test): matches!(
  F? LINE   crates/store/src/file.rs:9074 [assert] (test): matches!(
  F? LINE   crates/store/src/file.rs:9280 [assert] (test): matches!(
  F? LINE   crates/store/src/file.rs:10225 [assert] (test): matches!(
  F? LINE   crates/store/src/file.rs:10351 [assert] (test): matches!(
  GAP BRANCH crates/store/src/file.rs:3762:16 False: if let Some(tix) = self.tix.as_mut() {
  GAP BRANCH crates/store/src/file.rs:3855:21 False: let Some(tix) = self.tix.as_ref() else {
  GAP BRANCH crates/store/src/file.rs:3869:13 False: let Some(tix) = self.tix.as_ref() else {
  GAP BRANCH crates/store/src/file.rs:3973:13 False: let Some(tix) = self.tix.as_ref() else {
  GAP BRANCH crates/store/src/file.rs:4052:13 False: let Some(path) = self.tix.as_ref().map(|tix| tix.path.clone()) else {
  GAP BRANCH crates/store/src/file.rs:4063:13 False: let Some(tix) = self.tix.as_ref() else {
  GAP BRANCH crates/store/src/file.rs:4080:13 False: let Some(tix) = self.tix.as_ref() else {
  GAP BRANCH crates/store/src/file.rs:4083:13 False: let Some(TixState::Ready(index)) = tix.state.get() else {
  GAP BRANCH crates/store/src/file.rs:4189:36 True: if cache.block != block || cache.n_valid != n_valid {
  GAP BRANCH crates/store/src/file.rs:4451:16 True: if index >= n_valid {
  GAP BRANCH crates/store/src/file.rs:4503:12 True: if self.header.n_valid == 0 {
  GAP BRANCH crates/store/src/file.rs:5666:42 True: BatchFlush::OncePerDevice if O_SYNC.is_none() => return Ok(false),
  GAP BRANCH crates/store/src/file.rs:5760:19 False: while let Some(dir) = next {
  F? BRANCH crates/store/src/file.rs:6879:82 False [assert] (test): matches!(&refused, Err(StoreError::Denied { action: at, .. }) if *at == action),
  GAP BRANCH crates/store/src/file.rs:8675:16 True (test): if &slot[..7] != b"BRUTEXB" {
  GAP BRANCH crates/store/src/file.rs:9091:16 False (test): if let Some(head) = cache.bytes.get_mut(..8) {
  F? BRANCH crates/store/src/file.rs:9103:13 False [assert] (test): shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  F? BRANCH crates/store/src/file.rs:9103:45 False [assert] (test): shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  F? BRANCH crates/store/src/file.rs:9103:73 False [assert] (test): shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  GAP BRANCH crates/store/src/file.rs:10721:24 False (test): if *ts_micros == bar(10).ts_micros
  F? BRANCH crates/store/src/file_cover_tests.rs:91:9 False [assert] (test): follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
  F? BRANCH crates/store/src/file_cover_tests.rs:91:65 False [assert] (test): follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
  F? BRANCH crates/store/src/file_cover_tests.rs:118:9 False [assert] (test): whole.starts_with(&last.took) && last.took.len() < whole.len(),
  F? BRANCH crates/store/src/file_cover_tests.rs:118:42 False [assert] (test): whole.starts_with(&last.took) && last.took.len() < whole.len(),
  F? LINE   crates/store/src/file_fault_tests.rs:320 [panic] (test): other => panic!("read {skip}: {other:?}"),
  F? LINE   crates/store/src/file_fault_tests.rs:657 [panic] (test): Err(other) => panic!("read {skip}: {other:?}"),
  F? LINE   crates/store/src/file_fault_tests.rs:696 [panic] (test): other => panic!("{}: {other:?}", looked.display()),
  F? LINE   crates/store/src/file_fault_tests.rs:869 [panic] (test): other => panic!("{kind:?} measure {skip}: {other:?}"),
  GAP BRANCH crates/store/src/file_fault_tests.rs:315:57 False (test): Err(StoreError::Denied { path, action }) if fired => {
  F? BRANCH crates/store/src/file_fault_tests.rs:334:9 False [assert] (test): routed >= 1 && refused >= 1,
  F? BRANCH crates/store/src/file_fault_tests.rs:334:24 False [assert] (test): routed >= 1 && refused >= 1,
  F? BRANCH crates/store/src/file_fault_tests.rs:650:21 False [assert] (test): reason.starts_with(&format!("{}: ", srt.display()))
  F? BRANCH crates/store/src/file_fault_tests.rs:651:28 False [assert] (test): && reason.contains("refused by the file test seam"),
  F? BRANCH crates/store/src/file_fault_tests.rs:661:9 False [assert] (test): built >= 1 && refused >= 1,
  F? BRANCH crates/store/src/file_fault_tests.rs:661:23 False [assert] (test): built >= 1 && refused >= 1,
  F? BRANCH crates/store/src/file_fault_tests.rs:755:71 False [assert] (test): matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),
  GAP BRANCH crates/store/src/file_fault_tests.rs:845:61 False (test): Err(StoreError::Denied { path, action }) if fired => {
  F? BRANCH crates/store/src/file_fault_tests.rs:879:26 False [assert] (test): _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  F? BRANCH crates/store/src/file_fault_tests.rs:879:42 False [assert] (test): _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  F? BRANCH crates/store/src/file_fault_tests.rs:912:13 False [assert] (test): assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F? BRANCH crates/store/src/file_fault_tests.rs:912:54 False [assert] (test): assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F? BRANCH crates/store/src/file_fault_tests.rs:949:13 False [assert] (test): assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F? BRANCH crates/store/src/file_fault_tests.rs:949:54 False [assert] (test): assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  GAP LINE   crates/store/src/file_sealed.rs:136: return Err(StoreError::SealedRefused {
  GAP LINE   crates/store/src/file_sealed.rs:137: path: at,
  GAP LINE   crates/store/src/file_sealed.rs:138: reason: "block_rows",
  GAP LINE   crates/store/src/file_sealed.rs:139: detail: format!(
  GAP LINE   crates/store/src/file_sealed.rs:140: "the head declares {} rows a frame and a handle caches at most {}",
  GAP LINE   crates/store/src/file_sealed.rs:141: head.block_rows,
  GAP LINE   crates/store/src/file_sealed.rs:142: CACHED_BLOCK_LEN / RECORD_LEN
  GAP LINE   crates/store/src/file_sealed.rs:143: ),
  GAP LINE   crates/store/src/file_sealed.rs:144: });
  GAP LINE   crates/store/src/file_sealed.rs:231: )?;
  GAP LINE   crates/store/src/file_sealed.rs:236: path: self.bars_path.clone(),
  GAP LINE   crates/store/src/file_sealed.rs:237: source: FormatError::OffsetOverflow,
  GAP LINE   crates/store/src/file_sealed.rs:238: })?;
  GAP LINE   crates/store/src/file_sealed.rs:359: }
  GAP LINE   crates/store/src/file_sealed.rs:372: continue;
  GAP LINE   crates/store/src/file_sealed.rs:418: return Ok(None);
  GAP BRANCH crates/store/src/file_sealed.rs:135:12 True: if usize::from(head.block_rows).saturating_mul(RECORD_LEN) > CACHED_BLOCK_LEN {
  GAP BRANCH crates/store/src/file_sealed.rs:202:36 True: if cache.block != block || cache.n_valid != n_valid {
  GAP BRANCH crates/store/src/file_sealed.rs:341:20 False: if let Some(count) = first.get_mut(at + 1) {
  GAP BRANCH crates/store/src/file_sealed.rs:347:20 False: if let Some(start) = first.get_mut(at) {
  GAP BRANCH crates/store/src/file_sealed.rs:354:20 False: if let Some(slot) = next_slot.get_mut(at) {
  GAP BRANCH crates/store/src/file_sealed.rs:355:24 False: if let Some(place) = placed.get_mut(*slot) {
  GAP BRANCH crates/store/src/file_sealed.rs:371:21 False: let Some(&row) = rows.get(asked) else {
  GAP BRANCH crates/store/src/file_sealed.rs:379:24 False: if let Some(slot) = out.get_mut(asked) {
  GAP BRANCH crates/store/src/file_sealed.rs:417:9 False: let Some(sealed) = file.sealed.as_ref() else {
  GAP BRANCH crates/store/src/file_sealed.rs:425:21 False: Err(why) if why.kind() == ErrorKind::NotFound => Err(format!(
  GAP BRANCH crates/store/src/file_sealed.rs:460:12 False: && header.symbol_id == sealed.header.symbol_id
  GAP BRANCH crates/store/src/file_sealed.rs:461:12 False: && header.timeframe_secs == sealed.header.timeframe_secs
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:67:9 False [assert] (test): fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:67:51 False [assert] (test): fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:74:9 False [assert] (test): twice.contains("rows 900 against 900")
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:75:16 False [assert] (test): && twice.contains("generation 2 against 1")
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:76:16 False [assert] (test): && twice.contains("records BLAKE3 equal: true"),
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:90:9 False [assert] (test): shifted.contains("rows 900 against 900")
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:91:16 False [assert] (test): && shifted.contains("generation 1 against 1")
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:92:16 False [assert] (test): && shifted.contains("records BLAKE3 equal: false"),
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:100:9 False [assert] (test): late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
  F? BRANCH crates/store/src/file_sealed_cover_tests.rs:100:50 False [assert] (test): late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
  F? LINE   crates/store/src/file_sealed_tests.rs:321 [panic] (test): other => panic!("expected Missing naming the .bin, got {other:?}"),
  F? LINE   crates/store/src/file_sealed_tests.rs:419 [panic] (test): other => panic!("a damaged frame must be refused by name, got {other:?}"),
  F? LINE   crates/store/src/file_sealed_tests.rs:439 [panic] (test): other => panic!("a damaged head must be refused, got {other:?}"),
  F? LINE   crates/store/src/file_sealed_tests.rs:798 [assert] (test): matches!(
  F? LINE   crates/store/src/file_sealed_tests.rs:803 [assert] (test): crate::provenance::read(&root.0, path)
  F? LINE   crates/store/src/file_sealed_tests.rs:885 [assert_eq] (test): page.len()
  F? LINE   crates/store/src/file_sealed_tests.rs:898 [assert_eq] (test): page.len()
  GAP LINE   crates/store/src/file_sealed_tests.rs:913 (test): fn read_rows_at_timing() {
  GAP LINE   crates/store/src/file_sealed_tests.rs:914 (test): let _sink_is_mine = crate::emits::hold_the_sink();
  GAP LINE   crates/store/src/file_sealed_tests.rs:915 (test): for n in [2_250_u64, 22_500, 225_000] {
  GAP LINE   crates/store/src/file_sealed_tests.rs:916 (test): let root = Root::new(&format!("rows-at-timing-{n}"));
  GAP LINE   crates/store/src/file_sealed_tests.rs:917 (test): let path = month_path(None);
  GAP LINE   crates/store/src/file_sealed_tests.rs:918 (test): let bars = seconds(n);
  GAP LINE   crates/store/src/file_sealed_tests.rs:919 (test): write_raw(&root.0, path, &bars);
  GAP LINE   crates/store/src/file_sealed_tests.rs:920 (test): seal(&root.0, path);
  GAP LINE   crates/store/src/file_sealed_tests.rs:921 (test): let sealed = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
  GAP LINE   crates/store/src/file_sealed_tests.rs:922 (test): for width in [100_u64, 1_000] {
  GAP LINE   crates/store/src/file_sealed_tests.rs:923 (test): let page: Vec<u64> = (0..width).map(|k| (k * 7_919 + 13) % n).collect();
  GAP LINE   crates/store/src/file_sealed_tests.rs:924 (test): let _warm = sealed.read_rows_at(&page).unwrap();
  GAP LINE   crates/store/src/file_sealed_tests.rs:925 (test): let mut took = Vec::with_capacity(2_000);
  GAP LINE   crates/store/src/file_sealed_tests.rs:926 (test): for _ in 0..2_000 {
  GAP LINE   crates/store/src/file_sealed_tests.rs:927 (test): let began = std::time::Instant::now();
  GAP LINE   crates/store/src/file_sealed_tests.rs:928 (test): let got = sealed.read_rows_at(&page).unwrap();
  GAP LINE   crates/store/src/file_sealed_tests.rs:929 (test): took.push(began.elapsed());
  F? LINE   crates/store/src/file_sealed_tests.rs:930 [assert_eq] (test): assert_eq!(got.len(), page.len());
  GAP LINE   crates/store/src/file_sealed_tests.rs:932 (test): took.sort_unstable();
  GAP LINE   crates/store/src/file_sealed_tests.rs:933 (test): let k = took.len();
  GAP LINE   crates/store/src/file_sealed_tests.rs:934 (test): println!(
  GAP LINE   crates/store/src/file_sealed_tests.rs:936 (test): took[k / 2],
  GAP LINE   crates/store/src/file_sealed_tests.rs:937 (test): took[k * 99 / 100],
  GAP LINE   crates/store/src/file_sealed_tests.rs:938 (test): took[k - 1],
  GAP LINE   crates/store/src/file_sealed_tests.rs:939 (test): took[k / 2] / u32::try_from(width).unwrap()
  GAP LINE   crates/store/src/file_sealed_tests.rs:943 (test): }
  F? BRANCH crates/store/src/file_sealed_tests.rs:214:69 False [assert] (test): Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  F? BRANCH crates/store/src/file_sealed_tests.rs:214:83 False [assert] (test): Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  F? BRANCH crates/store/src/file_sealed_tests.rs:315:13 False [assert] (test): assert!(!bin.exists() && retired.exists() && sbar.exists());
  F? BRANCH crates/store/src/file_sealed_tests.rs:315:30 False [assert] (test): assert!(!bin.exists() && retired.exists() && sbar.exists());
  F? BRANCH crates/store/src/file_sealed_tests.rs:315:50 False [assert] (test): assert!(!bin.exists() && retired.exists() && sbar.exists());
  F? BRANCH crates/store/src/file_sealed_tests.rs:407:17 False [assert] (test): !reason.is_empty() && !detail.is_empty(),
  F? BRANCH crates/store/src/file_sealed_tests.rs:407:39 False [assert] (test): !reason.is_empty() && !detail.is_empty(),
  F? BRANCH crates/store/src/file_sealed_tests.rs:824:9 False [assert] (test): refused.starts_with("Err(") && refused.contains("sbar"),
  F? BRANCH crates/store/src/file_sealed_tests.rs:824:40 False [assert] (test): refused.starts_with("Err(") && refused.contains("sbar"),
  F? BRANCH crates/store/src/file_sealed_tests.rs:874:73 False [assert] (test): Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  F? BRANCH crates/store/src/file_sealed_tests.rs:874:92 False [assert] (test): Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  GAP LINE   crates/store/src/file_write_door_tests.rs:68 (test): continue;
  F? LINE   crates/store/src/file_write_door_tests.rs:165 [panic] (test): panic!("a new bar at a sealed month is `Sealed`, got {refused}");
  F? LINE   crates/store/src/file_write_door_tests.rs:322 [panic] (test): _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),
  F? LINE   crates/store/src/file_write_door_tests.rs:439 [panic] (test): _ => panic!("{case} sealed={sealed}: raw {raw:?}, offered {got:?}"),
  F? LINE   crates/store/src/file_write_door_tests.rs:539 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/file_write_door_tests.rs:595 [panic] (test): other => panic!("{n}: {other}"),
  F? LINE   crates/store/src/file_write_door_tests.rs:605 [panic] (test): other => panic!("{n}: {other}"),
  F? LINE   crates/store/src/file_write_door_tests.rs:721 [assert] (test): file.display()
  F? LINE   crates/store/src/file_write_door_tests.rs:749 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/file_write_door_tests.rs:765 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/file_write_door_tests.rs:809 [panic] (test): Err(other) => panic!("{other}"),
  F? LINE   crates/store/src/file_write_door_tests.rs:844 [assert] (test): matches!(
  F? LINE   crates/store/src/file_write_door_tests.rs:859 [panic] (test): panic!("{case}: {repaired:?}");
  F? LINE   crates/store/src/file_write_door_tests.rs:902 [assert] (test): matches!(
  F? LINE   crates/store/src/file_write_door_tests.rs:943 [panic] (test): panic!("{repaired:?}");
  F? LINE   crates/store/src/file_write_door_tests.rs:983 [assert] (test): matches!(
  F? LINE   crates/store/src/file_write_door_tests.rs:1027 [panic] (test): panic!("{case}: {repaired:?}");
  F? LINE   crates/store/src/file_write_door_tests.rs:1067 [panic] (test): panic!("{case}: {repaired:?}");
  GAP BRANCH crates/store/src/file_write_door_tests.rs:67:13 False (test): let Ok(entries) = fs::read_dir(&at) else {
  GAP BRANCH crates/store/src/file_write_door_tests.rs:156:13 False (test): let StoreError::Sealed {
  GAP BRANCH crates/store/src/file_write_door_tests.rs:852:13 False (test): let MonthRepair::Repaired {
  GAP BRANCH crates/store/src/file_write_door_tests.rs:942:9 False (test): let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  GAP BRANCH crates/store/src/file_write_door_tests.rs:1026:9 False (test): let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  GAP BRANCH crates/store/src/file_write_door_tests.rs:1060:9 False (test): let MonthRepair::Repaired {
  GAP LINE   crates/store/src/flock.rs:306: return;
  GAP LINE   crates/store/src/flock.rs:335: || "an unnamed program".to_owned(),
  GAP LINE   crates/store/src/flock.rs:557: |_| std::fs::FileTimes::new(),
  GAP LINE   crates/store/src/flock.rs:627: Impatient::Host(why) => TryLockError::Error(why),
  GAP LINE   crates/store/src/flock.rs:682: note_unnoted(self.path.as_ref(), &why);
  GAP LINE   crates/store/src/flock.rs:791: return;
  GAP LINE   crates/store/src/flock.rs:883: )?;
  GAP LINE   crates/store/src/flock.rs:974: host @ Impatient::Host(_) => host,
  GAP BRANCH crates/store/src/flock.rs:181:9 False: self.writer.is_none() && self.readers == 0 && self.queue.is_none()
  GAP BRANCH crates/store/src/flock.rs:305:9 False: let Some(holders) = held.as_mut() else {
  GAP BRANCH crates/store/src/flock.rs:560:24 False: if let Ok(modified) = meta.modified() {
  GAP BRANCH crates/store/src/flock.rs:563:24 False: if let Ok(accessed) = meta.accessed() {
  GAP BRANCH crates/store/src/flock.rs:681:20 True: if let Err(why) = file.set_len(0).and_then(|()| file.set_times(times)) {
  GAP BRANCH crates/store/src/flock.rs:790:8 True: if SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
  GAP BRANCH crates/store/src/flock.rs:1094:24 False: if idle && let Some(holders) = held.as_mut() {
  GAP BRANCH crates/store/src/flock.rs:1099:16 False: if empty {
  F? LINE   crates/store/src/flock_patient_tests.rs:109 [panic] (test): panic!("contention, not a host refusal: {refused}");
  F? LINE   crates/store/src/flock_patient_tests.rs:173 [panic] (test): panic!("a host refusal: {got:?}");
  F? LINE   crates/store/src/flock_patient_tests.rs:195 [panic] (test): panic!("{got:?}");
  F? LINE   crates/store/src/flock_patient_tests.rs:265 [panic] (test): panic!("{got:?}");
  GAP LINE   crates/store/src/flock_patient_tests.rs:323 (test): )?;
  GAP LINE   crates/store/src/flock_patient_tests.rs:364 (test): Err(_) => refused += 1,
  F? LINE   crates/store/src/flock_patient_tests.rs:439 [panic] (test): panic!("it gave up: {refused:?}");
  GAP LINE   crates/store/src/flock_patient_tests.rs:611 (test): Err(_) => refused += 1,
  F? LINE   crates/store/src/flock_patient_tests.rs:630 [panic] (test): Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),
  GAP LINE   crates/store/src/flock_patient_tests.rs:640 (test): unnamed += 1;
  F? LINE   crates/store/src/flock_patient_tests.rs:687 [assert] (test): matches!(
  GAP BRANCH crates/store/src/flock_patient_tests.rs:101:9 False (test): let Impatient::Held {
  F? BRANCH crates/store/src/flock_patient_tests.rs:123:9 False [assert] (test): took >= bound && took < bound + MAX_PAUSE + SLACK,
  F? BRANCH crates/store/src/flock_patient_tests.rs:123:26 False [assert] (test): took >= bound && took < bound + MAX_PAUSE + SLACK,
  GAP BRANCH crates/store/src/flock_patient_tests.rs:172:9 False (test): let Err(Impatient::Host(why)) = got else {
  GAP BRANCH crates/store/src/flock_patient_tests.rs:191:9 False (test): let Err(Impatient::Held {
  F? BRANCH crates/store/src/flock_patient_tests.rs:219:62 False [assert] (test): matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))
  GAP BRANCH crates/store/src/flock_patient_tests.rs:261:9 False (test): let Err(Impatient::Held {
  GAP BRANCH crates/store/src/flock_patient_tests.rs:435:9 False (test): let Err(Impatient::Held {
  F? BRANCH crates/store/src/flock_patient_tests.rs:565:9 False [assert] (test): note.contains("thread ag06-successor") && note.contains("doing the second"),
  F? BRANCH crates/store/src/flock_patient_tests.rs:565:51 False [assert] (test): note.contains("thread ag06-successor") && note.contains("doing the second"),
  GAP BRANCH crates/store/src/flock_patient_tests.rs:639:16 True (test): if !named {
  F? BRANCH crates/store/src/flock_patient_tests.rs:683:9 False [assert] (test): text.contains("held by this process") && text.contains("doing the journal index build"),
  F? BRANCH crates/store/src/flock_patient_tests.rs:683:50 False [assert] (test): text.contains("held by this process") && text.contains("doing the journal index build"),
  F? LINE   crates/store/src/flock_tests.rs:213 [panic] (test): panic!(
  F? LINE   crates/store/src/flock_tests.rs:244 [panic] (test): panic!("{take:?} (release: {release}) over &File kept the lock — {why}")
  F? LINE   crates/store/src/flock_tests.rs:264 [panic] (test): panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")
  F? LINE   crates/store/src/flock_tests.rs:283 [panic] (test): panic!(
  F? LINE   crates/store/src/flock_tests.rs:320 [panic] (test): panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")
  GAP LINE   crates/store/src/flock_tests.rs:699 (test): matches!(
  F? LINE   crates/store/src/flock_tests.rs:924 [assert] (test): matches!(
  F? BRANCH crates/store/src/flock_tests.rs:732:13 False [assert] (test): assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  F? BRANCH crates/store/src/flock_tests.rs:732:38 False [assert] (test): assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  F? BRANCH crates/store/src/flock_tests.rs:759:9 False [assert] (test): said.contains("r3-08-holder") && !said.contains("nreadable"),
  F? BRANCH crates/store/src/flock_tests.rs:759:42 False [assert] (test): said.contains("r3-08-holder") && !said.contains("nreadable"),
  F? BRANCH crates/store/src/flock_tests.rs:841:9 False [assert] (test): said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  F? BRANCH crates/store/src/flock_tests.rs:841:51 False [assert] (test): said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  F? BRANCH crates/store/src/flock_tests.rs:952:9 False [assert] (test): note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  F? BRANCH crates/store/src/flock_tests.rs:952:66 False [assert] (test): note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  GAP LINE   crates/store/src/generation.rs:457: )?;
  GAP LINE   crates/store/src/generation.rs:473: )?;
  GAP LINE   crates/store/src/generation.rs:480: Err(std::fs::TryLockError::Error(why)) => {
  GAP LINE   crates/store/src/generation.rs:481: return io_at(Err(why), &pointer_file, "lock pointer", false);
  GAP LINE   crates/store/src/generation.rs:519: )?;
  GAP LINE   crates/store/src/generation.rs:607: path: path.to_path_buf(),
  GAP LINE   crates/store/src/generation.rs:608: operation,
  GAP LINE   crates/store/src/generation.rs:609: kind: error.kind(),
  GAP LINE   crates/store/src/generation.rs:610: publication_may_be_visible: visible,
  GAP LINE   crates/store/src/generation.rs:611: })
  GAP BRANCH crates/store/src/generation.rs:307:21 False: Err(why) if why.kind() == ErrorKind::NotADirectory && !root.is_dir() => {
  F? BRANCH crates/store/src/generation_cover_tests.rs:125:55 False [assert] (test): Err(StoreError::Generation { reason, .. }) if reason == "it is not a regular file"
  F? BRANCH crates/store/src/generation_cover_tests.rs:132:55 False [assert] (test): Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
  F? BRANCH crates/store/src/generation_cover_tests.rs:138:55 True [assert] (test): Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
  F? LINE   crates/store/src/generation_door_cover_tests.rs:99 [panic] (test): panic!("revision 1 is served");
  F? LINE   crates/store/src/generation_door_cover_tests.rs:155 [panic] (test): other => panic!("the pointer read is refused: {other:?}"),
  GAP BRANCH crates/store/src/generation_door_cover_tests.rs:98:9 False (test): let Served::Revision(record0) = served(&root.0, month()).unwrap() else {
  F? BRANCH crates/store/src/generation_door_cover_tests.rs:136:59 False [assert] (test): Ok(Promoted::Promoted(Pointer { record, .. })) if record == MAX_RECORDS - 1
  GAP LINE   crates/store/src/month_summary.rs:1087: }
  GAP BRANCH crates/store/src/month_summary.rs:239:12 False: if let Some(slot) = words.get_mut(word) {
  GAP BRANCH crates/store/src/month_summary.rs:939:24 False: if let Some(words) = entry.seconds.as_deref() {
  GAP BRANCH crates/store/src/month_summary.rs:1055:12 True: if at.short || at.at != payload.len() {
  GAP BRANCH crates/store/src/month_summary.rs:1085:24 False: if let Some(words) = entry.seconds.as_deref_mut() {
  GAP BRANCH crates/store/src/month_summary.rs:1195:12 False: if let Some(slot) = words.get_mut(word) {
  F? LINE   crates/store/src/month_summary_decode_cover_tests.rs:171 [panic] (test): other => panic!("the open is refused in the host's words: {other:?}"),
  GAP LINE   crates/store/src/month_summary_tests.rs:294 (test): let _release = fs::OpenOptions::new().write(true).open(pipe);
  F? LINE   crates/store/src/month_summary_tests.rs:295 [panic] (test): panic!("a read of {} waited for a writer", pipe.display());
  F? LINE   crates/store/src/month_summary_tests.rs:791 [panic] (test): other => panic!("{other:?}"),
  F? BRANCH crates/store/src/month_summary_tests.rs:278:52 False [assert] (test): matches!(&built, Summarized::Built(why) if why.starts_with("torn")),
  GAP BRANCH crates/store/src/month_summary_tests.rs:293:9 False (test): let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  F? BRANCH crates/store/src/month_summary_tests.rs:320:53 False [assert] (test): matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),
  F? BRANCH crates/store/src/month_summary_tests.rs:340:53 False [assert] (test): matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  F? BRANCH crates/store/src/month_summary_tests.rs:355:53 False [assert] (test): matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  F? BRANCH crates/store/src/month_summary_tests.rs:377:13 False [assert] (test): assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  F? BRANCH crates/store/src/month_summary_tests.rs:377:40 False [assert] (test): assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  F? BRANCH crates/store/src/month_summary_tests.rs:461:32 False [assert] (test): Refused::Shape(why) if why.contains("flag")
  F? BRANCH crates/store/src/month_summary_tests.rs:467:60 False [assert] (test): matches!(shape(&unflagged), Refused::Shape(why) if why.contains("rung")),
  F? BRANCH crates/store/src/month_summary_tests.rs:480:59 False [assert] (test): assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));
  F? BRANCH crates/store/src/month_summary_tests.rs:483:63 False [assert] (test): assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));
  F? BRANCH crates/store/src/month_summary_tests.rs:488:61 False [assert] (test): assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));
  F? BRANCH crates/store/src/month_summary_tests.rs:491:59 False [assert] (test): assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));
  F? BRANCH crates/store/src/month_summary_tests.rs:655:13 False [assert] (test): assert!(round.tracks_carries() && !round.wants_carries());
  F? BRANCH crates/store/src/month_summary_tests.rs:655:39 False [assert] (test): assert!(round.tracks_carries() && !round.wants_carries());
  F? BRANCH crates/store/src/month_summary_tests.rs:658:13 False [assert] (test): assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  F? BRANCH crates/store/src/month_summary_tests.rs:658:42 False [assert] (test): assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  F? BRANCH crates/store/src/month_summary_tests.rs:739:37 False [assert] (test): Err(Refused::Shape(why)) if why.contains("not a held minute")
  F? BRANCH crates/store/src/month_summary_tests.rs:757:13 False [assert] (test): assert!(!read.tracks_carries() && read.wants_carries());
  F? BRANCH crates/store/src/month_summary_tests.rs:757:39 False [assert] (test): assert!(!read.tracks_carries() && read.wants_carries());
  F? BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:13 False [assert] (test): assert!(staging.is_dir() && !root.sum().exists());
  F? BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:33 False [assert] (test): assert!(staging.is_dir() && !root.sum().exists());
  GAP LINE   crates/store/src/provenance.rs:1170: io_error(
  GAP LINE   crates/store/src/provenance.rs:1171: &at,
  GAP LINE   crates/store/src/provenance.rs:1173: io::Error::other(why.to_string()),
  GAP LINE   crates/store/src/provenance.rs:1175: };
  GAP LINE   crates/store/src/provenance.rs:1271: fs::TryLockError::Error(why) => io_error(&at, "lock", why),
  GAP LINE   crates/store/src/provenance.rs:1282: )?;
  GAP LINE   crates/store/src/provenance.rs:1569: path: at.to_path_buf(),
  GAP LINE   crates/store/src/provenance.rs:1571: source: io::Error::other(why.to_string()),
  GAP LINE   crates/store/src/provenance.rs:1572: })?;
  GAP BRANCH crates/store/src/provenance.rs:1658:12 False: if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
  GAP BRANCH crates/store/src/provenance.rs:1666:12 False: if let Some(run) = bytes.get(at..at.saturating_add(N)) {
  F? LINE   crates/store/src/provenance_cover_tests.rs:166 [panic] (test): other => panic!("the pointer refuses the read: {other:?}"),
  F? LINE   crates/store/src/provenance_cover_tests.rs:186 [panic] (test): other => panic!("the sealed form refuses the read: {other:?}"),
  F? LINE   crates/store/src/provenance_cover_tests.rs:204 [panic] (test): panic!("the raw sidecar holds a record");
  F? LINE   crates/store/src/provenance_cover_tests.rs:218 [panic] (test): other => panic!("the sealed image's record is served: {other:?}"),
  F? LINE   crates/store/src/provenance_cover_tests.rs:252 [panic] (test): other => panic!("the open is refused by name: {other:?}"),
  F? LINE   crates/store/src/provenance_cover_tests.rs:276 [panic] (test): panic!("the batch's record is on disk");
  GAP BRANCH crates/store/src/provenance_cover_tests.rs:203:9 False (test): let Found::Present(raw) = read(&root.0, june).unwrap() else {
  GAP BRANCH crates/store/src/provenance_cover_tests.rs:275:9 False (test): let Found::Present(sidecar) = read(&root.0, june).unwrap() else {
  GAP LINE   crates/store/src/quarantine.rs:831: )?;
  GAP LINE   crates/store/src/quarantine.rs:861: )?;
  GAP LINE   crates/store/src/quarantine.rs:1032: Err(std::fs::TryLockError::Error(_)) => false,
  GAP LINE   crates/store/src/quarantine.rs:1186: )?;
  GAP LINE   crates/store/src/quarantine.rs:1188: return Err(QuarantineError::Torn {
  GAP LINE   crates/store/src/quarantine.rs:1189: path: index.to_path_buf(),
  GAP LINE   crates/store/src/quarantine.rs:1190: len: image.len() as u64,
  GAP LINE   crates/store/src/quarantine.rs:1191: });
  GAP LINE   crates/store/src/quarantine.rs:1354: path: lock_path(root, feed),
  GAP LINE   crates/store/src/quarantine.rs:1356: kind: io::ErrorKind::Other,
  GAP LINE   crates/store/src/quarantine.rs:1357: })
  GAP LINE   crates/store/src/quarantine.rs:1460: )?;
  GAP LINE   crates/store/src/quarantine.rs:1467: Err(other) => Err(QuarantineError::Io {
  GAP LINE   crates/store/src/quarantine.rs:1468: path,
  GAP LINE   crates/store/src/quarantine.rs:1469: operation: "lock the quarantine",
  GAP LINE   crates/store/src/quarantine.rs:1470: kind: impatient_kind(&other),
  GAP LINE   crates/store/src/quarantine.rs:1471: }),
  GAP LINE   crates/store/src/quarantine.rs:1759: )?;
  GAP LINE   crates/store/src/quarantine.rs:2172: Err(other) => return Err(other),
  GAP LINE   crates/store/src/quarantine.rs:2218: )?;
  GAP LINE   crates/store/src/quarantine.rs:2224: )?;
  GAP LINE   crates/store/src/quarantine.rs:2638: )?;
  GAP LINE   crates/store/src/quarantine.rs:2708: Some(Applied::Resumed { bars, .. }) => bars.saturating_add(intent.bars),
  GAP BRANCH crates/store/src/quarantine.rs:1187:8 True: if image.len() as u64 != INDEX_LEN {
  GAP BRANCH crates/store/src/quarantine.rs:1439:25 False: Err(why) if why.kind() != io::ErrorKind::AlreadyExists => {
  GAP BRANCH crates/store/src/quarantine.rs:2054:35 False: } if held == *hash && was == month => return Ok((slot, Some(before))),
  F? LINE   crates/store/src/quarantine_cover_tests.rs:635 [panic] (test): Ok(Level::Current(_)) => panic!("a torn image is never current"),
  F? LINE   crates/store/src/quarantine_cover_tests.rs:636 [panic] (test): Err(refused) => panic!("a torn image is rebuilt, not refused: {refused}"),
  F? LINE   crates/store/src/quarantine_cover_tests.rs:643 [panic] (test): Ok(Level::Current(_)) => panic!("a damaged header is never current"),
  F? LINE   crates/store/src/quarantine_cover_tests.rs:644 [panic] (test): Err(refused) => panic!("a damaged header is rebuilt, not refused: {refused}"),
  F? BRANCH crates/store/src/quarantine_cover_tests.rs:449:65 False [assert] (test): matches!(&refused, QuarantineError::Reason(text) if text.contains(said)),
  F? LINE   crates/store/src/quarantine_tests.rs:145 [panic] (test): panic!("expected a dropped generation, got {applied:?}");
  F? LINE   crates/store/src/quarantine_tests.rs:175 [panic] (test): panic!("the month is labelled");
  F? LINE   crates/store/src/quarantine_tests.rs:264 [panic] (test): panic!("the month is answered by name");
  F? LINE   crates/store/src/quarantine_tests.rs:551 [panic] (test): other => panic!("expected the hold named, got {other:?}"),
  F? LINE   crates/store/src/quarantine_tests.rs:800 [panic] (test): panic!("the month is still answered by name");
  F? LINE   crates/store/src/quarantine_tests.rs:892 [panic] (test): other => panic!("{why}: {other:?}"),
  F? LINE   crates/store/src/quarantine_tests.rs:1092 [panic] (test): other => panic!("expected the quarantine to refuse by name, got {other:?}"),
  F? LINE   crates/store/src/quarantine_tests.rs:1268 [panic] (test): other => panic!("the rerun indexes the drop: {other:?}"),
  F? LINE   crates/store/src/quarantine_tests.rs:1995 [panic] (test): other => panic!("{other:?}"),
  GAP BRANCH crates/store/src/quarantine_tests.rs:144:9 False (test): let Applied::Dropped { drop, .. } = applied else {
  GAP BRANCH crates/store/src/quarantine_tests.rs:174:9 False (test): let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {
  F? BRANCH crates/store/src/quarantine_tests.rs:210:63 False [assert] (test): assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));
  GAP BRANCH crates/store/src/quarantine_tests.rs:263:9 False (test): let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {
  F? BRANCH crates/store/src/quarantine_tests.rs:342:66 False [assert] (test): matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),
  F? BRANCH crates/store/src/quarantine_tests.rs:348:77 False [assert] (test): assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));
  F? BRANCH crates/store/src/quarantine_tests.rs:497:42 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  F? BRANCH crates/store/src/quarantine_tests.rs:497:71 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  F? BRANCH crates/store/src/quarantine_tests.rs:508:73 False [assert] (test): matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  F? BRANCH crates/store/src/quarantine_tests.rs:508:92 False [assert] (test): matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  GAP BRANCH crates/store/src/quarantine_tests.rs:799:9 False (test): let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {
  F? BRANCH crates/store/src/quarantine_tests.rs:855:38 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F? BRANCH crates/store/src/quarantine_tests.rs:855:70 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F? BRANCH crates/store/src/quarantine_tests.rs:1071:38 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  F? BRANCH crates/store/src/quarantine_tests.rs:1072:16 False [assert] (test): && e.bars == 6
  F? BRANCH crates/store/src/quarantine_tests.rs:1073:16 False [assert] (test): && e.causes.said() == "before_listing and foreign_level"
  F? BRANCH crates/store/src/quarantine_tests.rs:1327:38 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F? BRANCH crates/store/src/quarantine_tests.rs:1327:70 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F? BRANCH crates/store/src/quarantine_tests.rs:1386:38 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  F? BRANCH crates/store/src/quarantine_tests.rs:1386:70 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  F? BRANCH crates/store/src/quarantine_tests.rs:1419:38 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  F? BRANCH crates/store/src/quarantine_tests.rs:1466:38 False [assert] (test): Ok(Probe::Quarantined(e)) if e.bars == 3
  F? BRANCH crates/store/src/quarantine_tests.rs:1533:9 False [assert] (test): refused.is_err() && words.contains("quarantined"),
  F? BRANCH crates/store/src/quarantine_tests.rs:1533:29 False [assert] (test): refused.is_err() && words.contains("quarantined"),
  F? BRANCH crates/store/src/quarantine_tests.rs:1584:9 False [assert] (test): refused.is_err() && words.contains("quarantined"),
  F? BRANCH crates/store/src/quarantine_tests.rs:1584:29 False [assert] (test): refused.is_err() && words.contains("quarantined"),
  F? BRANCH crates/store/src/quarantine_tests.rs:1639:38 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  F? BRANCH crates/store/src/quarantine_tests.rs:1639:67 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  F? BRANCH crates/store/src/quarantine_tests.rs:1639:82 False [assert] (test): Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  F? BRANCH crates/store/src/quarantine_tests.rs:1747:58 False [assert] (test): matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  F? BRANCH crates/store/src/quarantine_tests.rs:1747:94 False [assert] (test): matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  F? BRANCH crates/store/src/quarantine_tests.rs:1865:77 False [assert] (test): matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),
  GAP LINE   crates/store/src/repair.rs:600: )?;
  GAP LINE   crates/store/src/repair.rs:643: return;
  GAP LINE   crates/store/src/repair.rs:851: rows: header.n_valid,
  GAP LINE   crates/store/src/repair.rs:852: ceiling,
  GAP LINE   crates/store/src/repair.rs:853: };
  GAP LINE   crates/store/src/repair.rs:1184: return Err(RepairError::ReadBack {
  GAP LINE   crates/store/src/repair.rs:1185: index: index as u64,
  GAP LINE   crates/store/src/repair.rs:1186: });
  GAP LINE   crates/store/src/repair.rs:1228: return;
  GAP LINE   crates/store/src/repair.rs:1342: )?;
  GAP LINE   crates/store/src/repair.rs:1374: return Err(RepairError::Conflict);
  GAP LINE   crates/store/src/repair.rs:1392: )?;
  GAP LINE   crates/store/src/repair.rs:1398: )?;
  GAP LINE   crates/store/src/repair.rs:1458: Err(TryLockError::Error(error)) => io_at(Err(error), &path, "lock shared", false),
  GAP BRANCH crates/store/src/repair.rs:642:9 False: let (Some(first), Some(last)) = (held_back.first(), held_back.last()) else {
  GAP BRANCH crates/store/src/repair.rs:1183:24 True: if reader.read_record(index as u64)? != *bar {
  GAP BRANCH crates/store/src/repair.rs:1219:9 False: let MonthRestate::Restated {
  GAP BRANCH crates/store/src/repair.rs:1373:12 True: if writer.read_record(index as u64)? != *offered {
  F? BRANCH crates/store/src/repair_cover_tests.rs:62:41 False [assert] (test): text.contains("refused") || text.contains("drop names"),
  GAP LINE   crates/store/src/repair_door_cover_tests.rs:221 (test): other @ MonthRestate::NothingNew { .. } => {
  F? LINE   crates/store/src/repair_door_cover_tests.rs:222 [panic] (test): panic!("the added bars are published: {other:?}")
  F? LINE   crates/store/src/repair_door_cover_tests.rs:246 [assert_eq] (test): other @ MonthRepair::NothingNew { .. } => {
  F? LINE   crates/store/src/repair_door_cover_tests.rs:247 [panic] (test): panic!("the empty month takes the bars: {other:?}")
  F? LINE   crates/store/src/repair_door_cover_tests.rs:323 [assert] (test): matches!(
  F? LINE   crates/store/src/repair_tests.rs:116 [panic] (test): Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),
  F? LINE   crates/store/src/repair_tests.rs:630 [assert_eq] (test): timeframe.as_str()
  GAP LINE   crates/store/src/repair_tests.rs:700 (test): }
  F? LINE   crates/store/src/repair_tests.rs:891 [panic] (test): panic!("expected a restated generation, got {done:?}");
  GAP BRANCH crates/store/src/repair_tests.rs:691:12 False (test): if let Ok(entries) = fs::read_dir(dir) {
  F? BRANCH crates/store/src/repair_tests.rs:824:94 False [assert] (test): matches!(&held, Err(RepairError::Store(crate::file::StoreError::Locked { path })) if *path == lock),
  GAP BRANCH crates/store/src/repair_tests.rs:882:9 False (test): let super::MonthRestate::Restated {
  GAP LINE   crates/store/src/safe_open.rs:107: Some(Self::Unknown)
  GAP LINE   crates/store/src/safe_open.rs:300: return Err(io(
  GAP LINE   crates/store/src/safe_open.rs:301: "open",
  GAP LINE   crates/store/src/safe_open.rs:302: io::Error::new(
  GAP LINE   crates/store/src/safe_open.rs:303: ErrorKind::Unsupported,
  GAP LINE   crates/store/src/safe_open.rs:304: "safe reads require verified macOS or Linux x86_64/aarch64 open flags",
  GAP LINE   crates/store/src/safe_open.rs:305: ),
  GAP LINE   crates/store/src/safe_open.rs:306: ));
  GAP BRANCH crates/store/src/safe_open.rs:104:19 False: } else if kind.is_block_device() {
  GAP BRANCH crates/store/src/safe_open.rs:299:9 False: let Some(flags) = FLAGS else {
  GAP LINE   crates/store/src/safe_open_tests.rs:67 (test): tried.push(format!("{}: {why}", link.display()));
  GAP LINE   crates/store/src/safe_open_tests.rs:68 (test): continue;
  GAP LINE   crates/store/src/safe_open_tests.rs:74 (test): Err(why) => tried.push(format!("{}: {why}", link.display())),
  F? LINE   crates/store/src/safe_open_tests.rs:77 [panic] (test): panic!(
  F? LINE   crates/store/src/safe_open_tests.rs:79 [panic] (test): path.display(),
  F? LINE   crates/store/src/safe_open_tests.rs:80 [panic] (test): tried.join("; ")
  F? LINE   crates/store/src/safe_open_tests.rs:103 [assert] (test): found.file_type()
  F? LINE   crates/store/src/safe_open_tests.rs:110 [assert] (test): link.display()
  GAP LINE   crates/store/src/safe_open_tests.rs:123 (test): let _release = fs::OpenOptions::new().write(true).open(pipe);
  F? LINE   crates/store/src/safe_open_tests.rs:124 [panic] (test): panic!("an open of {} waited for a writer", pipe.display());
  GAP LINE   crates/store/src/safe_open_tests.rs:269 (test): let refusal = open_read(&path).unwrap_err();
  F? LINE   crates/store/src/safe_open_tests.rs:270 [assert_eq] (test): assert_eq!((refusal.word(), refusal.found()), ("io", None), "{refusal}");
  F? LINE   crates/store/src/safe_open_tests.rs:271 [assert] (test): assert!(!refusal.is_missing());
  F? LINE   crates/store/src/safe_open_tests.rs:401 [panic] (test): Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),
  F? LINE   crates/store/src/safe_open_tests.rs:402 [panic] (test): Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),
  GAP BRANCH crates/store/src/safe_open_tests.rs:66:16 True (test): if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
  F? BRANCH crates/store/src/safe_open_tests.rs:95:42 False [assert] (test): matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  GAP BRANCH crates/store/src/safe_open_tests.rs:122:9 False (test): let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {
  GAP BRANCH crates/store/src/safe_open_tests.rs:203:12 False (test): if let Some(device) = a_block_device() {
  GAP BRANCH crates/store/src/safe_open_tests.rs:268:8 True (test): if fs::File::open(&path).is_err() {
  GAP BRANCH crates/store/src/safe_open_tests.rs:298:16 False (test): if *root == shadow && *bars == shadow.join(BARS_DIR)
  GAP BRANCH crates/store/src/safe_open_tests.rs:298:35 False (test): if *root == shadow && *bars == shadow.join(BARS_DIR)
  GAP BRANCH crates/store/src/safe_open_tests.rs:299:20 False (test): && target.as_deref() == Some(real.join(BARS_DIR).as_path())),
  F? BRANCH crates/store/src/safe_open_tests.rs:459:77 False [assert] (test): matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),
  GAP BRANCH crates/store/src/sealed.rs:1499:23 False: } else if let Some(range) = self.ranges.get_mut(5) {
  GAP BRANCH crates/store/src/sealed.rs:1504:15 False: while self.table.len() < self.table_rows && self.next_start <= i128::from(ts) {
  GAP BRANCH crates/store/src/sealed.rs:1525:16 False: if let Some(oi) = columns.get_mut(5) {
  GAP BRANCH crates/store/src/sealed.rs:2352:35 False: Some(previous) if block > 0 => entries.get(previous).map_or(0, |entry| entry.end),
  F? LINE   crates/store/src/sealed_codec_tests.rs:81 [assert] (test): frame.len()
  F? LINE   crates/store/src/sealed_codec_tests.rs:282 [assert] (test): frame.len()
  F? LINE   crates/store/src/sealed_codec_tests.rs:439 [assert] (test): largest[rows],
  F? LINE   crates/store/src/sealed_codec_tests.rs:440 [assert] (test): max_frame_len(m)
  F? BRANCH crates/store/src/sealed_codec_tests.rs:1511:21 False [assert] (test): assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  F? BRANCH crates/store/src/sealed_codec_tests.rs:1511:38 False [assert] (test): assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  F? BRANCH crates/store/src/sealed_codec_tests.rs:1556:13 False [assert] (test): last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  F? BRANCH crates/store/src/sealed_codec_tests.rs:1556:33 False [assert] (test): last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  F? LINE   crates/store/src/sealed_tests.rs:159 [panic] (test): Ok(value) => panic!("expected a refusal, got {value:?}"),
  GAP LINE   crates/store/src/sealed_tests.rs:499 (test): Ok(())
  F? LINE   crates/store/src/sealed_tests.rs:502 [panic] (test): Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),
  F? LINE   crates/store/src/sealed_tests.rs:653 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/sealed_tests.rs:837 [panic] (test): other => panic!("{field}: {other}"),
  GAP LINE   crates/store/src/sealed_tests.rs:939 (test): fn flush(&mut self) -> io::Result<()> {
  GAP LINE   crates/store/src/sealed_tests.rs:940 (test): Ok(())
  GAP LINE   crates/store/src/sealed_tests.rs:941 (test): }
  F? LINE   crates/store/src/sealed_tests.rs:1170 [assert] (test): packed.len(),
  F? LINE   crates/store/src/sealed_tests.rs:1171 [assert] (test): image.len()
  F? LINE   crates/store/src/sealed_tests.rs:1192 [panic] (test): other => panic!("{other:?}"),
  F? LINE   crates/store/src/sealed_tests.rs:1241 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/sealed_tests.rs:1319 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/sealed_tests.rs:1656 [panic] (test): other => panic!("{field}: {other}"),
  F? LINE   crates/store/src/sealed_tests.rs:1673 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/sealed_tests.rs:1689 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/sealed_tests.rs:1774 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/sealed_tests.rs:1802 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/sealed_tests.rs:1891 [panic] (test): other => panic!("{other}"),
  F? LINE   crates/store/src/sealed_tests.rs:1982 [assert] (test): matches!(
  F? LINE   crates/store/src/sealed_tests.rs:2104 [panic] (test): other => panic!("{other}"),
  GAP BRANCH crates/store/src/sealed_tests.rs:237:12 False (test): if offset <= self.fail_at && self.fail_at < offset + buf.len() as u64 {
  F? BRANCH crates/store/src/sealed_tests.rs:696:27 False [assert] (test): found == 0 || bars[found as usize - 1].ts_micros < ts,
  F? BRANCH crates/store/src/sealed_tests.rs:1816:100 False [assert] (test): matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)
  F? BRANCH crates/store/src/sealed_tests.rs:1822:105 False [assert] (test): matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  F? BRANCH crates/store/src/sealed_tests.rs:1828:105 False [assert] (test): matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)
  F? BRANCH crates/store/src/sealed_tests.rs:1836:105 False [assert] (test): matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  GAP LINE   crates/store/src/settings.rs:886: return Err(fault(format!(
  GAP LINE   crates/store/src/settings.rs:887: "it grew past the {MAX_FILE_BYTES} bytes a settings file may hold while it was read"
  GAP LINE   crates/store/src/settings.rs:888: )));
  GAP LINE   crates/store/src/settings.rs:1227: _ => ErrorKind::AlreadyExists,
  GAP LINE   crates/store/src/settings.rs:1235: let _ = std::fs::remove_file(&temp);
  GAP LINE   crates/store/src/settings.rs:1236: return Err(io_refusal(&temp, &why));
  GAP LINE   crates/store/src/settings.rs:1321: path: fault.path,
  GAP LINE   crates/store/src/settings.rs:1322: kind: ErrorKind::Other,
  GAP LINE   crates/store/src/settings.rs:1323: why: format!("the settings directory cannot be read: {}", fault.why),
  GAP LINE   crates/store/src/settings.rs:1324: })?;
  GAP LINE   crates/store/src/settings.rs:1367: match linked {
  GAP LINE   crates/store/src/settings.rs:1369: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
  GAP LINE   crates/store/src/settings.rs:1370: let _ = reader.refresh();
  GAP LINE   crates/store/src/settings.rs:1371: return Err(Refusal::Stale {
  GAP LINE   crates/store/src/settings.rs:1372: expected: change.expected,
  GAP LINE   crates/store/src/settings.rs:1373: newest: reader.highest().max(number),
  GAP LINE   crates/store/src/settings.rs:1374: });
  GAP LINE   crates/store/src/settings.rs:1376: Err(why) => return Err(io_refusal(&target, &why)),
  GAP LINE   crates/store/src/settings.rs:1445: Ok(_) => "it reads when read again, and the refresh did not step to it".to_owned(),
  GAP BRANCH crates/store/src/settings.rs:220:16 False: if let Some(slot) = self.values.get_mut(key.slot()) {
  GAP BRANCH crates/store/src/settings.rs:688:30 False: let percent_too = if unit.is_empty() && matches!(amount, 1..=95) {
  GAP BRANCH crates/store/src/settings.rs:885:8 True: if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
  GAP BRANCH crates/store/src/settings.rs:1234:12 True: if let Err(why) = written {
  GAP BRANCH crates/store/src/settings.rs:1369:21 True: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
  GAP BRANCH crates/store/src/settings.rs:1369:21 False: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
  F? BRANCH crates/store/src/settings_cover_tests.rs:198:57 False [assert] (test): Err(Refusal::Invalid { key: None, why }) if why.contains("1 to 240 bytes")
  F? LINE   crates/store/src/settings_door_cover_tests.rs:116 [panic] (test): other => panic!("the listing refuses the save: {other:?}"),
  F? BRANCH crates/store/src/settings_door_cover_tests.rs:150:9 False [assert] (test): fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
  F? BRANCH crates/store/src/settings_door_cover_tests.rs:150:51 False [assert] (test): fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
  F? LINE   crates/store/src/settings_tests.rs:152 [panic] (test): other => panic!("{bad:?} for {key:?}: {other:?}"),
  F? LINE   crates/store/src/settings_tests.rs:228 [assert] (test): matches!(
  GAP LINE   crates/store/src/settings_tests.rs:261 (test): Ok(())
  F? LINE   crates/store/src/settings_tests.rs:414 [assert] (test): reader.faults()
  F? LINE   crates/store/src/settings_tests.rs:580 [assert_eq] (test): reader.faults()
  F? LINE   crates/store/src/settings_tests.rs:651 [assert] (test): reader.stopped()
  GAP LINE   crates/store/src/settings_tests.rs:709 (test): if let Some(pipe) = pipe {
  GAP LINE   crates/store/src/settings_tests.rs:710 (test): let _release = std::fs::OpenOptions::new().write(true).open(pipe);
  GAP LINE   crates/store/src/settings_tests.rs:711 (test): }
  F? LINE   crates/store/src/settings_tests.rs:712 [panic] (test): panic!("{what}: the settings read waited instead of naming a fault");
  F? LINE   crates/store/src/settings_tests.rs:990 [assert] (test): matches!(
  GAP LINE   crates/store/src/settings_tests.rs:1154 (test): return None;
  GAP BRANCH crates/store/src/settings_tests.rs:255:12 False (test): if s.get(Key::OpenFeed) == Some("dhan") {
  GAP BRANCH crates/store/src/settings_tests.rs:708:9 False (test): let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  GAP BRANCH crates/store/src/settings_tests.rs:709:16 True (test): if let Some(pipe) = pipe {
  GAP BRANCH crates/store/src/settings_tests.rs:709:16 False (test): if let Some(pipe) = pipe {
  F? BRANCH crates/store/src/settings_tests.rs:786:48 False [assert] (test): matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),
  F? BRANCH crates/store/src/settings_tests.rs:910:60 False [assert] (test): matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),
  F? BRANCH crates/store/src/settings_tests.rs:956:60 False [assert] (test): matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("exhausted")),
  F? BRANCH crates/store/src/settings_tests.rs:1097:9 False [assert] (test): read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  F? BRANCH crates/store/src/settings_tests.rs:1097:52 False [assert] (test): read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  GAP BRANCH crates/store/src/settings_tests.rs:1153:20 True (test): if started.elapsed() > std::time::Duration::from_secs(10) {
  F? BRANCH crates/store/src/settings_tests.rs:1187:13 True [assert] (test): a.superseded_by == Some(3) || a.in_force == Some(2),
  F? BRANCH crates/store/src/settings_tests.rs:1187:43 False [assert] (test): a.superseded_by == Some(3) || a.in_force == Some(2),
  F? BRANCH crates/store/src/small_month_tests.rs:152:52 False [assert] (test): Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1
  F? BRANCH crates/store/src/sort_index_cover_tests.rs:294:38 False [assert] (test): Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
  F? BRANCH crates/store/src/sort_index_cover_tests.rs:301:38 False [assert] (test): Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
  F? BRANCH crates/store/src/sort_index_cover_tests.rs:305:38 False [assert] (test): Err(Why::Unreadable(why)) if why.contains("is not a regular file")
  F? BRANCH crates/store/src/sort_index_cover_tests.rs:312:38 False [assert] (test): Err(Why::Unreadable(why)) if why.contains("is a symbolic link")
  F? BRANCH crates/store/src/sort_index_cover_tests.rs:318:38 False [assert] (test): Err(Why::Unreadable(why)) if why.starts_with(&under.display().to_string())
  F? BRANCH crates/store/src/sort_index_tests.rs:324:41 False [assert] (test): assert!(why.contains("kind") || why.contains("reserved"), "{why}");
  F? LINE   crates/store/src/staging_cover_tests.rs:56 [panic] (test): Err(other) => panic!("the removal is refused by name: {other:?}"),
  F? LINE   crates/store/src/staging_cover_tests.rs:57 [panic] (test): Ok(_) => panic!("the removal is refused by name, not staged"),
  F? LINE   crates/store/src/staging_tests.rs:421 [assert_eq] (test): within.display()
  GAP BRANCH crates/store/src/staging_tests.rs:143:16 False (test): if source.kind() == ErrorKind::PermissionDenied && *p == path),
  GAP BRANCH crates/store/src/staging_tests.rs:143:64 False (test): if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F? BRANCH crates/store/src/staging_tests.rs:206:67 False [assert] (test): matches!(&refusal, StagingError::Contended { path: p } if *p == path),
  GAP BRANCH crates/store/src/staging_tests.rs:238:16 False (test): if source.kind() == ErrorKind::PermissionDenied && *p == path),
  GAP BRANCH crates/store/src/staging_tests.rs:238:64 False (test): if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F? BRANCH crates/store/src/staging_tests.rs:297:78 False [assert] (test): matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
  F? BRANCH crates/store/src/staging_tests.rs:334:88 False [assert] (test): matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),
  F? BRANCH crates/store/src/staging_tests.rs:369:77 False [assert] (test): matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
  F? BRANCH crates/store/src/time_index.rs:921:21 False [assert] (test): assert!(entry_reads <= 1 && bar_reads <= 1);
  F? BRANCH crates/store/src/time_index.rs:921:41 False [assert] (test): assert!(entry_reads <= 1 && bar_reads <= 1);
  F? BRANCH crates/store/src/time_index.rs:1057:17 False [assert] (test): assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  F? BRANCH crates/store/src/time_index.rs:1057:38 False [assert] (test): assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  F? BRANCH crates/store/src/time_index.rs:1057:59 False [assert] (test): assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  F? BRANCH crates/store/src/time_index.rs:1058:17 False [assert] (test): assert!(!entry.occupied(1) && !entry.occupied(62));
  F? BRANCH crates/store/src/time_index.rs:1058:39 False [assert] (test): assert!(!entry.occupied(1) && !entry.occupied(62));
  F? BRANCH crates/store/src/time_index.rs:1105:17 False [assert] (test): assert!(!daily.exact() && g.exact());
  F? BRANCH crates/store/src/time_index.rs:1105:35 False [assert] (test): assert!(!daily.exact() && g.exact());
  GAP LINE   crates/store/src/verdict.rs:717: return None;
  GAP LINE   crates/store/src/verdict.rs:3924: return Err(VerdictError::ImageMissing { path });
  GAP LINE   crates/store/src/verdict.rs:4076: return Err(VerdictError::ImageMissing { path });
  GAP LINE   crates/store/src/verdict.rs:4137: return Err(VerdictError::ImageMissing { path });
  F? LINE   crates/store/src/verdict.rs:4219 [panic] (test): other => panic!("not image_corrupt: {other:?}"),
  F? LINE   crates/store/src/verdict.rs:4227 [panic] (test): other => panic!("not written_by_newer_build: {other:?}"),
  GAP BRANCH crates/store/src/verdict.rs:716:8 True: if Vendor::ALL.get(at) != Some(&feed) {
  GAP BRANCH crates/store/src/verdict.rs:2278:16 False (test): if let Some(slot) = out.first.get_mut(ordinal) {
  GAP BRANCH crates/store/src/verdict.rs:2312:16 False (test): if let Some(count) = out.counts.get_mut(usize::from(code.code())) {
  GAP BRANCH crates/store/src/verdict.rs:2478:16 False (test): if let Some(c) = counts.get_mut(usize::from(record.facts.code.code())) {
  GAP BRANCH crates/store/src/verdict.rs:3143:15 False: while let Some(parent) = at.parent() {
  GAP BRANCH crates/store/src/verdict.rs:3417:12 False: if let Some(entry) = days.get_mut(doy) {
  GAP BRANCH crates/store/src/verdict.rs:3853:24 False: && let Some(oldest) = self
  GAP BRANCH crates/store/src/verdict.rs:3923:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
  GAP BRANCH crates/store/src/verdict.rs:4075:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
  GAP BRANCH crates/store/src/verdict.rs:4136:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
  GAP BRANCH crates/store/src/verdict.rs:4637:25 False (test): .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  GAP BRANCH crates/store/src/verdict.rs:4637:37 True (test): .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  GAP BRANCH crates/store/src/verdict.rs:4637:37 False (test): .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  GAP BRANCH crates/store/src/verdict.rs:4974:24 False (test): .find(|&i| mark(&bytes, i) != 0 && tag(&bytes, i) == other_tag)
  F? LINE   crates/store/src/verdict_tests.rs:366 [panic] (test): other => panic!("expected the host's refusal, got {other}"),
  GAP LINE   crates/store/src/verdict_tests.rs:461 (test): let _release = fs::OpenOptions::new().write(true).open(pipe);
  F? LINE   crates/store/src/verdict_tests.rs:462 [panic] (test): panic!("a verdict call waited on the pipe at {}", pipe.display());
  F? LINE   crates/store/src/verdict_tests.rs:568 [panic] (test): other => panic!("expected the host's refusal, got {other}"),
  F? LINE   crates/store/src/verdict_tests.rs:813 [panic] (test): other => panic!("not image_corrupt: {other:?}"),
  F? LINE   crates/store/src/verdict_tests.rs:879 [assert_eq] (test): other.dir().display()
  F? LINE   crates/store/src/verdict_tests.rs:951 [panic] (test): panic!("the key's record: {:?}", found.found);
  GAP BRANCH crates/store/src/verdict_tests.rs:460:9 False (test): let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  GAP BRANCH crates/store/src/verdict_tests.rs:950:9 False (test): let super::Found::Record { ordinal, record } = found.found else {
  GAP LINE   crates/store/tests/catalog.rs:308 (test): match made {
  GAP LINE   crates/store/tests/catalog.rs:310 (test): Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {
  GAP LINE   crates/store/tests/catalog.rs:311 (test): refused_by_the_volume += 1;
  GAP LINE   crates/store/tests/catalog.rs:312 (test): }
  F? LINE   crates/store/tests/catalog.rs:313 [panic] (test): Err(why) => panic!("{} could not be made: {why}", full.display()),
  GAP LINE   crates/store/tests/catalog.rs:321 (test): eprintln!(
  GAP LINE   crates/store/tests/catalog.rs:322 (test): "this volume refused all {refused_by_the_volume} non-UTF-8 names (os error 92); \
  GAP LINE   crates/store/tests/catalog.rs:323 (test): the walk is checked on what it holds and the five shapes by \
  GAP LINE   crates/store/tests/catalog.rs:324 (test): every_non_utf8_shape_is_counted_by_the_walks_own_step_on_every_host"
  GAP LINE   crates/store/tests/catalog.rs:325 (test): );
  GAP LINE   crates/store/tests/catalog.rs:815 (test): tried.push(format!("{}: {why}", link.display()));
  GAP LINE   crates/store/tests/catalog.rs:816 (test): continue;
  GAP LINE   crates/store/tests/catalog.rs:822 (test): Err(why) => tried.push(format!("{}: {why}", link.display())),
  F? LINE   crates/store/tests/catalog.rs:825 [panic] (test): panic!(
  F? LINE   crates/store/tests/catalog.rs:827 [panic] (test): path.display(),
  F? LINE   crates/store/tests/catalog.rs:828 [panic] (test): tried.join("; ")
  F? LINE   crates/store/tests/catalog.rs:852 [assert] (test): found.file_type()
  F? LINE   crates/store/tests/catalog.rs:859 [assert] (test): link.display()
  F? LINE   crates/store/tests/catalog.rs:875 [assert] (test): path.display()
  GAP BRANCH crates/store/tests/catalog.rs:68:12 False (test): if let Some(parent) = full.parent() {
  F? BRANCH crates/store/tests/catalog.rs:317:9 False [assert] (test): refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  F? BRANCH crates/store/tests/catalog.rs:317:39 True [assert] (test): refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  F? BRANCH crates/store/tests/catalog.rs:317:39 False [assert] (test): refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  GAP BRANCH crates/store/tests/catalog.rs:320:8 True (test): if refused_by_the_volume > 0 {
  GAP BRANCH crates/store/tests/catalog.rs:814:16 True (test): if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
  F? BRANCH crates/store/tests/catalog.rs:844:42 False [assert] (test): matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  GAP LINE   crates/store/tests/cited_commits.rs:289 (test): println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");
  GAP LINE   crates/store/tests/cited_commits.rs:290 (test): return false;
  GAP LINE   crates/store/tests/cited_commits.rs:568 (test): faults.push(format!(
  GAP LINE   crates/store/tests/cited_commits.rs:571 (test): continue;
  GAP LINE   crates/store/tests/cited_commits.rs:575 (test): faults.push(format!(
  GAP LINE   crates/store/tests/cited_commits.rs:576 (test): "{commit} is cited in {} by a sentence that does not say \"{says}\": {}",
  GAP LINE   crates/store/tests/cited_commits.rs:577 (test): at.name, at.sentence
  GAP LINE   crates/store/tests/cited_commits.rs:578 (test): ));
  GAP LINE   crates/store/tests/cited_commits.rs:579 (test): }
  GAP LINE   crates/store/tests/cited_commits.rs:602 (test): let names: BTreeSet<&str> = citing.iter().map(|at| at.name.as_str()).collect();
  GAP LINE   crates/store/tests/cited_commits.rs:603 (test): faults.push(format!(
  GAP LINE   crates/store/tests/cited_commits.rs:611 (test): faults.push(format!("{commit} is listed as {off:?}, and it is on main"));
  GAP LINE   crates/store/tests/cited_commits.rs:614 (test): faults.push(format!(
  GAP LINE   crates/store/tests/cited_commits.rs:615 (test): "{commit} is listed as {off:?}, and it is an ancestor of HEAD"
  GAP LINE   crates/store/tests/cited_commits.rs:616 (test): ));
  F? LINE   crates/store/tests/cited_commits.rs:673 [assert] (test): cited.len()
  F? LINE   crates/store/tests/cited_commits.rs:709 [assert] (test): String::from_utf8_lossy(&out.stderr)
  GAP LINE   crates/store/tests/cited_commits.rs:732 (test): println!("SKIPPING: git cannot run the synthetic history fixture");
  GAP LINE   crates/store/tests/cited_commits.rs:733 (test): return;
  GAP LINE   crates/store/tests/cited_commits.rs:820 (test): println!("SKIPPING: git cannot run the synthetic history fixture");
  GAP LINE   crates/store/tests/cited_commits.rs:821 (test): return;
  F? LINE   crates/store/tests/cited_commits.rs:976 [assert] (test): sites.len()
  GAP LINE   crates/store/tests/cited_commits.rs:979 (test): return;
  GAP LINE   crates/store/tests/cited_commits.rs:997 (test): wrong.push(format!("{module}.rs:{number}: {}", line.trim()));
  GAP BRANCH crates/store/tests/cited_commits.rs:288:12 True (test): if let Err(why) = Command::new("git").arg("--version").output() {
  GAP BRANCH crates/store/tests/cited_commits.rs:294:21 False (test): Err(why) if why.kind() == std::io::ErrorKind::NotFound
  GAP BRANCH crates/store/tests/cited_commits.rs:306:12 False (test): if let Some(above) = root.parent() {
  F? BRANCH crates/store/tests/cited_commits.rs:371:9 False [assert] (test): said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  F? BRANCH crates/store/tests/cited_commits.rs:372:16 False [assert] (test): && said.contains("not a git repository"),
  F? BRANCH crates/store/tests/cited_commits.rs:384:9 False [assert] (test): said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  F? BRANCH crates/store/tests/cited_commits.rs:385:16 False [assert] (test): && said.contains("not a git repository"),
  GAP BRANCH crates/store/tests/cited_commits.rs:567:13 False (test): let Some(citing) = cited.get(commit) else {
  GAP BRANCH crates/store/tests/cited_commits.rs:601:16 True (test): if !is_ancestor_at(root, commit, main_ref) {
  GAP BRANCH crates/store/tests/cited_commits.rs:610:12 True (test): if is_ancestor_at(root, commit, main_ref) {
  GAP BRANCH crates/store/tests/cited_commits.rs:613:35 True (test): if off == Off::Neither && is_ancestor_at(root, commit, "HEAD") {
  GAP BRANCH crates/store/tests/cited_commits.rs:676:8 False (test): if history_is_here() {
  GAP BRANCH crates/store/tests/cited_commits.rs:731:8 True (test): if Command::new("git").arg("--version").output().is_err() {
  GAP BRANCH crates/store/tests/cited_commits.rs:819:8 True (test): if Command::new("git").arg("--version").output().is_err() {
  GAP BRANCH crates/store/tests/cited_commits.rs:898:39 False (test): } else if token.len() == 2 && token.starts_with('v') {
  GAP BRANCH crates/store/tests/cited_commits.rs:903:19 False (test): } else if !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit() || b == b'/') {
  GAP BRANCH crates/store/tests/cited_commits.rs:978:8 True (test): if !history_is_here() {
  GAP BRANCH crates/store/tests/cited_commits.rs:996:12 True (test): if !takes_a_lock(line) {
  F? LINE   crates/store/tests/cloud_mutants_store.rs:367 [panic] (test): other => panic!("{door}: {other:?}"),
  F? LINE   crates/store/tests/cloud_mutants_store.rs:371 [panic] (test): panic!("a moved revision is audited");
  F? LINE   crates/store/tests/cloud_mutants_store.rs:446 [panic] (test): other => panic!("{other:?}"),
  F? LINE   crates/store/tests/cloud_mutants_store.rs:1073 [panic] (test): other => panic!("not image_corrupt: {other:?}"),
  F? LINE   crates/store/tests/cloud_mutants_store.rs:1098 [panic] (test): panic!("ZZB is in the day");
  F? LINE   crates/store/tests/cloud_mutants_store.rs:1171 [assert] (test): matches!(
  F? LINE   crates/store/tests/cloud_mutants_store.rs:1361 [panic] (test): other => panic!("{other:?}"),
  F? LINE   crates/store/tests/cloud_mutants_store.rs:1568 [panic] (test): other => panic!("{other:?}"),
  F? LINE   crates/store/tests/cloud_mutants_store.rs:1925 [panic] (test): panic!("{name} is in the day");
  GAP LINE   crates/store/tests/cloud_mutants_store.rs:2385 (test): return Err(std::io::ErrorKind::UnexpectedEof.into());
  F? LINE   crates/store/tests/cloud_mutants_store.rs:2876 [assert] (test): writer.time_lookup()
  GAP LINE   crates/store/tests/cloud_mutants_store.rs:3172 (test): return Err(std::io::Error::other("flushed alone"));
  GAP LINE   crates/store/tests/cloud_mutants_store.rs:3253 (test): _ => None,
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:144:48 False (test): while (vanished < 20 || contended < 20) && Instant::now() < deadline {
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:162:9 False [assert] (test): vanished >= 20 && contended >= 20,
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:162:27 False [assert] (test): vanished >= 20 && contended >= 20,
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:201:18 False (test): } if source.kind() == std::io::ErrorKind::PermissionDenied
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:370:9 False (test): let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:374:9 False [assert] (test): audited.starts_with("repair revision 1 of ")
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:375:16 False [assert] (test): && audited.ends_with(" is not the one its promotion recorded; not audited"),
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1097:9 False (test): let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:1315:9 False [assert] (test): generation.contains("/nonexistent/cloud/2031-02.tix")
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:1316:16 False [assert] (test): && generation.contains("revision 1 is not the one its promotion recorded"),
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1642:20 False (test): if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1642:37 False (test): if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1686:20 False (test): if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1686:37 False (test): if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1720:24 False (test): if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1720:38 False (test): if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1823:20 False (test): if path == &lock_path && *tries == steps + 1
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1823:42 False (test): if path == &lock_path && *tries == steps + 1
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1923:13 False (test): let Some(Found::Record { ordinal, record }) = lookup.answer.map(|answer| answer.found)
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1986:20 False (test): if path.parent() == Some(year.as_path())
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1987:24 False (test): && path.extension().is_some_and(|ext| ext == "tmp")
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:1988:24 False (test): && source.kind() == std::io::ErrorKind::PermissionDenied
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:2256:94 False [assert] (test): TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { path, .. })) if *path == tix
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:2349:20 False (test): if *path == bar_path && *offset == 32_768 + 56 * 4
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:2349:41 False (test): if *path == bar_path && *offset == 32_768 + 56 * 4
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:2384:13 False (test): let Some(bytes) = self.bytes.get(at..at + buf.len()) else {
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:2636:20 False [assert] (test): if *path == bar_path
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:2873:80 False [assert] (test): TimeLookup::Bisection(Why::SharedSlot { index: 10, ts_micros }) if ts_micros == second.ts_micros
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:2962:20 False (test): if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:2962:40 False (test): if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:3073:76 False [assert] (test): StoreError::NotADirectory { path, action: Action::Measure } if *path == bars.to_path_buf(&other)
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:3124:59 False [assert] (test): matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  GAP BRANCH crates/store/tests/cloud_mutants_store.rs:3171:20 True (test): if since.elapsed() > std::time::Duration::from_secs(5) {
  F? BRANCH crates/store/tests/cloud_mutants_store.rs:3357:59 False [assert] (test): matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  GAP LINE   crates/store/tests/docs.rs:226 (test): }
  GAP LINE   crates/store/tests/docs.rs:227 (test): let entry = DECISIONS_DOC
  GAP LINE   crates/store/tests/docs.rs:228 (test): .split("\n### D-0790 ")
  GAP LINE   crates/store/tests/docs.rs:229 (test): .nth(1)
  GAP LINE   crates/store/tests/docs.rs:230 (test): .and_then(|rest| rest.split("\n### ").next())
  GAP LINE   crates/store/tests/docs.rs:231 (test): .map(flat)
  GAP LINE   crates/store/tests/docs.rs:232 (test): .expect("docs/05-decisions.md has a D-0790 entry");
  F? LINE   crates/store/tests/docs.rs:233 [assert] (test): assert!(
  F? LINE   crates/store/tests/docs.rs:234 [assert] (test): entry.contains(
  F? LINE   crates/store/tests/docs.rs:235 [assert] (test): "The paragraph under C-01 in `docs/04-invariants.md` also says no bench times a syscall"
  F? LINE   crates/store/tests/docs.rs:236 [assert] (test): ) && entry.contains("this entry supersedes that sentence"),
  F? LINE   crates/store/tests/docs.rs:239 [assert] (test): assert!(
  F? LINE   crates/store/tests/docs.rs:240 [assert] (test): INVARIANTS_DOC.contains("\n| C4-DOCS-WEB-07 | While the C-01 paragraph"),
  F? BRANCH crates/store/tests/docs.rs:217:9 False [assert] (test): BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  F? BRANCH crates/store/tests/docs.rs:217:47 False [assert] (test): BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  GAP BRANCH crates/store/tests/docs.rs:224:8 False (test): if !flat(INVARIANTS_DOC).contains("no bench in this repository times a syscall") {
  F? BRANCH crates/store/tests/docs.rs:234:9 True [assert] (test): entry.contains(
  F? BRANCH crates/store/tests/docs.rs:234:9 False [assert] (test): entry.contains(
  F? BRANCH crates/store/tests/docs.rs:236:14 True [assert] (test): ) && entry.contains("this entry supersedes that sentence"),
  F? BRANCH crates/store/tests/docs.rs:236:14 False [assert] (test): ) && entry.contains("this entry supersedes that sentence"),
  F? BRANCH crates/store/tests/docs.rs:253:9 False [assert] (test): FILE_RS.contains("if claimed > header.n_valid {")
  F? BRANCH crates/store/tests/docs.rs:254:16 False [assert] (test): && FILE_RS.contains("source: FormatError::CounterExceedsFile,"),
  F? LINE   crates/store/tests/durability.rs:381 [panic] (test): panic!(
  F? LINE   crates/store/tests/fault.rs:376 [panic] (test): panic!("bit {bit} of byte {byte} took the whole file down: {e}")
  F? BRANCH crates/store/tests/fault.rs:143:36 False [assert] (test): read.n_valid == 150 || read.n_valid == 175,
  F? BRANCH crates/store/tests/fault.rs:220:59 False [assert] (test): horizon == previous_commit.durable_through || horizon == commit.durable_through,
  F? LINE   crates/store/tests/fifo.rs:108 [assert] (test): at.display()
  GAP LINE   crates/store/tests/fifo.rs:124 (test): }
  GAP LINE   crates/store/tests/fifo.rs:126 (test): drop(fs::OpenOptions::new().write(true).open(release));
  GAP LINE   crates/store/tests/fifo.rs:127 (test): drop(worker.join());
  F? LINE   crates/store/tests/fifo.rs:128 [panic] (test): panic!(
  F? LINE   crates/store/tests/fifo.rs:130 [panic] (test): release.display()
  GAP LINE   crates/store/tests/fifo.rs:213 (test): return;
  GAP BRANCH crates/store/tests/fifo.rs:121:12 False (test): if let Ok(outcome) = recv.recv_timeout(BOUND) {
  F? BRANCH crates/store/tests/fifo.rs:157:13 False [assert] (test): rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  F? BRANCH crates/store/tests/fifo.rs:157:61 False [assert] (test): rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  F? BRANCH crates/store/tests/fifo.rs:162:53 False [assert] (test): fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),
  GAP BRANCH crates/store/tests/fifo.rs:212:8 True (test): if !null.exists() {
  F? LINE   crates/store/tests/generation.rs:165 [panic] (test): panic!("{done:?}");
  F? LINE   crates/store/tests/generation.rs:217 [panic] (test): other => panic!("{other:?}"),
  F? LINE   crates/store/tests/generation.rs:238 [panic] (test): other => panic!("{other:?}"),
  F? LINE   crates/store/tests/generation.rs:331 [assert] (test): matches!(
  F? LINE   crates/store/tests/generation.rs:367 [panic] (test): panic!("repaired");
  F? LINE   crates/store/tests/generation.rs:388 [panic] (test): panic!("{second:?}");
  F? LINE   crates/store/tests/generation.rs:475 [panic] (test): other => panic!("{other:?}"),
  F? LINE   crates/store/tests/generation.rs:517 [panic] (test): other => panic!("{other:?}"),
  F? LINE   crates/store/tests/generation.rs:758 [panic] (test): panic!("{repaired:?}");
  GAP BRANCH crates/store/tests/generation.rs:153:9 False (test): let MonthRepair::Repaired {
  F? BRANCH crates/store/tests/generation.rs:179:57 False [assert] (test): assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  F? BRANCH crates/store/tests/generation.rs:179:76 False [assert] (test): assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  GAP BRANCH crates/store/tests/generation.rs:364:9 False (test): let MonthRepair::Repaired { to, skipped, .. } =
  GAP BRANCH crates/store/tests/generation.rs:384:9 False (test): let MonthRepair::Repaired {
  F? BRANCH crates/store/tests/generation.rs:670:13 False [assert] (test): assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  F? BRANCH crates/store/tests/generation.rs:670:42 True [assert] (test): assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  F? BRANCH crates/store/tests/generation.rs:670:42 False [assert] (test): assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  F? BRANCH crates/store/tests/generation.rs:694:74 False [assert] (test): matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),
  GAP BRANCH crates/store/tests/generation.rs:749:9 False (test): let MonthRepair::Repaired {
  F? BRANCH crates/store/tests/geometry.rs:99:13 False [assert] (test): start >= block_start && end <= block_end,
  F? BRANCH crates/store/tests/geometry.rs:99:37 False [assert] (test): start >= block_start && end <= block_end,
  GAP BRANCH crates/store/tests/geometry.rs:134:13 True (test): start < block_start || end > block_end
  GAP LINE   crates/store/tests/libm_key.rs:205 (test): matches!(
  F? LINE   crates/store/tests/libm_key.rs:1330 [assert_eq] (test): if mentions { "is not" } else { "is" }
  F? BRANCH crates/store/tests/libm_key.rs:428:9 False [assert] (test): decision < caveat && caveat < rejected,
  F? BRANCH crates/store/tests/libm_key.rs:428:30 False [assert] (test): decision < caveat && caveat < rejected,
  F? BRANCH crates/store/tests/libm_key.rs:1330:16 True [assert_eq] (test): if mentions { "is not" } else { "is" }
  F? BRANCH crates/store/tests/libm_key.rs:1330:16 False [assert_eq] (test): if mentions { "is not" } else { "is" }
  F? LINE   crates/store/tests/misfiled.rs:184 [panic] (test): other => panic!("expected Misfiled, found {other:?}"),
  F? LINE   crates/store/tests/misfiled.rs:321 [assert_eq] (test): unborn.len()
  GAP BRANCH crates/store/tests/misfiled.rs:125:16 True (test): let y = if month <= 2 { year - 1 } else { year };
  GAP BRANCH crates/store/tests/misfiled.rs:129:26 False (test): let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i32::from(day) - 1;
  F? BRANCH crates/store/tests/misfiled.rs:217:9 False [assert] (test): said.contains("2019-11-28-1250000-CE")
  F? BRANCH crates/store/tests/misfiled.rs:218:16 False [assert] (test): && said.contains("its store path is not this month's")
  F? BRANCH crates/store/tests/misfiled.rs:219:16 False [assert] (test): && said.contains("DB-B65"),
  F? LINE   crates/store/tests/month_repair.rs:156 [panic] (test): other => panic!("not a repair: {other:?}"),
  F? LINE   crates/store/tests/month_repair.rs:324 [assert_eq] (test): timeframe.as_str()
  GAP LINE   crates/store/tests/month_repair.rs:637 (test): calls += 1;
  GAP LINE   crates/store/tests/month_repair.rs:638 (test): Ok(())
  GAP LINE   crates/store/tests/month_repair.rs:639 (test): };
  F? LINE   crates/store/tests/month_repair.rs:856 [panic] (test): panic!("{outcome:?}");
  F? LINE   crates/store/tests/month_repair.rs:918 [assert] (test): left.keys()
  F? LINE   crates/store/tests/month_repair.rs:944 [panic] (test): other => panic!("not a repair: {other:?}"),
  F? BRANCH crates/store/tests/month_repair.rs:606:13 False [assert] (test): assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  F? BRANCH crates/store/tests/month_repair.rs:606:47 False [assert] (test): assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  F? BRANCH crates/store/tests/month_repair.rs:739:83 False [assert] (test): matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),
  GAP BRANCH crates/store/tests/month_repair.rs:852:9 False (test): let Ok(MonthRepair::Repaired {
  F? BRANCH crates/store/tests/month_repair.rs:879:38 False [assert] (test): Served::Revision(pointer) if pointer.revision.ordinal() == 3
  GAP LINE   crates/store/tests/month_summary_log.rs:74 (test): OwnedValue::Uint(n) => n.to_string(),
  GAP LINE   crates/store/tests/month_summary_log.rs:75 (test): other => format!("{other:?}"),
  GAP LINE   crates/store/tests/open_flags.rs:176 (test): offenders.push(format!("{}:{}", path.display(), number + 1));
  GAP BRANCH crates/store/tests/open_flags.rs:175:16 True (test): if names_the_x86_bit(line) {
  F? LINE   crates/store/tests/provenance.rs:66 [panic] (test): other => panic!("expected a sidecar, found {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:161 [panic] (test): Recorded::NothingAppended => panic!("expected a record"),
  F? LINE   crates/store/tests/provenance.rs:451 [panic] (test): other => panic!("expected NotTheNewestCommit, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:464 [panic] (test): other => panic!("expected NotABarPath, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:476 [panic] (test): other => panic!("expected WrongBarFile, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:531 [panic] (test): other => panic!("expected OutOfOrder, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:639 [panic] (test): other => panic!("expected Damaged, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:698 [panic] (test): result => panic!("expected a {field} mismatch, got {result:?}"),
  F? LINE   crates/store/tests/provenance.rs:746 [panic] (test): other => panic!("expected PastTheBars, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:1200 [panic] (test): other => panic!("expected {fault:?}, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:1308 [panic] (test): other => panic!("expected TooLarge, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:1409 [panic] (test): other => panic!("expected Record, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:1611 [panic] (test): other => panic!("expected NotABarPath, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:1617 [panic] (test): other => panic!("expected NotABarPath, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:1671 [panic] (test): other => panic!("expected the directory sync refused, got {other:?}"),
  F? LINE   crates/store/tests/provenance.rs:1721 [panic] (test): other => panic!("expected the open refused, got {other:?}"),
  F? BRANCH crates/store/tests/repair.rs:506:13 True [assert] (test): assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  F? BRANCH crates/store/tests/repair.rs:506:41 False [assert] (test): assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  F? LINE   crates/store/tests/sealed_codec_alloc.rs:115 [panic] (test): panic!("a frame cut to {len} bytes left {frame_answer:?}");
  F? LINE   crates/store/tests/sealed_codec_alloc.rs:125 [assert] (test): full.len()
  GAP BRANCH crates/store/tests/sealed_codec_alloc.rs:114:13 False (test): let (Err(FrameError::Truncated { part }), 0) = frame_answer else {
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:123:9 False [assert] (test): in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:123:32 False [assert] (test): in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:332:13 False [assert] (test): !text.contains("by its checked rows and no more")
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:333:20 False [assert] (test): && !text.contains("checked rows times 56 bytes, once"),
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:341:13 False [assert] (test): text.contains("the largest of twice") && text.contains("and 4"),
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:341:54 False [assert] (test): text.contains("the largest of twice") && text.contains("and 4"),
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:347:13 False [assert] (test): text.contains("capacity 0") && text.contains("cleared"),
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:347:44 False [assert] (test): text.contains("capacity 0") && text.contains("cleared"),
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:357:13 False [assert] (test): !text.contains("random fields cost at most 57 bytes a bar")
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:358:20 False [assert] (test): && !text.contains("at most 57 bytes a bar, 1 more than raw"),
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:387:9 False [assert] (test): SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  F? BRANCH crates/store/tests/sealed_codec_alloc.rs:387:52 False [assert] (test): SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:65 (test): fn read_month(path: &Path) -> Vec<Bar> {
  F? LINE   crates/store/tests/sealed_codec_samples.rs:68 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_codec_samples.rs:69 [assert] (test): path.extension().is_some_and(|e| e == "bin"),
  F? LINE   crates/store/tests/sealed_codec_samples.rs:71 [assert] (test): path.display()
  F? LINE   crates/store/tests/sealed_codec_samples.rs:73 [panic] (test): let data = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:74 (test): let header = data.get(..RECORDS_AT).unwrap_or_else(|| {
  F? LINE   crates/store/tests/sealed_codec_samples.rs:75 [panic] (test): panic!(
  F? LINE   crates/store/tests/sealed_codec_samples.rs:77 [panic] (test): path.display(),
  F? LINE   crates/store/tests/sealed_codec_samples.rs:78 [panic] (test): data.len()
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:81 (test): let header = Header::read_region(header, data.len() as u64)
  F? LINE   crates/store/tests/sealed_codec_samples.rs:82 [panic] (test): .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:83 (test): let n = header.n_valid as usize;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:84 (test): data.get(RECORDS_AT..RECORDS_AT + n * STRIDE)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:85 (test): .unwrap_or_else(|| {
  F? LINE   crates/store/tests/sealed_codec_samples.rs:86 [panic] (test): panic!(
  F? LINE   crates/store/tests/sealed_codec_samples.rs:88 [panic] (test): path.display()
  F? LINE   crates/store/tests/sealed_codec_samples.rs:91 [panic] (test): .chunks_exact(STRIDE)
  F? LINE   crates/store/tests/sealed_codec_samples.rs:92 [panic] (test): .map(|record| Bar::decode(record).expect("a committed record decodes"))
  F? LINE   crates/store/tests/sealed_codec_samples.rs:93 [panic] (test): .collect()
  F? LINE   crates/store/tests/sealed_codec_samples.rs:94 [panic] (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:97 (test): fn origin_of(name: &str) -> i64 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:98 (test): let stem = name.trim_end_matches(".bin");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:99 (test): let month = &stem[stem.len() - 7..];
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:100 (test): let year: u16 = month[..4].parse().expect("year");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:101 (test): let mm: u8 = month[5..].parse().expect("month");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:102 (test): YearMonth::new(year, mm)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:103 (test): .expect("a month")
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:104 (test): .ist_bounds_micros()
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:105 (test): .0
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:106 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:111 (test): fn judge_row0(rows: &[Bar]) -> u64 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:112 (test): fn len(mut v: u64) -> u64 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:113 (test): let mut n = 1;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:114 (test): while v >= 0x80 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:115 (test): v >>= 7;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:116 (test): n += 1;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:117 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:118 (test): n
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:119 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:120 (test): fn zz(x: i64) -> u64 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:121 (test): ((x << 1) ^ (x >> 63)).cast_unsigned()
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:122 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:123 (test): let first = rows.first().map_or(0, |b| b.ts_micros);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:124 (test): rows.iter()
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:125 (test): .step_by(256)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:126 (test): .map(|b| {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:127 (test): let d = b.ts_micros - first;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:128 (test): let ts = if d % 1_000_000 == 0 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:129 (test): len(zz(d / 1_000_000))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:131 (test): len(zz(d))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:133 (test): let oi = if b.open_interest == OI_NULL {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:134 (test): 1
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:136 (test): 1 + len(zz(b.open_interest))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:138 (test): ts + 1
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:139 (test): + len(zz(b.close))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:140 (test): + len(zz(b.open - b.close))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:141 (test): + len(zz(b.high - b.close))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:142 (test): + len(zz(b.low - b.close))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:143 (test): + len(zz(b.volume))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:144 (test): + oi
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:145 (test): })
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:146 (test): .sum()
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:147 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:150 (test): const fn zz(x: i64) -> u64 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:151 (test): ((x << 1) ^ (x >> 63)).cast_unsigned()
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:152 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:154 (test): const fn varint_len(value: u64) -> usize {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:155 (test): let bits = (64 - value.leading_zeros()) as usize;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:156 (test): if bits == 0 { 1 } else { bits.div_ceil(7) }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:157 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:160 (test): fn row0_len(first: &Bar, origin: i64) -> usize {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:161 (test): let d = first.ts_micros.wrapping_sub(origin);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:162 (test): let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:163 (test): let oi = if first.open_interest == OI_NULL {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:164 (test): 0
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:166 (test): varint_len(zz(first.open_interest))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:168 (test): varint_len(zz(stamp))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:169 (test): + varint_len(zz(first.close))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:170 (test): + varint_len(zz(first.open.wrapping_sub(first.close)))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:171 (test): + varint_len(zz(first.high.wrapping_sub(first.close)))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:172 (test): + varint_len(zz(first.low.wrapping_sub(first.close)))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:173 (test): + varint_len(zz(first.volume))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:174 (test): + oi
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:175 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:179 (test): fn residuals(rows: &[Bar], column: usize, mode: u8) -> Vec<i64> {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:180 (test): rows.windows(2)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:181 (test): .map(|pair| {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:182 (test): let (p, b) = (pair[0], pair[1]);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:183 (test): let (value, predictor) = match (column, mode) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:184 (test): (0, _) => (b.ts_micros, p.ts_micros),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:185 (test): (1, _) => (b.close, p.close),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:186 (test): (2, 0) => (b.open, p.close),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:187 (test): (2, _) => (b.open, b.close),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:188 (test): (3, _) => (b.high, b.open.max(b.close)),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:189 (test): (4, _) => (b.low, b.open.min(b.close)),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:190 (test): (5, 0) => (b.volume, 0),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:191 (test): (5, _) => (b.volume, p.volume),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:192 (test): (6, 0) => (b.open_interest, p.open_interest),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:193 (test): _ => (b.open_interest, 0),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:195 (test): value.wrapping_sub(predictor)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:196 (test): })
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:197 (test): .collect()
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:198 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:205 (test): fn prototype_stream_len(residuals: &[i64]) -> usize {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:206 (test): fn gcd(mut a: u64, mut b: u64) -> u64 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:207 (test): while b != 0 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:208 (test): (a, b) = (b, a % b);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:209 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:210 (test): a
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:211 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:212 (test): let n = residuals.len();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:213 (test): let mut g = residuals.iter().fold(0, |g, r| gcd(g, r.unsigned_abs()));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:214 (test): if g == 0 || g >= 1 << 62 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:215 (test): g = 1;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:216 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:217 (test): let values: Vec<u64> = residuals.iter().map(|&r| zz(r / g.cast_signed())).collect();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:218 (test): let base = values.iter().copied().min().unwrap_or(0);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:219 (test): let mut histogram = [0usize; 65];
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:220 (test): for v in &values {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:221 (test): histogram[(64 - (v - base).leading_zeros()) as usize] += 1;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:222 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:223 (test): let mut top = 64;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:224 (test): while top > 0 && histogram[top] == 0 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:225 (test): top -= 1;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:226 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:227 (test): let p = ((usize::BITS - n.saturating_sub(1).leading_zeros()) as usize).max(1);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:228 (test): let (mut best, mut best_w, mut exceptions, mut w) = (n * top, top, 0, top);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:229 (test): while w > 0 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:230 (test): exceptions += histogram[w];
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:231 (test): w -= 1;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:232 (test): let cost = n * w + exceptions * (p + top) + 16;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:233 (test): if cost < best {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:234 (test): best = cost;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:235 (test): best_w = w;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:236 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:238 (test): let limit = if best_w == 64 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:239 (test): u64::MAX
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:241 (test): (1 << best_w) - 1
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:243 (test): let exceptions = values.iter().filter(|&&v| v - base > limit).count();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:244 (test): let mut len = 1
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:245 (test): + varint_len(exceptions as u64)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:246 (test): + varint_len(base)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:247 (test): + varint_len(g)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:248 (test): + (n * best_w).div_ceil(8);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:249 (test): if exceptions > 0 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:250 (test): len += 1 + (exceptions * p).div_ceil(8) + (exceptions * top).div_ceil(8);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:251 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:252 (test): len
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:253 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:256 (test): fn prototype_frame_len(rows: &[Bar], origin: i64) -> usize {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:257 (test): let streams: usize = (0..7)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:258 (test): .map(|column| {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:259 (test): let zero = prototype_stream_len(&residuals(rows, column, 0));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:260 (test): if matches!(column, 2 | 5 | 6) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:261 (test): zero.min(prototype_stream_len(&residuals(rows, column, 1)))
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:263 (test): zero
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:265 (test): })
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:266 (test): .sum();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:267 (test): 4 + row0_len(&rows[0], origin) + streams
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:268 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:281 (test): fn seal(rows: &[Bar], origin: i64, label: &str) -> Sealed {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:282 (test): let mut frames = Vec::with_capacity(rows.len().div_ceil(256));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:283 (test): let (mut bytes, mut prototype_rule, mut first_rows) = (0, 0, 0);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:284 (test): for chunk in rows.chunks(usize::from(BLOCK_ROWS)) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:285 (test): let rule = prototype_frame_len(chunk, origin);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:286 (test): first_rows += row0_len(&chunk[0], origin) as u64;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:287 (test): let mut frame = Vec::new();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:288 (test): let encoded = encode_frame(chunk, origin, BLOCK_ROWS, &mut frame).expect("encodes");
  F? LINE   crates/store/tests/sealed_codec_samples.rs:289 [assert] (test): assert!(frame.len() <= MAX_FRAME_LEN, "{label}");
  F? LINE   crates/store/tests/sealed_codec_samples.rs:290 [assert_eq] (test): assert_eq!(encoded.crc, crc32c(&frame));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:291 (test): let mut back = Vec::new();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:292 (test): decode_frame(&frame, encoded.crc, origin, BLOCK_ROWS, &mut back).expect("decodes");
  F? LINE   crates/store/tests/sealed_codec_samples.rs:293 [assert_eq] (test): assert_eq!(back, chunk, "{label}: a frame came back different");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:294 (test): let mut stamps = Vec::new();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:295 (test): decode_stamps(&frame, encoded.crc, origin, BLOCK_ROWS, &mut stamps).expect("stamps");
  F? LINE   crates/store/tests/sealed_codec_samples.rs:296 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_codec_samples.rs:297 [assert] (test): stamps.iter().zip(chunk).all(|(s, b)| *s == b.ts_micros),
  F? LINE   crates/store/tests/sealed_codec_samples.rs:301 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_codec_samples.rs:302 [assert] (test): frame.len() <= rule,
  F? LINE   crates/store/tests/sealed_codec_samples.rs:304 [assert] (test): frame.len()
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:306 (test): bytes += frame.len() as u64;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:307 (test): prototype_rule += rule as u64;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:308 (test): frames.push((frame, encoded.crc));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:310 (test): Sealed {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:311 (test): bytes,
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:312 (test): prototype_rule,
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:313 (test): row0: first_rows,
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:314 (test): frames,
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:315 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:316 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:318 (test): fn percentiles(label: &str, mut ns: Vec<u64>) -> String {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:319 (test): ns.sort_unstable();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:320 (test): let at = |q: usize| ns[((ns.len() - 1) * q + 50) / 100];
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:321 (test): format!(
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:323 (test): ns.len(),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:324 (test): at(50),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:325 (test): at(99),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:326 (test): ns.last().copied().unwrap_or(0)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:328 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:331 (test): fn time(rows: &[Bar], origin: i64, sealed: &Sealed, label: &str) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:332 (test): let mut decode = Vec::new();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:333 (test): let mut stamps = Vec::new();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:334 (test): let mut out = Vec::with_capacity(512);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:335 (test): let mut ts = Vec::with_capacity(256);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:336 (test): for _ in 0..(2_000 / sealed.frames.len()).max(1) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:337 (test): for (frame, crc) in &sealed.frames {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:338 (test): out.clear();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:339 (test): let t = Instant::now();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:340 (test): decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:341 (test): decode.push(t.elapsed().as_nanos() as u64);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:342 (test): ts.clear();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:343 (test): let t = Instant::now();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:344 (test): decode_stamps(frame, *crc, origin, BLOCK_ROWS, &mut ts).expect("stamps");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:345 (test): stamps.push(t.elapsed().as_nanos() as u64);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:346 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:348 (test): let mut encode = Vec::new();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:349 (test): let mut buffer = Vec::with_capacity(MAX_FRAME_LEN);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:350 (test): for chunk in rows.chunks(256).take(2_000) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:351 (test): buffer.clear();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:352 (test): let t = Instant::now();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:353 (test): encode_frame(chunk, origin, BLOCK_ROWS, &mut buffer).expect("encodes");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:354 (test): encode.push(t.elapsed().as_nanos() as u64);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:355 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:357 (test): let mut page = Vec::new();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:358 (test): let mut state = 0x9E37_79B9_7F4A_7C15_u64;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:359 (test): let n = rows.len() as u64;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:360 (test): for _ in 0..2_000 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:361 (test): state ^= state << 13;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:362 (test): state ^= state >> 7;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:363 (test): state ^= state << 17;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:364 (test): let first = state % n;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:365 (test): let last = (first + 99).min(n - 1);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:366 (test): out.clear();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:367 (test): let t = Instant::now();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:368 (test): for k in (first / 256)..=(last / 256) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:369 (test): let (frame, crc) = &sealed.frames[k as usize];
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:370 (test): decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:371 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:372 (test): page.push(t.elapsed().as_nanos() as u64);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:373 (test): let skip = (first % 256) as usize;
  F? LINE   crates/store/tests/sealed_codec_samples.rs:374 [assert_eq] (test): assert_eq!(out[skip], rows[first as usize]);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:376 (test): println!(
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:378 (test): percentiles(&format!("{label} frame decode+crc"), decode)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:380 (test): println!(
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:382 (test): percentiles(&format!("{label} frame stamps+crc"), stamps)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:384 (test): println!(
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:386 (test): percentiles(&format!("{label} frame encode"), encode)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:388 (test): println!(
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:390 (test): percentiles(&format!("{label} page100 from memory"), page)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:392 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:409 (test): fn add(&mut self, label: &str, bars: usize, sealed: &Sealed, exact: u64, judge: u64) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:410 (test): if sealed.prototype_rule != exact {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:411 (test): self.not_exact.push(format!(
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:412 (test): "{label}: rule {} exact {exact}",
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:413 (test): sealed.prototype_rule
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:414 (test): ));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:415 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:416 (test): if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:417 (test): self.larger
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:418 (test): .push(format!("{label}: codec1 {} judge {judge}", sealed.bytes));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:419 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:420 (test): self.codec1 += sealed.bytes;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:421 (test): self.rule += sealed.prototype_rule;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:422 (test): self.exact += exact;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:423 (test): self.judge += judge;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:424 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:426 (test): fn check(&self, label: &str) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:427 (test): let d = |x: u64, of: u64| per_cent(x.cast_signed() - of.cast_signed(), of);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:428 (test): println!(
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:434 (test): d(self.codec1, self.exact),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:435 (test): d(self.codec1, self.judge),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:436 (test): self.not_exact.len(),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:437 (test): self.larger.len()
  F? LINE   crates/store/tests/sealed_codec_samples.rs:440 [assert] (test): assert!(self.not_exact.is_empty(), "{label}: {:?}", self.not_exact);
  F? LINE   crates/store/tests/sealed_codec_samples.rs:441 [assert_eq] (test): assert_eq!(self.rule, self.exact, "{label}");
  F? LINE   crates/store/tests/sealed_codec_samples.rs:443 [assert] (test): assert!(self.codec1 <= self.rule, "{label}");
  F? LINE   crates/store/tests/sealed_codec_samples.rs:444 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_codec_samples.rs:445 [assert] (test): within(self.codec1, self.exact).0,
  F? LINE   crates/store/tests/sealed_codec_samples.rs:450 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_codec_samples.rs:451 [assert] (test): within(self.codec1, self.judge).0,
  F? LINE   crates/store/tests/sealed_codec_samples.rs:454 [assert] (test): assert!(self.larger.is_empty(), "{label}: {:?}", self.larger);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:455 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:459 (test): fn per_cent(diff: i64, of: u64) -> String {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:460 (test): let thousandths = diff.unsigned_abs() * 100_000 / of;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:461 (test): let sign = if diff < 0 { "-" } else { "+" };
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:462 (test): format!("{sign}{}.{:03}%", thousandths / 1_000, thousandths % 1_000)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:463 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:466 (test): fn within(mine: u64, expected: u64) -> (bool, i64) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:467 (test): let diff = mine.cast_signed() - expected.cast_signed();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:469 (test): (diff.unsigned_abs() * 200 <= expected, diff)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:470 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:474 (test): fn the_sample_months_encode_to_the_prototype_sizes_less_the_first_row_saving() {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:475 (test): let root = std::env::var("BRUTEX_SEAL_SAMPLES")
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:476 (test): .expect("BRUTEX_SEAL_SAMPLES names the store-compression evidence directory");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:477 (test): let root = Path::new(&root);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:488 (test): println!("eight sample months:");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:489 (test): let mut totals = Totals::default();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:490 (test): for (dir, month, body, saved) in SAMPLES {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:491 (test): let path = root
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:492 (test): .join("blocks/samples")
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:493 (test): .join(dir)
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:494 (test): .join(format!("{month}.bin"));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:495 (test): let rows = read_month(&path);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:496 (test): let origin = origin_of(month);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:497 (test): let sealed = seal(&rows, origin, dir);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:498 (test): let blocks = sealed.frames.len() as u64;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:499 (test): let exact = body - 56 * blocks + sealed.row0;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:500 (test): let judge = body - saved;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:501 (test): println!(
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:502 (test): "{dir} {month}: bars={} frames={blocks} codec1={} rule={} exact={exact} judge={judge} codec1-judge={} raw={} ({} B/bar x100)",
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:503 (test): rows.len(),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:504 (test): sealed.bytes,
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:505 (test): sealed.prototype_rule,
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:506 (test): per_cent(sealed.bytes.cast_signed() - judge.cast_signed(), judge),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:507 (test): rows.len() * 56,
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:508 (test): sealed.bytes * 100 / rows.len() as u64,
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:509 (test): );
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:510 (test): time(&rows, origin, &sealed, dir);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:511 (test): totals.add(dir, rows.len(), &sealed, exact, judge);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:512 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:513 (test): totals.check("eight samples in all");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:515 (test): let manifest =
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:516 (test): std::fs::read_to_string(root.join("blocks/proto/strat-manifest.tsv")).expect("manifest");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:517 (test): let out =
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:518 (test): std::fs::read_to_string(root.join("blocks/proto/strat-out.tsv")).expect("prototype output");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:519 (test): let mut totals = Totals::default();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:520 (test): let (mut months, mut bars, mut empty) = (0, 0, 0);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:521 (test): for (line, result) in manifest.lines().zip(out.lines()) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:522 (test): let fields: Vec<&str> = line.split('\t').collect();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:523 (test): let measured: Vec<&str> = result.split('\t').collect();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:524 (test): let rows = read_month(&root.join("blocks").join(fields[0]));
  F? LINE   crates/store/tests/sealed_codec_samples.rs:525 [assert_eq] (test): assert_eq!(rows.len().to_string(), measured[2], "{}", fields[4]);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:526 (test): if rows.is_empty() {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:527 (test): empty += 1;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:528 (test): continue;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:529 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:530 (test): let origin = origin_of(fields[4]);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:531 (test): let sealed = seal(&rows, origin, fields[4]);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:532 (test): let body: u64 = measured[3].parse().expect("body");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:533 (test): let blocks = sealed.frames.len() as u64;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:534 (test): let exact = body - 56 * blocks + sealed.row0;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:535 (test): let judge = body - (56 * blocks - judge_row0(&rows));
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:536 (test): totals.add(fields[4], rows.len(), &sealed, exact, judge);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:537 (test): months += 1;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:538 (test): bars += rows.len();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:540 (test): println!("539-month manifest: {months} months sealed, {empty} empty, {bars} bars");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:541 (test): totals.check("the manifest in all");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:542 (test): if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:543 (test): println!("extra months (read only, round trip and size):");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:544 (test): for path in extra.lines().filter(|l| !l.trim().is_empty()) {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:545 (test): let path = Path::new(path.trim());
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:546 (test): let rows = read_month(path);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:547 (test): let name = path.to_string_lossy();
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:548 (test): let origin = origin_of(&name);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:549 (test): if rows.is_empty() {
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:550 (test): println!("{name}: empty month");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:551 (test): continue;
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:552 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:553 (test): let sealed = seal(&rows, origin, &name);
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:554 (test): println!(
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:556 (test): rows.len(),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:557 (test): sealed.frames.len(),
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:559 (test): rows.len() * 56
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:561 (test): time(&rows, origin, &sealed, "extra");
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:563 (test): }
  GAP LINE   crates/store/tests/sealed_codec_samples.rs:564 (test): }
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:114:15 True (test): while v >= 0x80 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:114:15 False (test): while v >= 0x80 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:128:25 True (test): let ts = if d % 1_000_000 == 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:128:25 False (test): let ts = if d % 1_000_000 == 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:133:25 True (test): let oi = if b.open_interest == OI_NULL {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:133:25 False (test): let oi = if b.open_interest == OI_NULL {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:156:8 True (test): if bits == 0 { 1 } else { bits.div_ceil(7) }
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:156:8 False (test): if bits == 0 { 1 } else { bits.div_ceil(7) }
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:162:20 True (test): let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:162:20 False (test): let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:163:17 True (test): let oi = if first.open_interest == OI_NULL {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:163:17 False (test): let oi = if first.open_interest == OI_NULL {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:207:15 True (test): while b != 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:207:15 False (test): while b != 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:214:8 True (test): if g == 0 || g >= 1 << 62 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:214:8 False (test): if g == 0 || g >= 1 << 62 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:214:18 True (test): if g == 0 || g >= 1 << 62 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:214:18 False (test): if g == 0 || g >= 1 << 62 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:224:11 True (test): while top > 0 && histogram[top] == 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:224:11 False (test): while top > 0 && histogram[top] == 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:224:22 True (test): while top > 0 && histogram[top] == 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:224:22 False (test): while top > 0 && histogram[top] == 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:229:11 True (test): while w > 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:229:11 False (test): while w > 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:233:12 True (test): if cost < best {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:233:12 False (test): if cost < best {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:238:20 True (test): let limit = if best_w == 64 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:238:20 False (test): let limit = if best_w == 64 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:249:8 True (test): if exceptions > 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:249:8 False (test): if exceptions > 0 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:410:12 True (test): if sealed.prototype_rule != exact {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:410:12 False (test): if sealed.prototype_rule != exact {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:416:12 True (test): if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:416:12 False (test): if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:416:29 True (test): if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:416:29 False (test): if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:461:19 True (test): let sign = if diff < 0 { "-" } else { "+" };
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:461:19 False (test): let sign = if diff < 0 { "-" } else { "+" };
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:526:12 True (test): if rows.is_empty() {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:526:12 False (test): if rows.is_empty() {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:542:12 True (test): if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:542:12 False (test): if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:549:16 True (test): if rows.is_empty() {
  GAP BRANCH crates/store/tests/sealed_codec_samples.rs:549:16 False (test): if rows.is_empty() {
  GAP LINE   crates/store/tests/sealed_door.rs:237 (test): fn store_path(rel: &str) -> StorePath<'static> {
  GAP LINE   crates/store/tests/sealed_door.rs:238 (test): let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
  GAP LINE   crates/store/tests/sealed_door.rs:239 (test): let parts: Vec<&'static str> = rel.split('/').collect();
  F? LINE   crates/store/tests/sealed_door.rs:240 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_door.rs:241 [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? LINE   crates/store/tests/sealed_door.rs:244 [assert_eq] (test): assert_eq!(parts[0], "bars", "{rel}");
  GAP LINE   crates/store/tests/sealed_door.rs:245 (test): let vendor = Vendor::ALL
  GAP LINE   crates/store/tests/sealed_door.rs:246 (test): .into_iter()
  GAP LINE   crates/store/tests/sealed_door.rs:247 (test): .find(|v| v.as_str() == parts[1])
  F? LINE   crates/store/tests/sealed_door.rs:248 [panic] (test): .unwrap_or_else(|| panic!("{rel}: vendor"));
  GAP LINE   crates/store/tests/sealed_door.rs:249 (test): let contract = (parts.len() == 8)
  GAP LINE   crates/store/tests/sealed_door.rs:250 (test): .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
  GAP LINE   crates/store/tests/sealed_door.rs:251 (test): let tf = parts[parts.len() - 2];
  GAP LINE   crates/store/tests/sealed_door.rs:252 (test): let timeframe = *Timeframe::KNOWN
  GAP LINE   crates/store/tests/sealed_door.rs:253 (test): .iter()
  GAP LINE   crates/store/tests/sealed_door.rs:254 (test): .find(|t| t.as_str() == tf)
  F? LINE   crates/store/tests/sealed_door.rs:255 [panic] (test): .unwrap_or_else(|| panic!("{rel}: rung"));
  GAP LINE   crates/store/tests/sealed_door.rs:256 (test): let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");
  GAP LINE   crates/store/tests/sealed_door.rs:257 (test): let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
  GAP LINE   crates/store/tests/sealed_door.rs:258 (test): StorePath::new(PathParts {
  GAP LINE   crates/store/tests/sealed_door.rs:259 (test): vendor,
  GAP LINE   crates/store/tests/sealed_door.rs:260 (test): exchange: parts[2],
  GAP LINE   crates/store/tests/sealed_door.rs:261 (test): segment: parts[3],
  GAP LINE   crates/store/tests/sealed_door.rs:262 (test): symbol: parts[4],
  GAP LINE   crates/store/tests/sealed_door.rs:263 (test): contract,
  GAP LINE   crates/store/tests/sealed_door.rs:264 (test): timeframe,
  GAP LINE   crates/store/tests/sealed_door.rs:265 (test): month,
  GAP LINE   crates/store/tests/sealed_door.rs:266 (test): file: FileKind::Bars,
  GAP LINE   crates/store/tests/sealed_door.rs:267 (test): })
  GAP LINE   crates/store/tests/sealed_door.rs:268 (test): .unwrap()
  GAP LINE   crates/store/tests/sealed_door.rs:269 (test): }
  GAP LINE   crates/store/tests/sealed_door.rs:280 (test): fn seal_a_private_store_root_for_the_shadow_check() {
  GAP LINE   crates/store/tests/sealed_door.rs:281 (test): let root = PathBuf::from(std::env::var("BRUTEX_SC3_ROOT").expect("BRUTEX_SC3_ROOT"));
  GAP LINE   crates/store/tests/sealed_door.rs:282 (test): let list = std::env::var("BRUTEX_SC3_MONTHS").expect("BRUTEX_SC3_MONTHS");
  GAP LINE   crates/store/tests/sealed_door.rs:283 (test): let bars_dir = root.join("bars");
  F? LINE   crates/store/tests/sealed_door.rs:284 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_door.rs:285 [assert] (test): !fs::symlink_metadata(&bars_dir)
  F? LINE   crates/store/tests/sealed_door.rs:286 [assert] (test): .unwrap()
  F? LINE   crates/store/tests/sealed_door.rs:287 [assert] (test): .file_type()
  F? LINE   crates/store/tests/sealed_door.rs:288 [assert] (test): .is_symlink(),
  F? LINE   crates/store/tests/sealed_door.rs:290 [assert] (test): bars_dir.display()
  F? LINE   crates/store/tests/sealed_door.rs:292 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_door.rs:293 [assert] (test): !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
  GAP LINE   crates/store/tests/sealed_door.rs:296 (test): let (mut months, mut rows, mut raw_bytes, mut sealed_bytes) = (0u64, 0u64, 0u64, 0u64);
  GAP LINE   crates/store/tests/sealed_door.rs:297 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_door.rs:298 (test): for rel in fs::read_to_string(&list).unwrap().lines().map(str::trim) {
  GAP LINE   crates/store/tests/sealed_door.rs:299 (test): if rel.is_empty() || rel.starts_with('#') {
  GAP LINE   crates/store/tests/sealed_door.rs:300 (test): continue;
  GAP LINE   crates/store/tests/sealed_door.rs:301 (test): }
  GAP LINE   crates/store/tests/sealed_door.rs:302 (test): let at = store_path(rel);
  GAP LINE   crates/store/tests/sealed_door.rs:303 (test): let symbol = rel.split('/').nth(4).unwrap().to_owned();
  GAP LINE   crates/store/tests/sealed_door.rs:304 (test): let symbol_id = brutex_core::universe::fnv1a(&symbol) as u32;
  GAP LINE   crates/store/tests/sealed_door.rs:305 (test): let served = store::generation::served(&root, at).unwrap();
  GAP LINE   crates/store/tests/sealed_door.rs:306 (test): let served_root = served.root(&root);
  GAP LINE   crates/store/tests/sealed_door.rs:307 (test): let sbar = at.with_file(FileKind::SealedBars).to_path_buf(&served_root);
  GAP LINE   crates/store/tests/sealed_door.rs:308 (test): if sbar.exists() {
  GAP LINE   crates/store/tests/sealed_door.rs:309 (test): println!("SKIP {rel}: already sealed");
  GAP LINE   crates/store/tests/sealed_door.rs:310 (test): continue;
  GAP LINE   crates/store/tests/sealed_door.rs:311 (test): }
  GAP LINE   crates/store/tests/sealed_door.rs:312 (test): let raw = BarFile::open_existing(&root, at, symbol_id).unwrap();
  F? LINE   crates/store/tests/sealed_door.rs:313 [assert_eq] (test): assert_eq!(raw.form(), Form::Raw, "{rel}");
  GAP LINE   crates/store/tests/sealed_door.rs:314 (test): let bars = raw.read_range(0, raw.records()).unwrap();
  GAP LINE   crates/store/tests/sealed_door.rs:315 (test): let prv = fs::read(at.with_file(FileKind::Provenance).to_path_buf(&served_root)).ok();
  GAP LINE   crates/store/tests/sealed_door.rs:316 (test): let built = store::sealed::build(at, &raw.header(), &bars, prv.as_deref(), None).unwrap();
  GAP LINE   crates/store/tests/sealed_door.rs:317 (test): let staged = sbar.with_extension("sbar-staged");
  GAP LINE   crates/store/tests/sealed_door.rs:318 (test): let mut file = fs::File::create_new(&staged).unwrap();
  GAP LINE   crates/store/tests/sealed_door.rs:319 (test): built.write_to(&mut file).unwrap();
  GAP LINE   crates/store/tests/sealed_door.rs:320 (test): file.sync_all().unwrap();
  GAP LINE   crates/store/tests/sealed_door.rs:321 (test): drop(file);
  GAP LINE   crates/store/tests/sealed_door.rs:322 (test): fs::rename(&staged, &sbar).unwrap();
  GAP LINE   crates/store/tests/sealed_door.rs:323 (test): let sealed = BarFile::open_existing(&root, at, symbol_id).unwrap();
  F? LINE   crates/store/tests/sealed_door.rs:324 [assert_eq] (test): assert_eq!(sealed.form(), Form::Sealed, "{rel}");
  F? LINE   crates/store/tests/sealed_door.rs:325 [assert_eq] (test): assert_eq!(
  F? LINE   crates/store/tests/sealed_door.rs:326 [assert_eq] (test): sealed.read_range(0, sealed.records()).unwrap(),
  GAP LINE   crates/store/tests/sealed_door.rs:330 (test): months += 1;
  GAP LINE   crates/store/tests/sealed_door.rs:331 (test): rows += bars.len() as u64;
  GAP LINE   crates/store/tests/sealed_door.rs:332 (test): raw_bytes += fs::metadata(at.to_path_buf(&served_root)).unwrap().len();
  GAP LINE   crates/store/tests/sealed_door.rs:333 (test): sealed_bytes += built.file_len();
  GAP LINE   crates/store/tests/sealed_door.rs:334 (test): println!(
  GAP LINE   crates/store/tests/sealed_door.rs:336 (test): bars.len(),
  GAP LINE   crates/store/tests/sealed_door.rs:337 (test): built.file_len(),
  GAP LINE   crates/store/tests/sealed_door.rs:338 (test): served_root.display()
  GAP LINE   crates/store/tests/sealed_door.rs:341 (test): println!(
  GAP LINE   crates/store/tests/sealed_door.rs:344 (test): started.elapsed().as_secs_f64()
  GAP LINE   crates/store/tests/sealed_door.rs:346 (test): }
  F? BRANCH crates/store/tests/sealed_door.rs:153:17 True [assert] (test): assert!(at == rows || bars[at as usize].ts_micros >= probe);
  F? BRANCH crates/store/tests/sealed_door.rs:153:31 False [assert] (test): assert!(at == rows || bars[at as usize].ts_micros >= probe);
  F? BRANCH crates/store/tests/sealed_door.rs:231:9 False [assert] (test): large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  F? BRANCH crates/store/tests/sealed_door.rs:231:45 False [assert] (test): large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  F? BRANCH crates/store/tests/sealed_door.rs:241:9 True [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? BRANCH crates/store/tests/sealed_door.rs:241:9 False [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? BRANCH crates/store/tests/sealed_door.rs:241:29 True [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? BRANCH crates/store/tests/sealed_door.rs:241:29 False [assert] (test): parts.len() == 7 || parts.len() == 8,
  GAP BRANCH crates/store/tests/sealed_door.rs:299:12 True (test): if rel.is_empty() || rel.starts_with('#') {
  GAP BRANCH crates/store/tests/sealed_door.rs:299:12 False (test): if rel.is_empty() || rel.starts_with('#') {
  GAP BRANCH crates/store/tests/sealed_door.rs:299:30 True (test): if rel.is_empty() || rel.starts_with('#') {
  GAP BRANCH crates/store/tests/sealed_door.rs:299:30 False (test): if rel.is_empty() || rel.starts_with('#') {
  GAP BRANCH crates/store/tests/sealed_door.rs:308:12 True (test): if sbar.exists() {
  GAP BRANCH crates/store/tests/sealed_door.rs:308:12 False (test): if sbar.exists() {
  GAP LINE   crates/store/tests/sealed_samples.rs:49 (test): fn next(&mut self) -> u64 {
  GAP LINE   crates/store/tests/sealed_samples.rs:50 (test): self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
  GAP LINE   crates/store/tests/sealed_samples.rs:51 (test): let mut z = self.0;
  GAP LINE   crates/store/tests/sealed_samples.rs:52 (test): z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
  GAP LINE   crates/store/tests/sealed_samples.rs:53 (test): z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
  GAP LINE   crates/store/tests/sealed_samples.rs:54 (test): z ^ (z >> 31)
  GAP LINE   crates/store/tests/sealed_samples.rs:55 (test): }
  GAP LINE   crates/store/tests/sealed_samples.rs:56 (test): fn below(&mut self, n: u64) -> u64 {
  GAP LINE   crates/store/tests/sealed_samples.rs:57 (test): self.next() % n
  GAP LINE   crates/store/tests/sealed_samples.rs:58 (test): }
  GAP LINE   crates/store/tests/sealed_samples.rs:62 (test): fn store_path(rel: &str) -> StorePath<'static> {
  GAP LINE   crates/store/tests/sealed_samples.rs:63 (test): let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
  GAP LINE   crates/store/tests/sealed_samples.rs:64 (test): let parts: Vec<&'static str> = rel.split('/').collect();
  F? LINE   crates/store/tests/sealed_samples.rs:65 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_samples.rs:66 [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? LINE   crates/store/tests/sealed_samples.rs:69 [assert_eq] (test): assert_eq!(parts[0], "bars", "{rel}");
  GAP LINE   crates/store/tests/sealed_samples.rs:70 (test): let vendor = Vendor::ALL
  GAP LINE   crates/store/tests/sealed_samples.rs:71 (test): .into_iter()
  GAP LINE   crates/store/tests/sealed_samples.rs:72 (test): .find(|v| v.as_str() == parts[1])
  F? LINE   crates/store/tests/sealed_samples.rs:73 [panic] (test): .unwrap_or_else(|| panic!("{rel}: vendor"));
  GAP LINE   crates/store/tests/sealed_samples.rs:74 (test): let contract = (parts.len() == 8).then(|| Contract::parse(parts[5]).expect("contract"));
  GAP LINE   crates/store/tests/sealed_samples.rs:75 (test): let tf = parts[parts.len() - 2];
  GAP LINE   crates/store/tests/sealed_samples.rs:76 (test): let timeframe = *Timeframe::KNOWN
  GAP LINE   crates/store/tests/sealed_samples.rs:77 (test): .iter()
  GAP LINE   crates/store/tests/sealed_samples.rs:78 (test): .find(|t| t.as_str() == tf)
  F? LINE   crates/store/tests/sealed_samples.rs:79 [panic] (test): .unwrap_or_else(|| panic!("{rel}: rung"));
  GAP LINE   crates/store/tests/sealed_samples.rs:80 (test): let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");
  GAP LINE   crates/store/tests/sealed_samples.rs:81 (test): let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:82 (test): StorePath::new(PathParts {
  GAP LINE   crates/store/tests/sealed_samples.rs:83 (test): vendor,
  GAP LINE   crates/store/tests/sealed_samples.rs:84 (test): exchange: parts[2],
  GAP LINE   crates/store/tests/sealed_samples.rs:85 (test): segment: parts[3],
  GAP LINE   crates/store/tests/sealed_samples.rs:86 (test): symbol: parts[4],
  GAP LINE   crates/store/tests/sealed_samples.rs:87 (test): contract,
  GAP LINE   crates/store/tests/sealed_samples.rs:88 (test): timeframe,
  GAP LINE   crates/store/tests/sealed_samples.rs:89 (test): month,
  GAP LINE   crates/store/tests/sealed_samples.rs:90 (test): file: FileKind::Bars,
  GAP LINE   crates/store/tests/sealed_samples.rs:91 (test): })
  GAP LINE   crates/store/tests/sealed_samples.rs:92 (test): .unwrap()
  GAP LINE   crates/store/tests/sealed_samples.rs:93 (test): }
  GAP LINE   crates/store/tests/sealed_samples.rs:95 (test): fn raw_month(file: &Path) -> (Header, Vec<Bar>) {
  F? LINE   crates/store/tests/sealed_samples.rs:96 [panic] (test): let data = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
  GAP LINE   crates/store/tests/sealed_samples.rs:97 (test): let at = HEADER_LEN as usize;
  GAP LINE   crates/store/tests/sealed_samples.rs:98 (test): let header = Header::read_region(&data[..at], data.len() as u64).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:99 (test): let n = header.n_valid as usize;
  GAP LINE   crates/store/tests/sealed_samples.rs:100 (test): let bars = data[at..at + n * RECORD_LEN]
  GAP LINE   crates/store/tests/sealed_samples.rs:101 (test): .chunks_exact(RECORD_LEN)
  GAP LINE   crates/store/tests/sealed_samples.rs:102 (test): .map(|record| Bar::decode(record).unwrap())
  GAP LINE   crates/store/tests/sealed_samples.rs:103 (test): .collect();
  GAP LINE   crates/store/tests/sealed_samples.rs:104 (test): (header, bars)
  GAP LINE   crates/store/tests/sealed_samples.rs:105 (test): }
  GAP LINE   crates/store/tests/sealed_samples.rs:108 (test): fn us(ns: u128) -> String {
  GAP LINE   crates/store/tests/sealed_samples.rs:109 (test): format!("{}.{}", ns / 1_000, ns % 1_000 / 100)
  GAP LINE   crates/store/tests/sealed_samples.rs:110 (test): }
  GAP LINE   crates/store/tests/sealed_samples.rs:112 (test): fn percentiles(label: &str, mut ns: Vec<u128>) -> String {
  GAP LINE   crates/store/tests/sealed_samples.rs:113 (test): ns.sort_unstable();
  GAP LINE   crates/store/tests/sealed_samples.rs:114 (test): let n = ns.len();
  GAP LINE   crates/store/tests/sealed_samples.rs:115 (test): format!(
  GAP LINE   crates/store/tests/sealed_samples.rs:117 (test): us(ns[n / 2]),
  GAP LINE   crates/store/tests/sealed_samples.rs:118 (test): us(ns[n * 99 / 100]),
  GAP LINE   crates/store/tests/sealed_samples.rs:119 (test): us(ns[n - 1])
  GAP LINE   crates/store/tests/sealed_samples.rs:121 (test): }
  GAP LINE   crates/store/tests/sealed_samples.rs:125 (test): fn real_months_seal_read_back_bit_for_bit_and_answer_like_the_raw_form() {
  GAP LINE   crates/store/tests/sealed_samples.rs:126 (test): let root = PathBuf::from(std::env::var("BRUTEX_SEALED_ROOT").expect("BRUTEX_SEALED_ROOT"));
  GAP LINE   crates/store/tests/sealed_samples.rs:127 (test): let list = std::env::var("BRUTEX_SEALED_MONTHS").expect("BRUTEX_SEALED_MONTHS");
  GAP LINE   crates/store/tests/sealed_samples.rs:128 (test): let out = PathBuf::from(std::env::var("BRUTEX_SEALED_OUT").expect("BRUTEX_SEALED_OUT"));
  GAP LINE   crates/store/tests/sealed_samples.rs:129 (test): let months: Vec<String> = std::fs::read_to_string(&list)
  GAP LINE   crates/store/tests/sealed_samples.rs:130 (test): .unwrap()
  GAP LINE   crates/store/tests/sealed_samples.rs:131 (test): .lines()
  GAP LINE   crates/store/tests/sealed_samples.rs:132 (test): .map(str::trim)
  GAP LINE   crates/store/tests/sealed_samples.rs:133 (test): .filter(|line| !line.is_empty() && !line.starts_with('#'))
  GAP LINE   crates/store/tests/sealed_samples.rs:134 (test): .map(str::to_owned)
  GAP LINE   crates/store/tests/sealed_samples.rs:135 (test): .collect();
  F? LINE   crates/store/tests/sealed_samples.rs:136 [assert] (test): assert!(!months.is_empty(), "{list}: no months");
  GAP LINE   crates/store/tests/sealed_samples.rs:137 (test): let mut rng = Rng(2026);
  GAP LINE   crates/store/tests/sealed_samples.rs:138 (test): for rel in &months {
  GAP LINE   crates/store/tests/sealed_samples.rs:139 (test): let path = store_path(rel);
  GAP LINE   crates/store/tests/sealed_samples.rs:140 (test): let bin = path.to_path_buf(&root);
  GAP LINE   crates/store/tests/sealed_samples.rs:141 (test): let (header, bars) = raw_month(&bin);
  GAP LINE   crates/store/tests/sealed_samples.rs:142 (test): let prv = std::fs::read(path.with_file(FileKind::Provenance).to_path_buf(&root)).ok();
  GAP LINE   crates/store/tests/sealed_samples.rs:143 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_samples.rs:144 (test): let built = build(path, &header, &bars, prv.as_deref(), None).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:145 (test): let seal_us = started.elapsed().as_micros();
  GAP LINE   crates/store/tests/sealed_samples.rs:146 (test): let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&out);
  GAP LINE   crates/store/tests/sealed_samples.rs:147 (test): std::fs::create_dir_all(sbar.parent().unwrap()).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:148 (test): let mut file = std::fs::File::create(&sbar).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:149 (test): built.write_to(&mut file).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:150 (test): file.sync_all().unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:151 (test): drop(file);
  GAP LINE   crates/store/tests/sealed_samples.rs:153 (test): let symbol = header.symbol_id;
  GAP LINE   crates/store/tests/sealed_samples.rs:154 (test): let sealed = SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:155 (test): let audited = sealed.audit().unwrap();
  F? LINE   crates/store/tests/sealed_samples.rs:156 [assert_eq] (test): assert_eq!(audited.rows, bars.len() as u64, "{rel}");
  F? LINE   crates/store/tests/sealed_samples.rs:157 [assert_eq] (test): assert_eq!(sealed.head().bar_header(), header, "{rel}");
  F? LINE   crates/store/tests/sealed_samples.rs:158 [assert_eq] (test): assert_eq!(sealed.provenance().unwrap(), prv, "{rel}");
  GAP LINE   crates/store/tests/sealed_samples.rs:159 (test): let raw = BarFile::open_existing(&root, path, symbol).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:160 (test): let n = bars.len() as u64;
  GAP LINE   crates/store/tests/sealed_samples.rs:161 (test): let mut page = Vec::new();
  GAP LINE   crates/store/tests/sealed_samples.rs:162 (test): let mut first = 0;
  GAP LINE   crates/store/tests/sealed_samples.rs:163 (test): while first < n {
  GAP LINE   crates/store/tests/sealed_samples.rs:164 (test): let count = (n - first).min(1_000);
  GAP LINE   crates/store/tests/sealed_samples.rs:165 (test): page.clear();
  GAP LINE   crates/store/tests/sealed_samples.rs:166 (test): sealed.read_rows(first, count, &mut page).unwrap();
  F? LINE   crates/store/tests/sealed_samples.rs:167 [assert_eq] (test): assert_eq!(
  F? LINE   crates/store/tests/sealed_samples.rs:169 [assert_eq] (test): bars[first as usize..(first + count) as usize],
  GAP LINE   crates/store/tests/sealed_samples.rs:172 (test): first += count;
  GAP LINE   crates/store/tests/sealed_samples.rs:174 (test): for at in (0..n).step_by(1 + n as usize / 500) {
  F? LINE   crates/store/tests/sealed_samples.rs:175 [assert_eq] (test): assert_eq!(
  F? LINE   crates/store/tests/sealed_samples.rs:176 [assert_eq] (test): raw.read_record(at).unwrap(),
  F? LINE   crates/store/tests/sealed_samples.rs:177 [assert_eq] (test): bars[at as usize],
  GAP LINE   crates/store/tests/sealed_samples.rs:182 (test): let (mut open_ns, mut page_sealed, mut page_raw) = (Vec::new(), Vec::new(), Vec::new());
  GAP LINE   crates/store/tests/sealed_samples.rs:183 (test): let (mut row_sealed, mut row_raw, mut find_sealed, mut find_raw) =
  GAP LINE   crates/store/tests/sealed_samples.rs:184 (test): (Vec::new(), Vec::new(), Vec::new(), Vec::new());
  GAP LINE   crates/store/tests/sealed_samples.rs:185 (test): let first_ts = bars.first().map_or(0, |bar| bar.ts_micros);
  GAP LINE   crates/store/tests/sealed_samples.rs:186 (test): let last_ts = bars.last().map_or(0, |bar| bar.ts_micros);
  GAP LINE   crates/store/tests/sealed_samples.rs:187 (test): for _ in 0..2_000 {
  GAP LINE   crates/store/tests/sealed_samples.rs:188 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_samples.rs:189 (test): let month =
  GAP LINE   crates/store/tests/sealed_samples.rs:190 (test): SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:191 (test): open_ns.push(started.elapsed().as_nanos());
  GAP LINE   crates/store/tests/sealed_samples.rs:192 (test): drop(month);
  GAP LINE   crates/store/tests/sealed_samples.rs:193 (test): if n == 0 {
  GAP LINE   crates/store/tests/sealed_samples.rs:194 (test): continue;
  GAP LINE   crates/store/tests/sealed_samples.rs:195 (test): }
  GAP LINE   crates/store/tests/sealed_samples.rs:196 (test): let count = n.min(100);
  GAP LINE   crates/store/tests/sealed_samples.rs:197 (test): let at = rng.below(n - count + 1);
  GAP LINE   crates/store/tests/sealed_samples.rs:198 (test): page.clear();
  GAP LINE   crates/store/tests/sealed_samples.rs:199 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_samples.rs:200 (test): sealed.read_rows(at, count, &mut page).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:201 (test): page_sealed.push(started.elapsed().as_nanos());
  GAP LINE   crates/store/tests/sealed_samples.rs:202 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_samples.rs:203 (test): for i in at..at + count {
  GAP LINE   crates/store/tests/sealed_samples.rs:204 (test): let _ = raw.read_record(i).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:205 (test): }
  GAP LINE   crates/store/tests/sealed_samples.rs:206 (test): page_raw.push(started.elapsed().as_nanos());
  F? LINE   crates/store/tests/sealed_samples.rs:207 [assert_eq] (test): assert_eq!(page, bars[at as usize..(at + count) as usize]);
  GAP LINE   crates/store/tests/sealed_samples.rs:209 (test): let at = rng.below(n);
  GAP LINE   crates/store/tests/sealed_samples.rs:210 (test): page.clear();
  GAP LINE   crates/store/tests/sealed_samples.rs:211 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_samples.rs:212 (test): sealed.read_rows(at, 1, &mut page).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:213 (test): row_sealed.push(started.elapsed().as_nanos());
  GAP LINE   crates/store/tests/sealed_samples.rs:214 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_samples.rs:215 (test): let one = raw.read_record(at).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:216 (test): row_raw.push(started.elapsed().as_nanos());
  F? LINE   crates/store/tests/sealed_samples.rs:217 [assert_eq] (test): assert_eq!(page, [one]);
  GAP LINE   crates/store/tests/sealed_samples.rs:219 (test): let ts = first_ts + rng.below((last_ts - first_ts) as u64 + 1) as i64;
  GAP LINE   crates/store/tests/sealed_samples.rs:220 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_samples.rs:221 (test): let found = sealed.first_at_or_after(ts).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:222 (test): find_sealed.push(started.elapsed().as_nanos());
  GAP LINE   crates/store/tests/sealed_samples.rs:223 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_samples.rs:224 (test): let raw_found = raw.first_at_or_after(ts).unwrap();
  GAP LINE   crates/store/tests/sealed_samples.rs:225 (test): find_raw.push(started.elapsed().as_nanos());
  GAP LINE   crates/store/tests/sealed_samples.rs:226 (test): let scanned = bars.partition_point(|bar| bar.ts_micros < ts) as u64;
  F? LINE   crates/store/tests/sealed_samples.rs:227 [assert_eq] (test): assert_eq!(found, scanned, "{rel} ts {ts}");
  F? LINE   crates/store/tests/sealed_samples.rs:228 [assert_eq] (test): assert_eq!(raw_found, scanned, "{rel} raw ts {ts}");
  GAP LINE   crates/store/tests/sealed_samples.rs:230 (test): let raw_len = std::fs::metadata(&bin).unwrap().len();
  GAP LINE   crates/store/tests/sealed_samples.rs:231 (test): println!(
  GAP LINE   crates/store/tests/sealed_samples.rs:233 (test): sealed.head().n_blocks,
  GAP LINE   crates/store/tests/sealed_samples.rs:234 (test): sealed.head().table_entries,
  GAP LINE   crates/store/tests/sealed_samples.rs:235 (test): built.file_len(),
  GAP LINE   crates/store/tests/sealed_samples.rs:236 (test): built.file_len() * 100 / n.max(1),
  GAP LINE   crates/store/tests/sealed_samples.rs:237 (test): built.file_len() * 1_000 / raw_len,
  GAP LINE   crates/store/tests/sealed_samples.rs:238 (test): sealed.head().prv_raw_len,
  GAP LINE   crates/store/tests/sealed_samples.rs:239 (test): sealed.head().prv_packed_len,
  GAP LINE   crates/store/tests/sealed_samples.rs:241 (test): println!("  {}", percentiles("open sealed (file + head)", open_ns));
  GAP LINE   crates/store/tests/sealed_samples.rs:242 (test): if n > 0 {
  GAP LINE   crates/store/tests/sealed_samples.rs:243 (test): println!("  {}", percentiles("100-row page sealed", page_sealed));
  GAP LINE   crates/store/tests/sealed_samples.rs:244 (test): println!("  {}", percentiles("100-row page raw   ", page_raw));
  GAP LINE   crates/store/tests/sealed_samples.rs:245 (test): println!("  {}", percentiles("one row sealed     ", row_sealed));
  GAP LINE   crates/store/tests/sealed_samples.rs:246 (test): println!("  {}", percentiles("one row raw        ", row_raw));
  GAP LINE   crates/store/tests/sealed_samples.rs:247 (test): println!("  {}", percentiles("first_at_or_after sealed", find_sealed));
  GAP LINE   crates/store/tests/sealed_samples.rs:248 (test): println!("  {}", percentiles("first_at_or_after raw   ", find_raw));
  GAP LINE   crates/store/tests/sealed_samples.rs:249 (test): }
  GAP LINE   crates/store/tests/sealed_samples.rs:251 (test): }
  F? BRANCH crates/store/tests/sealed_samples.rs:66:9 True [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? BRANCH crates/store/tests/sealed_samples.rs:66:9 False [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? BRANCH crates/store/tests/sealed_samples.rs:66:29 True [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? BRANCH crates/store/tests/sealed_samples.rs:66:29 False [assert] (test): parts.len() == 7 || parts.len() == 8,
  GAP BRANCH crates/store/tests/sealed_samples.rs:133:24 True (test): .filter(|line| !line.is_empty() && !line.starts_with('#'))
  GAP BRANCH crates/store/tests/sealed_samples.rs:133:24 False (test): .filter(|line| !line.is_empty() && !line.starts_with('#'))
  GAP BRANCH crates/store/tests/sealed_samples.rs:163:15 True (test): while first < n {
  GAP BRANCH crates/store/tests/sealed_samples.rs:163:15 False (test): while first < n {
  GAP BRANCH crates/store/tests/sealed_samples.rs:193:16 True (test): if n == 0 {
  GAP BRANCH crates/store/tests/sealed_samples.rs:193:16 False (test): if n == 0 {
  GAP BRANCH crates/store/tests/sealed_samples.rs:242:12 True (test): if n > 0 {
  GAP BRANCH crates/store/tests/sealed_samples.rs:242:12 False (test): if n > 0 {
  GAP LINE   crates/store/tests/sealed_write_door.rs:44 (test): fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
  GAP LINE   crates/store/tests/sealed_write_door.rs:45 (test): let y = if m <= 2 { y - 1 } else { y };
  GAP LINE   crates/store/tests/sealed_write_door.rs:46 (test): let era = y.div_euclid(400);
  GAP LINE   crates/store/tests/sealed_write_door.rs:47 (test): let yoe = y - era * 400;
  GAP LINE   crates/store/tests/sealed_write_door.rs:48 (test): let mp = (m + 9) % 12;
  GAP LINE   crates/store/tests/sealed_write_door.rs:49 (test): let doy = (153 * mp + 2) / 5 + d - 1;
  GAP LINE   crates/store/tests/sealed_write_door.rs:50 (test): let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
  GAP LINE   crates/store/tests/sealed_write_door.rs:51 (test): era * 146_097 + doe - 719_468
  GAP LINE   crates/store/tests/sealed_write_door.rs:52 (test): }
  GAP LINE   crates/store/tests/sealed_write_door.rs:54 (test): fn ist_day(bar: &Bar) -> i64 {
  GAP LINE   crates/store/tests/sealed_write_door.rs:55 (test): (bar.ts_micros / 1_000_000 + IST).div_euclid(86_400)
  GAP LINE   crates/store/tests/sealed_write_door.rs:56 (test): }
  GAP LINE   crates/store/tests/sealed_write_door.rs:58 (test): fn store_path(rel: &str) -> (StorePath<'static>, &'static str) {
  GAP LINE   crates/store/tests/sealed_write_door.rs:59 (test): let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
  GAP LINE   crates/store/tests/sealed_write_door.rs:60 (test): let parts: Vec<&'static str> = rel.split('/').collect();
  F? LINE   crates/store/tests/sealed_write_door.rs:61 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_write_door.rs:62 [assert] (test): parts.len() == 7 || parts.len() == 8,
  GAP LINE   crates/store/tests/sealed_write_door.rs:65 (test): let vendor = Vendor::ALL
  GAP LINE   crates/store/tests/sealed_write_door.rs:66 (test): .into_iter()
  GAP LINE   crates/store/tests/sealed_write_door.rs:67 (test): .find(|v| v.as_str() == parts[1])
  GAP LINE   crates/store/tests/sealed_write_door.rs:68 (test): .unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:69 (test): let contract = (parts.len() == 8)
  GAP LINE   crates/store/tests/sealed_write_door.rs:70 (test): .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
  GAP LINE   crates/store/tests/sealed_write_door.rs:71 (test): let tf = parts[parts.len() - 2];
  GAP LINE   crates/store/tests/sealed_write_door.rs:72 (test): let timeframe = *Timeframe::KNOWN.iter().find(|t| t.as_str() == tf).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:73 (test): let stem = parts[parts.len() - 1].strip_suffix(".bin").unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:74 (test): let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:75 (test): let path = StorePath::new(PathParts {
  GAP LINE   crates/store/tests/sealed_write_door.rs:76 (test): vendor,
  GAP LINE   crates/store/tests/sealed_write_door.rs:77 (test): exchange: parts[2],
  GAP LINE   crates/store/tests/sealed_write_door.rs:78 (test): segment: parts[3],
  GAP LINE   crates/store/tests/sealed_write_door.rs:79 (test): symbol: parts[4],
  GAP LINE   crates/store/tests/sealed_write_door.rs:80 (test): contract,
  GAP LINE   crates/store/tests/sealed_write_door.rs:81 (test): timeframe,
  GAP LINE   crates/store/tests/sealed_write_door.rs:82 (test): month,
  GAP LINE   crates/store/tests/sealed_write_door.rs:83 (test): file: FileKind::Bars,
  GAP LINE   crates/store/tests/sealed_write_door.rs:84 (test): })
  GAP LINE   crates/store/tests/sealed_write_door.rs:85 (test): .unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:86 (test): (path, parts[4])
  GAP LINE   crates/store/tests/sealed_write_door.rs:87 (test): }
  GAP LINE   crates/store/tests/sealed_write_door.rs:89 (test): fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
  GAP LINE   crates/store/tests/sealed_write_door.rs:90 (test): let mut files = BTreeMap::new();
  GAP LINE   crates/store/tests/sealed_write_door.rs:91 (test): let mut stack = vec![dir.to_path_buf()];
  GAP LINE   crates/store/tests/sealed_write_door.rs:92 (test): while let Some(at) = stack.pop() {
  GAP LINE   crates/store/tests/sealed_write_door.rs:93 (test): let Ok(entries) = fs::read_dir(&at) else {
  GAP LINE   crates/store/tests/sealed_write_door.rs:94 (test): continue;
  GAP LINE   crates/store/tests/sealed_write_door.rs:96 (test): for entry in entries {
  GAP LINE   crates/store/tests/sealed_write_door.rs:97 (test): let path = entry.unwrap().path();
  GAP LINE   crates/store/tests/sealed_write_door.rs:98 (test): if path.is_dir() {
  GAP LINE   crates/store/tests/sealed_write_door.rs:99 (test): stack.push(path);
  GAP LINE   crates/store/tests/sealed_write_door.rs:100 (test): } else {
  GAP LINE   crates/store/tests/sealed_write_door.rs:101 (test): files.insert(path.clone(), fs::read(&path).unwrap());
  GAP LINE   crates/store/tests/sealed_write_door.rs:102 (test): }
  GAP LINE   crates/store/tests/sealed_write_door.rs:105 (test): files
  GAP LINE   crates/store/tests/sealed_write_door.rs:106 (test): }
  GAP LINE   crates/store/tests/sealed_write_door.rs:108 (test): fn micros(started: Instant) -> f64 {
  GAP LINE   crates/store/tests/sealed_write_door.rs:109 (test): started.elapsed().as_secs_f64() * 1e6
  GAP LINE   crates/store/tests/sealed_write_door.rs:110 (test): }
  GAP LINE   crates/store/tests/sealed_write_door.rs:117 (test): fn the_write_door_over_real_months_sealed_without_one_day() {
  GAP LINE   crates/store/tests/sealed_write_door.rs:118 (test): let source = PathBuf::from(std::env::var("BRUTEX_SC4_SOURCE").expect("BRUTEX_SC4_SOURCE"));
  GAP LINE   crates/store/tests/sealed_write_door.rs:119 (test): let work = PathBuf::from(std::env::var("BRUTEX_SC4_WORK").expect("BRUTEX_SC4_WORK"));
  GAP LINE   crates/store/tests/sealed_write_door.rs:120 (test): let list = std::env::var("BRUTEX_SC4_MONTHS").expect("BRUTEX_SC4_MONTHS");
  GAP LINE   crates/store/tests/sealed_write_door.rs:121 (test): for root in [&source, &work] {
  F? LINE   crates/store/tests/sealed_write_door.rs:122 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_write_door.rs:123 [assert] (test): !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
  F? LINE   crates/store/tests/sealed_write_door.rs:126 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_write_door.rs:127 [assert] (test): !fs::symlink_metadata(root.join("bars"))
  F? LINE   crates/store/tests/sealed_write_door.rs:128 [assert] (test): .is_ok_and(|meta| meta.file_type().is_symlink()),
  F? LINE   crates/store/tests/sealed_write_door.rs:130 [assert] (test): root.display()
  GAP LINE   crates/store/tests/sealed_write_door.rs:133 (test): for (line_no, line) in fs::read_to_string(&list).unwrap().lines().enumerate() {
  GAP LINE   crates/store/tests/sealed_write_door.rs:134 (test): let line = line.trim();
  GAP LINE   crates/store/tests/sealed_write_door.rs:135 (test): if line.is_empty() || line.starts_with('#') {
  GAP LINE   crates/store/tests/sealed_write_door.rs:136 (test): continue;
  GAP LINE   crates/store/tests/sealed_write_door.rs:137 (test): }
  GAP LINE   crates/store/tests/sealed_write_door.rs:138 (test): let (rel, date) = line.split_once(' ').expect("<path> <yyyy-mm-dd>");
  GAP LINE   crates/store/tests/sealed_write_door.rs:139 (test): let (path, symbol) = store_path(rel);
  GAP LINE   crates/store/tests/sealed_write_door.rs:140 (test): let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;
  GAP LINE   crates/store/tests/sealed_write_door.rs:141 (test): let full = {
  GAP LINE   crates/store/tests/sealed_write_door.rs:142 (test): let file = BarFile::open_existing(&source, path, symbol_id).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:143 (test): file.read_range(0, file.records()).unwrap()
  GAP LINE   crates/store/tests/sealed_write_door.rs:146 (test): let day = if date == "last" {
  GAP LINE   crates/store/tests/sealed_write_door.rs:147 (test): ist_day(full.last().expect("a month with bars"))
  GAP LINE   crates/store/tests/sealed_write_door.rs:149 (test): let ymd: Vec<i64> = date.split('-').map(|p| p.parse().unwrap()).collect();
  GAP LINE   crates/store/tests/sealed_write_door.rs:150 (test): days_from_civil(ymd[0], ymd[1], ymd[2])
  GAP LINE   crates/store/tests/sealed_write_door.rs:152 (test): let (held, offered): (Vec<Bar>, Vec<Bar>) =
  GAP LINE   crates/store/tests/sealed_write_door.rs:153 (test): full.iter().copied().partition(|bar| ist_day(bar) != day);
  F? LINE   crates/store/tests/sealed_write_door.rs:154 [assert] (test): assert!(!offered.is_empty(), "{rel}: no bar on {date}");
  F? LINE   crates/store/tests/sealed_write_door.rs:155 [assert] (test): assert!(!held.is_empty(), "{rel}: nothing but {date}");
  GAP LINE   crates/store/tests/sealed_write_door.rs:156 (test): let root = work.join(format!("m{line_no}"));
  GAP LINE   crates/store/tests/sealed_write_door.rs:157 (test): fs::create_dir_all(&root).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:159 (test): let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:160 (test): for chunk in held.chunks(65_536) {
  GAP LINE   crates/store/tests/sealed_write_door.rs:161 (test): writer.append(chunk).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:162 (test): }
  GAP LINE   crates/store/tests/sealed_write_door.rs:164 (test): let raw = BarFile::open_existing(&root, path, symbol_id).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:165 (test): let built = store::sealed::build(path, &raw.header(), &held, None, None).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:166 (test): drop(raw);
  GAP LINE   crates/store/tests/sealed_write_door.rs:167 (test): let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&root);
  GAP LINE   crates/store/tests/sealed_write_door.rs:168 (test): let staged = sbar.with_extension("sbar-staged");
  GAP LINE   crates/store/tests/sealed_write_door.rs:169 (test): fs::write(&staged, built.to_bytes()).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:170 (test): fs::rename(&staged, &sbar).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:171 (test): let before = snapshot(&root.join("bars"));
  GAP LINE   crates/store/tests/sealed_write_door.rs:173 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_write_door.rs:174 (test): let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:175 (test): let open_us = micros(started);
  F? LINE   crates/store/tests/sealed_write_door.rs:176 [assert_eq] (test): assert_eq!(writer.form(), Form::Sealed, "{rel}");
  GAP LINE   crates/store/tests/sealed_write_door.rs:177 (test): let first_day = ist_day(&held[0]);
  GAP LINE   crates/store/tests/sealed_write_door.rs:178 (test): let reoffer: Vec<Bar> = held
  GAP LINE   crates/store/tests/sealed_write_door.rs:179 (test): .iter()
  GAP LINE   crates/store/tests/sealed_write_door.rs:180 (test): .copied()
  GAP LINE   crates/store/tests/sealed_write_door.rs:181 (test): .filter(|bar| ist_day(bar) == first_day)
  GAP LINE   crates/store/tests/sealed_write_door.rs:182 (test): .collect();
  GAP LINE   crates/store/tests/sealed_write_door.rs:183 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_write_door.rs:184 (test): let again = writer.append(&reoffer).unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:185 (test): let reoffer_us = micros(started);
  F? LINE   crates/store/tests/sealed_write_door.rs:186 [assert_eq] (test): assert_eq!(
  F? LINE   crates/store/tests/sealed_write_door.rs:188 [assert_eq] (test): Appended::AlreadyPresent {
  GAP LINE   crates/store/tests/sealed_write_door.rs:189 (test): first_index: 0,
  GAP LINE   crates/store/tests/sealed_write_door.rs:190 (test): n_valid: held.len() as u64
  GAP LINE   crates/store/tests/sealed_write_door.rs:191 (test): },
  GAP LINE   crates/store/tests/sealed_write_door.rs:194 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_write_door.rs:195 (test): let refused = writer.append(&offered).unwrap_err();
  GAP LINE   crates/store/tests/sealed_write_door.rs:196 (test): let refused_us = micros(started);
  GAP LINE   crates/store/tests/sealed_write_door.rs:197 (test): let StoreError::Sealed { offer, at, .. } = refused else {
  F? LINE   crates/store/tests/sealed_write_door.rs:198 [panic] (test): panic!("{rel}: {refused}");
  GAP LINE   crates/store/tests/sealed_write_door.rs:200 (test): let is_last_day = day == ist_day(full.last().unwrap());
  F? LINE   crates/store/tests/sealed_write_door.rs:201 [assert_eq] (test): assert_eq!(
  F? LINE   crates/store/tests/sealed_write_door.rs:203 [assert_eq] (test): if is_last_day {
  GAP LINE   crates/store/tests/sealed_write_door.rs:204 (test): SealedOffer::Follows
  GAP LINE   crates/store/tests/sealed_write_door.rs:206 (test): SealedOffer::Disagrees(store::file::Conflict::NotHeld)
  F? LINE   crates/store/tests/sealed_write_door.rs:210 [assert_eq] (test): assert_eq!(at, 0, "{rel}");
  GAP LINE   crates/store/tests/sealed_write_door.rs:211 (test): drop(writer);
  F? LINE   crates/store/tests/sealed_write_door.rs:212 [assert_eq] (test): assert_eq!(
  F? LINE   crates/store/tests/sealed_write_door.rs:213 [assert_eq] (test): snapshot(&root.join("bars")),
  GAP LINE   crates/store/tests/sealed_write_door.rs:218 (test): let started = Instant::now();
  GAP LINE   crates/store/tests/sealed_write_door.rs:219 (test): let repaired = store::repair::repair_month(
  GAP LINE   crates/store/tests/sealed_write_door.rs:220 (test): &root,
  GAP LINE   crates/store/tests/sealed_write_door.rs:221 (test): path,
  GAP LINE   crates/store/tests/sealed_write_door.rs:222 (test): symbol_id,
  GAP LINE   crates/store/tests/sealed_write_door.rs:223 (test): &offered,
  GAP LINE   crates/store/tests/sealed_write_door.rs:224 (test): Policy {
  GAP LINE   crates/store/tests/sealed_write_door.rs:225 (test): // No offered bar shares a stamp with a held one: the day is
  GAP LINE   crates/store/tests/sealed_write_door.rs:226 (test): // wholly absent, so the comparison this sets is never made.
  GAP LINE   crates/store/tests/sealed_write_door.rs:227 (test): open_interest_applies: true,
  GAP LINE   crates/store/tests/sealed_write_door.rs:228 (test): basis_witness: false,
  GAP LINE   crates/store/tests/sealed_write_door.rs:229 (test): },
  GAP LINE   crates/store/tests/sealed_write_door.rs:231 (test): .unwrap();
  GAP LINE   crates/store/tests/sealed_write_door.rs:232 (test): let repair_ms = micros(started) / 1e3;
  GAP LINE   crates/store/tests/sealed_write_door.rs:233 (test): let MonthRepair::Repaired { added, .. } = repaired else {
  F? LINE   crates/store/tests/sealed_write_door.rs:234 [panic] (test): panic!("{rel}: {repaired:?}");
  F? LINE   crates/store/tests/sealed_write_door.rs:236 [assert_eq] (test): assert_eq!(added, offered.len() as u64, "{rel}");
  GAP LINE   crates/store/tests/sealed_write_door.rs:237 (test): let served = BarFile::open_existing(&root, path, symbol_id).unwrap();
  F? LINE   crates/store/tests/sealed_write_door.rs:238 [assert_eq] (test): assert_eq!(served.form(), Form::Raw, "{rel}: the successor is raw");
  GAP LINE   crates/store/tests/sealed_write_door.rs:239 (test): let now = served.read_range(0, served.records()).unwrap();
  F? LINE   crates/store/tests/sealed_write_door.rs:240 [assert] (test): assert!(
  F? LINE   crates/store/tests/sealed_write_door.rs:241 [assert] (test): now == full,
  F? LINE   crates/store/tests/sealed_write_door.rs:244 [assert_eq] (test): assert_eq!(
  F? LINE   crates/store/tests/sealed_write_door.rs:245 [assert_eq] (test): snapshot(&root.join("bars")),
  GAP LINE   crates/store/tests/sealed_write_door.rs:249 (test): println!(
  GAP LINE   crates/store/tests/sealed_write_door.rs:253 (test): full.len(),
  GAP LINE   crates/store/tests/sealed_write_door.rs:254 (test): held.len(),
  GAP LINE   crates/store/tests/sealed_write_door.rs:255 (test): offered.len(),
  GAP LINE   crates/store/tests/sealed_write_door.rs:256 (test): offer.as_str(),
  GAP LINE   crates/store/tests/sealed_write_door.rs:257 (test): built.file_len()
  GAP LINE   crates/store/tests/sealed_write_door.rs:260 (test): }
  GAP BRANCH crates/store/tests/sealed_write_door.rs:45:16 True (test): let y = if m <= 2 { y - 1 } else { y };
  GAP BRANCH crates/store/tests/sealed_write_door.rs:45:16 False (test): let y = if m <= 2 { y - 1 } else { y };
  F? BRANCH crates/store/tests/sealed_write_door.rs:62:9 True [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? BRANCH crates/store/tests/sealed_write_door.rs:62:9 False [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? BRANCH crates/store/tests/sealed_write_door.rs:62:29 True [assert] (test): parts.len() == 7 || parts.len() == 8,
  F? BRANCH crates/store/tests/sealed_write_door.rs:62:29 False [assert] (test): parts.len() == 7 || parts.len() == 8,
  GAP BRANCH crates/store/tests/sealed_write_door.rs:92:15 True (test): while let Some(at) = stack.pop() {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:92:15 False (test): while let Some(at) = stack.pop() {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:93:13 True (test): let Ok(entries) = fs::read_dir(&at) else {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:93:13 False (test): let Ok(entries) = fs::read_dir(&at) else {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:98:16 True (test): if path.is_dir() {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:98:16 False (test): if path.is_dir() {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:135:12 True (test): if line.is_empty() || line.starts_with('#') {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:135:12 False (test): if line.is_empty() || line.starts_with('#') {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:135:31 True (test): if line.is_empty() || line.starts_with('#') {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:135:31 False (test): if line.is_empty() || line.starts_with('#') {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:146:22 True (test): let day = if date == "last" {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:146:22 False (test): let day = if date == "last" {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:197:13 True (test): let StoreError::Sealed { offer, at, .. } = refused else {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:197:13 False (test): let StoreError::Sealed { offer, at, .. } = refused else {
  F? BRANCH crates/store/tests/sealed_write_door.rs:203:16 True [assert_eq] (test): if is_last_day {
  F? BRANCH crates/store/tests/sealed_write_door.rs:203:16 False [assert_eq] (test): if is_last_day {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:233:13 True (test): let MonthRepair::Repaired { added, .. } = repaired else {
  GAP BRANCH crates/store/tests/sealed_write_door.rs:233:13 False (test): let MonthRepair::Repaired { added, .. } = repaired else {
  F? LINE   crates/store/tests/slot_audit.rs:116 [assert] (test): matches!(
  F? LINE   crates/store/tests/slot_audit.rs:137 [panic] (test): panic!("v{version}: a damaged newest slot is a revert: {audit:?}");
  F? LINE   crates/store/tests/slot_audit.rs:277 [assert] (test): matches!(
  F? LINE   crates/store/tests/slot_audit.rs:347 [assert] (test): matches!(
  F? LINE   crates/store/tests/slot_audit.rs:410 [assert] (test): matches!(
  F? LINE   crates/store/tests/slot_audit.rs:485 [panic] (test): panic!("damaged");
  GAP BRANCH crates/store/tests/slot_audit.rs:130:13 False (test): let MonthVerdict::Reverted {
  GAP BRANCH crates/store/tests/slot_audit.rs:484:9 False (test): let SlotState::Damaged { reason, .. } = audit.slots[1] else {
  F? LINE   crates/store/tests/slot_audit_sealed.rs:95 [assert] (test): raw.slot_audit()
  GAP LINE   crates/store/tests/slot_audit_sealed.rs:149 (test): Ok(file) => {
  F? LINE   crates/store/tests/slot_audit_sealed.rs:150 [assert_eq] (test): assert_eq!(file.form(), Form::Sealed, "byte {at}");
  F? LINE   crates/store/tests/slot_audit_sealed.rs:151 [assert_eq] (test): assert_eq!(file.slot_audit(), None, "byte {at}");
  F? LINE   crates/store/tests/slot_audit_sealed.rs:152 [assert_eq] (test): assert_eq!(file.header().n_valid, 40, "byte {at}: never read short");
  GAP LINE   crates/store/tests/slot_audit_sealed.rs:153 (test): if let Ok(last) = file.read_record(39) {
  F? LINE   crates/store/tests/slot_audit_sealed.rs:154 [assert_eq] (test): assert_eq!(last.close, 7_039, "byte {at}");
  GAP LINE   crates/store/tests/slot_audit_sealed.rs:155 (test): }
  GAP BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 True (test): if let Ok(last) = file.read_record(39) {
  GAP BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 False (test): if let Ok(last) = file.read_record(39) {
  GAP LINE   crates/store/tests/slot_damaged_log.rs:93 (test): Some(OwnedValue::Uint(value)) => *value,
  F? LINE   crates/store/tests/slot_damaged_log.rs:94 [panic] (test): other => panic!("{key}: {other:?}"),
  F? LINE   crates/store/tests/slot_damaged_log.rs:101 [panic] (test): other => panic!("{key}: {other:?}"),
  F? LINE   crates/store/tests/slot_damaged_log.rs:192 [assert_eq] (test): found
  F? LINE   crates/store/tests/slot_damaged_log.rs:193 [assert_eq] (test): .records
  F? LINE   crates/store/tests/slot_damaged_log.rs:194 [assert_eq] (test): .iter()
  F? LINE   crates/store/tests/slot_damaged_log.rs:195 [assert_eq] (test): .map(|r| (r.target.clone(), r.message.clone()))
  F? LINE   crates/store/tests/slot_damaged_log.rs:196 [assert_eq] (test): .collect::<Vec<_>>()
  F? LINE   crates/store/tests/sort_index.rs:488 [assert] (test): index.torn()
  F? LINE   crates/store/tests/sort_index.rs:761 [assert_eq] (test): column.as_str()
  F? LINE   crates/store/tests/sort_index.rs:843 [assert_eq] (test): column.as_str()
  F? BRANCH crates/store/tests/sort_index.rs:387:62 False [assert] (test): matches!(&again.outcome, Resorted::Built(why) if why.contains("fails the checksum")),
  F? BRANCH crates/store/tests/sort_index.rs:430:56 False [assert] (test): matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),
  F? BRANCH crates/store/tests/sort_index.rs:446:63 False [assert] (test): assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));
  F? BRANCH crates/store/tests/sort_index.rs:660:58 False [assert] (test): matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  F? BRANCH crates/store/tests/sort_index.rs:736:58 False [assert] (test): matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  F? BRANCH crates/store/tests/sort_index.rs:874:16 False [assert] (test): if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  F? BRANCH crates/store/tests/sort_index.rs:874:54 False [assert] (test): if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  F? BRANCH crates/store/tests/sort_index.rs:889:16 False [assert] (test): if reason.contains("2031-02.new.srt")),
  GAP LINE   crates/store/tests/sort_index_log.rs:74 (test): OwnedValue::Uint(n) => n.to_string(),
  GAP LINE   crates/store/tests/sort_index_log.rs:75 (test): other => format!("{other:?}"),
  GAP BRANCH crates/store/tests/support/mod.rs:62:15 False (test): let ran = output.status.success() && stdout.contains("1 passed");
  F? BRANCH crates/store/tests/tail_proof.rs:144:9 False [assert] (test): found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  F? BRANCH crates/store/tests/tail_proof.rs:144:36 False [assert] (test): found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  F? BRANCH crates/store/tests/tail_proof.rs:144:60 False [assert] (test): found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  F? LINE   crates/store/tests/tix.rs:173 [assert_eq] (test): timeframe.as_str()
  GAP BRANCH crates/store/tests/tix.rs:127:12 False (test): if let Some(last) = bars.last() {
  GAP BRANCH crates/store/tests/tix.rs:605:76 True (test): && matches!(value, telemetry::OwnedValue::Str(file) if file.starts_with(&ours))
  GAP LINE   crates/store/tests/tix_latency.rs:43 (test): fn path(timeframe: Timeframe) -> StorePath<'static> {
  GAP LINE   crates/store/tests/tix_latency.rs:44 (test): StorePath::new(PathParts {
  GAP LINE   crates/store/tests/tix_latency.rs:45 (test): vendor: Vendor::Groww,
  GAP LINE   crates/store/tests/tix_latency.rs:46 (test): exchange: "NSE",
  GAP LINE   crates/store/tests/tix_latency.rs:47 (test): segment: "INDEX",
  GAP LINE   crates/store/tests/tix_latency.rs:48 (test): symbol: "NIFTY",
  GAP LINE   crates/store/tests/tix_latency.rs:49 (test): contract: None,
  GAP LINE   crates/store/tests/tix_latency.rs:50 (test): timeframe,
  GAP LINE   crates/store/tests/tix_latency.rs:51 (test): month: YearMonth::new(2024, 7).unwrap(),
  GAP LINE   crates/store/tests/tix_latency.rs:52 (test): file: FileKind::Bars,
  GAP LINE   crates/store/tests/tix_latency.rs:53 (test): })
  GAP LINE   crates/store/tests/tix_latency.rs:54 (test): .unwrap()
  GAP LINE   crates/store/tests/tix_latency.rs:55 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:59 (test): fn month(days: i64, per_day: i64, step: i64, weekdays: bool) -> Vec<Bar> {
  GAP LINE   crates/store/tests/tix_latency.rs:60 (test): let mut out = Vec::new();
  GAP LINE   crates/store/tests/tix_latency.rs:61 (test): let (mut day, mut taken) = (0, 0);
  GAP LINE   crates/store/tests/tix_latency.rs:62 (test): while taken < days {
  GAP LINE   crates/store/tests/tix_latency.rs:64 (test): if !weekdays || day % 7 < 5 {
  GAP LINE   crates/store/tests/tix_latency.rs:65 (test): for k in 0..per_day {
  GAP LINE   crates/store/tests/tix_latency.rs:66 (test): let ts = JULY_START + day * 86_400 * MICROS + (555 * 60 + k * step) * MICROS;
  GAP LINE   crates/store/tests/tix_latency.rs:67 (test): out.push(Bar {
  GAP LINE   crates/store/tests/tix_latency.rs:68 (test): ts_micros: ts,
  GAP LINE   crates/store/tests/tix_latency.rs:69 (test): open: 2_000_000 + k,
  GAP LINE   crates/store/tests/tix_latency.rs:70 (test): high: 2_000_100 + k,
  GAP LINE   crates/store/tests/tix_latency.rs:71 (test): low: 1_999_900 + k,
  GAP LINE   crates/store/tests/tix_latency.rs:72 (test): close: 2_000_050 + k,
  GAP LINE   crates/store/tests/tix_latency.rs:73 (test): volume: 10,
  GAP LINE   crates/store/tests/tix_latency.rs:74 (test): open_interest: OI_NULL,
  GAP LINE   crates/store/tests/tix_latency.rs:75 (test): });
  GAP LINE   crates/store/tests/tix_latency.rs:76 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:77 (test): taken += 1;
  GAP LINE   crates/store/tests/tix_latency.rs:78 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:79 (test): day += 1;
  GAP LINE   crates/store/tests/tix_latency.rs:81 (test): out
  GAP LINE   crates/store/tests/tix_latency.rs:82 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:87 (test): fn next(&mut self) -> u64 {
  GAP LINE   crates/store/tests/tix_latency.rs:88 (test): self.0 ^= self.0 >> 12;
  GAP LINE   crates/store/tests/tix_latency.rs:89 (test): self.0 ^= self.0 << 25;
  GAP LINE   crates/store/tests/tix_latency.rs:90 (test): self.0 ^= self.0 >> 27;
  GAP LINE   crates/store/tests/tix_latency.rs:91 (test): self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
  GAP LINE   crates/store/tests/tix_latency.rs:92 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:95 (test): fn percentiles(mut ns: Vec<u64>) -> (u64, u64, u64) {
  GAP LINE   crates/store/tests/tix_latency.rs:96 (test): ns.sort_unstable();
  GAP LINE   crates/store/tests/tix_latency.rs:97 (test): let at = |q: usize| ns[(ns.len() * q / 1000).min(ns.len() - 1)];
  GAP LINE   crates/store/tests/tix_latency.rs:98 (test): (at(500), at(990), *ns.last().unwrap())
  GAP LINE   crates/store/tests/tix_latency.rs:99 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:101 (test): fn report(case: &str, ns: Vec<u64>) {
  GAP LINE   crates/store/tests/tix_latency.rs:102 (test): let (p50, p99, max) = percentiles(ns);
  GAP LINE   crates/store/tests/tix_latency.rs:103 (test): println!("{case:<58} p50 {p50:>8} ns   p99 {p99:>8} ns   max {max:>9} ns");
  GAP LINE   crates/store/tests/tix_latency.rs:104 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:107 (test): fn time<T: Copy>(inputs: &[T], mut op: impl FnMut(T) -> u64) -> Vec<u64> {
  GAP LINE   crates/store/tests/tix_latency.rs:108 (test): inputs
  GAP LINE   crates/store/tests/tix_latency.rs:109 (test): .iter()
  GAP LINE   crates/store/tests/tix_latency.rs:110 (test): .map(|&input| {
  GAP LINE   crates/store/tests/tix_latency.rs:111 (test): let start = Instant::now();
  GAP LINE   crates/store/tests/tix_latency.rs:112 (test): black_box(op(black_box(input)));
  GAP LINE   crates/store/tests/tix_latency.rs:113 (test): start.elapsed().as_nanos() as u64
  GAP LINE   crates/store/tests/tix_latency.rs:114 (test): })
  GAP LINE   crates/store/tests/tix_latency.rs:115 (test): .collect()
  GAP LINE   crates/store/tests/tix_latency.rs:116 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:118 (test): fn measure(name: &str, timeframe: Timeframe, bars: &[Bar]) {
  GAP LINE   crates/store/tests/tix_latency.rs:119 (test): let root: PathBuf =
  GAP LINE   crates/store/tests/tix_latency.rs:120 (test): std::env::temp_dir().join(format!("brutex-tix-latency-{name}-{}", std::process::id()));
  GAP LINE   crates/store/tests/tix_latency.rs:121 (test): let _ = fs::remove_dir_all(&root);
  GAP LINE   crates/store/tests/tix_latency.rs:122 (test): fs::create_dir_all(&root).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:124 (test): let mut writer = BarFile::open_or_create(&root, path(timeframe), 7).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:125 (test): for chunk in bars.chunks(22_500) {
  GAP LINE   crates/store/tests/tix_latency.rs:126 (test): writer.append(chunk).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:127 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:129 (test): let n = bars.len() as u64;
  GAP LINE   crates/store/tests/tix_latency.rs:130 (test): let first = bars[0].ts_micros;
  GAP LINE   crates/store/tests/tix_latency.rs:131 (test): let last = bars[bars.len() - 1].ts_micros;
  GAP LINE   crates/store/tests/tix_latency.rs:132 (test): let width = i64::from(timeframe.secs()) * MICROS;
  GAP LINE   crates/store/tests/tix_latency.rs:133 (test): let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
  GAP LINE   crates/store/tests/tix_latency.rs:135 (test): let anywhere: Vec<i64> = (0..LOOKUPS)
  GAP LINE   crates/store/tests/tix_latency.rs:136 (test): .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)
  GAP LINE   crates/store/tests/tix_latency.rs:137 (test): .collect();
  GAP LINE   crates/store/tests/tix_latency.rs:139 (test): let on_grid: Vec<i64> = anywhere
  GAP LINE   crates/store/tests/tix_latency.rs:140 (test): .iter()
  GAP LINE   crates/store/tests/tix_latency.rs:141 (test): .map(|&ts| first + (ts - first) / width * width)
  GAP LINE   crates/store/tests/tix_latency.rs:142 (test): .collect();
  GAP LINE   crates/store/tests/tix_latency.rs:143 (test): let rows: Vec<u64> = (0..LOOKUPS).map(|_| rng.next() % n).collect();
  GAP LINE   crates/store/tests/tix_latency.rs:145 (test): let tix = path(timeframe).to_path_buf(&root).with_extension("tix");
  GAP LINE   crates/store/tests/tix_latency.rs:146 (test): let passes: &[bool] = if tix.exists() {
  GAP LINE   crates/store/tests/tix_latency.rs:147 (test): &[true, false]
  GAP LINE   crates/store/tests/tix_latency.rs:149 (test): &[false]
  GAP LINE   crates/store/tests/tix_latency.rs:151 (test): for &with_index in passes {
  GAP LINE   crates/store/tests/tix_latency.rs:152 (test): if !with_index && tix.exists() {
  GAP LINE   crates/store/tests/tix_latency.rs:153 (test): fs::remove_file(&tix).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:154 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:155 (test): let tag = if with_index { "index" } else { "bisection" };
  GAP LINE   crates/store/tests/tix_latency.rs:156 (test): let label = |what: &str| format!("{name} ({n} bars) {what} [{tag}]");
  GAP LINE   crates/store/tests/tix_latency.rs:159 (test): let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:160 (test): report(
  GAP LINE   crates/store/tests/tix_latency.rs:161 (test): &label("by time, random us"),
  GAP LINE   crates/store/tests/tix_latency.rs:162 (test): time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),
  GAP LINE   crates/store/tests/tix_latency.rs:164 (test): let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:165 (test): report(
  GAP LINE   crates/store/tests/tix_latency.rs:166 (test): &label("by time, on grid"),
  GAP LINE   crates/store/tests/tix_latency.rs:167 (test): time(&on_grid, |ts| file.first_at_or_after(ts).unwrap()),
  GAP LINE   crates/store/tests/tix_latency.rs:169 (test): let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:170 (test): report(
  GAP LINE   crates/store/tests/tix_latency.rs:171 (test): &label("by row, random rows (cold block)"),
  GAP LINE   crates/store/tests/tix_latency.rs:172 (test): time(&rows, |row| file.read_record(row).unwrap().ts_micros as u64),
  GAP LINE   crates/store/tests/tix_latency.rs:176 (test): let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:177 (test): let same = vec![rows[0]; LOOKUPS];
  GAP LINE   crates/store/tests/tix_latency.rs:178 (test): report(
  GAP LINE   crates/store/tests/tix_latency.rs:179 (test): &label("by row, one row (warm block)"),
  GAP LINE   crates/store/tests/tix_latency.rs:180 (test): time(&same, |row| file.read_record(row).unwrap().ts_micros as u64),
  GAP LINE   crates/store/tests/tix_latency.rs:183 (test): let _ = fs::remove_dir_all(&root);
  GAP LINE   crates/store/tests/tix_latency.rs:184 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:188 (test): fn time_and_row_lookup_latency() {
  GAP LINE   crates/store/tests/tix_latency.rs:189 (test): measure("1min", Timeframe::MINUTE_1, &month(31, 375, 60, false));
  GAP LINE   crates/store/tests/tix_latency.rs:190 (test): measure("1s", Timeframe::SECOND_1, &month(23, 22_500, 1, true));
  GAP LINE   crates/store/tests/tix_latency.rs:191 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:209 (test): fn small_month_rule_latency() {
  GAP LINE   crates/store/tests/tix_latency.rs:211 (test): let root: PathBuf =
  GAP LINE   crates/store/tests/tix_latency.rs:212 (test): std::env::temp_dir().join(format!("brutex-tix-latency-small-{}", std::process::id()));
  GAP LINE   crates/store/tests/tix_latency.rs:213 (test): let _ = fs::remove_dir_all(&root);
  GAP LINE   crates/store/tests/tix_latency.rs:214 (test): fs::create_dir_all(&root).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:215 (test): let all = month(1, 1_100, 1, true);
  GAP LINE   crates/store/tests/tix_latency.rs:216 (test): for (name, n) in [
  GAP LINE   crates/store/tests/tix_latency.rs:217 (test): ("1s, 1023 bars, below", 1_023_usize),
  GAP LINE   crates/store/tests/tix_latency.rs:218 (test): ("1s, 1024 bars, at", 1_024),
  GAP LINE   crates/store/tests/tix_latency.rs:219 (test): ] {
  GAP LINE   crates/store/tests/tix_latency.rs:220 (test): let at = root.join(name.replace([' ', ','], "-"));
  GAP LINE   crates/store/tests/tix_latency.rs:221 (test): fs::create_dir_all(&at).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:222 (test): BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7)
  GAP LINE   crates/store/tests/tix_latency.rs:223 (test): .unwrap()
  GAP LINE   crates/store/tests/tix_latency.rs:224 (test): .append(&all[..n])
  GAP LINE   crates/store/tests/tix_latency.rs:225 (test): .unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:226 (test): let tix = path(Timeframe::SECOND_1)
  GAP LINE   crates/store/tests/tix_latency.rs:227 (test): .to_path_buf(&at)
  GAP LINE   crates/store/tests/tix_latency.rs:228 (test): .with_extension("tix");
  F? LINE   crates/store/tests/tix_latency.rs:229 [assert_eq] (test): assert_eq!(tix.exists(), n >= 1_024, "{name}");
  GAP LINE   crates/store/tests/tix_latency.rs:230 (test): let (first, last) = (all[0].ts_micros, all[n - 1].ts_micros);
  GAP LINE   crates/store/tests/tix_latency.rs:231 (test): let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
  GAP LINE   crates/store/tests/tix_latency.rs:232 (test): let anywhere: Vec<i64> = (0..LOOKUPS)
  GAP LINE   crates/store/tests/tix_latency.rs:233 (test): .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)
  GAP LINE   crates/store/tests/tix_latency.rs:234 (test): .collect();
  GAP LINE   crates/store/tests/tix_latency.rs:235 (test): let file = BarFile::open_existing(&at, path(Timeframe::SECOND_1), 7).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:236 (test): report(
  GAP LINE   crates/store/tests/tix_latency.rs:237 (test): &format!("{name}: by time, random us [{:?}]", file.time_lookup()),
  GAP LINE   crates/store/tests/tix_latency.rs:238 (test): time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),
  GAP LINE   crates/store/tests/tix_latency.rs:241 (test): for (name, held) in [
  GAP LINE   crates/store/tests/tix_latency.rs:242 (test): ("commit of 1 bar under the threshold (500 held)", 500_usize),
  GAP LINE   crates/store/tests/tix_latency.rs:243 (test): ("commit of 1 bar reaching the threshold (1023 held)", 1_023),
  GAP LINE   crates/store/tests/tix_latency.rs:244 (test): ("commit of 1 bar past the threshold (1099 held)", 1_099),
  GAP LINE   crates/store/tests/tix_latency.rs:245 (test): ] {
  GAP LINE   crates/store/tests/tix_latency.rs:246 (test): let mut ns = Vec::with_capacity(COMMITS);
  GAP LINE   crates/store/tests/tix_latency.rs:247 (test): for k in 0..COMMITS {
  GAP LINE   crates/store/tests/tix_latency.rs:248 (test): let at = root.join(format!("commit-{held}-{k}"));
  GAP LINE   crates/store/tests/tix_latency.rs:249 (test): fs::create_dir_all(&at).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:250 (test): let mut writer = BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:251 (test): writer.append(&all[..held]).unwrap();
  GAP LINE   crates/store/tests/tix_latency.rs:252 (test): let start = Instant::now();
  GAP LINE   crates/store/tests/tix_latency.rs:253 (test): black_box(writer.append(black_box(&all[held..=held])).unwrap());
  GAP LINE   crates/store/tests/tix_latency.rs:254 (test): ns.push(start.elapsed().as_nanos() as u64);
  GAP LINE   crates/store/tests/tix_latency.rs:255 (test): drop(writer);
  GAP LINE   crates/store/tests/tix_latency.rs:256 (test): let _ = fs::remove_dir_all(&at);
  GAP LINE   crates/store/tests/tix_latency.rs:257 (test): }
  GAP LINE   crates/store/tests/tix_latency.rs:258 (test): report(&format!("{name}, n={COMMITS}"), ns);
  GAP LINE   crates/store/tests/tix_latency.rs:260 (test): let _ = fs::remove_dir_all(&root);
  GAP LINE   crates/store/tests/tix_latency.rs:261 (test): }
  GAP BRANCH crates/store/tests/tix_latency.rs:62:11 True (test): while taken < days {
  GAP BRANCH crates/store/tests/tix_latency.rs:62:11 False (test): while taken < days {
  GAP BRANCH crates/store/tests/tix_latency.rs:64:12 True (test): if !weekdays || day % 7 < 5 {
  GAP BRANCH crates/store/tests/tix_latency.rs:64:12 False (test): if !weekdays || day % 7 < 5 {
  GAP BRANCH crates/store/tests/tix_latency.rs:64:25 True (test): if !weekdays || day % 7 < 5 {
  GAP BRANCH crates/store/tests/tix_latency.rs:64:25 False (test): if !weekdays || day % 7 < 5 {
  GAP BRANCH crates/store/tests/tix_latency.rs:146:30 True (test): let passes: &[bool] = if tix.exists() {
  GAP BRANCH crates/store/tests/tix_latency.rs:146:30 False (test): let passes: &[bool] = if tix.exists() {
  GAP BRANCH crates/store/tests/tix_latency.rs:152:12 True (test): if !with_index && tix.exists() {
  GAP BRANCH crates/store/tests/tix_latency.rs:152:12 False (test): if !with_index && tix.exists() {
  GAP BRANCH crates/store/tests/tix_latency.rs:152:27 True (test): if !with_index && tix.exists() {
  GAP BRANCH crates/store/tests/tix_latency.rs:152:27 False (test): if !with_index && tix.exists() {
  GAP BRANCH crates/store/tests/tix_latency.rs:155:22 True (test): let tag = if with_index { "index" } else { "bisection" };
  GAP BRANCH crates/store/tests/tix_latency.rs:155:22 False (test): let tag = if with_index { "index" } else { "bisection" };
  GAP BRANCH crates/store/tests/tix_rebuilder.rs:154:12 False (test): if let Some(last) = bars.last() {
  GAP BRANCH crates/store/tests/tix_rebuilder.rs:521:15 False (test): }) if asked == SYMBOL + 1
  F? LINE   crates/store/tests/tix_small_month.rs:160 [panic] (test): other => panic!("not committed: {other:?}"),
  F? LINE   crates/store/tests/tix_small_month.rs:463 [assert] (test): reader.time_lookup()
  GAP BRANCH crates/store/tests/tix_small_month.rs:143:12 False (test): if let Some(last) = bars.last() {
  F? BRANCH crates/store/tests/tix_small_month.rs:461:74 False [assert] (test): matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),
  GAP LINE   crates/store/tests/tix_warn_once.rs:77 (test): other => format!("{other:?}"),
  GAP LINE   crates/store/tests/tix_warn_once.rs:125 (test): Some((_, OwnedValue::Uint(n))) => *n,
  F? LINE   crates/store/tests/tix_warn_once.rs:127 [panic] (test): other => panic!("{key} is not a count: {other:?}"),
  F? LINE   crates/store/tests/tix_warn_once.rs:160 [panic] (test): other => panic!("unexpected store.tix info line {other:?}: {line:?}"),
  F? LINE   crates/store/tests/tix_warn_once.rs:162 [panic] (test): other => panic!("unexpected store.tix line at {other:?}: {line:?}"),
  F? LINE   crates/store/tests/unit.rs:110 [assert_eq] (test): input.len()
  F? LINE   crates/store/tests/unit.rs:527 [panic] (test): _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),
  F? LINE   crates/store/tests/unit.rs:1820 [assert] (test): matches!(
  F? LINE   crates/store/tests/unit.rs:1859 [assert] (test): matches!(
  F? LINE   crates/store/tests/unit.rs:1874 [assert] (test): matches!(
  F? LINE   crates/store/tests/unit.rs:2096 [assert_eq] (test): vendor.as_str(),
  F? LINE   crates/store/tests/unit.rs:2826 [assert_ne] (test): a.as_str(),
  F? LINE   crates/store/tests/unit.rs:2827 [assert_ne] (test): b.as_str()
  F? LINE   crates/store/tests/unit.rs:2833 [assert_ne] (test): a.as_str()
  F? LINE   crates/store/tests/unit.rs:2893 [assert_eq] (test): tf.as_str()
  F? BRANCH crates/store/tests/unit.rs:399:13 False [assert] (test): assert!(real.ohlc_is_sane() && real.counts_are_sane());
  F? BRANCH crates/store/tests/unit.rs:399:36 False [assert] (test): assert!(real.ohlc_is_sane() && real.counts_are_sane());
  F? BRANCH crates/store/tests/unit.rs:868:13 False [assert] (test): assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  F? BRANCH crates/store/tests/unit.rs:868:54 False [assert] (test): assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  GAP BRANCH crates/store/tests/unit.rs:908:16 False (test): if one.magic()[..7] == other.magic()[..7]
  F? BRANCH crates/store/tests/unit.rs:1475:32 False [assert] (test): read == old || read == new,
  F? BRANCH crates/store/tests/unit.rs:2047:26 False [assert] (test): .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  F? BRANCH crates/store/tests/unit.rs:2047:43 False [assert] (test): .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  F? BRANCH crates/store/tests/unit.rs:2173:13 False [assert] (test): err.to_string().contains(field) && err.to_string().contains("case"),
  F? BRANCH crates/store/tests/unit.rs:2173:48 False [assert] (test): err.to_string().contains(field) && err.to_string().contains("case"),
  F? LINE   crates/store/tests/verdict.rs:129 [panic] (test): Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),
  F? LINE   crates/store/tests/verdict.rs:168 [panic] (test): other => panic!("100x must be refused table_too_large, got {other:?}"),
  F? LINE   crates/store/tests/verdict.rs:395 [panic] (test): other => panic!("a 65-byte instrument key must be refused, got {other:?}"),
  F? LINE   crates/store/tests/verdict.rs:450 [panic] (test): panic!("{whole} must be found");
  GAP LINE   crates/store/tests/verdict.rs:588 (test): }
  F? LINE   crates/store/tests/verdict.rs:728 [panic] (test): other => panic!("expected image_missing, got {other:?}"),
  F? LINE   crates/store/tests/verdict.rs:793 [panic] (test): other => panic!("expected generation_collision, got {other:?}"),
  F? LINE   crates/store/tests/verdict.rs:1318 [panic] (test): other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),
  F? LINE   crates/store/tests/verdict.rs:1821 [panic] (test): other => panic!(
  F? LINE   crates/store/tests/verdict.rs:1823 [panic] (test): clean.len()
  F? LINE   crates/store/tests/verdict.rs:1856 [panic] (test): other => panic!("expected record_corrupt, got {other:?}"),
  F? LINE   crates/store/tests/verdict.rs:2283 [assert_eq] (test): feed.as_str(),
  F? LINE   crates/store/tests/verdict.rs:2284 [assert_eq] (test): timeframe.as_str()
  F? LINE   crates/store/tests/verdict.rs:2435 [panic] (test): other => panic!("expected lock_stalled, got {other:?}"),
  F? LINE   crates/store/tests/verdict.rs:2455 [assert] (test): matches!(
  F? LINE   crates/store/tests/verdict.rs:2549 [assert] (test): image.display()
  F? LINE   crates/store/tests/verdict.rs:2554 [panic] (test): other => panic!("expected index_missing, got {other}"),
  F? LINE   crates/store/tests/verdict.rs:2730 [panic] (test): other => panic!("{other}"),
  F? BRANCH crates/store/tests/verdict.rs:228:13 False [assert] (test): got.cost.reads <= 5 && got.cost.opens <= 2,
  F? BRANCH crates/store/tests/verdict.rs:228:36 False [assert] (test): got.cost.reads <= 5 && got.cost.opens <= 2,
  GAP BRANCH crates/store/tests/verdict.rs:449:13 False (test): let Found::Record { ordinal, record } = &answer.found else {
  GAP BRANCH crates/store/tests/verdict.rs:584:12 False (test): if count > 2 {
  F? BRANCH crates/store/tests/verdict.rs:1890:17 False [assert] (test): assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  F? BRANCH crates/store/tests/verdict.rs:1890:40 False [assert] (test): assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  F? BRANCH crates/store/tests/verdict.rs:2123:17 False [assert] (test): assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  F? BRANCH crates/store/tests/verdict.rs:2123:40 False [assert] (test): assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  F? BRANCH crates/store/tests/verdict.rs:2358:58 False [assert] (test): matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),
  F? BRANCH crates/store/tests/verdict.rs:2545:21 False [assert] (test): image.extension().is_some_and(|ext| ext == "vdt")
  F? BRANCH crates/store/tests/verdict.rs:2546:28 False [assert] (test): && image.parent().is_some_and(|dir| dir.ends_with("2031"))
  F? BRANCH crates/store/tests/verdict.rs:2547:28 False [assert] (test): && image.exists(),
  F? BRANCH crates/store/tests/verdict.rs:2604:13 False [assert] (test): assert!(named && (1..=2).contains(&seen), "seen {seen}");
  F? BRANCH crates/store/tests/verdict.rs:2604:22 False [assert] (test): assert!(named && (1..=2).contains(&seen), "seen {seen}");
  F? BRANCH crates/store/tests/verdict.rs:2782:9 False [assert] (test): said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  F? BRANCH crates/store/tests/verdict.rs:2782:61 False [assert] (test): said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  F? LINE   crates/store/tests/write.rs:1402 [assert] (test): sidecar.display()
  F? LINE   crates/store/tests/write.rs:1419 [assert] (test): sidecar.display()
  F? LINE   crates/store/tests/write.rs:1619 [panic] (test): panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")
  F? LINE   crates/store/tests/write.rs:1663 [panic] (test): panic!(
  F? BRANCH crates/store/tests/write.rs:1564:13 False [assert] (test): text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  F? BRANCH crates/store/tests/write.rs:1564:62 False [assert] (test): text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  F? BRANCH crates/store/tests/write.rs:2103:85 False [assert] (test): Err(StoreError::Format { path, source: FormatError::NoValidHeader }) if *path == bars
`````

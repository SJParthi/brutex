# gaps-LS1.txt.md

BASE-COV-LIBS-LS1 (crate `store`): Route B figures and every uncovered line and branch side, written by the session from its own measurement (continuation of 5 Nov 2026). One fenced block.

`````text
FINAL: Route B (the kit's d3-route-b-cloud.sh, sha256 f032b525..., unchanged; d3-covset.mjs ecef3bdc...) on tip 4b3ee558a38fbc65f6c0dfcf59893cff750568c0 (the code tip of the 5 Nov continuation), set 'reading whole:store' (117 .rs files: the 113 of 3 Nov and 4 new test files), pinned rustc 1.99.0-nightly (1a98b1e13 2026-08-07) installed as nightly-2026-08-08, LLVM 23.1.0-rust-1.99.0-nightly, Linux x86_64 root host (4 cores), run under umask 000 so the profile folder takes the nobody re-runs' profiles (D-0995) and the fresh-process children's. Run 2026-11-05 08:47:41-09:05:56 IST (instrumented build 08:47:41-08:47:57 IST, 45 artifacts as its log reports them; tests 08:47:57-09:04:52 IST, 17 min: 1,064 passed, 0 failed, 8 ignored; 82 profiles merged). BEFORE (3 Nov final, tip fc9b489f0e3c): lines 51,094/52,629 (1,535 uncovered), branches 3,351/3,916 (565 uncovered). PROGRESS (this session, tip 38ee3db3d2fc): lines 51,474/52,972 (1,498 uncovered), branches 3,364/3,924 (560 uncovered).
Classes: (U) = a production row with its proof in fu-LS1.txt.md; HOST / RACE = a production row only a host refusal or a race reaches (fu-LS1.txt.md says which call and what is needed); GAP = a production row with neither; F? = the failing arm of an assert/panic/unreachable in test code by pattern (an (F) candidate of recipe step 4); IGNORED = a body of an #[ignore]d test or a helper only such tests call; GAP (test) = other uncovered test code.

ROUTE B LOG (result lines)
  ROUTE-B START 2026-11-05 08:47:41 IST tree /home/user/ls1-deep tip 4b3ee558a38fbc65f6c0dfcf59893cff750568c0 packages store K /home/user/brutex/target P <runs>/P-4b3ee558 (entries before: 0)
  versions: rustc 1.99.0-nightly (1a98b1e13 2026-08-07) | llvm-cov  LLVM version 23.1.0-rust-1.99.0-nightly | llvm-profdata  LLVM version 23.1.0-rust-1.99.0-nightly
  == build START 2026-11-05 08:47:41 IST: cargo test -p store --locked --no-run --message-format=json (RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch', opt-level 0, CARGO_TARGET_DIR=/home/user/brutex/target/cov)
  == build rc=0 END 2026-11-05 08:47:57 IST (0 min); 45 artifacts, 0 reused
  objects: 42 (<runs>/P-4b3ee558/objects.txt, from this build's own report)
  == test store START 2026-11-05 08:47:57 IST
  == test store rc=0 END 2026-11-05 09:04:52 IST (16 min); tests passed 1064 failed 0 ignored 8
  profraw files: 82
  merged <runs>/P-4b3ee558/store/merged.profdata
  ROUTE-B NOT PROVED: crates/store/benches/ratio.rs: not read (<runs>/L-4b3ee558/show-store-crates_store_benches_ratio.rs.err)
  ROUTE-B NOT PROVED: crates/store/benches/ratio/months.rs: not read (<runs>/L-4b3ee558/show-store-crates_store_benches_ratio_months.rs.err)
  ROUTE-B NOT PROVED: crates/store/benches/ratio/quarantine.rs: not read (<runs>/L-4b3ee558/show-store-crates_store_benches_ratio_quarantine.rs.err)
  ROUTE-B NOT PROVED: crates/store/benches/ratio/safe_files.rs: not read (<runs>/L-4b3ee558/show-store-crates_store_benches_ratio_safe_files.rs.err)
  ROUTE-B NOT PROVED: crates/store/src/lib.rs: not read (<runs>/L-4b3ee558/show-store-crates_store_src_lib.rs.err)
  ROUTE-B DONE 2026-11-05 09:05:56 IST WITH NOT PROVED lines above
  EXIT 3

TOTALS AND PER FILE (lines covered/counted, branches covered/counted; uncovered rows by class)
  TOTAL store: lines 52295/53771 (uncovered 1476), branches 3405/3964 (uncovered 559), files 112
  rows: 2035 (production 142: (U) 81, HOST/RACE 61, GAP 0; ignored 1108; F? 641; test GAP 144)
  crates/store/src/block.rs lines 115/115 branches 10/10
  crates/store/src/catalog.rs lines 189/189 branches 50/50
  crates/store/src/catalog_cover_tests.rs lines 49/49 branches 0/0
  crates/store/src/catalog_tests.rs lines 338/341 branches 9/18  F? 12
  crates/store/src/checksum_audit.rs lines 235/235 branches 29/30  production 1
  crates/store/src/checksum_audit_cover_tests.rs lines 59/59 branches 0/0
  crates/store/src/checksum_audit_tests.rs lines 400/403 branches 11/12  test GAP 4
  crates/store/src/crc.rs lines 90/90 branches 8/8
  crates/store/src/emits.rs lines 300/306 branches 3/4  F? 6, test GAP 1
  crates/store/src/file.rs lines 4609/4662 branches 371/382  production 16, ignored 24, F? 20, test GAP 4
  crates/store/src/file_cover_tests.rs lines 64/64 branches 6/10  F? 4
  crates/store/src/file_fault_tests.rs lines 571/575 branches 25/40  F? 19
  crates/store/src/file_index_cover_tests.rs lines 421/423 branches 8/10  F? 4
  crates/store/src/file_sealed.rs lines 255/271 branches 48/58  production 26
  crates/store/src/file_sealed_cover_tests.rs lines 105/105 branches 12/24  F? 12
  crates/store/src/file_sealed_tests.rs lines 584/617 branches 25/36  ignored 26, F? 18
  crates/store/src/file_write_door_tests.rs lines 726/744 branches 24/30  F? 22, test GAP 2
  crates/store/src/flock.rs lines 449/454 branches 45/48  production 8
  crates/store/src/flock_cover_tests.rs lines 171/175 branches 9/16  F? 7, test GAP 4
  crates/store/src/flock_patient_tests.rs lines 495/506 branches 23/36  F? 19, test GAP 5
  crates/store/src/flock_tests.rs lines 563/570 branches 18/26  F? 14, test GAP 1
  crates/store/src/format.rs lines 278/278 branches 42/42
  crates/store/src/generation.rs lines 324/325 branches 60/60  production 1
  crates/store/src/generation_cover_tests.rs lines 118/118 branches 3/6  F? 3
  crates/store/src/generation_door_cover_tests.rs lines 139/145 branches 2/4  F? 8
  crates/store/src/generation_tests.rs lines 37/37 branches 0/0
  crates/store/src/header.rs lines 524/524 branches 76/76
  crates/store/src/header_audit_tests.rs lines 49/49 branches 0/0
  crates/store/src/layout.rs lines 186/186 branches 40/40
  crates/store/src/month_summary.rs lines 682/683 branches 191/194  production 4
  crates/store/src/month_summary_cover_tests.rs lines 147/147 branches 0/0
  crates/store/src/month_summary_decode_cover_tests.rs lines 106/107 branches 0/0  F? 1
  crates/store/src/month_summary_tests.rs lines 559/562 branches 32/52  F? 22, test GAP 1
  crates/store/src/month_summary_writer.rs lines 165/165 branches 34/34
  crates/store/src/month_summary_writer_cover_tests.rs lines 175/175 branches 2/4  F? 2
  crates/store/src/open_flags.rs lines 6/6 branches 0/0
  crates/store/src/path.rs lines 206/206 branches 30/30
  crates/store/src/provenance.rs lines 764/765 branches 150/152  production 3
  crates/store/src/provenance_cover_tests.rs lines 237/246 branches 2/4  F? 11
  crates/store/src/quarantine.rs lines 1467/1483 branches 186/188  production 18
  crates/store/src/quarantine_cover_tests.rs lines 516/520 branches 1/2  F? 5
  crates/store/src/quarantine_tests.rs lines 1735/1746 branches 54/90  F? 47
  crates/store/src/repair.rs lines 730/741 branches 130/132  production 13
  crates/store/src/repair_cover_tests.rs lines 67/67 branches 3/4  F? 1
  crates/store/src/repair_door_cover_tests.rs lines 286/292 branches 6/8  F? 8
  crates/store/src/repair_tests.rs lines 639/643 branches 15/18  F? 5, test GAP 2
  crates/store/src/safe_open.rs lines 169/177 branches 20/22  production 10
  crates/store/src/safe_open_tests.rs lines 283/298 branches 9/18  F? 17, test GAP 7
  crates/store/src/sealed.rs lines 1434/1434 branches 271/274  production 3
  crates/store/src/sealed_codec.rs lines 646/646 branches 90/90
  crates/store/src/sealed_codec_tests.rs lines 996/1000 branches 30/34  F? 8
  crates/store/src/sealed_fold_cover_tests.rs lines 28/28 branches 0/0
  crates/store/src/sealed_tests.rs lines 1719/1740 branches 34/40  F? 22, test GAP 5
  crates/store/src/settings.rs lines 724/743 branches 144/150  production 25
  crates/store/src/settings_cover_tests.rs lines 187/187 branches 1/2  F? 1
  crates/store/src/settings_door_cover_tests.rs lines 101/102 branches 4/8  F? 5
  crates/store/src/settings_tests.rs lines 832/844 branches 16/28  F? 14, test GAP 10
  crates/store/src/sidecar_checksum_tests.rs lines 391/391 branches 10/10
  crates/store/src/small_month_tests.rs lines 106/106 branches 1/2  F? 1
  crates/store/src/sort_index.rs lines 794/794 branches 126/126
  crates/store/src/sort_index_cover_tests.rs lines 230/230 branches 5/10  F? 5
  crates/store/src/sort_index_tests.rs lines 385/385 branches 9/10  F? 1
  crates/store/src/sort_index_writer.rs lines 388/388 branches 62/62
  crates/store/src/sort_index_writer_tests.rs lines 114/114 branches 0/0
  crates/store/src/staging.rs lines 109/109 branches 14/14
  crates/store/src/staging_cover_tests.rs lines 26/28 branches 0/0  F? 2
  crates/store/src/staging_tests.rs lines 222/223 branches 8/16  F? 9
  crates/store/src/time_index.rs lines 744/744 branches 71/80  F? 9
  crates/store/src/verdict.rs lines 2679/2685 branches 296/310  production 14, F? 6
  crates/store/src/verdict_tests.rs lines 594/601 branches 24/26  F? 8, test GAP 1
  crates/store/tests/admission.rs lines 182/182 branches 0/0
  crates/store/tests/base_cov_words.rs lines 46/46 branches 0/0
  crates/store/tests/bisect_cost.rs lines 80/80 branches 0/0
  crates/store/tests/catalog.rs lines 549/568 branches 5/12  F? 11, test GAP 15
  crates/store/tests/cited_commits.rs lines 508/532 branches 72/90  F? 7, test GAP 35
  crates/store/tests/cloud_mutants_store.rs lines 2190/2203 branches 62/96  F? 35, test GAP 12
  crates/store/tests/cold_read.rs lines 199/199 branches 4/4
  crates/store/tests/docs.rs lines 229/242 branches 5/14  F? 14, test GAP 8
  crates/store/tests/durability.rs lines 194/195 branches 0/0  F? 1
  crates/store/tests/fault.rs lines 471/472 branches 10/12  F? 3
  crates/store/tests/fifo.rs lines 194/201 branches 7/12  F? 6, test GAP 6
  crates/store/tests/generation.rs lines 483/492 branches 8/18  F? 19
  crates/store/tests/geometry.rs lines 433/433 branches 9/12  F? 3
  crates/store/tests/libm_key.rs lines 778/780 branches 8/12  F? 5, test GAP 1
  crates/store/tests/misfiled.rs lines 205/207 branches 9/14  F? 5, test GAP 2
  crates/store/tests/month_repair.rs lines 605/613 branches 13/18  F? 10, test GAP 3
  crates/store/tests/month_summary_log.rs lines 94/96 branches 0/0  test GAP 2
  crates/store/tests/open_flags.rs lines 97/98 branches 17/18  test GAP 2
  crates/store/tests/provenance.rs lines 1222/1238 branches 0/0  F? 16
  crates/store/tests/repair.rs lines 384/384 branches 4/6  F? 2
  crates/store/tests/roundtrip.rs lines 111/111 branches 0/0
  crates/store/tests/seal_buffer.rs lines 100/100 branches 0/0
  crates/store/tests/sealed_codec_alloc.rs lines 230/232 branches 15/28  F? 15
  crates/store/tests/sealed_codec_samples.rs lines 0/366 branches 0/44  ignored 410
  crates/store/tests/sealed_door.rs lines 126/211 branches 6/20  ignored 95, F? 4
  crates/store/tests/sealed_samples.rs lines 0/174 branches 0/12  ignored 186
  crates/store/tests/sealed_write_door.rs lines 0/165 branches 0/24  ignored 189
  crates/store/tests/slot_audit.rs lines 348/354 branches 8/10  F? 8
  crates/store/tests/slot_audit_sealed.rs lines 88/96 branches 0/2  F? 9, test GAP 1
  crates/store/tests/slot_damaged_log.rs lines 137/145 branches 0/0  F? 7, test GAP 1
  crates/store/tests/sort_index.rs lines 599/602 branches 14/22  F? 11
  crates/store/tests/sort_index_log.rs lines 128/130 branches 0/0  test GAP 2
  crates/store/tests/support/mod.rs lines 32/32 branches 3/4  F? 1
  crates/store/tests/tail_proof.rs lines 175/175 branches 3/6  F? 3
  crates/store/tests/tix.rs lines 406/407 branches 10/12  F? 2, test GAP 1
  crates/store/tests/tix_latency.rs lines 0/164 branches 0/14  ignored 178
  crates/store/tests/tix_rebuilder.rs lines 336/336 branches 6/8  F? 1, test GAP 1
  crates/store/tests/tix_small_month.rs lines 394/396 branches 6/8  F? 3, test GAP 1
  crates/store/tests/tix_warn_once.rs lines 212/217 branches 0/0  F? 3, test GAP 2
  crates/store/tests/unit.rs lines 1711/1721 branches 30/40  F? 19, test GAP 1
  crates/store/tests/verdict.rs lines 1989/2007 branches 28/44  F? 33, test GAP 1
  crates/store/tests/write.rs lines 1199/1203 branches 5/8  F? 7

EVERY UNCOVERED ROW (by file and line)
  F?         BRANCH crates/store/src/catalog_tests.rs:267:9 False: let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {
  F?         LINE   crates/store/src/catalog_tests.rs:268: panic!("a dangling `bars` link is refused");
  F?         BRANCH crates/store/src/catalog_tests.rs:281:9 False: let Err(CatalogError::BarsUnreadable { because }) = locked else {
  F?         LINE   crates/store/src/catalog_tests.rs:282: panic!("an unsearchable root is refused, not empty: {locked:?}");
  F?         BRANCH crates/store/src/catalog_tests.rs:314:13 False: assert!(stack.is_empty() && out.held.is_empty());
  F?         BRANCH crates/store/src/catalog_tests.rs:314:33 False: assert!(stack.is_empty() && out.held.is_empty());
  F?         BRANCH crates/store/src/catalog_tests.rs:320:13 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  F?         BRANCH crates/store/src/catalog_tests.rs:320:32 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  F?         BRANCH crates/store/src/catalog_tests.rs:320:50 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  F?         BRANCH crates/store/src/catalog_tests.rs:336:13 False: assert!(stack.is_empty() && out.held.is_empty());
  F?         BRANCH crates/store/src/catalog_tests.rs:336:33 False: assert!(stack.is_empty() && out.held.is_empty());
  F?         LINE   crates/store/src/catalog_tests.rs:460: path.display()
  (U)        BRANCH crates/store/src/checksum_audit.rs:288:12 True: || layout.header_len() != HEADER_BYTES as u64
  GAP (test) BRANCH crates/store/src/checksum_audit_tests.rs:152:16 True: if started.elapsed() > std::time::Duration::from_secs(2) {
  GAP (test) LINE   crates/store/src/checksum_audit_tests.rs:153: child.kill().expect("bounded timeout");
  GAP (test) LINE   crates/store/src/checksum_audit_tests.rs:154: child.wait().expect("reap child");
  GAP (test) LINE   crates/store/src/checksum_audit_tests.rs:155: return Err("strict checksum source open waited for a FIFO peer".into());
  GAP (test) BRANCH crates/store/src/emits.rs:311:12 False: if let Some(byte) = region.get_mut(16_384 + 56) {
  F?         LINE   crates/store/src/emits.rs:835: found
  F?         LINE   crates/store/src/emits.rs:836: .records
  F?         LINE   crates/store/src/emits.rs:837: .iter()
  F?         LINE   crates/store/src/emits.rs:838: .map(|line| (line.target.clone(), line.message.clone()))
  F?         LINE   crates/store/src/emits.rs:839: .collect::<Vec<_>>()
  F?         LINE   crates/store/src/emits.rs:848: panic!(
  (U)        LINE   crates/store/src/file.rs:2891: )?;
  (U)        LINE   crates/store/src/file.rs:3172: return Err(StoreError::Format {
  (U)        LINE   crates/store/src/file.rs:3173: path: self.bars_path.clone(),
  (U)        LINE   crates/store/src/file.rs:3174: source,
  (U)        LINE   crates/store/src/file.rs:3175: });
  (U)        BRANCH crates/store/src/file.rs:3855:21 False: let Some(tix) = self.tix.as_ref() else {
  (U)        LINE   crates/store/src/file.rs:3856: return Ok(None);
  (U)        BRANCH crates/store/src/file.rs:3869:13 False: let Some(tix) = self.tix.as_ref() else {
  (U)        LINE   crates/store/src/file.rs:3870: return Ok(None);
  (U)        LINE   crates/store/src/file.rs:3891: Err(other) => Err(index_refused(&tix.path, &other)),
  (U)        LINE   crates/store/src/file.rs:4013: Err(other) => return Err(index_refused(&tix.path, &other)),
  (U)        BRANCH crates/store/src/file.rs:4189:36 True: if cache.block != block || cache.n_valid != n_valid {
  (U)        LINE   crates/store/src/file.rs:4316: path: self.bars_path.clone(),
  (U)        LINE   crates/store/src/file.rs:4317: source: FormatError::OffsetOverflow,
  (U)        LINE   crates/store/src/file.rs:4318: })?;
  (U)        BRANCH crates/store/src/file.rs:5666:42 True: BatchFlush::OncePerDevice if O_SYNC.is_none() => return Ok(false),
  F?         LINE   crates/store/src/file.rs:6732: parent.display()
  F?         BRANCH crates/store/src/file.rs:6879:82 False: matches!(&refused, Err(StoreError::Denied { action: at, .. }) if *at == action),
  F?         LINE   crates/store/src/file.rs:6936: matches!(
  F?         LINE   crates/store/src/file.rs:6956: level.display()
  F?         LINE   crates/store/src/file.rs:7215: matches!(
  F?         LINE   crates/store/src/file.rs:7269: other => panic!("the flush was refused by path: {other:?}"),
  F?         LINE   crates/store/src/file.rs:7589: matches!(
  IGNORED    LINE   crates/store/src/file.rs:7835: fn the_kernels_own_enospc_on_a_full_volume_is_disk_full_by_name() {
  IGNORED    LINE   crates/store/src/file.rs:7836: let dir = std::env::var_os("BRUTEX_FULL_VOLUME")
  IGNORED    LINE   crates/store/src/file.rs:7837: .map(PathBuf::from)
  IGNORED    LINE   crates/store/src/file.rs:7838: .expect("BRUTEX_FULL_VOLUME names a directory on a small volume");
  IGNORED    LINE   crates/store/src/file.rs:7839: let at = dir.join("brutex-enospc-probe.bin");
  IGNORED    LINE   crates/store/src/file.rs:7840: let file = std::fs::OpenOptions::new()
  IGNORED    LINE   crates/store/src/file.rs:7841: .write(true)
  IGNORED    LINE   crates/store/src/file.rs:7842: .create_new(true)
  IGNORED    LINE   crates/store/src/file.rs:7843: .open(&at)
  IGNORED    LINE   crates/store/src/file.rs:7844: .expect("a new file on the small volume");
  IGNORED    LINE   crates/store/src/file.rs:7845: let chunk = vec![0xAB_u8; 1 << 16];
  IGNORED    LINE   crates/store/src/file.rs:7846: let mut offset = 0_u64;
  IGNORED    LINE   crates/store/src/file.rs:7847: let refused = loop {
  IGNORED    LINE   crates/store/src/file.rs:7848: assert!(offset < 1 << 30, "the volume took 1 GiB without refusing");
  IGNORED    LINE   crates/store/src/file.rs:7849: match write_fully(&file, &at, offset, &chunk) {
  IGNORED    LINE   crates/store/src/file.rs:7850: Ok(()) => offset = offset.saturating_add(1 << 16),
  IGNORED    LINE   crates/store/src/file.rs:7851: Err(refused) => break refused,
  IGNORED    LINE   crates/store/src/file.rs:7854: assert_eq!(
  IGNORED    LINE   crates/store/src/file.rs:7856: StoreError::DiskFull {
  IGNORED    LINE   crates/store/src/file.rs:7857: path: at.clone(),
  IGNORED    LINE   crates/store/src/file.rs:7858: action: Action::Write,
  IGNORED    LINE   crates/store/src/file.rs:7859: },
  IGNORED    LINE   crates/store/src/file.rs:7863: eprintln!("ENOSPC: DiskFull by name after {offset} bytes accepted");
  IGNORED    LINE   crates/store/src/file.rs:7864: }
  F?         LINE   crates/store/src/file.rs:8353: reads.get()
  GAP (test) BRANCH crates/store/src/file.rs:8675:16 True: if &slot[..7] != b"BRUTEXB" {
  GAP (test) LINE   crates/store/src/file.rs:8676: continue;
  F?         LINE   crates/store/src/file.rs:8692: other => panic!("a version-3 month without its flag must be refused, got {other:?}"),
  F?         LINE   crates/store/src/file.rs:8910: matches!(
  F?         LINE   crates/store/src/file.rs:8919: matches!(
  F?         LINE   crates/store/src/file.rs:8929: matches!(
  F?         LINE   crates/store/src/file.rs:9031: matches!(
  F?         LINE   crates/store/src/file.rs:9074: matches!(
  GAP (test) BRANCH crates/store/src/file.rs:9091:16 False: if let Some(head) = cache.bytes.get_mut(..8) {
  F?         BRANCH crates/store/src/file.rs:9103:13 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  F?         BRANCH crates/store/src/file.rs:9103:45 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  F?         BRANCH crates/store/src/file.rs:9103:73 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  F?         LINE   crates/store/src/file.rs:9280: matches!(
  F?         LINE   crates/store/src/file.rs:10225: matches!(
  F?         LINE   crates/store/src/file.rs:10351: matches!(
  GAP (test) BRANCH crates/store/src/file.rs:10727:24 False: if *ts_micros == bar(10).ts_micros
  F?         BRANCH crates/store/src/file_cover_tests.rs:91:9 False: follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
  F?         BRANCH crates/store/src/file_cover_tests.rs:91:65 False: follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
  F?         BRANCH crates/store/src/file_cover_tests.rs:118:9 False: whole.starts_with(&last.took) && last.took.len() < whole.len(),
  F?         BRANCH crates/store/src/file_cover_tests.rs:118:42 False: whole.starts_with(&last.took) && last.took.len() < whole.len(),
  F?         BRANCH crates/store/src/file_fault_tests.rs:315:57 False: Err(StoreError::Denied { path, action }) if fired => {
  F?         LINE   crates/store/src/file_fault_tests.rs:320: other => panic!("read {skip}: {other:?}"),
  F?         BRANCH crates/store/src/file_fault_tests.rs:334:9 False: routed >= 1 && refused >= 1,
  F?         BRANCH crates/store/src/file_fault_tests.rs:334:24 False: routed >= 1 && refused >= 1,
  F?         BRANCH crates/store/src/file_fault_tests.rs:650:21 False: reason.starts_with(&format!("{}: ", srt.display()))
  F?         BRANCH crates/store/src/file_fault_tests.rs:651:28 False: && reason.contains("refused by the file test seam"),
  F?         LINE   crates/store/src/file_fault_tests.rs:657: Err(other) => panic!("read {skip}: {other:?}"),
  F?         BRANCH crates/store/src/file_fault_tests.rs:661:9 False: built >= 1 && refused >= 1,
  F?         BRANCH crates/store/src/file_fault_tests.rs:661:23 False: built >= 1 && refused >= 1,
  F?         LINE   crates/store/src/file_fault_tests.rs:696: other => panic!("{}: {other:?}", looked.display()),
  F?         BRANCH crates/store/src/file_fault_tests.rs:755:71 False: matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),
  F?         BRANCH crates/store/src/file_fault_tests.rs:845:61 False: Err(StoreError::Denied { path, action }) if fired => {
  F?         LINE   crates/store/src/file_fault_tests.rs:869: other => panic!("{kind:?} measure {skip}: {other:?}"),
  F?         BRANCH crates/store/src/file_fault_tests.rs:879:26 False: _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  F?         BRANCH crates/store/src/file_fault_tests.rs:879:42 False: _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  F?         BRANCH crates/store/src/file_fault_tests.rs:912:13 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F?         BRANCH crates/store/src/file_fault_tests.rs:912:54 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F?         BRANCH crates/store/src/file_fault_tests.rs:949:13 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F?         BRANCH crates/store/src/file_fault_tests.rs:949:54 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F?         LINE   crates/store/src/file_index_cover_tests.rs:158: matches!(
  F?         BRANCH crates/store/src/file_index_cover_tests.rs:574:9 False: output.status.success() && stdout.contains("1 passed"),
  F?         BRANCH crates/store/src/file_index_cover_tests.rs:574:36 False: output.status.success() && stdout.contains("1 passed"),
  F?         LINE   crates/store/src/file_index_cover_tests.rs:639: other => panic!("the leaf is refused by the last step: {other:?}"),
  (U)        BRANCH crates/store/src/file_sealed.rs:135:12 True: if usize::from(head.block_rows).saturating_mul(RECORD_LEN) > CACHED_BLOCK_LEN {
  (U)        LINE   crates/store/src/file_sealed.rs:136: return Err(StoreError::SealedRefused {
  (U)        LINE   crates/store/src/file_sealed.rs:137: path: at,
  (U)        LINE   crates/store/src/file_sealed.rs:138: reason: "block_rows",
  (U)        LINE   crates/store/src/file_sealed.rs:139: detail: format!(
  (U)        LINE   crates/store/src/file_sealed.rs:140: "the head declares {} rows a frame and a handle caches at most {}",
  (U)        LINE   crates/store/src/file_sealed.rs:141: head.block_rows,
  (U)        LINE   crates/store/src/file_sealed.rs:142: CACHED_BLOCK_LEN / RECORD_LEN
  (U)        LINE   crates/store/src/file_sealed.rs:143: ),
  (U)        LINE   crates/store/src/file_sealed.rs:144: });
  (U)        BRANCH crates/store/src/file_sealed.rs:202:36 True: if cache.block != block || cache.n_valid != n_valid {
  (U)        LINE   crates/store/src/file_sealed.rs:231: )?;
  (U)        LINE   crates/store/src/file_sealed.rs:236: path: self.bars_path.clone(),
  (U)        LINE   crates/store/src/file_sealed.rs:237: source: FormatError::OffsetOverflow,
  (U)        LINE   crates/store/src/file_sealed.rs:238: })?;
  (U)        BRANCH crates/store/src/file_sealed.rs:341:20 False: if let Some(count) = first.get_mut(at + 1) {
  (U)        BRANCH crates/store/src/file_sealed.rs:347:20 False: if let Some(start) = first.get_mut(at) {
  (U)        BRANCH crates/store/src/file_sealed.rs:354:20 False: if let Some(slot) = next_slot.get_mut(at) {
  (U)        BRANCH crates/store/src/file_sealed.rs:355:24 False: if let Some(place) = placed.get_mut(*slot) {
  (U)        LINE   crates/store/src/file_sealed.rs:359: }
  (U)        BRANCH crates/store/src/file_sealed.rs:371:21 False: let Some(&row) = rows.get(asked) else {
  (U)        LINE   crates/store/src/file_sealed.rs:372: continue;
  (U)        BRANCH crates/store/src/file_sealed.rs:379:24 False: if let Some(slot) = out.get_mut(asked) {
  (U)        BRANCH crates/store/src/file_sealed.rs:417:9 False: let Some(sealed) = file.sealed.as_ref() else {
  (U)        LINE   crates/store/src/file_sealed.rs:418: return Ok(None);
  RACE       BRANCH crates/store/src/file_sealed.rs:425:21 False: Err(why) if why.kind() == ErrorKind::NotFound => Err(format!(
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:67:9 False: fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:67:51 False: fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:74:9 False: twice.contains("rows 900 against 900")
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:75:16 False: && twice.contains("generation 2 against 1")
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:76:16 False: && twice.contains("records BLAKE3 equal: true"),
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:90:9 False: shifted.contains("rows 900 against 900")
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:91:16 False: && shifted.contains("generation 1 against 1")
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:92:16 False: && shifted.contains("records BLAKE3 equal: false"),
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:100:9 False: late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:100:50 False: late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:166:13 False: refused.contains("other.sbar") && refused.contains("disagree"),
  F?         BRANCH crates/store/src/file_sealed_cover_tests.rs:166:47 False: refused.contains("other.sbar") && refused.contains("disagree"),
  F?         BRANCH crates/store/src/file_sealed_tests.rs:214:69 False: Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  F?         BRANCH crates/store/src/file_sealed_tests.rs:214:83 False: Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  F?         BRANCH crates/store/src/file_sealed_tests.rs:315:13 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  F?         BRANCH crates/store/src/file_sealed_tests.rs:315:30 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  F?         BRANCH crates/store/src/file_sealed_tests.rs:315:50 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  F?         LINE   crates/store/src/file_sealed_tests.rs:321: other => panic!("expected Missing naming the .bin, got {other:?}"),
  F?         BRANCH crates/store/src/file_sealed_tests.rs:407:17 False: !reason.is_empty() && !detail.is_empty(),
  F?         BRANCH crates/store/src/file_sealed_tests.rs:407:39 False: !reason.is_empty() && !detail.is_empty(),
  F?         LINE   crates/store/src/file_sealed_tests.rs:419: other => panic!("a damaged frame must be refused by name, got {other:?}"),
  F?         LINE   crates/store/src/file_sealed_tests.rs:439: other => panic!("a damaged head must be refused, got {other:?}"),
  F?         LINE   crates/store/src/file_sealed_tests.rs:798: matches!(
  F?         LINE   crates/store/src/file_sealed_tests.rs:803: crate::provenance::read(&root.0, path)
  F?         BRANCH crates/store/src/file_sealed_tests.rs:824:9 False: refused.starts_with("Err(") && refused.contains("sbar"),
  F?         BRANCH crates/store/src/file_sealed_tests.rs:824:40 False: refused.starts_with("Err(") && refused.contains("sbar"),
  F?         BRANCH crates/store/src/file_sealed_tests.rs:874:73 False: Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  F?         BRANCH crates/store/src/file_sealed_tests.rs:874:92 False: Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  F?         LINE   crates/store/src/file_sealed_tests.rs:885: page.len()
  F?         LINE   crates/store/src/file_sealed_tests.rs:898: page.len()
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:913: fn read_rows_at_timing() {
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:914: let _sink_is_mine = crate::emits::hold_the_sink();
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:915: for n in [2_250_u64, 22_500, 225_000] {
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:916: let root = Root::new(&format!("rows-at-timing-{n}"));
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:917: let path = month_path(None);
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:918: let bars = seconds(n);
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:919: write_raw(&root.0, path, &bars);
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:920: seal(&root.0, path);
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:921: let sealed = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:922: for width in [100_u64, 1_000] {
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:923: let page: Vec<u64> = (0..width).map(|k| (k * 7_919 + 13) % n).collect();
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:924: let _warm = sealed.read_rows_at(&page).unwrap();
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:925: let mut took = Vec::with_capacity(2_000);
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:926: for _ in 0..2_000 {
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:927: let began = std::time::Instant::now();
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:928: let got = sealed.read_rows_at(&page).unwrap();
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:929: took.push(began.elapsed());
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:930: assert_eq!(got.len(), page.len());
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:932: took.sort_unstable();
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:933: let k = took.len();
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:934: println!(
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:936: took[k / 2],
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:937: took[k * 99 / 100],
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:938: took[k - 1],
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:939: took[k / 2] / u32::try_from(width).unwrap()
  IGNORED    LINE   crates/store/src/file_sealed_tests.rs:943: }
  GAP (test) BRANCH crates/store/src/file_write_door_tests.rs:67:13 False: let Ok(entries) = fs::read_dir(&at) else {
  GAP (test) LINE   crates/store/src/file_write_door_tests.rs:68: continue;
  F?         BRANCH crates/store/src/file_write_door_tests.rs:156:13 False: let StoreError::Sealed {
  F?         LINE   crates/store/src/file_write_door_tests.rs:165: panic!("a new bar at a sealed month is `Sealed`, got {refused}");
  F?         LINE   crates/store/src/file_write_door_tests.rs:322: _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),
  F?         LINE   crates/store/src/file_write_door_tests.rs:439: _ => panic!("{case} sealed={sealed}: raw {raw:?}, offered {got:?}"),
  F?         LINE   crates/store/src/file_write_door_tests.rs:539: other => panic!("{other}"),
  F?         LINE   crates/store/src/file_write_door_tests.rs:595: other => panic!("{n}: {other}"),
  F?         LINE   crates/store/src/file_write_door_tests.rs:605: other => panic!("{n}: {other}"),
  F?         LINE   crates/store/src/file_write_door_tests.rs:721: file.display()
  F?         LINE   crates/store/src/file_write_door_tests.rs:749: other => panic!("{other}"),
  F?         LINE   crates/store/src/file_write_door_tests.rs:765: other => panic!("{other}"),
  F?         LINE   crates/store/src/file_write_door_tests.rs:809: Err(other) => panic!("{other}"),
  F?         LINE   crates/store/src/file_write_door_tests.rs:844: matches!(
  F?         BRANCH crates/store/src/file_write_door_tests.rs:852:13 False: let MonthRepair::Repaired {
  F?         LINE   crates/store/src/file_write_door_tests.rs:859: panic!("{case}: {repaired:?}");
  F?         LINE   crates/store/src/file_write_door_tests.rs:902: matches!(
  F?         BRANCH crates/store/src/file_write_door_tests.rs:942:9 False: let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  F?         LINE   crates/store/src/file_write_door_tests.rs:943: panic!("{repaired:?}");
  F?         LINE   crates/store/src/file_write_door_tests.rs:983: matches!(
  F?         BRANCH crates/store/src/file_write_door_tests.rs:1026:9 False: let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  F?         LINE   crates/store/src/file_write_door_tests.rs:1027: panic!("{case}: {repaired:?}");
  F?         BRANCH crates/store/src/file_write_door_tests.rs:1060:9 False: let MonthRepair::Repaired {
  F?         LINE   crates/store/src/file_write_door_tests.rs:1067: panic!("{case}: {repaired:?}");
  (U)        LINE   crates/store/src/flock.rs:335: || "an unnamed program".to_owned(),
  HOST       LINE   crates/store/src/flock.rs:557: |_| std::fs::FileTimes::new(),
  (U)        BRANCH crates/store/src/flock.rs:560:24 False: if let Ok(modified) = meta.modified() {
  (U)        BRANCH crates/store/src/flock.rs:563:24 False: if let Ok(accessed) = meta.accessed() {
  HOST       LINE   crates/store/src/flock.rs:627: Impatient::Host(why) => TryLockError::Error(why),
  HOST       BRANCH crates/store/src/flock.rs:681:20 True: if let Err(why) = file.set_len(0).and_then(|()| file.set_times(times)) {
  HOST       LINE   crates/store/src/flock.rs:682: note_unnoted(self.path.as_ref(), &why);
  HOST       LINE   crates/store/src/flock.rs:974: host @ Impatient::Host(_) => host,
  F?         BRANCH crates/store/src/flock_cover_tests.rs:68:9 False: output.status.success() && stdout.contains("1 passed"),
  F?         BRANCH crates/store/src/flock_cover_tests.rs:68:36 False: output.status.success() && stdout.contains("1 passed"),
  GAP (test) BRANCH crates/store/src/flock_cover_tests.rs:85:12 False: if call == self.refuse {
  GAP (test) LINE   crates/store/src/flock_cover_tests.rs:87: }
  GAP (test) LINE   crates/store/src/flock_cover_tests.rs:88: self.took.push_str(s);
  GAP (test) LINE   crates/store/src/flock_cover_tests.rs:89: Ok(())
  F?         LINE   crates/store/src/flock_cover_tests.rs:116: matches!(
  F?         BRANCH crates/store/src/flock_cover_tests.rs:133:17 False: assert!(entry.writer.is_none() && entry.queue.is_none());
  F?         BRANCH crates/store/src/flock_cover_tests.rs:133:43 False: assert!(entry.writer.is_none() && entry.queue.is_none());
  F?         BRANCH crates/store/src/flock_cover_tests.rs:199:9 False: whole_sentence.starts_with("still held after waiting 21 ms of a 20 ms bound")
  F?         BRANCH crates/store/src/flock_cover_tests.rs:200:16 False: && whole_sentence.ends_with("; pid 1 holds it"),
  F?         BRANCH crates/store/src/flock_patient_tests.rs:101:9 False: let Impatient::Held {
  F?         LINE   crates/store/src/flock_patient_tests.rs:109: panic!("contention, not a host refusal: {refused}");
  F?         BRANCH crates/store/src/flock_patient_tests.rs:123:9 False: took >= bound && took < bound + MAX_PAUSE + SLACK,
  F?         BRANCH crates/store/src/flock_patient_tests.rs:123:26 False: took >= bound && took < bound + MAX_PAUSE + SLACK,
  F?         BRANCH crates/store/src/flock_patient_tests.rs:172:9 False: let Err(Impatient::Host(why)) = got else {
  F?         LINE   crates/store/src/flock_patient_tests.rs:173: panic!("a host refusal: {got:?}");
  F?         BRANCH crates/store/src/flock_patient_tests.rs:191:9 False: let Err(Impatient::Held {
  F?         LINE   crates/store/src/flock_patient_tests.rs:195: panic!("{got:?}");
  F?         BRANCH crates/store/src/flock_patient_tests.rs:219:62 False: matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))
  F?         BRANCH crates/store/src/flock_patient_tests.rs:261:9 False: let Err(Impatient::Held {
  F?         LINE   crates/store/src/flock_patient_tests.rs:265: panic!("{got:?}");
  GAP (test) LINE   crates/store/src/flock_patient_tests.rs:323: )?;
  GAP (test) LINE   crates/store/src/flock_patient_tests.rs:364: Err(_) => refused += 1,
  F?         BRANCH crates/store/src/flock_patient_tests.rs:435:9 False: let Err(Impatient::Held {
  F?         LINE   crates/store/src/flock_patient_tests.rs:439: panic!("it gave up: {refused:?}");
  F?         BRANCH crates/store/src/flock_patient_tests.rs:565:9 False: note.contains("thread ag06-successor") && note.contains("doing the second"),
  F?         BRANCH crates/store/src/flock_patient_tests.rs:565:51 False: note.contains("thread ag06-successor") && note.contains("doing the second"),
  GAP (test) LINE   crates/store/src/flock_patient_tests.rs:611: Err(_) => refused += 1,
  F?         LINE   crates/store/src/flock_patient_tests.rs:630: Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),
  GAP (test) BRANCH crates/store/src/flock_patient_tests.rs:639:16 True: if !named {
  GAP (test) LINE   crates/store/src/flock_patient_tests.rs:640: unnamed += 1;
  F?         BRANCH crates/store/src/flock_patient_tests.rs:683:9 False: text.contains("held by this process") && text.contains("doing the journal index build"),
  F?         BRANCH crates/store/src/flock_patient_tests.rs:683:50 False: text.contains("held by this process") && text.contains("doing the journal index build"),
  F?         LINE   crates/store/src/flock_patient_tests.rs:687: matches!(
  F?         LINE   crates/store/src/flock_tests.rs:213: panic!(
  F?         LINE   crates/store/src/flock_tests.rs:244: panic!("{take:?} (release: {release}) over &File kept the lock — {why}")
  F?         LINE   crates/store/src/flock_tests.rs:264: panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")
  F?         LINE   crates/store/src/flock_tests.rs:283: panic!(
  F?         LINE   crates/store/src/flock_tests.rs:320: panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")
  GAP (test) LINE   crates/store/src/flock_tests.rs:699: matches!(
  F?         BRANCH crates/store/src/flock_tests.rs:732:13 False: assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  F?         BRANCH crates/store/src/flock_tests.rs:732:38 False: assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  F?         BRANCH crates/store/src/flock_tests.rs:759:9 False: said.contains("r3-08-holder") && !said.contains("nreadable"),
  F?         BRANCH crates/store/src/flock_tests.rs:759:42 False: said.contains("r3-08-holder") && !said.contains("nreadable"),
  F?         BRANCH crates/store/src/flock_tests.rs:841:9 False: said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  F?         BRANCH crates/store/src/flock_tests.rs:841:51 False: said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  F?         LINE   crates/store/src/flock_tests.rs:924: matches!(
  F?         BRANCH crates/store/src/flock_tests.rs:952:9 False: note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  F?         BRANCH crates/store/src/flock_tests.rs:952:66 False: note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  HOST       LINE   crates/store/src/generation.rs:519: )?;
  F?         BRANCH crates/store/src/generation_cover_tests.rs:125:55 False: Err(StoreError::Generation { reason, .. }) if reason == "it is not a regular file"
  F?         BRANCH crates/store/src/generation_cover_tests.rs:132:55 False: Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
  F?         BRANCH crates/store/src/generation_cover_tests.rs:138:55 True: Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
  F?         BRANCH crates/store/src/generation_door_cover_tests.rs:98:9 False: let Served::Revision(record0) = served(&root.0, month()).unwrap() else {
  F?         LINE   crates/store/src/generation_door_cover_tests.rs:99: panic!("revision 1 is served");
  F?         BRANCH crates/store/src/generation_door_cover_tests.rs:136:59 False: Ok(Promoted::Promoted(Pointer { record, .. })) if record == MAX_RECORDS - 1
  F?         LINE   crates/store/src/generation_door_cover_tests.rs:155: other => panic!("the pointer read is refused: {other:?}"),
  F?         LINE   crates/store/src/generation_door_cover_tests.rs:180: other => panic!("the linked pointer is refused: {other:?}"),
  F?         LINE   crates/store/src/generation_door_cover_tests.rs:212: other => panic!("the directory's creation is refused: {other:?}"),
  F?         LINE   crates/store/src/generation_door_cover_tests.rs:223: other => panic!("the pointer's open is refused: {other:?}"),
  F?         LINE   crates/store/src/generation_door_cover_tests.rs:234: other => panic!("the pointer's lock is refused: {other:?}"),
  (U)        BRANCH crates/store/src/month_summary.rs:939:24 False: if let Some(words) = entry.seconds.as_deref() {
  (U)        BRANCH crates/store/src/month_summary.rs:1055:12 True: if at.short || at.at != payload.len() {
  (U)        BRANCH crates/store/src/month_summary.rs:1085:24 False: if let Some(words) = entry.seconds.as_deref_mut() {
  (U)        LINE   crates/store/src/month_summary.rs:1087: }
  F?         LINE   crates/store/src/month_summary_decode_cover_tests.rs:171: other => panic!("the open is refused in the host's words: {other:?}"),
  F?         BRANCH crates/store/src/month_summary_tests.rs:278:52 False: matches!(&built, Summarized::Built(why) if why.starts_with("torn")),
  F?         BRANCH crates/store/src/month_summary_tests.rs:293:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  GAP (test) LINE   crates/store/src/month_summary_tests.rs:294: let _release = fs::OpenOptions::new().write(true).open(pipe);
  F?         LINE   crates/store/src/month_summary_tests.rs:295: panic!("a read of {} waited for a writer", pipe.display());
  F?         BRANCH crates/store/src/month_summary_tests.rs:320:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),
  F?         BRANCH crates/store/src/month_summary_tests.rs:340:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  F?         BRANCH crates/store/src/month_summary_tests.rs:355:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  F?         BRANCH crates/store/src/month_summary_tests.rs:377:13 False: assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  F?         BRANCH crates/store/src/month_summary_tests.rs:377:40 False: assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  F?         BRANCH crates/store/src/month_summary_tests.rs:461:32 False: Refused::Shape(why) if why.contains("flag")
  F?         BRANCH crates/store/src/month_summary_tests.rs:467:60 False: matches!(shape(&unflagged), Refused::Shape(why) if why.contains("rung")),
  F?         BRANCH crates/store/src/month_summary_tests.rs:480:59 False: assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));
  F?         BRANCH crates/store/src/month_summary_tests.rs:483:63 False: assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));
  F?         BRANCH crates/store/src/month_summary_tests.rs:488:61 False: assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));
  F?         BRANCH crates/store/src/month_summary_tests.rs:491:59 False: assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));
  F?         BRANCH crates/store/src/month_summary_tests.rs:655:13 False: assert!(round.tracks_carries() && !round.wants_carries());
  F?         BRANCH crates/store/src/month_summary_tests.rs:655:39 False: assert!(round.tracks_carries() && !round.wants_carries());
  F?         BRANCH crates/store/src/month_summary_tests.rs:658:13 False: assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  F?         BRANCH crates/store/src/month_summary_tests.rs:658:42 False: assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  F?         BRANCH crates/store/src/month_summary_tests.rs:739:37 False: Err(Refused::Shape(why)) if why.contains("not a held minute")
  F?         BRANCH crates/store/src/month_summary_tests.rs:757:13 False: assert!(!read.tracks_carries() && read.wants_carries());
  F?         BRANCH crates/store/src/month_summary_tests.rs:757:39 False: assert!(!read.tracks_carries() && read.wants_carries());
  F?         LINE   crates/store/src/month_summary_tests.rs:791: other => panic!("{other:?}"),
  F?         BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:13 False: assert!(staging.is_dir() && !root.sum().exists());
  F?         BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:33 False: assert!(staging.is_dir() && !root.sum().exists());
  HOST       LINE   crates/store/src/provenance.rs:1282: )?;
  (U)        BRANCH crates/store/src/provenance.rs:1658:12 False: if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
  (U)        BRANCH crates/store/src/provenance.rs:1666:12 False: if let Some(run) = bytes.get(at..at.saturating_add(N)) {
  F?         LINE   crates/store/src/provenance_cover_tests.rs:166: other => panic!("the pointer refuses the read: {other:?}"),
  F?         LINE   crates/store/src/provenance_cover_tests.rs:186: other => panic!("the sealed form refuses the read: {other:?}"),
  F?         BRANCH crates/store/src/provenance_cover_tests.rs:203:9 False: let Found::Present(raw) = read(&root.0, june).unwrap() else {
  F?         LINE   crates/store/src/provenance_cover_tests.rs:204: panic!("the raw sidecar holds a record");
  F?         LINE   crates/store/src/provenance_cover_tests.rs:218: other => panic!("the sealed image's record is served: {other:?}"),
  F?         LINE   crates/store/src/provenance_cover_tests.rs:252: other => panic!("the open is refused by name: {other:?}"),
  F?         BRANCH crates/store/src/provenance_cover_tests.rs:275:9 False: let Found::Present(sidecar) = read(&root.0, june).unwrap() else {
  F?         LINE   crates/store/src/provenance_cover_tests.rs:276: panic!("the batch's record is on disk");
  F?         LINE   crates/store/src/provenance_cover_tests.rs:325: other => panic!("the undecodable head refuses the read: {other:?}"),
  F?         LINE   crates/store/src/provenance_cover_tests.rs:349: other => panic!("the lock refuses the landing: {other:?}"),
  F?         LINE   crates/store/src/provenance_cover_tests.rs:394: other => panic!("the refused look refuses the flush: {other:?}"),
  HOST       LINE   crates/store/src/quarantine.rs:831: )?;
  HOST       LINE   crates/store/src/quarantine.rs:861: )?;
  HOST       LINE   crates/store/src/quarantine.rs:1032: Err(std::fs::TryLockError::Error(_)) => false,
  HOST       LINE   crates/store/src/quarantine.rs:1186: )?;
  RACE       BRANCH crates/store/src/quarantine.rs:1187:8 True: if image.len() as u64 != INDEX_LEN {
  RACE       LINE   crates/store/src/quarantine.rs:1188: return Err(QuarantineError::Torn {
  RACE       LINE   crates/store/src/quarantine.rs:1189: path: index.to_path_buf(),
  RACE       LINE   crates/store/src/quarantine.rs:1190: len: image.len() as u64,
  RACE       LINE   crates/store/src/quarantine.rs:1191: });
  RACE       BRANCH crates/store/src/quarantine.rs:1439:25 False: Err(why) if why.kind() != io::ErrorKind::AlreadyExists => {
  HOST       LINE   crates/store/src/quarantine.rs:1467: Err(other) => Err(QuarantineError::Io {
  HOST       LINE   crates/store/src/quarantine.rs:1468: path,
  HOST       LINE   crates/store/src/quarantine.rs:1469: operation: "lock the quarantine",
  HOST       LINE   crates/store/src/quarantine.rs:1470: kind: impatient_kind(&other),
  HOST       LINE   crates/store/src/quarantine.rs:1471: }),
  HOST       LINE   crates/store/src/quarantine.rs:1759: )?;
  HOST       LINE   crates/store/src/quarantine.rs:2218: )?;
  HOST       LINE   crates/store/src/quarantine.rs:2224: )?;
  F?         BRANCH crates/store/src/quarantine_cover_tests.rs:449:65 False: matches!(&refused, QuarantineError::Reason(text) if text.contains(said)),
  F?         LINE   crates/store/src/quarantine_cover_tests.rs:635: Ok(Level::Current(_)) => panic!("a torn image is never current"),
  F?         LINE   crates/store/src/quarantine_cover_tests.rs:636: Err(refused) => panic!("a torn image is rebuilt, not refused: {refused}"),
  F?         LINE   crates/store/src/quarantine_cover_tests.rs:643: Ok(Level::Current(_)) => panic!("a damaged header is never current"),
  F?         LINE   crates/store/src/quarantine_cover_tests.rs:644: Err(refused) => panic!("a damaged header is rebuilt, not refused: {refused}"),
  F?         BRANCH crates/store/src/quarantine_tests.rs:144:9 False: let Applied::Dropped { drop, .. } = applied else {
  F?         LINE   crates/store/src/quarantine_tests.rs:145: panic!("expected a dropped generation, got {applied:?}");
  F?         BRANCH crates/store/src/quarantine_tests.rs:174:9 False: let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {
  F?         LINE   crates/store/src/quarantine_tests.rs:175: panic!("the month is labelled");
  F?         BRANCH crates/store/src/quarantine_tests.rs:210:63 False: assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));
  F?         BRANCH crates/store/src/quarantine_tests.rs:263:9 False: let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {
  F?         LINE   crates/store/src/quarantine_tests.rs:264: panic!("the month is answered by name");
  F?         BRANCH crates/store/src/quarantine_tests.rs:342:66 False: matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),
  F?         BRANCH crates/store/src/quarantine_tests.rs:348:77 False: assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));
  F?         BRANCH crates/store/src/quarantine_tests.rs:497:42 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  F?         BRANCH crates/store/src/quarantine_tests.rs:497:71 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  F?         BRANCH crates/store/src/quarantine_tests.rs:508:73 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  F?         BRANCH crates/store/src/quarantine_tests.rs:508:92 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  F?         LINE   crates/store/src/quarantine_tests.rs:551: other => panic!("expected the hold named, got {other:?}"),
  F?         BRANCH crates/store/src/quarantine_tests.rs:799:9 False: let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {
  F?         LINE   crates/store/src/quarantine_tests.rs:800: panic!("the month is still answered by name");
  F?         BRANCH crates/store/src/quarantine_tests.rs:855:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F?         BRANCH crates/store/src/quarantine_tests.rs:855:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F?         LINE   crates/store/src/quarantine_tests.rs:892: other => panic!("{why}: {other:?}"),
  F?         BRANCH crates/store/src/quarantine_tests.rs:1071:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  F?         BRANCH crates/store/src/quarantine_tests.rs:1072:16 False: && e.bars == 6
  F?         BRANCH crates/store/src/quarantine_tests.rs:1073:16 False: && e.causes.said() == "before_listing and foreign_level"
  F?         LINE   crates/store/src/quarantine_tests.rs:1092: other => panic!("expected the quarantine to refuse by name, got {other:?}"),
  F?         LINE   crates/store/src/quarantine_tests.rs:1268: other => panic!("the rerun indexes the drop: {other:?}"),
  F?         BRANCH crates/store/src/quarantine_tests.rs:1327:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F?         BRANCH crates/store/src/quarantine_tests.rs:1327:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F?         BRANCH crates/store/src/quarantine_tests.rs:1386:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  F?         BRANCH crates/store/src/quarantine_tests.rs:1386:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  F?         BRANCH crates/store/src/quarantine_tests.rs:1419:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  F?         BRANCH crates/store/src/quarantine_tests.rs:1466:38 False: Ok(Probe::Quarantined(e)) if e.bars == 3
  F?         BRANCH crates/store/src/quarantine_tests.rs:1533:9 False: refused.is_err() && words.contains("quarantined"),
  F?         BRANCH crates/store/src/quarantine_tests.rs:1533:29 False: refused.is_err() && words.contains("quarantined"),
  F?         BRANCH crates/store/src/quarantine_tests.rs:1584:9 False: refused.is_err() && words.contains("quarantined"),
  F?         BRANCH crates/store/src/quarantine_tests.rs:1584:29 False: refused.is_err() && words.contains("quarantined"),
  F?         BRANCH crates/store/src/quarantine_tests.rs:1639:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  F?         BRANCH crates/store/src/quarantine_tests.rs:1639:67 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  F?         BRANCH crates/store/src/quarantine_tests.rs:1639:82 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  F?         BRANCH crates/store/src/quarantine_tests.rs:1747:58 False: matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  F?         BRANCH crates/store/src/quarantine_tests.rs:1747:94 False: matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  F?         BRANCH crates/store/src/quarantine_tests.rs:1865:77 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),
  F?         LINE   crates/store/src/quarantine_tests.rs:1995: other => panic!("{other:?}"),
  F?         BRANCH crates/store/src/quarantine_tests.rs:2261:42 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
  F?         BRANCH crates/store/src/quarantine_tests.rs:2261:71 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
  F?         BRANCH crates/store/src/quarantine_tests.rs:2261:86 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
  F?         BRANCH crates/store/src/quarantine_tests.rs:2316:73 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth),
  F?         LINE   crates/store/src/quarantine_tests.rs:2348: other => panic!("the lock's open refuses the act: {other:?}"),
  F?         LINE   crates/store/src/quarantine_tests.rs:2365: other => panic!("the index's open refuses the act: {other:?}"),
  (U)        LINE   crates/store/src/repair.rs:851: rows: header.n_valid,
  (U)        LINE   crates/store/src/repair.rs:852: ceiling,
  (U)        LINE   crates/store/src/repair.rs:853: };
  RACE       BRANCH crates/store/src/repair.rs:1183:24 True: if reader.read_record(index as u64)? != *bar {
  RACE       LINE   crates/store/src/repair.rs:1184: return Err(RepairError::ReadBack {
  RACE       LINE   crates/store/src/repair.rs:1185: index: index as u64,
  RACE       LINE   crates/store/src/repair.rs:1186: });
  HOST       LINE   crates/store/src/repair.rs:1342: )?;
  RACE       BRANCH crates/store/src/repair.rs:1373:12 True: if writer.read_record(index as u64)? != *offered {
  RACE       LINE   crates/store/src/repair.rs:1374: return Err(RepairError::Conflict);
  HOST       LINE   crates/store/src/repair.rs:1392: )?;
  HOST       LINE   crates/store/src/repair.rs:1398: )?;
  HOST       LINE   crates/store/src/repair.rs:1458: Err(TryLockError::Error(error)) => io_at(Err(error), &path, "lock shared", false),
  F?         BRANCH crates/store/src/repair_cover_tests.rs:62:41 False: text.contains("refused") || text.contains("drop names"),
  F?         LINE   crates/store/src/repair_door_cover_tests.rs:221: other @ MonthRestate::NothingNew { .. } => {
  F?         LINE   crates/store/src/repair_door_cover_tests.rs:222: panic!("the added bars are published: {other:?}")
  F?         LINE   crates/store/src/repair_door_cover_tests.rs:246: other @ MonthRepair::NothingNew { .. } => {
  F?         LINE   crates/store/src/repair_door_cover_tests.rs:247: panic!("the empty month takes the bars: {other:?}")
  F?         LINE   crates/store/src/repair_door_cover_tests.rs:323: matches!(
  F?         LINE   crates/store/src/repair_door_cover_tests.rs:393: other => panic!("the added bar is restated: {other:?}"),
  F?         BRANCH crates/store/src/repair_door_cover_tests.rs:424:9 False: output.status.success() && stdout.contains("1 passed"),
  F?         BRANCH crates/store/src/repair_door_cover_tests.rs:424:36 False: output.status.success() && stdout.contains("1 passed"),
  F?         LINE   crates/store/src/repair_tests.rs:116: Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),
  F?         LINE   crates/store/src/repair_tests.rs:630: timeframe.as_str()
  GAP (test) BRANCH crates/store/src/repair_tests.rs:691:12 False: if let Ok(entries) = fs::read_dir(dir) {
  GAP (test) LINE   crates/store/src/repair_tests.rs:700: }
  F?         BRANCH crates/store/src/repair_tests.rs:824:94 False: matches!(&held, Err(RepairError::Store(crate::file::StoreError::Locked { path })) if *path == lock),
  F?         BRANCH crates/store/src/repair_tests.rs:882:9 False: let super::MonthRestate::Restated {
  F?         LINE   crates/store/src/repair_tests.rs:891: panic!("expected a restated generation, got {done:?}");
  HOST       BRANCH crates/store/src/safe_open.rs:104:19 False: } else if kind.is_block_device() {
  HOST       LINE   crates/store/src/safe_open.rs:107: Some(Self::Unknown)
  (U)        BRANCH crates/store/src/safe_open.rs:299:9 False: let Some(flags) = FLAGS else {
  (U)        LINE   crates/store/src/safe_open.rs:300: return Err(io(
  (U)        LINE   crates/store/src/safe_open.rs:301: "open",
  (U)        LINE   crates/store/src/safe_open.rs:302: io::Error::new(
  (U)        LINE   crates/store/src/safe_open.rs:303: ErrorKind::Unsupported,
  (U)        LINE   crates/store/src/safe_open.rs:304: "safe reads require verified macOS or Linux x86_64/aarch64 open flags",
  (U)        LINE   crates/store/src/safe_open.rs:305: ),
  (U)        LINE   crates/store/src/safe_open.rs:306: ));
  GAP (test) BRANCH crates/store/src/safe_open_tests.rs:66:16 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
  GAP (test) LINE   crates/store/src/safe_open_tests.rs:67: tried.push(format!("{}: {why}", link.display()));
  GAP (test) LINE   crates/store/src/safe_open_tests.rs:68: continue;
  GAP (test) LINE   crates/store/src/safe_open_tests.rs:74: Err(why) => tried.push(format!("{}: {why}", link.display())),
  F?         LINE   crates/store/src/safe_open_tests.rs:77: panic!(
  F?         LINE   crates/store/src/safe_open_tests.rs:79: path.display(),
  F?         LINE   crates/store/src/safe_open_tests.rs:80: tried.join("; ")
  F?         BRANCH crates/store/src/safe_open_tests.rs:95:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  F?         LINE   crates/store/src/safe_open_tests.rs:103: found.file_type()
  F?         LINE   crates/store/src/safe_open_tests.rs:110: link.display()
  F?         BRANCH crates/store/src/safe_open_tests.rs:122:9 False: let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {
  GAP (test) LINE   crates/store/src/safe_open_tests.rs:123: let _release = fs::OpenOptions::new().write(true).open(pipe);
  F?         LINE   crates/store/src/safe_open_tests.rs:124: panic!("an open of {} waited for a writer", pipe.display());
  GAP (test) BRANCH crates/store/src/safe_open_tests.rs:203:12 False: if let Some(device) = a_block_device() {
  F?         BRANCH crates/store/src/safe_open_tests.rs:268:8 True: if fs::File::open(&path).is_err() {
  GAP (test) LINE   crates/store/src/safe_open_tests.rs:269: let refusal = open_read(&path).unwrap_err();
  F?         LINE   crates/store/src/safe_open_tests.rs:270: assert_eq!((refusal.word(), refusal.found()), ("io", None), "{refusal}");
  F?         LINE   crates/store/src/safe_open_tests.rs:271: assert!(!refusal.is_missing());
  F?         BRANCH crates/store/src/safe_open_tests.rs:298:16 False: if *root == shadow && *bars == shadow.join(BARS_DIR)
  F?         BRANCH crates/store/src/safe_open_tests.rs:298:35 False: if *root == shadow && *bars == shadow.join(BARS_DIR)
  F?         BRANCH crates/store/src/safe_open_tests.rs:299:20 False: && target.as_deref() == Some(real.join(BARS_DIR).as_path())),
  F?         LINE   crates/store/src/safe_open_tests.rs:401: Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),
  F?         LINE   crates/store/src/safe_open_tests.rs:402: Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),
  F?         BRANCH crates/store/src/safe_open_tests.rs:459:77 False: matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),
  (U)        BRANCH crates/store/src/sealed.rs:1499:23 False: } else if let Some(range) = self.ranges.get_mut(5) {
  (U)        BRANCH crates/store/src/sealed.rs:1525:16 False: if let Some(oi) = columns.get_mut(5) {
  (U)        BRANCH crates/store/src/sealed.rs:2352:35 False: Some(previous) if block > 0 => entries.get(previous).map_or(0, |entry| entry.end),
  F?         LINE   crates/store/src/sealed_codec_tests.rs:81: frame.len()
  F?         LINE   crates/store/src/sealed_codec_tests.rs:282: frame.len()
  F?         LINE   crates/store/src/sealed_codec_tests.rs:439: largest[rows],
  F?         LINE   crates/store/src/sealed_codec_tests.rs:440: max_frame_len(m)
  F?         BRANCH crates/store/src/sealed_codec_tests.rs:1511:21 False: assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  F?         BRANCH crates/store/src/sealed_codec_tests.rs:1511:38 False: assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  F?         BRANCH crates/store/src/sealed_codec_tests.rs:1556:13 False: last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  F?         BRANCH crates/store/src/sealed_codec_tests.rs:1556:33 False: last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  F?         LINE   crates/store/src/sealed_tests.rs:159: Ok(value) => panic!("expected a refusal, got {value:?}"),
  GAP (test) BRANCH crates/store/src/sealed_tests.rs:237:12 False: if offset <= self.fail_at && self.fail_at < offset + buf.len() as u64 {
  GAP (test) LINE   crates/store/src/sealed_tests.rs:499: Ok(())
  F?         LINE   crates/store/src/sealed_tests.rs:502: Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),
  F?         LINE   crates/store/src/sealed_tests.rs:653: other => panic!("{other}"),
  F?         BRANCH crates/store/src/sealed_tests.rs:696:27 False: found == 0 || bars[found as usize - 1].ts_micros < ts,
  F?         LINE   crates/store/src/sealed_tests.rs:837: other => panic!("{field}: {other}"),
  GAP (test) LINE   crates/store/src/sealed_tests.rs:939: fn flush(&mut self) -> io::Result<()> {
  GAP (test) LINE   crates/store/src/sealed_tests.rs:940: Ok(())
  GAP (test) LINE   crates/store/src/sealed_tests.rs:941: }
  F?         LINE   crates/store/src/sealed_tests.rs:1170: packed.len(),
  F?         LINE   crates/store/src/sealed_tests.rs:1171: image.len()
  F?         LINE   crates/store/src/sealed_tests.rs:1192: other => panic!("{other:?}"),
  F?         LINE   crates/store/src/sealed_tests.rs:1241: other => panic!("{other}"),
  F?         LINE   crates/store/src/sealed_tests.rs:1319: other => panic!("{other}"),
  F?         LINE   crates/store/src/sealed_tests.rs:1656: other => panic!("{field}: {other}"),
  F?         LINE   crates/store/src/sealed_tests.rs:1673: other => panic!("{other}"),
  F?         LINE   crates/store/src/sealed_tests.rs:1689: other => panic!("{other}"),
  F?         LINE   crates/store/src/sealed_tests.rs:1774: other => panic!("{other}"),
  F?         LINE   crates/store/src/sealed_tests.rs:1802: other => panic!("{other}"),
  F?         BRANCH crates/store/src/sealed_tests.rs:1816:100 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)
  F?         BRANCH crates/store/src/sealed_tests.rs:1822:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  F?         BRANCH crates/store/src/sealed_tests.rs:1828:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)
  F?         BRANCH crates/store/src/sealed_tests.rs:1836:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  F?         LINE   crates/store/src/sealed_tests.rs:1891: other => panic!("{other}"),
  F?         LINE   crates/store/src/sealed_tests.rs:1982: matches!(
  F?         LINE   crates/store/src/sealed_tests.rs:2104: other => panic!("{other}"),
  (U)        BRANCH crates/store/src/settings.rs:220:16 False: if let Some(slot) = self.values.get_mut(key.slot()) {
  (U)        BRANCH crates/store/src/settings.rs:688:30 False: let percent_too = if unit.is_empty() && matches!(amount, 1..=95) {
  RACE       BRANCH crates/store/src/settings.rs:885:8 True: if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
  RACE       LINE   crates/store/src/settings.rs:886: return Err(fault(format!(
  RACE       LINE   crates/store/src/settings.rs:887: "it grew past the {MAX_FILE_BYTES} bytes a settings file may hold while it was read"
  RACE       LINE   crates/store/src/settings.rs:888: )));
  RACE       LINE   crates/store/src/settings.rs:1227: _ => ErrorKind::AlreadyExists,
  HOST       BRANCH crates/store/src/settings.rs:1234:12 True: if let Err(why) = written {
  HOST       LINE   crates/store/src/settings.rs:1235: let _ = std::fs::remove_file(&temp);
  HOST       LINE   crates/store/src/settings.rs:1236: return Err(io_refusal(&temp, &why));
  RACE       LINE   crates/store/src/settings.rs:1321: path: fault.path,
  RACE       LINE   crates/store/src/settings.rs:1322: kind: ErrorKind::Other,
  RACE       LINE   crates/store/src/settings.rs:1323: why: format!("the settings directory cannot be read: {}", fault.why),
  RACE       LINE   crates/store/src/settings.rs:1324: })?;
  RACE       LINE   crates/store/src/settings.rs:1367: match linked {
  RACE       BRANCH crates/store/src/settings.rs:1369:21 True: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
  RACE       BRANCH crates/store/src/settings.rs:1369:21 False: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
  RACE       LINE   crates/store/src/settings.rs:1369: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
  RACE       LINE   crates/store/src/settings.rs:1370: let _ = reader.refresh();
  RACE       LINE   crates/store/src/settings.rs:1371: return Err(Refusal::Stale {
  RACE       LINE   crates/store/src/settings.rs:1372: expected: change.expected,
  RACE       LINE   crates/store/src/settings.rs:1373: newest: reader.highest().max(number),
  RACE       LINE   crates/store/src/settings.rs:1374: });
  RACE       LINE   crates/store/src/settings.rs:1376: Err(why) => return Err(io_refusal(&target, &why)),
  RACE       LINE   crates/store/src/settings.rs:1445: Ok(_) => "it reads when read again, and the refresh did not step to it".to_owned(),
  F?         BRANCH crates/store/src/settings_cover_tests.rs:198:57 False: Err(Refusal::Invalid { key: None, why }) if why.contains("1 to 240 bytes")
  F?         BRANCH crates/store/src/settings_door_cover_tests.rs:92:9 False: with_mb.contains("is not a floor") && !with_mb.contains("percent"),
  F?         BRANCH crates/store/src/settings_door_cover_tests.rs:92:47 False: with_mb.contains("is not a floor") && !with_mb.contains("percent"),
  F?         LINE   crates/store/src/settings_door_cover_tests.rs:123: other => panic!("the listing refuses the save: {other:?}"),
  F?         BRANCH crates/store/src/settings_door_cover_tests.rs:157:9 False: fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
  F?         BRANCH crates/store/src/settings_door_cover_tests.rs:157:51 False: fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
  F?         LINE   crates/store/src/settings_tests.rs:152: other => panic!("{bad:?} for {key:?}: {other:?}"),
  F?         LINE   crates/store/src/settings_tests.rs:228: matches!(
  GAP (test) BRANCH crates/store/src/settings_tests.rs:255:12 False: if s.get(Key::OpenFeed) == Some("dhan") {
  GAP (test) LINE   crates/store/src/settings_tests.rs:261: Ok(())
  F?         LINE   crates/store/src/settings_tests.rs:414: reader.faults()
  F?         LINE   crates/store/src/settings_tests.rs:580: reader.faults()
  F?         LINE   crates/store/src/settings_tests.rs:651: reader.stopped()
  GAP (test) BRANCH crates/store/src/settings_tests.rs:708:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  GAP (test) BRANCH crates/store/src/settings_tests.rs:709:16 True: if let Some(pipe) = pipe {
  GAP (test) BRANCH crates/store/src/settings_tests.rs:709:16 False: if let Some(pipe) = pipe {
  GAP (test) LINE   crates/store/src/settings_tests.rs:709: if let Some(pipe) = pipe {
  GAP (test) LINE   crates/store/src/settings_tests.rs:710: let _release = std::fs::OpenOptions::new().write(true).open(pipe);
  GAP (test) LINE   crates/store/src/settings_tests.rs:711: }
  F?         LINE   crates/store/src/settings_tests.rs:712: panic!("{what}: the settings read waited instead of naming a fault");
  F?         BRANCH crates/store/src/settings_tests.rs:786:48 False: matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),
  F?         BRANCH crates/store/src/settings_tests.rs:910:60 False: matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),
  F?         BRANCH crates/store/src/settings_tests.rs:956:60 False: matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("exhausted")),
  F?         LINE   crates/store/src/settings_tests.rs:990: matches!(
  F?         BRANCH crates/store/src/settings_tests.rs:1097:9 False: read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  F?         BRANCH crates/store/src/settings_tests.rs:1097:52 False: read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  GAP (test) BRANCH crates/store/src/settings_tests.rs:1153:20 True: if started.elapsed() > std::time::Duration::from_secs(10) {
  GAP (test) LINE   crates/store/src/settings_tests.rs:1154: return None;
  F?         BRANCH crates/store/src/settings_tests.rs:1187:13 True: a.superseded_by == Some(3) || a.in_force == Some(2),
  F?         BRANCH crates/store/src/settings_tests.rs:1187:43 False: a.superseded_by == Some(3) || a.in_force == Some(2),
  F?         BRANCH crates/store/src/small_month_tests.rs:152:52 False: Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1
  F?         BRANCH crates/store/src/sort_index_cover_tests.rs:294:38 False: Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
  F?         BRANCH crates/store/src/sort_index_cover_tests.rs:301:38 False: Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
  F?         BRANCH crates/store/src/sort_index_cover_tests.rs:305:38 False: Err(Why::Unreadable(why)) if why.contains("is not a regular file")
  F?         BRANCH crates/store/src/sort_index_cover_tests.rs:312:38 False: Err(Why::Unreadable(why)) if why.contains("is a symbolic link")
  F?         BRANCH crates/store/src/sort_index_cover_tests.rs:318:38 False: Err(Why::Unreadable(why)) if why.starts_with(&under.display().to_string())
  F?         BRANCH crates/store/src/sort_index_tests.rs:324:41 False: assert!(why.contains("kind") || why.contains("reserved"), "{why}");
  F?         LINE   crates/store/src/staging_cover_tests.rs:56: Err(other) => panic!("the removal is refused by name: {other:?}"),
  F?         LINE   crates/store/src/staging_cover_tests.rs:57: Ok(_) => panic!("the removal is refused by name, not staged"),
  F?         BRANCH crates/store/src/staging_tests.rs:143:16 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F?         BRANCH crates/store/src/staging_tests.rs:143:64 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F?         BRANCH crates/store/src/staging_tests.rs:206:67 False: matches!(&refusal, StagingError::Contended { path: p } if *p == path),
  F?         BRANCH crates/store/src/staging_tests.rs:238:16 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F?         BRANCH crates/store/src/staging_tests.rs:238:64 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F?         BRANCH crates/store/src/staging_tests.rs:297:78 False: matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
  F?         BRANCH crates/store/src/staging_tests.rs:334:88 False: matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),
  F?         BRANCH crates/store/src/staging_tests.rs:369:77 False: matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
  F?         LINE   crates/store/src/staging_tests.rs:421: within.display()
  F?         BRANCH crates/store/src/time_index.rs:921:21 False: assert!(entry_reads <= 1 && bar_reads <= 1);
  F?         BRANCH crates/store/src/time_index.rs:921:41 False: assert!(entry_reads <= 1 && bar_reads <= 1);
  F?         BRANCH crates/store/src/time_index.rs:1057:17 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  F?         BRANCH crates/store/src/time_index.rs:1057:38 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  F?         BRANCH crates/store/src/time_index.rs:1057:59 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  F?         BRANCH crates/store/src/time_index.rs:1058:17 False: assert!(!entry.occupied(1) && !entry.occupied(62));
  F?         BRANCH crates/store/src/time_index.rs:1058:39 False: assert!(!entry.occupied(1) && !entry.occupied(62));
  F?         BRANCH crates/store/src/time_index.rs:1105:17 False: assert!(!daily.exact() && g.exact());
  F?         BRANCH crates/store/src/time_index.rs:1105:35 False: assert!(!daily.exact() && g.exact());
  (U)        BRANCH crates/store/src/verdict.rs:716:8 True: if Vendor::ALL.get(at) != Some(&feed) {
  (U)        LINE   crates/store/src/verdict.rs:717: return None;
  (U)        BRANCH crates/store/src/verdict.rs:2278:16 False: if let Some(slot) = out.first.get_mut(ordinal) {
  (U)        BRANCH crates/store/src/verdict.rs:2312:16 False: if let Some(count) = out.counts.get_mut(usize::from(code.code())) {
  (U)        BRANCH crates/store/src/verdict.rs:2478:16 False: if let Some(c) = counts.get_mut(usize::from(record.facts.code.code())) {
  (U)        BRANCH crates/store/src/verdict.rs:3143:15 False: while let Some(parent) = at.parent() {
  (U)        BRANCH crates/store/src/verdict.rs:3417:12 False: if let Some(entry) = days.get_mut(doy) {
  (U)        BRANCH crates/store/src/verdict.rs:3853:24 False: && let Some(oldest) = self
  (U)        BRANCH crates/store/src/verdict.rs:3923:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
  (U)        LINE   crates/store/src/verdict.rs:3924: return Err(VerdictError::ImageMissing { path });
  (U)        BRANCH crates/store/src/verdict.rs:4075:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
  (U)        LINE   crates/store/src/verdict.rs:4076: return Err(VerdictError::ImageMissing { path });
  (U)        BRANCH crates/store/src/verdict.rs:4136:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
  (U)        LINE   crates/store/src/verdict.rs:4137: return Err(VerdictError::ImageMissing { path });
  F?         LINE   crates/store/src/verdict.rs:4219: other => panic!("not image_corrupt: {other:?}"),
  F?         LINE   crates/store/src/verdict.rs:4227: other => panic!("not written_by_newer_build: {other:?}"),
  F?         BRANCH crates/store/src/verdict.rs:4637:25 False: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  F?         BRANCH crates/store/src/verdict.rs:4637:37 True: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  F?         BRANCH crates/store/src/verdict.rs:4637:37 False: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  F?         BRANCH crates/store/src/verdict.rs:4974:24 False: .find(|&i| mark(&bytes, i) != 0 && tag(&bytes, i) == other_tag)
  F?         LINE   crates/store/src/verdict_tests.rs:366: other => panic!("expected the host's refusal, got {other}"),
  F?         BRANCH crates/store/src/verdict_tests.rs:460:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  GAP (test) LINE   crates/store/src/verdict_tests.rs:461: let _release = fs::OpenOptions::new().write(true).open(pipe);
  F?         LINE   crates/store/src/verdict_tests.rs:462: panic!("a verdict call waited on the pipe at {}", pipe.display());
  F?         LINE   crates/store/src/verdict_tests.rs:568: other => panic!("expected the host's refusal, got {other}"),
  F?         LINE   crates/store/src/verdict_tests.rs:813: other => panic!("not image_corrupt: {other:?}"),
  F?         LINE   crates/store/src/verdict_tests.rs:879: other.dir().display()
  F?         BRANCH crates/store/src/verdict_tests.rs:950:9 False: let super::Found::Record { ordinal, record } = found.found else {
  F?         LINE   crates/store/src/verdict_tests.rs:951: panic!("the key's record: {:?}", found.found);
  GAP (test) BRANCH crates/store/tests/catalog.rs:68:12 False: if let Some(parent) = full.parent() {
  GAP (test) LINE   crates/store/tests/catalog.rs:308: match made {
  GAP (test) LINE   crates/store/tests/catalog.rs:310: Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {
  GAP (test) LINE   crates/store/tests/catalog.rs:311: refused_by_the_volume += 1;
  GAP (test) LINE   crates/store/tests/catalog.rs:312: }
  F?         LINE   crates/store/tests/catalog.rs:313: Err(why) => panic!("{} could not be made: {why}", full.display()),
  F?         BRANCH crates/store/tests/catalog.rs:317:9 False: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  F?         BRANCH crates/store/tests/catalog.rs:317:39 True: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  F?         BRANCH crates/store/tests/catalog.rs:317:39 False: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  GAP (test) BRANCH crates/store/tests/catalog.rs:320:8 True: if refused_by_the_volume > 0 {
  GAP (test) LINE   crates/store/tests/catalog.rs:321: eprintln!(
  GAP (test) LINE   crates/store/tests/catalog.rs:322: "this volume refused all {refused_by_the_volume} non-UTF-8 names (os error 92); \
  GAP (test) LINE   crates/store/tests/catalog.rs:323: the walk is checked on what it holds and the five shapes by \
  GAP (test) LINE   crates/store/tests/catalog.rs:324: every_non_utf8_shape_is_counted_by_the_walks_own_step_on_every_host"
  GAP (test) LINE   crates/store/tests/catalog.rs:325: );
  GAP (test) BRANCH crates/store/tests/catalog.rs:814:16 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
  GAP (test) LINE   crates/store/tests/catalog.rs:815: tried.push(format!("{}: {why}", link.display()));
  GAP (test) LINE   crates/store/tests/catalog.rs:816: continue;
  GAP (test) LINE   crates/store/tests/catalog.rs:822: Err(why) => tried.push(format!("{}: {why}", link.display())),
  F?         LINE   crates/store/tests/catalog.rs:825: panic!(
  F?         LINE   crates/store/tests/catalog.rs:827: path.display(),
  F?         LINE   crates/store/tests/catalog.rs:828: tried.join("; ")
  F?         BRANCH crates/store/tests/catalog.rs:844:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  F?         LINE   crates/store/tests/catalog.rs:852: found.file_type()
  F?         LINE   crates/store/tests/catalog.rs:859: link.display()
  F?         LINE   crates/store/tests/catalog.rs:875: path.display()
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:288:12 True: if let Err(why) = Command::new("git").arg("--version").output() {
  GAP (test) LINE   crates/store/tests/cited_commits.rs:289: println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");
  GAP (test) LINE   crates/store/tests/cited_commits.rs:290: return false;
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:294:21 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:306:12 False: if let Some(above) = root.parent() {
  F?         BRANCH crates/store/tests/cited_commits.rs:371:9 False: said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  F?         BRANCH crates/store/tests/cited_commits.rs:372:16 False: && said.contains("not a git repository"),
  F?         BRANCH crates/store/tests/cited_commits.rs:384:9 False: said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  F?         BRANCH crates/store/tests/cited_commits.rs:385:16 False: && said.contains("not a git repository"),
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:567:13 False: let Some(citing) = cited.get(commit) else {
  GAP (test) LINE   crates/store/tests/cited_commits.rs:568: faults.push(format!(
  GAP (test) LINE   crates/store/tests/cited_commits.rs:571: continue;
  GAP (test) LINE   crates/store/tests/cited_commits.rs:575: faults.push(format!(
  GAP (test) LINE   crates/store/tests/cited_commits.rs:576: "{commit} is cited in {} by a sentence that does not say \"{says}\": {}",
  GAP (test) LINE   crates/store/tests/cited_commits.rs:577: at.name, at.sentence
  GAP (test) LINE   crates/store/tests/cited_commits.rs:578: ));
  GAP (test) LINE   crates/store/tests/cited_commits.rs:579: }
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:601:16 True: if !is_ancestor_at(root, commit, main_ref) {
  GAP (test) LINE   crates/store/tests/cited_commits.rs:602: let names: BTreeSet<&str> = citing.iter().map(|at| at.name.as_str()).collect();
  GAP (test) LINE   crates/store/tests/cited_commits.rs:603: faults.push(format!(
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:610:12 True: if is_ancestor_at(root, commit, main_ref) {
  GAP (test) LINE   crates/store/tests/cited_commits.rs:611: faults.push(format!("{commit} is listed as {off:?}, and it is on main"));
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:613:35 True: if off == Off::Neither && is_ancestor_at(root, commit, "HEAD") {
  GAP (test) LINE   crates/store/tests/cited_commits.rs:614: faults.push(format!(
  GAP (test) LINE   crates/store/tests/cited_commits.rs:615: "{commit} is listed as {off:?}, and it is an ancestor of HEAD"
  GAP (test) LINE   crates/store/tests/cited_commits.rs:616: ));
  F?         LINE   crates/store/tests/cited_commits.rs:673: cited.len()
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:676:8 False: if history_is_here() {
  F?         LINE   crates/store/tests/cited_commits.rs:709: String::from_utf8_lossy(&out.stderr)
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:731:8 True: if Command::new("git").arg("--version").output().is_err() {
  GAP (test) LINE   crates/store/tests/cited_commits.rs:732: println!("SKIPPING: git cannot run the synthetic history fixture");
  GAP (test) LINE   crates/store/tests/cited_commits.rs:733: return;
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:819:8 True: if Command::new("git").arg("--version").output().is_err() {
  GAP (test) LINE   crates/store/tests/cited_commits.rs:820: println!("SKIPPING: git cannot run the synthetic history fixture");
  GAP (test) LINE   crates/store/tests/cited_commits.rs:821: return;
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:898:39 False: } else if token.len() == 2 && token.starts_with('v') {
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:903:19 False: } else if !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit() || b == b'/') {
  F?         LINE   crates/store/tests/cited_commits.rs:976: sites.len()
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:978:8 True: if !history_is_here() {
  GAP (test) LINE   crates/store/tests/cited_commits.rs:979: return;
  GAP (test) BRANCH crates/store/tests/cited_commits.rs:996:12 True: if !takes_a_lock(line) {
  GAP (test) LINE   crates/store/tests/cited_commits.rs:997: wrong.push(format!("{module}.rs:{number}: {}", line.trim()));
  GAP (test) BRANCH crates/store/tests/cloud_mutants_store.rs:144:48 False: while (vanished < 20 || contended < 20) && Instant::now() < deadline {
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:162:9 False: vanished >= 20 && contended >= 20,
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:162:27 False: vanished >= 20 && contended >= 20,
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:201:18 False: } if source.kind() == std::io::ErrorKind::PermissionDenied
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:367: other => panic!("{door}: {other:?}"),
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:370:9 False: let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:371: panic!("a moved revision is audited");
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:374:9 False: audited.starts_with("repair revision 1 of ")
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:375:16 False: && audited.ends_with(" is not the one its promotion recorded; not audited"),
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:446: other => panic!("{other:?}"),
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:1073: other => panic!("not image_corrupt: {other:?}"),
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1097:9 False: let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:1098: panic!("ZZB is in the day");
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:1171: matches!(
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1315:9 False: generation.contains("/nonexistent/cloud/2031-02.tix")
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1316:16 False: && generation.contains("revision 1 is not the one its promotion recorded"),
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:1361: other => panic!("{other:?}"),
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:1568: other => panic!("{other:?}"),
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1642:20 False: if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1642:37 False: if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP (test) BRANCH crates/store/tests/cloud_mutants_store.rs:1686:20 False: if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP (test) BRANCH crates/store/tests/cloud_mutants_store.rs:1686:37 False: if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP (test) BRANCH crates/store/tests/cloud_mutants_store.rs:1720:24 False: if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  GAP (test) BRANCH crates/store/tests/cloud_mutants_store.rs:1720:38 False: if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1823:20 False: if path == &lock_path && *tries == steps + 1
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1823:42 False: if path == &lock_path && *tries == steps + 1
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1923:13 False: let Some(Found::Record { ordinal, record }) = lookup.answer.map(|answer| answer.found)
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:1925: panic!("{name} is in the day");
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1986:20 False: if path.parent() == Some(year.as_path())
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1987:24 False: && path.extension().is_some_and(|ext| ext == "tmp")
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:1988:24 False: && source.kind() == std::io::ErrorKind::PermissionDenied
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:2256:94 False: TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { path, .. })) if *path == tix
  GAP (test) BRANCH crates/store/tests/cloud_mutants_store.rs:2349:20 False: if *path == bar_path && *offset == 32_768 + 56 * 4
  GAP (test) BRANCH crates/store/tests/cloud_mutants_store.rs:2349:41 False: if *path == bar_path && *offset == 32_768 + 56 * 4
  GAP (test) BRANCH crates/store/tests/cloud_mutants_store.rs:2384:13 False: let Some(bytes) = self.bytes.get(at..at + buf.len()) else {
  GAP (test) LINE   crates/store/tests/cloud_mutants_store.rs:2385: return Err(std::io::ErrorKind::UnexpectedEof.into());
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:2636:20 False: if *path == bar_path
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:2873:80 False: TimeLookup::Bisection(Why::SharedSlot { index: 10, ts_micros }) if ts_micros == second.ts_micros
  F?         LINE   crates/store/tests/cloud_mutants_store.rs:2876: writer.time_lookup()
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:2962:20 False: if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:2962:40 False: if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:3073:76 False: StoreError::NotADirectory { path, action: Action::Measure } if *path == bars.to_path_buf(&other)
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:3124:59 False: matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  GAP (test) BRANCH crates/store/tests/cloud_mutants_store.rs:3171:20 True: if since.elapsed() > std::time::Duration::from_secs(5) {
  GAP (test) LINE   crates/store/tests/cloud_mutants_store.rs:3172: return Err(std::io::Error::other("flushed alone"));
  GAP (test) LINE   crates/store/tests/cloud_mutants_store.rs:3253: _ => None,
  F?         BRANCH crates/store/tests/cloud_mutants_store.rs:3357:59 False: matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  F?         BRANCH crates/store/tests/docs.rs:217:9 False: BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  F?         BRANCH crates/store/tests/docs.rs:217:47 False: BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  GAP (test) BRANCH crates/store/tests/docs.rs:224:8 False: if !flat(INVARIANTS_DOC).contains("no bench in this repository times a syscall") {
  GAP (test) LINE   crates/store/tests/docs.rs:226: }
  GAP (test) LINE   crates/store/tests/docs.rs:227: let entry = DECISIONS_DOC
  GAP (test) LINE   crates/store/tests/docs.rs:228: .split("\n### D-0790 ")
  GAP (test) LINE   crates/store/tests/docs.rs:229: .nth(1)
  GAP (test) LINE   crates/store/tests/docs.rs:230: .and_then(|rest| rest.split("\n### ").next())
  GAP (test) LINE   crates/store/tests/docs.rs:231: .map(flat)
  GAP (test) LINE   crates/store/tests/docs.rs:232: .expect("docs/05-decisions.md has a D-0790 entry");
  F?         LINE   crates/store/tests/docs.rs:233: assert!(
  F?         BRANCH crates/store/tests/docs.rs:234:9 True: entry.contains(
  F?         BRANCH crates/store/tests/docs.rs:234:9 False: entry.contains(
  F?         LINE   crates/store/tests/docs.rs:234: entry.contains(
  F?         LINE   crates/store/tests/docs.rs:235: "The paragraph under C-01 in `docs/04-invariants.md` also says no bench times a syscall"
  F?         BRANCH crates/store/tests/docs.rs:236:14 True: ) && entry.contains("this entry supersedes that sentence"),
  F?         BRANCH crates/store/tests/docs.rs:236:14 False: ) && entry.contains("this entry supersedes that sentence"),
  F?         LINE   crates/store/tests/docs.rs:236: ) && entry.contains("this entry supersedes that sentence"),
  F?         LINE   crates/store/tests/docs.rs:239: assert!(
  F?         LINE   crates/store/tests/docs.rs:240: INVARIANTS_DOC.contains("\n| C4-DOCS-WEB-07 | While the C-01 paragraph"),
  F?         BRANCH crates/store/tests/docs.rs:253:9 False: FILE_RS.contains("if claimed > header.n_valid {")
  F?         BRANCH crates/store/tests/docs.rs:254:16 False: && FILE_RS.contains("source: FormatError::CounterExceedsFile,"),
  F?         LINE   crates/store/tests/durability.rs:381: panic!(
  F?         BRANCH crates/store/tests/fault.rs:143:36 False: read.n_valid == 150 || read.n_valid == 175,
  F?         BRANCH crates/store/tests/fault.rs:220:59 False: horizon == previous_commit.durable_through || horizon == commit.durable_through,
  F?         LINE   crates/store/tests/fault.rs:376: panic!("bit {bit} of byte {byte} took the whole file down: {e}")
  F?         LINE   crates/store/tests/fifo.rs:108: at.display()
  GAP (test) BRANCH crates/store/tests/fifo.rs:121:12 False: if let Ok(outcome) = recv.recv_timeout(BOUND) {
  GAP (test) LINE   crates/store/tests/fifo.rs:124: }
  GAP (test) LINE   crates/store/tests/fifo.rs:126: drop(fs::OpenOptions::new().write(true).open(release));
  GAP (test) LINE   crates/store/tests/fifo.rs:127: drop(worker.join());
  F?         LINE   crates/store/tests/fifo.rs:128: panic!(
  F?         LINE   crates/store/tests/fifo.rs:130: release.display()
  F?         BRANCH crates/store/tests/fifo.rs:157:13 False: rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  F?         BRANCH crates/store/tests/fifo.rs:157:61 False: rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  F?         BRANCH crates/store/tests/fifo.rs:162:53 False: fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),
  GAP (test) BRANCH crates/store/tests/fifo.rs:212:8 True: if !null.exists() {
  GAP (test) LINE   crates/store/tests/fifo.rs:213: return;
  F?         BRANCH crates/store/tests/generation.rs:153:9 False: let MonthRepair::Repaired {
  F?         LINE   crates/store/tests/generation.rs:165: panic!("{done:?}");
  F?         BRANCH crates/store/tests/generation.rs:179:57 False: assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  F?         BRANCH crates/store/tests/generation.rs:179:76 False: assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  F?         LINE   crates/store/tests/generation.rs:217: other => panic!("{other:?}"),
  F?         LINE   crates/store/tests/generation.rs:238: other => panic!("{other:?}"),
  F?         LINE   crates/store/tests/generation.rs:331: matches!(
  F?         BRANCH crates/store/tests/generation.rs:364:9 False: let MonthRepair::Repaired { to, skipped, .. } =
  F?         LINE   crates/store/tests/generation.rs:367: panic!("repaired");
  F?         BRANCH crates/store/tests/generation.rs:384:9 False: let MonthRepair::Repaired {
  F?         LINE   crates/store/tests/generation.rs:388: panic!("{second:?}");
  F?         LINE   crates/store/tests/generation.rs:475: other => panic!("{other:?}"),
  F?         LINE   crates/store/tests/generation.rs:517: other => panic!("{other:?}"),
  F?         BRANCH crates/store/tests/generation.rs:670:13 False: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  F?         BRANCH crates/store/tests/generation.rs:670:42 True: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  F?         BRANCH crates/store/tests/generation.rs:670:42 False: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  F?         BRANCH crates/store/tests/generation.rs:694:74 False: matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),
  F?         BRANCH crates/store/tests/generation.rs:749:9 False: let MonthRepair::Repaired {
  F?         LINE   crates/store/tests/generation.rs:758: panic!("{repaired:?}");
  F?         BRANCH crates/store/tests/geometry.rs:99:13 False: start >= block_start && end <= block_end,
  F?         BRANCH crates/store/tests/geometry.rs:99:37 False: start >= block_start && end <= block_end,
  F?         BRANCH crates/store/tests/geometry.rs:134:13 True: start < block_start || end > block_end
  GAP (test) LINE   crates/store/tests/libm_key.rs:205: matches!(
  F?         BRANCH crates/store/tests/libm_key.rs:428:9 False: decision < caveat && caveat < rejected,
  F?         BRANCH crates/store/tests/libm_key.rs:428:30 False: decision < caveat && caveat < rejected,
  F?         BRANCH crates/store/tests/libm_key.rs:1330:16 True: if mentions { "is not" } else { "is" }
  F?         BRANCH crates/store/tests/libm_key.rs:1330:16 False: if mentions { "is not" } else { "is" }
  F?         LINE   crates/store/tests/libm_key.rs:1330: if mentions { "is not" } else { "is" }
  GAP (test) BRANCH crates/store/tests/misfiled.rs:125:16 True: let y = if month <= 2 { year - 1 } else { year };
  GAP (test) BRANCH crates/store/tests/misfiled.rs:129:26 False: let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i32::from(day) - 1;
  F?         LINE   crates/store/tests/misfiled.rs:184: other => panic!("expected Misfiled, found {other:?}"),
  F?         BRANCH crates/store/tests/misfiled.rs:217:9 False: said.contains("2019-11-28-1250000-CE")
  F?         BRANCH crates/store/tests/misfiled.rs:218:16 False: && said.contains("its store path is not this month's")
  F?         BRANCH crates/store/tests/misfiled.rs:219:16 False: && said.contains("DB-B65"),
  F?         LINE   crates/store/tests/misfiled.rs:321: unborn.len()
  F?         LINE   crates/store/tests/month_repair.rs:156: other => panic!("not a repair: {other:?}"),
  F?         LINE   crates/store/tests/month_repair.rs:324: timeframe.as_str()
  F?         BRANCH crates/store/tests/month_repair.rs:606:13 False: assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  F?         BRANCH crates/store/tests/month_repair.rs:606:47 False: assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  GAP (test) LINE   crates/store/tests/month_repair.rs:637: calls += 1;
  GAP (test) LINE   crates/store/tests/month_repair.rs:638: Ok(())
  GAP (test) LINE   crates/store/tests/month_repair.rs:639: };
  F?         BRANCH crates/store/tests/month_repair.rs:739:83 False: matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),
  F?         BRANCH crates/store/tests/month_repair.rs:852:9 False: let Ok(MonthRepair::Repaired {
  F?         LINE   crates/store/tests/month_repair.rs:856: panic!("{outcome:?}");
  F?         BRANCH crates/store/tests/month_repair.rs:879:38 False: Served::Revision(pointer) if pointer.revision.ordinal() == 3
  F?         LINE   crates/store/tests/month_repair.rs:918: left.keys()
  F?         LINE   crates/store/tests/month_repair.rs:944: other => panic!("not a repair: {other:?}"),
  GAP (test) LINE   crates/store/tests/month_summary_log.rs:74: OwnedValue::Uint(n) => n.to_string(),
  GAP (test) LINE   crates/store/tests/month_summary_log.rs:75: other => format!("{other:?}"),
  GAP (test) BRANCH crates/store/tests/open_flags.rs:175:16 True: if names_the_x86_bit(line) {
  GAP (test) LINE   crates/store/tests/open_flags.rs:176: offenders.push(format!("{}:{}", path.display(), number + 1));
  F?         LINE   crates/store/tests/provenance.rs:66: other => panic!("expected a sidecar, found {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:161: Recorded::NothingAppended => panic!("expected a record"),
  F?         LINE   crates/store/tests/provenance.rs:451: other => panic!("expected NotTheNewestCommit, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:464: other => panic!("expected NotABarPath, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:476: other => panic!("expected WrongBarFile, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:531: other => panic!("expected OutOfOrder, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:639: other => panic!("expected Damaged, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:698: result => panic!("expected a {field} mismatch, got {result:?}"),
  F?         LINE   crates/store/tests/provenance.rs:746: other => panic!("expected PastTheBars, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:1200: other => panic!("expected {fault:?}, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:1308: other => panic!("expected TooLarge, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:1409: other => panic!("expected Record, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:1611: other => panic!("expected NotABarPath, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:1617: other => panic!("expected NotABarPath, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:1671: other => panic!("expected the directory sync refused, got {other:?}"),
  F?         LINE   crates/store/tests/provenance.rs:1721: other => panic!("expected the open refused, got {other:?}"),
  F?         BRANCH crates/store/tests/repair.rs:506:13 True: assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  F?         BRANCH crates/store/tests/repair.rs:506:41 False: assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:114:13 False: let (Err(FrameError::Truncated { part }), 0) = frame_answer else {
  F?         LINE   crates/store/tests/sealed_codec_alloc.rs:115: panic!("a frame cut to {len} bytes left {frame_answer:?}");
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:123:9 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:123:32 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  F?         LINE   crates/store/tests/sealed_codec_alloc.rs:125: full.len()
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:332:13 False: !text.contains("by its checked rows and no more")
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:333:20 False: && !text.contains("checked rows times 56 bytes, once"),
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:341:13 False: text.contains("the largest of twice") && text.contains("and 4"),
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:341:54 False: text.contains("the largest of twice") && text.contains("and 4"),
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:347:13 False: text.contains("capacity 0") && text.contains("cleared"),
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:347:44 False: text.contains("capacity 0") && text.contains("cleared"),
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:357:13 False: !text.contains("random fields cost at most 57 bytes a bar")
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:358:20 False: && !text.contains("at most 57 bytes a bar, 1 more than raw"),
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:387:9 False: SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  F?         BRANCH crates/store/tests/sealed_codec_alloc.rs:387:52 False: SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:65: fn read_month(path: &Path) -> Vec<Bar> {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:68: assert!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:69: path.extension().is_some_and(|e| e == "bin"),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:71: path.display()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:73: let data = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:74: let header = data.get(..RECORDS_AT).unwrap_or_else(|| {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:75: panic!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:77: path.display(),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:78: data.len()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:81: let header = Header::read_region(header, data.len() as u64)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:82: .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:83: let n = header.n_valid as usize;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:84: data.get(RECORDS_AT..RECORDS_AT + n * STRIDE)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:85: .unwrap_or_else(|| {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:86: panic!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:88: path.display()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:91: .chunks_exact(STRIDE)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:92: .map(|record| Bar::decode(record).expect("a committed record decodes"))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:93: .collect()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:94: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:97: fn origin_of(name: &str) -> i64 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:98: let stem = name.trim_end_matches(".bin");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:99: let month = &stem[stem.len() - 7..];
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:100: let year: u16 = month[..4].parse().expect("year");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:101: let mm: u8 = month[5..].parse().expect("month");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:102: YearMonth::new(year, mm)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:103: .expect("a month")
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:104: .ist_bounds_micros()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:105: .0
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:106: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:111: fn judge_row0(rows: &[Bar]) -> u64 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:112: fn len(mut v: u64) -> u64 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:113: let mut n = 1;
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:114:15 True: while v >= 0x80 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:114:15 False: while v >= 0x80 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:114: while v >= 0x80 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:115: v >>= 7;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:116: n += 1;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:117: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:118: n
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:119: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:120: fn zz(x: i64) -> u64 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:121: ((x << 1) ^ (x >> 63)).cast_unsigned()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:122: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:123: let first = rows.first().map_or(0, |b| b.ts_micros);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:124: rows.iter()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:125: .step_by(256)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:126: .map(|b| {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:127: let d = b.ts_micros - first;
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:128:25 True: let ts = if d % 1_000_000 == 0 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:128:25 False: let ts = if d % 1_000_000 == 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:128: let ts = if d % 1_000_000 == 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:129: len(zz(d / 1_000_000))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:131: len(zz(d))
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:133:25 True: let oi = if b.open_interest == OI_NULL {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:133:25 False: let oi = if b.open_interest == OI_NULL {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:133: let oi = if b.open_interest == OI_NULL {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:134: 1
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:136: 1 + len(zz(b.open_interest))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:138: ts + 1
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:139: + len(zz(b.close))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:140: + len(zz(b.open - b.close))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:141: + len(zz(b.high - b.close))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:142: + len(zz(b.low - b.close))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:143: + len(zz(b.volume))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:144: + oi
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:145: })
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:146: .sum()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:147: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:150: const fn zz(x: i64) -> u64 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:151: ((x << 1) ^ (x >> 63)).cast_unsigned()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:152: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:154: const fn varint_len(value: u64) -> usize {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:155: let bits = (64 - value.leading_zeros()) as usize;
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:156:8 True: if bits == 0 { 1 } else { bits.div_ceil(7) }
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:156:8 False: if bits == 0 { 1 } else { bits.div_ceil(7) }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:156: if bits == 0 { 1 } else { bits.div_ceil(7) }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:157: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:160: fn row0_len(first: &Bar, origin: i64) -> usize {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:161: let d = first.ts_micros.wrapping_sub(origin);
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:162:20 True: let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:162:20 False: let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:162: let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:163:17 True: let oi = if first.open_interest == OI_NULL {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:163:17 False: let oi = if first.open_interest == OI_NULL {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:163: let oi = if first.open_interest == OI_NULL {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:164: 0
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:166: varint_len(zz(first.open_interest))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:168: varint_len(zz(stamp))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:169: + varint_len(zz(first.close))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:170: + varint_len(zz(first.open.wrapping_sub(first.close)))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:171: + varint_len(zz(first.high.wrapping_sub(first.close)))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:172: + varint_len(zz(first.low.wrapping_sub(first.close)))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:173: + varint_len(zz(first.volume))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:174: + oi
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:175: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:179: fn residuals(rows: &[Bar], column: usize, mode: u8) -> Vec<i64> {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:180: rows.windows(2)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:181: .map(|pair| {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:182: let (p, b) = (pair[0], pair[1]);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:183: let (value, predictor) = match (column, mode) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:184: (0, _) => (b.ts_micros, p.ts_micros),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:185: (1, _) => (b.close, p.close),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:186: (2, 0) => (b.open, p.close),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:187: (2, _) => (b.open, b.close),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:188: (3, _) => (b.high, b.open.max(b.close)),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:189: (4, _) => (b.low, b.open.min(b.close)),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:190: (5, 0) => (b.volume, 0),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:191: (5, _) => (b.volume, p.volume),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:192: (6, 0) => (b.open_interest, p.open_interest),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:193: _ => (b.open_interest, 0),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:195: value.wrapping_sub(predictor)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:196: })
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:197: .collect()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:198: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:205: fn prototype_stream_len(residuals: &[i64]) -> usize {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:206: fn gcd(mut a: u64, mut b: u64) -> u64 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:207:15 True: while b != 0 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:207:15 False: while b != 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:207: while b != 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:208: (a, b) = (b, a % b);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:209: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:210: a
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:211: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:212: let n = residuals.len();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:213: let mut g = residuals.iter().fold(0, |g, r| gcd(g, r.unsigned_abs()));
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:214:8 True: if g == 0 || g >= 1 << 62 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:214:8 False: if g == 0 || g >= 1 << 62 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:214:18 True: if g == 0 || g >= 1 << 62 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:214:18 False: if g == 0 || g >= 1 << 62 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:214: if g == 0 || g >= 1 << 62 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:215: g = 1;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:216: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:217: let values: Vec<u64> = residuals.iter().map(|&r| zz(r / g.cast_signed())).collect();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:218: let base = values.iter().copied().min().unwrap_or(0);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:219: let mut histogram = [0usize; 65];
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:220: for v in &values {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:221: histogram[(64 - (v - base).leading_zeros()) as usize] += 1;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:222: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:223: let mut top = 64;
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:224:11 True: while top > 0 && histogram[top] == 0 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:224:11 False: while top > 0 && histogram[top] == 0 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:224:22 True: while top > 0 && histogram[top] == 0 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:224:22 False: while top > 0 && histogram[top] == 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:224: while top > 0 && histogram[top] == 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:225: top -= 1;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:226: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:227: let p = ((usize::BITS - n.saturating_sub(1).leading_zeros()) as usize).max(1);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:228: let (mut best, mut best_w, mut exceptions, mut w) = (n * top, top, 0, top);
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:229:11 True: while w > 0 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:229:11 False: while w > 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:229: while w > 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:230: exceptions += histogram[w];
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:231: w -= 1;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:232: let cost = n * w + exceptions * (p + top) + 16;
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:233:12 True: if cost < best {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:233:12 False: if cost < best {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:233: if cost < best {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:234: best = cost;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:235: best_w = w;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:236: }
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:238:20 True: let limit = if best_w == 64 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:238:20 False: let limit = if best_w == 64 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:238: let limit = if best_w == 64 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:239: u64::MAX
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:241: (1 << best_w) - 1
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:243: let exceptions = values.iter().filter(|&&v| v - base > limit).count();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:244: let mut len = 1
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:245: + varint_len(exceptions as u64)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:246: + varint_len(base)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:247: + varint_len(g)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:248: + (n * best_w).div_ceil(8);
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:249:8 True: if exceptions > 0 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:249:8 False: if exceptions > 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:249: if exceptions > 0 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:250: len += 1 + (exceptions * p).div_ceil(8) + (exceptions * top).div_ceil(8);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:251: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:252: len
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:253: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:256: fn prototype_frame_len(rows: &[Bar], origin: i64) -> usize {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:257: let streams: usize = (0..7)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:258: .map(|column| {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:259: let zero = prototype_stream_len(&residuals(rows, column, 0));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:260: if matches!(column, 2 | 5 | 6) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:261: zero.min(prototype_stream_len(&residuals(rows, column, 1)))
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:263: zero
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:265: })
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:266: .sum();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:267: 4 + row0_len(&rows[0], origin) + streams
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:268: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:281: fn seal(rows: &[Bar], origin: i64, label: &str) -> Sealed {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:282: let mut frames = Vec::with_capacity(rows.len().div_ceil(256));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:283: let (mut bytes, mut prototype_rule, mut first_rows) = (0, 0, 0);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:284: for chunk in rows.chunks(usize::from(BLOCK_ROWS)) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:285: let rule = prototype_frame_len(chunk, origin);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:286: first_rows += row0_len(&chunk[0], origin) as u64;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:287: let mut frame = Vec::new();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:288: let encoded = encode_frame(chunk, origin, BLOCK_ROWS, &mut frame).expect("encodes");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:289: assert!(frame.len() <= MAX_FRAME_LEN, "{label}");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:290: assert_eq!(encoded.crc, crc32c(&frame));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:291: let mut back = Vec::new();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:292: decode_frame(&frame, encoded.crc, origin, BLOCK_ROWS, &mut back).expect("decodes");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:293: assert_eq!(back, chunk, "{label}: a frame came back different");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:294: let mut stamps = Vec::new();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:295: decode_stamps(&frame, encoded.crc, origin, BLOCK_ROWS, &mut stamps).expect("stamps");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:296: assert!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:297: stamps.iter().zip(chunk).all(|(s, b)| *s == b.ts_micros),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:301: assert!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:302: frame.len() <= rule,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:304: frame.len()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:306: bytes += frame.len() as u64;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:307: prototype_rule += rule as u64;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:308: frames.push((frame, encoded.crc));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:310: Sealed {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:311: bytes,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:312: prototype_rule,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:313: row0: first_rows,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:314: frames,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:315: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:316: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:318: fn percentiles(label: &str, mut ns: Vec<u64>) -> String {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:319: ns.sort_unstable();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:320: let at = |q: usize| ns[((ns.len() - 1) * q + 50) / 100];
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:321: format!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:323: ns.len(),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:324: at(50),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:325: at(99),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:326: ns.last().copied().unwrap_or(0)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:328: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:331: fn time(rows: &[Bar], origin: i64, sealed: &Sealed, label: &str) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:332: let mut decode = Vec::new();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:333: let mut stamps = Vec::new();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:334: let mut out = Vec::with_capacity(512);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:335: let mut ts = Vec::with_capacity(256);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:336: for _ in 0..(2_000 / sealed.frames.len()).max(1) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:337: for (frame, crc) in &sealed.frames {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:338: out.clear();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:339: let t = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:340: decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:341: decode.push(t.elapsed().as_nanos() as u64);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:342: ts.clear();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:343: let t = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:344: decode_stamps(frame, *crc, origin, BLOCK_ROWS, &mut ts).expect("stamps");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:345: stamps.push(t.elapsed().as_nanos() as u64);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:346: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:348: let mut encode = Vec::new();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:349: let mut buffer = Vec::with_capacity(MAX_FRAME_LEN);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:350: for chunk in rows.chunks(256).take(2_000) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:351: buffer.clear();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:352: let t = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:353: encode_frame(chunk, origin, BLOCK_ROWS, &mut buffer).expect("encodes");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:354: encode.push(t.elapsed().as_nanos() as u64);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:355: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:357: let mut page = Vec::new();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:358: let mut state = 0x9E37_79B9_7F4A_7C15_u64;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:359: let n = rows.len() as u64;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:360: for _ in 0..2_000 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:361: state ^= state << 13;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:362: state ^= state >> 7;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:363: state ^= state << 17;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:364: let first = state % n;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:365: let last = (first + 99).min(n - 1);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:366: out.clear();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:367: let t = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:368: for k in (first / 256)..=(last / 256) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:369: let (frame, crc) = &sealed.frames[k as usize];
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:370: decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:371: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:372: page.push(t.elapsed().as_nanos() as u64);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:373: let skip = (first % 256) as usize;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:374: assert_eq!(out[skip], rows[first as usize]);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:376: println!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:378: percentiles(&format!("{label} frame decode+crc"), decode)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:380: println!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:382: percentiles(&format!("{label} frame stamps+crc"), stamps)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:384: println!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:386: percentiles(&format!("{label} frame encode"), encode)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:388: println!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:390: percentiles(&format!("{label} page100 from memory"), page)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:392: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:409: fn add(&mut self, label: &str, bars: usize, sealed: &Sealed, exact: u64, judge: u64) {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:410:12 True: if sealed.prototype_rule != exact {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:410:12 False: if sealed.prototype_rule != exact {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:410: if sealed.prototype_rule != exact {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:411: self.not_exact.push(format!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:412: "{label}: rule {} exact {exact}",
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:413: sealed.prototype_rule
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:414: ));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:415: }
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:416:12 True: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:416:12 False: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:416:29 True: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:416:29 False: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:416: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:417: self.larger
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:418: .push(format!("{label}: codec1 {} judge {judge}", sealed.bytes));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:419: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:420: self.codec1 += sealed.bytes;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:421: self.rule += sealed.prototype_rule;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:422: self.exact += exact;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:423: self.judge += judge;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:424: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:426: fn check(&self, label: &str) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:427: let d = |x: u64, of: u64| per_cent(x.cast_signed() - of.cast_signed(), of);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:428: println!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:434: d(self.codec1, self.exact),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:435: d(self.codec1, self.judge),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:436: self.not_exact.len(),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:437: self.larger.len()
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:440: assert!(self.not_exact.is_empty(), "{label}: {:?}", self.not_exact);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:441: assert_eq!(self.rule, self.exact, "{label}");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:443: assert!(self.codec1 <= self.rule, "{label}");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:444: assert!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:445: within(self.codec1, self.exact).0,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:450: assert!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:451: within(self.codec1, self.judge).0,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:454: assert!(self.larger.is_empty(), "{label}: {:?}", self.larger);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:455: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:459: fn per_cent(diff: i64, of: u64) -> String {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:460: let thousandths = diff.unsigned_abs() * 100_000 / of;
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:461:19 True: let sign = if diff < 0 { "-" } else { "+" };
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:461:19 False: let sign = if diff < 0 { "-" } else { "+" };
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:461: let sign = if diff < 0 { "-" } else { "+" };
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:462: format!("{sign}{}.{:03}%", thousandths / 1_000, thousandths % 1_000)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:463: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:466: fn within(mine: u64, expected: u64) -> (bool, i64) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:467: let diff = mine.cast_signed() - expected.cast_signed();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:469: (diff.unsigned_abs() * 200 <= expected, diff)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:470: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:474: fn the_sample_months_encode_to_the_prototype_sizes_less_the_first_row_saving() {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:475: let root = std::env::var("BRUTEX_SEAL_SAMPLES")
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:476: .expect("BRUTEX_SEAL_SAMPLES names the store-compression evidence directory");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:477: let root = Path::new(&root);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:488: println!("eight sample months:");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:489: let mut totals = Totals::default();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:490: for (dir, month, body, saved) in SAMPLES {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:491: let path = root
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:492: .join("blocks/samples")
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:493: .join(dir)
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:494: .join(format!("{month}.bin"));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:495: let rows = read_month(&path);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:496: let origin = origin_of(month);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:497: let sealed = seal(&rows, origin, dir);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:498: let blocks = sealed.frames.len() as u64;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:499: let exact = body - 56 * blocks + sealed.row0;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:500: let judge = body - saved;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:501: println!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:502: "{dir} {month}: bars={} frames={blocks} codec1={} rule={} exact={exact} judge={judge} codec1-judge={} raw={} ({} B/bar x100)",
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:503: rows.len(),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:504: sealed.bytes,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:505: sealed.prototype_rule,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:506: per_cent(sealed.bytes.cast_signed() - judge.cast_signed(), judge),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:507: rows.len() * 56,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:508: sealed.bytes * 100 / rows.len() as u64,
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:509: );
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:510: time(&rows, origin, &sealed, dir);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:511: totals.add(dir, rows.len(), &sealed, exact, judge);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:512: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:513: totals.check("eight samples in all");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:515: let manifest =
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:516: std::fs::read_to_string(root.join("blocks/proto/strat-manifest.tsv")).expect("manifest");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:517: let out =
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:518: std::fs::read_to_string(root.join("blocks/proto/strat-out.tsv")).expect("prototype output");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:519: let mut totals = Totals::default();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:520: let (mut months, mut bars, mut empty) = (0, 0, 0);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:521: for (line, result) in manifest.lines().zip(out.lines()) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:522: let fields: Vec<&str> = line.split('\t').collect();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:523: let measured: Vec<&str> = result.split('\t').collect();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:524: let rows = read_month(&root.join("blocks").join(fields[0]));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:525: assert_eq!(rows.len().to_string(), measured[2], "{}", fields[4]);
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:526:12 True: if rows.is_empty() {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:526:12 False: if rows.is_empty() {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:526: if rows.is_empty() {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:527: empty += 1;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:528: continue;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:529: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:530: let origin = origin_of(fields[4]);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:531: let sealed = seal(&rows, origin, fields[4]);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:532: let body: u64 = measured[3].parse().expect("body");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:533: let blocks = sealed.frames.len() as u64;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:534: let exact = body - 56 * blocks + sealed.row0;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:535: let judge = body - (56 * blocks - judge_row0(&rows));
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:536: totals.add(fields[4], rows.len(), &sealed, exact, judge);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:537: months += 1;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:538: bars += rows.len();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:540: println!("539-month manifest: {months} months sealed, {empty} empty, {bars} bars");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:541: totals.check("the manifest in all");
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:542:12 True: if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:542:12 False: if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:542: if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:543: println!("extra months (read only, round trip and size):");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:544: for path in extra.lines().filter(|l| !l.trim().is_empty()) {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:545: let path = Path::new(path.trim());
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:546: let rows = read_month(path);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:547: let name = path.to_string_lossy();
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:548: let origin = origin_of(&name);
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:549:16 True: if rows.is_empty() {
  IGNORED    BRANCH crates/store/tests/sealed_codec_samples.rs:549:16 False: if rows.is_empty() {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:549: if rows.is_empty() {
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:550: println!("{name}: empty month");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:551: continue;
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:552: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:553: let sealed = seal(&rows, origin, &name);
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:554: println!(
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:556: rows.len(),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:557: sealed.frames.len(),
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:559: rows.len() * 56
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:561: time(&rows, origin, &sealed, "extra");
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:563: }
  IGNORED    LINE   crates/store/tests/sealed_codec_samples.rs:564: }
  F?         BRANCH crates/store/tests/sealed_door.rs:153:17 True: assert!(at == rows || bars[at as usize].ts_micros >= probe);
  F?         BRANCH crates/store/tests/sealed_door.rs:153:31 False: assert!(at == rows || bars[at as usize].ts_micros >= probe);
  F?         BRANCH crates/store/tests/sealed_door.rs:231:9 False: large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  F?         BRANCH crates/store/tests/sealed_door.rs:231:45 False: large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  IGNORED    LINE   crates/store/tests/sealed_door.rs:237: fn store_path(rel: &str) -> StorePath<'static> {
  IGNORED    LINE   crates/store/tests/sealed_door.rs:238: let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
  IGNORED    LINE   crates/store/tests/sealed_door.rs:239: let parts: Vec<&'static str> = rel.split('/').collect();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:240: assert!(
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:241:9 True: parts.len() == 7 || parts.len() == 8,
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:241:9 False: parts.len() == 7 || parts.len() == 8,
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:241:29 True: parts.len() == 7 || parts.len() == 8,
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:241:29 False: parts.len() == 7 || parts.len() == 8,
  IGNORED    LINE   crates/store/tests/sealed_door.rs:241: parts.len() == 7 || parts.len() == 8,
  IGNORED    LINE   crates/store/tests/sealed_door.rs:244: assert_eq!(parts[0], "bars", "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_door.rs:245: let vendor = Vendor::ALL
  IGNORED    LINE   crates/store/tests/sealed_door.rs:246: .into_iter()
  IGNORED    LINE   crates/store/tests/sealed_door.rs:247: .find(|v| v.as_str() == parts[1])
  IGNORED    LINE   crates/store/tests/sealed_door.rs:248: .unwrap_or_else(|| panic!("{rel}: vendor"));
  IGNORED    LINE   crates/store/tests/sealed_door.rs:249: let contract = (parts.len() == 8)
  IGNORED    LINE   crates/store/tests/sealed_door.rs:250: .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
  IGNORED    LINE   crates/store/tests/sealed_door.rs:251: let tf = parts[parts.len() - 2];
  IGNORED    LINE   crates/store/tests/sealed_door.rs:252: let timeframe = *Timeframe::KNOWN
  IGNORED    LINE   crates/store/tests/sealed_door.rs:253: .iter()
  IGNORED    LINE   crates/store/tests/sealed_door.rs:254: .find(|t| t.as_str() == tf)
  IGNORED    LINE   crates/store/tests/sealed_door.rs:255: .unwrap_or_else(|| panic!("{rel}: rung"));
  IGNORED    LINE   crates/store/tests/sealed_door.rs:256: let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");
  IGNORED    LINE   crates/store/tests/sealed_door.rs:257: let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:258: StorePath::new(PathParts {
  IGNORED    LINE   crates/store/tests/sealed_door.rs:259: vendor,
  IGNORED    LINE   crates/store/tests/sealed_door.rs:260: exchange: parts[2],
  IGNORED    LINE   crates/store/tests/sealed_door.rs:261: segment: parts[3],
  IGNORED    LINE   crates/store/tests/sealed_door.rs:262: symbol: parts[4],
  IGNORED    LINE   crates/store/tests/sealed_door.rs:263: contract,
  IGNORED    LINE   crates/store/tests/sealed_door.rs:264: timeframe,
  IGNORED    LINE   crates/store/tests/sealed_door.rs:265: month,
  IGNORED    LINE   crates/store/tests/sealed_door.rs:266: file: FileKind::Bars,
  IGNORED    LINE   crates/store/tests/sealed_door.rs:267: })
  IGNORED    LINE   crates/store/tests/sealed_door.rs:268: .unwrap()
  IGNORED    LINE   crates/store/tests/sealed_door.rs:269: }
  IGNORED    LINE   crates/store/tests/sealed_door.rs:280: fn seal_a_private_store_root_for_the_shadow_check() {
  IGNORED    LINE   crates/store/tests/sealed_door.rs:281: let root = PathBuf::from(std::env::var("BRUTEX_SC3_ROOT").expect("BRUTEX_SC3_ROOT"));
  IGNORED    LINE   crates/store/tests/sealed_door.rs:282: let list = std::env::var("BRUTEX_SC3_MONTHS").expect("BRUTEX_SC3_MONTHS");
  IGNORED    LINE   crates/store/tests/sealed_door.rs:283: let bars_dir = root.join("bars");
  IGNORED    LINE   crates/store/tests/sealed_door.rs:284: assert!(
  IGNORED    LINE   crates/store/tests/sealed_door.rs:285: !fs::symlink_metadata(&bars_dir)
  IGNORED    LINE   crates/store/tests/sealed_door.rs:286: .unwrap()
  IGNORED    LINE   crates/store/tests/sealed_door.rs:287: .file_type()
  IGNORED    LINE   crates/store/tests/sealed_door.rs:288: .is_symlink(),
  IGNORED    LINE   crates/store/tests/sealed_door.rs:290: bars_dir.display()
  IGNORED    LINE   crates/store/tests/sealed_door.rs:292: assert!(
  IGNORED    LINE   crates/store/tests/sealed_door.rs:293: !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
  IGNORED    LINE   crates/store/tests/sealed_door.rs:296: let (mut months, mut rows, mut raw_bytes, mut sealed_bytes) = (0u64, 0u64, 0u64, 0u64);
  IGNORED    LINE   crates/store/tests/sealed_door.rs:297: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:298: for rel in fs::read_to_string(&list).unwrap().lines().map(str::trim) {
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:299:12 True: if rel.is_empty() || rel.starts_with('#') {
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:299:12 False: if rel.is_empty() || rel.starts_with('#') {
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:299:30 True: if rel.is_empty() || rel.starts_with('#') {
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:299:30 False: if rel.is_empty() || rel.starts_with('#') {
  IGNORED    LINE   crates/store/tests/sealed_door.rs:299: if rel.is_empty() || rel.starts_with('#') {
  IGNORED    LINE   crates/store/tests/sealed_door.rs:300: continue;
  IGNORED    LINE   crates/store/tests/sealed_door.rs:301: }
  IGNORED    LINE   crates/store/tests/sealed_door.rs:302: let at = store_path(rel);
  IGNORED    LINE   crates/store/tests/sealed_door.rs:303: let symbol = rel.split('/').nth(4).unwrap().to_owned();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:304: let symbol_id = brutex_core::universe::fnv1a(&symbol) as u32;
  IGNORED    LINE   crates/store/tests/sealed_door.rs:305: let served = store::generation::served(&root, at).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:306: let served_root = served.root(&root);
  IGNORED    LINE   crates/store/tests/sealed_door.rs:307: let sbar = at.with_file(FileKind::SealedBars).to_path_buf(&served_root);
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:308:12 True: if sbar.exists() {
  IGNORED    BRANCH crates/store/tests/sealed_door.rs:308:12 False: if sbar.exists() {
  IGNORED    LINE   crates/store/tests/sealed_door.rs:308: if sbar.exists() {
  IGNORED    LINE   crates/store/tests/sealed_door.rs:309: println!("SKIP {rel}: already sealed");
  IGNORED    LINE   crates/store/tests/sealed_door.rs:310: continue;
  IGNORED    LINE   crates/store/tests/sealed_door.rs:311: }
  IGNORED    LINE   crates/store/tests/sealed_door.rs:312: let raw = BarFile::open_existing(&root, at, symbol_id).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:313: assert_eq!(raw.form(), Form::Raw, "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_door.rs:314: let bars = raw.read_range(0, raw.records()).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:315: let prv = fs::read(at.with_file(FileKind::Provenance).to_path_buf(&served_root)).ok();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:316: let built = store::sealed::build(at, &raw.header(), &bars, prv.as_deref(), None).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:317: let staged = sbar.with_extension("sbar-staged");
  IGNORED    LINE   crates/store/tests/sealed_door.rs:318: let mut file = fs::File::create_new(&staged).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:319: built.write_to(&mut file).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:320: file.sync_all().unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:321: drop(file);
  IGNORED    LINE   crates/store/tests/sealed_door.rs:322: fs::rename(&staged, &sbar).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:323: let sealed = BarFile::open_existing(&root, at, symbol_id).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:324: assert_eq!(sealed.form(), Form::Sealed, "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_door.rs:325: assert_eq!(
  IGNORED    LINE   crates/store/tests/sealed_door.rs:326: sealed.read_range(0, sealed.records()).unwrap(),
  IGNORED    LINE   crates/store/tests/sealed_door.rs:330: months += 1;
  IGNORED    LINE   crates/store/tests/sealed_door.rs:331: rows += bars.len() as u64;
  IGNORED    LINE   crates/store/tests/sealed_door.rs:332: raw_bytes += fs::metadata(at.to_path_buf(&served_root)).unwrap().len();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:333: sealed_bytes += built.file_len();
  IGNORED    LINE   crates/store/tests/sealed_door.rs:334: println!(
  IGNORED    LINE   crates/store/tests/sealed_door.rs:336: bars.len(),
  IGNORED    LINE   crates/store/tests/sealed_door.rs:337: built.file_len(),
  IGNORED    LINE   crates/store/tests/sealed_door.rs:338: served_root.display()
  IGNORED    LINE   crates/store/tests/sealed_door.rs:341: println!(
  IGNORED    LINE   crates/store/tests/sealed_door.rs:344: started.elapsed().as_secs_f64()
  IGNORED    LINE   crates/store/tests/sealed_door.rs:346: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:49: fn next(&mut self) -> u64 {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:50: self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:51: let mut z = self.0;
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:52: z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:53: z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:54: z ^ (z >> 31)
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:55: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:56: fn below(&mut self, n: u64) -> u64 {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:57: self.next() % n
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:58: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:62: fn store_path(rel: &str) -> StorePath<'static> {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:63: let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:64: let parts: Vec<&'static str> = rel.split('/').collect();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:65: assert!(
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:66:9 True: parts.len() == 7 || parts.len() == 8,
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:66:9 False: parts.len() == 7 || parts.len() == 8,
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:66:29 True: parts.len() == 7 || parts.len() == 8,
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:66:29 False: parts.len() == 7 || parts.len() == 8,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:66: parts.len() == 7 || parts.len() == 8,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:69: assert_eq!(parts[0], "bars", "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:70: let vendor = Vendor::ALL
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:71: .into_iter()
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:72: .find(|v| v.as_str() == parts[1])
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:73: .unwrap_or_else(|| panic!("{rel}: vendor"));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:74: let contract = (parts.len() == 8).then(|| Contract::parse(parts[5]).expect("contract"));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:75: let tf = parts[parts.len() - 2];
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:76: let timeframe = *Timeframe::KNOWN
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:77: .iter()
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:78: .find(|t| t.as_str() == tf)
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:79: .unwrap_or_else(|| panic!("{rel}: rung"));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:80: let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:81: let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:82: StorePath::new(PathParts {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:83: vendor,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:84: exchange: parts[2],
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:85: segment: parts[3],
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:86: symbol: parts[4],
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:87: contract,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:88: timeframe,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:89: month,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:90: file: FileKind::Bars,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:91: })
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:92: .unwrap()
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:93: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:95: fn raw_month(file: &Path) -> (Header, Vec<Bar>) {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:96: let data = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:97: let at = HEADER_LEN as usize;
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:98: let header = Header::read_region(&data[..at], data.len() as u64).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:99: let n = header.n_valid as usize;
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:100: let bars = data[at..at + n * RECORD_LEN]
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:101: .chunks_exact(RECORD_LEN)
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:102: .map(|record| Bar::decode(record).unwrap())
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:103: .collect();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:104: (header, bars)
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:105: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:108: fn us(ns: u128) -> String {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:109: format!("{}.{}", ns / 1_000, ns % 1_000 / 100)
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:110: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:112: fn percentiles(label: &str, mut ns: Vec<u128>) -> String {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:113: ns.sort_unstable();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:114: let n = ns.len();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:115: format!(
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:117: us(ns[n / 2]),
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:118: us(ns[n * 99 / 100]),
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:119: us(ns[n - 1])
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:121: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:125: fn real_months_seal_read_back_bit_for_bit_and_answer_like_the_raw_form() {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:126: let root = PathBuf::from(std::env::var("BRUTEX_SEALED_ROOT").expect("BRUTEX_SEALED_ROOT"));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:127: let list = std::env::var("BRUTEX_SEALED_MONTHS").expect("BRUTEX_SEALED_MONTHS");
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:128: let out = PathBuf::from(std::env::var("BRUTEX_SEALED_OUT").expect("BRUTEX_SEALED_OUT"));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:129: let months: Vec<String> = std::fs::read_to_string(&list)
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:130: .unwrap()
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:131: .lines()
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:132: .map(str::trim)
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:133:24 True: .filter(|line| !line.is_empty() && !line.starts_with('#'))
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:133:24 False: .filter(|line| !line.is_empty() && !line.starts_with('#'))
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:133: .filter(|line| !line.is_empty() && !line.starts_with('#'))
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:134: .map(str::to_owned)
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:135: .collect();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:136: assert!(!months.is_empty(), "{list}: no months");
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:137: let mut rng = Rng(2026);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:138: for rel in &months {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:139: let path = store_path(rel);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:140: let bin = path.to_path_buf(&root);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:141: let (header, bars) = raw_month(&bin);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:142: let prv = std::fs::read(path.with_file(FileKind::Provenance).to_path_buf(&root)).ok();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:143: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:144: let built = build(path, &header, &bars, prv.as_deref(), None).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:145: let seal_us = started.elapsed().as_micros();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:146: let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&out);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:147: std::fs::create_dir_all(sbar.parent().unwrap()).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:148: let mut file = std::fs::File::create(&sbar).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:149: built.write_to(&mut file).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:150: file.sync_all().unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:151: drop(file);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:153: let symbol = header.symbol_id;
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:154: let sealed = SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:155: let audited = sealed.audit().unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:156: assert_eq!(audited.rows, bars.len() as u64, "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:157: assert_eq!(sealed.head().bar_header(), header, "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:158: assert_eq!(sealed.provenance().unwrap(), prv, "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:159: let raw = BarFile::open_existing(&root, path, symbol).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:160: let n = bars.len() as u64;
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:161: let mut page = Vec::new();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:162: let mut first = 0;
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:163:15 True: while first < n {
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:163:15 False: while first < n {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:163: while first < n {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:164: let count = (n - first).min(1_000);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:165: page.clear();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:166: sealed.read_rows(first, count, &mut page).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:167: assert_eq!(
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:169: bars[first as usize..(first + count) as usize],
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:172: first += count;
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:174: for at in (0..n).step_by(1 + n as usize / 500) {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:175: assert_eq!(
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:176: raw.read_record(at).unwrap(),
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:177: bars[at as usize],
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:182: let (mut open_ns, mut page_sealed, mut page_raw) = (Vec::new(), Vec::new(), Vec::new());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:183: let (mut row_sealed, mut row_raw, mut find_sealed, mut find_raw) =
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:184: (Vec::new(), Vec::new(), Vec::new(), Vec::new());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:185: let first_ts = bars.first().map_or(0, |bar| bar.ts_micros);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:186: let last_ts = bars.last().map_or(0, |bar| bar.ts_micros);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:187: for _ in 0..2_000 {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:188: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:189: let month =
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:190: SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:191: open_ns.push(started.elapsed().as_nanos());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:192: drop(month);
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:193:16 True: if n == 0 {
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:193:16 False: if n == 0 {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:193: if n == 0 {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:194: continue;
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:195: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:196: let count = n.min(100);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:197: let at = rng.below(n - count + 1);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:198: page.clear();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:199: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:200: sealed.read_rows(at, count, &mut page).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:201: page_sealed.push(started.elapsed().as_nanos());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:202: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:203: for i in at..at + count {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:204: let _ = raw.read_record(i).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:205: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:206: page_raw.push(started.elapsed().as_nanos());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:207: assert_eq!(page, bars[at as usize..(at + count) as usize]);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:209: let at = rng.below(n);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:210: page.clear();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:211: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:212: sealed.read_rows(at, 1, &mut page).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:213: row_sealed.push(started.elapsed().as_nanos());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:214: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:215: let one = raw.read_record(at).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:216: row_raw.push(started.elapsed().as_nanos());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:217: assert_eq!(page, [one]);
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:219: let ts = first_ts + rng.below((last_ts - first_ts) as u64 + 1) as i64;
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:220: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:221: let found = sealed.first_at_or_after(ts).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:222: find_sealed.push(started.elapsed().as_nanos());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:223: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:224: let raw_found = raw.first_at_or_after(ts).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:225: find_raw.push(started.elapsed().as_nanos());
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:226: let scanned = bars.partition_point(|bar| bar.ts_micros < ts) as u64;
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:227: assert_eq!(found, scanned, "{rel} ts {ts}");
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:228: assert_eq!(raw_found, scanned, "{rel} raw ts {ts}");
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:230: let raw_len = std::fs::metadata(&bin).unwrap().len();
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:231: println!(
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:233: sealed.head().n_blocks,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:234: sealed.head().table_entries,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:235: built.file_len(),
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:236: built.file_len() * 100 / n.max(1),
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:237: built.file_len() * 1_000 / raw_len,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:238: sealed.head().prv_raw_len,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:239: sealed.head().prv_packed_len,
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:241: println!("  {}", percentiles("open sealed (file + head)", open_ns));
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:242:12 True: if n > 0 {
  IGNORED    BRANCH crates/store/tests/sealed_samples.rs:242:12 False: if n > 0 {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:242: if n > 0 {
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:243: println!("  {}", percentiles("100-row page sealed", page_sealed));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:244: println!("  {}", percentiles("100-row page raw   ", page_raw));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:245: println!("  {}", percentiles("one row sealed     ", row_sealed));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:246: println!("  {}", percentiles("one row raw        ", row_raw));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:247: println!("  {}", percentiles("first_at_or_after sealed", find_sealed));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:248: println!("  {}", percentiles("first_at_or_after raw   ", find_raw));
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:249: }
  IGNORED    LINE   crates/store/tests/sealed_samples.rs:251: }
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:44: fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:45:16 True: let y = if m <= 2 { y - 1 } else { y };
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:45:16 False: let y = if m <= 2 { y - 1 } else { y };
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:45: let y = if m <= 2 { y - 1 } else { y };
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:46: let era = y.div_euclid(400);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:47: let yoe = y - era * 400;
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:48: let mp = (m + 9) % 12;
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:49: let doy = (153 * mp + 2) / 5 + d - 1;
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:50: let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:51: era * 146_097 + doe - 719_468
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:52: }
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:54: fn ist_day(bar: &Bar) -> i64 {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:55: (bar.ts_micros / 1_000_000 + IST).div_euclid(86_400)
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:56: }
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:58: fn store_path(rel: &str) -> (StorePath<'static>, &'static str) {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:59: let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:60: let parts: Vec<&'static str> = rel.split('/').collect();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:61: assert!(
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:62:9 True: parts.len() == 7 || parts.len() == 8,
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:62:9 False: parts.len() == 7 || parts.len() == 8,
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:62:29 True: parts.len() == 7 || parts.len() == 8,
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:62:29 False: parts.len() == 7 || parts.len() == 8,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:62: parts.len() == 7 || parts.len() == 8,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:65: let vendor = Vendor::ALL
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:66: .into_iter()
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:67: .find(|v| v.as_str() == parts[1])
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:68: .unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:69: let contract = (parts.len() == 8)
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:70: .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:71: let tf = parts[parts.len() - 2];
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:72: let timeframe = *Timeframe::KNOWN.iter().find(|t| t.as_str() == tf).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:73: let stem = parts[parts.len() - 1].strip_suffix(".bin").unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:74: let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:75: let path = StorePath::new(PathParts {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:76: vendor,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:77: exchange: parts[2],
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:78: segment: parts[3],
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:79: symbol: parts[4],
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:80: contract,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:81: timeframe,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:82: month,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:83: file: FileKind::Bars,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:84: })
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:85: .unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:86: (path, parts[4])
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:87: }
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:89: fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:90: let mut files = BTreeMap::new();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:91: let mut stack = vec![dir.to_path_buf()];
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:92:15 True: while let Some(at) = stack.pop() {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:92:15 False: while let Some(at) = stack.pop() {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:92: while let Some(at) = stack.pop() {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:93:13 True: let Ok(entries) = fs::read_dir(&at) else {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:93:13 False: let Ok(entries) = fs::read_dir(&at) else {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:93: let Ok(entries) = fs::read_dir(&at) else {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:94: continue;
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:96: for entry in entries {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:97: let path = entry.unwrap().path();
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:98:16 True: if path.is_dir() {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:98:16 False: if path.is_dir() {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:98: if path.is_dir() {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:99: stack.push(path);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:100: } else {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:101: files.insert(path.clone(), fs::read(&path).unwrap());
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:102: }
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:105: files
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:106: }
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:108: fn micros(started: Instant) -> f64 {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:109: started.elapsed().as_secs_f64() * 1e6
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:110: }
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:117: fn the_write_door_over_real_months_sealed_without_one_day() {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:118: let source = PathBuf::from(std::env::var("BRUTEX_SC4_SOURCE").expect("BRUTEX_SC4_SOURCE"));
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:119: let work = PathBuf::from(std::env::var("BRUTEX_SC4_WORK").expect("BRUTEX_SC4_WORK"));
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:120: let list = std::env::var("BRUTEX_SC4_MONTHS").expect("BRUTEX_SC4_MONTHS");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:121: for root in [&source, &work] {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:122: assert!(
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:123: !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:126: assert!(
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:127: !fs::symlink_metadata(root.join("bars"))
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:128: .is_ok_and(|meta| meta.file_type().is_symlink()),
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:130: root.display()
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:133: for (line_no, line) in fs::read_to_string(&list).unwrap().lines().enumerate() {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:134: let line = line.trim();
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:135:12 True: if line.is_empty() || line.starts_with('#') {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:135:12 False: if line.is_empty() || line.starts_with('#') {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:135:31 True: if line.is_empty() || line.starts_with('#') {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:135:31 False: if line.is_empty() || line.starts_with('#') {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:135: if line.is_empty() || line.starts_with('#') {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:136: continue;
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:137: }
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:138: let (rel, date) = line.split_once(' ').expect("<path> <yyyy-mm-dd>");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:139: let (path, symbol) = store_path(rel);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:140: let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:141: let full = {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:142: let file = BarFile::open_existing(&source, path, symbol_id).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:143: file.read_range(0, file.records()).unwrap()
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:146:22 True: let day = if date == "last" {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:146:22 False: let day = if date == "last" {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:146: let day = if date == "last" {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:147: ist_day(full.last().expect("a month with bars"))
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:149: let ymd: Vec<i64> = date.split('-').map(|p| p.parse().unwrap()).collect();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:150: days_from_civil(ymd[0], ymd[1], ymd[2])
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:152: let (held, offered): (Vec<Bar>, Vec<Bar>) =
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:153: full.iter().copied().partition(|bar| ist_day(bar) != day);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:154: assert!(!offered.is_empty(), "{rel}: no bar on {date}");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:155: assert!(!held.is_empty(), "{rel}: nothing but {date}");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:156: let root = work.join(format!("m{line_no}"));
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:157: fs::create_dir_all(&root).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:159: let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:160: for chunk in held.chunks(65_536) {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:161: writer.append(chunk).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:162: }
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:164: let raw = BarFile::open_existing(&root, path, symbol_id).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:165: let built = store::sealed::build(path, &raw.header(), &held, None, None).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:166: drop(raw);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:167: let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&root);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:168: let staged = sbar.with_extension("sbar-staged");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:169: fs::write(&staged, built.to_bytes()).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:170: fs::rename(&staged, &sbar).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:171: let before = snapshot(&root.join("bars"));
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:173: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:174: let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:175: let open_us = micros(started);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:176: assert_eq!(writer.form(), Form::Sealed, "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:177: let first_day = ist_day(&held[0]);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:178: let reoffer: Vec<Bar> = held
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:179: .iter()
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:180: .copied()
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:181: .filter(|bar| ist_day(bar) == first_day)
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:182: .collect();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:183: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:184: let again = writer.append(&reoffer).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:185: let reoffer_us = micros(started);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:186: assert_eq!(
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:188: Appended::AlreadyPresent {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:189: first_index: 0,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:190: n_valid: held.len() as u64
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:191: },
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:194: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:195: let refused = writer.append(&offered).unwrap_err();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:196: let refused_us = micros(started);
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:197:13 True: let StoreError::Sealed { offer, at, .. } = refused else {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:197:13 False: let StoreError::Sealed { offer, at, .. } = refused else {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:197: let StoreError::Sealed { offer, at, .. } = refused else {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:198: panic!("{rel}: {refused}");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:200: let is_last_day = day == ist_day(full.last().unwrap());
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:201: assert_eq!(
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:203:16 True: if is_last_day {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:203:16 False: if is_last_day {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:203: if is_last_day {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:204: SealedOffer::Follows
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:206: SealedOffer::Disagrees(store::file::Conflict::NotHeld)
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:210: assert_eq!(at, 0, "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:211: drop(writer);
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:212: assert_eq!(
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:213: snapshot(&root.join("bars")),
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:218: let started = Instant::now();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:219: let repaired = store::repair::repair_month(
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:220: &root,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:221: path,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:222: symbol_id,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:223: &offered,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:224: Policy {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:225: // No offered bar shares a stamp with a held one: the day is
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:226: // wholly absent, so the comparison this sets is never made.
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:227: open_interest_applies: true,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:228: basis_witness: false,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:229: },
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:231: .unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:232: let repair_ms = micros(started) / 1e3;
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:233:13 True: let MonthRepair::Repaired { added, .. } = repaired else {
  IGNORED    BRANCH crates/store/tests/sealed_write_door.rs:233:13 False: let MonthRepair::Repaired { added, .. } = repaired else {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:233: let MonthRepair::Repaired { added, .. } = repaired else {
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:234: panic!("{rel}: {repaired:?}");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:236: assert_eq!(added, offered.len() as u64, "{rel}");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:237: let served = BarFile::open_existing(&root, path, symbol_id).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:238: assert_eq!(served.form(), Form::Raw, "{rel}: the successor is raw");
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:239: let now = served.read_range(0, served.records()).unwrap();
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:240: assert!(
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:241: now == full,
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:244: assert_eq!(
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:245: snapshot(&root.join("bars")),
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:249: println!(
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:253: full.len(),
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:254: held.len(),
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:255: offered.len(),
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:256: offer.as_str(),
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:257: built.file_len()
  IGNORED    LINE   crates/store/tests/sealed_write_door.rs:260: }
  F?         LINE   crates/store/tests/slot_audit.rs:116: matches!(
  F?         BRANCH crates/store/tests/slot_audit.rs:130:13 False: let MonthVerdict::Reverted {
  F?         LINE   crates/store/tests/slot_audit.rs:137: panic!("v{version}: a damaged newest slot is a revert: {audit:?}");
  F?         LINE   crates/store/tests/slot_audit.rs:277: matches!(
  F?         LINE   crates/store/tests/slot_audit.rs:347: matches!(
  F?         LINE   crates/store/tests/slot_audit.rs:410: matches!(
  F?         BRANCH crates/store/tests/slot_audit.rs:484:9 False: let SlotState::Damaged { reason, .. } = audit.slots[1] else {
  F?         LINE   crates/store/tests/slot_audit.rs:485: panic!("damaged");
  F?         LINE   crates/store/tests/slot_audit_sealed.rs:95: raw.slot_audit()
  F?         LINE   crates/store/tests/slot_audit_sealed.rs:149: Ok(file) => {
  F?         LINE   crates/store/tests/slot_audit_sealed.rs:150: assert_eq!(file.form(), Form::Sealed, "byte {at}");
  F?         LINE   crates/store/tests/slot_audit_sealed.rs:151: assert_eq!(file.slot_audit(), None, "byte {at}");
  F?         LINE   crates/store/tests/slot_audit_sealed.rs:152: assert_eq!(file.header().n_valid, 40, "byte {at}: never read short");
  F?         BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 True: if let Ok(last) = file.read_record(39) {
  F?         BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 False: if let Ok(last) = file.read_record(39) {
  F?         LINE   crates/store/tests/slot_audit_sealed.rs:153: if let Ok(last) = file.read_record(39) {
  F?         LINE   crates/store/tests/slot_audit_sealed.rs:154: assert_eq!(last.close, 7_039, "byte {at}");
  GAP (test) LINE   crates/store/tests/slot_audit_sealed.rs:155: }
  GAP (test) LINE   crates/store/tests/slot_damaged_log.rs:93: Some(OwnedValue::Uint(value)) => *value,
  F?         LINE   crates/store/tests/slot_damaged_log.rs:94: other => panic!("{key}: {other:?}"),
  F?         LINE   crates/store/tests/slot_damaged_log.rs:101: other => panic!("{key}: {other:?}"),
  F?         LINE   crates/store/tests/slot_damaged_log.rs:192: found
  F?         LINE   crates/store/tests/slot_damaged_log.rs:193: .records
  F?         LINE   crates/store/tests/slot_damaged_log.rs:194: .iter()
  F?         LINE   crates/store/tests/slot_damaged_log.rs:195: .map(|r| (r.target.clone(), r.message.clone()))
  F?         LINE   crates/store/tests/slot_damaged_log.rs:196: .collect::<Vec<_>>()
  F?         BRANCH crates/store/tests/sort_index.rs:387:62 False: matches!(&again.outcome, Resorted::Built(why) if why.contains("fails the checksum")),
  F?         BRANCH crates/store/tests/sort_index.rs:430:56 False: matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),
  F?         BRANCH crates/store/tests/sort_index.rs:446:63 False: assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));
  F?         LINE   crates/store/tests/sort_index.rs:488: index.torn()
  F?         BRANCH crates/store/tests/sort_index.rs:660:58 False: matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  F?         BRANCH crates/store/tests/sort_index.rs:736:58 False: matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  F?         LINE   crates/store/tests/sort_index.rs:761: column.as_str()
  F?         LINE   crates/store/tests/sort_index.rs:843: column.as_str()
  F?         BRANCH crates/store/tests/sort_index.rs:874:16 False: if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  F?         BRANCH crates/store/tests/sort_index.rs:874:54 False: if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  F?         BRANCH crates/store/tests/sort_index.rs:889:16 False: if reason.contains("2031-02.new.srt")),
  GAP (test) LINE   crates/store/tests/sort_index_log.rs:74: OwnedValue::Uint(n) => n.to_string(),
  GAP (test) LINE   crates/store/tests/sort_index_log.rs:75: other => format!("{other:?}"),
  F?         BRANCH crates/store/tests/support/mod.rs:62:15 False: let ran = output.status.success() && stdout.contains("1 passed");
  F?         BRANCH crates/store/tests/tail_proof.rs:144:9 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  F?         BRANCH crates/store/tests/tail_proof.rs:144:36 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  F?         BRANCH crates/store/tests/tail_proof.rs:144:60 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  GAP (test) BRANCH crates/store/tests/tix.rs:127:12 False: if let Some(last) = bars.last() {
  F?         LINE   crates/store/tests/tix.rs:173: timeframe.as_str()
  F?         BRANCH crates/store/tests/tix.rs:605:76 True: && matches!(value, telemetry::OwnedValue::Str(file) if file.starts_with(&ours))
  IGNORED    LINE   crates/store/tests/tix_latency.rs:43: fn path(timeframe: Timeframe) -> StorePath<'static> {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:44: StorePath::new(PathParts {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:45: vendor: Vendor::Groww,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:46: exchange: "NSE",
  IGNORED    LINE   crates/store/tests/tix_latency.rs:47: segment: "INDEX",
  IGNORED    LINE   crates/store/tests/tix_latency.rs:48: symbol: "NIFTY",
  IGNORED    LINE   crates/store/tests/tix_latency.rs:49: contract: None,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:50: timeframe,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:51: month: YearMonth::new(2024, 7).unwrap(),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:52: file: FileKind::Bars,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:53: })
  IGNORED    LINE   crates/store/tests/tix_latency.rs:54: .unwrap()
  IGNORED    LINE   crates/store/tests/tix_latency.rs:55: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:59: fn month(days: i64, per_day: i64, step: i64, weekdays: bool) -> Vec<Bar> {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:60: let mut out = Vec::new();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:61: let (mut day, mut taken) = (0, 0);
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:62:11 True: while taken < days {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:62:11 False: while taken < days {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:62: while taken < days {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:64:12 True: if !weekdays || day % 7 < 5 {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:64:12 False: if !weekdays || day % 7 < 5 {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:64:25 True: if !weekdays || day % 7 < 5 {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:64:25 False: if !weekdays || day % 7 < 5 {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:64: if !weekdays || day % 7 < 5 {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:65: for k in 0..per_day {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:66: let ts = JULY_START + day * 86_400 * MICROS + (555 * 60 + k * step) * MICROS;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:67: out.push(Bar {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:68: ts_micros: ts,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:69: open: 2_000_000 + k,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:70: high: 2_000_100 + k,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:71: low: 1_999_900 + k,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:72: close: 2_000_050 + k,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:73: volume: 10,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:74: open_interest: OI_NULL,
  IGNORED    LINE   crates/store/tests/tix_latency.rs:75: });
  IGNORED    LINE   crates/store/tests/tix_latency.rs:76: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:77: taken += 1;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:78: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:79: day += 1;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:81: out
  IGNORED    LINE   crates/store/tests/tix_latency.rs:82: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:87: fn next(&mut self) -> u64 {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:88: self.0 ^= self.0 >> 12;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:89: self.0 ^= self.0 << 25;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:90: self.0 ^= self.0 >> 27;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:91: self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
  IGNORED    LINE   crates/store/tests/tix_latency.rs:92: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:95: fn percentiles(mut ns: Vec<u64>) -> (u64, u64, u64) {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:96: ns.sort_unstable();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:97: let at = |q: usize| ns[(ns.len() * q / 1000).min(ns.len() - 1)];
  IGNORED    LINE   crates/store/tests/tix_latency.rs:98: (at(500), at(990), *ns.last().unwrap())
  IGNORED    LINE   crates/store/tests/tix_latency.rs:99: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:101: fn report(case: &str, ns: Vec<u64>) {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:102: let (p50, p99, max) = percentiles(ns);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:103: println!("{case:<58} p50 {p50:>8} ns   p99 {p99:>8} ns   max {max:>9} ns");
  IGNORED    LINE   crates/store/tests/tix_latency.rs:104: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:107: fn time<T: Copy>(inputs: &[T], mut op: impl FnMut(T) -> u64) -> Vec<u64> {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:108: inputs
  IGNORED    LINE   crates/store/tests/tix_latency.rs:109: .iter()
  IGNORED    LINE   crates/store/tests/tix_latency.rs:110: .map(|&input| {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:111: let start = Instant::now();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:112: black_box(op(black_box(input)));
  IGNORED    LINE   crates/store/tests/tix_latency.rs:113: start.elapsed().as_nanos() as u64
  IGNORED    LINE   crates/store/tests/tix_latency.rs:114: })
  IGNORED    LINE   crates/store/tests/tix_latency.rs:115: .collect()
  IGNORED    LINE   crates/store/tests/tix_latency.rs:116: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:118: fn measure(name: &str, timeframe: Timeframe, bars: &[Bar]) {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:119: let root: PathBuf =
  IGNORED    LINE   crates/store/tests/tix_latency.rs:120: std::env::temp_dir().join(format!("brutex-tix-latency-{name}-{}", std::process::id()));
  IGNORED    LINE   crates/store/tests/tix_latency.rs:121: let _ = fs::remove_dir_all(&root);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:122: fs::create_dir_all(&root).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:124: let mut writer = BarFile::open_or_create(&root, path(timeframe), 7).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:125: for chunk in bars.chunks(22_500) {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:126: writer.append(chunk).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:127: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:129: let n = bars.len() as u64;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:130: let first = bars[0].ts_micros;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:131: let last = bars[bars.len() - 1].ts_micros;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:132: let width = i64::from(timeframe.secs()) * MICROS;
  IGNORED    LINE   crates/store/tests/tix_latency.rs:133: let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:135: let anywhere: Vec<i64> = (0..LOOKUPS)
  IGNORED    LINE   crates/store/tests/tix_latency.rs:136: .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)
  IGNORED    LINE   crates/store/tests/tix_latency.rs:137: .collect();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:139: let on_grid: Vec<i64> = anywhere
  IGNORED    LINE   crates/store/tests/tix_latency.rs:140: .iter()
  IGNORED    LINE   crates/store/tests/tix_latency.rs:141: .map(|&ts| first + (ts - first) / width * width)
  IGNORED    LINE   crates/store/tests/tix_latency.rs:142: .collect();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:143: let rows: Vec<u64> = (0..LOOKUPS).map(|_| rng.next() % n).collect();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:145: let tix = path(timeframe).to_path_buf(&root).with_extension("tix");
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:146:30 True: let passes: &[bool] = if tix.exists() {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:146:30 False: let passes: &[bool] = if tix.exists() {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:146: let passes: &[bool] = if tix.exists() {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:147: &[true, false]
  IGNORED    LINE   crates/store/tests/tix_latency.rs:149: &[false]
  IGNORED    LINE   crates/store/tests/tix_latency.rs:151: for &with_index in passes {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:152:12 True: if !with_index && tix.exists() {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:152:12 False: if !with_index && tix.exists() {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:152:27 True: if !with_index && tix.exists() {
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:152:27 False: if !with_index && tix.exists() {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:152: if !with_index && tix.exists() {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:153: fs::remove_file(&tix).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:154: }
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:155:22 True: let tag = if with_index { "index" } else { "bisection" };
  IGNORED    BRANCH crates/store/tests/tix_latency.rs:155:22 False: let tag = if with_index { "index" } else { "bisection" };
  IGNORED    LINE   crates/store/tests/tix_latency.rs:155: let tag = if with_index { "index" } else { "bisection" };
  IGNORED    LINE   crates/store/tests/tix_latency.rs:156: let label = |what: &str| format!("{name} ({n} bars) {what} [{tag}]");
  IGNORED    LINE   crates/store/tests/tix_latency.rs:159: let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:160: report(
  IGNORED    LINE   crates/store/tests/tix_latency.rs:161: &label("by time, random us"),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:162: time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:164: let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:165: report(
  IGNORED    LINE   crates/store/tests/tix_latency.rs:166: &label("by time, on grid"),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:167: time(&on_grid, |ts| file.first_at_or_after(ts).unwrap()),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:169: let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:170: report(
  IGNORED    LINE   crates/store/tests/tix_latency.rs:171: &label("by row, random rows (cold block)"),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:172: time(&rows, |row| file.read_record(row).unwrap().ts_micros as u64),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:176: let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:177: let same = vec![rows[0]; LOOKUPS];
  IGNORED    LINE   crates/store/tests/tix_latency.rs:178: report(
  IGNORED    LINE   crates/store/tests/tix_latency.rs:179: &label("by row, one row (warm block)"),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:180: time(&same, |row| file.read_record(row).unwrap().ts_micros as u64),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:183: let _ = fs::remove_dir_all(&root);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:184: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:188: fn time_and_row_lookup_latency() {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:189: measure("1min", Timeframe::MINUTE_1, &month(31, 375, 60, false));
  IGNORED    LINE   crates/store/tests/tix_latency.rs:190: measure("1s", Timeframe::SECOND_1, &month(23, 22_500, 1, true));
  IGNORED    LINE   crates/store/tests/tix_latency.rs:191: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:209: fn small_month_rule_latency() {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:211: let root: PathBuf =
  IGNORED    LINE   crates/store/tests/tix_latency.rs:212: std::env::temp_dir().join(format!("brutex-tix-latency-small-{}", std::process::id()));
  IGNORED    LINE   crates/store/tests/tix_latency.rs:213: let _ = fs::remove_dir_all(&root);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:214: fs::create_dir_all(&root).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:215: let all = month(1, 1_100, 1, true);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:216: for (name, n) in [
  IGNORED    LINE   crates/store/tests/tix_latency.rs:217: ("1s, 1023 bars, below", 1_023_usize),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:218: ("1s, 1024 bars, at", 1_024),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:219: ] {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:220: let at = root.join(name.replace([' ', ','], "-"));
  IGNORED    LINE   crates/store/tests/tix_latency.rs:221: fs::create_dir_all(&at).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:222: BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7)
  IGNORED    LINE   crates/store/tests/tix_latency.rs:223: .unwrap()
  IGNORED    LINE   crates/store/tests/tix_latency.rs:224: .append(&all[..n])
  IGNORED    LINE   crates/store/tests/tix_latency.rs:225: .unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:226: let tix = path(Timeframe::SECOND_1)
  IGNORED    LINE   crates/store/tests/tix_latency.rs:227: .to_path_buf(&at)
  IGNORED    LINE   crates/store/tests/tix_latency.rs:228: .with_extension("tix");
  IGNORED    LINE   crates/store/tests/tix_latency.rs:229: assert_eq!(tix.exists(), n >= 1_024, "{name}");
  IGNORED    LINE   crates/store/tests/tix_latency.rs:230: let (first, last) = (all[0].ts_micros, all[n - 1].ts_micros);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:231: let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:232: let anywhere: Vec<i64> = (0..LOOKUPS)
  IGNORED    LINE   crates/store/tests/tix_latency.rs:233: .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)
  IGNORED    LINE   crates/store/tests/tix_latency.rs:234: .collect();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:235: let file = BarFile::open_existing(&at, path(Timeframe::SECOND_1), 7).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:236: report(
  IGNORED    LINE   crates/store/tests/tix_latency.rs:237: &format!("{name}: by time, random us [{:?}]", file.time_lookup()),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:238: time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:241: for (name, held) in [
  IGNORED    LINE   crates/store/tests/tix_latency.rs:242: ("commit of 1 bar under the threshold (500 held)", 500_usize),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:243: ("commit of 1 bar reaching the threshold (1023 held)", 1_023),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:244: ("commit of 1 bar past the threshold (1099 held)", 1_099),
  IGNORED    LINE   crates/store/tests/tix_latency.rs:245: ] {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:246: let mut ns = Vec::with_capacity(COMMITS);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:247: for k in 0..COMMITS {
  IGNORED    LINE   crates/store/tests/tix_latency.rs:248: let at = root.join(format!("commit-{held}-{k}"));
  IGNORED    LINE   crates/store/tests/tix_latency.rs:249: fs::create_dir_all(&at).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:250: let mut writer = BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:251: writer.append(&all[..held]).unwrap();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:252: let start = Instant::now();
  IGNORED    LINE   crates/store/tests/tix_latency.rs:253: black_box(writer.append(black_box(&all[held..=held])).unwrap());
  IGNORED    LINE   crates/store/tests/tix_latency.rs:254: ns.push(start.elapsed().as_nanos() as u64);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:255: drop(writer);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:256: let _ = fs::remove_dir_all(&at);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:257: }
  IGNORED    LINE   crates/store/tests/tix_latency.rs:258: report(&format!("{name}, n={COMMITS}"), ns);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:260: let _ = fs::remove_dir_all(&root);
  IGNORED    LINE   crates/store/tests/tix_latency.rs:261: }
  GAP (test) BRANCH crates/store/tests/tix_rebuilder.rs:154:12 False: if let Some(last) = bars.last() {
  F?         BRANCH crates/store/tests/tix_rebuilder.rs:521:15 False: }) if asked == SYMBOL + 1
  GAP (test) BRANCH crates/store/tests/tix_small_month.rs:143:12 False: if let Some(last) = bars.last() {
  F?         LINE   crates/store/tests/tix_small_month.rs:160: other => panic!("not committed: {other:?}"),
  F?         BRANCH crates/store/tests/tix_small_month.rs:461:74 False: matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),
  F?         LINE   crates/store/tests/tix_small_month.rs:463: reader.time_lookup()
  GAP (test) LINE   crates/store/tests/tix_warn_once.rs:77: other => format!("{other:?}"),
  GAP (test) LINE   crates/store/tests/tix_warn_once.rs:125: Some((_, OwnedValue::Uint(n))) => *n,
  F?         LINE   crates/store/tests/tix_warn_once.rs:127: other => panic!("{key} is not a count: {other:?}"),
  F?         LINE   crates/store/tests/tix_warn_once.rs:160: other => panic!("unexpected store.tix info line {other:?}: {line:?}"),
  F?         LINE   crates/store/tests/tix_warn_once.rs:162: other => panic!("unexpected store.tix line at {other:?}: {line:?}"),
  F?         LINE   crates/store/tests/unit.rs:110: input.len()
  F?         BRANCH crates/store/tests/unit.rs:399:13 False: assert!(real.ohlc_is_sane() && real.counts_are_sane());
  F?         BRANCH crates/store/tests/unit.rs:399:36 False: assert!(real.ohlc_is_sane() && real.counts_are_sane());
  F?         LINE   crates/store/tests/unit.rs:527: _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),
  F?         BRANCH crates/store/tests/unit.rs:868:13 False: assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  F?         BRANCH crates/store/tests/unit.rs:868:54 False: assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  GAP (test) BRANCH crates/store/tests/unit.rs:908:16 False: if one.magic()[..7] == other.magic()[..7]
  F?         BRANCH crates/store/tests/unit.rs:1475:32 False: read == old || read == new,
  F?         LINE   crates/store/tests/unit.rs:1820: matches!(
  F?         LINE   crates/store/tests/unit.rs:1859: matches!(
  F?         LINE   crates/store/tests/unit.rs:1874: matches!(
  F?         BRANCH crates/store/tests/unit.rs:2047:26 False: .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  F?         BRANCH crates/store/tests/unit.rs:2047:43 False: .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  F?         LINE   crates/store/tests/unit.rs:2096: vendor.as_str(),
  F?         BRANCH crates/store/tests/unit.rs:2173:13 False: err.to_string().contains(field) && err.to_string().contains("case"),
  F?         BRANCH crates/store/tests/unit.rs:2173:48 False: err.to_string().contains(field) && err.to_string().contains("case"),
  F?         LINE   crates/store/tests/unit.rs:2826: a.as_str(),
  F?         LINE   crates/store/tests/unit.rs:2827: b.as_str()
  F?         LINE   crates/store/tests/unit.rs:2833: a.as_str()
  F?         LINE   crates/store/tests/unit.rs:2893: tf.as_str()
  F?         LINE   crates/store/tests/verdict.rs:129: Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),
  F?         LINE   crates/store/tests/verdict.rs:168: other => panic!("100x must be refused table_too_large, got {other:?}"),
  F?         BRANCH crates/store/tests/verdict.rs:228:13 False: got.cost.reads <= 5 && got.cost.opens <= 2,
  F?         BRANCH crates/store/tests/verdict.rs:228:36 False: got.cost.reads <= 5 && got.cost.opens <= 2,
  F?         LINE   crates/store/tests/verdict.rs:395: other => panic!("a 65-byte instrument key must be refused, got {other:?}"),
  F?         BRANCH crates/store/tests/verdict.rs:449:13 False: let Found::Record { ordinal, record } = &answer.found else {
  F?         LINE   crates/store/tests/verdict.rs:450: panic!("{whole} must be found");
  F?         BRANCH crates/store/tests/verdict.rs:584:12 False: if count > 2 {
  GAP (test) LINE   crates/store/tests/verdict.rs:588: }
  F?         LINE   crates/store/tests/verdict.rs:728: other => panic!("expected image_missing, got {other:?}"),
  F?         LINE   crates/store/tests/verdict.rs:793: other => panic!("expected generation_collision, got {other:?}"),
  F?         LINE   crates/store/tests/verdict.rs:1318: other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),
  F?         LINE   crates/store/tests/verdict.rs:1821: other => panic!(
  F?         LINE   crates/store/tests/verdict.rs:1823: clean.len()
  F?         LINE   crates/store/tests/verdict.rs:1856: other => panic!("expected record_corrupt, got {other:?}"),
  F?         BRANCH crates/store/tests/verdict.rs:1890:17 False: assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  F?         BRANCH crates/store/tests/verdict.rs:1890:40 False: assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  F?         BRANCH crates/store/tests/verdict.rs:2123:17 False: assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  F?         BRANCH crates/store/tests/verdict.rs:2123:40 False: assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  F?         LINE   crates/store/tests/verdict.rs:2283: feed.as_str(),
  F?         LINE   crates/store/tests/verdict.rs:2284: timeframe.as_str()
  F?         BRANCH crates/store/tests/verdict.rs:2358:58 False: matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),
  F?         LINE   crates/store/tests/verdict.rs:2435: other => panic!("expected lock_stalled, got {other:?}"),
  F?         LINE   crates/store/tests/verdict.rs:2455: matches!(
  F?         BRANCH crates/store/tests/verdict.rs:2545:21 False: image.extension().is_some_and(|ext| ext == "vdt")
  F?         BRANCH crates/store/tests/verdict.rs:2546:28 False: && image.parent().is_some_and(|dir| dir.ends_with("2031"))
  F?         BRANCH crates/store/tests/verdict.rs:2547:28 False: && image.exists(),
  F?         LINE   crates/store/tests/verdict.rs:2549: image.display()
  F?         LINE   crates/store/tests/verdict.rs:2554: other => panic!("expected index_missing, got {other}"),
  F?         BRANCH crates/store/tests/verdict.rs:2604:13 False: assert!(named && (1..=2).contains(&seen), "seen {seen}");
  F?         BRANCH crates/store/tests/verdict.rs:2604:22 False: assert!(named && (1..=2).contains(&seen), "seen {seen}");
  F?         LINE   crates/store/tests/verdict.rs:2730: other => panic!("{other}"),
  F?         BRANCH crates/store/tests/verdict.rs:2782:9 False: said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  F?         BRANCH crates/store/tests/verdict.rs:2782:61 False: said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  F?         LINE   crates/store/tests/write.rs:1402: sidecar.display()
  F?         LINE   crates/store/tests/write.rs:1419: sidecar.display()
  F?         BRANCH crates/store/tests/write.rs:1564:13 False: text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  F?         BRANCH crates/store/tests/write.rs:1564:62 False: text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  F?         LINE   crates/store/tests/write.rs:1619: panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")
  F?         LINE   crates/store/tests/write.rs:1663: panic!(
  F?         BRANCH crates/store/tests/write.rs:2103:85 False: Err(StoreError::Format { path, source: FormatError::NoValidHeader }) if *path == bars
`````

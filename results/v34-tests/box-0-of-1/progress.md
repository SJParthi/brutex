2026-11-16T15:53:24Z B checkout 7a5428d30bdb8a5e0fec427848178c82907de2fb, code manifest 151 files 80fa426f2370a97377d5d35600db260af36b30f488526920f6e539cc84a7a34a: OK
2026-11-16T15:53:24Z C box facts in box.txt; rustc 1.97.1 (8bab26f4f 2026-07-14)
2026-11-16T15:53:24Z D build start: cargo test --release --locked --offline -p spikefade --no-run (CARGO_BUILD_JOBS=4)
2026-11-16T15:53:36Z set up: rustc 1.97.1 (8bab26f4f 2026-07-14); commit 7a5428d30bdb8a5e0fec427848178c82907de2fb
2026-11-16T15:54:01Z D build rc 0, wall 37s (MEASURED)
2026-11-16T15:54:01Z E list: Linux 217 names, Mac 217; only on the Mac 0, only on Linux 0
2026-11-16T15:54:01Z F run start: cargo test --release --locked --offline -p spikefade -- --test-threads 4 --exact + 10 x --skip (skipped.txt)
2026-11-16T15:54:01Z F run rc 101, wall 0s (MEASURED): passed 204, failed 2, ignored 1; test result: FAILED. 204 passed; 2 failed; 1 ignored; 0 measured; 10 filtered out; finished in 0.66s
2026-11-16T15:54:02Z G alone tests34s::root_digest_same_for_every_form: FAILED (KNOWN-ZSTD-PATH)
2026-11-16T15:54:02Z G alone tests34s::sfz_roundtrip_all_rows: FAILED (KNOWN-ZSTD-PATH)
2026-11-16T15:54:03Z H1 skipped tests run alone: 10 of 10 fail without their data
2026-11-16T15:54:03Z H2 /opt/homebrew/bin/zstd -> /usr/bin/zstd (*** Zstandard CLI (64-bit) v1.5.5, by Yann Collet ***)
2026-11-16T15:54:04Z H2 tests34s::root_digest_same_for_every_form with the Mac path present: ok
2026-11-16T15:54:04Z H2 tests34s::sfz_roundtrip_all_rows with the Mac path present: ok
2026-11-16T15:54:04Z I SUMMARY job=v34-tests box=0/1 status=DONE commit=7a5428d30bdb8a5e0fec427848178c82907de2fb manifest=151/80fa426f2370a97377d5d35600db260af36b30f488526920f6e539cc84a7a34a rustc="rustc 1.97.1 (8bab26f4f 2026-07-14)" mac=217(216 ok, 1 ignored) linux_listed=217 passed=204 failed=2 ignored=1 skipped_data=10 same=205 known_zstd=2 contention=0 differs_on_linux=0 not_run=0 only_mac=0 only_linux=0 build_wall=37s run_wall=0s total_wall=40s

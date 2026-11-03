STATUS: IN PROGRESS

# BASE-COV-LIBS-LS4: crates core, costs, engine, greeks, telemetry

TOKEN `BASE-COV-LIBS-LS4-20261103-1`. Branch `cloud/cov-libs-ls4-20261103`, cut from `origin/cloud/base-kit-libs-20261103` at `eeec369b` (START `bd04fb299d23`). Session in progress: this file is rewritten at each step.

## Done so far

1. OWNER commit `a805d0cc`, pushed. Kit files checked against `MANIFEST.md` (all 14 hashes used match).
2. Pins: `rustup toolchain install nightly-2026-08-08 --profile minimal --component llvm-tools-preview`; `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; `llvm-cov` and `llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly`. Host: x86_64 Linux, 4 cores, 15 GB, root.
3. Attempt 4's two never-compiled files compile on stable 1.97.1 (`cargo test -p telemetry -p engine --lib --locked --no-run`, 17 s, no warning).
4. The brief's command, with `-p core` (the package is `core`; `brutex_core` is its lib target, and `-p brutex-core` names no package): `cargo test -p core -p costs -p engine -p greeks -p telemetry --lib --locked --no-fail-fast`.
5. Fix in my files: `crates/engine/src/refusal_cover_tests.rs` failed `clippy -D warnings` (`explicit_counter_loop`: its counter equalled the loop index). Commit `d76b11b7`.
6. Two engine reds whose cause and test are both in `crates/engine/src/lib.rs` (LSA's file): see BLOCKED OUTSIDE below.

## BLOCKED OUTSIDE (for LSA): `crates/engine/src/lib.rs`

Attempt 4 (D-7329) put an indented `#[cfg(test)]` arm inside `reserved` (line 208). Two source-shape tests cut the file at its FIRST `#[cfg(test)]` to get the shipping region, so that region now ends at line 208 and they fail:

- `tests::the_sweep_counts_support_against_the_fixed_width_column` (lib.rs:4719, `left: 0, right: 2`)
- `tests::the_walk_has_exactly_three_early_exits_and_each_is_a_budget` (lib.rs:4445)

The file itself says (lines 2517-2518) the tests read "only up to its first `#[cfg(test)]`"; the sibling test `every_level_appends_its_survivors_through_the_one_primitive` already splits on `"\n#[cfg(test)]\n"` (the first one at the margin) and passes. The fix splits the two failing tests the same way, so they read exactly the region they read before attempt 4, plus the seam's six lines (no exit and no `column.support(` in them). It also corrects the stale `too_many_lines` figure (114 before the patch by clippy's own count, 116 after; the reason said 101). Proved on this host: patch applied, `rustfmt --check` clean, `cargo clippy -p engine --all-targets --locked -- -D warnings` clean, `cargo test -p engine --locked` green (lib 110 passed, all integration and doc tests pass); `lib.rs` restored afterwards, never committed. Draft decision D-8566 below. The exact change (sha256 of the patch text `c4cbc10e6c8dee30d8efd6952282239aa86ad27fcd9097964e9110e463d1e45c`):

```diff
diff --git a/crates/engine/src/lib.rs b/crates/engine/src/lib.rs
index 756789fc..71baf1c1 100644
--- a/crates/engine/src/lib.rs
+++ b/crates/engine/src/lib.rs
@@ -2515,7 +2515,8 @@ const WORST_K3: usize = LIVE_POSITIONS * (LIVE_POSITIONS - 1) * (LIVE_POSITIONS
 const WORST_K4: usize = WORST_K3 * (LIVE_POSITIONS - 3) / 4;
 
 /// The manifest reader behind V-06b. Declared here, past the shipping region, because
-/// several tests below read this file only up to its first `#[cfg(test)]`.
+/// several tests below read this file only up to its first `#[cfg(test)]` line at the
+/// margin (the test-only arm inside `reserved` is indented, D-7329).
 #[cfg(test)]
 mod manifest;
 
@@ -4387,7 +4388,7 @@ mod tests {
     #[test]
     #[expect(
         clippy::too_many_lines,
-        reason = "at 101 lines, and every one of them is the accounting this \
+        reason = "at 116 lines, and every one of them is the accounting this \
                   test exists to publish: it reads its own shipping region and \
                   names each early exit together with the budget that licenses \
                   it. The length IS the enumeration -- splitting it would leave \
@@ -4395,7 +4396,7 @@ mod tests {
                   which is precisely the drift the assertion refuses. Placed on \
                   this function rather than the module so it claims no more than \
                   it needs. Safe against the scan itself: the region examined is \
-                  `src.split(\"#[cfg(test)]\").next()`, which ends far above here"
+                  `src.split_once(\"\\n#[cfg(test)]\\n\")`, which ends far above here"
     )]
     fn the_walk_has_exactly_three_early_exits_and_each_is_a_budget() {
         let src = include_str!("lib.rs");
@@ -4410,8 +4411,12 @@ mod tests {
         //
         // `return` is counted only in the SHIPPING region: the test module below
         // is full of ordinary returns and closures, and scanning it would pin a
-        // number that moves whenever a test is added.
-        let shipping = src.split("#[cfg(test)]").next().unwrap_or(src);
+        // number that moves whenever a test is added. The region ends at the
+        // first `#[cfg(test)]` line at the margin: the test-only arm inside
+        // `reserved` is indented and stands in it (D-7329).
+        let shipping = src
+            .split_once("\n#[cfg(test)]\n")
+            .map_or(src, |(code, _)| code);
         let count_exits = |text: &str| -> usize {
             text.lines()
                 .map(|l| l.split_once("//").map_or(l, |(code, _)| code))
@@ -4712,10 +4717,14 @@ mod tests {
         // `drain` iterates it, so its call reads `support(mask)` on an already
         // borrowed item and the old needle stopped seeing it.
         //
-        // Splitting on the first `#[cfg(test)]` is the same discipline the
-        // `column.rs` check below already uses, and it separates the two by what
-        // they ARE rather than by how they happen to be written.
-        let shipped = src.split("#[cfg(test)]").next().unwrap_or(src);
+        // Splitting on the first `#[cfg(test)]` line at the margin is the same
+        // discipline the `column.rs` check below already uses, and it separates
+        // the two by what they ARE rather than by how they happen to be written.
+        // At the margin, because the test-only arm inside `reserved` is indented
+        // and stands in the shipping region (D-7329).
+        let shipped = src
+            .split_once("\n#[cfg(test)]\n")
+            .map_or(src, |(code, _)| code);
         assert_eq!(
             shipped.matches(concat!("column.", "support(")).count(),
             2,
```

## Host reds (proved; for the Mac)

As root, `cargo test -p core -p costs -p engine -p greeks -p telemetry --locked --no-fail-fast`: passed 1088, failed 6, ignored 3 (30 result lines). The 6 are the two engine tests above and four telemetry tests that `chmod` a directory and expect the host to refuse, which root bypasses; they are the base's "telemetry 4" root reds (`CONTEXT.md`). Proof: the same test binary run as `nobody` passes each one (`setpriv --reuid=65534 --regid=65534 --clear-groups <binary> --exact <test>`: `test result: ok. 1 passed`), and as root each fails (`0 passed; 1 failed`):

- `sink::tests::a_coverage_file_is_read_line_by_line_and_extended_where_it_stands`
- `sink::tests::a_failed_delete_is_counted_named_and_left_for_the_next_start`
- `sink::tests::a_rename_aside_that_cannot_happen_fails_the_roll_by_name`
- `sink::tests::an_audit_stream_that_cannot_be_looked_for_is_an_error`

Non-root run (every test binary as `nobody` through `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='setpriv --reuid=65534 --regid=65534 --clear-groups'`, with `TMPDIR` and the target's `tmp` sticky and world-writable and a `GIT_CONFIG_GLOBAL` file naming this root-owned checkout `safe.directory`, so git does not refuse it as "dubious ownership" for another user): passed 1092, failed 2 (the two engine tests above), ignored 3. Coverage is measured the same non-root way, so permission tests run as they do on CI and the Mac.

## NEEDS THE MAC

Route B on the Mac's pin (aarch64-apple-darwin), the macOS rows, the non-root 14-core test run, the ignored real-data tests (L-BCL-3), the D1 landing.

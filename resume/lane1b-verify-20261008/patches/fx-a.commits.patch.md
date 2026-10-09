From 221f29fc581d28f7ad816300050557b678775e12 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:21:44 +0000
Subject: [PATCH 1/5] cli: pool-oos pass 1 is pool's pass 1, in surface order;
 guard every recording-kernel caller
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

G1-1. Before: `pool_oos::run_under` screened its surface with
`surface.par_iter().map(|symbol| .. crate::one_rung(..))`, which is the shape
D-1701 removed from `pool`. As a result:
- every screen's ledger row, frontier, trade and receipt blocks and
  attempts were written from rayon workers, in thread-completion order;
- up to the pool's width of whole-machine sweeps ran at once, with nothing
  raising `SWEEPS_SHARING_THIS_MACHINE` (D-1709 measured that
  oversubscription at 157 GB claimed of 48).
Both order guards named fixed files and never read pool_oos.rs.

After:
- `pool::screen_pass_one` is pass 1 for both verbs:
  `crate::in_input_order` around `one_rung_cached`, under the root `run`
  resolved and the commit stamp, with no SharedBy.
- `pool`'s pass 2 moved into `price_surface`.
- `ordered::tests::every_caller_of_a_recording_rung_kernel_runs_it_in_input_order`
  reads every .rs file under crates/cli/src at test time. It finds each
  caller of `one_rung(`/`one_rung_cached(` and their non-test callers, and
  refuses any parallel primitive or SharedBy in them. Its scanner is pinned
  on a synthetic source.
- `pool::tests::pass_one_files_in_surface_order_on_any_pool_width` holds
  the first instrument back. On a 4-thread and on a 1-thread pool it
  requires ledger row i to be screen i.
- pool.rs's "in parallel" header and two stale test docs are corrected.

Fail-before:
- On the unfixed tree the scanner test failed at ordered_tests.rs:471:
  "pool.rs `run_under` calls a recording rung kernel and spells parallel
  work". That body held pass 2's par_iter. With that moved out, the same
  message fired for pool_oos.rs `run_under`.
- With screen_pass_one's map made parallel, the order test failed at
  pool.rs:4051: "4 thread(s): ledger row 0 is surface instrument 0".

D-4700, L1FA-01, L1FA-02; docs/06 pool-oos pass 1, input order, §171.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/ordered.rs       |   2 +-
 crates/cli/src/ordered_tests.rs | 392 +++++++++++++++++++++++++++++++-
 crates/cli/src/pool.rs          | 363 ++++++++++++++++++++++++-----
 crates/cli/src/pool_oos.rs      |  31 ++-
 docs/04-invariants.md           |   7 +
 docs/05-decisions.md            |  89 ++++++++
 docs/06-limits.md               |  22 +-
 7 files changed, 816 insertions(+), 90 deletions(-)

diff --git a/crates/cli/src/ordered.rs b/crates/cli/src/ordered.rs
index b4986832..187d2e4b 100644
--- a/crates/cli/src/ordered.rs
+++ b/crates/cli/src/ordered.rs
@@ -224,4 +224,4 @@ pub(crate) fn turn() -> Turn {
 
 #[cfg(test)]
 #[path = "ordered_tests.rs"]
-mod tests;
+pub(crate) mod tests;
diff --git a/crates/cli/src/ordered_tests.rs b/crates/cli/src/ordered_tests.rs
index 757905ab..0d0a28fd 100644
--- a/crates/cli/src/ordered_tests.rs
+++ b/crates/cli/src/ordered_tests.rs
@@ -190,11 +190,393 @@ fn body<'a>(source: &'a str, name: &str) -> &'a str {
     &rest[..end]
 }
 
+/// The functions that sweep AND RECORD one rung. Each writes preparation and
+/// probe attempts, frontier, trade and receipt blocks and a `runs.bin` row
+/// from inside itself, so two in flight at once file them in thread-completion
+/// order (GAP13-13) and each takes the whole machine's ceiling and every core
+/// (R9-cli-o1-0). D-1701, D-4700.
+const RECORDING_KERNELS: [&str; 2] = ["one_rung", "one_rung_cached"];
+
+/// Every spelling in this crate that puts work on another thread.
+pub(crate) const PARALLEL: [&str; 12] = [
+    "par_iter",
+    "par_bridge",
+    "par_chunks",
+    "par_extend",
+    "par_drain",
+    "rayon::join",
+    "rayon::scope",
+    "rayon::spawn",
+    "ThreadPoolBuilder",
+    "thread::spawn",
+    "thread::scope",
+    "ordered::map",
+];
+
+/// A function that calls a recording rung kernel (level 1) or calls a
+/// non-test level-1 caller (level 2), as [`recording_callers`] finds it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub(crate) struct Caller {
+    /// Its file, below `src`.
+    pub(crate) file: String,
+    pub(crate) name: String,
+    /// A `#[test]`, or any function in a test-only file or `mod tests`.
+    pub(crate) test: bool,
+    pub(crate) level: u8,
+    /// From its head line to its closing brace.
+    pub(crate) body: String,
+}
+
+/// Every `.rs` file under this crate's `src`, as its path below `src` and its
+/// text, read when the test runs, in path order. A new file is in the scan the
+/// day it is added: there is no list to forget to extend.
+pub(crate) fn sources() -> Vec<(String, String)> {
+    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
+    let mut pending = vec![root.clone()];
+    let mut found = Vec::new();
+    while let Some(dir) = pending.pop() {
+        for entry in std::fs::read_dir(&dir).expect("a source directory") {
+            let path = entry.expect("a directory entry").path();
+            if path.is_dir() {
+                pending.push(path);
+            } else if path.extension().is_some_and(|ext| ext == "rs") {
+                let name = path
+                    .strip_prefix(&root)
+                    .expect("below src")
+                    .to_string_lossy()
+                    .into_owned();
+                found.push((name, std::fs::read_to_string(&path).expect("a source file")));
+            }
+        }
+    }
+    found.sort();
+    found
+}
+
+fn is_ident(c: char) -> bool {
+    c.is_ascii_alphanumeric() || c == '_'
+}
+
+/// `line` with its leading visibility and qualifier words removed, so a
+/// function head reads `fn name(`.
+fn without_qualifiers(line: &str) -> &str {
+    let mut rest = line.trim_start();
+    loop {
+        let before = rest;
+        for word in [
+            "pub(crate) ",
+            "pub(super) ",
+            "pub ",
+            "const ",
+            "async ",
+            "unsafe ",
+        ] {
+            rest = rest.strip_prefix(word).unwrap_or(rest);
+        }
+        if rest == before {
+            return rest;
+        }
+    }
+}
+
+/// The byte offset of each CALL `name(` in `text`: not on a comment line, not
+/// inside a string literal on its line, not a definition (`fn name(`), not a
+/// method (`.name(`) and not the tail of a longer name.
+fn calls(text: &str, name: &str) -> Vec<usize> {
+    let needle = format!("{name}(");
+    let mut out = Vec::new();
+    let mut line_start = 0;
+    for line in text.split_inclusive('\n') {
+        if !line.trim_start().starts_with("//") {
+            let mut from = 0;
+            while let Some(found) = line.get(from..).and_then(|rest| rest.find(&needle)) {
+                let at = from + found;
+                from = at + needle.len();
+                let before = &line[..at];
+                let quotes = before.matches('"').count() - before.matches("\\\"").count();
+                if before
+                    .chars()
+                    .next_back()
+                    .is_some_and(|c| is_ident(c) || c == '.')
+                    || before.ends_with("fn ")
+                    || quotes % 2 == 1
+                {
+                    continue;
+                }
+                out.push(line_start + at);
+            }
+        }
+        line_start += line.len();
+    }
+    out
+}
+
+/// The function whose body holds byte `at` of `text`: its name, whether it is
+/// a test, and its text from its head line to its closing brace. A function
+/// that closes before `at` (one nested earlier in the same body) is skipped.
+fn enclosing(text: &str, at: usize) -> Option<(String, bool, String)> {
+    let mut lines = Vec::new();
+    let mut start = 0;
+    for line in text.split_inclusive('\n') {
+        lines.push((start, line));
+        start += line.len();
+    }
+    let index = lines.iter().rposition(|&(start, _)| start <= at)?;
+    let test_file = text.starts_with("#![cfg(test)]");
+    let test_module = text.find("\nmod tests {").is_some_and(|module| module < at);
+    for head in (0..=index).rev() {
+        let (head_start, line) = lines[head];
+        let Some(after) = without_qualifiers(line).strip_prefix("fn ") else {
+            continue;
+        };
+        let name: String = after.chars().take_while(|&c| is_ident(c)).collect();
+        if name.is_empty() {
+            continue;
+        }
+        let indent = &line[..line.len() - line.trim_start().len()];
+        let close = format!("{indent}}}");
+        let Some(&(end_start, end_line)) = lines
+            .get(head + 1..)?
+            .iter()
+            .find(|(_, l)| l.trim_end() == close)
+        else {
+            continue;
+        };
+        if end_start < at {
+            continue;
+        }
+        let marked = lines[..head]
+            .iter()
+            .rev()
+            .map(|(_, l)| l.trim())
+            .take_while(|l| !l.is_empty() && *l != "}")
+            .any(|l| l == "#[test]");
+        let body = text[head_start..end_start + end_line.len()].to_owned();
+        return Some((name, marked || test_file || test_module, body));
+    }
+    None
+}
+
+/// Every function in `sources` that calls one of `names`, at `level`.
+///
+/// # Errors
+///
+/// A call outside any function: refused rather than skipped.
+fn callers_of(
+    sources: &[(String, String)],
+    names: &[String],
+    level: u8,
+) -> Result<Vec<Caller>, String> {
+    let mut found: Vec<Caller> = Vec::new();
+    for (file, text) in sources {
+        for name in names {
+            for at in calls(text, name) {
+                let (caller, test, body) = enclosing(text, at)
+                    .ok_or_else(|| format!("{file}: a call of {name} outside any fn"))?;
+                let row = Caller {
+                    file: file.clone(),
+                    name: caller,
+                    test,
+                    level,
+                    body,
+                };
+                if !found.contains(&row) {
+                    found.push(row);
+                }
+            }
+        }
+    }
+    Ok(found)
+}
+
+/// Every caller of a [`RECORDING_KERNELS`] function in `sources`: level 1.
+///
+/// # Errors
+///
+/// As [`callers_of`].
+pub(crate) fn kernel_callers(sources: &[(String, String)]) -> Result<Vec<Caller>, String> {
+    let kernels: Vec<String> = RECORDING_KERNELS.iter().map(|k| (*k).to_owned()).collect();
+    callers_of(sources, &kernels, 1)
+}
+
+/// Every caller of each non-test level-1 caller that is not itself a kernel:
+/// level 2. Found by scanning, so a new verb, file or wrapper is checked the
+/// day it lands.
+///
+/// # Errors
+///
+/// A wrapper whose name is defined in more than one file, so its callers
+/// cannot be told apart by name: refused rather than skipped.
+pub(crate) fn wrapper_callers(
+    sources: &[(String, String)],
+    level_one: &[Caller],
+) -> Result<Vec<Caller>, String> {
+    let mut wrappers: Vec<String> = level_one
+        .iter()
+        .filter(|c| !c.test && !RECORDING_KERNELS.contains(&c.name.as_str()))
+        .map(|c| c.name.clone())
+        .collect();
+    wrappers.sort();
+    wrappers.dedup();
+    for wrapper in &wrappers {
+        let head = format!("fn {wrapper}(");
+        let generic = format!("fn {wrapper}<");
+        let defined: Vec<&str> = sources
+            .iter()
+            .filter(|(_, text)| {
+                text.lines().any(|line| {
+                    let line = without_qualifiers(line);
+                    line.starts_with(&head) || line.starts_with(&generic)
+                })
+            })
+            .map(|(file, _)| file.as_str())
+            .collect();
+        if defined.len() != 1 {
+            return Err(format!(
+                "`{wrapper}` calls a recording rung kernel and is defined in {defined:?}; \
+                 a wrapper needs a crate-unique name so its callers can be checked"
+            ));
+        }
+    }
+    callers_of(sources, &wrappers, 2)
+}
+
+/// [`kernel_callers`] then [`wrapper_callers`].
+///
+/// # Errors
+///
+/// As either.
+pub(crate) fn recording_callers(sources: &[(String, String)]) -> Result<Vec<Caller>, String> {
+    let mut found = kernel_callers(sources)?;
+    let more = wrapper_callers(sources, &found)?;
+    found.extend(more);
+    Ok(found)
+}
+
+/// The first [`PARALLEL`] spelling in `body`, if any.
+pub(crate) fn parallel_in(body: &str) -> Option<&'static str> {
+    PARALLEL.iter().copied().find(|token| body.contains(token))
+}
+
+/// **Every function in this crate that calls a recording rung kernel, and
+/// every function that calls one of those, runs it on the calling thread:**
+/// no parallel spelling anywhere in its body, and no level-1 production
+/// caller raises `SharedBy`. Found by scanning every file under `src`, not a
+/// list, so `pool-oos`'s pass 1 (a rayon parallel map over the surface around
+/// `one_rung`, G1-1) fails here, and so would the next verb that copies it.
+/// GAP13-13, R9-cli-o1-0, D-1701, D-4700.
+#[test]
+fn every_caller_of_a_recording_rung_kernel_runs_it_in_input_order() {
+    let sources = sources();
+    let level_one = kernel_callers(&sources).unwrap_or_else(|why| panic!("{why}"));
+    let mut callers = level_one.clone();
+    for caller in &level_one {
+        assert_eq!(
+            parallel_in(&caller.body),
+            None,
+            "{} `{}` calls a recording rung kernel and spells parallel work",
+            caller.file,
+            caller.name
+        );
+    }
+    callers.extend(wrapper_callers(&sources, &level_one).unwrap_or_else(|why| panic!("{why}")));
+    for caller in &callers {
+        assert_eq!(
+            parallel_in(&caller.body),
+            None,
+            "{} `{}` (level {}) calls a recording rung kernel and spells parallel work",
+            caller.file,
+            caller.name,
+            caller.level
+        );
+        if caller.level == 1 && !caller.test {
+            assert!(
+                !caller.body.contains("SharedBy::these"),
+                "{} `{}`: one sweep in flight shares nothing",
+                caller.file,
+                caller.name
+            );
+        }
+    }
+    let has = |file: &str, name: &str, level: u8| {
+        callers
+            .iter()
+            .any(|c| c.file == file && c.name == name && c.level == level && !c.test)
+    };
+    assert!(has("lib.rs", "sweep_rungs", 1), "{callers:#?}");
+    assert!(has("lib.rs", "one_rung", 1), "{callers:#?}");
+    assert!(has("lib.rs", "descend_in", 1), "{callers:#?}");
+    assert!(has("pool.rs", "screen_pass_one", 1), "{callers:#?}");
+    assert!(
+        has("pool.rs", "run_under", 2),
+        "pool pass 1 is the shared one"
+    );
+    assert!(has("pool_oos.rs", "run_under", 2), "pool-oos pass 1 is too");
+}
+
+/// The scan finds a parallel caller, skips comments, strings, methods,
+/// definitions and an earlier nested function, follows a wrapper one level
+/// up, and refuses a wrapper whose name is not unique.
+#[test]
+fn the_recording_caller_scan_sees_calls_and_nothing_else() {
+    let kernel = concat!("one_rung", "(");
+    let wrap = concat!("wrapper", "(");
+    let file = format!(
+        "fn wrapper(items: &[u8]) {{\n    \
+         fn nested() {{}}\n    \
+         // {kernel}x) in a comment\n    \
+         let s = \"{kernel}\";\n    \
+         x.{kernel}1);\n    \
+         items.par_iter().map(|i| {kernel}i));\n\
+         }}\n\
+         fn outer() {{\n    \
+         {wrap}&[]);\n\
+         }}\n\
+         #[test]\n\
+         fn a_test() {{\n    \
+         crate::{kernel}1);\n\
+         }}\n"
+    );
+    let callers = recording_callers(&[("x.rs".to_owned(), file.clone())]).expect("scanned");
+    let named: Vec<(&str, u8, bool)> = callers
+        .iter()
+        .map(|c| (c.name.as_str(), c.level, c.test))
+        .collect();
+    assert_eq!(
+        named,
+        [
+            ("wrapper", 1, false),
+            ("a_test", 1, true),
+            ("outer", 2, false)
+        ]
+    );
+    assert_eq!(parallel_in(&callers[0].body), Some("par_iter"));
+    assert_eq!(parallel_in(&callers[1].body), None);
+    assert_eq!(parallel_in(&callers[2].body), None);
+    let twice = [
+        ("x.rs".to_owned(), file),
+        ("y.rs".to_owned(), "fn wrapper() {\n}\n".to_owned()),
+    ];
+    assert!(
+        recording_callers(&twice).is_err_and(|why| why.contains("crate-unique")),
+        "an ambiguous wrapper is refused"
+    );
+    let quiet = "fn f() {\n    // one_rung(\n    let s = \"one_rung(\";\n}\n";
+    assert!(
+        recording_callers(&[("q.rs".to_owned(), quiet.to_owned())])
+            .expect("scanned")
+            .is_empty()
+    );
+}
+
 /// **Every whole-command fan-out that writes the shared journal or ledger
 /// writes in input order: the Boolean family pools as ordered lanes (D-1556),
-/// `range-all` and pool pass 1 one call at a time through `in_input_order`
-/// (D-1701, kept over D-1556 for those two by D-1709).** None is an indexed
-/// parallel map or a private thread pool.
+/// `range-all` and pool pass 1 -- `pool`'s and `pool-oos`'s, one shared
+/// `screen_pass_one` -- one call at a time through `in_input_order` (D-1701,
+/// kept over D-1556 for those two by D-1709; D-4700).** None is an indexed
+/// parallel map or a private thread pool. Every OTHER caller of a recording
+/// rung kernel is found by scanning, in
+/// `every_caller_of_a_recording_rung_kernel_runs_it_in_input_order`.
 #[test]
 fn every_whole_command_fan_out_writes_in_input_order() {
     for (file, source, name, shape) in [
@@ -207,8 +589,8 @@ fn every_whole_command_fan_out_writes_in_input_order() {
         (
             "pool.rs",
             include_str!("pool.rs"),
-            "fn run_under(",
-            "crate::in_input_order(&surface,",
+            "fn screen_pass_one(",
+            "crate::in_input_order(surface,",
         ),
         (
             "boolean_catalog_prepared.rs",
diff --git a/crates/cli/src/pool.rs b/crates/cli/src/pool.rs
index 49961a4d..317fa8c1 100644
--- a/crates/cli/src/pool.rs
+++ b/crates/cli/src/pool.rs
@@ -19,8 +19,10 @@
 //!
 //! 1. **PER SYMBOL.** Every instrument the store holds on this rung and this
 //!    feed that is on the engine surface — the two indices and every F&O cash
-//!    equity present — is screened exactly as `range-rung` screens one, in
-//!    parallel, one run identity each. The rows are the best combination on
+//!    equity present — is screened exactly as `range-rung` screens one, one
+//!    at a time in surface order (D-1701; this said "in parallel" until
+//!    D-4700, long after the code stopped being), one run identity each. The
+//!    rows are the best combination on
 //!    each stock ALONE. A stock whose own best row already has a tiny worst
 //!    trade and a huge best one is a candidate on its own, and that table says
 //!    which.
@@ -145,6 +147,114 @@ pub(crate) struct Screened {
     pub(crate) outcome: Result<crate::results::Record, String>,
 }
 
+/// **Pass 1, `pool`'s and `pool-oos`'s: every surface instrument screened
+/// exactly as `range-rung` screens one, ONE AT A TIME, IN SURFACE ORDER.**
+///
+/// Each screen is a [`crate::one_rung_cached`], which writes its preparation
+/// and probe attempts, frontier, trade and receipt blocks and `runs.bin` row
+/// from inside itself. So the loop is [`crate::in_input_order`], as
+/// `sweep_rungs` runs rungs (D-1701, D-1709), and both verbs call this one
+/// function rather than each spelling its own loop.
+///
+/// # Why not in parallel
+///
+/// `pool-oos` screened its surface as a rayon parallel map (G1-1, D-4700), the
+/// shape D-1701 removed from `pool`: every instrument's durable rows landed in
+/// thread-completion order (GAP13-13), and up to the pool's width of sweeps
+/// ran at once while nothing raised `SWEEPS_SHARING_THIS_MACHINE`, so each
+/// took the whole machine's candidate ceiling and every core (R9-cli-o1-0;
+/// D-1709 measured eight such sweeps claiming 157 GB of 48). Dividing the
+/// ceiling instead would fold a different ceiling into every identity, so a
+/// pool's run would differ from `range-rung`'s for the same instrument. One
+/// at a time, the counter's 1 is the truth, and each screen's own support
+/// lanes and grid pricing still use every core.
+///
+/// # The root is the caller's
+///
+/// `root` is the store the caller already resolved through
+/// `crate::store_root`, the root `one_rung` would resolve again from the
+/// same environment, so production reads exactly what it read. Passing it
+/// lets an in-process test drive pass 1 on a generated store.
+///
+/// # Cost
+///
+/// `surface.len()` screens in sequence, each what `range-rung` costs on that
+/// instrument. Not a §3 rule-4 operation; `docs/06-limits.md` §171 states it.
+pub(crate) fn screen_pass_one(
+    root: &std::path::Path,
+    commit: Option<&'static str>,
+    vendor_word: &str,
+    surface: &[String],
+    rung: &'static str,
+    (from, to): ((u16, u8), (u16, u8)),
+    support_ppm: Option<u64>,
+) -> Vec<Screened> {
+    crate::in_input_order(surface, |symbol| {
+        #[cfg(test)]
+        pause_if_held_back(root, symbol);
+        Screened {
+            symbol: symbol.clone(),
+            outcome: crate::one_rung_cached(
+                crate::RungAsk {
+                    vendor_word,
+                    underlying: symbol,
+                    rung,
+                    from,
+                    to,
+                    support_ppm,
+                    attempt: None,
+                },
+                crate::RungStore {
+                    root: Ok(root.to_path_buf()),
+                    commit,
+                },
+                &mut crate::AuditCache::default(),
+            )
+            .outcome,
+        }
+    })
+}
+
+/// A test seam: the store root and the instrument whose pass-1 screen is held
+/// back, so the screens would finish in a different order from the surface's.
+/// Process-wide, so a screen on any thread sees it, and keyed by root, so no
+/// other test's store is slowed. D-4700.
+#[cfg(test)]
+static HELD_BACK: std::sync::Mutex<Option<(std::path::PathBuf, String)>> =
+    std::sync::Mutex::new(None);
+
+/// Hold back every later pass-1 screen of `symbol` on `root`; `None` clears.
+#[cfg(test)]
+pub(crate) fn hold_back(held: Option<(&std::path::Path, &str)>) {
+    let mut slot = HELD_BACK
+        .lock()
+        .unwrap_or_else(std::sync::PoisonError::into_inner);
+    *slot = held.map(|(root, symbol)| (root.to_path_buf(), symbol.to_owned()));
+}
+
+/// Whether the seam holds this screen back: exactly the named root AND
+/// symbol, and nothing when none is named. Its own function so the choice is
+/// asserted directly: the ordering test would pass whichever screen was slow.
+#[cfg(test)]
+fn is_held_back(
+    held: Option<&(std::path::PathBuf, String)>,
+    root: &std::path::Path,
+    symbol: &str,
+) -> bool {
+    held.is_some_and(|(held_root, held_symbol)| held_root == root && held_symbol == symbol)
+}
+
+#[cfg(test)]
+fn pause_if_held_back(root: &std::path::Path, symbol: &str) {
+    let held = HELD_BACK
+        .lock()
+        .unwrap_or_else(std::sync::PoisonError::into_inner)
+        .clone();
+    if is_held_back(held.as_ref(), root, symbol) {
+        std::thread::sleep(std::time::Duration::from_millis(1_500));
+    }
+}
+
 /// One `(combination, side)` the union holds, in first-seen order.
 #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
 pub(crate) struct Candidate {
@@ -321,19 +431,17 @@ fn run(
 /// returned: [`at_least_one_screened`] refuses the verb instead, with every
 /// instrument's reason and the head's `unread` blocks (D-0696).
 ///
-/// **The root is supplied for the head, the union and pass 2, not pass 1.**
-/// Pass 1 screens each instrument through `crate::one_rung`, exactly as
-/// `range-rung` does, and that reads the store root from the environment. An
-/// in-process test therefore drives this function only on a surface that is
-/// empty, where it returns after the head. A surface with an instrument on it
-/// is driven through the verb itself, by
-/// `the_pool_verb_prints_its_whole_page_on_a_generated_store`, from a child
-/// process whose environment names a generated store. That reaches this
-/// function only in a stamped build -- one of a tree equal to HEAD, as a
-/// clean checkout is -- because [`run`] checks the stamp first and an
-/// unstamped build refuses there. The stamp never kept a clean build's test
-/// out of [`run`]; the store root read from the environment is what keeps an
-/// in-process test off a non-empty surface (D-0696).
+/// **The root is supplied for the head, pass 1, the union and pass 2.**
+/// Until D-4700 pass 1 screened through `crate::one_rung`, which reads the
+/// store root from the environment; [`screen_pass_one`] takes this root, the
+/// one [`run`] resolved from that same environment. Pass 1 also takes the
+/// commit stamp, so an unstamped build's in-process call still refuses each
+/// instrument there: this function is driven in-process on an empty surface,
+/// where it returns after the head, and with an instrument on it through the
+/// verb itself, by `the_pool_verb_prints_its_whole_page_on_a_generated_store`,
+/// from a child process whose environment names a generated store, in a
+/// stamped build only. Pass 1 itself is driven in-process through
+/// [`screen_pass_one`] with a stated commit (D-0696, D-4700).
 fn run_under(
     root: &std::path::Path,
     vendor: brutex_core::vendor::Vendor,
@@ -359,18 +467,20 @@ fn run_under(
 
     // ── PASS 1: every instrument, exactly as `range-rung` screens one ──
     //
-    // ONE AT A TIME, IN SURFACE ORDER, as `sweep_rungs` runs rungs. This was a
-    // rayon parallel map over the surface, which wrote every instrument's ledger row and
-    // attempts in thread-completion order (GAP13-13) and ran up to the rayon
-    // pool's width of sweeps at once while nothing raised
-    // `SWEEPS_SHARING_THIS_MACHINE`, so each concurrent sweep took the whole
-    // machine's ceiling and every core (R9-cli-o1-0). With one sweep in flight
-    // the counter's 1 is the truth, and each sweep's own support lanes and
-    // pricing still use every core. D-1701.
-    let screened: Vec<Screened> = crate::in_input_order(&surface, |symbol| Screened {
-        symbol: symbol.clone(),
-        outcome: crate::one_rung(vendor_word, symbol, rung, from, to, support_ppm, None).outcome,
-    });
+    // ONE AT A TIME, IN SURFACE ORDER, in the function `pool-oos` shares
+    // (D-1701, D-4700). This was a rayon parallel map over the surface until
+    // D-1701, which filed every instrument's ledger row and attempts in
+    // thread-completion order (GAP13-13) and gave each concurrent sweep the
+    // whole machine (R9-cli-o1-0); see `screen_pass_one`.
+    let screened = screen_pass_one(
+        root,
+        crate::commit_stamp(),
+        vendor_word,
+        &surface,
+        rung,
+        (from, to),
+        support_ppm,
+    );
     let screened_ok = screened.iter().filter(|s| s.outcome.is_ok()).count();
     crate::note(
         &telemetry::Event::info("cli.pool", "pass 1 finished")
@@ -406,10 +516,7 @@ fn run_under(
             .with("candidates", count(union.len()))
             .with("instruments", count(surface.len())),
     );
-    let priced: Vec<Result<Vec<Priced>, String>> = surface
-        .par_iter()
-        .map(|symbol| price_all(root, vendor, symbol, rung, from, to, &union))
-        .collect();
+    let priced = price_surface(root, vendor, &surface, rung, (from, to), &union);
     let rules = crate::Rules::operator();
     let pooled = fold(&union, &surface, &priced, rules);
     crate::note(
@@ -429,6 +536,26 @@ fn run_under(
     Ok(out)
 }
 
+/// Pass 2's pricing: [`price_all`] for every surface instrument, in parallel
+/// over INSTRUMENTS with an indexed `collect`, so the result is in surface
+/// order whatever the thread count. It writes nothing, which is why it may be
+/// parallel where pass 1 may not. Its own function so pass 1's caller holds
+/// no parallel spelling (`every_caller_of_a_recording_rung_kernel_runs_it_in_input_order`,
+/// D-4700).
+fn price_surface(
+    root: &std::path::Path,
+    vendor: brutex_core::vendor::Vendor,
+    surface: &[String],
+    rung: &'static str,
+    (from, to): ((u16, u8), (u16, u8)),
+    union: &[Candidate],
+) -> Vec<Result<Vec<Priced>, String>> {
+    surface
+        .par_iter()
+        .map(|symbol| price_all(root, vendor, symbol, rung, from, to, union))
+        .collect()
+}
+
 /// Everything the page says before a bar is read, and the surface pass 1
 /// screens, with the store root supplied rather than read from the
 /// environment. D-0696.
@@ -2216,8 +2343,12 @@ mod tests {
     /// test stayed green. Everything after the stamp, feed, rung and root
     /// checks is now `run_under`, which this drives on a scratch store whose
     /// surface is empty, where it returns after the head. A surface with an
-    /// instrument on it is screened through `one_rung`, which reads the root
-    /// from the environment, so no in-process test drives that path.
+    /// instrument on it is screened by `screen_pass_one`, under the root
+    /// handed here and the build's commit stamp, so an unstamped build's
+    /// in-process call refuses each instrument; pass 1 itself is driven
+    /// in-process through `screen_pass_one` with a stated commit (D-4700;
+    /// until then it screened through `one_rung`, which read the root from
+    /// the environment).
     /// `the_pool_verb_prints_its_whole_page_on_a_generated_store` drives it
     /// from a child process, in a stamped build only, and here it is held by
     /// the shape of the source in every build: `out` is bound from
@@ -2716,7 +2847,8 @@ mod tests {
     /// commit-stamp check, was false: the build script stamps a tree equal to
     /// HEAD, and a clean checkout, CI's among them, is one. What keeps an
     /// in-process test off a surface with an instrument on it is the store
-    /// root, which `run` and pass 1's `one_rung` read from the environment.
+    /// root, which `run` reads from the environment and hands to pass 1
+    /// (until D-4700 pass 1's `one_rung` read it there itself).
     /// So this test runs itself again as a child, as
     /// `public_generated_probe_and_screen_agree_with_durable_results` runs
     /// itself. The child inherits no `BRUTEX_` variable from the shell that
@@ -3817,6 +3949,101 @@ mod tests {
         assert!(!root.exists(), "a refused feed creates no tree");
     }
 
+    /// `run` on a rayon pool of exactly `threads`, so a verdict about order
+    /// never depends on the width of the machine running the test.
+    fn on_pool<R: Send>(threads: usize, run: impl FnOnce() -> R + Send) -> R {
+        rayon::ThreadPoolBuilder::new()
+            .num_threads(threads)
+            .build()
+            .expect("a pool")
+            .install(run)
+    }
+
+    /// **Pass 1 -- `pool`'s and `pool-oos`'s -- files every instrument's
+    /// ledger row and attempts in SURFACE order, whatever order its screens
+    /// would finish in, on a four-thread pool and on a one-thread pool.**
+    /// G1-1, D-4700.
+    ///
+    /// The surface's first instrument is held back. `pool-oos` screened its
+    /// surface as a rayon parallel map, so on any pool wider than one thread
+    /// the held-back instrument was filed LAST: its ledger row was not row 0
+    /// and its attempt token was not the smallest.
+    #[test]
+    fn pass_one_files_in_surface_order_on_any_pool_width() {
+        let _knobs = crate::knobs::serially();
+        crate::knobs::clear_all();
+        crate::knobs::set("BRUTEX_VALIDATE", "0");
+        for threads in [4, 1] {
+            crate::audited_stored::with_warmed_store_of(
+                &["NIFTY", "BANKNIFTY", "RELIANCE"],
+                |root| {
+                    let span = ((2025, 5), (2025, 5));
+                    let (_, surface, _) =
+                        super::head_under(root, "zerodha", "5min", span.0, span.1, None)
+                            .expect("the head");
+                    assert_eq!(surface, ["BANKNIFTY", "NIFTY", "RELIANCE"], "premise");
+                    super::hold_back(Some((root, "BANKNIFTY")));
+                    let screened = on_pool(threads, || {
+                        super::screen_pass_one(
+                            root,
+                            Some("generated-pass-one-order"),
+                            "zerodha",
+                            &surface,
+                            "5min",
+                            span,
+                            Some(600_000),
+                        )
+                    });
+                    super::hold_back(None);
+                    let names: Vec<&str> = screened.iter().map(|s| s.symbol.as_str()).collect();
+                    assert_eq!(names, surface, "{threads} thread(s)");
+                    let mut ledger = crate::results::Results::open_read(root).expect("ledger");
+                    assert_eq!(ledger.len().expect("rows"), 3, "{threads} thread(s)");
+                    let mut previous = 0;
+                    for (index, screen) in (0_u64..).zip(&screened) {
+                        let record = screen.outcome.as_ref().expect("each instrument records");
+                        let row = ledger.read(index).expect("row");
+                        assert_eq!(
+                            row.identity, record.identity,
+                            "{threads} thread(s): ledger row {index} is surface instrument {index}"
+                        );
+                        assert_eq!(
+                            crate::results::read_field(&row.underlying),
+                            screen.symbol,
+                            "{threads} thread(s)"
+                        );
+                        let evidence =
+                            crate::sweep_evidence::read(root, record.identity, 1_048_576)
+                                .expect("evidence")
+                                .expect("its attempt");
+                        assert!(
+                            evidence.attempt > previous,
+                            "{threads} thread(s): attempt tokens rise in surface order"
+                        );
+                        previous = evidence.attempt;
+                    }
+                },
+            );
+        }
+        crate::knobs::clear_all();
+    }
+
+    /// The seam holds back exactly the named root and instrument, and nothing
+    /// when none is named.
+    #[test]
+    fn the_pass_one_seam_holds_back_exactly_the_named_screen() {
+        let root = std::path::Path::new("/a");
+        let held = (root.to_path_buf(), "NIFTY".to_owned());
+        assert!(super::is_held_back(Some(&held), root, "NIFTY"));
+        assert!(!super::is_held_back(Some(&held), root, "BANKNIFTY"));
+        assert!(!super::is_held_back(
+            Some(&held),
+            std::path::Path::new("/b"),
+            "NIFTY"
+        ));
+        assert!(!super::is_held_back(None, root, "NIFTY"));
+    }
+
     /// **`in_input_order` runs one call at a time, in input order, even when
     /// the first is the slowest.** GAP13-13, R9-cli-o1-0, D-1701.
     #[test]
@@ -3838,37 +4065,47 @@ mod tests {
         assert!(crate::in_input_order(&[] as &[u8], |_| 0_u8).is_empty());
     }
 
-    /// Both outer loops over `one_rung` -- `range-all`'s and pool pass 1's --
-    /// go through `in_input_order` and neither is a `par_iter`. Each
-    /// `one_rung` writes durable rows from inside the kernel, so a parallel
-    /// outer loop writes them in completion order (GAP13-13) and runs several
-    /// whole-machine sweeps at once (R9-cli-o1-0). D-1701.
+    /// **Every outer loop over a recording rung kernel goes through
+    /// `in_input_order`, and no caller of one -- found by scanning every file
+    /// under `src`, not a list -- spells parallel work.** `range-all`'s
+    /// `sweep_rungs` and pass 1's `screen_pass_one` are the two loops; `pool`
+    /// and `pool-oos` both reach pass 1 through `screen_pass_one`, and
+    /// neither holds a parallel spelling of its own. Each kernel writes
+    /// durable rows from inside itself, so a parallel caller writes them in
+    /// completion order (GAP13-13) and runs several whole-machine sweeps at
+    /// once (R9-cli-o1-0). This read two named files and missed `pool-oos`'s
+    /// pass 1 (G1-1). D-1701, D-4700.
     #[test]
     fn every_outer_loop_over_one_rung_runs_in_input_order() {
-        let body = |source: &'static str, head: &str| -> &'static str {
-            let from = source.find(head).expect("the function");
-            source
-                .get(from..)
-                .and_then(|rest| rest.find("\n}\n").and_then(|to| rest.get(..to)))
-                .expect("its body")
+        use crate::ordered::tests::{parallel_in, recording_callers, sources};
+        let callers = recording_callers(&sources()).expect("every caller is found");
+        let find = |file: &str, name: &str, level: u8| {
+            callers
+                .iter()
+                .find(|c| c.file == file && c.name == name && c.level == level && !c.test)
+                .unwrap_or_else(|| unreachable!("{file} {name} level {level}: {callers:#?}"))
         };
-        let rungs = body(include_str!("lib.rs"), "\nfn sweep_rungs(");
-        assert!(rungs.contains("in_input_order(rungs,") && !rungs.contains("par_iter"));
-        assert!(
-            !rungs.contains("SharedBy::these"),
-            "one sweep in flight shares nothing"
-        );
-        let pool = body(include_str!("pool.rs"), "\nfn run_under(");
-        let pass_1 = pool
-            .split_once("PASS 1:")
-            .and_then(|(_, rest)| rest.split_once("PASS 2:"))
-            .expect("pass 1")
-            .0;
-        assert!(
-            pass_1.contains("crate::in_input_order(&surface,"),
-            "{pass_1}"
-        );
-        assert!(pass_1.contains("crate::one_rung("));
-        assert!(!pass_1.contains("par_iter"), "{pass_1}");
+        for (file, name, shape) in [
+            ("lib.rs", "sweep_rungs", "in_input_order(rungs,"),
+            (
+                "pool.rs",
+                "screen_pass_one",
+                "crate::in_input_order(surface,",
+            ),
+        ] {
+            let body = &find(file, name, 1).body;
+            assert!(body.contains(shape), "{file} {name}:\n{body}");
+            assert!(
+                !body.contains("SharedBy::these"),
+                "one sweep in flight shares nothing"
+            );
+        }
+        for file in ["pool.rs", "pool_oos.rs"] {
+            let body = &find(file, "run_under", 2).body;
+            assert!(body.contains("screen_pass_one("), "{file}");
+        }
+        for caller in &callers {
+            assert_eq!(parallel_in(&caller.body), None, "{caller:#?}");
+        }
     }
 }
diff --git a/crates/cli/src/pool_oos.rs b/crates/cli/src/pool_oos.rs
index 123fc54f..477a5d55 100644
--- a/crates/cli/src/pool_oos.rs
+++ b/crates/cli/src/pool_oos.rs
@@ -77,7 +77,7 @@ use std::path::Path;
 use rayon::prelude::*;
 
 use crate::frontier::Direction;
-use crate::pool::{Candidate, PreparedSpan, Screened};
+use crate::pool::{Candidate, PreparedSpan};
 
 /// A month range, first and last month inclusive, as `(year, month)`.
 type Months = ((u16, u8), (u16, u8));
@@ -573,22 +573,19 @@ fn run_under(
     if surface.is_empty() {
         return Ok(out);
     }
-    let screened: Vec<Screened> = surface
-        .par_iter()
-        .map(|symbol| Screened {
-            symbol: symbol.clone(),
-            outcome: crate::one_rung(
-                vendor_word,
-                symbol,
-                rung,
-                training.0,
-                training.1,
-                support_ppm,
-                None,
-            )
-            .outcome,
-        })
-        .collect();
+    // PASS 1 IS `pool`'s, CALLED, NOT COPIED: one instrument at a time in
+    // surface order. This was a rayon parallel map around `one_rung` (G1-1,
+    // D-4700), so every screen's durable rows landed in thread-completion
+    // order and each concurrent sweep took the whole machine.
+    let screened = crate::pool::screen_pass_one(
+        root,
+        crate::commit_stamp(),
+        vendor_word,
+        &surface,
+        rung,
+        training,
+        support_ppm,
+    );
     crate::pool::at_least_one_screened(&screened, &unread)?;
     crate::pool::render_per_symbol(&mut out, &screened);
     let (union, unread) = crate::pool::union_of(root, &screened);
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 31e467ea..85d6066f 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7052,3 +7052,10 @@ old line regex the same input and watched it pass.
 | G18-api-27 | The seek path's `records unreadable` line names the first file that refused a record, not the first file read (D-2046) | `api::bars::window_tests::the_unreadable_line_names_the_first_damaged_file_not_the_first_file` | ✓ |
 | G18-api-28 | The route test's HTTP exchange is bounded at 30 s per read and write, so a server that admits or answers nothing fails it rather than hanging (D-2047) | `api::ingest::route_tests::the_three_routes_answer_and_none_of_them_shadows_the_front_end` | ✓ |
 | G18-api-29 | A dropped calendar `Landing` marks its flight `Abandoned` (or answered), removes it from the flight table, and wakes every follower (D-2047) | `api::calendar_of::tests::a_calendar_landing_releases_its_flight_and_wakes_its_followers_when_dropped` | ✓ |
+
+### Lane 1-b fixer A: pass-1 order, refused terminals and the pool's own tests (D-4700 onward)
+
+| Id | Invariant | Proof | |
+|---|---|---|---|
+| L1FA-01 | `pool` and `pool-oos` pass 1 is one function, `pool::screen_pass_one`. It screens one instrument at a time in surface order, under the root `run` resolved and the stamp it is handed, and raises no `SharedBy`. With the first instrument held back, on an explicit 4-thread pool and a 1-thread pool, ledger row i is screen i by identity and by instrument, and the attempt tokens rise. AF-39's "screened through `one_rung`, which reads the root from the environment" held until this entry (D-4700) | `cli::pool::tests::pass_one_files_in_surface_order_on_any_pool_width`, `cli::pool::tests::the_pass_one_seam_holds_back_exactly_the_named_screen` | ✓ |
+| L1FA-02 | No non-test function in `crates/cli/src` that calls `one_rung(` or `one_rung_cached(`, and no non-test caller of such a function, spells a parallel primitive, and no first-level caller raises `SharedBy::these`. The scan reads the crate's files at test time and must find the callers it names. It refuses a wrapper name defined in two files, and it does not count a definition, a method, a comment or a string as a call (D-4700) | `cli::ordered::tests::every_caller_of_a_recording_rung_kernel_runs_it_in_input_order`, `cli::ordered::tests::the_recording_caller_scan_sees_calls_and_nothing_else`, `cli::pool::tests::every_outer_loop_over_one_rung_runs_in_input_order` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 552ce57a..9652cc07 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65025,3 +65025,92 @@ on a live leader would derive a second time and lose the single-flight
 guarantee D-1443 exists for. **Honest limit:** the `Landing` kill depends on
 test order. A rename that sorted a single-flight test ahead of it would
 restore the timeout, so the ordering is pinned in the test's own doc.
+
+### D-4700 — `pool-oos` pass 1 is `pool`'s pass 1, one instrument at a time in surface order, and the order guard reads every caller of a recording rung kernel — 2026-10-09
+
+**What was wrong.** G1-1, which leaves GAP13-13 and R9-cli-o1-0 PARTIAL.
+`pool_oos::run_under` screened its surface with
+`surface.par_iter().map(|symbol| Screened { outcome: crate::one_rung(..) })`.
+That is the shape D-1701 removed from `pool`. `pool-oos` was added after
+D-1701 (D-1576) with its own copy of pass 1. Every screen writes durable
+records from inside itself: preparation and probe attempts, frontier, trade
+and receipt blocks, its `runs.bin` row and its terminals. Here they were all
+written from rayon workers, so they landed in thread-completion order. Up to
+the pool's width of sweeps also ran at once, while
+`SWEEPS_SHARING_THIS_MACHINE` stayed at its resting 1, so each one took the
+whole machine's candidate ceiling and every core. D-1709 measured that
+oversubscription: eight such sweeps claimed 157 GB of 48.
+
+Neither guard could see it.
+`ordered::tests::every_whole_command_fan_out_writes_in_input_order` and
+`pool::tests::every_outer_loop_over_one_rung_runs_in_input_order` each named
+fixed files and functions, and neither read `pool_oos.rs`.
+
+**Decided.**
+- **One pass-1 function serves both verbs.** `pool::screen_pass_one` is
+  `crate::in_input_order(surface, ..)` around `one_rung_cached`. It takes the
+  root `run` resolved and the commit stamp as arguments, and raises no
+  `SharedBy`. That is D-1709's shape: one sweep in flight, the counter's 1
+  is true, and each sweep's own support lanes and pricing use every core.
+  `pool::run_under` and `pool_oos::run_under` both call it.
+  - `pool`'s pass 2 moved into `pool::price_surface`, so no caller of a
+    recording kernel spells parallel work itself.
+- **The guard is exhaustive within its reach.**
+  `ordered::tests::every_caller_of_a_recording_rung_kernel_runs_it_in_input_order`
+  reads every `.rs` file under `crates/cli/src` at test time.
+  - It finds each call of `one_rung(` and `one_rung_cached(`. A definition, a
+    method, a longer name, a comment and a string literal are not calls.
+  - For each call it finds the enclosing function. It then finds each
+    non-test caller of those functions, one level up.
+  - No body at either level may spell a parallel primitive: `par_iter`,
+    `par_bridge`, `par_chunks`, `par_extend`, `par_drain`, `rayon::join`,
+    `rayon::scope`, `rayon::spawn`, `ThreadPoolBuilder`, `thread::spawn`,
+    `thread::scope` or `ordered::map`.
+  - No first-level caller may raise `SharedBy::these`.
+  - A wrapper whose name is defined in two files is refused, not guessed.
+  - The test requires the callers it knows to be found, so a scanner that
+    found nothing would fail.
+  - `the_recording_caller_scan_sees_calls_and_nothing_else` holds the scanner
+    to a synthetic source.
+  - `pool::tests::every_outer_loop_over_one_rung_runs_in_input_order` now
+    reads its callers through the same scanner instead of a list.
+- **A behavioural test drives the order.**
+  `pool::tests::pass_one_files_in_surface_order_on_any_pool_width` runs
+  `screen_pass_one` over three instruments, with the first one held back
+  1.5 s by a test seam. It runs once on an explicit 4-thread pool and once
+  on a 1-thread pool. On each, ledger row i must be screen i, by identity and
+  by instrument, and the attempt tokens must rise.
+  `the_pass_one_seam_holds_back_exactly_the_named_screen` pins the seam.
+- **Two stale texts are corrected.** `pool.rs`'s module header said pass 1
+  screens "in parallel". Two test docs said pass 1 reads its root from the
+  environment.
+
+**Measured before the fix.** On the unfixed tree the scanner test failed at
+`ordered_tests.rs:471` with "pool.rs `run_under` calls a recording rung
+kernel and spells parallel work". That body held pass 2's `par_iter` beside
+pass 1, which is why pass 2 moved out. With only that moved, the test failed
+with the same message for `pool_oos.rs`'s `run_under`, which is the defect.
+The order test, with `screen_pass_one`'s map made parallel, failed at
+`pool.rs:4051` with "4 thread(s): ledger row 0 is surface instrument 0".
+
+**What changes in results.** Nothing stored changes shape, digest or
+identity. It is the same `one_rung_cached` call under the same root and the
+same stamp: `run` resolves the root from the environment and hands it
+through, where `one_rung` used to resolve it again from that same
+environment. Only two things change: the order in which `pool-oos` pass 1
+appends, and how many sweeps it runs at once. A `pool-oos` pass 1 now costs
+the sum of its screens rather than overlapping them. NOT MEASURED.
+
+**Rejected.**
+- `SharedBy::these(n)` around a parallel pass 1. D-1709 rejected this: the
+  divided ceiling enters `policy_of` and so the run identity, and each screen
+  would then record a run that differs from `range-rung`'s.
+- Adding `pool_oos.rs` to the fixed lists. The defect was a file the list did
+  not name, and the next one would be another.
+
+**Honest limit.** The scanner reads text, not a syntax tree.
+- It follows callers two levels up. A third wrapper level, a call through a
+  function pointer or a macro, or a parallel primitive not on its list would
+  not be seen.
+- It relies on rustfmt's layout: a function closes with `}` at its own
+  indentation.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 12e9bf09..db04582a 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -8774,7 +8774,8 @@ made exact by exposing each cell's trade sequence from the grid, which is a
 change inside the pricing loop and is not made. Every report labels the
 column as a bound.
 
-Cost: pass 1 is I screens, one at a time in surface order (D-1701), each
+Cost: pass 1 is I screens, one at a time in surface order (D-1701), in
+`pool::screen_pass_one`, which `pool-oos` shares since D-4700, each
 what `range-rung` costs on that instrument, with that screen's sweep and
 pricing parallel inside it. The union admits the parent ledger and receipt
 sidecar once, O(L + R) for L ledger rows and R receipts, then reads one
@@ -12995,8 +12996,13 @@ not:
   to `runs.bin` follow input order. D-1709 keeps this shape over D-1556's
   ordered lanes for those two loops. The Boolean family pools run as
   `ordered::map` lanes (D-1556, "Ordered lanes" below), so their writes follow
-  input order too. Reports are gathered in input order and every ledger
-  lookup is by identity. A plain `descend` step's cost is stated under
+  input order too. `pool-oos` pass 1 was a parallel map around `one_rung`
+  from D-1576 until D-4700 (G1-1), so its rows followed thread completion and
+  its concurrent sweeps each took the whole machine; it now calls
+  `pool::screen_pass_one`, the one function `pool` pass 1 runs, and an order
+  guard reads every caller of `one_rung`/`one_rung_cached` in the crate
+  rather than a list of files. Reports are gathered in input order and every
+  ledger lookup is by identity. A plain `descend` step's cost is stated under
   "Plain `descend` (D-1557)" below, which replaced the D-1567 statement here.
 - **`latest_for` (D-1567, removed by D-1700).** It was O(runs) per call: it
   opened the results ledger, which builds the identity index and hashes the
@@ -15162,6 +15168,13 @@ most 100,000).
   on a rung above one minute they are coarser than the exit grid's minute
   replay. One later span is one draw. Multiplicity across separate
   `pool-oos` invocations is not controlled (gaps-12).
+- **`pool-oos` pass 1 (D-4700).** One screen at a time in surface order,
+  through `pool::screen_pass_one`, so each screen is what `range-rung` costs
+  on that instrument, with its sweep and pricing parallel inside it, and pass
+  1 costs the sum of I screens. Until D-4700 it was a rayon map of up to the
+  pool's width of whole-machine sweeps at once, with nothing raising
+  `SWEEPS_SHARING_THIS_MACHINE` (G1-1): the oversubscription D-1709 measured
+  at 157 GB claimed of 48. The wall-clock change is NOT MEASURED.
 - **`pool-oos` memory (D-2300).** The spans are streamed: each lane prepares
   one span, walks every union candidate over it, and drops its bars and
   column before the next, so at most one span per running Rayon lane is
@@ -15460,7 +15473,8 @@ The rollback on a failed append is one `seek`, one `set_len` and one
   chunk's slowest month plus its sequential filing (one ledger append and one
   terminal per month), not the parallel makespan of the whole walk. Memory per
   chunk is what one month per worker holds, as before.
-- **`range-all` and pool pass 1 run `one_rung` one call at a time.** Each
+- **`range-all` and pool pass 1 run `one_rung` one call at a time** (and
+  `pool-oos` pass 1, the same function since D-4700). Each
   sweep's support lanes and each screen's candidate pricing are still
   parallel, and each call now gets the whole machine's ceiling and cores; what
   no longer overlaps is each rung's or instrument's span loads, column folds
-- 
2.43.0


From fe8227be08e40dc8d79094796e320a7048de415a Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:22:03 +0000
Subject: [PATCH 2/5] cli: sweep-all seals column refusals in input order;
 order tests pin their pools

G1-2. Before: when a sweep-all month began and then its column build
refused, `batch::sweep_prepared` dropped the begun Attempt inside the rayon
worker. Its Drop journaled the Refused terminal to the shared attempts.bin
from there, in completion order.

After:
- `sweep_prepared` returns the attempt with the refusal.
- Phase 4, in input order, seals it with `refuse_begun`, which calls
  `finish(Completion::Refused)`. A failed seal is appended to the row's
  reason, never dropped.

GAP13-13 test gaps:
- The chunk order test now builds a 3-thread and a 1-thread pool itself.
  Before, a 1-thread global pool could not see a revert.
- New: a `begin_many` refusal partway through a chunk. Month 0 sweeps and
  files; months 1 and 2 refuse with the reason, by identity.
- New: column refusals in a chunk journal their terminals in input order.
- New: a direct `refuse_begun` test of the sealed and failed-seal paths.

Fail-before: with the first month held back, the column-refusal order test
failed at batch_stored_tests.rs:380 with "each begun month's Refused
terminal, in input order". The other three tests are proven against
temporary mutants; D-4701 and D-4702 record each one's failure.

D-4701, D-4702, L1FA-03..06; docs/06 ordered stored commands.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/audited_stored.rs       |   4 +-
 crates/cli/src/audited_stored_tests.rs |  71 ++++--
 crates/cli/src/batch.rs                |  55 ++++-
 crates/cli/src/batch_stored_tests.rs   | 330 +++++++++++++++++++++----
 docs/04-invariants.md                  |   4 +
 docs/05-decisions.md                   |  89 +++++++
 docs/06-limits.md                      |   5 +-
 7 files changed, 479 insertions(+), 79 deletions(-)

diff --git a/crates/cli/src/audited_stored.rs b/crates/cli/src/audited_stored.rs
index 614c5914..2b484fed 100644
--- a/crates/cli/src/audited_stored.rs
+++ b/crates/cli/src/audited_stored.rs
@@ -248,4 +248,6 @@ fn error(why: impl std::fmt::Display) -> String {
 mod tests;
 
 #[cfg(test)]
-pub(crate) use tests::{with_unsourceable_close, with_warmed_store, with_warmed_store_of};
+pub(crate) use tests::{
+    with_unsourceable_close, with_unsourceable_close_of, with_warmed_store, with_warmed_store_of,
+};
diff --git a/crates/cli/src/audited_stored_tests.rs b/crates/cli/src/audited_stored_tests.rs
index 7f1a83d2..f9813963 100644
--- a/crates/cli/src/audited_stored_tests.rs
+++ b/crates/cli/src/audited_stored_tests.rs
@@ -51,29 +51,32 @@ pub(crate) fn with_unsourceable_close<R>(
     cut: usize,
     run: impl FnOnce(&std::path::Path, i64) -> R,
 ) -> R {
-    const SHORT_DAY: u8 = 6;
     let fixture = Fixture::for_symbol("NIFTY");
-    let mut short_day = None;
-    for day in 5..=13 {
-        let rows = generated_session(5, day);
-        if rows.is_empty() {
-            continue;
-        }
-        let minutes = if day == SHORT_DAY {
-            short_day = rows.first().map(|bar| indicators::ist_day(bar.ts_micros));
-            &rows[..rows.len().saturating_sub(cut)]
-        } else {
-            &rows[..]
-        };
-        fixture.write(5, Timeframe::MINUTE_1, minutes);
-        fixture.write(5, Timeframe::DAY_1, &rows[..1]);
-        fixture.write(
-            5,
-            Timeframe::MINUTE_5,
-            &rows.iter().step_by(5).copied().collect::<Vec<_>>(),
-        );
+    let short_day = fixture.warm_cutting_one_close(cut);
+    run(&fixture.root, short_day)
+}
+
+/// [`with_unsourceable_close`] for every symbol in `symbols`, in one root:
+/// each symbol's months written exactly as that function writes NIFTY's. The
+/// first symbol's fixture owns (and removes) the root, as in
+/// [`with_warmed_store_of`]. G1-2, D-4701.
+pub(crate) fn with_unsourceable_close_of<R>(
+    symbols: &[&'static str],
+    cut: usize,
+    run: impl FnOnce(&std::path::Path) -> R,
+) -> R {
+    let (&first, rest) = symbols.split_first().expect("at least one symbol");
+    let owner = Fixture::for_symbol(first);
+    owner.warm_cutting_one_close(cut);
+    for &symbol in rest {
+        let other = std::mem::ManuallyDrop::new(Fixture {
+            root: owner.root.clone(),
+            symbol,
+        });
+        other.seed();
+        other.warm_cutting_one_close(cut);
     }
-    run(&fixture.root, short_day.expect("2025-05-06 is a session"))
+    run(&owner.root)
 }
 
 struct Fixture {
@@ -137,6 +140,32 @@ impl Fixture {
             );
         }
     }
+    /// [`Self::warm`], with the one-minute series of 2025-05-06 stopping `cut`
+    /// minutes early; returns that day's IST day number. D-1707.
+    fn warm_cutting_one_close(&self, cut: usize) -> i64 {
+        const SHORT_DAY: u8 = 6;
+        let mut short_day = None;
+        for day in 5..=13 {
+            let rows = generated_session(5, day);
+            if rows.is_empty() {
+                continue;
+            }
+            let minutes = if day == SHORT_DAY {
+                short_day = rows.first().map(|bar| indicators::ist_day(bar.ts_micros));
+                &rows[..rows.len().saturating_sub(cut)]
+            } else {
+                &rows[..]
+            };
+            self.write(5, Timeframe::MINUTE_1, minutes);
+            self.write(5, Timeframe::DAY_1, &rows[..1]);
+            self.write(
+                5,
+                Timeframe::MINUTE_5,
+                &rows.iter().step_by(5).copied().collect::<Vec<_>>(),
+            );
+        }
+        short_day.expect("2025-05-06 is a session")
+    }
     fn path(&self, month: u8, timeframe: Timeframe) -> PathBuf {
         let key = stored::swept_index(self.symbol).expect("key");
         StorePath::for_key(
diff --git a/crates/cli/src/batch.rs b/crates/cli/src/batch.rs
index 51a7fa7c..015aebc2 100644
--- a/crates/cli/src/batch.rs
+++ b/crates/cli/src/batch.rs
@@ -516,9 +516,12 @@ fn held_back(slow: Option<&str>, symbol: &str) -> bool {
 /// 2. Begin every identified month's attempt with one `begin_many`, in input
 ///    order, so attempt tokens follow input order.
 /// 3. Build the column and sweep each month under its attempt, in parallel.
-///    The only writes are each attempt's own depth rows, in its own file.
-/// 4. File each swept month -- ledger row, then the attempt's terminal -- one
-///    at a time, in input order.
+///    The only writes are each attempt's own depth rows, in its own file. A
+///    month whose column build refuses carries its attempt OUT of this phase
+///    unfinished (G1-2, D-4701).
+/// 4. File each swept month -- ledger row, then the attempt's terminal -- and
+///    seal each refused month's Refused terminal, one at a time, in input
+///    order.
 ///
 /// Until this, `one` did all four inside the rayon worker, so the ledger rows,
 /// the attempt tokens and the journal's terminals were appended in thread
@@ -526,6 +529,11 @@ fn held_back(slow: Option<&str>, symbol: &str) -> bool {
 /// differently. A begin refusal refuses exactly the months it left without an
 /// attempt, by name; the months it began still sweep.
 ///
+/// Until D-4701 a month whose column build refused in phase 3 dropped its
+/// attempt inside the rayon worker, and the attempt's `Drop` journaled the
+/// Refused terminal to the shared `attempts.bin` from there, in completion
+/// order. It is now sealed in phase 4 with every other terminal.
+///
 /// The cost of the order is a barrier per chunk: a chunk's slowest month holds
 /// the next chunk back. `docs/06-limits.md` states it.
 fn sweep_chunk(root: &std::path::Path, chunk: &[&Held], min_hits: u64, commit: &str) -> Vec<Row> {
@@ -564,26 +572,48 @@ fn sweep_chunk(root: &std::path::Path, chunk: &[&Held], min_hits: u64, commit: &
             }
         })
         .collect();
-    let swept: Vec<Result<Swept<'_>, Row>> = staged
+    let swept: Vec<Result<Swept<'_>, NotSwept>> = staged
         .into_par_iter()
         .map(|staged| {
-            let (p, attempt) = staged?;
+            let (p, attempt) = staged.map_err(|row| (row, None))?;
             #[cfg(test)]
             if held_back(slow.as_deref(), p.held.symbol.as_str()) {
                 std::thread::sleep(std::time::Duration::from_millis(300));
             }
-            sweep_prepared(p, attempt)
+            sweep_prepared(p, attempt).map_err(|(row, attempt)| (row, Some(attempt)))
         })
         .collect();
     swept
         .into_iter()
         .map(|swept| match swept {
             Ok(swept) => file_swept(root, swept, min_hits),
-            Err(row) => row,
+            Err((row, None)) => row,
+            Err((row, Some(attempt))) => refuse_begun(row, *attempt),
         })
         .collect()
 }
 
+/// A month that did not sweep: its row, and the attempt it began, when it
+/// began one. Phase 3 hands the attempt back unfinished so phase 4 seals it
+/// in input order (D-4701). Boxed, because an attempt is several hundred
+/// bytes and every `Err` the phase carries would be that size.
+type NotSwept = (Row, Option<Box<crate::sweep_evidence::Attempt>>);
+
+/// A begun month whose column build refused: its Refused terminal, sealed
+/// here, in input order, rather than by the attempt's `Drop` inside a rayon
+/// worker (G1-2, D-4701). A seal that fails is added to the row's reason;
+/// the attempt's `Drop` then makes its own best-effort Refused terminal, as
+/// it always did.
+fn refuse_begun(mut row: Row, attempt: crate::sweep_evidence::Attempt) -> Row {
+    if let Err(why) = attempt.finish(crate::sweep_evidence::Completion::Refused) {
+        let refused = row.refused.take().unwrap_or_default();
+        row.refused = Some(format!(
+            "{refused}; sealing its refused terminal failed: {why}"
+        ));
+    }
+    row
+}
+
 /// One instrument-month through all four phases of [`sweep_chunk`]: the
 /// tests' door to a single month. Production sweeps whole chunks.
 #[cfg(test)]
@@ -763,11 +793,13 @@ fn prepare<'h>(
     })
 }
 
-/// Builds one month's column and sweeps it under its begun attempt.
+/// Builds one month's column and sweeps it under its begun attempt. A column
+/// that refuses returns the attempt unfinished beside the row, for phase 4 to
+/// seal in input order (D-4701).
 fn sweep_prepared(
     prepared: Prepared<'_>,
     attempt: crate::sweep_evidence::Attempt,
-) -> Result<Swept<'_>, Row> {
+) -> Result<Swept<'_>, (Row, Box<crate::sweep_evidence::Attempt>)> {
     let Prepared {
         held,
         label,
@@ -788,7 +820,10 @@ fn sweep_prepared(
     ) {
         Ok(column) => column,
         Err(why) => {
-            return Err(Row::refused_before(label, Some(id.hex()), why));
+            return Err((
+                Row::refused_before(label, Some(id.hex()), why),
+                Box::new(attempt),
+            ));
         }
     };
     let outcome = Sweeper::new(ladder).run_prepared_streamed_reporting(
diff --git a/crates/cli/src/batch_stored_tests.rs b/crates/cli/src/batch_stored_tests.rs
index f98cec16..7979404f 100644
--- a/crates/cli/src/batch_stored_tests.rs
+++ b/crates/cli/src/batch_stored_tests.rs
@@ -1,6 +1,6 @@
 #![cfg(test)]
 //! Generated finite stores exercise batch publication and its refusal ledger.
-#![allow(clippy::expect_used)]
+#![allow(clippy::expect_used, clippy::indexing_slicing)]
 
 use super::*;
 use crate::results::Results;
@@ -169,74 +169,312 @@ fn batch_publication_failure_is_durable_refusal_and_a_repaired_retry_can_complet
     crate::knobs::clear_all();
 }
 
+/// `run` on a rayon pool of exactly `threads`, so a verdict about order never
+/// depends on the width of the machine running the test. The slow seam is a
+/// thread-local read by `sweep_chunk`'s caller, so a test sets it inside `run`.
+fn on_pool<R: Send>(threads: usize, run: impl FnOnce() -> R + Send) -> R {
+    rayon::ThreadPoolBuilder::new()
+        .num_threads(threads)
+        .build()
+        .expect("a pool")
+        .install(run)
+}
+
+/// The three May months of `rung` in a store [`with_warmed_store_of`] or
+/// [`with_unsourceable_close_of`] wrote, in walk order.
+///
+/// [`with_warmed_store_of`]: crate::audited_stored::with_warmed_store_of
+/// [`with_unsourceable_close_of`]: crate::audited_stored::with_unsourceable_close_of
+fn may_months(root: &Path, rung: &str) -> Vec<Held> {
+    let wanted: Vec<Held> = catalog::walk(root)
+        .expect("fixture census")
+        .held
+        .into_iter()
+        .filter(|held| {
+            held.timeframe.as_str() == rung && held.month.year() == 2025 && held.month.month() == 5
+        })
+        .collect();
+    assert_eq!(wanted.len(), 3, "premise: three May months at {rung}");
+    wanted
+}
+
+/// The identities of the shared journal's TERMINAL rows, in journal order:
+/// each attempt's second row, its first being the start `begin` allocated.
+fn journaled_terminals(root: &Path) -> Vec<[u8; 32]> {
+    let journal = fs::read(
+        root.join("results")
+            .join("sweep-evidence-v1")
+            .join("attempts.bin"),
+    )
+    .expect("the shared journal");
+    let mut seen = std::collections::HashMap::new();
+    let mut terminals = Vec::new();
+    for row in journal.get(16..).expect("a header").chunks(96) {
+        let identity: [u8; 32] = row[8..40].try_into().expect("32 bytes");
+        let token: [u8; 8] = row[..8].try_into().expect("8 bytes");
+        let rows = seen.entry(token).or_insert(0_u8);
+        *rows += 1;
+        if *rows == 2 {
+            terminals.push(identity);
+        }
+    }
+    terminals
+}
+
+/// A 64-character identity as its bytes.
+fn identity_bytes(hex: &str) -> [u8; 32] {
+    let bytes: Vec<u8> = (0..32)
+        .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex"))
+        .collect();
+    bytes.try_into().expect("32 bytes")
+}
+
 /// **A chunk files its months in INPUT order, whatever order they finish
-/// in.** GAP13-13, D-1701.
+/// in, on a three-thread pool and on a one-thread pool.** GAP13-13, D-1701,
+/// D-4702.
 ///
 /// Three instrument-months sweep in one chunk while the FIRST is held back,
 /// so it finishes last. Its ledger row is still row 0, the attempt tokens rise
 /// in input order, and the journal's last terminal is the last month's.
 /// Before D-1701 each worker appended its own row and began its own attempt,
-/// so the held-back month was filed last.
+/// so the held-back month was filed last. The pool is built here: on the
+/// global pool a one-thread runner finished the months in input order anyway
+/// and could not see a revert (GAP13-13 test gap, D-4702).
 #[test]
 fn a_chunk_files_its_months_in_input_order_whatever_order_they_finish() {
+    let _knobs = crate::knobs::serially();
+    crate::knobs::clear_all();
+    for threads in [3, 1] {
+        crate::audited_stored::with_warmed_store_of(&["NIFTY", "BANKNIFTY", "RELIANCE"], |root| {
+            let wanted = may_months(root, "1min");
+            let chunk: Vec<&Held> = wanted.iter().collect();
+            let first = chunk.first().expect("a first month").symbol.clone();
+            let rows = on_pool(threads, || {
+                slow_symbol(Some(first.as_str()));
+                let rows = sweep_chunk(root, &chunk, u64::MAX, COMMIT);
+                slow_symbol(None);
+                rows
+            });
+            assert_eq!(rows.len(), 3);
+            for row in &rows {
+                require_completed(row);
+            }
+            let mut ledger = Results::open_read(root).expect("ledger");
+            assert_eq!(ledger.len().expect("rows"), 3);
+            let mut previous = 0;
+            for (index, row) in (0_u64..).zip(&rows) {
+                let identity = row.identity.as_ref().expect("identity");
+                assert_eq!(
+                    &ledger.read(index).expect("row").identity_hex(),
+                    identity,
+                    "{threads} thread(s): ledger row {index} is input month {index}"
+                );
+                let evidence = sweep_evidence::read(root, identity_bytes(identity), 1_048_576)
+                    .expect("evidence")
+                    .expect("its attempt");
+                assert_eq!(evidence.completion, Completion::Completed);
+                assert!(
+                    evidence.attempt > previous,
+                    "{threads} thread(s): attempt tokens rise in input order"
+                );
+                previous = evidence.attempt;
+            }
+            let last = sweep_evidence::latest(root, 1_048_576)
+                .expect("journal")
+                .expect("a terminal");
+            assert_eq!(
+                Some(crate::identity_hex(&last.identity)),
+                rows.last().and_then(|row| row.identity.clone()),
+                "{threads} thread(s): the last terminal journaled is the last input month's"
+            );
+        });
+    }
+    crate::knobs::clear_all();
+}
+
+/// **A begin that refuses partway through a chunk sweeps exactly the months it
+/// began and refuses the rest by name.** GAP13-13 test gap, D-1701, D-4702.
+///
+/// The second month's `starts.bin` is a directory, so `begin_many` makes the
+/// first month's start durable and then refuses at the second. The first
+/// month still sweeps and is filed; the second and third are refused with the
+/// begin's reason and their identities, and file nothing.
+#[test]
+fn a_begin_refused_partway_sweeps_the_months_it_began_and_names_the_rest() {
     let _knobs = crate::knobs::serially();
     crate::knobs::clear_all();
     crate::audited_stored::with_warmed_store_of(&["NIFTY", "BANKNIFTY", "RELIANCE"], |root| {
-        let wanted: Vec<Held> = catalog::walk(root)
-            .expect("fixture census")
-            .held
-            .into_iter()
-            .filter(|held| {
-                held.timeframe.as_str() == "1min"
-                    && held.month.year() == 2025
-                    && held.month.month() == 5
-            })
-            .collect();
-        assert_eq!(wanted.len(), 3, "premise: three May months");
+        let wanted = may_months(root, "1min");
         let chunk: Vec<&Held> = wanted.iter().collect();
-        slow_symbol(Some(chunk.first().expect("a first month").symbol.as_str()));
+        let second = prepare(root, chunk[1], u64::MAX, COMMIT)
+            .map_err(|row| row.refused)
+            .expect("premise: the second month is identified")
+            .id
+            .hex();
+        fs::create_dir_all(
+            root.join("results")
+                .join("sweep-evidence-v1")
+                .join(&second)
+                .join("starts.bin"),
+        )
+        .expect("an obstructed start index");
         let rows = sweep_chunk(root, &chunk, u64::MAX, COMMIT);
-        slow_symbol(None);
         assert_eq!(rows.len(), 3);
-        for row in &rows {
-            require_completed(row);
-        }
-        let mut ledger = Results::open_read(root).expect("ledger");
-        assert_eq!(ledger.len().expect("rows"), 3);
-        let mut previous = 0;
-        for (index, row) in (0_u64..).zip(&rows) {
-            let identity = row.identity.as_ref().expect("identity");
-            assert_eq!(
-                &ledger.read(index).expect("row").identity_hex(),
-                identity,
-                "ledger row {index} is input month {index}"
-            );
-            let bytes: Vec<u8> = (0..32)
-                .map(|i| u8::from_str_radix(&identity[i * 2..i * 2 + 2], 16).expect("hex"))
-                .collect();
-            let evidence =
-                sweep_evidence::read(root, bytes.try_into().expect("32 bytes"), 1_048_576)
-                    .expect("evidence")
-                    .expect("its attempt");
-            assert_eq!(evidence.completion, Completion::Completed);
+        require_completed(&rows[0]);
+        let why = rows[1]
+            .refused
+            .clone()
+            .expect("the second month is refused");
+        assert!(!why.is_empty());
+        assert_eq!(rows[1].identity.as_deref(), Some(second.as_str()));
+        assert_eq!(
+            rows[2].refused.as_deref(),
+            Some(why.as_str()),
+            "the same begin"
+        );
+        assert!(rows[2].identity.is_some());
+        for row in &rows[1..] {
             assert!(
-                evidence.attempt > previous,
-                "attempt tokens rise in input order"
+                !row.ran && !row.completed && row.bars == 0,
+                "{:?}",
+                row.refused
             );
-            previous = evidence.attempt;
         }
-        let last = sweep_evidence::latest(root, 1_048_576)
-            .expect("journal")
-            .expect("a terminal");
+        let mut ledger = Results::open_read(root).expect("ledger");
+        assert_eq!(
+            ledger.len().expect("rows"),
+            1,
+            "only the begun month is filed"
+        );
         assert_eq!(
-            Some(crate::identity_hex(&last.identity)),
-            rows.last().and_then(|row| row.identity.clone()),
-            "the last terminal journaled is the last input month's"
+            Some(ledger.read(0).expect("row").identity_hex()),
+            rows[0].identity.clone()
         );
     });
     crate::knobs::clear_all();
 }
 
+/// **A month whose column build refuses files its Refused terminal in INPUT
+/// order, as every other month files.** G1-2, D-4701.
+///
+/// Three 5min months each miss the closing minutes of one session, so each is
+/// loaded, identified and begun, and its column build then refuses. The first
+/// is held back. Before D-4701 each refused attempt was dropped inside its
+/// rayon worker, whose `Drop` journaled the Refused terminal there, in thread
+/// completion order: the held-back month's terminal landed last.
+#[test]
+fn a_chunk_files_its_column_refusals_in_input_order() {
+    let _knobs = crate::knobs::serially();
+    crate::knobs::clear_all();
+    crate::audited_stored::with_unsourceable_close_of(
+        &["NIFTY", "BANKNIFTY", "RELIANCE"],
+        5,
+        |root| {
+            let wanted = may_months(root, "5min");
+            let chunk: Vec<&Held> = wanted.iter().collect();
+            let first = chunk.first().expect("a first month").symbol.clone();
+            let rows = on_pool(3, || {
+                slow_symbol(Some(first.as_str()));
+                let rows = sweep_chunk(root, &chunk, u64::MAX, COMMIT);
+                slow_symbol(None);
+                rows
+            });
+            assert_eq!(rows.len(), 3);
+            let mut begun = Vec::new();
+            for row in &rows {
+                assert!(
+                    !row.ran && row.refused.as_ref().is_some_and(|why| !why.is_empty()),
+                    "premise: the column build refused: {:?}",
+                    row.refused
+                );
+                begun.push(identity_bytes(row.identity.as_ref().expect("identified")));
+            }
+            assert!(
+                !Results::path(root).exists(),
+                "a refused month files no row"
+            );
+            assert_eq!(
+                journaled_terminals(root),
+                begun,
+                "each begun month's Refused terminal, in input order"
+            );
+            for identity in begun {
+                let evidence = sweep_evidence::read(root, identity, 1_048_576)
+                    .expect("evidence")
+                    .expect("its attempt");
+                assert_eq!(evidence.completion, Completion::Refused);
+            }
+        },
+    );
+    crate::knobs::clear_all();
+}
+
+/// **A begun month whose column refused is sealed Refused by phase 4, and a
+/// seal that fails is named on its row, not swallowed.** G1-2, D-4701.
+///
+/// `refuse_begun` is driven alone, twice, on one scratch evidence root. A
+/// seal that succeeds leaves the row's reason exactly as the column gave it
+/// and journals the identity's Refused terminal. With the shared journal
+/// replaced by a directory, the seal fails, and the row's reason is the
+/// column's followed by the seal's own failure. Leaving the attempt to its
+/// `Drop` instead -- what phase 3 did inside the worker until D-4701 --
+/// writes the same terminal when it can, so only the failing seal tells the
+/// two apart.
+#[test]
+fn a_refused_months_terminal_is_sealed_and_a_failed_seal_is_named() {
+    let root =
+        std::env::temp_dir().join(format!("brutex-batch-refuse-begun-{}", std::process::id()));
+    let _ = fs::remove_dir_all(&root);
+    fs::create_dir_all(&root).expect("scratch");
+    let refused = |byte: u8| {
+        Row::refused_before(
+            "zerodha NIFTY 5min 2025-05".to_owned(),
+            Some(format!("{byte:02x}").repeat(32)),
+            "the column refused".to_owned(),
+        )
+    };
+    let sealed = [7_u8; 32];
+    let attempt = sweep_evidence::begin(&root, sealed, sweep_evidence::Operation::Sweep)
+        .expect("a begun attempt");
+    let row = refuse_begun(refused(7), attempt);
+    assert_eq!(row.refused.as_deref(), Some("the column refused"));
+    assert!(!row.ran && !row.completed);
+    assert_eq!(journaled_terminals(&root), vec![sealed]);
+    assert_eq!(
+        sweep_evidence::read(&root, sealed, 1_048_576)
+            .expect("evidence")
+            .expect("its attempt")
+            .completion,
+        Completion::Refused
+    );
+
+    let unsealed = [8_u8; 32];
+    let attempt = sweep_evidence::begin(&root, unsealed, sweep_evidence::Operation::Sweep)
+        .expect("a second begun attempt");
+    let journal = root
+        .join("results")
+        .join("sweep-evidence-v1")
+        .join("attempts.bin");
+    let aside = journal.with_extension("aside");
+    fs::rename(&journal, &aside).expect("move the journal aside");
+    fs::create_dir(&journal).expect("obstruct the journal");
+    let row = refuse_begun(refused(8), attempt);
+    fs::remove_dir(&journal).expect("clear the obstruction");
+    fs::rename(&aside, &journal).expect("restore the journal");
+    let why = row.refused.expect("still refused");
+    assert!(
+        why.starts_with("the column refused; sealing its refused terminal failed: ")
+            && why.len() > "the column refused; sealing its refused terminal failed: ".len(),
+        "{why}"
+    );
+    assert_eq!(
+        journaled_terminals(&root),
+        vec![sealed],
+        "no terminal reached the obstructed journal"
+    );
+    fs::remove_dir_all(&root).expect("scratch removed");
+}
+
 /// A chunk whose every month refuses before identification begins nothing and
 /// files nothing; the refusals keep input order. D-1701.
 #[test]
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 85d6066f..f6dc130a 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7059,3 +7059,7 @@ old line regex the same input and watched it pass.
 |---|---|---|---|
 | L1FA-01 | `pool` and `pool-oos` pass 1 is one function, `pool::screen_pass_one`. It screens one instrument at a time in surface order, under the root `run` resolved and the stamp it is handed, and raises no `SharedBy`. With the first instrument held back, on an explicit 4-thread pool and a 1-thread pool, ledger row i is screen i by identity and by instrument, and the attempt tokens rise. AF-39's "screened through `one_rung`, which reads the root from the environment" held until this entry (D-4700) | `cli::pool::tests::pass_one_files_in_surface_order_on_any_pool_width`, `cli::pool::tests::the_pass_one_seam_holds_back_exactly_the_named_screen` | ✓ |
 | L1FA-02 | No non-test function in `crates/cli/src` that calls `one_rung(` or `one_rung_cached(`, and no non-test caller of such a function, spells a parallel primitive, and no first-level caller raises `SharedBy::these`. The scan reads the crate's files at test time and must find the callers it names. It refuses a wrapper name defined in two files, and it does not count a definition, a method, a comment or a string as a call (D-4700) | `cli::ordered::tests::every_caller_of_a_recording_rung_kernel_runs_it_in_input_order`, `cli::ordered::tests::the_recording_caller_scan_sees_calls_and_nothing_else`, `cli::pool::tests::every_outer_loop_over_one_rung_runs_in_input_order` | ✓ |
+| L1FA-03 | A `sweep-all` month whose column build refuses after its attempt began is sealed Refused in the ordered phase. With the first such month held back on a 3-thread pool, the journal's terminals are the begun identities in input order, each Refused, and no ledger row is written (D-4701) | `cli::batch::stored_tests::a_chunk_files_its_column_refusals_in_input_order` | ✓ |
+| L1FA-04 | `batch::refuse_begun` journals the Refused terminal and leaves the column's reason exactly as it was. A seal that fails is appended to the reason, never dropped (D-4701) | `cli::batch::stored_tests::a_refused_months_terminal_is_sealed_and_a_failed_seal_is_named` | ✓ |
+| L1FA-05 | `sweep-all` files a chunk in input order on an explicit 3-thread pool and on a 1-thread pool, built by the test itself, whatever the global pool's width (D-4702) | `cli::batch::stored_tests::a_chunk_files_its_months_in_input_order_whatever_order_they_finish` | ✓ |
+| L1FA-06 | When `begin_many` refuses partway through a chunk, the months it began sweep and file, and every later month refuses with that reason, naming its identity (D-4702) | `cli::batch::stored_tests::a_begin_refused_partway_sweeps_the_months_it_began_and_names_the_rest` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 9652cc07..72dfb280 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65114,3 +65114,92 @@ the sum of its screens rather than overlapping them. NOT MEASURED.
   not be seen.
 - It relies on rustfmt's layout: a function closes with `}` at its own
   indentation.
+
+### D-4701 — A `sweep-all` month whose column build refuses seals its Refused terminal in the ordered phase, and a seal that fails is named — 2026-10-09
+
+**What was wrong.** G1-2. `batch::sweep_chunk` has four phases:
+
+1. Prepare, in parallel.
+2. Begin every attempt with one `begin_many`, in input order.
+3. Build and sweep, in parallel.
+4. File, one month at a time, in input order.
+
+A month can begin and then have its column build refuse. That happens to a
+1-minute series that is missing a session's closing minute, which is
+D-1707's premise; D-1781/D-1707 measured 28 such minutes on NIFTY. When it
+did, `sweep_prepared` returned the refused row and dropped its begun
+`Attempt` inside the rayon worker. `Attempt::drop` then journaled the
+Refused terminal to the shared `attempts.bin` from that worker, in
+completion order. D-1701's rule is that everything durable happens in input
+order.
+
+**Decided.**
+- **The attempt comes out of phase 3.** `sweep_prepared` returns the
+  attempt beside the refusal, as `Err((Row, Attempt))`. Phase 3 carries
+  `(Row, Option<Attempt>)` out; the attempt is `None` for a month that was
+  never begun.
+- **Phase 4 seals it in input order.** `batch::refuse_begun` calls
+  `attempt.finish(Completion::Refused)`. A seal that fails is added to the
+  row's reason, as "; sealing its refused terminal failed: ..", rather than
+  dropped. The attempt's `Drop` then makes its usual best-effort terminal,
+  and that too now happens in the ordered phase.
+- **Two tests.**
+  - `batch::stored_tests::a_chunk_files_its_column_refusals_in_input_order`
+    sweeps three months whose column build refuses, with the first held
+    back, on a 3-thread pool. The journal's terminals must be the begun
+    identities in input order, each must be Refused, and no ledger row is
+    written. The premise, that each refusal came from the column build after
+    a begin, is asserted first.
+  - `batch::stored_tests::a_refused_months_terminal_is_sealed_and_a_failed_seal_is_named`
+    drives `refuse_begun` alone. A seal that succeeds leaves the reason
+    exactly as the column gave it and journals one Refused terminal. With
+    the journal obstructed, the reason carries the seal's failure.
+- **One fixture.** `audited_stored::with_unsourceable_close_of` writes
+  D-1707's short-close store for several symbols in one root.
+
+**Measured before the fix.** On the unfixed tree the order test failed at
+`batch_stored_tests.rs:380` with "each begun month's Refused terminal, in
+input order": the held-back month's terminal was journaled last. The seal
+test has no unfixed form, because `refuse_begun` is new. Against a mutant
+that leaves the attempt to its `Drop`, it failed at
+`batch_stored_tests.rs:465` with the bare reason "the column refused".
+
+**What changes in results.** The terminal bytes and completion are the same.
+Only their order in `attempts.bin` changes, and it now follows the input. No
+format changes.
+
+### D-4702 — The `sweep-all` order test runs on its own 3-thread and 1-thread pools, and a `begin_many` refused partway is driven — 2026-10-09
+
+**What was wrong.** GAP13-13 had two test gaps.
+- `batch::stored_tests::a_chunk_files_its_months_in_input_order_whatever_order_they_finish`
+  ran on the global rayon pool. On a 1-thread runner the months finish in
+  input order whatever the code does, so a revert of D-1701 passed it.
+- No test refused `begin_many` after it had begun part of a chunk. D-1701's
+  rule is that the months it began still sweep, and only the rest are
+  refused, by name. That rule was stated and not driven.
+
+**Decided.**
+- **The order test builds its own pools.** It runs once on a 3-thread pool
+  and once on a 1-thread pool, built inside the test. The held-back seam is
+  thread-local and is read on the thread that calls `sweep_chunk`, so it is
+  set inside each pool. On each pool, ledger row i must be input month i,
+  the attempt tokens must rise, and the journal's last terminal must be the
+  last month's.
+- **The partway refusal is driven.**
+  `batch::stored_tests::a_begin_refused_partway_sweeps_the_months_it_began_and_names_the_rest`
+  obstructs the second month's `starts.bin` with a directory, so
+  `begin_many` admits month 0 and refuses at month 1.
+  - Month 0 must complete and file the chunk's one ledger row.
+  - Months 1 and 2 must refuse with the same reason, each naming its
+    identity.
+
+**Proof.** Both tests passed on the unfixed tree, because the code was
+right. Each one is proven against a temporary mutant, measured with
+`RAYON_NUM_THREADS=1`:
+- **Filing moved back into the parallel phase** (the pre-D-1701 shape). A
+  verbatim copy of the old test passed. The new one failed in its 3-thread
+  half, at `batch_stored_tests.rs:267`, with "3 thread(s): ledger row 0 is
+  input month 0".
+- **A begin refusal refusing every month.** The partway test failed at
+  month 0's completion check, with "sweep evidence I/O refused: Is a
+  directory (os error 21)".
diff --git a/docs/06-limits.md b/docs/06-limits.md
index db04582a..0eb1c8af 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15469,7 +15469,10 @@ The rollback on a failed append is one `seek`, one `set_len` and one
 - **`sweep-all` files in input order behind a barrier per chunk.** Each chunk
   of at most the rayon pool's width loads and sweeps its months in parallel,
   then files them one at a time; the next chunk starts only when the chunk's
-  slowest month is filed. Wall-clock is therefore the sum over chunks of each
+  slowest month is filed. A month whose column build refuses after its
+  attempt began is sealed Refused in that same sequential phase (D-4701),
+  one terminal append, where its attempt's `Drop` had journaled it from the
+  worker. Wall-clock is therefore the sum over chunks of each
   chunk's slowest month plus its sequential filing (one ledger append and one
   terminal per month), not the parallel makespan of the whole walk. Memory per
   chunk is what one month per worker holds, as before.
-- 
2.43.0


From 1fb06b7915ca1bc5dc821e3b90bf16f21922994c Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:22:18 +0000
Subject: [PATCH 3/5] cli: drive one_rung_cached A-B-A, and hold pool pass 2 to
 pass 1's recorded cells

Two test gaps.

W2-cli8-9. Before: both tests of D-1700's read-back call `recorded_row`
directly, so rewiring `one_rung_cached`'s success arm back to a key lookup
passed every test. After:
`audited_stored::tests::a_range_rung_rerun_through_one_rung_cached_returns_its_own_row`
runs `one_rung_cached` as A, then B with A's key, then A again. The rerun
must return A's row, not B's, which is the key's newest.

GAP13-15. Before: `price_all` was held only to a cell that the test module
re-assembles from audit-path functions. That covered the empty mask only,
and never 60min. After: `pool::tests::pass_two_prices_each_frontier_row_as_pass_one_recorded_it`
screens NIFTY with `screen_pass_one`, at 5min (May) and at 60min (March to
May), and reads the frontier rows of the recorded identity. Every row's
`(mask, side)` must get from `price_all` exactly the cell figures it
carries. A non-empty mask must have traded.

The new fixture is `with_varied_store`, a reproducible random walk from
2025-02-28 to May, with its coarse rungs folded by `pull::fold`. One month
of 60min bars halted the ladder at every support from 60% to 99%
(measured); March to May, 213 swept bars, completed.

Fail-before, on temporary mutants:
- With the success arm reading the newest ledger row, the A-B-A test
  failed at audited_stored_tests.rs:2960: the identities differ. The
  direct `recorded_row` test still passed.
- With `price_grids` given one more ladder rung, the cell test failed at
  60min: "60min rank 1 Long: pass 2's cell is pass 1's recorded cell",
  pessimistic -4595 against -4525. The old audit-path test still passed.

D-4703, D-4704, L1FA-07, L1FA-08.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/audited_stored.rs       |   3 +-
 crates/cli/src/audited_stored_tests.rs | 159 +++++++++++++++++++++++++
 crates/cli/src/pool.rs                 | 104 ++++++++++++++++
 docs/04-invariants.md                  |   2 +
 docs/05-decisions.md                   |  88 ++++++++++++++
 5 files changed, 355 insertions(+), 1 deletion(-)

diff --git a/crates/cli/src/audited_stored.rs b/crates/cli/src/audited_stored.rs
index 2b484fed..20910c88 100644
--- a/crates/cli/src/audited_stored.rs
+++ b/crates/cli/src/audited_stored.rs
@@ -249,5 +249,6 @@ mod tests;
 
 #[cfg(test)]
 pub(crate) use tests::{
-    with_unsourceable_close, with_unsourceable_close_of, with_warmed_store, with_warmed_store_of,
+    with_unsourceable_close, with_unsourceable_close_of, with_varied_store, with_warmed_store,
+    with_warmed_store_of,
 };
diff --git a/crates/cli/src/audited_stored_tests.rs b/crates/cli/src/audited_stored_tests.rs
index f9813963..17080678 100644
--- a/crates/cli/src/audited_stored_tests.rs
+++ b/crates/cli/src/audited_stored_tests.rs
@@ -40,6 +40,108 @@ pub(crate) fn with_warmed_store_of<R>(
     run(&owner.root)
 }
 
+/// A NIFTY store whose minutes WALK: 2025-02-28 and every session of March,
+/// April and May 2025, each minute from [`varied_session`], the price walking
+/// on from one session to the next. Each session's 1day bar spans its
+/// minutes, and from March the 5min and 60min months are those minutes folded
+/// by `pull::fold`, the one fold authority, so every signal bar closes on its
+/// last minute's close, as the exact-minute overlay checks.
+///
+/// GAP13-15's 60min rung needs both halves. The warmed store's constant
+/// prices hold every condition on every bar, and a month of 60min bars is too
+/// few: on this walk, May alone halted the 60min ladder at every support from
+/// 60% to 99% (measured), and March to May, 213 swept bars, completed at each.
+/// The warmed store also samples one minute per 5min bar, which only constant
+/// prices let past that overlay. D-4704.
+pub(crate) fn with_varied_store<R>(run: impl FnOnce(&std::path::Path) -> R) -> R {
+    let mut days = vec![(2_u8, 28_u8)];
+    for (month, last) in [(3_u8, 31_u8), (4, 30), (5, 31)] {
+        days.extend((1..=last).map(|date| (month, date)));
+    }
+    with_varied_store_over(&days, &[3, 4, 5], run)
+}
+
+/// [`with_varied_store`] over `days`, each `(month, date)` of 2025 in order,
+/// a date that is no session skipped; the 5min and 60min months are written
+/// for each month in `coarse`. The price walks on from one session's close
+/// to the next session's open.
+fn with_varied_store_over<R>(
+    days: &[(u8, u8)],
+    coarse: &[u8],
+    run: impl FnOnce(&std::path::Path) -> R,
+) -> R {
+    let root = std::env::temp_dir().join(format!(
+        "brutex-audited-varied-fixture-{}-{}",
+        std::process::id(),
+        NEXT.fetch_add(1, Ordering::Relaxed)
+    ));
+    fs::create_dir(&root).expect("scratch");
+    let fixture = Fixture {
+        root,
+        symbol: "NIFTY",
+    };
+    let mut walk = Walk {
+        price: 100_000,
+        state: 0x9E37_79B9_7F4A_7C15,
+    };
+    for &(month, day) in days {
+        let rows = varied_session(month, day, &mut walk);
+        let (Some(first), Some(last)) = (rows.first(), rows.last()) else {
+            continue;
+        };
+        fixture.write(month, Timeframe::MINUTE_1, &rows);
+        let daily = Bar {
+            high: rows.iter().map(|bar| bar.high).max().unwrap_or(first.high),
+            low: rows.iter().map(|bar| bar.low).min().unwrap_or(first.low),
+            close: last.close,
+            volume: rows.iter().map(|bar| bar.volume).sum(),
+            ..*first
+        };
+        fixture.write(month, Timeframe::DAY_1, &[daily]);
+        if coarse.contains(&month) {
+            for (secs, timeframe) in [(300, Timeframe::MINUTE_5), (3_600, Timeframe::MINUTE_60)] {
+                let folded = pull::fold::fold_from_bars(
+                    &rows,
+                    pull::fold::Bucket::of_secs(secs).expect("a width"),
+                    pull::fold::Bucket::MINUTE,
+                )
+                .expect("whole minutes fold");
+                fixture.write(month, timeframe, &folded);
+            }
+        }
+    }
+    run(&fixture.root)
+}
+
+/// The generated price walk [`varied_session`] steps: the price and a
+/// 64-bit linear congruential state.
+struct Walk {
+    price: i64,
+    state: u64,
+}
+
+/// [`generated_session`]'s minutes with a price that walks: each minute
+/// steps by a multiple of 5 paisa in -100..=100, drawn from `walk`'s
+/// generator, so the series is reproducible and neither stands still nor
+/// repeats; every bar spans 15 paisa beyond its open and close.
+fn varied_session(month: u8, date: u8, walk: &mut Walk) -> Vec<Bar> {
+    let mut rows = generated_session(month, date);
+    for bar in &mut rows {
+        walk.state = walk
+            .state
+            .wrapping_mul(6_364_136_223_846_793_005)
+            .wrapping_add(1_442_695_040_888_963_407);
+        let draw = i64::try_from((walk.state >> 33) % 41).expect("below 41");
+        let open = walk.price;
+        walk.price += (draw - 20) * 5;
+        bar.open = open;
+        bar.close = walk.price;
+        bar.high = open.max(walk.price) + 15;
+        bar.low = open.min(walk.price) - 15;
+    }
+    rows
+}
+
 /// A warmed NIFTY store whose one-minute series stops `cut` minutes early on
 /// 2025-05-06, every other file as [`with_warmed_store`] writes it. The cut is
 /// at the session's end, so no interior minute is missing and
@@ -2798,6 +2900,63 @@ fn a_reused_range_rung_reads_its_own_row_and_not_the_newest_with_its_key() {
     crate::knobs::clear_all();
 }
 
+/// **`one_rung_cached` itself, driven through run A, run B and A again, hands
+/// the rerun A's own row.** W2-cli8-9 test gap, D-1700, D-4703.
+///
+/// The test above calls `recorded_row` directly, so rewiring the success arm
+/// of `one_rung_cached` back to a key lookup -- the newest row with the feed,
+/// instrument, rung, span and `min_hits`, as `latest_for` read it -- passed
+/// it. This drives the arm. A and B share every key term and differ by commit,
+/// so B's row is the newest with the key, and A's rerun must come back as A's.
+#[test]
+fn a_range_rung_rerun_through_one_rung_cached_returns_its_own_row() {
+    let _knobs = crate::knobs::serially();
+    crate::knobs::clear_all();
+    crate::knobs::set("BRUTEX_VALIDATE", "0");
+    let fixture = Fixture::warmed();
+    let rung = |commit: &'static str| {
+        crate::one_rung_cached(
+            crate::RungAsk {
+                vendor_word: "zerodha",
+                underlying: fixture.symbol,
+                rung: "5min",
+                from: (2025, 5),
+                to: (2025, 5),
+                support_ppm: Some(600_000),
+                attempt: Some(21),
+            },
+            crate::RungStore {
+                root: Ok(fixture.root.clone()),
+                commit: Some(commit),
+            },
+            &mut crate::AuditCache::default(),
+        )
+        .outcome
+        .map_err(|why| format!("{commit}: {why}"))
+        .expect("the rung records")
+    };
+    let a = rung("generated-one-rung-readback-a");
+    let b = rung("generated-one-rung-readback-b");
+    let again = rung("generated-one-rung-readback-a");
+    assert_ne!(a.identity, b.identity, "premise: two runs");
+    assert_eq!(
+        (a.min_hits, &a.feed, &a.underlying, &a.timeframe),
+        (b.min_hits, &b.feed, &b.underlying, &b.timeframe),
+        "premise: one key"
+    );
+    let mut ledger = crate::results::Results::open_read(&fixture.root).expect("ledger");
+    assert_eq!(ledger.len().expect("rows"), 2, "the rerun appends nothing");
+    assert_eq!(
+        ledger.read(1).expect("newest").identity,
+        b.identity,
+        "premise: the newest row with the key is B's"
+    );
+    assert_eq!(again.identity, a.identity, "the rerun returns its own row");
+    assert_eq!(again, ledger.read(0).expect("A's stored row"));
+    assert_eq!(a, again);
+    crate::knobs::clear_all();
+}
+
 /// The identity a page names is read exactly, or the page is refused: no
 /// block, two blocks, no identity line, uppercase or short hex. D-1700.
 #[test]
diff --git a/crates/cli/src/pool.rs b/crates/cli/src/pool.rs
index 317fa8c1..0b5b3597 100644
--- a/crates/cli/src/pool.rs
+++ b/crates/cli/src/pool.rs
@@ -3754,6 +3754,110 @@ mod tests {
         });
     }
 
+    /// **Pass 2 prices each frontier row exactly as pass 1 RECORDED it, at
+    /// 5min and at 60min.** GAP13-15 test gap, D-1702, D-4704.
+    ///
+    /// The test above holds `price_all` to a cell re-assembled here from the
+    /// audit path's functions, so a drift in what pass 1 records (its tier
+    /// cascade, `price_grids`) would move neither side and pass, and it
+    /// prices the empty mask only. This reads the cells pass 1 actually
+    /// wrote: `screen_pass_one` screens the instrument, and every frontier row
+    /// of its recorded identity is a `(mask, side)` with the money its cell
+    /// carried. `price_all` must return that money for that candidate, and no
+    /// cell for a row that did not trade. Every recorded row is checked, and
+    /// a non-empty mask among them must have traded (a premise). 60min is the
+    /// rung the finding measured; it runs over March to May, because one
+    /// month of 60min bars halts the ladder (see `with_varied_store`). The
+    /// ceiling is named, so the ladder's reach does not follow the machine.
+    #[test]
+    fn pass_two_prices_each_frontier_row_as_pass_one_recorded_it() {
+        let _knobs = crate::knobs::serially();
+        crate::knobs::clear_all();
+        crate::knobs::set("BRUTEX_VALIDATE", "0");
+        crate::knobs::set("BRUTEX_CEILING", "4096");
+        crate::audited_stored::with_varied_store(|root| {
+            let vendor = brutex_core::vendor::Vendor::Zerodha;
+            let surface = ["NIFTY".to_owned()];
+            for (rung, span) in [
+                ("5min", ((2025, 5), (2025, 5))),
+                ("60min", ((2025, 3), (2025, 5))),
+            ] {
+                let screened = super::screen_pass_one(
+                    root,
+                    Some("generated-pass-two-reference"),
+                    "zerodha",
+                    &surface,
+                    rung,
+                    span,
+                    Some(600_000),
+                );
+                let record = screened
+                    .first()
+                    .expect("one screen")
+                    .outcome
+                    .as_ref()
+                    .map_err(|why| format!("{rung}: {why}"))
+                    .expect("premise: pass 1 records");
+                let (rows, damage) = crate::frontier::Frontier::open_read(root)
+                    .expect("frontier")
+                    .of_run(&record.identity)
+                    .expect("its rows");
+                assert!(damage.is_none(), "{damage:?}");
+                let (union, unread) = super::union_of(root, &screened);
+                assert!(unread.is_empty(), "{unread:?}");
+                let priced = super::price_all(root, vendor, "NIFTY", rung, span.0, span.1, &union)
+                    .expect("pass 2 prices the span");
+                let (mut fired, mut fired_non_empty) = (0, 0);
+                for row in &rows {
+                    let at = union
+                        .iter()
+                        .position(|c| c.words == row.mask_words && c.direction == row.direction)
+                        .expect("every frontier row is in the union");
+                    let figures = priced.get(at).copied().flatten().map(|c| {
+                        (
+                            c.trades,
+                            c.wins,
+                            c.pessimistic,
+                            c.worst_trade,
+                            c.max_drawdown,
+                            c.min_win,
+                            c.gross_win,
+                            c.gross_loss,
+                        )
+                    });
+                    let recorded = (row.trades > 0).then_some((
+                        row.trades,
+                        row.cell_wins,
+                        row.pessimistic,
+                        row.worst_trade,
+                        row.max_drawdown,
+                        row.min_win,
+                        row.gross_win,
+                        row.gross_loss,
+                    ));
+                    assert_eq!(
+                        figures, recorded,
+                        "{rung} rank {} {:?}: pass 2's cell is pass 1's recorded cell",
+                        row.rank, row.direction
+                    );
+                    if row.trades > 0 {
+                        fired += 1;
+                        if row.mask_words != [0; 6] {
+                            fired_non_empty += 1;
+                        }
+                    }
+                }
+                assert!(fired > 0, "premise: a recorded {rung} row fired");
+                assert!(
+                    fired_non_empty > 0,
+                    "premise: a non-empty {rung} mask fired: {} row(s)",
+                    rows.len()
+                );
+            }
+        });
+        crate::knobs::clear_all();
+    }
+
     /// **Pass 2 withholds the day pass 1 withholds when that day's closing
     /// minute cannot be sourced, and prices the rest exactly where the audit
     /// path does.** D-1707, closing the difference D-1702 stated.
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index f6dc130a..782e070d 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7063,3 +7063,5 @@ old line regex the same input and watched it pass.
 | L1FA-04 | `batch::refuse_begun` journals the Refused terminal and leaves the column's reason exactly as it was. A seal that fails is appended to the reason, never dropped (D-4701) | `cli::batch::stored_tests::a_refused_months_terminal_is_sealed_and_a_failed_seal_is_named` | ✓ |
 | L1FA-05 | `sweep-all` files a chunk in input order on an explicit 3-thread pool and on a 1-thread pool, built by the test itself, whatever the global pool's width (D-4702) | `cli::batch::stored_tests::a_chunk_files_its_months_in_input_order_whatever_order_they_finish` | ✓ |
 | L1FA-06 | When `begin_many` refuses partway through a chunk, the months it began sweep and file, and every later month refuses with that reason, naming its identity (D-4702) | `cli::batch::stored_tests::a_begin_refused_partway_sweeps_the_months_it_began_and_names_the_rest` | ✓ |
+| L1FA-07 | `one_rung_cached` run as A, then B with A's key, then A again, returns A's own ledger row, not the key's newest (D-4703) | `cli::audited_stored::tests::a_range_rung_rerun_through_one_rung_cached_returns_its_own_row` | ✓ |
+| L1FA-08 | At 5min (May 2025) and at 60min (March to May), on a generated random-walk store, pool pass 2 prices every frontier row pass 1 recorded to exactly the cell figures the row carries, and gives no cell to a row with no trade. A non-empty mask among them traded; the empty mask stays with the older audit-path test (D-4704) | `cli::pool::tests::pass_two_prices_each_frontier_row_as_pass_one_recorded_it` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 72dfb280..00535771 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65203,3 +65203,91 @@ right. Each one is proven against a temporary mutant, measured with
 - **A begin refusal refusing every month.** The partway test failed at
   month 0's completion check, with "sweep evidence I/O refused: Is a
   directory (os error 21)".
+
+### D-4703 — `one_rung_cached` is driven A, then B with the same key, then A again, end to end — 2026-10-09
+
+**What was wrong.** W2-cli8-9 test gap. D-1700 made the success arm of
+`one_rung_cached` read a rung's row back through `recorded_row`, by the
+identity its page names, rather than taking the newest row that matches its
+key. Both of its tests call `recorded_row` directly. If the arm were
+rewired back to a key lookup, every test would still pass.
+
+**Decided.**
+`audited_stored::tests::a_range_rung_rerun_through_one_rung_cached_returns_its_own_row`
+calls `one_rung_cached` three times on one warmed store, at 5min with a
+fixed support, under three explicit commits:
+1. "generated-one-rung-readback-a"
+2. "-b"
+3. "-a" again
+
+The first two runs share feed, instrument, rung, span and `min_hits`, and
+have different identities. The second run is the ledger's newest row. The
+third run's row must have the first run's identity, must equal ledger row 0,
+and must equal the first run's row field for field. The premises are
+asserted first: two distinct identities, one key, and two ledger rows.
+
+**Proof.** The test passed on the unfixed tree, because the arm is right.
+Against a temporary mutant whose arm reads the newest ledger row, it failed
+at `audited_stored_tests.rs:2960`: the rerun's identity was B's. Under the
+same mutant, `a_reused_range_rung_reads_its_own_row_and_not_the_newest_with_its_key`,
+which calls `recorded_row` directly, still passed.
+
+### D-4704 — Pool pass 2 is held to the cell pass 1 recorded, at 5min and 60min, for every recorded row — 2026-10-09
+
+**What was wrong.** GAP13-15 test gap.
+`pool::tests::the_pool_prices_a_span_exactly_where_the_audit_path_does`
+holds `price_all` to `audit_path_cell`, a cell that the test module
+re-assembles from the audit path's functions. Three things followed:
+- A divergence in what pass 1 actually records, in `price_grids` or the tier
+  cascade, would move neither side of that test.
+- Only the empty mask was priced.
+- 60min, the rung the finding measured, was never exercised. The warmed
+  store's constant prices halt a 60min ladder, because every condition holds
+  on every bar.
+
+**Decided.**
+`pool::tests::pass_two_prices_each_frontier_row_as_pass_one_recorded_it`
+runs at 5min over May 2025 and at 60min over March to May:
+1. `screen_pass_one` screens NIFTY.
+2. The test reads that run's frontier rows by its recorded identity.
+3. `union_of` and `price_all` price the union.
+4. Each frontier row's `(mask, side)` must get from `price_all` the cell the
+   row carries: trades, wins, pessimistic net, worst trade, drawdown,
+   smallest win, gross win and gross loss. A row with no trade must get no
+   cell.
+
+Its premise is that at least one recorded row with a non-empty mask traded.
+On this store every recorded row has a non-empty mask, and
+`the_pool_prices_a_span_exactly_where_the_audit_path_does` keeps the empty
+mask. The ceiling is named (`BRUTEX_CEILING=4096`), so the ladder's reach
+does not follow the machine.
+
+The store is `audited_stored::with_varied_store`:
+- a reproducible random walk through 2025-02-28 and every session of March
+  to May;
+- its 1day bars span each session's minutes;
+- its 5min and 60min months are folded by `pull::fold`, so every signal bar
+  closes on its last minute's close, as the exact-minute overlay requires.
+
+Two simpler stores failed, both measured:
+- The warmed store's 5min bars are sampled minutes. Varied prices fail that
+  overlay ("SignalCloseMismatch").
+- One month of 60min bars halted the ladder at every support from 60% to
+  99%, on either walk. March to May, 213 swept bars, completed at each.
+
+**Proof.** Measured against temporary mutants of the levels `price_grids`
+builds for what pass 1 records:
+- One more rung. The test failed at 60min: "60min rank 1 Long: pass 2's cell
+  is pass 1's recorded cell", pessimistic -4595 against -4525. Under the
+  same mutant `the_pool_prices_a_span_exactly_where_the_audit_path_does`
+  still passed, which is the gap. The 5min half did not differ, which is
+  why the 60min half is needed.
+- A doubled step. Pass 1 then refused to record ("could not reproduce its
+  selected exit cell on an exact grid rebuild"), so the test failed at its
+  premise.
+- No forced stop, and no ratio targets. These recorded the same cells on
+  this store, and the test passed. They are equivalent here, not caught.
+
+**Honest limit.** The fixture's prices are generated and say nothing about a
+market. The test pins that the two passes agree, not what a cell is worth.
+A drift that moves no recorded cell on this store is not seen.
-- 
2.43.0


From 0f60960eec1ce9953421824b1e3df95f75843366 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:22:19 +0000
Subject: [PATCH 4/5] cli: pool-oos walk_span builds its slice facts once per
 span

G2-4, the sixth site of AC-whp-o1-2. Before: `pool_oos::walk_span` called
`runner::trade::walk` once per union candidate. `walk` rebuilds
`SliceFacts::of(bars, column)`, which is O(B), on every call, so a span
paid U builds where one suffices.

After: `walk_span` builds the facts once, before the parallel map, and each
candidate calls `trade::walk_over(.., &facts)`.
`pool_oos::tests::the_hoisted_walk_equals_the_per_candidate_walk` holds
days, tallies and bookings byte for byte against the old shape, which the
test keeps as its reference. It covers two spans, two horizons, booking on
and off, and four masks on both sides.
`walk_span_builds_its_slice_facts_once` pins the shape.

Fail-before: the shape test failed at pool_oos_tests.rs:816 on
`!body.contains("trade::walk(")`.

D-4705, L1FA-09; docs/06 pool-oos judging.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/pool_oos.rs       |  19 +++++-
 crates/cli/src/pool_oos_tests.rs | 103 +++++++++++++++++++++++++++++++
 docs/04-invariants.md            |   1 +
 docs/05-decisions.md             |  37 +++++++++++
 docs/06-limits.md                |   4 +-
 5 files changed, 161 insertions(+), 3 deletions(-)

diff --git a/crates/cli/src/pool_oos.rs b/crates/cli/src/pool_oos.rs
index 477a5d55..3512e287 100644
--- a/crates/cli/src/pool_oos.rs
+++ b/crates/cli/src/pool_oos.rs
@@ -154,6 +154,14 @@ pub(crate) struct Walked {
 /// `book` false only the tallies are kept: the training span needs no series
 /// (sweep audit OS-2, D-2300).
 ///
+/// The span's [`runner::trade::SliceFacts`] -- its timestamp index, prefix
+/// sums and forced-exit table, each O(B) -- are built ONCE here and shared by
+/// every candidate's [`runner::trade::walk_over`]. Each candidate called
+/// [`runner::trade::walk`], which rebuilds them, so a span paid U facts builds
+/// where one suffices (G2-4, the sixth site of AC-whp-o1-2, D-4705). Each walk
+/// still visits every row of the column, Θ(B) per candidate:
+/// `docs/06-limits.md` states it.
+///
 /// # Errors
 ///
 /// A trade naming a bar outside its span, a non-positive entry open, or a
@@ -165,13 +173,20 @@ pub(crate) fn walk_span(
     book: bool,
 ) -> Result<Walked, String> {
     let bars = span.bars.as_slice();
+    let facts = runner::trade::SliceFacts::of(bars, &span.column);
     let per_candidate: Vec<Result<(Tally, Vec<Booking>), String>> = union
         .par_iter()
         .enumerate()
         .map(|(at, candidate)| {
             let mask = vocab::ConditionMask::from_words(candidate.words);
-            let walked =
-                runner::trade::walk(bars, &span.column, &mask, horizon, candidate.direction);
+            let walked = runner::trade::walk_over(
+                bars,
+                &span.column,
+                &mask,
+                horizon,
+                candidate.direction,
+                &facts,
+            );
             let mut tally = Tally::default();
             let mut bookings = Vec::new();
             if book {
diff --git a/crates/cli/src/pool_oos_tests.rs b/crates/cli/src/pool_oos_tests.rs
index 70c499ea..43599759 100644
--- a/crates/cli/src/pool_oos_tests.rs
+++ b/crates/cli/src/pool_oos_tests.rs
@@ -730,3 +730,106 @@ fn a_catalog_at_the_byte_bound_is_written_and_one_byte_over_is_refused() {
     );
     assert!(!over.exists(), "nothing was written");
 }
+
+/// `walk_span` as it stood before D-4705: [`runner::trade::walk`] once per
+/// candidate, so the slice facts were rebuilt for every one. The reference
+/// the hoisted walk must equal.
+fn walk_span_per_candidate_facts(
+    span: &PreparedSpan,
+    horizon: runner::outcome::Horizon,
+    union: &[Candidate],
+    book: bool,
+) -> super::Walked {
+    let bars = span.bars.as_slice();
+    let mut out = super::Walked {
+        days: crate::session_index(bars),
+        tallies: Vec::new(),
+        bookings: Vec::new(),
+    };
+    for (at, candidate) in union.iter().enumerate() {
+        let mask = vocab::ConditionMask::from_words(candidate.words);
+        let walked = runner::trade::walk(bars, &span.column, &mask, horizon, candidate.direction);
+        let mut tally = super::Tally::default();
+        for trade in &walked.trades {
+            let (entry, exit) = (&bars[trade.entry_bar], &bars[trade.exit_bar]);
+            let ppm = i64::try_from(i128::from(trade.worst) * 1_000_000 / i128::from(entry.open))
+                .expect("fits");
+            tally.trades += 1;
+            tally.sum_ppm += i128::from(ppm);
+            if book {
+                out.bookings
+                    .push((at, indicators::ist_day(exit.ts_micros), ppm));
+            }
+        }
+        out.tallies.push(tally);
+    }
+    out
+}
+
+/// **`walk_span` over a span equals the per-candidate walk it replaced, byte
+/// for byte: days, every tally and every booking, on both sides, booked and
+/// not, at two horizons.** G2-4, D-4705.
+#[test]
+fn the_hoisted_walk_equals_the_per_candidate_walk() {
+    let spans = [
+        prepared(span(FIRST_MONDAY, 6, 2_000_000, 200, true)),
+        prepared(span(FIRST_MONDAY, 6, 10_000, 1, false)),
+    ];
+    let mut family = Vec::new();
+    for words in [mask(MONDAY), mask(TUESDAY), [0; 6], {
+        let mut both = mask(MONDAY);
+        both[usize::try_from(TUESDAY / 64).expect("word")] |= 1 << (TUESDAY % 64);
+        both
+    }] {
+        for direction in [Direction::Long, Direction::Short] {
+            family.push(Candidate { words, direction });
+        }
+    }
+    let mut fired = 0;
+    for span in &spans {
+        for horizon in [
+            span.horizon,
+            runner::outcome::Horizon::bars(40).expect("40"),
+        ] {
+            for book in [true, false] {
+                let hoisted = walk_span(span, horizon, &family, book).expect("walked");
+                let reference = walk_span_per_candidate_facts(span, horizon, &family, book);
+                assert_eq!(hoisted.days, reference.days);
+                assert_eq!(hoisted.tallies, reference.tallies, "{horizon:?} {book}");
+                assert_eq!(hoisted.bookings, reference.bookings, "{horizon:?} {book}");
+                fired += hoisted.tallies.iter().filter(|t| t.trades > 0).count();
+            }
+        }
+    }
+    assert!(
+        fired > 0,
+        "premise: candidates fire, so the comparison reads trades"
+    );
+}
+
+/// **`walk_span` builds the span's slice facts ONCE, not once per union
+/// candidate.** G2-4, the sixth site of AC-whp-o1-2's defect, D-4705.
+#[test]
+fn walk_span_builds_its_slice_facts_once() {
+    let source = include_str!("pool_oos.rs");
+    let from = source
+        .find("\npub(crate) fn walk_span(")
+        .expect("walk_span");
+    let body = source
+        .get(from..)
+        .and_then(|rest| rest.find("\n}\n").and_then(|to| rest.get(..to)))
+        .expect("its body");
+    assert!(!body.contains(concat!("trade::walk", "(")), "{body}");
+    assert_eq!(
+        body.matches(concat!("SliceFacts", "::of(")).count(),
+        1,
+        "{body}"
+    );
+    assert!(body.contains(concat!("trade::walk_over", "(")), "{body}");
+    let facts = body.find(concat!("SliceFacts", "::of(")).expect("facts");
+    let lanes = body.find(".par_iter()").expect("the candidate lanes");
+    assert!(
+        facts < lanes,
+        "the facts are built before the candidate loop"
+    );
+}
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 782e070d..78115c84 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -7065,3 +7065,4 @@ old line regex the same input and watched it pass.
 | L1FA-06 | When `begin_many` refuses partway through a chunk, the months it began sweep and file, and every later month refuses with that reason, naming its identity (D-4702) | `cli::batch::stored_tests::a_begin_refused_partway_sweeps_the_months_it_began_and_names_the_rest` | ✓ |
 | L1FA-07 | `one_rung_cached` run as A, then B with A's key, then A again, returns A's own ledger row, not the key's newest (D-4703) | `cli::audited_stored::tests::a_range_rung_rerun_through_one_rung_cached_returns_its_own_row` | ✓ |
 | L1FA-08 | At 5min (May 2025) and at 60min (March to May), on a generated random-walk store, pool pass 2 prices every frontier row pass 1 recorded to exactly the cell figures the row carries, and gives no cell to a row with no trade. A non-empty mask among them traded; the empty mask stays with the older audit-path test (D-4704) | `cli::pool::tests::pass_two_prices_each_frontier_row_as_pass_one_recorded_it` | ✓ |
+| L1FA-09 | `pool_oos::walk_span` builds its slice facts once per span. Its days, tallies and bookings equal the per-candidate `trade::walk` shape, byte for byte (D-4705) | `cli::pool_oos::tests::the_hoisted_walk_equals_the_per_candidate_walk`, `cli::pool_oos::tests::walk_span_builds_its_slice_facts_once` | ✓ |
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index 00535771..e3f83f04 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65291,3 +65291,40 @@ builds for what pass 1 records:
 **Honest limit.** The fixture's prices are generated and say nothing about a
 market. The test pins that the two passes agree, not what a cell is worth.
 A drift that moves no recorded cell on this store is not seen.
+
+### D-4705 — `pool_oos::walk_span` builds its slice facts once per span — 2026-10-09
+
+**What was wrong.** G2-4 is the sixth site of AC-whp-o1-2's defect, which
+D-1730 fixed at its first five. `walk_span` called `runner::trade::walk`
+once per union candidate, and `walk` rebuilds `SliceFacts::of(bars, column)`
+on every call. A span therefore paid U builds where one suffices.
+
+**Decided.** `walk_span` builds `SliceFacts::of(bars, &span.column)` once,
+before the parallel map over the union. Each candidate then calls
+`runner::trade::walk_over(.., &facts)`.
+
+Two tests:
+- `pool_oos::tests::the_hoisted_walk_equals_the_per_candidate_walk` holds
+  the output byte for byte against the old per-candidate shape, which the
+  test keeps as its reference. It covers:
+  - two generated spans and two horizons;
+  - booking on and off;
+  - the masks MONDAY, TUESDAY, empty and MONDAY|TUESDAY;
+  - each mask Long and Short.
+
+  Days, tallies and bookings must all be equal, with the premise that some
+  candidate fired.
+- `pool_oos::tests::walk_span_builds_its_slice_facts_once` reads
+  `walk_span`'s body. It must hold no `trade::walk(`, exactly one
+  `SliceFacts::of(`, and `trade::walk_over(`, and the facts must be built
+  before `.par_iter()`.
+
+**Measured before the fix.** The source test failed at
+`pool_oos_tests.rs:816` on `!body.contains("trade::walk(")`.
+
+**What changes in results.** Nothing. `walk` is `walk_over` over facts it
+builds itself (`runner/src/trade.rs`), and the equality test drives that.
+
+**Cost.** One O(B) facts build per span instead of U. Each candidate's walk
+is unchanged: it still visits every row of the column, which is Θ(B) per
+candidate. `docs/06-limits.md` states it.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 0eb1c8af..2342b948 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -15161,7 +15161,9 @@ Let I be the surface's instruments, U the discovered union, B a span's bars,
 N the later IST sessions and D the bootstrap draws (`bootstrap_draws(N)`, at
 most 100,000).
 
-- **`pool-oos` judging.** Each span costs I × U walks of O(B) each, plus one
+- **`pool-oos` judging.** Each span costs I × U walks of O(B) each, over one
+  `SliceFacts` build of O(B) per span (D-4705; until then each candidate's
+  `trade::walk` rebuilt it, U builds per span, G2-4), plus one
   Romano-Wolf stepdown and one Reality Check of O(D × U × N) each. Neither
   is a §3 rule-4 primitive. Nothing here is measured: UNVERIFIED. Fills are
   on the signal rung's bars, as the audit stack's bootstrap family's are, so
-- 
2.43.0


From 93489016f812e58c51d41edadaf3a32ea2aec691 Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 9 Oct 2026 04:22:25 +0000
Subject: [PATCH 5/5] cli, docs: correct two false texts about pass 1's sharing
 and order

G1-3.
- `range_rung_arm`'s doc said that, asked for one rung, it "sets
  `SharedBy::these(1)`". No code does that. Since D-1701 `sweep_rungs`
  raises no SharedBy, and D-1709 kept that shape. The doc now says what
  holds, and what it used to claim.
- docs/11-findings.md credits D-1564 (row for hunt-conc-1 / GAP13-13) and
  D-1556 (narrative bullets) for loops that D-1701, D-1708 and D-1709
  superseded. The file is append-only, so a narrative correction is added
  at its tail. No row is edited or removed, no F- row is added, and no
  commit is cited.

The third text G1-3 names, pool.rs's "in parallel" header, was corrected
with the code under D-4700.

D-4706.

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01R6fBjvEpAjr5PZ8rpwQhV8
---
 crates/cli/src/lib.rs |  8 ++++++--
 docs/05-decisions.md  | 28 ++++++++++++++++++++++++++++
 docs/11-findings.md   | 27 +++++++++++++++++++++++++++
 3 files changed, 61 insertions(+), 2 deletions(-)

diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index 4ae07321..92db06c8 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -1980,10 +1980,14 @@ fn descend_arm(
 /// row — indistinguishable from a rung that never started.
 ///
 /// `range_over` has taken a rung subset since it was written; it simply had no
-/// verb. Asked for ONE rung it sets `SharedBy::these(1)`, so that rung gets the
+/// verb. Asked for ONE rung, one sweep is in flight and nothing raises
+/// `SWEEPS_SHARING_THIS_MACHINE` above its resting 1, so that rung gets the
 /// full ceiling and every support lane — and a caller running the eight in
 /// sequence gets both a deeper search per rung and a row after each one, rather
-/// than eight starved rungs and nothing until they all finish.
+/// than eight starved rungs and nothing until they all finish. (This said it
+/// "sets `SharedBy::these(1)`"; no code does, and since D-1701 `sweep_rungs`
+/// runs every rung one at a time and raises no `SharedBy` at all -- D-1709,
+/// D-4706.)
 ///
 /// It takes the same `auto` token: support derived per rung from that rung's own
 /// bars, nothing typed.
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index e3f83f04..192452fb 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -65328,3 +65328,31 @@ builds itself (`runner/src/trade.rs`), and the equality test drives that.
 **Cost.** One O(B) facts build per span instead of U. Each candidate's walk
 is unchanged: it still visits every row of the column, which is Θ(B) per
 candidate. `docs/06-limits.md` states it.
+
+### D-4706 — Two false texts about pass 1's sharing and order are corrected — 2026-10-09
+
+**What was wrong.** G1-3. Two texts stated something about pass 1 that is not
+true.
+- **`range_rung_arm`'s doc** (`crates/cli/src/lib.rs`) said that, asked for
+  one rung, it "sets `SharedBy::these(1)`". No code does that. Since D-1701,
+  `sweep_rungs` raises no `SharedBy`, and D-1709 kept that shape, so one
+  sweep is in flight and the counter stays at its resting 1. The only
+  production `SharedBy::these` is in `batch.rs`.
+- **Two places in `docs/11-findings.md`** credit superseded decisions.
+  - The table row for audit-20261003 hunt-conc-1 (KNOWN GAP13-13) credits
+    D-1564. It says `range-all`, `pool` and the Boolean pools are "stated as
+    completion-ordered … not changed".
+  - The narrative bullets credit D-1556 for `range-all` and `pool` pass 1.
+  - Both were superseded: by D-1701 and D-1708 for `sweep-all`, and by
+    D-1701 and D-1709 for `range-all` and `pool` pass 1.
+
+**Decided.**
+- The doc now states what holds and says what it claimed until this entry.
+- `docs/11-findings.md` is append-only, so no row there is edited. A
+  narrative correction at its tail names the superseding decisions. It adds
+  no `F-` row and cites no commit.
+
+`pool.rs`'s "in parallel" header, the third text G1-3 names, is corrected
+with the code under D-4700.
+
+**What changes in results.** Nothing. These are text changes only.
diff --git a/docs/11-findings.md b/docs/11-findings.md
index 80c3f349..c87d92db 100644
--- a/docs/11-findings.md
+++ b/docs/11-findings.md
@@ -1072,3 +1072,30 @@ stays IN PROGRESS naming its branch commit until the squash merge to `main`.
 - **`F-8D5073`** (`gap`) — The vocabulary documents stated counts and kinds the table does not hold: void Near rows marked untoleranced, sixteen void names for thirteen, 70 free positions for 14, and a group table stopping at 273. Where: `docs/03-vocabulary.md` (rows 235–271, crossings section); `docs/04-invariants.md` CX-04, CX-05; `crates/vocab/src/table.rs:14-29`; `crates/vocab/src/lib.rs`. Disposition: IN PROGRESS — fixed on `attack/permutations` 3c5c237d (D-3406, XPERM-06); lands with the squash merge to `main`.
 - **`F-0486DA`** (`wrong`) — worst_reward_risk_bp scored a single observation i64::MAX, above payoff_bp, so one lucky move topped the asymmetry ranking. Where: `crates/runner/src/outcome.rs` (`Edge::worst_reward_risk_bp`); `crates/runner/src/rank.rs` (`ByAsymmetry`). Disposition: IN PROGRESS — fixed on `attack/permutations` 24c7e3a (D-3407, XPERM-07); lands with the squash merge to `main`.
 - **`F-1D5275`** (`wrong`) — trades_needed_for documented measured values its ceiling-rounded record does not return, and a monotone threshold it does not have. Where: `crates/runner/src/grid.rs` (`trades_needed_for` doc, `Cell::at_rate`). Disposition: IN PROGRESS — fixed on `attack/permutations` bc1e1f45 (D-3408, XPERM-08); lands with the squash merge to `main`.
+
+### Correction: two ordered-write records credit superseded decisions — 2026-10-09
+
+Narrative only. No row is added to the tables above, and no row above is
+edited, because this file is append-only. D-4706 records the correction.
+
+- **audit-20261003 hunt-conc-1 (KNOWN GAP13-13)**, the table row that
+  credits **D-1564**. It credits D-1564's windowed four-phase walk and says
+  `range-all`, `pool` and the Boolean pools "are stated as
+  completion-ordered … not changed". Both halves were superseded.
+  - `sweep-all`: D-1708 kept D-1701's shape over D-1564. Chunks are one
+    month per worker. Every attempt in a chunk is begun by one `begin_many`
+    in input order, and the months are then filed one at a time in input
+    order.
+  - `range-all` and `pool` pass 1: these were changed. They run `one_rung`
+    one call at a time in input order (D-1701), and D-1709 kept that shape.
+  - The Boolean family pools write through `ordered::map` lanes (D-1556).
+- **audit-20261003 hunt-conc-1** (`range-all`, `pool` pass 1), the narrative
+  bullet that credits **D-1556**. For those two loops D-1709 superseded it
+  and kept D-1701's one-at-a-time loop over D-1556's ordered lanes. D-1556
+  still holds for the Boolean family pools (hunt-conc-2).
+- **`pool-oos` pass 1 (G1-1)** re-introduced the pre-D-1701 parallel shape
+  in a verb added after D-1701. IN PROGRESS — fixed on `final/all-fixes`
+  (D-4700); lands with the squash merge to `main`.
+- **A `sweep-all` month whose column build refused (G1-2)** still journaled
+  its Refused terminal from a rayon worker. IN PROGRESS — fixed on
+  `final/all-fixes` (D-4701); lands with the squash merge to `main`.
-- 
2.43.0


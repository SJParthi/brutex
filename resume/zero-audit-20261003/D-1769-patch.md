# D-1769 working-tree patch (insurance copy, 2026-10-03 20:20 UTC)

Base: final/all-fixes-zero @ 724e4a61. Apply with: extract the fenced block to a file, then `git apply`.

````diff
diff --git a/crates/api/src/assets.rs b/crates/api/src/assets.rs
index d1432b91..45c63734 100644
--- a/crates/api/src/assets.rs
+++ b/crates/api/src/assets.rs
@@ -93,8 +93,12 @@ const MASTERS: &str = "masters.js";
 const SOURCES: &str = "src";
 
 /// The front-end directory: [`WEB_ENV`], or `web/` beside this workspace.
-#[must_use]
-pub fn web_dir() -> PathBuf {
+///
+/// # Errors
+///
+/// [`WEB_ENV`] is set but empty, which would serve `./build` of the working
+/// directory as the front end (CE-37, D-1769).
+pub fn web_dir() -> Result<PathBuf, String> {
     web_dir_from(std::env::var_os(WEB_ENV))
 }
 
@@ -105,9 +109,8 @@ pub fn web_dir() -> PathBuf {
 /// cannot set an environment variable — `set_var` is `unsafe` under edition
 /// 2024, this crate forbids `unsafe`, and mutating process-wide state would
 /// race every other test in the binary.
-#[must_use]
-fn web_dir_from(value: Option<std::ffi::OsString>) -> PathBuf {
-    value.map_or_else(default_web_dir, PathBuf::from)
+fn web_dir_from(value: Option<std::ffi::OsString>) -> Result<PathBuf, String> {
+    Ok(brutex_core::knob::folder(WEB_ENV, value)?.unwrap_or_else(default_web_dir))
 }
 
 /// `web/` beside the workspace this binary was built from.
@@ -1093,13 +1096,18 @@ mod tests {
     fn the_directory_is_the_environment_or_web_beside_the_workspace() {
         assert_eq!(
             web_dir_from(Some("/somewhere/else".into())),
-            PathBuf::from("/somewhere/else"),
+            Ok(PathBuf::from("/somewhere/else")),
             "the environment wins when it is set"
         );
+        // CE-37, D-1769: set but empty is refused by name, never `./build`.
+        assert!(
+            web_dir_from(Some("".into()))
+                .is_err_and(|why| why.starts_with("BRUTEX_WEB is set but empty")),
+        );
         // Compared against `default_web_dir` rather than only against
         // `web_dir_from(None)`, which calls it: a function compared only
         // against itself cannot be falsified.
-        assert_eq!(web_dir_from(None), default_web_dir());
+        assert_eq!(web_dir_from(None), Ok(default_web_dir()));
         assert!(
             default_web_dir().ends_with("web"),
             "the default names the front-end directory: {}",
diff --git a/crates/api/src/http_admission_tests.rs b/crates/api/src/http_admission_tests.rs
index 54283fe2..f664ffd3 100644
--- a/crates/api/src/http_admission_tests.rs
+++ b/crates/api/src/http_admission_tests.rs
@@ -243,6 +243,31 @@ async fn every_answer_forbids_framing_and_sniffing() {
             .expect("join");
             assert!(status_line(&too_big).contains("413"), "{too_big}");
             assert_never_framed("the body limit", &too_big);
+
+            // `/pull/run` and `/pull/recovery` read the run bound: a body past
+            // 8 KiB reaches the route's own JSON refusal, not a framework 413
+            // in plain text (P3-01-01, D-1769).
+            for path in ["/pull/run", "/pull/recovery"] {
+                let wide = format!("pad={}", "x".repeat(2 * crate::server::MAX_FORM_BYTES));
+                let request = format!(
+                    "POST {path} HTTP/1.1\r\nHost: {local}\r\n{origin}\
+                     Content-Type: application/x-www-form-urlencoded\r\n\
+                     Content-Length: {}\r\nConnection: close\r\n\r\n{wide}",
+                    wide.len()
+                );
+                let answer = tokio::task::spawn_blocking(move || {
+                    use std::io::{Read as _, Write as _};
+                    let mut socket = std::net::TcpStream::connect(addr).expect("connect");
+                    socket.write_all(request.as_bytes()).expect("write");
+                    let mut answer = String::new();
+                    socket.read_to_string(&mut answer).expect("read");
+                    answer
+                })
+                .await
+                .expect("join");
+                assert!(!status_line(&answer).contains("413"), "{path}: {answer}");
+                assert!(status_line(&answer).contains("400"), "{path}: {answer}");
+            }
         },
     )
     .await;
diff --git a/crates/api/src/livejson.rs b/crates/api/src/livejson.rs
index 1dff5f91..e2be5820 100644
--- a/crates/api/src/livejson.rs
+++ b/crates/api/src/livejson.rs
@@ -228,7 +228,14 @@ fn write_row(out: &mut String, row: &cli::frontier::Row, bar_milli: i64) {
             // removes the chance of two integers on different scales being
             // eyeballed against each other. `t_milli` can be negative for a
             // short's evidence; the bar is on |t|.
-            row.t_milli.saturating_abs() >= bar_milli,
+            //
+            // STRICTLY ABOVE. `t_milli` is rounded and `bar_milli` is the
+            // ceiling of the bar, so `t_milli > bar_milli` is the comparison
+            // that cannot call a |t| below the bar a clearance: |t| is at least
+            // `t_milli - 0.5` thousandths, which is at least `bar_milli + 0.5`
+            // and above the bar. `>=` read a |t| up to 1.5 milli short as
+            // clearing it (CE-7, D-1769).
+            row.t_milli.saturating_abs() > bar_milli,
         ),
     );
 }
@@ -296,6 +303,48 @@ mod tests {
         );
     }
 
+    /// A |t| whose rounded thousandths EQUAL the ceilinged bar is not called a
+    /// clearance, and one a milli above it is; the sign of `t` does not matter
+    /// (CE-7, D-1769).
+    #[test]
+    fn a_t_at_the_rounded_bar_does_not_clear_it_and_one_above_does() {
+        let row = |t_milli: i64| cli::frontier::Row {
+            identity: [0; 32],
+            rank: 1,
+            mask_words: [1, 0, 0, 0, 0, 0],
+            hits: 1,
+            n: 1,
+            mean_milli_paisa: 0,
+            t_milli,
+            payoff_bp: 0,
+            wins: 0,
+            trades: 0,
+            cell_wins: 0,
+            pessimistic: 0,
+            worst_trade: 0,
+            max_drawdown: 0,
+            min_win: 0,
+            gross_win: 0,
+            gross_loss: 0,
+            direction: cli::frontier::Direction::Long,
+            rules: cli::Rules::elite(400, 25),
+        };
+        // bar 5.6739 is carried as 5674; t 5.6735 rounds to 5674 and is below it.
+        for (t_milli, clears) in [
+            (5_674, false),
+            (-5_674, false),
+            (5_675, true),
+            (-5_675, true),
+        ] {
+            let mut out = String::new();
+            super::write_row(&mut out, &row(t_milli), 5_674);
+            assert!(
+                out.contains(&format!(r#""clears_bar":{clears}"#)),
+                "{t_milli}: {out}"
+            );
+        }
+    }
+
     /// A published run is served with its bar, and the rows are NOT judged.
     ///
     /// The `meets` block `/frontier.json` carries would be computed from eight
diff --git a/crates/api/src/logs.rs b/crates/api/src/logs.rs
index c7228f06..9ede2218 100644
--- a/crates/api/src/logs.rs
+++ b/crates/api/src/logs.rs
@@ -881,8 +881,16 @@ fn health_banner(health: Option<&telemetry::Health>) -> String {
     if h.rotation_failures > 0 {
         let _ = write!(
             out,
-            "{} roll(s) failed, so the current file is past its bound and the \
-             oldest events may already have been overwritten. ",
+            // WHAT THE SINK ACTUALLY DOES AFTER A FAILED ROLL. It stops
+            // rotating for the life of the process (`Sink::rotation_broken`),
+            // so nothing is overwritten after the failure: the file GROWS. The
+            // banner said the opposite and never said a restart resumes
+            // rotation (CE-41, D-1769).
+            "{} roll(s) failed, so rotation has stopped for the life of this \
+             process: the current file is growing past its bound and no later \
+             event overwrites an older one. The failed roll itself may have \
+             removed the oldest retained file. Fix the cause named below and \
+             restart the server to resume rotation. ",
             h.rotation_failures,
         );
     }
@@ -1696,6 +1704,27 @@ mod tests {
         );
     }
 
+    /// CE-41, D-1769: after a failed roll the banner says rotation STOPPED
+    /// and the file is growing, never that events were overwritten, and that a
+    /// restart resumes it.
+    #[test]
+    fn a_failed_roll_is_described_as_stopped_rotation_not_overwrite() {
+        let page = health_banner(Some(&health(0, 1, Some("rename refused"))));
+        assert!(
+            page.contains("rotation has stopped for the life of this process"),
+            "{page}"
+        );
+        assert!(
+            page.contains("restart the server to resume rotation"),
+            "{page}"
+        );
+        assert!(
+            !page.contains("may already have been overwritten"),
+            "{page}"
+        );
+        assert!(page.contains("rename refused"), "{page}");
+    }
+
     fn health(dropped: u64, rotation_failures: u64, last_error: Option<&str>) -> telemetry::Health {
         telemetry::Health {
             path: std::path::PathBuf::from("events.ndjson"),
diff --git a/crates/api/src/pullrun.rs b/crates/api/src/pullrun.rs
index 3c7cce47..7397f2db 100644
--- a/crates/api/src/pullrun.rs
+++ b/crates/api/src/pullrun.rs
@@ -53,6 +53,34 @@
 use crate::census;
 use crate::server::{Loaded, Site, percent_decode};
 
+/// How many legs one press may carry: one per feed, per rung the ingest page
+/// offers (`1s`, `1min`, `1day`), per route (`/pull/spot`, `/pull/fno`).
+///
+/// A form with more is refused by name ([`Refusal::TooManyLegs`]) rather than
+/// read, so [`MAX_RUN_FORM_BYTES`] has a count to be sized from (P3-01-01,
+/// D-1769).
+pub const MAX_RUN_LEGS: usize = pull::vendor::FEED_COUNT * 3 * 2;
+
+/// The worst size of one `leg=` field: a member form the inner route admits
+/// ([`crate::ingest::MAX_MEMBER_FORM_BYTES`]) plus an ordinary form's worth of
+/// envelope, percent-encoded twice more by the page. Encoding an
+/// already-encoded byte turns `%` into `%25`, so each pass costs at most a
+/// further two bytes per original escape: five bytes per form byte in all.
+pub const MAX_LEG_FIELD_BYTES: usize =
+    "leg=".len() + 5 * (crate::ingest::MAX_MEMBER_FORM_BYTES + crate::server::MAX_FORM_BYTES);
+
+/// The body `/pull/run` and `/pull/recovery` read.
+///
+/// Both carry legs, and each leg repeats its member list twice-encoded, so the
+/// shared 8 KiB bound answered a framework 413 in plain text at about 340
+/// ticked members on one leg and about 55 across six; the page then reported
+/// a `SyntaxError` instead of a reason (P3-01-01, D-1769). Sized so every run
+/// [`legs_from`] would accept is read, and one leg too many reaches
+/// [`Refusal::TooManyLegs`]. About 26.5 MB at most, held in memory once; see
+/// `docs/06-limits.md`.
+pub const MAX_RUN_FORM_BYTES: usize =
+    crate::server::MAX_FORM_BYTES + (MAX_RUN_LEGS + 1) * MAX_LEG_FIELD_BYTES;
+
 /// How many passes one press may make.
 ///
 /// A window can be larger than one sitting at a legal rate, so the run keeps
@@ -373,6 +401,8 @@ pub enum Refusal {
         /// Which half disagreed, and with what.
         why: String,
     },
+    /// The form carried more legs than one press may ([`MAX_RUN_LEGS`]).
+    TooManyLegs(usize),
 }
 
 impl Refusal {
@@ -399,6 +429,11 @@ impl Refusal {
                  envelope while the payload decides what is fetched. {why}. The \
                  leg was: {leg}"
             ),
+            Self::TooManyLegs(count) => format!(
+                "The run carried {count} legs and NOTHING was started. One press \
+                 carries at most {MAX_RUN_LEGS}: one per feed, per rung, per \
+                 route. Split the selection into two presses."
+            ),
         }
     }
 }
@@ -437,6 +472,13 @@ pub fn legs_from(body: &str) -> Result<Vec<Leg>, Refusal> {
         let Some(("leg", raw)) = field.split_once('=') else {
             continue;
         };
+        if legs.len() == MAX_RUN_LEGS {
+            let count = body
+                .split('&')
+                .filter(|field| field.starts_with("leg="))
+                .count();
+            return Err(Refusal::TooManyLegs(count));
+        }
         let decoded = percent_decode(raw);
         let mut parts = decoded.splitn(5, '|');
         let (Some(route), Some(vendor), Some(dir), Some(label), Some(payload)) = (
@@ -1441,6 +1483,60 @@ mod tests {
     /// every leg names a real feed. Before D-0906 `legs_from` copied the vendor
     /// unchecked, so a form of invented vendor names grew one group and one
     /// chain per distinct name. W1-api3-3.
+    /// P3-01-01, D-1769: one leg past the bound is refused by name, and the
+    /// widest leg the page can write fits the field bound.
+    #[test]
+    fn a_run_past_the_leg_bound_is_refused_by_name_and_the_widest_leg_fits() {
+        let one = field("/pull/spot", "dhan", "1day", "d", "a=1");
+        let full = vec![one.clone(); MAX_RUN_LEGS].join("&");
+        assert_eq!(
+            legs_from(&full).map(|legs| legs.len()).ok(),
+            Some(MAX_RUN_LEGS)
+        );
+        let over = vec![one; MAX_RUN_LEGS + 1].join("&");
+        match legs_from(&over) {
+            Err(Refusal::TooManyLegs(count)) => {
+                assert_eq!(count, MAX_RUN_LEGS + 1);
+                let why = Refusal::TooManyLegs(count).why();
+                assert!(why.contains(&MAX_RUN_LEGS.to_string()), "{why}");
+            }
+            other => panic!("one leg past the bound must be named, got {other:?}"),
+        }
+        // The page's own shape: a member form of `MAX_MEMBERS` symbols that
+        // each need escaping, encoded once more for the payload and once
+        // more for the field.
+        let encode = |text: &str| -> String {
+            text.bytes()
+                .map(|b| {
+                    if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
+                        char::from(b).to_string()
+                    } else {
+                        format!("%{b:02X}")
+                    }
+                })
+                .collect()
+        };
+        let symbol = "&".repeat(brutex_core::symbol::SYMBOL_CAPACITY);
+        let members =
+            vec![format!("member={}", encode(&symbol)); crate::ingest::MAX_MEMBERS].join("&");
+        let form = format!("target=nifty50&vendor=dhan&{members}");
+        assert!(
+            form.len() <= crate::ingest::MAX_MEMBER_FORM_BYTES,
+            "{}",
+            form.len()
+        );
+        let leg = format!(
+            "leg={}",
+            encode(&["/pull/spot", "dhan", "1day", "label", &encode(&form)].join("|"))
+        );
+        assert!(
+            leg.len() <= MAX_LEG_FIELD_BYTES,
+            "{} > {MAX_LEG_FIELD_BYTES}",
+            leg.len()
+        );
+        const { assert!(MAX_RUN_FORM_BYTES > MAX_RUN_LEGS * MAX_LEG_FIELD_BYTES) };
+    }
+
     #[test]
     fn a_leg_naming_no_feed_refuses_the_run_so_groups_never_outnumber_feeds() {
         let invented: Vec<String> = (0..=pull::vendor::FEED_COUNT)
diff --git a/crates/api/src/render.rs b/crates/api/src/render.rs
index 30e9db39..18230283 100644
--- a/crates/api/src/render.rs
+++ b/crates/api/src/render.rs
@@ -3079,9 +3079,23 @@ fn folder_input(suggestions: &[String]) -> String {
         let _ = write!(out, "<option value=\"{}\">", escape(f));
     }
     out.push_str("</datalist>");
-    if !archive_suggestions_enabled() {
-        out.push_str("<p class=\"fine\">Automatic CSV-folder suggestions are disabled for this process (BRUTEX_ARCHIVE_SUGGESTIONS=0). No discovery scan was performed; explicit folder imports remain available.</p>");
-        return out;
+    match archive_suggestions() {
+        Ok(true) => {}
+        Ok(false) => {
+            out.push_str("<p class=\"fine\">Automatic CSV-folder suggestions are disabled for this process (BRUTEX_ARCHIVE_SUGGESTIONS is off). No discovery scan was performed; explicit folder imports remain available.</p>");
+            return out;
+        }
+        // A WORD THE SWITCH DOES NOT TAKE TURNS THE WALK OFF AND SAYS SO. It
+        // used to leave the walk running for anything but a literal `0`, so
+        // `false`, `off` and `no` silently did nothing (CE-39, D-1769).
+        Err(why) => {
+            let _ = write!(
+                out,
+                "<p class=\"fine\">Automatic CSV-folder suggestions are off: {}. No discovery scan was performed; explicit folder imports remain available.</p>",
+                escape(&why)
+            );
+            return out;
+        }
     }
     let _ = write!(
         out,
@@ -3119,11 +3133,19 @@ const MAX_FOLDER_SUGGESTIONS: usize = 60;
 /// `docs/06-limits.md` §34 records the cost and what is not bounded about it.
 #[must_use]
 pub fn folder_suggestions() -> Vec<String> {
-    folders_when(archive_suggestions_enabled(), discover_folders)
+    folders_when(archive_suggestions() == Ok(true), discover_folders)
 }
 
-fn archive_suggestions_enabled() -> bool {
-    std::env::var_os("BRUTEX_ARCHIVE_SUGGESTIONS").as_deref() != Some(std::ffi::OsStr::new("0"))
+/// `BRUTEX_ARCHIVE_SUGGESTIONS`, read by the shared switch: on unless turned
+/// off, and a word it does not take refused by name (CE-39, D-1769).
+fn archive_suggestions() -> Result<bool, String> {
+    let raw = std::env::var_os("BRUTEX_ARCHIVE_SUGGESTIONS");
+    brutex_core::knob::switch(
+        "BRUTEX_ARCHIVE_SUGGESTIONS",
+        raw.as_deref()
+            .map(|value| value.to_str().unwrap_or("\u{fffd}")),
+        true,
+    )
 }
 
 fn folders_when(enabled: bool, discover: impl FnOnce() -> Vec<String>) -> Vec<String> {
@@ -3132,8 +3154,10 @@ fn folders_when(enabled: bool, discover: impl FnOnce() -> Vec<String>) -> Vec<St
 
 fn discover_folders() -> Vec<String> {
     let mut found: Vec<String> = Vec::new();
-    if let Some(home) = std::env::var_os("HOME") {
-        let home = std::path::PathBuf::from(home);
+    // AN EMPTY OR RELATIVE HOME WALKS NOTHING. It used to walk the working
+    // directory's `Downloads` (CE-38, D-1769); a convenience list has no
+    // refusal to give, so it is simply empty.
+    if let Ok(home) = brutex_core::knob::home(std::env::var_os("HOME")) {
         for root in [
             home.join("Downloads"),
             home.join(".brutex").join("vendor-data"),
@@ -3170,10 +3194,23 @@ mod archive_suggestion_tests {
 /// Directories at or under `dir` that directly contain a `.csv`.
 ///
 /// Depth-limited and allocation-bounded. `depth` counts down, so the recursion
-/// cannot outlive the number it was given — there is no cycle check because
-/// there is no cycle a bounded depth can complete.
+/// cannot outlive the number it was given. A linked directory below the root
+/// is not followed, so a link back to an ancestor cannot multiply the walk
+/// (CE-35, D-1769); the depth bound remains the backstop.
 fn collect_csv_dirs(dir: &std::path::Path, depth: usize, out: &mut Vec<String>) {
-    if depth == 0 || out.len() >= MAX_FOLDER_SUGGESTIONS || !dir.is_dir() {
+    // ONE PAST THE CAP, SO THE PAGE CAN TELL A FULL LIST FROM A CAPPED ONE.
+    // The walk stopped at exactly `MAX_FOLDER_SUGGESTIONS`, and `folder_input`
+    // states the cap only when it holds MORE than that — so the notice could
+    // never print and a capped list read as complete (CE-32, D-1769). The one
+    // extra folder is collected and never offered.
+    //
+    // AND A LINK IS NOT A FOLDER TO WALK. `is_dir` followed symlinks, so
+    // `~/Downloads/loop -> ~/Downloads` re-walked the whole tree at every level
+    // to depth six before the server started (CE-35, D-1769). A linked child
+    // directory is now skipped, as `assets` and `store::catalog` already do.
+    // The ROOT may still be a link — `~/Downloads` on an external drive is an
+    // ordinary setup — because a root is followed once, not at every level.
+    if depth == 0 || out.len() > MAX_FOLDER_SUGGESTIONS || !dir.is_dir() {
         return;
     }
     let Ok(entries) = std::fs::read_dir(dir) else {
@@ -3193,9 +3230,12 @@ fn collect_csv_dirs(dir: &std::path::Path, depth: usize, out: &mut Vec<String>)
         {
             continue;
         }
-        if p.is_dir() {
+        let Ok(kind) = e.file_type() else {
+            continue;
+        };
+        if kind.is_dir() {
             children.push(p);
-        } else if p.extension().is_some_and(|x| x == "csv") {
+        } else if !kind.is_symlink() && p.extension().is_some_and(|x| x == "csv") {
             has_csv = true;
         }
     }
@@ -3224,6 +3264,64 @@ mod tests {
     // record read back off disk cannot rebuild a counter without counting.
     use pull::session::{DropCensus, DropReason};
 
+    /// A directory link is not walked, so a link back to an ancestor cannot
+    /// multiply the startup walk (CE-35, D-1769).
+    #[cfg(unix)]
+    #[test]
+    fn a_linked_directory_is_not_walked() {
+        let root = std::env::temp_dir().join(format!("brutex-ce35-{}", std::process::id()));
+        let _ = std::fs::remove_dir_all(&root);
+        let data = root.join("data");
+        std::fs::create_dir_all(&data).unwrap();
+        std::fs::write(data.join("x.csv"), "a\n").unwrap();
+        std::os::unix::fs::symlink(&root, data.join("loop")).unwrap();
+        let mut found = Vec::new();
+        collect_csv_dirs(&root, 6, &mut found);
+        // A root that is itself a link is followed once, and the link inside
+        // it is still not walked.
+        let mut through_link = Vec::new();
+        collect_csv_dirs(&data.join("loop"), 6, &mut through_link);
+        std::fs::remove_dir_all(&root).unwrap();
+        assert_eq!(found, vec![data.to_string_lossy().into_owned()]);
+        assert_eq!(through_link.len(), 1, "{through_link:?}");
+    }
+
+    /// A tree holding more CSV folders than the cap is walked one past it, so
+    /// the picker offers sixty and SAYS it was capped; a tree holding exactly
+    /// sixty is offered whole with no notice (CE-32, D-1769).
+    #[test]
+    fn a_capped_folder_walk_is_stated_and_an_exact_one_is_not() {
+        for (made, capped) in [
+            (MAX_FOLDER_SUGGESTIONS + 5, true),
+            (MAX_FOLDER_SUGGESTIONS, false),
+        ] {
+            let root =
+                std::env::temp_dir().join(format!("brutex-ce32-{}-{made}", std::process::id()));
+            let _ = std::fs::remove_dir_all(&root);
+            for i in 0..made {
+                let d = root.join(format!("f{i:03}"));
+                std::fs::create_dir_all(&d).unwrap();
+                std::fs::write(d.join("x.csv"), "a\n").unwrap();
+            }
+            let mut found = Vec::new();
+            collect_csv_dirs(&root, 6, &mut found);
+            let html = folder_input(&found);
+            std::fs::remove_dir_all(&root).unwrap();
+            assert_eq!(found.len(), made.min(MAX_FOLDER_SUGGESTIONS + 1));
+            assert_eq!(
+                html.matches("<option value=").count(),
+                made.min(MAX_FOLDER_SUGGESTIONS)
+            );
+            if archive_suggestions() == Ok(true) {
+                assert_eq!(
+                    html.contains(&format!("capped at {MAX_FOLDER_SUGGESTIONS}")),
+                    capped,
+                    "{html}"
+                );
+            }
+        }
+    }
+
     /// The store filter bar shows the bar length it was given, one pill per
     /// rung the store knows, and "All" only when no rung narrows the view
     /// (P1-02-05, D-1765).
diff --git a/crates/api/src/server.rs b/crates/api/src/server.rs
index 1f7f2f35..0edd449b 100644
--- a/crates/api/src/server.rs
+++ b/crates/api/src/server.rs
@@ -239,7 +239,9 @@ pub fn masters_dir() -> Result<PathBuf, String> {
 ///
 /// No value and no `HOME`. See [`default_masters_dir_from`].
 fn masters_dir_from(value: Option<std::ffi::OsString>) -> Result<PathBuf, String> {
-    value.map_or_else(default_masters_dir, |v| Ok(PathBuf::from(v)))
+    // SET BUT EMPTY IS REFUSED, NOT RESOLVED. `PathBuf::from("")` is the
+    // working directory (CE-33, D-1769).
+    brutex_core::knob::folder("BRUTEX_MASTERS", value)?.map_or_else(default_masters_dir, Ok)
 }
 
 /// The directory the bar store and its manifests are read from.
@@ -290,8 +292,12 @@ fn store_dir_from(
     value: Option<std::ffi::OsString>,
     home: Option<std::ffi::OsString>,
 ) -> Result<PathBuf, String> {
-    if let Some(value) = value {
-        return Ok(PathBuf::from(value));
+    // SET BUT EMPTY IS REFUSED, NOT RESOLVED: `PathBuf::from("")` is the
+    // working directory, the `.` fallback this function's doc says was removed,
+    // reached by another road (CE-33, D-1769). An empty or relative HOME is
+    // refused the same way (CE-38).
+    if let Some(value) = brutex_core::knob::folder("BRUTEX_STORE", value)? {
+        return Ok(value);
     }
     let Some(home) = home else {
         return Err(format!(
@@ -304,7 +310,9 @@ fn store_dir_from(
                 .map_or_else(|e| format!("unreadable ({e})"), |p| p.display().to_string())
         ));
     };
-    Ok(PathBuf::from(home).join(".brutex").join("store"))
+    Ok(brutex_core::knob::home(Some(home))?
+        .join(".brutex")
+        .join("store"))
 }
 
 /// Where the masters live when `BRUTEX_MASTERS` says nothing.
@@ -351,7 +359,11 @@ fn default_masters_dir_from(home: Option<std::ffi::OsString>) -> Result<PathBuf,
                  under an environment that has HOME.",
             ))
         },
-        |home| Ok(PathBuf::from(home).join(".brutex").join("masters")),
+        |home| {
+            Ok(brutex_core::knob::home(Some(home))?
+                .join(".brutex")
+                .join("masters"))
+        },
     )
 }
 
@@ -8517,13 +8529,17 @@ where
         // and so must this one, or the two paths disagree about one credential.
         let invalid_auth = why.detail.contains("Invalid_Authentication");
 
-        match step(why.status, invalid_auth, None, attempt, server_errors) {
+        // THE VENDOR'S OWN NAME FOR IT, read from the body by the transport
+        // (CE-29, D-1769). `None` here sent a 403 "not entitled" down the dead
+        // token arm and a 400 `DH-906` down the answered one.
+        match step(why.status, invalid_auth, why.named, attempt, server_errors) {
             Step::NotEntitled => {
                 return Err(pull::chain::Refusal {
                     // ALIVE, AND NOT ENTITLED. Two different facts, and
                     // conflating them halts a feed over a subscription gap.
                     credential_dead: false,
                     status: why.status,
+                    named: why.named,
                     detail: format!(
                         "{why} — the credential is ALIVE and this API key is not \
                          entitled to this call. Re-running later cannot fix it; \
@@ -8539,6 +8555,7 @@ where
                     // re-derived downstream by grepping a rendered page.
                     credential_dead: true,
                     status: why.status,
+                    named: why.named,
                     detail: format!(
                         "{why} — the access token is no longer valid mid-walk. \
                          This repository never mints one (§8): the refreshed \
@@ -8555,6 +8572,7 @@ where
                     // THE VENDOR'S SIDE FAILED; the credential was fine.
                     credential_dead: false,
                     status: why.status,
+                    named: why.named,
                     // MARKED, SO THE RUN LOOP DOES NOT HAVE TO READ THIS
                     // SENTENCE TO KNOW WHAT IT SAYS. The marker is stripped in
                     // `broker_run` before the reason reaches an operator or the
@@ -8582,6 +8600,7 @@ where
         // EXHAUSTED IS ABOUT THE TRANSPORT, NOT THE TOKEN.
         credential_dead: false,
         status: last.status,
+        named: last.named,
         detail: format!(
             "{last} — and it failed {THROTTLE_ATTEMPTS} times, so the transport \
              is not blipping, it is down"
@@ -9815,8 +9834,14 @@ async fn with_retry(
                         // vendor had just refused. So the decrease is taken here
                         // for the named case only, which is precisely the set
                         // the transport misses. D-0322.
+                        //
+                        // AND THE TRANSPORT DOES NOT MISS A 2xx. A throttle
+                        // named in a 2xx body is recorded by
+                        // `weigh_body_parsed` (D-0950), so taking it here too
+                        // counted it twice and halved the refusals needed to
+                        // floor the allowance (CE-30, D-1769).
                         if throttled
-                            && status != Some(429)
+                            && transport_missed_throttle(status)
                             && let Ok(budgets) = site.budgets.lock()
                             && let Some(Some(shared)) = budgets.get(feed as usize)
                         {
@@ -9840,6 +9865,36 @@ async fn with_retry(
     ))
 }
 
+/// Whether a throttle `step` named under `status` is one the transport did not
+/// already record against the shared governor.
+///
+/// `window_async` records a 429 on its status, and a throttle named in a 2xx
+/// body in `weigh_body_parsed`. What it cannot see is a named throttle under
+/// any other answered status, and that is the only set `with_retry` may record
+/// (D-0322, CE-30, D-1769).
+fn transport_missed_throttle(status: Option<u16>) -> bool {
+    status.is_some_and(|code| code != 429 && !(200..=299).contains(&code))
+}
+
+#[cfg(test)]
+mod throttle_record_tests {
+    use super::transport_missed_throttle;
+
+    /// One named throttle is one decrement: a 429 and a 2xx body are the
+    /// transport's to record, a named throttle under any other status is
+    /// `with_retry`'s, and no status means nothing was answered (CE-30, D-1769).
+    #[test]
+    fn a_throttle_the_transport_recorded_is_not_recorded_again() {
+        for code in [200, 201, 204, 299, 429] {
+            assert!(!transport_missed_throttle(Some(code)), "{code}");
+        }
+        for code in [199, 300, 400, 403, 428, 430, 500, 503] {
+            assert!(transport_missed_throttle(Some(code)), "{code}");
+        }
+        assert!(!transport_missed_throttle(None));
+    }
+}
+
 /// Every secret this feed's [`pull::vendor::AuthScheme`] names, read from
 /// Parameter Store and handed straight to the source.
 ///
@@ -10202,7 +10257,14 @@ pub(crate) async fn credentialed_source(
             "HOME is unset, so ~/.brutex/credentials.toml cannot be located".to_owned(),
         ));
     };
-    let config_path = pull::config::default_config_path(std::path::Path::new(&home));
+    // AN EMPTY OR RELATIVE HOME IS REFUSED, not read as the working
+    // directory's `.brutex/credentials.toml` (CE-38, D-1769).
+    let home = brutex_core::knob::home(Some(home)).map_err(|why| {
+        Unreadable::configuration(format!(
+            "{why}, so ~/.brutex/credentials.toml cannot be located"
+        ))
+    })?;
+    let config_path = pull::config::default_config_path(&home);
     let config = pull::config::CredentialConfig::load(&config_path).map_err(|why| {
         Unreadable::configuration(format!(
             "the credential configuration at {} is not usable: {why}",
@@ -15935,12 +15997,16 @@ pub enum Broker {
 /// `web/src/routes/+layout.svelte` have both said `/` is since they were
 /// written. The server-rendered dashboard it displaced answers at
 /// `/dashboard`, unchanged and still linked from the nav; D-0064.
-pub fn router(site: Loaded) -> axum::Router {
-    router_serving(
+///
+/// # Errors
+///
+/// `BRUTEX_WEB` is set but empty; see [`assets::web_dir`].
+pub fn router(site: Loaded) -> Result<axum::Router, String> {
+    Ok(router_serving(
         site,
-        std::sync::Arc::new(assets::Assets::new(&assets::web_dir())),
+        std::sync::Arc::new(assets::Assets::new(&assets::web_dir()?)),
         DEFAULT_ADDR,
-    )
+    ))
 }
 
 /// Production HTTP surface with durable sweep-control and result-read auditing.
@@ -16292,10 +16358,21 @@ fn route_table(assets: std::sync::Arc<assets::Assets>) -> axum::Router<Loaded> {
         // still serve one leg each; this starts a run that drives them itself,
         // so the operator's retry loop is no longer inside a browser tab. See
         // `crate::pullrun`.
-        .route("/pull/run", axum::routing::post(pull_run))
+        // Both read legs, each repeating its member list twice-encoded, so
+        // both read the run bound, not 8 KiB (P3-01-01, D-1769).
+        .route(
+            "/pull/run",
+            axum::routing::post(pull_run).layer(axum::extract::DefaultBodyLimit::max(
+                crate::pullrun::MAX_RUN_FORM_BYTES,
+            )),
+        )
         .route(
             "/pull/recovery",
-            axum::routing::post(crate::recovery::start).get(crate::recovery::page),
+            axum::routing::post(crate::recovery::start)
+                .layer(axum::extract::DefaultBodyLimit::max(
+                    crate::pullrun::MAX_RUN_FORM_BYTES,
+                ))
+                .get(crate::recovery::page),
         )
         .route(
             "/pull/recovery.json",
@@ -18333,7 +18410,7 @@ fn open_unless_suppressed(url: &str, suppressed: Option<&std::ffi::OsStr>) -> Re
 /// because a server started from `/` or from a read-only directory must still
 /// log somewhere it can write, and the store root is already required to be
 /// writable for the process to do anything at all.
-fn served_log_dir(store_root: &Path) -> PathBuf {
+fn served_log_dir(store_root: &Path) -> Result<PathBuf, String> {
     log_dir_from(
         std::env::var_os(LOG_DIR_ENV),
         std::env::current_dir().ok().as_deref(),
@@ -18376,14 +18453,17 @@ fn log_dir_from(
     named: Option<std::ffi::OsString>,
     cwd: Option<&Path>,
     store_root: &Path,
-) -> PathBuf {
-    if let Some(named) = named {
-        return PathBuf::from(named);
+) -> Result<PathBuf, String> {
+    // SET BUT EMPTY IS REFUSED: `PathBuf::from("")` opened `events.ndjson`
+    // in the working directory, an untracked file in the checkout that gate 1
+    // forbids (CE-36, D-1769).
+    if let Some(named) = brutex_core::knob::folder(LOG_DIR_ENV, named)? {
+        return Ok(named);
     }
-    match cwd {
+    Ok(match cwd {
         Some(cwd) if is_workspace_root(cwd) => cwd.join("logs"),
         _ => telemetry::dir_beneath_store(store_root),
-    }
+    })
 }
 
 /// Read here rather than threaded through [`run_in`] for the same reason
@@ -18510,7 +18590,13 @@ async fn run_in_over(
                 // let a retarget after admission send later writers elsewhere
                 // while this process still held the old store's lock.
                 let store_root = one_server.root.clone();
-                let log_dir = served_log_dir(&store_root);
+                let log_dir = match served_log_dir(&store_root) {
+                    Ok(dir) => dir,
+                    Err(why) => {
+                        warn_line!("REFUSED — {why}. No request was served.");
+                        return FAILED;
+                    }
+                };
                 // The ENV DECIDES THE LEVELS AND THE CALLER DECIDES THE
                 // DIRECTORY. `served_log_level` builds a template it cannot
                 // know the path for, so the directory is set here, where it is
@@ -18528,7 +18614,14 @@ async fn run_in_over(
                 // is not being served looks exactly like a front end that is
                 // broken, and the operator has no way to tell the two apart
                 // from the browser. `CLAUDE.md` §4.
-                let front = std::sync::Arc::new(assets::Assets::new(&assets::web_dir()));
+                let web = match assets::web_dir() {
+                    Ok(web) => web,
+                    Err(why) => {
+                        warn_line!("REFUSED — {why}. No request was served.");
+                        return FAILED;
+                    }
+                };
+                let front = std::sync::Arc::new(assets::Assets::new(&web));
                 announce_front_end(&front);
                 // `serving`, not `load`: this is the one process that may
                 // reach a broker. See `Broker`.
@@ -23658,7 +23751,7 @@ mod tests {
             "a crate is not the workspace root"
         );
         assert_eq!(
-            log_dir_from(None, Some(repo), store),
+            log_dir_from(None, Some(repo), store).expect("no variable"),
             telemetry::dir_beneath_store(store),
             "a crate directory falls back to the store, never to ./logs"
         );
@@ -23670,7 +23763,7 @@ mod tests {
             .expect("crates/api has two parents");
         assert!(is_workspace_root(root), "the workspace root is recognised");
         assert_eq!(
-            log_dir_from(None, Some(root), store),
+            log_dir_from(None, Some(root), store).expect("no variable"),
             root.join("logs"),
             "the workspace root takes ./logs, which is what pressing Run does"
         );
@@ -23894,7 +23987,8 @@ mod tests {
                 Some(std::ffi::OsString::from("/named/elsewhere")),
                 None,
                 store
-            ),
+            )
+            .expect("a named folder"),
             std::path::Path::new("/named/elsewhere"),
             "an explicit BRUTEX_LOGS wins over every probe"
         );
@@ -23906,14 +24000,22 @@ mod tests {
             .and_then(std::path::Path::parent)
             .expect("crates/api has two parents");
         assert_eq!(
-            log_dir_from(Some(std::ffi::OsString::from("/named")), Some(root), store),
+            log_dir_from(Some(std::ffi::OsString::from("/named")), Some(root), store)
+                .expect("a named folder"),
             std::path::Path::new("/named"),
         );
         assert_eq!(
-            log_dir_from(None, None, store),
+            log_dir_from(None, None, store).expect("no variable"),
             telemetry::dir_beneath_store(store),
             "no variable and no working directory falls back to the store"
         );
+        // CE-36, D-1769: set but empty is refused by name, never the cwd.
+        for blank in ["", "  "] {
+            assert!(
+                log_dir_from(Some(std::ffi::OsString::from(blank)), Some(root), store)
+                    .is_err_and(|why| why.starts_with("BRUTEX_LOGS is set but empty"))
+            );
+        }
     }
 
     /// A directory with no manifest at all is not a workspace root, and the
@@ -23929,7 +24031,8 @@ mod tests {
                 None,
                 Some(std::path::Path::new("/nonexistent-uCe1r")),
                 store
-            ),
+            )
+            .expect("no variable"),
             telemetry::dir_beneath_store(store),
         );
     }
@@ -23972,6 +24075,33 @@ mod tests {
         assert_eq!(browser_handler(BrowserHost::Other), ("xdg-open", &[][..]));
     }
 
+    /// CE-33 and CE-38, D-1769: a store or masters folder set but EMPTY, and
+    /// a HOME that is empty or relative, are refused by name, never resolved
+    /// against the working directory.
+    #[test]
+    fn an_empty_folder_or_a_relative_home_is_refused_not_resolved() {
+        for blank in ["", " "] {
+            let why = store_dir_from(Some(blank.into()), Some("/home/who".into()))
+                .expect_err("an empty store refuses");
+            assert!(why.starts_with("BRUTEX_STORE is set but empty"), "{why}");
+            let why = masters_dir_from(Some(blank.into())).expect_err("an empty masters refuses");
+            assert!(why.starts_with("BRUTEX_MASTERS is set but empty"), "{why}");
+            let why = store_dir_from(None, Some(blank.into())).expect_err("an empty HOME refuses");
+            assert!(why.starts_with("HOME is set but empty"), "{why}");
+            let why =
+                default_masters_dir_from(Some(blank.into())).expect_err("an empty HOME refuses");
+            assert!(why.starts_with("HOME is set but empty"), "{why}");
+        }
+        assert!(
+            store_dir_from(None, Some("relative/home".into()))
+                .is_err_and(|why| why.contains("relative"))
+        );
+        assert!(
+            default_masters_dir_from(Some("relative/home".into()))
+                .is_err_and(|why| why.contains("relative"))
+        );
+    }
+
     #[test]
     fn the_store_root_comes_from_the_environment_or_defaults_under_home() {
         assert_eq!(
@@ -25134,7 +25264,8 @@ mod tests {
         let stop_addr = stopper.local_addr().expect("addr");
         let served = tokio::spawn(serve(
             listener,
-            router(Loaded::new(Site::load(&dir, &store_root("health503")))),
+            router(Loaded::new(Site::load(&dir, &store_root("health503"))))
+                .expect("BRUTEX_WEB is not set empty in a test"),
             Box::pin(async move { stopper.accept().await.map(|_| ()) }),
         ));
 
diff --git a/crates/cli/src/boolean_admission_v1.rs b/crates/cli/src/boolean_admission_v1.rs
index 38c1358e..95e536cb 100644
--- a/crates/cli/src/boolean_admission_v1.rs
+++ b/crates/cli/src/boolean_admission_v1.rs
@@ -247,8 +247,10 @@ pub(super) fn base_values(row: &BooleanCoordinateV1) -> Result<AdmissionEvidence
         average_win_paisa: gross_win
             .checked_div(cell.wins)
             .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
-        average_loss_paisa: gross_loss
-            .checked_div(losses)
+        average_loss_paisa: (losses != 0)
+            // Rounded UP: this field is gated by a maximum, and a floored mean
+            // passed a 150 cap at a true 150.5 (p3floor-1, D-1769).
+            .then(|| gross_loss.div_ceil(losses))
             .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
         profit_factor_ppm: ratio(gross_win, gross_loss, "profit factor")?,
         consecutive_losing_streak: measured(u64::from(cell.max_losing_streak)),
diff --git a/crates/cli/src/checksum_receipts.rs b/crates/cli/src/checksum_receipts.rs
index f8fe2188..7f2b13ce 100644
--- a/crates/cli/src/checksum_receipts.rs
+++ b/crates/cli/src/checksum_receipts.rs
@@ -478,7 +478,12 @@ fn regular_generation(
     let held = file.metadata().map_err(error)?;
     let named = fs::symlink_metadata(path).map_err(error)?;
     if !held.is_file() || !named.is_file() || held.nlink() != 1 {
-        return Err("checksum receipt refuses a non-regular file or alias".to_owned());
+        return Err(format!(
+            "checksum receipt refuses {}: it is not a regular file with one link \
+             ({} links); remove any other hard link to it (CE-40, D-1769)",
+            path.display(),
+            held.nlink()
+        ));
     }
     crate::result_set::file_generation(file, path)
 }
diff --git a/crates/cli/src/execution_lease.rs b/crates/cli/src/execution_lease.rs
index 3bbccbec..46b2734b 100644
--- a/crates/cli/src/execution_lease.rs
+++ b/crates/cli/src/execution_lease.rs
@@ -79,9 +79,17 @@ fn verify(file: &File, path: &Path) -> Result<(), Refusal> {
         || !named.is_file()
         || (held.dev(), held.ino()) != (named.dev(), named.ino())
     {
-        return Err(unavailable(
-            "the execution lock is not the same empty regular file; nothing was repaired",
-        ));
+        // NAMED, WITH THE REMEDY. An extra hard link (a `cp -al` snapshot, an
+        // `ln`) refused every sweep for good with a sentence that named no
+        // file (CE-40, D-1769).
+        return Err(unavailable(format!(
+            "the execution lock {} is not the same empty regular file with one \
+             link (it has {} link(s), {} byte(s)); nothing was repaired. Remove \
+             any other hard link to it, or the file itself if no sweep is running",
+            path.display(),
+            held.nlink(),
+            held.len()
+        )));
     }
     Ok(())
 }
@@ -296,5 +304,16 @@ mod tests {
         fs::remove_file(&lock_path).expect("remove private symlink");
         fs::hard_link(&target, &lock_path).expect("hardlink fixture");
         assert!(matches!(probe(&root.0), Err(Refusal::Unavailable(_))));
+        // CE-40, D-1769: the refusal names the lock file, its link count and
+        // what to remove.
+        let why = probe(&root.0)
+            .expect_err("a hard-linked lock refuses")
+            .to_string();
+        let canonical = fs::canonicalize(&root.0)
+            .expect("canonical root")
+            .join(NAME);
+        assert!(why.contains(&canonical.display().to_string()), "{why}");
+        assert!(why.contains("it has 2 link(s)"), "{why}");
+        assert!(why.contains("Remove any other hard link"), "{why}");
     }
 }
diff --git a/crates/cli/src/global_replay_v4.rs b/crates/cli/src/global_replay_v4.rs
index 5616d746..09382f8e 100644
--- a/crates/cli/src/global_replay_v4.rs
+++ b/crates/cli/src/global_replay_v4.rs
@@ -467,6 +467,24 @@ fn push(
 trait VixLookup {
     fn stamp(&mut self, feed: Vendor, ts: i64) -> Result<VixStamp, String>;
 }
+/// The most VIX months a Global Replay V4 run holds at once (CE-8, D-1769).
+const VIX_MONTHS_HELD: usize = 4;
+
+/// Drops the held month with the earliest `(month, feed)` while `held` is at
+/// `cap`, so one insert after it never takes the map past `cap`.
+fn make_room<V>(held: &mut HashMap<(Vendor, store::path::YearMonth), V>, cap: usize) {
+    while held.len() >= cap.max(1) {
+        let Some(oldest) = held
+            .keys()
+            .copied()
+            .min_by_key(|(feed, month)| (*month, *feed))
+        else {
+            return;
+        };
+        held.remove(&oldest);
+    }
+}
+
 struct VixCatalog<'a> {
     root: &'a Path,
     months: HashMap<(Vendor, store::path::YearMonth), VixReferenceMonth>,
@@ -477,6 +495,16 @@ impl VixLookup for VixCatalog<'_> {
             .map_err(|why| why.to_string())?;
         let month = moment.day().year_month().map_err(|why| why.to_string())?;
         if !self.months.contains_key(&(feed, month)) {
+            // BOUNDED, OLDEST OUT. Every opened month was kept for the whole
+            // replay, about 2.86 MB each, so a ten-year span held hundreds of
+            // MB that no record bound counted (CE-8, D-1769). Trades are
+            // stamped in entry order, entry then exit, so only the month being
+            // walked and the one after it are live; the oldest held month is
+            // dropped before a new one opens. A month asked for again is
+            // re-opened from the same file, so the stamp is unchanged and only
+            // the read repeats. The scan is over at most `VIX_MONTHS_HELD`
+            // keys, a constant.
+            make_room(&mut self.months, VIX_MONTHS_HELD);
             self.months.try_reserve(1).map_err(|why| why.to_string())?;
             self.months.insert(
                 (feed, month),
diff --git a/crates/cli/src/global_replay_v4_store.rs b/crates/cli/src/global_replay_v4_store.rs
index faca6f77..c194b5ab 100644
--- a/crates/cli/src/global_replay_v4_store.rs
+++ b/crates/cli/src/global_replay_v4_store.rs
@@ -190,7 +190,12 @@ fn open(path: &Path, writable: bool) -> Result<File, String> {
     {
         use std::os::unix::fs::MetadataExt as _;
         if metadata.nlink() != 1 {
-            return Err("Global Replay V4 refuses hard-link aliases".to_owned());
+            return Err(format!(
+                "Global Replay V4 refuses hard-linked file {} ({} links); remove the \
+                 other hard link to it (CE-40, D-1769)",
+                path.display(),
+                metadata.nlink()
+            ));
         }
     }
     crate::result_set::file_generation(&file, path)?;
diff --git a/crates/cli/src/global_replay_v4_tests.rs b/crates/cli/src/global_replay_v4_tests.rs
index 65245ab1..d7b314c3 100644
--- a/crates/cli/src/global_replay_v4_tests.rs
+++ b/crates/cli/src/global_replay_v4_tests.rs
@@ -569,3 +569,33 @@ fn the_candidate_budget_reserves_three_records_per_candidate() {
     assert!(candidate_budget(209, 200).is_err());
     assert!(candidate_budget(u64::MAX, u64::MAX).is_err());
 }
+
+/// CE-8, D-1769: the VIX month catalogue is bounded. Opening one month past
+/// the cap drops the earliest held one, so a replay over any span holds at
+/// most `VIX_MONTHS_HELD` months, and the months still held are the newest.
+#[test]
+fn the_vix_month_catalogue_drops_the_oldest_month_at_its_cap() {
+    use brutex_core::vendor::Vendor;
+    use std::collections::HashMap;
+    let month = |year, m| store::path::YearMonth::new(year, m).expect("a month");
+    let mut held: HashMap<(Vendor, store::path::YearMonth), u32> = HashMap::new();
+    for (n, m) in (1..=12).enumerate() {
+        super::make_room(&mut held, super::VIX_MONTHS_HELD);
+        held.insert(
+            (Vendor::Dhan, month(2025, m)),
+            u32::try_from(n).expect("small"),
+        );
+        assert!(held.len() <= super::VIX_MONTHS_HELD, "{n}: {}", held.len());
+    }
+    let mut kept: Vec<_> = held.keys().map(|(_, m)| *m).collect();
+    kept.sort_unstable();
+    assert_eq!(
+        kept,
+        (13 - super::VIX_MONTHS_HELD..=12)
+            .map(|m| month(2025, u8::try_from(m).expect("a month number")))
+            .collect::<Vec<_>>()
+    );
+    // A cap of zero still leaves room for the one month being stamped.
+    super::make_room(&mut held, 0);
+    assert!(held.is_empty());
+}
diff --git a/crates/cli/src/index_stop_qualification_metrics.rs b/crates/cli/src/index_stop_qualification_metrics.rs
index 168ffecd..8fd7f65b 100644
--- a/crates/cli/src/index_stop_qualification_metrics.rs
+++ b/crates/cli/src/index_stop_qualification_metrics.rs
@@ -207,9 +207,9 @@ pub(super) fn base<S: Snapshot>(row: &S) -> Result<AdmissionEvidenceValuesV1, St
             .gross_win
             .checked_div(m.wins)
             .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
-        average_loss_paisa: t
-            .gross_loss
-            .checked_div(losses)
+        // Rounded UP: gated by a maximum (p3floor-1, D-1769).
+        average_loss_paisa: (losses != 0)
+            .then(|| t.gross_loss.div_ceil(losses))
             .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
         profit_factor_ppm: ratio(t.gross_win, t.gross_loss, "native profit factor")?,
         consecutive_losing_streak: measured(t.losing_streak),
diff --git a/crates/cli/src/institutional_evidence.rs b/crates/cli/src/institutional_evidence.rs
index 7f67c13f..4d80286e 100644
--- a/crates/cli/src/institutional_evidence.rs
+++ b/crates/cli/src/institutional_evidence.rs
@@ -318,7 +318,7 @@ pub const ADMISSION_EVIDENCE_SOURCE_MATRIX_V1: [EvidenceSourceMapRowV1; 44] = [
     EvidenceSourceMapRowV1 {
         field: "average_loss_paisa",
         availability: EvidenceAvailabilityV1::MeasuredDirect,
-        source: "|Cell.avg_loss()| with explicit no-loser Unmeasured state",
+        source: "Cell.avg_loss_magnitude_ceil() (rounded up) with explicit no-loser Unmeasured state",
     },
     EvidenceSourceMapRowV1 {
         field: "profit_factor_ppm",
@@ -1409,7 +1409,7 @@ fn direct_cell_values(cell: &Cell) -> Result<DirectCellEvidenceV1, String> {
     let average_loss = if losing == 0 {
         ObservedU64V1::Unmeasured
     } else {
-        ObservedU64V1::Measured(cell.avg_loss().unsigned_abs())
+        ObservedU64V1::Measured(cell.avg_loss_magnitude_ceil())
     };
     let worst_loss = negative_magnitude(cell.worst_trade);
     let min_win = nonnegative_u64("minimum win", cell.min_win)?;
diff --git a/crates/cli/src/lib.rs b/crates/cli/src/lib.rs
index ac124989..180f8940 100644
--- a/crates/cli/src/lib.rs
+++ b/crates/cli/src/lib.rs
@@ -376,11 +376,13 @@ usage: cli sweep    SESSIONS MIN_HITS   walk the ladder at one threshold
                                    qualify a predeclared finite catalog on later
                                    real OHLCV across all eight intraday timeframes;
                                    retain every failed check and fixed-training fold.
-       cli boolean-qualified-search-stored VENDOR SYMBOLS FROM_Y FROM_M TO_Y TO_M BITS HORIZON MAX_POINTS BATCH_PROGRAMS NODE_ALLOWANCE BATCH_ALLOWANCE OUTPUT_ROOT LATER_FROM_Y LATER_FROM_M LATER_TO_Y LATER_TO_M
+       cli boolean-qualified-search-stored VENDOR SYMBOLS FROM_Y FROM_M TO_Y TO_M BITS HORIZON MAX_POINTS BATCH_PROGRAMS NODE_ALLOWANCE BATCH_ALLOWANCE OUTPUT_ROOT LATER_FROM_Y LATER_FROM_M LATER_TO_Y LATER_TO_M [TIMEFRAMES]
                                    continue the complete fixed AND/OR/NOT grammar
                                    with one immutable search-wide testing allowance;
-                                   automatically process bounded batches across all
-                                   eight intraday rungs and retain later qualification.
+                                   automatically process bounded batches across the
+                                   comma-separated TIMEFRAMES, or all eight intraday
+                                   rungs when it is omitted, and retain later
+                                   qualification.
                                    Work limits pause; they never claim exhaustion.
        cli auto-stored VENDOR UNDERLYING RUNG FROM_Y FROM_M TO_Y TO_M
                                    the same search, over REAL stored bars. There
@@ -472,8 +474,8 @@ usage: cli sweep    SESSIONS MIN_HITS   walk the ladder at one threshold
                                    cheap ceiling toward the floor one trade a week
                                    implies, stopping at the first support that
                                    admits a row. Cheap answers arrive first.
-                                   Every rule is on: 80%% of trades won AND 80%% on
-                                   the 95%% lower bound, the smallest win at least
+                                   Every rule is on: 80% of trades won AND 80% on
+                                   the 95% lower bound, the smallest win at least
                                    3x the largest loss, total profit at least 5x
                                    the worst peak-to-trough fall, and the weakest
                                    calendar grain still half positive. There is NO
@@ -585,7 +587,8 @@ usage: cli sweep    SESSIONS MIN_HITS   walk the ladder at one threshold
 SESSIONS  how many generated trading days to sweep, 1..=3650
 MIN_HITS  bars a combination must fire on to be kept, 1 or more
 VENDOR    the feed that wrote them -- groww, dhan, truedata, gdfl, zerodha
-UNDERLYING  the index, e.g. NIFTY or BANKNIFTY
+UNDERLYING  the index, e.g. NIFTY or BANKNIFTY, or one of the F&O
+            underlyings that is a share, e.g. RELIANCE (its cash equity)
 RUNG      the bar length as its directory word -- 1min, 1day
 
 The stored commands read a run identity off the build. They refuse unless the
@@ -2943,12 +2946,20 @@ fn root_from(
     explicit: Option<std::ffi::OsString>,
     home: Option<std::ffi::OsString>,
 ) -> Result<std::path::PathBuf, stored::Refusal> {
-    if let Some(explicit) = explicit {
-        return Ok(std::path::PathBuf::from(explicit));
+    // ONE READER WITH THE API'S. An empty BRUTEX_STORE or an empty or relative
+    // HOME is refused by name rather than resolved against the working
+    // directory, where `.brutex/store` could exist and canonicalize (CE-38,
+    // D-1769).
+    if let Some(explicit) = brutex_core::knob::folder("BRUTEX_STORE", explicit)? {
+        return Ok(explicit);
     }
     home.map_or_else(
         || Err("neither BRUTEX_STORE nor HOME is set, so the store cannot be found".to_owned()),
-        |home| Ok(std::path::PathBuf::from(home).join(".brutex").join("store")),
+        |home| {
+            Ok(brutex_core::knob::home(Some(home))?
+                .join(".brutex")
+                .join("store"))
+        },
     )
 }
 
@@ -2990,10 +3001,12 @@ fn root_from(
 fn log_dir_from(
     explicit: Option<std::ffi::OsString>,
     store: Option<std::path::PathBuf>,
-) -> Option<std::path::PathBuf> {
-    explicit
-        .map(std::path::PathBuf::from)
-        .or_else(|| store.map(|s| s.join("logs").join("cli")))
+) -> Result<Option<std::path::PathBuf>, String> {
+    // SET BUT EMPTY IS REFUSED, NOT RESOLVED. `PathBuf::from("")` opened
+    // `events.ndjson` in the working directory, the banner printed
+    // `events -> ` and `/logs` never read it (CE-5, D-1769).
+    Ok(brutex_core::knob::folder("BRUTEX_LOG_DIR", explicit)?
+        .or_else(|| store.map(|s| s.join("logs").join("cli"))))
 }
 
 /// Installs the process-wide event sink, or says why it could not.
@@ -3058,7 +3071,11 @@ fn log_dir_from(
 /// unwritable one by changing permissions.
 #[must_use]
 pub fn install_log() -> String {
-    let Some(dir) = log_dir_from(std::env::var_os("BRUTEX_LOG_DIR"), store_root().ok()) else {
+    let dir = match log_dir_from(std::env::var_os("BRUTEX_LOG_DIR"), store_root().ok()) {
+        Ok(dir) => dir,
+        Err(why) => return format!("events are NOT being recorded: {why}"),
+    };
+    let Some(dir) = dir else {
         return "events are NOT being recorded: neither BRUTEX_LOG_DIR nor a store root \
                 is set, so there is nowhere to write them. Set BRUTEX_LOG_DIR, or set \
                 BRUTEX_STORE or HOME so the log can sit beside the store."
@@ -10341,7 +10358,14 @@ fn cap_within_budget(sampled: usize, elapsed_nanos: u128, budget_ms: u64, offere
 /// `validate: false` and only the rung that lands is re-run with the stack. This
 /// gives the same choice to the command an operator reaches for first.
 fn validate_from_env() -> bool {
-    validates(crate::knobs::var("BRUTEX_VALIDATE").as_deref())
+    let raw = crate::knobs::var("BRUTEX_VALIDATE");
+    validates(raw.as_deref()).unwrap_or_else(|| {
+        // A WORD THE SWITCH DOES NOT TAKE IS NAMED, AND THE STACK STAYS ON.
+        // The banner's KNOB REFUSED block says which value was not used
+        // (CE-6, D-1769).
+        crate::knobs::refuse_value("BRUTEX_VALIDATE", raw.as_deref().unwrap_or_default());
+        true
+    })
 }
 
 /// The rule itself, over the raw value, so it can be tested without touching the
@@ -10352,14 +10376,17 @@ fn validate_from_env() -> bool {
 /// from the LOOKUP is what makes the rule provable at all — and the reader above
 /// is then one line with nothing left to get wrong.
 ///
-/// Only a literal `0` turns validation off. A malformed or unexpected value
-/// leaves it ON: a typo must never silently buy a weaker answer, which is the
-/// same argument §4 makes against a fallback that hides a failure.
-fn validates(raw: Option<&str>) -> bool {
-    // NOT a `const fn`: matching a `&str` in one needs `PartialEq` as a const
-    // trait, which is not stable. `trim` is the honest rule anyway -- an
-    // operator who exports the variable with a trailing space meant `0`.
-    raw.is_none_or(|value| value.trim() != "0")
+/// The words are `brutex_core::knob::switch`'s — `0`, `false`, `off`, `no`
+/// turn validation off, `1`, `true`, `on`, `yes` leave it on, in any case and
+/// with surrounding space ignored — the same eight the request path already
+/// took. Only a literal `0` used to turn it off here, so `false`, `off` and
+/// `no` silently left it ON against the operator's stated choice (CE-6,
+/// D-1769). Any other word is `None`: [`validate_from_env`] keeps the stack on
+/// and names the refused value, because a typo must never silently buy a
+/// weaker answer, which is the same argument §4 makes against a fallback that
+/// hides a failure.
+fn validates(raw: Option<&str>) -> Option<bool> {
+    brutex_core::knob::switch("BRUTEX_VALIDATE", raw, true).ok()
 }
 
 /// What a report says when the validation stack did not run.
@@ -15085,8 +15112,13 @@ fn return_over_drawdown_cell(pessimistic: i64, max_drawdown: i64) -> String {
         // A variant that never gave anything back. Unbounded, so it is named
         // rather than printed as a number no divisor produced.
         i64::MAX => "inf".to_owned(),
-        // No profit to divide. `-` is the honest answer, not a zero.
-        0 => "-".to_owned(),
+        // No profit to divide. `-` is the honest answer, not a zero — and it
+        // is decided from the MONEY, not from the ratio. The ratio is also 0
+        // for a profitable variant whose return is under a hundredth of its
+        // drawdown (1,000 paisa against 200,000), and printing `-` there told
+        // the reader it made nothing (CE-17, D-1769).
+        0 if pessimistic <= 0 => "-".to_owned(),
+        0 => "<0.01".to_owned(),
         // HUNDREDTHS, RENDERED AS A DECIMAL, and the raw integer was a second
         // way to misread this column. `return_over_drawdown` is `x100` by the
         // same convention `win_rate_bp` and `profit_factor_bp` use, so a run
@@ -17889,9 +17921,21 @@ fn publish_ranked(
                   threshold does not move a verdict that is 3.42 against 6.19. The
                   multiply is the only arithmetic and it touches no price: gate
                   11 counts the float TYPE NAME and this line writes
-                  none."
+                  none. ROUNDED AGAINST THE FINDING, though: truncating put
+                  the bar up to a milli BELOW itself, and `t_milli` is rounded,
+                  so a |t| 1.5 milli short of the bar read as clearing it
+                  (CE-7, D-1769). The bar is now the ceiling, the page compares
+                  strictly above it, and a bar that is not finite is one no row
+                  clears."
     )]
-    let bar_milli = (bar * 1_000.0) as i64;
+    let bar_milli = {
+        let scaled = bar * 1_000.0;
+        if scaled.is_finite() {
+            scaled.ceil() as i64
+        } else {
+            i64::MAX
+        }
+    };
     let summary = crate::live::Summary {
         trials,
         bar_milli,
@@ -21549,6 +21593,16 @@ mod tests {
             "the refusal names both: {why}"
         );
         assert!(why.contains("HOME"), "the refusal names both: {why}");
+        // CE-38, D-1769: empty or relative values are refused by name.
+        assert!(
+            root_from(Some("".into()), Some("/Users/x".into()))
+                .is_err_and(|why| why.starts_with("BRUTEX_STORE is set but empty"))
+        );
+        assert!(
+            root_from(None, Some("".into()))
+                .is_err_and(|why| why.starts_with("HOME is set but empty"))
+        );
+        assert!(root_from(None, Some("rel".into())).is_err_and(|why| why.contains("relative")));
     }
 
     /// A MISSING STORE ROOT IS A REFUSAL, NEVER AN IMPLICIT DIRECTORY CREATE.
@@ -21926,7 +21980,7 @@ mod tests {
                 Some(OsString::from("/tmp/brutex-events")),
                 Some(PathBuf::from("/srv/store")),
             ),
-            Some(PathBuf::from("/tmp/brutex-events")),
+            Ok(Some(PathBuf::from("/tmp/brutex-events"))),
         );
 
         // WITH NO VARIABLE, THE LOG SITS UNDER THE STORE, IN ITS OWN
@@ -21944,7 +21998,7 @@ mod tests {
         // which is the only shape that stays O(1) per event with two writers.
         assert_eq!(
             log_dir_from(None, Some(PathBuf::from("/srv/store"))),
-            Some(PathBuf::from("/srv/store/logs/cli")),
+            Ok(Some(PathBuf::from("/srv/store/logs/cli"))),
             "the CLI owns a child directory so it cannot interleave with the \
              server's file"
         );
@@ -21953,13 +22007,44 @@ mod tests {
         // guess at the working directory. `install_log` turns this into the
         // printed "events are NOT being recorded" line rather than a silent
         // absence — degrade loudly, per §4.
-        assert_eq!(log_dir_from(None, None), None);
+        assert_eq!(log_dir_from(None, None), Ok(None));
+        // CE-5, D-1769: set but EMPTY is refused by name, never the cwd, and
+        // never quietly replaced by the store's directory either.
+        for blank in ["", " "] {
+            assert!(
+                log_dir_from(
+                    Some(OsString::from(blank)),
+                    Some(PathBuf::from("/srv/store"))
+                )
+                .is_err_and(|why| why.starts_with("BRUTEX_LOG_DIR is set but empty"))
+            );
+        }
 
         // AND AN EXPLICIT DIRECTORY STILL WINS WITH NO STORE AT ALL, which is
         // the case for an operator who has moved the store away entirely.
         assert_eq!(
             log_dir_from(Some(OsString::from("/var/log/brutex")), None),
-            Some(PathBuf::from("/var/log/brutex")),
+            Ok(Some(PathBuf::from("/var/log/brutex"))),
+        );
+    }
+
+    /// P3-02-03..05, D-1769: the help text names the optional TIMEFRAMES
+    /// argument, prints no printf escape, and names the cash equities on the
+    /// UNDERLYING line (CLAUDE.md §1).
+    #[test]
+    fn the_usage_text_states_what_the_commands_actually_take() {
+        assert!(!USAGE.contains("%%"), "a Rust &str has no printf escapes");
+        assert!(USAGE.contains("80% of trades won"));
+        assert!(USAGE.contains("LATER_TO_M [TIMEFRAMES]"));
+        let underlying = USAGE
+            .lines()
+            .skip_while(|line| !line.starts_with("UNDERLYING"))
+            .take(2)
+            .collect::<Vec<_>>()
+            .join(" ");
+        assert!(
+            underlying.contains("NIFTY") && underlying.contains("cash equity"),
+            "{underlying}"
         );
     }
 
@@ -23303,9 +23388,9 @@ mod tests {
     /// in shape to one that ran all three, so the banner is the only thing
     /// separating a CANDIDATE from a FINDING.
     ///
-    /// The switch defaults ON and only a literal `0` turns it off: a typo must
+    /// The switch defaults ON and only an off word turns it off: a typo must
     /// not silently buy a weaker answer, which is why a malformed value is not
-    /// read as off.
+    /// read as off, and is named (CE-6, D-1769).
     #[test]
     fn an_unvalidated_screen_cannot_be_mistaken_for_a_validated_one() {
         assert!(
@@ -23328,26 +23413,35 @@ mod tests {
             );
         }
 
-        // THE DEFAULT IS ON, AND ONLY `0` TURNS IT OFF.
+        // THE DEFAULT IS ON, AND THE EIGHT SWITCH WORDS DECIDE IT.
         //
         // Asserted on `validates`, the same decision `validate_from_env` calls,
         // rather than by re-deriving the rule here — so a change to one is a
         // change to both. The lookup is not tested because it cannot be: this
         // crate is `#![forbid(unsafe_code)]` and `std::env::set_var` is unsafe.
         // Splitting the decision out is what made the rule provable at all.
+        // CE-6, D-1769: the eight switch words, and anything else is refused
+        // (`None`), which `validate_from_env` turns into ON plus a named
+        // KNOB REFUSED line.
         for (raw, want, why) in [
-            (None, true, "absent means the full stack runs"),
-            (Some("0"), false, "a literal zero is the only way off"),
-            (Some(" 0 "), false, "and whitespace around it still counts"),
-            (Some("1"), true, "any other value leaves it on"),
+            (None, Some(true), "absent means the full stack runs"),
+            (Some("0"), Some(false), "zero turns it off"),
+            (
+                Some(" 0 "),
+                Some(false),
+                "and whitespace around it still counts",
+            ),
+            (Some("1"), Some(true), "one leaves it on"),
             (
                 Some("no"),
-                true,
-                "including one that LOOKS like an off switch",
+                Some(false),
+                "an off word is honoured, not reversed",
             ),
-            (Some("false"), true, "and one that reads like one"),
-            (Some(""), true, "and empty is not off"),
-            (Some("00"), true, "and a near-miss is not off either"),
+            (Some("FALSE"), Some(false), "in any case"),
+            (Some("off"), Some(false), "all four of them"),
+            (Some("yes"), Some(true), "and an on word is on"),
+            (Some(""), None, "empty is refused, not read as either"),
+            (Some("00"), None, "and a near-miss is refused by name"),
         ] {
             assert_eq!(validates(raw), want, "{why}");
         }
@@ -25954,6 +26048,9 @@ mod tests {
         // No profit to divide.
         assert_eq!(return_over_drawdown_cell(0, 29_163), "-");
         assert_eq!(return_over_drawdown_cell(-5_000, 29_163), "-");
+        // CE-17, D-1769: a PROFIT under a hundredth of its drawdown is not
+        // "no profit". The ratio truncates to 0; the money decides the dash.
+        assert_eq!(return_over_drawdown_cell(1_000, 200_000), "<0.01");
 
         // THE REGRESSION ITSELF. A negative drawdown is not a value the engine
         // can produce; if one ever reaches here the answer must not be a
@@ -25976,7 +26073,8 @@ mod tests {
                 };
                 let want = match cell.return_over_drawdown() {
                     i64::MAX => "inf".to_owned(),
-                    0 => "-".to_owned(),
+                    0 if pess <= 0 => "-".to_owned(),
+                    0 => "<0.01".to_owned(),
                     r => hundredths_of(r),
                 };
                 assert_eq!(
diff --git a/crates/cli/src/pool.rs b/crates/cli/src/pool.rs
index ec9f6375..66017f93 100644
--- a/crates/cli/src/pool.rs
+++ b/crates/cli/src/pool.rs
@@ -678,17 +678,22 @@ fn render_per_symbol(out: &mut String, screened: &[Screened]) {
     // SORTED BY THE MONEY, not by name: the smallest drawdown first, then the
     // worst trade closest to zero, then the net. Refusals sort last and are
     // named, never dropped.
+    //
+    // ONE ASCENDING SORT, NO REVERSE. This sorted `(refused, -dd, worst, net)`
+    // ascending and then reversed the whole list, which got the money order
+    // right and put every refusal on TOP, against the sentence above, and
+    // listed tied rows in reverse input order (CE-4, D-1769). The key now
+    // states each direction itself and the sort is stable.
     let mut rows: Vec<&Screened> = screened.iter().collect();
     rows.sort_by_key(|s| match &s.outcome {
         Ok(r) => (
             false,
-            r.max_drawdown.saturating_neg(),
-            r.worst_trade,
-            r.pessimistic,
+            r.max_drawdown,
+            core::cmp::Reverse(r.worst_trade),
+            core::cmp::Reverse(r.pessimistic),
         ),
-        Err(_) => (true, i64::MIN, i64::MIN, i64::MIN),
+        Err(_) => (true, 0, core::cmp::Reverse(0), core::cmp::Reverse(0)),
     });
-    rows.reverse();
     // LAID OUT TOGETHER (D-1420). Raw paisa at `i64::MIN` is 20 characters
     // in a 12-character column, and a 14-character symbol filled its column,
     // so `worst`, `net` and `max_dd` could read as one number.
@@ -2104,6 +2109,51 @@ mod tests {
         (writes, renders, returns)
     }
 
+    /// Pass 1 lists priced rows by the money, smallest drawdown first, ties in
+    /// the order they were screened, and every refusal LAST (CE-4, D-1769).
+    #[test]
+    fn pass_one_puts_refusals_last_and_keeps_ties_in_screened_order() {
+        let record = |dd: i64, worst: i64, net: i64| crate::results::Record {
+            max_drawdown: dd,
+            worst_trade: worst,
+            pessimistic: net,
+            ..crate::results::Record::from_bytes(&[0; crate::results::STRIDE_BYTES])
+        };
+        let row = |symbol: &str, outcome| super::Screened {
+            symbol: symbol.to_owned(),
+            outcome,
+        };
+        let screened = [
+            row("REFUSED_A", Err("a".to_owned())),
+            row("DEEP", Ok(record(900, -50, 10))),
+            row("TIE_FIRST", Ok(record(100, -20, 5))),
+            row("TIE_SECOND", Ok(record(100, -20, 5))),
+            row("SHALLOW_WORSE", Ok(record(100, -80, 5))),
+            row("REFUSED_B", Err("b".to_owned())),
+            row("SHALLOWEST", Ok(record(10, -90, 1))),
+        ];
+        let mut out = String::new();
+        super::render_per_symbol(&mut out, &screened);
+        let order: Vec<&str> = out
+            .lines()
+            .filter_map(|line| line.split_whitespace().next())
+            .filter(|word| screened.iter().any(|s| s.symbol == *word))
+            .collect();
+        assert_eq!(
+            order,
+            [
+                "SHALLOWEST",
+                "TIE_FIRST",
+                "TIE_SECOND",
+                "SHALLOW_WORSE",
+                "DEEP",
+                "REFUSED_A",
+                "REFUSED_B"
+            ],
+            "{out}"
+        );
+    }
+
     /// **Each renderer `run_under` hands the page to only appends to it.**
     /// D-0696.
     ///
diff --git a/crates/cli/src/population_admission_writer.rs b/crates/cli/src/population_admission_writer.rs
index f8d3c3d2..efb5cdc9 100644
--- a/crates/cli/src/population_admission_writer.rs
+++ b/crates/cli/src/population_admission_writer.rs
@@ -1308,7 +1308,7 @@ pub(crate) fn metrics_from_cell(
         u64::try_from(cell.min_win).map_err(|_| "cell minimum win does not fit u64".to_owned())?;
     let average_win =
         u64::try_from(cell.avg_win()).map_err(|_| "cell average win is negative".to_owned())?;
-    let average_loss = negative_magnitude(cell.avg_loss());
+    let average_loss = cell.avg_loss_magnitude_ceil();
     Ok(TopMetricsV1 {
         drawdown,
         worst_loss,
diff --git a/crates/cli/src/population_base_evidence_v2.rs b/crates/cli/src/population_base_evidence_v2.rs
index cae062dc..c3bb0768 100644
--- a/crates/cli/src/population_base_evidence_v2.rs
+++ b/crates/cli/src/population_base_evidence_v2.rs
@@ -607,8 +607,10 @@ impl BaseEvidenceRecordV2 {
             average_win_paisa: gross_win
                 .checked_div(a.wins)
                 .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
-            average_loss_paisa: gross_loss
-                .checked_div(a.losses)
+            average_loss_paisa: (a.losses != 0)
+                // Rounded UP: this field is gated by a maximum, and a floored mean
+                // passed a 150 cap at a true 150.5 (p3floor-1, D-1769).
+                .then(|| gross_loss.div_ceil(a.losses))
                 .map_or(ObservedU64V1::Unmeasured, ObservedU64V1::Measured),
             profit_factor_ppm: ratio_observed(gross_win, gross_loss, "profit factor")?,
             consecutive_losing_streak: measured_when(
diff --git a/crates/cli/src/search_checkpoint.rs b/crates/cli/src/search_checkpoint.rs
index 3bd04ee2..5e6d8612 100644
--- a/crates/cli/src/search_checkpoint.rs
+++ b/crates/cli/src/search_checkpoint.rs
@@ -415,6 +415,12 @@ fn error(why: impl std::fmt::Display) -> String {
 #[path = "search_checkpoint_tests.rs"]
 pub(crate) mod tests;
 
+/// Finder's `.DS_Store` and the `._<name>` `AppleDouble` files a copy to a
+/// non-HFS volume writes: operating-system litter, never a reservation.
+fn is_os_litter(name: &str) -> bool {
+    name == ".DS_Store" || name.starts_with("._")
+}
+
 fn discover(directory: &Path) -> Result<(u64, Option<u64>, u64, u64), String> {
     discover_through(directory, None)
 }
@@ -438,13 +444,29 @@ fn discover_through(
         if name == "owner.lock" {
             continue;
         }
-        let sequence = u64::from_str_radix(&name, 16)
-            .map_err(|_| "invalid checkpoint reservation name".to_owned())?;
+        // MACOS LITTER IS NOT A RESERVATION, AND A STRANGER IS NAMED. Finder
+        // writes `.DS_Store` into a folder it opens, and a copy to a non-HFS
+        // volume writes `._<name>` beside each file; either one refused every
+        // start, resume and dashboard read of this search with a sentence that
+        // named no file (CE-34, D-1769). Those two, as plain files, are passed
+        // over; anything else still refuses, now by its name.
+        if is_os_litter(&name) && entry.file_type().map_err(error)?.is_file() {
+            continue;
+        }
+        let sequence = u64::from_str_radix(&name, 16).map_err(|_| {
+            format!(
+                "invalid checkpoint reservation name {name:?} in {}",
+                directory.display()
+            )
+        })?;
         if sequence == 0
             || name != format!("{sequence:016x}")
             || !entry.file_type().map_err(error)?.is_dir()
         {
-            return Err("invalid checkpoint reservation type or sequence".to_owned());
+            return Err(format!(
+                "invalid checkpoint reservation type or sequence {name:?} in {}",
+                directory.display()
+            ));
         }
         next = next.max(
             sequence
diff --git a/crates/cli/src/search_checkpoint_tests.rs b/crates/cli/src/search_checkpoint_tests.rs
index 8e1eb897..ebed5aa8 100644
--- a/crates/cli/src/search_checkpoint_tests.rs
+++ b/crates/cli/src/search_checkpoint_tests.rs
@@ -235,3 +235,27 @@ fn a_dropped_search_journal_is_released_despite_a_duplicated_descriptor() -> Res
     drop(child);
     Ok(())
 }
+
+/// CE-34, D-1769: Finder's `.DS_Store` and an `AppleDouble` `._` file are
+/// passed over, and any other stray entry refuses BY NAME.
+#[test]
+fn os_litter_is_passed_over_and_a_stranger_is_named() -> Result<(), String> {
+    let scratch = Scratch::new().map_err(error)?;
+    let dir = scratch.0.join("litter");
+    fs::create_dir_all(dir.join(format!("{:016x}", 1))).map_err(error)?;
+    fs::write(dir.join(".DS_Store"), b"finder").map_err(error)?;
+    fs::write(dir.join("._0000000000000001"), b"appledouble").map_err(error)?;
+    let (next, ..) = discover(&dir)?;
+    assert_eq!(next, 2, "litter does not block the search");
+    // Litter by name but a DIRECTORY is not litter.
+    fs::create_dir(dir.join("._odd")).map_err(error)?;
+    let why = discover(&dir)
+        .err()
+        .ok_or("a directory is not passed over")?;
+    assert!(why.contains("\"._odd\""), "{why}");
+    fs::remove_dir(dir.join("._odd")).map_err(error)?;
+    fs::write(dir.join("notes.txt"), b"x").map_err(error)?;
+    let why = discover(&dir).err().ok_or("a stranger refuses")?;
+    assert!(why.contains("\"notes.txt\""), "{why}");
+    Ok(())
+}
diff --git a/crates/cli/src/selection_v6.rs b/crates/cli/src/selection_v6.rs
index 15a3209f..bd8ba3e7 100644
--- a/crates/cli/src/selection_v6.rs
+++ b/crates/cli/src/selection_v6.rs
@@ -241,7 +241,12 @@ fn open(root: &Path, writable: bool) -> Result<(File, PathBuf), String> {
     {
         use std::os::unix::fs::MetadataExt as _;
         if metadata.nlink() != 1 {
-            return Err("Selection V6 refuses aliased hard-linked files".to_owned());
+            return Err(format!(
+                "Selection V6 refuses aliased hard-linked file {} ({} links); remove \
+                 the other hard link to it (CE-40, D-1769)",
+                path.display(),
+                metadata.nlink()
+            ));
         }
     }
     crate::result_set::file_generation(&file, &path)?;
diff --git a/crates/cli/src/stored.rs b/crates/cli/src/stored.rs
index 7966ccbf..351aad5a 100644
--- a/crates/cli/src/stored.rs
+++ b/crates/cli/src/stored.rs
@@ -2916,6 +2916,42 @@ fn daily_eligibility_of(day: i64) -> Result<DailyEligibility, Refusal> {
     }
 }
 
+/// The first signal day's daily context must be anchored to the session it
+/// follows on the canonical calendar (CE-10, D-1769).
+fn anchored_to_prior_session(
+    references: &[DailyReference],
+    first_signal_day: i64,
+) -> Result<(), Refusal> {
+    // THE NEWEST ELIGIBLE RECORD BEFORE THE FIRST SIGNAL DAY MUST BE THE
+    // SESSION THAT DAY FOLLOWS. This asked only for ANY eligible record before
+    // it, and `daily_context_from_span`'s per-day walk covers a day only once a
+    // previous signal day exists, so a previous-month file that ended early
+    // anchored the first day's pivots, previous-day and gap bits to an older
+    // session while GapFib,
+    // which asks `prior_accepted_session`, anchored to the right one (CE-10,
+    // D-1769). Both sides now ask the canonical calendar the same question.
+    let newest_before = references
+        .iter()
+        .rev()
+        .find(|reference| {
+            reference.ist_day() < first_signal_day
+                && reference.eligibility() == DailyEligibility::Eligible
+        })
+        .map(DailyReference::ist_day);
+    let Some(newest_before) = newest_before else {
+        return Err(format!(
+            "the first signal IST day {first_signal_day} has no eligible stored 1day record strictly before it. Same-day OHLCV, a coarse reconstruction, and a guessed holiday are all forbidden; load the preceding daily history"
+        ));
+    };
+    let (prior_session, _) = prior_accepted_session(first_signal_day)?;
+    if newest_before != prior_session {
+        return Err(format!(
+            "the first signal IST day {first_signal_day} follows the accepted session {prior_session}, but the newest eligible stored 1day record before it is {newest_before}. The daily file ends early, and anchoring the first day's previous-day, pivot and gap context to an older session is refused rather than guessed; load the missing daily history"
+        ));
+    }
+    Ok(())
+}
+
 /// Turn a complete stored one-day span into explicit causal reference records.
 pub(crate) fn daily_context_from_span(
     daily: Span,
@@ -2975,14 +3011,7 @@ pub(crate) fn daily_context_from_span(
         eligibility.push(u8::from(decision == DailyEligibility::Eligible));
     }
 
-    if !references.iter().any(|reference| {
-        reference.ist_day() < first_signal_day
-            && reference.eligibility() == DailyEligibility::Eligible
-    }) {
-        return Err(format!(
-            "the first signal IST day {first_signal_day} has no eligible stored 1day record strictly before it. Same-day OHLCV, a coarse reconstruction, and a guessed holiday are all forbidden; load the preceding daily history"
-        ));
-    }
+    anchored_to_prior_session(&references, first_signal_day)?;
 
     // Every regular signal session that is followed by another observed signal
     // session must itself have a stored daily record.  The signal stream is the
@@ -4672,6 +4701,34 @@ mod tests {
         );
     }
 
+    /// CE-10, D-1769: a daily file that ends before the session the first
+    /// signal day follows refuses, instead of anchoring that day to an older
+    /// session. Monday's record exists, Tuesday traded and has none, and the
+    /// first signal day is Wednesday.
+    #[test]
+    fn a_first_signal_day_whose_prior_session_has_no_daily_record_refuses() {
+        let daily = daily_span(vec![candle_on_ist_day(OPEN_MONDAY_2026_08_03, 2_500_000)]);
+        let signal = [candle_on_ist_day(OPEN_WEDNESDAY_2026_08_05, 2_600_100)];
+        let why = daily_context_from_span(daily, &signal)
+            .expect_err("Wednesday follows Tuesday, not Monday");
+        assert!(
+            why.contains(&format!(
+                "follows the accepted session {OPEN_TUESDAY_2026_08_04}"
+            )),
+            "{why}"
+        );
+        assert!(
+            why.contains(&format!("is {OPEN_MONDAY_2026_08_03}")),
+            "{why}"
+        );
+        // And with Tuesday's record present the same first day is accepted.
+        let daily = daily_span(vec![
+            candle_on_ist_day(OPEN_MONDAY_2026_08_03, 2_500_000),
+            candle_on_ist_day(OPEN_TUESDAY_2026_08_04, 2_550_000),
+        ]);
+        daily_context_from_span(daily, &signal).expect("the prior session is on file");
+    }
+
     #[test]
     fn a_non_regular_observed_day_is_not_invented_as_an_eligible_anchor() {
         let non_regular = CHARTER_NON_REGULAR_IST_DAYS[0];
diff --git a/crates/core/src/knob.rs b/crates/core/src/knob.rs
new file mode 100644
index 00000000..dfc2a871
--- /dev/null
+++ b/crates/core/src/knob.rs
@@ -0,0 +1,141 @@
+//! How an environment value is read when it names a folder or a switch.
+//!
+//! # Why one reader, here
+//!
+//! Every crate that takes a folder from the environment read it as
+//! `PathBuf::from(value)` and refused only an UNSET variable. A variable set
+//! but EMPTY is `PathBuf::from("")`, a path relative to the working directory,
+//! so `BRUTEX_LOG_DIR=` wrote the event log into the cwd, `BRUTEX_STORE=` put
+//! `bars/` and `manifest/` there, and an empty `HOME` turned every `$HOME/...`
+//! default into a cwd-relative one (CE-5, CE-33, CE-36, CE-37, CE-38). And two
+//! switches turned off only for a literal `0`, so `false`, `off` and `no` left
+//! the operator's stated choice silently reversed (CE-6, CE-39).
+//!
+//! Each copy was fixed once, here, so the next reader cannot drift from it
+//! (D-1769). The functions take the RAW value rather than reading the
+//! environment, so the rule is testable without `set_var`, which is unsafe and
+//! which the crates that call this forbid.
+
+use std::ffi::OsString;
+use std::path::PathBuf;
+
+/// A folder named by `name`, or `None` when the variable is unset.
+///
+/// # Errors
+///
+/// The value is empty or only whitespace. That is a path relative to the
+/// working directory, never a folder the operator meant, and it is refused by
+/// the variable's name rather than resolved.
+pub fn folder(name: &str, raw: Option<OsString>) -> Result<Option<PathBuf>, String> {
+    let Some(raw) = raw else {
+        return Ok(None);
+    };
+    if raw.to_string_lossy().trim().is_empty() {
+        return Err(format!(
+            "{name} is set but empty. An empty folder is the working directory, \
+             which is never what an operator names; unset it to take the default \
+             or give a folder"
+        ));
+    }
+    Ok(Some(PathBuf::from(raw)))
+}
+
+/// The operator's home directory from `HOME`.
+///
+/// # Errors
+///
+/// `HOME` is unset, empty or relative. Every `$HOME/...` default is a place
+/// this build writes or reads secrets from, and a relative `HOME` would make
+/// it the working directory instead.
+pub fn home(raw: Option<OsString>) -> Result<PathBuf, String> {
+    let home = folder("HOME", raw)?.ok_or_else(|| "HOME is not set".to_owned())?;
+    if home.is_relative() {
+        return Err(format!(
+            "HOME is {:?}, which is relative. Every default under it would be \
+             read from the working directory instead",
+            home.display()
+        ));
+    }
+    Ok(home)
+}
+
+/// An on/off switch named by `name`, or `default` when the variable is unset.
+///
+/// The words are the ones `BRUTEX_VALIDATE` already took through the request
+/// path: `1`, `true`, `on`, `yes` and `0`, `false`, `off`, `no`, in any case,
+/// with surrounding space ignored.
+///
+/// # Errors
+///
+/// Any other value, refused by name. A typo must never silently keep a switch
+/// the operator meant to flip.
+pub fn switch(name: &str, raw: Option<&str>, default: bool) -> Result<bool, String> {
+    let Some(raw) = raw else {
+        return Ok(default);
+    };
+    match raw.trim().to_ascii_lowercase().as_str() {
+        "1" | "true" | "on" | "yes" => Ok(true),
+        "0" | "false" | "off" | "no" => Ok(false),
+        _ => Err(format!(
+            "{name}={raw:?} is not a switch. Use 1, true, on or yes to turn it on, \
+             and 0, false, off or no to turn it off"
+        )),
+    }
+}
+
+#[cfg(test)]
+#[allow(clippy::unwrap_used, clippy::expect_used)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn an_empty_or_blank_folder_is_refused_by_name_and_unset_is_none() {
+        assert_eq!(folder("BRUTEX_STORE", None), Ok(None));
+        for blank in ["", " ", "\t\n"] {
+            let why = folder("BRUTEX_STORE", Some(blank.into())).unwrap_err();
+            assert!(why.starts_with("BRUTEX_STORE is set but empty"), "{why}");
+        }
+        assert_eq!(
+            folder("BRUTEX_STORE", Some("/data/store".into())),
+            Ok(Some(PathBuf::from("/data/store")))
+        );
+    }
+
+    #[test]
+    fn home_must_be_set_non_empty_and_absolute() {
+        assert_eq!(home(None).unwrap_err(), "HOME is not set");
+        assert!(
+            home(Some("".into()))
+                .unwrap_err()
+                .contains("HOME is set but empty")
+        );
+        assert!(
+            home(Some("rel/dir".into()))
+                .unwrap_err()
+                .contains("relative")
+        );
+        assert_eq!(
+            home(Some("/Users/op".into())),
+            Ok(PathBuf::from("/Users/op"))
+        );
+    }
+
+    #[test]
+    fn a_switch_takes_eight_words_and_refuses_the_rest() {
+        assert_eq!(switch("K", None, true), Ok(true));
+        assert_eq!(switch("K", None, false), Ok(false));
+        for on in ["1", "true", "ON", " yes "] {
+            assert_eq!(switch("K", Some(on), false), Ok(true), "{on}");
+        }
+        for off in ["0", "False", "off", "NO", " 0"] {
+            assert_eq!(switch("K", Some(off), true), Ok(false), "{off}");
+        }
+        for odd in ["", "2", "nope", "o n"] {
+            let why = switch("K", Some(odd), true).unwrap_err();
+            assert!(
+                why.starts_with(&format!("K={odd:?} is not a switch")),
+                "{why}"
+            );
+        }
+    }
+}
diff --git a/crates/core/src/lib.rs b/crates/core/src/lib.rs
index 06d6c854..76bfe92f 100644
--- a/crates/core/src/lib.rs
+++ b/crates/core/src/lib.rs
@@ -16,6 +16,7 @@
 //! | [`error`] | every error this crate can return |
 //! | [`instrument`] | the one identity every vendor resolves to |
 //! | [`isin`] | the cross-check that is deliberately not part of identity |
+//! | [`knob`] | how an environment value naming a folder or a switch is read |
 //! | [`price`] | paisa integers and the one float boundary |
 //! | [`symbol`] | fixed-width symbols, one identity for every vendor |
 //! | [`universe`] | which lists a symbol belongs to |
@@ -27,6 +28,7 @@ pub mod blake3;
 pub mod error;
 pub mod instrument;
 pub mod isin;
+pub mod knob;
 pub mod price;
 pub mod symbol;
 pub mod universe;
diff --git a/crates/core/tests/lint.rs b/crates/core/tests/lint.rs
index 2f123dfa..a6048fdf 100644
--- a/crates/core/tests/lint.rs
+++ b/crates/core/tests/lint.rs
@@ -64,11 +64,12 @@ const PRICE: &str = include_str!("../src/price.rs");
 /// Listed by hand because `include_str!` takes a literal, and checked against
 /// `lib.rs` by [`the_module_list_is_the_whole_crate`] so the hand-written part
 /// cannot fall behind the crate.
-const OTHERS: [(&str, &str); 7] = [
+const OTHERS: [(&str, &str); 8] = [
     ("blake3.rs", include_str!("../src/blake3.rs")),
     ("error.rs", include_str!("../src/error.rs")),
     ("instrument.rs", include_str!("../src/instrument.rs")),
     ("isin.rs", include_str!("../src/isin.rs")),
+    ("knob.rs", include_str!("../src/knob.rs")),
     ("symbol.rs", include_str!("../src/symbol.rs")),
     ("universe.rs", include_str!("../src/universe.rs")),
     ("vendor.rs", include_str!("../src/vendor.rs")),
diff --git a/crates/costs/src/expiry.rs b/crates/costs/src/expiry.rs
index 60b0bac8..38195ae3 100644
--- a/crates/costs/src/expiry.rs
+++ b/crates/costs/src/expiry.rs
@@ -41,8 +41,10 @@
 //! contract settles on the previous trading day, and this module does not
 //! account for that — exactly as the source does not. It is stated here rather
 //! than left to be discovered: an expiry returned by this module is the
-//! *calendar* expiry, and a holiday calendar is a separate, unbuilt thing.
-//! `docs/06-limits.md` carries it.
+//! *calendar* expiry. The holiday calendar is `pull::calendar`, which this
+//! crate cannot name (`pull` depends on `costs`), so the caller that files
+//! contracts, `pull::rolling::expiry_of`, refuses an expiry it marks closed
+//! (CE-14, D-1769). `docs/06-limits.md` carries it.
 //!
 //! # Why the weekday regime is dated, and where it refuses
 //!
diff --git a/crates/pull/src/archive.rs b/crates/pull/src/archive.rs
index 7970dba5..88ee31ca 100644
--- a/crates/pull/src/archive.rs
+++ b/crates/pull/src/archive.rs
@@ -600,14 +600,21 @@ pub fn read_dir_reporting(
 /// forever, and BOTH feed folders on this operator's machine are symlinks, so
 /// that is a live hazard rather than a theoretical one.
 ///
-/// Four, measured. The deepest real member sits three below the feed folder —
-/// `gdfl/GFDLNFO_TICK_01072025/Futures/-I/AARTIIND-I.NFO.csv` — and four leaves
-/// exactly one level of headroom for a vendor that adds a wrapper. A fifth
+/// Five, measured. The deepest real member's folder sits three below the feed
+/// folder — `gdfl/GFDLNFO_TICK_01072025/Futures/-I/AARTIIND-I.NFO.csv`, whose
+/// `-I` is walked at depth 3 — and five leaves exactly one level of headroom
+/// for a vendor that adds a wrapper, which puts that folder at depth 4. A sixth
 /// would be room for a mistake rather than for a vendor.
 ///
+/// THIS WAS FOUR, AND FOUR WAS NO HEADROOM AT ALL. The check is
+/// `depth >= MAX_DEPTH`, so at four one wrapper folder put every Futures
+/// `-I/-II/-III` contract at the bound and skipped them all, leaving only a
+/// `skipped` count, while this sentence promised a level to spare (CE-21,
+/// D-1769). `a_gdfl_tree_inside_one_wrapper_folder_is_still_walked` pins it.
+///
 /// Deeper members are SKIPPED and counted, never silently dropped:
 /// [`Passed::skipped`] is on the walk's own log line.
-pub const MAX_DEPTH: usize = 4;
+pub const MAX_DEPTH: usize = 5;
 
 fn walk(
     dir: &Path,
diff --git a/crates/pull/src/chain.rs b/crates/pull/src/chain.rs
index 28a5ba8c..24d2dac5 100644
--- a/crates/pull/src/chain.rs
+++ b/crates/pull/src/chain.rs
@@ -102,6 +102,17 @@ pub struct Refusal {
     /// one bool; re-deriving it downstream from prose is guesswork that a
     /// reworded sentence silently breaks. D-0351.
     pub credential_dead: bool,
+    /// The refusal the vendor NAMED in its body, read through its own error
+    /// contract where the whole body was in hand.
+    ///
+    /// The rolling POST and the discovery GET returned a non-2xx refusal as
+    /// its status alone and never read the body, so Dhan's dead-token answer —
+    /// HTTP 400 carrying `DH-906` "Invalid Token" (D-0325) — was a plain
+    /// answered refusal and the token was sent to every remaining cell, while a
+    /// 403 "not entitled" was read as a dead token. The bars path already read
+    /// and classified the body; this carries the same verdict here (CE-29,
+    /// D-1769).
+    pub named: Option<crate::refusal::Disposition>,
 }
 
 impl Refusal {
@@ -116,6 +127,7 @@ impl Refusal {
             // network blip, which is the opposite of the defect it exists to
             // fix.
             credential_dead: false,
+            named: None,
         }
     }
 
@@ -126,6 +138,7 @@ impl Refusal {
             status: Some(status),
             detail,
             credential_dead: false,
+            named: None,
         }
     }
 
@@ -141,8 +154,16 @@ impl Refusal {
             status,
             detail,
             credential_dead: true,
+            named: None,
         }
     }
+
+    /// This refusal, carrying the disposition the vendor named in its body.
+    #[must_use]
+    pub const fn named_by_vendor(mut self, named: Option<crate::refusal::Disposition>) -> Self {
+        self.named = named;
+        self
+    }
 }
 
 impl core::fmt::Display for Refusal {
@@ -315,6 +336,17 @@ pub async fn month<D: Discovery>(feed: Feed, ask: &Ask, from: &D) -> Result<Chai
             month: ask.month,
             expiry,
         };
+        // DECODED BEFORE IT IS SENT. The vendor's expiry string went into the
+        // next request's URL and was validated only after that request had
+        // been made, so the "its contracts were not asked for" below was
+        // false and a malformed value reached the vendor (CE-15, D-1769).
+        let Some(keyed_expiry) = iso_expiry(&keyed.expiry) else {
+            chain.unreadable.push(format!(
+                "expiry {:?} (its contracts were not asked for)",
+                keyed.expiry
+            ));
+            continue;
+        };
         let url = fno::contracts_url(&spec, &keyed).map_err(ChainError::Lookup)?;
         let body = from.get(&url).await.map_err(|why| ChainError::Transport {
             url: url.clone(),
@@ -332,14 +364,7 @@ pub async fn month<D: Discovery>(feed: Feed, ask: &Ask, from: &D) -> Result<Chai
         // An expiry this build cannot decode takes its own contracts down and
         // says so by name, rather than the whole walk failing or the batch
         // vanishing: the vendor answered, and what could not be read is the
-        // thing to report.
-        let Some(keyed_expiry) = iso_expiry(&keyed.expiry) else {
-            chain.unreadable.push(format!(
-                "expiry {:?} (its contracts were not asked for)",
-                keyed.expiry
-            ));
-            continue;
-        };
+        // thing to report. The decode itself is above, before the request.
         let names = fno::names(&body, contracts_field).map_err(ChainError::Lookup)?;
         // A repeated name WITHIN THIS EXPIRY'S ANSWER is filed once: one
         // contract held as two inflates the count and builds its bars request
diff --git a/crates/pull/src/fno.rs b/crates/pull/src/fno.rs
index 93ec7e25..d3a4cb57 100644
--- a/crates/pull/src/fno.rs
+++ b/crates/pull/src/fno.rs
@@ -214,9 +214,10 @@ fn join(base: &str, path: &[PathSegment], params: &[Param], ask: &Ask) -> String
             // A value segment resolves through the same table the query does,
             // so a feed that puts its underlying in the path is one row rather
             // than a second builder.
-            PathSegment::Value { placeholder, value } => {
-                out.push_str(&resolve(value, ask).unwrap_or_else(|| placeholder.to_owned()));
-            }
+            PathSegment::Value { placeholder, value } => match resolve(value, ask) {
+                Some(value) => push_encoded(&mut out, &value),
+                None => out.push_str(placeholder),
+            },
         }
     }
     let mut first = true;
@@ -228,11 +229,37 @@ fn join(base: &str, path: &[PathSegment], params: &[Param], ask: &Ask) -> String
         first = false;
         out.push_str(p.name);
         out.push('=');
-        out.push_str(&value);
+        push_encoded(&mut out, &value);
     }
     out
 }
 
+/// `value`, percent-encoded so it is one query or path component whatever it
+/// holds.
+///
+/// The expiry in a contracts URL is the vendor's OWN string from its expiries
+/// answer, and it was appended raw, so a value carrying `&`, `=`, `#` or a
+/// space became a different request (CE-15, D-1769). Every byte outside RFC
+/// 3986's unreserved set is escaped; a value made only of those passes
+/// through unchanged, which is every value a live feed sends today.
+fn push_encoded(out: &mut String, value: &str) {
+    for byte in value.bytes() {
+        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
+            out.push(char::from(byte));
+        } else {
+            out.push('%');
+            // A nibble is below sixteen, so `from_digit` always answers.
+            for nibble in [byte >> 4, byte & 0x0f] {
+                out.push(
+                    char::from_digit(u32::from(nibble), 16)
+                        .unwrap_or('0')
+                        .to_ascii_uppercase(),
+                );
+            }
+        }
+    }
+}
+
 /// What one parameter is worth for this ask, or [`None`] where this ask does
 /// not carry it.
 fn resolve(value: ParamValue, ask: &Ask) -> Option<String> {
@@ -311,6 +338,24 @@ mod tests {
         );
     }
 
+    /// CE-15, D-1769: a value carrying URL syntax stays ONE component. The
+    /// vendor's own expiry string and an underlying such as `M&M` were
+    /// appended raw, so `&`, `=`, `#` and spaces became a different request.
+    #[test]
+    fn a_value_with_url_syntax_is_percent_encoded_into_one_component() {
+        let hostile = Ask {
+            underlying: "M&M".to_owned(),
+            expiry: "2025-01-25&x=1#f g".to_owned(),
+            year: 2025,
+            ..ask()
+        };
+        assert_eq!(
+            contracts_url(&groww(), &hostile).expect("it builds"),
+            "https://api.groww.in/v1/historical/contracts\
+             ?exchange=NSE&underlying_symbol=M%26M&expiry_date=2025-01-25%26x%3D1%23f%20g"
+        );
+    }
+
     /// THE CONTRACTS CALL IS FED BY THE EXPIRIES CALL, and says so.
     #[test]
     fn the_contracts_lookup_refuses_without_an_expiry_to_key_on() {
diff --git a/crates/pull/src/folder.rs b/crates/pull/src/folder.rs
index 52bd27b8..62942847 100644
--- a/crates/pull/src/folder.rs
+++ b/crates/pull/src/folder.rs
@@ -100,6 +100,10 @@ pub enum FolderError {
     /// `CLAUDE.md` §4 bans, so this arm refuses instead, and the deviation
     /// from the two siblings is named here rather than left to be discovered.
     NoHome,
+    /// [`ROOT_ENV`] or `HOME` is set but empty, or `HOME` is relative: either
+    /// would resolve against the working directory (CE-33, CE-38, D-1769). The
+    /// sentence is `brutex_core::knob`'s, naming the variable.
+    Unusable(String),
     /// The feed is REST. It has no folder at all.
     ///
     /// Refused rather than answered with a path, because a path invented for a
@@ -141,6 +145,7 @@ impl core::fmt::Display for FolderError {
                  is its folder, so a guess here would report a precise and \
                  wrong answer"
             ),
+            Self::Unusable(ref why) => f.write_str(why),
             Self::NotAFolderFeed { feed } => write!(
                 f,
                 "{feed} is a {}, so it has no folder — its bars are {} over the \
@@ -193,13 +198,16 @@ const HOME_ENV: &str = "HOME";
 ///
 /// [`FolderError::NoHome`] when both are absent.
 fn root_from(value: Option<OsString>, home: Option<OsString>) -> Result<PathBuf, FolderError> {
-    if let Some(named) = value {
-        return Ok(PathBuf::from(named));
+    if let Some(named) =
+        brutex_core::knob::folder(ROOT_ENV, value).map_err(FolderError::Unusable)?
+    {
+        return Ok(named);
     }
     let Some(home) = home else {
         return Err(FolderError::NoHome);
     };
-    Ok(PathBuf::from(home)
+    Ok(brutex_core::knob::home(Some(home))
+        .map_err(FolderError::Unusable)?
         .join(crate::config::CONFIG_DIR)
         .join(ROOT_DIR))
 }
@@ -620,6 +628,25 @@ mod tests {
         );
     }
 
+    /// CE-33 and CE-38, D-1769: an empty archive root and an empty or relative
+    /// HOME refuse by name instead of naming the working directory.
+    #[test]
+    fn an_empty_root_or_an_unusable_home_refuses_by_name() {
+        let why = root_from(Some("".into()), Some("/home/x".into())).expect_err("empty root");
+        assert!(
+            why.to_string()
+                .starts_with(&format!("{ROOT_ENV} is set but empty")),
+            "{why}"
+        );
+        let why = root_from(None, Some(" ".into())).expect_err("empty HOME");
+        assert!(
+            why.to_string().starts_with("HOME is set but empty"),
+            "{why}"
+        );
+        let why = root_from(None, Some("rel".into())).expect_err("relative HOME");
+        assert!(why.to_string().contains("relative"), "{why}");
+    }
+
     /// With neither set there is NO default folder, and the halt says so.
     #[test]
     fn with_no_variable_and_no_home_there_is_no_default_folder() {
diff --git a/crates/pull/src/http.rs b/crates/pull/src/http.rs
index 86b02d59..8b7e7161 100644
--- a/crates/pull/src/http.rs
+++ b/crates/pull/src/http.rs
@@ -2591,10 +2591,7 @@ impl HttpSource {
             //
             // CARRIED AS A NUMBER BESIDE THE SENTENCE, which is what makes the
             // rolling path's retry decidable at all.
-            return Err(Refusal::answered(
-                status.as_u16(),
-                format!("the vendor answered {status}"),
-            ));
+            return Err(self.refused_status(&mut answer).await);
         }
         // A FAILED BODY READ IS A TRANSPORT FAILURE, NOT A REFUSAL. The status
         // was already a success; what failed is the socket delivering the rest.
@@ -2620,6 +2617,46 @@ impl HttpSource {
         Ok(body)
     }
 
+    /// A non-2xx answer on the rolling POST or the discovery GET, as a
+    /// [`crate::chain::Refusal`] that carries what the BODY said.
+    ///
+    /// Both paths returned the status alone and never read the body (CE-29,
+    /// D-1769). Dhan answers a dead token with HTTP 400 and `DH-906` "Invalid
+    /// Token" in the body (D-0325), so without it the token was sent on to
+    /// every remaining cell instead of halting the feed, and a 403 "not
+    /// entitled" was read as a dead token. This reads the body under the same
+    /// bound and through the same classifier as `window_async`
+    /// ([`refusal_words`]), marks a named dead session as one, and records a
+    /// throttle the vendor named under a status other than 429 — the one the
+    /// status test above cannot see, so the shared governor learns it exactly
+    /// once.
+    async fn refused_status(&self, answer: &mut reqwest::Response) -> crate::chain::Refusal {
+        use crate::chain::Refusal;
+        use crate::refusal::Disposition;
+        let status = answer.status();
+        let (detail, named) = refusal_words(answer, self.spec.error_names).await;
+        if named == Some(Disposition::Throttled)
+            && status.as_u16() != 429
+            && let Some(lock) = self.governor.as_ref()
+        {
+            let mut g = lock
+                .lock()
+                .unwrap_or_else(std::sync::PoisonError::into_inner);
+            g.record_throttled();
+        }
+        let said = if detail.is_empty() {
+            format!("the vendor answered {status}")
+        } else {
+            format!("the vendor answered {status}: {detail}")
+        };
+        let refusal = if named == Some(Disposition::SessionDead) {
+            Refusal::credential(Some(status.as_u16()), said)
+        } else {
+            Refusal::answered(status.as_u16(), said)
+        };
+        refusal.named_by_vendor(named)
+    }
+
     /// The throttle half of the governor feedback, the only half that can be
     /// decided from the status alone.
     ///
@@ -2760,10 +2797,7 @@ impl crate::chain::Discovery for HttpSource {
             // this was retryable had to search prose for `429` — which is the
             // coupling `api::server::with_retry` refuses by name on the bars
             // path, and the reason discovery had no retry ladder at all.
-            return Err(Refusal::answered(
-                status.as_u16(),
-                format!("the vendor answered {status}"),
-            ));
+            return Err(self.refused_status(&mut answer).await);
         }
         // THE BODY READ IS A TRANSPORT FAILURE, NOT A REFUSAL. The status was
         // already a success; what failed is the socket delivering the rest of
@@ -5210,6 +5244,116 @@ mod tests {
         }
     }
 
+    /// **A REFUSING STATUS ON THE ROLLING POST AND ON A DISCOVERY GET IS READ
+    /// FOR ITS BODY.** CE-29, D-1769.
+    ///
+    /// Both paths returned a non-2xx refusal as its status alone, so Dhan's
+    /// dead-token answer — HTTP 400 carrying `DH-906` "Invalid Token" (D-0325)
+    /// — was an ordinary answered refusal and the token went on to every
+    /// remaining cell. Each row is driven over a real loopback socket: the
+    /// body's words reach the detail, the vendor's named disposition travels
+    /// with the refusal, a named dead session is marked as one, and a throttle
+    /// named under a 400 narrows the allowance by exactly one step.
+    #[test]
+    fn a_refusing_status_on_the_post_and_discovery_paths_is_read_for_its_body() {
+        use crate::refusal::Disposition;
+        let shipped = match crate::vendor::Feed::Dhan.descriptor().transport {
+            crate::vendor::Transport::Http(spec) => spec,
+            crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
+        };
+        let allowance = |source: &HttpSource| -> Option<u32> {
+            source.governor.as_ref().and_then(|lock| {
+                lock.lock()
+                    .unwrap_or_else(std::sync::PoisonError::into_inner)
+                    .permitted(crate::rate::WindowSpan::Second)
+            })
+        };
+        let once = {
+            let mut g = crate::rate::Governor::new(
+                shipped.budget.per_second,
+                shipped.budget.per_minute,
+                shipped.budget.per_day,
+            )
+            .expect("the shipped budget");
+            g.record_throttled();
+            g.permitted(crate::rate::WindowSpan::Second)
+        };
+        let full = shipped.budget.per_second;
+        let dead =
+            r#"{"errorType":"Order_Error","errorCode":"DH-906","errorMessage":"Invalid Token"}"#;
+        let throttled = r#"{"errorCode":"DH-904","errorMessage":"Too many requests"}"#;
+        let wrong = r#"{"errorCode":"DH-905","errorMessage":"Missing required fields"}"#;
+        // (status line, body, credential dead?, named, allowance after, a word the detail carries)
+        let rows = [
+            (
+                "400 Bad Request",
+                dead,
+                true,
+                Some(Disposition::SessionDead),
+                full,
+                "Invalid Token",
+            ),
+            (
+                "400 Bad Request",
+                throttled,
+                false,
+                Some(Disposition::Throttled),
+                once,
+                "DH-904",
+            ),
+            (
+                "400 Bad Request",
+                wrong,
+                false,
+                Some(Disposition::RequestWrong),
+                full,
+                "Missing required",
+            ),
+            (
+                "502 Bad Gateway",
+                "upstream down",
+                false,
+                None,
+                full,
+                "upstream down",
+            ),
+        ];
+        let runtime = tokio::runtime::Builder::new_current_thread()
+            .enable_all()
+            .build()
+            .expect("a runtime");
+        for post in [true, false] {
+            for (line, body, credential_dead, named, after, word) in rows {
+                let source = HttpSource::new(shipped, Credential::token("shhh".to_owned()))
+                    .expect("a client for the shipped Dhan row");
+                let (url, _seen, _) = listener(Some(format!(
+                    "HTTP/1.1 {line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
+                    body.len()
+                )));
+                let got = runtime.block_on(async {
+                    if post {
+                        source.post_json(&url, "{}".to_owned()).await
+                    } else {
+                        crate::chain::Discovery::get(&source, &url).await
+                    }
+                });
+                let path = if post { "post_json" } else { "Discovery::get" };
+                let refusal = got.expect_err(&format!("{path}: {line} is a refusal"));
+                assert_eq!(refusal.credential_dead, credential_dead, "{path}: {body}");
+                assert_eq!(refusal.named, named, "{path}: {body}");
+                assert!(refusal.detail.contains(word), "{path}: {}", refusal.detail);
+                assert!(
+                    refusal
+                        .detail
+                        .contains(line.split(' ').next().unwrap_or_default()),
+                    "{path}: the status is still named: {}",
+                    refusal.detail
+                );
+                assert_eq!(allowance(&source), after, "{path}: {body}");
+            }
+        }
+    }
+
     /// **A 429 ON THE ROLLING POST AND ON A DISCOVERY GET STILL NARROWS THE
     /// ALLOWANCE, AND NO OTHER REFUSING STATUS MOVES IT.** W1-pull2-11, D-0950.
     ///
diff --git a/crates/pull/src/rate.rs b/crates/pull/src/rate.rs
index 0dddd51d..abc3db67 100644
--- a/crates/pull/src/rate.rs
+++ b/crates/pull/src/rate.rs
@@ -729,7 +729,8 @@ impl Window {
     /// real reduction in a few refusals; a day quota erodes gently and stays
     /// usable.
     ///
-    /// **The floor is one, never zero.** An allowance of zero is an absorbing
+    /// **The floor is one permit a second, never zero** ([`Self::floor_of`],
+    /// D-1769; it was one permit per span). An allowance of zero is an absorbing
     /// state — no request is ever admitted, so no success is ever observed, so
     /// nothing ever raises it again. A governor that can reach it is a governor
     /// that can permanently stop a pull on one bad minute, and the recovery
@@ -751,9 +752,48 @@ impl Window {
         self.permitted = self
             .permitted
             .saturating_sub(Self::step_of(self.ceiling))
-            .max(1);
+            .max(Self::floor_of(self.span, self.ceiling));
         self.credit = 0;
     }
+
+    /// The lowest allowance a refusal may step this span down to: one permit
+    /// per second across the span, or the ceiling where that is smaller, and
+    /// never below one.
+    ///
+    /// # THE FLOOR WAS ONE FOR EVERY SPAN, AND ON A DAY SPAN ONE IS A DAY'S SLEEP
+    ///
+    /// Each refusal steps every span down by `ceiling / BACKOFF_STEPS`, so the
+    /// 100,000/day window reached ONE after exactly 32 refusals and the next
+    /// permit was a day away — measured: a wait of 86,390 s, and a second
+    /// queued one of 172,790 s — while recovery adds a step back per
+    /// [`SUCCESSES_PER_STEP`] clean answers that a one-a-day allowance cannot
+    /// earn. The feed stalled until the process restarted, behind a sleep no
+    /// caller bounded (CE-28, D-1769).
+    ///
+    /// A refusal is evidence the arrival RATE is too high, and the slowest rate
+    /// any vendor here publishes is per second. So every span floors at the
+    /// same rate — one permit per second — expressed in its own length: 1 for a
+    /// second, 60 for a minute, 86,400 for a day. The second span's floor is
+    /// unchanged; a long span can no longer be stepped below the rate the
+    /// shortest one already floors at, which is the only thing it was ever
+    /// doing past that point.
+    ///
+    /// # A span published slower than one per second keeps its old floor
+    ///
+    /// Flooring it at one per second would sit ABOVE its own ceiling, and a
+    /// refusal would then narrow nothing. Such a span floors at one step,
+    /// `ceiling / BACKOFF_STEPS` and never below one, as before. No live feed
+    /// publishes one — every ceiling in `crate::vendor` is faster than a permit
+    /// a second, pinned by
+    /// `every_live_rate_span_floors_at_one_permit_per_second` — so that branch
+    /// guards a descriptor nobody has written yet. Named rather than hidden:
+    /// `docs/06-limits.md` records it.
+    fn floor_of(span: WindowSpan, ceiling: u32) -> u32 {
+        match u32::try_from(span.len_micros() / MICROS_PER_SECOND) {
+            Ok(per_second) if per_second < ceiling => per_second,
+            _ => Self::step_of(ceiling),
+        }
+    }
 }
 
 /// One vendor's governor for one request kind.
diff --git a/crates/pull/src/rolling.rs b/crates/pull/src/rolling.rs
index e542d803..e507ef97 100644
--- a/crates/pull/src/rolling.rs
+++ b/crates/pull/src/rolling.rs
@@ -173,6 +173,25 @@ pub fn expiry_of(
     let settled = found.ok_or(RollingError::NoExpiry {
         why: "no expiry was reached",
     })?;
+    // A CLOSED DAY IS NOT AN EXPIRY. `costs::expiry` returns the plain
+    // calendar weekday and leaves holidays to its caller, and this caller never
+    // asked, so a holiday week's contract got a closed day as its expiry, was
+    // filed under that key, and priced at a tenor ~4.8x too long (CE-14,
+    // D-1769). The exchange's holiday-shift rule is not recorded in
+    // `docs/00-charter.md`, so the closed day is REFUSED rather than stepped
+    // back (`CLAUDE.md` §3 rule 1). A day past the calendar's last measured
+    // day cannot be checked and is passed through; `docs/06-limits.md` names
+    // that limit.
+    if matches!(
+        crate::calendar::kind_of(i64::from(settled.ordinal())),
+        crate::calendar::DayKind::Closed
+    ) {
+        return Err(RollingError::NoExpiry {
+            why: "the computed expiry falls on a day the exchange calendar marks closed, and the \
+                  rule that moves an expiry off a holiday is not charter-sourced, so no date is \
+                  guessed",
+        });
+    }
     brutex_core::instrument::Expiry::new(settled.year(), settled.month(), settled.day()).map_err(
         |_| RollingError::NoExpiry {
             why: "the calendar produced a date this store cannot name",
@@ -272,6 +291,17 @@ pub enum RollingError {
         /// The cell exactly as the vendor wrote it.
         text: String,
     },
+    /// A decimal cell that is present and cannot be read exactly — not text
+    /// or a number, or a form the six-place shift does not read (an exponent,
+    /// an overflow, junk). It was stored as the null sentinel, which is the
+    /// record that the vendor SENT NONE, and pricing then solved its own value
+    /// and labelled it so (CE-16, D-1769).
+    Undecimal {
+        /// Which field.
+        field: &'static str,
+        /// The cell exactly as the vendor wrote it.
+        text: String,
+    },
     /// A price cell that is negative, or is not zero and snaps to zero.
     /// GAP16-23, D-1492.
     NotAPrice {
@@ -332,6 +362,12 @@ impl core::fmt::Display for RollingError {
             Self::Unrepresentable { field } => {
                 write!(f, "a `{field}` value does not fit paisa as an i64")
             }
+            Self::Undecimal { field, text } => write!(
+                f,
+                "a `{field}` cell holds {text}, which is not a decimal this build \
+                 reads exactly. Refused rather than stored as the absence the \
+                 vendor did not state"
+            ),
             Self::Uncountable { field, text } => write!(
                 f,
                 "a `{field}` cell holds {text}, which is not a non-negative whole \
@@ -630,7 +666,10 @@ pub fn read(
             // IV IN MILLIONTHS, and the multiply happens on the way in so the
             // store never holds a float. See `Overlay`'s header for why a
             // volatility is stored as an integer despite being a statistic.
-            iv_micros: iv.map_or(OI_NULL, |a| a.get(at).map_or(OI_NULL, micros_of)),
+            iv_micros: match iv.and_then(|a| a.get(at)) {
+                None | Some(serde_json::Value::Null) => OI_NULL,
+                Some(cell) => micros_of(cell, f.implied_volatility)?,
+            },
         };
         // THE STRIKE FOR THIS STAMP. A rolling series is ATM-relative, so the
         // resolved strike genuinely can differ between two bars of one answer
@@ -838,13 +877,23 @@ fn paisa(
 /// # Cost
 ///
 /// One pass over at most a few dozen characters. No allocation.
-fn micros_of(cell: &serde_json::Value) -> i64 {
+///
+/// # Errors
+///
+/// [`RollingError::Undecimal`] for a present cell that is not text or a
+/// number, or that `shift_six` cannot read. A JSON `null` never reaches here:
+/// the caller files it as absent, which is what the vendor said.
+fn micros_of(cell: &serde_json::Value, field: &'static str) -> Result<i64, RollingError> {
+    let refuse = || RollingError::Undecimal {
+        field,
+        text: cell.to_string(),
+    };
     let text = match cell {
         serde_json::Value::String(text) => text.clone(),
         serde_json::Value::Number(n) => n.to_string(),
-        _ => return OI_NULL,
+        _ => return Err(refuse()),
     };
-    shift_six(text.trim()).unwrap_or(OI_NULL)
+    shift_six(text.trim()).ok_or_else(refuse)
 }
 
 /// A decimal string as millionths, half-up, or `None` when it will not read.
@@ -1078,6 +1127,39 @@ mod tests {
         );
     }
 
+    /// CE-16, D-1769: an implied-volatility cell that is present and cannot
+    /// be read refuses the answer by name; only a `null` or a missing cell is
+    /// filed as "the vendor sent none".
+    #[test]
+    fn an_unreadable_volatility_cell_is_refused_not_filed_as_absent() {
+        let with = |iv: &str| {
+            format!(
+                r#"{{"data":{{"ce":{{
+                "timestamp":[1700000000],
+                "open":[100.0],"high":[100.0],"low":[100.0],"close":[100.0],
+                "volume":[1],"iv":[{iv}]
+            }}}}}}"#
+            )
+        };
+        for bad in [
+            r#""junk""#,
+            "1e-7",
+            "true",
+            r#"{"v":1}"#,
+            "99999999999999999999",
+        ] {
+            let got = read(&with(bad), &spec(), "CALL", PriceScale::Rupees);
+            assert!(
+                matches!(got, Err(RollingError::Undecimal { field: "iv", .. })),
+                "{bad}: {got:?}"
+            );
+        }
+        let rows = read(&with("null"), &spec(), "CALL", PriceScale::Rupees).expect("null reads");
+        assert_eq!(rows[0].overlay.iv_micros, OI_NULL);
+        let rows = read(&with("0.125"), &spec(), "CALL", PriceScale::Rupees).expect("reads");
+        assert_eq!(rows[0].overlay.iv_micros, 125_000);
+    }
+
     /// **IV AND SPOT LAND IN THE OVERLAY, KEYED BY THE BAR'S OWN STAMP.**
     ///
     /// The stamp is the join. Position would be faster and wrong the first time
@@ -1241,6 +1323,39 @@ mod tests {
         );
     }
 
+    /// CE-14, D-1769: a computed expiry the exchange calendar marks CLOSED is
+    /// refused, not filed. NIFTY's weekly from 2024-08-14 lands on 2024-08-15
+    /// (Independence Day) and its monthly from 2023-03-29 on 2023-03-30 (Ram
+    /// Navami); both are `Closed` in `pull::calendar`.
+    #[test]
+    fn a_computed_expiry_on_a_closed_day_is_refused() {
+        use crate::session::Day;
+        for (on, flag) in [
+            (Day::new(2024, 8, 14).expect("a real day"), "WEEK"),
+            (Day::new(2023, 3, 29).expect("a real day"), "MONTH"),
+        ] {
+            let got = expiry_of("NIFTY", &spec(), flag, "1", on);
+            assert!(
+                matches!(
+                    got,
+                    Err(RollingError::NoExpiry { why }) if why.contains("marks closed")
+                ),
+                "{on:?} {flag}: {got:?}"
+            );
+        }
+        // An ordinary week still resolves.
+        assert!(
+            expiry_of(
+                "NIFTY",
+                &spec(),
+                "WEEK",
+                "1",
+                Day::new(2024, 8, 21).expect("a day")
+            )
+            .is_ok()
+        );
+    }
+
     /// **AN UNKNOWN UNDERLYING, CADENCE OR ORDINAL IS REFUSED, NEVER GUESSED.**
     ///
     /// Guessing a date for a contract the exchange never listed files bars
diff --git a/crates/pull/src/ssm.rs b/crates/pull/src/ssm.rs
index d710f9d6..fe70efa0 100644
--- a/crates/pull/src/ssm.rs
+++ b/crates/pull/src/ssm.rs
@@ -331,12 +331,12 @@ impl AwsIdentity {
                 "HOME is unset, so ~/.aws/credentials cannot be located".to_owned(),
             ));
         };
-        Self::from_credentials_file(
-            &std::path::PathBuf::from(home)
-                .join(".aws")
-                .join("credentials"),
-            profile,
-        )
+        // AN EMPTY OR RELATIVE HOME IS REFUSED, not read as the working
+        // directory's `.aws/credentials` (CE-38, D-1769).
+        let home = brutex_core::knob::home(Some(home)).map_err(|why| {
+            SsmError::unreachable(format!("{why}, so ~/.aws/credentials cannot be located"))
+        })?;
+        Self::from_credentials_file(&home.join(".aws").join("credentials"), profile)
     }
 
     /// One profile out of a credentials file **the caller names**.
diff --git a/crates/pull/tests/folder.rs b/crates/pull/tests/folder.rs
index fa717272..e93824b3 100644
--- a/crates/pull/tests/folder.rs
+++ b/crates/pull/tests/folder.rs
@@ -902,6 +902,33 @@ fn the_walk_descends_into_group_folders_rather_than_skipping_them() {
     assert_eq!(earliest, day(2022, 9, 30));
 }
 
+/// CE-21, D-1769: a member whose folder is exactly one level inside the bound
+/// is walked — the GDFL shape `GFDLNFO_.../Futures/-I/` inside one wrapper
+/// folder, which the old bound of four skipped.
+#[test]
+fn a_gdfl_tree_inside_one_wrapper_folder_is_still_walked() {
+    let scratch = Scratch::new();
+    let root = scratch.dir("one-wrapper");
+    let mut at = root.clone();
+    for level in 1..archive::MAX_DEPTH {
+        at = at.join(format!("l{level}"));
+    }
+    assert_eq!(
+        archive::MAX_DEPTH - 1,
+        4,
+        "wrapper / GFDLNFO_... / Futures / -I is four folders below the root"
+    );
+    fs::create_dir_all(&at).expect("a wrapped tree");
+    fs::write(at.join("AT_LIMIT.csv"), TWO_DAYS).expect("a member at the limit");
+
+    let census = folder::read_census(&root, Feed::TrueData, Columns::TrueDataIndex)
+        .expect("the walk reads it");
+    assert!(
+        !census.instruments.is_empty(),
+        "a member one level inside the bound is walked"
+    );
+}
+
 /// THE DESCENT IS BOUNDED, and a member past the bound is COUNTED not dropped.
 ///
 /// Both feed folders on the operator's machine are SYMLINKS, so an unbounded
diff --git a/crates/pull/tests/unit.rs b/crates/pull/tests/unit.rs
index d2ef0dd6..444f1f98 100644
--- a/crates/pull/tests/unit.rs
+++ b/crates/pull/tests/unit.rs
@@ -2819,6 +2819,100 @@ fn a_refusal_steps_down_every_allowance_and_drains_every_bucket() {
     );
 }
 
+/// CE-28 — refusals floor every span at one permit a second, never at one per
+/// span.
+///
+/// Each refusal steps every span down by `ceiling / BACKOFF_STEPS`, and the
+/// floor was one for all of them, so Dhan's 100,000/day window reached ONE
+/// after exactly 32 refusals and the next permit was a day away: measured at
+/// 86,390 s, and 172,790 s for the next one queued. A day of sleep behind no
+/// bound and no refusal. The floor is now the rate the second span already
+/// floors at, in each span's own length (D-1769).
+#[test]
+fn ce28_a_day_span_never_floors_below_one_permit_a_second() {
+    let mut governor =
+        Governor::new(Some(DHAN_PER_SECOND), None, Some(DHAN_PER_DAY)).expect("within bounds");
+    for _ in 0..1_000 {
+        governor.record_throttled();
+    }
+    assert_eq!(governor.permitted(WindowSpan::Second), Some(1));
+    assert_eq!(governor.permitted(WindowSpan::Day), Some(86_400));
+    // The drained buckets clear in one second, not one day.
+    match governor.admit(0) {
+        Verdict::Deny { wait_micros, .. } => assert!(
+            wait_micros <= MICROS_PER_SECOND,
+            "a floored governor waits at most a second, waited {wait_micros} µs"
+        ),
+        Verdict::Admit => panic!("a drained bucket admits nothing at once"),
+    }
+    // A minute span floors at sixty, the same rate.
+    let mut minute = only(WindowSpan::Minute, 500);
+    for _ in 0..1_000 {
+        minute.record_throttled();
+    }
+    assert_eq!(minute.permitted(WindowSpan::Minute), Some(60));
+}
+
+/// Every span every live feed publishes is faster than a permit a second, so
+/// every one floors at exactly that rate and no refusal can make a permit wait
+/// longer than a second on any span (CE-28, D-1769).
+#[test]
+fn every_live_rate_span_floors_at_one_permit_per_second() {
+    let mut seen = 0;
+    for feed in pull::vendor::Feed::ALL {
+        let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
+            continue;
+        };
+        let mut governor = Governor::new(
+            spec.budget.per_second,
+            spec.budget.per_minute,
+            spec.budget.per_day,
+        )
+        .expect("a live budget is within bounds");
+        for _ in 0..10_000 {
+            governor.record_throttled();
+        }
+        for span in WindowSpan::ALL {
+            let Some(left) = governor.permitted(span) else {
+                continue;
+            };
+            seen += 1;
+            assert_eq!(
+                u64::from(left),
+                span.len_micros() / MICROS_PER_SECOND,
+                "{feed:?} {span} floored at {left}"
+            );
+        }
+    }
+    assert!(seen >= 4, "only {seen} live spans were checked");
+}
+
+/// A span published SLOWER than a permit a second keeps a floor of one step,
+/// because one a second would sit above its own ceiling and a refusal would
+/// narrow nothing (D-1769).
+#[test]
+fn a_span_published_slower_than_one_a_second_floors_at_one_step() {
+    for (span, ceiling, floor) in [
+        (WindowSpan::Minute, 30, 1),
+        (WindowSpan::Minute, 60, 1),
+        (WindowSpan::Minute, 61, 60),
+        (WindowSpan::Day, 3_200, 100),
+        (WindowSpan::Day, 86_400, 2_700),
+        (WindowSpan::Day, 86_401, 86_400),
+    ] {
+        let mut governor = only(span, ceiling);
+        governor.record_throttled();
+        assert!(
+            governor.permitted(span) < Some(ceiling),
+            "{span} {ceiling} must narrow"
+        );
+        for _ in 0..10_000 {
+            governor.record_throttled();
+        }
+        assert_eq!(governor.permitted(span), Some(floor), "{span} {ceiling}");
+    }
+}
+
 /// **A REFUSED DAY SPAN COSTS LITTLE AND RECOVERS IN A BOUNDED NUMBER OF
 /// REQUESTS**, and under halve-down / `+1`-up it did neither.
 ///
diff --git a/crates/runner/src/excursion.rs b/crates/runner/src/excursion.rs
index b80774f0..e9b00366 100644
--- a/crates/runner/src/excursion.rs
+++ b/crates/runner/src/excursion.rs
@@ -752,6 +752,28 @@ impl Crossings {
         self.peak_ppm_at(offset, entry, side, true)
     }
 
+    /// [`Self::adverse_ppm_at`] rounded UP, for the figure a maximum gates.
+    ///
+    /// `worst_mae` is admitted by `<= max_mae_ppm`, and a floored `10000.495`
+    /// ppm read as `10000` and passed a `10000` cap (p3floor-2, D-1769). The
+    /// floored reading still drives rung crossing and every trade row, so this
+    /// is a second reading rather than a change to [`ppm_of`].
+    #[must_use]
+    pub fn adverse_ppm_ceil_at(&self, offset: usize, entry: i64, side: Side) -> Ppm {
+        let Some(last) = self.low_run.len().checked_sub(1) else {
+            return 0;
+        };
+        let at = offset.min(last);
+        let (Some(&low), Some(&high)) = (self.low_run.get(at), self.high_run.get(at)) else {
+            return 0;
+        };
+        let move_paisa = match side {
+            Side::Long => entry.saturating_sub(low),
+            Side::Short => high.saturating_sub(entry),
+        };
+        ppm_ceil_of(move_paisa, entry)
+    }
+
     /// The best the path had gone FOR `entry` by offset `offset`, in ppm.
     ///
     /// The mirror of [`Self::adverse_ppm_at`]; every word there applies, with
@@ -1467,6 +1489,22 @@ fn ppm_of(move_paisa: i64, entry: i64) -> Ppm {
     i64::try_from(scaled).unwrap_or(i64::MAX)
 }
 
+/// [`ppm_of`] rounded up: the smallest whole ppm not below the exact move.
+fn ppm_ceil_of(move_paisa: i64, entry: i64) -> Ppm {
+    if move_paisa <= 0 || entry <= 0 {
+        return 0;
+    }
+    let numerator = i128::from(move_paisa).saturating_mul(i128::from(PPM_ONE));
+    let denominator = i128::from(entry);
+    let floor = numerator / denominator;
+    let scaled = if numerator % denominator == 0 {
+        floor
+    } else {
+        floor.saturating_add(1)
+    };
+    i64::try_from(scaled).unwrap_or(i64::MAX)
+}
+
 #[cfg(test)]
 #[allow(
     clippy::expect_used,
@@ -1688,6 +1726,38 @@ mod tests {
         assert_eq!(c.target_at(0), NEVER, "the path never went favourable");
     }
 
+    /// p3floor-2, D-1769: the reading a maximum gates rounds UP, and the
+    /// crossing reading beside it still floors.
+    #[test]
+    fn the_gated_adverse_reading_rounds_up_and_the_crossing_reading_floors() {
+        let entry = 199_999_i64;
+        let bars = vec![bar(0, entry - 2_000, entry + 2_000)];
+        let rungs = ladder(&[5_000]);
+        let ladders = Ladders {
+            stops: &rungs,
+            targets: &rungs,
+            trails: &rungs,
+        };
+        for side in [Side::Long, Side::Short] {
+            let c = crossings(&bars, 0, 0, entry, side, ladders);
+            // 2,000 * 1,000,000 / 199,999 = 10,000.05 ppm.
+            assert_eq!(c.adverse_ppm_at(0, entry, side), 10_000, "{side:?}");
+            assert_eq!(c.adverse_ppm_ceil_at(0, entry, side), 10_001, "{side:?}");
+        }
+        let exact = 200_000_i64;
+        let c = crossings(&bars, 0, 0, exact, Side::Long, ladders);
+        let floor = c.adverse_ppm_at(0, exact, Side::Long);
+        assert_eq!(
+            c.adverse_ppm_ceil_at(0, exact, Side::Long),
+            floor,
+            "exact stays exact"
+        );
+        assert_eq!(super::ppm_ceil_of(0, exact), 0);
+        assert_eq!(super::ppm_ceil_of(5, 0), 0);
+        let empty = crossings(&[], 0, 0, entry, Side::Long, ladders);
+        assert_eq!(empty.adverse_ppm_ceil_at(0, entry, Side::Long), 0);
+    }
+
     #[test]
     fn a_short_reads_the_high_as_adverse_and_the_low_as_favourable() {
         // The mirror of the case above. Getting this backwards would make every
diff --git a/crates/runner/src/grid.rs b/crates/runner/src/grid.rs
index 1d672626..70684363 100644
--- a/crates/runner/src/grid.rs
+++ b/crates/runner/src/grid.rs
@@ -562,6 +562,21 @@ impl Cell {
         self.gross_loss.saturating_div(losers.cast_signed())
     }
 
+    /// The MAGNITUDE of the mean loss, rounded UP, for a field a maximum gates.
+    ///
+    /// [`Self::avg_loss`] truncates toward zero, so `-301` over two losers is
+    /// `-150` and a `150` cap admitted a true mean of `150.5`. Admission
+    /// evidence reads this one instead (p3floor-1, D-1769); the display keeps
+    /// the truncated figure.
+    #[must_use]
+    pub const fn avg_loss_magnitude_ceil(&self) -> u64 {
+        let losers = self.trades.saturating_sub(self.wins);
+        if losers == 0 {
+            return 0;
+        }
+        self.gross_loss.unsigned_abs().div_ceil(losers)
+    }
+
     /// Mean holding time, in execution bars — minutes under the 1-minute layer.
     #[must_use]
     pub const fn avg_bars_held(&self) -> u64 {
@@ -4450,8 +4465,12 @@ fn one_variant(
         if let Some(rows) = trades.as_deref_mut() {
             rows.push(row_of(bars, c, exit, pess, opt, went_against, went_for));
         }
-        if went_against > cell.worst_mae {
-            cell.worst_mae = went_against;
+        // `worst_mae` is gated by a maximum, so it takes the reading rounded UP
+        // (p3floor-2, D-1769); `went_against` stays the floored figure the
+        // trade rows and the all-trades sum have always carried.
+        let worst_of_this = c.cross.adverse_ppm_ceil_at(pess_off, c.entry_pess, side);
+        if worst_of_this > cell.worst_mae {
+            cell.worst_mae = worst_of_this;
         }
 
         if pess > 0 {
@@ -6150,6 +6169,33 @@ mod exit_family_tests {
 mod tests {
     use super::{Cell, Grid, Levels, Streaks, evaluate, evaluate_over, tally_trade};
 
+    /// p3floor-1, D-1769: the gated mean-loss magnitude rounds UP.
+    #[test]
+    fn the_gated_mean_loss_magnitude_rounds_up() {
+        let cell = Cell {
+            trades: 2,
+            wins: 0,
+            gross_loss: -301,
+            ..Cell::default()
+        };
+        assert_eq!(cell.avg_loss(), -150, "the display figure still truncates");
+        assert_eq!(cell.avg_loss_magnitude_ceil(), 151);
+        let even = Cell {
+            trades: 3,
+            wins: 1,
+            gross_loss: -300,
+            ..Cell::default()
+        };
+        assert_eq!(even.avg_loss_magnitude_ceil(), 150);
+        let none = Cell {
+            trades: 2,
+            wins: 2,
+            gross_loss: 0,
+            ..Cell::default()
+        };
+        assert_eq!(none.avg_loss_magnitude_ceil(), 0);
+    }
+
     /// BOTH RUNS ARE MEASURED, AND EACH ONE ENDS THE OTHER.
     ///
     /// `max_losing_streak` shipped alone. A reader shown only the bad run learns
diff --git a/docs/04-invariants.md b/docs/04-invariants.md
index 5c6a2f77..8f01aea5 100644
--- a/docs/04-invariants.md
+++ b/docs/04-invariants.md
@@ -3039,7 +3039,7 @@ a downloader.
 | MR-17 | **The refresh is reachable without reading the route table.** `GET /masters` carries one row per source — asserted against `SOURCES` rather than against `4`, so a fifth master cannot be added and silently left off — and `("/masters", "Masters", true)` puts it in the nav beside every other page. D-0308 shipped the two JSON routes with neither | `api::mastersrun::the_page_carries_a_row_for_every_source_and_names_what_each_costs` · `the_page_is_reachable_from_the_navigation` | ✓ |
 | MR-18 | **Opening the page fetches nothing.** The load path stats four files; the refresh is bound to a press and to nothing else. Both halves are asserted — that the listener exists, and that `refresh();` appears nowhere in the script — because a page that quietly spends a vendor request on load is the operator's one standing prohibition | `api::mastersrun::the_page_carries_one_external_script_and_no_inline_handler` (the HTML half), and CI's `web` job over `web/masters.js` (the binding half). **The single test this row used to name has never existed and could not**: gate 1e runs the whole workspace with `web/` DETACHED to prove no crate depends on the front end, so a Rust test that opened `masters.js` would go red in the one job whose green is that guarantee. The proof is split because the subject is | ✓ |
 | MR-19 | **The credentialed source is visibly distinguished from the free ones, and a primed source says so.** Pressing a control that spends a shared token must not look like pressing one that fetches a public CDN file; and an unexplained extra request in the ledger reads as a bug, so the row that causes it names the priming URL | `api::mastersrun::the_page_separates_the_free_sources_from_the_one_that_spends_a_token` · `the_page_says_a_prime_happens_where_one_does` | ✓ |
-| MR-20 | **New bytes on disk do not silently change what the server answers, and the page says so.** `Site::load` parses the masters once at startup with no reload path. The row reports `newer_than_parse` and the post-refresh line asks for a restart; `status.json` carries `modified_unix_millis` so "present" can be told from "present and eleven months old", `null` rather than a zero that renders as 1970 | `api::mastersrun::the_page_says_a_restart_is_required_rather_than_pretending_otherwise` · `the_status_answer_carries_when_each_master_was_written` | ✓ |
+| MR-20 | **New bytes on disk do not silently change what the server answers, and the page says so.** `Site::load` parses the masters at startup and a refresh re-parses them in place through `Site::reparse`, answering `restart_required: false`; a restart is named only when that reparse is refused (`reloaded: false`, reason named) or a master changed by another hand, which the row reports as `newer_than_parse` (P3-01-05, D-1769); `status.json` carries `modified_unix_millis` so "present" can be told from "present and eleven months old", `null` rather than a zero that renders as 1970 | `api::mastersrun::the_page_says_a_refresh_reloads_and_names_when_a_restart_is_required` · `the_status_answer_carries_when_each_master_was_written` | ✓ |
 | MR-21 | **Every step of a refresh is durable, not only the response body.** One event per network round trip and one per source outcome, so an operator asking "why did this fail an hour ago" can search it. Before this, the whole route wrote three counts and the attempt ledger died with the browser tab | `api::emitted::every_reachable_emit_site_puts_a_record_in_the_file`, rows `api.masters.attempt refused settled` and `api.masters.source could not be refreshed` | ✓ |
 | MR-22 | **The log level follows the outcome, at both granularities.** A refresh where all four sources refused used to emit `Info: the masters were refreshed` with `landed=0` — a success-shaped line over a total failure. A settled refusal is `Error`, a retryable one is `Warn`, a landing is `Info`; a skipped source is `Warn` because the guard working is neither | the two census rows above pin the `Error` arms; `crates/api/src/mastersrun.rs`'s `record` and the summary emit carry the rest | ✓ |
 | MR-23 | **The join every master exists for is reachable in one press, and names the symbols it could not resolve.** `/indexmap.json` reported them and had no page and no nav entry. A symbol the exchange does not confirm is one whose bars are filed under a name nothing corroborates, so it is shown rather than tallied | `api::mastersrun::the_page_cross_verifies_every_feed_against_the_exchange` · `an_unconfirmed_symbol_is_never_filtered_out_of_the_view` · `the_cross_verification_is_also_bound_to_a_press` | ✓ |
@@ -6308,6 +6308,31 @@ old line regex the same input and watched it pass.
 | ZR-29 | A stop after the autopilot task returned keeps the halt, and a resume stays refused (D-1767) | `api::autopilot::tests::a_stop_after_the_task_returned_keeps_the_halt_and_resume_stays_refused` | ✓ |
 | ZR-30 | `/masters` shows every server string as text; nothing the server sends reaches `innerHTML` (D-1768) | `web/tests/masters-text.test.js` · *a host-quoted refusal is shown as text and never parsed as markup* | ✓ |
 | ZR-31 | A `/masters` refresh the server did not reload is reported with its status and reason, never as success (D-1768) | `web/tests/masters-text.test.js` · *a host-quoted refusal is shown as text and never parsed as markup* · *a refresh that reloaded says so* | ✓ |
+| ZR-32 | Every reason code `crates/api/src/bars.rs` can send for a withheld change has a sentence on `/db` (D-1769) | `web/tests/bar-why.test.js` · *every withheld-change code bars.rs can send has a sentence on /db* | ✓ |
+| ZR-33 | A folder walk holding more than the cap is offered capped and states the cap; an exact-cap walk states none (D-1769) | `api::render::tests::a_capped_folder_walk_is_stated_and_an_exact_one_is_not` | ✓ |
+| ZR-34 | No refusal steps any rate span below one permit a second while its ceiling is faster than that, so no permit waits longer than a second (D-1769) | `pull::unit::ce28_a_day_span_never_floors_below_one_permit_a_second` · `every_live_rate_span_floors_at_one_permit_per_second` · `a_span_published_slower_than_one_a_second_floors_at_one_step` | ✓ |
+| ZR-35 | A non-2xx answer on the rolling POST and the discovery GET carries its body's words and the vendor's named disposition, and a named dead session is marked dead (D-1769) | `pull::http::tests::a_refusing_status_on_the_post_and_discovery_paths_is_read_for_its_body` | ✓ |
+| ZR-36 | One named throttle is one decrement: `with_retry` records only what the transport could not (D-1769) | `api::server::throttle_record_tests::a_throttle_the_transport_recorded_is_not_recorded_again` | ✓ |
+| ZR-37 | Pool pass 1 lists the smallest drawdown first, ties in screened order, and every refusal last (D-1769) | `cli::pool::tests::pass_one_puts_refusals_last_and_keeps_ties_in_screened_order` | ✓ |
+| ZR-38 | `/live.json` never calls a `\|t\|` below the Bonferroni bar a clearance (D-1769) | `api::livejson::tests::a_t_at_the_rounded_bar_does_not_clear_it_and_one_above_does` | ✓ |
+| ZR-39 | The first signal day's daily context is anchored to the session it follows on the canonical calendar, or the run refuses (D-1769) | `cli::stored::tests::a_first_signal_day_whose_prior_session_has_no_daily_record_refuses` | ✓ |
+| ZR-40 | Global Replay V4 holds at most `VIX_MONTHS_HELD` VIX months, the newest ones (D-1769) | `cli::global_replay_v4::tests::the_vix_month_catalogue_drops_the_oldest_month_at_its_cap` | ✓ |
+| ZR-41 | The return-over-drawdown cell prints `-` only for a variant with no profit; a profit under 0.01x its drawdown prints `<0.01` (D-1769) | `cli::tests::the_ledger_ratio_is_the_cell_ratio` | ✓ |
+| ZR-42 | The startup CSV-folder walk does not follow a directory link below its root (D-1769) | `api::render::tests::a_linked_directory_is_not_walked` | ✓ |
+| ZR-43 | A search checkpoint folder passes over OS litter files and refuses any other stray entry by name (D-1769) | `cli::search_checkpoint::tests::os_litter_is_passed_over_and_a_stranger_is_named` | ✓ |
+| ZR-44 | A folder variable set but empty, and an empty or relative `HOME`, are refused by name and never resolved against the working directory; a switch takes eight words and refuses the rest (D-1769) | `brutex_core::knob::tests::an_empty_or_blank_folder_is_refused_by_name_and_unset_is_none` · `home_must_be_set_non_empty_and_absolute` · `a_switch_takes_eight_words_and_refuses_the_rest` | ✓ |
+| ZR-45 | Every reader of a store, masters, archive, log or web folder, of `HOME`, and of `BRUTEX_VALIDATE` goes through that reader (D-1769) | `api::server::tests::an_empty_folder_or_a_relative_home_is_refused_not_resolved` · `api::server::tests::the_log_directory_prefers_the_environment_and_falls_back_to_the_store` · `api::assets::tests::the_directory_is_the_environment_or_web_beside_the_workspace` · `pull::folder::tests::an_empty_root_or_an_unusable_home_refuses_by_name` · `cli::tests::the_log_directory_follows_the_store_unless_it_is_named_outright` · `cli::tests::an_unvalidated_screen_cannot_be_mistaken_for_a_validated_one` | ✓ |
+| ZR-46 | A hard-linked execution lock is refused with its path, its link count and the remedy (D-1769) | `cli::execution_lease::tests::missing_store_and_nonregular_or_changed_lock_paths_refuse_without_repair` | ✓ |
+| ZR-47 | After a failed roll `/logs` says rotation stopped and a restart resumes it, never that events were overwritten (D-1769) | `api::logs::tests::a_failed_roll_is_described_as_stopped_rotation_not_overwrite` | ✓ |
+| ZR-48 | The archive walk reaches a GDFL `-I` folder inside one wrapper folder (D-1769) | `pull::folder::a_gdfl_tree_inside_one_wrapper_folder_is_still_walked` · `a_member_nested_past_the_depth_bound_is_not_walked` | ✓ |
+| ZR-49 | Every resolved value in a discovery URL is one percent-encoded component, and an undecodable vendor expiry is refused before its contracts are asked for (D-1769) | `pull::fno::tests::a_value_with_url_syntax_is_percent_encoded_into_one_component` | ✓ |
+| ZR-50 | A rolling-option expiry the exchange calendar marks closed is refused, never filed (D-1769) | `pull::rolling::tests::a_computed_expiry_on_a_closed_day_is_refused` | ✓ |
+| ZR-51 | A present implied-volatility cell this build cannot read refuses the answer; only `null` or a missing cell is filed as absent (D-1769) | `pull::rolling::tests::an_unreadable_volatility_cell_is_refused_not_filed_as_absent` | ✓ |
+| ZR-52 | The help text names `boolean-qualified-search-stored`'s optional TIMEFRAMES, has no `%%` escape, and names the cash equities as underlyings (D-1769) | `cli::tests::the_usage_text_states_what_the_commands_actually_take` | ✓ |
+| ZR-53 | The mean-loss magnitude a maximum gates is rounded up; the display mean still truncates (D-1769) | `runner::grid::tests::the_gated_mean_loss_magnitude_rounds_up` | ✓ |
+| ZR-54 | `worst_mae`, gated by a maximum, is the adverse excursion rounded up to the next whole ppm; the crossing reading still floors (D-1769) | `runner::excursion::tests::the_gated_adverse_reading_rounds_up_and_the_crossing_reading_floors` | ✓ |
+| ZR-55 | The backtest page names a stored series by its whole symbol, hyphens included, and a contract row names no swept symbol (D-1769) | `web/tests/instrument.test.js` "a swept symbol keeps its own hyphen and a contract row names no swept symbol" | ✓ |
+| ZR-56 | `/pull/run` and `/pull/recovery` read every run `legs_from` accepts, and one leg past `MAX_RUN_LEGS` is refused by name (D-1769) | `api::pullrun::tests::a_run_past_the_leg_bound_is_refused_by_name_and_the_widest_leg_fits` · `http_admission_tests` body-limit test | ✓ |
 
 ### Documents state what the code writes — zero-findings round (D-1940 onward)
 
diff --git a/docs/05-decisions.md b/docs/05-decisions.md
index a768a3c4..d43354d6 100644
--- a/docs/05-decisions.md
+++ b/docs/05-decisions.md
@@ -55034,3 +55034,139 @@ measured on.
   said "All four are on disk", reading a `restart_required` the route always
   sends false. It now reports the status and the reason, and success only when
   the server reloaded (CE-26).
+
+### D-1769 — /db explains every withheld-change code, and a capped folder list says so — 2026-10-03
+
+- `crates/api/src/bars.rs` withholds a bar's change with `previous_unreadable`
+  (the bar before failed its checksum) or `overflow` (the change does not fit
+  the server's integer range). `/db`'s `BAR_WHY` table had no sentence for
+  either, so a real, explained withholding was shown as "unknown … a defect
+  here". Both now have one, and `web/tests/bar-why.test.js` reads every code
+  `bars.rs` can send and fails when the page cannot explain one (CE-27).
+- `collect_csv_dirs` stopped at exactly `MAX_FOLDER_SUGGESTIONS` and
+  `folder_input` states the cap only when it holds more than that, so the
+  "capped at 60" notice could never print and a capped list read as complete.
+  The walk now collects one past the cap, which is never offered; it only lets
+  the page tell a full list from a capped one (CE-32).
+- Rate floor (CE-28). Every refusal stepped every span down by
+  `ceiling / BACKOFF_STEPS` to a floor of ONE, so 32 refusals left Dhan's
+  100,000/day window at one permit and the next request slept 86,390 s behind
+  no bound. Each span now floors at one permit a second in its own length (1,
+  60, 86,400), the rate the second span already floored at. A span published
+  slower than one a second floors at one step instead; no live feed has one,
+  and `docs/06-limits.md` names that branch.
+- Dead token on the rolling and discovery paths (CE-29). `post_json` and
+  `Discovery::get` returned a non-2xx refusal as its status alone. They now read
+  the body under the bars path's bound and classifier (`refusal_words`), mark a
+  named dead session as one, carry the vendor's disposition on
+  `chain::Refusal::named`, and `laddered` hands it to `step`. A 400 `DH-906`
+  "Invalid Token" now halts the feed, and a named not-entitled 403 is no longer
+  read as a dead token.
+- Double throttle (CE-30). `with_retry` recorded a throttle named in a 2xx body
+  that `weigh_body_parsed` had already recorded. It now records only a named
+  throttle under a non-2xx, non-429 status, the one set the transport cannot
+  see (`transport_missed_throttle`). The rolling and discovery transport now
+  records that set itself, so `laddered`'s "the transport already took the
+  decrease" is true there too.
+- Pool pass 1 order (CE-4). `render_per_symbol` sorted ascending and reversed
+  the whole list, which put every refusal on top against its own comment and
+  listed ties in reverse input order. One ascending sort with each direction in
+  the key now puts the smallest drawdown first, ties in screened order and
+  refusals last.
+- Live Bonferroni bar (CE-7). `bar_milli` truncated the bar and the page
+  compared a ROUNDED `t_milli` with `>=`, so a |t| up to 1.5 milli below the
+  bar was shown as clearing it. The bar is now the ceiling (a non-finite bar is
+  one no row clears) and `/live.json` compares strictly above it, which cannot
+  call a |t| below the bar a clearance.
+- First signal day's daily anchor (CE-10). `daily_context_from_span` asked only
+  for ANY eligible daily record before the first signal day, and its per-day
+  walk covers a day only once a previous signal day exists, so a previous-month
+  file that ended early anchored the first day's pivots, previous-day and gap
+  bits to an older session while GapFib anchored to the right one. The newest
+  eligible record before the first signal day must now be
+  `prior_accepted_session(first_signal_day)`, or the run refuses by name.
+- Global Replay V4 VIX months (CE-8). `VixCatalog` kept every opened VIX month,
+  about 2.86 MB each, for the whole replay with no bound. It now holds at most
+  `VIX_MONTHS_HELD` (4) and drops the earliest held month before opening a new
+  one; a month asked for again is re-read from the same file, so every stamp is
+  unchanged.
+- Return over drawdown below a hundredth (CE-17). The range-all table printed
+  `-` ("no profit") whenever the integer ratio was 0, which is also what a
+  profitable variant whose return is under 0.01x its drawdown truncates to. The
+  dash is now decided from `pessimistic <= 0`, and a truncated profit prints
+  `<0.01`.
+- Startup folder walk follows links (CE-35). `collect_csv_dirs` used `is_dir`,
+  which follows symlinks, so `~/Downloads/loop -> ~/Downloads` re-walked the
+  tree at every level to depth six. A linked directory below the root is now
+  skipped via `DirEntry::file_type`; the root itself may still be a link.
+- Search checkpoint litter (CE-34). One `.DS_Store` in a search checkpoint
+  folder refused every start, resume and dashboard read with a sentence that
+  named no file. Finder's `.DS_Store` and AppleDouble `._` files are passed
+  over when they are plain files, and any other stray entry refuses with its
+  name and folder.
+- Empty or loosely-read configuration values (CE-5, CE-6, CE-33, CE-36, CE-37,
+  CE-38, CE-39). Every folder variable was read as `PathBuf::from(value)` and
+  only an UNSET one was refused, so a variable set but EMPTY resolved against
+  the working directory: `BRUTEX_LOG_DIR`, `BRUTEX_LOGS`, `BRUTEX_STORE`,
+  `BRUTEX_MASTERS`, `BRUTEX_ARCHIVES` and `BRUTEX_WEB`, and an empty `HOME`
+  under every `$HOME/...` default including the §8 credential configuration and
+  `~/.aws/credentials`. Two switches turned off only for a literal `0`. One
+  reader now lives in `brutex_core::knob` (`core` depends on nothing, so every
+  crate can share it): `folder` refuses an empty or blank value by the
+  variable's name, `home` also refuses a relative `HOME`, and `switch` takes
+  `0/false/off/no` and `1/true/on/yes` and refuses any other word. An unusable
+  `BRUTEX_VALIDATE` keeps validation ON and is named in the KNOB REFUSED block;
+  an unusable `BRUTEX_ARCHIVE_SUGGESTIONS` turns the walk off and the page says
+  why; the api refuses to serve on an empty `BRUTEX_LOGS` or `BRUTEX_WEB`.
+- Unnamed hard-link refusals (CE-40). An extra hard link to the store's
+  execution lock (a `cp -al` snapshot, an `ln`) refused every sweep for good
+  with a sentence naming no file. That refusal and the same one in Selection
+  V6, Global Replay V4 and the checksum receipt now name the path, the link
+  count and what to remove.
+- The `/logs` banner after a failed roll (CE-41). The sink deliberately stops
+  rotating for the life of the process after a failed roll, because a
+  re-attempted roll empties the retained history one file per event
+  (`Sink::rotation_broken`). The banner said the oldest events "may already
+  have been overwritten", which that design guarantees did not happen after the
+  failure, and never said a restart resumes rotation. It now says rotation has
+  stopped, the file is growing, the failed roll may have removed the oldest
+  file, and a restart after fixing the named cause resumes rotation. Retrying
+  in-process is not added: a retry that fails half-way is the
+  history-emptying defect D-1324 removed.
+- Archive walk depth (CE-21). `archive::MAX_DEPTH` was 4 under a doc promising
+  one level of headroom, but the check `depth >= MAX_DEPTH` put the deepest
+  real GDFL folder (`-I`, at depth 3) at the limit as soon as one wrapper
+  folder was added, so every Futures contract was skipped with only a count.
+  The bound is 5, the doc states the depth of each level, and a test walks a
+  member exactly one level inside it.
+- CE-22 needed no change here: `limit=0` on `/operation-audit.json` was already
+  refused with 400 by D-1762's canonical-integer parser, and
+  `queries_reject_aliases_duplicates_overflow_and_mixed_exact_pages` pins it.
+- Discovery URL values (CE-15). The vendor's own expiry string went into the
+  next contracts URL unescaped and was decoded only after that request was
+  sent, while the refusal for an undecodable one said "its contracts were not
+  asked for". The expiry is now decoded before the request, and every resolved
+  path and query value is percent-encoded outside RFC 3986's unreserved set, so
+  an underlying such as `M&M` is one component too.
+- Rolling-option expiry on a closed day (CE-14). `pull::rolling::expiry_of`
+  took `costs::expiry`'s calendar weekday and never asked `pull::calendar`, so
+  a holiday week's contract (NIFTY weekly from 2024-08-14 landed on the
+  2024-08-15 holiday) was filed under a closed day and priced at a tenor about
+  4.8 times too long. A computed expiry the calendar marks `Closed` is now
+  refused by name. It is not stepped back to the previous trading day, because
+  that rule is not in `docs/00-charter.md`. Expiries after the calendar's last
+  measured day (2026-09-04) pass through unchecked, named in
+  `docs/06-limits.md`, whose sentence "there is no holiday calendar in this
+  repository" was stale and is corrected along with `costs::expiry`'s.
+- Unreadable vendor IV (CE-16). `rolling::micros_of` turned any present cell it
+  could not read (an exponent form, an overflow, junk, a non-number) into the
+  null sentinel, which the append-only overlay records as "the vendor sent no
+  IV", and pricing then solved its own. A present unreadable cell now refuses
+  the answer with `RollingError::Undecimal` naming the field and the cell; only
+  `null` or a missing cell is filed as absent.
+- P3-02-03/04/05: `USAGE` names the optional `[TIMEFRAMES]` argument of `boolean-qualified-search-stored`, prints `80%` rather than the printf escape `80%%` a Rust string never needed, and the `UNDERLYING` line names the F&O cash equities CLAUDE.md §1 admits. Pinned by `the_usage_text_states_what_the_commands_actually_take`.
+- p3floor-1: `average_loss_paisa` is gated by `<= max_average_loss_paisa` and was a floored mean, so a true 150.5 passed a 150 cap. The four evidence builders now round it UP (`div_ceil`, and `runner::grid::Cell::avg_loss_magnitude_ceil` for the two that read a cell). The display `avg_loss` still truncates; `average_win`, gated by a minimum, keeps its floor, which is already the safe direction.
+- p3floor-2: `worst_mae` is gated by `<= max_mae_ppm` and was the floored ppm, so 10,000.495 ppm passed a 10,000 cap. It now takes `Crossings::adverse_ppm_ceil_at`, a second reading rounded up; `ppm_of` is unchanged because rung crossing, the trade rows and the all-trades sum still read the floored figure.
+- P3-01-05: docs/04 row MR-20 said the masters had "no reload path" and cited a test renamed by D-1762. It now states the in-place reparse and cites `the_page_says_a_refresh_reloads_and_names_when_a_restart_is_required`.
+- P3-02-02: the backtest page named a stored instrument by its last `-` segment, so the sweepable shares `BAJAJ-AUTO` and `NAM-INDIA` were offered, run and printed in the `cli range-all` hint as `AUTO` and `INDIA`, which the engine refuses. Both joins now read `instrument.js`'s `sweptSymbolOf`, which takes everything after the exchange and segment through `parseKey` and answers `null` for a contract row so a future never lends its months to the spot series.
+- P3-01-01: `/pull/run` and `/pull/recovery` read legs that repeat their member list twice-encoded, under the shared 8 KiB bound, so a modest press answered a framework 413 in plain text and the page showed a `SyntaxError`. Both routes now read `pullrun::MAX_RUN_FORM_BYTES`, sized from `MAX_RUN_LEGS` and the member form the inner route admits, and a press past `MAX_RUN_LEGS` is refused by name. The worst-case size is named in docs/06-limits.md.
diff --git a/docs/06-limits.md b/docs/06-limits.md
index 4fe8b6ce..eb02c7b4 100644
--- a/docs/06-limits.md
+++ b/docs/06-limits.md
@@ -1432,10 +1432,19 @@ end date would be as much a fabrication as inventing a rate.
 An expiry returned by `expiry::next_weekly_expiry` or
 `expiry::next_monthly_expiry` is the **calendar** expiry. When an expiry day
 falls on an NSE trading holiday the contract settles on the previous trading
-day, and neither this crate nor the source knows which days those are. There is
-no holiday calendar in this repository. **Every expiry this crate returns can be
-one or more days late on a holiday week**, and a caller that needs the settled
-date must apply a holiday calendar it obtains elsewhere.
+day, and this crate does not know which days those are. **Every expiry this
+crate returns can be one or more days late on a holiday week**, and a caller
+that needs the settled date must apply a holiday calendar.
+
+**This said there is no holiday calendar in this repository, and there is one:**
+`pull::calendar::kind_of` marks every day from 2019-12-02 to 2026-09-04 open or
+closed, measured from stored daily bars (D-1769, CE-14). `costs` cannot use it
+(`pull` depends on `costs`, not the reverse), so the one caller that turns these
+into contract keys, `pull::rolling::expiry_of`, now REFUSES a computed expiry
+the calendar marks closed. It does not step back to the previous trading day,
+because that rule is not recorded in `docs/00-charter.md`. An expiry after
+2026-09-04 is outside the calendar's measured range and cannot be checked; it
+is passed through unchanged until the calendar is extended.
 
 **How often that bites is UNVERIFIED and is not estimated here.** It depends on
 the NSE holiday calendar, which this repository does not hold and which
@@ -14171,3 +14180,25 @@ and H the committed blocks in `global-selection-v6.bin` (at most
 
 None of these is O(1), and none grows with the request alone. They are not
 reduced here (D-1642).
+
+## A rate span published slower than one permit a second keeps the old floor — D-1769, 3 October 2026
+
+`pull::rate::Window::floor_of` floors every span at one permit a second in its
+own length, so no refusal can make a live feed's permit wait longer than a
+second (CE-28). A span whose PUBLISHED ceiling is already slower than one a
+second cannot take that floor, since it would sit above its own ceiling and a
+refusal would narrow nothing. That span floors at one step,
+`ceiling / BACKOFF_STEPS` and at least one, and a long such span (a day quota
+below 86,400) can still be stepped to a wait of hours. No live feed publishes
+one: `every_live_rate_span_floors_at_one_permit_per_second` pins every shipped
+descriptor. A feed added with one inherits this limit until a bound on the wait
+itself exists.
+
+## `/pull/run` and `/pull/recovery` read up to about 26.5 MB of form — D-1769, 3 October 2026
+
+`MAX_RUN_FORM_BYTES` is sized so every run `pullrun::legs_from` accepts is read:
+`MAX_RUN_LEGS` (feeds × 3 rungs × 2 routes = 30) legs, each a member form the
+inner route admits, percent-encoded twice more. That is the worst case, held in
+memory once per request; a real press is a few kilobytes per leg. One leg past
+the bound is refused by name (`Refusal::TooManyLegs`) rather than read further.
+The figure is arithmetic from the constants, not a measurement.
diff --git a/web/src/lib/instrument.js b/web/src/lib/instrument.js
index 779de468..c6aa547d 100644
--- a/web/src/lib/instrument.js
+++ b/web/src/lib/instrument.js
@@ -213,3 +213,20 @@ export function strikeExact(paisa) {
   const body = paise === 0 ? String(rupees) : `${rupees}.${String(paise).padStart(2, '0')}`;
   return neg ? `-${body}` : body;
 }
+
+/**
+ * The symbol a sweep is asked for, from a census instrument name.
+ *
+ * `NSE-CASH-BAJAJ-AUTO` is `BAJAJ-AUTO`, not `AUTO`: a symbol may hold a `-`,
+ * so the name is everything after the exchange and segment, read by
+ * {@link parseKey}. Only a spot series is swept; a future or option row
+ * answers `null` rather than lending its months to the underlying's spot
+ * series (P3-02-02, D-1769).
+ *
+ * @param {unknown} full
+ * @returns {string|null}
+ */
+export function sweptSymbolOf(full) {
+  const p = parseKey(full);
+  return p.kind === 'spot' && p.underlying ? p.underlying : null;
+}
diff --git a/web/src/routes/backtest/+page.svelte b/web/src/routes/backtest/+page.svelte
index ae3dac66..87814eab 100644
--- a/web/src/routes/backtest/+page.svelte
+++ b/web/src/routes/backtest/+page.svelte
@@ -82,6 +82,7 @@
   import { page as routePage } from '$app/state';
   import { feeds, selectFeed } from '$lib/feeds.svelte.js';
   import { readStoreCensus } from '$lib/store.svelte.js';
+  import { sweptSymbolOf } from '$lib/instrument.js';
   /* RENAMED ON IMPORT. This page's Run control owns a state object called
      `ask` — what the operator is asking the sweep for — and the fetch helper
      is a different thing entirely. One name for one value. */
@@ -3329,10 +3330,11 @@
       for (const row of rows ?? []) {
         const full = String(row.instrument ?? '');
         // The census names an instrument `NSE-INDEX-NIFTY`; the ledger and
-        // the run route both name it `NIFTY`. Matching on the LAST segment
-        // is what joins them without this page holding an exchange or a
-        // segment literal — the same join `loadRungs` already makes.
-        const leaf = full.split('-').pop() ?? '';
+        // the run route both name it `NIFTY`. The symbol is everything after
+        // the exchange and segment, NOT the last `-` segment: `BAJAJ-AUTO`
+        // and `NAM-INDIA` are sweepable shares and were offered as `AUTO`
+        // and `INDIA`, a name the engine refuses (P3-02-02, D-1769).
+        const leaf = sweptSymbolOf(full) ?? '';
         const month = String(row.month ?? '');
         const rung = String(row.timeframe ?? '');
         if (!leaf || !month || !rung) continue;
@@ -4777,9 +4779,8 @@
       const months = new Map();
       for (const row of rows ?? []) {
         // The census names an instrument `NSE-INDEX-NIFTY`; the ledger names
-        // it `NIFTY`. Matching on the LAST segment is what joins them without
-        // this page holding an exchange or a segment literal.
-        const leaf = String(row.instrument ?? '').split('-').pop();
+        // it `NIFTY`. The symbol may itself hold a `-` (P3-02-02, D-1769).
+        const leaf = sweptSymbolOf(String(row.instrument ?? ''));
         if (leaf !== run.underlying) continue;
         months.set(row.timeframe, (months.get(row.timeframe) ?? 0) + 1);
       }
diff --git a/web/src/routes/db/+page.svelte b/web/src/routes/db/+page.svelte
index 262508e8..0068a438 100644
--- a/web/src/routes/db/+page.svelte
+++ b/web/src/routes/db/+page.svelte
@@ -4882,7 +4882,11 @@
       'open interest on this bar is the store’s null sentinel, i64::MIN - this feed stamps none for this segment. It is NOT zero; a zero here would be a real zero.',
     oi_null_before:
       'the previous bar carries no open interest, so there is nothing to measure this one against.',
-    previous_oi_zero: 'the previous open interest is zero, and a ratio against zero is not a number.'
+    previous_oi_zero: 'the previous open interest is zero, and a ratio against zero is not a number.',
+    previous_unreadable:
+      'the bar before this one failed its checksum and was not read, so there is no trusted value to measure this one against. Its own value is shown; only the change is withheld.',
+    overflow:
+      'the change does not fit the integer range the server measures in, so it is withheld rather than shown wrapped. Both values are real.'
   };
   /** @param {string | null} code */
   const barWhyText = (code) =>
diff --git a/web/tests/bar-why.test.js b/web/tests/bar-why.test.js
new file mode 100644
index 00000000..6c1f5c5c
--- /dev/null
+++ b/web/tests/bar-why.test.js
@@ -0,0 +1,24 @@
+// EVERY REASON CODE THE BAR WINDOW CAN SEND HAS A SENTENCE ON /db.
+//
+// `crates/api/src/bars.rs` withholds a change with one of a small set of codes,
+// and `/db` turns each into a sentence. `previous_unreadable` and `overflow`
+// had none, so a real, explained withholding was shown as "unknown … a defect
+// here" (CE-27, D-1769). This reads both files and fails when the server can
+// send a code the page cannot explain.
+
+import { test } from 'node:test';
+import assert from 'node:assert/strict';
+import { readFileSync } from 'node:fs';
+
+const BARS = readFileSync(new URL('../../crates/api/src/bars.rs', import.meta.url), 'utf8');
+const PAGE = readFileSync(new URL('../src/routes/db/+page.svelte', import.meta.url), 'utf8');
+
+test('every withheld-change code bars.rs can send has a sentence on /db', () => {
+  const code = BARS.split('\n#[cfg(test)]')[0];
+  const sent = new Set([...code.matchAll(/\(None, "([a-z_]+)"\)/g)].map((m) => m[1]));
+  assert.ok(sent.size >= 7, `found only ${[...sent].join(', ')}`);
+  const table = PAGE.split('const BAR_WHY = {')[1]?.split('\n  };')[0] ?? '';
+  const explained = new Set([...table.matchAll(/^\s{4}([a-z_]+):/gm)].map((m) => m[1]));
+  const missing = [...sent].filter((c) => !explained.has(c));
+  assert.deepEqual(missing, [], `codes /db would call a defect: ${missing.join(', ')}`);
+});
diff --git a/web/tests/instrument.test.js b/web/tests/instrument.test.js
index 383cad27..eb03b673 100644
--- a/web/tests/instrument.test.js
+++ b/web/tests/instrument.test.js
@@ -17,7 +17,8 @@ import {
   segmentOf,
   strikeExact,
   FUTURE_TAIL,
-  OPTION_TAIL
+  OPTION_TAIL,
+  sweptSymbolOf
 } from '../src/lib/instrument.js';
 
 /**
@@ -176,3 +177,14 @@ test('the tail patterns are anchored, so a symbol cannot be mistaken for one', (
   assert.equal(parseKey('NSE-CASH-FUT').underlying, 'FUT', 'a symbol called FUT is a symbol');
   assert.equal(parseKey('NSE-CASH-FUT').kind, 'spot');
 });
+
+test('a swept symbol keeps its own hyphen and a contract row names no swept symbol', () => {
+  // P3-02-02, D-1769: the backtest page took the LAST `-` segment.
+  assert.equal(sweptSymbolOf('NSE-CASH-BAJAJ-AUTO'), 'BAJAJ-AUTO');
+  assert.equal(sweptSymbolOf('NSE-CASH-NAM-INDIA'), 'NAM-INDIA');
+  assert.equal(sweptSymbolOf('NSE-INDEX-NIFTY'), 'NIFTY');
+  assert.equal(sweptSymbolOf('NSE-FNO-NIFTY-2026-07-28-FUT'), null);
+  assert.equal(sweptSymbolOf('NSE-FNO-NIFTY-2026-07-28-2450000-CE'), null);
+  assert.equal(sweptSymbolOf(''), null);
+  assert.equal(sweptSymbolOf(undefined), null);
+});
````

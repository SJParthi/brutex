```diff
diff --git a/crates/api/src/sweeprun.rs b/crates/api/src/sweeprun.rs
index 1bf6c90e..eee40297 100644
--- a/crates/api/src/sweeprun.rs
+++ b/crates/api/src/sweeprun.rs
@@ -7296,6 +7296,72 @@ mod tests {
         }
     }
 
+    /// D-4770. A DESCENT FLOOR OUTSIDE `cli`'s ONE SUPPORT DOMAIN IS REFUSED
+    /// HERE BY THAT DOMAIN'S SENTENCE, as at the argv `elite`.
+    ///
+    /// The descent route takes no support: the walk derives its floor, the
+    /// round trips the stated win rate needs over the rows the column swept.
+    /// At 5,400 bp the warmed May's 1,500 swept one-minute rows cannot hold
+    /// them, so the floor passes 1,000,000 ppm. The walk took it as its one
+    /// step and answered NOTHING PASSED; the route now relays `cli`'s refusal,
+    /// which the browser shows as the run's refusal. In a child process, for
+    /// the reason `a_zero_point_screen_command_runs_as_no_ceiling_end_to_end`
+    /// gives.
+    #[test]
+    fn a_descent_floor_outside_the_support_domain_is_refused_by_its_sentence() {
+        const CHILD: &str = "BRUTEX_TEST_API_DESCENT_FLOOR_DOMAIN";
+        if std::env::var_os(CHILD).is_some() {
+            descent_floor_domain_child();
+            return;
+        }
+        let root = crate::scratch::path("descent-floor-domain");
+        let _ = std::fs::remove_dir_all(&root);
+        std::fs::create_dir_all(&root).expect("the child's store root");
+        write_warmed_nifty_may(&root);
+        let logs = root.join("logs");
+        let out = crate::isolated::rerun(
+            "sweeprun::tests::a_descent_floor_outside_the_support_domain_is_refused_by_its_sentence",
+            &[
+                (CHILD, std::ffi::OsStr::new("1")),
+                ("BRUTEX_STORE", root.as_os_str()),
+                ("BRUTEX_LOG_DIR", logs.as_os_str()),
+                ("BRUTEX_VALIDATE", std::ffi::OsStr::new("0")),
+                ("BRUTEX_MIN_WIN_RATE_BP", std::ffi::OsStr::new("5400")),
+            ],
+        );
+        assert!(out.contains("DESCENT FLOOR REFUSED BY THE DOMAIN"), "{out}");
+        let _ = std::fs::remove_dir_all(&root);
+    }
+
+    /// The child half: one descent over the stored May 2025.
+    fn descent_floor_domain_child() {
+        let asked = descent_from(
+            r#"{"feed":"zerodha","underlying":"NIFTY","from_year":2025,"from_month":5,"to_year":2025,"to_month":5,"rung":"1min","max_points":0,"top":1}"#,
+        )
+        .expect("a well-formed descent");
+        let progress = conduct_descent(&asked, 1, 2);
+        let refusal = progress.refusal.as_deref().unwrap_or("");
+        // The domain's own sentence for any support at or above a million,
+        // which the floor here is: `cli::support_ppm_in_domain`.
+        let domain =
+            cli::support_ppm_in_domain(1_000_000).expect_err("a million is outside the domain");
+        assert!(
+            refusal.contains(domain) && refusal.contains("support floor is "),
+            "report: {:?} refusal: {refusal}",
+            progress.report
+        );
+        assert!(
+            !progress
+                .report
+                .as_deref()
+                .unwrap_or("")
+                .contains("NOTHING PASSED"),
+            "{:?}",
+            progress.report
+        );
+        println!("DESCENT FLOOR REFUSED BY THE DOMAIN");
+    }
+
     /// W2-cli8-11, D-4718. THE `screen` COMMAND'S SUPPORT DOMAIN IS `cli`'s.
     ///
     /// This door refused only 0, so 1,000,000 ppm (100%) or more ran and
diff --git a/crates/cli/src/audited_stored_tests.rs b/crates/cli/src/audited_stored_tests.rs
index ad7eb924..0ef0be8d 100644
--- a/crates/cli/src/audited_stored_tests.rs
+++ b/crates/cli/src/audited_stored_tests.rs
@@ -3997,3 +3997,183 @@ fn a_column_builds_retry_loop_reads_the_daily_context_once() {
         )
     );
 }
+
+/// D-4769. THE ARGV `screen` READS A ZERO STOP CEILING AS NO CEILING, as
+/// `elite`, `screen_range_in_points` (D-4717) and both api doors (D-1732) do.
+///
+/// `screen_arm` refused `MAX_POINTS <= 0` at its own door with "1 or more",
+/// so the one verb whose stop the operator types positionally was the one
+/// that could not ask for the derived ladder alone. Over a SWEPT index, so
+/// the zero reaches the code under test: a negative ceiling is still refused
+/// by the shared sentence before anything is read, and zero runs.
+#[test]
+fn the_argv_screen_reads_a_zero_point_ceiling_as_no_ceiling() {
+    const CHILD: &str = "BRUTEX_TEST_ARGV_ZERO_POINT_SCREEN";
+    if std::env::var_os(CHILD).is_some() {
+        argv_zero_point_screen_child();
+        return;
+    }
+    let fixture = Fixture::warmed();
+    rerun_over_store(
+        "audited_stored::tests::the_argv_screen_reads_a_zero_point_ceiling_as_no_ceiling",
+        CHILD,
+        &fixture.root,
+        "ARGV ZERO-POINT SCREEN RAN",
+    );
+}
+
+/// The child half, over the warmed NIFTY store `BRUTEX_STORE` names.
+fn argv_zero_point_screen_child() {
+    let _knobs = crate::knobs::serially();
+    crate::knobs::clear_all();
+    crate::knobs::set("BRUTEX_VALIDATE", "0");
+    crate::knobs::set("BRUTEX_CEILING", "256");
+    // THE PREMISE, CHECKED BEFORE THE WORK (G18-cli-a-36, D-2018).
+    assert!(
+        !crate::validate_from_env(),
+        "BRUTEX_VALIDATE=0 turns validation off"
+    );
+    let root = crate::store_root().expect("the fixture store");
+    let words = |points: &str| -> Vec<String> {
+        [
+            "screen", "zerodha", "NIFTY", "1min", "2025", "5", "2025", "5", "999999", points, "0",
+            "1",
+        ]
+        .iter()
+        .map(ToString::to_string)
+        .collect()
+    };
+    let recorded = || {
+        crate::results::Results::open_read(&root)
+            .map_or(0, |mut ledger| ledger.len().expect("a readable ledger"))
+    };
+
+    let mut report = String::new();
+    let status = crate::dispatch(&words("0"), &mut report);
+    assert!(
+        !report.contains("MAX_POINTS must be a whole number of index points, 1 or more"),
+        "zero was refused: {report}"
+    );
+    assert!(
+        !report.contains("admits nothing"),
+        "zero was read as a ceiling: {report}"
+    );
+    if crate::commit_stamp().is_none() {
+        assert_eq!(status, crate::FAILED, "{report}");
+        assert!(report.contains("no verified commit stamp"), "{report}");
+    } else {
+        assert_eq!(status, crate::OK, "{report}");
+        assert!(report.starts_with(crate::STORED_PROVENANCE), "{report}");
+        assert!(report.contains("RESULT RECORDED"), "{report}");
+    }
+
+    let before = recorded();
+    for (points, why) in [
+        ("-1", "A negative ceiling is unsatisfiable"),
+        ("twenty", "0 for no ceiling beyond the ladder"),
+    ] {
+        let mut refused = String::new();
+        assert_eq!(
+            crate::dispatch(&words(points), &mut refused),
+            crate::MISUSED,
+            "{refused}"
+        );
+        assert!(
+            refused.starts_with("refused: MAX_POINTS is a whole number of index points")
+                && refused.contains(why),
+            "{points}: {refused}"
+        );
+    }
+    assert_eq!(recorded(), before, "a refused ceiling recorded nothing");
+    crate::knobs::clear_all();
+    println!("ARGV ZERO-POINT SCREEN RAN");
+}
+
+/// D-4770. A DESCENT FLOOR OUTSIDE THE ONE SUPPORT DOMAIN IS REFUSED, BY THE
+/// ONE DOMAIN'S SENTENCE, AT EVERY ENTRY A DESCENT IS STARTED FROM.
+///
+/// The floor is derived: the round trips the stated win rate needs, over the
+/// rows the span's column swept. At 5,400 bp those are more than the warmed
+/// month's 1,500 swept one-minute rows, so the floor is above 1,000,000 ppm --
+/// a pattern on more than every bar. `descent_floor` never asked
+/// `support_ppm_in_domain`, so the ladder was that one support and the walk
+/// reported NOTHING PASSED AT ANY SUPPORT as though it had measured something.
+/// The argv `elite` and the door the api's descent route calls refuse it with
+/// the same sentence before any step is walked or recorded.
+#[test]
+fn a_descent_floor_outside_the_support_domain_refuses_at_every_entry() {
+    const CHILD: &str = "BRUTEX_TEST_DESCENT_FLOOR_DOMAIN";
+    if std::env::var_os(CHILD).is_some() {
+        descent_floor_domain_child();
+        return;
+    }
+    let fixture = Fixture::warmed();
+    rerun_over_store(
+        "audited_stored::tests::a_descent_floor_outside_the_support_domain_refuses_at_every_entry",
+        CHILD,
+        &fixture.root,
+        "DESCENT FLOOR DOMAIN CHECKED",
+    );
+}
+
+/// The child half, over the warmed NIFTY store `BRUTEX_STORE` names.
+fn descent_floor_domain_child() {
+    let _knobs = crate::knobs::serially();
+    crate::knobs::clear_all();
+    crate::knobs::set("BRUTEX_VALIDATE", "0");
+    crate::knobs::set("BRUTEX_MIN_WIN_RATE_BP", "5400");
+    let root = crate::store_root().expect("the fixture store");
+    // THE PREMISE: the rows that can hit, and the round trips 5,400 bp needs.
+    let swept = 1_500_u64;
+    let needed = runner::grid::trades_needed_for(
+        5_400,
+        crate::assurance_floor_bp(5_400),
+        crate::TRADES_SEARCH_CEILING,
+    );
+    assert!(
+        needed > swept && needed < crate::TRADES_SEARCH_CEILING,
+        "premise: {needed} round trips do not fit {swept} rows"
+    );
+    let floor = needed * 1_000_000 / swept;
+    let domain = crate::support_ppm_in_domain(floor).expect_err("premise: outside the domain");
+    let elite: Vec<String> = [
+        "elite", "zerodha", "NIFTY", "1min", "2025", "5", "2025", "5", "0", "1",
+    ]
+    .iter()
+    .map(ToString::to_string)
+    .collect();
+    let mut argv = String::new();
+    let code = crate::dispatch(&elite, &mut argv);
+    let api_door = crate::elite_descend_in_points_for_attempt(
+        "zerodha",
+        "NIFTY",
+        "1min",
+        ((2025, 5), (2025, 5)),
+        0,
+        1,
+        7,
+    );
+    for (door, page) in [("argv elite", argv.as_str()), ("api door", api_door.as_str())] {
+        assert!(
+            page.starts_with("refused: ") && page.contains(domain),
+            "{door}: {page}"
+        );
+        assert!(
+            page.contains(&format!("support floor is {floor} ppm")),
+            "{door}: {page}"
+        );
+        assert!(!page.contains("NOTHING PASSED"), "{door}: {page}");
+    }
+    assert_eq!(code, crate::FAILED, "{argv}");
+    assert!(
+        !crate::results::Results::path(&root).exists(),
+        "a refused floor recorded nothing"
+    );
+    assert_eq!(
+        crate::sweep_evidence::latest(&root, 1_048_576).expect("readable evidence"),
+        None,
+        "and began no attempt"
+    );
+    crate::knobs::clear_all();
+    println!("DESCENT FLOOR DOMAIN CHECKED");
+}
diff --git a/crates/cli/src/operation_audit_tests.rs b/crates/cli/src/operation_audit_tests.rs
index 32f5afdc..d3c62265 100644
--- a/crates/cli/src/operation_audit_tests.rs
+++ b/crates/cli/src/operation_audit_tests.rs
@@ -515,7 +515,10 @@ fn an_empty_journal_left_by_a_crash_reads_as_its_unconfirmed_start() {
         0,
         "nothing was repaired"
     );
-    for torn in [1, STRIDE - 1, STRIDE + 1] {
+    // 1 and `STRIDE - 1` bytes read as the same unconfirmed start since
+    // D-4772; see `a_sub_stride_journal_torn_by_a_crash_reads_as_its_unconfirmed_start`.
+    // A torn record after a whole one still refuses.
+    for torn in [STRIDE + 1, 2 * STRIDE - 1] {
         OpenOptions::new()
             .write(true)
             .open(&path)
@@ -529,3 +532,103 @@ fn an_empty_journal_left_by_a_crash_reads_as_its_unconfirmed_start() {
         );
     }
 }
+
+/// W2-cli9-5 residual, D-4772. A kill or power loss between the journal's
+/// first write and its sync can leave `0 < len < STRIDE` bytes: no record was
+/// acknowledged, because a record is acknowledged only after its whole stride
+/// is synced. That is the empty journal's fact, an indexed start nobody
+/// confirmed, and it read as a torn journal that refused every page covering
+/// the ID forever: no writer ever reopens an old ID's journal to cut it.
+///
+/// Lengths 1, half and `STRIDE - 1`, as a prefix of the start a short write
+/// leaves and as bytes that are not one (a crash may keep the length and not
+/// the data), all read as the unconfirmed start, and nothing is repaired. A
+/// torn record AFTER a whole one (`STRIDE + 1`, `2 * STRIDE - 1`), an index
+/// row that is not a start, a FIFO and a symlink at the journal path still
+/// refuse.
+#[test]
+fn a_sub_stride_journal_torn_by_a_crash_reads_as_its_unconfirmed_start() {
+    use std::os::unix::fs::symlink;
+    let root = Scratch::new();
+    let mut first = begin(&root.0, Origin::Cli, "range-all").unwrap();
+    first.armed = false;
+    let mut second = begin(&root.0, Origin::Browser, "sweep").unwrap();
+    second.finish(Phase::Completed, 0).unwrap();
+    drop(second);
+    let path = own(&base(&root.0), ID_BASE + 1);
+    let start = fs::read(&path).unwrap();
+    assert_eq!(start.len(), BYTES, "premise: the journal holds its start");
+    let half = usize::try_from(STRIDE / 2).unwrap();
+    let whole = BYTES;
+    let torn_shapes: [Vec<u8>; 5] = [
+        start[..1].to_vec(),
+        start[..half].to_vec(),
+        start[..whole - 1].to_vec(),
+        vec![0; whole - 1],
+        vec![0xA5; half],
+    ];
+    for torn in torn_shapes {
+        fs::write(&path, &torn).unwrap();
+        let read_back = read(&root.0, ID_BASE + 1).unwrap().unwrap();
+        assert_eq!(
+            (read_back.id, read_back.phase, read_back.label.as_str()),
+            (ID_BASE + 1, Phase::Started, "range-all"),
+            "{} bytes",
+            torn.len()
+        );
+        assert_eq!(read_back.phase.label(), "unconfirmed");
+        let listed = page(&root.0, None, 10).unwrap();
+        assert_eq!(
+            listed.iter().map(|r| (r.id, r.phase)).collect::<Vec<_>>(),
+            vec![
+                (ID_BASE + 2, Phase::Completed),
+                (ID_BASE + 1, Phase::Started)
+            ],
+            "{} bytes",
+            torn.len()
+        );
+        assert_eq!(fs::read(&path).unwrap(), torn, "nothing was repaired");
+    }
+
+    // A torn record after a whole start still refuses, loudly.
+    for extra in [1, whole - 1] {
+        let mut torn = start.clone();
+        torn.extend(std::iter::repeat_n(7_u8, extra));
+        fs::write(&path, &torn).unwrap();
+        assert!(read(&root.0, ID_BASE + 1).is_err(), "{} bytes", torn.len());
+        assert!(page(&root.0, None, 10).is_err(), "{} bytes", torn.len());
+    }
+
+    // An index row that is not a start refuses before the journal is read.
+    fs::write(&path, &start[..1]).unwrap();
+    let index_path = base(&root.0).join("index.bin");
+    let index = fs::read(&index_path).unwrap();
+    let mut not_started = Record::decode(&index[..BYTES].try_into().unwrap()).unwrap();
+    not_started.phase = Phase::Completed;
+    let mut rewritten = not_started.encode().unwrap().to_vec();
+    rewritten.extend_from_slice(&index[BYTES..]);
+    fs::write(&index_path, &rewritten).unwrap();
+    assert!(read(&root.0, ID_BASE + 1).is_err(), "a non-start index row");
+    fs::write(&index_path, &index).unwrap();
+    assert_eq!(
+        read(&root.0, ID_BASE + 1).unwrap().unwrap().phase,
+        Phase::Started,
+        "premise: the restored index reads"
+    );
+
+    // A FIFO and a symlink at the journal path refuse without blocking or
+    // following, whatever the target's length.
+    fs::remove_file(&path).unwrap();
+    let made = std::process::Command::new("mkfifo")
+        .arg(&path)
+        .status()
+        .unwrap();
+    assert!(made.success(), "mkfifo");
+    assert!(read(&root.0, ID_BASE + 1).is_err(), "a FIFO journal");
+    fs::remove_file(&path).unwrap();
+    let target = root.0.join("short-target");
+    fs::write(&target, [1_u8]).unwrap();
+    symlink(&target, &path).unwrap();
+    assert!(read(&root.0, ID_BASE + 1).is_err(), "a symlinked journal");
+    assert_eq!(fs::read(&target).unwrap(), [1_u8], "the target is untouched");
+}
diff --git a/crates/cli/src/operator_boundary_tests.rs b/crates/cli/src/operator_boundary_tests.rs
index af796dfd..6d9d7895 100644
--- a/crates/cli/src/operator_boundary_tests.rs
+++ b/crates/cli/src/operator_boundary_tests.rs
@@ -410,7 +410,9 @@ fn a_descent_over_a_column_that_swept_nothing_refuses() {
                 && why.contains("none of this span's 600 bar(s)")),
         "{refused:?}"
     );
-    for can_hit in [1, 300, 1_500] {
+    // Two rows, not one: BASELINE needs one round trip, so a one-row column
+    // asks for 1,000,000 ppm, which the one support domain refuses (D-4770).
+    for can_hit in [2, 300, 1_500] {
         assert_eq!(
             crate::descent_floor(&rules, can_hit, 3_000),
             Ok(crate::statistical_floor_ppm(&rules, can_hit))
@@ -423,6 +425,63 @@ fn a_descent_over_a_column_that_swept_nothing_refuses() {
     );
 }
 
+/// D-4770. THE DESCENT'S DERIVED FLOOR ASKS THE ONE SUPPORT DOMAIN.
+///
+/// `descent_floor` returned `needed × 1,000,000 / can_hit` whatever it came
+/// to, so a column with fewer rows than the round trips the rules need gave a
+/// floor of 1,000,000 ppm or more, and the walk swept that one support and
+/// reported nothing passing. It is refused now with
+/// `support_ppm_in_domain`'s own sentence, at the exact boundary: as many
+/// rows as round trips is 100% and refused, one row more is admitted.
+#[test]
+fn a_descent_floor_at_or_past_one_hundred_percent_is_refused_by_the_one_domain() {
+    let rules = crate::Rules::BASELINE.with_win_rate(5_400);
+    let needed = runner::grid::trades_needed_for(
+        rules.min_win_rate_bp,
+        rules.min_assurance_bp,
+        crate::TRADES_SEARCH_CEILING,
+    );
+    assert!(
+        needed > 2 && needed < crate::TRADES_SEARCH_CEILING,
+        "premise: a satisfiable pair needing {needed} round trips"
+    );
+    for (can_hit, admitted) in [
+        (1, false),
+        (needed - 1, false),
+        (needed, false),
+        (needed + 1, true),
+        (u64::MAX, true),
+    ] {
+        let floor = crate::statistical_floor_ppm(&rules, can_hit);
+        let got = crate::descent_floor(&rules, can_hit, 3_000);
+        if admitted {
+            assert!(floor < 1_000_000, "{can_hit}: {floor}");
+            assert_eq!(got, Ok(floor), "{can_hit}");
+            continue;
+        }
+        let domain = crate::support_ppm_in_domain(floor).err().unwrap_or_default();
+        assert!(!domain.is_empty(), "premise: {floor} ppm is outside the domain");
+        let why = got.err().unwrap_or_default();
+        for part in [
+            "refused: ",
+            domain,
+            &format!("support floor is {floor} ppm"),
+            &format!("{needed} round trip(s)"),
+            &format!("the {can_hit} row(s)"),
+        ] {
+            assert!(why.contains(part), "{can_hit}: missing {part:?} in {why}");
+        }
+        assert!(why.starts_with("refused: "), "{why}");
+    }
+    let one_trip = crate::descent_floor(&crate::Rules::BASELINE, 1, 3_000);
+    assert!(
+        one_trip
+            .as_ref()
+            .is_err_and(|why| why.contains("support floor is 1000000 ppm")),
+        "one round trip over one row is 100%: {one_trip:?}"
+    );
+}
+
 /// The descent banner names the rows it sized on as swept bars, and its
 /// round-trip estimate is the floor over those rows. D-2101.
 #[test]
diff --git a/crates/cli/src/search_checkpoint_tests.rs b/crates/cli/src/search_checkpoint_tests.rs
index 6d822038..32c7ab1b 100644
--- a/crates/cli/src/search_checkpoint_tests.rs
+++ b/crates/cli/src/search_checkpoint_tests.rs
@@ -717,3 +717,84 @@ fn a_failed_marker_names_its_temporary_only_when_it_could_not_be_removed() -> Re
     assert!(!reservation.join("complete").exists());
     Ok(())
 }
+
+/// W2-cli13-5 residual, D-4773. Discovery counted EVERY directory entry
+/// against `directory_limit()`, operating-system litter included, while a
+/// publication counts only what the journal itself creates. So a namespace
+/// published to exactly the limit became unopenable the moment Finder (or a
+/// copy to a non-HFS volume) wrote one `.DS_Store` or `._` file into it.
+///
+/// Now one rule decides what counts, at both: `owner.lock` and reservation
+/// directories count against `directory_limit()`; plain-file litter is passed
+/// over and counted against its own ceiling, `litter_limit()`, which is one
+/// `AppleDouble` twin for every counted entry plus `.DS_Store` and its own
+/// twin. One litter file past that ceiling refuses by name, and litter that is
+/// not a plain file is a stranger and refuses by name, as before.
+#[test]
+fn litter_after_publication_never_makes_a_full_namespace_unopenable() -> Result<(), String> {
+    LIMIT.with(|limit| limit.set(Some(4)));
+    let result = (|| {
+        let scratch = Scratch::new().map_err(error)?;
+        let identity = [21; 32];
+        let mut journal = Journal::open(&scratch.0, "and-checkpoint-v1", identity)?;
+        for payload in [&b"one"[..], b"two", b"three"] {
+            journal.publish(payload, 1024)?;
+        }
+        let full = Err("checkpoint namespace reached its directory admission limit".to_owned());
+        assert_eq!(journal.publish(b"four", 1024), full);
+        drop(journal);
+
+        let dir = journal_dir(&scratch.0, identity);
+        let mut litter = vec![".DS_Store".to_owned(), "._.DS_Store".to_owned()];
+        litter.push("._owner.lock".to_owned());
+        litter.extend((1_u64..=3).map(|sequence| format!("._{sequence:016x}")));
+        assert_eq!(litter.len(), litter_limit(), "premise: litter at its ceiling");
+        for name in &litter {
+            fs::write(dir.join(name), b"os litter").map_err(error)?;
+        }
+        let mut reopened = Journal::open(&scratch.0, "and-checkpoint-v1", identity)?;
+        assert_eq!(reopened.entries, 4, "litter is not counted");
+        assert_eq!(reopened.latest(1024)?.ok_or("latest")?.payload, b"three");
+        assert_eq!(
+            reopened.publish(b"four", 1024),
+            full,
+            "a publication refuses at the count discovery admits"
+        );
+        drop(reopened);
+        let snapshot =
+            Snapshot::open(&scratch.0, "and-checkpoint-v1", identity)?.ok_or("observed")?;
+        assert_eq!((snapshot.latest, snapshot.acknowledged), (Some(3), 3));
+
+        fs::write(dir.join("._0000000000000009"), b"one too many").map_err(error)?;
+        let refused = Journal::open(&scratch.0, "and-checkpoint-v1", identity).err();
+        assert!(
+            refused
+                .as_deref()
+                .is_some_and(|why| why.contains("litter") && why.contains(&dir.display().to_string())),
+            "{refused:?}"
+        );
+        fs::remove_file(dir.join("._0000000000000009")).map_err(error)?;
+
+        // Litter by name and not a plain file is a stranger, named.
+        std::os::unix::fs::symlink(dir.join("._owner.lock"), dir.join("._link")).map_err(error)?;
+        let refused = Journal::open(&scratch.0, "and-checkpoint-v1", identity).err();
+        assert!(
+            refused.as_deref().is_some_and(|why| why.contains("\"._link\"")),
+            "{refused:?}"
+        );
+        fs::remove_file(dir.join("._link")).map_err(error)?;
+        let made = std::process::Command::new("mkfifo")
+            .arg(dir.join("._fifo"))
+            .status()
+            .map_err(error)?;
+        assert!(made.success(), "mkfifo");
+        let refused = Journal::open(&scratch.0, "and-checkpoint-v1", identity).err();
+        assert!(
+            refused.as_deref().is_some_and(|why| why.contains("\"._fifo\"")),
+            "{refused:?}"
+        );
+        Ok(())
+    })();
+    LIMIT.with(|limit| limit.set(None));
+    result
+}
diff --git a/docs/19-candidate-trades.md b/docs/19-candidate-trades.md
index 9766d4ea..d6c3e525 100644
--- a/docs/19-candidate-trades.md
+++ b/docs/19-candidate-trades.md
@@ -1,12 +1,16 @@
 # Exact priced-candidate evidence, version 1
 
 This format captures the exact policy-selected exit cell for **both directions
-of every retained signal candidate actually evaluated by each visited screen
-pass**. It preserves nonwinning candidates and explicit no-cell outcomes.
-It does not call every potential combination priced. The upstream retention
-limit, screen cap, budget calibration and visited policy tiers still bound the
-work. Nor does it store every exit-grid cell, bootstrap sample or validation-fold
-grid as a separate candidate.
+of every retained signal candidate actually evaluated by each captured screen
+pass**: the operator's own policy and, when it admitted nothing and a recorded
+tier walk ran, the one tier that walk ended on. Tiers the walk judged on the
+way are judged without a capture (D-4716), so a recorded cascade's
+`tier_count` is at most two, and the browser labels it "Captured screen
+passes" (D-4771). It preserves nonwinning candidates and explicit no-cell
+outcomes. It does not call every potential combination priced. The upstream
+retention limit, screen cap, budget calibration and the captured screen passes
+still bound the work. Nor does it store every exit-grid cell, bootstrap sample
+or validation-fold grid as a separate candidate.
 
 ## Authoritative execution boundary
 
diff --git a/web/src/lib/CandidateTrades.svelte b/web/src/lib/CandidateTrades.svelte
index f461d7bd..c8f02834 100644
--- a/web/src/lib/CandidateTrades.svelte
+++ b/web/src/lib/CandidateTrades.svelte
@@ -51,16 +51,16 @@
 
 <section class="candidate-detail" aria-label="Exact candidate trades">
   <div class="heading"><div><span class="eyebrow">Every priced candidate</span><h3>Explore the exact trades behind a result</h3></div><button onclick={() => { tier = '0'; void load({ identity, attempt, model, digest: initialDigest }); }}>Refresh capture</button></div>
-  <p class="scope">Both directions, for each candidate actually priced in each visited policy pass. Each row opens its own selected exit cell. Pricing evidence and whole-audit completion are separate facts.</p>
+  <p class="scope">Both directions, for each candidate actually priced in each captured screen pass: your own policy and, when it admitted nothing and the walk ran, the tier the walk ended on. Tiers judged on the way are not captured. Each row opens its own selected exit cell. Pricing evidence and whole-audit completion are separate facts.</p>
   {#if loaded.phase === 'loading'}<p role="status">Verifying the saved candidate catalog…</p>
   {:else if loaded.phase === 'failed'}<p role="alert" class="problem">Candidate evidence unavailable: {loaded.why}</p>
   {:else if loaded.body.status === 'missing'}<p class="notice">{loaded.body.why}</p>
   {:else}
     {@const body = loaded.body}
     {#if body.expression}<p class="notice">Expression pricing uses the complete saved predicate. The condition names below show referenced bits only; they do not imply AND. <b>{body.expression.source ?? 'Versioned predicate saved'}</b></p><details><summary>Exact versioned predicate bytes</summary><code class="predicate">{body.expression.encoded_hex}</code></details>{/if}
-    <p class="facts">Saved candidate-side outcomes <b>{body.candidate_side_count}</b> · Visited policy passes <b>{body.tier_count}</b> · Audit state <b>{body.audit_completion ?? 'unknown'}</b></p>
+    <p class="facts">Saved candidate-side outcomes <b>{body.candidate_side_count}</b> · Captured screen passes <b>{body.tier_count}</b> · Audit state <b>{body.audit_completion ?? 'unknown'}</b></p>
     {#if body.tier}
-      <div class="chooser"><label>Policy pass (starts at 0) <input inputmode="numeric" bind:value={tier} /></label><button onclick={chooseTier}>Open pass</button></div>
+      <div class="chooser"><label>Captured pass (starts at 0) <input inputmode="numeric" bind:value={tier} /></label><button onclick={chooseTier}>Open pass</button></div>
       <p class="notice">This pass priced <b>{body.tier.evaluated}</b> of <b>{body.tier.eligible}</b> retained signal candidates in both directions. Hold: {body.tier.horizon} execution bars. A cell-rule pass does not establish calendar consistency or institutional admission.</p>
       <details><summary>Applied policy and exact grid inputs</summary><dl>{#each Object.entries(body.tier.rules) as [name, value]}<dt>{name.replaceAll('_', ' ')}</dt><dd>{String(value)}</dd>{/each}<dt>Grid rungs</dt><dd>{body.tier.rungs}</dd><dt>Forced stop, ppm</dt><dd>{body.tier.forced_ppm ?? 'none'}</dd><dt>Grid step, ppm</dt><dd>{body.tier.step_ppm ?? 'derived'}</dd></dl></details>
       <div class="scroll"><table><caption>Exact candidate-side results · {body.total_count} in this pass</caption><thead><tr><th>Signal rank</th><th>Direction</th><th>Conditions</th><th>Trades</th><th>Worst-fill total</th><th>Best-fill total</th><th>Cell rules</th><th>Detail</th></tr></thead><tbody>
@@ -73,13 +73,13 @@
       </tbody></table></div>
       {#if body.rows.length === 0}<p class="notice">This pass explicitly recorded no candidate-side rows.</p>{/if}
       <div class="paging"><button disabled={body.offset === '0'} onclick={() => candidates(previous(body.offset))}>Previous candidates</button><span>Rows from {body.offset}</span><button disabled={body.next_offset === null} onclick={() => candidates(body.next_offset)}>Next candidates</button></div>
-    {:else}<p class="notice">The capture is sealed with no visited pricing pass. This does not imply a completed profitable search.</p>{/if}
+    {:else}<p class="notice">The capture is sealed with no captured screen pass. This does not imply a completed profitable search.</p>{/if}
   {/if}
   {#if trades.phase === 'loading'}<p role="status">Verifying this candidate’s exact trade file…</p>
   {:else if trades.phase === 'failed'}<p role="alert" class="problem">Exact trades unavailable: {trades.why} No other candidate’s trades are substituted.</p>
   {:else if trades.phase === 'ready'}
     {@const detail = trades.body}
-    <h4>Signal rank {detail.selected.rank} · {detail.selected.direction} · policy pass {detail.selected.tier}</h4>
+    <h4>Signal rank {detail.selected.rank} · {detail.selected.direction} · captured pass {detail.selected.tier}</h4>
     <p class="scope">{candidateConditions(detail.selected, vocabulary)}</p>
     <div class="scroll"><table><caption>Exact selected-cell trades · {detail.total_count} recorded</caption><thead><tr><th>Trade</th><th>Entry (IST)</th><th>Exit (IST)</th><th>Entry bar</th><th>Exit bar</th><th>Worst fill</th><th>Best fill</th><th>Adverse move</th><th>Favourable move</th></tr></thead><tbody>{#each detail.rows as row (row.seq)}<tr><td>{String(BigInt(row.seq) + 1n)}</td><td title={row.entry_micros + ' µs'}>{candidateTime(row.entry_micros)}</td><td title={row.exit_micros + ' µs'}>{candidateTime(row.exit_micros)}</td><td>{row.entry_bar}</td><td>{row.exit_bar}</td><td>{candidateMoney(row.worst)}</td><td>{candidateMoney(row.best)}</td><td>{candidateMoney(row.adverse_paisa)}</td><td>{candidateMoney(row.favourable_paisa)}</td></tr>{/each}</tbody></table></div>
     {#if detail.rows.length === 0}<p class="notice">This exact cell recorded zero trades.</p>{/if}
diff --git a/web/tests/candidate-pass-label.test.js b/web/tests/candidate-pass-label.test.js
new file mode 100644
index 00000000..d0273c12
--- /dev/null
+++ b/web/tests/candidate-pass-label.test.js
@@ -0,0 +1,48 @@
+import { readFileSync } from 'node:fs';
+import { test } from 'node:test';
+import assert from 'node:assert/strict';
+
+// D-4771. `tier_count` is `cli::candidate_trades::Summary::tiers`, which D-4716
+// made the count of CAPTURED screen passes: the operator's own policy and, when
+// a recorded walk ran, the one tier it ended on. Every tier judged on the way
+// is judged without a capture. The page labelled the number "Visited policy
+// passes", which reads as every tier the walk visited -- up to the whole
+// ladder -- while the number is at most two.
+
+const component = readFileSync(new URL('../src/lib/CandidateTrades.svelte', import.meta.url), 'utf8');
+const source = readFileSync(new URL('../../crates/cli/src/candidate_trades.rs', import.meta.url), 'utf8');
+
+/** The markup of the facts line that labels `tier_count`. */
+const factsLine = () => {
+  const start = component.indexOf('<p class="facts">');
+  assert.notEqual(start, -1, 'the component renders the facts line');
+  return component.slice(start, component.indexOf('</p>', start));
+};
+
+test('the server still defines the number as captured screen passes', () => {
+  const field = source.indexOf('    pub tiers: u64,');
+  assert.notEqual(field, -1, 'Summary carries tiers');
+  const doc = source
+    .slice(source.lastIndexOf('pub execution_digest', field), field)
+    .replace(/\s*\/\/\/\s*/g, ' ');
+  assert.match(doc, /Number of captured screen passes, not generated policy tiers/);
+  assert.match(doc, /the operator's own policy and the tier a recorded walk ended on/);
+  assert.match(component, /body\.tier_count/, 'the component shows tier_count');
+});
+
+test('tier_count is labelled as captured screen passes, not visited ones', () => {
+  const facts = factsLine();
+  assert.match(facts, /Captured screen passes <b>\{body\.tier_count\}<\/b>/);
+  assert.doesNotMatch(component, /[Vv]isited (policy|pricing) pass/);
+});
+
+test('the page says which screens a pass is and that judged tiers are not captured', () => {
+  const scope = component.slice(component.indexOf('<p class="scope">'), component.indexOf('</p>', component.indexOf('<p class="scope">')));
+  assert.match(scope, /each captured screen pass/);
+  assert.match(scope, /your own policy/);
+  assert.match(scope, /the tier the walk ended on/);
+  assert.match(scope, /not captured/);
+  assert.match(component, /The capture is sealed with no captured screen pass\./);
+  assert.match(component, /Captured pass \(starts at 0\)/);
+  assert.match(component, /captured pass \{detail\.selected\.tier\}/);
+});
```

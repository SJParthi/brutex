# base-cov-api-fu.txt.md

Source: `state/fix/round2/base-cov-api-fu.txt` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````text
(F)/(U) list of BASE-COV-API repair 2, tip 8091e72f7d857df474de2224a382b1e41c4c5ce9, whole:api, from /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/deep-BASE-COV-API-logs/8091e72f7d85-routeb-r2.log.
(F) = the failing side of an assert!/assert_eq!/assert_ne!/panic!/unreachable! whose statement ran, in test code (recipe step 4); each row read by hand (state/fix/round2/repair-BASE-COV-API-progress.md).
(U) = lib.rs: mod items, attributes and comments, no function; no object holds a region of it.
(F) 427 lines 1053 sides.

(F) LINE crates/api/src/assets.rs:1137  [failing arm of the statement at 1134, which ran]  default_web_dir().display()
(F) LINE crates/api/src/assets.rs:1453  [failing arm of the statement at 1450, which ran]  assets.named().display()
(F) LINE crates/api/src/assets.rs:1566  [panic/unreachable line]  panic!(
(F) BRANCH crates/api/src/assets.rs:1530:17 False  [the false side fails the check, and its line ran]  assert!(note.contains("NO SHELL") && note.contains(INDEX), "{note}");
(F) BRANCH crates/api/src/assets.rs:1530:46 False  [the false side fails the check, and its line ran]  assert!(note.contains("NO SHELL") && note.contains(INDEX), "{note}");
(F) BRANCH crates/api/src/assets.rs:1565:13 False  [the else of a let-else that panics, whose line ran]  let Build::Stale { ref newer, by_secs } = *built.build() else {
(F) BRANCH crates/api/src/assets.rs:1582:13 False  [the false side fails the check, and its line ran]  note.contains("STALE") && note.contains("does not carry that edit"),
(F) BRANCH crates/api/src/assets.rs:1582:39 False  [the false side fails the check, and its line ran]  note.contains("STALE") && note.contains("does not carry that edit"),
(F) LINE crates/api/src/audit.rs:3566  [panic/unreachable line]  panic!("three entries: {page:?}");
(F) LINE crates/api/src/audit.rs:3635  [failing arm of the statement at 3632, which ran]  &read[read.len().saturating_sub(40)..]
(F) LINE crates/api/src/audit.rs:4041  [failing arm of the statement at 4035, which ran]  kind.label(),
(F) LINE crates/api/src/audit.rs:4042  [failing arm of the statement at 4035, which ran]  scope.label(),
(F) LINE crates/api/src/audit.rs:4043  [failing arm of the statement at 4035, which ran]  outcome.label()
(F) LINE crates/api/src/audit.rs:5587  [panic/unreachable line]  panic!(
(F) BRANCH crates/api/src/audit.rs:3410:13 False  [the false side fails the check, and its line ran]  empty.failures.is_empty() && empty.balances(),
(F) BRANCH crates/api/src/audit.rs:3410:42 False  [the false side fails the check, and its line ran]  empty.failures.is_empty() && empty.balances(),
(F) BRANCH crates/api/src/audit.rs:3565:13 False  [the else of a let-else that panics, whose line ran]  let [run, fits, whole] = page.as_slice() else {
(F) BRANCH crates/api/src/audit.rs:4431:13 False  [the false side fails the check, and its line ran]  refusal.contains("injected short write")
(F) BRANCH crates/api/src/audit.rs:4432:20 False  [the false side fails the check, and its line ran]  && refusal.contains("truncated back to 256 bytes"),
(F) BRANCH crates/api/src/audit.rs:5586:13 False  [the else of a let-else that panics, whose line ran]  let Some(Err(why)) = &page[0].whole_note else {
(F) LINE crates/api/src/audit_json.rs:1538  [failing arm of the statement at 1535, which ran]  other.as_str()
(F) LINE crates/api/src/audit_json.rs:1840  [panic/unreachable line]  panic!("counted again for an unchanged census")
(F) BRANCH crates/api/src/audit_json.rs:1425:17 False  [the false side fails the check, and its line ran]  assert!(json.starts_with('{') && json.ends_with('}'), "{json}");
(F) BRANCH crates/api/src/audit_json.rs:1425:42 False  [the false side fails the check, and its line ran]  assert!(json.starts_with('{') && json.ends_with('}'), "{json}");
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:86:80 False  [the false side fails the check, and its line ran]  matches!(census.state, crate::census::Census::Held { ref manifest } if manifest.is_partitioned()),
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:184:9 False  [the false side fails the check, and its line ran]  note.contains("CASH") && note.contains("lower bound"),
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:184:34 False  [the false side fails the check, and its line ran]  note.contains("CASH") && note.contains("lower bound"),
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:256:9 False  [the false side fails the check, and its line ran]  note.contains("is a directory") && note.contains("not a commit"),
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:256:44 False  [the false side fails the check, and its line ran]  note.contains("is a directory") && note.contains("not a commit"),
(F) LINE crates/api/src/audit_lock_wait_tests.rs:659  [panic/unreachable line]  panic!("the parent should have killed this child");
(F) BRANCH crates/api/src/audit_lock_wait_tests.rs:715:9 False  [the false side fails the check, and its line ran]  !source.contains("\nstatic ") && !source.contains("LazyLock") && !source.contains("Mutex<"),
(F) BRANCH crates/api/src/audit_lock_wait_tests.rs:715:42 False  [the false side fails the check, and its line ran]  !source.contains("\nstatic ") && !source.contains("LazyLock") && !source.contains("Mutex<"),
(F) BRANCH crates/api/src/audit_lock_wait_tests.rs:715:74 False  [the false side fails the check, and its line ran]  !source.contains("\nstatic ") && !source.contains("LazyLock") && !source.contains("Mutex<"),
(F) LINE crates/api/src/autopilot.rs:4814  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) LINE crates/api/src/autopilot.rs:4817  [panic/unreachable line]  panic!("Dhan's floor is the rolling one");
(F) LINE crates/api/src/autopilot.rs:4878  [panic/unreachable line]  panic!("{} is driven, so it is HTTP", feed.display());
(F) LINE crates/api/src/autopilot.rs:4883  [failing arm of the statement at 4880, which ran]  feed.display()
(F) LINE crates/api/src/autopilot.rs:4888  [failing arm of the statement at 4885, which ran]  feed.display()
(F) LINE crates/api/src/autopilot.rs:4893  [failing arm of the statement at 4890, which ran]  feed.display()
(F) LINE crates/api/src/autopilot.rs:4905  [failing arm of the statement at 4902, which ran]  feed.display()
(F) LINE crates/api/src/autopilot.rs:4915  [failing arm of the statement at 4912, which ran]  feed.display()
(F) LINE crates/api/src/autopilot.rs:4945  [failing arm of the statement at 4941, which ran]  feed.display()
(F) LINE crates/api/src/autopilot.rs:5058  [panic/unreachable line]  panic!("a same-value verdict must halt");
(F) LINE crates/api/src/autopilot.rs:5143  [panic/unreachable line]  panic!("attempt {MAX_MONTH_ATTEMPTS} must stall, got {verdicts:?}");
(F) LINE crates/api/src/autopilot.rs:5161  [failing arm of the statement at 5158, which ran]  status.json()
(F) LINE crates/api/src/autopilot.rs:5395  [panic/unreachable line]  panic!("a census whose migration receipt will not read is refused, never held");
(F) LINE crates/api/src/autopilot.rs:5464  [failing arm of the statement at 5460, which ran]  action.slug()
(F) LINE crates/api/src/autopilot.rs:6042  [panic/unreachable line]  panic!("a repeated disk failure must halt");
(F) LINE crates/api/src/autopilot.rs:6767  [failing arm of the statement at 6764, which ran]  phase.state()
(F) LINE crates/api/src/autopilot.rs:7141  [panic/unreachable line]  panic!("a same-value verdict must halt");
(F) LINE crates/api/src/autopilot.rs:7191  [panic/unreachable line]  panic!("{credential:?}: the bound stalls the month, it does not halt the feed");
(F) LINE crates/api/src/autopilot.rs:7224  [panic/unreachable line]  panic!("an unusable configuration halts");
(F) LINE crates/api/src/autopilot.rs:7328  [failing arm of the statement at 7323, which ran]  reports.iter().map(|r| &r.last_reason).collect::<Vec<_>>()
(F) LINE crates/api/src/autopilot.rs:7365  [panic/unreachable line]  panic!("an unarmed feed must not answer Now");
(F) LINE crates/api/src/autopilot.rs:7389  [panic/unreachable line]  panic!("probe {expected} of {STORE_PROBES} must be due, at {now}");
(F) LINE crates/api/src/autopilot.rs:7401  [panic/unreachable line]  panic!("the {STORE_PROBES}-probe allowance must be spent, not renewed by time");
(F) LINE crates/api/src/autopilot.rs:7443  [failing arm of the statement at 7440, which ran]  missing.display()
(F) LINE crates/api/src/autopilot.rs:7567  [panic/unreachable line]  panic!("the same store refusal twice must halt");
(F) LINE crates/api/src/autopilot.rs:7784  [panic/unreachable line]  panic!("one tracked instrument is a universe");
(F) LINE crates/api/src/autopilot.rs:8048  [failing arm of the statement at 8043, which ran]  kind.word()
(F) LINE crates/api/src/autopilot.rs:8261  [failing arm of the statement at 8256, which ran]  .file_name()
(F) LINE crates/api/src/autopilot.rs:8262  [failing arm of the statement at 8256, which ran]  .to_string_lossy()
(F) LINE crates/api/src/autopilot.rs:8263  [failing arm of the statement at 8256, which ran]  .starts_with(STORE_PROBE_PREFIX)),
(F) LINE crates/api/src/autopilot.rs:8398  [failing arm of the statement at 8395, which ran]  site.autopilot.json()
(F) LINE crates/api/src/autopilot.rs:8410  [failing arm of the statement at 8407, which ran]  site.autopilot.json()
(F) LINE crates/api/src/autopilot.rs:8785  [failing arm of the statement at 8782, which ran]  reports[0].halted
(F) LINE crates/api/src/autopilot.rs:8874  [failing arm of the statement at 8871, which ran]  site.autopilot.json()
(F) LINE crates/api/src/autopilot.rs:8971  [panic/unreachable line]  panic!("the tick wrote its journal: {:?}", journal.look());
(F) BRANCH crates/api/src/autopilot.rs:4816:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::HistoryFloor::Rolling { years } = spec.history_floor else {
(F) BRANCH crates/api/src/autopilot.rs:4877:17 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/autopilot.rs:4942:45 False  [the false side fails the check, and its line ran]  !path_names_the_rung || spec.granularity_token(rung).is_some(),
(F) BRANCH crates/api/src/autopilot.rs:5057:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { reason } = state.observe(&dead) else {
(F) BRANCH crates/api/src/autopilot.rs:5142:13 False  [the else of a let-else that panics, whose line ran]  let Some(Next::Stall { reason }) = verdicts.last() else {
(F) BRANCH crates/api/src/autopilot.rs:5312:26 False  [the false side fails the check, and its line ran]  .all(|f| f.halt_kind == Some(Halt::Census) && f.halted.is_some()),
(F) BRANCH crates/api/src/autopilot.rs:5317:17 False  [the false side fails the check, and its line ran]  report.halted.contains(drift) && report.halted.contains("is not known"),
(F) BRANCH crates/api/src/autopilot.rs:5317:50 False  [the false side fails the check, and its line ran]  report.halted.contains(drift) && report.halted.contains("is not known"),
(F) BRANCH crates/api/src/autopilot.rs:5337:50 False  [the false side fails the check, and its line ran]  Census::Unreadable { reason, .. } if reason == "entry 41 fails its own checksum"
(F) BRANCH crates/api/src/autopilot.rs:5394:13 False  [the else of a let-else that panics, whose line ran]  let Census::Unreadable { reason, .. } = &refused.state else {
(F) BRANCH crates/api/src/autopilot.rs:5398:13 False  [the false side fails the check, and its line ran]  reason.contains("is not known")
(F) BRANCH crates/api/src/autopilot.rs:5399:20 False  [the false side fails the check, and its line ran]  && reason.contains("MIGRATED")
(F) BRANCH crates/api/src/autopilot.rs:5400:20 False  [the false side fails the check, and its line ran]  && reason.contains("cannot be read"),
(F) BRANCH crates/api/src/autopilot.rs:5531:13 False  [the false side fails the check, and its line ran]  body.contains("the broker credential is dead")
(F) BRANCH crates/api/src/autopilot.rs:5532:20 False  [the false side fails the check, and its line ran]  && body.contains("the store refused the same write twice"),
(F) BRANCH crates/api/src/autopilot.rs:6041:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { reason } = state.observe(&full) else {
(F) BRANCH crates/api/src/autopilot.rs:7120:13 False  [the false side fails the check, and its line ran]  state.halted.is_none() && state.halt_kind.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7120:39 False  [the false side fails the check, and its line ran]  state.halted.is_none() && state.halt_kind.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7140:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { reason } = token.observe(&dead) else {
(F) BRANCH crates/api/src/autopilot.rs:7190:17 False  [the else of a let-else that panics, whose line ran]  let Next::Stall { reason: said } = state.observe(&out) else {
(F) BRANCH crates/api/src/autopilot.rs:7223:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { reason } = state.observe(&blocked) else {
(F) BRANCH crates/api/src/autopilot.rs:7292:26 False  [the false side fails the check, and its line ran]  .all(|f| f.halt_kind == Some(Halt::Census) && f.halted.is_some()),
(F) BRANCH crates/api/src/autopilot.rs:7320:26 False  [the false side fails the check, and its line ran]  .all(|f| f.halted.is_none() && f.halt_kind.is_none()),
(F) BRANCH crates/api/src/autopilot.rs:7326:26 False  [the false side fails the check, and its line ran]  .all(|r| r.halted.is_empty() && r.last_reason.contains("manifest loads again")),
(F) BRANCH crates/api/src/autopilot.rs:7364:13 False  [the else of a let-else that panics, whose line ran]  let Due::Spent { saying } = store_due(None, 0) else {
(F) BRANCH crates/api/src/autopilot.rs:7388:17 False  [the else of a let-else that panics, whose line ran]  let Due::Now { made } = store_due(Some(&probe), now) else {
(F) BRANCH crates/api/src/autopilot.rs:7400:13 False  [the else of a let-else that panics, whose line ran]  let Due::Spent { saying } = store_due(Some(&probe), now.saturating_add(1_000_000)) else {
(F) BRANCH crates/api/src/autopilot.rs:7509:13 False  [the false side fails the check, and its line ran]  why.contains("already holds the probe's name") && why.contains(STORE_PROBE_PREFIX),
(F) BRANCH crates/api/src/autopilot.rs:7509:63 False  [the false side fails the check, and its line ran]  why.contains("already holds the probe's name") && why.contains(STORE_PROBE_PREFIX),
(F) BRANCH crates/api/src/autopilot.rs:7566:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { .. } = first.observe(&disk) else {
(F) BRANCH crates/api/src/autopilot.rs:7588:13 False  [the false side fails the check, and its line ran]  back.halted.is_none() && back.halt_kind.is_none() && back.probe.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7588:38 False  [the false side fails the check, and its line ran]  back.halted.is_none() && back.halt_kind.is_none() && back.probe.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7588:66 False  [the false side fails the check, and its line ran]  back.halted.is_none() && back.halt_kind.is_none() && back.probe.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7663:17 False  [the false side fails the check, and its line ran]  saying.contains("2021-03") && saying.contains("being reconsidered"),
(F) BRANCH crates/api/src/autopilot.rs:7663:47 False  [the false side fails the check, and its line ran]  saying.contains("2021-03") && saying.contains("being reconsidered"),
(F) BRANCH crates/api/src/autopilot.rs:7783:13 False  [the else of a let-else that panics, whose line ran]  let Settled::Complete { instruments } = Settled::over(&one) else {
(F) BRANCH crates/api/src/autopilot.rs:7906:13 False  [the false side fails the check, and its line ran]  detail.contains("NOT COMPLETE") && detail.contains("NOTHING IS TRACKED"),
(F) BRANCH crates/api/src/autopilot.rs:7906:48 False  [the false side fails the check, and its line ran]  detail.contains("NOT COMPLETE") && detail.contains("NOTHING IS TRACKED"),
(F) BRANCH crates/api/src/autopilot.rs:8051:21 False  [the false side fails the check, and its line ran]  assert!(state.halted.is_none() && state.halt_kind.is_none());
(F) BRANCH crates/api/src/autopilot.rs:8051:47 False  [the false side fails the check, and its line ran]  assert!(state.halted.is_none() && state.halt_kind.is_none());
(F) BRANCH crates/api/src/autopilot.rs:8214:17 False  [the false side fails the check, and its line ran]  assert!(due > before && due <= before + 3, "{due} vs {before}");
(F) BRANCH crates/api/src/autopilot.rs:8214:33 False  [the false side fails the check, and its line ran]  assert!(due > before && due <= before + 3, "{due} vs {before}");
(F) BRANCH crates/api/src/autopilot.rs:8832:13 False  [the false side fails the check, and its line ran]  shown >= before && shown <= after,
(F) BRANCH crates/api/src/autopilot.rs:8832:32 False  [the false side fails the check, and its line ran]  shown >= before && shown <= after,
(F) BRANCH crates/api/src/autopilot.rs:8970:13 False  [the else of a let-else that panics, whose line ran]  let audit::Log::Held { records, .. } = journal.look() else {
(F) BRANCH crates/api/src/autopilot.rs:8992:13 False  [the false side fails the check, and its line ran]  whole.starts_with(&closed.note) && !closed.note.is_empty(),
(F) BRANCH crates/api/src/autopilot.rs:8992:48 False  [the false side fails the check, and its line ran]  whole.starts_with(&closed.note) && !closed.note.is_empty(),
(F) LINE crates/api/src/autopilot_stall_tests.rs:105  [failing arm of the statement at 101, which ran]  feeds[0].stalls
(F) LINE crates/api/src/backtest.rs:2326  [failing arm of the statement at 2323, which ran]  broken.to_json()
(F) BRANCH crates/api/src/backtest.rs:2151:13 False  [the false side fails the check, and its line ran]  body.contains(r#""runs":[]"#) && body.contains(r#""best_complete":null"#),
(F) BRANCH crates/api/src/backtest.rs:2151:46 False  [the false side fails the check, and its line ran]  body.contains(r#""runs":[]"#) && body.contains(r#""best_complete":null"#),
(F) LINE crates/api/src/bars.rs:2869  [failing arm of the statement at 2866, which ran]  found.len()
(F) LINE crates/api/src/bars.rs:3028  [failing arm of the statement at 3025, which ran]  compared.get()
(F) LINE crates/api/src/bars.rs:3103  [failing arm of the statement at 3099, which ran]  ist_clock(at)
(F) LINE crates/api/src/bars.rs:3656  [failing arm of the statement at 3653, which ran]  got.bars[0].chg_why
(F) LINE crates/api/src/bars.rs:4620  [panic/unreachable line]  panic!("one request took more than 256 looks at its flag");
(F) BRANCH crates/api/src/bars.rs:3920:17 False  [the false side fails the check, and its line ran]  !body.contains("note_unreadable_records") && !body.contains("telemetry::emit"),
(F) BRANCH crates/api/src/bars.rs:3920:62 False  [the false side fails the check, and its line ran]  !body.contains("note_unreadable_records") && !body.contains("telemetry::emit"),
(F) BRANCH crates/api/src/bars.rs:4202:13 False  [the false side fails the check, and its line ran]  got.faults[0].contains(" record 73: ")
(F) BRANCH crates/api/src/bars.rs:4203:20 False  [the false side fails the check, and its line ran]  && got.faults[0].contains("73 kept rows of 2026-01 in this window would not read"),
(F) BRANCH crates/api/src/bars.rs:4301:17 False  [the false side fails the check, and its line ran]  assert!(first.oi_applies && first.v_why.is_none());
(F) BRANCH crates/api/src/bars.rs:4301:37 False  [the false side fails the check, and its line ran]  assert!(first.oi_applies && first.v_why.is_none());
(F) BRANCH crates/api/src/bars_change.rs:764:13 False  [the false side fails the check, and its line ran]  why.contains("UNVERIFIED") && why.contains("absent"),
(F) BRANCH crates/api/src/bars_change.rs:764:43 False  [the false side fails the check, and its line ran]  why.contains("UNVERIFIED") && why.contains("absent"),
(F) BRANCH crates/api/src/bars_change.rs:1036:13 False  [the false side fails the check, and its line ran]  fault.starts_with("cannot read ") && fault.ends_with("no lineage was applied"),
(F) BRANCH crates/api/src/bars_change.rs:1036:50 False  [the false side fails the check, and its line ran]  fault.starts_with("cannot read ") && fault.ends_with("no lineage was applied"),
(F) LINE crates/api/src/bars_export.rs:1120  [panic/unreachable line]  panic!("the pools stayed full");
(F) LINE crates/api/src/bars_export.rs:1139  [panic/unreachable line]  panic!("the pools stayed full");
(F) BRANCH crates/api/src/bars_export.rs:1689:13 False  [the false side fails the check, and its line ran]  body.contains("export_not_whole") && body.contains("2025-06"),
(F) BRANCH crates/api/src/bars_export.rs:1689:50 False  [the false side fails the check, and its line ran]  body.contains("export_not_whole") && body.contains("2025-06"),
(F) BRANCH crates/api/src/bars_export.rs:1769:13 False  [the false side fails the check, and its line ran]  body.contains("\"being_written\":true") && body.contains("r3-10-export-writer"),
(F) BRANCH crates/api/src/bars_export.rs:1769:56 False  [the false side fails the check, and its line ran]  body.contains("\"being_written\":true") && body.contains("r3-10-export-writer"),
(F) LINE crates/api/src/bars_page_tests.rs:465  [panic/unreachable line]  panic!("{route}: the pool stayed full for two seconds");
(F) BRANCH crates/api/src/bars_page_tests.rs:286:9 False  [the false side fails the check, and its line ran]  !page.contains("/FNO/NIFTY/1s/") && !page.contains("/INDEX/"),
(F) BRANCH crates/api/src/bars_page_tests.rs:286:45 False  [the false side fails the check, and its line ran]  !page.contains("/FNO/NIFTY/1s/") && !page.contains("/INDEX/"),
(F) LINE crates/api/src/bars_route.rs:1581  [failing arm of the statement at 1575, which ran]  &field[field.len() - 80..]
(F) BRANCH crates/api/src/bars_route.rs:2530:13 False  [the false side fails the check, and its line ran]  scan_body.contains("a bar window scan not admitted")
(F) BRANCH crates/api/src/bars_route.rs:2531:20 False  [the false side fails the check, and its line ran]  && scan_body.contains(&format!("at most {MAX_SCAN_CONCURRENT} run at once")),
(F) BRANCH crates/api/src/bars_route.rs:2577:40 False  [the false side fails the check, and its line ran]  .is_some_and(|why| why.contains("not held") && why.contains("2024-01")),
(F) LINE crates/api/src/bars_series.rs:1942  [failing arm of the statement at 1939, which ran]  memo.len(),
(F) LINE crates/api/src/bars_series.rs:1943  [failing arm of the statement at 1939, which ran]  memo.turns()
(F) LINE crates/api/src/bars_series.rs:2351  [failing arm of the statement at 2348, which ran]  memo.bytes()
(F) LINE crates/api/src/bars_series.rs:2362  [failing arm of the statement at 2359, which ran]  memo.bytes()
(F) LINE crates/api/src/bars_sort.rs:2934  [failing arm of the statement at 2931, which ran]  shared.bytes()
(F) LINE crates/api/src/bars_sort.rs:4195  [failing arm of the statement at 4192, which ran]  refused.as_ref().map(|_| ()).err().map(|why| match why {
(F) BRANCH crates/api/src/bars_sort.rs:3821:13 False  [the false side fails the check, and its line ran]  why.contains("; index them with: cli srt-build ") && why.ends_with("2031-02 2031-02"),
(F) BRANCH crates/api/src/bars_sort.rs:3821:66 False  [the false side fails the check, and its line ran]  why.contains("; index them with: cli srt-build ") && why.ends_with("2031-02 2031-02"),
(F) BRANCH crates/api/src/bars_sort.rs:3895:13 False  [the false side fails the check, and its line ran]  why.starts_with("scan_budget_exceeded: the extremes need 120 records read")
(F) BRANCH crates/api/src/bars_sort.rs:3896:20 False  [the false side fails the check, and its line ran]  && why.contains("past the 119 one request may fold"),
(F) BRANCH crates/api/src/bars_sort.rs:3935:13 False  [the false side fails the check, and its line ran]  why.starts_with("scan_budget_exceeded: the extremes need 120 records read")
(F) BRANCH crates/api/src/bars_sort.rs:3936:20 False  [the false side fails the check, and its line ran]  && why.contains("past the 119 one request may fold"),
(F) BRANCH crates/api/src/bars_sort.rs:4193:69 False  [the false side fails the check, and its line ran]  matches!(&refused, Err(super::Attempt::Refused(why)) if why == "a value page needs a value column"),
(F) BRANCH crates/api/src/bars_sort.rs:4563:17 False  [the false side fails the check, and its line ran]  assert!(read[1] > 0 && read[1] < 110, "{read:?}");
(F) BRANCH crates/api/src/bars_sort.rs:4563:32 False  [the false side fails the check, and its line ran]  assert!(read[1] > 0 && read[1] < 110, "{read:?}");
(F) LINE crates/api/src/bars_sort_tests.rs:246  [failing arm of the statement at 242, which ran]  column.as_str()
(F) LINE crates/api/src/bars_sort_tests.rs:1265  [failing arm of the statement at 1261, which ran]  column.as_str()
(F) BRANCH crates/api/src/bars_sort_tests.rs:403:13 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("would not serve")
(F) BRANCH crates/api/src/bars_sort_tests.rs:404:20 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("checksum"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:590:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:591:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("small-month rule"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:594:13 False  [the false side fails the check, and its line ran]  assert!(report.scanned_why[1].contains("2031-03") && report.scanned_why[1].contains("0..260"));
(F) BRANCH crates/api/src/bars_sort_tests.rs:594:58 False  [the false side fails the check, and its line ran]  assert!(report.scanned_why[1].contains("2031-03") && report.scanned_why[1].contains("0..260"));
(F) BRANCH crates/api/src/bars_sort_tests.rs:665:13 False  [the false side fails the check, and its line ran]  fault.contains("2031-02") && fault.contains("none of its 360 kept rows"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:665:42 False  [the false side fails the check, and its line ran]  fault.contains("2031-02") && fault.contains("none of its 360 kept rows"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:702:13 False  [the false side fails the check, and its line ran]  assert!(answer.extremes.is_some() && answer.extremes_why.is_none());
(F) BRANCH crates/api/src/bars_sort_tests.rs:702:42 False  [the false side fails the check, and its line ran]  assert!(answer.extremes.is_some() && answer.extremes_why.is_none());
(F) BRANCH crates/api/src/bars_sort_tests.rs:746:13 False  [the false side fails the check, and its line ran]  assert!(report.unread_rows > 0 && report.unread_rows + report.scanned_rows == answer.total);
(F) BRANCH crates/api/src/bars_sort_tests.rs:746:39 False  [the false side fails the check, and its line ran]  assert!(report.unread_rows > 0 && report.unread_rows + report.scanned_rows == answer.total);
(F) BRANCH crates/api/src/bars_sort_tests.rs:850:9 False  [the false side fails the check, and its line ran]  why.contains("2031-03") && why.contains("is torn") && why.contains("0..260"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:850:36 False  [the false side fails the check, and its line ran]  why.contains("2031-03") && why.contains("is torn") && why.contains("0..260"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:850:63 False  [the false side fails the check, and its line ran]  why.contains("2031-03") && why.contains("is torn") && why.contains("0..260"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:908:21 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:909:28 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("failed when it was read")
(F) BRANCH crates/api/src/bars_sort_tests.rs:910:28 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("checksum"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1200:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-05")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1201:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("small-month rule"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1205:9 False  [the false side fails the check, and its line ran]  report.scanned_why[1].contains("2031-06")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1206:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[1].contains("the month has no .srt sort index")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1207:16 False  [the false side fails the check, and its line ran]  && !report.scanned_why[1].contains("small-month rule"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1469:13 False  [the false side fails the check, and its line ran]  assert!(stamps_served.contains(&lo) && stamps_served.contains(&(hi - MICROS)));
(F) BRANCH crates/api/src/bars_sort_tests.rs:1469:44 False  [the false side fails the check, and its line ran]  assert!(stamps_served.contains(&lo) && stamps_served.contains(&(hi - MICROS)));
(F) BRANCH crates/api/src/bars_sort_tests.rs:1527:26 False  [the false side fails the check, and its line ran]  .all(|fault| fault.contains("the sort index names")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1582:13 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1583:20 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("is not this bar file's"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1672:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1673:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("would not read"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1686:13 False  [the false side fails the check, and its line ran]  assert!(report.scanned_why.is_empty() && report.build.is_empty());
(F) BRANCH crates/api/src/bars_sort_tests.rs:1686:46 False  [the false side fails the check, and its line ran]  assert!(report.scanned_why.is_empty() && report.build.is_empty());
(F) BRANCH crates/api/src/bars_sort_tests.rs:1742:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1743:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("the host refused its .srt sort index")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1744:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("the next request asks again"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1786:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1787:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("symbolic link; no link is followed"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1826:9 False  [the false side fails the check, and its line ran]  answer.faults[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1827:16 False  [the false side fails the check, and its line ran]  && answer.faults[0].contains("none of its 37 rows on this page was read"),
(F) LINE crates/api/src/bars_window_route_tests.rs:113  [panic/unreachable line]  panic!("the window pool stayed full for two seconds");
(F) LINE crates/api/src/bars_window_route_tests.rs:128  [panic/unreachable line]  panic!("the store-read pool stayed full for two seconds");
(F) LINE crates/api/src/bars_window_route_tests.rs:871  [failing arm of the statement at 868, which ran]  value(a),
(F) LINE crates/api/src/bars_window_route_tests.rs:872  [failing arm of the statement at 868, which ran]  value(b)
(F) BRANCH crates/api/src/bars_window_route_tests.rs:543:9 False  [the false side fails the check, and its line ran]  withheld[0].contains("`v`") && withheld[0].contains(&past.to_string()),
(F) BRANCH crates/api/src/bars_window_route_tests.rs:543:40 False  [the false side fails the check, and its line ran]  withheld[0].contains("`v`") && withheld[0].contains(&past.to_string()),
(F) BRANCH crates/api/src/bars_window_route_tests.rs:549:9 False  [the false side fails the check, and its line ran]  none.is_empty() && edge.contains("9007199254740991"),
(F) BRANCH crates/api/src/bars_window_route_tests.rs:549:28 False  [the false side fails the check, and its line ran]  none.is_empty() && edge.contains("9007199254740991"),
(F) BRANCH crates/api/src/bars_window_route_tests.rs:1232:24 False  [the false side fails the check, and its line ran]  .all(|row| !row.oi_applies && row.oichg_why == "not_a_derivative")
(F) LINE crates/api/src/basis_route_tests.rs:181  [failing arm of the statement at 178, which ran]  body["corporate_actions"]
(F) LINE crates/api/src/basis_route_tests.rs:243  [failing arm of the statement at 240, which ran]  body["corporate_actions"]
(F) LINE crates/api/src/basis_route_tests.rs:499  [failing arm of the statement at 495, which ran]  repaired.actions.table().err()
(F) BRANCH crates/api/src/boolean_observation_budget.rs:92:21 False  [the false side fails the check, and its line ran]  assert!(why.contains(ENV) && why.contains("no default substituted"));
(F) BRANCH crates/api/src/boolean_observation_budget.rs:92:42 False  [the false side fails the check, and its line ran]  assert!(why.contains(ENV) && why.contains("no default substituted"));
(F) BRANCH crates/api/src/boolean_observation_budget.rs:109:13 False  [the false side fails the check, and its line ran]  why.contains("receipt seal differs")
(F) BRANCH crates/api/src/boolean_observation_budget.rs:110:20 False  [the false side fails the check, and its line ran]  && why.contains("123 bytes")
(F) BRANCH crates/api/src/boolean_observation_budget.rs:111:20 False  [the false side fails the check, and its line ran]  && why.contains("no body prefix")
(F) BRANCH crates/api/src/booleanevidencejson_tests.rs:165:13 False  [the false side fails the check, and its line ran]  why.contains("1610612736 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES")
(F) BRANCH crates/api/src/booleanevidencejson_tests.rs:165:49 False  [the false side fails the check, and its line ran]  why.contains("1610612736 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES")
(F) BRANCH crates/api/src/booleanevidencejson_tests.rs:167:17 False  [the false side fails the check, and its line ran]  assert!(why.contains(model.name()) && why.contains("no body prefix returned"));
(F) BRANCH crates/api/src/booleanevidencejson_tests.rs:167:47 False  [the false side fails the check, and its line ran]  assert!(why.contains(model.name()) && why.contains("no body prefix returned"));
(F) BRANCH crates/api/src/booleanjson_tests.rs:192:13 False  [the false side fails the check, and its line ran]  assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
(F) BRANCH crates/api/src/booleanjson_tests.rs:192:40 False  [the false side fails the check, and its line ran]  assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
(F) BRANCH crates/api/src/booleanoosjson_tests.rs:184:13 False  [the false side fails the check, and its line ran]  assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
(F) BRANCH crates/api/src/booleanoosjson_tests.rs:184:40 False  [the false side fails the check, and its line ran]  assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
(F) LINE crates/api/src/calendar_build.rs:822  [panic/unreachable line]  panic!("the first request leads");
(F) LINE crates/api/src/calendar_build.rs:825  [panic/unreachable line]  panic!("the second follows");
(F) LINE crates/api/src/calendar_build.rs:856  [panic/unreachable line]  panic!("the first request leads");
(F) LINE crates/api/src/calendar_build.rs:881  [panic/unreachable line]  panic!("leads");
(F) LINE crates/api/src/calendar_build.rs:904  [panic/unreachable line]  panic!("leads");
(F) LINE crates/api/src/calendar_build.rs:933  [failing arm of the statement at 932, which ran]  matches!(
(F) BRANCH crates/api/src/calendar_build.rs:821:13 False  [the else of a let-else that panics, whose line ran]  let Joined::Lead(lead) = builds.join(&key("NIFTY"), None) else {
(F) BRANCH crates/api/src/calendar_build.rs:824:13 False  [the else of a let-else that panics, whose line ran]  let Joined::Follow(follow) = builds.join(&key("NIFTY"), None) else {
(F) BRANCH crates/api/src/calendar_build.rs:855:13 False  [the else of a let-else that panics, whose line ran]  let Joined::Lead(lead) = builds.join(&key("NIFTY"), Some(&stamp)) else {
(F) BRANCH crates/api/src/calendar_build.rs:880:13 False  [the else of a let-else that panics, whose line ran]  let Joined::Lead(second) = builds.join(&key("BANKNIFTY"), Some(&stamp)) else {
(F) BRANCH crates/api/src/calendar_build.rs:903:13 False  [the else of a let-else that panics, whose line ran]  let Joined::Lead(lead) = builds.join(&key("NIFTY"), None) else {
(F) LINE crates/api/src/calendar_of.rs:1788  [failing arm of the statement at 1784, which ran]  24 * 48
(F) LINE crates/api/src/calendar_of.rs:3032  [failing arm of the statement at 3029, which ran]  json(&exchange)
(F) BRANCH crates/api/src/calendar_of.rs:2756:17 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) BRANCH crates/api/src/calendar_of.rs:2756:42 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) BRANCH crates/api/src/calendar_of.rs:3168:17 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) BRANCH crates/api/src/calendar_of.rs:3168:42 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) LINE crates/api/src/calendar_record_tests.rs:78  [failing arm of the statement at 75, which ran]  iso(day)
(F) LINE crates/api/src/calendar_record_tests.rs:551  [failing arm of the statement at 544, which ran]  widened["special"]
(F) BRANCH crates/api/src/calendar_record_tests.rs:172:13 False  [the false side fails the check, and its line ran]  assert!(!minutes.contains(&562) && !minutes.contains(&620));
(F) BRANCH crates/api/src/calendar_record_tests.rs:172:40 False  [the false side fails the check, and its line ran]  assert!(!minutes.contains(&562) && !minutes.contains(&620));
(F) BRANCH crates/api/src/calendar_record_tests.rs:173:13 False  [the false side fails the check, and its line ran]  assert!(minutes.contains(&561) && minutes.contains(&621));
(F) BRANCH crates/api/src/calendar_record_tests.rs:173:39 False  [the false side fails the check, and its line ran]  assert!(minutes.contains(&561) && minutes.contains(&621));
(F) BRANCH crates/api/src/calendar_record_tests.rs:412:32 False  [the false side fails the check, and its line ran]  .is_some_and(|why| why.contains("1day") && why.contains("NIFTY")),
(F) LINE crates/api/src/calendar_route_tests.rs:663  [panic/unreachable line]  panic!("{query}: still building after 1,000 asks");
(F) BRANCH crates/api/src/calendar_route_tests.rs:326:17 False  [the false side fails the check, and its line ran]  assert!(reason.contains(rung) && reason.contains("1s"), "{reason}");
(F) BRANCH crates/api/src/calendar_route_tests.rs:326:42 False  [the false side fails the check, and its line ran]  assert!(reason.contains(rung) && reason.contains("1s"), "{reason}");
(F) BRANCH crates/api/src/calendar_route_tests.rs:496:9 False  [the false side fails the check, and its line ran]  spanned.contains("calendar_json_held(site, uri).await")
(F) BRANCH crates/api/src/calendar_route_tests.rs:497:16 False  [the false side fails the check, and its line ran]  && !spanned.contains("census")
(F) BRANCH crates/api/src/calendar_route_tests.rs:498:16 False  [the false side fails the check, and its line ran]  && !spanned.contains("lead("),
(F) BRANCH crates/api/src/calendar_route_tests.rs:503:9 False  [the false side fails the check, and its line ran]  calendar.contains("lead(&site, uri, &asked, &build, now)")
(F) BRANCH crates/api/src/calendar_route_tests.rs:504:16 False  [the false side fails the check, and its line ran]  && !calendar.contains("calendar_json_reading(")
(F) BRANCH crates/api/src/calendar_route_tests.rs:505:16 False  [the false side fails the check, and its line ran]  && !calendar.contains("census_now_stamped"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:510:9 False  [the false side fails the check, and its line ran]  lead.contains("crate::detail::run_calendar(move ||")
(F) BRANCH crates/api/src/calendar_route_tests.rs:511:16 False  [the false side fails the check, and its line ran]  && lead.contains("calendar_json_reading(&site, &uri, census_now_stamped)"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:518:9 False  [the false side fails the check, and its line ran]  gaps.contains("voted(&site, &asked).await")
(F) BRANCH crates/api/src/calendar_route_tests.rs:519:16 False  [the false side fails the check, and its line ran]  && !gaps.contains("peer_calendar(")
(F) BRANCH crates/api/src/calendar_route_tests.rs:520:16 False  [the false side fails the check, and its line ran]  && !gaps.contains("peer_vote_reading("),
(F) BRANCH crates/api/src/calendar_route_tests.rs:540:9 False  [the false side fails the check, and its line ran]  body.contains("Saturated") && body.contains("at most 8"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:540:39 False  [the false side fails the check, and its line ran]  body.contains("Saturated") && body.contains("at most 8"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:614:9 False  [the false side fails the check, and its line ran]  !map.contains("Published::read") && !map.contains("masters_dir()"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:614:45 False  [the false side fails the check, and its line ran]  !map.contains("Published::read") && !map.contains("masters_dir()"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:887:13 False  [the false side fails the check, and its line ran]  assert!(feeds.contains(&Vendor::Gdfl) && feeds.contains(&Vendor::Zerodha));
(F) BRANCH crates/api/src/calendar_route_tests.rs:887:46 False  [the false side fails the check, and its line ran]  assert!(feeds.contains(&Vendor::Gdfl) && feeds.contains(&Vendor::Zerodha));
(F) BRANCH crates/api/src/calendar_route_tests.rs:934:9 False  [the false side fails the check, and its line ran]  why.contains(&crate::calendar_of::build::MAX_BUILDS.to_string()) && why.contains("retry"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:934:77 False  [the false side fails the check, and its line ran]  why.contains(&crate::calendar_of::build::MAX_BUILDS.to_string()) && why.contains("retry"),
(F) BRANCH crates/api/src/catalog.rs:918:17 False  [the false side fails the check, and its line ran]  assert!(Pill::Fno.admits(Universe::FNO) && !Pill::Fno.admits(Universe::INDEX));
(F) BRANCH crates/api/src/catalog.rs:918:52 False  [the false side fails the check, and its line ran]  assert!(Pill::Fno.admits(Universe::FNO) && !Pill::Fno.admits(Universe::INDEX));
(F) BRANCH crates/api/src/catalog.rs:919:17 False  [the false side fails the check, and its line ran]  assert!(Pill::Ntm.admits(Universe::TOTAL_MARKET) && !Pill::Ntm.admits(Universe::FNO));
(F) BRANCH crates/api/src/catalog.rs:919:61 False  [the false side fails the check, and its line ran]  assert!(Pill::Ntm.admits(Universe::TOTAL_MARKET) && !Pill::Ntm.admits(Universe::FNO));
(F) BRANCH crates/api/src/catalog.rs:920:17 False  [the false side fails the check, and its line ran]  assert!(Pill::Index.admits(Universe::INDEX) && !Pill::Index.admits(Universe::NONE));
(F) BRANCH crates/api/src/catalog.rs:920:56 False  [the false side fails the check, and its line ran]  assert!(Pill::Index.admits(Universe::INDEX) && !Pill::Index.admits(Universe::NONE));
(F) LINE crates/api/src/census.rs:2646  [failing arm of the statement at 2643, which ran]  got.note()
(F) LINE crates/api/src/census.rs:3189  [failing arm of the statement at 3184, which ran]  grid.len()
(F) BRANCH crates/api/src/census.rs:2764:13 False  [the false side fails the check, and its line ran]  note.contains(&(MAX_MANIFEST_BYTES + 1).to_string())
(F) BRANCH crates/api/src/census.rs:2765:20 False  [the false side fails the check, and its line ran]  && note.contains(&MAX_MANIFEST_BYTES.to_string()),
(F) BRANCH crates/api/src/census.rs:2824:17 False  [the false side fails the check, and its line ran]  why.contains("(char_device)")
(F) BRANCH crates/api/src/census.rs:2825:24 False  [the false side fails the check, and its line ran]  && why.contains("a character device, not a regular file"),
(F) BRANCH crates/api/src/census.rs:2836:17 False  [the false side fails the check, and its line ran]  why.contains("(symlink)") && why.contains("a symbolic link, not a regular file"),
(F) BRANCH crates/api/src/census.rs:2836:46 False  [the false side fails the check, and its line ran]  why.contains("(symlink)") && why.contains("a symbolic link, not a regular file"),
(F) BRANCH crates/api/src/census.rs:2841:21 False  [the false side fails the check, and its line ran]  assert!(census.is_loud() && census.note().contains("not a regular file"));
(F) BRANCH crates/api/src/census.rs:2841:41 False  [the false side fails the check, and its line ran]  assert!(census.is_loud() && census.note().contains("not a regular file"));
(F) BRANCH crates/api/src/census.rs:3193:26 False  [the false side fails the check, and its line ran]  rows == 0 || filled > 0,
(F) LINE crates/api/src/census_epoch_link_tests.rs:92  [panic/unreachable line]  panic!("a link must be refused, never read: {}", census.note());
(F) LINE crates/api/src/census_epoch_link_tests.rs:104  [failing arm of the statement at 100, which ran]  path.display()
(F) LINE crates/api/src/census_epoch_link_tests.rs:136  [failing arm of the statement at 133, which ran]  census.note()
(F) LINE crates/api/src/census_epoch_link_tests.rs:231  [failing arm of the statement at 228, which ran]  census.note()
(F) LINE crates/api/src/census_epoch_link_tests.rs:294  [panic/unreachable line]  panic!("a link must be refused, never read: {}", census.note());
(F) LINE crates/api/src/census_epoch_link_tests.rs:323  [failing arm of the statement at 320, which ran]  census.note()
(F) BRANCH crates/api/src/census_epoch_link_tests.rs:91:9 False  [the else of a let-else that panics, whose line ran]  let Census::Unreadable { reason, fault } = &census.state else {
(F) BRANCH crates/api/src/census_epoch_link_tests.rs:293:9 False  [the else of a let-else that panics, whose line ran]  let Census::Unreadable { reason, fault } = &census.state else {
(F) LINE crates/api/src/census_request_tests.rs:1801  [failing arm of the statement at 1800, which ran]  matches!(
(F) LINE crates/api/src/census_request_tests.rs:1814  [failing arm of the statement at 1811, which ran]  dhan.note()
(F) LINE crates/api/src/census_request_tests.rs:1980  [failing arm of the statement at 1977, which ran]  at_path.file_type()
(F) LINE crates/api/src/census_request_tests.rs:2032  [failing arm of the statement at 2031, which ran]  matches!(
(F) LINE crates/api/src/census_request_tests.rs:2679  [failing arm of the statement at 2675, which ran]  fixture.fresh_states()
(F) LINE crates/api/src/census_request_tests.rs:3242  [failing arm of the statement at 3239, which ran]  partial.calendar.map(|calendar| calendar.sessions())
(F) LINE crates/api/src/census_request_tests.rs:4017  [panic/unreachable line]  panic!("the census is read only for an NSE spot or cash one-minute window");
(F) BRANCH crates/api/src/census_request_tests.rs:326:81 False  [the false side fails the check, and its line ran]  || matches!(located, Err(Unlocated::NotHeld(ref unreadable)) if unreadable.is_empty()),
(F) BRANCH crates/api/src/census_request_tests.rs:996:9 False  [the false side fails the check, and its line ran]  page.contains("NSE/CASH") && page.contains("NSE/INDEX"),
(F) BRANCH crates/api/src/census_request_tests.rs:996:38 False  [the false side fails the check, and its line ran]  page.contains("NSE/CASH") && page.contains("NSE/INDEX"),
(F) BRANCH crates/api/src/census_request_tests.rs:1064:9 False  [the false side fails the check, and its line ran]  exchange.contains(r#""target":{"kind":"exchange"}"#)
(F) BRANCH crates/api/src/census_request_tests.rs:1065:16 False  [the false side fails the check, and its line ran]  && exchange.contains(r#""derivedFrom":[]"#),
(F) BRANCH crates/api/src/census_request_tests.rs:1419:13 False  [the false side fails the check, and its line ran]  page.contains("DHAN · CASH · 2025-05") && page.contains("<b>1</b> bar(s)"),
(F) BRANCH crates/api/src/census_request_tests.rs:1419:57 False  [the false side fails the check, and its line ran]  page.contains("DHAN · CASH · 2025-05") && page.contains("<b>1</b> bar(s)"),
(F) BRANCH crates/api/src/census_request_tests.rs:1971:42 False  [the false side fails the check, and its line ran]  matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
(F) BRANCH crates/api/src/census_request_tests.rs:2226:73 False  [the false side fails the check, and its line ran]  matches!(state, census::Census::Unreadable { fault, .. } if fault == from),
(F) BRANCH crates/api/src/census_request_tests.rs:2831:31 False  [the false side fails the check, and its line ran]  .all(|record| record.level == telemetry::Level::Warn
(F) BRANCH crates/api/src/census_request_tests.rs:3065:9 False  [the false side fails the check, and its line ran]  report.contains(concat!("eprintln", "!"))
(F) BRANCH crates/api/src/census_request_tests.rs:3066:16 False  [the false side fails the check, and its line ran]  && report.contains(".compare_exchange(false, true,"),
(F) BRANCH crates/api/src/census_request_tests.rs:3166:13 False  [the false side fails the check, and its line ran]  body.contains("NIFTY") && body.contains("2025-05"),
(F) BRANCH crates/api/src/census_request_tests.rs:3166:39 False  [the false side fails the check, and its line ran]  body.contains("NIFTY") && body.contains("2025-05"),
(F) BRANCH crates/api/src/census_request_tests.rs:3681:17 False  [the false side fails the check, and its line ran]  raced.starts_with("503 ")
(F) BRANCH crates/api/src/census_request_tests.rs:3682:24 False  [the false side fails the check, and its line ran]  && raced.contains(r#""census":"unreadable""#)
(F) BRANCH crates/api/src/census_request_tests.rs:3683:24 False  [the false side fails the check, and its line ran]  && raced.contains(CONTRADICTED_ABSENCE),
(F) BRANCH crates/api/src/census_tail_tests.rs:574:13 False  [the false side fails the check, and its line ran]  assert!(why.contains("capped") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/census_tail_tests.rs:574:39 False  [the false side fails the check, and its line ran]  assert!(why.contains("capped") && why.contains("FNO"), "{why}");
(F) LINE crates/api/src/census_warm_tests.rs:197  [panic/unreachable line]  panic!("not held: {}", census.note());
(F) LINE crates/api/src/census_warm_tests.rs:428  [failing arm of the statement at 425, which ran]  took[499]
(F) LINE crates/api/src/census_warm_tests.rs:440  [failing arm of the statement at 435, which ran]  &reads[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:687  [failing arm of the statement at 683, which ran]  &store.reads()[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:702  [failing arm of the statement at 697, which ran]  &reads[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:823  [failing arm of the statement at 818, which ran]  &reads[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:983  [failing arm of the statement at 978, which ran]  &reads[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:1499  [failing arm of the statement at 1496, which ran]  began.elapsed()
(F) LINE crates/api/src/census_warm_tests.rs:1516  [failing arm of the statement at 1513, which ran]  began.elapsed()
(F) LINE crates/api/src/census_warm_tests.rs:1728  [failing arm of the statement at 1725, which ran]  began.elapsed()
(F) LINE crates/api/src/census_warm_tests.rs:1799  [failing arm of the statement at 1796, which ran]  began.elapsed()
(F) BRANCH crates/api/src/census_warm_tests.rs:196:9 False  [the else of a let-else that panics, whose line ran]  let census::Census::Held { ref manifest } = census.state else {
(F) BRANCH crates/api/src/census_warm_tests.rs:785:41 False  [the false side fails the check, and its line ran]  .all(|(path, kind, thread)| *kind == "whole"
(F) BRANCH crates/api/src/census_warm_tests.rs:786:20 False  [the false side fails the check, and its line ran]  && path.starts_with(Path::new(&store.root))
(F) BRANCH crates/api/src/census_warm_tests.rs:981:38 False  [the false side fails the check, and its line ran]  .all(|(_, kind, thread)| *kind != "whole" && is_census_thread(thread)),
(F) BRANCH crates/api/src/census_warm_tests.rs:1032:13 False  [the false side fails the check, and its line ran]  assert!(whole["underlying"].is_null() && whole["exchange"].is_null());
(F) BRANCH crates/api/src/census_warm_tests.rs:1032:46 False  [the false side fails the check, and its line ran]  assert!(whole["underlying"].is_null() && whole["exchange"].is_null());
(F) BRANCH crates/api/src/chain_route_tests.rs:423:17 False  [the false side fails the check, and its line ran]  assert!(chain["atm"].is_null() && chain["pick"]["rung"].is_null());
(F) BRANCH crates/api/src/chain_route_tests.rs:423:43 False  [the false side fails the check, and its line ran]  assert!(chain["atm"].is_null() && chain["pick"]["rung"].is_null());
(F) BRANCH crates/api/src/chain_route_tests.rs:440:13 False  [the false side fails the check, and its line ran]  assert!(chain["spot"].is_null() && chain["spot_t"].is_null());
(F) BRANCH crates/api/src/chain_route_tests.rs:440:40 False  [the false side fails the check, and its line ran]  assert!(chain["spot"].is_null() && chain["spot_t"].is_null());
(F) BRANCH crates/api/src/constituents.rs:1732:17 False  [the false side fails the check, and its line ran]  assert!(head.contains("matched") && head.contains("vendor lacks"));
(F) BRANCH crates/api/src/constituents.rs:1732:45 False  [the false side fails the check, and its line ran]  assert!(head.contains("matched") && head.contains("vendor lacks"));
(F) BRANCH crates/api/src/constituents.rs:1733:17 False  [the false side fails the check, and its line ran]  assert!(head.contains("ambiguous") && head.contains("malformed"));
(F) BRANCH crates/api/src/constituents.rs:1733:47 False  [the false side fails the check, and its line ran]  assert!(head.contains("ambiguous") && head.contains("malformed"));
(F) BRANCH crates/api/src/constituents.rs:1785:13 False  [the false side fails the check, and its line ran]  quiet_head.contains("1 matched") && quiet_head.contains("50 of 50 published"),
(F) BRANCH crates/api/src/constituents.rs:1785:49 False  [the false side fails the check, and its line ran]  quiet_head.contains("1 matched") && quiet_head.contains("50 of 50 published"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:108:9 False  [the false side fails the check, and its line ran]  !inside.contains("ingestion_observations") && inside.contains("land_bodies_observed("),
(F) BRANCH crates/api/src/cost_limits_tests.rs:108:55 False  [the false side fails the check, and its line ran]  !inside.contains("ingestion_observations") && inside.contains("land_bodies_observed("),
(F) BRANCH crates/api/src/cost_limits_tests.rs:149:13 False  [the false side fails the check, and its line ran]  assert!(append < hook && repair < hook, "{body}");
(F) BRANCH crates/api/src/cost_limits_tests.rs:149:30 False  [the false side fails the check, and its line ran]  assert!(append < hook && repair < hook, "{body}");
(F) BRANCH crates/api/src/cost_limits_tests.rs:156:9 False  [the false side fails the check, and its line ran]  walk[hook..].contains("(&landing, door)") && walk.contains("done.absorb(landing.ingested)"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:156:54 False  [the false side fails the check, and its line ran]  walk[hook..].contains("(&landing, door)") && walk.contains("done.absorb(landing.ingested)"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:160:9 False  [the false side fails the check, and its line ran]  item(INGEST, "pub fn from_window(").contains("from_window_with_refusals(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:161:16 False  [the false side fails the check, and its line ran]  && item(INGEST, "pub fn repair_window(").contains("repair_window_with_refusals("),
(F) BRANCH crates/api/src/cost_limits_tests.rs:177:9 False  [the false side fails the check, and its line ran]  item(CENSUS_STORE, "pub fn cycle(").contains("cycle_with(target, held, mode, None)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:178:16 False  [the false side fails the check, and its line ran]  && item(CENSUS_STORE, "pub fn record(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:180:16 False  [the false side fails the check, and its line ran]  && item(CENSUS_STORE, "fn cycle_with(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:189:9 False  [the false side fails the check, and its line ran]  load.contains("read_single(&path, self.vendor)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:190:16 False  [the false side fails the check, and its line ran]  && load.contains("read_epoch(&path, self.vendor, segment)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:191:16 False  [the false side fails the check, and its line ran]  && item(CENSUS_STORE, "fn read_single(").contains("read_census(path, vendor)"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:229:9 False  [the false side fails the check, and its line ran]  miss.contains("let (mut censuses, kept_under) = read(&site.store_root);")
(F) BRANCH crates/api/src/cost_limits_tests.rs:230:16 False  [the false side fails the check, and its line ran]  && item(CENSUS_CACHE, "pub(super) fn census_now_stamping(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:235:9 False  [the false side fails the check, and its line ran]  miss.contains("HeldEntries::of(&censuses)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:236:16 False  [the false side fails the check, and its line ran]  && !miss.contains("census::held_entries(&censuses)"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:248:9 False  [the false side fails the check, and its line ran]  item(CENSUS_CACHE, "fn vendor_slot_fresh(").contains("rebuild(&site.store_root, cell,")
(F) BRANCH crates/api/src/cost_limits_tests.rs:249:16 False  [the false side fails the check, and its line ran]  && item(CENSUS_CACHE, "fn rebuild(").contains("let census = fresh_read(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:250:16 False  [the false side fails the check, and its line ran]  && item(CENSUS_CACHE, "fn fresh_read(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:291:9 False  [the false side fails the check, and its line ran]  answer.contains("site.census_wire.kept(") && answer.contains("universe.generation"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:291:54 False  [the false side fails the check, and its line ran]  answer.contains("site.census_wire.kept(") && answer.contains("universe.generation"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:342:9 False  [the false side fails the check, and its line ran]  window.contains("sort::page(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:343:16 False  [the false side fails the check, and its line ran]  && window.contains("scan(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:346:16 False  [the false side fails the check, and its line ran]  && scan.contains("plan.read_stretch(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:347:16 False  [the false side fails the check, and its line ran]  && scan.contains("(run.start, run.end),"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:359:9 False  [the false side fails the check, and its line ran]  !window.contains("all.sort_by(order)") && !scan.contains("want - 1"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:359:51 False  [the false side fails the check, and its line ran]  !window.contains("all.sort_by(order)") && !scan.contains("want - 1"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:393:9 False  [the false side fails the check, and its line ran]  walk.contains("manifest.held_keys()") && !walk.contains("held_entries"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:393:50 False  [the false side fails the check, and its line ran]  walk.contains("manifest.held_keys()") && !walk.contains("held_entries"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:479:9 False  [the false side fails the check, and its line ran]  before.contains("file.first_at_or_after(at)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:480:16 False  [the false side fails the check, and its line ran]  && after.contains("file.read_record(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:481:16 False  [the false side fails the check, and its line ran]  && after.contains(r#"return Ok((axum::http::StatusCode::OK, "[]".to_owned()));"#),
(F) BRANCH crates/api/src/cost_limits_tests.rs:490:9 False  [the false side fails the check, and its line ran]  decision.contains("held.checked_sub(1).filter(|_| landed >= held)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:491:16 False  [the false side fails the check, and its line ran]  && decision.contains("stamp != 0 && stamp == header_last_ts && stamp < from"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:560:9 False  [the false side fails the check, and its line ran]  SERVER.contains("// D-0136.") && SERVER.contains("THE TARGET GUARD IS GONE"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:560:42 False  [the false side fails the check, and its line ran]  SERVER.contains("// D-0136.") && SERVER.contains("THE TARGET GUARD IS GONE"),
(F) BRANCH crates/api/src/coverage.rs:840:13 False  [the false side fails the check, and its line ran]  why.contains("900001") && why.contains("no ISIN column"),
(F) BRANCH crates/api/src/coverage.rs:840:39 False  [the false side fails the check, and its line ran]  why.contains("900001") && why.contains("no ISIN column"),
(F) BRANCH crates/api/src/coverage.rs:1206:17 False  [the false side fails the check, and its line ran]  line.contains("not pull readiness")
(F) BRANCH crates/api/src/coverage.rs:1207:24 False  [the false side fails the check, and its line ran]  && line.contains("does not silently skip")
(F) BRANCH crates/api/src/coverage.rs:1208:24 False  [the false side fails the check, and its line ran]  && line.contains("non-ISIN"),
(F) LINE crates/api/src/credential_law_tests.rs:193  [panic/unreachable line]  panic!("a same-value verdict halts the feed");
(F) LINE crates/api/src/credential_law_tests.rs:392  [panic/unreachable line]  panic!("an unusable configuration halts");
(F) LINE crates/api/src/credential_law_tests.rs:445  [panic/unreachable line]  panic!("Dhan is HTTP");
(F) LINE crates/api/src/credential_law_tests.rs:508  [failing arm of the statement at 503, which ran]  vendor.seen()
(F) LINE crates/api/src/credential_law_tests.rs:633  [panic/unreachable line]  panic!("Groww is HTTP");
(F) BRANCH crates/api/src/credential_law_tests.rs:192:9 False  [the else of a let-else that panics, whose line ran]  let autopilot::Next::Halt { reason } = state.observe(&outcome) else {
(F) BRANCH crates/api/src/credential_law_tests.rs:227:13 False  [the false side fails the check, and its line ran]  assert!(seen[0].contains("stale") && !seen[0].contains("fresh"));
(F) BRANCH crates/api/src/credential_law_tests.rs:227:42 False  [the false side fails the check, and its line ran]  assert!(seen[0].contains("stale") && !seen[0].contains("fresh"));
(F) BRANCH crates/api/src/credential_law_tests.rs:229:9 False  [the false side fails the check, and its line ran]  seen[1].contains("fresh") && !seen[1].contains("stale"),
(F) BRANCH crates/api/src/credential_law_tests.rs:229:38 False  [the false side fails the check, and its line ran]  seen[1].contains("fresh") && !seen[1].contains("stale"),
(F) BRANCH crates/api/src/credential_law_tests.rs:286:28 False  [the false side fails the check, and its line ran]  .all(|carried| carried.contains("fresh") && !carried.contains("stale")),
(F) BRANCH crates/api/src/credential_law_tests.rs:444:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/credential_law_tests.rs:514:9 False  [the false side fails the check, and its line ran]  said.contains("same value") && said.contains("Dhan"),
(F) BRANCH crates/api/src/credential_law_tests.rs:514:40 False  [the false side fails the check, and its line ran]  said.contains("same value") && said.contains("Dhan"),
(F) BRANCH crates/api/src/credential_law_tests.rs:584:9 False  [the false side fails the check, and its line ran]  seen[0].contains("stale") && seen[1].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:584:38 False  [the false side fails the check, and its line ran]  seen[0].contains("stale") && seen[1].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:632:9 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/credential_law_tests.rs:700:9 False  [the false side fails the check, and its line ran]  seen[1].contains("fresh") && seen[2].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:700:38 False  [the false side fails the check, and its line ran]  seen[1].contains("fresh") && seen[2].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:720:9 False  [the false side fails the check, and its line ran]  seen[0].contains("stale") && seen[1].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:720:38 False  [the false side fails the check, and its line ran]  seen[0].contains("stale") && seen[1].contains("fresh"),
(F) LINE crates/api/src/dayflags.rs:7120  [failing arm of the statement at 7115, which ran]  none["lineage"]
(F) LINE crates/api/src/dayflags.rs:8478  [failing arm of the statement at 8473, which ran]  says(muhurat)
(F) LINE crates/api/src/dayflags.rs:8520  [failing arm of the statement at 8515, which ran]  says(nothing)
(F) LINE crates/api/src/dayflags.rs:8950  [failing arm of the statement at 8947, which ran]  got["sources"]
(F) LINE crates/api/src/dayflags.rs:8962  [failing arm of the statement at 8953, which ran]  got["notes"]
(F) LINE crates/api/src/dayflags.rs:9864  [failing arm of the statement at 9861, which ran]  sampled_rule_held()
(F) BRANCH crates/api/src/dayflags.rs:6972:13 False  [the false side fails the check, and its line ran]  phases.contains(&"halted") && phases.contains(&"reopening_auction"),
(F) BRANCH crates/api/src/dayflags.rs:6972:43 False  [the false side fails the check, and its line ran]  phases.contains(&"halted") && phases.contains(&"reopening_auction"),
(F) BRANCH crates/api/src/dayflags.rs:8556:58 False  [the false side fails the check, and its line ran]  matches!(first.via, Via::Walk { ref why } if why.contains("no index")),
(F) BRANCH crates/api/src/dayflags.rs:8715:42 False  [the false side fails the check, and its line ran]  Via::Walk { ref why } if why.contains("1 of the index's entries cannot be read")
(F) BRANCH crates/api/src/dayflags.rs:8787:42 False  [the false side fails the check, and its line ran]  Via::Walk { ref why } if why.contains("is not a refusal entry")
(F) BRANCH crates/api/src/dayflags.rs:8788:24 False  [the false side fails the check, and its line ran]  && why.contains("ZZOIL.idx")
(F) BRANCH crates/api/src/dayflags.rs:8847:42 False  [the false side fails the check, and its line ran]  Via::Walk { ref why } if why.contains("covers 4 records and the journal holds 2")
(F) BRANCH crates/api/src/dayflags.rs:9706:26 False  [the false side fails the check, and its line ran]  .all(|s| !s.starts_with("the import placed")
(F) LINE crates/api/src/dayflags_filter_tests.rs:144  [failing arm of the statement at 139, which ran]  says(&record)
(F) LINE crates/api/src/dayflags_filter_tests.rs:248  [failing arm of the statement at 243, which ran]  says(&bank)
(F) LINE crates/api/src/dayflags_filter_tests.rs:435  [failing arm of the statement at 430, which ran]  says(&put)
(F) LINE crates/api/src/dayflags_filter_tests.rs:594  [failing arm of the statement at 589, which ran]  says(&bank)
(F) LINE crates/api/src/dayflags_filter_tests.rs:869  [failing arm of the statement at 864, which ran]  says(&abcapital)
(F) LINE crates/api/src/dayflags_rules_tests.rs:178  [failing arm of the statement at 173, which ran]  says(shown)
(F) LINE crates/api/src/dayflags_rules_tests.rs:577  [failing arm of the statement at 574, which ran]  got[6]
(F) LINE crates/api/src/dayflags_rules_tests.rs:796  [failing arm of the statement at 791, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:862  [failing arm of the statement at 857, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:970  [failing arm of the statement at 964, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:1014  [failing arm of the statement at 1009, which ran]  sidecars.refusals()
(F) LINE crates/api/src/dayflags_rules_tests.rs:1028  [failing arm of the statement at 1022, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:1052  [failing arm of the statement at 1046, which ran]  sidecars.refusals()
(F) LINE crates/api/src/dayflags_rules_tests.rs:1097  [failing arm of the statement at 1092, which ran]  says(after)
(F) LINE crates/api/src/dayflags_rules_tests.rs:1370  [failing arm of the statement at 1365, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:1533  [failing arm of the statement at 1525, which ran]  says(record(&got, "2024-10-03"))
(F) LINE crates/api/src/dayflags_rules_tests.rs:1579  [failing arm of the statement at 1574, which ran]  says(unheld)
(F) LINE crates/api/src/dayflags_rules_tests.rs:2354  [failing arm of the statement at 2348, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:2673  [failing arm of the statement at 2669, which ran]  TIMED[1]
(F) LINE crates/api/src/dayflags_rules_tests.rs:3174  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3202  [panic/unreachable line]  panic!("the month opens again");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3387  [failing arm of the statement at 3382, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:3479  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3649  [failing arm of the statement at 3644, which ran]  says(record(&got, "2024-11-09"))
(F) LINE crates/api/src/dayflags_rules_tests.rs:3728  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3772  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3877  [failing arm of the statement at 3874, which ran]  says(closed)
(F) LINE crates/api/src/dayflags_rules_tests.rs:3964  [failing arm of the statement at 3959, which ran]  says(after)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4068  [failing arm of the statement at 4065, which ran]  says(quiet)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4298  [failing arm of the statement at 4293, which ran]  super::super::tests::says(unstamped)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4307  [failing arm of the statement at 4302, which ran]  super::super::tests::says(short)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4585  [failing arm of the statement at 4582, which ran]  says(moving)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4600  [failing arm of the statement at 4593, which ran]  says(both)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4649  [failing arm of the statement at 4646, which ran]  says(aside)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4985  [failing arm of the statement at 4982, which ran]  says(empty)
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:98:9 False  [the false side fails the check, and its line ran]  why.starts_with("timeframe=\"7min\" is not a rung this store files: one of ")
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:99:16 False  [the false side fails the check, and its line ran]  && why.contains("1day")
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:100:16 False  [the false side fails the check, and its line ran]  && why.contains("1s"),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:317:56 False  [the false side fails the check, and its line ran]  matches!(&flag.basis, CasBasis::Unread(why) if why.starts_with("2026-08-04: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:405:58 False  [the false side fails the check, and its line ran]  matches!(&got[2].basis, CasBasis::Unread(why) if why.starts_with("2026-08-10: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:478:25 False  [the false side fails the check, and its line ran]  .all(|flag| flag.eligibility == CasEligibility::Eligible
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:485:13 False  [the false side fails the check, and its line ran]  assert!(kept && held >= 1);
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:485:21 False  [the false side fails the check, and its line ran]  assert!(kept && held >= 1);
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:505:58 False  [the false side fails the check, and its line ran]  matches!(&absent.basis, CasBasis::Unread(why) if why.starts_with("2026-08-11: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:520:59 False  [the false side fails the check, and its line ran]  matches!(&outside.basis, CasBasis::Unread(why) if why.starts_with("2026-08-11: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:575:58 False  [the false side fails the check, and its line ran]  matches!(&got[6].basis, CasBasis::Unread(why) if why.starts_with("2026-08-25: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:967:28 False  [the false side fails the check, and its line ran]  .all(|say| !say.contains("its print is not kept apart")
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:1025:24 False  [the false side fails the check, and its line ran]  .all(|say| !say.contains("holds no auction sidecar")
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:1095:25 False  [the false side fails the check, and its line ran]  .all(|said| !said.starts_with("no GDFL") && !said.contains("holds no file")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3196:13 False  [the false side fails the check, and its line ran]  assert!(whole.is_empty() && whole_faults.is_empty(), "{whole:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3196:33 False  [the false side fails the check, and its line ran]  assert!(whole.is_empty() && whole_faults.is_empty(), "{whole:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3247:13 False  [the false side fails the check, and its line ran]  assert!(quiet.is_empty() && quiet_faults.is_empty(), "{quiet:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3247:33 False  [the false side fails the check, and its line ran]  assert!(quiet.is_empty() && quiet_faults.is_empty(), "{quiet:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:5089:13 False  [the false side fails the check, and its line ran]  assert!(kept_flag(&a, on, "RELIANCE").0 && kept_flag(&b, on, "RELIANCE").0);
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:5089:48 False  [the false side fails the check, and its line ran]  assert!(kept_flag(&a, on, "RELIANCE").0 && kept_flag(&b, on, "RELIANCE").0);
(F) LINE crates/api/src/detail.rs:1978  [failing arm of the statement at 1975, which ran]  pool.name()
(F) LINE crates/api/src/detail.rs:2147  [failing arm of the statement at 2144, which ran]  started.elapsed()
(F) BRANCH crates/api/src/detail.rs:1976:17 False  [the false side fails the check, and its line ran]  pool.max() >= 1 && pool.max() <= cores.max(1),
(F) BRANCH crates/api/src/detail.rs:1976:36 False  [the false side fails the check, and its line ran]  pool.max() >= 1 && pool.max() <= cores.max(1),
(F) BRANCH crates/api/src/detail.rs:2286:17 False  [the false side fails the check, and its line ran]  why.contains(field) && why.contains("canonical"),
(F) BRANCH crates/api/src/detail.rs:2286:40 False  [the false side fails the check, and its line ran]  why.contains(field) && why.contains("canonical"),
(F) BRANCH crates/api/src/detail.rs:2646:17 False  [the false side fails the check, and its line ran]  assert!(!held.is_empty() && held.len() <= super::MAX_OPS_READ_CONCURRENT);
(F) BRANCH crates/api/src/detail.rs:2646:37 False  [the false side fails the check, and its line ran]  assert!(!held.is_empty() && held.len() <= super::MAX_OPS_READ_CONCURRENT);
(F) LINE crates/api/src/disk_route.rs:1665  [failing arm of the statement at 1660, which ran]  dir.display()
(F) LINE crates/api/src/disk_route.rs:1834  [failing arm of the statement at 1831, which ran]  root.display(),
(F) LINE crates/api/src/disk_route.rs:1855  [failing arm of the statement at 1851, which ran]  root.display()
(F) BRANCH crates/api/src/disk_route.rs:1485:17 False  [the false side fails the check, and its line ran]  why.contains(said) && why.contains("ops/backups.log"),
(F) BRANCH crates/api/src/disk_route.rs:1485:39 False  [the false side fails the check, and its line ran]  why.contains(said) && why.contains("ops/backups.log"),
(F) BRANCH crates/api/src/disk_route.rs:1567:17 False  [the false side fails the check, and its line ran]  why.contains("ops/backups.log") && why.contains("cannot be told"),
(F) BRANCH crates/api/src/disk_route.rs:1567:52 False  [the false side fails the check, and its line ran]  why.contains("ops/backups.log") && why.contains("cannot be told"),
(F) BRANCH crates/api/src/disk_route.rs:1739:17 False  [the false side fails the check, and its line ran]  why.starts_with(
(F) BRANCH crates/api/src/disk_route.rs:1741:22 False  [the false side fails the check, and its line ran]  ) && why.contains(says)
(F) BRANCH crates/api/src/disk_route.rs:1742:24 False  [the false side fails the check, and its line ran]  && why.contains(&format!("{bad:?}")),
(F) BRANCH crates/api/src/disk_route.rs:1873:13 False  [the false side fails the check, and its line ran]  why.contains(&named) && why.contains("does not exist"),
(F) BRANCH crates/api/src/disk_route.rs:1873:37 False  [the false side fails the check, and its line ran]  why.contains(&named) && why.contains("does not exist"),
(F) BRANCH crates/api/src/disk_route.rs:1911:13 False  [the false side fails the check, and its line ran]  why.contains(&named) && why.contains("is not a directory"),
(F) BRANCH crates/api/src/disk_route.rs:1911:37 False  [the false side fails the check, and its line ran]  why.contains(&named) && why.contains("is not a directory"),
(F) BRANCH crates/api/src/disk_route.rs:1920:13 False  [the false side fails the check, and its line ran]  why.contains(&below.display().to_string()) && why.contains("cannot be measured"),
(F) BRANCH crates/api/src/disk_route.rs:1920:59 False  [the false side fails the check, and its line ran]  why.contains(&below.display().to_string()) && why.contains("cannot be measured"),
(F) BRANCH crates/api/src/disk_route_seal_tests.rs:233:13 False  [the false side fails the check, and its line ran]  assert!(receipt(root).is_file() && census_store::requires_path(root).is_file());
(F) BRANCH crates/api/src/disk_route_seal_tests.rs:233:40 False  [the false side fails the check, and its line ran]  assert!(receipt(root).is_file() && census_store::requires_path(root).is_file());
(F) BRANCH crates/api/src/disk_route_seal_tests.rs:586:9 False  [the false side fails the check, and its line ran]  why.contains("a seal was interrupted after its link and before its stub")
(F) BRANCH crates/api/src/disk_route_seal_tests.rs:587:16 False  [the false side fails the check, and its line ran]  && why.contains(&format!("one file of {bytes} bytes")),
(F) BRANCH crates/api/src/facets_tests.rs:652:13 False  [the false side fails the check, and its line ran]  assert!(EXCHANGES.is_sorted() && SEGMENTS.is_sorted());
(F) BRANCH crates/api/src/facets_tests.rs:652:38 False  [the false side fails the check, and its line ran]  assert!(EXCHANGES.is_sorted() && SEGMENTS.is_sorted());
(F) BRANCH crates/api/src/facets_tests.rs:1777:13 False  [the false side fails the check, and its line ran]  exchange.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1777:35 False  [the false side fails the check, and its line ran]  exchange.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1777:54 False  [the false side fails the check, and its line ran]  exchange.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/feed_checks_tests.rs:884:13 False  [the false side fails the check, and its line ran]  assert!(first > 1970 && last >= first, "{first}..{last}");
(F) BRANCH crates/api/src/feed_checks_tests.rs:884:29 False  [the false side fails the check, and its line ran]  assert!(first > 1970 && last >= first, "{first}..{last}");
(F) BRANCH crates/api/src/feed_checks_tests.rs:1085:13 False  [the false side fails the check, and its line ran]  why.contains("cut or damaged") && why.contains("held:"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1085:47 False  [the false side fails the check, and its line ran]  why.contains("cut or damaged") && why.contains("held:"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1108:13 False  [the false side fails the check, and its line ran]  why.contains("is not the CSV its index") && why.contains("held:"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1108:57 False  [the false side fails the check, and its line ran]  why.contains("is not the CSV its index") && why.contains("held:"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1151:13 False  [the false side fails the check, and its line ran]  why.contains("is not a folder") && why.contains("feed-reconcile"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1151:48 False  [the false side fails the check, and its line ran]  why.contains("is not a folder") && why.contains("feed-reconcile"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1158:13 False  [the false side fails the check, and its line ran]  why.contains("is not a folder") && why.contains("tick-to-second"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1158:48 False  [the false side fails the check, and its line ran]  why.contains("is not a folder") && why.contains("tick-to-second"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1185:9 False  [the false side fails the check, and its line ran]  gap.contains("2025-02") && gap.contains("feed-reconcile"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1185:36 False  [the false side fails the check, and its line ran]  gap.contains("2025-02") && gap.contains("feed-reconcile"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1418:13 False  [the false side fails the check, and its line ran]  why.contains(&format!("line {line}:")) && why.contains("(day, symbol, role) order"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1418:55 False  [the false side fails the check, and its line ran]  why.contains(&format!("line {line}:")) && why.contains("(day, symbol, role) order"),
(F) BRANCH crates/api/src/feeds_route.rs:1763:13 False  [the false side fails the check, and its line ran]  body.contains(r#""standing":"operator""#)
(F) BRANCH crates/api/src/feeds_route.rs:1764:20 False  [the false side fails the check, and its line ran]  && body.contains(r#""standing":"vendor_doc""#),
(F) BRANCH crates/api/src/feeds_route.rs:1786:13 False  [the false side fails the check, and its line ran]  oldest.starts_with("20") && oldest.contains('-'),
(F) BRANCH crates/api/src/feeds_route.rs:1786:41 False  [the false side fails the check, and its line ran]  oldest.starts_with("20") && oldest.contains('-'),
(F) BRANCH crates/api/src/feeds_route.rs:2009:13 False  [the false side fails the check, and its line ran]  fields.contains(r#""kind":"unknown""#) && fields.contains(r#""source":null"#),
(F) BRANCH crates/api/src/feeds_route.rs:2009:55 False  [the false side fails the check, and its line ran]  fields.contains(r#""kind":"unknown""#) && fields.contains(r#""source":null"#),
(F) BRANCH crates/api/src/feeds_route.rs:2231:13 False  [the false side fails the check, and its line ran]  !why.is_empty() && !why.contains("vendor-data"),
(F) BRANCH crates/api/src/feeds_route.rs:2231:32 False  [the false side fails the check, and its line ran]  !why.is_empty() && !why.contains("vendor-data"),
(F) BRANCH crates/api/src/feeds_route.rs:2432:17 False  [the false side fails the check, and its line ran]  feed["archive"].is_null() && feed["window"].is_null(),
(F) BRANCH crates/api/src/feeds_route.rs:2432:46 False  [the false side fails the check, and its line ran]  feed["archive"].is_null() && feed["window"].is_null(),
(F) BRANCH crates/api/src/feeds_route.rs:2450:13 False  [the false side fails the check, and its line ran]  tf.secs() > 60 && tf.secs() % 60 == 0 && *tf != store::path::Timeframe::DAY_1
(F) BRANCH crates/api/src/feeds_route.rs:2450:31 False  [the false side fails the check, and its line ran]  tf.secs() > 60 && tf.secs() % 60 == 0 && *tf != store::path::Timeframe::DAY_1
(F) BRANCH crates/api/src/feeds_route.rs:2485:17 False  [the false side fails the check, and its line ran]  assert!(!ready && why.contains("has no census row"), "{why}");
(F) BRANCH crates/api/src/feeds_route.rs:2485:27 False  [the false side fails the check, and its line ran]  assert!(!ready && why.contains("has no census row"), "{why}");
(F) BRANCH crates/api/src/feeds_route.rs:2773:13 False  [the false side fails the check, and its line ran]  why.contains("2099-01-01") && why.contains("2026-10-15"),
(F) BRANCH crates/api/src/feeds_route.rs:2773:43 False  [the false side fails the check, and its line ran]  why.contains("2099-01-01") && why.contains("2026-10-15"),
(F) BRANCH crates/api/src/feeds_route.rs:2827:13 False  [the false side fails the check, and its line ran]  source.contains("2099-01-01") && source.contains("passed over"),
(F) BRANCH crates/api/src/feeds_route.rs:2827:46 False  [the false side fails the check, and its line ran]  source.contains("2099-01-01") && source.contains("passed over"),
(F) BRANCH crates/api/src/folder.rs:606:13 False  [the false side fails the check, and its line ran]  why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
(F) BRANCH crates/api/src/folder.rs:606:38 False  [the false side fails the check, and its line ran]  why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
(F) BRANCH crates/api/src/folder.rs:606:62 False  [the false side fails the check, and its line ran]  why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
(F) BRANCH crates/api/src/frontierjson.rs:1085:13 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) BRANCH crates/api/src/frontierjson.rs:1085:32 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) BRANCH crates/api/src/frontierjson.rs:1275:13 False  [the false side fails the check, and its line ran]  body.contains(r#""rank":2"#) && !body.contains(r#""rank":1"#),
(F) BRANCH crates/api/src/frontierjson.rs:1275:45 False  [the false side fails the check, and its line ran]  body.contains(r#""rank":2"#) && !body.contains(r#""rank":1"#),
(F) BRANCH crates/api/src/frontierjson.rs:1284:17 False  [the false side fails the check, and its line ran]  body.contains(r#""rank":1"#) && body.contains(r#""rank":2"#),
(F) BRANCH crates/api/src/frontierjson.rs:1284:49 False  [the false side fails the check, and its line ran]  body.contains(r#""rank":1"#) && body.contains(r#""rank":2"#),
(F) LINE crates/api/src/gap_peer_route_tests.rs:398  [panic/unreachable line]  panic!("the peer vote was still building after 1,000 asks");
(F) LINE crates/api/src/gaps_route.rs:2636  [panic/unreachable line]  panic!("a reader gone first: no month is read")
(F) LINE crates/api/src/gaps_route.rs:4466  [failing arm of the statement at 4462, which ran]  iso(*halt)
(F) LINE crates/api/src/gaps_route.rs:5474  [panic/unreachable line]  panic!("a vote with no build behind it answered");
(F) LINE crates/api/src/gaps_route.rs:5493  [panic/unreachable line]  panic!("the first join leads");
(F) LINE crates/api/src/gaps_route.rs:5504  [panic/unreachable line]  panic!("a parked refusal is handed out as it is");
(F) LINE crates/api/src/gaps_route.rs:5512  [panic/unreachable line]  panic!("room for build {other}");
(F) LINE crates/api/src/gaps_route.rs:5517  [panic/unreachable line]  panic!("a full build map admits no vote");
(F) LINE crates/api/src/gaps_route.rs:5567  [failing arm of the statement at 5564, which ran]  String::from_utf8_lossy(&output.stderr)
(F) LINE crates/api/src/gaps_route.rs:5613  [panic/unreachable line]  panic!("a vote whose reading was not admitted answered");
(F) BRANCH crates/api/src/gaps_route.rs:3164:13 False  [the false side fails the check, and its line ran]  three.contains(r#""months":3"#) && three.contains(r#""months_absent":2"#),
(F) BRANCH crates/api/src/gaps_route.rs:3164:48 False  [the false side fails the check, and its line ran]  three.contains(r#""months":3"#) && three.contains(r#""months_absent":2"#),
(F) BRANCH crates/api/src/gaps_route.rs:3182:13 False  [the false side fails the check, and its line ran]  one.contains(r#""months":1"#) && one.contains(r#""months_absent":0"#),
(F) BRANCH crates/api/src/gaps_route.rs:3182:46 False  [the false side fails the check, and its line ran]  one.contains(r#""months":1"#) && one.contains(r#""months_absent":0"#),
(F) BRANCH crates/api/src/gaps_route.rs:3206:13 False  [the false side fails the check, and its line ran]  typo.contains("YYYY-MM") && typo.contains("to"),
(F) BRANCH crates/api/src/gaps_route.rs:3206:41 False  [the false side fails the check, and its line ran]  typo.contains("YYYY-MM") && typo.contains("to"),
(F) BRANCH crates/api/src/gaps_route.rs:3244:13 False  [the false side fails the check, and its line ran]  three.contains(r#""source":"record""#) && three.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/gaps_route.rs:3244:55 False  [the false side fails the check, and its line ran]  three.contains(r#""source":"record""#) && three.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/gaps_route.rs:3478:13 False  [the false side fails the check, and its line ran]  reasons.contains("no-print") && reasons.contains("expired"),
(F) BRANCH crates/api/src/gaps_route.rs:3478:45 False  [the false side fails the check, and its line ran]  reasons.contains("no-print") && reasons.contains("expired"),
(F) BRANCH crates/api/src/gaps_route.rs:3660:37 False  [the false side fails the check, and its line ran]  .is_some_and(|from| from.contains("import journal") && from.contains("indices")),
(F) BRANCH crates/api/src/gaps_route.rs:3669:28 False  [the false side fails the check, and its line ran]  .all(|run| run["reason"] == "not-imported" && run["minutes"] == 375),
(F) BRANCH crates/api/src/gaps_route.rs:3745:37 False  [the false side fails the check, and its line ran]  .is_some_and(|from| from.contains("2099-01-01") && from.contains("passed over")),
(F) BRANCH crates/api/src/gaps_route.rs:3800:21 False  [the false side fails the check, and its line ran]  assert!(reach.not_yet(day(16)) && !reach.not_imported(day(16)));
(F) BRANCH crates/api/src/gaps_route.rs:3800:47 False  [the false side fails the check, and its line ran]  assert!(reach.not_yet(day(16)) && !reach.not_imported(day(16)));
(F) BRANCH crates/api/src/gaps_route.rs:4017:13 False  [the false side fails the check, and its line ran]  why.contains("NSE/INDEX") && why.contains("NSE/CASH"),
(F) BRANCH crates/api/src/gaps_route.rs:4017:42 False  [the false side fails the check, and its line ran]  why.contains("NSE/INDEX") && why.contains("NSE/CASH"),
(F) BRANCH crates/api/src/gaps_route.rs:4690:17 False  [the false side fails the check, and its line ran]  !faults.is_empty()
(F) BRANCH crates/api/src/gaps_route.rs:4691:24 False  [the false side fails the check, and its line ran]  && !faults
(F) BRANCH crates/api/src/gaps_route.rs:5029:59 False  [the false side fails the check, and its line ran]  pull::calendar::DayKind::Open(session) if session.expects(555)
(F) BRANCH crates/api/src/gaps_route.rs:5299:13 False  [the false side fails the check, and its line ran]  body.contains(&crate::calendar_of::build::MAX_BUILDS.to_string())
(F) BRANCH crates/api/src/gaps_route.rs:5300:20 False  [the false side fails the check, and its line ran]  && body.contains("calendar=record"),
(F) BRANCH crates/api/src/gaps_route.rs:5473:17 False  [the else of a let-else that panics, whose line ran]  let Err((status, _, body)) = answer else {
(F) BRANCH crates/api/src/gaps_route.rs:5492:13 False  [the else of a let-else that panics, whose line ran]  let Joined::Lead(queued) = builds.votes.join(&key, Some(&now), None) else {
(F) BRANCH crates/api/src/gaps_route.rs:5503:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, _, body)) = voted(&site, &asked).await else {
(F) BRANCH crates/api/src/gaps_route.rs:5511:17 False  [the else of a let-else that panics, whose line ran]  let Joined::Lead(build) = builds.votes.join(&key, Some(&now), None) else {
(F) BRANCH crates/api/src/gaps_route.rs:5516:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, _, body)) = voted(&site, &asked).await else {
(F) BRANCH crates/api/src/gaps_route.rs:5565:17 False  [the false side fails the check, and its line ran]  output.status.success() && stdout.contains("1 passed"),
(F) BRANCH crates/api/src/gaps_route.rs:5565:44 False  [the false side fails the check, and its line ran]  output.status.success() && stdout.contains("1 passed"),
(F) BRANCH crates/api/src/gaps_route.rs:5612:17 False  [the else of a let-else that panics, whose line ran]  let Err((status, _, body)) = voted(&site, &asked).await else {
(F) LINE crates/api/src/head_deadline_body_tests.rs:168  [failing arm of the statement at 165, which ran]  String::from_utf8_lossy(&got)
(F) LINE crates/api/src/head_deadline_body_tests.rs:235  [failing arm of the statement at 232, which ran]  String::from_utf8_lossy(&got)
(F) LINE crates/api/src/head_deadline_body_tests.rs:244  [panic/unreachable line]  panic!("byte {at} of {} is not the byte sent there", body.len());
(F) LINE crates/api/src/head_deadline_body_tests.rs:294  [failing arm of the statement at 291, which ran]  body.len()
(F) LINE crates/api/src/head_deadline_body_tests.rs:351  [failing arm of the statement at 348, which ran]  body.len()
(F) LINE crates/api/src/head_deadline_body_tests.rs:433  [failing arm of the statement at 427, which ran]  cut[0]
(F) LINE crates/api/src/head_deadline_body_tests.rs:530  [failing arm of the statement at 527, which ran]  body.len()
(F) LINE crates/api/src/head_deadline_body_tests.rs:674  [failing arm of the statement at 671, which ran]  framed.len(),
(F) LINE crates/api/src/head_deadline_body_tests.rs:675  [failing arm of the statement at 671, which ran]  began.elapsed()
(F) LINE crates/api/src/head_deadline_body_tests.rs:761  [failing arm of the statement at 758, which ran]  left.elapsed()
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:304:9 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:304:19 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:304:58 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:338:13 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:338:23 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:349:9 False  [the false side fails the check, and its line ran]  closed && body.len() < length,
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:349:19 False  [the false side fails the check, and its line ran]  closed && body.len() < length,
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:371:9 False  [the false side fails the check, and its line ran]  went > 0 && went < u64::try_from(BIG).unwrap(),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:371:21 False  [the false side fails the check, and its line ran]  went > 0 && went < u64::try_from(BIG).unwrap(),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:424:17 False  [the false side fails the check, and its line ran]  assert!(closed && body.len() < BIG, "{target}: {} bytes", body.len());
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:424:27 False  [the false side fails the check, and its line ran]  assert!(closed && body.len() < BIG, "{target}: {} bytes", body.len());
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:450:13 False  [the false side fails the check, and its line ran]  assert!(!closed && body.len() == BIG);
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:450:24 False  [the false side fails the check, and its line ran]  assert!(!closed && body.len() == BIG);
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:469:13 False  [the false side fails the check, and its line ran]  assert!(!closed && body.len() == length);
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:469:24 False  [the false side fails the check, and its line ran]  assert!(!closed && body.len() == length);
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:528:13 False  [the false side fails the check, and its line ran]  closed && body.len() < length,
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:528:23 False  [the false side fails the check, and its line ran]  closed && body.len() < length,
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:567:9 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:567:19 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:567:58 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:610:17 False  [the false side fails the check, and its line ran]  assert!(closed && said.is_empty(), "{request}: {said:?}");
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:610:27 False  [the false side fails the check, and its line ran]  assert!(closed && said.is_empty(), "{request}: {said:?}");
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:720:17 False  [the false side fails the check, and its line ran]  assert!(closed && said.is_empty(), "{size}: {said:?}");
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:720:27 False  [the false side fails the check, and its line ran]  assert!(closed && said.is_empty(), "{size}: {said:?}");
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:757:13 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:757:23 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:802:13 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:802:23 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/import_journal.rs:1065:28 False  [the false side fails the check, and its line ran]  .all(|end| end.end_line.is_none() && end.fields.len() == 3),
(F) BRANCH crates/api/src/import_journal.rs:1374:17 False  [the false side fails the check, and its line ran]  assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
(F) BRANCH crates/api/src/import_journal.rs:1374:24 False  [the false side fails the check, and its line ran]  assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
(F) BRANCH crates/api/src/import_journal.rs:1374:57 False  [the false side fails the check, and its line ran]  assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
(F) LINE crates/api/src/imports.rs:2563  [failing arm of the statement at 2559, which ran]  ids(&ends_inside)
(F) BRANCH crates/api/src/imports.rs:2088:17 False  [the false side fails the check, and its line ran]  assert!(why.contains("bogus") && why.contains("reasn"), "{why}");
(F) BRANCH crates/api/src/imports.rs:2088:42 False  [the false side fails the check, and its line ran]  assert!(why.contains("bogus") && why.contains("reasn"), "{why}");
(F) BRANCH crates/api/src/imports_filter_tests.rs:136:24 False  [the false side fails the check, and its line ran]  .all(|row| row["filter"] != "*" && row["outcome_from"] == "journal"),
(F) BRANCH crates/api/src/indexmap.rs:912:17 False  [the false side fails the check, and its line ran]  assert!(banknifty.is_some() && nifty.is_some());
(F) BRANCH crates/api/src/indexmap.rs:912:40 False  [the false side fails the check, and its line ran]  assert!(banknifty.is_some() && nifty.is_some());
(F) BRANCH crates/api/src/indexmap.rs:1346:13 False  [the false side fails the check, and its line ran]  error.contains("gdfl") && error.contains("no manifest"),
(F) BRANCH crates/api/src/indexmap.rs:1346:39 False  [the false side fails the check, and its line ran]  error.contains("gdfl") && error.contains("no manifest"),
(F) BRANCH crates/api/src/indexstoprankingjson.rs:540:17 False  [the false side fails the check, and its line ran]  assert!(discover < cache && pinned < cache);
(F) BRANCH crates/api/src/indexstoprankingjson.rs:540:37 False  [the false side fails the check, and its line ran]  assert!(discover < cache && pinned < cache);
(F) BRANCH crates/api/src/indexstoprankingjson.rs:643:37 False  [the false side fails the check, and its line ran]  Ok(Err(why)) if why.starts_with("Cumulative saved comparison: no saved checkpoint exists")
(F) LINE crates/api/src/ingest.rs:2518  [failing arm of the statement at 2517, which ran]  matches!(
(F) LINE crates/api/src/ingest.rs:2832  [failing arm of the statement at 2829, which ran]  target.slug(),
(F) LINE crates/api/src/ingest.rs:2833  [failing arm of the statement at 2829, which ran]  target.note()
(F) LINE crates/api/src/ingest.rs:2840  [failing arm of the statement at 2837, which ran]  target.slug(),
(F) LINE crates/api/src/ingest.rs:2841  [failing arm of the statement at 2837, which ran]  target.note()
(F) LINE crates/api/src/ingest.rs:2847  [failing arm of the statement at 2844, which ran]  target.slug(),
(F) LINE crates/api/src/ingest.rs:2848  [failing arm of the statement at 2844, which ran]  target.note()
(F) LINE crates/api/src/ingest.rs:3110  [failing arm of the statement at 3105, which ran]  target.slug()
(F) LINE crates/api/src/ingest.rs:3145  [failing arm of the statement at 3142, which ran]  tier.slug()
(F) LINE crates/api/src/ingest.rs:3190  [failing arm of the statement at 3183, which ran]  target.slug()
(F) LINE crates/api/src/ingest.rs:3403  [failing arm of the statement at 3400, which ran]  target.slug()
(F) LINE crates/api/src/ingest.rs:3858  [failing arm of the statement at 3857, which ran]  matches!(
(F) LINE crates/api/src/ingest.rs:4514  [failing arm of the statement at 4510, which ran]  root.display()
(F) BRANCH crates/api/src/ingest.rs:2623:13 False  [the false side fails the check, and its line ran]  text.contains("2026-08-08") && text.contains("2026-08-07"),
(F) BRANCH crates/api/src/ingest.rs:2623:44 False  [the false side fails the check, and its line ran]  text.contains("2026-08-08") && text.contains("2026-08-07"),
(F) BRANCH crates/api/src/ingest.rs:2732:13 False  [the false side fails the check, and its line ran]  text.contains("expiry") && text.contains("2024-2-9"),
(F) BRANCH crates/api/src/ingest.rs:2732:40 False  [the false side fails the check, and its line ran]  text.contains("expiry") && text.contains("2024-2-9"),
(F) BRANCH crates/api/src/ingest.rs:2770:13 False  [the false side fails the check, and its line ran]  text.contains("2022-02-08") && text.contains("2022-01-08"),
(F) BRANCH crates/api/src/ingest.rs:2770:44 False  [the false side fails the check, and its line ran]  text.contains("2022-02-08") && text.contains("2022-01-08"),
(F) BRANCH crates/api/src/ingest.rs:2816:17 False  [the false side fails the check, and its line ran]  assert!(text.contains("3654") && text.contains("3653"), "{text}");
(F) BRANCH crates/api/src/ingest.rs:2816:42 False  [the false side fails the check, and its line ran]  assert!(text.contains("3654") && text.contains("3653"), "{text}");
(F) BRANCH crates/api/src/ingest.rs:3840:21 False  [the false side fails the check, and its line ran]  assert!(text.contains(live) && text.contains("2026-08-07"), "{text}");
(F) BRANCH crates/api/src/ingest.rs:3840:44 False  [the false side fails the check, and its line ran]  assert!(text.contains(live) && text.contains("2026-08-07"), "{text}");
(F) BRANCH crates/api/src/ingest.rs:4871:13 False  [the false side fails the check, and its line ran]  idle["in_flight"].is_null() && idle["run_target"].is_null(),
(F) BRANCH crates/api/src/ingest.rs:4871:44 False  [the false side fails the check, and its line ran]  idle["in_flight"].is_null() && idle["run_target"].is_null(),
(F) BRANCH crates/api/src/instruments_route.rs:882:17 False  [the false side fails the check, and its line ran]  answer.contains("x-brutex-census-note:") && answer.contains("UNREADABLE"),
(F) BRANCH crates/api/src/instruments_route.rs:882:61 False  [the false side fails the check, and its line ran]  answer.contains("x-brutex-census-note:") && answer.contains("UNREADABLE"),
(F) BRANCH crates/api/src/instruments_route.rs:992:13 False  [the false side fails the check, and its line ran]  !json.contains('\n') && !json.contains('\t'),
(F) BRANCH crates/api/src/instruments_route.rs:992:37 False  [the false side fails the check, and its line ran]  !json.contains('\n') && !json.contains('\t'),
(F) BRANCH crates/api/src/instruments_route.rs:996:13 False  [the false side fails the check, and its line ran]  json.starts_with('[') && json.ends_with(']'),
(F) BRANCH crates/api/src/instruments_route.rs:996:38 False  [the false side fails the check, and its line ran]  json.starts_with('[') && json.ends_with(']'),
(F) BRANCH crates/api/src/instruments_route.rs:1176:13 False  [the false side fails the check, and its line ran]  archive.contains(r#""mastered":false"#)
(F) BRANCH crates/api/src/instruments_route.rs:1177:20 False  [the false side fails the check, and its line ran]  && archive.contains(r#""counted_from":"no master""#),
(F) BRANCH crates/api/src/instruments_route.rs:1196:13 False  [the false side fails the check, and its line ran]  refused.contains(r#""feed":"upstox""#)
(F) BRANCH crates/api/src/instruments_route.rs:1197:20 False  [the false side fails the check, and its line ran]  && refused.contains("this build reads no feed called that")
(F) BRANCH crates/api/src/instruments_route.rs:1198:20 False  [the false side fails the check, and its line ran]  && refused.contains("groww")
(F) BRANCH crates/api/src/instruments_route.rs:1199:20 False  [the false side fails the check, and its line ran]  && refused.contains("dhan"),
(F) BRANCH crates/api/src/instruments_route.rs:1231:32 False  [the false side fails the check, and its line ran]  .is_ok_and(|t| t.starts_with('"') && t.ends_with('"')),
(F) LINE crates/api/src/ladder.rs:938  [panic/unreachable line]  panic!("nothing held must refuse");
(F) LINE crates/api/src/ladder.rs:1061  [failing arm of the statement at 1060, which ran]  matches!(
(F) LINE crates/api/src/ladder.rs:1076  [failing arm of the statement at 1075, which ran]  matches!(
(F) LINE crates/api/src/ladder.rs:1281  [failing arm of the statement at 1277, which ran]  tf.as_str()
(F) LINE crates/api/src/ladder.rs:1294  [failing arm of the statement at 1291, which ran]  tf.as_str()
(F) LINE crates/api/src/livejson.rs:682  [failing arm of the statement at 678, which ran]  body.len()
(F) LINE crates/api/src/livejson.rs:687  [failing arm of the statement at 684, which ran]  body.len()
(F) BRANCH crates/api/src/livejson.rs:336:13 False  [the false side fails the check, and its line ran]  !body.contains(r#""stale""#) && !body.contains(r#""strays""#),
(F) BRANCH crates/api/src/livejson.rs:336:45 False  [the false side fails the check, and its line ran]  !body.contains(r#""stale""#) && !body.contains(r#""strays""#),
(F) BRANCH crates/api/src/livejson.rs:528:13 False  [the false side fails the check, and its line ran]  body.contains(r#""t_milli":1802"#) && body.contains(r#""clears_bar":false"#),
(F) BRANCH crates/api/src/livejson.rs:528:51 False  [the false side fails the check, and its line ran]  body.contains(r#""t_milli":1802"#) && body.contains(r#""clears_bar":false"#),
(F) BRANCH crates/api/src/livejson.rs:537:13 False  [the false side fails the check, and its line ran]  !body.contains(r#""meets""#) && !body.contains(r#""win_rate_bp""#),
(F) BRANCH crates/api/src/livejson.rs:537:45 False  [the false side fails the check, and its line ran]  !body.contains(r#""meets""#) && !body.contains(r#""win_rate_bp""#),
(F) BRANCH crates/api/src/livejson.rs:550:13 False  [the false side fails the check, and its line ran]  body.contains(r#""idle_secs":"#) && !body.contains(r#""idle_secs":null"#),
(F) BRANCH crates/api/src/livejson.rs:550:49 False  [the false side fails the check, and its line ran]  body.contains(r#""idle_secs":"#) && !body.contains(r#""idle_secs":null"#),
(F) BRANCH crates/api/src/livejson.rs:555:13 False  [the false side fails the check, and its line ran]  body.contains(r#""stale":false"#) && body.contains(r#""stale_after_secs":86400"#),
(F) BRANCH crates/api/src/livejson.rs:555:50 False  [the false side fails the check, and its line ran]  body.contains(r#""stale":false"#) && body.contains(r#""stale_after_secs":86400"#),
(F) BRANCH crates/api/src/livejson.rs:690:13 False  [the false side fails the check, and its line ran]  body.contains("hard response ceiling") && body.contains(r#""runs":[]"#),
(F) BRANCH crates/api/src/livejson.rs:690:55 False  [the false side fails the check, and its line ran]  body.contains("hard response ceiling") && body.contains(r#""runs":[]"#),
(F) BRANCH crates/api/src/livejson.rs:703:13 False  [the false side fails the check, and its line ran]  encoded > 0 && encoded < cli::live::LIVE_RUN_LIMIT,
(F) BRANCH crates/api/src/livejson.rs:703:28 False  [the false side fails the check, and its line ran]  encoded > 0 && encoded < cli::live::LIVE_RUN_LIMIT,
(F) LINE crates/api/src/logs.rs:7372  [failing arm of the statement at 7368, which ran]  note_of(&after_restart)
(F) LINE crates/api/src/logs.rs:7377  [failing arm of the statement at 7374, which ran]  note_of(&after_restart)
(F) LINE crates/api/src/logs.rs:7386  [failing arm of the statement at 7383, which ran]  note_of(&same_process)
(F) LINE crates/api/src/logs.rs:7396  [failing arm of the statement at 7393, which ran]  note_of(&rolled)
(F) LINE crates/api/src/logs.rs:7404  [failing arm of the statement at 7401, which ran]  note_of(&none)
(F) LINE crates/api/src/logs.rs:9073  [failing arm of the statement at 9070, which ran]  dir.name()
(F) LINE crates/api/src/logs.rs:9089  [failing arm of the statement at 9086, which ran]  kind.as_str(),
(F) LINE crates/api/src/logs.rs:9097  [failing arm of the statement at 9092, which ran]  written.name()
(F) LINE crates/api/src/logs.rs:9120  [failing arm of the statement at 9115, which ran]  dir.display()
(F) LINE crates/api/src/logs.rs:9171  [panic/unreachable line]  panic!("{half} reads {state:?}, not unreadable");
(F) LINE crates/api/src/logs.rs:9356  [panic/unreachable line]  panic!("mode 000 does not stop this process from reading: run as a user it binds");
(F) LINE crates/api/src/logs.rs:9366  [failing arm of the statement at 9363, which ran]  first.errors
(F) LINE crates/api/src/logs.rs:10094  [failing arm of the statement at 10090, which ran]  got["keyed"]
(F) LINE crates/api/src/logs.rs:10101  [failing arm of the statement at 10098, which ran]  got["keyed"]
(F) LINE crates/api/src/logs.rs:10180  [failing arm of the statement at 10176, which ran]  got["keyed"]
(F) LINE crates/api/src/logs.rs:10223  [failing arm of the statement at 10216, which ran]  none["keyed"]
(F) LINE crates/api/src/logs.rs:10822  [failing arm of the statement at 10819, which ran]  held.bytes - read
(F) LINE crates/api/src/logs.rs:10869  [failing arm of the statement at 10866, which ran]  now.unread("options")
(F) LINE crates/api/src/logs.rs:10973  [failing arm of the statement at 10970, which ran]  held.malformed
(F) LINE crates/api/src/logs.rs:11421  [failing arm of the statement at 11418, which ran]  again["keyed"]
(F) LINE crates/api/src/logs.rs:11479  [failing arm of the statement at 11475, which ran]  old["keyed"]
(F) LINE crates/api/src/logs.rs:11618  [failing arm of the statement at 11614, which ran]  second["keyed"]
(F) LINE crates/api/src/logs.rs:11683  [failing arm of the statement at 11679, which ran]  second["keyed"]
(F) LINE crates/api/src/logs.rs:11690  [failing arm of the statement at 11687, which ran]  second["keyed"]["index"]
(F) LINE crates/api/src/logs.rs:12225  [failing arm of the statement at 12222, which ran]  read_words(&indexed)
(F) BRANCH crates/api/src/logs.rs:6746:17 False  [the false side fails the check, and its line ran]  assert!(with("cross-site") && with("same-site"));
(F) BRANCH crates/api/src/logs.rs:6746:39 False  [the false side fails the check, and its line ran]  assert!(with("cross-site") && with("same-site"));
(F) BRANCH crates/api/src/logs.rs:6747:17 False  [the false side fails the check, and its line ran]  assert!(!with("same-origin") && !with("none"));
(F) BRANCH crates/api/src/logs.rs:6747:41 False  [the false side fails the check, and its line ran]  assert!(!with("same-origin") && !with("none"));
(F) BRANCH crates/api/src/logs.rs:6890:13 False  [the false side fails the check, and its line ran]  json.contains(r#""hit_scan_cap":false"#) && json.contains(r#""malformed":0"#),
(F) BRANCH crates/api/src/logs.rs:6890:57 False  [the false side fails the check, and its line ran]  json.contains(r#""hit_scan_cap":false"#) && json.contains(r#""malformed":0"#),
(F) BRANCH crates/api/src/logs.rs:7204:13 False  [the false side fails the check, and its line ran]  all.contains("landed") && all.contains("did not land"),
(F) BRANCH crates/api/src/logs.rs:7204:39 False  [the false side fails the check, and its line ran]  all.contains("landed") && all.contains("did not land"),
(F) BRANCH crates/api/src/logs.rs:7329:13 False  [the false side fails the check, and its line ran]  plain.contains("BRUTEX_LOG_LEVEL=debug") && !plain.contains("the filters in force"),
(F) BRANCH crates/api/src/logs.rs:7329:57 False  [the false side fails the check, and its line ran]  plain.contains("BRUTEX_LOG_LEVEL=debug") && !plain.contains("the filters in force"),
(F) BRANCH crates/api/src/logs.rs:7443:13 False  [the false side fails the check, and its line ran]  page.contains("clock is held") && page.contains("3 event(s)"),
(F) BRANCH crates/api/src/logs.rs:7443:47 False  [the false side fails the check, and its line ran]  page.contains("clock is held") && page.contains("3 event(s)"),
(F) BRANCH crates/api/src/logs.rs:7449:13 False  [the false side fails the check, and its line ran]  json.contains(r#""clock_held":3"#) && json.contains(r#""loud":true"#),
(F) BRANCH crates/api/src/logs.rs:7449:51 False  [the false side fails the check, and its line ran]  json.contains(r#""clock_held":3"#) && json.contains(r#""loud":true"#),
(F) BRANCH crates/api/src/logs.rs:7526:13 False  [the false side fails the check, and its line ran]  page.contains("halt") && page.contains("2 roll(s) failed"),
(F) BRANCH crates/api/src/logs.rs:7526:38 False  [the false side fails the check, and its line ran]  page.contains("halt") && page.contains("2 roll(s) failed"),
(F) BRANCH crates/api/src/logs.rs:7543:13 False  [the false side fails the check, and its line ran]  none.contains("No sink") && none.contains("older run"),
(F) BRANCH crates/api/src/logs.rs:7543:41 False  [the false side fails the check, and its line ran]  none.contains("No sink") && none.contains("older run"),
(F) BRANCH crates/api/src/logs.rs:7865:13 False  [the false side fails the check, and its line ran]  both.contains("<code>/tmp/served</code>")
(F) BRANCH crates/api/src/logs.rs:7866:20 False  [the false side fails the check, and its line ran]  && both.contains("<code>/tmp/served/cli</code>"),
(F) BRANCH crates/api/src/logs.rs:8041:13 False  [the false side fails the check, and its line ran]  body.starts_with(r#"{"records":["#) && body.ends_with('}'),
(F) BRANCH crates/api/src/logs.rs:8041:52 False  [the false side fails the check, and its line ran]  body.starts_with(r#"{"records":["#) && body.ends_with('}'),
(F) BRANCH crates/api/src/logs.rs:8667:20 False  [the false side fails the check, and its line ran]  || keyed_page.contains("/logs.json?limit=50&feed=gdfl&day=2022-03-07"),
(F) BRANCH crates/api/src/logs.rs:8771:17 False  [the false side fails the check, and its line ran]  assert!(is_db_read("/bars.json") && !is_db_read("/pull/spot"));
(F) BRANCH crates/api/src/logs.rs:8771:45 False  [the false side fails the check, and its line ran]  assert!(is_db_read("/bars.json") && !is_db_read("/pull/spot"));
(F) BRANCH crates/api/src/logs.rs:9170:17 False  [the else of a let-else that panics, whose line ran]  let DirState::Unreadable(why) = state else {
(F) BRANCH crates/api/src/logs.rs:9219:21 False  [the false side fails the check, and its line ran]  said.starts_with(&format!("{dir} unreadable: "))
(F) BRANCH crates/api/src/logs.rs:9220:28 False  [the false side fails the check, and its line ran]  && said.contains("Permission denied"),
(F) BRANCH crates/api/src/logs.rs:9508:28 False  [the false side fails the check, and its line ran]  .all(|row| row["half"] == "gdfl/indices" && row["source"] == "audit"),
(F) BRANCH crates/api/src/logs.rs:10013:17 False  [the false side fails the check, and its line ran]  got.ignored.contains(&(bad.0, bad.1.to_owned()))
(F) BRANCH crates/api/src/logs.rs:10014:24 False  [the false side fails the check, and its line ran]  && got.ignored.contains(&(good, said.to_owned())),
(F) BRANCH crates/api/src/logs.rs:10745:17 False  [the false side fails the check, and its line ran]  assert!(none["audit_stream"].is_null() && none["state_stream"].is_null());
(F) BRANCH crates/api/src/logs.rs:10745:51 False  [the false side fails the check, and its line ran]  assert!(none["audit_stream"].is_null() && none["state_stream"].is_null());
(F) BRANCH crates/api/src/logs.rs:11204:17 False  [the false side fails the check, and its line ran]  assert!(none.pointers.is_empty() && !more);
(F) BRANCH crates/api/src/logs.rs:11204:45 False  [the false side fails the check, and its line ran]  assert!(none.pointers.is_empty() && !more);
(F) BRANCH crates/api/src/logs.rs:11223:56 False  [the false side fails the check, and its line ran]  matches!(spent, Ok(Walked::Stopped(at)) if at == len),
(F) BRANCH crates/api/src/logs.rs:11235:57 False  [the false side fails the check, and its line ran]  matches!(walked, Ok(Walked::Stopped(at)) if at == first),
(F) BRANCH crates/api/src/logs.rs:11244:59 False  [the false side fails the check, and its line ran]  assert!(matches!(torn, Ok(Walked::Stopped(at)) if at == len - 1));
(F) BRANCH crates/api/src/logs.rs:11246:17 False  [the false side fails the check, and its line ran]  assert!(out.page.pointers.is_empty() && out.damage.is_empty());
(F) BRANCH crates/api/src/logs.rs:11246:49 False  [the false side fails the check, and its line ran]  assert!(out.page.pointers.is_empty() && out.damage.is_empty());
(F) BRANCH crates/api/src/logs.rs:11252:59 False  [the false side fails the check, and its line ran]  assert!(matches!(full, Ok(Walked::Stopped(at)) if at == len));
(F) BRANCH crates/api/src/logs.rs:11407:13 False  [the false side fails the check, and its line ran]  probed.contains(&"day") && !probed.contains(&"span"),
(F) BRANCH crates/api/src/logs.rs:11407:40 False  [the false side fails the check, and its line ran]  probed.contains(&"day") && !probed.contains(&"span"),
(F) BRANCH crates/api/src/logs.rs:11835:17 False  [the false side fails the check, and its line ran]  assert!(read > 0 && read <= 1_000, "{read}");
(F) BRANCH crates/api/src/logs.rs:11835:29 False  [the false side fails the check, and its line ran]  assert!(read > 0 && read <= 1_000, "{read}");
(F) BRANCH crates/api/src/logs.rs:11863:20 False  [the false side fails the check, and its line ran]  if why.contains(&rolled.display().to_string())),
(F) BRANCH crates/api/src/logs.rs:12055:13 False  [the false side fails the check, and its line ran]  why.contains("damaged.ndjson") && why.contains("undecodable lines"),
(F) BRANCH crates/api/src/logs.rs:12055:47 False  [the false side fails the check, and its line ran]  why.contains("damaged.ndjson") && why.contains("undecodable lines"),
(F) BRANCH crates/api/src/logs.rs:12139:40 False  [the false side fails the check, and its line ran]  ms > to || start < cut.at,
(F) BRANCH crates/api/src/logs.rs:12165:13 False  [the false side fails the check, and its line ran]  exact > 0 && around > 0,
(F) BRANCH crates/api/src/logs.rs:12165:26 False  [the false side fails the check, and its line ran]  exact > 0 && around > 0,
(F) BRANCH crates/api/src/logs_units_tests.rs:77:17 False  [the false side fails the check, and its line ran]  keys.contract.is_none() && keys.code.is_none() && keys.reason.is_none(),
(F) BRANCH crates/api/src/logs_units_tests.rs:77:44 False  [the false side fails the check, and its line ran]  keys.contract.is_none() && keys.code.is_none() && keys.reason.is_none(),
(F) BRANCH crates/api/src/logs_units_tests.rs:77:67 False  [the false side fails the check, and its line ran]  keys.contract.is_none() && keys.code.is_none() && keys.reason.is_none(),
(F) BRANCH crates/api/src/logs_units_tests.rs:89:13 False  [the false side fails the check, and its line ran]  assert!(keys.month.is_none() && keys.code.is_none() && keys.contract.is_none());
(F) BRANCH crates/api/src/logs_units_tests.rs:89:37 False  [the false side fails the check, and its line ran]  assert!(keys.month.is_none() && keys.code.is_none() && keys.contract.is_none());
(F) BRANCH crates/api/src/logs_units_tests.rs:89:60 False  [the false side fails the check, and its line ran]  assert!(keys.month.is_none() && keys.code.is_none() && keys.contract.is_none());
(F) BRANCH crates/api/src/logs_units_tests.rs:107:13 False  [the false side fails the check, and its line ran]  assert!(tail.records.is_empty() && tail.errors.is_empty());
(F) BRANCH crates/api/src/logs_units_tests.rs:107:40 False  [the false side fails the check, and its line ran]  assert!(tail.records.is_empty() && tail.errors.is_empty());
(F) LINE crates/api/src/master.rs:698  [failing arm of the statement at 693, which ran]  got.kept.len(),
(F) LINE crates/api/src/master.rs:699  [failing arm of the statement at 693, which ran]  got.declined.len(),
(F) BRANCH crates/api/src/master.rs:626:13 False  [the false side fails the check, and its line ran]  kept.contains(&"INFY") && kept.contains(&"RELIANCE"),
(F) BRANCH crates/api/src/master.rs:626:39 False  [the false side fails the check, and its line ran]  kept.contains(&"INFY") && kept.contains(&"RELIANCE"),
(F) BRANCH crates/api/src/master.rs:969:13 False  [the false side fails the check, and its line ran]  reason.contains("row has 5 field(s)") && reason.contains("run to 10"),
(F) BRANCH crates/api/src/master.rs:969:54 False  [the false side fails the check, and its line ran]  reason.contains("row has 5 field(s)") && reason.contains("run to 10"),
(F) BRANCH crates/api/src/master.rs:995:13 False  [the false side fails the check, and its line ran]  reason.contains("row has 9 field(s)") && reason.contains("run to 10"),
(F) BRANCH crates/api/src/master.rs:995:54 False  [the false side fails the check, and its line ran]  reason.contains("row has 9 field(s)") && reason.contains("run to 10"),
(F) BRANCH crates/api/src/master.rs:1108:13 False  [the false side fails the check, and its line ran]  err.contains(&(MAX_MASTER_BYTES + 1).to_string())
(F) BRANCH crates/api/src/master.rs:1109:20 False  [the false side fails the check, and its line ran]  && err.contains(&MAX_MASTER_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1148:13 False  [the false side fails the check, and its line ran]  err.contains("header row is") && err.contains(&MAX_ROW_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1148:46 False  [the false side fails the check, and its line ran]  err.contains("header row is") && err.contains(&MAX_ROW_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1189:13 False  [the false side fails the check, and its line ran]  reason.contains("row is") && reason.contains(&MAX_ROW_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1189:42 False  [the false side fails the check, and its line ran]  reason.contains("row is") && reason.contains(&MAX_ROW_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1251:17 False  [the false side fails the check, and its line ran]  assert!(got.kept.is_empty() && got.skipped.is_empty());
(F) BRANCH crates/api/src/master.rs:1251:40 False  [the false side fails the check, and its line ran]  assert!(got.kept.is_empty() && got.skipped.is_empty());
(F) BRANCH crates/api/src/mastersrun.rs:1589:64 False  [the false side fails the check, and its line ran]  .all(|(_, l, _)| matches!(*l, Ok(ref inner) if inner.is_written())),
(F) BRANCH crates/api/src/mastersrun.rs:2131:13 False  [the false side fails the check, and its line ran]  credentialed.contains("Landed::Refused(why.clone())")
(F) BRANCH crates/api/src/mastersrun.rs:2132:20 False  [the false side fails the check, and its line ran]  && credentialed.contains("Fetched::default()"),
(F) BRANCH crates/api/src/merge.rs:1017:13 False  [the false side fails the check, and its line ran]  e.universe.contains(Universe::FNO) && e.universe.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/api/src/merge.rs:1017:51 False  [the false side fails the check, and its line ran]  e.universe.contains(Universe::FNO) && e.universe.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/api/src/merge.rs:1042:17 False  [the false side fails the check, and its line ran]  assert!(line.contains("groww") && line.contains("dhan"), "{line}");
(F) BRANCH crates/api/src/merge.rs:1042:43 False  [the false side fails the check, and its line ran]  assert!(line.contains("groww") && line.contains("dhan"), "{line}");
(F) BRANCH crates/api/src/merge.rs:1055:17 False  [the false side fails the check, and its line ran]  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1055:54 False  [the false side fails the check, and its line ran]  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1339:17 False  [the false side fails the check, and its line ran]  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1339:54 False  [the false side fails the check, and its line ran]  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1379:17 False  [the false side fails the check, and its line ran]  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1379:54 False  [the false side fails the check, and its line ran]  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/month_change.rs:534:13 False  [the false side fails the check, and its line ran]  !can_be_rebased(Segment::Index, "NIFTY")
(F) BRANCH crates/api/src/month_change.rs:535:20 False  [the false side fails the check, and its line ran]  && can_be_rebased(Segment::Cash, "RELIANCE")
(F) BRANCH crates/api/src/month_change.rs:536:20 False  [the false side fails the check, and its line ran]  && can_be_rebased(Segment::Fno, "RELIANCE")
(F) LINE crates/api/src/nse_identity.rs:491  [panic/unreachable line]  panic!("an absent file refuses");
(F) LINE crates/api/src/nse_identity.rs:504  [panic/unreachable line]  panic!("an unreadable file refuses");
(F) LINE crates/api/src/nse_identity.rs:525  [panic/unreachable line]  panic!("an undated Zerodha dump refuses");
(F) LINE crates/api/src/nse_identity.rs:533  [panic/unreachable line]  panic!("an oversized file refuses");
(F) LINE crates/api/src/nse_identity.rs:542  [panic/unreachable line]  panic!("a file at the bound that is not the constituent file refuses");
(F) BRANCH crates/api/src/nse_identity.rs:490:13 False  [the else of a let-else that panics, whose line ran]  let Err(Refusal::FileUnavailable(why)) = absent.confirm(&merged, "RELIANCE") else {
(F) LINE crates/api/src/operation_audit_tests.rs:200  [failing arm of the statement at 197, which ran]  routes.len()
(F) BRANCH crates/api/src/operation_audit_tests.rs:339:29 False  [the false side fails the check, and its line ran]  Asked::Exact(id) if id == ID_BASE + 1
(F) LINE crates/api/src/ops_safety_tests.rs:82  [panic/unreachable line]  panic!("the store-read pool stayed full for two seconds");
(F) LINE crates/api/src/ops_safety_tests.rs:493  [failing arm of the statement at 490, which ran]  most.len()
(F) BRANCH crates/api/src/ops_safety_tests.rs:147:34 False  [the false side fails the check, and its line ran]  label == "unstamped" || (label.len() == 40 && label.bytes().all(|b| b.is_ascii_hexdigit())),
(F) BRANCH crates/api/src/ops_safety_tests.rs:147:55 False  [the false side fails the check, and its line ran]  label == "unstamped" || (label.len() == 40 && label.bytes().all(|b| b.is_ascii_hexdigit())),
(F) BRANCH crates/api/src/ops_safety_tests.rs:183:9 False  [the false side fails the check, and its line ran]  drift.contains("1128 bytes") && drift.contains("1000"),
(F) BRANCH crates/api/src/ops_safety_tests.rs:183:41 False  [the false side fails the check, and its line ran]  drift.contains("1128 bytes") && drift.contains("1000"),
(F) BRANCH crates/api/src/ops_safety_tests.rs:399:29 False  [the false side fails the check, and its line ran]  !wrote.contains(READ_ONLY_ENV) && !wrote.contains(UNSTAMPED),
(F) BRANCH crates/api/src/ops_safety_tests.rs:399:63 False  [the false side fails the check, and its line ran]  !wrote.contains(READ_ONLY_ENV) && !wrote.contains(UNSTAMPED),
(F) BRANCH crates/api/src/ops_safety_tests.rs:422:13 False  [the false side fails the check, and its line ran]  assert!(why.contains(UNSTAMPED) && why.contains("POST"), "{why}");
(F) BRANCH crates/api/src/ops_safety_tests.rs:422:40 False  [the false side fails the check, and its line ran]  assert!(why.contains(UNSTAMPED) && why.contains("POST"), "{why}");
(F) LINE crates/api/src/optiongreeks_route_tests.rs:253  [panic/unreachable line]  unreachable!("loaded above")
(F) LINE crates/api/src/optiongreeks_route_tests.rs:811  [panic/unreachable line]  panic!("loaded: {moved:?}")
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:252:9 False  [the else of a let-else that panics, whose line ran]  let RateTable::Loaded(series) = &rate else {
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:304:17 False  [the false side fails the check, and its line ran]  assert!(iv > 0.0 && iv < 1.0, "{iv}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:304:29 False  [the false side fails the check, and its line ran]  assert!(iv > 0.0 && iv < 1.0, "{iv}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:451:13 False  [the false side fails the check, and its line ran]  detail.contains("rate_carried")
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:452:20 False  [the false side fails the check, and its line ran]  && detail.contains("2026-08-12")
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:453:20 False  [the false side fails the check, and its line ran]  && detail.contains("2026-08-01"),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:502:17 False  [the false side fails the check, and its line ran]  row.get("iv").is_none() && row.get("delta").is_none(),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:502:44 False  [the false side fails the check, and its line ran]  row.get("iv").is_none() && row.get("delta").is_none(),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:505:21 False  [the false side fails the check, and its line ran]  assert!(row["premium"].is_i64() && row["spot"].is_i64(), "{row}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:505:48 False  [the false side fails the check, and its line ran]  assert!(row["premium"].is_i64() && row["spot"].is_i64(), "{row}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:506:21 False  [the false side fails the check, and its line ran]  assert!(row["tenor_s"].is_i64() && row["tenor_y"].is_f64(), "{row}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:506:48 False  [the false side fails the check, and its line ran]  assert!(row["tenor_s"].is_i64() && row["tenor_y"].is_f64(), "{row}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:621:9 False  [the false side fails the check, and its line ran]  detail.contains("ZERO carry") && detail.contains("not a verdict on the print"),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:621:42 False  [the false side fails the check, and its line ran]  detail.contains("ZERO carry") && detail.contains("not a verdict on the print"),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:627:9 False  [the false side fails the check, and its line ran]  row["tenor_s"].is_i64() && row["above_rate_tenor"] == false,
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:627:36 False  [the false side fails the check, and its line ran]  row["tenor_s"].is_i64() && row["above_rate_tenor"] == false,
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:810:9 False  [the else of a let-else that panics, whose line ran]  let RateTable::Loaded(series) = &*moved else {
(F) BRANCH crates/api/src/owed_tests.rs:249:9 False  [the false side fails the check, and its line ran]  evidence.contains("CML58560") && evidence.contains("CML60618"),
(F) BRANCH crates/api/src/owed_tests.rs:249:42 False  [the false side fails the check, and its line ran]  evidence.contains("CML58560") && evidence.contains("CML60618"),
(F) BRANCH crates/api/src/owed_tests.rs:372:25 False  [the false side fails the check, and its line ran]  .all(|word| word != "no-print" && word != "vendor-hole"),
(F) BRANCH crates/api/src/page_report_route_tests.rs:153:60 False  [the false side fails the check, and its line ran]  matches!((asked, read_back), (Some(a), Some(r)) if a < r),
(F) LINE crates/api/src/pool_table_tests.rs:750  [failing arm of the statement at 746, which ran]  named.pool().name()
(F) LINE crates/api/src/pool_table_tests.rs:1151  [failing arm of the statement at 1148, which ran]  head_text(answer)
(F) LINE crates/api/src/pool_table_tests.rs:1181  [failing arm of the statement at 1178, which ran]  json.len()
(F) LINE crates/api/src/pool_table_tests.rs:1238  [failing arm of the statement at 1235, which ran]  whole.len()
(F) LINE crates/api/src/pool_table_tests.rs:1300  [failing arm of the statement at 1295, which ran]  body["days"]
(F) LINE crates/api/src/pool_table_tests.rs:1307  [failing arm of the statement at 1303, which ran]  body["month_faults"]
(F) LINE crates/api/src/pool_table_tests.rs:1333  [failing arm of the statement at 1330, which ran]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1374  [failing arm of the statement at 1369, which ran]  body["source_faults"]
(F) LINE crates/api/src/pool_table_tests.rs:1380  [failing arm of the statement at 1377, which ran]  body["days"]
(F) LINE crates/api/src/pool_table_tests.rs:1464  [failing arm of the statement at 1461, which ran]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1469  [failing arm of the statement at 1466, which ran]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1544  [failing arm of the statement at 1540, which ran]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1549  [failing arm of the statement at 1546, which ran]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1575  [failing arm of the statement at 1571, which ran]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1735  [failing arm of the statement at 1731, which ran]  head_text(answer)
(F) LINE crates/api/src/pool_table_tests.rs:1740  [failing arm of the statement at 1737, which ran]  head_text(answer)
(F) LINE crates/api/src/pool_table_tests.rs:1839  [failing arm of the statement at 1836, which ran]  body.len()
(F) LINE crates/api/src/pool_table_tests.rs:2085  [failing arm of the statement at 2082, which ran]  head_text(&answer)
(F) BRANCH crates/api/src/pool_table_tests.rs:1297:24 False  [the false side fails the check, and its line ran]  .all(|day| day["held"]["why"] == "being_written"
(F) BRANCH crates/api/src/pool_table_tests.rs:1331:17 False  [the false side fails the check, and its line ran]  !answer.contains("being written") && !answer.contains("\"unreadable\""),
(F) BRANCH crates/api/src/pool_table_tests.rs:1331:54 False  [the false side fails the check, and its line ran]  !answer.contains("being written") && !answer.contains("\"unreadable\""),
(F) BRANCH crates/api/src/pool_table_tests.rs:2104:13 False  [the false side fails the check, and its line ran]  say.contains("A pool's answered is work it ran to its end")
(F) BRANCH crates/api/src/pool_table_tests.rs:2105:20 False  [the false side fails the check, and its line ran]  && say.contains("the two answered columns count different things"),
(F) BRANCH crates/api/src/pool_table_tests.rs:2118:43 False  [the false side fails the check, and its line ran]  route.starts_with('/') || route == crate::detail::FRONT_END,
(F) LINE crates/api/src/provenance_tests.rs:1230  [panic/unreachable line]  panic!("a sidecar");
(F) LINE crates/api/src/provenance_tests.rs:1331  [failing arm of the statement at 1328, which ran]  wire.len()
(F) BRANCH crates/api/src/provenance_tests.rs:1229:9 False  [the else of a let-else that panics, whose line ran]  let Found::Present(sidecar) = store::provenance::read(&root, path).unwrap() else {
(F) LINE crates/api/src/pullrun.rs:4757  [failing arm of the statement at 4754, which ran]  form.len()
(F) LINE crates/api/src/pullrun.rs:4766  [failing arm of the statement at 4763, which ran]  leg.len()
(F) LINE crates/api/src/pullrun.rs:5327  [failing arm of the statement at 5322, which ran]  observed(&site).feeds
(F) LINE crates/api/src/pullrun.rs:6988  [failing arm of the statement at 6985, which ran]  restart_in(&unseen)
(F) LINE crates/api/src/pullrun.rs:7311  [failing arm of the statement at 7307, which ran]  run.words()
(F) LINE crates/api/src/pullrun.rs:7397  [panic/unreachable line]  panic!("with the lock held nothing is known: {held:?}");
(F) LINE crates/api/src/pullrun.rs:7423  [panic/unreachable line]  panic!("an index that cannot be written leaves nothing known: {refused:?}");
(F) LINE crates/api/src/pullrun.rs:8598  [panic/unreachable line]  panic!("a terminal feed must never be requested again");
(F) LINE crates/api/src/pullrun.rs:10861  [panic/unreachable line]  panic!("one counted pairing: {:?}", found.left_open);
(F) LINE crates/api/src/pullrun.rs:10936  [panic/unreachable line]  panic!("one counted pairing: {:?}", found.left_open);
(F) LINE crates/api/src/pullrun.rs:11073  [panic/unreachable line]  panic!("one doubted run: {:?}", answer.doubted);
(F) BRANCH crates/api/src/pullrun.rs:5834:17 False  [the false side fails the check, and its line ran]  assert!(summary.contains("INCOMPLETE") && summary.contains("zerodha"));
(F) BRANCH crates/api/src/pullrun.rs:5834:51 False  [the false side fails the check, and its line ran]  assert!(summary.contains("INCOMPLETE") && summary.contains("zerodha"));
(F) BRANCH crates/api/src/pullrun.rs:5836:17 False  [the false side fails the check, and its line ran]  assert!(!summary.contains("asked for everything") && !summary.contains("no more data"));
(F) BRANCH crates/api/src/pullrun.rs:5836:62 False  [the false side fails the check, and its line ran]  assert!(!summary.contains("asked for everything") && !summary.contains("no more data"));
(F) BRANCH crates/api/src/pullrun.rs:5866:17 False  [the false side fails the check, and its line ran]  assert!(summary.contains("HTTP 422") && summary.contains("PERMANENT REFUSAL"));
(F) BRANCH crates/api/src/pullrun.rs:5866:49 False  [the false side fails the check, and its line ran]  assert!(summary.contains("HTTP 422") && summary.contains("PERMANENT REFUSAL"));
(F) BRANCH crates/api/src/pullrun.rs:5889:17 False  [the false side fails the check, and its line ran]  assert!(summary.contains("Idle retries stopped") && summary.contains("Empty responses"));
(F) BRANCH crates/api/src/pullrun.rs:5889:61 False  [the false side fails the check, and its line ran]  assert!(summary.contains("Idle retries stopped") && summary.contains("Empty responses"));
(F) BRANCH crates/api/src/pullrun.rs:6050:21 False  [the false side fails the check, and its line ran]  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6050:38 False  [the false side fails the check, and its line ran]  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6125:17 False  [the false side fails the check, and its line ran]  assert!(healthy.finished && healthy.last_error.is_none());
(F) BRANCH crates/api/src/pullrun.rs:6125:37 False  [the false side fails the check, and its line ran]  assert!(healthy.finished && healthy.last_error.is_none());
(F) BRANCH crates/api/src/pullrun.rs:6168:21 False  [the false side fails the check, and its line ran]  assert!(summary.contains("INCOMPLETE") && summary.contains(&format!("HTTP {code}")));
(F) BRANCH crates/api/src/pullrun.rs:6168:55 False  [the false side fails the check, and its line ran]  assert!(summary.contains("INCOMPLETE") && summary.contains(&format!("HTTP {code}")));
(F) BRANCH crates/api/src/pullrun.rs:6194:17 False  [the false side fails the check, and its line ran]  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6194:34 False  [the false side fails the check, and its line ran]  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6230:17 False  [the false side fails the check, and its line ran]  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6230:34 False  [the false side fails the check, and its line ran]  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6271:26 False  [the false side fails the check, and its line ran]  .all(|f| f.finished && f.doing.is_empty())
(F) BRANCH crates/api/src/pullrun.rs:6358:13 False  [the false side fails the check, and its line ran]  summary.contains("Idle retries stopped")
(F) BRANCH crates/api/src/pullrun.rs:6359:20 False  [the false side fails the check, and its line ran]  && summary.contains("; 0 bar(s) written by this run's own legs"),
(F) BRANCH crates/api/src/pullrun.rs:6565:13 False  [the false side fails the check, and its line ran]  summary.contains("FINISHED WITH STANDING FAILURES")
(F) BRANCH crates/api/src/pullrun.rs:6566:20 False  [the false side fails the check, and its line ran]  && summary.contains("zerodha (1 leg(s))")
(F) BRANCH crates/api/src/pullrun.rs:6567:20 False  [the false side fails the check, and its line ran]  && summary.contains("is not asked again"),
(F) BRANCH crates/api/src/pullrun.rs:6878:13 False  [the false side fails the check, and its line ran]  words.starts_with(
(F) BRANCH crates/api/src/pullrun.rs:6880:18 False  [the false side fails the check, and its line ran]  ) && words.contains("with 2 member failure(s) on record"),
(F) BRANCH crates/api/src/pullrun.rs:6886:13 False  [the false side fails the check, and its line ran]  !words.contains("The server stopped while it ran")
(F) BRANCH crates/api/src/pullrun.rs:6887:20 False  [the false side fails the check, and its line ran]  && words.contains("unless another process"),
(F) BRANCH crates/api/src/pullrun.rs:7396:17 False  [the else of a let-else that panics, whose line ran]  let Restart::NotKnown(why) = &held else {
(F) BRANCH crates/api/src/pullrun.rs:7400:17 False  [the false side fails the check, and its line ran]  why.contains("no index at")
(F) BRANCH crates/api/src/pullrun.rs:7401:24 False  [the false side fails the check, and its line ran]  && why.contains("another writer held the journal's lock through 2 wait(s)"),
(F) BRANCH crates/api/src/pullrun.rs:7411:17 False  [the false side fails the check, and its line ran]  words.starts_with("NOT KNOWN whether a pull run was left open")
(F) BRANCH crates/api/src/pullrun.rs:7412:24 False  [the false side fails the check, and its line ran]  && !words.contains("INTERRUPTED"),
(F) BRANCH crates/api/src/pullrun.rs:7422:17 False  [the else of a let-else that panics, whose line ran]  let Restart::NotKnown(why) = &refused else {
(F) BRANCH crates/api/src/pullrun.rs:7550:13 False  [the false side fails the check, and its line ran]  why.contains("leave the run opened at record 0 open")
(F) BRANCH crates/api/src/pullrun.rs:7551:20 False  [the false side fails the check, and its line ran]  && why.contains("may be its closing record"),
(F) BRANCH crates/api/src/pullrun.rs:7620:13 False  [the false side fails the check, and its line ran]  why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
(F) BRANCH crates/api/src/pullrun.rs:7620:43 False  [the false side fails the check, and its line ran]  why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
(F) BRANCH crates/api/src/pullrun.rs:7620:73 False  [the false side fails the check, and its line ran]  why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
(F) BRANCH crates/api/src/pullrun.rs:7983:13 False  [the false side fails the check, and its line ran]  words.contains("whose opening record names the same ask")
(F) BRANCH crates/api/src/pullrun.rs:7984:20 False  [the false side fails the check, and its line ran]  && words.contains("a refusal that opened no run closes nothing"),
(F) BRANCH crates/api/src/pullrun.rs:8289:38 False  [the false side fails the check, and its line ran]  .is_some_and(|words| words.starts_with("INTERRUPTED: a pull run")
(F) BRANCH crates/api/src/pullrun.rs:8290:24 False  [the false side fails the check, and its line ran]  && words.contains("run the same pull again from /pull/spot")
(F) BRANCH crates/api/src/pullrun.rs:8457:13 False  [the false side fails the check, and its line ran]  summary.contains("Idle retries stopped")
(F) BRANCH crates/api/src/pullrun.rs:8458:20 False  [the false side fails the check, and its line ran]  && summary.contains("Full basket coverage has not been verified")
(F) BRANCH crates/api/src/pullrun.rs:8515:13 False  [the false side fails the check, and its line ran]  summary.contains("ceiling")
(F) BRANCH crates/api/src/pullrun.rs:8516:20 False  [the false side fails the check, and its line ran]  && summary.contains("Full basket coverage has not been verified")
(F) BRANCH crates/api/src/pullrun.rs:8611:17 False  [the false side fails the check, and its line ran]  assert!(why.contains("HTTP 422") && !why.contains("HTTP 503"));
(F) BRANCH crates/api/src/pullrun.rs:8611:45 False  [the false side fails the check, and its line ran]  assert!(why.contains("HTTP 422") && !why.contains("HTTP 503"));
(F) BRANCH crates/api/src/pullrun.rs:8677:17 False  [the false side fails the check, and its line ran]  assert!(progress.feeds[1].finished && progress.feeds[1].doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:8677:47 False  [the false side fails the check, and its line ran]  assert!(progress.feeds[1].finished && progress.feeds[1].doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:9343:17 False  [the false side fails the check, and its line ran]  assert!(!feed.store_unavailable && !feed.credential_dead);
(F) BRANCH crates/api/src/pullrun.rs:9343:44 False  [the false side fails the check, and its line ran]  assert!(!feed.store_unavailable && !feed.credential_dead);
(F) BRANCH crates/api/src/pullrun.rs:10860:13 False  [the else of a let-else that panics, whose line ran]  let [counted] = found.left_open.as_slice() else {
(F) BRANCH crates/api/src/pullrun.rs:10935:13 False  [the else of a let-else that panics, whose line ran]  let [counted] = found.left_open.as_slice() else {
(F) BRANCH crates/api/src/pullrun.rs:11043:17 False  [the false side fails the check, and its line ran]  assert!(answer.sure.is_empty() && answer.doubted.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:11043:43 False  [the false side fails the check, and its line ran]  assert!(answer.sure.is_empty() && answer.doubted.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:11072:13 False  [the else of a let-else that panics, whose line ran]  let [doubted] = answer.doubted.as_slice() else {
(F) LINE crates/api/src/pullrun_restart_tests.rs:374  [failing arm of the statement at 370, which ran]  run.words()
(F) LINE crates/api/src/pullrun_restart_tests.rs:731  [failing arm of the statement at 728, which ran]  answer.words()
(F) LINE crates/api/src/pullrun_restart_tests.rs:925  [failing arm of the statement at 920, which ran]  answer.words()
(F) LINE crates/api/src/pullrun_restart_tests.rs:1022  [failing arm of the statement at 1019, which ran]  answer.words()
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:168:9 False  [the false side fails the check, and its line ran]  words.contains("3 runs read have no closing record")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:169:16 False  [the false side fails the check, and its line ran]  && words.contains("record 2 (a pull run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:170:16 False  [the false side fails the check, and its line ran]  && words.contains("record 0 (a pull run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:171:16 False  [the false side fails the check, and its line ran]  && words.contains("ag01-three-zerodha"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:532:9 False  [the false side fails the check, and its line ran]  why.contains("the closing record at record 4")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:533:16 False  [the false side fails the check, and its line ran]  && why.contains("a close written without its run's thread could end either run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:534:16 False  [the false side fails the check, and its line ran]  && why.contains("opened at records 1, 0"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:660:13 False  [the false side fails the check, and its line ran]  guessed.why.contains("the closing record at record 2")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:661:20 False  [the false side fails the check, and its line ran]  && guessed.why.contains("opened at records 1, 0"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:679:13 False  [the false side fails the check, and its line ran]  words.starts_with("NOT KNOWN which pull runs were left open, though at least 1 run(s)")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:680:20 False  [the false side fails the check, and its line ran]  && words.contains("so 1 of them have no closing record; which is not known"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:776:9 False  [the false side fails the check, and its line ran]  words.starts_with("NOT KNOWN which pull runs were left open, though at least 2 run(s)")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:777:16 False  [the false side fails the check, and its line ran]  && words.contains("Open whichever way the guesses are read")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:778:16 False  [the false side fails the check, and its line ran]  && words.contains("record 0 (a pull run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:779:16 False  [the false side fails the check, and its line ran]  && words.contains("ag01-p4-kite")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:780:16 False  [the false side fails the check, and its line ran]  && words.contains("A guess names the run(s) opened at records 2, 3"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:981:13 False  [the false side fails the check, and its line ran]  words.contains("named by no guess, though something read may have closed it")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:982:20 False  [the false side fails the check, and its line ran]  && words.contains(&format!(
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:985:20 False  [the false side fails the check, and its line ran]  && words.contains(CLOSE_UNDECODED)
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:986:20 False  [the false side fails the check, and its line ran]  && words.contains("3 older run record(s) that the journal's index names"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1184:9 False  [the false side fails the check, and its line ran]  why.contains(
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1186:14 False  [the false side fails the check, and its line ran]  ) && why.contains("closing record at record 900"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1380:13 False  [the false side fails the check, and its line ran]  words.contains("writes no opening record") && words.contains("never reported here"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1380:59 False  [the false side fails the check, and its line ran]  words.contains("writes no opening record") && words.contains("never reported here"),
(F) LINE crates/api/src/quarantine_route_tests.rs:137  [panic/unreachable line]  panic!("the pool stayed full for two seconds");
(F) LINE crates/api/src/quarantine_route_tests.rs:543  [failing arm of the statement at 542, which ran]  matches!(
(F) BRANCH crates/api/src/quarantine_route_tests.rs:210:62 False  [the false side fails the check, and its line ran]  text.contains(&february[0].ts_micros.to_string()) || text.contains("410"),
(F) LINE crates/api/src/recovery.rs:2493  [panic/unreachable line]  panic!("a server that refuses writes resumed, or said nothing");
(F) LINE crates/api/src/recovery.rs:2503  [failing arm of the statement at 2497, which ran]  hex(id)
(F) LINE crates/api/src/recovery.rs:2765  [failing arm of the statement at 2762, which ran]  attempts
(F) LINE crates/api/src/recovery.rs:2766  [failing arm of the statement at 2762, which ran]  .latest
(F) LINE crates/api/src/recovery.rs:2767  [failing arm of the statement at 2762, which ran]  .values()
(F) LINE crates/api/src/recovery.rs:2768  [failing arm of the statement at 2762, which ran]  .map(|row| row.attempts)
(F) LINE crates/api/src/recovery.rs:2769  [failing arm of the statement at 2762, which ran]  .collect::<Vec<_>>()
(F) BRANCH crates/api/src/recovery.rs:2272:17 False  [the false side fails the check, and its line ran]  assert!(plan_records > 40 && attempt_records == 25 && active_records == 1);
(F) BRANCH crates/api/src/recovery.rs:2272:38 False  [the false side fails the check, and its line ran]  assert!(plan_records > 40 && attempt_records == 25 && active_records == 1);
(F) BRANCH crates/api/src/recovery.rs:2272:63 False  [the false side fails the check, and its line ran]  assert!(plan_records > 40 && attempt_records == 25 && active_records == 1);
(F) BRANCH crates/api/src/recovery.rs:2506:17 False  [the false side fails the check, and its line ran]  line.starts_with(&format!("recovery not resumed: {word}"))
(F) BRANCH crates/api/src/recovery.rs:2507:24 False  [the false side fails the check, and its line ran]  && line.contains(&format!("plan {} is IN FLIGHT", hex(id))),
(F) BRANCH crates/api/src/recovery.rs:3323:28 False  [the false side fails the check, and its line ran]  .all(|row| row.status == Status::Queued && row.attempts == 0)
(F) BRANCH crates/api/src/recovery_journal.rs:1639:17 False  [the false side fails the check, and its line ran]  assert!(journal.latest.is_empty() && journal.order.is_empty());
(F) BRANCH crates/api/src/recovery_journal.rs:1639:46 False  [the false side fails the check, and its line ran]  assert!(journal.latest.is_empty() && journal.order.is_empty());
(F) BRANCH crates/api/src/recovery_journal.rs:1683:21 False  [the false side fails the check, and its line ran]  error.contains("may be partial")
(F) BRANCH crates/api/src/recovery_journal.rs:1684:28 False  [the false side fails the check, and its line ran]  && error.contains("truncation back to 1024 bytes also failed"),
(F) LINE crates/api/src/render.rs:3929  [failing arm of the statement at 3926, which ran]  feed.display()
(F) LINE crates/api/src/render.rs:3934  [failing arm of the statement at 3931, which ran]  feed.display()
(F) LINE crates/api/src/render.rs:3947  [failing arm of the statement at 3940, which ran]  feed.display()
(F) LINE crates/api/src/render.rs:4088  [failing arm of the statement at 4085, which ran]  list.get(..160).unwrap_or(list)
(F) LINE crates/api/src/render.rs:4241  [failing arm of the statement at 4237, which ran]  reason.label()
(F) LINE crates/api/src/render.rs:5690  [failing arm of the statement at 5687, which ran]  String::from_utf8_lossy(&output.stderr)
(F) BRANCH crates/api/src/render.rs:3539:17 False  [the false side fails the check, and its line ran]  assert!(mid.contains("previous") && mid.contains("next"));
(F) BRANCH crates/api/src/render.rs:3539:45 False  [the false side fails the check, and its line ran]  assert!(mid.contains("previous") && mid.contains("next"));
(F) BRANCH crates/api/src/render.rs:3845:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("785") && html.contains("Tracked"));
(F) BRANCH crates/api/src/render.rs:3845:41 False  [the false side fails the check, and its line ran]  assert!(html.contains("785") && html.contains("Tracked"));
(F) BRANCH crates/api/src/render.rs:3951:13 False  [the false side fails the check, and its line ran]  !html.contains("broker \u{b7}") && !html.contains("archive \u{b7}"),
(F) BRANCH crates/api/src/render.rs:3951:48 False  [the false side fails the check, and its line ran]  !html.contains("broker \u{b7}") && !html.contains("archive \u{b7}"),
(F) BRANCH crates/api/src/render.rs:4001:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"));
(F) BRANCH crates/api/src/render.rs:4001:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"));
(F) BRANCH crates/api/src/render.rs:4199:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("12040") && html.contains("11990"));
(F) BRANCH crates/api/src/render.rs:4199:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("12040") && html.contains("11990"));
(F) BRANCH crates/api/src/render.rs:4201:17 False  [the false side fails the check, and its line ran]  assert!(html.contains(">40<") && html.contains(">10<"));
(F) BRANCH crates/api/src/render.rs:4201:42 False  [the false side fails the check, and its line ran]  assert!(html.contains(">40<") && html.contains(">10<"));
(F) BRANCH crates/api/src/render.rs:4219:46 False  [the false side fails the check, and its line ran]  html.contains("class=\"r0\"") || html.contains("class=\"r1\""),
(F) BRANCH crates/api/src/render.rs:4397:13 False  [the false side fails the check, and its line ran]  strip.contains("x=\"0\"") && strip.contains("x=\"2\"") && strip.contains("x=\"3\"")
(F) BRANCH crates/api/src/render.rs:4397:42 False  [the false side fails the check, and its line ran]  strip.contains("x=\"0\"") && strip.contains("x=\"2\"") && strip.contains("x=\"3\"")
(F) BRANCH crates/api/src/render.rs:4397:71 False  [the false side fails the check, and its line ran]  strip.contains("x=\"0\"") && strip.contains("x=\"2\"") && strip.contains("x=\"3\"")
(F) BRANCH crates/api/src/render.rs:4457:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("#41") && html.contains("#40"), "{html}");
(F) BRANCH crates/api/src/render.rs:4457:41 False  [the false side fails the check, and its line ran]  assert!(html.contains("#41") && html.contains("#40"), "{html}");
(F) BRANCH crates/api/src/render.rs:4459:50 False  [the false side fails the check, and its line ran]  html.contains("2 record(s) held") || html.contains("showing 2"),
(F) BRANCH crates/api/src/render.rs:4572:21 False  [the false side fails the check, and its line ran]  !url.contains("07?page") && !url.contains("=0?") && !url.contains("=1?"),
(F) BRANCH crates/api/src/render.rs:4572:49 False  [the false side fails the check, and its line ran]  !url.contains("07?page") && !url.contains("=0?") && !url.contains("=1?"),
(F) BRANCH crates/api/src/render.rs:4572:73 False  [the false side fails the check, and its line ran]  !url.contains("07?page") && !url.contains("=0?") && !url.contains("=1?"),
(F) BRANCH crates/api/src/render.rs:4686:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("2026-07") && html.contains("2026-06"));
(F) BRANCH crates/api/src/render.rs:4686:45 False  [the false side fails the check, and its line ran]  assert!(html.contains("2026-07") && html.contains("2026-06"));
(F) BRANCH crates/api/src/render.rs:4842:17 False  [the false side fails the check, and its line ran]  assert!(cell.contains("INE121A01024") && cell.contains("INE121A08PJ0"));
(F) BRANCH crates/api/src/render.rs:4842:50 False  [the false side fails the check, and its line ran]  assert!(cell.contains("INE121A01024") && cell.contains("INE121A08PJ0"));
(F) BRANCH crates/api/src/render.rs:4857:17 False  [the false side fails the check, and its line ran]  assert!(fut.contains("Future") && fut.contains("2026-09-29"));
(F) BRANCH crates/api/src/render.rs:4857:43 False  [the false side fails the check, and its line ran]  assert!(fut.contains("Future") && fut.contains("2026-09-29"));
(F) BRANCH crates/api/src/render.rs:5284:17 False  [the false side fails the check, and its line ran]  assert!(n.contains("index") && n.contains("F&amp;O"));
(F) BRANCH crates/api/src/render.rs:5284:40 False  [the false side fails the check, and its line ran]  assert!(n.contains("index") && n.contains("F&amp;O"));
(F) BRANCH crates/api/src/render.rs:5294:17 False  [the false side fails the check, and its line ran]  assert!(r.contains("F&amp;O") && r.contains("total mkt"));
(F) BRANCH crates/api/src/render.rs:5294:42 False  [the false side fails the check, and its line ran]  assert!(r.contains("F&amp;O") && r.contains("total mkt"));
(F) BRANCH crates/api/src/render.rs:5688:13 False  [the false side fails the check, and its line ran]  output.status.success() && stdout.contains("1 passed"),
(F) BRANCH crates/api/src/render.rs:5688:40 False  [the false side fails the check, and its line ran]  output.status.success() && stdout.contains("1 passed"),
(F) LINE crates/api/src/repair_route.rs:598  [panic/unreachable line]  panic!("an HTTP descriptor");
(F) BRANCH crates/api/src/repair_route.rs:597:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) LINE crates/api/src/research_doors_tests.rs:95  [panic/unreachable line]  panic!("{door} {query}: a refused body is JSON ({why}): {body}");
(F) LINE crates/api/src/server.rs:18907  [failing arm of the statement at 18904, which ran]  began.elapsed()
(F) LINE crates/api/src/server.rs:18969  [failing arm of the statement at 18966, which ran]  String::from_utf8_lossy(&got)
(F) LINE crates/api/src/server.rs:21680  [failing arm of the statement at 21674, which ran]  withheld[0].1
(F) LINE crates/api/src/server.rs:22709  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24030  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24155  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24205  [failing arm of the statement at 24202, which ran]  dir.display()
(F) LINE crates/api/src/server.rs:24211  [panic/unreachable line]  panic!("{receipts:?}");
(F) LINE crates/api/src/server.rs:24214  [panic/unreachable line]  panic!("{segment:?}");
(F) LINE crates/api/src/server.rs:24335  [panic/unreachable line]  panic!("the walk's feed is an HTTP feed");
(F) LINE crates/api/src/server.rs:24369  [panic/unreachable line]  panic!("a walk opened while another process held the vendor");
(F) LINE crates/api/src/server.rs:24374  [panic/unreachable line]  panic!("a spot request opened while another process held the vendor");
(F) LINE crates/api/src/server.rs:24399  [panic/unreachable line]  panic!("a walk took an id from a damaged ledger");
(F) LINE crates/api/src/server.rs:24408  [panic/unreachable line]  panic!("a spot request took an id from a damaged ledger");
(F) LINE crates/api/src/server.rs:24443  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24538  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24581  [failing arm of the statement at 24578, which ran]  away.map(|calendar| calendar.sessions())
(F) LINE crates/api/src/server.rs:24628  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24730  [panic/unreachable line]  panic!("HTTP descriptor")
(F) LINE crates/api/src/server.rs:25516  [failing arm of the statement at 25513, which ran]  root.display()
(F) LINE crates/api/src/server.rs:28789  [panic/unreachable line]  panic!("{feed:?} is HTTP");
(F) LINE crates/api/src/server.rs:29575  [failing arm of the statement at 29570, which ran]  stock.len(),
(F) LINE crates/api/src/server.rs:29576  [failing arm of the statement at 29570, which ran]  index.len()
(F) LINE crates/api/src/server.rs:30661  [panic/unreachable line]  panic!("{} is an HTTP feed and must have a budget", feed.display())
(F) LINE crates/api/src/server.rs:30695  [failing arm of the statement at 30691, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:30740  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) LINE crates/api/src/server.rs:31090  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) LINE crates/api/src/server.rs:31155  [panic/unreachable line]  panic!("Zerodha is an HTTP feed");
(F) LINE crates/api/src/server.rs:31986  [panic/unreachable line]  panic!(
(F) LINE crates/api/src/server.rs:32083  [failing arm of the statement at 32080, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32129  [failing arm of the statement at 32125, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32137  [failing arm of the statement at 32133, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32142  [failing arm of the statement at 32139, which ran]  feed.display(),
(F) LINE crates/api/src/server.rs:32143  [failing arm of the statement at 32139, which ran]  months.len()
(F) LINE crates/api/src/server.rs:32209  [panic/unreachable line]  panic!("{} declares no HTTP transport", feed.display());
(F) LINE crates/api/src/server.rs:32215  [failing arm of the statement at 32211, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32222  [failing arm of the statement at 32217, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32236  [failing arm of the statement at 32232, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32242  [failing arm of the statement at 32238, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32255  [failing arm of the statement at 32252, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32262  [failing arm of the statement at 32258, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32453  [failing arm of the statement at 32446, which ran]  &body[..body.len().min(120)]
(F) LINE crates/api/src/server.rs:32983  [failing arm of the statement at 32980, which ran]  marked[0]
(F) LINE crates/api/src/server.rs:33173  [panic/unreachable line]  panic!(
(F) LINE crates/api/src/server.rs:34454  [failing arm of the statement at 34450, which ran]  got.bodies.iter().map(|(w, _)| *w).collect::<Vec<_>>()
(F) LINE crates/api/src/server.rs:34568  [failing arm of the statement at 34564, which ran]  got.bodies.iter().map(|(w, _)| *w).collect::<Vec<_>>()
(F) LINE crates/api/src/server.rs:34936  [panic/unreachable line]  panic!("this feed authenticates, so its transport is HTTP")
(F) LINE crates/api/src/server.rs:35052  [failing arm of the statement at 35047, which ran]  chunks.len()
(F) LINE crates/api/src/server.rs:35250  [failing arm of the statement at 35245, which ran]  refused.len()
(F) LINE crates/api/src/server.rs:35482  [failing arm of the statement at 35476, which ran]  ledger.fact(1).1
(F) LINE crates/api/src/server.rs:36159  [panic/unreachable line]  panic!("{feed} is an HTTP broker");
(F) LINE crates/api/src/server.rs:36477  [failing arm of the statement at 36473, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:36529  [failing arm of the statement at 36525, which ran]  tf.as_str()
(F) LINE crates/api/src/server.rs:36577  [failing arm of the statement at 36573, which ran]  rung.as_str()
(F) LINE crates/api/src/server.rs:37129  [failing arm of the statement at 37126, which ran]  bucket.word()
(F) BRANCH crates/api/src/server.rs:19172:17 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK"),
(F) BRANCH crates/api/src/server.rs:19172:27 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK"),
(F) BRANCH crates/api/src/server.rs:19316:21 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 408"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19316:31 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 408"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19348:17 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19348:27 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19369:17 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19369:27 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19523:21 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 408"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19523:31 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 408"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19569:21 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19569:31 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:21312:17 False  [the false side fails the check, and its line ran]  why.contains("1day") && why.contains("1 of 1"),
(F) BRANCH crates/api/src/server.rs:21312:41 False  [the false side fails the check, and its line ran]  why.contains("1day") && why.contains("1 of 1"),
(F) BRANCH crates/api/src/server.rs:21765:13 False  [the false side fails the check, and its line ran]  run.months == 1 && run.from == month_of(2026, 7) && run.why == ladder::OwedWhy::Interior
(F) BRANCH crates/api/src/server.rs:21765:32 False  [the false side fails the check, and its line ran]  run.months == 1 && run.from == month_of(2026, 7) && run.why == ladder::OwedWhy::Interior
(F) BRANCH crates/api/src/server.rs:21769:80 False  [the false side fails the check, and its line ran]  ladder::Gate::RungFirst { missing: 13, runs: 13, ref owed, .. } if owed.len() == ladder::OWED_NAMED
(F) BRANCH crates/api/src/server.rs:21792:13 False  [the false side fails the check, and its line ran]  at_close.contains("ends at 15:30 IST and is asked for from 15:35")
(F) BRANCH crates/api/src/server.rs:21793:20 False  [the false side fails the check, and its line ran]  && at_close.contains("it is 15:30 now"),
(F) BRANCH crates/api/src/server.rs:21813:61 False  [the false side fails the check, and its line ran]  assert!(matches!(muhurat, DayKind::Open(session) if session != Session::full()));
(F) BRANCH crates/api/src/server.rs:21946:13 False  [the false side fails the check, and its line ran]  said.starts_with("STORE UNAVAILABLE") && said.contains("after 9 of 208"),
(F) BRANCH crates/api/src/server.rs:21946:54 False  [the false side fails the check, and its line ran]  said.starts_with("STORE UNAVAILABLE") && said.contains("after 9 of 208"),
(F) BRANCH crates/api/src/server.rs:21995:13 False  [the false side fails the check, and its line ran]  line.why.starts_with(pull::calendar::PAST_RECORD_FINDING)
(F) BRANCH crates/api/src/server.rs:21996:20 False  [the false side fails the check, and its line ran]  && line.why.contains(
(F) BRANCH crates/api/src/server.rs:21999:20 False  [the false side fails the check, and its line ran]  && line
(F) BRANCH crates/api/src/server.rs:22402:13 False  [the false side fails the check, and its line ran]  said.starts_with("transient — 0 failure(s) another attempt could change")
(F) BRANCH crates/api/src/server.rs:22403:20 False  [the false side fails the check, and its line ran]  && said.contains("2 of the store and not the vendor")
(F) BRANCH crates/api/src/server.rs:22404:20 False  [the false side fails the check, and its line ran]  && said.contains("and 0 it would repeat"),
(F) BRANCH crates/api/src/server.rs:22698:13 False  [the false side fails the check, and its line ran]  fault.contains(".store-probe") && fault.contains("could not be unlinked"),
(F) BRANCH crates/api/src/server.rs:22698:47 False  [the false side fails the check, and its line ran]  fault.contains(".store-probe") && fault.contains("could not be unlinked"),
(F) BRANCH crates/api/src/server.rs:23578:13 False  [the false side fails the check, and its line ran]  !read.merged.contributed.contains(&Vendor::Groww)
(F) BRANCH crates/api/src/server.rs:23579:20 False  [the false side fails the check, and its line ran]  && !read.merged.contributed.contains(&Vendor::Dhan),
(F) BRANCH crates/api/src/server.rs:24029:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24154:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24210:13 False  [the else of a let-else that panics, whose line ran]  let [(0, segment)] = receipts.as_slice() else {
(F) BRANCH crates/api/src/server.rs:24213:13 False  [the else of a let-else that panics, whose line ran]  let [receipt] = segment.receipts.as_slice() else {
(F) BRANCH crates/api/src/server.rs:24334:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24368:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, body)) = fno_landing(&page, Vec::new(), (&site, &wire, &asked), 1) else {
(F) BRANCH crates/api/src/server.rs:24372:17 False  [the false side fails the check, and its line ran]  assert!(body.contains("run_busy") && body.contains(holder), "{body}");
(F) BRANCH crates/api/src/server.rs:24372:46 False  [the false side fails the check, and its line ran]  assert!(body.contains("run_busy") && body.contains(holder), "{body}");
(F) BRANCH crates/api/src/server.rs:24373:13 False  [the else of a let-else that panics, whose line ran]  let Err((why, code)) = open_spot_landing(&spot, &site, 1) else {
(F) BRANCH crates/api/src/server.rs:24377:17 False  [the false side fails the check, and its line ran]  assert!(why.starts_with("run_busy") && why.contains(holder), "{why}");
(F) BRANCH crates/api/src/server.rs:24377:48 False  [the false side fails the check, and its line ran]  assert!(why.starts_with("run_busy") && why.contains(holder), "{why}");
(F) BRANCH crates/api/src/server.rs:24398:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, body)) = fno_landing(&page, Vec::new(), (&site, &wire, &asked), 1) else {
(F) BRANCH crates/api/src/server.rs:24407:13 False  [the else of a let-else that panics, whose line ran]  let Err((why, code)) = open_spot_landing(&spot, &site, 1) else {
(F) BRANCH crates/api/src/server.rs:24442:17 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24537:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24627:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24729:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:25093:17 False  [the false side fails the check, and its line ran]  assert!(e.contains("unknown argument") && e.contains("usage"), "{e}");
(F) BRANCH crates/api/src/server.rs:25093:51 False  [the false side fails the check, and its line ran]  assert!(e.contains("unknown argument") && e.contains("usage"), "{e}");
(F) BRANCH crates/api/src/server.rs:25149:13 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_MASTERS") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:25149:51 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_MASTERS") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:25256:13 False  [the false side fails the check, and its line ran]  refused.contains("pid=4242") && refused.contains("addr=127.0.0.1:8080"),
(F) BRANCH crates/api/src/server.rs:25256:45 False  [the false side fails the check, and its line ran]  refused.contains("pid=4242") && refused.contains("addr=127.0.0.1:8080"),
(F) BRANCH crates/api/src/server.rs:25452:13 False  [the false side fails the check, and its line ran]  body.contains(concat!("if let Some(warning) = ", "stamped {"))
(F) BRANCH crates/api/src/server.rs:25453:20 False  [the false side fails the check, and its line ran]  && body.contains(concat!(
(F) BRANCH crates/api/src/server.rs:25464:13 False  [the false side fails the check, and its line ran]  refused.contains("os error 28") && refused.contains("os error 5"),
(F) BRANCH crates/api/src/server.rs:25464:48 False  [the false side fails the check, and its line ran]  refused.contains("os error 28") && refused.contains("os error 5"),
(F) BRANCH crates/api/src/server.rs:25620:13 False  [the false side fails the check, and its line ran]  html.contains("appended to") && !html.contains("NOT in the journal"),
(F) BRANCH crates/api/src/server.rs:25620:45 False  [the false side fails the check, and its line ran]  html.contains("appended to") && !html.contains("NOT in the journal"),
(F) BRANCH crates/api/src/server.rs:25812:13 False  [the false side fails the check, and its line ran]  text.contains("INE121A01024") && text.contains("INE121A08PJ0"),
(F) BRANCH crates/api/src/server.rs:25812:46 False  [the false side fails the check, and its line ran]  text.contains("INE121A01024") && text.contains("INE121A08PJ0"),
(F) BRANCH crates/api/src/server.rs:25897:17 False  [the false side fails the check, and its line ran]  assert!(up.contains("NSE-NIFTY") && up.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25897:45 False  [the false side fails the check, and its line ran]  assert!(up.contains("NSE-NIFTY") && up.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25898:17 False  [the false side fails the check, and its line ran]  assert!(down.contains("NSE-NIFTY") && down.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25898:47 False  [the false side fails the check, and its line ran]  assert!(down.contains("NSE-NIFTY") && down.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25902:17 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25902:48 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25918:17 False  [the false side fails the check, and its line ran]  assert!(one.contains("NSE-NIFTY") && one.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25918:46 False  [the false side fails the check, and its line ran]  assert!(one.contains("NSE-NIFTY") && one.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25925:13 False  [the false side fails the check, and its line ran]  past.contains("NSE-NIFTY") && past.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:25925:43 False  [the false side fails the check, and its line ran]  past.contains("NSE-NIFTY") && past.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:25941:17 False  [the false side fails the check, and its line ran]  html.contains("NSE-NIFTY") && html.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:25941:47 False  [the false side fails the check, and its line ran]  html.contains("NSE-NIFTY") && html.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:25957:17 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25957:48 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25999:17 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25999:48 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26021:17 False  [the false side fails the check, and its line ran]  assert!(all.contains("NSE-NIFTY") && all.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26021:46 False  [the false side fails the check, and its line ran]  assert!(all.contains("NSE-NIFTY") && all.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26409:13 False  [the false side fails the check, and its line ran]  reach.contains(r#""feed":"groww""#) && reach.contains(r#""target":"n50""#),
(F) BRANCH crates/api/src/server.rs:26409:52 False  [the false side fails the check, and its line ran]  reach.contains(r#""feed":"groww""#) && reach.contains(r#""target":"n50""#),
(F) BRANCH crates/api/src/server.rs:27108:21 False  [the false side fails the check, and its line ran]  !fine.contains("400 Bad Request") && !fine.contains("reads no feed called"),
(F) BRANCH crates/api/src/server.rs:27108:58 False  [the false side fails the check, and its line ran]  !fine.contains("400 Bad Request") && !fine.contains("reads no feed called"),
(F) BRANCH crates/api/src/server.rs:27158:17 False  [the false side fails the check, and its line ran]  back.contains("2022-02-08") && back.contains("2022-01-08"),
(F) BRANCH crates/api/src/server.rs:27158:48 False  [the false side fails the check, and its line ran]  back.contains("2022-02-08") && back.contains("2022-01-08"),
(F) BRANCH crates/api/src/server.rs:27767:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"), "{html}");
(F) BRANCH crates/api/src/server.rs:27767:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"), "{html}");
(F) BRANCH crates/api/src/server.rs:27786:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("groww") && html.contains("dhan"), "{html}");
(F) BRANCH crates/api/src/server.rs:27786:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("groww") && html.contains("dhan"), "{html}");
(F) BRANCH crates/api/src/server.rs:28036:13 False  [the false side fails the check, and its line ran]  note.contains("debg") && note.contains("notalevel"),
(F) BRANCH crates/api/src/server.rs:28036:38 False  [the false side fails the check, and its line ran]  note.contains("debg") && note.contains("notalevel"),
(F) BRANCH crates/api/src/server.rs:28152:31 False  [the false side fails the check, and its line ran]  .all(|(t, l)| t == "pull" && *l == telemetry::Level::Debug),
(F) BRANCH crates/api/src/server.rs:28157:13 False  [the false side fails the check, and its line ran]  note.contains("DROPPED") && note.contains("api=trace"),
(F) BRANCH crates/api/src/server.rs:28157:41 False  [the false side fails the check, and its line ran]  note.contains("DROPPED") && note.contains("api=trace"),
(F) BRANCH crates/api/src/server.rs:28365:13 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_STORE") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:28365:49 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_STORE") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:28369:13 False  [the false side fails the check, and its line ran]  refused.contains("bars/") && refused.contains("manifest/"),
(F) BRANCH crates/api/src/server.rs:28369:42 False  [the false side fails the check, and its line ran]  refused.contains("bars/") && refused.contains("manifest/"),
(F) BRANCH crates/api/src/server.rs:28788:17 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = descriptor.transport else {
(F) BRANCH crates/api/src/server.rs:28863:13 False  [the false side fails the check, and its line ran]  html.contains("NSE-INDEX-NIFTY") && html.contains("NSE-INDEX-BANKNIFTY"),
(F) BRANCH crates/api/src/server.rs:28863:49 False  [the false side fails the check, and its line ran]  html.contains("NSE-INDEX-NIFTY") && html.contains("NSE-INDEX-BANKNIFTY"),
(F) BRANCH crates/api/src/server.rs:28867:13 False  [the false side fails the check, and its line ran]  html.contains("groww: UNAVAILABLE") && html.contains("dhan: UNAVAILABLE"),
(F) BRANCH crates/api/src/server.rs:28867:52 False  [the false side fails the check, and its line ran]  html.contains("groww: UNAVAILABLE") && html.contains("dhan: UNAVAILABLE"),
(F) BRANCH crates/api/src/server.rs:29060:13 False  [the false side fails the check, and its line ran]  !CREDENTIAL_FACT.is_empty() && CREDENTIAL_FACT.contains("no longer valid"),
(F) BRANCH crates/api/src/server.rs:29060:44 False  [the false side fails the check, and its line ran]  !CREDENTIAL_FACT.is_empty() && CREDENTIAL_FACT.contains("no longer valid"),
(F) BRANCH crates/api/src/server.rs:29116:17 False  [the false side fails the check, and its line ran]  assert!(!plain.reached_wire && !plain.vendor_down && !plain.credential_dead);
(F) BRANCH crates/api/src/server.rs:29116:40 False  [the false side fails the check, and its line ran]  assert!(!plain.reached_wire && !plain.vendor_down && !plain.credential_dead);
(F) BRANCH crates/api/src/server.rs:29116:62 False  [the false side fails the check, and its line ran]  assert!(!plain.reached_wire && !plain.vendor_down && !plain.credential_dead);
(F) BRANCH crates/api/src/server.rs:29123:13 False  [the false side fails the check, and its line ran]  !cred.reached_wire && !cred.vendor_down,
(F) BRANCH crates/api/src/server.rs:29123:35 False  [the false side fails the check, and its line ran]  !cred.reached_wire && !cred.vendor_down,
(F) BRANCH crates/api/src/server.rs:29133:17 False  [the false side fails the check, and its line ran]  assert!(all.reached_wire && all.vendor_down && all.credential_dead);
(F) BRANCH crates/api/src/server.rs:29133:37 False  [the false side fails the check, and its line ran]  assert!(all.reached_wire && all.vendor_down && all.credential_dead);
(F) BRANCH crates/api/src/server.rs:29133:56 False  [the false side fails the check, and its line ran]  assert!(all.reached_wire && all.vendor_down && all.credential_dead);
(F) BRANCH crates/api/src/server.rs:29142:13 False  [the false side fails the check, and its line ran]  real.reached_wire && real.credential_dead && !real.vendor_down,
(F) BRANCH crates/api/src/server.rs:29142:34 False  [the false side fails the check, and its line ran]  real.reached_wire && real.credential_dead && !real.vendor_down,
(F) BRANCH crates/api/src/server.rs:29142:58 False  [the false side fails the check, and its line ran]  real.reached_wire && real.credential_dead && !real.vendor_down,
(F) BRANCH crates/api/src/server.rs:29360:13 False  [the false side fails the check, and its line ran]  words.starts_with("INTERRUPTED: an expired-F&O walk for 2026-07-01..2026-07-30")
(F) BRANCH crates/api/src/server.rs:29361:20 False  [the false side fails the check, and its line ran]  && words.contains("run the same walk again from /pull/fno")
(F) BRANCH crates/api/src/server.rs:29362:20 False  [the false side fails the check, and its line ran]  && !words.contains("/pull/spot")
(F) BRANCH crates/api/src/server.rs:29363:20 False  [the false side fails the check, and its line ran]  && !words.contains("press Pull"),
(F) BRANCH crates/api/src/server.rs:30000:13 False  [the false side fails the check, and its line ran]  whole_body.contains(&format!(r#""expected":{}"#, owed.len()))
(F) BRANCH crates/api/src/server.rs:30001:20 False  [the false side fails the check, and its line ran]  && holed_body.contains(&format!(r#""expected":{}"#, owed.len())),
(F) BRANCH crates/api/src/server.rs:30009:13 False  [the false side fails the check, and its line ran]  whole_body.contains(r#""source":"record""#) && whole_body.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/server.rs:30009:60 False  [the false side fails the check, and its line ran]  whole_body.contains(r#""source":"record""#) && whole_body.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/server.rs:30280:17 False  [the false side fails the check, and its line ran]  damaged.contains("x-brutex-census-note:") && damaged.contains("UNREADABLE"),
(F) BRANCH crates/api/src/server.rs:30280:62 False  [the false side fails the check, and its line ran]  damaged.contains("x-brutex-census-note:") && damaged.contains("UNREADABLE"),
(F) BRANCH crates/api/src/server.rs:30411:13 False  [the false side fails the check, and its line ran]  !clean.contains('\r') && !clean.contains('\n'),
(F) BRANCH crates/api/src/server.rs:30411:38 False  [the false side fails the check, and its line ran]  !clean.contains('\r') && !clean.contains('\n'),
(F) BRANCH crates/api/src/server.rs:30415:13 False  [the false side fails the check, and its line ran]  clean.contains("groww UNREADABLE") && clean.contains("at /tmp/x"),
(F) BRANCH crates/api/src/server.rs:30415:51 False  [the false side fails the check, and its line ran]  clean.contains("groww UNREADABLE") && clean.contains("at /tmp/x"),
(F) BRANCH crates/api/src/server.rs:30739:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:31161:17 False  [the false side fails the check, and its line ran]  assert!(spec.fno.by_name().is_none() && spec.fno.by_offset().is_none());
(F) BRANCH crates/api/src/server.rs:31161:49 False  [the false side fails the check, and its line ran]  assert!(spec.fno.by_name().is_none() && spec.fno.by_offset().is_none());
(F) BRANCH crates/api/src/server.rs:31758:17 False  [the false side fails the check, and its line ran]  body.contains("detached_pull(") && body.contains(held),
(F) BRANCH crates/api/src/server.rs:31758:52 False  [the false side fails the check, and its line ran]  body.contains("detached_pull(") && body.contains(held),
(F) BRANCH crates/api/src/server.rs:31847:13 False  [the false side fails the check, and its line ran]  !source.contains("sort_unstable_by_key") && !source.contains(".sort("),
(F) BRANCH crates/api/src/server.rs:31847:57 False  [the false side fails the check, and its line ran]  !source.contains("sort_unstable_by_key") && !source.contains(".sort("),
(F) BRANCH crates/api/src/server.rs:32208:17 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:32427:26 False  [the false side fails the check, and its line ran]  .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
(F) BRANCH crates/api/src/server.rs:33002:13 False  [the false side fails the check, and its line ran]  code.contains("BAD_REQUEST") && code.contains("BAD_GATEWAY"),
(F) BRANCH crates/api/src/server.rs:33002:45 False  [the false side fails the check, and its line ran]  code.contains("BAD_REQUEST") && code.contains("BAD_GATEWAY"),
(F) BRANCH crates/api/src/server.rs:33006:13 False  [the false side fails the check, and its line ran]  !code.contains("contains(") && !code.contains("starts_with("),
(F) BRANCH crates/api/src/server.rs:33006:44 False  [the false side fails the check, and its line ran]  !code.contains("contains(") && !code.contains("starts_with("),
(F) BRANCH crates/api/src/server.rs:33070:13 False  [the false side fails the check, and its line ran]  !code.contains("15:30") && !code.contains("930"),
(F) BRANCH crates/api/src/server.rs:33070:40 False  [the false side fails the check, and its line ran]  !code.contains("15:30") && !code.contains("930"),
(F) BRANCH crates/api/src/server.rs:33633:13 False  [the false side fails the check, and its line ran]  page.contains(">376</td>") && page.contains(">375</td>"),
(F) BRANCH crates/api/src/server.rs:33633:43 False  [the false side fails the check, and its line ran]  page.contains(">376</td>") && page.contains(">375</td>"),
(F) BRANCH crates/api/src/server.rs:34657:13 False  [the false side fails the check, and its line ran]  note.contains("BEFORE FIRST BAR") && note.contains("2018-01-01..=2023-06-23"),
(F) BRANCH crates/api/src/server.rs:34657:50 False  [the false side fails the check, and its line ran]  note.contains("BEFORE FIRST BAR") && note.contains("2018-01-01..=2023-06-23"),
(F) BRANCH crates/api/src/server.rs:34671:13 False  [the false side fails the check, and its line ran]  failure.contains("NO ROWS AT ALL") && failure.contains("2016-03"),
(F) BRANCH crates/api/src/server.rs:34671:51 False  [the false side fails the check, and its line ran]  failure.contains("NO ROWS AT ALL") && failure.contains("2016-03"),
(F) BRANCH crates/api/src/server.rs:34689:13 False  [the false side fails the check, and its line ran]  failure.contains("1 of 3") && failure.contains(&late.from().to_string()),
(F) BRANCH crates/api/src/server.rs:34689:43 False  [the false side fails the check, and its line ran]  failure.contains("1 of 3") && failure.contains(&late.from().to_string()),
(F) BRANCH crates/api/src/server.rs:35067:17 False  [the false side fails the check, and its line ran]  crate::emitted::counts(record, "chunk", 1)
(F) BRANCH crates/api/src/server.rs:35068:24 False  [the false side fails the check, and its line ran]  && crate::emitted::counts(record, "of", 1),
(F) BRANCH crates/api/src/server.rs:35260:17 False  [the false side fails the check, and its line ran]  crate::emitted::says(record, "instrument", "NIFTY")
(F) BRANCH crates/api/src/server.rs:35261:24 False  [the false side fails the check, and its line ran]  && crate::emitted::says(record, "feed", "dhan"),
(F) BRANCH crates/api/src/server.rs:35265:17 False  [the false side fails the check, and its line ran]  crate::emitted::counts(record, "index", 1)
(F) BRANCH crates/api/src/server.rs:35266:24 False  [the false side fails the check, and its line ran]  && crate::emitted::counts(record, "of", 1),
(F) BRANCH crates/api/src/server.rs:35675:13 False  [the false side fails the check, and its line ran]  !ledger.opened() && ledger.opened_refused.is_some(),
(F) BRANCH crates/api/src/server.rs:35675:33 False  [the false side fails the check, and its line ran]  !ledger.opened() && ledger.opened_refused.is_some(),
(F) BRANCH crates/api/src/server.rs:35876:13 False  [the false side fails the check, and its line ran]  crate::emitted::says(mine, "rung", "1min")
(F) BRANCH crates/api/src/server.rs:35877:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "target", asked.target.label()),
(F) BRANCH crates/api/src/server.rs:35892:13 False  [the false side fails the check, and its line ran]  crate::emitted::counts(mine, "failed", 0)
(F) BRANCH crates/api/src/server.rs:35893:20 False  [the false side fails the check, and its line ran]  && crate::emitted::counts(mine, "bars_stored", 0),
(F) BRANCH crates/api/src/server.rs:35942:13 False  [the false side fails the check, and its line ran]  crate::emitted::says(mine, "month", "2026-08")
(F) BRANCH crates/api/src/server.rs:35943:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "day", "2026-08-04")
(F) BRANCH crates/api/src/server.rs:35944:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "window", "2026-08-03..=2026-08-05")
(F) BRANCH crates/api/src/server.rs:35945:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "why", "the store refused the month"),
(F) BRANCH crates/api/src/server.rs:35997:13 False  [the false side fails the check, and its line ran]  crate::emitted::counts(ordinary, "status", 200)
(F) BRANCH crates/api/src/server.rs:35998:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(ordinary, "method", "GET")
(F) BRANCH crates/api/src/server.rs:35999:20 False  [the false side fails the check, and its line ran]  && ordinary.field("micros").is_some(),
(F) BRANCH crates/api/src/server.rs:36158:17 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:36517:13 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) BRANCH crates/api/src/server.rs:36517:37 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) BRANCH crates/api/src/server.rs:36566:13 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) BRANCH crates/api/src/server.rs:36566:37 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) LINE crates/api/src/server_cloud_mutant_tests_a.rs:392  [panic/unreachable line]  panic!("re-armed above");
(F) LINE crates/api/src/server_cloud_mutant_tests_a.rs:1805  [failing arm of the statement at 1802, which ran]  answered.map(|schedule| schedule.is_some())
(F) LINE crates/api/src/server_cloud_mutant_tests_a.rs:2030  [failing arm of the statement at 2027, which ran]  placed.as_ref().ok()
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:391:9 False  [the else of a let-else that panics, whose line ran]  let HeadState::Awaiting { deadline, .. } = wrapped.state else {
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:491:13 False  [the false side fails the check, and its line ran]  assert!(!note.is_empty() && note != "xyzzy", "{note}");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:491:33 False  [the false side fails the check, and its line ran]  assert!(!note.is_empty() && note != "xyzzy", "{note}");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:2028:69 False  [the false side fails the check, and its line ran]  matches!(placed, Err(Unlocated::NotHeld(ref unreadable)) if unreadable.is_empty()),
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:214  [panic/unreachable line]  panic!("{} is an HTTP feed", feed.display());
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:516  [panic/unreachable line]  panic!("no store rung, so nothing is probed")
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1438  [failing arm of the statement at 1435, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1443  [failing arm of the statement at 1440, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1448  [failing arm of the statement at 1445, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1474  [failing arm of the statement at 1471, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1482  [failing arm of the statement at 1476, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1514  [failing arm of the statement at 1511, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1563  [failing arm of the statement at 1560, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1572  [failing arm of the statement at 1569, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1580  [failing arm of the statement at 1577, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1588  [failing arm of the statement at 1585, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1617  [failing arm of the statement at 1614, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1660  [failing arm of the statement at 1657, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1665  [failing arm of the statement at 1662, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1687  [failing arm of the statement at 1681, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1720  [failing arm of the statement at 1714, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1763  [failing arm of the statement at 1760, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:2330  [failing arm of the statement at 2327, which ran]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:2367  [panic/unreachable line]  panic!("the premise: Zerodha is an HTTP feed");
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:2383  [failing arm of the statement at 2377, which ran]  visible(&body)
(F) BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:213:9 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2366:9 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = asked.feed.descriptor().transport else {
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:1090  [failing arm of the statement at 1087, which ran]  started.elapsed()
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:1712  [panic/unreachable line]  panic!("a recovered census still loads: {:?}", census.state.name());
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:2452  [failing arm of the statement at 2449, which ran]  site.autopilot.json()
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:3211  [panic/unreachable line]  panic!("nothing landed before the refusal");
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:3296  [panic/unreachable line]  panic!("nothing landed before the refusal");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:1711:9 False  [the else of a let-else that panics, whose line ran]  let census::Census::Held { ref manifest } = census.state else {
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:2384:13 False  [the false side fails the check, and its line ran]  !why.contains(VENDOR_DOWN) && !why.contains(WIRE_REACHED),
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:2384:43 False  [the false side fails the check, and its line ran]  !why.contains(VENDOR_DOWN) && !why.contains(WIRE_REACHED),
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:3210:9 False  [the else of a let-else that panics, whose line ran]  let Err(refused) = refused else {
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:3295:13 False  [the else of a let-else that panics, whose line ran]  let Err(refused) = refused else {
(F) LINE crates/api/src/server_cloud_mutant_tests_d.rs:103  [failing arm of the statement at 100, which ran]  answered.map(|schedule| schedule.is_some())
(F) LINE crates/api/src/server_cloud_mutant_tests_d.rs:679  [panic/unreachable line]  panic!("HTTP descriptor");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_d.rs:678:9 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) LINE crates/api/src/server_cloud_mutant_tests_e.rs:261  [panic/unreachable line]  panic!("the flush re-arms the head clock");
(F) LINE crates/api/src/settings_route_tests.rs:561  [panic/unreachable line]  panic!("the snapshot of a store with a FIFO generation waited");
(F) LINE crates/api/src/settings_route_tests.rs:570  [panic/unreachable line]  panic!("another store's snapshot waited behind the FIFO");
(F) LINE crates/api/src/settings_route_tests.rs:577  [panic/unreachable line]  panic!("/settings.json?history=1 waited on the FIFO");
(F) LINE crates/api/src/settings_route_tests.rs:914  [failing arm of the statement at 911, which ran]  path.display()
(F) BRANCH crates/api/src/settings_route_tests.rs:559:9 False  [the else of a let-else that panics, whose line ran]  let Some(stuck_snapshot) = within(move || snapshot(&at)) else {
(F) BRANCH crates/api/src/settings_route_tests.rs:568:9 False  [the else of a let-else that panics, whose line ran]  let Some(clean) = within(move || snapshot(&at)) else {
(F) BRANCH crates/api/src/settings_route_tests.rs:575:9 False  [the else of a let-else that panics, whose line ran]  let Some(answer) = within(move || super::answer(&at, 1)) else {
(F) BRANCH crates/api/src/settings_route_tests.rs:683:9 False  [the false side fails the check, and its line ran]  got.source.contains("is in force") && got.source.contains("would refuse"),
(F) BRANCH crates/api/src/settings_route_tests.rs:683:47 False  [the false side fails the check, and its line ran]  got.source.contains("is in force") && got.source.contains("would refuse"),
(F) BRANCH crates/api/src/settings_route_tests.rs:733:34 False  [the false side fails the check, and its line ran]  .is_some_and(|holes| holes.starts_with("1 generation number(s) missing")
(F) BRANCH crates/api/src/settings_route_tests.rs:792:9 False  [the false side fails the check, and its line ran]  warning.contains("generation 2 was saved") && warning.contains("generation 3"),
(F) BRANCH crates/api/src/settings_route_tests.rs:792:55 False  [the false side fails the check, and its line ran]  warning.contains("generation 2 was saved") && warning.contains("generation 3"),
(F) BRANCH crates/api/src/settings_route_tests.rs:812:9 False  [the false side fails the check, and its line ran]  warning.contains("did not read back") && warning.contains("built on generation 1"),
(F) BRANCH crates/api/src/settings_route_tests.rs:812:50 False  [the false side fails the check, and its line ran]  warning.contains("did not read back") && warning.contains("built on generation 1"),
(F) LINE crates/api/src/sources_tests.rs:977  [failing arm of the statement at 974, which ran]  a["marks"]
(F) LINE crates/api/src/sources_tests.rs:1359  [failing arm of the statement at 1358, which ran]  matches!(
(F) BRANCH crates/api/src/sources_tests.rs:1174:25 False  [the false side fails the check, and its line ran]  .all(|flag| flag != "basis_differs" && flag != "basis_step"),
(F) LINE crates/api/src/speed_tests.rs:930  [failing arm of the statement at 927, which ran]  row_of()
(F) BRANCH crates/api/src/speed_tests.rs:489:9 False  [the false side fails the check, and its line ran]  unknown.starts_with("HTTP/1.1 404 ")
(F) BRANCH crates/api/src/speed_tests.rs:490:16 False  [the false side fails the check, and its line ran]  && unknown.contains("no speed row is named '/no-such-route-anywhere'"),
(F) BRANCH crates/api/src/speed_tests.rs:543:9 False  [the false side fails the check, and its line ran]  empty_what.starts_with("HTTP/1.1 400 ")
(F) BRANCH crates/api/src/speed_tests.rs:544:16 False  [the false side fails the check, and its line ran]  && empty_what.contains("\"written\":false")
(F) BRANCH crates/api/src/speed_tests.rs:545:16 False  [the false side fails the check, and its line ran]  && empty_what.contains("what="),
(F) BRANCH crates/api/src/speed_tests.rs:554:9 False  [the false side fails the check, and its line ran]  too_long.starts_with("HTTP/1.1 413 ") && too_long.contains("\"written\":false"),
(F) BRANCH crates/api/src/speed_tests.rs:554:50 False  [the false side fails the check, and its line ran]  too_long.starts_with("HTTP/1.1 413 ") && too_long.contains("\"written\":false"),
(F) BRANCH crates/api/src/speed_tests.rs:561:9 False  [the false side fails the check, and its line ran]  cross.starts_with("HTTP/1.1 403 ") && !cross.contains("\"written\""),
(F) BRANCH crates/api/src/speed_tests.rs:561:47 False  [the false side fails the check, and its line ran]  cross.starts_with("HTTP/1.1 403 ") && !cross.contains("\"written\""),
(F) BRANCH crates/api/src/speed_tests.rs:583:13 False  [the false side fails the check, and its line ran]  assert!(figures.p50_micros <= figures.p99_micros && figures.p99_micros <= figures.max_micros);
(F) BRANCH crates/api/src/speed_tests.rs:583:57 False  [the false side fails the check, and its line ran]  assert!(figures.p50_micros <= figures.p99_micros && figures.p99_micros <= figures.max_micros);
(F) BRANCH crates/api/src/speed_tests.rs:696:53 False  [the false side fails the check, and its line ran]  (Some(i), Some(t), Some(d), Some(r)) if i < d && t < d && d < r
(F) BRANCH crates/api/src/speed_tests.rs:696:62 False  [the false side fails the check, and its line ran]  (Some(i), Some(t), Some(d), Some(r)) if i < d && t < d && d < r
(F) BRANCH crates/api/src/speed_tests.rs:696:71 False  [the false side fails the check, and its line ran]  (Some(i), Some(t), Some(d), Some(r)) if i < d && t < d && d < r
(F) BRANCH crates/api/src/speed_tests.rs:715:53 False  [the false side fails the check, and its line ran]  (Some(g), Some(e), Some(a), Some(b)) if g < e && e < a && a < b
(F) BRANCH crates/api/src/speed_tests.rs:715:62 False  [the false side fails the check, and its line ran]  (Some(g), Some(e), Some(a), Some(b)) if g < e && e < a && a < b
(F) BRANCH crates/api/src/speed_tests.rs:715:71 False  [the false side fails the check, and its line ran]  (Some(g), Some(e), Some(a), Some(b)) if g < e && e < a && a < b
(F) BRANCH crates/api/src/speed_tests.rs:716:14 False  [the false side fails the check, and its line ran]  ) && main.contains(concat!("run_held(&args, ", "stop, &mut held)")),
(F) BRANCH crates/api/src/speed_tests.rs:724:54 False  [the false side fails the check, and its line ran]  matches!((ran, ended), (Some(r), Some(e)) if r < e),
(F) BRANCH crates/api/src/speed_tests.rs:745:61 False  [the false side fails the check, and its line ran]  matches!((graceful, flushed), (Some(g), Some(f)) if g < f),
(F) BRANCH crates/api/src/speed_tests.rs:819:74 False  [the false side fails the check, and its line ran]  matches!((armed, raced, flushed), (Some(a), Some(r), Some(f)) if a < r && r < f),
(F) BRANCH crates/api/src/speed_tests.rs:819:83 False  [the false side fails the check, and its line ran]  matches!((armed, raced, flushed), (Some(a), Some(r), Some(f)) if a < r && r < f),
(F) BRANCH crates/api/src/speed_tests.rs:895:78 False  [the false side fails the check, and its line ran]  matches!((started, landed, finished), (Some(s), Some(l), Some(f)) if s < l && l < f),
(F) BRANCH crates/api/src/speed_tests.rs:895:87 False  [the false side fails the check, and its line ran]  matches!((started, landed, finished), (Some(s), Some(l), Some(f)) if s < l && l < f),
(F) BRANCH crates/api/src/speed_tests.rs:976:75 False  [the false side fails the check, and its line ran]  matches!((shared, scrub, counted), (Some(a), Some(b), Some(c)) if a < b && b < c),
(F) BRANCH crates/api/src/speed_tests.rs:976:84 False  [the false side fails the check, and its line ran]  matches!((shared, scrub, counted), (Some(a), Some(b), Some(c)) if a < b && b < c),
(F) BRANCH crates/api/src/spot_route_tests.rs:481:9 False  [the false side fails the check, and its line ran]  fault.contains("2025-05.bin") && fault.contains(&format!("t={t0}:")),
(F) BRANCH crates/api/src/spot_route_tests.rs:481:42 False  [the false side fails the check, and its line ran]  fault.contains("2025-05.bin") && fault.contains(&format!("t={t0}:")),
(F) BRANCH crates/api/src/spot_route_tests.rs:531:9 False  [the false side fails the check, and its line ran]  bars_route.contains("crate::detail::Named::StoreRead")
(F) BRANCH crates/api/src/spot_route_tests.rs:532:16 False  [the false side fails the check, and its line ran]  && bars_route.contains("crate::detail::cancellable_in(named, work).await"),
(F) BRANCH crates/api/src/spot_route_tests.rs:541:9 False  [the false side fails the check, and its line ran]  greeks.contains("super::bars_route::cancellable(false, move |cancel| {")
(F) BRANCH crates/api/src/spot_route_tests.rs:542:16 False  [the false side fails the check, and its line ran]  && greeks.contains("RateTable::current(std::env::var_os(RATE_ENV))"),
(F) BRANCH crates/api/src/spot_route_tests.rs:550:9 False  [the false side fails the check, and its line ran]  body.contains("a spot read") && body.contains("retry"),
(F) BRANCH crates/api/src/spot_route_tests.rs:550:41 False  [the false side fails the check, and its line ran]  body.contains("a spot read") && body.contains("retry"),
(F) LINE crates/api/src/stop_tests.rs:567  [failing arm of the statement at 566, which ran]  matches!(
(F) LINE crates/api/src/stop_tests.rs:575  [failing arm of the statement at 574, which ran]  matches!(
(F) LINE crates/api/src/stop_tests.rs:587  [panic/unreachable line]  panic!("a failed stop future is a stop, not a load");
(F) LINE crates/api/src/stop_tests.rs:603  [panic/unreachable line]  panic!("a load that panicked is a failure");
(F) BRANCH crates/api/src/stop_tests.rs:203:17 False  [the false side fails the check, and its line ran]  !log_is_free(&root) && !store_is_free(&root) && !port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:203:40 False  [the false side fails the check, and its line ran]  !log_is_free(&root) && !store_is_free(&root) && !port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:203:65 False  [the false side fails the check, and its line ran]  !log_is_free(&root) && !store_is_free(&root) && !port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:332:9 False  [the false side fails the check, and its line ran]  store_is_free(&root) && port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:332:33 False  [the false side fails the check, and its line ran]  store_is_free(&root) && port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:354:13 False  [the false side fails the check, and its line ran]  assert!(store_is_free(&root) && port_is_free(addr));
(F) BRANCH crates/api/src/stop_tests.rs:354:37 False  [the false side fails the check, and its line ran]  assert!(store_is_free(&root) && port_is_free(addr));
(F) BRANCH crates/api/src/stop_tests.rs:586:9 False  [the else of a let-else that panics, whose line ran]  let Loading::Stopped(failed) = loaded_or_stopped(|| 7_u8, &mut broken).await else {
(F) BRANCH crates/api/src/stop_tests.rs:641:9 False  [the false side fails the check, and its line ran]  race < load && load < awaited,
(F) BRANCH crates/api/src/stop_tests.rs:641:24 False  [the false side fails the check, and its line ran]  race < load && load < awaited,
(F) BRANCH crates/api/src/store_route.rs:1778:17 False  [the false side fails the check, and its line ran]  why.starts_with("behind: ") && why.contains("2019-11-15"),
(F) BRANCH crates/api/src/store_route.rs:1778:48 False  [the false side fails the check, and its line ran]  why.starts_with("behind: ") && why.contains("2019-11-15"),
(F) BRANCH crates/api/src/store_route.rs:1880:13 False  [the false side fails the check, and its line ran]  parts.starts_with("INDEX=")
(F) BRANCH crates/api/src/store_route.rs:1881:20 False  [the false side fails the check, and its line ran]  && parts.contains(",CASH=")
(F) BRANCH crates/api/src/store_route.rs:1882:20 False  [the false side fails the check, and its line ran]  && parts.ends_with(&format!(",FNO={}", fno.header().generation)),
(F) BRANCH crates/api/src/store_route.rs:1902:17 False  [the false side fails the check, and its line ran]  assert!(why.starts_with("capped: ") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/store_route.rs:1902:48 False  [the false side fails the check, and its line ran]  assert!(why.starts_with("capped: ") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/store_route.rs:2575:13 False  [the false side fails the check, and its line ran]  one.starts_with('"') && one.ends_with('"'),
(F) BRANCH crates/api/src/store_route.rs:2575:37 False  [the false side fails the check, and its line ran]  one.starts_with('"') && one.ends_with('"'),
(F) LINE crates/api/src/store_wire.rs:347  [panic/unreachable line]  panic!("warm read rebuilt")
(F) LINE crates/api/src/store_wire.rs:403  [panic/unreachable line]  panic!("GDFL's body was dropped by another vendor's read")
(F) LINE crates/api/src/strict_sweep_tests.rs:299  [failing arm of the statement at 296, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:4571  [failing arm of the statement at 4568, which ran]  String::from_utf8_lossy(&result.stderr)
(F) LINE crates/api/src/sweeprun.rs:4824  [failing arm of the statement at 4821, which ran]  poll_text.len()
(F) LINE crates/api/src/sweeprun.rs:5034  [failing arm of the statement at 5031, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5106  [failing arm of the statement at 5103, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5240  [failing arm of the statement at 5237, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5245  [failing arm of the statement at 5242, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5454  [failing arm of the statement at 5450, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5533  [failing arm of the statement at 5529, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5580  [failing arm of the statement at 5576, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:6541  [failing arm of the statement at 6537, which ran]  refusal.why()
(F) LINE crates/api/src/sweeprun.rs:6783  [failing arm of the statement at 6779, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:6789  [failing arm of the statement at 6785, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:6818  [failing arm of the statement at 6814, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:6888  [failing arm of the statement at 6885, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:6894  [failing arm of the statement at 6891, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7037  [failing arm of the statement at 7033, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7042  [failing arm of the statement at 7039, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7076  [failing arm of the statement at 7072, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7112  [failing arm of the statement at 7108, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7148  [failing arm of the statement at 7145, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7198  [failing arm of the statement at 7194, which ran]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7203  [failing arm of the statement at 7200, which ran]  why.why()
(F) BRANCH crates/api/src/sweeprun.rs:4726:17 False  [the false side fails the check, and its line ran]  assert!(installed.in_flight() && installed.attempt == 1);
(F) BRANCH crates/api/src/sweeprun.rs:4726:42 False  [the false side fails the check, and its line ran]  assert!(installed.in_flight() && installed.attempt == 1);
(F) BRANCH crates/api/src/sweeprun.rs:4775:17 False  [the false side fails the check, and its line ran]  !body.contains("sweep.lock()") && !body.contains(".sweep\n"),
(F) BRANCH crates/api/src/sweeprun.rs:4775:51 False  [the false side fails the check, and its line ran]  !body.contains("sweep.lock()") && !body.contains(".sweep\n"),
(F) BRANCH crates/api/src/sweeprun.rs:5227:38 False  [the false side fails the check, and its line ran]  .all(|(asked, name)| *asked != "screen_budget_ms"
(F) BRANCH crates/api/src/sweeprun.rs:6539:24 False  [the false side fails the check, and its line ran]  || refusal.why().contains("No engine work was started"),
(F) BRANCH crates/api/src/sweeprun.rs:7635:13 False  [the false side fails the check, and its line ran]  done.launch_clear && !done.uncertain && !done.in_flight,
(F) BRANCH crates/api/src/sweeprun.rs:7635:34 False  [the false side fails the check, and its line ran]  done.launch_clear && !done.uncertain && !done.in_flight,
(F) BRANCH crates/api/src/sweeprun.rs:7635:53 False  [the false side fails the check, and its line ran]  done.launch_clear && !done.uncertain && !done.in_flight,
(F) BRANCH crates/api/src/sweeprun.rs:7649:13 False  [the false side fails the check, and its line ran]  running.in_flight && !running.launch_clear,
(F) BRANCH crates/api/src/sweeprun.rs:7649:34 False  [the false side fails the check, and its line ran]  running.in_flight && !running.launch_clear,
(F) BRANCH crates/api/src/sweeprun.rs:7660:17 False  [the false side fails the check, and its line ran]  assert!(lost.uncertain && !lost.launch_clear, "{}", lost.body);
(F) BRANCH crates/api/src/sweeprun.rs:7660:35 False  [the false side fails the check, and its line ran]  assert!(lost.uncertain && !lost.launch_clear, "{}", lost.body);
(F) BRANCH crates/api/src/sweeprun.rs:7729:17 False  [the false side fails the check, and its line ran]  assert!(clear.launch_clear && !clear.uncertain, "{}", clear.body);
(F) BRANCH crates/api/src/sweeprun.rs:7729:39 False  [the false side fails the check, and its line ran]  assert!(clear.launch_clear && !clear.uncertain, "{}", clear.body);
(F) BRANCH crates/api/src/sweeprun.rs:7818:13 False  [the false side fails the check, and its line ran]  why.starts_with(
(F) BRANCH crates/api/src/sweeprun.rs:7820:18 False  [the false side fails the check, and its line ran]  ) && why.contains("so it is running or was stopped without logging one.")
(F) BRANCH crates/api/src/sweeprun.rs:7821:20 False  [the false side fails the check, and its line ran]  && why.contains(
(F) LINE crates/api/src/sweeprun_status_cache_tests.rs:28  [panic/unreachable line]  unreachable!("u32 suffixes are not exhausted by one test")
(F) BRANCH crates/api/src/trades.rs:748:13 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) BRANCH crates/api/src/trades.rs:748:32 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) LINE crates/api/src/verify.rs:397  [failing arm of the statement at 394, which ran]  empty.say()
(F) LINE crates/api/src/verify.rs:427  [failing arm of the statement at 423, which ran]  bad.say()
(F) LINE crates/api/tests/cloud_mutants_bars.rs:464  [panic/unreachable line]  panic!("February is owed: {gated:?}");
(F) LINE crates/api/tests/cloud_mutants_bars.rs:573  [panic/unreachable line]  panic!("every month is owed: {gated:?}");
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:899:13 False  [the false side fails the check, and its line ran]  none.starts_with("too_many_series") && none.contains("0 were named"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:899:52 False  [the false side fails the check, and its line ran]  none.starts_with("too_many_series") && none.contains("0 were named"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2069:13 False  [the false side fails the check, and its line ran]  unopened[0].contains("2025-04") && unopened[0].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2069:48 False  [the false side fails the check, and its line ran]  unopened[0].contains("2025-04") && unopened[0].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2073:13 False  [the false side fails the check, and its line ran]  unopened[1].contains("2025-05") && unopened[1].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2073:48 False  [the false side fails the check, and its line ran]  unopened[1].contains("2025-05") && unopened[1].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2307:13 False  [the false side fails the check, and its line ran]  got.faults[1].starts_with("2025-06: ") && got.faults[1].contains("not a regular file"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2307:55 False  [the false side fails the check, and its line ran]  got.faults[1].starts_with("2025-06: ") && got.faults[1].contains("not a regular file"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2602:13 False  [the false side fails the check, and its line ran]  why.contains(&format!("{} is not a regular file", srt.display()))
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2603:20 False  [the false side fails the check, and its line ran]  && why.starts_with("NSE-INDEX-NIFTY 2025-05: 3 rows read: "),
(F) LINE crates/api/tests/pool_load.rs:382  [failing arm of the statement at 379, which ran]  strange.iter().take(5).collect::<Vec<_>>()
(F) LINE crates/api/tests/pool_load.rs:564  [failing arm of the statement at 561, which ran]  other
(F) LINE crates/api/tests/pool_load.rs:565  [failing arm of the statement at 561, which ran]  .iter()
(F) LINE crates/api/tests/pool_load.rs:566  [failing arm of the statement at 561, which ran]  .take(3)
(F) LINE crates/api/tests/pool_load.rs:567  [failing arm of the statement at 561, which ran]  .map(|(s, _, b)| (s, b.chars().take(400).collect::<String>()))
(F) LINE crates/api/tests/pool_load.rs:568  [failing arm of the statement at 561, which ran]  .collect::<Vec<_>>()
(F) LINE crates/api/tests/pool_load.rs:710  [failing arm of the statement at 707, which ran]  &strange[..strange.len().min(3)]
(F) LINE crates/api/tests/pool_load.rs:810  [failing arm of the statement at 807, which ran]  &strange[..strange.len().min(3)]
(F) BRANCH crates/api/tests/pool_load.rs:579:9 False  [the false side fails the check, and its line ran]  served > 0 && holds > 0,
(F) BRANCH crates/api/tests/pool_load.rs:579:23 False  [the false side fails the check, and its line ran]  served > 0 && holds > 0,
(F) BRANCH crates/api/tests/pool_load.rs:716:17 False  [the false side fails the check, and its line ran]  body.contains("log read not admitted") && body.contains("run at once"),
(F) BRANCH crates/api/tests/pool_load.rs:716:59 False  [the false side fails the check, and its line ran]  body.contains("log read not admitted") && body.contains("run at once"),

(U) FILE crates/api/src/lib.rs  [no function: mod declarations, attributes and comments only; no instrumented region exists]
`````

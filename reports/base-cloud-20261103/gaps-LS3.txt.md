# gaps-LS3: the uncovered rows of lake, indicators and vocab

One fenced block: the list as plain text (CLOUD.md item 10).

`````text
BASE-COV-LIBS-LS3 Route B on tip 68d588fc34f6e327219ddc26b420b22f7eb44439: crates lake, indicators, vocab, every .rs file whole (SET OVERRIDE).
Pinned nightly rustc 1.99.0-nightly (1a98b1e13 2026-08-07), llvm-cov/llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly, Linux x86_64 (the Mac re-measures).
NOT READ by the route: crates/{lake,indicators,vocab}/benches/ratio.rs ([[bench]] test = false: cargo test --no-run builds none; L-BCL-1) and crates/lake/src/lib.rs (module declarations only).

TOTALS per crate (lines covered/counted; branch sides covered/counted; files read)
  indicators  lines 16909/16914 (uncovered 5)  branches 1892/2078 (uncovered 186)  files 39
  lake        lines 5678/6135 (uncovered 457)  branches 305/364 (uncovered 59)  files 17
  vocab       lines 7725/7819 (uncovered 94)  branches 1131/1280 (uncovered 149)  files 19

Uncovered rows 950: (F) 415, (U) 4, NEEDS THE MAC (#[ignore]d real-lake tests, L-BCL-3) 353, GAP 178.

PER FILE (lines a/b, branches c/d; the marks count what is left in that file)
  crates/indicators/src/anchored.rs lines 1682/1682 branches 103/106
  crates/indicators/src/column.rs lines 1244/1246 branches 62/70
  crates/indicators/src/column_cover_tests.rs lines 28/28 branches 0/0
  crates/indicators/src/daily.rs lines 695/695 branches 61/86
  crates/indicators/src/evaluator.rs lines 1705/1705 branches 136/146
  crates/indicators/src/evaluator_cover_tests.rs lines 55/55 branches 0/0
  crates/indicators/src/fib.rs lines 407/407 branches 24/26
  crates/indicators/src/gap.rs lines 724/724 branches 56/58
  crates/indicators/src/gap_cover_tests.rs lines 39/39 branches 0/0
  crates/indicators/src/lib.rs lines 773/773 branches 54/54
  crates/indicators/src/orb.rs lines 470/470 branches 51/58
  crates/indicators/src/orb_cover_tests.rs lines 62/62 branches 0/0
  crates/indicators/src/pattern.rs lines 1399/1399 branches 587/594
  crates/indicators/src/session.rs lines 698/698 branches 152/194
  crates/indicators/src/session_cover_tests.rs lines 26/26 branches 0/0
  crates/indicators/src/trend.rs lines 1257/1257 branches 161/168
  crates/indicators/src/trend_cover_tests.rs lines 42/42 branches 0/0
  crates/indicators/src/vwap.rs lines 951/951 branches 85/94
  crates/indicators/src/vwap_cover_tests.rs lines 35/35 branches 0/0
  crates/indicators/tests/audit_readiness.rs lines 49/49 branches 0/0
  crates/indicators/tests/crossing_known_readiness.rs lines 178/178 branches 37/38
  crates/indicators/tests/current_day_fib_known_readiness.rs lines 206/206 branches 20/22
  crates/indicators/tests/evaluator_position_count.rs lines 26/26 branches 0/0
  crates/indicators/tests/exact_minute_orb_gap_readiness.rs lines 355/355 branches 23/30
  crates/indicators/tests/extremes.rs lines 462/462 branches 14/14
  crates/indicators/tests/fib_known_readiness.rs lines 295/295 branches 20/24
  crates/indicators/tests/fib_rung_rounding.rs lines 182/182 branches 16/16
  crates/indicators/tests/gap_known_readiness.rs lines 337/337 branches 20/24
  crates/indicators/tests/invariants.rs lines 213/213 branches 0/0
  crates/indicators/tests/module_doc_counts.rs lines 211/214 branches 28/28
  crates/indicators/tests/orb_known_readiness.rs lines 259/259 branches 21/24
  crates/indicators/tests/session_from_outside.rs lines 186/186 branches 2/4
  crates/indicators/tests/session_known_readiness.rs lines 394/394 branches 33/46
  crates/indicators/tests/shared_core_doc.rs lines 184/184 branches 2/2
  crates/indicators/tests/sweep_predicate_readiness.rs lines 292/292 branches 72/96
  crates/indicators/tests/time_known_readiness.rs lines 145/145 branches 6/6
  crates/indicators/tests/trend_known_readiness.rs lines 306/306 branches 26/30
  crates/indicators/tests/withheld_cut_fold.rs lines 133/133 branches 8/8
  crates/indicators/tests/withheld_fold.rs lines 204/204 branches 12/12
  crates/lake/src/bar.rs lines 96/98 branches 2/2
  crates/lake/src/batch.rs lines 192/192 branches 12/12
  crates/lake/src/contract.rs lines 349/356 branches 25/26
  crates/lake/src/error.rs lines 383/383 branches 2/2
  crates/lake/src/footer.rs lines 190/190 branches 36/36
  crates/lake/src/footer_tests.rs lines 150/154 branches 5/6
  crates/lake/src/page.rs lines 639/644 branches 38/48
  crates/lake/src/reader.rs lines 1102/1135 branches 46/48
  crates/lake/src/reader_cover_tests.rs lines 31/33 branches 0/0
  crates/lake/src/schema.rs lines 205/206 branches 29/30
  crates/lake/src/schema_cover_tests.rs lines 76/78 branches 2/2
  crates/lake/src/zstd_v082_frames.rs lines 70/71 branches 8/8
  crates/lake/tests/events.rs lines 194/197 branches 18/18
  crates/lake/tests/real_lake.rs lines 0/132 branches 0/18  MAC 150
  crates/lake/tests/real_lake_regression.rs lines 0/126 branches 0/16  MAC 142
  crates/lake/tests/refusals.rs lines 1154/1269 branches 58/68  MAC 61
  crates/lake/tests/synthetic.rs lines 847/871 branches 24/24
  crates/vocab/src/error.rs lines 103/103 branches 0/0
  crates/vocab/src/expression.rs lines 553/554 branches 78/78  GAP 1
  crates/vocab/src/expression_search.rs lines 459/459 branches 86/96
  crates/vocab/src/implication.rs lines 157/158 branches 25/26
  crates/vocab/src/lib.rs lines 55/55 branches 0/0
  crates/vocab/src/mask.rs lines 273/273 branches 14/18
  crates/vocab/src/table.rs lines 434/435 branches 53/60
  crates/vocab/src/tolerance.rs lines 171/171 branches 17/18
  crates/vocab/tests/expression.rs lines 382/383 branches 24/26
  crates/vocab/tests/expression_search.rs lines 219/221 branches 10/12  GAP 1
  crates/vocab/tests/expression_search_readiness.rs lines 122/123 branches 1/2
  crates/vocab/tests/implication_pointers.rs lines 72/72 branches 2/2
  crates/vocab/tests/mask.rs lines 207/208 branches 17/20  GAP 3
  crates/vocab/tests/no_float.rs lines 63/63 branches 4/6  GAP 1
  crates/vocab/tests/no_py_language.rs lines 458/480 branches 52/58  GAP 20
  crates/vocab/tests/no_runtime_spawn.rs lines 2531/2586 branches 605/694  GAP 141
  crates/vocab/tests/stale_claims.rs lines 287/288 branches 14/14
  crates/vocab/tests/table.rs lines 498/501 branches 55/68
  crates/vocab/tests/workspace_is_rust.rs lines 681/686 branches 74/82  GAP 11

GAP ROWS (every uncovered line or side that is neither (F), (U) nor the Mac's):
GAP production  LINE   crates/vocab/src/expression.rs:440  )?,
GAP test code  LINE   crates/vocab/tests/expression_search.rs:74  Step::Paused => assert!(work < 5_000_000, "finite fixture exceeded work ceiling"),
GAP test code  LINE   crates/vocab/tests/mask.rs:312  assert!(before);
GAP assert! condition: an uncovered PASSING side (a clause no case reaches)  BRANCH crates/vocab/tests/mask.rs:307:17 True  before || !after,
GAP test code  BRANCH crates/vocab/tests/mask.rs:311:16 True  if after {
GAP test code  BRANCH crates/vocab/tests/no_float.rs:178:16 False  if std::path::Path::new(&name)
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:264  Err(why) => {
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:265  let how = format!(
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:269  return (walk(root), how);
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:424  println!(
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:427  found.absent.len(),
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:428  found
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:429  .absent
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:430  .iter()
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:431  .take(10)
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:432  .map(|path| masked(path.as_bytes()))
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:433  .collect::<Vec<_>>()
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:639  println!("SKIPPING the git half: `git` cannot be run here ({why}).");
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:640  return;
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:747  println!("SKIPPING: `git` cannot be run here ({why}).");
GAP test code  LINE   crates/vocab/tests/no_py_language.rs:748  return;
GAP test code  BRANCH crates/vocab/tests/no_py_language.rs:238:21 True  name == ".git" || name == "node_modules" || (at_root && name == "target");
GAP test code  BRANCH crates/vocab/tests/no_py_language.rs:296:21 False  Err(why) if why.kind() == std::io::ErrorKind::NotFound => return None,
GAP test code  BRANCH crates/vocab/tests/no_py_language.rs:423:8 True  if !found.absent.is_empty() {
GAP test code  BRANCH crates/vocab/tests/no_py_language.rs:638:12 True  if let Err(why) = git(&repo, &["--version"]) {
GAP test code  BRANCH crates/vocab/tests/no_py_language.rs:746:12 True  if let Err(why) = git(&repo, &["--version"]) {
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:422  (Some('/'), Some('/'), third) => {
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:423  if third == Some('!') || (third == Some('/') && at(i + 3) != Some('/')) {
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:424  break;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:425  }
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:426  while at(i).is_some_and(|x| x != '\n') {
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:427  i += 1;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:428  }
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:435  _ => break,
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:538  Err(_) => value.push('x'),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:553  None => value.push('u'),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:562  *line += 1;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:664  None
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:672  return (false, i + 1);
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:779  return true;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:834  k += 1;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:835  if matches!(t.get(k).map(|x| &x.tok), Some(Tok::Str(_))) {
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:836  k += 1;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:837  }
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:864  false
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:964  k += 1;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:965  if punct(t, k, '(') {
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:966  k = matching(t, k).map_or(t.len(), |close| close + 1);
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:967  }
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1027  i
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1041  k += 1;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1042  if punct(t, k, '(') {
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1043  k = matching(t, k).map_or(t.len(), |close| close + 1);
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1044  }
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1064  return i;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1078  i
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1189  Tok::Punct('$') => Some("imports from the process module through a macro variable"),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1367  _ => {}
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1599  return None;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1678  (Some(_), None) => {}
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1700  out.record(test, line, at.function(), what);
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1701  }
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1953  k += 1;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1954  if punct(item, k, '(') {
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1955  k = matching(item, k).map_or(item.len(), |close| close + 1);
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:1956  }
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2067  line += 1;
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2070  2
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2112  None | Some('\n') => return (Toml::Unread, i),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2116  Some('b') => (Some('\u{8}'), 2),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2117  Some('t') => (Some('\t'), 2),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2118  Some('n') => (Some('\n'), 2),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2119  Some('f') => (Some('\u{c}'), 2),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2120  Some('r') => (Some('\r'), 2),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2121  Some('e') => (Some('\u{1b}'), 2),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2123  Some('U') => (hex(i + 2, 8), 10),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2124  other => (other, 2),
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2208  _ => {}
GAP test code  LINE   crates/vocab/tests/no_runtime_spawn.rs:2287  | ["crates", _, "src", "bin", _, "main.rs"]
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:423:20 True  if third == Some('!') || (third == Some('/') && at(i + 3) != Some('/')) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:423:20 False  if third == Some('!') || (third == Some('/') && at(i + 3) != Some('/')) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:423:43 True  if third == Some('!') || (third == Some('/') && at(i + 3) != Some('/')) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:423:43 False  if third == Some('!') || (third == Some('/') && at(i + 3) != Some('/')) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:423:65 True  if third == Some('!') || (third == Some('/') && at(i + 3) != Some('/')) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:423:65 False  if third == Some('!') || (third == Some('/') && at(i + 3) != Some('/')) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:426:23 True  while at(i).is_some_and(|x| x != '\n') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:426:23 False  while at(i).is_some_and(|x| x != '\n') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:431:27 True  let doc = third == Some('!')
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:432:25 True  || (third == Some('*') && !matches!(at(i + 3), Some('*' | '/')));
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:434:35 False  Some(next) if !doc => i = next,
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:490:29 False  ("r", Some('#')) if hashes == 1 && get(i + 1).is_some_and(is_ident_start) => {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:490:44 False  ("r", Some('#')) if hashes == 1 && get(i + 1).is_some_and(is_ident_start) => {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:541:28 False  'u' if c.get(i) == Some(&'{') => {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:561:32 True  if skipped == '\n' {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:671:9 False  let Some(name) = ident(t.get(i)) else {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:682:11 False  while k < t.len() && !punct(t, k, ')') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:692:18 False  "any" => !members.is_empty() && members.iter().all(|&m| m),
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:704:32 False  keyword(attr, 0, "cfg") && punct(attr, 1, '(') && predicate(attr, 2).0
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:770:42 False  if !(keyword(attr, 0, "cfg_attr") && punct(attr, 1, '(')) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:778:12 True  if !punct(attr, k, ',') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:784:44 True  while k < close && !(depth == 0 && punct(attr, k, ',')) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:792:12 False  if k > from && may_be_macro(&attr[from..k], rebinds_test, rebinds_derive) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:833:19 True  } else if keyword(t, k, "extern") {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:855:28 False  return punct(t, k, '[')
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:856:29 True  && (hash(before)
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:857:33 False  || (before.is_some_and(|at| punct(t, at, '!'))
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:926:71 False  Some(Tok::Str(file)) if ident(attr.get(k)) == Some("path") && punct(attr, k + 1, '=') => {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:963:8 True  if keyword(t, k, "pub") {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:965:12 True  if punct(t, k, '(') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:965:12 False  if punct(t, k, '(') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:972:41 False  && (punct(t, k + 2, '{') || punct(t, k + 2, '<') || keyword(t, k + 2, "where")))
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:972:65 True  && (punct(t, k + 2, '{') || punct(t, k + 2, '<') || keyword(t, k + 2, "where")))
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:972:65 False  && (punct(t, k + 2, '{') || punct(t, k + 2, '<') || keyword(t, k + 2, "where")))
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:973:38 True  || (keyword(t, k, "auto") && keyword(t, k + 1, "trait"))
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1000:15 False  while let Some(tok) = t.get(i) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1013:71 True  Tok::Punct(',') if depth == 0 && (!item || (angle == 0 && !clause)) => return i + 1,
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1021:46 False  Tok::Punct('|') if depth == 0 && !item => return i,
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1040:8 True  if keyword(t, k, "pub") {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1042:12 True  if punct(t, k, '(') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1042:12 False  if punct(t, k, '(') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1047:12 False  && !fn_item(t, k)
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1048:12 False  && !punct(t, k + 1, '{'))
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1049:12 True  || keyword(t, k, "type")
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1050:12 True  || keyword(t, k, "use")
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1051:13 True  || (keyword(t, k, "extern") && keyword(t, k + 1, "crate"));
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1056:15 False  while let Some(tok) = t.get(i) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1063:20 True  if depth == 0 {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1103:26 True  if !is_type(t, i) || i == 0 {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1106:21 False  let joined = if i >= 2 && path_sep(t, i - 2) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1107:26 False  let plain_name = i >= 3 && ident(t.get(i - 3)).is_some() && !(i >= 4 && variable(t, i - 4));
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1107:71 False  let plain_name = i >= 3 && ident(t.get(i - 3)).is_some() && !(i >= 4 && variable(t, i - 4));
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1108:26 False  !(plain_name || (i >= 3 && punct(t, i - 3, '>')))
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1110:65 False  matches!(t[i - 1].tok, Tok::Punct('*' | '+' | '?')) || (i >= 2 && variable(t, i - 2))
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1464:9 False  self.pending.is_none()
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1551:19 False  while let Some(tok) = t.get(k) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1565:15 False  while let Some(tok) = t.get(k) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1598:8 True  if !punct(t, open, '[') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1615:18 False  let pasted = includes(t, i) && matching(t, i + 2) == Some(i + 4);
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1617:27 False  Tok::Str(file) if pasted => Some(file),
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1706:26 True  if path.is_some() || !also.is_empty() {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1784:16 False  && statement
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1796:24 True  out.record(test || gate_here, line, at.function(), what);
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1798:58 False  if attrs.macro_use && keyword(t, i, "extern") && keyword(t, i + 1, "crate") {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1800:17 True  test || gate_here,
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1846:75 True  if matches!(scope, Scope::Mod(_)) && (attrs.path.is_some() || !attrs.also.is_empty()) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1952:8 True  if keyword(item, k, "pub") {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1954:12 True  if punct(item, k, '(') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1954:12 False  if punct(item, k, '(') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:1967:23 False  Some(name) if punct(item, k + 2, '{') => {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2021:30 False  path == "Cargo.toml" || (path.starts_with("crates/") && path.ends_with("/Cargo.toml"))
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2058:56 False  '"' | '\'' if c.get(i + 1) == Some(&ch) && c.get(i + 2) == Some(&ch) => {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2061:27 False  while let Some(&x) = c.get(i) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2062:35 False  if x == ch && c.get(i + 1) == Some(&ch) && c.get(i + 2) == Some(&ch) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2062:64 False  if x == ch && c.get(i + 1) == Some(&ch) && c.get(i + 2) == Some(&ch) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2066:24 True  if x == '\n' {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2069:29 True  i += if x == '\\' && ch == '"' && c.get(i + 1) != Some(&'\n') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2069:42 True  i += if x == '\\' && ch == '"' && c.get(i + 1) != Some(&'\n') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2069:42 False  i += if x == '\\' && ch == '"' && c.get(i + 1) != Some(&'\n') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2069:55 True  i += if x == '\\' && ch == '"' && c.get(i + 1) != Some(&'\n') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2069:55 False  i += if x == '\\' && ch == '"' && c.get(i + 1) != Some(&'\n') {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2082:48 True  _ if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' => {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2114:27 False  Some('\\') if quote == '"' => {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2211:32 False  if key == "members" && is(k + 2, &Toml::Punct('[')) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2265:20 False  && let Some(scan) = scans.get(file.as_str())
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2404:58 False  } else if files.contains_key(target.as_str()) && !is_manifest(&target) {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2427:64 False  if pasted.contains(path) && shipped.contains(*path) && !scan.whole_file_test {
GAP test code  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2439:43 False  if path.ends_with("_tests.rs") && !under_tests_dir(path) && !tests.contains(*path) {
GAP test code  LINE   crates/vocab/tests/workspace_is_rust.rs:1412  }
GAP test code  LINE   crates/vocab/tests/workspace_is_rust.rs:1424  }
GAP test code  LINE   crates/vocab/tests/workspace_is_rust.rs:1437  }
GAP test code  BRANCH crates/vocab/tests/workspace_is_rust.rs:241:34 False  if t.starts_with('[') && !in_deps {
GAP test code  BRANCH crates/vocab/tests/workspace_is_rust.rs:243:16 False  if in_package {
GAP test code  BRANCH crates/vocab/tests/workspace_is_rust.rs:259:27 False  } else if let Some(dep) = quoted(t) {
GAP test code  BRANCH crates/vocab/tests/workspace_is_rust.rs:317:24 False  if let (Some(flag), Some(child)) = (seen.get_mut(j), pkgs.get(j))
GAP test code  BRANCH crates/vocab/tests/workspace_is_rust.rs:1410:16 False  if let [a, b] = pair {
GAP test code  BRANCH crates/vocab/tests/workspace_is_rust.rs:1422:16 False  if let [a, b] = pair {
GAP test code  BRANCH crates/vocab/tests/workspace_is_rust.rs:1435:16 False  if let [a, b] = pair {
GAP test code  BRANCH crates/vocab/tests/workspace_is_rust.rs:1551:9 False  let Ok(probe) = probe else { return };

NEEDS THE MAC (L-BCL-3), every row:
MAC  LINE   crates/lake/tests/real_lake.rs:57  fn lake_path(rel: &str) -> Option<PathBuf> {
MAC  LINE   crates/lake/tests/real_lake.rs:58  let home = std::env::var_os("HOME")?;
MAC  LINE   crates/lake/tests/real_lake.rs:59  let p = Path::new(&home).join(rel);
MAC  LINE   crates/lake/tests/real_lake.rs:60  p.exists().then_some(p)
MAC  LINE   crates/lake/tests/real_lake.rs:61  }
MAC  LINE   crates/lake/tests/real_lake.rs:64  fn skipped(what: &str) {
MAC  LINE   crates/lake/tests/real_lake.rs:65  println!("SKIPPING {what}: ~/.brutex/lake is not on this machine.");
MAC  LINE   crates/lake/tests/real_lake.rs:66  println!("  The fixture cannot be tracked — CI gate 1 forbids a .parquet in");
MAC  LINE   crates/lake/tests/real_lake.rs:67  println!("  this repository — so this test proves nothing here. The decode");
MAC  LINE   crates/lake/tests/real_lake.rs:68  println!("  path is covered by tests/synthetic.rs, which writes its own.");
MAC  LINE   crates/lake/tests/real_lake.rs:69  }
MAC  LINE   crates/lake/tests/real_lake.rs:73  fn the_real_fno_sample_decodes_to_the_values_it_holds() {
MAC  LINE   crates/lake/tests/real_lake.rs:74  let Some(path) = lake_path(SAMPLE_FNO) else {
MAC  LINE   crates/lake/tests/real_lake.rs:75  panic!(
MAC  LINE   crates/lake/tests/real_lake.rs:83  let f = LakeFile::open(&path).expect("the real sample file must open");
MAC  LINE   crates/lake/tests/real_lake.rs:84  assert_eq!(f.layout(), Layout::Fno, "17 columns with greeks");
MAC  LINE   crates/lake/tests/real_lake.rs:85  assert_eq!(f.created_by(), Some("Polars"));
MAC  LINE   crates/lake/tests/real_lake.rs:86  assert_eq!(f.num_rows(), 2_480);
MAC  LINE   crates/lake/tests/real_lake.rs:87  assert_eq!(f.row_groups(), 1);
MAC  LINE   crates/lake/tests/real_lake.rs:89  let b = f.read_row_group(0).expect("the ZSTD pages must decode");
MAC  LINE   crates/lake/tests/real_lake.rs:90  assert_eq!(b.len(), 2_480);
MAC  LINE   crates/lake/tests/real_lake.rs:94  let r = b.row(0).expect("row 0");
MAC  LINE   crates/lake/tests/real_lake.rs:95  assert_eq!(r.timestamp_micros, 1_584_685_680_000_000);
MAC  LINE   crates/lake/tests/real_lake.rs:98  assert_eq!(r.open.raw(), 4_950);
MAC  LINE   crates/lake/tests/real_lake.rs:99  assert_eq!(r.high.raw(), 4_950);
MAC  LINE   crates/lake/tests/real_lake.rs:100  assert_eq!(r.low.raw(), 4_950);
MAC  LINE   crates/lake/tests/real_lake.rs:101  assert_eq!(r.close.raw(), 4_950);
MAC  LINE   crates/lake/tests/real_lake.rs:102  assert_eq!(r.volume, 76);
MAC  LINE   crates/lake/tests/real_lake.rs:103  assert_eq!(r.open_interest(), Some(75));
MAC  LINE   crates/lake/tests/real_lake.rs:106  assert_eq!(r.spot_at_bar.expect("spot").raw(), 847_310);
MAC  LINE   crates/lake/tests/real_lake.rs:107  assert_eq!(r.greeks_provenance_id, Some(1_380_610));
MAC  LINE   crates/lake/tests/real_lake.rs:110  let g = r.greeks.expect("row 0 has greeks");
MAC  LINE   crates/lake/tests/real_lake.rs:111  assert_eq!(g.iv.to_bits(), 0.678_618_464_061_773_3_f64.to_bits());
MAC  LINE   crates/lake/tests/real_lake.rs:112  assert_eq!(g.delta.to_bits(), 0.103_791_597_602_742_98_f64.to_bits());
MAC  LINE   crates/lake/tests/real_lake.rs:113  assert_eq!(
MAC  LINE   crates/lake/tests/real_lake.rs:114  g.gamma.to_bits(),
MAC  LINE   crates/lake/tests/real_lake.rs:115  0.000_171_426_804_295_494_02_f64.to_bits()
MAC  LINE   crates/lake/tests/real_lake.rs:117  assert_eq!(g.theta.to_bits(), (-7.788_961_946_081_791_f64).to_bits());
MAC  LINE   crates/lake/tests/real_lake.rs:118  assert_eq!(g.vega.to_bits(), 2.791_593_074_801_929_3_f64.to_bits());
MAC  LINE   crates/lake/tests/real_lake.rs:119  assert_eq!(g.rho.to_bits(), 0.276_837_206_382_207_27_f64.to_bits());
MAC  LINE   crates/lake/tests/real_lake.rs:120  assert_eq!(
MAC  LINE   crates/lake/tests/real_lake.rs:121  g.t_years_used.to_bits(),
MAC  LINE   crates/lake/tests/real_lake.rs:122  0.033_280_060_882_800_61_f64.to_bits()
MAC  LINE   crates/lake/tests/real_lake.rs:124  assert_eq!(g.rate_used.to_bits(), 0.065_f64.to_bits());
MAC  LINE   crates/lake/tests/real_lake.rs:127  assert_eq!(b.iter().count(), 2_480);
MAC  LINE   crates/lake/tests/real_lake.rs:128  }
MAC  LINE   crates/lake/tests/real_lake.rs:132  fn the_real_cash_sample_has_the_seven_column_layout() {
MAC  LINE   crates/lake/tests/real_lake.rs:133  let Some(path) = lake_path(SAMPLE_CASH) else {
MAC  LINE   crates/lake/tests/real_lake.rs:134  panic!(
MAC  LINE   crates/lake/tests/real_lake.rs:142  let f = LakeFile::open(&path).expect("open");
MAC  LINE   crates/lake/tests/real_lake.rs:143  assert_eq!(f.layout(), Layout::Cash, "no greeks in a cash file");
MAC  LINE   crates/lake/tests/real_lake.rs:144  assert_eq!(f.num_rows(), 2_143);
MAC  LINE   crates/lake/tests/real_lake.rs:146  let b = f.read_row_group(0).expect("decode");
MAC  LINE   crates/lake/tests/real_lake.rs:147  let r = b.row(0).expect("row 0");
MAC  LINE   crates/lake/tests/real_lake.rs:148  assert_eq!(r.timestamp_micros, 1_674_445_500_000_000);
MAC  LINE   crates/lake/tests/real_lake.rs:149  assert_eq!(r.open.raw(), 195_000); // 1950.00
MAC  LINE   crates/lake/tests/real_lake.rs:150  assert_eq!(r.high.raw(), 195_000);
MAC  LINE   crates/lake/tests/real_lake.rs:151  assert_eq!(r.low.raw(), 193_200); // 1932.00
MAC  LINE   crates/lake/tests/real_lake.rs:152  assert_eq!(r.close.raw(), 194_300); // 1943.00
MAC  LINE   crates/lake/tests/real_lake.rs:153  assert_eq!(r.volume, 2_663);
MAC  LINE   crates/lake/tests/real_lake.rs:156  assert_eq!(r.open_interest(), None);
MAC  LINE   crates/lake/tests/real_lake.rs:157  assert_eq!(r.open_interest, i64::MIN);
MAC  LINE   crates/lake/tests/real_lake.rs:160  assert!(!r.has_greeks());
MAC  LINE   crates/lake/tests/real_lake.rs:161  assert_eq!(r.spot_at_bar, None);
MAC  LINE   crates/lake/tests/real_lake.rs:162  assert_eq!(r.greeks_provenance_id, None);
MAC  LINE   crates/lake/tests/real_lake.rs:163  }
MAC  LINE   crates/lake/tests/real_lake.rs:167  fn a_real_contract_directory_name_parses_and_round_trips() {
MAC  LINE   crates/lake/tests/real_lake.rs:168  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
MAC  LINE   crates/lake/tests/real_lake.rs:169  skipped("the contract directory walk");
MAC  LINE   crates/lake/tests/real_lake.rs:170  return;
MAC  LINE   crates/lake/tests/real_lake.rs:173  let mut seen = 0_usize;
MAC  LINE   crates/lake/tests/real_lake.rs:174  let mut futures = 0_usize;
MAC  LINE   crates/lake/tests/real_lake.rs:175  let entries = std::fs::read_dir(&root).expect("read the F&O directory");
MAC  LINE   crates/lake/tests/real_lake.rs:176  for e in entries.take(500) {
MAC  LINE   crates/lake/tests/real_lake.rs:177  let e = e.expect("entry");
MAC  LINE   crates/lake/tests/real_lake.rs:178  let name = e.file_name();
MAC  LINE   crates/lake/tests/real_lake.rs:179  let name = name.to_str().expect("a lake name is utf-8");
MAC  LINE   crates/lake/tests/real_lake.rs:181  let parsed = ContractName::parse(name)
MAC  LINE   crates/lake/tests/real_lake.rs:182  .unwrap_or_else(|err| panic!("real lake directory {name} was refused: {err}"));
MAC  LINE   crates/lake/tests/real_lake.rs:183  assert_eq!(
MAC  LINE   crates/lake/tests/real_lake.rs:184  parsed.to_string(),
MAC  LINE   crates/lake/tests/real_lake.rs:188  if parsed.kind() == ContractKind::Future {
MAC  LINE   crates/lake/tests/real_lake.rs:189  futures += 1;
MAC  LINE   crates/lake/tests/real_lake.rs:190  }
MAC  LINE   crates/lake/tests/real_lake.rs:191  seen += 1;
MAC  LINE   crates/lake/tests/real_lake.rs:193  assert!(seen > 0, "the F&O directory must not be empty");
MAC  LINE   crates/lake/tests/real_lake.rs:194  println!("parsed {seen} real contract names ({futures} futures), all round-tripped");
MAC  LINE   crates/lake/tests/real_lake.rs:195  }
MAC  LINE   crates/lake/tests/real_lake.rs:199  fn a_spread_of_real_files_decodes_with_no_failures() {
MAC  LINE   crates/lake/tests/real_lake.rs:200  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
MAC  LINE   crates/lake/tests/real_lake.rs:201  skipped("the multi-file decode");
MAC  LINE   crates/lake/tests/real_lake.rs:202  return;
MAC  LINE   crates/lake/tests/real_lake.rs:205  let mut files = 0_usize;
MAC  LINE   crates/lake/tests/real_lake.rs:206  let mut rows = 0_usize;
MAC  LINE   crates/lake/tests/real_lake.rs:207  let mut null_oi = 0_usize;
MAC  LINE   crates/lake/tests/real_lake.rs:208  let mut real_oi = 0_usize;
MAC  LINE   crates/lake/tests/real_lake.rs:209  let mut with_greeks = 0_usize;
MAC  LINE   crates/lake/tests/real_lake.rs:211  let dirs = std::fs::read_dir(&root).expect("read dir");
MAC  LINE   crates/lake/tests/real_lake.rs:212  for d in dirs.take(40) {
MAC  LINE   crates/lake/tests/real_lake.rs:213  let d = d.expect("entry").path();
MAC  LINE   crates/lake/tests/real_lake.rs:214  let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
MAC  LINE   crates/lake/tests/real_lake.rs:215  continue;
MAC  LINE   crates/lake/tests/real_lake.rs:217  for y in years {
MAC  LINE   crates/lake/tests/real_lake.rs:218  let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
MAC  LINE   crates/lake/tests/real_lake.rs:219  continue;
MAC  LINE   crates/lake/tests/real_lake.rs:221  for m in months {
MAC  LINE   crates/lake/tests/real_lake.rs:222  let p = m.expect("month").path();
MAC  LINE   crates/lake/tests/real_lake.rs:223  let f = LakeFile::open(&p)
MAC  LINE   crates/lake/tests/real_lake.rs:224  .unwrap_or_else(|e| panic!("real lake file {} refused: {e}", p.display()));
MAC  LINE   crates/lake/tests/real_lake.rs:225  for g in 0..f.row_groups() {
MAC  LINE   crates/lake/tests/real_lake.rs:226  let b = f
MAC  LINE   crates/lake/tests/real_lake.rs:227  .read_row_group(g)
MAC  LINE   crates/lake/tests/real_lake.rs:228  .unwrap_or_else(|e| panic!("{} group {g} refused: {e}", p.display()));
MAC  LINE   crates/lake/tests/real_lake.rs:229  for bar in b.iter() {
MAC  LINE   crates/lake/tests/real_lake.rs:230  rows += 1;
MAC  LINE   crates/lake/tests/real_lake.rs:231  if bar.open_interest().is_none() {
MAC  LINE   crates/lake/tests/real_lake.rs:232  null_oi += 1;
MAC  LINE   crates/lake/tests/real_lake.rs:233  } else {
MAC  LINE   crates/lake/tests/real_lake.rs:234  real_oi += 1;
MAC  LINE   crates/lake/tests/real_lake.rs:235  }
MAC  LINE   crates/lake/tests/real_lake.rs:236  if bar.has_greeks() {
MAC  LINE   crates/lake/tests/real_lake.rs:237  with_greeks += 1;
MAC  LINE   crates/lake/tests/real_lake.rs:238  }
MAC  LINE   crates/lake/tests/real_lake.rs:241  files += 1;
MAC  LINE   crates/lake/tests/real_lake.rs:246  assert!(files > 0, "at least one real file must have been read");
MAC  LINE   crates/lake/tests/real_lake.rs:247  println!("decoded {files} real files, {rows} rows, 0 failures");
MAC  LINE   crates/lake/tests/real_lake.rs:248  println!("  open interest: {real_oi} real, {null_oi} null");
MAC  LINE   crates/lake/tests/real_lake.rs:249  println!("  rows carrying greeks: {with_greeks}");
MAC  LINE   crates/lake/tests/real_lake.rs:253  assert!(rows > 0);
MAC  LINE   crates/lake/tests/real_lake.rs:254  assert!(null_oi > 0, "the sample must contain a null open interest");
MAC  LINE   crates/lake/tests/real_lake.rs:255  assert!(real_oi > 0, "and a real one");
MAC  LINE   crates/lake/tests/real_lake.rs:256  }
MAC  BRANCH crates/lake/tests/real_lake.rs:74:9 True  let Some(path) = lake_path(SAMPLE_FNO) else {
MAC  BRANCH crates/lake/tests/real_lake.rs:74:9 False  let Some(path) = lake_path(SAMPLE_FNO) else {
MAC  BRANCH crates/lake/tests/real_lake.rs:133:9 True  let Some(path) = lake_path(SAMPLE_CASH) else {
MAC  BRANCH crates/lake/tests/real_lake.rs:133:9 False  let Some(path) = lake_path(SAMPLE_CASH) else {
MAC  BRANCH crates/lake/tests/real_lake.rs:168:9 True  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
MAC  BRANCH crates/lake/tests/real_lake.rs:168:9 False  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
MAC  BRANCH crates/lake/tests/real_lake.rs:188:12 True  if parsed.kind() == ContractKind::Future {
MAC  BRANCH crates/lake/tests/real_lake.rs:188:12 False  if parsed.kind() == ContractKind::Future {
MAC  BRANCH crates/lake/tests/real_lake.rs:200:9 True  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
MAC  BRANCH crates/lake/tests/real_lake.rs:200:9 False  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
MAC  BRANCH crates/lake/tests/real_lake.rs:214:13 True  let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
MAC  BRANCH crates/lake/tests/real_lake.rs:214:13 False  let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
MAC  BRANCH crates/lake/tests/real_lake.rs:218:17 True  let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
MAC  BRANCH crates/lake/tests/real_lake.rs:218:17 False  let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
MAC  BRANCH crates/lake/tests/real_lake.rs:231:28 True  if bar.open_interest().is_none() {
MAC  BRANCH crates/lake/tests/real_lake.rs:231:28 False  if bar.open_interest().is_none() {
MAC  BRANCH crates/lake/tests/real_lake.rs:236:28 True  if bar.has_greeks() {
MAC  BRANCH crates/lake/tests/real_lake.rs:236:28 False  if bar.has_greeks() {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:57  fn eat(mut h: u64, v: u64) -> u64 {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:58  for b in v.to_le_bytes() {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:59  h ^= u64::from(b);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:60  h = h.wrapping_mul(FNV_PRIME);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:61  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:62  h
MAC  LINE   crates/lake/tests/real_lake_regression.rs:63  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:66  fn eat_opt(h: u64, v: Option<u64>) -> u64 {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:67  match v {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:68  Some(v) => eat(eat(h, 1), v),
MAC  LINE   crates/lake/tests/real_lake_regression.rs:69  None => eat(h, 0),
MAC  LINE   crates/lake/tests/real_lake_regression.rs:71  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:78  fn digest_file(path: &Path) -> Result<(u64, usize, Layout), String> {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:79  let file = LakeFile::open(path).map_err(|e| e.to_string())?;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:80  let layout = file.layout();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:81  let mut h = eat(FNV_OFFSET, u64::from(u8::from(layout.has_greeks())));
MAC  LINE   crates/lake/tests/real_lake_regression.rs:82  let mut rows = 0_usize;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:84  for g in 0..file.row_groups() {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:85  let batch = file
MAC  LINE   crates/lake/tests/real_lake_regression.rs:86  .read_row_group(g)
MAC  LINE   crates/lake/tests/real_lake_regression.rs:87  .map_err(|e| format!("group {g}: {e}"))?;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:88  for bar in batch.iter() {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:94  h = eat(h, bar.timestamp_micros as u64);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:95  h = eat(h, bar.open.raw() as u64);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:96  h = eat(h, bar.high.raw() as u64);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:97  h = eat(h, bar.low.raw() as u64);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:98  h = eat(h, bar.close.raw() as u64);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:99  h = eat(h, bar.volume as u64);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:102  h = eat(h, bar.open_interest as u64);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:103  h = eat_opt(h, bar.spot_at_bar.map(|p| p.raw() as u64));
MAC  LINE   crates/lake/tests/real_lake_regression.rs:104  h = eat_opt(h, bar.greeks_provenance_id.map(|v| i64::from(v) as u64));
MAC  LINE   crates/lake/tests/real_lake_regression.rs:106  match bar.greeks {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:107  Some(g) => {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:108  h = eat(h, 1);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:109  for v in [
MAC  LINE   crates/lake/tests/real_lake_regression.rs:110  g.iv,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:111  g.delta,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:112  g.gamma,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:113  g.theta,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:114  g.vega,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:115  g.rho,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:116  g.t_years_used,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:117  g.rate_used,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:118  ] {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:119  h = eat(h, v.to_bits());
MAC  LINE   crates/lake/tests/real_lake_regression.rs:120  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:122  None => h = eat(h, 0),
MAC  LINE   crates/lake/tests/real_lake_regression.rs:124  rows += 1;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:127  Ok((h, rows, layout))
MAC  LINE   crates/lake/tests/real_lake_regression.rs:128  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:131  fn month_files(contract: &Path, into: &mut Vec<PathBuf>) {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:132  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:133  return;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:135  let mut ys: Vec<PathBuf> = years.flatten().map(|e| e.path()).collect();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:136  ys.sort();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:137  for y in ys {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:138  let Ok(months) = std::fs::read_dir(&y) else {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:139  continue;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:141  let mut ms: Vec<PathBuf> = months.flatten().map(|e| e.path()).collect();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:142  ms.sort();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:143  into.extend(ms);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:145  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:149  fn sample_dirs(root: &Path, want: usize) -> Vec<PathBuf> {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:150  let Ok(entries) = std::fs::read_dir(root) else {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:151  return Vec::new();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:153  let mut all: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:154  all.sort();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:155  if all.is_empty() || want == 0 {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:156  return Vec::new();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:157  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:158  let stride = (all.len() / want).max(1);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:159  all.into_iter().step_by(stride).take(want).collect()
MAC  LINE   crates/lake/tests/real_lake_regression.rs:160  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:164  fn a_wide_sample_of_the_real_lake_decodes_with_no_refusal_and_a_stable_digest() {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:165  let Some(home) = std::env::var_os("HOME") else {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:166  panic!(
MAC  LINE   crates/lake/tests/real_lake_regression.rs:170  let root = Path::new(&home).join(".brutex/lake/bars/NSE");
MAC  LINE   crates/lake/tests/real_lake_regression.rs:171  assert!(
MAC  LINE   crates/lake/tests/real_lake_regression.rs:172  root.exists(),
MAC  LINE   crates/lake/tests/real_lake_regression.rs:176  let mut targets: Vec<PathBuf> = Vec::new();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:177  for contract in sample_dirs(&root.join("FNO"), 120) {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:178  month_files(&contract, &mut targets);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:179  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:180  for symbol in sample_dirs(&root.join("CASH"), 20) {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:181  month_files(&symbol, &mut targets);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:182  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:183  for symbol in sample_dirs(&root.join("INDEX"), 5) {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:184  month_files(&symbol, &mut targets);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:185  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:186  targets.sort();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:188  let mut refused: Vec<String> = Vec::new();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:189  let mut files = 0_usize;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:190  let mut rows = 0_usize;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:191  let mut fno = 0_usize;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:192  let mut cash = 0_usize;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:193  let mut whole = FNV_OFFSET;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:195  for path in &targets {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:196  let shown = path.strip_prefix(&root).unwrap_or(path).display();
MAC  LINE   crates/lake/tests/real_lake_regression.rs:197  match digest_file(path) {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:198  Ok((h, n, layout)) => {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:199  println!("{h:016x} {n:>6} {shown}");
MAC  LINE   crates/lake/tests/real_lake_regression.rs:200  whole = eat(whole, h);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:201  files += 1;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:202  rows += n;
MAC  LINE   crates/lake/tests/real_lake_regression.rs:203  match layout {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:204  Layout::Fno => fno += 1,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:205  Layout::Cash => cash += 1,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:208  Err(e) => refused.push(format!("{shown}: {e}")),
MAC  LINE   crates/lake/tests/real_lake_regression.rs:212  println!("REAL LAKE DIGEST");
MAC  LINE   crates/lake/tests/real_lake_regression.rs:213  println!("  files    : {files}");
MAC  LINE   crates/lake/tests/real_lake_regression.rs:214  println!("  rows     : {rows}");
MAC  LINE   crates/lake/tests/real_lake_regression.rs:215  println!("  F&O      : {fno}, cash/index: {cash}");
MAC  LINE   crates/lake/tests/real_lake_regression.rs:216  println!("  refusals : {}", refused.len());
MAC  LINE   crates/lake/tests/real_lake_regression.rs:217  println!("  WHOLE    : {whole:016x}");
MAC  LINE   crates/lake/tests/real_lake_regression.rs:218  for line in refused.iter().take(20) {
MAC  LINE   crates/lake/tests/real_lake_regression.rs:219  println!("    REFUSED {line}");
MAC  LINE   crates/lake/tests/real_lake_regression.rs:220  }
MAC  LINE   crates/lake/tests/real_lake_regression.rs:222  assert!(
MAC  LINE   crates/lake/tests/real_lake_regression.rs:223  refused.is_empty(),
MAC  LINE   crates/lake/tests/real_lake_regression.rs:225  refused.join("\n  ")
MAC  LINE   crates/lake/tests/real_lake_regression.rs:227  assert!(
MAC  LINE   crates/lake/tests/real_lake_regression.rs:228  files >= MINIMUM_FILES,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:231  assert!(
MAC  LINE   crates/lake/tests/real_lake_regression.rs:232  fno > 0 && cash > 0,
MAC  LINE   crates/lake/tests/real_lake_regression.rs:235  assert!(rows > 0);
MAC  LINE   crates/lake/tests/real_lake_regression.rs:236  }
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:132:9 True  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:132:9 False  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:138:13 True  let Ok(months) = std::fs::read_dir(&y) else {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:138:13 False  let Ok(months) = std::fs::read_dir(&y) else {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:150:9 True  let Ok(entries) = std::fs::read_dir(root) else {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:150:9 False  let Ok(entries) = std::fs::read_dir(root) else {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:155:8 True  if all.is_empty() || want == 0 {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:155:8 False  if all.is_empty() || want == 0 {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:155:26 True  if all.is_empty() || want == 0 {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:155:26 False  if all.is_empty() || want == 0 {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:165:9 True  let Some(home) = std::env::var_os("HOME") else {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:165:9 False  let Some(home) = std::env::var_os("HOME") else {
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:232:9 True  fno > 0 && cash > 0,
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:232:9 False  fno > 0 && cash > 0,
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:232:20 True  fno > 0 && cash > 0,
MAC  BRANCH crates/lake/tests/real_lake_regression.rs:232:20 False  fno > 0 && cash > 0,
MAC  LINE   crates/lake/tests/refusals.rs:1313  fn no_real_lake_file_triggers_either_defect() {
MAC  LINE   crates/lake/tests/refusals.rs:1314  let Some(home) = std::env::var_os("HOME") else {
MAC  LINE   crates/lake/tests/refusals.rs:1315  panic!(
MAC  LINE   crates/lake/tests/refusals.rs:1319  let root = std::path::Path::new(&home).join(".brutex/lake/bars/NSE");
MAC  LINE   crates/lake/tests/refusals.rs:1320  assert!(
MAC  LINE   crates/lake/tests/refusals.rs:1321  root.exists(),
MAC  LINE   crates/lake/tests/refusals.rs:1325  let mut targets: Vec<(std::path::PathBuf, String)> = Vec::new();
MAC  LINE   crates/lake/tests/refusals.rs:1326  for contract in sample_dirs(&root.join("FNO"), 400) {
MAC  LINE   crates/lake/tests/refusals.rs:1327  month_files(&contract, &mut targets);
MAC  LINE   crates/lake/tests/refusals.rs:1328  }
MAC  LINE   crates/lake/tests/refusals.rs:1329  for symbol in sample_dirs(&root.join("CASH"), 30) {
MAC  LINE   crates/lake/tests/refusals.rs:1330  month_files(&symbol, &mut targets);
MAC  LINE   crates/lake/tests/refusals.rs:1331  }
MAC  LINE   crates/lake/tests/refusals.rs:1332  for symbol in sample_dirs(&root.join("INDEX"), 8) {
MAC  LINE   crates/lake/tests/refusals.rs:1333  month_files(&symbol, &mut targets);
MAC  LINE   crates/lake/tests/refusals.rs:1334  }
MAC  LINE   crates/lake/tests/refusals.rs:1336  let mut scan = Scan::default();
MAC  LINE   crates/lake/tests/refusals.rs:1337  for (path, year) in &targets {
MAC  LINE   crates/lake/tests/refusals.rs:1338  scan_file(path, year, &mut scan);
MAC  LINE   crates/lake/tests/refusals.rs:1339  }
MAC  LINE   crates/lake/tests/refusals.rs:1341  println!("scanned {} real lake files", scan.files);
MAC  LINE   crates/lake/tests/refusals.rs:1342  println!(
MAC  LINE   crates/lake/tests/refusals.rs:1346  println!("  row groups: {}, rows: {}", scan.row_groups, scan.rows);
MAC  LINE   crates/lake/tests/refusals.rs:1347  println!("  years covered: {:?}", scan.years);
MAC  LINE   crates/lake/tests/refusals.rs:1348  println!(
MAC  LINE   crates/lake/tests/refusals.rs:1350  scan.declared_short.len()
MAC  LINE   crates/lake/tests/refusals.rs:1352  println!(
MAC  LINE   crates/lake/tests/refusals.rs:1354  scan.pages_short.len()
MAC  LINE   crates/lake/tests/refusals.rs:1356  println!(
MAC  LINE   crates/lake/tests/refusals.rs:1358  scan.nested.len()
MAC  LINE   crates/lake/tests/refusals.rs:1360  println!(
MAC  LINE   crates/lake/tests/refusals.rs:1362  scan.unreadable.len()
MAC  LINE   crates/lake/tests/refusals.rs:1364  for line in scan.declared_short.iter().take(10) {
MAC  LINE   crates/lake/tests/refusals.rs:1365  println!("    DECLARED SHORT {line}");
MAC  LINE   crates/lake/tests/refusals.rs:1366  }
MAC  LINE   crates/lake/tests/refusals.rs:1367  for line in scan.pages_short.iter().take(10) {
MAC  LINE   crates/lake/tests/refusals.rs:1368  println!("    PAGES SHORT {line}");
MAC  LINE   crates/lake/tests/refusals.rs:1369  }
MAC  LINE   crates/lake/tests/refusals.rs:1370  for line in scan.nested.iter().take(10) {
MAC  LINE   crates/lake/tests/refusals.rs:1371  println!("    NESTED {line}");
MAC  LINE   crates/lake/tests/refusals.rs:1372  }
MAC  LINE   crates/lake/tests/refusals.rs:1373  for line in scan.unreadable.iter().take(10) {
MAC  LINE   crates/lake/tests/refusals.rs:1374  println!("    UNREADABLE {line}");
MAC  LINE   crates/lake/tests/refusals.rs:1375  }
MAC  LINE   crates/lake/tests/refusals.rs:1377  assert!(
MAC  LINE   crates/lake/tests/refusals.rs:1378  scan.files >= 200,
MAC  LINE   crates/lake/tests/refusals.rs:1382  assert!(
MAC  LINE   crates/lake/tests/refusals.rs:1383  scan.years.len() >= 3,
MAC  LINE   crates/lake/tests/refusals.rs:1387  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
MAC  LINE   crates/lake/tests/refusals.rs:1388  assert!(
MAC  LINE   crates/lake/tests/refusals.rs:1389  scan.declared_short.is_empty(),
MAC  LINE   crates/lake/tests/refusals.rs:1392  assert!(scan.pages_short.is_empty(), "defect 1 fires on real data");
MAC  LINE   crates/lake/tests/refusals.rs:1393  assert!(scan.nested.is_empty(), "defect 3 fires on real data");
MAC  LINE   crates/lake/tests/refusals.rs:1394  assert!(scan.unreadable.is_empty(), "a real file would not parse");
MAC  LINE   crates/lake/tests/refusals.rs:1395  }
MAC  BRANCH crates/lake/tests/refusals.rs:1314:9 True  let Some(home) = std::env::var_os("HOME") else {
MAC  BRANCH crates/lake/tests/refusals.rs:1314:9 False  let Some(home) = std::env::var_os("HOME") else {
MAC  BRANCH crates/lake/tests/refusals.rs:1387:13 True  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
MAC  BRANCH crates/lake/tests/refusals.rs:1387:13 False  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
MAC  BRANCH crates/lake/tests/refusals.rs:1387:29 True  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
MAC  BRANCH crates/lake/tests/refusals.rs:1387:29 False  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
`````

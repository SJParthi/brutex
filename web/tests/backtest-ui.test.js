import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
const unavailable = readFileSync(new URL('../src/lib/Lock.svelte', import.meta.url), 'utf8');

/** @param {string} source @param {string} from @param {string} to */
const between = (source, from, to) => {
  const start = source.indexOf(from);
  assert.notEqual(start, -1, `missing start marker: ${from}`);
  const end = source.indexOf(to, start);
  assert.notEqual(end, -1, `missing end marker: ${to}`);
  return source.slice(start, end);
};

test('unavailable data is neutral evidence, never a padlock or permission claim', () => {
  assert.doesNotMatch(unavailable, /<svg|<path|<rect/);
  assert.match(unavailable, /small \? '—' : word/);
  assert.match(unavailable, /'Undefined'/);
  assert.match(unavailable, /'Not applicable'/);
  assert.match(unavailable, /title=\{label\}/);
  assert.match(unavailable, /aria-label=\{label\}/);
  assert.match(unavailable, /\$\{word\} for this recorded run/);
  assert.match(unavailable, /font-size: max\(12px, 0\.78em\)/);
  assert.match(unavailable, /\.lk-detail \{[\s\S]*?font-size: 12px/);
});

test('unavailable reasons are reachable by pointer, keyboard, and assistive technology', () => {
  assert.match(unavailable, /<button[\s\S]*?type="button"[\s\S]*?aria-label=\{label\}/);
  assert.match(unavailable, /aria-expanded=\{expanded\}/);
  assert.match(unavailable, /onclick=\{\(\) => \(expanded = !expanded\)\}/);
  assert.match(unavailable, /\{#if expanded\}<span class="lk-detail" aria-hidden="true">\{label\}/);
  assert.match(page, /activate or hover one for\s+the exact missing field/);
});

test('keyboard ledger activation preserves its opener and suppresses Space scrolling', () => {
  assert.match(
    page,
    /onclick=\{\(event\) => toggle\(r, event\.currentTarget\)\}[\s\S]{0,500}?onkeydown=\{\(event\) => \{[\s\S]*?event\.preventDefault\(\);[\s\S]*?toggle\(r, event\.currentTarget\)/
  );
});

test('testing-period dialog owns focus, Escape, outside dismissal, and report resets', () => {
  assert.match(page, /<svelte:window onclick=\{dismissPeriodOnOutsideClick\} onkeydown=\{closePeriodOnEscape\}/);
  assert.match(page, /bind:this=\{periodRoot\} onfocusout=\{dismissPeriodOnFocusOut\}/);
  assert.match(page, /bind:this=\{periodTrigger\}[\s\S]*?onclick=\{togglePeriodMenu\}/);
  assert.match(page, /bind:this=\{periodDialog\}/);
  assert.match(page, /periodDialog\?\.querySelector\('button:not\(:disabled\)'\)[\s\S]*?current\.focus\(\)/);
  assert.match(page, /event\.key !== 'Escape'[\s\S]*?closePeriodMenu\(true\)/);

  const close = between(page, 'async function closeDrill()', 'Open or close a run');
  assert.match(close, /periodOpen = false/);
  const toggle = between(page, 'async function toggle(run, trigger = null)', 'THE PRICE TERMINAL');
  assert.match(toggle, /periodOpen = false/);
});

test('property tables expose captions and native row headers', () => {
  for (const caption of [
    'Recorded run properties',
    'Exit geometry',
    'Winning combination conditions'
  ]) {
    assert.match(page, new RegExp(`<caption class="sr-only">${caption}</caption>`));
  }
  assert.match(page, /<th class="tt-rowhead" scope="row">Feed<\/th>/);
  assert.match(page, /<th class="tt-rowhead" scope="row">\{EXIT_AXES\[i\]/);
  assert.match(page, /<th class="tt-rowhead" scope="row">\{bit \? bit\.name/);
});

test('recorded toolbar facts do not masquerade as controls', () => {
  assert.equal((page.match(/<span class="tt-fact"/g) ?? []).length, 2);
  assert.doesNotMatch(page, /<span class="tt-ctl"/);
  assert.match(page, /\.tt-fact \{[\s\S]*?cursor: default/);
  assert.doesNotMatch(page, /\.tt-fact:hover/);
});

test('disabled drill-down selectors look disabled and do not hover', () => {
  assert.match(page, /\.tt-segbtn:not\(:disabled\):hover/);
  assert.match(page, /\.tt-segbtn:disabled \{[\s\S]*?cursor: not-allowed;[\s\S]*?opacity: 0\.5/);
  assert.match(page, /\.rangebtn:not\(:disabled\):hover/);
  assert.match(page, /\.rangebtn:disabled \{[\s\S]*?cursor: not-allowed;[\s\S]*?opacity: 0\.5/);
});

test('narrow scroll strips keep focus rings and the period dialog inside the panel', () => {
  assert.match(page, /\.tt-pill:focus-visible \{[\s\S]*?outline-offset: -2px/);
  assert.match(page, /\.rungbtn:focus-visible \{[\s\S]*?outline-offset: -2px/);
  assert.match(
    page,
    /@media \(max-width: 520px\)[\s\S]*?\.tt-menu \{[\s\S]*?right: 0;[\s\S]*?left: auto;/
  );
});

test('known commission is shown as zero and its unmodeled spread stays visible', () => {
  const breakdown = between(page, '<span class="tt-k">Commission load</span>', '<div class="tt-hrow tight">');
  assert.match(breakdown, /0\.00%/);
  assert.match(breakdown, /statutory charges are zero for spot indices/);
  assert.match(breakdown, /spread is not modeled/);
  assert.doesNotMatch(breakdown, /<Lock/);
});

test('the chosen-grid trade ledger exposes every durable row fact without inventing prices or causes', () => {
  const ledger = between(page, '<table class="tt-tbl trade-ledger">', '</table>');
  for (const heading of [
    'Trade',
    'Leg',
    'Direction',
    'Date and time',
    'Bar',
    'Worst-fill PnL',
    'Best-fill PnL',
    'PnL / current-store open',
    'Cumulative',
    'MAE',
    'MFE',
    'Duration'
  ]) {
    assert.match(ledger, new RegExp(`>${heading}(?:<| )`), heading);
  }
  for (const unsupported of ['Price', 'Size', 'Commission', 'Exit cause']) {
    assert.doesNotMatch(ledger, new RegExp(`>${unsupported}<`), unsupported);
  }
  assert.doesNotMatch(ledger, /<Lock/);
  assert.match(page, /selected direction, and exact MAE\/MFE/);
  assert.match(page, /Entry\/exit prices and the exit-cause tag are not recorded/);
  assert.match(ledger, /t\.adverse_paisa/);
  assert.match(ledger, /t\.favourable_paisa/);
});

test('the details table owns one recorded run and names its sealed selected direction', () => {
  const details = between(page, '<table class="tt-tbl run-metrics">', '</table>');
  assert.equal((details.match(/<th(?:\s|>)/g) ?? []).length, 2, 'Metric and This run are the only columns');
  assert.doesNotMatch(details, />Long</);
  assert.doesNotMatch(details, />Short</);
  assert.doesNotMatch(details, /<Lock small\s*\/>/, 'every unavailable value must retain its exact reason');
  assert.match(details, /Total open trades[\s\S]*?<span class="tv">0<\/span>[\s\S]*?closed at span end/);
  assert.match(details, /No outlier rule is part of the recorded run/);
  assert.match(page, /Selected direction:[\s\S]*?tradeList\.direction/);
  assert.match(page, /frequency\s+sweep identity remains undirected/);
});

test('recorded-period controls cannot pretend to mutate an immutable run', () => {
  assert.match(page, /disabled=\{p !== 'Available chart range'\}/);
  assert.match(page, /Custom date range[\s\S]{0,120}new run required/);
  assert.doesNotMatch(page, /\btestingPeriod\b/);
  assert.match(page, /aria-haspopup="dialog"/);
  assert.doesNotMatch(page, /aria-label="Chart settings"|aria-label="Snapshot"|aria-label="Expand"/);
  assert.doesNotMatch(page, /aria-label="Download"|aria-label="Columns"/);
});

test('every enabled drill-down selector changes evidence or exposes its state', () => {
  assert.match(page, /aria-label="Metrics report"[\s\S]*?aria-pressed=\{testerView === 'metrics'\}/);
  assert.match(page, /aria-label="Show cumulative PnL"[\s\S]*?aria-pressed=\{plot === 'equity'\}/);
  assert.match(page, /class:on=\{plSplit === 'signals'\} aria-pressed=\{plSplit === 'signals'\}/);
  assert.match(page, /class:on=\{periodScale === s\} aria-pressed=\{periodScale === s\}/);
  assert.match(page, /class:on=\{timeGrain === grain\.key\}[\s\S]*?aria-pressed=\{timeGrain === grain\.key\}/);
  assert.match(page, /class:on=\{streakMode === 'count'\}[\s\S]*?disabled=\{streakRows\.rows\.length === 0\}/);
  assert.match(page, /class:on=\{chartRung === r\.name\}[\s\S]*?aria-pressed=\{chartRung === r\.name\}/);
  assert.match(page, /class="rangebtn" disabled=\{!chartReady\}/);

  const benchmarking = between(page, "{:else if paTab === 'benchmarking'}", "{:else if paTab === 'margin'}");
  assert.doesNotMatch(benchmarking, /aria-label="Period"/, 'a bar-by-bar benchmark has no fake scale selector');
});

test('dense chart and fixed-order table evidence is assistive-technology-readable', () => {
  assert.match(page, /class="rbt-col"[\s\S]*?role="img"[\s\S]*?aria-label="[\s\S]*?winners and/);
  assert.match(page, /class="streak-col"[\s\S]*?role="img"[\s\S]*?aria-label="[\s\S]*?absolute worst-fill PnL/);
  assert.match(page, /<th aria-sort="descending">Trade[\s\S]*?newest first/);
  assert.match(page, /class="tt-tblwrap trade-ledger-wrap"[\s\S]*?aria-label="Trade list; scroll horizontally and vertically"[\s\S]*?tabindex="0"/);
  assert.match(page, /\.trade-ledger-wrap:focus-visible \{[\s\S]*?outline: 2px solid var\(--focus\)/);
  assert.match(page, /panel\.focus\(\{ preventScroll: true \}\)/);
  assert.match(page, /drillTrigger\?\.isConnected[\s\S]*?drillTrigger\.focus\(\)/);
});

test('the drill-down owns the reference dark ramp and selection is not confused with focus', () => {
  assert.match(page, /--n3: #0f1113/);
  assert.match(page, /--up: #089981/);
  assert.match(page, /--down: #f23645/);
  assert.match(page, /\.tester \.tt-pill\.on \{[\s\S]*?background: #f2f2f2/);
  assert.match(page, /\.tt-pill:focus-visible \{[\s\S]*?outline: 2px solid var\(--focus\)/);
  assert.match(page, /@media \(max-width: 520px\)[\s\S]*?grid-template-columns: 1fr/);
});

test('trade analytics is admitted before publication and a refusal is visible across the report', () => {
  const fetch = between(page, 'async function fetchTrades(identity)', 'Everything the reference');
  // THE DOOR, NOT THE VARIABLE NAME. This pinned the literal
  // `validateTradePayload(body, expectedRun)`, and `fetchTrades` now pages
  // `/trades.json` and validates the ASSEMBLED body — one call over every page's
  // rows rather than one call over the first page's. The property under test is
  // that the response crosses the admission door before any row is published,
  // and that is unchanged; the argument's name is not the property.
  const admission = fetch.search(/validateTradePayload\(\w+, expectedRun\)/);
  const publication = fetch.indexOf('rows: checked.rows');
  assert.ok(admission >= 0, 'the response must cross the exact-integer admission door');
  // AND EVERY PAGE IS FETCHED, which is the defect that made the door moot: a
  // single-page request on a 257+ trade run reconciled short and refused the
  // whole payload, sending every trade-derived figure to a padlock.
  assert.match(fetch, /next_page/, 'the paged resource must be paged');
  assert.ok(publication > admission, 'validated rows alone may enter report state');
  assert.match(fetch, /policy: checked\.policy/);
  assert.match(fetch, /direction: checked\.direction/);
  assert.match(fetch, /if \(!checked\.ok\)[\s\S]*?rows: \[\][\s\S]*?periods: null/);
  assert.match(page, /role="alert"[\s\S]*?trade analytics payload was refused/i);
  assert.match(page, /No trade row,[\s\S]*?derived money figure from that response is[\s\S]*?displayed/);
});

test('both frontier consumers admit exact complete payloads before publishing any ranking row', () => {
  assert.match(
    page,
    /import \{ validateFrontierPayload \} from '\$lib\/frontier-analytics\.js'/
  );

  const combos = between(page, 'async function fetchCombos(identity)', 'The top N by the operator');
  const comboAdmission = combos.indexOf('validateFrontierPayload(body, identity)');
  const comboPublication = combos.indexOf('rows: checked.rows');
  assert.ok(comboAdmission >= 0, 'the open-run frontier must cross the exact-integer door');
  assert.ok(comboPublication > comboAdmission, 'only validated open-run rows may be published');
  assert.match(combos, /if \(!checked\.ok\)[\s\S]*?rows: \[\][\s\S]*?rules: null/);

  const board = between(page, 'async function fetchBoard(rowsIn)', '/** @param {string} rung e.g.');
  const boardAdmission = board.indexOf('validateFrontierPayload(body, run.identity)');
  const boardPublication = board.indexOf('rows: checked.rows');
  assert.ok(boardAdmission >= 0, 'every timeframe frontier must cross the exact-integer door');
  assert.ok(boardPublication > boardAdmission, 'only validated board rows may be published');
  assert.match(
    board,
    /if \(!checked\.ok\)[\s\S]*?rows: \[\][\s\S]*?rules: null[\s\S]*?admitted: 0/
  );
});

test('ledger rows cross one exact-integer door before answer, rung, sort, or drill computation', () => {
  assert.match(
    page,
    /import \{[\s\S]*?compareRuns,[\s\S]*?validateRunForComputation[\s\S]*?\} from '\$lib\/comparison\.js'/
  );
  assert.match(
    page,
    /const runAdmissions = \$derived\([\s\S]*?validateRunForComputation\(run\)[\s\S]*?const runs = \$derived\([\s\S]*?admission\.ok/
  );
  assert.match(page, /compareRuns\(scopedRuns, ledger\?\.signal_rungs\)/);

  const toggle = between(page, 'async function toggle(run, trigger = null)', 'THE PRICE TERMINAL');
  assert.match(toggle, /if \(!validateRunForComputation\(run\)\.ok\) return/);
  assert.match(page, /disabled=\{!r\.admitted\}/);
  assert.match(page, /Report refused/);
  assert.match(page, /records remain visible as metadata in the comparison table/i);
  assert.match(page, /No scoped run is safe to compute/);
  assert.match(page, /const signalRungSet = \$derived\.by/);
  assert.match(page, /signalRungSet !== null && signalRungSet\.has/);
  assert.match(page, /void fetchBoard\(rankableRuns\)/);
  assert.doesNotMatch(
    between(page, 'const swept = (r)', '/** Recorded runs'),
    /\/min\$\//,
    'a suffix must never decide whether a ledger row is a swept signal rung'
  );
});

test('drill-down detail responses are generation and identity guarded', () => {
  assert.match(page, /import \{ createRequestGate \} from '\$lib\/request-gate\.js'/);
  assert.match(page, /const comboGate = createRequestGate\(\)/);
  assert.match(page, /const tradeGate = createRequestGate\(\)/);

  const combos = between(page, 'async function fetchCombos(identity)', 'The top N by the operator');
  assert.match(combos, /const ticket = comboGate\.begin\(identity\)/);
  assert.equal(
    (combos.match(/comboGate\.admits\(ticket, openRun\?\.identity\)/g) ?? []).length,
    2,
    'both success and failure publication must belong to the still-open identity'
  );

  const trades = between(page, 'async function fetchTrades(identity)', 'Everything the reference');
  assert.match(trades, /const ticket = tradeGate\.begin\(identity\)/);
  assert.match(trades, /if \(!expectedRun\)[\s\S]*?unbound request/);
  assert.equal(
    (trades.match(/tradeGate\.admits\(ticket, openRun\?\.identity\)/g) ?? []).length,
    2,
    'both success and failure publication must belong to the still-open identity'
  );

  const close = between(page, 'async function closeDrill()', 'Open or close a run');
  assert.match(close, /void fetchTrades\(undefined\)/);
  assert.match(close, /void fetchCombos\(undefined\)/);
});

test('every mask display uses the all-or-nothing decoder and names refusal', () => {
  assert.match(page, /import \{ decodeMaskWords \} from '\$lib\/mask\.js'/);
  assert.doesNotMatch(page, /function positionsIn|positionsIn\(/);
  const names = between(page, '{#snippet conditionNames', '{/snippet}');
  assert.match(names, /decodeMaskWords\(words\)/);
  assert.match(names, /Combination cannot be decoded/);
  assert.match(page, /No subset of[\s\S]*?partial condition list would name a different[\s\S]*?strategy/);
});

test('unsafe comparison counts render refusal words rather than finite-number formatting', () => {
  const comparison = between(page, '<section class="block rise compare-block"', '</section>');
  assert.match(comparison, /r\.admitted \? comparisonSpan\(r\) : 'span not computed'/);
  assert.match(comparison, /r\.admitted \? `\$\{comparisonCount\(r\.months_found\)\}/);
  assert.match(comparison, /r\.admitted \? comparisonCount\(r\.depth\) : 'not computed'/);
  assert.match(comparison, /r\.admitted \? comparisonCount\(r\.trades\) : 'not computed'/);
  assert.match(comparison, /not decoded — \{r\.admissionWhy\}/);
  assert.match(page, /Number\.isSafeInteger\(value\)[\s\S]*?'not exact'/);
  assert.match(page, /Number\.isSafeInteger\(run\.from_year\)[\s\S]*?'span not exact'/);
});

test('malformed ledger envelopes and unsafe derived arithmetic fail closed', () => {
  const fetch = between(page, 'async function fetchLedger()', '$effect(() =>');
  assert.match(fetch, /validateLedgerPayload\(body\)/);
  assert.match(fetch, /if \(!checked\.ok\)/);
  assert.match(fetch, /why: `\$\{checked\.why\} Nothing from it was ranked or opened\.`/);
  assert.match(page, /exactIntegerDelta\(bench\.close, bench\.open\)/);
  assert.match(page, /exactIntegerDelta\(openRun\.pessimistic, buyHold\.gain\)/);
  assert.match(page, /roundedScaledRatio\(openRun\.pessimistic, bench\.open, 10_000\)/);
  assert.match(page, /exactIntegerDelta\(strategyBps, buyHold\.bps\)/);
});

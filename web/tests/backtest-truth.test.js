import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const page = readFileSync(
  new URL('../src/routes/backtest/+page.svelte', import.meta.url),
  'utf8'
);
const neutralValue = readFileSync(new URL('../src/lib/Lock.svelte', import.meta.url), 'utf8');

test('a retained evidence prefix is never advertised as a global Top N', () => {
  assert.match(page, /Preview: top \{topShown\} within each stored prefix/);
  assert.match(page, /This is not yet a global Top \{topShown\}/);
  assert.match(page, /rows omitted before the exit grid remain unknown—not losers/);
  assert.doesNotMatch(page, /<h2 class="bh">Top \{topShown\} by comparable question and timeframe<\/h2>/);
});

/** @param {string} source @param {string} from @param {string} to */
const between = (source, from, to) => {
  const start = source.indexOf(from);
  assert.notEqual(start, -1, `missing start marker: ${from}`);
  const end = source.indexOf(to, start + from.length);
  assert.notEqual(end, -1, `missing end marker: ${to}`);
  return source.slice(start, end);
};

/** @param {string} source @param {string} label @param {string} nextLabel */
const metric = (source, label, nextLabel) =>
  between(source, `<span class="tt-k">${label}</span>`, `<span class="tt-k">${nextLabel}</span>`);

/**
 * Return the branch which is rendered when the ledger already proves that the
 * run opened zero trades. Unknown/missing detail is allowed to use a neutral
 * marker in a later branch; a known empty set is not.
 *
 * @param {string} source
 * @param {string} name
 */
const knownZeroBranch = (source, name) => {
  const starts = ['{#if noTrades}', '{#if openRun.trades === 0}']
    .map((marker) => source.indexOf(marker))
    .filter((index) => index >= 0);
  assert.ok(starts.length > 0, `${name} must branch on the ledger's known zero-trade fact`);
  const start = Math.min(...starts);
  const end = source.indexOf('{:else', start);
  assert.notEqual(end, -1, `${name} must keep known zero separate from unknown evidence`);
  const branch = source.slice(start, end);
  assert.doesNotMatch(branch, /<Lock\b/, `${name} must not mark a known empty set unavailable`);
  assert.match(
    branch,
    /(?:money|exact|group)\(0\)|\bnone\b|>\s*0(?:\.00)?\s*</i,
    `${name} must visibly render zero or none`
  );
  return branch;
};

/** @param {string} source @param {RegExp} condition @param {string} name */
const stateWindow = (source, condition, name) => {
  const match = condition.exec(source);
  assert.ok(match, `${name} needs its own explicit state branch`);
  return source.slice(match.index, match.index + 700);
};

test('trade-derived reasons distinguish loading, failure, ready-empty, and one-trade evidence', () => {
  const fetchTrades = between(page, 'async function fetchTrades(identity)', 'Everything the reference');
  for (const phase of ['loading', 'failed', 'ready']) {
    assert.match(fetchTrades, new RegExp(`phase: '${phase}'`), `${phase} must remain a distinct fetch state`);
  }

  const loading = stateWindow(
    page,
    /tradeList\.phase === 'loading'/g,
    'loading trade evidence'
  );
  assert.match(loading, /Reading|loading|still being read/i);

  const failed = stateWindow(
    page,
    /tradeList\.phase === 'failed'/g,
    'failed trade evidence'
  );
  assert.match(failed, /tradeList\.why/, 'a failed response must retain the refusal/fetch reason');

  const readyEmptyCondition =
    /tradeList\.phase === 'ready'[\s\S]{0,140}(?:tradeList\.rows|tradeRows)\.length === 0|(?:tradeList\.rows|tradeRows)\.length === 0[\s\S]{0,140}tradeList\.phase === 'ready'/g;
  const readyEmpty = stateWindow(page, readyEmptyCondition, 'ready empty trade evidence');
  assert.match(
    readyEmpty,
    /tradeList\.why|no (?:round trips|trades)|empty/i,
    'a valid empty file and an absent historical file need an honest ready-state reason'
  );

  const oneTrade = stateWindow(
    page,
    /(?:tradeList\.rows|tradeRows)\.length === 1/g,
    'one-trade evidence'
  );
  assert.match(
    oneTrade,
    /one (?:completed )?trade|at least two trades|insufficient sample/i,
    'a one-trade curve is present but too short; it is not a missing file'
  );

  const literalLocks = page.match(/<Lock\b[\s\S]*?\/>/g) ?? [];
  const stateBlindReasons = literalLocks.filter((tag) =>
    /\bwhy="(?:Needs the trade file|This run recorded no trade file)/.test(tag)
  );
  assert.deepEqual(
    stateBlindReasons,
    [],
    'a literal Lock reason cannot truthfully cover loading, failed, ready-empty, and short samples'
  );
});

test('a known zero-trade run renders zero gross money instead of unavailable values', () => {
  const breakdown = between(page, "{#if paTab === 'breakdown'}", "{:else if paTab === 'periodical'}");
  const grossProfit = metric(breakdown, 'Gross profit', 'Gross loss');
  const grossLoss = metric(breakdown, 'Gross loss', 'Profit factor');

  assert.match(knownZeroBranch(grossProfit, 'Gross profit'), /money\(0\)|>\s*0(?:\.00)?\s*</);
  assert.match(knownZeroBranch(grossLoss, 'Gross loss'), /money\(0\)|>\s*0(?:\.00)?\s*</);
});

test('a known zero-trade donut shows zero winner, loser, and breakeven counts', () => {
  const distribution = between(page, "{#if taTab === 'distribution'}", "{:else if taTab === 'time'}");
  const donut = between(distribution, '<div class="tt-donutwrap">', '</div>\n                  </div>');
  const zero = knownZeroBranch(donut, 'Trades distribution');

  for (const [label, next] of [
    ['Winners', 'Losers'],
    ['Losers', 'Breakevens']
  ]) {
    const item = between(zero, `<span class="nm">${label}</span>`, `<span class="nm">${next}</span>`);
    assert.match(item, /exact\(0\)|0 trades|\bnone\b/i, `${label} must be visibly zero/none`);
  }
  const breakevens = zero.slice(zero.indexOf('<span class="nm">Breakevens</span>'));
  assert.notEqual(breakevens.indexOf('<span class="nm">Breakevens</span>'), -1);
  assert.match(breakevens, /exact\(0\)|0 trades|\bnone\b/i, 'Breakevens must be visibly zero/none');
});

test('known zero totals and longest streaks are zero or none, never unavailable', () => {
  const details = between(page, '<table class="tt-tbl run-metrics">', '</table>');
  const totalWinners = between(details, '<td>Total winners</td>', '<td>Total losers</td>');
  const totalLosers = between(details, '<td>Total losers</td>', '<td>Percent profitable</td>');
  knownZeroBranch(totalWinners, 'Total winners');
  knownZeroBranch(totalLosers, 'Total losers');

  const streaks = between(page, "{:else if taTab === 'streaks'}", "{:else}\n                  <p class=\"tt-note2 run-scope\">");
  const longestWin = metric(streaks, 'Longest winning streak', 'Longest losing streak');
  const longestLoss = metric(streaks, 'Longest losing streak', 'Average winning streak');
  knownZeroBranch(longestWin, 'Longest winning streak');
  knownZeroBranch(longestLoss, 'Longest losing streak');
});

test('time-pattern facts guaranteed by a nonempty validated payload have no dead Lock branch', () => {
  const time = between(page, "{:else if taTab === 'time'}", "{:else if taTab === 'streaks'}");
  for (const [label, next] of [
    ['Best day for entries', 'Best month for entries'],
    ['Best month for entries', 'Average trade duration'],
    ['Average trade duration', '<div class="tt-hrow tight">']
  ]) {
    const tile = next.startsWith('<')
      ? between(time, `<span class="tt-k">${label}</span>`, next)
      : metric(time, label, next);
    assert.doesNotMatch(
      tile,
      /<Lock\b/,
      `${label} is guaranteed inside the nonempty timePatterns branch`
    );
  }
});

test('undefined arithmetic and not-applicable metrics are labelled by their semantics', () => {
  const keyStats = between(page, '<h3 class="tt-h">Key stats</h3>', '<!-- ================= PERFORMANCE');
  const keyProfitFactor = between(
    keyStats,
    '<span class="tt-k">Profit factor</span>',
    '</div>\n                </div>'
  );
  const breakdown = between(page, "{#if paTab === 'breakdown'}", "{:else if paTab === 'periodical'}");
  const breakdownProfitFactor = between(
    breakdown,
    '<span class="tt-k">Profit factor</span>',
    '<div class="tt-hrow tight">'
  );

  /** @param {string} source */
  const semanticUndefined = (source) =>
    />\s*Undefined\s*</.test(source) ||
    /<Lock\b(?=[^>]*\b(?:kind|state|status|meaning)=["']undefined["'])[^>]*\/>/i.test(source);
  assert.ok(semanticUndefined(keyProfitFactor), 'Key stats must visibly call an absent denominator undefined');
  assert.ok(
    semanticUndefined(breakdownProfitFactor),
    'Breakdown must visibly call an absent denominator undefined'
  );

  const margin = between(page, "{:else if paTab === 'margin'}", "{:else}\n                  <div class=\"tt-quad\">");
  assert.equal(
    (margin.match(/>Not applicable</g) ?? []).length,
    4,
    'all four account/margin metrics are inapplicable to a one-unit spot-index report'
  );
  assert.doesNotMatch(margin, /<Lock\b|@render chartFrame\(/);

  if (/<Lock\b(?=[^>]*\b(?:kind|state|status|meaning)=)/i.test(page)) {
    assert.match(neutralValue, /Undefined/);
    assert.match(neutralValue, /Not applicable/);
  }
});

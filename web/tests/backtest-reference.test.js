import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');

/** @param {string} source @param {string} from @param {string} to */
const between = (source, from, to) => {
  const start = source.indexOf(from);
  assert.notEqual(start, -1, `missing start marker: ${from}`);
  const end = source.indexOf(to, start);
  assert.notEqual(end, -1, `missing end marker: ${to}`);
  return source.slice(start, end);
};

const performance = between(
  page,
  '<!-- ================= PERFORMANCE ================= -->',
  '<!-- ================= PERFORMANCE ANALYSIS ================= -->'
);

test('performance combines cumulative equity and one profit/loss bar per trade in one plot', () => {
  const equityFold = between(
    page,
    'const equity = $derived.by(() => {',
    'When this run made its money'
  );
  const pnlField = equityFold.match(
    /\b(pnl|delta|tradePnl):\s*t\.worst\s*\?\?\s*0/
  );
  assert.ok(
    pnlField,
    'each equity point must retain that trade\'s realised P/L as `pnl`, `delta`, or `tradePnl`'
  );

  const equityBranch = between(
    performance,
    "{#if equity && plot === 'equity'}",
    '{:else if bench.phase ==='
  );
  const render = equityBranch.match(
    /\{@render\s+([A-Za-z_$][\w$]*)\(\s*performanceWindow\.rows\b/
  );
  assert.ok(
    render,
    'the selected equity branch must render the bounded exact-trade window'
  );
  assert.match(
    equityFold,
    /windowSeries\(equity\?\.points\s*\?\?\s*\[\],\s*performancePage,\s*PERFORMANCE_PAGE_SIZE\)/,
    'the bounded window must be sliced directly from the exact cumulative equity points'
  );

  const chart = between(page, `{#snippet ${render[1]}(`, '{/snippet}');
  assert.match(
    chart,
    /<path\b[\s\S]*?linePath\(points, resolvedScale\)/,
    'the cumulative equity line must use the same complete-series scale as its bars and axis'
  );
  assert.match(
    chart,
    /areaPath\(points, resolvedScale\)/,
    'the cumulative equity fill must use the same complete-series scale as its bars and axis'
  );
  assert.match(
    chart,
    /\{#each\s+points\s+as\s+[A-Za-z_$][\w$]*(?:,\s*[A-Za-z_$][\w$]*)?[^}]*\}[\s\S]*?<rect\b/,
    'the combined plot must draw one histogram rectangle from each trade point'
  );
  assert.match(
    chart,
    new RegExp(`\\.${pnlField[1]}\\b`),
    'the histogram rectangles must be sized from per-trade P/L, not cumulative equity'
  );
  assert.match(
    chart,
    /<rect\b[\s\S]*?var\(--up\)[\s\S]*?var\(--down\)|<rect\b[\s\S]*?var\(--down\)[\s\S]*?var\(--up\)/,
    'positive and negative trade bars must keep the reference green/red distinction'
  );
  assert.match(
    equityBranch,
    /const performanceScale = curveScale\(equity\.points\)/,
    'paging must retain the complete equity series scale'
  );
});

test('every dense chart page keeps its complete-series vertical scale', () => {
  const timeWindow = between(page, 'const timeRows = $derived.by(() => {', 'const streakRows =');
  assert.match(timeWindow, /const tallest = all\.reduce\(/);
  assert.doesNotMatch(timeWindow, /const tallest = rows\.reduce\(/);

  const streakWindow = between(page, 'const streakRows = $derived.by(() => {', '/** Mean bars held');
  assert.match(streakWindow, /const tallest = all\.reduce\(/);
  assert.doesNotMatch(streakWindow, /const tallest = rows\.reduce\(/);

  const swing = between(page, '{#snippet swingChart', '{/snippet}');
  assert.match(swing, /const visibleMax = sw\.max/);
  assert.doesNotMatch(swing, /visibleMax\s*=\s*[^\n]*swingWindow\.rows/);
});

test('strategy run-ups and drawdowns are a selectable plot, not unavailable evidence', () => {
  const row = between(performance, '<span>Run-ups and drawdowns</span>', '</div>');
  assert.doesNotMatch(row, /<Lock\b/, 'the trade equity curve already supplies this strategy plot');
  assert.match(row, /<button\b/, 'the plot list row must be selectable');
  assert.match(
    row,
    /aria-label="Show (?:strategy )?run-ups and drawdowns"/i,
    'the selector needs an explicit accessible name'
  );

  const key = row.match(/aria-pressed=\{plot === '([^']+)'\}/)?.[1];
  assert.ok(key, 'the selector must expose its selected plot key through aria-pressed');
  assert.notEqual(key, 'equity');
  assert.notEqual(key, 'hold');
  assert.match(
    performance,
    new RegExp(`plot === '${key}'[\\s\\S]{0,1200}?\\{@render\\s+[A-Za-z_$][\\w$]*\\(\\s*equity\\.`),
    'selecting the row must render a strategy series derived from the equity curve'
  );
});

test('the newest-first trade ledger renders a bounded page, never the full reversed run', () => {
  const pageSize = page.match(/const TRADE_PAGE_SIZE = (\d+);/);
  assert.ok(pageSize, 'declare the maximum number of trade blocks rendered at once');
  assert.equal(Number(pageSize[1]), 20, 'the compact ledger shows twenty two-row trades at once');
  assert.match(page, /let tradePage = \$state\(0\)/);

  const shown = between(page, 'const tradeRowsShown', '/** Latest-request-wins');
  assert.match(shown, /\.reverse\(\)/, 'the reference opens newest first');
  assert.match(shown, /\.slice\(/, 'the reversed rows must be sliced before entering the DOM');
  assert.match(shown, /TRADE_PAGE_SIZE/, 'the slice must use the declared bound');
  assert.match(shown, /tradePage/, 'the slice offset must move with the selected page');
  assert.doesNotMatch(
    page,
    /\{#each\s+\[\.\.\.tradeRows\]\.reverse\(\)/,
    'no template loop may reverse and render the full run directly'
  );

  const ledger = between(page, '<table class="tt-tbl trade-ledger">', '</table>');
  assert.match(ledger, /\{#each tradeRowsShown as t \(t\.seq\)\}/);
  assert.match(ledger, />PnL \/ current-store open</);
  assert.doesNotMatch(ledger, />Return</);
  assert.match(page, /aria-label="Trade(?: list)? pages"/);
  assert.match(page, /tradePage\s*[+-]\s*1|tradePage\s*=\s*Math\.(?:max|min)/);
  assert.match(page, /tradePage\s*=\s*0/, 'opening a different run must reset the visible trade page');

  const listAt = page.indexOf('<!-- ================= LIST OF TRADES ================= -->');
  const rankingAt = page.indexOf('<!-- ============ TOP COMBINATIONS');
  assert.ok(listAt >= 0 && rankingAt > listAt, 'List must precede Top combinations in DOM and visual order');
  assert.doesNotMatch(page, /\.tester \.(?:trade-list|combo-ranking)\s*\{\s*order:/);
  assert.match(page, /\.tester \.trade-ledger\s*\{[\s\S]*?min-width:\s*1300px/);
  assert.match(page, /\.tester \.trade-ledger thead\s*\{[\s\S]*?position:\s*sticky/);
  assert.match(page, /class="tt-tblwrap trade-ledger-wrap"[\s\S]*?role="region"[\s\S]*?tabindex="0"/);
});

test('unsupported account-capital and script-execution controls remain explicit non-controls', () => {
  const toolbar = between(page, '<!-- ---- toolbar ---- -->', "{#if testerView === 'metrics'}");

  assert.equal(
    (toolbar.match(/aria-haspopup="dialog"/g) ?? []).length,
    1,
    'testing period is the toolbar\'s only popup'
  );
  assert.doesNotMatch(toolbar, />Initial capital<|>Script execution</);
  assert.doesNotMatch(toolbar, /capitalOpen|executionOpen|initialCapital|scriptExecution/);
  assert.equal((toolbar.match(/<span class="tt-fact"/g) ?? []).length, 2);

  assert.match(
    toolbar,
    /<span class="tt-fact" title="Recorded unit; this report does not model an account">[\s\S]*?1 unit <span class="tt-dim2">index points<\/span>/
  );
  assert.match(
    toolbar,
    /<span class="tt-fact" title="Recorded signal and execution resolution">[\s\S]*?\{openRun\.timeframe\} signal · 1min execution/
  );
  assert.doesNotMatch(
    toolbar,
    /<button[^>]*(?:account|capital|execution)|<span class="tt-fact"[^>]*(?:onclick|aria-haspopup|role="button"|tabindex)/i,
    'recorded facts must not imitate editable TradingView controls'
  );
});

// R3-12 (RA-258, RA-233, RA-259; NEW-10): THE /db PAGE LOGIC TYPES NO FEED,
// NO STORE SEGMENT AND NO DATE.
//
// A list the server already knows, typed into the page, is right the day it is
// written and silently wrong the first time the server's changes: a new feed,
// segment or delivery day then shows up wrongly or not at all, with no error.
// Every such word a /db page needs is the server's to state (`/feeds.json`, the
// census root answer, `/settings.json`). This build check walks the page-logic
// files R3-12 owns, with comments removed, and fails on:
//
//   - a feed's wire name as a literal (`'gdfl'`, `feed=zerodha`), every name
//     read from the descriptor table in `crates/pull/src/vendor.rs`, so a new
//     feed is checked the day its row exists;
//   - a store segment's word as a literal (`'FNO'`), every word read from
//     `Segment::as_str` in `crates/core/src/instrument.rs`;
//   - a calendar day or month as a literal (`'2018-09-03'`, `from=2018-09-03`).
//
// It reads the Rust source as text and runs no Rust (`settings-ceilings.test.js`
// gives the reason). One exception is allowed, by name and with its source:
// `CLOSE_CHANGE_DAY`, an exchange rule's dated change (docs/00-charter.md §7,
// D-2831, D-3320) that the server's session tables hold and no route serves
// yet; it is named for docs/06-limits.md with its owner. The store segment
// rule (`selectionSegments`) and the other owners' files are outside this
// check, and the typed lists still in them are named in the R3-12 report.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

/** The page logic R3-12 owns. */
const FILES = [
  'src/lib/store-census.js', 'src/lib/store-census-wire.js', 'src/lib/census-worker.js', 'src/lib/store.svelte.js',
  'src/lib/feeds.svelte.js', 'src/lib/feed-row.js', 'src/lib/feed-summary.js', 'src/lib/db/selection.svelte.js',
  'src/lib/db/selection-model.js', 'src/lib/db/bars-plan.js', 'src/routes/db/DbCensusBanner.svelte',
  'src/routes/db/DbCoverageGrid.svelte', 'src/routes/db/DbControls.svelte'
];

/** Each allowed literal: the file, the line's text, and why it stays. */
const ALLOWED = [
  { file: 'src/lib/db/selection-model.js', line: "export const CLOSE_CHANGE_DAY = '2026-08-03';",
    why: 'an exchange rule\'s dated change, sourced (charter §7, D-2831, D-3320); no route serves the session schedule yet' }
];

const rust = (/** @type {string} */ path) => readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8');
/** Every feed's wire name, from the descriptor table. */
const FEEDS = [...rust('crates/pull/src/vendor.rs').matchAll(/^\s*wire: "([a-z0-9]+)",$/gm)].map((m) => m[1]);
/** Every store segment's word, from `Segment::as_str`. */
const SEGMENTS = (() => {
  const text = rust('crates/core/src/instrument.rs');
  const body = text.slice(text.indexOf('impl Segment {'), text.indexOf('pub fn parse', text.indexOf('impl Segment {')));
  return [...body.matchAll(/Self::\w+ => "([A-Z]+)"/g)].map((m) => m[1]);
})();

/** The source with every comment blanked out, line for line. @param {string} source */
export function withoutComments(source) {
  const blank = (/** @type {string} */ text) => text.replace(/[^\n]/g, ' ');
  return source
    .replace(/<!--[\s\S]*?-->/g, blank)
    .replace(/\/\*[\s\S]*?\*\//g, blank)
    .replace(/(^|[^:'"`\\])\/\/[^\n]*/g, (all, before) => before + blank(all.slice(before.length)));
}

/**
 * Every typed feed, segment or date literal in `source`, with its line.
 * @param {string} source @param {string[]} feeds @param {string[]} segments
 * @returns {Array<{line: number, kind: 'feed' | 'segment' | 'date', text: string, word: string}>}
 */
export function typedLiterals(source, feeds, segments) {
  const words = (/** @type {string[]} */ list) => list.map((word) => word.replace(/[^A-Za-z0-9]/g, '')).join('|');
  const patterns = /** @type {const} */ ([
    ['feed', new RegExp(`(['"\`])(${words(feeds)})\\1|feed=(${words(feeds)})\\b`, 'g')],
    ['segment', new RegExp(`(['"\`])(${words(segments)})\\1`, 'g')],
    ['date', /(['"`])\d{4}-\d{2}(?:-\d{2})?\1|\b(?:from|to)=\d{4}-\d{2}(?:-\d{2})?/g]
  ]);
  const raw = source.split('\n');
  /** @type {Array<{line: number, kind: 'feed' | 'segment' | 'date', text: string, word: string}>} */
  const out = [];
  withoutComments(source).split('\n').forEach((line, at) => {
    for (const [kind, pattern] of patterns) {
      for (const hit of line.matchAll(pattern)) out.push({ line: at + 1, kind, text: raw[at].trim(), word: hit[0] });
    }
  });
  return out;
}

test('no_typed_feed_kind_or_date_in_page_logic', () => {
  assert.ok(FEEDS.length >= 2 && FEEDS.every((wire) => /^[a-z0-9]+$/.test(wire)), `the descriptor table's feeds: ${FEEDS}`);
  assert.ok(SEGMENTS.length >= 3, `Segment::as_str's words: ${SEGMENTS}`);
  /** @type {string[]} */
  const found = [];
  for (const file of FILES) {
    const source = readFileSync(new URL(`../${file}`, import.meta.url), 'utf8');
    for (const hit of typedLiterals(source, FEEDS, SEGMENTS)) {
      if (ALLOWED.some((allowed) => allowed.file === file && allowed.line === hit.text)) continue;
      found.push(`${file}:${hit.line} types the ${hit.kind} ${hit.word}: ${hit.text}`);
    }
  }
  assert.deepEqual(found, [], 'a list the server states is read from it, never typed into the page');
  // EACH ALLOWED LITERAL IS STILL THERE AND STILL ALONE: an exception that
  // outlived its line, or two lines hiding behind one, fails too.
  for (const allowed of ALLOWED) {
    const source = readFileSync(new URL(`../${allowed.file}`, import.meta.url), 'utf8');
    assert.equal(source.split('\n').filter((line) => line.trim() === allowed.line).length, 1, `${allowed.file}: ${allowed.line} (${allowed.why})`);
  }
});

test('the check sees every shape it refuses, and nothing in a comment', () => {
  const feeds = ['alpha', 'beta'];
  const segments = ['INDEX', 'FNO'];
  const hits = typedLiterals([
    "const a = feeds.active === 'alpha';",
    'const b = `/imports.json?feed=beta&kind=x`;',
    "if (seg === \"FNO\") return null;",
    "const day = '2018-09-03'; const month = `2019-03`;",
    'ask(`/x.json?from=2018-09-03`);',
    '// a comment naming alpha, \'FNO\' and \'2018-09-03\' is not logic',
    "/* nor 'beta' here */ const c = 1;",
    '<!-- nor \'INDEX\' here -->',
    "const url = 'https://example.invalid/alpha'; const word = 'alphabet';"
  ].join('\n'), feeds, segments);
  assert.deepEqual(hits.map((hit) => `${hit.line} ${hit.kind} ${hit.word}`), [
    "1 feed 'alpha'", '2 feed feed=beta', '3 segment "FNO"', "4 date '2018-09-03'", '4 date `2019-03`', '5 date from=2018-09-03'
  ]);
  assert.equal(withoutComments("const u = 'http://x/y'; // gone").includes('gone'), false);
  assert.equal(withoutComments("const u = 'http://x/y';").includes('http://x/y'), true, 'a URL is not a comment');
});

/* ----------------------------------------------------------------------
   THE REQUEST CEILINGS IN FORCE ARE THE SERVER'S (R3-12; RA-259)
   ---------------------------------------------------------------------- */
import { adoptCeilings, ceilingsInForce, ceilingOf, ceilingText, seriesPlan, MAX_SERIES, MAX_WINDOW_MONTHS } from '../src/lib/db/bars-plan.js';

test('the ceilings /settings.json serves are the ones a plan is refused by, and said before the request', () => {
  try {
    // BEFORE THE SERVER ANSWERS: the build's copies, and said.
    adoptCeilings(null, 'not asked yet');
    assert.equal(ceilingsInForce().from, 'build');
    assert.equal(ceilingOf('max_series'), MAX_SERIES);
    assert.match(String(ceilingsInForce().why), /could not be read \(not asked yet\), so this build's copies are in force/);
    // THE SERVER'S, adopted: a server of another build plans with its own.
    const served = { ceilings: [{ name: 'max_series', value: 3 }, { name: 'max_window_months', value: 2 }, { name: 'max_window_limit', value: 1000 },
      { name: 'browser_sort_budget', value: 20000 }, { name: 'export_cap', value: 20000 }, { name: 'max_scan_bars', value: 24000000 }] };
    const now = adoptCeilings(served);
    assert.deepEqual([now.from, now.why, now.values.max_series, now.values.max_window_months], ['server', null, 3, 2]);
    const row = (/** @type {string} */ k, /** @type {string} */ m) => ({ instrument: `NSE-FNO-ALPHA-${k}`, month: m, timeframe: '1s', rows: 1 });
    const four = seriesPlan([row('A', '2019-01'), row('B', '2019-01'), row('C', '2019-01'), row('D', '2019-01')]);
    assert.match(String(four.why), /^too_many_series: this selection reaches 4 series and one window request reads at most 3/);
    const long = seriesPlan([row('A', '2019-01'), row('A', '2019-03')]);
    assert.match(String(long.why), /span 3, past the 2 one window request reads/);
    assert.equal(ceilingText(2, 2), "2 of 3 series · 2 of 2 months (the server's ceilings, /settings.json)");
    assert.equal(ceilingText(0, 0), '', 'a request naming nothing is not counted');
    // A CEILING THE ANSWER LACKS, OR SERVES AS NO WHOLE NUMBER, keeps the build's copy, named.
    const partial = adoptCeilings({ ceilings: [{ name: 'max_series', value: -1 }, { name: 'max_window_months', value: 'x' }] });
    assert.equal(partial.values.max_series, MAX_SERIES);
    assert.equal(partial.values.max_window_months, MAX_WINDOW_MONTHS);
    assert.match(String(partial.why), /serves no usable max_series, max_window_months, max_window_limit, browser_sort_budget, export_cap, so this build's copy is in force for them/);
    assert.match(String(adoptCeilings({ ceilings: [{ name: 'max_series', value: 9 }, { name: 'max_window_months', value: 9 }, { name: 'max_window_limit', value: 9 }, { name: 'browser_sort_budget', value: 9 }] }).why),
      /no usable export_cap, so this build's copy is in force for it$/);
    assert.match(String(adoptCeilings({ schema_version: 1 }).why), /serves no ceilings/);
    assert.match(ceilingText(1, 1, adoptCeilings(null)), /this build's copies of the ceilings/);
    // SOURCE SHAPE: the banner reads /settings.json once and adopts it; the controls say it.
    const banner = readFileSync(new URL('../src/routes/db/DbCensusBanner.svelte', import.meta.url), 'utf8');
    assert.match(banner, /void ask\('\/settings\.json', \{ cache: 'no-store' \}\)/);
    assert.match(banner, /ceilings: \{ get: \(\) => ceilings \}/);
    const controls = readFileSync(new URL('../src/routes/db/DbControls.svelte', import.meta.url), 'utf8');
    assert.match(controls, /\{ceilingText\(db\.barQueryPlan\.keys\.length, monthSpan\(db\.barQueryPlan\.from, db\.barQueryPlan\.to\), db\.ceilings\)\}/);
  } finally {
    adoptCeilings(null, 'reset by the test');
  }
});

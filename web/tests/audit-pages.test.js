import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { mergePages, nextOrdinal, pageFor } from '../src/lib/audit-pages.js';

const PER = 200;

/**
 * What the server returns for `page` when the journal holds `total`.
 * @param {number} page
 * @param {number} total
 */
function serve(page, total) {
  const end = Math.max(0, total - page * PER);
  const start = Math.max(0, end - PER);
  const rows = [];
  for (let o = end - 1; o >= start; o -= 1) rows.push({ ordinal: o });
  return rows;
}

test('growth between reads leaves a named gap instead of a silent hole (P1-06-03)', () => {
  // 450 records: page 0 [250,450), then "load older" reads page 1 [50,250).
  const older = serve(1, 450);
  // A pull appends 30; page 0 refreshes to [280,480). [250,280) is on neither.
  const head = serve(0, 480);
  const merged = mergePages(head, older);
  assert.deepEqual(merged.gaps, [{ from: 250, to: 279 }]);
  // The next read targets the newest missing ordinal, on the page that holds
  // it for the total the server last reported.
  const want = nextOrdinal(merged);
  assert.equal(want, 279);
  const page = pageFor(/** @type {number} */ (want), 480, PER);
  assert.equal(page, 1);
  const filled = mergePages(head, [...older, ...serve(/** @type {number} */ (page), 480)]);
  assert.deepEqual(filled.gaps, []);
  assert.equal(filled.runs.length, 480 - 50);
});

test('an overlapping older page never repeats an ordinal (P1-06-03)', () => {
  // Page 0 read at 450 records; page 1 read after growth to 480 overlaps it.
  const head = serve(0, 450);
  const older = serve(1, 480);
  const merged = mergePages(head, older);
  const ordinals = merged.runs.map((r) => r.ordinal);
  assert.equal(new Set(ordinals).size, ordinals.length, 'no ordinal twice');
  assert.deepEqual(merged.gaps, []);
  assert.ok(ordinals.every((o, i) => i === 0 || ordinals[i - 1] > o), 'newest first');
});

test('the next read below a contiguous set is the oldest held minus one, and none below zero', () => {
  assert.equal(nextOrdinal(mergePages(serve(0, 450), [])), 249);
  assert.equal(nextOrdinal(mergePages(serve(0, 150), [])), null);
  assert.equal(pageFor(0, 0, PER), null);
  assert.equal(pageFor(449, 450, PER), 0);
  assert.equal(pageFor(250, 450, PER), 0);
  assert.equal(pageFor(249, 450, PER), 1);
  assert.equal(pageFor(450, 450, PER), null);
});

test('the audit page merges by ordinal and names a gap', () => {
  const page = readFileSync(new URL('../src/routes/audit/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /mergePages\(/);
  assert.match(page, /pageFor\(/);
  assert.doesNotMatch(page, /\[\.\.\.\(payload\?\.runs \?\? \[\]\), \.\.\.older\]/, 'no positional concatenation');
});

test('an older page the server could not read is an error, never an empty page held (OBSV-10)', async () => {
  // D-3209. `journal_block` answers 200 with `runs:[]` and `runs_error` when
  // its page read fails. `readOlder` appended the empty list and counted a
  // page held, so the failure was never shown -- `runs_error` was rendered
  // only while no run at all was on screen -- and a failed read spent one of
  // `MAX_PAGES`.
  const { olderPageOf } = await import('../src/lib/audit-pages.js');
  assert.deepEqual(olderPageOf({ runs: [], runs_error: 'page 3: short read' }), {
    runs: [],
    error: 'page 3: short read'
  });
  assert.deepEqual(olderPageOf({ runs: [{ ordinal: 1 }] }), { runs: [{ ordinal: 1 }], error: null });
  assert.deepEqual(olderPageOf({ runs: [{ ordinal: 1 }], runs_error: '  ' }), {
    runs: [{ ordinal: 1 }],
    error: null
  });
  assert.deepEqual(olderPageOf({ runs: 'nope' }), {
    runs: [],
    error: 'the older page carried no runs list'
  });
  assert.deepEqual(olderPageOf(null), { runs: [], error: 'the older page carried no runs list' });

  const page = readFileSync(new URL('../src/routes/audit/+page.svelte', import.meta.url), 'utf8');
  const body = page.slice(page.indexOf('async function readOlder'), page.indexOf('function loadOlder'));
  assert.match(body, /olderPageOf\(/, 'readOlder reads the page through the helper');
  assert.ok(
    body.indexOf('olderError') !== -1 && body.indexOf('olderError') < body.indexOf('pagesHeld += 1'),
    'a failed page is named before any page is counted held'
  );
  assert.match(page, /\{#if olderError\}[\s\S]*?role="alert"[\s\S]*?\{olderError\}/);
});

// F3 (OBSV-21, D-3219): `/audit.json` sends `store.generation` and
// `store.commits` as null when no census is held (`crates/api/src/audit_json.rs`;
// its own test pins `"generation":null` for an absent census). The footer
// rendered `n0(generation ?? 0)` -- "generation 0, 0 committed entries" -- and
// each sample held `generation ?? 0`, so a census that became readable while the
// page was open read as thousands of commits "measured" in seconds.
test('an unknown census generation is never rendered or subtracted as zero (F3)', async () => {
  const lib = await import('../src/lib/audit-pages.js');
  const { generationOf, generationStep } = /** @type {any} */ (lib);
  for (const [g, kept] of [[null, null], [undefined, null], [0, 0], [4000, 4000], [-1, null], [1.5, null], ['7', null]]) {
    assert.equal(generationOf({ generation: g }), kept, `generation ${String(g)}`);
  }
  assert.equal(generationOf(null), null);
  assert.equal(generationStep(null, 4000), null);
  assert.equal(generationStep(4000, null), null);
  assert.equal(generationStep(4000, 4003), 3);
  assert.equal(generationStep(0, 0), 0);

  const page = readFileSync(new URL('../src/routes/audit/+page.svelte', import.meta.url), 'utf8');
  const { parse } = await import('svelte/compiler');
  const { whole } = await import('../src/lib/money.js');
  const ast = /** @type {any} */ (parse(page, { modern: true }));
  /** @type {string[]} */ const footer = [];
  (function walk(/** @type {any} */ node) {
    if (!node || typeof node !== 'object') return;
    if (node.type === 'ExpressionTag') {
      const text = page.slice(node.expression.start, node.expression.end);
      if (/payload\.store\.(generation|commits)/.test(text)) footer.push(text);
    }
    for (const value of Object.values(node)) if (value && typeof value === 'object' && value !== node.expression) walk(value);
  })(ast.fragment);
  assert.equal(footer.length, 2, 'the census line renders generation and commits');
  for (const text of footer) {
    assert.equal(new Function('n0', 'payload', `return (${text});`)(whole, { store: { generation: null, commits: null } }), '—', text);
    assert.equal(new Function('n0', 'payload', `return (${text});`)(whole, { store: { generation: 0, commits: 0 } }), '0', text);
  }

  const body = ast.instance.content.body;
  const pulseDecl = body.flatMap((/** @type {any} */ n) => n.type === 'VariableDeclaration' ? n.declarations : [])
    .find((/** @type {any} */ d) => d.id?.name === 'pulse');
  const arrow = pulseDecl.init.arguments[0];
  const pulse = (/** @type {any[]} */ samples, /** @type {any} */ base) => new Function('payload', 'samples', 'base', 'FRESH_SECS', 'generationStep',
    `return (${page.slice(arrow.start, arrow.end)})();`)({ at: 100, store: { state: 'held', committed_at: 0 } }, samples, base, 30, generationStep);
  const sample = (/** @type {number} */ at, /** @type {number|null} */ gen) => ({ at, bars: 10, im: 1, gen, months: new Map() });
  const unknownThenHeld = pulse([sample(90, null), sample(100, 4000)], { at: 90, bars: 10, im: 1, gen: null });
  assert.equal(unknownThenHeld.dGen, null, 'no commits were measured across an unknown generation');
  assert.equal(unknownThenHeld.stepped, null);
  assert.equal(unknownThenHeld.sinceOpen.gen, null);
  assert.notEqual(unknownThenHeld.state, 'moving', 'an unknown-to-known generation is not a run filing bars');
  const measured = pulse([sample(90, 4000), sample(100, 4002)], { at: 90, bars: 10, im: 1, gen: 4000 });
  assert.equal(measured.dGen, 2);
  assert.equal(measured.stepped, 2);
  assert.equal(measured.sinceOpen.gen, 2);
  assert.equal(measured.state, 'moving');
  assert.match(page, /\{#if pulse\.sinceOpen && pulse\.sinceOpen\.gen === null\}/, 'an unknown delta is said, not shown as "unchanged"');
});

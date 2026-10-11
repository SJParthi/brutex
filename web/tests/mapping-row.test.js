// WP-61, DB-G27: /mapping promised the candidates of an ambiguous row and
// printed only a count. The rows below are shaped like `/indexmap.json`'s.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { candidatesOf, noteOf, listedFrom } from '../src/lib/mapping-row.js';

const page = readFileSync(new URL('../src/routes/mapping/+page.svelte', import.meta.url), 'utf8');

const ambiguous = {
  symbol: 'NIFTY500QLTY50',
  nse: null,
  why: 'ambiguous',
  candidates: 2,
  candidate_names: ['NIFTY500 MULTICAP MOMENTUM QUALITY 50', 'NIFTY500 QUALITY 50'],
  kind: 'ambiguous'
};
const renamed = {
  symbol: 'NIFTYFINSERVICE',
  nse: 'NIFTY FINANCIAL SERVICES',
  basis: 'aliased',
  alias: { source: 'docs/00-charter.md, Index aliases (D-3332)' },
  unaided: {
    why: 'ambiguous',
    candidates: 3,
    candidate_names: [
      'NIFTY FINANCIAL SERVICES',
      'NIFTY FINANCIAL SERVICES EX-BANK',
      'NIFTY MIDSMALL FINANCIAL SERVICES'
    ]
  },
  kind: 'aliased'
};

test('an ambiguous row names every candidate the server sent', () => {
  const c = candidatesOf(ambiguous);
  assert.deepEqual(c?.names, ambiguous.candidate_names);
  assert.equal(c?.count, 2);
  assert.equal(c?.unaided, false);
  assert.match(noteOf(ambiguous), /^2 published names accept this abbreviation, listed below\. Picking one would be a guess\.$/);
});

test('a renamed row shows the three names the rule alone matched (NIFTYFINSERVICE)', () => {
  const c = candidatesOf(renamed);
  assert.equal(c?.count, 3);
  assert.equal(c?.names.length, 3);
  assert.equal(c?.unaided, true);
  const said = noteOf(renamed);
  assert.match(said, /rule alone matches 3 published names/);
  assert.match(said, /Source: docs\/00-charter\.md/);
});

test('an older API that sends only a count says the names are not shown, not absent', () => {
  const old = { symbol: 'NIFTYEV', why: 'ambiguous', candidates: 9, kind: 'ambiguous' };
  assert.deepEqual(candidatesOf(old)?.names, []);
  assert.match(noteOf(old), /^9 published names accept this abbreviation\. Picking one would be a guess\. The server named 0 of the 9/);
  assert.equal(candidatesOf({ symbol: 'NIFTY', basis: 'published', kind: 'published' }), null);
  assert.equal(candidatesOf(null), null);
});

test('GDFL, which ships no master, is labelled stored, not listed', () => {
  assert.equal(listedFrom({ listed_from: 'census' }).label, 'Stored by feed');
  assert.match(listedFrom({ listed_from: 'census' }).sentence, /stored, not mastered/);
  assert.equal(listedFrom({ listed_from: 'master' }).label, 'Listed by feed');
});

test('the page renders the names, from the module', () => {
  assert.match(page, /from '\$lib\/mapping-row\.js'/);
  assert.match(page, /\{#each candidatesOf\(row\)\?\.names \?\? \[\] as name \(name\)\}/);
  assert.match(page, /\{source\.label\}/);
});

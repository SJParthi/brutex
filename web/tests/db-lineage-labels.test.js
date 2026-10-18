// A RE-KEYED CONTRACT NAMES ITS SUCCESSOR OR PREDECESSOR (AH-03,
// g-ui-dom-vs-json-2; D-3331, WP-12c).
//
// `node --test web/tests/` — the words /db prints for `/bars/window.json`'s
// `superseded_by` / `supersedes` and `/dayflags.json`'s lineage records, and
// the one lineage request for a key the census does not hold. The shapes are
// the routes' (`crates/api/src/bars_route.rs`, `crates/api/src/dayflags.rs`);
// every key, strike and count is invented (underlying FIXTUREIDX, strikes no
// listed contract carries).

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  supersessionOf,
  supersededWords,
  continuesWords,
  lifeCutBySupersession,
  lineageRecordLine,
  lineageLines,
  contractKeyOf,
  fnoExchanges,
  askLineage,
  forgetLineage,
  withdrawnWords,
  keyHref,
  kindWord,
  LINEAGE_MEMO_MAX
} from '../src/lib/db/lineage-labels.js';
import { lineageOf, bandOf } from '../src/lib/db/dayflags.js';
import { instrumentDescription } from '../src/lib/db/decoration.js';
import { readFileSync } from 'node:fs';

/** Census rows as `db.deco` holds them: the key and its decoration's fields. */
const deco = (/** @type {string[]} */ keys) => keys.map((instrument) => ({ instrument, ...instrumentDescription(instrument) }));

const OLD = 'NSE-FNO-FIXTUREIDX-2031-07-31-777700-PE';
const NEW = 'NSE-FNO-FIXTUREIDX-2031-07-30-777700-PE';
const NEWER = 'NSE-FNO-FIXTUREIDX-2031-07-29-777700-PE';

/** A `/bars/window.json` body with the re-key's two lists. */
function windowBody(/** @type {any} */ by = null, /** @type {any} */ of = null) {
  return { total: 3, offset: 0, superseded_by: by, supersedes: of, bars: [] };
}

test('a_superseded_key_names_its_successor_in_the_expiry_note', () => {
  const s = supersessionOf(windowBody([{ key: OLD, by: NEW, kind: 'vendor_redated', recorded: false }]));
  assert.equal(s.by.length, 1);
  assert.equal(supersededWords(s.by[0]), `superseded by ${NEW} (the vendor re-dated its expiry; recorded no: the re-key has not moved its bars, so each key holds part of its life)`);
  // The expiry note's clause names the successor instead of "may not have
  // traded, or the census is behind".
  const note = lifeCutBySupersession(s);
  assert.match(note, new RegExp(`^the contract continues under another key: ${OLD} is superseded by ${NEW}`));
  assert.doesNotMatch(note, /may not have traded|census is behind/);
  // A recorded move says so.
  const rec = supersessionOf(windowBody([{ key: OLD, by: NEW, kind: 'closed_day', recorded: true }]));
  assert.match(supersededWords(rec.by[0]), /its stated expiry is a closed day; recorded yes: the re-key copied its bars there/);
  // And the link opens the new key on /db.
  assert.equal(keyHref('gdfl', NEW, '1s'), `/db?feed=gdfl&key=${NEW}&tf=1s`);
  assert.equal(keyHref('gdfl', 'not a key', '1s'), null);
  // Nothing superseded: the caller keeps its own words.
  assert.equal(lifeCutBySupersession(supersessionOf(windowBody())), '');
});

test('a_new_key_names_the_key_it_continues', () => {
  const s = supersessionOf(windowBody(null, [{ key: NEW, of: OLD, kind: 'vendor_redated', bars: 4321, blake3: 'ab'.repeat(32) }]));
  assert.equal(s.of.length, 1);
  assert.equal(continuesWords(s.of[0]), `continues ${OLD}: 4,321 bar(s) copied from it (the vendor re-dated its expiry)`);
  // No count sent: no number invented.
  assert.equal(continuesWords({ of: OLD, kind: 'vendor_redated', bars: null }), `continues ${OLD} (the vendor re-dated its expiry)`);
});

test('lineageOf_keeps_supersedes_superseded_by_and_retired', () => {
  const got = lineageLines([
    { kind: 'superseded_by', key: NEW, why: 'vendor_redated', months: 2, bars: 53, blake3: 'cd'.repeat(32) },
    { kind: 'supersedes', key: OLD, why: 'vendor_redated', months: 2, bars: 53, blake3: 'cd'.repeat(32) },
    { kind: 'retired', why: 'strike_implausible', day: '2031-07-01', median: 5, days: 3, bars: 9, refusal: 'fixture refusal sentence' },
    { kind: 'stopped', ex: '2031-06-15' },
    { kind: 'continues_as', key: NEW, ex: '2031-06-15', factor: '1/2' },
    { kind: 'renamed_from', key: OLD, ex: '2031-06-15', factor: '1/2' },
    { kind: 'stitched', ex: '2031-06-15', before: null, after: OLD },
    { kind: 'a_kind_from_tomorrow' }
  ]);
  assert.equal(got.lines[0], `superseded by ${NEW} (the vendor re-dated its expiry; recorded yes: the re-key copied its 53 bar(s) in 2 month(s) there)`);
  assert.equal(got.lines[1], `continues ${OLD}: 53 bar(s) copied from it in 2 month(s) (the vendor re-dated its expiry)`);
  assert.match(got.lines[2], /^retired: the decoder refuses every day of it — its strike is implausible for the underlying \(9 bar\(s\) stored\); fixture refusal sentence$/);
  assert.match(got.lines[3], /^stopped: .* ended the day before the 2031-06-15 ex-date/);
  assert.equal(got.lines[4], `renamed at the 2031-06-15 ex-date, continues as ${NEW} (factor 1/2)`);
  assert.equal(got.lines[5], `renamed at the 2031-06-15 ex-date from ${OLD} (factor 1/2)`);
  assert.match(got.lines[6], /holds two exchange contracts/);
  // A kind this page has no words for is counted and said, never dropped.
  assert.equal(got.unknown, 1);
  assert.match(got.lines[7], /^1 lineage record\(s\) of a kind this page has no words for/);
  assert.equal(got.successor, NEW);
  assert.equal(got.retired, true);
  assert.equal(lineageRecordLine({ kind: 'retired', why: 'expiry_unsettled' }), 'retired: the decoder refuses every day of it — its expiry is not a settled one');
  assert.equal(lineageRecordLine(null), null);
  assert.deepEqual(lineageLines(undefined), { lines: [], unknown: 0, malformed: 0, successor: null, retired: false });
  // THE DAY BAND'S lineageOf (dayflags.js) keeps every kind too: it used to
  // word only continues_as, renamed_from and stitched and drop the rest.
  const band = lineageOf({ present: true, records: [
    { kind: 'superseded_by', key: NEW, why: 'vendor_redated', months: 1, bars: 6 },
    { kind: 'supersedes', key: OLD, why: 'closed_day', months: 1, bars: 6 },
    { kind: 'retired', why: 'expiry_unsettled', bars: 2 }
  ], why: null });
  assert.equal(band?.lines.length, 3);
  assert.match(band?.lines[0] ?? '', new RegExp(`^superseded by ${NEW} `));
  assert.match(band?.lines[1] ?? '', new RegExp(`^continues ${OLD}: 6 bar`));
  assert.match(band?.lines[2] ?? '', /^retired: /);
  assert.equal(band?.successor, NEW);
  assert.equal(band?.retired, true);
  assert.equal(band?.present, true);
  // Through bandOf, as the page reads it.
  const viaBand = bandOf({ days: [], lineage: { present: true, records: [{ kind: 'supersedes', key: OLD, why: 'vendor_redated', bars: 1 }] } });
  assert.match(viaBand.lineage?.lines[0] ?? '', new RegExp(`^continues ${OLD}`));
});

test('a key superseded twice, and a key that both supersedes and is superseded, each name their own link', () => {
  // NEW continues OLD and is itself superseded by NEWER (a chain).
  const s = supersessionOf(
    windowBody([{ key: NEW, by: NEWER, kind: 'closed_day', recorded: true }], [{ key: NEW, of: OLD, kind: 'vendor_redated', bars: 7 }])
  );
  assert.equal(s.by[0].by, NEWER);
  assert.equal(s.of[0].of, OLD);
  const chain = lineageLines([
    { kind: 'supersedes', key: OLD, why: 'vendor_redated', months: 1, bars: 7 },
    { kind: 'superseded_by', key: NEWER, why: 'closed_day', months: 1, bars: 7 }
  ]);
  // The successor is the key it is superseded BY, never the one it continues.
  assert.equal(chain.successor, NEWER);
  // Many superseded keys: three named, the rest counted.
  const many = supersessionOf(windowBody([1, 2, 3, 4, 5].map((i) => ({ key: `${OLD}${i}`, by: NEW, kind: 'vendor_redated', recorded: false }))));
  assert.match(lifeCutBySupersession(many), /; and 2 more key\(s\) the same way$/);
  // An entry not in the route's shape is counted, never dropped in silence.
  const bad = supersessionOf(windowBody([{ key: OLD }, 'x', null], [{ of: 3 }]));
  assert.deepEqual([bad.by.length, bad.of.length, bad.malformed], [0, 0, 4]);
  // A kind word the ledger adds tomorrow is printed as spelled.
  assert.equal(kindWord('lot_rebased'), 'lot rebased');
});

test('a_withdrawn_key_is_explained_by_its_lineage_not_the_filters', async () => {
  forgetLineage();
  const rows = fnoExchanges(deco(['NSE-FNO-FIXTUREIDX-2031-08-28-800000-CE', 'NSE-INDEX-FIXTUREIDX']));
  const key = contractKeyOf(rows, { sym: 'FIXTUREIDX', expiry: '2031-07-31', strikes: new Set(['777700']), side: 'PE' });
  assert.equal(key, OLD);
  /** @type {string[]} */
  const asked = [];
  const fetchJson = async (/** @type {string} */ url) => {
    asked.push(url);
    return { days: [], lineage: { present: true, records: [{ kind: 'superseded_by', key: NEW, why: 'vendor_redated', months: 1, bars: 53 }], why: null } };
  };
  // Pending: the filters are not blamed while the one request runs.
  const pending = withdrawnWords(OLD, null);
  assert.equal(pending.state, 'pending');
  assert.match(pending.text, /reading its lineage before any filter is blamed/);
  const [a, b] = await Promise.all([askLineage('gdfl', OLD, fetchJson), askLineage('gdfl', OLD, fetchJson)]);
  // ONE request for the key, however many surfaces ask: the route the
  // O(1) proof measures, one day (the expiry) of the key.
  assert.equal(asked.length, 1);
  assert.equal(asked[0], `/dayflags.json?feed=gdfl&instrument=${OLD}&from=2031-07-31&to=2031-07-31&order=asc`);
  assert.equal(a, b);
  const w = withdrawnWords(OLD, a);
  assert.equal(w.state, 'explained');
  assert.equal(w.successor, NEW);
  assert.match(w.text, new RegExp(`^${OLD} is not in the census: superseded by ${NEW}`));
  assert.match(w.text, /The rungs it set are its own fields, not a filter that hides it\.$/);
  assert.doesNotMatch(w.text, /no longer offer/);
});

test('a withdrawn key the ledger holds nothing for, or whose lineage fails, says which', async () => {
  forgetLineage();
  const quiet = await askLineage('gdfl', OLD, async () => ({ lineage: { present: true, records: [], why: null } }));
  const q = withdrawnWords(OLD, quiet);
  assert.equal(q.state, 'unexplained');
  assert.match(q.text, /the lineage ledger files no record under it$/);
  forgetLineage();
  const noLedger = await askLineage('gdfl', OLD, async () => ({ lineage: { present: false, records: [], why: 'fixture: no ledger yet' } }));
  assert.match(withdrawnWords(OLD, noLedger).text, /\(fixture: no ledger yet\)$/);
  forgetLineage();
  const failed = await askLineage('gdfl', OLD, async () => {
    throw new Error('fixture: refused 503');
  });
  const f = withdrawnWords(OLD, failed);
  assert.equal(f.state, 'failed');
  assert.match(f.text, /its lineage could not be read \(fixture: refused 503\)/);
  // A feed whose answer carries no lineage section (not a GDFL contract).
  forgetLineage();
  const zerodha = await askLineage('zerodha', OLD, async () => ({ days: [] }));
  assert.equal(withdrawnWords(OLD, zerodha).state, 'unexplained');
});

test('the lineage memo is bounded and the rungs must name one contract', async () => {
  forgetLineage();
  let n = 0;
  const fetchJson = async () => {
    n++;
    return { lineage: { present: true, records: [] } };
  };
  for (let i = 0; i < LINEAGE_MEMO_MAX + 5; i++) await askLineage('gdfl', `NSE-FNO-FIXTUREIDX-2031-07-31-${100000 + i}-CE`, fetchJson);
  assert.equal(n, LINEAGE_MEMO_MAX + 5);
  // The oldest was forgotten, so asking it again asks once more; the newest is kept.
  await askLineage('gdfl', 'NSE-FNO-FIXTUREIDX-2031-07-31-100000-CE', fetchJson);
  await askLineage('gdfl', `NSE-FNO-FIXTUREIDX-2031-07-31-${100000 + LINEAGE_MEMO_MAX + 4}-CE`, fetchJson);
  assert.equal(n, LINEAGE_MEMO_MAX + 6);
  const rows = fnoExchanges(deco(['NSE-FNO-FIXTUREIDX-2031-08-28-FUT']));
  // Two strikes, no side, a bad expiry, an unknown underlying: no one contract.
  assert.equal(contractKeyOf(rows, { sym: 'FIXTUREIDX', expiry: '2031-07-31', strikes: ['1', '2'], side: 'PE' }), null);
  assert.equal(contractKeyOf(rows, { sym: 'FIXTUREIDX', expiry: '2031-07-31', strikes: ['1'], side: '' }), null);
  assert.equal(contractKeyOf(rows, { sym: 'FIXTUREIDX', expiry: '31-07-2031', strikes: ['1'], side: 'PE' }), null);
  assert.equal(contractKeyOf(rows, { sym: 'OTHERIDX', expiry: '2031-07-31', strikes: ['1'], side: 'PE' }), null);
  assert.equal(contractKeyOf(new Map(), { sym: 'FIXTUREIDX', expiry: '2031-07-31', strikes: ['1'], side: 'PE' }), null);
  // A future: the expiry alone.
  assert.equal(contractKeyOf(rows, { sym: 'FIXTUREIDX', expiry: '2031-07-31', strikes: [], side: '' }), 'NSE-FNO-FIXTUREIDX-2031-07-31-FUT');
});

test('contractKeyOf_is_one_probe_of_a_fold_made_once_per_census_load', () => {
  // AH-03 REPAIR 1: the first cut walked the census on every recompute of the
  // withdrawn-key derived (134-280 ms on GDFL's 2,098,428 rows for an
  // old-name or absent underlying). The census here is an iterable that
  // counts its passes; the fold takes one, and a thousand keys take none.
  const keys = [
    'NSE-INDEX-FIXTUREIDX',
    'NSE-CASH-FIXTURESHARE',
    'NSE-FNO-FIXTURESHARE-2031-07-31-FUT',
    'NSE-FNO-FIXTUREIDX-2031-08-28-800000-CE',
    // A second exchange's F&O row of the same underlying, later in served
    // order: the FIRST row's exchange is the one taken, as before.
    'BSE-FNO-FIXTUREIDX-2031-08-28-800000-CE',
    // A malformed key the decoration kept as its own name adds nothing.
    'NSE-FNO'
  ];
  const rows = deco(keys);
  let passes = 0;
  const census = {
    [Symbol.iterator]() {
      passes++;
      return rows[Symbol.iterator]();
    }
  };
  const exchanges = fnoExchanges(census);
  assert.equal(passes, 1);
  assert.deepEqual([...exchanges], [
    ['FIXTURESHARE', 'NSE'],
    ['FIXTUREIDX', 'NSE']
  ]);
  for (let i = 0; i < 1000; i++) {
    assert.equal(
      contractKeyOf(exchanges, { sym: 'FIXTUREIDX', expiry: '2031-07-31', strikes: new Set([String(700000 + i)]), side: 'PE' }),
      `NSE-FNO-FIXTUREIDX-2031-07-31-${700000 + i}-PE`
    );
  }
  assert.equal(contractKeyOf(exchanges, { sym: 'FIXTURESHARE', expiry: '2031-07-31', strikes: [], side: '' }), 'NSE-FNO-FIXTURESHARE-2031-07-31-FUT');
  // An old-name or absent underlying is one probe that misses, not a full pass.
  assert.equal(contractKeyOf(exchanges, { sym: 'FIXTUREGONE', expiry: '2031-07-31', strikes: ['1'], side: 'PE' }), null);
  assert.equal(passes, 1);
  // Given the rows, it refuses loudly instead of walking them.
  assert.throws(() => contractKeyOf(/** @type {any} */ (rows), { sym: 'FIXTUREIDX', expiry: '2031-07-31', strikes: ['1'], side: 'PE' }), TypeError);
  assert.equal(fnoExchanges(null).size, 0);
});

test('the withdrawn-key derived folds the census in its own derived of db.deco, never inside the key', () => {
  // The fold must recompute once per census load, so it is a `$derived` of
  // `db.deco` alone, and the key derived probes it: source-shape, because
  // Svelte's dependency tracking is what bounds it.
  const src = readFileSync(new URL('../src/routes/db/DbBarsGrid.svelte', import.meta.url), 'utf8');
  const at = src.indexOf('function withdrawnState(db)');
  assert.ok(at > 0);
  const body = src.slice(at, src.indexOf('Object.defineProperties(db', at));
  assert.match(body, /const exchanges = \$derived\(fnoExchanges\(db\.deco\)\);/);
  assert.match(body, /contractKeyOf\(exchanges, /);
  assert.doesNotMatch(src, /contractKeyOf\(db\.deco/);
  assert.equal((src.match(/fnoExchanges\(/g) ?? []).length, 1);
  // The lineage memo is forgotten on a new census generation or epoch, never on
  // the rows swap the page makes once per open (facet rows, then the whole
  // census): keyed on the rows, the shadow asked the same key twice.
  assert.match(body, /const censusId = \$derived\(`\$\{store\.feed \?\? ''\}\\u0000\$\{store\.lag\?\.generation \?\? ''\}\\u0000\$\{store\.lag\?\.epoch \?\? ''\}`\);/);
  assert.match(body, /const census = censusId;\n\s*if \(seenCensus !== null && seenCensus !== census\) forgetLineage\(\);/);
});

test('a failed lineage read is not kept: the next ask is a new request, and one key can be forgotten alone', async () => {
  forgetLineage();
  let n = 0;
  const flaky = async () => {
    n++;
    if (n === 1) throw new Error('fixture: refused 503');
    return { lineage: { present: true, records: [{ kind: 'superseded_by', key: NEW, why: 'vendor_redated', bars: 53 }], why: null } };
  };
  const first = await askLineage('gdfl', OLD, flaky);
  assert.equal(first.state, 'failed');
  const second = await askLineage('gdfl', OLD, flaky);
  assert.equal(n, 2);
  assert.equal(withdrawnWords(OLD, second).state, 'explained');
  // A read answer is kept…
  await askLineage('gdfl', OLD, flaky);
  assert.equal(n, 2);
  // …until that one key is forgotten ("Read the lineage again"); another key's stays.
  await askLineage('gdfl', NEW, flaky);
  assert.equal(n, 3);
  forgetLineage('gdfl', OLD);
  await askLineage('gdfl', OLD, flaky);
  await askLineage('gdfl', NEW, flaky);
  assert.equal(n, 4);
  // A census load forgets every one.
  forgetLineage();
  await askLineage('gdfl', NEW, flaky);
  assert.equal(n, 5);
});

test('a lineage record that names no key is malformed, never "superseded by undefined"', () => {
  for (const kind of ['superseded_by', 'supersedes', 'continues_as', 'renamed_from']) {
    assert.equal(lineageRecordLine({ kind, why: 'vendor_redated', bars: 5 }), null, kind);
    assert.equal(lineageRecordLine({ kind, key: '', why: 'vendor_redated' }), null, kind);
  }
  const got = lineageLines([{ kind: 'superseded_by', why: 'vendor_redated', bars: 5 }, { kind: 'superseded_by', key: NEW, why: 'vendor_redated' }, { kind: 'fixture_kind' }]);
  assert.equal(got.malformed, 1);
  assert.equal(got.unknown, 1);
  assert.equal(got.successor, NEW);
  assert.ok(got.lines.every((l) => !l.includes('undefined')), got.lines.join(' | '));
  assert.match(got.lines.at(-1) ?? '', /^1 lineage record\(s\) that name no key/);
  // A re-key list that is not a list is one malformed entry, not "not re-keyed".
  assert.equal(supersessionOf({ superseded_by: { key: OLD, by: NEW }, supersedes: null }).malformed, 1);
  assert.equal(supersessionOf({ superseded_by: 'fixture', supersedes: 7 }).malformed, 2);
  assert.equal(supersessionOf({ superseded_by: null }).malformed, 0);
  assert.equal(supersessionOf({}).malformed, 0);
});

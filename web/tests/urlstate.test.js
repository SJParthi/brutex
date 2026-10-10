// THE SELECTION IN THE ADDRESS BAR, DRIVEN.
//
// `/ingest` kept every control in memory and wrote none of it down, so a reload
// reset the universe and the instrument list and re-derived the feed from
// whatever the store happened to hold. `$lib/urlstate.js` is the half of the
// fix with no runes in it, extracted for the same reason `find.js` and
// `prefix.js` were: the page imports `$lib/ask.js`, node cannot resolve that
// alias, and a rule that cannot be driven is a rule nobody is checking.
//
// The two properties worth pinning are idempotence -- one selection is always
// one address, byte for byte -- and the three-way distinction the page depends
// on: a field that is ABSENT, a field that is EMPTY, and a field that names
// things are three different statements and must not collapse into two.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { encode, decode, packSet, unpackSet, same, KEYS } from '../src/lib/urlstate.js';

/**
 * The Set a field decoded to, asserted present and sorted.
 *
 * `decode` returns `Set | undefined` DELIBERATELY -- absent and empty are
 * different answers, which is the property half these tests exist to pin -- so
 * every read of one has to say which it expected. This says "present", once,
 * instead of a cast at each call site.
 *
 * @param {Set<string> | undefined} s
 * @returns {string[]}
 */
function sorted(s) {
  assert.ok(s instanceof Set, `expected a Set, got ${String(s)}`);
  return [...s].sort();
}

/* ── the round trip ───────────────────────────────────────────────────── */

test('a full selection survives the round trip unchanged', () => {
  const sel = {
    feeds: ['dhan', 'groww'],
    universe: 'fno',
    members: ['BANKNIFTY'],
    segs: ['spot', 'xfut', 'xopt'],
    rungs: ['1min', '1day'],
    from: '2026-01-01',
    to: '2026-08-19'
  };
  const back = decode(encode(sel));
  assert.deepEqual(sorted(back.feeds), ['dhan', 'groww']);
  assert.equal(back.universe, 'fno');
  assert.deepEqual(sorted(back.members), ['BANKNIFTY']);
  assert.deepEqual(sorted(back.segs), ['spot', 'xfut', 'xopt']);
  assert.deepEqual(sorted(back.rungs), ['1day', '1min']);
  assert.equal(back.from, '2026-01-01');
  assert.equal(back.to, '2026-08-19');
});

test('THE REGRESSION: the universe and the instruments are what reset', () => {
  // Measured on the running page before this existed: F&O Underlyings became
  // NIFTY 50 and "All 211 ticked" became "All 50 ticked" on a bare reload.
  const url = encode({ universe: 'fno', members: ['BANKNIFTY'] });
  const back = decode(url);
  assert.equal(back.universe, 'fno', 'the universe must survive a reload');
  assert.deepEqual(sorted(back.members), ['BANKNIFTY']);
});

test('the same selection is always the same address, whatever order it was built in', () => {
  // CLAUDE.md §3 rule 5. `Set` iteration is insertion order, so ticking A then
  // B and ticking B then A would otherwise bookmark to two different URLs.
  const a = encode({ feeds: new Set(['groww', 'dhan']), segs: new Set(['xopt', 'spot']) });
  const b = encode({ feeds: new Set(['dhan', 'groww']), segs: new Set(['spot', 'xopt']) });
  assert.equal(a, b);
  assert.equal(a, encode(decode(a)), 'encode(decode(x)) is a fixed point');
});

/* ── absent is not empty ──────────────────────────────────────────────── */

test('ABSENT and EMPTY are different answers and never collapse', () => {
  // "the operator said nothing" keeps the page's default; "the operator said
  // none" is a choice to honour. A parser that returned an empty Set for both
  // would silently un-tick every segment on a first visit.
  const nothing = decode('');
  assert.equal(nothing.segs, undefined, 'no seg= at all is undefined');
  assert.equal(nothing.feeds, undefined);
  assert.equal(nothing.universe, undefined);

  const none = decode('seg=');
  assert.ok(none.segs instanceof Set, 'seg= with no value is a Set');
  assert.equal(none.segs.size, 0, 'and it is empty');
});

test('members: null means ALL and is written as no field at all', () => {
  // The same rule `wireBodyFor` follows on the wire: naming every member and
  // naming none ask for the same set, and 750 fields to say "everything" is
  // waste on a query string as much as on a request body.
  assert.equal(encode({ members: null }).includes('member'), false);
  assert.equal(decode(encode({ members: null })).members, undefined);
  // An explicit empty set is the OTHER statement and does travel.
  assert.equal(encode({ members: [] }), 'member=');
  assert.equal(sorted(decode('member=').members).length, 0);
});

test('a blank SCALAR is omitted; a supplied SET is written even when empty', () => {
  // These two are not the same rule and this test originally said they were --
  // it asserted `encode({feeds: []})` was '', which is the bug the `same()`
  // test below caught. A scalar has no "empty but chosen" state: nobody
  // deliberately picks no universe. A set does: un-ticking every segment is a
  // choice the ladder gates on and names, and it has to survive a reload.
  assert.equal(encode({}), '', 'nothing supplied is nothing written');
  assert.equal(encode({ universe: '', from: '', to: '' }), '', 'blank scalars are omitted');
  assert.equal(encode({ feeds: [] }), 'feed=', 'a supplied empty set is a statement');
  assert.equal(encode({ segs: new Set() }), 'seg=');
  assert.equal(encode({ universe: 'n50' }), 'universe=n50');
});

/* ── it does not validate, deliberately ───────────────────────────────── */

test('an unknown token comes back verbatim, because the vocabulary is the page’s', () => {
  // A universe this build dropped, a rung it never had. Silently discarding
  // them here would narrow a selection with nothing on screen to say so, which
  // is the fallback that hides a failure CLAUDE.md §4 bans. The page owns the
  // vocabularies and owns saying what it could not honour.
  const back = decode('universe=nifty_next_50&tf=7min,1min&feed=notafeed');
  assert.equal(back.universe, 'nifty_next_50');
  assert.deepEqual(sorted(back.rungs), ['1min', '7min']);
  assert.deepEqual(sorted(back.feeds), ['notafeed']);
});

/* ── the field helpers ────────────────────────────────────────────────── */

test('packSet drops blanks and duplicates and sorts what is left', () => {
  assert.equal(packSet(['b', 'a', 'b', '', '  ', ' c ']), 'a,b,c');
  assert.equal(packSet([]), '');
  assert.equal(packSet(null), '');
  assert.equal(packSet(undefined), '');
});

test('unpackSet is packSet’s inverse over the tokens that survive it', () => {
  assert.deepEqual([...unpackSet('a,b,c')], ['a', 'b', 'c']);
  assert.deepEqual([...unpackSet(' a , , b ')], ['a', 'b'], 'blanks and padding go');
  assert.equal(unpackSet('').size, 0);
  assert.equal(unpackSet(null).size, 0);
});

test('a value carrying a comma or a space still survives the round trip', () => {
  // URLSearchParams encodes; the sets are tokenised on the comma, so a token
  // that CONTAINS one cannot round-trip and this pins which way it fails.
  const back = decode(encode({ universe: 'a b' }));
  assert.equal(back.universe, 'a b', 'a space survives, because it is not a separator');
});

/* ── the write-avoidance check ────────────────────────────────────────── */

test('same() sees through ordering and through absent-versus-empty', () => {
  assert.ok(same('feed=dhan,groww', 'feed=groww,dhan'), 'order is not a difference');
  assert.ok(same('universe=n50&seg=spot', 'seg=spot&universe=n50'), 'field order is not one either');
  assert.ok(!same('seg=', ''), 'none-ticked is NOT the same as nothing-said');
  assert.ok(!same('universe=n50', 'universe=fno'));
});

test('the key list is what encode actually emits, in that order', () => {
  const url = encode({
    feeds: ['dhan'], universe: 'fno', members: ['X'],
    segs: ['spot'], rungs: ['1min'], from: '2026-01-01', to: '2026-08-19'
  });
  const emitted = [...new URLSearchParams(url).keys()];
  assert.deepEqual(emitted, KEYS, 'a new field must be added to KEYS as well');
});

/* ── /db's selection in the address bar (WP-50; D-3337 item 4, DB-077) ── */

import { encodeDb, decodeDb, dbHref, DB_KEYS } from '../src/lib/urlstate.js';

test('THE REGRESSION: a /db option selection survives the address bar, field for field', () => {
  const sel = {
    feed: 'gdfl',
    seg: 'options',
    sym: 'NIFTY',
    tf: '1s',
    exp: '2019-03-28',
    k: ['1160000', '1150000'],
    side: 'CE',
    from: '2019-03-15',
    to: '2019-03-15',
    tfrom: '10:00:00',
    tto: '10:04:59',
    sort: 'c',
    dir: /** @type {'asc'} */ ('asc'),
    page: 3,
    view: 'census',
    oow: true
  };
  const q = encodeDb(sel);
  const { sel: back, refused } = decodeDb(q);
  assert.deepEqual(refused, []);
  assert.deepEqual(back, { ...sel, k: ['1150000', '1160000'] });
});

test('one /db selection is one address: strikes sorted and deduplicated, keys in D-3337 order', () => {
  const a = encodeDb({ feed: 'gdfl', sym: 'NIFTY', k: ['1160000', '1150000', '1150000'] });
  const b = encodeDb({ k: [1150000, 1160000], sym: 'NIFTY', feed: 'gdfl' });
  assert.equal(a, b);
  assert.equal(a, 'feed=gdfl&sym=NIFTY&k=1150000&k=1160000');
  const order = [...new URLSearchParams(encodeDb({ oow: true, feed: 'z', tto: '10:00', seg: 'spot' })).keys()];
  assert.deepEqual(order, ['feed', 'seg', 'tto', 'oow']);
  assert.deepEqual(DB_KEYS, ['feed', 'seg', 'sym', 'key', 'tf', 'exp', 'k', 'side', 'from', 'to', 'tfrom', 'tto', 'sort', 'dir', 'page', 'view', 'oow']);
});

test('the opening values are not written: page 1, Bars, newest first, the window applied', () => {
  assert.equal(encodeDb({ feed: 'zerodha', sym: 'NIFTY', sort: 'ts', dir: 'desc', page: 1, view: 'bars', oow: false }), 'feed=zerodha&sym=NIFTY');
  assert.equal(encodeDb({}), '');
  assert.equal(dbHref({}), '/db');
  assert.equal(dbHref({ feed: 'zerodha', seg: 'spot', sym: 'NIFTY', tf: '1day' }), '/db?feed=zerodha&seg=spot&sym=NIFTY&tf=1day');
});

test('a malformed field is named, never silently dropped or guessed', () => {
  const { sel, refused } = decodeDb('?k=12.5&from=2019-02-30&to=2019-13-01&side=xx&page=0&tfrom=25:00&dir=up&oow=yes&view=chart&sym=nifty');
  assert.deepEqual(sel, { sym: 'NIFTY' });
  assert.deepEqual(
    refused.map((r) => r.key).sort(),
    ['dir', 'from', 'k', 'oow', 'page', 'side', 'tfrom', 'to', 'view']
  );
  for (const r of refused) assert.ok(r.why.length > 0, r.key);
});

test('a link from another page carries an instrument key and a day window', () => {
  const href = dbHref({ feed: 'gdfl', key: 'NSE-FNO-NIFTY-2019-03-28-1150000-CE', tf: '1s', from: '2019-03-01', to: '2019-03-31' });
  const { sel } = decodeDb(href.slice('/db'.length));
  assert.equal(sel.key, 'NSE-FNO-NIFTY-2019-03-28-1150000-CE');
  assert.equal(sel.from, '2019-03-01');
  assert.equal(sel.to, '2019-03-31');
});

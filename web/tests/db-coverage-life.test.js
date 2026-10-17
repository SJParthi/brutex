// A CONTRACT'S STORED LIFE, AND WHERE THE STORE CUTS IT (WP-54; DB-G67,
// DB-G68, DB-G69, DB-G72; D-3331, D-3337, D-3338).
//
// "Days held" was honest and silent: nothing said a life that ends before
// its expiry was cut by the vendor's archive, by an import that has not
// reached those days, by a rename at an ex-date, or by trades that stopped;
// and nothing said a life that begins on the archive's first day began
// before it. `contractLife` names each cause from facts the caller holds.
// Every key and day below is invented in shape only.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { contractLife } from '../src/lib/db/decoration.js';

const ARCHIVE = { from: '2018-09-03', to: '2026-09-24' };
const TODAY = '2026-10-10';
const base = { archive: ARCHIVE, importedTo: '2020-09-16', today: TODAY, sessionsAfter: null, lineage: null };

/** @param {ReturnType<typeof contractLife>} marks */
const codes = (marks) => marks.map((m) => m.code);

test('a life that starts on the archive\'s first day began before it (DB-G68)', () => {
  const marks = contractLife({ firstDay: '2018-09-03', lastDay: '2018-09-27', expiry: '2018-09-27' }, { ...base, sessionsAfter: 0 });
  assert.deepEqual(codes(marks), ['before_archive']);
  assert.match(marks[0].text, /2018-09-03/);
  // A contract first stored after it carries no such mark.
  assert.deepEqual(codes(contractLife({ firstDay: '2018-09-04', lastDay: '2018-09-27', expiry: '2018-09-27' }, { ...base, sessionsAfter: 0 })), []);
});

test('a contract settling on or after today is not expired; one past the archive end names the archive (DB-G67)', () => {
  const unexpired = contractLife({ firstDay: '2026-06-01', lastDay: '2026-09-24', expiry: '2026-12-29' }, { ...base, importedTo: '2026-09-24', sessionsAfter: 60 });
  assert.deepEqual(codes(unexpired), ['unexpired', 'archive_ended']);
  // `/feeds.json` `archive` is the vendor's archive AS MEASURED, its last day
  // a dated lower bound (R3-05 repair 2, SC-CLOSE-2): past it the sessions
  // are not imported, never "not in the vendor's archive".
  assert.match(unexpired[1].text, /archive as measured reaches 2026-09-24/);
  assert.match(unexpired[1].text, /not imported/);
  assert.doesNotMatch(unexpired[1].text, /vendor archive/);
  // Settled after the archive's last day but before today: cut by the archive, and expired.
  const pastArchive = contractLife({ firstDay: '2026-08-01', lastDay: '2026-09-24', expiry: '2026-09-29' }, { ...base, importedTo: '2026-09-24', sessionsAfter: 3 });
  assert.deepEqual(codes(pastArchive), ['archive_ended']);
});

test('a life cut where the import has not reached names the import, never "no trades"', () => {
  const marks = contractLife({ firstDay: '2020-08-03', lastDay: '2020-09-16', expiry: '2020-09-24' }, { ...base, sessionsAfter: 6 });
  assert.deepEqual(codes(marks), ['import_behind']);
  assert.match(marks[0].text, /reached 2020-09-16/);
});

test('a life that ends before expiry with no rename recorded says so, and names the missing ledger (DB-G72)', () => {
  const life = { firstDay: '2018-10-01', lastDay: '2018-11-28', expiry: '2018-12-27' };
  const unknown = contractLife(life, { ...base, sessionsAfter: 20, lineage: { present: false, records: [] } });
  assert.deepEqual(codes(unknown), ['no_trade_after']);
  assert.match(unknown[0].text, /20 session\(s\) before the 2018-12-27 expiry/);
  assert.match(unknown[0].text, /no lineage ledger is written yet/);
  // A ledger that exists and names nothing for this key: the trades stopped.
  const none = contractLife(life, { ...base, sessionsAfter: 20, lineage: { present: true, records: [] } });
  assert.deepEqual(codes(none), ['no_trade_after']);
  assert.doesNotMatch(none[0].text, /lineage/);
  // Unread (Zerodha, or the read in flight): nothing is said about a rename.
  assert.doesNotMatch(contractLife(life, { ...base, sessionsAfter: 20 })[0].text, /renamed/);
});

test('a renamed contract names the key it continues as, and the new key names where it came from (DB-G72, D-3331)', () => {
  const old = 'NSE-FNO-ZZBRIT-2018-12-27-600000-CE';
  const renamed = 'NSE-FNO-ZZBRIT-2018-12-27-300000-CE';
  const before = contractLife(
    { firstDay: '2018-10-01', lastDay: '2018-11-28', expiry: '2018-12-27' },
    { ...base, sessionsAfter: 20, lineage: { present: true, records: [{ kind: 'continues_as', key: renamed, ex: '2018-11-29', factor: '2/1', lot_old: 100, lot_new: 200 }] } }
  );
  assert.deepEqual(codes(before), ['renamed']);
  assert.match(before[0].text, /continues as NSE-FNO-ZZBRIT-2018-12-27-300000-CE/);
  assert.match(before[0].text, /did not stop trading on 2018-11-28/);
  const after = contractLife(
    { firstDay: '2018-11-29', lastDay: '2018-12-27', expiry: '2018-12-27' },
    { ...base, sessionsAfter: 0, lineage: { present: true, records: [{ kind: 'renamed_from', key: old, ex: '2018-11-29', factor: '2/1' }] } }
  );
  assert.deepEqual(codes(after), ['renamed_from']);
  assert.match(after[0].text, /traded before it as NSE-FNO-ZZBRIT-2018-12-27-600000-CE/);
  const stitched = contractLife(
    { firstDay: '2018-10-01', lastDay: '2018-12-27', expiry: '2018-12-27' },
    { ...base, sessionsAfter: 0, lineage: { present: true, records: [{ kind: 'stitched', ex: '2018-11-29', before: null, after: old }] } }
  );
  assert.deepEqual(codes(stitched), ['stitched']);
  assert.match(stitched[0].text, /two exchange contracts/);
});

test('a spot series has no life to cut: no expiry, no marks past the archive one', () => {
  assert.deepEqual(codes(contractLife({ firstDay: '2018-09-04', lastDay: '2026-09-24', expiry: null }, base)), []);
});

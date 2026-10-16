// /db's BROKER FEED picker: the STORE decides whether a feed can be viewed,
// and ingest readiness (`/feeds.json` `ready`/`why`) is only a note.
//
// The regression: Global Datafeeds held 122,391 instrument-months of
// one-second bars and was struck through as "no folder at …/vendor-data/gdfl
// yet", because the picker gated on the ARCHIVE folder used for ingest.

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { feedPickerRow } from '../src/lib/feed-row.js';

const fmt = (/** @type {number} */ n) => n.toLocaleString('en-IN');
const WHY = 'no folder at /x/vendor-data/gdfl yet — the bought CSVs go there';
const gdfl = { wire: 'gdfl', display: 'Global Datafeeds', ready: false, why: WHY };

test('THE REGRESSION: a not-ready feed whose store holds bars is selectable with its count', () => {
  const row = feedPickerRow(gdfl, { cells: 122_391, bars: 3_046_713_946, error: null }, fmt);
  assert.equal(row.disabled, false);
  assert.equal(row.detail, `${fmt(122_391)} held`);
  assert.ok(row.why?.startsWith('ingest not ready'), String(row.why));
  assert.ok(row.why?.includes(WHY));
  assert.ok(row.title.startsWith('Selects Global Datafeeds.'));
  assert.ok(row.title.includes('does not stop the stored bars being read'));
});

test('a not-ready feed with nothing held stays disabled with the server reason', () => {
  for (const held of [undefined, { cells: 0, bars: 0, error: null }, { cells: 0, bars: 0, error: 'HTTP 500' }]) {
    const row = feedPickerRow(gdfl, held, fmt);
    assert.equal(row.disabled, true);
    assert.equal(row.detail, 'unavailable');
    assert.equal(row.why, WHY);
    assert.ok(row.title.includes(WHY));
  }
});

test('a not-ready feed with no stated reason says so rather than going blank', () => {
  const row = feedPickerRow({ ...gdfl, why: '' }, undefined, fmt);
  assert.equal(row.disabled, true);
  assert.match(row.why ?? '', /stated no reason/);
});

test('a ready feed keeps the four survey states and carries no note', () => {
  const z = { wire: 'zerodha', display: 'Zerodha', ready: true, why: '' };
  const held = feedPickerRow(z, { cells: 37_244, bars: 765_442, error: null }, fmt);
  assert.deepEqual([held.disabled, held.detail, held.why], [false, `${fmt(37_244)} held`, undefined]);
  assert.equal(feedPickerRow(z, { cells: 0, bars: 0, error: null }, fmt).detail, '0 held');
  const bad = feedPickerRow(z, { cells: 0, bars: 0, error: 'boom' }, fmt);
  assert.equal(bad.detail, 'not read');
  assert.ok(bad.title.includes('boom'));
  const none = feedPickerRow(z, undefined, fmt);
  assert.equal(none.detail, 'not surveyed');
  assert.ok(none.title.includes('has not been surveyed'));
  assert.equal(none.disabled, false);
});

import { offeredFeeds, excludedSentence } from '../src/lib/feed-row.js';

test('THE REGRESSION: the picker lists only the offered feeds, and says which are excluded (D-3338)', () => {
  const list = [
    { wire: 'dhan', display: 'Dhan', ready: false, offered: false, excluded_by: 'operator', excluded_on: '2026-10-06' },
    { wire: 'groww', display: 'Groww', ready: false, offered: false, excluded_by: 'operator', excluded_on: '2026-10-06' },
    { wire: 'truedata', display: 'TrueData', ready: false, offered: false, excluded_by: 'operator', excluded_on: '2026-10-06' },
    { wire: 'gdfl', display: 'Global Datafeeds', ready: true, offered: true },
    { wire: 'zerodha', display: 'Zerodha', ready: true, offered: true }
  ];
  assert.deepEqual(offeredFeeds(list).map((f) => f.wire), ['gdfl', 'zerodha']);
  assert.equal(
    excludedSentence(list),
    'Dhan, Groww and TrueData are excluded by the operator since 2026-10-06, so they are not offered here (D-3338).'
  );
  assert.equal(excludedSentence(offeredFeeds(list)), '');
  // An older API with no `offered` field hides nothing.
  assert.deepEqual(offeredFeeds([{ wire: 'x', display: 'X', ready: true }]).map((f) => f.wire), ['x']);
});

test("a feed's title states its archive or window from its descriptor, not a folder probe", () => {
  const g = {
    ...gdfl,
    ready: true,
    why: '',
    archive: { from: '2018-09-03', to: '2026-09-24', source: 'imported by cli gdfl-import (D-2804)' }
  };
  const row = feedPickerRow(g, { cells: 246_147, bars: 3_259_842_252, error: null }, fmt);
  assert.match(row.title, /Its archive runs 2018-09-03 to 2026-09-24 \(imported by cli gdfl-import \(D-2804\)\)\./);
  assert.equal(row.why, undefined);
  const z = feedPickerRow(
    { wire: 'zerodha', display: 'Zerodha', ready: true, why: '', window: { from: '2018-01-01', to: '2026-09-30' } },
    { cells: 250_025, bars: 451_629_928, error: null },
    fmt
  );
  assert.match(z.title, /Your window for it is 2018-01-01 to 2026-09-30; .*labelled outside your window/);
});

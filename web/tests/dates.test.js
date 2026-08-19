// EVERY DATE THIS PRODUCT SHOWS A HUMAN GOES THROUGH THIS MODULE, and until
// now nothing drove it.
//
// `node --test web/tests/` — the runner is node's own, so this costs no
// dependency and no toolchain. It drives `src/lib/dates.js` directly; nothing
// here is a copy of its arithmetic.
//
// The module exists because three pages rendered a stamp with a bare
// `toLocaleTimeString()` and printed the host's zone on an IST-only product.
// The fix routed all three through `stampLabel`. That makes this module's
// zone-correctness load-bearing for the whole front end, and the last test
// below is the one that actually proves it: it re-runs the assertions inside
// child processes with hostile `TZ` values, because a formatter that had
// inherited the host zone would pass every other test on a machine in India.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

import { MON, monthLabel, dayLabel, stampLabel, timeLabel, istMonth } from '../src/lib/dates.js';

const here = dirname(fileURLToPath(import.meta.url));

/* ── the literal month table ──────────────────────────────────────────── */

test('MON is twelve three-letter months, and that is deliberate', () => {
  assert.equal(MON.length, 12);
  for (const m of MON) assert.equal(m.length, 3, `${m} is not three letters`);
  assert.equal(MON[0], 'Jan');
  assert.equal(MON[8], 'Sep');
  assert.equal(MON[11], 'Dec');
  // The table is a literal rather than `Intl`'s month names precisely so a
  // CLDR update cannot silently make September four letters and reflow every
  // grid on the product.
  assert.equal(monthLabel('2024-09'), 'Sep 2024');
});

/* ── monthLabel: an unparseable key is returned AS ITSELF ──────────────── */

test('a month key becomes a label', () => {
  assert.equal(monthLabel('2020-01'), 'Jan 2020');
  assert.equal(monthLabel('2026-12'), 'Dec 2026');
});

test('an unparseable month key comes back verbatim, not as Invalid Date', () => {
  // The store's own value is worth seeing as it is. `undefined 2020` and
  // `Invalid Date` name neither the value nor the fault.
  for (const bad of ['', '2020', '2020-13', '2020-00', 'nonsense', '2020-xx']) {
    assert.equal(monthLabel(bad), bad, `${JSON.stringify(bad)} should pass through`);
  }
  assert.equal(monthLabel(null), null);
  assert.equal(monthLabel(undefined), undefined);
  assert.equal(monthLabel(/** @type {any} */ (5)), 5);
});

/* ── the em dash is the honest empty ──────────────────────────────────── */

test('no instant renders an em dash, never Invalid Date', () => {
  for (const nothing of [null, undefined, NaN, '', 'not a date']) {
    assert.equal(dayLabel(/** @type {any} */ (nothing)), '—');
    assert.equal(stampLabel(/** @type {any} */ (nothing)), '—');
  }
  assert.equal(istMonth(/** @type {any} */ ('not a date')), '');
});

/* ── IST, and the +5:30 that makes it a different day ─────────────────── */

test('the offset is applied: 18:29Z and 18:30Z are different IST days', () => {
  // 18:30 UTC is exactly midnight IST. This pair is the whole zone question in
  // two assertions — a formatter running in UTC prints "01 Sep" for both.
  assert.equal(stampLabel('2024-09-01T18:29:00Z'), '01 Sep 2024, 23:59');
  /* `timeLabel` IS `stampLabel`'S SECOND HALF AND MUST NOT DRIFT FROM IT.
     Same instant, same IST shift, same minute — asserted against the same
     boundary case, which is the one that catches a wrong offset: 18:29Z is
     23:59 IST on the SAME day, and an implementation that dropped the +5:30
     would answer 18:29 here and still look plausible. */
  assert.equal(timeLabel('2024-09-01T18:29:00Z'), '23:59');
  assert.equal(timeLabel(/** @type {any} */ (null)), '—');
  assert.equal(stampLabel('2024-09-01T18:30:00Z'), '02 Sep 2024, 00:00');
});

test('midnight is 00:00 and never 24:00', () => {
  // `hourCycle: 'h23'` is set explicitly because `hour12: false` alone has
  // historically yielded 24 for midnight, and 24:00 is not a time this product
  // renders.
  assert.match(stampLabel('2024-09-01T18:30:00Z'), /, 00:00$/);
});

test('the NSE session renders as the session', () => {
  // 03:45Z is 09:15 IST, the open; 10:00Z is 15:30 IST, the close. An operator
  // reading these should not have to do arithmetic to know the pull covered
  // the day.
  assert.equal(stampLabel('2024-09-02T03:45:00Z'), '02 Sep 2024, 09:15');
  assert.equal(stampLabel('2024-09-02T10:00:00Z'), '02 Sep 2024, 15:30');
});

/* ── the unit is never guessed ────────────────────────────────────────── */

test('a number is milliseconds and is never sniffed as seconds', () => {
  // This repository carries bar times in epoch SECONDS and response times in
  // milliseconds. A formatter that guessed from magnitude would render 1970
  // for half its callers silently; this one renders 1970 loudly, and the call
  // site multiplies where the unit is known.
  const seconds = 1_725_260_700; // a real bar time, in seconds
  assert.match(dayLabel(seconds), /1970$/, 'seconds passed as ms must land in 1970');
  assert.equal(dayLabel(seconds * 1000), dayLabel(new Date(seconds * 1000)));
});

test('a Date, epoch ms and an ISO string all agree', () => {
  const ms = Date.parse('2026-08-18T09:00:00Z');
  assert.equal(dayLabel(ms), dayLabel(new Date(ms)));
  assert.equal(dayLabel(ms), dayLabel('2026-08-18T09:00:00Z'));
  assert.equal(dayLabel(ms), '18 Aug 2026');
});

test('a bare YYYY-MM-DD is the day it names', () => {
  // The store and the date inputs both speak this. `Date.parse` reads it as
  // UTC midnight, which is 05:30 IST on the SAME day — so the label must not
  // slip backwards.
  assert.equal(dayLabel('2024-09-02'), '02 Sep 2024');
  assert.equal(dayLabel('2026-01-01'), '01 Jan 2026');
});

/* ── istMonth ─────────────────────────────────────────────────────────── */

test('istMonth is yyyy-mm and pads', () => {
  assert.equal(istMonth('2024-09-02T12:00:00Z'), '2024-09');
  assert.equal(istMonth('2024-01-15T12:00:00Z'), '2024-01');
  assert.match(istMonth(), /^\d{4}-\d{2}$/, 'no argument means now, in IST');
});

test('istMonth crosses the month at IST midnight, not UTC midnight', () => {
  // 31 Aug 18:29Z is still August in IST; 18:30Z is September.
  assert.equal(istMonth('2024-08-31T18:29:00Z'), '2024-08');
  assert.equal(istMonth('2024-08-31T18:30:00Z'), '2024-09');
});

/* ── THE ONE THAT MATTERS: the zone is named, not inherited ───────────── */

test('the same instant renders identically under any host TZ', () => {
  // A formatter that had inherited the host zone would pass every test above
  // on a machine set to IST. These run the assertions in child processes with
  // TZ set to zones on either side of the date line.
  const probe = [
    "import { stampLabel, istMonth } from '" + join(here, '..', 'src', 'lib', 'dates.js') + "';",
    "console.log([",
    "  stampLabel('2024-09-01T18:30:00Z'),",
    "  stampLabel('2024-09-02T03:45:00Z'),",
    "  istMonth('2024-08-31T18:30:00Z')",
    "].join('|'));"
  ].join('\n');

  /** @param {string} tz */
  const under = (tz) =>
    execFileSync(process.execPath, ['--input-type=module', '-e', probe], {
      env: { ...process.env, TZ: tz },
      encoding: 'utf8'
    }).trim();

  const expected = '02 Sep 2024, 00:00|02 Sep 2024, 09:15|2024-09';
  for (const tz of ['UTC', 'America/New_York', 'Asia/Tokyo', 'Pacific/Kiritimati']) {
    assert.equal(under(tz), expected, `TZ=${tz} changed the answer`);
  }
});

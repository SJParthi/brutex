// R3-02 (DB-045 and the absurd-input probes): the day boxes and the search
// boxes say the right words — a To before From names the window's order, 1969
// and 2099 are refused as typed in the held span's words, 6,000 pasted
// characters are cut and refused in one short sentence, a To box always holds
// a day, the calendar strikes exactly the days its sentence names, and Escape
// closes a calendar opened with the mouse.
//
// The words are `dayFieldWhy` and its neighbours in `notes-controls.js`; the
// REAL `DayField.svelte` is compiled from disk and rendered on the server with
// the props /db hands it, so what a reader sees under the box is what is
// asserted. Every day below is invented for the test.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';
import { dbSource } from './database-page-fixture.js';
import {
  cutText,
  dayFieldWhy,
  dayTakes,
  dayStruckWhy,
  calendarNote,
  DAY_TEXT_MAX,
  SEARCH_TEXT_MAX
} from '../src/lib/db/notes-controls.js';
import { checkText, MAX_TEXT, parseDay } from '../src/lib/db/input-check.js';

/** The held span and reason /db hands both boxes (`dayBounds2`, `dayReason`). */
const HELD = {
  min: '2026-09-01',
  max: '2026-09-30',
  reason: 'This selection holds 01 Sep 2026 to 30 Sep 2026 (the census’s first and last bar); days outside it are struck through because there is no bar there to show. Only bars on or before this day.'
};

/** The real component, compiled for the server, its `$lib` imports resolved from disk. */
async function dayField() {
  const src = readFileSync(new URL('../src/lib/DayField.svelte', import.meta.url), 'utf8');
  const code = compile(src, { generate: 'server', filename: 'DayField.svelte' })
    .js.code.replace(/from (["'])(svelte[^"']*)\1/g, (_, q, spec) => `from ${q}${import.meta.resolve(spec)}${q}`)
    .replace(/from (["'])\$lib\/([^"']+)\1/g, (_, q, rest) => `from ${q}${new URL(`../src/lib/${rest}`, import.meta.url).href}${q}`);
  return (await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`)).default;
}

/** The text of the refusal paragraph a render drew, whitespace folded, or `null`. @param {string} html */
const refusal = (html) => {
  const m = /<p class="dbad[^"]*"[^>]*>([\s\S]*?)<\/p>/.exec(html);
  return m ? m[1].replace(/<!--[\s\S]*?-->/g, '').replace(/<[^>]+>/g, '').replace(/\s+/g, ' ').trim() : null;
};

test('a_backwards_window_names_the_order', async () => {
  // THE PROBE: From 01 Sep 2026, To typed 01 Aug 2026 — but inside the held
  // span a To before From is the window's order, never the held range.
  const order = dayFieldWhy({ typed: '15 Sep 2026', value: '2026-09-30', label: 'To date', ...HELD, after: '2026-09-20' });
  assert.equal(order, 'From 20 Sep 2026 is after To 15 Sep 2026, so the window holds no day — swap them or move one.');
  assert.doesNotMatch(String(order), /This selection holds/);
  const fromAfter = dayFieldWhy({ typed: '25 Sep 2026', value: '2026-09-01', label: 'From date', ...HELD, before: '2026-09-10' });
  assert.equal(fromAfter, 'From 25 Sep 2026 is after To 10 Sep 2026, so the window holds no day — swap them or move one.');
  // 01 Aug 2026 is before the held span too: refused as typed, in the held span's words.
  assert.equal(
    dayFieldWhy({ typed: '01 Aug 2026', value: '2026-09-30', label: 'To date', ...HELD, after: '2026-09-01' }),
    'To date 01 Aug 2026: Outside 01 Sep 2026 to 30 Sep 2026 — This selection holds 01 Sep 2026 to 30 Sep 2026 (the census’s first and last bar); days outside it are struck through because there is no bar there to show.'
  );
  // 1969 and 2099, typed into To while From is 15 Mar 2019 — named as typed, never as the From day.
  const contract = { min: '2019-03-01', max: '2019-03-28', reason: 'This selection holds 01 Mar 2019 to 28 Mar 2019.' };
  assert.equal(dayFieldWhy({ typed: '01 Jan 1969', value: '2019-03-15', label: 'To date', ...contract, after: '2019-03-15' }), 'To date 01 Jan 1969: Outside 01 Mar 2019 to 28 Mar 2019 — This selection holds 01 Mar 2019 to 28 Mar 2019.');
  assert.equal(dayFieldWhy({ typed: '31 Dec 2099', value: '2019-03-15', label: 'To date', ...contract, after: '2019-03-15' }), 'To date 31 Dec 2099: Outside 01 Mar 2019 to 28 Mar 2019 — This selection holds 01 Mar 2019 to 28 Mar 2019.');
  assert.equal(dayFieldWhy({ typed: '14 Mar 2019', value: '2019-03-15', label: 'To date', ...contract, after: '2019-03-15' }), 'From 15 Mar 2019 is after To 14 Mar 2019, so the window holds no day — swap them or move one.');
  assert.equal(dayFieldWhy({ typed: '21 Mar 2019', value: '2019-03-15', label: 'To date', ...contract, after: '2019-03-15' }), null, 'an ordinary day is taken');
  assert.equal(dayFieldWhy({ typed: '15 mar 19', value: '2019-03-15', label: 'To date', ...contract, after: '2019-03-15' }), 'To date "15 mar 19" is not a date this field can read — it takes dd Mon yyyy or YYYY-MM-DD, like 01 Mar 2019.');
  assert.equal(dayFieldWhy({ typed: '', value: '2019-03-28', label: 'To date', ...contract }), null, 'an emptied box takes a day and says nothing');
  assert.equal(dayFieldWhy({ typed: null, value: '2019-03-20', label: 'To date', ...contract }), null);
  // One bound only, and none.
  assert.equal(dayFieldWhy({ typed: '2010-01-01', value: '', min: '2018-09-03' }), 'Date 01 Jan 2010: Earlier than 03 Sep 2018.');
  assert.equal(dayFieldWhy({ typed: '2030-01-01', value: '', max: '2026-09-30' }), 'Date 01 Jan 2030: Later than 30 Sep 2026.');
  assert.equal(dayFieldWhy({ typed: 'x', value: '' }), 'Date "x" is not a date this field can read — it takes dd Mon yyyy or YYYY-MM-DD, like 15 Mar 2019.');
  // A held value a link put outside the bounds is said too, with nothing typed.
  assert.match(String(dayFieldWhy({ typed: null, value: '2026-08-01', label: 'To date', ...HELD })), /^To date 01 Aug 2026: Outside/);

  // The grid's predicate and its titles: the same rule, its own words per kind.
  assert.equal(dayTakes('2026-09-05', { ...HELD, after: '2026-09-10' }), false);
  assert.equal(dayTakes('2026-09-15', { ...HELD, after: '2026-09-10', before: '2026-09-20' }), true);
  assert.equal(dayTakes('2026-09-25', { before: '2026-09-20' }), false);
  assert.equal(dayTakes('2026-10-01', HELD), false);
  assert.equal(dayTakes('2026-08-31', HELD), false);
  assert.equal(dayTakes('2026-10-01', {}), true);
  assert.equal(dayStruckWhy('2026-09-05', { ...HELD, after: '2026-09-10' }), 'Before the From date, 10 Sep 2026: a window ends on or after the day it starts.');
  assert.equal(dayStruckWhy('2026-09-25', { ...HELD, before: '2026-09-20', beforeLabel: 'To date' }), 'After the To date, 20 Sep 2026: a window starts on or before the day it ends.');
  assert.equal(dayStruckWhy('2026-09-25', { before: '2026-09-20' }), 'After the To date, 20 Sep 2026: a window starts on or before the day it ends.');
  assert.equal(dayStruckWhy('2026-09-05', { after: '2026-09-10', afterLabel: 'From' }), 'Before the From, 10 Sep 2026: a window ends on or after the day it starts.');
  assert.equal(dayStruckWhy('2026-10-05', HELD), HELD.reason);
  assert.equal(dayStruckWhy('2026-10-05', { max: '2026-09-30' }), 'Outside what this field may take.');
  assert.equal(dayStruckWhy('2026-08-05', { min: '2026-09-01' }), 'Outside what this field may take.');
  assert.equal(dayStruckWhy('2026-09-15', HELD), null);

  // THE CALENDAR STRIKES EXACTLY THE DAYS ITS SENTENCE NAMES (DB-045): with
  // From and To on 30 May 2019 the From grid strikes 31 May for the To day, and
  // the sentence now says so.
  const may = { min: '2019-05-01', max: '2019-05-31', reason: 'This selection holds 01 May 2019 to 31 May 2019.' };
  assert.equal(calendarNote({ ...may, before: '2019-05-30', beforeLabel: 'To date' }), '01 May 2019 – 31 May 2019 can be picked. This selection holds 01 May 2019 to 31 May 2019. Days after the To date, 30 May 2019, are struck too: a window starts on or before the day it ends.');
  assert.equal(calendarNote({ ...may, after: '2019-05-30' }), '01 May 2019 – 31 May 2019 can be picked. This selection holds 01 May 2019 to 31 May 2019. Days before the From date, 30 May 2019, are struck too: a window ends on or after the day it starts.');
  assert.equal(calendarNote({ ...may, before: '2019-05-31' }), '01 May 2019 – 31 May 2019 can be picked. This selection holds 01 May 2019 to 31 May 2019.', 'an order bound at the held end strikes nothing more and is not said');
  assert.equal(calendarNote({ ...may, after: '2019-05-01' }), '01 May 2019 – 31 May 2019 can be picked. This selection holds 01 May 2019 to 31 May 2019.');
  assert.equal(calendarNote({ min: '2019-05-01' }), '01 May 2019 – any later day can be picked.');
  assert.equal(calendarNote({ max: '2019-05-31' }), 'any earlier day – 31 May 2019 can be picked.');
  assert.equal(calendarNote({ after: '2019-05-03', before: '2019-05-09' }), 'Days before the From date, 03 May 2019, are struck too: a window ends on or after the day it starts. Days after the To date, 09 May 2019, are struck too: a window starts on or before the day it ends.');
  assert.equal(calendarNote({}), '');

  // THE REAL BOX, RENDERED: the To box /db draws holds 01 Aug 2026 with From on
  // 01 Sep 2026 — a value carried in, so nothing is typed — and says the order
  // when it is inside the held span, the held span when it is not.
  const DayField = await dayField();
  const inside = render(DayField, { props: { label: 'To date', value: '2026-09-05', after: '2026-09-10', min: HELD.min, max: HELD.max, boundsReason: HELD.reason } });
  assert.equal(refusal(inside.body), 'From 10 Sep 2026 is after To 05 Sep 2026, so the window holds no day — swap them or move one.');
  assert.match(inside.body, /aria-invalid="true"/);
  const outside = render(DayField, { props: { label: 'To date', value: '2026-08-01', after: '2026-09-01', min: HELD.min, max: HELD.max, boundsReason: HELD.reason } });
  assert.match(String(refusal(outside.body)), /^To date 01 Aug 2026: Outside 01 Sep 2026 to 30 Sep 2026 — This selection holds/);
  const fine = render(DayField, { props: { label: 'To date', value: '2026-09-20', after: '2026-09-10', min: HELD.min, max: HELD.max, boundsReason: HELD.reason } });
  assert.equal(refusal(fine.body), null);
  assert.match(fine.body, /value="20 Sep 2026"/);

  // WIRED on /db: the held span is the bound and the other box is the order.
  const controls = dbSource('../src/routes/db/DbControls.svelte');
  assert.doesNotMatch(controls, /max=\{db\.toDay \|\| db\.dayBounds2\.hi\}/, 'the To day was folded into the From box’s max');
  assert.doesNotMatch(controls, /min=\{db\.fromDay \|\| db\.dayBounds2\.lo\}/, 'the From day was folded into the To box’s min');
  assert.match(controls, /label="From date"\s*value=\{db\.fromDay\}\s*min=\{db\.dayBounds2\.lo\}\s*max=\{db\.dayBounds2\.hi\}\s*before=\{db\.toDay\}\s*beforeLabel="To date"\s*empty=\{db\.dayBounds2\.lo\}/);
  assert.match(controls, /label="To date"\s*value=\{db\.toDay\}\s*min=\{db\.dayBounds2\.lo\}\s*max=\{db\.dayBounds2\.hi\}\s*after=\{db\.fromDay\}\s*afterLabel="From date"\s*empty=\{db\.dayBounds2\.hi\}/);
});

test('six_thousand_characters_are_refused_in_one_sentence', () => {
  const paste = 'A'.repeat(6000);
  // The box keeps the first 32 and says the length once — the words a link carrying the same value gets.
  const cut = cutText(paste, DAY_TEXT_MAX, 'From date');
  assert.equal(cut.text, 'A'.repeat(32));
  assert.equal(cut.why, (/** @type {any} */ (checkText(paste, { label: 'From date', max: 32 }))).why);
  assert.equal(cut.why, 'From date is 6,000 characters long, and /db reads at most 32');
  assert.deepEqual(cutText('15 Mar 2019', DAY_TEXT_MAX, 'From date'), { text: '15 Mar 2019', why: '' });
  assert.deepEqual(cutText(undefined, DAY_TEXT_MAX, 'From date'), { text: '', why: '' });
  const said = String(dayFieldWhy({ typed: cut.text, value: '2020-03-02', label: 'From date', min: '2020-03-02', max: '2020-03-31', cut: cut.why }));
  assert.equal(said, 'From date is 6,000 characters long, and /db reads at most 32 — cut, not applied: the field still holds 02 Mar 2020.');
  // ONE sentence: one full stop, at the end, and no run of the pasted text.
  assert.equal(said.split(/\.(?:\s|$)/).filter(Boolean).length, 1);
  assert.doesNotMatch(said, /AAAA/);
  assert.equal(dayFieldWhy({ typed: cut.text, value: '', cut: cut.why }), 'From date is 6,000 characters long, and /db reads at most 32 — cut, not applied: the field still holds no bound.');
  // A search box: the same cut at its own bound.
  const search = cutText(paste, SEARCH_TEXT_MAX, 'This search');
  assert.equal(SEARCH_TEXT_MAX, MAX_TEXT);
  assert.equal(search.text.length, 128);
  assert.equal(search.why, 'This search is 6,000 characters long, and /db reads at most 128');
  // The parser the box reads with is the link's, and refuses the long text outright.
  assert.equal(parseDay(paste), '');

  // WIRED: the day box cuts as text arrives and draws the one sentence with no "Not applied" tail of its own.
  const day = dbSource('../src/lib/DayField.svelte');
  assert.match(day, /const cut = cutText\(next, DAY_TEXT_MAX, label\);\s*typing = cut\.text;\s*cutWhy = cut\.why;\s*if \(cut\.why\) return;/);
  assert.match(day, /\{#if typing !== null && !cutWhy\}\s*Not applied: the field still holds <b>\{render\(value\) \|\| 'no bound'\}<\/b>\. Escape restores it\./);
  assert.doesNotMatch(day, /<b>\{typing\}<\/b> is not a date/, 'the typed text was echoed whole');
  assert.match(day, /const parse = \(\/\*\* @type \{string\} \*\/ text\) => parseDay\(text\);/, 'one parser with the link reader');
  // The search box cuts too, and quotes at most 40 characters when nothing matches.
  const picker = dbSource('../src/lib/Picker.svelte');
  assert.match(picker, /const cut = cutText\(raw, SEARCH_TEXT_MAX, 'This search'\);/);
  assert.match(picker, /oninput=\{\(e\) => \{\s*search\(e\.currentTarget\.value\);\s*if \(e\.currentTarget\.value !== q\) e\.currentTarget\.value = q;/);
  assert.match(picker, /Nothing matches \$\{quote\(q\.trim\(\)\)\}/);
  assert.doesNotMatch(picker, /bind:value=\{q\}/);
  assert.match(picker, /\{#if qWhy\}\s*<p class="phint pcut" role="status">\{qWhy\}<\/p>/);
});

test('a To box always takes a day, and Escape closes a calendar opened with the mouse', async () => {
  const day = dbSource('../src/lib/DayField.svelte');
  // Emptied: the box takes `empty` (the held span's end on /db) instead of a blank "no bound".
  assert.match(day, /if \(cut\.text\.trim\(\) === ''\) onchange\?\.\(empty\);/);
  // ... and an emptied box is not refilled under the caret while the reader types the next day.
  assert.match(day, /if \(typing !== null && typing\.trim\(\) !== '' && parse\(typing\) !== v\) typing = null;/);
  // The calendar's clear button takes the same day, and says which on its face.
  assert.match(day, /onclick=\{\(\) => pick\(empty\)\}/);
  assert.match(day, /\{empty \? `Back to \$\{render\(empty\)\}` : 'Clear'\}/);
  // Escape: the field's root hears it from the opener the mouse left focused, and the panel takes focus as it opens.
  assert.match(day, /<div class="dcell" data-dayfield=\{id\} onkeydown=\{onFieldKey\} role="presentation">/);
  assert.match(day, /function onFieldKey\(e\) \{\s*if \(e\.key !== 'Escape' \|\| !open\) return;\s*e\.stopPropagation\(\);\s*openId = 0;\s*opener\?\.focus\(\);/);
  assert.match(day, /if \(open && panel\) panel\.focus\(\{ preventScroll: true \}\);/);
  assert.match(day, /bind:this=\{opener\}/);

  // THE REAL BOX holding the day an emptied To takes (the held span's end):
  // it renders that day and no refusal.
  const DayField = await dayField();
  const held = render(DayField, { props: { label: 'To date', value: '2019-03-28', empty: '2019-03-28', min: '2019-03-01', max: '2019-03-28' } });
  assert.match(held.body, /value="28 Mar 2019"/);
  assert.equal(refusal(held.body), null);
});

test('the edges of the day words: the bound itself, blank text, a day on either end', () => {
  // Exactly the bound is kept whole; one past it is cut.
  assert.deepEqual(cutText('x'.repeat(DAY_TEXT_MAX), DAY_TEXT_MAX, 'From date'), { text: 'x'.repeat(DAY_TEXT_MAX), why: '' });
  assert.equal(cutText('x'.repeat(DAY_TEXT_MAX + 1), DAY_TEXT_MAX, 'From date').why, 'From date is 33 characters long, and /db reads at most 32');
  // Spaces only is an emptied box, not a malformed day.
  assert.equal(dayFieldWhy({ typed: '   ', value: '2026-09-30', label: 'To date', ...HELD }), null);
  // Each bound takes the day it names.
  const o = { ...HELD, after: '2026-09-10', before: '2026-09-20' };
  for (const iso of ['2026-09-10', '2026-09-20']) {
    assert.equal(dayTakes(iso, o), true, iso);
    assert.equal(dayStruckWhy(iso, o), null, iso);
  }
  for (const iso of ['2026-09-01', '2026-09-30']) {
    assert.equal(dayTakes(iso, HELD), true, iso);
    assert.equal(dayStruckWhy(iso, HELD), null, iso);
  }
  assert.equal(dayStruckWhy('2026-09-15', o), null, 'between both bounds');
  assert.equal(dayFieldWhy({ typed: '10 Sep 2026', value: '', label: 'To date', ...HELD, after: '2026-09-10' }), null, 'To on the From day');
  assert.equal(dayFieldWhy({ typed: '20 Sep 2026', value: '', label: 'From date', ...HELD, before: '2026-09-20' }), null, 'From on the To day');
  assert.equal(dayFieldWhy({ typed: '15 Sep 2026', value: '', label: 'From date', ...HELD, after: '2026-09-10', before: '2026-09-20' }), null);
});

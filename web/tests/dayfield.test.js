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
  dayCellTitle,
  CLOSED_DAY_TITLE,
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
  // A long list opens without laying out its hidden rows (the measured opening cost, docs/06-limits.md L-R302-1).
  assert.match(picker, /content-visibility: auto;/);
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
  // Escape inside the open panel (where focus is once it opens) gives focus back to the opener too:
  // the panel stops the key, so the root's handler never runs, and focus fell to the body (repair round 1).
  assert.match(day, /function onKey\(e\) \{\s*if \(e\.key !== 'Escape'\) return;\s*e\.stopPropagation\(\);\s*openId = 0;\s*opener\?\.focus\(\);\s*\}/);
  assert.match(day, /onkeydown=\{onKey\}/);

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
  // The cut never leaves the first half of a surrogate pair in the box (repair round 2).
  const astral = 'x'.repeat(DAY_TEXT_MAX - 1) + '\u{1F600}' + 'y';
  const cutAstral = cutText(astral, DAY_TEXT_MAX, 'From date');
  assert.equal(cutAstral.text, 'x'.repeat(DAY_TEXT_MAX - 1), 'the pair that straddles the cut goes whole');
  assert.equal(cutAstral.text.isWellFormed(), true);
  assert.equal(cutText('x'.repeat(DAY_TEXT_MAX - 2) + '\u{1F600}' + 'y', DAY_TEXT_MAX, 'From date').text, 'x'.repeat(DAY_TEXT_MAX - 2) + '\u{1F600}', 'a pair inside the cut is kept');
  assert.equal(cutText('x'.repeat(DAY_TEXT_MAX - 1) + '\uDBFF' + 'y'.repeat(4), DAY_TEXT_MAX, 'From date').text.length, DAY_TEXT_MAX - 1, 'the last high half (0xDBFF) is not kept');
  assert.equal(cutText('x'.repeat(DAY_TEXT_MAX - 1) + '\uDC00' + 'y'.repeat(4), DAY_TEXT_MAX, 'From date').text.length, DAY_TEXT_MAX, 'a low half (0xDC00) is not the start of a pair');
  assert.equal(cutText('x'.repeat(DAY_TEXT_MAX - 1) + '\uD7FF' + 'y'.repeat(4), DAY_TEXT_MAX, 'From date').text.length, DAY_TEXT_MAX, '0xD7FF is a whole character');
  assert.equal(cutText('x'.repeat(DAY_TEXT_MAX - 1) + '\uD800' + 'y'.repeat(4), DAY_TEXT_MAX, 'From date').text.length, DAY_TEXT_MAX - 1, 'the first high half (0xD800) is not kept');
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

/**
 * A REAL COMPONENT, COMPILED FOR THE SERVER, IN THE STATE A READER LEAVES IT
 * (repair round 3). A server render draws a field as it first mounts — its
 * calendar shut, nothing typed — so the words a pick, a paste or an open grid
 * owe were pinned only as source text, and four mutants that changed them
 * passed every test. Here the module's `openId` names the first field a fresh
 * copy issues (what a click on its button does), and `drive` calls the
 * component's OWN handlers (`type`, `pick`, `search`) before it draws, so the
 * markup asserted is the markup that state renders. Each patch must match the
 * compiled code exactly once, so a change in the compiler's output fails here
 * by name rather than testing something else.
 *
 * @param {string} file `DayField.svelte` or `Picker.svelte`
 * @param {{open?: boolean, drive?: string}} [how]
 */
async function inState(file, { open = false, drive = '' } = {}) {
  const src = readFileSync(new URL(`../src/lib/${file}`, import.meta.url), 'utf8');
  let code = compile(src, { generate: 'server', filename: file })
    .js.code.replace(/from (["'])(svelte[^"']*)\1/g, (_, q, spec) => `from ${q}${import.meta.resolve(spec)}${q}`)
    .replace(/from (["'])\$lib\/([^"']+)\1/g, (_, q, rest) => `from ${q}${new URL(`../src/lib/${rest}`, import.meta.url).href}${q}`);
  /** @param {string} from @param {string} to */
  const patch = (from, to) => {
    assert.equal(code.split(from).length, 2, `${file}: the compiled code holds ${JSON.stringify(from)} once`);
    code = code.replace(from, () => to);
  };
  if (open) patch('let openId = 0;', 'let openId = 1;');
  if (drive) {
    const at = code.indexOf('$$renderer.push(');
    assert.match(code.slice(at, at + 40), /^\$\$renderer\.push\(`<div class="(dcell|picker) /, `${file}: the first output is the field's root, after every statement of its script`);
    code = `${code.slice(0, at)}${drive}\n${code.slice(at)}`;
  }
  return (await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}#${Math.random()}`)).default;
}

/** An attribute's text as written, its entities read back. @param {string} s */
const unescaped = (s) => s.replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');
/** The input's shown text. @param {string} html */
const boxText = (html) => unescaped(/<input class="din[^>]*\svalue="([^"]*)"/.exec(html)?.[1] ?? '');

test('a_cut_paste_is_said_only_while_its_text_is_in_the_box', async () => {
  // C02: the cut sentence is said while the box holds the cut text, and not once a picked day replaced it.
  const paste = JSON.stringify('A'.repeat(6000));
  const props = { label: 'From date', value: '2020-03-02', min: '2020-03-02', max: '2020-03-31' };
  const cut = render(await inState('DayField.svelte', { drive: `type(${paste});` }), { props });
  assert.equal(refusal(cut.body), 'From date is 6,000 characters long, and /db reads at most 32 — cut, not applied: the field still holds 02 Mar 2020.');
  assert.equal(boxText(cut.body), 'A'.repeat(DAY_TEXT_MAX), 'the box keeps the first 32');
  assert.match(cut.body, /aria-invalid="true"/);
  // The reader then picks 15 Mar 2020 from the grid: the caller's value is that day, and the box says nothing.
  const picked = render(await inState('DayField.svelte', { drive: `type(${paste}); pick('2020-03-15');` }), { props: { ...props, value: '2020-03-15' } });
  assert.equal(refusal(picked.body), null, 'a cut that is no longer in the box is not said under a day that was applied');
  assert.equal(boxText(picked.body), '15 Mar 2020');
  assert.match(picked.body, /aria-invalid="false"/);
  // Typed text that is not cut still says its own refusal, and Not applied, after the same pick path.
  const typed = render(await inState('DayField.svelte', { drive: `type('31 Feb 2020');` }), { props });
  assert.match(String(refusal(typed.body)), /Not applied: the field still holds 02 Mar 2020\. Escape restores it\.$/);
});

test('back_to_the_held_end_is_disabled_on_that_day_and_nowhere_else', async () => {
  // C05: the calendar's clear button does nothing when the box already holds the day it takes.
  /** One fresh copy per render: the open field is the first one a copy issues. @param {Record<string, string>} props */
  const clear = async (props) => {
    const html = render(await inState('DayField.svelte', { open: true }), { props: { label: 'To date', min: HELD.min, max: HELD.max, ...props } }).body;
    assert.match(html, /role="dialog"/, 'the calendar is open');
    const m = /<button type="button" class="clear[^"]*"([^>]*)>([^<]*)<\/button>/.exec(html);
    assert.ok(m, 'the clear button is drawn');
    return { disabled: /\sdisabled(\s|=|$)/.test(m[1]), face: m[2] };
  };
  assert.deepEqual(await clear({ value: '2026-09-30', empty: '2026-09-30' }), { disabled: true, face: 'Back to 30 Sep 2026' }, 'already on the held end: nothing to go back to');
  assert.deepEqual(await clear({ value: '2026-09-25', empty: '2026-09-30' }), { disabled: false, face: 'Back to 30 Sep 2026' });
  assert.deepEqual(await clear({ value: '', empty: '' }), { disabled: true, face: 'Clear' }, 'nothing held: nothing to clear');
  assert.deepEqual(await clear({ value: '', empty: '2026-09-30' }), { disabled: true, face: 'Back to 30 Sep 2026' }, 'nothing held: nothing to go back from');
  assert.deepEqual(await clear({ value: '2026-09-25', empty: '' }), { disabled: false, face: 'Clear' });
});

test('a_day_struck_by_the_other_box_says_the_order_on_hover', async () => {
  // C07 (DB-045 goal 3): a struck cell's title is its own kind's words — the held span's reason outside the
  // span, the window's order for a day before the From day — never the held span's reason for both.
  const DayField = await inState('DayField.svelte', { open: true });
  const html = render(DayField, { props: { label: 'To date', value: '2026-09-30', min: HELD.min, max: HELD.max, after: '2026-09-20', afterLabel: 'From date', boundsReason: HELD.reason } }).body;
  assert.match(html, /role="dialog"/, 'the calendar is open');
  const cells = [...html.matchAll(/<button type="button" class="day[^"]*"([^>]*)>(\d+)<\/button>/g)].map((m) => ({ day: +m[2], title: unescaped(/\stitle="([^"]*)"/.exec(m[1])?.[1] ?? ''), off: /\sdisabled(\s|=|$)/.test(m[1]) }));
  assert.equal(cells.length, 42, 'six weeks');
  // Sep 2026 starts on a Tuesday: the Monday-first grid opens on 31 Aug.
  assert.deepEqual(cells[0], { day: 31, title: HELD.reason, off: true }, 'outside the held span: the span’s reason');
  const order = 'Before the From date, 20 Sep 2026: a window ends on or after the day it starts.';
  assert.equal(order, dayStruckWhy('2026-09-10', { ...HELD, after: '2026-09-20' }));
  for (const d of [1, 10, 19]) assert.deepEqual(cells[d], { day: d, title: order, off: true }, `Sep ${d}: the order, not the held span`);
  assert.deepEqual(cells[20], { day: 20, title: '', off: false }, 'the From day itself is taken');
  assert.deepEqual(cells[31], { day: 1, title: HELD.reason, off: true }, '01 Oct: outside the held span');
  assert.equal(cells.filter((c) => c.title === order).length, 19, 'exactly the days the order strikes');
});

test('a_cut_search_is_said_under_the_search_box', async () => {
  // C09: 6,000 letters pasted into a list's search box are cut to 128 and the cut is said under the box,
  // never silently.
  const Picker = await inState('Picker.svelte', { open: true, drive: `search(${JSON.stringify('A'.repeat(6000))});` });
  const rows = [{ key: 'one', name: 'Invented one' }, { key: 'two', name: 'Invented two' }];
  const html = render(Picker, { props: { rows, label: 'instruments', filter: true } }).body;
  const hint = /<p class="phint([^"]*)"( role="status")?>([^<]*)<\/p>/.exec(html);
  assert.ok(hint, 'the hint under the search box is drawn');
  assert.equal(hint[3], `This search is 6,000 characters long, and /db reads at most ${SEARCH_TEXT_MAX} — cut to its first ${SEARCH_TEXT_MAX}.`);
  assert.match(hint[1], /\bpcut\b/);
  assert.equal(hint[2], ' role="status"');
  assert.equal(/<input class="pq[^>]*\svalue="([^"]*)"/.exec(html)?.[1], 'A'.repeat(SEARCH_TEXT_MAX), 'the box keeps the first 128');
  // A search inside the bound says the ordinary hint.
  const plain = render(await inState('Picker.svelte', { open: true, drive: `search('Invented');` }), { props: { rows, label: 'instruments', filter: true } }).body;
  assert.equal(/<p class="phint([^"]*)"( role="status")?>([^<]*)<\/p>/.exec(plain)?.[3], 'Matches name or detail');
});

test('an_empty_box_nothing_was_typed_into_says_nothing', async () => {
  // REPAIR ROUND 4. A box that holds no day and was not typed into — a selection with no held span leaves
  // both boxes so, and before this round a link naming one side left the other so — fell through to the
  // shape check: `To date "" is not a date this field can read …` under it, `aria-invalid`, over a box the
  // reader never touched. It is no bound, and says nothing; the base said nothing there too.
  for (const label of ['From date', 'To date']) {
    for (const bounds of [{}, HELD, { ...HELD, after: '2026-09-10' }, { ...HELD, before: '2026-09-20' }]) {
      assert.equal(dayFieldWhy({ typed: null, value: '', label, ...bounds }), null, `${label} holding nothing, ${JSON.stringify(bounds)}`);
    }
    // Emptied by the reader is the same; text typed into an empty box that is not a day still refuses.
    assert.equal(dayFieldWhy({ typed: '', value: '', label, ...HELD }), null);
    assert.equal(dayFieldWhy({ typed: 'garbage', value: '', label, ...HELD }), `${label} "garbage" is not a date this field can read — it takes dd Mon yyyy or YYYY-MM-DD, like 01 Sep 2026.`);
  }

  // THE REAL BOX, both labels, with the props /db hands each (DbControls: the held span as min/max, the
  // other box's day as the order bound, the span's edge as `empty`) and with none at all.
  const DayField = await dayField();
  const boxes = [
    { label: 'From date', value: '', min: HELD.min, max: HELD.max, before: '2026-09-20', beforeLabel: 'To date', empty: HELD.min, boundsReason: HELD.reason },
    { label: 'To date', value: '', min: HELD.min, max: HELD.max, after: '2026-09-10', afterLabel: 'From date', empty: HELD.max, boundsReason: HELD.reason },
    { label: 'From date', value: '' },
    { label: 'To date', value: '' }
  ];
  for (const props of boxes) {
    const html = render(DayField, { props }).body;
    assert.equal(refusal(html), null, `${props.label}: an empty box nothing was typed into says nothing`);
    assert.match(html, /aria-invalid="false"/, `${props.label}: and is not marked invalid`);
    assert.equal(boxText(html), '', `${props.label}: and shows no day`);
  }
  // The same box with text typed into it still refuses, and is marked: only the untouched box is silent.
  for (const props of boxes) {
    const html = render(await inState('DayField.svelte', { drive: `type('garbage');` }), { props }).body;
    assert.match(String(refusal(html)), new RegExp(`^${props.label} "garbage" is not a date this field can read`), props.label);
    assert.match(html, /aria-invalid="true"/);
  }
});

test('an_emptied_box_never_speaks_of_the_day_it_no_longer_shows', async () => {
  // REPAIR ROUND 5 (mutant V5-07, `o.typed ?? o.value` -> `o.typed || o.value`). A box the reader emptied
  // shows nothing; the day it held is still its value until the page answers the change. That day is not
  // said under the empty box: a link that put To on 01 Aug 2026, outside the held span, is refused while
  // it is shown, and the moment the reader empties the box the refusal of a day no longer in it goes.
  const out = { label: 'To date', value: '2026-08-01', ...HELD };
  assert.match(String(dayFieldWhy({ typed: null, ...out })), /^To date 01 Aug 2026: Outside/, 'shown, it is refused');
  for (const typed of ['', '   ']) assert.equal(dayFieldWhy({ typed, ...out }), null, `emptied (${JSON.stringify(typed)}), it says nothing`);
  // The real box in that state: emptied, before the page has answered with the held span's end.
  const props = { label: 'To date', value: '2026-08-01', min: HELD.min, max: HELD.max, empty: HELD.max, boundsReason: HELD.reason };
  const shown = render(await dayField(), { props }).body;
  assert.match(String(refusal(shown)), /^To date 01 Aug 2026: Outside/);
  assert.match(shown, /aria-invalid="true"/);
  const emptied = render(await inState('DayField.svelte', { drive: `type('');` }), { props }).body;
  assert.equal(boxText(emptied), '', 'the box shows nothing');
  assert.equal(refusal(emptied), null, 'and says nothing of the day it held');
  assert.match(emptied, /aria-invalid="false"/);
});

test('a_struck_day_that_is_flagged_says_both_on_hover', async () => {
  // REPAIR ROUND 5. A day both struck and flagged said only why it was struck, hiding its flag — common now
  // that flags are marked in every month the grid shows (DB-025) — and a closed day that was flagged said
  // only its flag. Each cell now says why it cannot be picked (or that it is a closed day it can), then its
  // flag. The flag words are invented.
  const FLAG = 'refused — an invented reason';
  const flaggedDays = new Set(['2026-08-31', '2026-09-10', '2026-09-25', '2026-09-26']);
  const closedDays = new Set(['2026-09-26', '2026-09-27']);
  const DayField = await inState('DayField.svelte', { open: true });
  const html = render(DayField, {
    props: {
      label: 'To date', value: '2026-09-30', min: HELD.min, max: HELD.max, after: '2026-09-20', afterLabel: 'From date', boundsReason: HELD.reason,
      flagged: (/** @type {string} */ iso) => (flaggedDays.has(iso) ? FLAG : null),
      closed: (/** @type {string} */ iso) => closedDays.has(iso)
    }
  }).body;
  assert.match(html, /role="dialog"/, 'the calendar is open');
  const cells = [...html.matchAll(/<button type="button" class="day[^"]*"([^>]*)>(\d+)<\/button>/g)].map((m) => ({ day: +m[2], title: unescaped(/\stitle="([^"]*)"/.exec(m[1])?.[1] ?? ''), off: /\sdisabled(\s|=|$)/.test(m[1]) }));
  assert.equal(cells.length, 42, 'six weeks');
  const order = 'Before the From date, 20 Sep 2026: a window ends on or after the day it starts.';
  // cells[0] is 31 Aug (the Monday-first grid opens there); cells[d] is d Sep.
  assert.deepEqual(cells[0], { day: 31, title: `${HELD.reason} Flagged: ${FLAG}`, off: true }, 'struck by the held span, and flagged');
  assert.deepEqual(cells[10], { day: 10, title: `${order} Flagged: ${FLAG}`, off: true }, 'struck by the order, and flagged');
  assert.deepEqual(cells[11], { day: 11, title: order, off: true }, 'struck, not flagged');
  assert.deepEqual(cells[25], { day: 25, title: `Flagged: ${FLAG}`, off: false }, 'taken and flagged');
  assert.deepEqual(cells[26], { day: 26, title: `${CLOSED_DAY_TITLE} Flagged: ${FLAG}`, off: false }, 'a closed day, flagged');
  assert.deepEqual(cells[27], { day: 27, title: CLOSED_DAY_TITLE, off: false }, 'a closed day');
  assert.deepEqual(cells[21], { day: 21, title: '', off: false }, 'an ordinary day says nothing');
  assert.equal(cells.filter((c) => c.title.includes('Flagged: ')).length, flaggedDays.size, 'every flagged day says so, struck or not');
  // The edges of the title itself: a struck day is never said to be a closed day that can be picked.
  assert.equal(dayCellTitle({ ok: false, struck: 'Struck.', shut: true, flag: null }), 'Struck.');
  assert.equal(dayCellTitle({ ok: false, struck: null, shut: false, flag: null }), undefined);
  assert.equal(dayCellTitle({ ok: true, struck: null, shut: false, flag: null }), undefined);
  assert.equal(dayCellTitle({ ok: true, struck: null, shut: false, flag: '' }), 'Flagged: ');
});

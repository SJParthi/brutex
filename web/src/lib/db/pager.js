/**
 * /db — THE BAR GRID'S PAGER, AS ARITHMETIC (R3-03)
 *
 * Which pages the server can serve, what the page box accepts, which page a
 * change of order reads, whether a question has already been asked, and how
 * two pages of one read are checked against each other. Every rule here was
 * a line inside `DbBarsGrid.svelte`'s 3,667-line script, or was missing from
 * it; it lives in its own file so later packages do not queue on the grid.
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: constants and pure functions only,
 * so `node --test` imports it with no compiler, and imports are relative
 * because `$lib` is a Vite alias node cannot see.
 *
 * Owner: R3-03.
 */
import { checkWhole } from './input-check.js';
import { fmt } from './notes-labels.js';

/* ---------------------------------------------------------------------
   THE SERVER'S DEPTHS, MIRRORED. `/bars/window.json` orders a value column
   from sort indexes and READS the rows no index orders; past what it holds
   in one request it serves a page only within a depth of the front (or of
   the end, when nothing was indexed), and refuses `sort_too_deep` deeper. A
   time-of-day order is always read, so it is held to the front depth alone.

   The pager offered every page all the same: typed 4,890,334, Next and Last
   all landed past row 100,000 of a sorted 153,107-row contract on a refusal
   where a page should have been (the 11 Oct re-check). These two numbers
   are the server's own — `bars::MAX_SORT_DEPTH` and
   `bars_sort::MAX_READ_HELD` — and `web/tests/db-pager.test.js` reads them
   out of the Rust source, so the two cannot drift apart unseen.
   --------------------------------------------------------------------- */
/** The deepest `offset + limit` a read order serves from one end. */
export const MAX_SORT_DEPTH = 100_000;
/** The most read rows one request orders whole, so every page of them is served. */
export const MAX_READ_HELD = 1_000_000;

/**
 * @typedef {{count: number, front: number, tail: number | null, why: string | null}} Served
 * `count` every page the total makes; `front` the last page served counting
 * from page 1; `tail` the first page served counting back from the end, or
 * `null` when no page past `front` is served; `why` the sentence a dead page
 * carries, `null` when every page is served.
 */

/**
 * THE PAGES THE SERVER CAN SERVE for one answer, in the order it was read.
 *
 * Time order serves every page. A time-of-day order (`tod`) keeps
 * `offset + limit` rows while it streams, so page `p` is served while
 * `p * size <= MAX_SORT_DEPTH`. A value order says in its answer
 * (`sort_index`) how many rows it read rather than took from an index: at or
 * under `MAX_READ_HELD` every page is served; past it a page is served within
 * `MAX_SORT_DEPTH` of the front, and — when no row came from an index and the
 * column is not open interest — of the end too. An order with no answer yet
 * is held to the front depth, the depth every order serves.
 *
 * @param {{total: number, size: number, wire: string | null, report?: any}} o
 * @returns {Served}
 */
export function servedPages({ total, size, wire, report }) {
  const rows = Math.max(1, Math.trunc(size) || 1);
  const n = Number.isSafeInteger(total) && total > 0 ? total : 0;
  const count = Math.max(1, Math.ceil(n / rows));
  const every = { count, front: count, tail: null, why: null };
  if (wire === null || wire === 'ts') return every;
  if (wire !== 'tod' && report && typeof report === 'object') {
    if (report.by_time === true) return every;
    const read = Number(report.scanned_rows);
    if (Number.isSafeInteger(read) && read >= 0 && read <= MAX_READ_HELD) return every;
  }
  const front = Math.min(count, Math.max(1, Math.floor(MAX_SORT_DEPTH / rows)));
  if (front >= count) return every;
  const fromEnd =
    wire !== 'tod' && wire !== 'oi' && report && Number(report.indexed_rows) === 0;
  let tail = fromEnd ? Math.ceil((n - MAX_SORT_DEPTH) / rows) + 1 : null;
  if (tail !== null && tail <= front + 1) return every;
  if (tail !== null && tail > count) tail = null;
  const what = wire === 'tod' ? 'a time-of-day order' : 'this value order';
  const ends = tail === null
    ? `from the front only: rows 1 to ${fmt(front * rows)}`
    : `within ${fmt(MAX_SORT_DEPTH)} rows of either end: pages 1 to ${fmt(front)} and ${fmt(tail)} to ${fmt(count)}`;
  const why =
    `the server reads ${what} rather than taking it from a sort index, and serves it ${ends} ` +
    `(${fmt(MAX_SORT_DEPTH)} rows deep, bars::MAX_SORT_DEPTH). Pages between are refused sort_too_deep, so they are not offered: ` +
    'page in time order, or narrow the days, the time of day or the series to reach every row.';
  return { count, front, tail, why };
}

/** Whether page `p` is one the server serves. @param {Served} s @param {number} p */
export const isServed = (s, p) =>
  Number.isSafeInteger(p) && p >= 1 && p <= s.count && (p <= s.front || (s.tail !== null && p >= s.tail));

/** The last page the server serves. @param {Served} s */
export const lastServed = (s) => (s.tail === null ? s.front : s.count);

/**
 * The page a reader is shown when `p` is asked: 1 below the first, the last
 * served page past it, and the last page before a refused stretch inside it.
 * A clamp at every read, never a write-back: a narrowing that shortens the run
 * cannot strand the reader on a page that does not exist or is not served.
 * @param {Served} s @param {number} p
 */
export function clampServed(s, p) {
  const n = Math.trunc(Number(p));
  if (!Number.isFinite(n) || n < 1) return 1;
  const last = lastServed(s);
  if (n >= last) return last;
  return isServed(s, n) ? n : s.front;
}

/**
 * The served page one step from `p` in direction `step` (+1 or -1), or `null`
 * at the end. Across the dead stretch the step jumps it, so Next never lands
 * on a refusal; the button names the jump.
 * @param {Served} s @param {number} p @param {1 | -1} step
 * @returns {number | null}
 */
export function stepServed(s, p, step) {
  const want = p + step;
  if (want < 1 || want > lastServed(s)) return null;
  if (isServed(s, want)) return want;
  return step > 0 ? s.tail : s.front;
}

/**
 * Which page buttons to draw: first, last, the current page and its
 * neighbours, the rest elided — and never a page the server refuses. A dead
 * stretch is ONE item, `dead: true`, carrying the reason, so the row of
 * buttons says why it stops rather than offering a page that will fail.
 * Constant per paint whatever the page count (a 4,890,333-page run draws at
 * most eleven items).
 *
 * @param {number} cur @param {Served} s
 * @returns {{k: string, p: number, gap: boolean, dead: boolean}[]}
 */
export function pageItems(cur, s) {
  const last = lastServed(s);
  const want = new Set([1, cur - 1, cur, cur + 1, last]);
  if (cur <= 3) for (const p of [2, 3, 4]) want.add(p);
  if (cur >= last - 2) for (const p of [last - 1, last - 2, last - 3]) want.add(p);
  if (s.tail !== null) {
    want.add(s.front);
    want.add(s.tail);
  }
  const list = [...want].filter((p) => isServed(s, p)).sort((a, b) => a - b);
  /** @type {{k: string, p: number, gap: boolean, dead: boolean}[]} */
  const out = [];
  list.forEach((p, i) => {
    const before = i > 0 ? list[i - 1] : 0;
    if (i > 0 && p - before > 1) {
      const dead = s.tail !== null && before <= s.front && p >= s.tail;
      out.push({ k: `${dead ? 'dead' : 'gap'}${p}`, p, gap: true, dead });
    }
    out.push({ k: `p${p}`, p, gap: false, dead: false });
  });
  if (s.tail === null && s.front < s.count) out.push({ k: 'dead-end', p: s.front + 1, gap: true, dead: true });
  return out;
}

/**
 * WHAT THE PAGE BOX TYPED, as a page the server serves, or why not — in words.
 *
 * `0`, `-5`, `2.5`, `1e308`, an empty box and 6,000 digits were each either
 * clamped to a page without a word or passed to the read as they stood; a
 * number box hands over `''` for most of them, so the box is text and is read
 * here. A page past the end, or inside a stretch the server refuses, is
 * refused naming the pages there are.
 *
 * @param {unknown} raw @param {Served} s
 * @returns {{ok: true, page: number} | {ok: false, why: string}}
 */
export function parsePageBox(raw, s) {
  const last = lastServed(s);
  const range = s.tail === null ? `1 to ${fmt(last)}` : `1 to ${fmt(s.front)} or ${fmt(s.tail)} to ${fmt(s.count)}`;
  const text = String(raw ?? '').trim();
  if (text === '') return { ok: false, why: `Type a page number, ${range}.` };
  const got = checkWhole(text, { label: 'Page', min: 1, max: s.count });
  if (!got.ok) {
    return { ok: false, why: `${got.why}: this selection has ${fmt(s.count)} page(s) at this many rows a page. Type ${range}.` };
  }
  const page = got.value;
  if (!isServed(s, page)) return { ok: false, why: `Page ${fmt(page)} is not served: ${s.why} Type ${range}.` };
  return { ok: true, page };
}

/**
 * THE PAGE ONE READ ASKS FOR (R3-03, DB-021).
 *
 * A change of order — another column, the other direction, another set of
 * rows — returns the reader to page one, and it used to do so in TWO
 * requests: the read ran first with the new order at the OLD page (offset
 * 45,000 of a Close sort, read and then thrown away), and only then did the
 * page reset arrive and ask for page one. The read now asks for page one
 * itself the moment it sees the order change, so the reset that follows asks
 * nothing new.
 *
 * `held` is a page an act chose together with the order — an address's page,
 * a Go to's landing — and is kept.
 *
 * @param {{page: number, order: string, lastOrder: string, held: boolean}} o
 */
export function pageToRead({ page, order, lastOrder, held }) {
  const asked = Math.max(1, Math.trunc(Number(page)) || 1);
  return !held && lastOrder !== '' && order !== lastOrder ? 1 : asked;
}

/**
 * ONE QUESTION, ASKED ONCE. The read effect re-runs for every reactive
 * value it touches; a re-run that would ask what is already asked (or
 * answered) sends nothing. `forget` re-arms it, so a refresh or a failure
 * can ask the same question again.
 */
export function askOnce() {
  let asked = '';
  return {
    /** Whether `q` is new; remembered when it is. @param {string} q */
    fresh(q) {
      if (q === asked) return false;
      asked = q;
      return true;
    },
    forget() {
      asked = '';
    },
    get asked() {
      return asked;
    }
  };
}

/**
 * THE STORE'S STATE AS ONE ANSWER SAW IT: its total and every series' own
 * count. Two pages of one read that see different states were read across a
 * write.
 * @param {any} answer
 */
export const generationOf = (answer) =>
  answer ? `${Number(answer.total)}|${(answer.series_totals ?? []).map((/** @type {any} */ s) => `${s.key}=${s.total}`).join(',')}` : '';

/**
 * TWO PAGES OF ONE READ, CHECKED AGAINST EACH OTHER (R3-03).
 *
 * The total and the store's state are compared with the page read before it
 * in the same order: an import that lands between two pages shifts every
 * offset of a newest-first read by the bars it added, so page 2 opened on the
 * last rows of page 1 again. A row already on screen on the page before is
 * DROPPED from the next, and the move is said with both totals — never a
 * repeat, never a silent shift.
 *
 * `prev` is the page read before in the same order, or `null`.
 *
 * @param {{rows: any[], generation: string, total: number} | null} prev
 * @param {{rows: any[], generation: string, total: number}} next
 * @param {(row: any) => string} keyOf the row's identity
 * @returns {{rows: any[], dropped: number, moved: string | null}}
 */
export function joinPage(prev, next, keyOf) {
  if (!prev || prev.generation === next.generation) return { rows: next.rows, dropped: 0, moved: null };
  const seen = new Set(prev.rows.map(keyOf));
  const rows = next.rows.filter((r) => !seen.has(keyOf(r)));
  const dropped = next.rows.length - rows.length;
  const grew = next.total - prev.total;
  const moved =
    `The store ${grew > 0 ? `grew by ${fmt(grew)}` : grew < 0 ? `shrank by ${fmt(-grew)}` : 'changed'} bar(s) between the page before and this one ` +
    `(${fmt(prev.total)} then ${fmt(next.total)}), so the offsets moved under the pager` +
    (dropped > 0 ? `; ${fmt(dropped)} row(s) already shown on the page before are dropped here rather than shown twice.` : '; no row of the page before came back.') +
    ' Read again for pages counted against the store as it is now.';
  return { rows, dropped, moved };
}

/**
 * EVERY ROW OF A WHOLE READ, ONCE (R3-03). A sort the browser does reads the
 * kept rows a thousand at a time; each slice's answer is checked against the
 * first one's total and store state, and a row a shifted slice repeats is
 * dropped. `moved` names a read taken across a write.
 *
 * @param {any[]} bars the slices' rows, in read order
 * @param {any[]} answers every slice's answer, the first one's state the reference
 * @param {(row: any) => string} keyOf
 * @returns {{bars: any[], dropped: number, moved: string | null}}
 */
export function joinSlices(bars, answers, keyOf) {
  const seen = new Set();
  /** @type {any[]} */
  const out = [];
  for (const b of bars) {
    const k = keyOf(b);
    if (seen.has(k)) continue;
    seen.add(k);
    out.push(b);
  }
  const dropped = bars.length - out.length;
  const first = answers[0] ?? null;
  const states = new Set(answers.map(generationOf));
  if (states.size <= 1 && dropped === 0) return { bars: out, dropped, moved: null };
  const last = answers[answers.length - 1] ?? first;
  const moved =
    `The store changed while this sort read its rows (${fmt(Number(first?.total) || 0)} bar(s) when it began, ` +
    `${fmt(Number(last?.total) || 0)} at its last slice)` +
    (dropped > 0 ? `; ${fmt(dropped)} repeated row(s) were dropped` : '') +
    '. The order shown is of the rows read; read again for the store as it is now.';
  return { bars: out, dropped, moved };
}

/**
 * ONE ROW OF /db'S BROKER FEED PICKER — pure, so `web/tests/feed-row.test.js`
 * can drive it under `node --test`.
 *
 * THE STORE DECIDES WHETHER A FEED CAN BE VIEWED; INGEST READINESS DOES NOT.
 *
 * `/feeds.json`'s `ready`/`why` answer an INGEST question — can this feed be
 * pulled or read from its archive folder right now (a credential for a broker,
 * a folder of bought CSVs for an archive vendor). /db reads bars that are
 * ALREADY in the store, and that read never touches the archive folder. The
 * picker used to disable every feed with `ready: false`, so Global Datafeeds —
 * with 122,391 instrument-months of one-second bars held under
 * `bars/gdfl/` — was struck through as "no folder at …/vendor-data/gdfl yet",
 * because the archive had been imported from somewhere else. A feed whose
 * store holds data is selectable, shows its held count like any other, and
 * carries the ingest reason as a secondary note rather than a strike-through.
 *
 * A feed that is not ready and is not known to hold anything stays drawn and
 * disabled with the server's reason (`CLAUDE.md` §4: degrade loudly).
 *
 * WHAT A FEED HOLDS AND HOW IT GOT THERE IS ON ITS TITLE (DB-051, D-3338).
 * `/feeds.json` carries GDFL's measured `archive` (the span its store holds,
 * imported by `cli gdfl-import`, D-2804) and Zerodha's operator `window`; the
 * title states whichever the feed has, in the server's own words, rather than
 * a folder probe's.
 *
 * @param {{wire: string, display: string, ready: boolean, why?: string | null,
 *   archive?: {from: string, to: string, source?: string} | null,
 *   window?: {from: string, to: string, source?: string} | null}} f
 * @param {{cells: number, bars: number, error?: string | null} | undefined} held
 *        this feed's entry in the store survey, or `undefined` when unsurveyed.
 * @param {(n: number) => string} fmt the page's integer formatter.
 */
export function feedPickerRow(f, held, fmt) {
  const ready = f.ready === true;
  const holds = Boolean(held && !held.error && held.cells > 0);
  const selectable = ready || holds;
  const reason =
    f.why ||
    'the server marked this feed not ready and stated no reason, which is itself the thing to fix.';
  const ingestNote = ready ? undefined : `ingest not ready — ${reason}; the bars already held are readable`;
  return {
    key: f.wire,
    name: f.display,
    detail: holds
      ? `${fmt(/** @type {{cells:number}} */ (held).cells)} held`
      : !ready
        ? 'unavailable'
        : held?.error
          ? 'not read'
          : held
            ? `${fmt(held.cells)} held`
            : 'not surveyed',
    disabled: !selectable,
    why: selectable ? ingestNote : reason,
    title: selectable
      ? `Selects ${f.display}. Every count and every row below becomes this feed's, read again from /store.json?feed=${f.wire} and /instruments.json?feed=${f.wire} — nothing is deleted, nothing is merged with the feed you are leaving, and no figure on this page ever puts the two side by side.${
          held
            ? held.error
              ? ` Its store could not be read on the last survey: ${held.error}.`
              : ` The last survey read ${fmt(held.cells)} instrument-month(s) and ${fmt(held.bars)} bar(s) under it.`
            : ' Its store has not been surveyed in this session, so the count beside it is a dash rather than a zero.'
        }${spanSentence(f)}${ready ? '' : ` Its ingest path is not ready (/feeds.json: ${reason}), which stops new data arriving and does not stop the stored bars being read.`}`
      : `${f.display} is refused: its ingest path is not ready and the store survey found nothing held under it. /feeds.json's own reason: ${reason}`
  };
}

/**
 * The span a feed's descriptor states, as one sentence, or `''`.
 *
 * @param {{archive?: {from: string, to: string, source?: string} | null,
 *   window?: {from: string, to: string, source?: string} | null}} f
 */
function spanSentence(f) {
  if (f.archive && f.archive.from && f.archive.to) {
    return ` Its archive runs ${f.archive.from} to ${f.archive.to}${f.archive.source ? ` (${f.archive.source})` : ''}.`;
  }
  if (f.window && f.window.from && f.window.to) {
    return ` Your window for it is ${f.window.from} to ${f.window.to}; a month outside it is hidden unless you ask to see it, and is labelled outside your window when shown.`;
  }
  return '';
}

/**
 * THE FEEDS A PICKER OFFERS (D-3338): Zerodha and GDFL. `/feeds.json` marks
 * every other feed `offered: false`, `excluded_by: "operator"`; those are not
 * listed. A row with no `offered` field is listed, so an older API hides
 * nothing by omission. The descriptors and their code stay; only the menu
 * shrinks. Returns the list unchanged in order.
 *
 * @template {{wire: string}} F
 * @param {F[]} list
 * @returns {F[]}
 */
export function offeredFeeds(list) {
  return Array.isArray(list) ? list.filter((f) => f && /** @type {any} */ (f).offered !== false) : [];
}

/**
 * The feeds the operator excluded, for the one line that says they exist
 * (never a silent omission): `"Dhan, Groww and TrueData are excluded by the
 * operator since 2026-10-06"`, or `''`.
 *
 * @param {Array<{wire: string, display: string}>} list
 */
export function excludedSentence(list) {
  /** @type {any[]} */
  const out = Array.isArray(list) ? list.filter((f) => f && /** @type {any} */ (f).offered === false) : [];
  if (out.length === 0) return '';
  const names = out.map((f) => f.display);
  const joined = names.length === 1 ? names[0] : `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}`;
  const by = out[0].excluded_by ? ` by the ${out[0].excluded_by}` : '';
  const on = out[0].excluded_on ? ` since ${out[0].excluded_on}` : '';
  return `${joined} ${out.length === 1 ? 'is' : 'are'} excluded${by}${on}, so ${out.length === 1 ? 'it is' : 'they are'} not offered here (D-3338).`;
}

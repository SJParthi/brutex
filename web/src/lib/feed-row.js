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
 * @param {{wire: string, display: string, ready: boolean, why?: string | null}} f
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
        }${ready ? '' : ` Its ingest path is not ready (/feeds.json: ${reason}), which stops new data arriving and does not stop the stored bars being read.`}`
      : `${f.display} is refused: its ingest path is not ready and the store survey found nothing held under it. /feeds.json's own reason: ${reason}`
  };
}

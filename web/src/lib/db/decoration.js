/**
 * /db — THE PER-ROW DECORATION OF A CENSUS ROW
 *
 * What one `/store.json` row becomes before anything reads it: the contract tail
 * parsed off the name, the expiry, the short count against the observed
 * denominator, the session count, and the display helpers the coverage grid
 * prints them with. `DecoRow` is the shape this file produces.
 *
 * Every line below was MOVED out of `routes/db/+page.svelte` by WP-00 (the /db
 * page split). The only edits to a moved line are the ones the move needs: a
 * page-level name that now lives in another file is imported, or, for state,
 * read as `db.<name>`; a plain module exports what it declares; and a callback
 * over such a value marks its parameter `any`, the way the page already did
 * elsewhere. Nothing was fixed in the move: every defect the audit found is
 * still present byte for byte, and is fixed by the package that owns this file
 * next.
 *
 * Owner after WP-00: WP-54 (coverage measured against calendar-owed bars).
 *
 * A PLAIN MODULE, NOT A `.svelte.js` ONE: it holds constants and pure
 * functions only, so `node --test` can import it with no compiler. Imports
 * are relative for the same reason — `$lib` is a Vite alias node cannot see.
 */
/* Display only. `month` stays the raw `YYYY-MM` because it is the filter
   itself — `r.month === month`, `r.month.startsWith(typed)`, the `{#each}`
   keys and the sort comparator all read the key, never the label. See the
   rule at the top of `$lib/dates.js`.

   `stampLabel` renders the one instant this page shows a human — when
   `/store.json` was last read. It names Asia/Kolkata rather than inheriting
   the host's zone, so "as of 15:29" is the same minute for every operator
   reading the same store.

   `dayLabel` renders the DAY WINDOW each stored month actually covers.
   `/store.json` has carried `first_ts` / `last_ts` per row — in MICROseconds
   — since the census gained them, and no page has ever read them. A month
   row that says "2020-03" and nothing else cannot distinguish a month held
   from the 2nd to the 31st from one held on the 2nd alone; the window says
   which, in the `02 Sep 2024` form this product uses everywhere. */
import { stampLabel, dayLabel } from '../dates.js';
// THE COMPLETENESS ARITHMETIC, OUT OF THE MARKUP AND UNDER A TEST. Two
// defects lived in these expressions — a denominator taken from whichever
// rung's row arrived first, and a row that was its own denominator reporting
// `full` — and neither was reachable by anything but loading this page
// against a store in exactly the right state. `web/tests/completeness.test.js`
// drives them under `node --test`.
import { denomKey, isSole, denominators } from '../completeness.js';
import { barsPerSession, fmt } from './notes-labels.js';
// A STORE KEY IS NOT AN INSTRUMENT NAME. `$lib/instrument.js` takes the
// contract tail off before the underlying, which is the difference between
// BANKNIFTY and BANKNIFTY-2026-07-28-4810000-PE.
import { parseKey, segmentOf, strikeExact } from '../instrument.js';
/** @typedef {import('../store.svelte.js').StoreRow} StoreRow */

/* FIXED TWO DECIMALS, ALWAYS. Adaptive precision made the digit count change
   from row to row, so the column's decimal point moved and the eye had to
   re-find it on every line. A column of numbers is a column or it is not. */
/* `null` IS A LEGAL ARGUMENT AND IT RENDERS A DASH. A ratio whose
   denominator is zero is not 100% and not 0% — it does not exist, and
   `(null * 100).toFixed(2)` would print `0.00%`, a measurement, over an
   absence. Every caller that can reach an empty denominator passes `null`
   and gets the dash; the REASON belongs beside it at the call site, which is
   the only place that knows which absence it is. */
/** @param {number | null | undefined} p a ratio in 0..1, or the absence of one */
export const pctText = (p) => (p === null || p === undefined ? '—' : `${(p * 100).toFixed(2)}%`);
/**
 * A session count, or a dash when the rung has no recorded session size.
 *
 * `null` reaches here from `sessions()` for a rung `barsPerSession` has no
 * number for. A dash is the honest render: `0.00` would be a measurement and
 * this is an absence. Same rule the % change columns follow.
 *
 * @param {number | null | undefined} d
 */
export const dayText = (d) =>
  d === null || d === undefined ? '—' : Number.isInteger(d) ? String(d) : d.toFixed(2);
/**
 * When `/store.json` was last read, as a date and an IST minute.
 *
 * A bare `toLocaleTimeString()` printed `15:29:04` in whatever zone the
 * machine happens to sit in and named neither the zone nor the day — so a
 * stamp from yesterday's session was indistinguishable from one taken a
 * minute ago, which is the single fact this line exists to report.
 *
 * `0` means the store has NOT been read, and that is an absence: passing it
 * through would render the epoch, `01 Jan 1970, 05:30`, which is a stamp
 * nobody took.
 *
 * @param {number} t epoch MILLISECONDS, or `0` for "never read"
 */
export const clock = (t) => (t ? stampLabel(t) : '—');

/**
 * The DAYS one stored month actually covers. `02 Sep 2024 → 30 Sep 2024`.
 *
 * `null` when the row carries no stamps — an older `/store.json` payload, or
 * an entry written before the census recorded them. The caller prints the
 * absence and names it; inventing the month's first and last calendar day
 * would be this page asserting a trading calendar it does not have, and the
 * first and last day of a month are very often not trading days at all.
 *
 * Both ends go through `dayLabel`, so the month is `Sep` and never `Sept`
 * and the zone is IST whatever the reading machine is set to.
 *
 * THE PARAMETER NAMES THE TWO FIELDS IT READS AND NOT THE WHOLE ROW. Every
 * caller passes a decorated census row, but what this function needs is a
 * pair of stamps — so a month card, a drawer line and a grid cell can all
 * reach it without one of them having to be a census row to qualify.
 *
 * @param {{ firstAt: number | null, lastAt: number | null } | null | undefined} it
 * @returns {string | null} `null` when the row carries no stamps at all
 */
export const heldWindow = (it) =>
  it && it.firstAt !== null && it.lastAt !== null
    ? `${dayLabel(it.firstAt)} → ${dayLabel(it.lastAt)}`
    : null;

/**
 * The denominator key for one row: its month AND its rung.
 *
 * @param {StoreRow} r
 */
export const fullestKey = (r) => denomKey(r.month, r.timeframe);

/**
 * Whole sessions held, from the bar count AT THIS ROW'S RUNG.
 *
 * Fractional means a partial day. `null` when the rung has no recorded
 * bars-per-session — the caller draws a dash, never a figure computed
 * against a denominator nobody chose.
 *
 * @param {StoreRow} r
 * @returns {number | null}
 */
export function sessions(r) {
  const per = barsPerSession(r.timeframe);
  return per === null ? null : r.rows / per;
}

/* ======================================================================
   THE CONTRACT TAIL — the ONE place a stored option names its strike
   ----------------------------------------------------------------------
   `/store.json` sends ten fields per row and NOT ONE OF THEM IS A STRIKE, a
   side, an expiry or a price: `instrument`, `month`, `timeframe`, `rows`,
   `first_ts`, `last_ts`, `chg_bps`, `chg_why`, `prev_chg_bps`,
   `prev_chg_why`. That list is the row writer in `crates/api/src/server.rs`
   and the table in the doc comment above `store_json`, and it is exhaustive.

   So the only place a strike can be read from is the instrument NAME, and
   that is readable only because this repository writes the name in exactly
   one place: `Display for InstrumentKey` in `crates/core/src/instrument.rs`
   ends an option `-{strike.raw()}-{CE|PE}` — the strike in PAISA, then the
   two-letter side every Indian vendor uses.

   THE PARSE IS A SUFFIX TEST AND IT REFUSES EVERYTHING ELSE. It is not a
   guess at a spelling: it recognises exactly the tail this repository emits
   and answers `null` for every other name — which today is every name in the
   store, because indices, equities and futures carry no strike at all. A
   name that is an option under some other spelling is reported as NO
   CONTRACT rather than as a strike this page decoded by pattern-matching.
   `CLAUDE.md` §3 rule 1: no invention.
   ====================================================================== */
export const OPTION_TAIL = /-(\d+)-(CE|PE)$/;

/**
 * The strike in paisa and the side one census instrument name carries, or
 * `null` when the name does not end the way this repository ends an option's.
 *
 * @param {string} name
 * @returns {{ strike: number, side: 'CE' | 'PE' } | null}
 */
export function contractOf(name) {
  const m = name.match(OPTION_TAIL);
  return m === null
    ? null
    : { strike: Number(m[1]), side: /** @type {'CE' | 'PE'} */ (m[2]) };
}

/* ---- THE SAME TAIL, ONE FIELD EARLIER — the expiry -------------------
   SAME SOURCE AND SAME RULE AS THE STRIKE ABOVE IT. `Display for
   InstrumentKey` in `crates/core/src/instrument.rs` writes exactly two
   contract shapes and no others:

       Kind::Future  ->  …-{expiry}-FUT
       Kind::Option  ->  …-{expiry}-{strike.raw()}-{CE|PE}

   and `Display for Expiry`, in the same file, renders that expiry
   `{:04}-{:02}-{:02}` — an ISO calendar day, fixed width, zero padded. The
   two patterns below are those two lines and nothing else.

   IT IS A SUFFIX TEST AND IT REFUSES EVERYTHING ELSE, exactly as
   `OPTION_TAIL` does. `/store.json` sends ten fields per row and not one of
   them is an expiry — the exhaustive list is in the block above
   `OPTION_TAIL` — so the NAME is the only source there is, and a name that
   does not end one of these two ways answers `null`, meaning NO CONTRACT.
   It never answers an expiry decoded out of a spelling this page guessed at.
   `CLAUDE.md` §3 rule 1: no invention.

   THE OPTION PATTERN IS TRIED FIRST AND THAT ORDER MATTERS. `-FUT` cannot
   end an option's name, so the two are disjoint and either order returns the
   same answer today; the option is tried first anyway so that the day a
   third shape is added, the more specific tail is already the one that
   wins. */
export const FUTURE_TAIL = /-(\d{4}-\d{2}-\d{2})-FUT$/;
export const OPTION_EXPIRY_TAIL = /-(\d{4}-\d{2}-\d{2})-\d+-(?:CE|PE)$/;

/**
 * The expiry one census instrument name carries, as the RAW ISO day, or
 * `null` when the name is not a contract this repository wrote.
 *
 * The returned string is a key and stays one: it is the `{#each}` key, the
 * Set member, the `===` in the gate and the sort key. `dayLabel` is applied
 * at the render site and nowhere near any of those.
 *
 * @param {string} name
 * @returns {string | null}
 */
export function expiryOf(name) {
  const m = name.match(OPTION_EXPIRY_TAIL) ?? name.match(FUTURE_TAIL);
  return m === null ? null : m[1];
}

/**
 * A strike in paisa, rendered as rupees, EXACTLY.
 *
 * The same integer arithmetic `bpsText` uses and for the same reason:
 * `CLAUDE.md` §7 bans the float for money, and `p / 100` is a float division
 * of a value that has an exact decimal expansion. The remainder is taken with
 * `%` — an integer operation — and the quotient is a multiple of 100 divided
 * by 100, which IEEE returns exactly inside 2^53.
 *
 * The paisa are dropped only when they are ZERO, which is the ordinary case
 * on an exchange-listed strike. A strike that is not a whole rupee prints its
 * two decimals rather than being rounded into one that was never listed.
 *
 * @param {number} p
 */
export function strikeText(p) {
  const frac = p % 100;
  const whole = (p - frac) / 100;
  return frac === 0 ? fmt(whole) : `${fmt(whole)}.${String(frac).padStart(2, '0')}`;
}

/* ======================================================================
   DECORATION — every derived per-row number, computed ONCE
   ----------------------------------------------------------------------
   The comparator runs O(n log n) times. A comparator that called `shortBy`
   would pay two Map probes per comparison, so the short count, the session
   count and the completeness ratio are attached to the row here, in one
   pass, and the sort compares plain numbers.
   ====================================================================== */
/** @param {StoreRow} r @param {ReturnType<typeof denominators>} observed
 * @param {ReturnType<typeof instrumentDescription>} meta */
export function decorateRow(r, observed, meta) {
    const denom = observed.fullest.get(fullestKey(r)) ?? r.rows;
    const short = Math.max(0, denom - r.rows);
    // THE CONTRACT IS PARSED ONCE PER ROW PER DATASET, here, beside every
    // other derived per-row number — never inside the chain, the counts, the
    // ladder or the row filter. Those run per paint and per keystroke; this
    // runs when the store is re-read. Same rule `short` and `days` follow.
    const ct = meta.ct;
    // THE EXPIRY IS PARSED IN THE SAME PASS AND FOR THE SAME REASON. It is
    // read by the gate, which runs on every keystroke, every scroll frame
    // and every click; parsing it there would put a regex match on the hot
    // path for a value that changes only when the store is re-read.
    const ex = meta.ex;
    // ITS OWN DENOMINATOR, OR A REAL ONE. See `soleDenom` and `SOLE_WHY`.
    const sole = isSole(observed.support, r);
    // ONCE PER ROW AT READ TIME, for the reason the line above `expiryOf`
    // gives: this is hoisted off the hot path deliberately.
    // HOISTED, the way `sessions()` above already does it. `barsPerSession`
    // was called TWICE in each of the two expressions below — once to guard
    // and once to divide — so the guard narrowed nothing the checker could
    // follow and the function ran twice per row per render. One call, one
    // value, and the null-check now provably covers the use.
    const per = barsPerSession(r.timeframe);
    return {
      ...r,
      // A SEPARATOR THAT CANNOT OCCUR IN THE DATA, WRITTEN AS AN ESCAPE.
      // This was two literal NUL bytes in the source, which made `grep` and
      // `ripgrep` classify the whole file as binary and return nothing for
      // every pattern — on the file whose completeness bug took an 89-agent
      // sweep to find. The runtime key is byte-identical; the source is now
      // ASCII and searchable.
      key: `${r.instrument}\u0000${r.month}\u0000${r.timeframe}`,
      /* WHAT DISTINGUISHES A ROW GOES IN `sym`, AND THE REST IN `head`.
         `head`/`sym` were "everything before the last hyphen" / "everything
         after it", which is right for a spot key and useless for a contract:
         on `NSE-FNO-BANKNIFTY-2026-07-28-4810000-PE` it puts thirty-six
         characters in `head` and the two characters `PE` in `sym`.

         The instrument column is 224px and that string needs 303px, so 137px
         are clipped -- and the clipped end is the STRIKE and the SIDE, the
         only fields that tell one contract from another. Measured on 18
         option rows: three visually distinct labels, because every one read
         `NSE-FNO-BANKNIFTY-2026-0...`. The column was showing the part that
         is identical on every row and hiding the part that is not.

         The underlying is already named once, at the top of the page, by the
         Instrument rung. Repeating it per row buys nothing and costs the
         strike. */
      head: meta.head,
      // `sym` IS THE TAIL AND IS NOT THE NAME. On a spot key the two agree;
      // on `NSE-FNO-BANKNIFTY-2026-07-28-4810000-PE` this is `PE`. It stays
      // because the table and the index have always displayed and probed it,
      // and `underlying` beside it is what an INSTRUMENT actually is.
      sym: meta.sym,
      // THE NAME A HUMAN USES, and the key the Instrument rung groups by.
      // Parsed once per row at store-read time, never per keystroke.
      underlying: meta.underlying,
      // WHICH SEGMENT RUNG THIS ROW BELONGS TO — spot, futures or options.
      // `kind` below is the key's own second token and cannot answer it:
      // FNO covers futures and options alike, and only the tail separates
      // them.
      // `expiry`, `strike` and `side` are NOT set here: the row already
      // carries them, parsed from `ct` further down with a fuller account of
      // why each is `null`. Adding a second spelling of the same three fields
      // is how two readers come to disagree about one row -- svelte-check
      // caught it as three duplicate keys, which is the cheap version of that
      // lesson.
      segRung: meta.segRung,
      kind: meta.kind,
      short,
      days: per === null ? null : r.rows / per,
      lost: per === null ? null : short / per,
      // NULL, NOT 100.00%. A ratio against itself is 1 for every row that
      // ever existed; `pctText` draws the dash and every meter beside it is
      // suppressed rather than filled — the rule the Coverage tile already
      // keeps: "a bar is a length, and a length is a claim".
      pct: sole ? null : denom > 0 ? r.rows / denom : 1,
      sole,
      denom,
      // THE WIRE IS TOTAL, AND SO IS THIS. `chg_bps` is an integer or it is
      // null, and `chg_why` is the reason for the null — exactly one of the
      // pair on every row. `Number.isInteger` rather than a truthiness test,
      // because 0 basis points is a real answer: a month that closed where it
      // opened, which `!r.chg_bps` would turn into an unknown.
      // THE STRIKE IN PAISA AND THE SIDE, or `null` on both when the name
      // carries no contract. `null` is NOT "strike zero": a zero strike is a
      // price and this is the absence of one, and every reader below tests
      // for `null` rather than for falsity so a hypothetical 0-paisa strike
      // could never be silently dropped the way `!r.strike` would drop it.
      strike: ct === null ? null : ct.strike,
      side: ct === null ? null : ct.side,
      // THE RAW ISO DAY, OR `null` FOR "THIS ROW IS NOT A CONTRACT". `null`
      // is not "expires never" and it is not the empty string: an index and
      // an equity HAVE no expiry, and the gate below reads this field for
      // exactly that distinction — a row with `null` is passed through
      // untouched, a row with a day must match the chosen one.
      expiry: ex,
      chg: Number.isInteger(r.chg_bps) ? r.chg_bps : null,
      chgWhy: r.chg_why ?? null,
      prevChg: Number.isInteger(r.prev_chg_bps) ? r.prev_chg_bps : null,
      prevChgWhy: r.prev_chg_why ?? null,
      // MICROSECONDS ON THE WIRE, MILLISECONDS AT THE CALL SITE. The server
      // sends `first_ts` / `last_ts` unconverted and says so; `$lib/dates.js`
      // takes epoch MILLISECONDS and refuses to sniff the unit, because this
      // repository carries bar times in seconds and response times in
      // milliseconds and a formatter that guessed would render 1970 for half
      // its callers. The `/1000` happens HERE, where the unit is known.
      //
      // `> 0` AND NOT `Number.isFinite`. A zero stamp is not an instant this
      // store ever recorded — it would render `01 Jan 1970, 05:30`, a date
      // nobody took, in a column whose whole job is to say which days are
      // actually held. `null` reaches the renderer and it draws the absence.
      firstAt: typeof r.first_ts === 'number' && r.first_ts > 0 ? r.first_ts / 1000 : null,
      lastAt: typeof r.last_ts === 'number' && r.last_ts > 0 ? r.last_ts / 1000 : null,
      // THREE STATES, NOT TWO. "not full" hides the difference between a
      // late start on one day and two whole trading days that never landed,
      // and the second is the one that needs a re-pull.
      // THE THRESHOLD IS ONE SESSION AT THIS ROW'S RUNG. Against a fixed
      // 375 a daily row could never reach 'gap' — every shortfall is under
      // 375 bars — so two missing trading days read as 'near', the state
      // that means "a late start on one day".
      state:
        sole
          ? 'sole'
          : short === 0
          ? 'full'
          // NO 375 FALLBACK. A rung with no recorded session size has no
            // threshold either, and `?? BARS_PER_SESSION` quietly restored the
            // minute yardstick this function exists to remove. An unknown rung
            // is 'gap' — the state that asks for a re-pull — because a
            // shortfall nobody can size is not a near miss.
            : per === null
              ? 'gap'
              : short < per
                ? 'near'
                : 'gap'
    };
}

/** @param {string} name */
export function instrumentDescription(name) {
  const parsed = parseKey(name), cut = name.lastIndexOf('-'), parts = name.split('-');
  return {
    ct: contractOf(name), ex: expiryOf(name),
    head: contractHead(parsed, cut, name), sym: contractSym(parsed, cut, name),
    underlying: parsed.underlying ?? name, segRung: segmentOf(parsed),
    kind: parts.length > 1 ? parts[1] : '—'
  };
}
/** @typedef {ReturnType<typeof decorateRow>} DecoRow */
/**
 * ONE DECORATED CENSUS ROW — the shape everything below this line walks.
 *
 * DERIVED FROM `deco` RATHER THAN WRITTEN OUT, and that is the whole point.
 * The decoration above attaches nineteen fields to the ten `/store.json`
 * sends, and a hand-written copy of that list is a second declaration of one
 * shape: the day `lost` or `expiry` changes its meaning, the copy still says
 * the old thing and every reader downstream believes the copy. Reading the
 * element type off the expression that produces it cannot drift, because
 * there is nothing to keep in step.
 *
 * `DecoRow` is inferred directly from the unchanged per-row decoration.
 */

/* ---- THE INSTRUMENT RUNG, WHICH IS THE TEXT BOX -----------------------
   ONE STATE, TWO ENTRY POINTS. The page has had an instrument filter since
   it was written, and it answers this rung's question faster than a list
   can: a Map probe against a typed prefix. A second, independent "chosen
   instrument" beside it would be two answers to one question, and the table
   would look correct while returning nothing the moment they disagreed.

   So the picker does not own a selection at all. It WRITES the text box, and
   it shows a row as ticked exactly when the text box currently holds that
   row's key. Typing anything else unticks it with no bookkeeping, because
   there is nothing to keep. The Picker is an enumeration of what the coarser
   rungs left; the text box is the state.
   ====================================================================== */
/* FOLDED BY UNDERLYING, NOT BY STORE KEY, and that one word is the fix.
   Measured on the live store: 27 keys, ONE underlying. Folding by
   `it.instrument` offered BANKNIFTY twenty-seven times -- once as the spot
   series and twenty-six times as its option contracts -- so the rung that
   asks "which instrument" was answering "which contract", and the Segment
   rung beside it was left asking a question that had already been answered.
   `key` stays the field name because everything downstream reads it; what
   changed is what a key IS. */
/**
 * The INVARIANT half of a row's name — dimmed, and the same on every sibling.
 *
 * @param {ReturnType<typeof parseKey>} p
 * @param {number} cut index of the last hyphen, for the spot path
 * @param {string} raw the whole store key
 */
export function contractHead(p, cut, raw) {
  if (p.underlying === null || p.kind === 'spot') {
    return cut > 0 ? raw.slice(0, cut + 1) : '';
  }
  /* NOTHING. Not the underlying, not the exchange, not the segment.
     All three are already named ABOVE the grid -- the Instrument rung says
     BANKNIFTY, the Segment rung says Expired options -- and they are
     identical on all 247 rows. Repeating them per row spent the column on
     the one part that never varies and clipped the part that does: with the
     prefix in place the strike fitted and the SIDE did not, so a CE and its
     PE still read the same. The full store key is on the cell's title. */
  return '';
}

/**
 * The half that VARIES — bold, and the reason one row is not another.
 *
 * A future is its expiry; an option is its strike and side. The expiry is on
 * the Contract rung above and is the same for every row under one chosen
 * contract, so it is not repeated per row for an option.
 *
 * @param {ReturnType<typeof parseKey>} p
 * @param {number} cut
 * @param {string} raw
 */
export function contractSym(p, cut, raw) {
  if (p.underlying === null || p.kind === 'spot') {
    return cut > 0 ? raw.slice(cut + 1) : raw;
  }
  if (p.kind === 'future') return `${p.expiry} FUT`;
  /* GROUPED THE INDIAN WAY when the strike is a whole rupee, which is the
     ordinary case for a listed strike -- the Strike rung above says
     "50,000 to 67,500" and a row reading `50000` beside it is the same
     number spelled two ways. `strikeExact` stays the source of truth: it is
     integer arithmetic on paisa per §7, and its output is only grouped, never
     recomputed. */
  const exact = strikeExact(p.strike);
  const rupees = exact.includes('.') ? exact : fmt(Number(exact));
  return `${rupees} ${p.side}`;
}

/* `ROW` AND `HEAD` HAVE MOVED UP, to the `Fit` block beside `pageSize`, and
   the move was forced rather than tidy: `rowsThatFit` is declared two
   thousand lines above this point and divides by both. It RAN — a `$derived`
   is evaluated lazily, long after the whole script has been through — so the
   only thing that reported it was `svelte-check`, four times, on a temporal
   dead zone that a browser never reaches. Working by luck is not the same as
   working, and the next reader of this file would have had to establish that
   for themselves. They are still the same two constants this section uses. */
export const OVER = 6;

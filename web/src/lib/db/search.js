/**
 * /db — ONE SEARCH BOX OVER TICKERS, NATURAL NAMES, ALIASES AND EXPIRIES
 * (WP-51; D-3332, D-3337; DB-001, DB-040, DB-G97).
 *
 * # What it replaced
 *
 * The box this page had was a prefix match over four tokens of every census
 * row (instrument, displayed tail, underlying, month). Measured over the
 * 2,42,341-row GDFL census on 6 Oct 2026 (DB-G97): `NIFTY28MAR1911500CE`,
 * `NIFTY 11500 CE`, `NIFTY 11,500 CE`, `11500`, `2019-03-28`, `28 Mar 2019`
 * and `28MAR19` each found 0 rows; `11,500 CE` found 182 rows across NIFTY
 * AND NESTLEIND; `FINNIFTY`, `NIFTY 50`, `NIFTY BANK`, `ZOMATO`,
 * `TATAMOTORS` and `IIFLWAM` found nothing (DB-040), although `/aliases.json`
 * (WP-20, D-3332) and `/indexmap.json` already knew those names. And a typed
 * prefix used to BE the selection, so `NIFTY` blended NIFTY100's closes into
 * NIFTY's (DB-001; WP-50 made the pick an exact key).
 *
 * # What it is now
 *
 * Two indexes, built once per census and alias table, and a probe per
 * keystroke:
 *
 * 1. THE CATALOG (`buildCatalog`): every underlying the feed's census holds,
 *    and under each its stored expiries, under each expiry its future and its
 *    strikes, under each strike its CE and PE — each leaf the store key and
 *    the instrument-months held for it. Three reverse maps answer the
 *    questions that name no underlying: expiry → underlyings, `YYYY-MM` →
 *    (underlying, expiry), strike → underlying → expiries.
 * 2. THE NAMES (`buildNames`): every name an underlying goes by — its store
 *    key, every name of a SOURCED index row (F&O symbol, NSE index name,
 *    `indexSymbol` = GDFL file stem, Kite tradingsymbol) and every old symbol
 *    of a SOURCED symbol change — compacted (spaces removed, upper case) and
 *    indexed by every infix up to `MAX_PREFIX` through `$lib/find.js`, so
 *    `BANK` finds BANKNIFTY (by `NIFTY BANK`) and AXISBANK alike. A row
 *    D-3332 leaves UNVERIFIED is not applied (D-3332 decision 1).
 *
 * A search (`search`) reads the text as a name (exact, prefix, then
 * substring) and, through `contract-parse.js`, as every contract spelling it
 * could be; it keeps only readings the catalog HOLDS, and returns hits GROUPED
 * BY UNDERLYING — a group is one underlying and only its own hits, so
 * NESTLEIND's 11,500 CE is never drawn under NIFTY. A hit carries the whole
 * selection it stands for (underlying, segment, expiry, strike, side) and
 * `selectionOf` turns it into the rung values WP-50's selection takes, so one
 * press sets them all.
 *
 * # Cost (`CLAUDE.md` §3 rule 4; measured in `web/tests/db-search.test.js`)
 *
 * - Building the catalog is one pass over the census rows, in cancellable
 *   chunks (`buildCatalog`), then one pass over the distinct contracts for
 *   the reverse maps. Once per census, never per keystroke.
 * - A keystroke is: tokenising the text (O(length)); one `find.js` probe (a
 *   `Map.get` plus a filter of that gram's bucket of NAMES — hundreds, never
 *   the census); for each reading, Map probes down the catalog. The work
 *   after the probes is the size of the answer, and the answer is CAPPED at
 *   `MAX_COUNT` hits, past which the count says "more than" instead of
 *   walking on. No keystroke walks the census.
 *
 * A PLAIN MODULE with relative imports and no runes, so `node --test` drives
 * it.
 */
import { parseQuery } from './contract-parse.js';
import { build as buildFind, probe as probeFind } from '../find.js';
import { strikeText } from './decoration.js';
import { dayLabel, monthLabel } from '../dates.js';
import { segmentOf, epochDayOf, isoOfEpoch, weekdayOf, WEEKDAYS } from './selection-model.js';
import { SEG_VIEW } from './notes-controls.js';
import { fmt, tfCmp } from './notes-labels.js';

/* ======================================================================
   THE ONE CONSTANT THE OLD CENSUS PREFIX SEAM STILL NAMES
   ----------------------------------------------------------------------
   `MAX_PREFIX` is the depth the old box probed the census row prefix
   buckets at (`DbSearch`'s retained `textMatched`), and
   `web/tests/database-page-fixture.js` (WP-52's) imports it from here. The
   search box no longer reads those buckets; the constant stays until the
   seam's owners retire it.
   ====================================================================== */
export const MAX_PREFIX = 4;

/** The most groups the list draws; the rest are counted. */
export const MAX_GROUPS = 8;
/** The most hits one group draws; the rest are counted. */
export const HITS_PER_GROUP = 6;
/**
 * How many hits a search counts before it stops and says "more than". A
 * bound on the work one keystroke may do (`11500` alone reaches every
 * underlying's every expiry holding that strike); the count it stops at is
 * printed as a lower bound, never as the total.
 */
export const MAX_COUNT = 5000;

/** How many census rows one chunk of `buildCatalog` folds before yielding. */
export const CATALOG_CHUNK = 8192;

/* ======================================================================
   THE CATALOG — what the census holds, folded once
   ====================================================================== */

/**
 * @typedef {object} Leaf one stored instrument
 * @property {string} instrument the store key, e.g. `NSE-FNO-NIFTY-2019-03-28-1150000-CE`
 * @property {number} months instrument-months the census holds for it (rows, every rung)
 */
/**
 * @typedef {object} ExpiryNode
 * @property {string} iso
 * @property {Leaf | null} fut
 * @property {Map<number, {CE: Leaf | null, PE: Leaf | null}>} strikes paisa → sides
 * @property {number} months instrument-months under this expiry
 */
/**
 * @typedef {object} Under one underlying
 * @property {string} key the store's own spelling (`NIFTY`, `NIFTYFINSERVICE`)
 * @property {string | null} spot the spot series' store key, or `null` (option-only)
 * @property {number} spotMonths
 * @property {number} contractMonths
 * @property {number} spotInWindow spot months inside the feed's operator window (all, with no window)
 * @property {Set<string>} spotTfs rungs the spot series is held at
 * @property {Set<string>} contractTfs rungs its contracts are held at
 * @property {Map<string, ExpiryNode>} expiries
 * @property {string[]} expiryList ascending, filled by `finishCatalog`
 */
/**
 * @typedef {object} Catalog
 * @property {Map<string, Under>} unders
 * @property {Map<string, string[]>} byExpiry ISO → underlyings, sorted
 * @property {Map<string, Array<[string, string]>>} byMonth `YYYY-MM` → (underlying, expiry)
 * @property {Map<number, Map<string, string[]>>} byStrike paisa → underlying → expiries (ascending)
 * @property {string[]} expiries every expiry, ascending
 * @property {number} rows census rows folded
 * @property {number} contracts distinct contracts
 */

/** @returns {Catalog} */
export function emptyCatalog() {
  return {
    unders: new Map(),
    byExpiry: new Map(),
    byMonth: new Map(),
    byStrike: new Map(),
    expiries: [],
    rows: 0,
    contracts: 0
  };
}

/**
 * Fold census rows `[from, to)` into `cat`. A row is a decorated census row
 * (`$lib/db/decoration.js`): `underlying`, `instrument`, `month`,
 * `timeframe`, and the `expiry`, `strike` and `side` read off its name — the
 * same parse the page's grid uses, so the search cannot read a contract the
 * page would read another way.
 *
 * @param {Catalog} cat
 * @param {ArrayLike<any>} rows
 * @param {number} from
 * @param {number} to
 * @param {{from: string, to: string} | null} window the operator window's months (`YYYY-MM`), or `null`
 */
export function foldRows(cat, rows, from, to, window) {
  for (let i = from; i < to; i += 1) {
    const r = rows[i];
    const key = r?.underlying;
    if (typeof key !== 'string' || key === '') continue;
    let u = cat.unders.get(key);
    if (!u) {
      u = {
        key,
        spot: null,
        spotMonths: 0,
        contractMonths: 0,
        spotInWindow: 0,
        spotTfs: new Set(),
        contractTfs: new Set(),
        expiries: new Map(),
        expiryList: []
      };
      cat.unders.set(key, u);
    }
    cat.rows += 1;
    if (r.expiry === null || r.expiry === undefined) {
      u.spot = r.instrument;
      u.spotMonths += 1;
      u.spotTfs.add(r.timeframe);
      if (window === null || (r.month >= window.from && r.month <= window.to)) u.spotInWindow += 1;
      continue;
    }
    u.contractMonths += 1;
    u.contractTfs.add(r.timeframe);
    let e = u.expiries.get(r.expiry);
    if (!e) {
      e = { iso: r.expiry, fut: null, strikes: new Map(), months: 0 };
      u.expiries.set(r.expiry, e);
    }
    e.months += 1;
    if (r.strike === null || r.strike === undefined) {
      if (e.fut === null) {
        e.fut = { instrument: r.instrument, months: 0 };
        cat.contracts += 1;
      }
      e.fut.months += 1;
      continue;
    }
    let s = e.strikes.get(r.strike);
    if (!s) {
      s = { CE: null, PE: null };
      e.strikes.set(r.strike, s);
    }
    const side = r.side === 'PE' ? 'PE' : 'CE';
    let leaf = s[side];
    if (leaf === null) {
      leaf = s[side] = { instrument: r.instrument, months: 0 };
      cat.contracts += 1;
    }
    leaf.months += 1;
  }
}

/**
 * The reverse maps, from the folded catalog: one pass over the distinct
 * (underlying, expiry, strike) nodes, never over the census rows again.
 *
 * @param {Catalog} cat
 * @returns {Catalog}
 */
export function finishCatalog(cat) {
  const all = new Set();
  for (const u of cat.unders.values()) {
    u.expiryList = [...u.expiries.keys()].sort();
    for (const iso of u.expiryList) {
      all.add(iso);
      let list = cat.byExpiry.get(iso);
      if (!list) cat.byExpiry.set(iso, (list = []));
      list.push(u.key);
      const ym = iso.slice(0, 7);
      let month = cat.byMonth.get(ym);
      if (!month) cat.byMonth.set(ym, (month = []));
      month.push([u.key, iso]);
      const node = /** @type {ExpiryNode} */ (u.expiries.get(iso));
      for (const k of node.strikes.keys()) {
        let by = cat.byStrike.get(k);
        if (!by) cat.byStrike.set(k, (by = new Map()));
        let exps = by.get(u.key);
        if (!exps) by.set(u.key, (exps = []));
        exps.push(iso);
      }
    }
  }
  for (const list of cat.byExpiry.values()) list.sort();
  cat.expiries = [...all].sort();
  return cat;
}

/**
 * The operator window's month bounds from a `/feeds.json` row, or `null`.
 *
 * @param {any} feed
 * @returns {{from: string, to: string} | null}
 */
export function windowOf(feed) {
  const w = feed?.window;
  return w && typeof w.from === 'string' && typeof w.to === 'string'
    ? { from: w.from.slice(0, 7), to: w.to.slice(0, 7) }
    : null;
}

/**
 * The catalog of a census, built in cancellable chunks so a 3,85,442-row GDFL
 * census never holds the main thread for the whole fold.
 *
 * @param {ArrayLike<any>} rows decorated census rows
 * @param {{signal?: AbortSignal, chunkSize?: number, yieldControl?: () => Promise<unknown>,
 *   window?: {from: string, to: string} | null}} [options]
 * @returns {Promise<Catalog>}
 */
export async function buildCatalog(rows, options = {}) {
  const size = options.chunkSize ?? CATALOG_CHUNK;
  if (!Number.isInteger(size) || size < 1) throw new Error(`catalog chunk size must be a positive integer, not ${size}`);
  const cat = emptyCatalog();
  const n = rows?.length ?? 0;
  for (let from = 0; from < n; from += size) {
    options.signal?.throwIfAborted();
    foldRows(cat, rows, from, Math.min(n, from + size), options.window ?? null);
    if (options.yieldControl && from + size < n) await options.yieldControl();
  }
  options.signal?.throwIfAborted();
  return finishCatalog(cat);
}

/**
 * The same catalog in one synchronous pass, for tests and small censuses.
 *
 * @param {ArrayLike<any>} rows
 * @param {{from: string, to: string} | null} [window]
 */
export function buildCatalogNow(rows, window = null) {
  const cat = emptyCatalog();
  foldRows(cat, rows, 0, rows?.length ?? 0, window);
  return finishCatalog(cat);
}

/* ======================================================================
   THE NAMES — every name an underlying goes by
   ====================================================================== */

/**
 * @typedef {object} NameRow
 * @property {string} compact upper case, spaces removed: what is indexed
 * @property {string} name as written (`NIFTY FIN SERVICE`)
 * @property {string} target the held underlying it leads to
 * @property {'key' | 'index' | 'formerly' | 'renamed'} via why it leads there
 * @property {string} note the sentence the group shows for this name, `''` for a plain key
 */
/**
 * @typedef {object} Names
 * @property {import('../find.js').Index<NameRow>} find
 * @property {NameRow[]} rows
 * @property {Map<string, string[]>} notes underlying → what it is also called (sourced)
 * @property {Map<string, string>} known every sourced alias name (compact) → what it is, held or not
 * @property {string | null} why why the alias table is not in force, or `null`
 */

/** @param {string} s */
export const compactOf = (s) => String(s ?? '').toUpperCase().replace(/\s+/g, '');

/**
 * A day as `DD Mon YYYY` from an ISO day — `dayLabel` reads an ISO day as
 * midnight UTC and prints its IST calendar day, which is the same day.
 *
 * @param {string} iso
 */
const dayText = (iso) => dayLabel(iso);

/**
 * The day before an ISO day: the last day an old symbol was in force when
 * the new one is effective from `iso` (NSE's `symbolchange.csv` dates the
 * change by the day the new symbol applies, D-3332).
 *
 * @param {string} iso
 */
const dayBefore = (iso) => isoOfEpoch(epochDayOf(iso) - 1);

/**
 * Whether an alias row is in force: D-3332 decision 1 applies a row only
 * with a source.
 *
 * @param {any} row
 */
const sourced = (row) => row?.status === 'sourced';

/**
 * The names of every held underlying, from the catalog and `/aliases.json`.
 *
 * @param {Catalog} cat
 * @param {any} aliases the `/aliases.json` body, or `null` when it could not be read
 * @param {string | null} [aliasWhy] why it could not be read
 * @returns {Names}
 */
export function buildNames(cat, aliases, aliasWhy = null) {
  /** @type {NameRow[]} */
  const rows = [];
  /** @type {Map<string, string[]>} */
  const notes = new Map();
  /** @type {Map<string, string>} */
  const known = new Map();
  const held = (/** @type {unknown} */ k) => typeof k === 'string' && cat.unders.has(k);
  /** @param {string} u @param {string} line */
  const note = (u, line) => {
    let list = notes.get(u);
    if (!list) notes.set(u, (list = []));
    if (!list.includes(line)) list.push(line);
  };
  for (const u of cat.unders.keys()) rows.push({ compact: compactOf(u), name: u, target: u, via: 'key', note: '' });

  const index = Array.isArray(aliases?.index) ? aliases.index : [];
  for (const row of index) {
    if (!sourced(row)) continue;
    const names = [row.fno, row.nse_name, row.index_symbol, row.kite, row.store_key].filter(
      (n, i, all) => typeof n === 'string' && n !== '' && all.indexOf(n) === i
    );
    /* KNOWN EVEN WHEN NOT HELD, so a name the table names and this feed does
       not hold is answered as that, not as "nothing is called this". */
    for (const n of names) {
      known.set(
        compactOf(n),
        `NSE index ${row.nse_name} (index symbol ${row.index_symbol}${row.fno ? `, F&O symbol ${row.fno}` : ''}, filed as ${row.store_key})`
      );
    }
    const targets = [row.store_key, row.fno].filter((t, i, all) => held(t) && all.indexOf(t) === i);
    if (targets.length === 0) continue;
    for (const t of targets) {
      const what =
        t === row.store_key
          ? `NSE index ${row.nse_name} (index symbol ${row.index_symbol}${row.fno ? `, F&O symbol ${row.fno}` : ''}${row.reference_only ? ', reference only' : ''})`
          : `F&O symbol of NSE index ${row.nse_name}; its spot is filed as ${row.store_key}`;
      note(t, what);
      for (const n of names) rows.push({ compact: compactOf(n), name: n, target: t, via: 'index', note: what });
    }
  }

  /* A SYMBOL CHANGE LEADS EVERY OLD NAME TO THE NEWEST HELD NAME ITS CHAIN
     REACHES (LTI → LTIM → LTM), and the group of that name says what it was
     formerly called and until when. An old name the store holds under its
     own key (GDFL's TATAMOTORS options) is its own group too, and says what
     it became. */
  const changes = Array.isArray(aliases?.symbol_changes) ? aliases.symbol_changes.filter(sourced) : [];
  /** @type {Map<string, any>} */
  const byOld = new Map();
  for (const c of changes) if (typeof c.old === 'string') byOld.set(c.old, c);
  /** @param {string} sym */
  const chainFrom = (sym) => {
    /** @type {any[]} */
    const out = [];
    const seen = new Set();
    let at = byOld.get(sym);
    while (at && !seen.has(at.old)) {
      seen.add(at.old);
      out.push(at);
      at = byOld.get(at.new);
    }
    return out;
  };
  /* A ROOT IS AN OLD SYMBOL NO CHANGE LEADS TO: the note on the current name
     is written once per root, with the whole chain (LTI, then LTIM), not
     once per link. */
  const isNew = new Set(changes.map((/** @type {any} */ c) => c.new));
  for (const c of changes) {
    const chain = chainFrom(c.old);
    const last = chain[chain.length - 1];
    if (!last) continue;
    const current = last.new;
    if (!known.has(compactOf(c.old))) {
      known.set(compactOf(c.old), `a symbol NSE renamed ${c.new} effective ${dayText(c.effective)}${current !== c.new ? `, now ${current}` : ''}`);
    }
    /* "formerly X until <day>" ON THE CURRENT NAME: every step of the chain
       from this root, each with the last day it was in force. */
    if (held(current) && !isNew.has(c.old)) {
      const steps = chain.map((s) => `${s.old} until ${dayText(dayBefore(s.effective))}`).join(', then ');
      const line = `formerly ${steps} (NSE symbolchange.csv; ${current} from ${dayText(last.effective)})`;
      note(current, line);
      for (const s of chain) {
        rows.push({ compact: compactOf(s.old), name: s.old, target: current, via: 'formerly', note: line });
      }
    }
    /* AN OLD SYMBOL THE STORE HOLDS UNDER ITS OWN KEY (GDFL's TATAMOTORS and
       L&TFH options) is its own group, found by its later names too, and says
       what it became. Its rows are never stitched onto the new key's
       (D-3331, D-3333). */
    if (held(c.old)) {
      const line = `renamed ${c.new} effective ${dayText(c.effective)}${current !== c.new ? `, now ${current}` : ''} (NSE symbolchange.csv); held here under the old symbol`;
      note(c.old, line);
      rows.push({ compact: compactOf(current), name: current, target: c.old, via: 'renamed', note: line });
      if (c.new !== current) rows.push({ compact: compactOf(c.new), name: c.new, target: c.old, via: 'renamed', note: line });
    }
  }
  for (const n of Array.isArray(aliases?.not_aliased) ? aliases.not_aliased : []) {
    if (held(n?.symbol) && typeof n.why === 'string') note(n.symbol, n.why);
  }
  return {
    find: buildFind(rows, (/** @type {NameRow} */ r) => r.compact),
    rows,
    notes,
    known,
    why: aliases ? null : aliasWhy ?? 'the alias table has not been read'
  };
}

/* ======================================================================
   THE SEARCH
   ====================================================================== */

/**
 * @typedef {object} Hit
 * @property {string} id stable within one answer
 * @property {'instrument' | 'expiry' | 'contract'} kind
 * @property {string} u the underlying (store spelling)
 * @property {string} seg the segment key it opens (`SEGMENT_KEYS`), `''` = the page's own seed
 * @property {string} expiry ISO, `''` for an instrument
 * @property {number | null} strike paisa
 * @property {'CE' | 'PE' | 'FUT' | null} side
 * @property {string} instrument the store key behind a contract, `''` otherwise
 * @property {number} months instrument-months behind it
 * @property {string} form the spelling the text was read in (`''` for a name match)
 * @property {string} label what the row says
 * @property {string} detail the counted clause under it
 * @property {string} title the whole sentence, for `title`
 * @property {number} rank lower draws first
 * @property {NameRow | null} match an instrument hit's matched name
 * @property {ExpiryNode | null} node an expiry or contract hit's expiry
 */
/**
 * @typedef {object} Group
 * @property {string} u
 * @property {string[]} notes
 * @property {Hit[]} hits drawn
 * @property {number} total counted for this group
 * @property {number} rank
 * @property {number} weight instrument-months behind its hits, the tie-break
 */
/**
 * @typedef {object} Answer
 * @property {string} text normalised
 * @property {Group[]} groups drawn
 * @property {number} total hits counted (a lower bound when `capped`)
 * @property {number} drawn hits drawn
 * @property {boolean} capped the count stopped at `MAX_COUNT`
 * @property {number} hiddenGroups groups counted and not drawn
 * @property {string[]} forms vendor forms a drawn hit was read in
 * @property {string | null} why when nothing matched: what was understood and why it is not held
 */

/** @param {string} seg */
export const segmentName = (seg) => SEG_VIEW.find((s) => s.key === seg)?.name ?? seg;

/**
 * Name matches for a compact text: every name row containing it, with a
 * rank — 0 the whole name, 1 a prefix, 2 inside.
 *
 * @param {Names} names
 * @param {string} compact
 * @returns {Map<string, {rank: number, row: NameRow}>} underlying → best match
 */
export function nameMatches(names, compact) {
  /** @type {Map<string, {rank: number, row: NameRow}>} */
  const out = new Map();
  if (compact === '') return out;
  for (const row of probeFind(names.find, names.rows, compact)) {
    const rank = row.compact === compact ? 0 : row.compact.startsWith(compact) ? 1 : 2;
    const was = out.get(row.target);
    if (!was || rank < was.rank || (rank === was.rank && row.via === 'key' && was.row.via !== 'key')) {
      out.set(row.target, { rank, row });
    }
  }
  return out;
}

/**
 * The underlyings a typed NAME inside a contract phrase leads to: the exact
 * names first; only when none is exact, the names it begins. A contract is
 * never looked up under a name that merely contains the text — `IT 11500 CE`
 * is not a question about NIFTY.
 *
 * @param {Names} names
 * @param {string} name
 * @returns {Map<string, {rank: number, row: NameRow}>}
 */
function underlyingsNamed(names, name) {
  const all = nameMatches(names, compactOf(name));
  const exact = new Map([...all].filter(([, m]) => m.rank === 0));
  if (exact.size > 0) return exact;
  return new Map([...all].filter(([, m]) => m.rank === 1));
}

/**
 * The expiries of `u` a reading names.
 *
 * @param {Under} u
 * @param {import('./contract-parse.js').Reading} r
 * @returns {string[]}
 */
function expiriesOf(u, r) {
  if (r.expiry) return u.expiries.has(r.expiry) ? [r.expiry] : [];
  if (r.month) {
    const inMonth = u.expiryList.filter((e) => e.startsWith(r.month + '-'));
    /* A VENDOR MONTHLY NAME NAMES THE MONTH'S CONTRACT, and the store files
       it under its settled expiry (D-2806 item 3): the underlying's last
       stored expiry of that month, the one WP-50's `expiryTags` tags
       `monthly`. */
    return r.monthly ? inMonth.slice(-1) : inMonth;
  }
  if (r.dayMonth) return u.expiryList.filter((e) => e.slice(5) === r.dayMonth);
  return u.expiryList;
}

/**
 * Search the catalog for `text`.
 *
 * @param {Catalog} cat
 * @param {Names} names
 * @param {string} text what was typed
 * @param {{today?: string, archiveTo?: string | null, maxGroups?: number, perGroup?: number,
 *   maxCount?: number}} [options]
 * @returns {Answer}
 */
export function search(cat, names, text, options = {}) {
  const q = parseQuery(text);
  const today = options.today ?? '';
  const archiveTo = options.archiveTo ?? null;
  const maxGroups = options.maxGroups ?? MAX_GROUPS;
  const perGroup = options.perGroup ?? HITS_PER_GROUP;
  const maxCount = options.maxCount ?? MAX_COUNT;
  /** @type {Map<string, {u: string, hits: Map<string, Hit>, rank: number}>} */
  const groups = new Map();
  let total = 0;
  let capped = false;

  /** @param {string} u @param {number} rank */
  const groupOf = (u, rank) => {
    let g = groups.get(u);
    if (!g) groups.set(u, (g = { u, hits: new Map(), rank }));
    if (rank < g.rank) g.rank = rank;
    return g;
  };
  /** @param {Hit} h */
  const add = (h) => {
    const g = groupOf(h.u, h.rank);
    if (g.hits.has(h.id)) {
      const was = /** @type {Hit} */ (g.hits.get(h.id));
      if (h.rank < was.rank) g.hits.set(h.id, h);
      return true;
    }
    if (total >= maxCount) {
      capped = true;
      return false;
    }
    g.hits.set(h.id, h);
    total += 1;
    return true;
  };

  /* ---- 1. THE TEXT AS A NAME ---------------------------------------- */
  for (const [u, m] of nameMatches(names, q.compact)) {
    const under = cat.unders.get(u);
    if (!under) continue;
    if (!add(lean('instrument', `${u}|i`, u, '', '', null, null, under.spot ?? '', under.spotMonths + under.contractMonths, '', m.rank * 10, m.row, null))) break;
  }

  /* ---- 2. THE TEXT AS A CONTRACT ------------------------------------ */
  for (const r of q.readings) {
    /** @type {Array<[string, number]>} underlying and the rank its name earned */
    let targets;
    if (r.name) {
      targets = [...underlyingsNamed(names, r.name)].map(([u, m]) => [u, m.rank]);
    } else if (r.strike !== null) {
      targets = [...(cat.byStrike.get(r.strike)?.keys() ?? [])].map((u) => [u, 1]);
    } else if (r.expiry) {
      targets = (cat.byExpiry.get(r.expiry) ?? []).map((u) => [u, 1]);
    } else if (r.month) {
      targets = [...new Set((cat.byMonth.get(r.month) ?? []).map(([u]) => u))].map((u) => [u, 1]);
    } else if (r.dayMonth) {
      const us = new Set();
      for (const e of cat.expiries) if (e.slice(5) === r.dayMonth) for (const u of cat.byExpiry.get(e) ?? []) us.add(u);
      targets = [...us].map((u) => [u, 1]);
    } else {
      continue;
    }
    for (const [key, nameRank] of targets) {
      if (capped) break;
      const u = cat.unders.get(key);
      if (!u) continue;
      /* WHICH EXPIRIES: a strike with no date named reads the strike's own
         list (one probe), never every expiry of the underlying. */
      const exps =
        r.strike !== null && !r.expiry && !r.month && !r.dayMonth
          ? (cat.byStrike.get(r.strike)?.get(key) ?? [])
          : expiriesOf(u, r);
      /* NEWEST FIRST, the page's own default order (D-3337 item 2). */
      for (let i = exps.length - 1; i >= 0 && !capped; i -= 1) {
        const node = u.expiries.get(exps[i]);
        if (!node) continue;
        /* A MONTHLY-FORM READING RANKS AFTER A DAY-FORM ONE. One ticker can
           read both ways and both can be held: `NIFTY20JUN1912500CE` is
           NIFTY 20 Jun 2019 12,500 CE by the day form, and by the monthly
           form the June 2020 contract at 19,12,500 — one of the three
           impossible NIFTY strikes D-3331 records, still stored. D-2806 item
           2: "their day reading wins over a monthly one at the same split",
           so the day reading is drawn first and the other still shown. */
        const rank = nameRank * 10 + (r.name ? 0 : 5) + (r.monthly ? 2 : 0);
        if (r.side === 'FUT') {
          if (node.fut) addContract(add, u.key, node, null, 'FUT', node.fut, r.form, rank + 1, today, archiveTo);
          continue;
        }
        if (r.strike !== null) {
          const s = node.strikes.get(r.strike);
          if (!s) continue;
          for (const side of /** @type {Array<'CE' | 'PE'>} */ (['CE', 'PE'])) {
            if (r.side && r.side !== side) continue;
            const leaf = s[side];
            if (leaf) addContract(add, u.key, node, r.strike, side, leaf, r.form, rank + 1, today, archiveTo);
          }
          continue;
        }
        /* NO STRIKE: THE EXPIRY ITSELF IS THE HIT, and a side named with it
           (`NIFTY 28 Mar 2019 CE`) goes with it. */
        addExpiry(add, u.key, node, r.side === 'CE' || r.side === 'PE' ? r.side : null, rank + 2, today, archiveTo, r.form);
      }
    }
  }

  /* ---- 3. GROUP, ORDER, CUT TO THE BUDGET ---------------------------
     A GROUP IS ONE UNDERLYING. Groups order by their best hit's rank (an
     exact name or a held contract under an exact name first), then by the
     instrument-months behind their hits — so `11500 CE` with no name opens
     on the underlying that holds most of that strike, not on the alphabet —
     then by name. Inside a group: the instrument, then expiries, then
     contracts, newest expiry first (D-3337 item 2), strikes ascending, CE
     before PE. */
  const ordered = [...groups.values()]
    .map((g) => {
      const hits = [...g.hits.values()].sort(
        (a, b) =>
          a.rank - b.rank ||
          kindOrder(a) - kindOrder(b) ||
          (a.expiry < b.expiry ? 1 : a.expiry > b.expiry ? -1 : 0) ||
          (a.strike ?? 0) - (b.strike ?? 0) ||
          String(a.side).localeCompare(String(b.side))
      );
      let weight = 0;
      for (const h of hits) weight += h.months;
      return { u: g.u, hits, rank: g.rank, weight };
    })
    .sort((a, b) => a.rank - b.rank || b.weight - a.weight || (a.u < b.u ? -1 : a.u > b.u ? 1 : 0));
  /** @type {Group[]} */
  const drawn = [];
  let drawnHits = 0;
  const forms = new Set();
  for (const g of ordered.slice(0, maxGroups)) {
    /* ONLY A DRAWN HIT IS WORDED: a strike alone can reach thousands of
       contracts, and their labels (a day name each) are the cost of the
       answer, so the counted rest stay as numbers. */
    const hits = g.hits.slice(0, perGroup).map((h) => dress(h, cat));
    drawnHits += hits.length;
    for (const h of hits) if (h.form) forms.add(h.form);
    drawn.push({ u: g.u, notes: names.notes.get(g.u) ?? [], hits, total: g.hits.length, rank: g.rank, weight: g.weight });
  }
  return {
    text: q.text,
    groups: drawn,
    total,
    drawn: drawnHits,
    capped,
    hiddenGroups: Math.max(0, ordered.length - maxGroups),
    forms: [...forms],
    why: total === 0 ? whyNothing(cat, names, q) : null
  };
}

/** @param {Hit} h */
const kindOrder = (h) => (h.kind === 'instrument' ? 0 : h.kind === 'expiry' ? 1 : 2);

/**
 * A hit as it is counted: no words yet (`dress` adds them to the drawn few).
 *
 * @param {Hit['kind']} kind
 * @param {string} id
 * @param {string} u
 * @param {string} seg
 * @param {string} expiry
 * @param {number | null} strike
 * @param {Hit['side']} side
 * @param {string} instrument
 * @param {number} months
 * @param {string} form
 * @param {number} rank
 * @param {NameRow | null} match
 * @param {ExpiryNode | null} node
 * @returns {Hit}
 */
function lean(kind, id, u, seg, expiry, strike, side, instrument, months, form, rank, match, node) {
  return { id, kind, u, seg, expiry, strike, side, instrument, months, form, label: '', detail: '', title: '', rank, match, node };
}

/**
 * @param {(h: Hit) => boolean} add
 * @param {string} u
 * @param {ExpiryNode} node
 * @param {number | null} strike
 * @param {'CE' | 'PE' | 'FUT'} side
 * @param {Leaf} leaf
 * @param {string} form
 * @param {number} rank
 * @param {string} today
 * @param {string | null} archiveTo
 */
function addContract(add, u, node, strike, side, leaf, form, rank, today, archiveTo) {
  const seg = segmentOf({ expiry: node.iso, instrument: leaf.instrument }, today, archiveTo);
  return add(lean('contract', `${u}|${node.iso}|${strike ?? 'F'}|${side}`, u, seg, node.iso, strike, side, leaf.instrument, leaf.months, form, rank, null, node));
}

/**
 * @param {(h: Hit) => boolean} add
 * @param {string} u
 * @param {ExpiryNode} node
 * @param {'CE' | 'PE' | null} side
 * @param {number} rank
 * @param {string} today
 * @param {string | null} archiveTo
 * @param {string} form
 */
function addExpiry(add, u, node, side, rank, today, archiveTo, form) {
  const seg = segmentOf({ expiry: node.iso, instrument: node.strikes.size > 0 ? 'contract-CE' : 'contract-FUT' }, today, archiveTo);
  return add(lean('expiry', `${u}|${node.iso}|e|${side ?? ''}`, u, seg, node.iso, null, side, '', node.months, form, rank, null, node));
}

/** ISO day → `DD Mon YYYY`, remembered: an answer names the same few expiries again and again. */
const dayMemo = new Map();
/** @param {string} iso */
function dayOnce(iso) {
  let t = dayMemo.get(iso);
  if (t === undefined) {
    t = dayText(iso);
    if (dayMemo.size < 20_000) dayMemo.set(iso, t);
  }
  return t;
}

/**
 * The words of one DRAWN hit: what the row says, the counted clause under it
 * and the whole sentence for its `title`.
 *
 * @param {Hit} h
 * @param {Catalog} cat
 * @returns {Hit}
 */
function dress(h, cat) {
  if (h.kind === 'instrument') {
    const under = /** @type {Under} */ (cat.unders.get(h.u));
    const m = h.match;
    const parts = [];
    /* INSTRUMENT-MONTHS, THE PAGE'S OWN WORD: one census row is one month at
       one rung, so Zerodha NIFTY's 981 are 141 months over nine rungs. */
    if (under.spotMonths > 0) {
      parts.push(`spot: ${fmt(under.spotMonths)} instrument-month(s) at ${[...under.spotTfs].sort(tfCmp).join(', ')}`);
    }
    if (under.contractMonths > 0) {
      parts.push(`contracts: ${fmt(under.contractMonths)} instrument-month(s) over ${fmt(under.expiryList.length)} expir${under.expiryList.length === 1 ? 'y' : 'ies'}`);
    }
    const how = m && m.via !== 'key' ? ` — matched “${m.name}”` : '';
    return {
      ...h,
      label: h.u,
      detail: `${parts.join(' · ') || 'nothing stored'}${how}`,
      title: `Open ${h.u}${under.spotMonths > 0 ? ' on Spot' : ' on the first segment it holds'}.${how && m ? ` The text matched its name “${m.name}”: ${m.note || 'its store key'}.` : ''}`
    };
  }
  const node = /** @type {ExpiryNode} */ (h.node);
  const day = dayOnce(h.expiry);
  const weekday = WEEKDAYS[weekdayOf(h.expiry)];
  if (h.kind === 'contract') {
    const what = h.strike === null ? 'FUT' : `${strikeText(h.strike)} ${h.side}`;
    return {
      ...h,
      label: `${h.u} ${day} ${what}`,
      detail: `${segmentName(h.seg)} · ${fmt(h.months)} instrument-month(s) held`,
      title: `Open ${h.instrument}: ${segmentName(h.seg)}, expiry ${day} (${weekday}), ${what}. ${fmt(h.months)} instrument-month(s) held. Text read as ${h.form}.`
    };
  }
  let contracts = node.fut ? 1 : 0;
  for (const s of node.strikes.values()) contracts += (s.CE ? 1 : 0) + (s.PE ? 1 : 0);
  return {
    ...h,
    label: `${h.u} ${day}${h.side ? ` ${h.side}` : ''}`,
    detail: `${segmentName(h.seg)} · ${fmt(contracts)} contract(s)${node.strikes.size > 0 ? ` over ${fmt(node.strikes.size)} strike(s)` : ''} · ${fmt(h.months)} instrument-month(s)`,
    title: `Open ${h.u}'s ${segmentName(h.seg)} for the expiry ${day} (${weekday})${h.side ? `, ${h.side} only` : ''}. Text read as ${h.form}.`
  };
}

/**
 * WHY NOTHING MATCHED, said as what the text was understood to mean and
 * which part of it the store does not hold — never a bare "no results".
 *
 * @param {Catalog} cat
 * @param {Names} names
 * @param {{text: string, compact: string, readings: import('./contract-parse.js').Reading[]}} q
 * @returns {string}
 */
function whyNothing(cat, names, q) {
  if (q.text === '') return '';
  const alias = names.why ? ` The alias table is not in force (${names.why}), so index names and old symbols were not searched.` : '';
  const none =
    cat.contracts === 0 && q.readings.some((r) => r.expiry || r.month || r.dayMonth || r.strike !== null || r.side)
      ? ' This feed\'s store holds no future or option at all: every instrument-month in it is a spot series.'
      : '';
  /* A NAME THE ALIAS TABLE KNOWS AND THIS FEED DOES NOT HOLD is said as
     that, before any contract reading of it (`NIFTY FPI 150` is an index's
     name, not NIFTY FPI at strike 150). */
  const what = names.known.get(q.compact);
  if (what) return `${q.text} is ${what}, and this feed's store holds nothing under any of its names.`;
  for (const r of q.readings) {
    const parts = [];
    if (r.name) parts.push(r.name);
    if (r.expiry) parts.push(`expiry ${dayText(r.expiry)}`);
    if (r.month) parts.push(`${r.monthly ? 'the monthly contract of ' : 'an expiry in '}${monthLabel(r.month)}`);
    if (r.dayMonth) parts.push(`an expiry on ${r.dayMonth}`);
    if (r.strike !== null) parts.push(`strike ${strikeText(r.strike)}`);
    if (r.side) parts.push(r.side);
    const named = r.name ? underlyingsNamed(names, r.name) : null;
    const where =
      named && named.size === 0
        ? `no underlying this feed holds is called ${r.name}`
        : named
          ? `${[...named.keys()].join(', ')} ${named.size === 1 ? 'is' : 'are'} held, but not with ${parts.slice(1).join(' ') || 'that contract'}`
          : 'this feed holds no such contract';
    return `Read as ${parts.join(' ')} (${r.form}): ${where}.${none}${alias}`;
  }
  return `Nothing this feed holds is called “${q.text}”, and the text is not a contract spelling this box reads (a vendor ticker, a name with an expiry, strike or side, or a date).${alias}`;
}

/* ======================================================================
   THE SELECTION A HIT STANDS FOR
   ====================================================================== */

/**
 * The rung values one hit sets, for WP-50's selection (`selection.svelte.js`)
 * to take in one step. Pure: the caller assigns them.
 *
 * - `filter` is the underlying (an exact key, D-3337 item 1).
 * - `kind` is the hit's segment; an instrument hit leaves it `''`, so the
 *   page's own seed opens Spot where the underlying has spot and its first
 *   held segment where it does not.
 * - `timeframe` keeps the rung on screen where the target holds it, else `''`
 *   so the seed takes the finest held rung (D-3337 item 2).
 * - `expiry`, `strikes`, `side` are the contract; `strikes` is a strike's
 *   paisa as a string, the key the strike rung holds.
 * - `universe` returns to Everything when the chosen universe does not hold
 *   the underlying, and `oow` shows the months outside the operator window
 *   when that is the only place the target is held (D-3338) — each named in
 *   `notes`, so neither change is silent.
 *
 * @param {Hit} hit
 * @param {Catalog} cat
 * @param {{timeframe: string, universe: string, universeNames: Set<string> | null, oow: boolean,
 *   universeLabel?: string}} now
 */
export function selectionOf(hit, cat, now) {
  const u = cat.unders.get(hit.u);
  /** @type {string[]} */
  const notes = [];
  const spotFirst = hit.kind === 'instrument' && (u?.spotMonths ?? 0) > 0;
  const tfs = hit.kind === 'instrument' ? (spotFirst ? u?.spotTfs : u?.contractTfs) : u?.contractTfs;
  const timeframe = now.timeframe && tfs?.has(now.timeframe) ? now.timeframe : '';
  let universe = now.universe;
  if (now.universeNames !== null && !now.universeNames.has(hit.u)) {
    universe = '';
    notes.push(`${hit.u} is not in ${now.universeLabel ?? 'the chosen universe'}, so the universe returned to Everything`);
  }
  let oow = now.oow;
  if (!oow && spotFirst && u && u.spotInWindow === 0 && u.spotMonths > 0) {
    oow = true;
    notes.push(`every ${hit.u} spot month lies outside your window, so months outside it are shown (D-3338)`);
  }
  return {
    filter: hit.u,
    kind: hit.kind === 'instrument' ? '' : hit.seg,
    timeframe,
    expiry: hit.expiry,
    strikes: hit.strike === null ? new Set() : new Set([String(hit.strike)]),
    side: hit.side === 'CE' || hit.side === 'PE' ? hit.side : '',
    universe,
    oow,
    notes
  };
}

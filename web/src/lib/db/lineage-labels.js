/**
 * /db — WHERE A RE-KEYED CONTRACT'S SERIES CONTINUES, AS WORDS (D-3331,
 * WP-12c; AH-03, g-ui-dom-vs-json-2).
 *
 * The GDFL re-key files a contract under the key the current decoder gives
 * it. `/bars/window.json` says so on every page it answers — `superseded_by`
 * (`[{key, by, kind, recorded}]`, the asked series the decoder files under
 * another key) and `supersedes` (`[{key, of, kind, bars, blake3}]`, the keys
 * whose bars were copied into an asked series) — and `/dayflags.json` sends
 * the lineage ledger's records for one key (`lineage.records`: `superseded_by`,
 * `supersedes`, `retired`, `continues_as`, `renamed_from`, `stitched`,
 * `stopped`). /db read none of the re-key's fields, so a contract split
 * across two keys was explained as "it may not have traded, or the census is
 * behind", and a key the census withdrew was explained by the Side and
 * Strike filters the page had derived from that very key.
 *
 * This module is the ONE place those fields become words; every surface
 * (the expiry note, the empty grid, the stranded-rung line, the day band's
 * lineage lines) calls it, so one record reads the same everywhere.
 *
 * Cost: every word function here is O(records) over the records one answer
 * carries for the asked series — a handful per key. `contractKeyOf` is ONE
 * Map probe: the exchange it needs comes from `fnoExchanges`, a fold over
 * the census that the page runs ONCE PER CENSUS LOAD (a `$derived` of
 * `db.deco` alone), never per reactive step (AH-03 repair 1: the first cut
 * walked the census on every recompute, 134-280 ms on GDFL's 2,098,428 rows
 * for an old-name or absent underlying). `askLineage` asks the server AT
 * MOST ONCE per (feed, key) per census generation; a failed read is not
 * kept, so the next ask is a new request.
 *
 * Owner: AH-03.
 */

import { dbHref } from '../urlstate.js';
import { parseKey } from '../instrument.js';
import { ask } from '../ask.js';
import { refusalFrom } from '../refusal.js';

/**
 * The page's JSON reader for the lineage request: `ask` (the page's one
 * request ceiling), and a refusal read as the server's own sentence.
 *
 * @param {string} url
 */
export async function readLineageJson(url) {
  const response = await ask(url, { cache: 'no-store' });
  if (!response.ok) throw new Error(await refusalFrom('/dayflags.json', response));
  return response.json();
}

/**
 * Why the re-key moved a contract's expiry, as the lineage ledger spells it
 * (`pull::gdfl_lineage::SupersededKind`), in words. A word this page has no
 * entry for is printed as the ledger spelled it, never dropped.
 */
const KIND_WORDS = new Map([
  ['vendor_redated', 'the vendor re-dated its expiry'],
  ['closed_day', 'its stated expiry is a closed day']
]);

/**
 * Why a key is retired (`pull::gdfl_lineage::RetiredWhy`), in words.
 */
const RETIRED_WORDS = new Map([
  ['strike_implausible', 'its strike is implausible for the underlying'],
  ['expiry_unsettled', 'its expiry is not a settled one']
]);

/** @param {unknown} kind */
export function kindWord(kind) {
  const k = String(kind ?? '');
  return KIND_WORDS.get(k) ?? (k ? k.replace(/_/g, ' ') : 'a reason the ledger does not name');
}

/** @param {unknown} n */
function count(n) {
  return Number.isSafeInteger(n) ? Number(n).toLocaleString('en-IN') : null;
}

/**
 * The /db link to one key at the finest rung of its feed (the rung the
 * contract's bars are stored at). `null` for a value that is not a key.
 *
 * @param {string} feed
 * @param {unknown} key
 * @param {string} [tf]
 */
export function keyHref(feed, key, tf = '') {
  if (typeof key !== 'string' || parseKey(key).why !== null) return null;
  return dbHref({ feed, key, tf });
}

/**
 * The re-key's two lists on one `/bars/window.json` answer, each entry
 * checked for shape; an entry that is not one is counted, never dropped in
 * silence. `null` lists (every series not re-keyed) read as empty.
 *
 * @param {any} answer one `/bars/window.json` body
 * @returns {{ by: { key: string, by: string, kind: string, recorded: boolean | null }[],
 *   of: { key: string, of: string, kind: string, bars: number | null }[], malformed: number }}
 */
export function supersessionOf(answer) {
  /** @type {{ key: string, by: string, kind: string, recorded: boolean | null }[]} */
  const by = [];
  /** @type {{ key: string, of: string, kind: string, bars: number | null }[]} */
  const of = [];
  // A LIST THAT IS NOT A LIST IS MALFORMED, not empty: `null` and absent are
  // the server's "not re-keyed"; anything else is one entry the page could
  // not read (the second agent's follow-up, AH-03 repair 1).
  let malformed = 0;
  for (const list of [answer?.superseded_by, answer?.supersedes]) if (list !== null && list !== undefined && !Array.isArray(list)) malformed++;
  for (const s of Array.isArray(answer?.superseded_by) ? answer.superseded_by : []) {
    if (typeof s?.key === 'string' && typeof s?.by === 'string') {
      by.push({ key: s.key, by: s.by, kind: String(s.kind ?? ''), recorded: typeof s.recorded === 'boolean' ? s.recorded : null });
    } else malformed++;
  }
  for (const s of Array.isArray(answer?.supersedes) ? answer.supersedes : []) {
    if (typeof s?.key === 'string' && typeof s?.of === 'string') {
      of.push({ key: s.key, of: s.of, kind: String(s.kind ?? ''), bars: Number.isSafeInteger(s.bars) ? s.bars : null });
    } else malformed++;
  }
  return { by, of, malformed };
}

/**
 * "superseded by <key> (<why>, recorded yes/no)". `recorded: false` is a key
 * the decoder files elsewhere while the re-key that moves its bars has not
 * run (`cli gdfl-rekey --apply`, WP-80), so the two keys each hold part of
 * the contract's life; `true` is a move the lineage ledger recorded.
 *
 * @param {{ by: string, kind: string, recorded: boolean | null }} s
 */
export function supersededWords(s) {
  return `superseded by ${s.by} (${supersededWhy(s)})`;
}

/**
 * The parenthesis of `supersededWords` alone — why, and whether recorded —
 * for a surface that draws the successor as a link.
 *
 * @param {{ kind: string, recorded: boolean | null }} s
 */
export function supersededWhy(s) {
  const rec =
    s.recorded === true
      ? 'recorded yes: the re-key copied its bars there'
      : s.recorded === false
        ? 'recorded no: the re-key has not moved its bars, so each key holds part of its life'
        : 'whether the re-key recorded it is not said';
  return `${kindWord(s.kind)}; ${rec}`;
}

/**
 * "continues <old key>: N bars" — on the key a copy landed in.
 *
 * @param {{ of: string, kind: string, bars: number | null }} s
 */
export function continuesWords(s) {
  const n = count(s.bars);
  return `continues ${s.of}${n === null ? '' : `: ${n} bar(s) copied from it`} (${kindWord(s.kind)})`;
}

/** The lineage kinds whose record names another key (`key`): one without
 *  it is malformed, never "superseded by undefined". */
const KEYED_KINDS = new Set(['continues_as', 'renamed_from', 'superseded_by', 'supersedes']);

/**
 * One lineage-ledger record (`/dayflags.json` `lineage.records`) as the band
 * says it, or `null` for a kind this page has no words for (counted by the
 * caller). Every kind the server files under a key is here (D-3331, D-3355).
 * A record of a kind that names another key and carries none is `null` too,
 * and `lineageLines` counts it as malformed rather than unknown.
 *
 * @param {any} r
 * @returns {string | null}
 */
export function lineageRecordLine(r) {
  if (KEYED_KINDS.has(r?.kind) && (typeof r.key !== 'string' || r.key === '')) return null;
  switch (r?.kind) {
    case 'continues_as':
      return `renamed at the ${r.ex} ex-date, continues as ${r.key} (factor ${r.factor})`;
    case 'renamed_from':
      return `renamed at the ${r.ex} ex-date from ${r.key} (factor ${r.factor})`;
    case 'stitched':
      return `holds two exchange contracts: its name was reused at the ${r.ex} ex-date${r.after ? `, then holding the contract that was ${r.after}` : ''}`;
    case 'superseded_by': {
      const n = count(r.bars);
      return `superseded by ${r.key} (${kindWord(r.why)}; recorded yes: the re-key copied ${n === null ? 'its' : `its ${n}`} bar(s)${Number.isSafeInteger(r.months) ? ` in ${r.months} month(s)` : ''} there)`;
    }
    case 'supersedes': {
      const n = count(r.bars);
      return `continues ${r.key}${n === null ? '' : `: ${n} bar(s) copied from it`}${Number.isSafeInteger(r.months) ? ` in ${r.months} month(s)` : ''} (${kindWord(r.why)})`;
    }
    case 'retired': {
      const why = RETIRED_WORDS.get(String(r.why ?? '')) ?? String(r.why ?? 'a reason the ledger does not name').replace(/_/g, ' ');
      const n = count(r.bars);
      return `retired: the decoder refuses every day of it — ${why}${n === null ? '' : ` (${n} bar(s) stored)`}${typeof r.refusal === 'string' && r.refusal ? `; ${r.refusal}` : ''}`;
    }
    case 'stopped':
      return `stopped: the exchange contract under this key ended the day before the ${r.ex} ex-date, and no key continues it`;
    default:
      return null;
  }
}

/**
 * Every record of one `/dayflags.json` `lineage` section as lines, plus how
 * many records carried a kind this page has no words for (said, never
 * dropped), and the successor the ledger names (the first `superseded_by`
 * or `continues_as` key), for a link.
 *
 * @param {any} records `lineage.records`
 * @returns {{ lines: string[], unknown: number, malformed: number, successor: string | null, retired: boolean }}
 */
export function lineageLines(records) {
  /** @type {string[]} */
  const lines = [];
  let unknown = 0;
  let malformed = 0;
  /** @type {string | null} */
  let successor = null;
  let retired = false;
  for (const r of Array.isArray(records) ? records : []) {
    const line = lineageRecordLine(r);
    if (line !== null) lines.push(line);
    else if (KEYED_KINDS.has(r?.kind)) malformed++;
    else unknown++;
    if (successor === null && (r?.kind === 'superseded_by' || r?.kind === 'continues_as') && typeof r.key === 'string') successor = r.key;
    if (r?.kind === 'retired') retired = true;
  }
  if (unknown > 0) lines.push(`${unknown} lineage record(s) of a kind this page has no words for — read /dayflags.json`);
  if (malformed > 0) lines.push(`${malformed} lineage record(s) that name no key, so this page cannot say where the contract went — read /dayflags.json`);
  return { lines, unknown, malformed, successor, retired };
}

/**
 * WHY A CONTRACT'S STORED LIFE STOPS BEFORE ITS EXPIRY, when the re-key
 * explains it: the clause that replaces "it may not have traded after that
 * day, or the census is behind" for every chosen key the decoder files
 * elsewhere (`superseded_by` on the window answer). At most `most` keys are
 * named, the rest counted. `''` when no asked series is superseded, so the
 * caller keeps its own words.
 *
 * @param {ReturnType<typeof supersessionOf>} s
 * @param {number} [most]
 */
export function lifeCutBySupersession(s, most = 3) {
  if (!s || s.by.length === 0) return '';
  const named = s.by.slice(0, most).map((x) => `${x.key} is ${supersededWords(x)}`);
  const rest = s.by.length - named.length;
  return `the contract continues under another key: ${named.join('; ')}${rest > 0 ? `; and ${rest} more key(s) the same way` : ''}`;
}

/**
 * EVERY UNDERLYING'S F&O EXCHANGE, FOLDED ONCE PER CENSUS LOAD (AH-03
 * repair 1): the exchange of the FIRST F&O row of each underlying, in the
 * census's served order — the row `contractKeyOf` used to search for on
 * every recompute. Read from the decorated row's own fields (`kind`, the
 * key's segment token; `underlying`, parsed once at store-read time), so
 * the fold parses nothing: one pass, one Map write per underlying. A row
 * whose key is not `<EX>-FNO-<underlying>-…` (a malformed key the
 * decoration kept as its own name) adds nothing.
 *
 * The caller holds it in a `$derived` of `db.deco` alone, so it recomputes
 * when a census is loaded and at no other step. O(rows) once per load.
 *
 * @param {Iterable<any> | null | undefined} rows the decorated census rows (`db.deco`)
 * @returns {Map<string, string>} underlying → exchange
 */
export function fnoExchanges(rows) {
  /** @type {Map<string, string>} */
  const out = new Map();
  for (const r of rows ?? []) {
    const u = r?.underlying;
    const ins = r?.instrument;
    if (r?.kind !== 'FNO' || typeof u !== 'string' || typeof ins !== 'string' || out.has(u)) continue;
    const cut = ins.indexOf('-');
    if (cut > 0 && ins.startsWith(`-FNO-${u}-`, cut)) out.set(u, ins.slice(0, cut));
  }
  return out;
}

/**
 * A /db selection's one contract key, from its rungs: the underlying, the
 * expiry, exactly one strike and a side (an option), or the expiry alone
 * with no strike and no side (a future). The exchange is the one the
 * census files this underlying's contracts under (`fnoExchanges`), never a
 * literal. `null` when the rungs name no single contract, or the census
 * holds no F&O row of the underlying to read an exchange from.
 *
 * ONE MAP PROBE, never a census walk: given anything but the folded Map it
 * THROWS, because a caller passing the rows would be the scan this repair
 * removed, and a quiet `null` would hide it. O(1) (at most two strikes
 * are read off the picked set).
 *
 * @param {Map<string, string>} exchanges `fnoExchanges(db.deco)`
 * @param {{ sym: string, expiry: string, strikes: Iterable<string>, side: string }} rungs
 */
export function contractKeyOf(exchanges, { sym, expiry, strikes, side }) {
  if (!(exchanges instanceof Map)) throw new TypeError('contractKeyOf takes fnoExchanges(db.deco), folded once per census load — never the census rows');
  if (!sym || !/^\d{4}-\d{2}-\d{2}$/.test(expiry ?? '')) return null;
  /** @type {string[]} */
  const ks = [];
  for (const k of strikes ?? []) {
    ks.push(String(k));
    if (ks.length > 1) break;
  }
  const exchange = exchanges.get(sym) ?? null;
  if (exchange === null) return null;
  if (ks.length === 1 && /^\d+$/.test(ks[0]) && (side === 'CE' || side === 'PE')) return `${exchange}-FNO-${sym}-${expiry}-${ks[0]}-${side}`;
  if (ks.length === 0 && !side) return `${exchange}-FNO-${sym}-${expiry}-FUT`;
  return null;
}

/**
 * THE ONE LINEAGE REQUEST FOR A KEY THE CENSUS DOES NOT HOLD (AH-03): one
 * `/dayflags.json` page of one day — the contract's expiry — for that key,
 * whose `lineage` section is the ledger's records filed under it. Asked at
 * most once per (feed, key) for the life of the page: the promise is kept,
 * so every surface that needs it (the stranded-rung line, the empty grid)
 * shares one request. At most `LINEAGE_MEMO_MAX` keys are remembered; the
 * oldest is forgotten first.
 *
 * Resolves `{ state: 'read', lines, successor, retired, present, why }`,
 * or `{ state: 'failed', why }` — never rejects, so a failed read is said
 * by the caller rather than swallowed.
 *
 * @param {string} feed
 * @param {string} key
 * @param {(url: string) => Promise<any>} [fetchJson] the page's JSON reader
 */
export function askLineage(feed, key, fetchJson = readLineageJson) {
  const memoKey = `${feed}\u0000${key}`;
  const kept = MEMO.get(memoKey);
  if (kept) return kept;
  const p = parseKey(key);
  const day = p.expiry ?? '';
  const q = new URLSearchParams({ feed, instrument: key, from: day, to: day, order: 'asc' });
  const url = `/dayflags.json?${q}`;
  const asked = Promise.resolve()
    .then(() => fetchJson(url))
    .then((body) => {
      const l = body?.lineage;
      if (!l || typeof l !== 'object') {
        return { state: 'read', url, present: false, lines: [], successor: null, retired: false, why: 'the answer carries no lineage section (not a GDFL contract key)' };
      }
      const got = lineageLines(l.records);
      return { state: 'read', url, present: l.present === true, ...got, why: typeof l.why === 'string' ? l.why : null };
    })
    .catch((/** @type {any} */ why) => {
      // A FAILED READ IS NOT REMEMBERED: the next ask for this key is a new
      // request ("Read the lineage again", a key change, a census reload),
      // never the same failure for the page's life (AH-03 repair 1).
      if (MEMO.get(memoKey) === asked) MEMO.delete(memoKey);
      return { state: 'failed', url, why: String(why?.message ?? why) };
    });
  MEMO.set(memoKey, asked);
  while (MEMO.size > LINEAGE_MEMO_MAX) MEMO.delete(/** @type {string} */ (MEMO.keys().next().value));
  return asked;
}

/** The most withdrawn keys whose lineage answer one page remembers. */
export const LINEAGE_MEMO_MAX = 32;

/** @type {Map<string, Promise<any>>} */
const MEMO = new Map();

/**
 * Forget remembered lineage answers: one (feed, key)'s when both are given
 * ("Read the lineage again"), else every one (a new census generation or
 * epoch: a re-key can have landed between two, so what the ledger said may
 * have changed).
 *
 * @param {string} [feed]
 * @param {string} [key]
 */
export function forgetLineage(feed, key) {
  if (typeof feed === 'string' && typeof key === 'string') MEMO.delete(`${feed}\u0000${key}`);
  else MEMO.clear();
}

/**
 * The sentence for a key the census does not hold, from its lineage answer:
 * what the ledger says happened to it, which REPLACES the stranded-rung
 * blame ("Side is set to PE, which the rungs above it no longer offer") —
 * the reader's filters are the key's own fields, and blaming them names the
 * wrong cause. `null` when the ledger holds no record of the key, so the
 * caller keeps its rung wording; a failed read is said as one.
 *
 * @param {string} key
 * @param {any} got an `askLineage` result, or `null` while it is pending
 * @returns {{ state: 'pending' | 'explained' | 'unexplained' | 'failed', text: string, successor: string | null }}
 */
export function withdrawnWords(key, got) {
  if (!got) return { state: 'pending', text: `${key} is not in the census; reading its lineage before any filter is blamed…`, successor: null };
  if (got.state === 'failed') {
    return { state: 'failed', text: `${key} is not in the census, and its lineage could not be read (${got.why}), so the rungs below are named as stranded without a cause`, successor: null };
  }
  if (got.lines.length === 0) {
    return {
      state: 'unexplained',
      text: `${key} is not in the census, and the lineage ledger files no record under it${got.why ? ` (${got.why})` : ''}`,
      successor: null
    };
  }
  return {
    state: 'explained',
    text: `${key} is not in the census: ${got.lines.join('; ')}. The rungs it set are its own fields, not a filter that hides it.`,
    successor: got.successor
  };
}

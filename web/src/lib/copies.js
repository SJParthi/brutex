/**
 * THE EXTRA-COPIES REGISTER, AS TABLES (CC-5).
 *
 * `GET /disk/copies.json` walks the folders `disk.extra_copy_roots` names and
 * answers, for each, its bytes, the reader that opens it (or "none found"),
 * the receipt that proves it, whether a twin of its bytes is proven, and every
 * name in it that is a second hard link to a live vendor file. It also carries
 * the retirement list: the folders and names the operator may take away by
 * hand, with the server's own text for them. The page adds no command of its
 * own to that list; it shows what the server sent and offers it as a file.
 *
 * This module turns the body into a plain view model, or into one line naming
 * what is wrong with it. Nothing is invented: a field the server did not send
 * is a fault, never a 0, and no folder is ever assumed when the setting is not
 * set. A path that is not UTF-8 arrives in its escaped, quoted form and is
 * passed through as given.
 */

/** The route this panel reads, as the one named line spells it. */
export const ROUTE = 'GET /disk/copies.json';

/** The three states a copy root can be in. */
export const ROOT_STATES = Object.freeze(['walked', 'partial', 'refused']);

/** @param {unknown} n */
const count = (n) => Number.isSafeInteger(n) && /** @type {number} */ (n) >= 0;
/** @param {unknown} s */
const named = (s) => typeof s === 'string' && s !== '';
/** @param {unknown} s */
const textOrNull = (s) => s === null || typeof s === 'string';
/** @param {unknown} o */
const object = (o) => o !== null && typeof o === 'object' && !Array.isArray(o);

/**
 * A byte count with thousands separators and its GiB figure, the same in
 * every locale: `1,073,741,824 B (1.00 GiB)`.
 * @param {number} n
 */
export function formatBytes(n) {
  const grouped = String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return `${grouped} B (${(n / 2 ** 30).toFixed(2)} GiB)`;
}

/**
 * The name the retirement list is saved under.
 * @param {number} walkedAtMicros the walk's `walked_at_micros`
 */
export function retirementFileName(walkedAtMicros) {
  if (!count(walkedAtMicros)) throw new RangeError('walked_at_micros is not a whole, non-negative count of microseconds');
  return `retirement-list-${walkedAtMicros}.txt`;
}

/**
 * The reader cell: the server's "none found", or who reads it and how.
 * @param {any} reader
 * @returns {string | null} null when the value is neither form
 */
function readerWords(reader) {
  if (reader === 'none found') return reader;
  if (!object(reader) || ![reader.kind, reader.path, reader.named_by, reader.how].every(named)) return null;
  return `${reader.kind} at ${reader.path}, named by ${reader.named_by}: ${reader.how}`;
}

/**
 * @typedef {{ path: string, live: string, kind: string, dev: number, ino: number, nlink: number, bytes: number, partial: string }} HazardRow
 * @typedef {{
 *   path: string, state: string, refused: string | null,
 *   files: string, bytes: string, extraBytes: string,
 *   reader: string, proof: string, twin: string, duplicatesLive: string,
 *   inside: string, faults: number, linkedElsewhere: number
 * }} RootRow
 * @typedef {{ kind: string, path: string, namedBy: string, walked: boolean, why: string, files: number }} LiveRow
 * @typedef {{ path: string, bytes: string, files: number, why: string, twin: string, caution: string, hazard: boolean }} RetireRow
 * @typedef {{ fault: string }} Fault
 * @typedef {{ fault: null, state: 'waiting', line: string, degraded: string | null }} Waiting
 * @typedef {{
 *   fault: null, state: 'walked',
 *   walkedAt: string, walkedAtMicros: number, ageS: number, walkMs: string,
 *   fresh: boolean, rewalking: boolean, why: string, note: string,
 *   hazards: HazardRow[], hazardCount: number,
 *   roots: RootRow[], live: LiveRow[], unsetLive: string[],
 *   retirement: RetireRow[], retirementSentence: string, retirementText: string,
 *   extraCopiesBytes: string, whole: boolean, wholeLine: string | null
 * }} Walked
 */

/**
 * @param {any} body the parsed `/disk/copies.json` answer to a 200
 * @returns {Fault | Waiting | Walked}
 */
export function copiesView(body) {
  if (!object(body)) return { fault: '/disk/copies.json did not answer an object.' };
  if (body.state === 'waiting') {
    if (!named(body.why)) return { fault: '/disk/copies.json is waiting and says not why.' };
    if (!textOrNull(body.settings_degraded)) return { fault: '/disk/copies.json carries a settings_degraded that is not text.' };
    return {
      fault: null,
      state: 'waiting',
      // TWO WAITS, ONE LINE: the server waits both when the setting is unset
      // and when its stored value does not read, and its own `why` says which.
      line: `Nothing was walked: the copy-roots setting is not set or does not read, and no folder is ever assumed. ${body.why}`,
      degraded: body.settings_degraded
    };
  }
  if (body.state !== 'walked') return { fault: `/disk/copies.json answered a state this page does not know: ${JSON.stringify(body.state)}.` };

  for (const f of ['walked_at', 'why', 'note', 'retirement_text', 'retirement_sentence']) {
    if (!named(body[f])) return { fault: `/disk/copies.json carries no ${f}.` };
  }
  for (const f of ['walked_at_micros', 'age_ms', 'walk_us', 'extra_copies_bytes', 'hazard_count']) {
    if (!count(body[f])) return { fault: `/disk/copies.json has no count for ${f}.` };
  }
  for (const f of ['fresh', 'whole']) {
    if (typeof body[f] !== 'boolean') return { fault: `/disk/copies.json does not say whether ${f}.` };
  }
  for (const f of ['roots', 'live', 'hazards', 'retirement_list', 'unset_live']) {
    if (!Array.isArray(body[f])) return { fault: `/disk/copies.json carries no ${f} list.` };
  }

  /** @type {HazardRow[]} */
  const hazards = [];
  for (const [i, h] of body.hazards.entries()) {
    if (!object(h) || !named(h.path) || !named(h.live) || !named(h.kind)) return { fault: `/disk/copies.json hazard row ${i} names no path, live file or kind.` };
    const missing = ['dev', 'ino', 'nlink', 'bytes'].find((f) => !count(h[f]));
    if (missing) return { fault: `/disk/copies.json hazard ${h.path} has no count for ${missing}.` };
    if (typeof h.partial !== 'boolean') return { fault: `/disk/copies.json hazard ${h.path} does not say whether it is partial.` };
    hazards.push({ path: h.path, live: h.live, kind: h.kind, dev: h.dev, ino: h.ino, nlink: h.nlink, bytes: h.bytes, partial: h.partial ? 'partial-download name' : '' });
  }
  if (body.hazard_count !== hazards.length) return { fault: `/disk/copies.json counts ${body.hazard_count} hazard(s) and lists ${hazards.length}.` };

  /** @type {RootRow[]} */
  const roots = [];
  for (const [i, r] of body.roots.entries()) {
    if (!object(r) || !named(r.path)) return { fault: `/disk/copies.json root row ${i} names no path.` };
    if (!ROOT_STATES.includes(r.state)) return { fault: `/disk/copies.json root ${r.path} is in no known state.` };
    const refused = r.state === 'refused';
    if (refused && !named(r.refused)) return { fault: `/disk/copies.json root ${r.path} is refused and says not why.` };
    const sizes = ['files', 'bytes', 'extra_bytes'];
    const unmeasured = sizes.find((f) => (refused ? r[f] !== null : !count(r[f])));
    if (unmeasured) return { fault: `/disk/copies.json root ${r.path} has no count for ${unmeasured}.` };
    const reader = readerWords(r.reader);
    if (reader === null) return { fault: `/disk/copies.json root ${r.path} names its reader in neither form.` };
    if (!named(r.proof)) return { fault: `/disk/copies.json root ${r.path} names no proof, not even "none".` };
    if (!object(r.twin) || !named(r.twin.words)) return { fault: `/disk/copies.json root ${r.path} carries no twin words.` };
    if (typeof r.duplicates_live !== 'boolean') return { fault: `/disk/copies.json root ${r.path} does not say whether it duplicates a live file.` };
    if (r.inside !== null && !(object(r.inside) && named(r.inside.kind) && named(r.inside.path))) return { fault: `/disk/copies.json root ${r.path} names what it lies inside in no known form.` };
    if (!Array.isArray(r.faults) || !Array.isArray(r.linked_elsewhere)) return { fault: `/disk/copies.json root ${r.path} carries no faults or linked_elsewhere list.` };
    roots.push({
      path: r.path,
      state: r.state,
      refused: refused ? r.refused : null,
      files: refused ? 'not measured' : String(r.files),
      bytes: refused ? 'not measured' : formatBytes(r.bytes),
      extraBytes: refused ? 'not measured' : formatBytes(r.extra_bytes),
      reader,
      proof: r.proof,
      twin: r.twin.words,
      duplicatesLive: r.duplicates_live ? 'yes' : 'no',
      inside: r.inside === null ? '' : `${r.inside.kind} ${r.inside.path}`,
      faults: r.faults.length,
      linkedElsewhere: r.linked_elsewhere.length
    });
  }

  /** @type {LiveRow[]} */
  const live = [];
  for (const [i, l] of body.live.entries()) {
    if (!object(l) || !named(l.kind) || !named(l.path) || !named(l.named_by)) return { fault: `/disk/copies.json live row ${i} names no kind, path or setting.` };
    if (typeof l.walked !== 'boolean' || !textOrNull(l.why) || !count(l.files)) return { fault: `/disk/copies.json live root ${l.path} does not say whether it was walked, why, or how many files.` };
    live.push({ kind: l.kind, path: l.path, namedBy: l.named_by, walked: l.walked, why: l.why ?? '', files: l.files });
  }

  if (!body.unset_live.every(named)) return { fault: '/disk/copies.json unset_live holds a line that is not text.' };

  /** @type {RetireRow[]} */
  const retirement = [];
  for (const [i, t] of body.retirement_list.entries()) {
    if (!object(t) || !named(t.path) || !named(t.why) || !named(t.twin)) return { fault: `/disk/copies.json retirement row ${i} names no path, why or twin.` };
    if (!count(t.bytes) || !count(t.files)) return { fault: `/disk/copies.json retirement row ${t.path} has no byte or file count.` };
    if (!textOrNull(t.caution) || typeof t.hazard !== 'boolean') return { fault: `/disk/copies.json retirement row ${t.path} carries no caution or hazard flag.` };
    retirement.push({ path: t.path, bytes: formatBytes(t.bytes), files: t.files, why: t.why, twin: t.twin, caution: t.caution ?? '', hazard: t.hazard });
  }

  return {
    fault: null,
    state: 'walked',
    walkedAt: body.walked_at,
    walkedAtMicros: body.walked_at_micros,
    ageS: Math.floor(body.age_ms / 1000),
    walkMs: (body.walk_us / 1000).toFixed(3),
    fresh: body.fresh,
    rewalking: body.rewalking === true,
    why: body.why,
    note: body.note,
    hazards,
    hazardCount: body.hazard_count,
    roots,
    live,
    unsetLive: [...body.unset_live],
    retirement,
    retirementSentence: body.retirement_sentence,
    retirementText: body.retirement_text,
    extraCopiesBytes: formatBytes(body.extra_copies_bytes),
    whole: body.whole,
    wholeLine: body.whole ? null : 'Not every root was walked whole: a refused or partial root is in the roots table, so the extra-copies total counts only what was walked.'
  };
}

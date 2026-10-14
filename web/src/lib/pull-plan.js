/**
 * THE PROJECTION LINE OF A PLAN ANSWER (R4-01; PERF-13 display, DATA-14).
 *
 * The contract R4-07 (`pull::disk_projection`), R4-23 (the Zerodha plan) and
 * R4-29 (the GDFL preflight) serve against: a plan answer MAY carry
 *
 *   projection: { bytes, free_after, floor, basis: "extrapolation", refused }
 *
 * and every page that shows a plan renders it through this one function:
 * `projected <X> (extrapolation); free after <Y>; floor <Z>`, and a refusal by
 * name. No field, no line — a plan that does not project says nothing about
 * disk rather than a zero.
 *
 * THE BASIS IS THE SERVER'S WORD, NEVER RELABELLED. An extrapolation reads
 * "(extrapolation)"; any other basis is shown as sent, and a projection with no
 * basis says so — a number with no basis must not read as a measurement
 * (`CLAUDE.md` §3 rule 6).
 *
 * A MALFORMED PROJECTION IS NAMED, NEVER DROPPED: a field that is present but
 * not a whole non-negative number of bytes is a line saying which field, not a
 * missing line.
 */

const UNITS = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];

/**
 * Bytes in binary units, one decimal from KiB up: `1.5 GiB`. Exact below 1 KiB.
 * @param {number} bytes a whole non-negative number
 */
export function bytesText(bytes) {
  let v = bytes;
  let u = 0;
  while (v >= 1024 && u < UNITS.length - 1) {
    v /= 1024;
    u += 1;
  }
  return u === 0 ? `${bytes} B` : `${v.toFixed(1)} ${UNITS[u]}`;
}

/** @param {unknown} v */
const wholeBytes = (v) => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0;

/**
 * The refusal, in words, when the projection carries one; `null` when it does not.
 * `refused` may be a string (the reason), an object `{ code, why }`, or `true`
 * (the floor: said with the two numbers).
 *
 * @param {unknown} refused
 * @param {string} freeAfter
 * @param {string} floor
 */
function refusalWords(refused, freeAfter, floor) {
  if (refused === null || refused === undefined || refused === false) return null;
  if (typeof refused === 'string') return refused.trim() === '' ? 'refused (no reason sent)' : `refused: ${refused.trim()}`;
  if (refused === true) return `refused: free space after the plan (${freeAfter}) would be below the floor (${floor})`;
  if (typeof refused === 'object') {
    const r = /** @type {{ code?: unknown, why?: unknown }} */ (refused);
    const code = typeof r.code === 'string' && r.code !== '' ? r.code : 'unnamed';
    const why = typeof r.why === 'string' && r.why.trim() !== '' ? `: ${r.why.trim()}` : '';
    return `refused (${code})${why}`;
  }
  return `refused (unreadable reason: ${JSON.stringify(refused).slice(0, 80)})`;
}

/**
 * THE ONE LINE, or `null` when the plan carries no projection.
 *
 * @param {any} plan a plan answer, parsed
 * @returns {{ line: string, refused: boolean, fault: boolean } | null}
 */
export function projectionLine(plan) {
  if (plan === null || typeof plan !== 'object' || !('projection' in plan)) return null;
  const p = plan.projection;
  if (p === null || p === undefined) return null;
  if (typeof p !== 'object' || Array.isArray(p)) {
    return { line: 'The plan sent a projection that is not an object, so no disk figure is shown.', refused: false, fault: true };
  }
  const bad = ['bytes', 'free_after', 'floor'].filter((f) => !wholeBytes(p[f]));
  if (bad.length > 0) {
    const refusal = refusalWords(p.refused, '?', '?');
    return {
      line: `The plan's projection carries no whole byte count in ${bad.join(', ')}, so no disk figure is shown${refusal ? `; ${refusal}` : ''}.`,
      refused: refusal !== null,
      fault: true
    };
  }
  const basis = typeof p.basis === 'string' && p.basis.trim() !== '' ? p.basis.trim() : 'no basis stated';
  const freeAfter = bytesText(p.free_after);
  const floor = bytesText(p.floor);
  const refusal = refusalWords(p.refused, freeAfter, floor);
  const line = `projected ${bytesText(p.bytes)} (${basis}); free after ${freeAfter}; floor ${floor}${refusal ? `; ${refusal}` : ''}`;
  return { line, refused: refusal !== null, fault: false };
}

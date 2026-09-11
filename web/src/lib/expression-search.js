import { decodeMaskWords } from './mask.js';
import { detailRefusal } from './detail-refusal.js';
const max = (1n << 64n) - 1n;
const hex = (/** @type {any} */ x) => typeof x === 'string' && /^[0-9a-f]{64}$/.test(x);
const uint = (/** @type {any} */ x) => typeof x === 'string' && /^(0|[1-9][0-9]*)$/.test(x) && x.length <= 20 && BigInt(x) <= max;
const object = (/** @type {any} */ x) => x !== null && typeof x === 'object' && !Array.isArray(x);
const anchor = (/** @type {any} */ x) => object(x) && uint(x.sequence) && x.sequence !== '0' && hex(x.seal);
const same = (/** @type {any} */ x, /** @type {any} */ y) => x?.sequence === y?.sequence && x?.seal === y?.seal;
/** One page of bounded checkpoint work, which can contain no candidate and still continue.
 * @param {any} selection @param {(url:string)=>Promise<any>} request */
export async function fetchExpressionSearch(selection, request) {
  const { identity, snapshot = null, cursor = null, limit = 8 } = selection;
  if (!hex(identity) || !Number.isInteger(limit) || limit < 1 || limit > 256 || (snapshot !== null && !anchor(snapshot)) || (cursor !== null && (!anchor(cursor) || snapshot === null))) throw new Error('An exact search ID, pinned checkpoint and bounded page are required.');
  const query = new URLSearchParams({ identity, limit: String(limit) });
  if (snapshot) { query.set('snapshot', snapshot.sequence); query.set('snapshot_seal', snapshot.seal); }
  if (cursor) { query.set('cursor', cursor.sequence); query.set('cursor_seal', cursor.seal); }
  const response = await request('/expression-search.json?' + query);
  if (!response?.ok) throw new Error(await detailRefusal(response, 'Search receipts unavailable (HTTP ' + String(response?.status) + ').'));
  const body = await response.json();
  if (!object(body) || body.schema_version !== 1 || body.identity !== identity || body.refusal !== null || !Array.isArray(body.rows)) throw new Error('Search evidence is refused or belongs to another identity.');
  if (body.status === 'missing') {
    if (snapshot !== null || cursor !== null || body.rows.length !== 0 || typeof body.why !== 'string' || !body.why.trim()) throw new Error('Missing search evidence cannot replace a pinned page.');
    return body;
  }
  if (body.status !== 'observed' || body.authority !== 'saved-receipts-and-grammar-transitions' ||
      (body.snapshot !== null && !anchor(body.snapshot)) || (snapshot !== null && !same(snapshot, body.snapshot)) ||
      !['work','candidates','qualifying','signal_rows','interrupted_reservations'].every((key) => uint(body[key])) ||
      BigInt(body.qualifying) > BigInt(body.candidates) || typeof body.writer_observed !== 'boolean' || typeof body.exhausted !== 'boolean' ||
      body.page_complete !== true || body.limit !== limit || !Number.isInteger(body.links) || body.links < 0 || body.links > limit || body.rows.length > body.links ||
      (body.next !== null && !anchor(body.next))) throw new Error('Search snapshot, bounds or counters are invalid.');
  const state = body.exhausted ? 'exhausted-observed' : body.writer_observed ? 'writer-observed' : body.snapshot ? 'paused-or-stopped' : 'awaiting-first-checkpoint';
  if (body.state !== state || (body.snapshot === null && (body.links !== 0 || body.candidates !== '0' || body.next !== null || body.exhausted))) throw new Error('Observed search status disagrees with its checkpoint.');
  const start = cursor ?? body.snapshot;
  if (body.next !== null && (!start || BigInt(body.next.sequence) >= BigInt(start.sequence))) throw new Error('Search continuation loops or moves outside its snapshot.');
  let prior = BigInt(body.candidates) + 1n;
  for (const row of body.rows) {
    if (!object(row) || !uint(row.ordinal) || row.ordinal === '0' || BigInt(row.ordinal) >= prior || !hex(row.identity) || !uint(row.attempt) || row.attempt === '0' ||
        typeof row.expression !== 'string' || row.expression.length === 0 || row.expression.length > 131072 || !decodeMaskWords(row.mask_words).ok ||
        !object(row.signals) || !['evaluated','hits','misses','unknown'].every((key) => uint(row.signals[key])) ||
        BigInt(row.signals.hits) + BigInt(row.signals.misses) + BigInt(row.signals.unknown) !== BigInt(row.signals.evaluated) ||
        (row.capture_digest !== null && !hex(row.capture_digest))) throw new Error('An expression child has invalid identity, order or signal accounting.');
    prior = BigInt(row.ordinal);
  }
  return body;
}

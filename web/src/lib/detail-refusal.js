/** The refusal words a detail body may carry: a route's own `refused`, and the
 * seven words of the one status rule's classes (D-7550, `Class::word` in
 * `crates/api/src/status_rule.rs`, which the reader tests read): `malformed`
 * (400), `not_found` (404, one probe's NotFound), `retired` (410),
 * `out_of_range` (416), `over_bound` (422), `saturated` (429) and `unreadable`
 * (503, what exists could not be read). A result word such as `saved` is never
 * quoted as a refusal. */
export const REFUSAL_STATUSES = Object.freeze([
  'refused', 'malformed', 'not_found', 'retired', 'out_of_range', 'over_bound', 'saturated', 'unreadable'
]);

/** The one not-found answer every identity route gives (D-7550): HTTP 404,
 * `status: "not_found"`, the asked identity, no rows and a bounded sentence.
 * Its sentence, or null for any other answer, so a reader shows "nothing is
 * saved for this identity" only when the server said exactly that.
 * @param {any} response @param {any} body @param {string} identity */
export function notFoundWhy(response, body, identity) {
  if (response?.status !== 404 || body === null || typeof body !== 'object' || Array.isArray(body)) return null;
  if (body.schema_version !== 1 || body.status !== 'not_found' || body.identity !== identity ||
      !Array.isArray(body.rows) || body.rows.length !== 0 ||
      typeof body.refusal !== 'string' || body.refusal.length > 4096 || !body.refusal.trim()) return null;
  return body.refusal.trim();
}

/** Keep bounded server refusal detail visible without accepting result rows.
 * @param {any} response @param {string} fallback */
export async function detailRefusal(response, fallback) {
  try {
    const body = await response?.json?.();
    if (body !== null && typeof body === 'object' && !Array.isArray(body) &&
        body.schema_version === 1 && REFUSAL_STATUSES.includes(body.status) &&
        Array.isArray(body.rows) && body.rows.length === 0 &&
        typeof body.refusal === 'string' && body.refusal.length <= 4096 && body.refusal.trim()) {
      return fallback + ' ' + body.refusal.trim();
    }
  } catch {
    // Proxies and connection failures need not return the application schema.
  }
  return fallback;
}

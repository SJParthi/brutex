/**
 * IS THIS ANSWER A RECEIPT AT ALL?
 *
 * # What a green "OK" used to mean
 *
 * `/ingest` posts to `/pull/spot` and reads the server's own HTML answer rather
 * than reconstructing one. `DOMParser` parses ANY string as HTML — it has no
 * failure mode — so an answer that never reached the API came back with no
 * `.badge` and no `table.kv`, and the fallbacks
 *
 * ```js
 * verdict: badge?.textContent?.trim() || (ok ? 'OK' : `HTTP ${status}`),
 * good:    badge ? badge.classList.contains('good') : ok,
 * ```
 *
 * turned it into `{ verdict: 'OK', good: true, reason: '' }`: a green dot, the
 * word OK, and a blank reason, over a request no server ever saw. The dev
 * server's own shell answers exactly this way when `/pull/spot` is missing from
 * the proxy list in `web/vite.config.js`, and so does any static host, and so
 * does a captive portal. Downstream, `runReason` was then null and every
 * instrument was filed "Already held" or "No bars landed" — never "the request
 * never arrived".
 *
 * `+layout.svelte` and `/autopilot` already guard their JSON routes this way,
 * each with the same sentence about the proxy list. This is that guard for the
 * one route that answers HTML.
 *
 * # The marker is the discriminator, and it is already on the wire
 *
 * A content-type test cannot separate a receipt from a page, because a receipt
 * legitimately IS `text/html`. D-0124 put a marker on the API side for exactly
 * this and recorded that the browser half was NOT done — `web/src` was held by
 * another session while it landed:
 *
 * > Every answer `pull_spot` gives carries `x-brutex-receipt: pull-spot` — the
 * > seat-conflict refusal, the malformed-form refusal and the completed run
 * > alike, because a marker is only worth requiring if it is unconditional.
 *
 * `/dashboard`, which also answers 200 with HTML, deliberately does not carry
 * it — that is what makes requiring it a discriminator rather than a
 * decoration. This is that half.
 *
 * Three tests, and the reason survives all of them: the answer must carry the
 * marker, it must BE html, and it must carry the verdict element
 * `api::render::receipt_page` puts on every answer this route can give —
 * REFUSED, NOT STARTED, STORED. Anything else is not a receipt and nothing in
 * it is a measurement.
 */

/** The header `api::server::RECEIPT_HEADER` writes, and its only value. */
export const RECEIPT_HEADER = 'x-brutex-receipt';
/** `api::server::RECEIPT_SPOT`. */
export const RECEIPT_SPOT = 'pull-spot';

/**
 * `null` when this answer is a receipt, or the refusal to render in its place.
 *
 * @param {object} answer
 * @param {string} answer.html the body, verbatim.
 * @param {number} answer.status the HTTP status, for the sentence.
 * @param {string | null} answer.ctype the `content-type` header, or null.
 * @param {string | null} answer.marker the `x-brutex-receipt` header, or null.
 * @param {boolean} answer.hasVerdict whether the parsed document carries the
 *        badge element.
 * @returns {ReturnType<typeof refusal> | null}
 */
export function notAReceipt({ html, status, ctype, marker, hasVerdict }) {
  const mime = String(ctype ?? '');
  const body = String(html ?? '');
  if (marker !== RECEIPT_SPOT) {
    return refusal(
      status,
      body,
      `POST /pull/spot answered without ${RECEIPT_HEADER}: ${RECEIPT_SPOT}${marker ? ` — it carried ${marker}` : ''}. Every answer this handler gives carries that header, so this one did not come from it: an authenticating proxy, a captive portal, a misdirected origin, or — in development — /pull/spot missing from the proxy list in web/vite.config.js. Nothing was asked of any vendor and nothing here is a measurement.`
    );
  }
  if (!mime.includes('html')) {
    return refusal(
      status,
      body,
      `POST /pull/spot answered ${mime || 'no content-type'}, not HTML. The API is not behind this route — in development, add /pull/spot to the proxy list in web/vite.config.js. Nothing was asked of any vendor and nothing here is a measurement.`
    );
  }
  if (!hasVerdict) {
    return refusal(
      status,
      body,
      `POST /pull/spot answered ${body.length} byte(s) of HTML carrying no verdict. Every answer this route gives carries one, so this document did not come from it and nothing in it is a measurement.`
    );
  }
  return null;
}

/**
 * The shape the page renders, with `good` false and the reason spelled out.
 *
 * @param {number} status
 * @param {string} html
 * @param {string} reason
 */
function refusal(status, html, reason) {
  return {
    ok: false,
    status,
    verdict: `NOT A RECEIPT — HTTP ${status}`,
    good: false,
    scope: '',
    reason,
    facts: [],
    raw: html
  };
}

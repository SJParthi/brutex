import { auditRefusal } from './refusal.js';

/** Keep bounded server refusal detail visible without accepting result rows.
 * Two envelopes are read: the detail envelope (`status:"refused"`, `rows:[]`)
 * and the audit layer's (`refusal.js` `auditRefusal`), which refuses any
 * audited detail route before or after its handler (F1, D-3218).
 * @param {any} response @param {string} fallback */
export async function detailRefusal(response, fallback) {
  try {
    const body = await response?.json?.();
    if (body !== null && typeof body === 'object' && !Array.isArray(body) &&
        body.schema_version === 1 && body.status === 'refused' &&
        Array.isArray(body.rows) && body.rows.length === 0 &&
        typeof body.refusal === 'string' && body.refusal.length <= 4096 && body.refusal.trim()) {
      return fallback + ' ' + body.refusal.trim();
    }
    const audit = auditRefusal(body);
    if (audit !== null) return fallback + ' ' + audit;
  } catch {
    // Proxies and connection failures need not return the application schema.
  }
  return fallback;
}

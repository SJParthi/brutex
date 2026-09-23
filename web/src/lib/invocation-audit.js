import { detailRefusal } from './detail-refusal.js';
const BASE = 1n << 63n, MAX = (1n << 64n) - 1n;
/** @param {unknown} value */
const uint = value => typeof value === 'string' && value.length <= 20 && /^(0|[1-9]\d*)$/.test(value) && BigInt(value) <= MAX;
/** @param {unknown} value */
const id = value => uint(value) && BigInt(/** @type {string} */ (value)) > BASE;
/** The server's audit IDs exceed JS's safe integer range. Keep them as exact
 * strings in sorting, links, paging and display. @param {unknown} before */
export function auditUrl(before = null) {
  if (before !== null && !id(before)) throw new Error('Choose a canonical durable audit cursor.');
  return '/backtest/audit.json?' + new URLSearchParams({ limit: '32', ...(before === null ? {} : { before: String(before) }) });
}
/** @param {any} body @param {string|null} [before] */
export function validateInvocationAudit(body, before = null) {
  auditUrl(before);
  if (body?.schema_version !== 1 || body.refusal !== null || !Array.isArray(body.records) ||
      body.records.length > 32 || typeof body.claim !== 'string' ||
      !(body.next_before === null || id(body.next_before))) throw new Error('The durable audit page is missing or unsupported.');
  let last = before === null ? MAX + 1n : BigInt(before);
  for (const row of body.records) {
    if (!id(row?.invocation) || BigInt(row.invocation) >= last ||
        !['cli', 'browser', 'http'].includes(row.origin) ||
        !['unconfirmed', 'completed', 'refused', 'failed', 'cancelled'].includes(row.status) ||
        row.terminal !== (row.status !== 'unconfirmed') ||
        typeof row.operation !== 'string' || !row.operation || row.operation.length > 160 || /[\u0000-\u001f\u007f]/.test(row.operation) ||
        !uint(row.at_millis) || !uint(row.elapsed_micros) || !uint(row.completed_boundaries) || row.total_boundaries !== null ||
        !(row.response_status === null || Number.isInteger(row.response_status) && row.response_status >= 100 && row.response_status <= 599) ||
        typeof row.meaning !== 'string') throw new Error('Audit identity, order or recorded outcome does not reconcile.');
    last = BigInt(row.invocation);
  }
  const tail = body.records.at(-1)?.invocation;
  const expected = tail && BigInt(tail) > BASE + 1n ? tail : null;
  if (body.next_before !== expected) throw new Error('Audit continuation differs from its last exact invocation.');
  return body;
}
/** @param {string|null} before @param {(url:string)=>Promise<any>} request */
export async function fetchInvocationAudit(before, request) {
  const response = await request(auditUrl(before));
  if (!response?.ok) {
    const fallback = 'Durable audit unavailable (HTTP ' + String(response?.status) + ').';
    let body;
    try { body = await response?.json?.(); } catch { throw new Error(fallback); }
    const writeFailure = body?.code === 'invocation_audit_unavailable' && typeof body.handler_completed === 'boolean';
    const readFailure = body?.code === 'invocation_audit_read_unavailable' && !Object.hasOwn(body, 'handler_completed');
    if (body?.schema_version === 1 && (writeFailure || readFailure) && typeof body.refusal === 'string' &&
        body.refusal.trim() && body.refusal.length <= 4096 &&
        typeof body.why === 'string' && body.why.length <= 4096) {
      throw new Error(`${fallback} ${body.refusal.trim()} ${body.why}`);
    }
    throw new Error(await detailRefusal({ json: async () => body }, fallback));
  }
  return validateInvocationAudit(await response.json(), before);
}
/** @param {string} millis */
export function auditTime(millis) {
  if (!uint(millis) || millis === '0' || BigInt(millis) > 8640000000000000n) return 'Time unavailable';
  return new Intl.DateTimeFormat('en-IN', { timeZone: 'Asia/Kolkata', dateStyle: 'medium', timeStyle: 'medium' }).format(new Date(Number(millis)));
}

/** Current writer admission is separate from the outcome of old commands.
 * A positive read is only a snapshot: the server must claim its OS lease again
 * at POST admission. No read here declares historical research completed. */

/** @param {unknown} body */
export function sweepAdmission(body) {
  const admission = body !== null && typeof body === 'object' && !Array.isArray(body)
    ? /** @type {Record<string,any>} */ (body).admission : null;
  if (!admission || Array.isArray(admission) || admission.schema_version !== 1 ||
      admission.scope !== 'cooperating-store-writers' || typeof admission.available !== 'boolean' ||
      typeof admission.why !== 'string' || !admission.why) {
    return { available: false, why: 'The server has not supplied a valid current execution-lease check. Rechecking status.' };
  }
  return { available: admission.available, why: admission.why };
}

/** @param {{phase:string,run?:any,why?:string}} sweep
 * @param {{available:boolean,why:string}} admission @param {boolean} unconfirmed */
export function sweepLaunchStop(sweep, admission, unconfirmed) {
  if (unconfirmed) return 'The previous launch has no confirmed response. It may have started; no duplicate request will be sent.';
  if (sweep.phase === 'starting' || sweep.phase === 'running') return 'A sweep is starting or running. Wait for its recorded outcome.';
  if (!admission.available) return admission.why;
  if (sweep.phase === 'unknown' && sweep.run?.where !== 'cli') return sweep.why || 'The current browser execution is unconfirmed.';
  return '';
}

/** An exact acceptance or explicit pre-dispatch refusal is the only decisive
 * POST response. Everything else remains unconfirmed and must not be resent.
 * @param {number} status @param {any} body */
export function sweepSubmission(status, body) {
  if (body?.accepted === false && typeof body.refusal === 'string' && body.refusal &&
      status >= 400 && status < 600 && !Object.hasOwn(body, 'attempt') &&
      !Object.hasOwn(body, 'attempt_key') && body.started !== true) {
    return { phase: 'failed', attempt: '', why: body.refusal, confirmed: true };
  }
  const attempt = body?.attempt;
  if (status === 202 && body?.accepted === true && body.refusal === null &&
      typeof attempt === 'string' && /^[1-9]\d{0,19}$/.test(attempt) &&
      BigInt(attempt) <= 18446744073709551615n &&
      (body.attempt_key === undefined || body.attempt_key === attempt) && body.started !== false) {
    return { phase: 'running', attempt, why: '', confirmed: true };
  }
  return { phase: 'unknown', attempt: '', why: 'The launch response did not confirm acceptance or refusal. It may have started; do not submit it again.', confirmed: false };
}

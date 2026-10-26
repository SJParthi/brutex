/**
 * THE QUARANTINE PANEL'S LOGIC (R4-18; ING-01, RT-DATA-02, Z5, Z6).
 *
 * `GET /pull/quarantine.json` reads the plan `cli zerodha-quarantine --plan`
 * prints (nothing written); `POST /pull/quarantine` is the press, which sets
 * aside the bars R4-17's series-identity plan names: a month named whole is
 * recorded (no empty generation), any other is served as a new generation
 * without them, the old one byte-identical. The server answers
 * `{ exit, applied, report }`, or `{ error, code }` when the question is
 * refused. This module builds the question and the body, checks the reason
 * the server checks, and holds the press so one press is ONE request.
 */

/** The longest reason the server takes, in bytes (`store::quarantine::MAX_REASON`). */
export const MAX_REASON_BYTES = 200;

/** The causes a plan measures that a quarantine acts on. */
export const CAUSES = Object.freeze(['before_listing', 'foreign_level', 'zero_price']);

/** The segments a Zerodha share or index is filed under. */
export const SEGMENTS = Object.freeze(['CASH', 'INDEX']);

/**
 * Why `reason` would be refused, or `null`.
 * @param {string} reason
 */
export function reasonFault(reason) {
  const text = String(reason ?? '').trim();
  if (text === '') return 'Name the reason: a quarantine is ledgered with why it was done.';
  const bytes = new TextEncoder().encode(text).length;
  if (bytes > MAX_REASON_BYTES) return `The reason is ${bytes} bytes; the ledger takes at most ${MAX_REASON_BYTES}.`;
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f]/.test(text)) return 'The reason holds a tab or a line break; it is one line.';
  return null;
}

/**
 * Why the question is not one the server takes, or `null`.
 * @param {{ symbol: string, from: string, to: string, cause: string, segment?: string }} q
 */
export function questionFault(q) {
  const symbol = String(q?.symbol ?? '').trim().toUpperCase();
  if (!/^[A-Z0-9&_-]{1,24}$/.test(symbol)) return 'Name a symbol (or ALL).';
  for (const [name, value] of [['From', q?.from], ['To', q?.to]]) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(String(value ?? ''))) return `${name} is a day, YYYY-MM-DD.`;
  }
  if (String(q.from) > String(q.to)) return 'From is after To.';
  if (!CAUSES.includes(String(q?.cause))) return `The cause is one of ${CAUSES.join(', ')}.`;
  if (q.segment !== undefined && !SEGMENTS.includes(String(q.segment))) return `The segment is ${SEGMENTS.join(' or ')}.`;
  return null;
}

/** @param {{ symbol: string, from: string, to: string, cause: string, segment?: string }} q */
function pairs(q) {
  const out = new URLSearchParams();
  out.set('symbol', String(q.symbol).trim().toUpperCase());
  out.set('from', String(q.from));
  out.set('to', String(q.to));
  out.set('cause', String(q.cause));
  out.set('segment', String(q.segment ?? 'CASH'));
  return out;
}

/**
 * The plan's URL.
 * @param {{ symbol: string, from: string, to: string, cause: string, segment?: string }} q
 */
export function planUrl(q) {
  return `/pull/quarantine.json?${pairs(q).toString()}`;
}

/**
 * The press's form body, or the fault that keeps it from being sent.
 * @param {{ symbol: string, from: string, to: string, cause: string, segment?: string }} q
 * @param {string} reason
 * @returns {{ body: string | null, fault: string | null }}
 */
export function pressBody(q, reason) {
  const fault = questionFault(q) ?? reasonFault(reason);
  if (fault) return { body: null, fault };
  const out = pairs(q);
  out.set('reason', String(reason).trim());
  return { body: out.toString(), fault: null };
}

/**
 * The server's answer in one line, and whether it is good.
 * @param {number} status
 * @param {string} text
 * @returns {{ ok: boolean, line: string, report: string }}
 */
export function answerOf(status, text) {
  /** @type {any} */
  let body = null;
  try {
    body = JSON.parse(text);
  } catch {
    body = null;
  }
  if (body && typeof body.report === 'string') {
    const ok = status >= 200 && status < 300 && body.exit === 0;
    const verb = body.applied ? 'Applied' : 'Plan';
    return { ok, line: `${verb}: HTTP ${status}, exit ${body.exit}${ok ? '' : ' (a month was refused; the report names it)'}.`, report: body.report };
  }
  if (body && typeof body.error === 'string') {
    return { ok: false, line: `HTTP ${status} ${body.code ?? ''}: ${body.error}`.trim(), report: '' };
  }
  return { ok: false, line: `HTTP ${status}: the server's answer is not the quarantine's (${String(text).slice(0, 200)}).`, report: '' };
}

/**
 * ONE REQUEST PER PRESS. A press while one is held sends nothing and says
 * what holds it; a press whose body is refused sends nothing and says why.
 * @param {(body: string) => Promise<{ status: number, text: string }>} send
 * @param {() => number} [now]
 */
export function createQuarantineGate(send, now = () => Date.now()) {
  /** @type {{ what: string, since: number } | null} */
  let holder = null;
  let sent = 0;
  return {
    get holder() { return holder; },
    get sent() { return sent; },
    /**
     * @param {{ symbol: string, from: string, to: string, cause: string, segment?: string }} q
     * @param {string} reason
     */
    async press(q, reason) {
      if (holder !== null) {
        return { sent: false, ok: false, line: `A quarantine press is already running from this page: ${holder.what}. Nothing was sent.`, report: '' };
      }
      const made = pressBody(q, reason);
      if (made.body === null) return { sent: false, ok: false, line: made.fault ?? 'refused', report: '' };
      holder = { what: `${String(q.symbol).toUpperCase()} ${q.from}..${q.to} ${q.cause}`, since: now() };
      sent += 1;
      try {
        const a = await send(made.body);
        return { sent: true, ...answerOf(a.status, a.text) };
      } catch (why) {
        return {
          sent: true,
          ok: false,
          line: `POST /pull/quarantine did not answer: ${why instanceof Error ? why.message : String(why)}. It may still be running; the ledger audit/<feed>.quarantine.ledger and /logs say what landed.`,
          report: ''
        };
      } finally {
        holder = null;
      }
    }
  };
}

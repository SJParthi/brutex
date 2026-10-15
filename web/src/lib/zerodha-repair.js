/**
 * THE REPAIR PANEL'S RULES (R4-01, Z6): the read-only plan as a table, the
 * reason a press must carry, the one request a press sends, and what the
 * panel says while one runs.
 *
 * `POST /pull/repair` (D-3350, WP-13c) publishes a NEW generation of a month —
 * every held bar kept, the offered bars added — and was reachable from no page:
 * the data agent's Z6 (minute bars that do not fit their own day bar; PIIND
 * 7,576 minutes in 23 months) stopped at "NOT RUN: it needs the repair route,
 * which no page of the app can start". `GET /pull/repair.json` already answers
 * what a repair would do, read-only, and writes the body each month needs; this
 * module reads that answer and never invents a field: the eight values come
 * from the server's own template, checked to be exactly those eight for the
 * plan's own member, only `repair_reason` is filled in, and the body is written
 * again with every value encoded.
 *
 * Pure apart from the `send` a gate is handed, so
 * `web/tests/zerodha-repair.test.js` drives every branch.
 */

/** The longest reason the server takes, in BYTES (`api::server::MAX_REPAIR_REASON`). */
export const MAX_REASON_BYTES = 200;

/** The fields one `POST /pull/repair` body carries, every one from the plan. */
export const BODY_FIELDS = /** @type {const} */ ([
  'target', 'vendor', 'granularity', 'cash_identity', 'member', 'from', 'to', 'repair_reason'
]);

/**
 * The plan's URL. Every field is encoded; nothing is defaulted here — the
 * server refuses a missing or malformed one by name.
 *
 * @param {{ feed: string, instrument: string, segment: string, rung: string, from: string, to: string }} sel
 */
export function planUrl(sel) {
  const q = new URLSearchParams({
    feed: sel.feed ?? '',
    instrument: sel.instrument ?? '',
    segment: sel.segment ?? '',
    rung: sel.rung ?? '',
    from: sel.from ?? '',
    to: sel.to ?? ''
  });
  return `/pull/repair.json?${q.toString()}`;
}

/**
 * @typedef {{
 *   month: string, generation: number | null, bars: number | null,
 *   sessions: number, held: number, front: number, interior: number,
 *   tail: number, absent: number, beforeFirst: number, owed: number,
 *   repair: boolean, refused: boolean, body: string | null, member: string,
 *   bodyFault: string | null, from: string | null, to: string | null
 * }} PlanRow
 */

/**
 * THE STORE SYMBOL A PLAN IS FOR: its `instrument` key is
 * `<EXCHANGE>-<SEGMENT>-<SYMBOL>` (`NSE-CASH-M&M`, `NSE-CASH-BAJAJ-AUTO`), and
 * neither the exchange nor the segment holds a dash, so the symbol is
 * everything after the second one. `''` when the key has no such shape.
 * @param {unknown} key
 */
export function planSymbol(key) {
  if (typeof key !== 'string') return '';
  const parts = key.split('-');
  if (parts.length < 3 || parts[0] === '' || parts[1] === '') return '';
  return parts.slice(2).join('-');
}

/**
 * WHY A SERVER TEMPLATE CANNOT BE SENT, or `null` when it can.
 *
 * The template must hold the eight fields, each exactly once and nothing else,
 * and its `member` must be the plan's own symbol. `/pull/repair.json` writes
 * `member=<symbol>` without encoding it, so for `M&M` the template reads
 * `member=M&M&from=…`: parsed, that is `member=M` plus a stray key `M`, and a
 * press sent the WRONG MEMBER (R4-01 repair round 1, test A1). A key outside
 * the eight, a key twice, or a member that is not the plan's is refused by
 * name; nothing is guessed back together.
 *
 * @param {string | null} template
 * @param {string} member the plan's own symbol (`planSymbol(plan.instrument)`)
 */
export function templateFault(template, member) {
  if (typeof template !== 'string' || template === '') return 'The plan carries no body for this month, so nothing is sent.';
  const q = new URLSearchParams(template);
  const lacking = BODY_FIELDS.filter((f) => !q.has(f) || (f !== 'repair_reason' && (q.get(f) ?? '') === ''));
  if (lacking.length > 0) return `The plan's body lacks ${lacking.join(', ')}, so nothing is sent.`;
  /** @type {Map<string, number>} */
  const seen = new Map();
  for (const [k] of q) seen.set(k, (seen.get(k) ?? 0) + 1);
  const known = /** @type {readonly string[]} */ (BODY_FIELDS);
  const stray = [...seen.keys()].filter((k) => !known.includes(k));
  const twice = [...seen].filter(([k, n]) => n > 1 && known.includes(k)).map(([k]) => k);
  if (stray.length > 0 || twice.length > 0) {
    const what = [
      stray.length > 0 ? `key(s) outside the eight fields (${stray.map((k) => JSON.stringify(k)).join(', ')})` : '',
      twice.length > 0 ? `field(s) given twice (${twice.join(', ')})` : ''
    ].filter((x) => x !== '').join(' and ');
    return `The plan's body for this month carries ${what}: a member holding "&" written into it without encoding splits exactly so, and it would send member=${JSON.stringify(q.get('member'))}${member ? ` for ${JSON.stringify(member)}` : ''}. Nothing is sent; the server's template must encode its fields.`;
  }
  if (typeof member !== 'string' || member === '') return "The plan names no instrument, so the body's member cannot be checked against it. Nothing is sent.";
  if (q.get('member') !== member) return `The plan's body names member ${JSON.stringify(q.get('member'))}, but the plan is for ${JSON.stringify(member)}. Nothing is sent.`;
  return null;
}

/** @param {unknown} v */
const count = (v) => (Number.isInteger(v) && /** @type {number} */ (v) >= 0 ? /** @type {number} */ (v) : null);

/**
 * THE PLAN AS A TABLE: one row per month, with the sessions it owes by what
 * fills them, and the body the server wrote for each month a repair must fill.
 *
 * A body that is not an object, a month without its counts, or a repair month
 * whose body the server did not write is a FAULT, named — never a row read as
 * "nothing owed".
 *
 * @param {any} body the parsed `/pull/repair.json` answer
 * @returns {{ rows: PlanRow[], totals: Record<string, number> | null, fault: string | null, bodyFaultLine: string | null, member: string, instrument: string, rung: string, from: string, to: string }}
 */
export function planRows(body) {
  const empty = (/** @type {string} */ fault) => ({ rows: [], totals: null, fault, bodyFaultLine: null, member: '', instrument: '', rung: '', from: '', to: '' });
  if (body === null || typeof body !== 'object' || Array.isArray(body)) return empty('The plan is not a JSON object, so nothing in it is read.');
  if (!Array.isArray(body.months)) return empty('The plan carries no months array, so nothing in it is read.');
  const bodies = Array.isArray(body.repair?.bodies) ? body.repair.bodies : [];
  /** @type {Map<string, string>} */
  const byMonth = new Map();
  for (const b of bodies) {
    if (typeof b !== 'string') continue;
    const from = new URLSearchParams(b).get('from') ?? '';
    if (/^\d{4}-\d{2}-\d{2}$/.test(from)) byMonth.set(from.slice(0, 7), b);
  }
  const member = planSymbol(body.instrument);
  /** @type {PlanRow[]} */
  const rows = [];
  for (const m of body.months) {
    const month = typeof m?.month === 'string' ? m.month : '';
    const sessions = count(m?.sessions);
    const held = count(m?.held);
    if (!/^\d{4}-(0[1-9]|1[0-2])$/.test(month) || sessions === null || held === null || !Array.isArray(m?.missing)) {
      return empty(`A month of the plan is malformed (${JSON.stringify(m).slice(0, 120)}), so no row of it is shown as a count.`);
    }
    const by = { front: 0, interior: 0, tail: 0, absent: 0, before_first: 0 };
    for (const x of m.missing) {
      const fill = typeof x?.fill === 'string' ? x.fill : '';
      if (fill in by) by[/** @type {keyof typeof by} */ (fill)] += 1;
    }
    const repair = m.repair_needed === true;
    const tpl = byMonth.get(month) ?? null;
    if (repair && tpl === null) {
      return empty(`The plan says ${month} needs a repair and carries no body for it, so no press is offered for any month.`);
    }
    const q = tpl ? new URLSearchParams(tpl) : null;
    rows.push({
      month,
      generation: count(m.generation),
      bars: count(m.bars),
      sessions,
      held,
      front: by.front,
      interior: by.interior,
      tail: by.tail,
      absent: by.absent,
      beforeFirst: by.before_first,
      owed: m.missing.length,
      repair,
      refused: m.refused === true,
      body: repair ? tpl : null,
      member,
      bodyFault: repair ? templateFault(tpl, member) : null,
      from: q?.get('from') ?? null,
      to: q?.get('to') ?? null
    });
  }
  const faulted = rows.filter((r) => r.bodyFault !== null);
  return {
    rows,
    totals: body.totals && typeof body.totals === 'object' ? body.totals : null,
    fault: null,
    // EVERY MONTH WHOSE PRESS IS REFUSED BEFORE IT IS PRESSED, in one line.
    bodyFaultLine: faulted.length === 0
      ? null
      : `${faulted.length} month(s) cannot be repaired from this page (${faulted.map((r) => r.month).join(', ')}): ${faulted[0].bodyFault}`,
    member,
    instrument: typeof body.instrument === 'string' ? body.instrument : '',
    rung: typeof body.rung === 'string' ? body.rung : '',
    from: typeof body.from === 'string' ? body.from : '',
    to: typeof body.to === 'string' ? body.to : ''
  };
}

/**
 * WHY A REASON CANNOT BE SENT, or `null` when it can: the server's own rule
 * (`api::server::pull_repair`) — one line, at most 200 bytes, no control
 * character, not blank. Checked here so a press the server would refuse is
 * never sent; the server checks again.
 *
 * @param {unknown} reason
 */
export function reasonFault(reason) {
  if (typeof reason !== 'string' || reason.trim() === '') return 'Name why this repair is run: the reason is journaled with every repaired month, and the server refuses a repair without one.';
  const text = reason.trim();
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f-\u009f]/.test(text)) return 'The reason holds a control character (a line break, a tab): it must be one line.';
  const bytes = new TextEncoder().encode(text).length;
  if (bytes > MAX_REASON_BYTES) return `The reason is ${bytes} bytes; the server takes at most ${MAX_REASON_BYTES}.`;
  return null;
}

/**
 * THE ONE BODY A PRESS SENDS: the server's template for the month, with the
 * reason in place of `<why>`, written again field by field in the eight
 * fields' order so every value is encoded (a member `M&M` goes out as
 * `member=M%26M`). Refused, by name, when the reason is not one the server
 * takes or the template is not exactly the eight fields for the plan's own
 * member (`templateFault`).
 *
 * @param {string | null} template one of `/pull/repair.json`'s `repair.bodies`
 * @param {string} reason
 * @param {string} member the plan's own symbol (`PlanRow.member`)
 * @returns {{ body: string, fault: null } | { body: null, fault: string }}
 */
export function repairBody(template, reason, member) {
  const why = reasonFault(reason);
  if (why !== null) return { body: null, fault: why };
  const bad = templateFault(template, member);
  if (bad !== null) return { body: null, fault: bad };
  const q = new URLSearchParams(/** @type {string} */ (template));
  const out = new URLSearchParams();
  for (const f of BODY_FIELDS) out.set(f, f === 'repair_reason' ? reason.trim() : (q.get(f) ?? ''));
  return { body: out.toString(), fault: null };
}

/**
 * The server's answer to a press, VERBATIM, with its HTTP status — a refusal
 * (a busy seat, an expired Kite token, a reason it would not take) is the
 * server's own sentence and is never paraphrased here.
 *
 * @param {{ status: number, verdict: string, reason: string }} a
 */
export function answerWords({ status, verdict, reason }) {
  const v = typeof verdict === 'string' && verdict.trim() !== '' ? verdict.trim() : 'no verdict';
  const r = typeof reason === 'string' && reason.trim() !== '' ? `: ${reason.trim()}` : '';
  return `HTTP ${status} · ${v}${r}`;
}

/**
 * ONE PRESS AT A TIME, AND EXACTLY ONE REQUEST PER PRESS.
 *
 * While a repair runs, a second press sends nothing and is answered with the
 * holder: which repair, of what, since when. The press is released when the
 * request answers or fails, never before.
 *
 * @param {(body: string) => Promise<{ status: number, verdict: string, reason: string }>} send
 * @param {() => number} [now]
 */
export function createRepairGate(send, now = () => Date.now()) {
  /** @type {{ what: string, since: number } | null} */
  let holder = null;
  let sent = 0;
  return {
    /** The repair in flight from this page, or null. */
    get holder() { return holder; },
    /** How many requests this gate has sent. */
    get sent() { return sent; },
    /**
     * @param {string | null} template
     * @param {string} reason
     * @param {string} what the words for the holder line, e.g. `NSE-CASH-PIIND 2024-03`
     * @param {string} member the plan's own symbol (`PlanRow.member`): the
     *   template's member must be it, or nothing is sent
     * @returns {Promise<{ sent: boolean, line: string, status: number | null }>}
     */
    async press(template, reason, what, member) {
      if (holder !== null) return { sent: false, line: `A repair is already running from this page: ${holder.what}, since ${new Date(holder.since).toISOString()}. Nothing was sent.`, status: null };
      const made = repairBody(template, reason, member);
      if (made.body === null) return { sent: false, line: made.fault, status: null };
      holder = { what, since: now() };
      sent += 1;
      try {
        const a = await send(made.body);
        return { sent: true, line: answerWords(a), status: a.status };
      } catch (why) {
        return { sent: true, line: `POST /pull/repair did not answer: ${why instanceof Error ? why.message : String(why)}. It may still be running on the server; /audit and the plan's journal say what landed.`, status: 0 };
      } finally {
        holder = null;
      }
    }
  };
}

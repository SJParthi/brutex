// R4-01 (Z6): the repair panel's plan table, its reason, and the one request
// a press sends. The plan below is shaped like `GET /pull/repair.json`
// (crates/api/src/repair_route.rs `render`); every number is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  BODY_FIELDS, MAX_REASON_BYTES, answerWords, createRepairGate, planRows, planSymbol, planUrl, reasonFault, repairBody, templateFault
} from '../src/lib/zerodha-repair.js';

const body = (/** @type {string} */ from, /** @type {string} */ to) =>
  `target=indices&vendor=zerodha&granularity=1min&cash_identity=zerodha_nse_checked&member=FAKEIDX&from=${from}&to=${to}&repair_reason=<why>`;

const PLAN = {
  feed: 'zerodha',
  instrument: 'NSE-INDEX-FAKEIDX',
  rung: '1min',
  from: '2099-01-01',
  to: '2099-03-31',
  months: [
    {
      month: '2099-01', generation: 0, bars: 7_000, sessions: 20, held: 18, first: null, last: null, unattested: [], refused: false,
      repair_needed: true,
      missing: [{ day: '2099-01-01', fill: 'front', by: 'versioned repair' }, { day: '2099-01-15', fill: 'interior', by: 'versioned repair' }]
    },
    { month: '2099-02', generation: 1, bars: 7_500, sessions: 20, held: 20, missing: [], unattested: [], refused: false, repair_needed: false },
    {
      month: '2099-03', generation: 0, bars: 300, sessions: 21, held: 1, unattested: [], refused: false, repair_needed: false,
      missing: Array.from({ length: 20 }, (_, i) => ({ day: `2099-03-${String(i + 2).padStart(2, '0')}`, fill: 'tail', by: 'ordinary pull' }))
    }
  ],
  totals: { sessions: 61, held: 39, missing: 22, repair_sessions: 2, repair_months: 1, before_first: 0 },
  repair: { route: 'POST /pull/repair', bodies: [body('2099-01-01', '2099-01-31')] },
  journal: { lines: [], truncated: false }
};

test('the plan reads as a table of what each month owes and what fills it', () => {
  const p = planRows(PLAN);
  assert.equal(p.fault, null);
  assert.deepEqual(p.rows.map((r) => [r.month, r.held, r.sessions, r.interior, r.front, r.tail, r.owed, r.repair]), [
    ['2099-01', 18, 20, 1, 1, 0, 2, true],
    ['2099-02', 20, 20, 0, 0, 0, 0, false],
    ['2099-03', 1, 21, 0, 0, 20, 20, false]
  ]);
  assert.equal(p.rows[0].body, body('2099-01-01', '2099-01-31'));
  assert.equal(p.rows[0].from, '2099-01-01');
  assert.equal(p.rows[1].body, null, 'no press for a month a repair need not fill');
  assert.equal(p.instrument, 'NSE-INDEX-FAKEIDX');
  // A malformed answer is a named fault, never an empty table read as "nothing owed".
  assert.match(planRows(null).fault ?? '', /not a JSON object/);
  assert.match(planRows({}).fault ?? '', /no months array/);
  assert.match(planRows({ months: [{ month: '2099-01' }] }).fault ?? '', /malformed/);
  assert.match(planRows({ ...PLAN, repair: { bodies: [] } }).fault ?? '', /2099-01 needs a repair and carries no body for it/);
  assert.equal(planUrl({ feed: 'zerodha', instrument: 'M&M', segment: 'CASH', rung: '1min', from: '2099-01-01', to: '2099-01-31' }),
    '/pull/repair.json?feed=zerodha&instrument=M%26M&segment=CASH&rung=1min&from=2099-01-01&to=2099-01-31');
});

test('a reason is one line of at most 200 bytes, as the server takes it', () => {
  assert.equal(reasonFault('re-pull: minute levels do not fit the day bar'), null);
  assert.match(reasonFault('') ?? '', /Name why this repair is run/);
  assert.match(reasonFault('   ') ?? '', /Name why/);
  assert.match(reasonFault(undefined) ?? '', /Name why/);
  assert.match(reasonFault('two\nlines') ?? '', /control character/);
  assert.match(reasonFault('a\u0085b') ?? '', /control character/, 'a C1 control too, as char::is_control');
  assert.equal(reasonFault('x'.repeat(MAX_REASON_BYTES)), null);
  assert.match(reasonFault('x'.repeat(MAX_REASON_BYTES + 1)) ?? '', /201 bytes; the server takes at most 200/);
  // BYTES, not characters: 67 rupee signs are 201 bytes.
  assert.match(reasonFault('₹'.repeat(67)) ?? '', /201 bytes/);
});

test('a_zerodha_repair_press_sends_one_request_with_its_reason', async () => {
  /** @type {string[]} */
  const sent = [];
  /** @type {(v: any) => void} */
  let answer = () => {};
  const gate = createRepairGate((b) => {
    sent.push(b);
    return new Promise((r) => { answer = r; });
  }, () => 4_102_444_800_000);
  const row = planRows(PLAN).rows[0];
  const first = gate.press(row.body, 'minute levels do not fit the day bar', 'NSE-INDEX-FAKEIDX 2099-01', row.member);
  // A second press while the first runs sends nothing and names the holder.
  const second = await gate.press(row.body, 'again', 'NSE-INDEX-FAKEIDX 2099-01', row.member);
  assert.equal(second.sent, false);
  assert.match(second.line, /already running from this page: NSE-INDEX-FAKEIDX 2099-01, since 2100-01-01T00:00:00\.000Z\. Nothing was sent\./);
  assert.equal(gate.holder?.what, 'NSE-INDEX-FAKEIDX 2099-01');
  answer({ status: 200, verdict: 'STORED', reason: '' });
  const done = await first;
  assert.equal(sent.length, 1, 'exactly one request for one press');
  assert.equal(gate.sent, 1);
  assert.equal(gate.holder, null, 'released when the server answers');
  assert.equal(done.line, 'HTTP 200 · STORED');
  const q = new URLSearchParams(sent[0]);
  assert.deepEqual([...q.keys()], [...BODY_FIELDS], 'the eight fields, every one from the plan');
  assert.equal(q.get('repair_reason'), 'minute levels do not fit the day bar');
  assert.equal(q.get('from'), '2099-01-01');
  assert.equal(q.get('member'), 'FAKEIDX');
  assert.ok(!sent[0].includes('%3Cwhy%3E'), 'the placeholder is replaced');
});

test('a refused press is the server\'s words with its code; a bad reason or body sends nothing', async () => {
  let calls = 0;
  const refusing = createRepairGate(async () => {
    calls += 1;
    return { status: 409, verdict: 'REFUSED', reason: "Repair pull zerodha's pull seat is held. It is held by the hand run started on /ingest." };
  });
  const row = planRows(PLAN).rows[0];
  const busy = await refusing.press(row.body, 'why', 'x', row.member);
  assert.equal(busy.line, "HTTP 409 · REFUSED: Repair pull zerodha's pull seat is held. It is held by the hand run started on /ingest.");
  assert.equal(busy.status, 409);
  const expired = createRepairGate(async () => ({ status: 400, verdict: 'REFUSED', reason: 'the Kite access token was refused: TokenException' }));
  assert.equal((await expired.press(row.body, 'why', 'x', row.member)).line, 'HTTP 400 · REFUSED: the Kite access token was refused: TokenException');
  // Nothing is sent for a reason the server would refuse, or a body missing a field.
  assert.equal((await refusing.press(row.body, 'two\nlines', 'x', row.member)).sent, false);
  assert.equal((await refusing.press(null, 'why', 'x', 'FAKEIDX')).sent, false);
  const lacking = repairBody('target=indices&vendor=zerodha&repair_reason=<why>', 'why', 'FAKEIDX');
  assert.match(lacking.fault ?? '', /lacks granularity, cash_identity, member, from, to/);
  assert.equal(calls, 1);
  // A request that never answers is said, and the press is released.
  const dead = createRepairGate(async () => { throw new Error('Failed to fetch'); });
  const lost = await dead.press(row.body, 'why', 'x', row.member);
  assert.equal(lost.sent, true);
  assert.match(lost.line, /POST \/pull\/repair did not answer: Failed to fetch\. It may still be running/);
  assert.equal(dead.holder, null);
  assert.equal(answerWords({ status: 500, verdict: '', reason: '' }), 'HTTP 500 · no verdict');
});

/* ─── R4-01 repair round 1 (test A1): a member holding "&" ─────────────────
   `/pull/repair.json` writes `member=<symbol>` unencoded, so the template for
   M&M reads `member=M&M&from=…`. Parsed, that is member=M plus a stray key
   `M`, and a press sent the WRONG member. Every fixture is invented. */
const cashPlan = (/** @type {string} */ key, /** @type {string} */ tpl) => ({
  ...PLAN,
  instrument: key,
  months: [PLAN.months[0]],
  repair: { route: 'POST /pull/repair', bodies: [tpl] }
});

test('a_member_holding_an_ampersand_is_never_sent_as_another_member', async () => {
  const unencoded = 'target=equities&vendor=zerodha&granularity=1min&cash_identity=zerodha_nse_checked&member=M&M&from=2099-01-01&to=2099-01-31&repair_reason=<why>';
  const plan = planRows(cashPlan('NSE-CASH-M&M', unencoded));
  assert.equal(plan.fault, null, 'the plan still reads as a table');
  assert.equal(plan.member, 'M&M');
  const row = plan.rows[0];
  assert.equal(row.repair, true);
  assert.match(row.bodyFault ?? '', /key\(s\) outside the eight fields \("M"\)/);
  assert.match(row.bodyFault ?? '', /would send member="M" for "M&M"\. Nothing is sent/);
  assert.match(plan.bodyFaultLine ?? '', /^1 month\(s\) cannot be repaired from this page \(2099-01\): /);
  let calls = 0;
  const gate = createRepairGate(async () => { calls += 1; return { status: 200, verdict: 'STORED', reason: '' }; });
  const press = await gate.press(row.body, 'probe reason', 'NSE-CASH-M&M 2099-01', row.member);
  assert.equal(press.sent, false);
  assert.equal(calls, 0, 'no request carries member=M');
  assert.match(press.line, /outside the eight fields/);
  // The same template, ENCODED as the server must write it: the member goes
  // out whole and encoded, and the body is the eight fields in their order.
  const encoded = unencoded.replace('member=M&M', 'member=M%26M');
  const good = planRows(cashPlan('NSE-CASH-M&M', encoded)).rows[0];
  assert.equal(good.bodyFault, null);
  const made = repairBody(good.body, 'probe reason', good.member);
  assert.equal(made.fault, null);
  assert.match(made.body ?? '', /&member=M%26M&from=2099-01-01&/);
  const q = new URLSearchParams(made.body ?? '');
  assert.deepEqual([...q.keys()], [...BODY_FIELDS]);
  assert.equal(q.get('member'), 'M&M');
  assert.equal(q.get('repair_reason'), 'probe reason');
  // A member the plan is not for, or a field given twice, is refused by name.
  assert.match(templateFault(encoded.replace('member=M%26M', 'member=M'), 'M&M') ?? '', /names member "M", but the plan is for "M&M"\. Nothing is sent\./);
  assert.match(templateFault(`${encoded}&from=2099-02-01`, 'M&M') ?? '', /field\(s\) given twice \(from\)/);
  assert.match(templateFault(encoded, '') ?? '', /names no instrument/);
  assert.match(repairBody(encoded, 'why', 'M&MFIN').fault ?? '', /but the plan is for "M&MFIN"/);
  // The symbol is everything after the exchange and the segment, dashes kept.
  assert.equal(planSymbol('NSE-CASH-BAJAJ-AUTO'), 'BAJAJ-AUTO');
  assert.equal(planSymbol('NSE-INDEX-NIFTY'), 'NIFTY');
  assert.equal(planSymbol('NIFTY'), '');
  assert.equal(planSymbol(null), '');
});

test('a plan month that is not a calendar month is malformed', () => {
  for (const month of ['2099-13', '2099-00', '2099-1']) {
    const bad = { ...PLAN, months: [{ ...PLAN.months[1], month }] };
    assert.match(planRows(bad).fault ?? '', /malformed/, month);
  }
  assert.equal(planRows({ ...PLAN, months: [{ ...PLAN.months[1], month: '2099-12' }] }).fault, null);
});

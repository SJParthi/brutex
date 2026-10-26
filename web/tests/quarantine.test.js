// R4-18: the quarantine panel's question, its reason and the one request a
// press sends. Every symbol and number is invented.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  CAUSES, MAX_REASON_BYTES, answerOf, createQuarantineGate, planUrl, pressBody, questionFault, reasonFault
} from '../src/lib/quarantine.js';

const Q = { symbol: 'qfixz', from: '2018-03-01', to: '2018-03-31', cause: 'before_listing', segment: 'CASH' };

test('a_quarantine_press_sends_one_request_with_its_reason', async () => {
  /** @type {string[]} */
  const bodies = [];
  /** @type {(v: any) => void} */
  let release = () => {};
  const gate = createQuarantineGate((body) => {
    bodies.push(body);
    return new Promise((resolve) => { release = resolve; });
  });
  const first = gate.press(Q, 'another security before listing');
  const second = await gate.press(Q, 'another security before listing');
  assert.equal(second.sent, false, 'a press while one is held sends nothing');
  assert.match(second.line, /already running/);
  release({ status: 200, text: JSON.stringify({ exit: 0, applied: true, report: 'QFIXZ 1min 2018-03: RECORDED WHOLE' }) });
  const done = await first;
  assert.equal(done.sent, true);
  assert.equal(done.ok, true);
  assert.equal(bodies.length, 1, 'exactly one request');
  const sent = new URLSearchParams(bodies[0]);
  assert.equal(sent.get('reason'), 'another security before listing');
  assert.equal(sent.get('symbol'), 'QFIXZ');
  assert.equal(sent.get('cause'), 'before_listing');
  assert.equal(gate.sent, 1);
  assert.equal(gate.holder, null, 'the press is let go after the answer');
});

test('a press with no reason, a long one or a garbage question sends nothing', async () => {
  let calls = 0;
  const gate = createQuarantineGate(async () => { calls += 1; return { status: 200, text: '{}' }; });
  for (const reason of ['', '   ', 'x'.repeat(MAX_REASON_BYTES + 1), 'line\nbreak']) {
    const r = await gate.press(Q, reason);
    assert.equal(r.sent, false, JSON.stringify(reason));
  }
  for (const bad of [
    { ...Q, symbol: '' }, { ...Q, from: '2018-3-1' }, { ...Q, from: '2018-04-01' }, { ...Q, cause: 'foreign' }, { ...Q, segment: 'NFO' }
  ]) {
    assert.ok(questionFault(bad), JSON.stringify(bad));
    assert.equal((await gate.press(bad, 'r')).sent, false);
  }
  assert.equal(calls, 0);
  assert.equal(reasonFault('x'.repeat(MAX_REASON_BYTES)), null, 'exactly the limit is taken');
  assert.deepEqual(CAUSES, ['before_listing', 'foreign_level', 'zero_price']);
});

test('the plan URL and the body carry exactly the question', () => {
  assert.equal(planUrl(Q), '/pull/quarantine.json?symbol=QFIXZ&from=2018-03-01&to=2018-03-31&cause=before_listing&segment=CASH');
  const made = pressBody({ ...Q, segment: undefined }, ' why ');
  assert.equal(made.fault, null);
  assert.equal(made.body, 'symbol=QFIXZ&from=2018-03-01&to=2018-03-31&cause=before_listing&segment=CASH&reason=why');
});

test('every answer is one named line: report, refusal, or a foreign body', () => {
  const ok = answerOf(200, JSON.stringify({ exit: 0, applied: false, report: 'plan: 3 bar(s)' }));
  assert.equal(ok.ok, true);
  assert.equal(ok.report, 'plan: 3 bar(s)');
  const failed = answerOf(422, JSON.stringify({ exit: 1, applied: true, report: 'REFUSED: x' }));
  assert.equal(failed.ok, false);
  assert.match(failed.line, /refused/);
  const refused = answerOf(409, JSON.stringify({ error: 'a quarantine press is running', code: 'quarantine_running' }));
  assert.equal(refused.ok, false);
  assert.match(refused.line, /quarantine_running/);
  const foreign = answerOf(502, '<html>bad gateway</html>');
  assert.equal(foreign.ok, false);
  assert.match(foreign.line, /not the quarantine/);
});

test('a press whose server does not answer says so and is let go', async () => {
  const gate = createQuarantineGate(async () => { throw new Error('socket hang up'); });
  const r = await gate.press(Q, 'r');
  assert.equal(r.sent, true);
  assert.equal(r.ok, false);
  assert.match(r.line, /did not answer/);
  assert.equal(gate.holder, null);
});

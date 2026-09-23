import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parse } from 'svelte/compiler';
import { monthVerdict, isWholeAudit } from '../src/lib/gap-verdict.js';
import { createPageRequests } from '../src/lib/page-requests.js';

/** @type {import('../src/lib/gap-verdict.js').MonthEvidence} */
const clean = {
  expected: 375,
  lost_minutes: 0,
  unmeasured_minutes: 0,
  truncated: false,
  unreadable_records: 0,
  absent_file: null
};

const faults = [
  { absent_file: 'Month file missing' },
  { invalid_timestamps: 1 },
  { unreadable_records: 1 },
  { evidence_error: '2026-08-03: exact stock session metadata missing' },
  { truncated: true },
  { unmeasured_minutes: 1440 }
];

function answer(month = [clean], overrides = {}) {
  return {
    expected: 375,
    lost_minutes: 0,
    unmeasured_minutes: 0,
    months: month.length,
    months_absent: 0,
    truncated: false,
    calendar: { covers_span: true, stale: false },
    month,
    ...overrides
  };
}

test('fully measured minutes are whole, known absences are short, and verified zero obligation is quiet', () => {
  assert.deepEqual(monthVerdict(clean), { word: 'whole', kind: 'ok' });
  assert.deepEqual(monthVerdict({ ...clean, lost_minutes: 1 }), { word: 'short', kind: 'bad' });
  assert.deepEqual(monthVerdict({ ...clean, expected: 0 }), { word: 'nothing owed', kind: 'quiet' });
  assert.deepEqual(monthVerdict({ ...clean, expected: 0, lost_minutes: 1 }), {
    word: 'short', kind: 'bad'
  });
});

test('every combination of uncertainty forbids whole and nothing owed, even when both totals are zero', () => {
  for (let mask = 1; mask < 2 ** faults.length; mask++) {
    const combined = Object.assign({}, ...faults.filter((_, bit) => mask & (1 << bit)));
    for (const expected of [0, 375]) {
      for (const lost_minutes of [0, 1]) {
        const month = { ...clean, expected, lost_minutes, ...combined };
        const verdict = monthVerdict(month);
        assert.ok(['bad', 'absent'].includes(verdict.kind), JSON.stringify(month));
        assert.notEqual(verdict.word, 'whole', JSON.stringify(month));
        assert.notEqual(verdict.word, 'nothing owed', JSON.stringify(month));
        assert.equal(isWholeAudit(answer([clean, month])), false, JSON.stringify(month));
      }
    }
  }
});

test('verdict precedence names absent files, invalid stamps, damaged records and missing evidence before arithmetic', () => {
  const words = ['no file', 'invalid stamps', 'unreadable', 'unverified', 'incomplete', 'unmeasured'];
  for (let first = 0; first < faults.length; first++) {
    const month = Object.assign({ ...clean, expected: 0, lost_minutes: 1 }, ...faults.slice(first));
    assert.equal(monthVerdict(month).word, words[first]);
  }
});

test('optional fields accept older responses and explicit zero/null, but even an empty evidence error blocks green', () => {
  assert.equal(monthVerdict(clean).kind, 'ok');
  assert.equal(monthVerdict({ ...clean, evidence_error: null, invalid_timestamps: 0 }).kind, 'ok');
  for (const evidence_error of ['', ' ', 'Dated session metadata unavailable']) {
    assert.equal(monthVerdict({ ...clean, evidence_error }).word, 'unverified');
    assert.equal(isWholeAudit(answer([{ ...clean, evidence_error }])), false);
  }
});

test('missing files retain known obligations and exact gap evidence', () => {
  const gaps = [{ day: 20668, from: 555, to: 569, minutes: 15, reason: 'vendor-hole' }];
  const month = { ...clean, absent_file: 'No file', expected: 375, lost_minutes: 375, gaps };
  assert.deepEqual(monthVerdict(month), { word: 'no file', kind: 'absent' });
  assert.equal(month.expected, 375);
  assert.equal(month.lost_minutes, 375);
  assert.strictEqual(month.gaps, gaps);
  assert.equal(isWholeAudit(answer([month])), false);
});

test('omitted required evidence never becomes a clean zero', () => {
  for (const field of Object.keys(clean)) {
    const month = { ...clean };
    Reflect.deleteProperty(month, field);
    assert.equal(monthVerdict(month).word, 'unverified', field);
    assert.equal(isWholeAudit(answer([month])), false, field);
  }
  for (const expected of [-1, NaN, Infinity, 0.5]) {
    assert.equal(monthVerdict({ ...clean, expected }).word, 'unverified');
  }
  // @ts-expect-error Deliberately malformed wire evidence must not become zero.
  assert.equal(monthVerdict({ ...clean, invalid_timestamps: null }).word, 'unverified');
});

test('summary green requires a nonempty fully covered range and clean aggregate evidence', () => {
  assert.equal(isWholeAudit(answer()), true);
  assert.equal(isWholeAudit(answer([clean, { ...clean, expected: 0 }])), true);
  assert.equal(isWholeAudit(answer([{ ...clean, expected: 0 }], { expected: 0 })), false);
  for (const overrides of [
    { expected: 0 },
    { expected: NaN },
    { lost_minutes: 1 },
    { unmeasured_minutes: 1440 },
    { months_absent: 1 },
    { truncated: true },
    { truncated: undefined },
    { months: 2 },
    { month: [], months: 0 },
    { month: undefined },
    { calendar: { covers_span: false } },
    { calendar: undefined }
  ]) {
    assert.equal(isWholeAudit(answer([clean], overrides)), false, JSON.stringify(overrides));
  }
});

test('month faults block summary green even when the aggregate reports zero missing and unmeasured minutes', () => {
  for (const fault of faults) {
    const audit = answer([clean, { ...clean, ...fault }]);
    assert.equal(audit.lost_minutes, 0);
    assert.equal(audit.unmeasured_minutes, 0);
    assert.equal(isWholeAudit(audit), false, JSON.stringify(fault));
  }
  assert.equal(isWholeAudit(answer([{ ...clean, lost_minutes: 1 }])), false);
});

test('a stale table can still support a fully covered historical span', () => {
  assert.equal(isWholeAudit(answer([clean], { calendar: { stale: true, covers_span: true } })), true);
});

const page = readFileSync(new URL('../src/routes/gaps/+page.svelte', import.meta.url), 'utf8');

test('the page uses the tested month and summary verdicts for green styling', () => {
  assert.match(page, /\{@const v = monthVerdict\(m\)\}/);
  assert.match(page, /\{@const whole = isWholeAudit\(b\)\}/);
  assert.equal((page.match(/whole \? 'p'/g) ?? []).length, 2);
  assert.doesNotMatch(page, /lost_minutes === 0|months_absent === 0/);
});

test('the minute audit cannot select or submit a coarser rung, and an absent source disables it', async () => {
  assert.match(page, /const rung = '1min';/);
  assert.match(page, /if \(row\.timeframe !== rung\) continue;/);
  assert.match(page, /if \(store\.feed !== feeds\.active\) return \{ lo, hi \};/);
  assert.match(page, /<select value=\{rung\} disabled[^>]*>\s*<option value="1min">1min<\/option>\s*<\/select>/);
  assert.match(page, /timeframe=\$\{encodeURIComponent\(rung\)\}/);
  assert.match(page, /<button[\s\S]*?disabled=\{[^}]*!hasMinuteSource[^}]*\}/);
  assert.match(page, /No 1min source is recorded/);
  assert.doesNotMatch(page, /bind:value=\{rung\}|rungs\[0\]/);

  // Execute the actual button entry point and owned request reader together.
  // The lifecycle extraction moved the guard from run into runCurrent; its
  // position is not the invariant. An absent minute source must issue no GET,
  // and a present source must still issue precisely the minute audit request.
  const ast = parse(page);
  const declarations = ['run', 'runCurrent'].map((name) => {
    const node = ast.instance?.content.body.find((/** @type {any} */ row) =>
      row.type === 'FunctionDeclaration' && row.id?.name === name);
    assert.ok(node, `The actual ${name} entry point is required by this check.`);
    return page.slice(node.start, node.end);
  }).join('\n');
  const create = new Function('hasMinuteSource', 'createPageRequests', `
    const calls=[],feeds={active:'fixture-feed'},symbol='NSE-INDEX-NIFTY',rung='1min';
    const from='2025-01',to='2025-02',questionKey='exact selected window';
    const parsed={exchange:'NSE',segment:'INDEX',underlying:'NIFTY'},contractTail=()=>'';
    let result={phase:'idle',body:null,why:''};
    const ask=async(url)=>{calls.push(url);return Response.json({minuteAudit:true});};
    const auditRequests=createPageRequests();
    ${declarations}
    return {run,calls,state:()=>result,dispose:()=>auditRequests.dispose()};
  `);
  for (const available of [false, true]) {
    const app = create(available, createPageRequests);
    await app.run();
    assert.equal(app.calls.length, available ? 1 : 0);
    assert.equal(app.state().phase, available ? 'done' : 'idle');
    if (available) {
      const query = new URL(app.calls[0], 'http://fixture').searchParams;
      assert.equal(query.get('timeframe'), '1min');
      assert.equal(query.get('feed'), 'fixture-feed');
      assert.equal(query.get('month'), '2025-01');
      assert.equal(query.get('to'), '2025-02');
    }
    app.dispose();
  }
});

test('the Where column names missing evidence and invalid stamps without hiding exact gap runs', () => {
  assert.match(page, /\{#if m\.absent_file != null\}[\s\S]*?\{\/if\}\s*\{#if m\.evidence_error != null\}/);
  assert.match(page, /Dated session evidence unavailable: \{m\.evidence_error/);
  assert.match(page, /\{#if m\.invalid_timestamps\}[\s\S]*?this month's totals are unverified/);
  assert.match(page, /\{#if m\.unmeasured_minutes > 0\}/);
  assert.match(page, /\{#each losses\(m\) as g/);
  assert.match(page, /\{dayLabel\(g\.day\)\}[\s\S]*?\{clock\(g\.from\)\}–\{clock\(g\.to\)\}/);
  assert.doesNotMatch(page, /losses\(m\)\.slice|and \{losses\(m\)\.length -/);
  assert.doesNotMatch(page, /size is\s+unknowable|Storing\s+a second feed's copy/);
});

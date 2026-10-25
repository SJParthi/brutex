// DATA-1w: the /ingest feed-checks panel's library. Every fixture is
// invented: shares Q&Q and ZZSYN, counts made up.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { address, calendarRows, monthRows, read, readAll, share, stockRows, tickMonthsOf, words, yearsOf } from '../src/lib/feed-checks.js';

const panel = readFileSync(new URL('../src/routes/ingest/panels/FeedChecks.svelte', import.meta.url), 'utf8');

/** A fake `ask` answering from a table of `url -> [status, body]`, logging every url. */
function fake(table) {
  const asked = [];
  const ask = async (url) => {
    asked.push(url);
    const [status, body] = table[url] ?? [404, { error: `no fixture for ${url}` }];
    return { ok: status >= 200 && status < 300, status, text: async () => (typeof body === 'string' ? body : JSON.stringify(body)) };
  };
  return { ask, asked };
}

test('an address encodes every value, so a share with & stays one field', () => {
  assert.equal(address('/feed-checks.json', { month: '2025-03', symbol: 'Q&Q', limit: 1, offset: 0 }),
    '/feed-checks.json?month=2025-03&symbol=Q%26Q&limit=1&offset=0');
  assert.equal(address('/calendar-days.json', { year: 2024, reason: null, limit: 3, offset: 0 }),
    '/calendar-days.json?year=2024&limit=3&offset=0');
  assert.equal(address('/tick-checks.json', {}), '/tick-checks.json');
});

test('a_whole_list_is_read_in_two_asks_never_with_a_typed_page_size', async () => {
  const { ask, asked } = fake({
    '/feed-checks.json?limit=1&offset=0': [200, { page: { total: 3 }, months: [{ month: '2025-03' }] }],
    '/feed-checks.json?limit=3&offset=0': [200, { page: { total: 3 }, months: [{ month: '2025-01' }, { month: '2025-02' }, { month: '2025-03' }] }]
  });
  const r = await readAll(ask, '/feed-checks.json', {});
  assert.equal(r.ok, true);
  assert.equal(r.body.months.length, 3);
  assert.deepEqual(asked, ['/feed-checks.json?limit=1&offset=0', '/feed-checks.json?limit=3&offset=0']);
  // One row: one ask.
  const one = fake({ '/x?limit=1&offset=0': [200, { page: { total: 1 }, rows: [1] }] });
  assert.equal((await readAll(one.ask, '/x', {})).ok, true);
  assert.equal(one.asked.length, 1);
});

test('a refusal is shown in the server words, and garbage is named', async () => {
  const { ask } = fake({
    '/feed-checks.json?month=2025-05&limit=1&offset=0': [404, { error: 'holds no feed-reconcile version of 2025-05' }],
    '/bad?limit=1&offset=0': [200, 'not json'],
    '/nototal?limit=1&offset=0': [200, { page: {} }]
  });
  const missing = await readAll(ask, '/feed-checks.json', { month: '2025-05' });
  assert.equal(missing.ok, false);
  assert.match(missing.error, /answered 404: holds no feed-reconcile version of 2025-05/);
  assert.match((await read(ask, '/bad?limit=1&offset=0')).error, /no JSON/);
  assert.match((await readAll(ask, '/nototal', {})).error, /named no total/);
});

test('a share is exact and a nothing-compared share is a dash', () => {
  assert.equal(share(1, 4), '25.00 % (1 of 4)');
  assert.equal(share(0, 0), '–');
  assert.equal(share(undefined, 3), '–');
  assert.equal(words({ matched: 5, gdfl_only: 1, none: 0 }), 'matched 5, gdfl_only 1');
  assert.equal(words({}), '–');
  assert.equal(words(null), '–');
});

test('the_month_table_joins_both_checks_newest_first_a_refused_month_named', () => {
  const feed = [
    { month: '2024-02', version: 1, stocks: 2, adjusted_stocks: 1, counts: { stock_days: 3, by_status: { matched: 2, gdfl_only: 1 }, ex_date_days: 1, zerodha_minute_short_days: 2 } },
    { month: '2024-03', version: 2, error: 'cut or damaged' }
  ];
  const tick = tickMonthsOf({ months: [
    { month: '2024-02', counts: { share_days: 3, rule_t_exact: { share_days: 2, of: 3 }, minute_match: { a: { compared: 25, high_match: 22 } } } },
    { month: '2024-04', counts: { share_days: 1, rule_t_exact: { share_days: 1, of: 1 }, minute_match: { a: { compared: 0, high_match: 0 } } } }
  ] });
  const rows = monthRows(feed, tick);
  assert.deepEqual(rows.map((r) => r.month), ['2024-04', '2024-03', '2024-02']);
  assert.equal(rows[0].stocks, null, 'a month only the tick check holds');
  assert.equal(rows[0].ruleAHigh, '–', 'no minute compared');
  assert.equal(rows[1].feedError, 'cut or damaged');
  assert.equal(rows[2].ruleT, '66.67 % (2 of 3)');
  assert.equal(rows[2].ruleAHigh, '88.00 % (22 of 25)');
  assert.equal(rows[2].shortDays, 2);
  assert.deepEqual(yearsOf(['2024-02', '2023-12', 'junk', '2024-03']), ['2024', '2023']);
  assert.deepEqual(monthRows([], {}), []);
});

test('the share table joins both checks by share', () => {
  const rows = stockRows([{ symbol: 'ZZSYN', counts: {} }, { symbol: 'Q&Q', counts: {} }], [{ symbol: 'Q&Q', counts: {} }, { symbol: 'ZZTICK', counts: {} }]);
  assert.deepEqual(rows.map((r) => r.symbol), ['Q&Q', 'ZZSYN', 'ZZTICK']);
  assert.ok(rows[0].feed && rows[0].tick);
  assert.equal(rows[2].feed, null);
});

test('the calendar table has a column per listed reason and names a refused year', () => {
  const { columns, rows } = calendarRows([
    { year: 2024, tradable: 246, excluded_sessions: 4, weekends: 104, unexplained: 0, holidays: { record: 'circular', circular: 'NSE/CMTR/1' }, by_reason: { Weekend: 104, Holiday: 14, DisasterRecoveryDrill: 2 } },
    { year: 2027, refused: '2027 is not wholly inside the calendar of record' }
  ]);
  assert.deepEqual(columns, ['DisasterRecoveryDrill', 'Holiday']);
  assert.deepEqual(rows[0].counts, [2, 14]);
  assert.equal(rows[0].holidays, 'NSE/CMTR/1');
  assert.match(rows[1].refused, /not wholly inside/);
});

test('the_panel_keeps_the_contract_and_reads_only_the_three_routes_never_polling', () => {
  assert.match(panel, /export const order = 40;/);
  assert.match(panel, /export const title = '[^']+';/);
  assert.doesNotMatch(panel, /setInterval\(|\bfetch\(/);
  for (const route of ['/feed-checks.json', '/tick-checks.json', '/calendar-days.json']) assert.ok(panel.includes(route), route);
  assert.doesNotMatch(panel, /limit:\s*\d/, 'no typed page size');
});

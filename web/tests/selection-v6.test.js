import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fetchSelectionV6, selectionV6Query, RUNGS } from '../src/lib/selection-v6.js';

const EQUITIES =
  'Selection V6 holds the NIFTY and BANKNIFTY families only. No equity result may enter Selection V6 or execution authority until a charter-sourced equity charge stack exists (CLAUDE.md §1, D-0509, D-0681).';
const h = (/** @type {string} */ b) => b.repeat(32);
/**
 * @param {number} rank
 * @param {string} [family]
 * @returns {any}
 */
function winner(rank, family = 'NIFTY') {
  return {
    rank, top_ten: rank < 10, family, direction: 'long', strategy: h('11'), disposition: h('22'),
    selected_exit: h('33'), global_sequence: String(rank), family_sequence: '0',
    score: '18446744073709551615', mask_words: ['1', '0', '0', '0', '0', '35184372088832'],
    drawdown: '5', worst_loss: '4', losing_rate_ppm: '1', losing_trades: '1', winning_trades: '9',
    win_rate_ppm: '900000', average_win: '10', average_loss: '4', assurance_ppm: '7',
    pessimistic_profit: '-9223372036854775808', loss_ratio_ppm: null, reward_to_risk_ppm: '0'
  };
}
/**
 * @param {any[]} winners
 * @returns {any}
 */
function record(winners) {
  return {
    identity: h('99'), rung_seconds: '300', horizon_bars: '15', population: h('44'),
    execution_completion: h('55'), policy: h('66'), rows: '12', authorized: '11', policy_refused: '1',
    families: [
      { family: 'NIFTY', terminal: 'evaluated', candidates: '3', evaluated: '3', decisions: '3' },
      { family: 'BANKNIFTY', terminal: 'naturally-extinct', candidates: '0', evaluated: '0', decisions: '0' }
    ],
    considered: '12', admitted: '11', refused: '1', unmeasured: '0', winner_count: winners.length, winners
  };
}
/**
 * @param {string | null} [family]
 * @returns {any}
 */
function payload(family = null) {
  return {
    schema_version: 1, status: 'saved', authority: 'sealed-stored-selection-v6-record', root: '/s',
    family, scope: 'Sealed Selection V6 blocks', equities: EQUITIES, refusal: null,
    rungs: RUNGS.map((rung) =>
      rung === '5min'
        ? { rung, status: 'saved', path: null, refusal: null, records: [record([winner(0), winner(1, 'BANKNIFTY')])] }
        : rung === '60min'
          ? { rung, status: 'refused', path: null, refusal: 'seal', records: [] }
          : { rung, status: 'absent', path: `/s/selection/${rung}/global-selection-v6.bin`, refusal: null, records: [] }
    )
  };
}
const reply = (/** @type {any} */ body) => async () => ({ ok: true, json: async () => body });

test('the page reads every rung and keeps exact figures as strings', async () => {
  let url = '';
  const body = await fetchSelectionV6(null, async (u) => { url = u; return { ok: true, json: async () => payload() }; });
  assert.equal(url, '/selection-v6.json');
  assert.equal(body.rungs.length, 8);
  assert.equal(body.rungs[3].records[0].winners[0].score, '18446744073709551615');
  assert.equal(body.rungs[3].records[0].winners[0].pessimistic_profit, '-9223372036854775808');
  assert.equal(body.rungs[7].status, 'refused');
});

test('an equity is refused before any request and a stray family in a payload is refused', async () => {
  for (const equity of ['RELIANCE', 'TCS', 'INDIAVIX']) {
    assert.throws(() => selectionV6Query(equity), /no equity result may enter it/);
    let asked = false;
    await assert.rejects(fetchSelectionV6(equity, async () => { asked = true; return { ok: true, json: async () => payload() }; }));
    assert.equal(asked, false, 'no request is made for an equity');
  }
  assert.equal(selectionV6Query('NIFTY'), '?family=NIFTY');
  const stray = payload();
  stray.rungs[3].records[0].winners[1].family = 'RELIANCE';
  await assert.rejects(fetchSelectionV6(null, reply(stray)), /malformed/);
  const narrowed = payload('NIFTY');
  narrowed.rungs[3].records[0].winners = [winner(0)];
  assert.equal((await fetchSelectionV6('NIFTY', reply(narrowed))).family, 'NIFTY');
  narrowed.rungs[3].records[0].winners = [winner(0), winner(1, 'BANKNIFTY')];
  await assert.rejects(fetchSelectionV6('NIFTY', reply(narrowed)));
});

test('a server refusal is shown, and a payload this page cannot account for is refused whole', async () => {
  await assert.rejects(
    fetchSelectionV6(null, async () => ({ ok: false, status: 400, json: async () => ({ schema_version: 1, status: 'refused', rows: [], rungs: [], refusal: 'RELIANCE is a cash equity.' }) })),
    /HTTP 400\)\. RELIANCE is a cash equity\./
  );
  for (const change of /** @type {Array<(b: any) => void>} */ ([
    (b) => { b.rungs.pop(); },
    (b) => { b.equities = ''; },
    (b) => { b.status = 'absent'; },
    (b) => { b.rungs[3].records[0].winners[0].rank = 1; },
    (b) => { b.rungs[3].records[0].winner_count = 3; },
    (b) => { b.rungs[3].records[0].families.reverse(); },
    (b) => { b.rungs[3].records[0].winners[0].score = 18446744073709551615; },
    (b) => { b.rungs[0].records = [record([])]; },
    (b) => { b.rungs[7].refusal = ''; }
  ])) {
    const body = payload();
    change(body);
    await assert.rejects(fetchSelectionV6(null, reply(body)));
  }
});

test('the page states the equity exclusion and is linked from the console', () => {
  const page = readFileSync(new URL('../src/routes/selection/+page.svelte', import.meta.url), 'utf8');
  assert.match(page, /fetchSelectionV6/);
  assert.match(page, /Equities excluded\./);
  assert.match(page, /result\.body\.equities/);
  assert.match(page, /result\.body\.scope/);
  assert.match(page, /\{#each FAMILIES as name/);
  const layout = readFileSync(new URL('../src/routes/+layout.svelte', import.meta.url), 'utf8');
  assert.match(layout, /href: '\/selection'/);
});

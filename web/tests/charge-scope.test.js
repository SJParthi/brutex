// A STOCK RUN IS GROSS OF EVERY CHARGE, AND THE PAGE MUST NOT SAY ZERO.
//
// `/backtest.json` puts `equity_note` on a cash-equity run's object
// (`crates/api/src/backtest.rs`, `a_stock_run_carries_the_equity_note_and_an_index_run_is_unchanged`),
// and `CLAUDE.md` §1 requires every equity report to state that it is gross
// of every charge. The report page printed `Commission load 0.00%` /
// "statutory charges are zero for spot indices", "Statutory charges are zero
// for this spot-index sweep" and "spot indices only" whatever the run's
// instrument, so a RELIANCE run reached the operator labelled as carrying no
// commission. D-0946.

import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const page = readFileSync(new URL('../src/routes/backtest/+page.svelte', import.meta.url), 'utf8');
const scope = () => import('../src/lib/charge-scope.js');

/** The server's own note, abbreviated: the page never parses its words. */
const NOTE = '  CASH EQUITY. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE: brokerage, ...';

test('the page states no zero-charge claim outside the charge-scope helper', () => {
  assert.doesNotMatch(page, /statutory charges are zero for spot indices/);
  assert.doesNotMatch(page, /Statutory charges are zero for this spot-index sweep/);
  assert.doesNotMatch(page, /'spot indices only'/);
  assert.doesNotMatch(page, /<span class="tt-qv">0\.00%<\/span>/);
  assert.doesNotMatch(page, /one-unit spot-index totals/);
});

test('the page reads the open run and the picked symbol through the helper', () => {
  assert.match(page, /import \{[^}]*chargeScope[^}]*\} from '\$lib\/charge-scope\.js'/);
  assert.match(page, /const openCharges = \$derived\(chargeScope\(openRun\)\)/);
  assert.match(page, /\{openCharges\.load\}/);
  assert.match(page, /\{openCharges\.note\}/);
  assert.match(page, /\{openCharges\.trades\}/);
  assert.match(page, /coverScope\(pickedSymbols\)/);
  assert.match(page, /isSweptIndex/);
  // The server's note is shown in both places the charge statement is: the
  // breakdown pane and over the trade list.
  assert.equal((page.match(/\{openCharges\.serverNote\}/g) ?? []).length, 2);
});

test('an index run keeps its zero-levy statement', async () => {
  const { chargeScope } = await scope();
  for (const underlying of ['NIFTY', 'BANKNIFTY', 'NSE-NIFTY', 'NSE-BANKNIFTY']) {
    const got = chargeScope({ underlying });
    assert.equal(got.gross, false, underlying);
    assert.equal(got.load, '0.00%');
    assert.match(got.note, /statutory charges are zero for spot indices/);
    assert.match(got.trades, /Statutory charges are zero for this spot-index run/);
    // Zero levies is not zero cost: the unmodeled spread stays named.
    assert.match(got.note, /spread is not modeled/);
    assert.match(got.trades, /spread remains unmodeled/);
    assert.equal(got.serverNote, null);
  }
});

test('a stock run is labelled gross and carries the server note verbatim', async () => {
  const { chargeScope } = await scope();
  const got = chargeScope({ underlying: 'RELIANCE', equity_note: NOTE });
  assert.equal(got.gross, true);
  assert.equal(got.load, 'Not subtracted');
  assert.match(got.note, /gross of every charge/);
  assert.doesNotMatch(got.note, /zero/);
  assert.match(got.trades, /gross of every charge/);
  assert.doesNotMatch(got.trades, /zero/);
  assert.match(got.note, /spread is not modeled/);
  assert.match(got.trades, /spread remains unmodeled/);
  assert.equal(got.serverNote, NOTE.trim());
});

test('a note on an index-named run still wins: the server decides, not the name', async () => {
  const { chargeScope } = await scope();
  const got = chargeScope({ underlying: 'NIFTY', equity_note: NOTE });
  assert.equal(got.gross, true);
  assert.equal(got.serverNote, NOTE.trim());
});

test('a non-index run WITHOUT the note is still gross, never zero', async () => {
  const { chargeScope } = await scope();
  for (const run of [{ underlying: 'RELIANCE' }, { underlying: 'TCS', equity_note: '   ' }, null, undefined, {}]) {
    const got = chargeScope(run);
    assert.equal(got.gross, true, JSON.stringify(run));
    assert.equal(got.load, 'Not subtracted');
    assert.equal(got.serverNote, null);
  }
});

test('the coverage clause names the pick: index only for an index', async () => {
  const { coverScope, isSweptIndex } = await scope();
  assert.equal(coverScope(new Set(['NIFTY'])), 'spot indices only');
  assert.equal(coverScope(new Set(['NIFTY', 'NSE-BANKNIFTY'])), 'spot indices only');
  assert.equal(coverScope(new Set(['RELIANCE'])), 'RELIANCE is not a spot index: its figures are gross of every charge');
  // The picker takes several instruments; an index picked FIRST must not
  // hide a stock picked beside it.
  assert.equal(coverScope(new Set(['NIFTY', 'RELIANCE'])), 'RELIANCE is not a spot index: its figures are gross of every charge');
  assert.equal(
    coverScope(new Set(['NIFTY', 'RELIANCE', 'TCS'])),
    '2 picked instruments are not spot indices: their figures are gross of every charge'
  );
  assert.equal(isSweptIndex('NIFTY'), true);
  assert.equal(isSweptIndex('NSE-INDIAVIX'), false);
  assert.equal(isSweptIndex('RELIANCE'), false);
  assert.equal(isSweptIndex(undefined), false);
});

// CE-94: the crown and the comparison board RANK across instruments, so a
// cash equity on either carries its gross-of-every-charge statement there, not
// only after "Drill in". CE-93 / D-2792: both carry the ledger's own in-sample,
// validation-unrecorded statement too.
test('the crown and the comparison board state charges and the in-sample limit', () => {
  assert.match(page, /const bestCharges = \$derived\(chargeScope\(best\)\)/);
  assert.match(page, /\{#if bestCharges\.gross\}/);
  assert.match(page, /data-crown-charges>\s*\{bestCharges\.serverNote \?\? bestCharges\.note\}/);
  assert.match(page, /for \(const run of rankableRuns\) \{\s*const scope = chargeScope\(run\);/);
  assert.match(page, /data-board-charges>\{boardChargeNote\}/);
  assert.match(page, /ledger\?\.in_sample/);
  assert.match(page, /data-crown-in-sample>\{ledgerInSample\}/);
  assert.match(page, /data-board-in-sample>\{ledgerInSample\}/);
});

test('a stock crowned over an index is labelled gross by the same helper', async () => {
  const { chargeScope } = await scope();
  const crowned = chargeScope({ underlying: 'RELIANCE', equity_note: NOTE });
  assert.equal(crowned.gross, true);
  assert.match(crowned.serverNote ?? crowned.note, /GROSS OF EVERY CHARGE/);
  assert.match(chargeScope({ underlying: 'RELIANCE' }).note, /gross of every charge/);
});

// CE-94: the rung leaders and the ledger table rank and list stock totals too,
// so each carries the gross statement where a stock appears.
test('the rung leaders and the ledger table state charges for a stock', () => {
  assert.match(page, /charges: chargeScope\(present\[0\]\)/);
  assert.match(page, /\{#if g\.charges\.gross\}\s*<span class="pill warn" data-rung-charges/);
  assert.match(page, /runs\.some\(\(run\) => chargeScope\(run\)\.gross\)/);
  assert.match(page, /data-ledger-charges>\s*Rows that are not a swept spot index: \{ledgerChargeNote\}/);
});

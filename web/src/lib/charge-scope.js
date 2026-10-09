/**
 * WHAT A RUN'S FIGURES ARE GROSS OF — the words the report page prints about
 * charges, with no runes in it, so `web/tests/charge-scope.test.js` can drive
 * it under `node --test`.
 *
 * # The failure this exists to remove
 *
 * The report page printed `Commission load 0.00%` beside "statutory charges
 * are zero for spot indices", said "Statutory charges are zero for this
 * spot-index sweep" over the trade list, and "spot indices only" under the
 * timeframe chips — hard-coded, whatever the run's instrument. The page
 * accepts a cash-equity pick, and `/backtest.json` marks such a run with
 * `equity_note` (`crates/api/src/backtest.rs`), so a RELIANCE run reached the
 * operator labelled as carrying no commission. `CLAUDE.md` §1 requires every
 * equity report to state that it is gross of every charge. D-0946.
 *
 * # The rule
 *
 * The zero-levy statement is printed ONLY for a run over one of the two swept
 * spot indices that the server did NOT mark with `equity_note`. Everything
 * else — a stock, a run whose instrument is unknown, no run at all — is
 * labelled gross. The server's note, when sent, wins over the symbol: it is
 * `runner::audit::CostScope::report_note`, the one wording every `cli` report
 * prints, and the page shows it verbatim rather than paraphrasing it.
 */

/** The two swept spot indices, bare and with the exchange prefix. */
export const SWEPT_INDICES = Object.freeze(['NIFTY', 'BANKNIFTY', 'NSE-NIFTY', 'NSE-BANKNIFTY']);

/**
 * Whether `symbol` names one of the two swept spot indices.
 *
 * @param {unknown} symbol
 * @returns {boolean}
 */
export function isSweptIndex(symbol) {
  return typeof symbol === 'string' && SWEPT_INDICES.includes(symbol);
}

/**
 * @typedef {object} ChargeScope
 * @property {boolean} gross         true unless the run is a swept index the server did not mark.
 * @property {string} load           the value beside "Commission load".
 * @property {string} note           the one-line note under it.
 * @property {string} trades         the sentence over the list of trades.
 * @property {string | null} serverNote the server's `equity_note`, trimmed, or `null`.
 */

/**
 * The charge statement for one run from `/backtest.json`.
 *
 * @param {{ underlying?: unknown, equity_note?: unknown } | null | undefined} run
 * @returns {ChargeScope}
 */
export function chargeScope(run) {
  const raw = run?.equity_note;
  const serverNote = typeof raw === 'string' && raw.trim() !== '' ? raw.trim() : null;
  if (serverNote === null && isSweptIndex(run?.underlying)) {
    return {
      gross: false,
      load: '0.00%',
      note: 'statutory charges are zero for spot indices · spread is not modeled',
      trades: 'Statutory charges are zero for this spot-index run; spread remains unmodeled.',
      serverNote
    };
  }
  return {
    gross: true,
    load: 'Not subtracted',
    note: 'gross of every charge: brokerage, STT, stamp duty, exchange charges, the SEBI fee, the IPFT, DP charges and GST (an UNVERIFIED list) are not subtracted · spread is not modeled',
    trades:
      'Every figure is gross of every charge: no brokerage, STT, stamp duty, exchange charge, SEBI fee, IPFT, DP charge or GST (an UNVERIFIED list) is subtracted, and spread remains unmodeled.',
    serverNote
  };
}

/**
 * The coverage clause under the timeframe chips for the picked instruments.
 *
 * "spot indices only" is printed only when EVERY picked instrument is a swept
 * spot index. The picker accepts several instruments at once, so a clause
 * read off the first pick alone would say "spot indices only" for NIFTY and
 * RELIANCE picked together. One non-index pick is named; several are counted.
 *
 * @param {Iterable<string>} symbols
 * @returns {string}
 */
export function coverScope(symbols) {
  const stocks = [...symbols].filter((symbol) => !isSweptIndex(symbol));
  if (stocks.length === 0) return 'spot indices only';
  if (stocks.length === 1) return `${stocks[0]} is not a spot index: its figures are gross of every charge`;
  return `${stocks.length} picked instruments are not spot indices: their figures are gross of every charge`;
}

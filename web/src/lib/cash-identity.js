export const DEFAULT_ZERODHA_CASH_IDENTITY = 'zerodha_cross_checked';

/** Explicit request choice. Unknown selections keep the strict default.
 * @param {unknown} feed
 * @param {unknown} choice
 */
export function cashIdentityFor(feed, choice) {
  return feed === 'zerodha' &&
    (choice === 'zerodha_cross_checked' || choice === 'zerodha_symbol')
    ? choice : 'isin';
}

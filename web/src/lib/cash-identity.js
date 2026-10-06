export const DEFAULT_ZERODHA_CASH_IDENTITY = 'zerodha_cross_checked';

/** Every explicit Zerodha-only choice the server accepts besides `isin`.
 * `zerodha_nse_checked` reads NSE's own ISIN file and no other vendor (D-3314).
 */
export const ZERODHA_CASH_IDENTITIES = Object.freeze([
  'zerodha_nse_checked',
  'zerodha_cross_checked',
  'zerodha_symbol',
]);

/** Explicit request choice. Unknown selections keep the strict default.
 * @param {unknown} feed
 * @param {unknown} choice
 */
export function cashIdentityFor(feed, choice) {
  return feed === 'zerodha' &&
    typeof choice === 'string' &&
    ZERODHA_CASH_IDENTITIES.includes(choice)
    ? choice : 'isin';
}

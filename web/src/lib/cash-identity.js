/** Explicit request choice. Never infer consent from a truthy non-boolean.
 * @param {unknown} feed
 * @param {unknown} optedIn
 */
export function cashIdentityFor(feed, optedIn) {
  return feed === 'zerodha' && optedIn === true ? 'zerodha_symbol' : 'isin';
}

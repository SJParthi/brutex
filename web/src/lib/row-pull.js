/** Narrow an existing encoded spot request to exactly one visible symbol.
 * @param {string} body
 * @param {string} symbol
 */
export function singleMemberBody(body, symbol) {
  if (typeof symbol !== 'string' || !symbol.trim()) throw new Error('A row pull requires a symbol');
  const params = new URLSearchParams(body);
  params.set('member', symbol);
  return params.toString();
}

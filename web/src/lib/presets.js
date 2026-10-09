// The backtest chart's month presets, in bars (D-3514).
//
// A regular NSE session is 09:15 to 15:29, 375 one-minute bars:
// `pull::session::BARS_PER_REGULAR_SESSION` on the Rust side. The page used
// 555 here, which is the minute of the open past IST midnight, so every preset
// spanned about 1.48 times what its label said.

// The session length is `ist.js`'s, the one web copy of it (D-3516).
import { SESSION_MINUTES } from './ist.js';

/** Minutes in one regular session, re-exported for the presets' callers. */
export { SESSION_MINUTES };

/** Sessions in a preset month. */
export const SESSIONS_PER_MONTH = 21;

/**
 * Bars one preset month holds at a rung of `secs` seconds: the bars one
 * session folds into (a short last bucket counts, so it rounds up, as
 * `pull::fold` does) times 21. A daily or wider rung is one bar a session.
 * A rung with no positive length has no month.
 *
 * @param {number | undefined} secs
 * @returns {number}
 */
export function barsPerMonth(secs) {
  const n = Number(secs);
  if (!(n > 0)) return 0;
  if (n >= 86_400) return SESSIONS_PER_MONTH;
  return Math.ceil((SESSION_MINUTES * 60) / n) * SESSIONS_PER_MONTH;
}

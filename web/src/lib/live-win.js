/**
 * THE LIVE PANEL'S WIN RATE, ONLY WHERE THE WIRE CAN STATE ONE.
 *
 * `/live.json` serves `edge_wins` = `runner::outcome::Edge::wins`, which counts
 * forward moves that were STRICTLY POSITIVE, on either side. For a `long` row
 * those are the trades it won. For a `short` row they are the trades it LOST:
 * a sell wins on a down move. The panel printed `edge_wins / n` as "% won" for
 * both, so the ideal short (`wins == 0`, outcome.rs
 * `the_perfect_short_and_the_perfect_long_are_worth_the_same`) showed as
 * "0% won" under "sell". p14num-1.
 *
 * A sell's win count is NOT `n - edge_wins`: moves of exactly zero are neither
 * a win nor a loss (`Edge::losses` exists because of them), and the live row
 * does not carry the down-move count. So for anything but a `long` row this
 * refuses by name instead of printing the up-move share as a win rate.
 *
 * @param {{ direction?: unknown, n?: unknown, edge_wins?: unknown } | null | undefined} row
 * @returns {{ known: true, wins: number, n: number, share: number } | { known: false, why: string }}
 */
export function liveWinShare(row) {
  const n = Number(row?.n) || 0;
  const wins = Number(row?.edge_wins) || 0;
  if (row?.direction !== 'long') {
    return {
      known: false,
      why:
        row?.direction === 'short'
          ? 'A sell wins on a down move. /live.json sends only the count of up moves (edge_wins), which for a sell are its losing trades, and flat moves count as neither, so this row has no win rate to show.'
          : `This row's direction is ${JSON.stringify(row?.direction ?? null)}, not long or short, so edge_wins cannot be read as wins.`
    };
  }
  return { known: true, wins, n, share: n > 0 ? wins / n : 0 };
}

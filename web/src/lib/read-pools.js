/**
 * THE READ POOLS, AS TABLES (R3-08: NEW-04, NEW-21; rules-audit RA-020,
 * RA-130).
 *
 * `GET /pools.json` names every pool the server's file reads run in — its
 * slots on this machine, what runs and waits now, what it answered, refused
 * busy and saw abandoned — and every route's tally since the process started:
 * answered, busy (429, every slot stayed taken for the wait), being written
 * (503, a writer held the month past the read's patience), cancelled (the
 * reader left before its answer), refused as asked and failed. This module
 * turns that body into rows, or into one line naming what is wrong with it.
 * Nothing is invented: a field the server did not send is a fault, never a 0.
 */

/** The counters every pool row must carry, as `/pools.json` serves them. */
export const POOL_FIELDS = Object.freeze(['slots', 'running', 'waiting', 'answered', 'busy', 'cancelled']);
/** The counters every route row must carry. */
export const ROUTE_FIELDS = Object.freeze(['answered', 'busy', 'being_written', 'cancelled', 'refused', 'failed']);

/** @param {unknown} n */
const count = (n) => Number.isSafeInteger(n) && /** @type {number} */ (n) >= 0;

/**
 * @param {any} body the parsed `/pools.json` answer
 * @returns {{ fault: string } | {
 *   fault: null,
 *   cores: number,
 *   waitMs: number,
 *   patienceMs: number | null,
 *   pools: { pool: string, slots: number, running: number, waiting: number, answered: number, busy: number, cancelled: number }[],
 *   routes: { route: string, answered: number, busy: number, being_written: number, cancelled: number, refused: number, failed: number, asked: number }[],
 *   say: string
 * }}
 */
export function poolTables(body) {
  if (body === null || typeof body !== 'object' || Array.isArray(body)) return { fault: '/pools.json did not answer an object.' };
  if (!Array.isArray(body.pools)) return { fault: '/pools.json carries no pools list.' };
  if (!Array.isArray(body.routes)) return { fault: '/pools.json carries no routes list.' };
  if (!count(body.cores) || !count(body.wait_ms)) return { fault: '/pools.json names no core count or wait.' };
  const pools = [];
  for (const [i, row] of body.pools.entries()) {
    if (typeof row?.pool !== 'string' || row.pool === '') return { fault: `/pools.json pool row ${i} has no name.` };
    const missing = POOL_FIELDS.find((f) => !count(row[f]));
    if (missing) return { fault: `/pools.json pool ${row.pool} has no count for ${missing}.` };
    pools.push({ pool: row.pool, ...Object.fromEntries(POOL_FIELDS.map((f) => [f, row[f]])) });
  }
  // THE FRONT END'S ROW (R3-08 repair 2): pages and assets match no route,
  // so the server counts them under one name that is no path and says which
  // (`front_end`). Taken from the body, never kept here: the row exists as
  // soon as a browser has loaded the page that asks, and refusing it hid both
  // tables behind a fault on every real server.
  const frontEnd = typeof body.front_end === 'string' && body.front_end !== '' ? body.front_end : null;
  const routes = [];
  for (const [i, row] of body.routes.entries()) {
    const named = typeof row?.route === 'string' && (row.route.startsWith('/') || row.route === frontEnd);
    if (!named) return { fault: `/pools.json route row ${i} names no route.` };
    const missing = ROUTE_FIELDS.find((f) => !count(row[f]));
    if (missing) return { fault: `/pools.json route ${row.route} has no count for ${missing}.` };
    const counts = Object.fromEntries(ROUTE_FIELDS.map((f) => [f, row[f]]));
    routes.push({ route: row.route, ...counts, asked: ROUTE_FIELDS.reduce((sum, f) => sum + row[f], 0) });
  }
  // THE ROUTES THAT WERE TURNED AWAY FIRST: busy, being written and
  // cancelled are what this table is for; ties by route, so the order is the
  // same for the same counts.
  routes.sort(
    (a, b) =>
      b.busy + b.being_written + b.cancelled - (a.busy + a.being_written + a.cancelled) ||
      (a.route < b.route ? -1 : a.route > b.route ? 1 : 0)
  );
  return {
    fault: null,
    cores: body.cores,
    waitMs: body.wait_ms,
    patienceMs: count(body.read_patience_ms) ? body.read_patience_ms : null,
    pools,
    routes,
    say: typeof body.say === 'string' ? body.say : ''
  };
}

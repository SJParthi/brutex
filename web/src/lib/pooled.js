/**
 * BOUNDED CONCURRENCY — run many jobs, keep few outstanding.
 *
 * # Why this file exists rather than an import
 *
 * `routes/db/+page.svelte` has held this function since it learned to read bar
 * windows, and it is right there. It is not importable: it sits at line 4404 of
 * a 13,000-line component, inside the component's instance scope, beside
 * `IN_FLIGHT` and the eight closures that use it. Lifting it out of that file
 * means editing that file, and `/db` is the one surface in this tree with a
 * recorded history of a lost write — 8,791 lines to 2,204 in a single bad
 * edit. A fifteen-line primitive is not worth reopening it.
 *
 * So this is the primitive, standing on its own, and `/db` keeps its copy until
 * someone has a reason to be in that file anyway. Two copies of fifteen lines
 * is a cost; it is a smaller cost than the alternative, and this comment is
 * what stops it becoming three.
 *
 * # Why it is `.js` and not `.svelte.js`
 *
 * No runes, so `node --test` can drive it directly — the same reason
 * `$lib/rows.js` was split out of `$lib/store.svelte.js`. A concurrency pool
 * that nothing can test is a concurrency pool nobody should trust.
 */

/**
 * How many requests may be outstanding to one origin at once.
 *
 * SIX, because that is what a browser grants a single origin over HTTP/1.1.
 * Asking for more does not create more; it creates a queue inside the browser
 * that this code cannot see, cannot measure and cannot report on — so the
 * number is chosen to keep the queue HERE, where a caller can say how much is
 * outstanding and why.
 */
export const IN_FLIGHT = 6;

/**
 * Runs `job` over `items` with at most `limit` outstanding at any moment.
 *
 * Results come back in the ORDER OF `items`, never the order they finished,
 * because every caller indexes them against its own plan and a pool that
 * reordered would silently shuffle a grid.
 *
 * A job that throws rejects the whole call. Callers that need per-item failure
 * must return their failures as VALUES — which is what every caller in this
 * tree does, because a table cell that cannot be filled has to say why in that
 * cell rather than take the other cells down with it.
 *
 * An empty `items` resolves to an empty array without starting a worker; a
 * `limit` below one is raised to one, so no argument can produce a pool that
 * never drains.
 *
 * @template T, R
 * @param {T[]} items
 * @param {number} limit
 * @param {(item: T, index: number) => Promise<R>} job
 * @param {AbortSignal} [signal] Stop dispatching queued reads when their page leaves.
 * @returns {Promise<R[]>}
 */
export async function pooled(items, limit, job, signal) {
  /** @type {R[]} */
  const out = new Array(items.length);
  let next = 0;
  let failed = false;
  const worker = async () => {
    for (;;) {
      signal?.throwIfAborted();
      if (failed) return;
      const i = next;
      next += 1;
      if (i >= items.length) return;
      try {
        out[i] = await job(/** @type {T} */ (items[i]), i);
      } catch (why) {
        failed = true;
        throw why;
      }
    }
  };
  const completed = await Promise.allSettled(
    Array.from({ length: Math.max(1, Math.min(limit, items.length)) }, worker)
  );
  // A replacement batch must not start while a rejected batch still owns
  // requests. Drain those workers before reporting the first named failure.
  const refused = completed.find((result) => result.status === 'rejected');
  if (refused?.status === 'rejected') throw refused.reason;
  return out;
}

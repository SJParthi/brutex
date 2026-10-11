/**
 * THE CENSUS OFF THE PAGE'S THREAD — WP-56, D-3327; DB-G84, DB-B52.
 *
 * # What this replaces
 *
 * A cold /db load on GDFL parsed, decoded, folded and decorated the WHOLE
 * census on the page's own thread. The audit measured one continuous stall of
 * about 1.3 s at 246,147 month rows (DB-G84). On a shadow of the operator's
 * store on 10 Oct 2026 (385,442 rows, 33,349,105 bytes; headless Chrome with
 * its own profile, 5 cold loads of the feat/db-fix build) the longest stall
 * was p50 2,272 ms and p99 2,315 ms.
 *
 * # What this does
 *
 * The one piece of that work that cannot be cut into slices is
 * `JSON.parse` of the whole body — 224 ms in node for the GDFL census, one
 * uninterruptible call. It runs here, in a Web Worker, with the decode into
 * COLUMNS (`decodeCensusColumns`): numbers as typed arrays that move to the
 * page without a copy, labels as the wire's own four dictionaries. The page
 * then builds its row objects from the columns (`rowsOfColumns`) in slices of
 * at most `budgetMs` (8 ms) of work, one real task each (`inSlices`,
 * `yieldTask`), and folds and decorates in slices the same way
 * (`store.svelte.js`, `DbCensusBanner.svelte`).
 *
 * # Why the decoration is not done here too
 *
 * Measured in node on the same census: decorating every row costs about the
 * same in a worker (1,018 ms) as on the page (803 ms), and sending 385,442
 * decorated rows back costs more than making them — 531 ms to pack them as
 * columns in the worker and 340 ms to rebuild them on the page, against 803
 * ms to decorate them on the page in slices. A worker preparation added
 * about a second to the time to first rows to save the page work it can do
 * in slices without a stall. So the page decorates, in slices.
 *
 * # The prefix index is gone (DB-B52)
 *
 * The preparation used to file every decorated row under four prefixes of
 * four tokens for a search box /db no longer draws (WP-51 replaced it with
 * `$lib/db/search.js`): 4,586,125 references in 699 buckets, the largest
 * holding every row, about 8 times the preparation's time and 42 MB of heap,
 * read by nothing. `DbCensusBanner.svelte` no longer asks for it.
 *
 * # What it does not change
 *
 * The rows every page reads are the same objects, field for field
 * (`census-worker.test.js` holds the two paths equal). The census is still
 * the WHOLE census: this removes the stall, not the O(store) bytes and heap;
 * those go only when /db's selection model reads `/census/facets.json`
 * (`store-census.js`, `createFacetsLoader`) instead of the whole census.
 * Where no worker can run (node, an old browser, a worker that failed), the
 * page decodes on its own thread and SAYS SO (`store.worker`), never
 * silently.
 */
import { decodeCensusColumns, rowsOfColumns, columnBuffers, columnsOrder, noteCensusMeasure } from './store-census-wire.js';

/* ======================================================================
   STRINGS, AS ONE TEXT AND ITS ENDS
   ----------------------------------------------------------------------
   385,000 strings posted as an array cost the page 35 ms to deserialise in
   one task; joined into one string with an Int32Array of ends, 8 ms, and
   cutting them back out is 3 ms more (node, 10 Oct 2026).
   ====================================================================== */
/** @param {string[]} list @returns {PackedStrings} */
export function packStrings(list) {
  const ends = new Int32Array(list.length);
  let at = 0;
  for (let i = 0; i < list.length; i++) {
    at += list[i].length;
    ends[i] = at;
  }
  return { text: list.join(''), ends };
}

/** @param {PackedStrings} packed @returns {string[]} */
export function unpackStrings({ text, ends }) {
  const out = new Array(ends.length);
  let at = 0;
  for (let i = 0; i < ends.length; i++) {
    out[i] = text.slice(at, ends[i]);
    at = ends[i];
  }
  return out;
}

/** @typedef {{text: string, ends: Int32Array}} PackedStrings */

/**
 * Distinct underlyings over decorated rows: the names the store holds, in
 * the unit the universe rows count (a membership list names underlyings;
 * DB-N07).
 * @param {Iterable<any>} deco
 */
export function heldNames(deco) {
  const names = new Set();
  for (const row of deco) names.add(row.underlying);
  return names.size;
}

/* ======================================================================
   THE WORKER
   ====================================================================== */
/**
 * What the worker answers one message with. Exported so a test drives the
 * protocol without a worker.
 *
 * `decode` takes the response body as bytes and the feed it was asked for,
 * and answers the census as columns (`decodeCensusColumns`), its instrument
 * dictionary packed (`packStrings`), with `order` (`null`, or the sentence
 * naming the first row out of the census's order) and `bars` (the bars the
 * rows sum to); a legacy JSON-array body is answered as itself. A malformed
 * census is refused with the same sentence the page's own decoder throws, so
 * the two paths cannot disagree about which census is valid.
 *
 * @param {any} data
 * @returns {{reply: any, transfer: ArrayBuffer[]}}
 */
export function answerMessage(data) {
  const { op, id } = data ?? {};
  try {
    if (op !== 'decode') throw new Error(`unknown census worker request ${JSON.stringify(op)}`);
    const body = JSON.parse(new TextDecoder().decode(data.buffer));
    const cols = decodeCensusColumns(body, data.feed);
    if (!cols) return { reply: { op: 'decoded', id, legacy: body }, transfer: [] };
    // MEASURED HERE, OFF THE PAGE'S THREAD: whether the rows are in the
    // order a refresh's delta is merged by (`columnsOrder`, which learns the
    // order of exchanges and segments from the rows themselves), and the
    // bars they sum to, which a merge is checked against.
    const { fault: order, ranks } = columnsOrder(cols);
    let bars = 0;
    for (let i = 0; i < cols.n; i++) bars += cols.rows[i];
    const sent = { ...cols, instruments: packStrings(cols.instruments) };
    return { reply: { op: 'decoded', id, cols: sent, order, ranks, bars },
      transfer: [...columnBuffers(cols), /** @type {ArrayBuffer} */ (sent.instruments.ends.buffer)] };
  } catch (why) {
    return { reply: { op: 'error', id, message: String(/** @type {any} */ (why)?.message ?? why) }, transfer: [] };
  }
}

const scope = /** @type {any} */ (globalThis);
if (typeof scope.WorkerGlobalScope !== 'undefined' && scope instanceof scope.WorkerGlobalScope) {
  scope.onmessage = (/** @type {MessageEvent} */ event) => {
    const { reply, transfer } = answerMessage(event.data);
    scope.postMessage(reply, transfer);
  };
}

/* ======================================================================
   THE PAGE'S SIDE
   ====================================================================== */
/** @type {{port: MessagePort, queue: (() => void)[]} | null} */
let channel = null;

/**
 * Yield ONE TASK: a MessageChannel message, so the timers, input and frames
 * queued behind this work run before it continues. `scheduler.yield` is not
 * used here: its continuation is scheduled ahead of other tasks, which is how
 * a "yielding" preparation still held every timer on /db for over two
 * seconds. Falls back to a zero-delay timer where there is no channel.
 * @returns {Promise<void>}
 */
export function yieldTask() {
  if (typeof MessageChannel !== 'function') return new Promise((done) => setTimeout(done, 0));
  if (!channel) {
    const pair = new MessageChannel();
    /** @type {(() => void)[]} */
    const queue = [];
    pair.port1.onmessage = () => queue.shift()?.();
    // NODE ONLY: a port with a listener keeps the process alive.
    /** @type {any} */ (pair.port1).unref?.();
    channel = { port: pair.port2, queue };
  }
  const { port, queue } = channel;
  return new Promise((done) => {
    queue.push(done);
    port.postMessage(null);
  });
}

/**
 * Run `work(start, end)` over `0..n` in slices of at most about `budgetMs` of
 * work each, with one task (`yieldControl`) between slices.
 *
 * Steps of `step` items are run until the budget is spent, so a slice
 * overruns the budget by at most one step; the caller sizes `step` so one
 * step is well under it.
 *
 * @param {number} n
 * @param {(start: number, end: number) => void} work
 * @param {AbortSignal | undefined} signal checked before every slice
 * @param {{yieldControl?: () => Promise<unknown>, budgetMs?: number, step?: number, clock?: () => number}} [options]
 */
export async function inSlices(n, work, signal, { yieldControl = yieldTask, budgetMs = 8, step = 512, clock = () => performance.now() } = {}) {
  let start = 0;
  while (start < n) {
    signal?.throwIfAborted();
    const began = clock();
    do {
      const end = Math.min(n, start + step);
      work(start, end);
      start = end;
    } while (start < n && clock() - began < budgetMs);
    if (start < n) await yieldControl();
  }
  signal?.throwIfAborted();
}

/**
 * How long the page waits for one answer of the census worker before it
 * decodes the census itself. The worker parses and decodes the 33 MB GDFL
 * census in about 0.1 s (Node, n=5, WP-56); ten seconds is a hundred times
 * that, and a third of the request's own ceiling (`STORE_CENSUS_MS`).
 */
export const WORKER_CEILING_MS = 10_000;

/**
 * The page's handle on the census worker.
 *
 * `decode(bytes, feed, signal)` resolves to the census rows, the same objects
 * `decodeCensus` builds, made from the worker's columns in slices.
 * `broken` is the reason the worker cannot be used, once it has failed; the
 * caller then decodes on the page and says so.
 *
 * A CEILING ON THE ANSWER (WP-56b; the WP-56 second agent's follow-up 1). A
 * worker that takes the message and then neither answers nor errors used to
 * leave the page on "Reading /store.json" for good: nothing rejected the
 * read. Each call now waits at most `ceilingMs`; past it the worker is
 * marked broken with that reason, every waiting read is rejected, and the
 * caller decodes on the page and says so, exactly as for a worker that died.
 *
 * @param {() => Worker} spawn
 * @param {{yieldControl?: () => Promise<unknown>, budgetMs?: number, step?: number, ceilingMs?: number,
 *   setTimer?: (work: () => void, ms: number) => any, clearTimer?: (timer: any) => void}} [options]
 */
export function createCensusWorker(spawn, options = {}) {
  const ceilingMs = options.ceilingMs ?? WORKER_CEILING_MS;
  const setTimer = options.setTimer ?? ((work, ms) => setTimeout(work, ms));
  const clearTimer = options.clearTimer ?? ((timer) => clearTimeout(timer));
  /** @type {Worker | null} */
  let worker = null;
  /** @type {string | null} */
  let broken = null;
  let next = 0;
  /** @type {Map<number, {resolve: (value: any) => void, reject: (why: Error) => void}>} */
  const pending = new Map();

  /** @param {string} why */
  function fail(why) {
    broken = why;
    for (const { reject } of pending.values()) reject(new Error(why));
    pending.clear();
    worker?.terminate();
    worker = null;
  }

  function live() {
    if (broken) throw new Error(broken);
    if (worker) return worker;
    try {
      worker = spawn();
    } catch (why) {
      fail(`the census worker could not start: ${String(/** @type {any} */ (why)?.message ?? why)}`);
      throw new Error(/** @type {string} */ (broken));
    }
    worker.onmessage = (event) => {
      const data = event.data;
      const waiting = pending.get(data?.id);
      if (!waiting) return; // cancelled, or a late answer to a superseded read
      pending.delete(data.id);
      if (data.op === 'error') waiting.reject(new Error(data.message));
      else waiting.resolve(data);
    };
    worker.onerror = (event) => {
      event.preventDefault?.();
      fail(`the census worker failed: ${event.message || 'it could not load or run'}`);
    };
    worker.onmessageerror = () => fail('the census worker sent a message the page could not read');
    return worker;
  }

  /**
   * @param {any} message
   * @param {AbortSignal | undefined} signal
   * @returns {Promise<any>}
   */
  function call(message, signal) {
    const target = live();
    return new Promise((resolve, reject) => {
      if (signal?.aborted) return reject(signal.reason);
      const onAbort = () => {
        const waiting = pending.get(message.id);
        pending.delete(message.id);
        if (waiting) waiting.reject(signal?.reason);
        else reject(signal?.reason);
      };
      signal?.addEventListener('abort', onAbort, { once: true });
      // THE CEILING: an answer, an error or an abort clears it; nothing else
      // can leave a read waiting.
      const timer = setTimer(() => {
        if (pending.has(message.id)) fail(`the census worker did not answer within ${ceilingMs / 1000} s`);
      }, ceilingMs);
      const settle = () => { clearTimer(timer); signal?.removeEventListener('abort', onAbort); };
      pending.set(message.id, {
        resolve: (value) => { settle(); resolve(value); },
        reject: (why) => { settle(); reject(why); }
      });
      // COPIED, NOT MOVED: one memcpy (a few milliseconds for 33 MB), and the
      // caller keeps the bytes to decode itself if the worker dies mid-read.
      target.postMessage(message);
    });
  }

  /**
   * @param {ArrayBuffer} bytes the response body; copied to the worker
   * @param {string} feed
   * @param {AbortSignal} [signal]
   * @returns {Promise<any[]>}
   */
  async function decode(bytes, feed, signal) {
    const reply = await call({ op: 'decode', id: ++next, feed, buffer: bytes }, signal);
    if (reply.legacy) return reply.legacy;
    const cols = { ...reply.cols, instruments: unpackStrings(reply.cols.instruments) };
    /** @type {any[]} */
    const rows = new Array(cols.n);
    await inSlices(cols.n, (start, end) => rowsOfColumns(cols, start, end, rows), signal, options);
    noteCensusMeasure(rows, { order: reply.order ?? null, bars: reply.bars, ranks: reply.ranks ?? null });
    return rows;
  }

  return {
    decode,
    /** The reason the worker cannot be used, or null. */
    get broken() { return broken; }
  };
}

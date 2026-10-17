/**
 * THE /backtest STATUS POLL: BACK OFF WHILE NOTHING CHANGES, PAUSE WHILE NOBODY
 * LOOKS, AND STOP OUT LOUD.
 *
 * `/backtest/run.json` was asked every 2 s for as long as the tab stayed open,
 * hidden or not, and for ever while the answer was `unknown` (p99-7, D-1445,
 * D-0954, D-2327). Every such GET writes one invocation-journal file and about
 * eight full fsyncs to the store drive, and the `unknown` answer is exactly the
 * one that does not change on its own: about 1,800 journal files an hour per
 * open tab, measured on the shadow store in the AF-06 report.
 *
 * This wraps the page's own `createPageRequests` owner, so every existing
 * cancellation and single-flight rule stays as it was, and changes only WHEN a
 * re-poll fires:
 *
 * - **A sweep in flight is never slowed.** While the page shows a run as
 *   `running`, the rung progress (`fetchLive`), the best rows so far
 *   (`fetchLiveTop`) and the finish are all read on this tick, and a browser
 *   sweep's status body (`Progress::to_json`) is the same on every read until
 *   the run ends. Backing off there made the live view and the finish up to
 *   60 s old (the AF-06 r1 blocking finding). The cost is bounded by the run:
 *   a browser sweep ends with its task, and a CLI sweep silent for
 *   `STALE_AFTER_MILLIS` (15 min) is answered `unknown`, which backs off.
 * - **Any change of answer** puts the next check at the delay the page asked
 *   for (2 s).
 * - **An unchanged settled, idle or `unknown` answer** doubles the wait, up to
 *   a 60 s ceiling. `unknown` counts as one answer whatever its reason text
 *   says, so a reason that embeds a count or a status code cannot keep the
 *   poll at 2 s for ever.
 * - **A hidden tab sends nothing.** A timer that fires while the tab is hidden
 *   parks its work; showing the tab runs it at once.
 * - **After `stopAfter` `unknown` answers in a row the poll stops** and says
 *   so. Every `unknown` answer counts, including one whose lease availability
 *   differs from the last: that difference is a change, so the wait returns to
 *   2 s, but a lease that flips on every read must not keep an unknown poll
 *   alive for ever. A known answer in between starts the count again. The stop
 *   sets `state().stopped` to the sentence the page shows next to its Refresh
 *   control. Never a silent stop — the page renders the reason.
 * - **`cancel()` starts a new chapter**: the page calls it before a launch and
 *   when a child observer takes over, so a launch is always followed at the
 *   page's own delay rather than an inherited back-off.
 *
 * Pure and DOM-free: visibility and timers are injected, so
 * `web/tests/status-poll.test.js` drives it with a fake clock.
 */

/** The page's own re-poll delay, and where an answer that changed puts it. */
export const BASE_MS = 2000;
/** The longest wait between two checks of an unchanged answer. */
export const CEILING_MS = 60000;
/**
 * `unknown` answers in a row before the poll stops and asks for a Refresh;
 * unchanged, they are 2 + 4 + 8 + 16 + 32 + 60 + 60 + 60 + 60 s, about five minutes of
 * checking after the answer stopped changing.
 */
export const STOP_AFTER = 10;

/**
 * Fields of a status answer that change on every read without carrying new
 * evidence. `age_millis` is the server's `now - at_unix_millis`, so it grows by
 * the poll interval on every answer while the event it measures stays the same.
 */
const VOLATILE = new Set(['age_millis']);

/** @typedef {import('./page-requests.js').ReadTicket} ReadTicket */
/** @typedef {(ticket:ReadTicket)=>Promise<void>} Work */
/** @typedef {{key:string,unknown:boolean,live:boolean,why:string}} Answer */
/**
 * `backedOff` is true only when the armed check waits longer than the page
 * asked because the answer did not change: the page's "has not changed" note
 * is shown on it, never on the phase alone.
 * @typedef {{delay:number,interval:number,unknownStreak:number,stopped:string,
 *   hidden:boolean,parked:boolean,checks:number,backedOff:boolean}} PollState
 */

/**
 * What the page currently believes, reduced to the part a re-poll compares.
 *
 * @param {{phase?:string,run?:any,why?:string}|null|undefined} sweep
 * @param {{available?:boolean,why?:string}|null|undefined} admission
 * @returns {Answer}
 */
export function statusAnswer(sweep, admission) {
  const available = admission?.available === true;
  if (sweep?.phase === 'unknown') {
    return {
      key: `unknown|${available}`,
      unknown: true,
      live: false,
      why: sweep.why || 'Execution state is unknown.'
    };
  }
  const run = sweep?.run;
  // IN FLIGHT: `sweepOutcome` says `running` only for `in_flight: true`, and a
  // launch the server accepted is `running` with no run yet. A contradictory
  // body (running, `in_flight: false`) is not treated as live.
  const live = sweep?.phase === 'running' && run?.in_flight !== false;
  const steady = run !== null && typeof run === 'object' && !Array.isArray(run)
    ? Object.fromEntries(Object.entries(run).filter(([field]) => !VOLATILE.has(field)))
    : run ?? null;
  return {
    key: JSON.stringify([sweep?.phase ?? '', steady, available, admission?.why ?? '']),
    unknown: false,
    live,
    why: ''
  };
}

/**
 * The sentence the page shows when the poll has stopped.
 * @param {number} answers @param {string} why
 */
export function stoppedWhy(answers, why) {
  return `Stopped checking after ${answers} "unknown" answers in a row, ` +
    `so that an open page does not write a journal file to the store every few seconds for ever. ` +
    `The last answer was: ${why} Press Refresh to check again.`;
}

/**
 * @param {{
 *   requests: {run:(work:Work)=>Promise<void>, schedule:(work:Work,delay:number)=>void, cancel:()=>void, dispose:()=>void},
 *   isPoll: (work:Work)=>boolean,
 *   followsRun?: (work:Work)=>boolean,
 *   answer: ()=>Answer,
 *   visible?: ()=>boolean,
 *   listen?: (wake:()=>void)=>(()=>void),
 *   report?: (state:PollState)=>void,
 *   base?: number, ceiling?: number, stopAfter?: number
 * }} options
 */
export function createStatusPoll(options) {
  const { requests, isPoll, answer } = options;
  /* WHICH POLLS READ THE RUN. Only a poll that has just read the run and set
     the page's answer from it may keep the 2 s pace for a `running` answer;
     a poll of something else (the page's lease re-check) would otherwise be
     held at 2 s by a `running` the page last saw before a child observer took
     over. Without the option every poll counts. */
  const followsRun = options.followsRun ?? (() => true);
  const base = options.base ?? BASE_MS;
  const ceiling = Math.max(options.ceiling ?? CEILING_MS, base);
  const stopAfter = Math.max(1, options.stopAfter ?? STOP_AFTER);
  const visible = options.visible ?? (() => true);
  let disposed = false;
  let interval = base, delay = base, streak = 0, checks = 0;
  let backedOff = false, lastUnknown = false;
  let lastKey = /** @type {string|null} */ (null);
  let stopped = /** @type {{why:string,work:Work}|null} */ (null);
  let parked = /** @type {Work|null} */ (null);
  let armed = /** @type {Work|null} */ (null);

  /** @returns {PollState} */
  function state() {
    return {
      delay, interval, unknownStreak: streak, stopped: stopped?.why ?? '',
      hidden: !visible(), parked: parked !== null, checks, backedOff
    };
  }
  function publish() {
    if (!disposed) options.report?.(state());
  }

  /** A hidden tab parks the work instead of sending its GET. @param {Work} work @returns {Work} */
  function wrap(work) {
    return async ticket => {
      if (armed === work) armed = null;
      if (!visible()) {
        parked = work;
        publish();
        return;
      }
      if (isPoll(work)) checks += 1;
      await work(ticket);
    };
  }

  /**
   * Where the next check of this answer goes, or null to stop.
   * @param {Answer} now @param {number} requested @param {boolean} live
   */
  function next(now, requested, live) {
    const changed = now.key !== lastKey;
    // AN UNKNOWN SPELL counts every unknown answer in a row, whatever its key:
    // a lease that flips on every read is a change for the wait, never for the
    // stop. The answer that opens the spell is a change and never stops it.
    const continuing = now.unknown && lastUnknown;
    lastKey = now.key;
    lastUnknown = now.unknown;
    streak = now.unknown ? (continuing ? streak + 1 : 1) : 0;
    if (continuing && streak >= stopAfter) return null;
    // A SWEEP IN FLIGHT keeps the page's own pace: its progress and its finish
    // are read on this tick (see the module comment).
    if (changed || live) {
      interval = base;
      return requested;
    }
    interval = Math.min(interval * 2, ceiling);
    return Math.max(requested, interval);
  }

  function reset() {
    interval = base;
    delay = base;
    streak = 0;
    backedOff = false;
    lastUnknown = false;
    lastKey = null;
    stopped = null;
    parked = null;
    armed = null;
  }

  /** @param {Work} work @param {number} requested */
  function schedule(work, requested) {
    if (disposed) return;
    let wait = requested;
    let slowed = false;
    if (isPoll(work)) {
      const now = answer();
      const decided = next(now, requested, now.live && followsRun(work));
      if (decided === null) {
        stopped = { why: stoppedWhy(streak, now.why), work };
        backedOff = false;
        armed = null;
        parked = null;
        publish();
        return;
      }
      wait = decided;
      slowed = decided > requested;
    }
    stopped = null;
    delay = wait;
    backedOff = slowed;
    armed = work;
    requests.schedule(wrap(work), wait);
    publish();
  }

  /** @param {Work} work */
  function run(work) {
    if (disposed) return Promise.resolve();
    armed = null;
    return requests.run(wrap(work));
  }

  /** Showing the tab runs the parked (or still waiting) check at once. */
  function wake() {
    if (disposed) return;
    if (visible() && !stopped) {
      const work = parked ?? armed;
      parked = null;
      armed = null;
      if (work) void requests.run(wrap(work));
    }
    publish();
  }

  const unlisten = options.listen?.(wake) ?? (() => {});

  return {
    run,
    schedule,
    /** A new chapter: forget the back-off, the stop and any parked check. */
    cancel() {
      reset();
      requests.cancel();
      publish();
    },
    /**
     * The Refresh control: check now, at the page's own pace again.
     * @param {Work} [fallback] what to run when nothing was stopped or waiting
     */
    refresh(fallback) {
      if (disposed) return;
      const work = stopped?.work ?? parked ?? armed ?? fallback ?? null;
      reset();
      publish();
      if (work) void requests.run(wrap(work));
    },
    state,
    dispose() {
      disposed = true;
      unlisten();
      requests.dispose();
    }
  };
}

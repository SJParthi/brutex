/** URL selects saved evidence only; it can never select a filesystem root.
 * @param {string} search */
export function savedSelection(search) {
  const q = new URLSearchParams(search);
  const allowed = ['identity', 'pin', 'batch', 'rung', 'offset', 'setting'];
  for (const key of q.keys()) {
    if (!allowed.includes(key) || q.getAll(key).length !== 1) throw new Error('Unsupported or repeated result selection.');
  }
  const identity = q.get('identity');
  const pin = q.get('pin');
  const batch = q.get('batch');
  const rung = q.get('rung') ?? '0';
  const offset = q.get('offset') ?? '0';
  const setting = q.get('setting');
  const uint = (/** @type {string} */ value) => /^(0|[1-9][0-9]*)$/.test(value) && BigInt(value) <= 18446744073709551615n;
  if (!/^[0-9a-f]{64}$/.test(identity ?? '') || pin !== null && !/^[0-9a-f]{64}$/.test(pin) ||
    batch !== null && !uint(batch) || !/^[0-7]$/.test(rung) || !uint(offset) || setting !== null && !uint(setting)) {
    throw new Error('Open a saved-search link containing its exact identity and optional checkpoint.');
  }
  if (setting !== null && (BigInt(setting) < BigInt(offset) || BigInt(setting) >= BigInt(offset) + 32n)) {
    throw new Error('The selected setting must belong to the requested 32-row page.');
  }
  return { identity, pin, batch, rung, offset, setting };
}

/** An exact-result link must actually occur in its authenticated page.
 * @param {{index:string}[]} rows @param {string|null} setting */
export function savedSettingIndex(rows, setting) {
  if (setting === null) return -1;
  const index = rows.findIndex(row => row.index === setting);
  if (index < 0) throw new Error(`Setting ${setting} is absent from this authenticated page. This link has not opened the requested result. Choose a recorded setting below.`);
  return index;
}

/** A single owned request. A late response can never replace a newer selection.
 * @param {(state:{phase:string,body:any,why:string})=>void} publish */
export function latestRequest(publish) {
  let generation = 0;
  /** @type {AbortController | undefined} */
  let controller;
  return {
    /** @param {(signal:AbortSignal)=>Promise<any>} work */
    async run(work) {
      const ticket = ++generation;
      controller?.abort();
      controller = new AbortController();
      const signal = controller.signal;
      publish({ phase: 'loading', body: null, why: '' });
      try {
        const body = await work(signal);
        if (generation === ticket) publish({ phase: 'ready', body, why: '' });
        return generation === ticket ? body : null;
      } catch (why) {
        if (generation === ticket) publish({ phase: 'failed', body: null, why: why instanceof Error ? why.message : String(why) });
        return null;
      }
    },
    stop() { generation += 1; controller?.abort(); }
  };
}

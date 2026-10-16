/**
 * THE DISK LINE ON /ingest (DB-B73, DB-B59; WP-81): free space under the store,
 * the store's bar bytes, what the census keeps forever, and whether a copy of
 * the store is recorded — read from `/disk.json`, which nothing showed before.
 * One drive held the raw vendor data and the whole store with no copy, and no
 * page said so.
 */

/** Bytes as the operator reads them, binary units, one decimal.
 * @param {unknown} bytes @returns {string} */
export function bytesText(bytes) {
  if (typeof bytes !== 'number' || !Number.isFinite(bytes) || bytes < 0) return '—';
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB'];
  let value = bytes, unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit += 1; }
  return unit === 0 ? `${value} B` : `${value.toFixed(1)} ${units[unit]}`;
}

/**
 * The tile's words from one `/disk.json` body. `alarm` is true when no copy is
 * recorded or the free space could not be measured: those are said in the
 * tile's own colour, never hidden behind a tooltip.
 * @param {any} body
 * @returns {{free:string, store:string, kept:string, backup:string, scrub:string, alarm:boolean, detail:string}}
 */
export function diskSummary(body) {
  const fs = body?.filesystem ?? {};
  const feeds = Array.isArray(body?.feeds) ? body.feeds : [];
  const bars = feeds.reduce((/** @type {number} */ sum, /** @type {any} */ feed) =>
    sum + (typeof feed?.bar_bytes_lower_bound === 'number' ? feed.bar_bytes_lower_bound : 0), 0);
  const measured = typeof fs.free_bytes === 'number';
  const recorded = body?.backup?.recorded === true;
  return {
    free: measured ? `${bytesText(fs.free_bytes)} free of ${bytesText(fs.total_bytes)}` : `free space unknown: ${fs.error ?? 'not measured'}`,
    store: `bars ≥ ${bytesText(bars)}`,
    kept: `census kept ${bytesText(body?.census_kept_bytes)}`,
    backup: recorded ? `last copy: ${String(body.backup.last)}` : 'NO COPY of this store is recorded',
    scrub: scrubText(feeds),
    alarm: !recorded || !measured || scrubAlarm(feeds),
    detail: [recorded ? '' : String(body?.backup?.why ?? ''), String(body?.retention ?? ''),
      String(body?.projection_why ?? '')].filter(Boolean).join(' · ')
  };
}

/** A recorded copy older than this many days is said on every page. */
export const STALE_COPY_DAYS = 7;

/**
 * THE STANDING LINE EVERY PAGE SHOWS (rules audit RA-163; DB-B73): one drive
 * holds the bought vendor files and the whole store, so while no copy is
 * recorded, the last one is older than {@link STALE_COPY_DAYS} days, or its
 * date cannot be read, every page says so — not only the /ingest disk line.
 * `null` only when a copy dated within the last seven days is recorded.
 *
 * The date is the receipt's own `copied=<UTC>` word (`docs/36-ops-runbook.md`
 * §4), read as written: a word that is no date, or a date after now, is said
 * as unreadable, never taken for a fresh copy.
 * @param {any} body one `/disk.json` answer
 * @param {number} nowMs the reader's clock, in milliseconds
 * @returns {{text: string, detail: string} | null}
 */
export function backupAlert(body, nowMs) {
  const backup = body?.backup;
  if (!backup || typeof backup !== 'object') {
    return { text: 'Whether a copy of this store exists is not known.',
      detail: '/disk.json answered without its backup field' };
  }
  if (backup.recorded !== true) {
    return { text: 'NO COPY of this store is recorded.', detail: String(backup.why ?? '') };
  }
  const last = String(backup.last ?? '');
  const word = last.split(' ').find((part) => part.startsWith('copied='))?.slice('copied='.length) ?? '';
  const at = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2})?Z$/.test(word) ? Date.parse(word) : NaN;
  if (!Number.isFinite(at) || !Number.isFinite(nowMs)) {
    return { text: 'The last copy of this store has a date that cannot be read.', detail: last };
  }
  if (at > nowMs) {
    return { text: 'The last copy of this store is dated after now.', detail: last };
  }
  const days = Math.floor((nowMs - at) / 86_400_000);
  if (days > STALE_COPY_DAYS) {
    return { text: `The last copy of this store is ${days} days old.`, detail: last };
  }
  return null;
}

/** The full-block scrub's line (DB-B74): per feed, never run, or its pass,
 * its months and what it found, from the totals the server parsed. A feed
 * never scrubbed is said by name: "nothing found" is not "never looked".
 * @param {any} feeds the answer's `feeds` @returns {string} */
export function scrubText(feeds) {
  const list = Array.isArray(feeds) ? feeds : [];
  if (list.length === 0) return 'blocks never scrubbed';
  return list.map((/** @type {any} */ feed) => {
    const name = String(feed?.feed ?? '?');
    const scrub = feed?.scrub;
    if (!scrub || scrub.recorded !== true) return `${name}: blocks never scrubbed`;
    const t = scrub.totals ?? {};
    const n = (/** @type {string} */ key) => (typeof t[key] === 'number' ? t[key] : '?');
    const state = scrub.state === 'complete' ? 'complete' : 'in progress';
    const found = [`${n('bad')} bad blocks`];
    if (t.unverifiable) found.push(`${t.unverifiable} unverifiable`);
    if (t.misfiled) found.push(`${t.misfiled} misfiled`);
    if (t.missing) found.push(`${t.missing} missing`);
    if (t.unbound) found.push(`${t.unbound} months bound by symbol hash only`);
    // MONTHS A STANDING PASS DID NOT READ (a writer held them: `busy`). The
    // totals above are what THIS pass found in the months it read; a block an
    // earlier pass found in a busy month still stands, so the count that
    // stands is said beside it and a pass with a busy month never reads clean.
    const busy = typeof scrub.standing_busy === 'number' ? scrub.standing_busy : Number(t.busy) || 0;
    if (busy > 0) {
      const standing = typeof scrub.standing_findings === 'number' ? scrub.standing_findings : '?';
      found.push(`${busy} months NOT READ (busy), ${standing} findings standing with those kept from earlier passes`);
    }
    // A HALF-BUILT INDEX COUNTS ONLY WHAT IT HAS REACHED: said, never served
    // as the whole.
    const partial = scrub.truncated === true ? ' (findings still being indexed after a restart: counts may be low)' : '';
    return `${name}: scrub pass ${scrub.pass} ${state}, ${n('months')} months, ${found.join(', ')}${partial}`;
  }).join(' · ');
}

/** Whether any feed's scrub holds a finding that still stands.
 * @param {any} feeds @returns {boolean} */
export function scrubAlarm(feeds) {
  return (Array.isArray(feeds) ? feeds : []).some((/** @type {any} */ feed) =>
    feed?.scrub?.recorded === true && Number(feed.scrub.standing_findings) > 0);
}

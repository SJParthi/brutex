// EVERY STRING FROM THE SERVER IS TEXT, NEVER MARKUP.
//
// A refusal quotes the first bytes a master host answered, and an attempt's
// detail and an index map's symbols come from third parties too. They were
// interpolated into `innerHTML`, so a host answering `x,<img src=x onerror=…>`
// ran script in the operator's console (CE-25, D-1768). Every cell is now built
// with `createElement` and `textContent`, the way `typeahead.js` builds its list.
const say = (t) => { document.getElementById('say').textContent = t; };
const cell = (file, klass) => document.querySelector(`tr[data-file="${file}"] .${klass}`);

const when = (ms) => ms ? new Date(ms).toLocaleString() : '—';

/** One element with a class and text content; children appended in order. */
function el(tag, klass, text, ...children) {
  const node = document.createElement(tag);
  if (klass) node.className = klass;
  if (text !== undefined && text !== null) node.textContent = String(text);
  for (const child of children) if (child) node.appendChild(child);
  return node;
}

function fill(node, ...children) {
  node.replaceChildren(...children.filter(Boolean));
}

async function status() {
  try {
    const r = await fetch('/masters/status.json');
    const d = await r.json();
    for (const m of (d.masters || [])) {
      const c = cell(m.file, 'disk');
      if (!c) continue;
      if (!m.present) { fill(c, el('span', 'bad', 'absent')); continue; }
      fill(
        c,
        el('span', 'ok', 'present'),
        el('div', 'sub', `${m.bytes} bytes`),
        el('div', 'sub', when(m.modified_unix_millis)),
        m.newer_than_parse
          ? el('div', 'sub bad', 'newer than this server’s parse — restart required')
          : null
      );
    }
  } catch (e) { say('Could not read what is on disk: ' + e); }
}

function ledger(rows) {
  const box = document.getElementById('ledger');
  box.replaceChildren();
  for (const m of rows) {
    if (!m.attempts || !m.attempts.length) continue;
    const steps = el('ol');
    for (const a of m.attempts) {
      const state = a.status === null ? 'no answer' : a.status;
      const waited = a.waited_ms ? ` after waiting ${a.waited_ms} ms` : '';
      const why = a.detail ? ` — ${a.detail}` : '';
      steps.appendChild(el('li', null, `#${a.number} ${a.got} (${state})${waited}${why}`));
    }
    box.appendChild(
      el('div', 'att', null,
        el('h3', null, `${m.file} — ${m.attempts.length} step(s), ${m.waited_ms} ms waited`),
        steps)
    );
  }
}

async function refresh() {
  const go = document.getElementById('go');
  go.disabled = true;
  say('Asking four hosts. A refused source is retried on a backoff, so this can take a minute.');
  for (const m of document.querySelectorAll('.out')) m.textContent = 'asking…';
  try {
    const r = await fetch('/masters/refresh', { method: 'POST' });
    const d = await r.json();
    if (d.refusal) { say('Refused before anything was asked: ' + d.refusal); return; }
    const rows = d.landed || [];
    for (const m of rows) {
      const c = cell(m.file, 'out');
      if (!c) continue;
      if (m.written) {
        fill(c, el('span', 'ok', m.changed ? 'updated' : 'unchanged'), el('div', 'sub', `${m.bytes} bytes`));
      } else if (m.skipped) {
        fill(c, el('span', 'skip', 'skipped'), el('div', 'sub', m.refusal || ''));
      } else {
        fill(c, el('span', 'bad', 'refused'), el('div', 'sub', m.refusal || ''));
      }
    }
    ledger(rows);
    // A REFRESH THAT DID NOT RELOAD IS NOT A SUCCESS. The route answers 502
    // with `reloaded:false` and the reason in `universe` when the new files
    // were refused on re-parse, and this page used to say "All four are on
    // disk" over it, reading a `restart_required` this route always sends
    // false (CE-26, D-1768).
    const missing = d.missing || [];
    if (!r.ok || d.reloaded === false) {
      say(`The refresh did not finish (HTTP ${r.status}). `
        + (missing.length ? `${missing.length} master(s) still missing: ${missing.join(', ')}. ` : '')
        + (d.reloaded === false
          ? `The server is still answering from its previous masters: ${d.universe || 'no reason given'}. `
            + 'Restart the server only after the files on disk parse; until then a restart reads the same refusal.'
          : (d.universe || '')));
    } else {
      say(missing.length
        ? `${missing.length} master(s) still missing: ${missing.join(', ')}.`
        : `All four are on disk and the server reloaded them. ${d.universe || ''}`);
    }
    await status();
  } catch (e) {
    say('The refresh call itself failed: ' + e);
  } finally {
    go.disabled = false;
  }
}

document.getElementById('go').addEventListener('click', refresh);

// ---- the cross-verification, which is what the four files are FOR ----
//
// `/indexmap.json?feed=X` joins the exchange's own index catalogue against
// one feed's index symbols and reports every row, including the ones it
// could not resolve — those are the symbols whose bars are being filed under
// a name no exchange confirms. It had no page and no nav entry, so the only
// way to see it was to type the URL.
async function verify() {
  const box = document.getElementById('xverify');
  fill(box, el('div', 'sub', 'joining…'));
  const feeds = ['dhan', 'groww', 'zerodha'];
  const body = el('tbody');
  const failed = (feed, why) => {
    const td = el('td', 'bad', why);
    td.colSpan = 5;
    body.appendChild(el('tr', null, null, el('td', null, null, el('b', null, feed)), td));
  };
  for (const feed of feeds) {
    try {
      const r = await fetch(`/indexmap.json?feed=${feed}`);
      const d = await r.json();
      if (d.error) { failed(feed, d.error); continue; }
      const refused = d.refused || 0;
      const names = (d.rows || [])
        .filter(x => x.nse === null || x.nse === undefined)
        .map(x => x.symbol);
      body.appendChild(el('tr', null, null,
        el('td', null, null, el('b', null, feed)),
        el('td', null, d.published),
        el('td', null, d.listed),
        el('td', 'ok', d.resolved),
        el('td', refused ? 'bad' : 'ok', refused),
        el('td', 'sub', names.slice(0, 12).join(', ')
          + (names.length > 12 ? ` … and ${names.length - 12} more` : ''))));
    } catch (e) {
      failed(feed, String(e));
    }
  }
  const head = el('tr');
  for (const h of ['Feed', 'NSE publishes', 'Feed lists', 'Resolved', 'Unconfirmed', 'Which ones']) {
    head.appendChild(el('th', null, h));
  }
  fill(box,
    el('h3', null, 'Cross‑verification — every feed’s index symbols against NSE’s own catalogue'),
    el('table', 'xv', null, el('thead', null, null, head), body),
    el('div', 'sub', 'An unconfirmed symbol is one whose bars are filed under a name '
      + 'no exchange confirms. It is never filtered out.'));
}

document.getElementById('verify').addEventListener('click', verify);
status();

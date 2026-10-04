const say = (t) => { document.getElementById('say').textContent = t; };
// EVERY STRING THAT REACHES innerHTML GOES THROUGH THIS. `detail` can carry up
// to 500 characters of a vendor's own response body, `refusal` and `error` are
// server sentences that quote vendor text, and `symbol` comes from vendor
// instrument masters. Markup in any of them would run on this origin
// (audit-20261003 webcontract-2, D-1585).
const esc = (v) => String(v ?? '').replace(/[&<>"']/g, (c) => ({
  '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
}[c]));
const cell = (file, klass) => document.querySelector(`tr[data-file="${file}"] .${klass}`);

const when = (ms) => ms ? new Date(ms).toLocaleString() : '—';

async function status() {
  try {
    const r = await fetch('/masters/status.json');
    const d = await r.json();
    for (const m of (d.masters || [])) {
      const c = cell(m.file, 'disk');
      if (!c) continue;
      if (!m.present) { c.innerHTML = '<span class="bad">absent</span>'; continue; }
      const stale = m.newer_than_parse
        ? '<div class="sub bad">newer than this server’s parse — restart required</div>'
        : '';
      c.innerHTML = `<span class="ok">present</span><div class="sub">${esc(m.bytes)} bytes</div>`
        + `<div class="sub">${esc(when(m.modified_unix_millis))}</div>${stale}`;
    }
  } catch (e) { say('Could not read what is on disk: ' + e); }
}

function ledger(rows) {
  const box = document.getElementById('ledger');
  box.innerHTML = '';
  for (const m of rows) {
    if (!m.attempts || !m.attempts.length) continue;
    const el = document.createElement('div');
    el.className = 'att';
    const steps = m.attempts.map(a => {
      const status = a.status === null ? 'no answer' : a.status;
      const waited = a.waited_ms ? ` after waiting ${esc(a.waited_ms)} ms` : '';
      const why = a.detail ? ` — ${esc(a.detail)}` : '';
      return `<li>#${esc(a.number)} ${esc(a.got)} (${esc(status)})${waited}${why}</li>`;
    }).join('');
    el.innerHTML = `<h3>${esc(m.file)} — ${m.attempts.length} step(s), `
      + `${esc(m.waited_ms)} ms waited</h3><ol>${steps}</ol>`;
    box.appendChild(el);
  }
}

// WHAT THE REFRESH ACTUALLY DID, from the fields the route sends. The route
// re-parses the masters into the running server (`reload`) and answers 502 with
// `reloaded:false` and the refusal in `universe` when that re-parse fails; the
// page used to read none of that and print "All four are on disk." over a
// universe that was NOT replaced (audit-20261003 webcontract-3, D-1585).
function outcome(r, d) {
  const missing = d.missing || [];
  const parts = [];
  if (missing.length) parts.push(`${missing.length} master(s) still missing: ${missing.join(', ')}.`);
  else parts.push('All four are on disk.');
  if (d.reloaded === true) {
    parts.push('The running server re-parsed them; every page now answers from the new masters.');
  } else {
    const why = typeof d.universe === 'string' && d.universe ? ` ${d.universe}` : '';
    parts.push(`The running server did NOT re-parse them, so every page still answers from the previous parse.${why} Fix the cause and refresh again, or Restart the server once it is fixed.`);
  }
  if (!r.ok && d.reloaded === true && !missing.length) {
    parts.push(`The refresh answered ${r.status}: at least one source was refused; see the rows above.`);
  }
  return parts.join(' ');
}

async function refresh() {
  const go = document.getElementById('go');
  go.disabled = true;
  say('Asking four hosts. A refused source is retried on a backoff, so this can take a minute.');
  for (const m of document.querySelectorAll('.out')) m.textContent = 'asking…';
  try {
    const r = await fetch('/masters/refresh', { method: 'POST' });
    const d = await r.json();
    if (d.refusal) { say('Refused before anything was asked: ' + d.refusal); go.disabled = false; return; }
    const rows = d.landed || [];
    for (const m of rows) {
      const c = cell(m.file, 'out');
      if (!c) continue;
      if (m.written) {
        c.innerHTML = `<span class="ok">${m.changed ? 'updated' : 'unchanged'}</span>`
          + `<div class="sub">${esc(m.bytes)} bytes</div>`;
      } else if (m.skipped) {
        c.innerHTML = `<span class="skip">skipped</span><div class="sub">${esc(m.refusal)}</div>`;
      } else {
        c.innerHTML = `<span class="bad">refused</span><div class="sub">${esc(m.refusal)}</div>`;
      }
    }
    ledger(rows);
    say(outcome(r, d));
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
  box.innerHTML = '<div class="sub">joining…</div>';
  const feeds = ['dhan', 'groww', 'zerodha'];
  const parts = [];
  for (const feed of feeds) {
    try {
      const r = await fetch(`/indexmap.json?feed=${feed}`);
      const d = await r.json();
      if (d.error) {
        parts.push(`<tr><td><b>${feed}</b></td><td colspan="5" class="bad">${esc(d.error)}</td></tr>`);
        continue;
      }
      const refused = d.refused || 0;
      const names = (d.rows || [])
        .filter(x => x.nse === null || x.nse === undefined)
        .map(x => esc(x.symbol));
      parts.push(
        `<tr><td><b>${feed}</b></td>`
        + `<td>${esc(d.published)}</td><td>${esc(d.listed)}</td>`
        + `<td class="ok">${esc(d.resolved)}</td>`
        + `<td class="${refused ? 'bad' : 'ok'}">${esc(refused)}</td>`
        + `<td class="sub">${names.slice(0, 12).join(', ')}`
        + `${names.length > 12 ? ` … and ${names.length - 12} more` : ''}</td></tr>`
      );
    } catch (e) {
      parts.push(`<tr><td><b>${feed}</b></td><td colspan="5" class="bad">${esc(e)}</td></tr>`);
    }
  }
  box.innerHTML =
    '<h3>Cross&#8209;verification — every feed’s index symbols against NSE’s own catalogue</h3>'
    + '<table class="xv"><thead><tr><th>Feed</th><th>NSE publishes</th><th>Feed lists</th>'
    + '<th>Resolved</th><th>Unconfirmed</th><th>Which ones</th></tr></thead><tbody>'
    + parts.join('') + '</tbody></table>'
    + '<div class="sub">An unconfirmed symbol is one whose bars are filed under a name '
    + 'no exchange confirms. It is never filtered out.</div>';
}

document.getElementById('verify').addEventListener('click', verify);
status();

const say = (t) => { document.getElementById('say').textContent = t; };
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
      c.innerHTML = `<span class="ok">present</span><div class="sub">${m.bytes} bytes</div>`
        + `<div class="sub">${when(m.modified_unix_millis)}</div>${stale}`;
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
      const waited = a.waited_ms ? ` after waiting ${a.waited_ms} ms` : '';
      const why = a.detail ? ` — ${a.detail}` : '';
      return `<li>#${a.number} ${a.got} (${status})${waited}${why}</li>`;
    }).join('');
    el.innerHTML = `<h3>${m.file} — ${m.attempts.length} step(s), `
      + `${m.waited_ms} ms waited</h3><ol>${steps}</ol>`;
    box.appendChild(el);
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
    if (d.refusal) { say('Refused before anything was asked: ' + d.refusal); go.disabled = false; return; }
    const rows = d.landed || [];
    for (const m of rows) {
      const c = cell(m.file, 'out');
      if (!c) continue;
      if (m.written) {
        c.innerHTML = `<span class="ok">${m.changed ? 'updated' : 'unchanged'}</span>`
          + `<div class="sub">${m.bytes} bytes</div>`;
      } else if (m.skipped) {
        c.innerHTML = `<span class="skip">skipped</span><div class="sub">${m.refusal || ''}</div>`;
      } else {
        c.innerHTML = `<span class="bad">refused</span><div class="sub">${m.refusal || ''}</div>`;
      }
    }
    ledger(rows);
    const missing = d.missing || [];
    say(missing.length
      ? `${missing.length} master(s) still missing: ${missing.join(', ')}.`
      : (d.restart_required
          ? 'All four are on disk. Restart the server so the new masters are parsed.'
          : 'All four are on disk.'));
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
        parts.push(`<tr><td><b>${feed}</b></td><td colspan="5" class="bad">${d.error}</td></tr>`);
        continue;
      }
      const refused = d.refused || 0;
      const names = (d.rows || [])
        .filter(x => x.nse === null || x.nse === undefined)
        .map(x => x.symbol);
      parts.push(
        `<tr><td><b>${feed}</b></td>`
        + `<td>${d.published}</td><td>${d.listed}</td>`
        + `<td class="ok">${d.resolved}</td>`
        + `<td class="${refused ? 'bad' : 'ok'}">${refused}</td>`
        + `<td class="sub">${names.slice(0, 12).join(', ')}`
        + `${names.length > 12 ? ` … and ${names.length - 12} more` : ''}</td></tr>`
      );
    } catch (e) {
      parts.push(`<tr><td><b>${feed}</b></td><td colspan="5" class="bad">${e}</td></tr>`);
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

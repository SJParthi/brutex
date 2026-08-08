// brutex — instrument type-ahead.
//
// PROGRESSIVE ENHANCEMENT, NOT A RENDERER. Every row this page shows is
// rendered by `api::render` before any of this runs. With scripting off the
// page is exactly what it was: a form, a Search button, a server-rendered
// table. This file only makes the search answer while you type.
//
// The rule it must never break: nothing here produces a row of DATA. It
// filters rows the server already sent, and it navigates. A page that renders
// a row ONLY from client-side data has moved rendering out of Rust, which
// D-0052 does not license.
//
// O(1) PER KEYSTROKE, and that is the whole reason the index ships to the
// browser. The universe is bounded at ~800 instruments (750 NIFTY Total Market
// + ~35 NSE indices), so the entire searchable set is a few tens of kilobytes.
// One request at load, then every keystroke is a prefix probe into a Map — no
// network, no debounce-and-hope, no per-character round-trip. A request per
// character would be ~800 requests to type one symbol and would make the
// keystroke latency a function of the network.
//
// NO innerHTML, ANYWHERE. Every node below is built with createElement and
// textContent. `core::symbol::Symbol` admits `&`, and a name carrying one is a
// real instrument (M&MFIN, M&M) — concatenating it into a markup string is how
// a legal symbol becomes an injection. textContent cannot be escaped out of,
// so the question does not arise rather than being answered carefully.

(() => {
  "use strict";

  const input = document.querySelector('input[name="q"], input[type="search"]');
  if (!input) return;

  // ---- the index -------------------------------------------------------
  // Built once. `byPrefix` maps every 1..4-character prefix of every symbol to
  // the rows starting with it, so the common case — one to four characters
  // typed — is a single Map probe rather than a scan of 800. Beyond four the
  // candidate set is already tiny and filtering it costs less than the memory
  // a deeper index would.
  const MAX_PREFIX = 4;
  const LIMIT = 12;
  let byPrefix = new Map();

  function index(list) {
    const next = new Map();
    for (const row of list) {
      if (typeof row.symbol !== "string") continue;
      const s = row.symbol;
      for (let n = 1; n <= Math.min(MAX_PREFIX, s.length); n += 1) {
        const key = s.slice(0, n);
        let bucket = next.get(key);
        if (!bucket) {
          bucket = [];
          next.set(key, bucket);
        }
        bucket.push(row);
      }
    }
    byPrefix = next;
  }

  function lookup(typed) {
    // The stored side is ALREADY upper-case — `core::symbol::Symbol` admits
    // only [A-Z0-9-_&] and upper-cases at construction — so only the
    // operator's typing is folded, and it is folded once, not per row.
    const q = typed.trim().toUpperCase();
    if (!q) return [];
    if (q.length <= MAX_PREFIX) return byPrefix.get(q) || [];
    const seed = byPrefix.get(q.slice(0, MAX_PREFIX)) || [];
    return seed.filter((r) => r.symbol.startsWith(q));
  }

  // ---- the panel -------------------------------------------------------
  const panel = document.createElement("div");
  panel.className = "ta-panel";
  panel.setAttribute("role", "listbox");
  panel.hidden = true;
  input.setAttribute("autocomplete", "off");
  input.setAttribute("aria-autocomplete", "list");
  input.parentElement.style.position = "relative";
  input.parentElement.appendChild(panel);

  let active = -1;
  let shown = [];

  function span(cls, text) {
    const el = document.createElement("span");
    el.className = cls;
    el.textContent = text == null ? "" : String(text);
    return el;
  }

  function render(matches) {
    shown = matches.slice(0, LIMIT);
    active = -1;
    input.removeAttribute("aria-activedescendant");
    panel.replaceChildren();
    if (shown.length === 0) {
      panel.hidden = true;
      return;
    }
    shown.forEach((row, i) => {
      const a = document.createElement("a");
      a.className = "ta-row";
      a.id = `ta-${i}`;
      a.setAttribute("role", "option");
      // SAME-ORIGIN ONLY. `href` arrives as data; a value of `javascript:…`
      // would otherwise be one click from executing. Resolved against this
      // origin and refused if it leaves — the server has no reason to send an
      // off-site link here, so a link that is one is not honoured.
      let href = "#";
      try {
        const url = new URL(String(row.href ?? ""), window.location.origin);
        if (url.origin === window.location.origin) href = url.pathname + url.search;
      } catch {
        // A malformed href is left as "#", which navigates nowhere.
      }
      a.setAttribute("href", href);
      a.append(span("ta-sym", row.symbol), span("ta-key", row.key), span("ta-kind", row.kind));
      panel.appendChild(a);
    });
    panel.hidden = false;
  }

  function move(delta) {
    if (shown.length === 0) return;
    const cells = panel.querySelectorAll(".ta-row");
    if (active >= 0) cells[active].classList.remove("is-active");
    active = (active + delta + shown.length) % shown.length;
    cells[active].classList.add("is-active");
    cells[active].scrollIntoView({ block: "nearest" });
    input.setAttribute("aria-activedescendant", `ta-${active}`);
  }

  input.addEventListener("input", () => render(lookup(input.value)));

  input.addEventListener("keydown", (e) => {
    if (panel.hidden) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      move(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      move(-1);
    } else if (e.key === "Enter" && active >= 0) {
      // Only when a suggestion is HIGHLIGHTED. With nothing highlighted, Enter
      // submits the form exactly as it does with scripting off — the
      // server-rendered search stays the fallback, not a casualty.
      e.preventDefault();
      panel.querySelectorAll(".ta-row")[active].click();
    } else if (e.key === "Escape") {
      panel.hidden = true;
    }
  });

  document.addEventListener("click", (e) => {
    if (!panel.contains(e.target) && e.target !== input) panel.hidden = true;
  });

  // ---- load ------------------------------------------------------------
  // One request, at load, for the whole bounded universe. If it fails the page
  // keeps working: the form and the Search button are untouched, so the
  // failure costs the type-ahead and nothing else. Logged rather than
  // swallowed — a feature that quietly does not exist is worse than one that
  // says why.
  fetch("/instruments.json")
    .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`))))
    .then((list) => index(Array.isArray(list) ? list : []))
    .catch((why) => {
      console.warn("brutex: type-ahead index unavailable, search still works —", why);
    });
})();

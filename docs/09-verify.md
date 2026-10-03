# Verifying this repository without trusting anyone

Every claim about this console is checkable from the repository in under two
minutes, apart from the build and test gates, which take as long as a cold
build takes. Nothing here starts a server, and nothing here spends vendor
quota.

Run these from the repository root. **Exit 0 on all six gates in §1, and the
printed output each later section asks for, means it is safe to press Run
api.** A `grep -c` that counts nothing prints `0` and exits `1`; for those
checks the printed count is the check, not the exit status.

This page was re-run on 3 October 2026 on the tree that carries D-1941
(P1-18-01).
Every command below was run there, and every expected output stated here is
what it printed, except where a section says otherwise.

---

## 1. The six gates

```
cargo build --workspace
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
npm --prefix web run build
find web/src -type f -newer web/build/index.html
```

The last one must print **nothing**. It is the one people forget, and it is the
one that bites: `web/build` is committed output (D-0068) and the binary serves
it from disk without ever compiling it — CLAUDE.md §2 forbids a crate depending
on the front-end toolchain, and CI gate 1e enforces that by building with `web/`
moved aside. So a source file newer than the bundle means the page you are
looking at is one revision old, silently. Rebuild before believing a screen.

Two limits, both measured on the re-run. `npm --prefix web run build` needs the
front end's packages installed first (`npm --prefix web ci`, from
`web/package-lock.json`); in a checkout without `web/node_modules` it exits
`127` with `vite: not found`, and that is a missing install, not a passing
gate. And the `find` check compares modification times, which a fresh `git`
checkout or worktree sets in checkout order rather than edit order: on the
re-run it printed three route pages in a worktree where nothing had been
edited. It is meaningful only in a working copy whose files you have been
editing.

## 2. That no crate needs Node

```
find crates -name build.rs | grep -vx 'crates/cli/build.rs'
grep -rlE 'Command::new\("(npm|npx|node|yarn|pnpm)"' crates/*/src
```

Both must print nothing. A hit in either is a §2 build failure, not a warning.

`crates/cli/build.rs` is excluded **by name** because it exists on purpose: it
stamps `BRUTEX_COMMIT` from git's own files so a plain `cargo run` can record a
run, its header records that it is CI gate 13 layer 3's allowlist escape taken
with a decisions entry, and it starts no process. This section used to say
`find crates -name build.rs` must print nothing, which failed at HEAD on that
file. Any other `build.rs` is still a failure.

## 3. That the shared dropdown can theme itself

```
grep -cE -- '--(raise|panel2|accdeep)[[:space:]]*:' web/src/lib/theme.css
```

Must print **3**. `web/src/lib/Picker.svelte` reads those three names at four
call sites with hardcoded dark-hex fallbacks. When they were undefined its menus
rendered dark on a light page on every route, and nothing failed — the fallback
is the feature, which is exactly why the bug survived.

## 4. That no vendor fact exists twice

```
grep -cE 'feeds\.active *=[^=]' web/src/routes/terminal/+page.svelte
grep -cE 'feeds\.active *=[^=]' web/src/routes/markets/+page.svelte
grep -cE 'feeds\.active *=[^=]' web/src/routes/ingest/+page.svelte
grep -cE 'feeds\.active *=[^=]' web/src/routes/db/+page.svelte
```

All four must print **0**: no page assigns the active feed. The one writer is
`web/src/lib/feed-startup.js`, behind `selectFeed` in
`web/src/lib/feeds.svelte.js`; the top bar and each page's jump-to-feed control
call that function rather than writing the value themselves.

The pattern matches an assignment and not a comparison. It used to be
`feeds.active *=`, which also matches `feeds.active ===`, and so counted
`/ingest`'s `feeds.active === 'zerodha'` test as a second picker. It also named
`web/src/routes/+page.svelte`, which no longer exists: `routes/` holds a
`+page.js` that redirects to `/terminal`. The pattern is not a parser. Over all
of `web/src/routes` it has one hit, in `/backtest`, and that hit is inside a
comment explaining why the page does not write `feeds.active = undefined`.

## 5. That nothing is invented

```
grep -cE 'blackScholes|normCdf' web/src/routes/db/+page.svelte
awk '/^<style/,/^<\/style>/' web/src/routes/markets/+page.svelte | grep -coE '#[0-9a-fA-F]{3,8}'
awk '/^<style/,/^<\/style>/' web/src/routes/db/+page.svelte | grep -coE '#[0-9a-fA-F]{3,8}'
```

All three must print **0**. No Greek is computed — ten columns read `no source`
instead. No raw hex appears in those pages' `<style>`; a literal there is a
second theme the toggle cannot reach.

**This check does not hold for every page, and it is not claimed to.** The same
`awk` over `web/src/routes/terminal/+page.svelte` prints **47**: that page
declares its own palette as custom properties at the top of its `<style>`, and
the comment above them says each value was sampled from source captures. Over
`web/src/routes/ingest/+page.svelte` it prints **3**, all fallbacks of the form
`var(--line, #2a333c)`. Neither is listed above as a passing check. This section
used to read `web/src/routes/+page.svelte`, which no longer exists, so it checked
nothing.

## 6. That the index mapping is keyed on identity, not on a name

```
grep -cE 'INE[0-9A-Z]{9}' crates/core/src/universe.rs
grep -c 'nse_isin' crates/api/src/constituents.rs
```

The first is the count of exchange-issued ISINs carried in the build — **1812**
on the re-run; the second must be non-zero, meaning the join reads NSE's own
ISIN rather than looking a published name up by symbol — **25** on the re-run.
`docs/06-limits.md` §62 records why that mattered and when it was closed.

---

## What none of this proves

**The pixels.** Every check above reads source, compiled output, or rendered
DOM. None of them looks at a screen. A page can pass all six and still be ugly,
misaligned, or unreadable in one theme. If a claim in this repository sounds
like "it looks right", ask what was measured — and if the answer is a DOM
outline, that is a strong check and it is not the same as seeing it.

# Verifying this repository without trusting anyone

Every claim about this console is checkable from the repository in under two
minutes. Nothing here starts a server, and nothing here spends vendor quota.

Run these from the repository root. **Exit 0 on all six means it is safe to
press Run api.**

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

## 2. That no crate needs Node

```
find crates -name build.rs
grep -rlE 'Command::new\("(npm|npx|node|yarn|pnpm)"' crates/*/src
```

Both must print nothing. A hit in either is a §2 build failure, not a warning.

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
grep -c 'feeds.active *=' web/src/routes/+page.svelte
grep -c 'feeds.active *=' web/src/routes/ingest/+page.svelte
```

Both must print **0**: the feed is chosen once, in the top bar. `/db` prints 2,
and those two are deliberate — they are jump-to-the-feed-that-has-this buttons
that only render in terminal empty states, not a second picker.

## 5. That nothing is invented

```
grep -cE 'blackScholes|normCdf' web/src/routes/db/+page.svelte
awk '/^<style/,/^<\/style>/' web/src/routes/+page.svelte | grep -coE '#[0-9a-fA-F]{3,8}'
```

Both must print **0**. No Greek is computed — ten columns read `no source`
instead. No raw hex appears in a page's `<style>`; a literal there is a second
theme the toggle cannot reach.

## 6. That the index mapping is keyed on identity, not on a name

```
grep -cE 'INE[0-9A-Z]{9}' crates/core/src/universe.rs
grep -c 'nse_isin' crates/api/src/constituents.rs
```

The first is the count of exchange-issued ISINs carried in the build; the second
must be non-zero, meaning the join reads NSE's own ISIN rather than looking a
published name up by symbol. `docs/06-limits.md` §62 records why that mattered
and when it was closed.

---

## What none of this proves

**The pixels.** Every check above reads source, compiled output, or rendered
DOM. None of them looks at a screen. A page can pass all six and still be ugly,
misaligned, or unreadable in one theme. If a claim in this repository sounds
like "it looks right", ask what was measured — and if the answer is a DOM
outline, that is a strong check and it is not the same as seeing it.

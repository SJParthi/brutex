<script>
  /**
   * INGEST — ask for a window, watch it happen, read every outcome.
   *
   * THE THREE THINGS THIS PAGE EXISTS TO FIX
   *
   * 1. A pull takes up to 32 minutes and used to show NOTHING until it
   *    finished. A working pull and a hung one looked identical, so the only
   *    way to tell them apart was to wait half an hour and find out.
   *
   * 2. A run that refused 407 of 785 instruments showed ONE reason. The
   *    receipt renders `done.failures.iter().take(5)` — five, and the run's
   *    own `Members failed` count is beside it, so the page can SEE that it
   *    was truncated and say so instead of implying the list is complete.
   *
   * 3. The bar length was not askable. `api::ingest::parse_spot` has read a
   *    `granularity` field all along; the server's own form never emitted one,
   *    so every request meant `1min` by omission and nobody could see that
   *    from the page.
   *
   * WHAT IS MEASURED AND WHAT IS ESTIMATED, AND THE PAGE NEVER BLURS THEM
   *
   * There is no pull-status endpoint. `crates/api/src/server.rs`'s router is
   * `/`, `/instruments`, `/instruments.json`, `/feeds.json`, `/bars.json`,
   * `/store.json`, `/typeahead.js`, `/pull`, `/pull/spot`, `/pull/fno`,
   * `/audit`, `/store`, `/health` — and nothing there answers "how far along
   * is the run". So this page does NOT invent a progress bar out of a timer.
   *
   * It polls `/store.json`, which the server rebuilds per request — its own
   * comment says "FRESH, NOT THE STARTUP SNAPSHOT" — and reports the growth it
   * measures. Instrument-months that gained rows are a FACT. The share of the
   * request they represent is an ESTIMATE, because the denominator is
   * arithmetic on what was asked for rather than a number the server states,
   * and the page labels it as one. A remaining time appears only once a rate
   * has actually been observed, and it is labelled an extrapolation.
   *
   * `CLAUDE.md` §3 rule 6: never claim a measurement not taken, and label
   * extrapolations as extrapolations.
   */
  import { feeds } from '$lib/feeds.svelte.js';
  import Picker from '$lib/Picker.svelte';
  import { catalogue } from '$lib/index.svelte.js';
  import { MON, dayLabel, monthLabel, stampLabel } from '$lib/dates.js';
  // `untrack`, because one effect on this page must react to a FEED CHANGE and
  // to nothing else — see the rung-drop effect for why an effect that reacts to
  // its own write is a hazard rather than a nicety.
  import { onMount, untrack } from 'svelte';
  import { store, syncStore, refreshStore, watchStore, foldMonths } from '$lib/store.svelte.js';
  import { notAReceipt, RECEIPT_HEADER } from '$lib/receipt.js';

  // ---------------------------------------------------------------- constants

  /**
   * THE LADDER'S ORDER, AND THE ORDER ONLY.
   *
   * `pull::vendor::Granularity::ALL` in its declared sequence, which ASCENDS
   * with coarseness. That is not an accident anybody may rely on quietly:
   * `crates/pull/src/vendor.rs` pins it with a `const` block that destructures
   * all eleven variants and asserts every discriminant, so a rung inserted in
   * the middle of the ladder is a BUILD FAILURE there before it is a wrong
   * answer here.
   *
   * IT CARRIES NOTHING BUT NAMES — no label, no store flag, no session count.
   * Those are facts about a rung that can go stale against the Rust, and this
   * list exists for one thing: making "is this rung finer than that one" ONE
   * integer comparison rather than a walk, the same comparison
   * `Granularity::is_finer_than` makes on the discriminant.
   *
   * IT IS ALL ELEVEN WHILE `RUNGS` BELOW IS THREE, AND THE SPLIT IS LOAD
   * BEARING. `/feeds.json` states a feed's floor with any rung name the enum
   * can spell, and a floor this file cannot PLACE is a floor it cannot compare
   * against: `finerThan` answers `false` for a name missing from the map, so a
   * rung BELOW an unplaceable floor would be drawn live and offered. The order
   * is therefore total over every name the wire can send, and what the control
   * OFFERS is a separate decision, made below and for separate reasons.
   */
  const LADDER = [
    'tick',
    '1s',
    '5s',
    '1min',
    '3min',
    '5min',
    '15min',
    '30min',
    '60min',
    '1day',
    '1week'
  ];
  /** O(1) per lookup, built once at module scope. It does not scan. */
  const RUNG_RANK = new Map(LADDER.map((dir, i) => [dir, i]));
  /**
   * Strictly finer — `a` sits nearer the event end of the ladder than `b`.
   *
   * `false` when either name is not on the ladder, which is the only honest
   * answer: an unplaceable rung is not finer and is not coarser, and returning
   * either would be an ordering claim about a name this page cannot place.
   */
  function finerThan(a, b) {
    const x = RUNG_RANK.get(a);
    const y = RUNG_RANK.get(b);
    return x !== undefined && y !== undefined && x < y;
  }

  /**
   * WHAT THE CONTROL OFFERS: ONE MINUTE, ONE DAY, AND THE ARCHIVES' SECOND.
   *
   * THE OWNER'S RULE, 14 Aug 2026: *"under timeframes just show one minute and
   * one day alone, and one and only for TrueData or GDFL alone ticks should be
   * displayed along with this. Even dynamic timeframes adding is not needed —
   * because after pulling one day or one min or ticks for TrueData or GDFL,
   * always we will do the internal calculations."*
   *
   * Eleven rows stood here. Three are drawn now, and the other eight were never
   * rungs anybody could pull:
   *
   *   NO FEED IN THIS BUILD DECLARES ONE OF THEM. `Descriptor::granularities`
   *   is Dhan `{1day}`, Groww `{1min, 1day}`, `TrueData` `{1s, 1min}`, GDFL
   *   `{1s}` — and the union of all four is EXACTLY the three rows below. Every
   *   other rung was drawn live and annotated "not fetched by this build", on
   *   every feed, forever. `crates/api`'s own no-JS form has said so all along:
   *   `render::granularity_select` offers a rung only where some feed declares
   *   it AND the store can file it, which is `1min` and `1day`. This control was
   *   the only surface in the product drawing eleven.
   *
   *   AND COARSER IS COMPUTED, NOT BOUGHT. Five minutes is a fold of five
   *   one-minute bars this repository already holds. Asking a vendor for it
   *   spends a request, a quota and a window on arithmetic `crates/pull`
   *   performs for nothing — which is the owner's second sentence, and it is
   *   why "add a timeframe" is not a control that is missing.
   *
   * THREE NAMES FOR ONE RUNG, AND EACH ONE ANSWERS A DIFFERENT QUESTION.
   *
   *   `dir`     THE WIRE, THE PATH AND THE PARSER. `Granularity::dir()` emits
   *             it, `api::ingest::parse_granularity` matches it, and the store
   *             writes a directory of that name. `CLAUDE.md` §3 rule 8 makes it
   *             append-only history: it is never reworded, and the detail column
   *             prints it beside every row so the operator can always see what
   *             his tick actually sends.
   *   `label`   THE WORD ON THE BUTTON, and it is the OPERATOR'S vocabulary.
   *   `phrase`  THE RUNG INSIDE A SENTENCE, and it is the MEASURED vocabulary.
   *
   * THE OWNER'S RULE, 14 Aug 2026: *"anyhow inside the CSV files the downloaded
   * data will be marked as ticks, right — so let us just keep it as ticks.
   * Meanwhile internally let us make it as seconds and minutes."* That is
   * exactly this split, and it is the split `crates/pull` already makes:
   * `Granularity::label` is "prose and may be reworded freely" while
   * `Granularity::dir` "may not". So the second rung is labelled **Ticks**,
   * because that is what the vendor marks the files he bought and what he calls
   * them — and nothing about the request, the path or the store moves an inch:
   * `dir` is still `1s`, the fold is still one-second buckets, and what lands is
   * still minutes.
   *
   * `phrase` EXISTS SO THE BUTTON WORD NEVER GETS READ AS A MEASUREMENT. Four
   * sentences on this page name a rung mid-clause — "it serves ___ and
   * coarser", "the store has no directory for ___", "___ is this feed's finest
   * rung", "one record at ___ is a ___". Dropping the button word into those
   * writes *"one record at Ticks is a conflated snapshot"*, which reads as this
   * page calling a snapshot a tick — the one claim `/feeds.json`'s `finest` was
   * built to stop, and the thing `server.rs`'s own
   * `feeds_json_carries_the_granularity_floor_and_never_calls_a_snapshot_a_tick`
   * asserts on the other side. The button says what he bought; the sentence says
   * what is in it. Both are true, and neither is allowed to answer for the other.
   *
   * WHY `tick` IS NOT ONE OF THE THREE, AND WHY `1s` CARRIES THE WORD INSTEAD.
   * `Granularity::is_requestable` is `false` for `Tick` and true for every other
   * rung, for every feed (D-0118) — so a `tick` row is a row that could never be
   * sent, and drawing one is the control-that-hides-a-failure `CLAUDE.md` §4
   * bans. What the operator BUYS and calls a tick is the archive:
   * `NSE_<seg>_TICK_<date>.zip` and `GFDLNFO_TICK_<date>.zip` are the file names
   * in `pull::vendor`'s own `ArchiveName` consts. Those files were MEASURED at
   * one row per whole second with no sub-second field — 22,426 rows across a
   * 22,500-second session, up to four rows sharing a second with no tiebreaker
   * (`docs/08-vendor-samples.md`). That is the `1s` rung, and it is a conflated
   * snapshot rather than a print stream. So `1s` is the row his word lands on,
   * and the row states what one record at it actually is rather than repeating
   * the file name back at him.
   *
   * "FOR TRUEDATA OR GDFL ALONE" IS NOT SPELLED HERE, AND MUST NOT BE. It falls
   * out of `/feeds.json`: `floorVerdict` marks `1s` permanently refused for any
   * feed whose stated floor is a minute, and `rungRows` drops a permanently
   * refused row (D-0137). Dhan and Groww bottom out at one minute, the two
   * archives bottom out at one second, so the row appears on exactly those two
   * — read from the server on every load, with no vendor named in this file. A
   * fifth feed that reaches a second gets the row the day its descriptor says
   * so; a hardcoded pair would not, and would be the second copy of a vendor
   * fact that D-0126, D-0131 and D-0138 each deleted from this page.
   *
   * `stored` is `Granularity::store_timeframe().is_some()`. It is FALSE on `1s`,
   * and that is the honest state of a tick pull today: the request parses, the
   * feed declares the rung, and `pull::ingest::Plan::timeframe` then refuses it
   * BY NAME at the write boundary because `crates/store` ships no `1s`
   * directory. The row says so before it is ticked. The fold that makes those
   * same archive seconds storable already exists — `pull::fold`, one-second
   * buckets, first/max/min/last in file order — and it runs when these two feeds
   * are asked for `1min`.
   *
   * `per` is BARS PER NSE SESSION, and `null` where this page cannot state one
   * from a verified fact. 09:15 to 15:30 is 375 minutes, so a full minute
   * session is 375 bars and a full day session is 1. `1s` is `null`: a session
   * is 22,500 seconds and the archive held 22,426 rows, so a second is not a
   * bar that either exists or is missing and no yardstick divides it —
   * `CLAUDE.md` §3 rule 6.
   */
  const RUNGS = [
    { dir: '1s', label: 'Ticks', phrase: 'one second', stored: false, per: null },
    { dir: '1min', label: '1 minute', phrase: 'one minute', stored: true, per: 375 },
    { dir: '1day', label: '1 day', phrase: 'one day', stored: true, per: 1 }
  ];

  // ───────────────────── HOW FINE EACH FEED CAN EVER ANSWER ─────────────────
  //
  // A SECOND FLOOR, AND IT IS NOT THE FIRST ONE. `pairFloor` further down
  // answers HOW FAR BACK, off the wire's `history` rows. This one answers HOW
  // FINE, and the two refusals share nothing but the word:
  //
  //   below the HISTORY floor    another DAY is refused. The rung is not — the
  //                              vendor serves it, just not that far back
  //   below THIS floor           nothing makes the rung exist. Not a pull, not
  //                              an entitlement, not a purchase, not a code
  //                              change. The vendor has never published it
  //   not fetched by this build  `Descriptor::granularities` does not carry it
  //                              and `crates/api`'s `served()` refuses the POST
  //                              BY NAME before it reads a credential. The
  //                              vendor is willing; a code change fixes it
  //   no store directory         `Granularity::store_timeframe` has nowhere to
  //                              file it, so the WRITE boundary refuses the
  //                              bar. A store-format version fixes it
  //
  // FOUR REFUSALS, AND EXACTLY ONE OF THEM IS PERMANENT. They were rendered
  // identically — `no directory`, on every rung the store cannot file, on a
  // Groww `tick` row no vendor on earth serves, and on a TrueData `1s` row a
  // vendor serves today. One annotation standing for four different amounts of
  // hope is the fallback `CLAUDE.md` §4 bans: the operator reads "nothing saved
  // here yet", ticks the box, and waits for a pull that can never arrive.
  //
  // ═════ THE TABLE THAT STOOD HERE IS GONE, AND THE FACT IS ON THE WIRE ═════
  //
  // Four rows used to sit here transcribing the four `GranularityFloor` consts
  // in `crates/pull/src/vendor.rs`, word for word, because `/feeds.json`
  // emitted no granularity floor and there was nothing to read. TWO COPIES OF
  // ONE VENDOR FACT IS THE DEFECT AND NOT THE WORKAROUND: they can disagree,
  // and the browser's copy is the one that would be wrong — it is versioned
  // with this file, and nothing rebuilds it when a const is reworded.
  //
  // `/feeds.json` now carries `finest` on every feed row:
  //
  //   rung         the finest rung the vendor can EVER serve, spelled with the
  //                same directory name `RUNGS` above is spelled with
  //   kind         `tick` | `snapshot` | `bar` — `pull::vendor::FinestKind`
  //   label        that kind's own sentence, so this file spells none of them
  //   tick_stream  whether this is a real print stream. FALSE on every row
  //   conflated    whether records between two of the feed's own slots were
  //                discarded before the file was written. TRUE for the two
  //                archives, and it is what makes one second of an archive a
  //                different object from one second of anything else
  //   because      why nothing finer exists, IN THE VENDOR'S OWN TERMS
  //   source       where those words were read — §3 rule 1
  //
  // THE TWO BOOLEANS ARE THE POINT OF THE FIELD. A page handed `1s` and
  // nothing else is free to write "tick" beside it, and the two feeds that
  // bottom out at one second serve no prints at all: what sits on that second
  // is a conflated snapshot of the best bid, the best ask and the best last
  // price, and every print between two of them was discarded before the file
  // was written. They are ASKED rather than inferred from the word, so nothing
  // here matches on a string to decide what may be printed next to a number.
  //
  // A FEED WHOSE ROW CARRIES NO `finest` CLAIMS NOTHING, AND THE PAGE SAYS
  // EXACTLY THAT. It does not fall back to a table — there is no table left to
  // fall back to, and that is the whole change: a hardcoded fallback that looks
  // authoritative is the `CLAUDE.md` §4 shape, and the operator cannot tell a
  // transcription from a reading. The honest response to a missing field is
  // that THE RUNNING BINARY PREDATES IT, which names the fix — restart on a
  // newer build — instead of quietly answering out of a copy of what the
  // vendors said the last time this file was edited.
  //
  // O(feeds) ONCE PER FEED-LIST CHANGE, then O(1) per lookup. `floorVerdict`
  // is called once per offered rung per render of the control and must not
  // scan.
  const finestByWire = $derived.by(() => {
    const m = new Map();
    for (const f of feeds.all) {
      const x = f?.finest;
      // EVERY PART OR NOTHING. A half-emitted floor is not a floor: a `rung`
      // with no `kind` would let this page state a number and stay silent
      // about what one record at it is, which is the exact silence the kind
      // exists to break.
      if (f?.wire && x && typeof x.rung === 'string' && typeof x.kind === 'string') m.set(f.wire, x);
    }
    return m;
  });

  /**
   * THE FEED'S FLOOR IN ONE SENTENCE, COMPOSED FROM FACTS THE SERVER SENT.
   *
   * Not a transcription and not prose about a vendor: every noun in it is a
   * field off `/feeds.json` — the feed's own `display`, `kind_label`, which is
   * `pull::vendor::SourceKind`'s own words, and the rung's phrase off the
   * ladder. This file supplies the grammar and nothing else, so a vendor whose
   * floor moves moves this sentence with no edit here.
   *
   * `phrase`, NOT `label`. This is a SENTENCE about what the vendor publishes,
   * and the button word is the operator's — "it serves ticks and coarser" is a
   * claim about the vendor's records that `/feeds.json` contradicts on the very
   * next line. See `RUNGS` for the three-name split and why it exists.
   *
   * `''` when the row carries no floor, and the caller states the absence
   * rather than printing an empty sentence.
   */
  function shortFloor(wire) {
    const floor = finestByWire.get(wire);
    if (!floor) return '';
    const feed = feeds.all.find((f) => f.wire === wire);
    const phrase = RUNGS.find((r) => r.dir === floor.rung)?.phrase ?? floor.rung;
    const kind = feed?.kind_label ? ` — ${feed.kind_label}` : '';
    return `${feed?.display ?? wire}${kind}. It serves ${phrase} and coarser.`;
  }

  /**
   * WHAT THIS FEED'S GRANULARITY FLOOR SAYS ABOUT ONE RUNG.
   *
   * The browser spelling of `pull::vendor::Descriptor::granularity_verdict`:
   *
   *   refused   finer than THIS feed's finest. Permanent.
   *   finest    this rung IS the feed's finest, and the floor says what one
   *             record at it is. The one rung where that is answerable at all.
   *   coarser   above the floor, so the floor has no opinion — and inventing
   *             one would be §3 rule 1. It says NOTHING about whether this
   *             build fetches the rung; that is a different question with its
   *             own answer below, and the two are deliberately not merged.
   *   unstated  THIS SERVER SENT NO FLOOR FOR THIS FEED. Rust has a descriptor
   *             for every feed it can name, so this is not a vendor with
   *             nothing to say — it is a running binary that predates the
   *             field, or a row that carries only half of one. Nothing is
   *             refused on the strength of it and the page says why.
   *
   * One map read and one integer comparison. It does not walk the ladder.
   *
   * A `never` arm stood above `refused`, answering for `tick` without consulting
   * the feed. `RUNGS` no longer offers that rung and nothing else on this page
   * can name one, so the arm was unreachable — and an unreachable branch in the
   * function that decides whether a control is dead is a claim nobody can check.
   * The fact it carried is not lost: `Granularity::is_requestable` still refuses
   * a tick for every feed in `crates/pull`, `/feeds.json` still states it from
   * the other side with `tick_stream: false` on every row, and the `finest` arm
   * below is where an operator now reads it — on the two archive feeds, on the
   * rung their tick files actually hold.
   */
  function floorVerdict(wire, dir) {
    const floor = finestByWire.get(wire);
    if (!floor) return { state: 'unstated', permanent: false, floor: null };
    if (finerThan(dir, floor.rung)) return { state: 'refused', permanent: true, floor };
    if (dir === floor.rung) return { state: 'finest', permanent: false, floor };
    return { state: 'coarser', permanent: false, floor };
  }

  /** How many rows one page of the census draws. */
  const PAGE_SIZE = 25;

  /**
   * THREE-WAY, ALWAYS. A comparator that answers 1 for "equal" is not a
   * comparator: it makes the sort order depend on the input order, so the same
   * rows in a different sequence rank differently and a stable second key never
   * gets to break the tie it exists for.
   */
  function cmpStr(a, b) {
    const x = String(a ?? '');
    const y = String(b ?? '');
    return x < y ? -1 : x > y ? 1 : 0;
  }
  function cmpNum(a, b) {
    const x = Number(a ?? 0);
    const y = Number(b ?? 0);
    return x < y ? -1 : x > y ? 1 : 0;
  }

  /**
   * THE EIGHT UNIVERSES, IN NSE'S OWN ORDER, AND WHAT EACH CAN PUT ON THE WIRE.
   *
   * A universe is a DISPLAY SET. The request is not: `api::ingest::SpotRequest`
   * carries exactly one field that names a set — `target` — and
   * `api::ingest::SpotTarget` spells exactly three values: `swept`, `indices`,
   * `equities`. So a universe can be ASKED FOR here only where one of those
   * three names the same set.
   *
   * Where none does, the row is DRAWN AND REFUSED, in place, in order, with the
   * reason on it. Dropping it would hide the gap — a list of four reads as if
   * the other four were not universes at all rather than universes this route
   * cannot spell. Inventing a target for it would build a request the server
   * cannot parse, which is the fallback that hides a failure.
   *
   * `token` is the `/instruments.json` universe bit a row is COUNTED from:
   * `row.universe` is a `+`-joined string of `index`, `fno` and `ntm`, and
   * `'*'` is every row rather than a fourth bit. `field` is the token a row
   * would need out of the `universes` ARRAY that `crates/api/src/server.rs`
   * emits (D-0089/D-0090) — a field the running binary may predate, which is
   * measured below and never assumed.
   *
   * `target: null` is the whole refusal: it disables the row and it is why.
   */
  const UNIVERSES = [
    // THE FOUR TIERS ARE REQUESTABLE. They carried `target: null` because
    // nothing turned (feed, universe) into a request: `SpotTarget::names`
    // decides membership from a universe bit and never consults the feed, so
    // the API could count a tier globally and not describe one PER FEED. The
    // join now exists — `crates/api/src/constituents.rs`, keyed on
    // (exchange, ISIN), never on symbol — and it resolves 50/50, 100/100,
    // 200/200 and 500/500 for BOTH masters on disk, with nothing lacking,
    // ambiguous or malformed. The slugs are the ones `SpotTarget` has parsed
    // since D-0105. A tier this feed cannot fill still refuses, but now it
    // refuses with a MEASURED count from /universes.json?feed= rather than by
    // being disabled at birth.
    { id: 'n50', label: 'NIFTY 50', field: 'n50', token: null, target: 'n50' },
    { id: 'n100', label: 'NIFTY 100', field: 'n100', token: null, target: 'n100' },
    { id: 'n200', label: 'NIFTY 200', field: 'n200', token: null, target: 'n200' },
    { id: 'n500', label: 'NIFTY 500', field: 'n500', token: null, target: 'n500' },
    { id: 'ntm', label: 'NIFTY Total Market', field: null, token: 'ntm', target: 'equities' },
    // REQUESTABLE SINCE D-0136, and it is the row whose join was built first
    // and reached last. `constituents::Tier::FnoUnderlyings` has always
    // existed, declares `published() == 213`, and had no `SpotTarget` pointing
    // at it — so this row said `no target` while the machinery that answers it
    // sat complete one crate away. `SpotTarget::Fno` now maps to that tier, so
    // the count beside this row is a MEASURED join per feed, not a roster
    // length.
    { id: 'fno', label: 'F&O Underlyings', field: null, token: 'fno', target: 'fno' },
    // NOT AN NSE CONSTITUENT FILE, AND THE LABEL MUST NOT IMPLY ONE.
    //
    // The four NIFTY tiers and the Total Market each come from a published NSE
    // CSV, transcribed with its URL and fetch date in `core::universe`, and are
    // joined to a vendor's master on (exchange, ISIN) by
    // `api::constituents` under a partition that must sum.
    //
    // This row is different in kind. `api::ingest::SpotTarget::Indices`
    // resolves as `universe.contains(Universe::INDEX)` — "whatever the vendor
    // master lists as an index series on this build" — and its `members()`
    // returns None ON PURPOSE, because NSE publishes no file naming that set
    // and hardcoding one would be invention (§3 rule 1) that went stale the day
    // a vendor added a series.
    //
    // The consequence the operator has to know: THIS SET IS PER FEED AND THE
    // FEEDS MAY LEGITIMATELY DIFFER. There is no canonical list to verify
    // against, so "Groww's indices" and "Dhan's indices" are two answers to two
    // questions, not one answer measured twice. `note` says so on the control.
    {
      id: 'index',
      label: 'NSE Indices',
      field: null,
      token: 'index',
      target: 'indices',
      note: 'this feed’s own index series — NSE publishes no list of them, so the set is the vendor’s and two feeds may differ'
    },
    // REQUESTABLE SINCE D-0136, and the ONE row whose slug this repository
    // chose rather than borrowed.
    //
    // `token: '*'` is a BROWSER SENTINEL and has never been a wire word —
    // `namedByUniverse` short-circuits on it and `masterCount` returns every
    // row. So there is no `/instruments.json` token to lift, which is the one
    // place D-0105's "the slugs are the wire's own words" rule has no answer.
    // `all` is invented, and D-0136 records that it is.
    //
    // The set is `catalog::tracked` — the union the pull path already filters
    // by — so the count on this row and the number of instruments a run
    // attempts are the same number by construction, which is the property
    // `SpotTarget::names` exists to hold.
    { id: 'all', label: 'Everything', field: null, token: '*', target: 'all' }
  ];

  /**
   * THE SWEPT PAIR, AND IT IS NOT ONE OF THE EIGHT.
   *
   * `CLAUDE.md` §1 names `NSE-NIFTY` and `NSE-BANKNIFTY` as the entire engine
   * surface. That is not an NSE index family — it is this repository's own
   * sweep surface, so it does not belong in a list of NSE universes and it is
   * not spelled like one.
   *
   * It is also the ONLY set a broker path can serve: `pull::vendor::HttpSpec`
   * carries no request-parameter map, so a target naming a wide set is refused
   * rather than fetching one series under a name nobody asked for (the caution
   * below states it). Dropping it to make room for the eight would leave this
   * form unable to build a legal broker request at all, so it is kept, beside
   * them, spelled as what it is.
   *
   * A concrete object, never `UNIVERSES.at(-1)`: it is the fallback for an
   * unrecognised id, and a fallback that can be `undefined` needs a guard at
   * every reader.
   */
  const SWEPT = {
    id: 'swept',
    label: 'NSE-NIFTY + NSE-BANKNIFTY',
    field: null,
    token: null,
    target: 'swept'
  };
  const UNIVERSE_BY_ID = new Map([...UNIVERSES, SWEPT].map((u) => [u.id, u]));

  /**
   * The three segments the store keys on, and WHICH ROUTE FILLS EACH.
   *
   * A segment is a row, not a branch — the same shape as `RUNGS` and `UNIVERSES`
   * above. `served` is the only thing that decides whether this form can offer
   * it, and it is a statement about the ROUTE rather than about the vendor:
   * `/pull/spot` fills a whole target in one request, `/pull/fno` fills one
   * settled contract. Offering a control that produces a request nothing can
   * send is the fallback that hides a failure.
   *
   * ══════ WHY THIS LIST LOOKED LIKE ONE ITEM, AND WHY IT NO LONGER DOES ══════
   *
   * The two refused rows were FILTERED OUT of the menu and counted in a clause
   * beside it. A one-item dropdown reads as "this axis does not exist"; the
   * operator asked why expired futures and options were missing, which is
   * exactly the question a hidden row cannot answer. Every row is drawn now —
   * dead, in place, carrying its own refusal — the same shape the universe menu
   * has always used, and the shape `$lib/Picker.svelte` documents its
   * `disabled`/`why` fields for.
   *
   * THE REFUSAL IS THREE MEASURED FACTS, not one. All three were read out of
   * this repository's own Rust, not assumed:
   *
   *   1. NO EXPIRED CONTRACT IS IN ANY MASTER. `core::vendor::decode_master_row`
   *      declines every `FUT`, `CE` and `PE` row with `Skip::LiveContract`
   *      ("live derivative contract"), and its comment states the measurement:
   *      both vendors PURGE ON EXPIRY, and the earliest expiry in either master
   *      is three days from now — so every derivative row a master carries is
   *      live by definition, and the master is not where history comes from.
   *      Declined rows never enter `api::merge::Merged::by_key`, so
   *      `/instruments.json` has no FNO row to list and this page's catalogue
   *      cannot show one. The dropdown is empty of them because the DATA is.
   *   2. NOTHING IN THIS BUILD FETCHES ONE. `POST /pull/fno` is registered
   *      (`api::server`) and parses in full, and then answers `503` with
   *      `Outcome::NotStarted` — "expired F&O has no local-archive path and no
   *      HTTP transport in this build". A well-formed request is refused, not
   *      served.
   *   3. THE REQUEST SHAPES DO NOT MEET. `api::ingest::FnoRequest` carries ONE
   *      underlying, ONE series and ONE settled expiry; a segment tick here
   *      names a whole target. There is no route that takes the two together.
   *
   * `short` is the VISIBLE sentence on the row; `why` is the long form on its
   * `title`. Both, because a refusal only a hover reveals is one most readers
   * never meet.
   */
  const SEGMENTS = [
    {
      key: 'spot',
      label: 'Spot',
      note: 'continuous series — no expiry',
      served: true,
      short: null,
      why: 'POST /pull/spot — the request this form builds.'
    },
    {
      key: 'futures',
      label: 'Expired futures',
      note: 'contracts that have already settled',
      served: false,
      short:
        'No expired future is listed anywhere this page can read, and no route in this build fetches one.',
      why: 'THREE REASONS, ALL MEASURED.\n\n1. Not in the master. core::vendor::decode_master_row declines every FUT row as Skip::LiveContract — "live derivative contract". Both vendors purge the master on expiry and the earliest expiry in either one is three days from now, so every derivative row a master carries is a LIVE contract and none of it is backtest data. A declined row never reaches api::merge::Merged::by_key, so /instruments.json emits no FNO row and this control has nothing to offer.\n\n2. Not fetchable. POST /pull/fno exists and parses in full, then answers 503 with audit Outcome::NotStarted: "expired F&O has no local-archive path and no HTTP transport in this build".\n\n3. Not the same request. api::ingest::FnoRequest carries ONE underlying, ONE series and ONE settled expiry — no set — so a segment naming a whole target has nothing to put on the wire. This form drives /pull/spot only.\n\nExpired history comes from the vendors’ historical endpoints and from the existing lake, never from the live instrument master. Until one of those is wired to a route, this row is a refusal and not a gap.'
    },
    {
      key: 'options',
      label: 'Expired options',
      note: 'contracts that have already settled',
      served: false,
      short:
        'Same three reasons as expired futures — purged from the master, no transport, one contract per request.',
      why: 'THREE REASONS, ALL MEASURED, and they are the futures ones.\n\n1. Not in the master. core::vendor::decode_master_row declines every CE and PE row as Skip::LiveContract after parsing its expiry and strike. Both vendors purge on expiry, so the ~148,000 contracts a live chain would add are all live and none of them is history.\n\n2. Not fetchable. POST /pull/fno answers 503 — no local-archive path and no HTTP transport in this build.\n\n3. Not the same request. One underlying, one series, one expiry per request, and the expiry must be strictly behind today (api::ingest::parse_fno refuses Refusal::LiveContract otherwise). There is no route that takes a target and a segment together.'
    }
  ];

  /** `api::ingest::MAX_WINDOW_DAYS`. Refused by the parser before anything runs. */
  const MAX_WINDOW_DAYS = 3653;

  /** The earliest year the server's own picker offers, read from `GET /pull`. */
  const FLOOR_YEAR = 2015;

  /**
   * The month names spelled in full, and they exist to READ a typed month, not
   * to write one.
   *
   * The THREE-LETTER table is not here — it is `MON` in `$lib/dates.js`, which
   * this file imports and the grid renders. A local copy would be a third
   * spelling of the same twelve strings, and the one that drifts is always the
   * copy nobody remembers exists. Nothing below ever renders one of these: they
   * are the accept-set `parseDay` matches an operator's typing against, so
   * `02 September 2024` is a day this page understands and `02 Sepxyz 2024` is
   * not — a three-letter prefix test alone would take the second for the first.
   */
  const MONTHS = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December'
  ];

  /** One virtual row, in pixels. Must equal `--row-h` or the spacer lies. */
  const ROW_H = 30;
  /** Rows kept above and below the viewport so a fast scroll does not tear. */
  const OVERSCAN = 8;
  /** Prefix depth of the outcome search index — the same bound `$lib/index` uses. */
  const MAX_PREFIX = 4;

  // ------------------------------------------------------------- date helpers
  //
  // EVERY DATE IS AN ISO STRING AND EVERY SUM IS IN UTC. A local-time `Date`
  // shifts under DST and this operator's own machine renders `dd/mm/yyyy`,
  // which is the exact ambiguity `crates/api/src/calendar.rs` was written to
  // remove: `01/07/2025` is 1 July here and 7 January in half the world.

  const DAY_MS = 86_400_000;
  const IST_OFFSET_MS = 19_800_000; // +05:30, and India has no daylight saving.

  /** The IST date of a moment, as `YYYY-MM-DD`. */
  function istDay(ms = Date.now()) {
    return new Date(ms + IST_OFFSET_MS).toISOString().slice(0, 10);
  }
  function pad(n, w = 2) {
    return String(n).padStart(w, '0');
  }
  function iso(y, m, d) {
    return `${pad(y, 4)}-${pad(m)}-${pad(d)}`;
  }
  function parts(isoDay) {
    const [y, m, d] = isoDay.split('-').map(Number);
    return { y, m, d };
  }
  /** Days since the epoch, so two dates can be compared and subtracted. */
  function dayNum(isoDay) {
    const { y, m, d } = parts(isoDay);
    return Math.round(Date.UTC(y, m - 1, d) / DAY_MS);
  }
  function addDays(isoDay, n) {
    return new Date((dayNum(isoDay) + n) * DAY_MS).toISOString().slice(0, 10);
  }
  function daysInMonth(y, m) {
    return new Date(Date.UTC(y, m, 0)).getUTCDate();
  }
  function isValidIso(s) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) return false;
    const { y, m, d } = parts(s);
    return m >= 1 && m <= 12 && d >= 1 && d <= daysInMonth(y, m);
  }
  /** Every `YYYY-MM` the window touches, inclusive at both ends. */
  function monthsBetween(a, b) {
    if (!isValidIso(a) || !isValidIso(b)) return [];
    const from = parts(a);
    const to = parts(b);
    const out = [];
    let y = from.y;
    let m = from.m;
    // Bounded by the window cap: 3,653 days is at most 122 months.
    while (y < to.y || (y === to.y && m <= to.m)) {
      out.push(`${pad(y, 4)}-${pad(m)}`);
      m += 1;
      if (m > 12) {
        m = 1;
        y += 1;
      }
    }
    return out;
  }
  // ------------------------------------------------- the NSE session calendar
  //
  // WHICH DAYS HOLD BARS AT ALL. It answers two questions and only two: where
  // the CEILING is (§ below), and whether a day in the picker is drawn faint
  // because nothing was ever traded on it. IT REFUSES NOTHING — see the floor
  // block below: the only bound that may take a day away from the operator is
  // the FEED's, because that is the one his own choice can move.
  //
  // THE HOLIDAY TABLE IS THE ONE THING HERE THAT CANNOT BE COMPUTED, and no
  // route on this server states it: `crates/api/src/server.rs`'s router carries
  // no calendar endpoint, and `crates/api/src/calendar.rs` renders a day picker
  // that knows about years and month lengths, not about trading days. So the
  // table is checked in HERE, dated, and it MUST MOVE SERVER-SIDE — the store
  // knows which days it holds bars for and the browser is guessing beside it.
  //
  // WHAT HAPPENS WHEN IT RUNS OUT is stated rather than hidden. Past
  // `HOL_THRU` a weekday is treated as a session, which will call a holiday a
  // session — and every sentence built from it says so. The alternative was
  // freezing the ceiling on the table's last day, which is a page whose "newest
  // day" stops moving and never explains why. `CLAUDE.md` §4: degrade loudly
  // and name the reason.

  /** The first day the table below is complete for. Sep 2024 carried no NSE trading holiday. */
  const HOL_FROM = '2024-09-01';
  /** The last day it covers. After this, weekday-only — and every reader is told. */
  const HOL_THRU = '2026-12-31';
  const HOLIDAYS = new Set([
    '2024-10-02', '2024-11-01', '2024-11-15', '2024-12-25',
    '2025-02-26', '2025-03-14', '2025-03-31', '2025-04-10', '2025-04-14', '2025-04-18',
    '2025-05-01', '2025-08-15', '2025-08-27', '2025-10-02', '2025-10-21', '2025-10-22',
    '2025-11-05', '2025-12-25',
    '2026-01-26', '2026-03-04', '2026-03-19', '2026-04-01', '2026-04-03', '2026-05-01',
    '2026-08-15', '2026-10-02', '2026-11-09', '2026-12-25'
  ]);

  /** How many holidays that table names. Counted from it, never written twice. */
  const HOLIDAY_COUNT = HOLIDAYS.size;

  const DAYNAME = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
  /** 0 = Sunday. Built at UTC midnight like every other date here, so no zone moves it. */
  function weekdayOf(isoDay) {
    const { y, m, d } = parts(isoDay);
    return new Date(Date.UTC(y, m - 1, d)).getUTCDay();
  }
  /** Whether the NSE traded that day, as far as this page can honestly say. */
  function isSession(isoDay) {
    const w = weekdayOf(isoDay);
    if (w === 0 || w === 6) return false;
    return !HOLIDAYS.has(isoDay);
  }
  /** Why a day holds no bars, or `null` when it is a session. */
  function noSessionWhy(isoDay) {
    const w = weekdayOf(isoDay);
    if (w === 0 || w === 6) return `${DAYNAME[w]}, no session`;
    if (HOLIDAYS.has(isoDay)) return 'NSE holiday, no session';
    return null;
  }
  /** Whether the holiday table actually covers a day, or is only guessing weekends. */
  function holidaysKnownFor(isoDay) {
    return isoDay >= HOL_FROM && isoDay <= HOL_THRU;
  }

  /** NSE equity close, 15:30 IST. The minute a session stops being partial. */
  const CLOSE_MIN = 15 * 60 + 30;
  /** Minutes past IST midnight, from the same instant `istDay` reads. */
  function istMinute(ms = Date.now()) {
    const d = new Date(ms + IST_OFFSET_MS);
    return d.getUTCHours() * 60 + d.getUTCMinutes();
  }

  // ------------------------------------------- how far back each feed answers
  //
  // THE FLOOR IS A PROPERTY OF THE FEED AND OF THE TIMEFRAME, NOT OF THIS PAGE.
  // How far back you may ask depends on WHO you ask and on WHAT you ask them
  // for: Dhan serves daily bars back to a scrip's inception and minute bars for
  // a rolling five years; Groww serves daily "full history" and one-minute bars
  // for the last three months. A floor keyed on the vendor alone is wrong for
  // one of that vendor's own two rungs, so the key is (feed, timeframe).
  //
  // ════════════ THE LAST SECOND COPY IS GONE, AND THIS IS WHAT IT WAS ═══════
  //
  // `GET /feeds.json` emits `wire`, `display`, `kind`, `kind_label`, `verb`,
  // `ready`, `why`, `finest` and `history` (`crates/api/src/server.rs`), and
  // `history` is exactly this fact: one row per rung carrying the binding
  // claim, the contested one, the RESOLVED oldest day and the source of each,
  // read off `pull::vendor::Descriptor`, which has carried a per-rung history
  // floor since D-0110.
  //
  // Until now this file also held its own copy — `FLOOR_OPERATOR` and
  // `FLOOR_DOC`, two hardcoded tables — and its own comment admitted they were
  // a second copy and that wiring them to the emitted fact "is not done here".
  // They are deleted. `pairFloor` below is a lookup into `feeds.all`.
  //
  // THE DRIFT WAS NOT HYPOTHETICAL, WHICH IS WHY THIS IS A FIX AND NOT TIDYING:
  //
  //   * `FLOOR_OPERATOR` carried a `zerodha` row for a feed no descriptor in
  //     this build names, and NEITHER table had a row for `truedata` or `gdfl`,
  //     which the wire answers for.
  //   * D-0131 reversed Groww's one-minute rung from a rolling three months to
  //     the operator's fixed 2020-01-01. The wire moved. These tables did not.
  //     The page was refusing January 2020 through May 2026 on a rung the
  //     server says answers for it.
  //
  // AND THE CLIENT-SIDE CLOCK WENT WITH THEM. A rolling floor was recomputed
  // here with `Date.UTC(y - years, ...)` and compared against an IST day — two
  // clocks in one comparison, off by a day for part of every day on any machine
  // west of IST. `oldest` arrives resolved, so there is nothing left to be
  // wrong: a server-named day is compared against a server-named day.
  //
  // FOUR SHAPES, AND THE WIRE'S OWN WORD FOR EACH:
  //   rolling — N years/months back from today. It moves every single day, so a
  //             window answerable yesterday can be refused today: a fact about
  //             the vendor, not a bug here. The SERVER resolves it.
  //   fixed   — a real calendar date that does not move.
  //   none    — the vendor states there is no floor. NOT the same as unknown.
  //   unknown — nothing states anything, and the page then claims NOTHING.
  /**
   * The oldest day (feed, timeframe) answers for, READ OFF THE WIRE.
   *
   * `/feeds.json` emits `history` as one row per rung, each carrying the
   * binding claim, the contested one, the resolved oldest day and the source of
   * each — `crates/api/src/server.rs`, built from
   * `pull::vendor::Descriptor::history`. This function is now a lookup into
   * that, and it holds no vendor fact of its own.
   *
   * # THE DAY IS THE SERVER'S, AND THAT DELETES A WHOLE CLASS OF BUG
   *
   * `oldest` arrives already resolved. The browser used to recompute a rolling
   * floor itself with `Date.UTC(y - years, ...)` and compare the result against
   * an IST day — two clocks in one comparison, wrong by a day for part of every
   * day on any machine west of IST. There is nothing left here to be wrong: the
   * page compares a day the server named against a day the server named.
   *
   * # `at` VERSUS `known`, AND WHY A CALLER MAY NEVER COLLAPSE THEM
   *
   * `at` is null both when the vendor states it holds everything and when
   * nobody has stated anything. `known` is what tells those apart, and it comes
   * from the wire's own word: `kind: "unknown"` is nobody having said anything,
   * `kind: "none"` is the vendor stating there is no floor. Emitting null for
   * both and letting the reader guess is exactly the fallback §4 bans.
   */
  function pairFloor(wire, rungDir) {
    const feed = feeds.all.find((f) => f.wire === wire);
    const row = feed?.history?.find((h) => h.rung === rungDir);
    // NO ROW IS NOT NO FLOOR. A rung the wire says nothing about is `unknown`,
    // and `known: false` is what the caller reads to say so out loud.
    if (!row) return { at: null, known: false, rolling: false, src: [] };

    const src = [];
    if (row.source) {
      src.push(`${row.source}${row.oldest ? ` → ${dayLabel(row.oldest)}` : ' → no floor'}`);
    }
    // THE DISPLACED CLAIM TRAVELS WITH THE BINDING ONE. Deleting it would leave
    // a number no reader could argue with, and this repository has two sources
    // that disagree on three of its four recorded rows.
    if (row.contested?.source) {
      src.push(
        `contested — ${row.contested.source}${
          row.contested.oldest ? ` → ${dayLabel(row.contested.oldest)}` : ' → no floor'
        }`
      );
    }
    if (row.binds_because) src.push(row.binds_because);

    return {
      at: row.oldest ?? null,
      known: row.kind !== 'unknown',
      // A ROLLING FLOOR MOVES EVERY DAY, and the sentence says so. The word is
      // the wire's, not a shape this file infers from the numbers.
      rolling: row.kind === 'rolling',
      src
    };
  }

  // ------------------------------------------------------------ month helpers
  //
  // THE MONTH IS NOT THE WINDOW ANY MORE — IT IS THE STORE'S FILE. `/store.json`
  // answers one row per instrument-MONTH and the store address ends in
  // `/2024-11`, so a day window still has to be able to say which month files it
  // touches and the calendar still pages by month.
  //
  // THE SAME RULE AS THE DAYS ABOVE: `YYYY-MM` is the KEY form. It is what the
  // state holds, what every comparison reads and what sorts correctly as a
  // plain string. `monthLabel` from `$lib/dates.js` is called at the render
  // site and nowhere else — see that file's header for why a reformatted key
  // is an identity change wearing a presentation change's clothes.

  /** Months since year 0, so two months can be compared and subtracted. */
  function monthNum(ym) {
    const [y, m] = ym.split('-').map(Number);
    return y * 12 + (m - 1);
  }
  function monthFromNum(num) {
    return `${pad(Math.floor(num / 12), 4)}-${pad((num % 12) + 1)}`;
  }
  function addMonths(ym, n) {
    return monthFromNum(monthNum(ym) + n);
  }

  /**
   * ═══════════ A TYPED DAY -> THE ISO KEY, OR A STATED REFUSAL ═══════════
   *
   * DISPLAY -> WIRE, and one of exactly two crossings between the two spellings
   * of a day on this page (`dayLabel` is the other, and it only goes the other
   * way). Everything that counts, compares, keys a Map or goes on the wire
   * reads `YYYY-MM-DD`; `02 Sep 2024` exists in the two text fields and
   * nowhere else. If a display string ever reached a comparison the page would
   * look perfect and narrow nothing, which is the silent failure this split
   * exists to make impossible.
   *
   * LIBERAL ABOUT SEPARATORS AND CASE, STRICT ABOUT THE MONTH: it has to be a
   * NAME. `02/09/2024` is refused deliberately and by name — it reads as 2
   * September to one reader and 9 February to another, which is the exact
   * ambiguity `crates/api/src/calendar.rs` was written to remove, and a parser
   * that picked one of them would be inventing an intent this page cannot know.
   * Both readings are quoted back when both exist, because the refusal is an
   * explanation of why the page will not choose, not a scolding.
   *
   * The ISO wire form is accepted too: it is the value the field's own state
   * carries, and pasting it back must not be an error.
   *
   * `{ iso }` on success, `{ iso: '' }` for a field cleared on purpose — which
   * is "not set", not a failure — and `{ err }` otherwise. Never a guess.
   */
  function parseDay(raw) {
    const s = String(raw ?? '').trim().replace(/\s+/g, ' ');
    if (!s) return { iso: '' };
    const wire = s.match(/^(\d{4})-(\d{1,2})-(\d{1,2})$/);
    if (wire) {
      const [y, m, d] = [Number(wire[1]), Number(wire[2]), Number(wire[3])];
      const built = m >= 1 && m <= 12 && d >= 1 && d <= daysInMonth(y, m) ? iso(y, m, d) : null;
      return built ? { iso: built } : { err: `${s} is not a date — that month is shorter than that.` };
    }
    const t = s.split(/[\s.,/-]+/).filter(Boolean);
    if (t.length !== 3) return { err: `“${s}” is not a date. Write it as 02 Sep 2024.` };
    const ai = t.findIndex((x) => /^[A-Za-z]{3,9}$/.test(x));
    if (ai < 0) {
      const a = Number(t[0]);
      const b = Number(t[1]);
      const y = t[2];
      const four = /^\d{4}$/.test(y);
      const reads = [];
      if (four && a >= 1 && a <= 31 && b >= 1 && b <= 12) reads.push(`${pad(a)} ${MON[b - 1]} ${y}`);
      if (four && b >= 1 && b <= 31 && a >= 1 && a <= 12) reads.push(`${pad(b)} ${MON[a - 1]} ${y}`);
      return {
        err:
          reads.length === 2
            ? `${s} is ambiguous — ${reads[0]} or ${reads[1]}? Write the month as a name.`
            : 'Write the month as a name — 02 Sep 2024 — or the wire form, 2024-09-02.'
      };
    }
    // The FULL names are the accept-set, never a three-letter prefix alone: a
    // prefix test takes `Sepxyz` for September, which is a month the operator
    // did not type. `Sept` is accepted on the way IN and never emitted on the
    // way out — `$lib/dates.js` fixes the abbreviation at three letters.
    const want = t[ai].toUpperCase();
    const mo = MONTHS.findIndex((full) => full.toUpperCase().startsWith(want));
    if (mo < 0) return { err: `“${t[ai]}” is not a month. Use Jan … Dec.` };
    const rest = t.filter((_, i) => i !== ai);
    const yi = rest.findIndex((x) => /^\d{4}$/.test(x));
    if (yi < 0) return { err: `Write the year in full: 02 ${MON[mo]} 2024.` };
    const dt = rest[1 - yi];
    if (!/^\d{1,2}$/.test(dt)) return { err: `“${dt}” is not a day of the month.` };
    const y = Number(rest[yi]);
    const d = Number(dt);
    return d >= 1 && d <= daysInMonth(y, mo + 1)
      ? { iso: iso(y, mo + 1, d) }
      : {
          err: `${dt} ${MON[mo]} ${y} is not a date — ${MON[mo]} ${y} has ${daysInMonth(y, mo + 1)} days.`
        };
  }

  /** The last day of a month file — the calendar's `End` key, and nothing else. */
  function monthLastDay(ym) {
    const [y, m] = ym.split('-').map(Number);
    return iso(y, m, daysInMonth(y, m));
  }
  function clockOf(ms) {
    const s = Math.max(0, Math.round(ms / 1000));
    const h = Math.floor(s / 3600);
    const rest = s % 3600;
    const body = `${pad(Math.floor(rest / 60))}:${pad(rest % 60)}`;
    return h > 0 ? `${h}:${body}` : body;
  }
  /**
   * A COUNT ON THE WAY TO A HUMAN, GROUPED THE INDIAN WAY: 8,78,28,617.
   *
   * THE LOCALE IS NAMED, NEVER INHERITED. `toLocaleString()` with no argument
   * takes the host's, so one count read `8,78,28,617` on this machine,
   * `87,828,617` on a US one and `87.828.617` on a German one — three spellings
   * of a single number, which makes a screenshot unreproducible and a bug
   * report unfalsifiable. `en-IN` is what the audit and autopilot pages already
   * pin, and it is the grouping this operator's own readers count in.
   *
   * It changes the SEPARATORS and nothing else: the default fraction bound is
   * three digits either way, and `NaN` and `∞` still render as themselves.
   *
   * Built ONCE, at module scope. `n` is called per row of an outcome list built
   * for 750 instruments today, and a formatter constructed per call would load
   * locale data on every one of them.
   */
  const IN = new Intl.NumberFormat('en-IN');
  function n(x) {
    return IN.format(Number(x ?? 0));
  }

  // ------------------------------------------------------------------- feed

  const active = $derived(feeds.all.find((f) => f.wire === feeds.active));

  // -------------------------------------------------------------- source kind
  //
  // THE OPERATOR'S RULE, 12 AUG 2026, AS A FIELD ON THE ROW — never a list of
  // vendor names in this file:
  //
  //   "for truedata and gdfl alone, one and only, we will pull the data
  //    entirely from csv files from the precise folder … except these two alone
  //    only, for all other vendors or brokers feeds it should be always REST."
  //
  // `pull::vendor::SourceKind` is that rule as a type, and `/feeds.json`
  // carries it per row as `kind`, `kind_label` and `verb`. All three are
  // `const fn` on the Rust side, so none costs the server a filesystem call and
  // all three can sit on a per-render route.
  //
  // FIVE THINGS TURN ON IT, and each is a real difference rather than a label:
  //
  //   | | REST | folder |
  //   |---|---|---|
  //   | credential | required | none, and §8 must not run |
  //   | quota | a budget | none — no request is made |
  //   | reach | a history floor | THE FILES PRESENT |
  //   | finest rung | one minute | one SECOND |
  //   | the verb | pull | READ |
  //
  // ═════ `transport` IS GONE, AND THE GUESS THAT READ IT WENT WITH IT ═════
  //
  // `/feeds.json` used to emit the SAME two-valued split twice: `transport`
  // (`broker` / `archive`), matched off `Transport`'s arms in the handler, and
  // `kind` (`rest` / `folder`), read from `SourceKind`. This file then wrote
  // `active?.kind ?? (active?.transport === 'archive' ? 'folder' : 'rest')` —
  // a hardcoded reconstruction of the fact, standing behind the fact, looking
  // exactly as authoritative as the real thing. It is the §4 shape and it is
  // the same one the granularity table above was: nothing on screen said which
  // of the two answered, so a server that stopped sending `kind` would have
  // gone on rendering a confident page built from a guess.
  //
  // ONE FIELD, NO FALLBACK. `null` when this server states nothing, and
  // `sourceKindUnstated` below turns that into a sentence rather than into a
  // default. Both flags are false in that state ON PURPOSE — the page must not
  // draw the broker form OR the folder form for a feed whose kind it does not
  // know, and `!isBroker` is no longer a spelling of "folder".
  const sourceKind = $derived(
    active?.kind === 'rest' || active?.kind === 'folder' ? active.kind : null
  );
  const isFolderFeed = $derived(sourceKind === 'folder');
  const isBroker = $derived(sourceKind === 'rest');
  /**
   * THE SERVER SENT A FEED AND DID NOT SAY WHAT KIND OF SOURCE IT IS.
   *
   * Not a shrug and not a default: `crates/pull` has a transport for every feed
   * it can name, so this is a running binary that predates the field. It is
   * stated where the form would otherwise have been drawn, because the form's
   * SHAPE is what the answer decides — a credential and a rate budget, or a
   * path on this disk.
   */
  const sourceKindUnstated = $derived(Boolean(active) && sourceKind === null);
  /**
   * The sentence that stands where the guess used to. Written once, because it
   * is said in three places and three wordings of one refusal is how a reader
   * learns to skip all of them.
   */
  const SOURCE_KIND_UNSTATED =
    'This server did not say whether this feed is a REST API or a folder of files. That is a MISSING FIELD and not a vendor with nothing to say: /feeds.json emits kind, kind_label and verb on every row from pull::vendor::SourceKind, which crates/pull derives from the transport every feed declares. This page holds no table to answer from — a guess reconstructed from another field would look exactly as authoritative as the fact, and the two decide different forms: a credential and a rate budget, or a path on this disk. Restart the server on a build that emits it.';
  /**
   * ARCHIVE FEEDS STAY OUT OF SIGHT UNTIL THEIR DATA IS BOUGHT, and the gate is
   * the server's own `ready` flag rather than a list of vendor names here. The
   * day TrueData has a store prefix its row flips and the folder form appears
   * with no edit to this file — the same reason the feed picker is built from
   * `/feeds.json`.
   */
  const archiveHidden = $derived(isFolderFeed && active?.ready === false);
  /**
   * THE HONEST VERB, and it is the server's word rather than this file's.
   *
   * A folder is not pulled, fetched or requested: nothing is asked of anybody,
   * no quota moves, nothing can rate-limit it and there is no remote party to
   * be unavailable — the bytes are already on the disk. Calling it a pull put
   * every failure in the wrong diagnostic frame, and the first questions asked
   * were always about tokens, entitlements and outages when the answer was a
   * path.
   *
   * AND THE FALLBACK IS NEUTRAL, not `pull`. A server that states no verb is
   * one this page cannot ask the question of, and defaulting to `pull` would
   * print the network word over a folder read — the exact wrong diagnostic
   * frame this field exists to stop. `ingest` claims neither side.
   */
  const verb = $derived(active?.verb ?? 'ingest');
  /** The verb with its first letter raised, for the start of a sentence. */
  const Verb = $derived(verb.charAt(0).toUpperCase() + verb.slice(1));

  // ------------------------------------------------ how far a folder reaches
  //
  // READ OFF THE DISK, AND THERE IS NOTHING ELSE IT COULD BE.
  //
  // A REST vendor publishes how far back it answers and `/feeds.json` carries
  // that claim with its source. NOBODY PUBLISHES ANYTHING about a directory on
  // this machine, so the reach of a folder feed is whatever files are in it —
  // which means it has to be READ, and reading it means walking the folder.
  //
  // That walk is why this is its own route. `/feeds.json` renders on every page
  // load and an O(files) probe there would be the cost `/store` exists to
  // avoid, so `GET /folder.json?feed=<wire>` answers it on demand and this page
  // asks once, when a folder feed is actually chosen. See `crates/api/src/folder.rs`.
  //
  // THREE ANSWERS AND A HALT, AND THE PAGE MUST NOT COLLAPSE THEM:
  //   empty — the folder is there and holds nothing. AN ANSWER. The operator
  //           has not bought that month yet.
  //   blank — files were delivered and they carry no rows. Different: they were
  //           bought, and they are blank.
  //   days  — the real span, both ends inclusive.
  //   halt  — missing, unreadable, or holding something that will not decode.
  //           NOT an empty reach. It names the PATH, because the path is the
  //           one thing an operator needs next and the one thing a run that
  //           produced no bars never used to say. `CLAUDE.md` §4 bans the
  //           version of this that reads as "no data yet".
  let folderReach = $state({ wire: null, state: 'idle', body: null, why: null });

  $effect(() => {
    const wire = feeds.active;
    if (!wire || !isFolderFeed) return;
    // `untrack` AROUND THE READ OF `reach`, for the reason the rung effect
    // below gives: this effect WRITES `reach`, so reading it as a dependency
    // would make every one of its own writes schedule another run. The only
    // thing that may trigger this is a FEED CHANGE.
    if (untrack(() => folderReach.wire === wire && folderReach.state !== 'idle')) return;
    let live = true;
    folderReach = { wire, state: 'reading', body: null, why: null };
    fetch(`/folder.json?feed=${encodeURIComponent(wire)}`)
      .then(async (r) => ({ ok: r.ok, data: await r.json() }))
      .then(({ ok, data }) => {
        if (!live) return;
        // A HALT IS KEPT AS A HALT. `ok` is false for every refusal the server
        // makes, and the body carries `path` on the ones that have one — so
        // the page can name the folder rather than saying "nothing found".
        folderReach = ok
          ? { wire, state: 'read', body: data, why: null }
          : { wire, state: 'halted', body: data, why: data?.refused ?? 'refused' };
      })
      .catch((why) => {
        if (!live) return;
        // LOUD, NOT SILENT. A reach that quietly fails to load must not render
        // as an empty folder — that is exactly the confusion this route exists
        // to end, arriving by a different door.
        folderReach = { wire, state: 'halted', body: null, why: String(why) };
      });
    return () => {
      live = false;
    };
  });

  /**
   * THE FOLDER'S ANSWERABLE RANGE, IN WORDS, and every one of them is read.
   *
   * `null` for a REST feed — it has a history floor instead, and the two must
   * never be rendered by the same sentence.
   */
  const folderReachSentence = $derived.by(() => {
    if (!isFolderFeed || !active) return null;
    const path = folderReach.body?.path;
    const at = path ? ` · ${path}` : '';
    if (folderReach.state === 'reading') return `Reading ${active.display}'s folder…`;
    if (folderReach.state === 'halted') {
      return `${active.display} · the folder could not be read${at} — ${folderReach.why}`;
    }
    const r = folderReach.body?.reach;
    if (!r) return null;
    if (r.state === 'days') {
      return `${active.display} answers for ${dayLabel(r.earliest)} – ${dayLabel(r.latest)} — read from the ${n(r.files)} file(s) present, ${n(r.rows)} row(s)${at}`;
    }
    if (r.state === 'blank') {
      return `${active.display} · ${n(r.files)} file(s) are there and not one carries a row${at}`;
    }
    return `${active.display} · the folder is there and holds nothing${at}`;
  });

  /**
   * THE ARCHIVE CENSUS — the folder described as a set, not as a sentence.
   *
   * # Why this exists beside `folderReachSentence` rather than instead of it
   *
   * `/folder.json` answers six facts — `path`, `state`, `earliest`, `latest`,
   * `files`, `rows` — and every one of them was being spent on ONE line at the
   * foot of a calendar popover. That line is the right thing in the place it
   * sits; it is the wrong thing to be the only place the six exist, because
   * they are the closest thing an archive feed HAS to a universe.
   *
   * # An archive has no universe to pick, and this is what it has instead
   *
   * A broker is asked for a SET and answers it: `SpotTarget` names one, the
   * master resolves it, and `/universes.json?feed=` counts what the feed
   * reaches. An archive is asked for nothing. Its instruments are the FILES the
   * operator bought, so the only true answer to "what is in this feed" is a
   * walk — which is what this is. `crates/api/src/constituents.rs` states the
   * other half from the join's side: a vendor with no master `lacks` every
   * name, which is a fact about a master that does not exist rather than about
   * a folder that does.
   *
   * # `null` FOR A REST FEED, ALWAYS
   *
   * Not an empty census — `null`. A broker has a history floor and a master,
   * and rendering "0 files" for one would be this page inventing a folder for a
   * vendor that has none. The two must never share a renderer.
   *
   * # WHAT IS NOT HERE, AND IT IS NAMED RATHER THAN OMITTED
   *
   * THE INSTRUMENT LIST. `pull::archive::Member` already carries `instrument`,
   * taken from the file name with its extensions removed, and
   * `pull::folder::reach_of` counts the members and discards their names. So
   * the walk HAS them and the wire does not carry them. That is a server field
   * that does not exist yet, not a fact this page may reconstruct: the file
   * names are the only identity an archive has, and a browser guessing them
   * from a count would be inventing the one thing it cannot check.
   *
   * O(1) — six field reads off a body fetched once per feed change.
   */
  const archiveCensus = $derived.by(() => {
    if (!isFolderFeed || !active) return null;
    if (folderReach.state === 'reading') return { state: 'reading' };
    if (folderReach.state === 'halted') {
      return { state: 'halted', path: folderReach.body?.path ?? null, why: folderReach.why };
    }
    const r = folderReach.body?.reach;
    if (!r || typeof r.state !== 'string') return null;
    // THE NAMES, AND `null` RATHER THAN `[]` WHEN THE SERVER SENT NONE.
    //
    // An empty array is a folder that names nothing — an ANSWER, and the one
    // `Reach::Empty` exists for. A missing key is a RUNNING BINARY THAT
    // PREDATES THE FIELD, which names a different fix: restart on a newer
    // build. Collapsing the two would print "0 instruments" at a server that
    // was never asked, which is the §4 shape and the same discipline
    // `rungServed` and `finestByWire` already read their fields under.
    const named = folderReach.body?.instruments;
    return {
      state: r.state,
      path: folderReach.body?.path ?? null,
      earliest: r.earliest ?? null,
      latest: r.latest ?? null,
      files: Number(r.files ?? 0),
      rows: Number(r.rows ?? 0),
      instruments: Array.isArray(named) ? named : null,
      collisions: Number(folderReach.body?.collisions ?? 0)
    };
  });

  // ---------------------------------------------------------------- the form

  /**
   * EVERY ONE OF THESE IS INTENT AND SURVIVES A FEED CHANGE.
   *
   * Nothing below resets them when the feed in the first control moves, and that is
   * deliberate: unticking a feed and re-ticking it must RESTORE the choice, not
   * destroy it. What a feed change does move is what those choices can REACH —
   * counted separately, further down, and never read off these.
   */
  /**
   * THE UNIVERSE IS THE CHOICE; THE TARGET IS WHAT IT SPELLS ON THE WIRE.
   *
   * Two names for one decision would be two things to keep in step, and the
   * one that drifts is always the copy nobody remembers exists. `universe` is
   * the only state; `target` is derived from it and is never assignable, so a
   * request cannot carry a target the rung above it did not choose.
   *
   * The default is the swept pair because it is the one set every transport
   * can serve — a broker included. A default further down the list would leave
   * the form's first legal state illegal on the feed it opens with.
   */
  // THE DEFAULT IS A SET THIS MENU ACTUALLY OFFERS.
  //
  // It was `'swept'`, and 1c06dab removed the sweep pair from the menu at the
  // operator's instruction — so the control kept DISPLAYING
  // "NSE-NIFTY + NSE-BANKNIFTY" as its value while the list no longer contained
  // it. Removing an option without moving the state off it leaves exactly that:
  // a button showing a choice nobody can make.
  //
  // `n50` is the first row of the menu and it resolves to a real target
  // (`target=n50`), so the page opens on something a request can name.
  let universe = $state('n50');
  /**
   * Is the universe drawer open? Sets no request can name are folded behind one
   * line rather than listed dead above the ones that work — see the drawer in
   * the markup. Closed by default; nothing is deleted and every refusal is one
   * click away.
   */
  let uniTuck = $state(false);
  /**
   * The feeds this build cannot pull, and whether their drawer must be open.
   *
   * Same trap as the universe drawer: `feeds.active` can BE one of these — the
   * default picks the feed holding the most bars, but an operator can select a
   * not-ready feed and the page has blank states for exactly that. A closed
   * drawer holding the current selection would draw a menu with no tick, so the
   * open state is derived and forced whenever the selection is inside.
   */
  const notReadyFeeds = $derived(feeds.all.filter((f) => f.ready !== true));
  let feedTuck = $state(false);
  // Same promotion rule as the universe drawer, for the same reason: forcing it
  // open whenever the selection was inside meant it was open whenever an
  // operator had selected a not-ready feed, which is exactly when the menu most
  // needs to be short. The selected feed is rendered above the drawer instead.
  const feedOpen = $derived(feedTuck);
  const tuckedFeeds = $derived(notReadyFeeds.filter((f) => f.wire !== feeds.active));
  const activeNotReady = $derived(notReadyFeeds.find((f) => f.wire === feeds.active) ?? null);
  // THE SELECTED ROW IS PROMOTED OUT OF THE DRAWER, NOT AUTO-OPENED INTO VIEW.
  //
  // The first draft forced the drawer open whenever the selection lived inside
  // it, so that a menu could never draw without a visible tick. That reasoning
  // was right and the result was useless: `universe` DEFAULTS to `'swept'`, so
  // the selection was always inside, so the drawer was always open, so the menu
  // looked exactly as it had before and the operator correctly said nothing had
  // changed.
  //
  // Promoting instead satisfies both: the chosen row is always visible with its
  // tick, and the drawer holds only what is NOT chosen. Pick a NIFTY family and
  // the sweep pair folds away; pick the sweep pair and it rides up beside them.
  const sweptIsChosen = $derived(universe === SWEPT.id);
  const uniTucked = $derived(refusedUniverses.map((x) => x.u));
  const uniOpen = $derived(uniTuck);
  const uniTuckLabel = $derived(
    uniTucked.length === 1
      ? `1 set ${sweptIsChosen ? 'no request can name' : 'more'}`
      : `${n(uniTucked.length)} set(s) not offered here`
  );
  let segSet = $state(new Set(['spot']));
  /**
   * THE TIMEFRAME IS A SET, NOT A VALUE.
   *
   * It was one rung and one only, which made the ask arithmetic multiply by a
   * constant 1 and made the census able to describe a single row of the store.
   * An operator who wants the minute bars AND the daily bars for a window was
   * asking twice and comparing by memory.
   *
   * WHAT THE WIRE STILL CARRIES IS ONE RUNG. `api::ingest::SpotRequest` has one
   * `granularity` field, so N ticked rungs are N requests, sent in the ladder's
   * own order, and the wire fold prints every one of them. That is a fact about
   * the route and it is stated rather than hidden behind a control that takes
   * more ticks than it can send.
   */
  let rungSet = $state(new Set(['1min']));
  /** The ticked rungs, in the ladder's order. Never the tick order. */
  const rungsChosen = $derived(RUNGS.filter((r) => rungSet.has(r.dir)));
  /**
   * THE FIRST TICKED RUNG, for the things that can only be about one: the
   * header chip and the feed floor's own sentence. Every COUNT reads
   * `rungsChosen`, never this.
   */
  const rung = $derived(rungsChosen[0]?.dir ?? '');

  /**
   * WHETHER THIS BUILD FETCHES A RUNG AT ALL — READ FROM THE SERVER, NOT
   * TRANSCRIBED.
   *
   * `GET /feeds.json` emits a `history` array per feed with one row per rung
   * the feed either serves or has a history claim recorded for, and `served`
   * on that row is `Descriptor::granularities` — the SAME bit
   * `crates/api`'s `served()` gates the POST on. That gate refuses an
   * undeclared rung by name, before it reads a credential, because this feed's
   * bars path is pinned to one rung and a request for another comes back with
   * a DIFFERENT bar length filed under the one that was asked for.
   *
   * A rung ABSENT FROM THE ARRAY is a rung the server declined to emit, which
   * its own comment defines as "neither served nor recorded" — so absent from
   * the array is `false`, not unknown.
   *
   * `null` WHERE THE ARRAY ITSELF IS ABSENT, and that is a real state rather
   * than a defensive shrug: the running binary may predate the field. A missing
   * answer is reported as missing and nothing is refused on the strength of it
   * — the same discipline `membershipTokens` reads the `universes` array under.
   */
  const rungServed = $derived.by(() => {
    const rows = active?.history;
    if (!Array.isArray(rows)) return null;
    const m = new Map();
    for (const h of rows) if (h && typeof h.rung === 'string') m.set(h.rung, h.served === true);
    return m;
  });
  /** `true`, `false`, or `null` when this server states nothing about it. */
  function feedFetches(dir) {
    if (!rungServed) return null;
    return rungServed.get(dir) ?? false;
  }

  /**
   * ONE ROW PER RUNG, CARRYING EVERY REFUSAL THAT APPLIES TO IT AND NO OTHERS.
   *
   * THREE INDEPENDENT QUESTIONS, ASKED SEPARATELY AND NEVER COLLAPSED INTO ONE
   * WORD:
   *
   *   can the VENDOR ever serve it   `/feeds.json` `finest` — and this is the
   *                                  only one that disables a row
   *   does this BUILD fetch it       `/feeds.json` `history[].served`
   *   can the STORE file it          `RUNGS.stored`
   *
   * ONLY THE FIRST DISABLES, and the rule is worth stating because the other
   * two are tempting: a row is drawn dead only where nothing anybody could ever
   * do makes it exist. Every other refusal is drawn LIVE and stated, because
   * each of them names work a person could do — record an endpoint in
   * `pull::vendor`, widen `store::path::Timeframe` — and a control that hides
   * them hides the work along with the refusal. It is also why the store's
   * annotation stays on rows the vendor refuses: a rung can be refused twice,
   * and saying only the louder half is still a half.
   *
   * O(1) per row, three rows. No lookup here scans.
   *
   * TWO OF THE THREE ARE DRAWN ON EVERY FEED and the third is drawn on the two
   * whose floor reaches it — see `RUNGS`. The vendor question is the only one
   * that removes a row, so "which feeds show the second rung" is answered by
   * `/feeds.json` on every load and by nothing written in this file.
   */
  const rungRows = $derived.by(() => {
    const wire = feeds.active ?? '';
    const name = feedName(wire);
    return RUNGS.map((r) => {
      const v = floorVerdict(wire, r.dir);
      const fetches = feedFetches(r.dir);
      const says = [];
      const long = [];

      // ── THE VISIBLE LINE IS THE VENDOR'S, AND ONLY THE VENDOR'S ──
      //
      // It carries the fact that had no representation anywhere before this,
      // and it carries nothing else. An earlier turn of this put all three
      // refusals on every row and the result was "the store has no directory
      // for it" repeated down nine of eleven rows — which is where a reader
      // stops reading, and the one sentence that mattered was on the row they
      // stopped at. Eleven is three now and the argument is unchanged: one row
      // says one thing. The other two refusals are compact in the detail column,
      // whole in the hover, and in full sentences under the form for the rungs
      // actually TICKED, which is where they are actionable.
      if (v.state === 'refused') {
        says.push(
          `${shortFloor(wire)} Nothing makes this rung exist — not a pull, not an entitlement, not a purchase, not a code change.`
        );
        long.push(`${v.floor.because} — ${v.floor.source}`);
      } else if (v.state === 'finest') {
        // THE BOOLEAN, NOT THE WORD. `conflated` is
        // `pull::vendor::FinestKind::is_conflated` answered on the server, so
        // this page never decides from a string whether the sentence beside a
        // number may say "tick".
        //
        // THIS IS THE ROW THE OPERATOR CALLS THE TICK ROW, and it is the only
        // place on this control where the word can be answered honestly. It is
        // drawn on exactly the feeds whose stated floor reaches a second — the
        // two archives — and its sentence is what the archive was MEASURED to
        // hold, not what its file name says.
        says.push(
          v.floor.conflated === true
            ? `The finest ${name} serves, and it is what its tick archive actually holds: one record is a CONFLATED SNAPSHOT — the best bid, the best ask and the best last price as of that second. SECOND-KEYED: two rows can share a second, and that is expected input. Never a print.`
            : `The finest ${name} serves, and one record is a ${v.floor.label ?? v.floor.kind}.`
        );
        long.push(`${v.floor.label ?? v.floor.kind}. ${v.floor.because} — ${v.floor.source}`);
      } else if (v.state === 'unstated' && wire) {
        long.push(
          `This server sent no granularity floor for ${name}, so nothing here refuses any rung for it. That is a MISSING FIELD and not a vendor with nothing to say: crates/pull carries a floor for every feed it can name, and /feeds.json emits it as finest on every row — a row without one is a running binary that predates the field, or one that sent only half of it. This page holds no copy of the table to answer from, deliberately. Restart the server on a build that emits it.`
        );
      }

      // ── THE OTHER TWO REFUSALS: COMPACT IN THE COLUMN, WHOLE IN THE HOVER ──
      //
      // The detail column states the STRONGEST true thing about the row and
      // stops. These two are the refusals a PERSON can close, so they never
      // remove a row — they annotate one. `no store directory` on the second
      // rung is the whole of why a tick pull cannot land today, and it is the
      // sentence that must not be softened: the feed answers, the request
      // parses, and the bar has nowhere to go.
      if (fetches === false) {
        long.push(
          `${name} does not declare this rung: crates/api's served() refuses the POST for a rung pull::vendor::Descriptor::granularities does not carry — by name, and before it reads a credential — because this feed's bars path is pinned to one rung and a request for another would be answered with a DIFFERENT bar length and filed under the one you asked for. Fixable, and not from here: record the endpoint in pull::vendor.`
        );
      }
      if (!r.stored) {
        long.push(
          'The store has no directory for it, and the refusal is at the WRITE boundary rather than here: store::path::Timeframe::KNOWN ships 1min, 3min, 5min, 15min, 30min, 60min and 1day, and nothing below a minute, so pull::vendor::Granularity::store_timeframe answers None and pull::ingest::Plan::timeframe refuses the bar BY NAME rather than filing it under a rung it is not. That is a store-format limit and not a vendor one — the feed answers, and nothing here can file what it answers with. The fold that makes these same one-second records storable already exists (pull::fold, one-second buckets); it runs when this feed is asked for 1 minute.'
        );
      }

      const status = v.permanent
        ? 'never served'
        : fetches === false && !r.stored
          ? 'not fetched · no store dir'
          : fetches === false
            ? 'not fetched by this build'
            : !r.stored
              ? 'no store directory'
              : r.per === null
                ? 'no session count'
                : `${n(r.per)} bar(s) per session`;

      return {
        key: r.dir,
        name: r.label,
        detail: `${r.dir} · ${status}`,
        disabled: v.permanent,
        why: says.join(' ') || undefined,
        title: long.join(' ') || undefined,
        state: v.state,
        fetches,
        stored: r.stored,
        label: r.label
      };
    })
      // A RUNG THIS FEED CAN NEVER SERVE IS NOT DRAWN AT ALL.
      //
      // This list used to keep them, struck through, behind a
      // "N rows this feed cannot serve" drawer, and the reasoning above this
      // function argued for it: hiding a refusal hides the work that would
      // close it. That argument holds for the rungs a PERSON could make exist
      // — one this build does not fetch yet, one the store has no directory
      // for — and those are still drawn, live, with their sentence.
      //
      // It does not hold for `permanent`. `v.permanent` is `/feeds.json`'s
      // granularity floor: the vendor does not publish this rung and no pull,
      // entitlement, purchase or code change makes it exist. There is no work
      // behind that row. On Groww it drew tick, 1 second and 5 seconds dead on
      // every load — three of eleven rows, permanently, saying nothing that
      // changes with anything the operator can do.
      //
      // Removed at the operator's instruction, 14 Aug 2026, stated twice. The
      // fact is not lost: the floor is still on `/feeds.json`, the feed's own
      // caption still names its finest rung, and a feed that DOES serve one
      // second — TrueData, GDFL — still shows it, because `permanent` is
      // answered per feed and not per rung. That is the whole of why this is a
      // filter here and not a deletion from the ladder. D-0137.
      //
      // ═══ AND THIS LINE IS NOW THE WHOLE OF "TICKS FOR TRUEDATA OR GDFL
      // ALONE" ═══
      //
      // The operator's rule of 14 Aug 2026 asks for the second rung on exactly
      // two feeds. Nothing in this file names them. `RUNGS` offers `1s`
      // unconditionally; `floorVerdict` marks it `refused` and `permanent` for
      // any feed whose `/feeds.json` floor is a minute; this filter drops it.
      // Dhan and Groww bottom out at a minute and the two archives bottom out
      // at a second, so the row is drawn for those two and for nobody else —
      // decided by the server on every load. A pair of feed names written into
      // a condition here would answer the same today and be a second copy of a
      // vendor fact tomorrow, which is what D-0126, D-0131 and D-0138 each
      // deleted from this page.
      .filter((row) => !row.disabled);
  });

  /**
   * THE SELECTED FEED'S FLOOR, RESOLVED ONCE.
   *
   * `null` when THIS SERVER sent no floor for the active feed, which the
   * control states rather than papers over — and it is the only reason it can
   * be null now, because the page holds no table of its own to fall back to.
   * `label` is the rung's DISPLAY name — the prose beside it is for a reader,
   * and `1min` is the key the store and the wire use, never the words a
   * sentence is built from. `short` is composed from the row's own fields; see
   * `shortFloor`.
   */
  const feedFinest = $derived.by(() => {
    const wire = feeds.active ?? '';
    const f = finestByWire.get(wire);
    if (!f) return null;
    // `rungPhrase`, NOT `label` AND NOT THE WIRE'S `label`. Three names collide
    // on this one line and each would be wrong in its own way:
    //
    //   the wire's `label`  is the KIND's sentence — "conflated snapshot — best
    //                       bid, best ask, best last price" — and it is the
    //                       right half of "is a ___". Spelling the rung into
    //                       this key would have printed "one record at 1 second
    //                       is a 1 second".
    //   the rung's `label`  is the BUTTON word, so this would read "one record
    //                       at Ticks is a conflated snapshot" — this page
    //                       calling a snapshot a tick, in the one sentence
    //                       written to say it is not one.
    //   the rung's `phrase` is the measured name, and it is the only one that
    //                       belongs in a clause: "one record at one second".
    return {
      ...f,
      short: shortFloor(wire),
      rungPhrase: RUNGS.find((r) => r.dir === f.rung)?.phrase ?? f.rung
    };
  });

  /** What the rows add up to, counted once and read by every line that states it. */
  const rungTally = $derived.by(() => {
    let dead = 0;
    let unfetched = 0;
    let unstored = 0;
    for (const row of rungRows) {
      if (row.disabled) dead += 1;
      if (row.fetches === false) unfetched += 1;
      if (!row.stored) unstored += 1;
    }
    // `dead` IS NOW ALWAYS ZERO and the field stays, because the two callers
    // read it to decide whether to say anything at all — and a feed whose floor
    // this server did not send has `permanent === false` on every row, so the
    // count is the honest way to ask "were any dropped" rather than an
    // assumption that none were.
    return { dead, unfetched, unstored, live: rungRows.length };
  });

  /**
   * A RUNG THE NEW FEED CAN NEVER SERVE IS DROPPED, AND THE DROP IS THE LOUDEST
   * THING ON THE CONTROL.
   *
   * THE OWNER'S RULE: "as per the chosen broker it should display the entire
   * below sections." Moving the feed re-gates every rung, and a tick that
   * survives the move is a tick the new feed has no word for.
   *
   * BOTH QUIET ANSWERS ARE WRONG, AND THEY ARE WRONG IN OPPOSITE DIRECTIONS.
   * Keeping a ticked `1s` while the feed moves from TrueData to Groww builds a
   * request Groww's wire cannot spell — the interval field has nothing to put
   * in it. Clearing it without a word is the silent rewrite §4 bans, and leaves
   * an operator looking at a control that lost a tick he made and never said
   * so. So it is dropped AND named, and the notice OUTLIVES the change that
   * caused it: it stands until the operator touches the timeframe himself or
   * moves the feed again.
   *
   * ONLY PERMANENT REFUSALS ARE DROPPED. A rung this build does not fetch, or
   * one the store cannot file, stays ticked — those refusals are somebody's
   * work item, and taking the tick away would be this page deciding the work
   * will never be done.
   *
   * `untrack` around the read of `rungSet`, so the only thing that can trigger
   * this is a FEED CHANGE. Without it the effect would re-run on its own write,
   * and an effect that reacts to itself is one bad predicate away from a loop.
   */
  let droppedRungs = $state(null);
  $effect(() => {
    const wire = feeds.active ?? '';
    untrack(() => {
      const gone = [];
      const kept = [];
      for (const r of RUNGS) {
        if (!rungSet.has(r.dir)) continue;
        (floorVerdict(wire, r.dir).permanent ? gone : kept).push(r);
      }
      if (gone.length === 0) {
        // A LATER FEED CHANGE THAT TAKES NOTHING RETIRES THE LAST ONE'S NOTICE.
        // The message is about what THIS change took; left standing under a
        // third feed it names a drop that is two decisions old, which is a
        // stale claim on screen rather than a record.
        droppedRungs = null;
        return;
      }
      rungSet = new Set(kept.map((r) => r.dir));
      droppedRungs = {
        feed: feedName(wire),
        kept: kept.length,
        // ONE REASON, BECAUSE THERE IS ONE WAY TO BE DROPPED. A `never` arm
        // stood beside this one and answered for `tick`; that rung is no longer
        // offered, so the only rung a feed change can take is the second — and
        // it is taken for exactly one reason, which is that the feed it was
        // ticked for reaches below a minute and this one does not.
        rows: gone.map((r) => ({
          dir: r.dir,
          label: r.label,
          why: `${shortFloor(wire)} It was ticked for a feed that reaches below a minute.`
        }))
      };
    });
  });
  /**
   * THE WINDOW IS TWO DAYS, AND THE ISO `YYYY-MM-DD` IS THE KEY FORM.
   *
   * DAYS, BECAUSE THAT IS WHAT THE VENDOR ANSWERS AND WHAT THE WIRE CARRIES:
   * `api::ingest::parse_spot` reads `from` and `to` as `YYYY-MM-DD` and
   * `MAX_WINDOW_DAYS` caps their difference. A month picker stood here and was
   * wrong — it named a unit of the STORE (the month file) as if it were the
   * unit of the REQUEST, and it silently widened `02 Sep` into all of
   * September.
   *
   * THE MONTH HAS NOT VANISHED, because the store still writes one file per
   * month: `/store.json` answers one row per instrument-MONTH and the store
   * address still ends in `/2024-11`. What the day window decides is how much
   * of each of those files is being asked for — `windowMonths` is the set it
   * touches, and it is counted, never assumed.
   *
   * `*Text` is what is IN the field — the characters the operator typed, kept
   * verbatim so a refusal can quote them. `*Day` is the parsed value, and it is
   * empty whenever the text does not parse: an unreadable string must not leave
   * a stale day behind it and quietly pull a window nobody asked for.
   */
  let fromDay = $state('');
  let toDay = $state('');
  let fromText = $state('');
  let toText = $state('');
  /** The refusal a typed string earned, per field. Cleared the moment one takes. */
  let typedErr = $state({ from: null, to: null });
  let folder = $state('');
  let showProblems = $state(false);

  /**
   * WHICH MENU IN THE STRIP IS OPEN — one value for the whole page, so "two are
   * open at once" is not a state this component can reach.
   *
   * The same rule `$lib/Picker.svelte` holds for itself, and for the same
   * reason: a per-menu `open` flag plus an outside-click close never shuts the
   * first menu, because a click on the SECOND menu's button is not outside the
   * first one's `.dd` — it is inside that one. Picker's own module-level id
   * closes its menus when anything else on the page is pressed, and the
   * pointer-capture handler in `onMount` closes this one for the same press,
   * so the two never overlap in either direction.
   *
   * `'feed' | 'uni' | 'ins' | null`.
   *
   * `'feed'` IS BACK, AND IT IS THE ONLY ONE OF ITS KIND. This page carried its
   * own feed dropdown, then lost it because the top bar carried one too, and
   * two controls for one value teach the reader they are independent when they
   * are not. The rule was right and the remedy was aimed at the wrong control:
   * the feed is the FIRST RUNG of this page's cascade — universe, instruments,
   * segments, timeframe and window are all its answer — so the control belongs
   * beside the four it governs. `+layout.svelte` now omits its picker on the
   * routes that own one (FEED_OWNED there), which keeps the count at exactly
   * one on every route rather than at one page-wide.
   */
  let drop = $state(null);

  /**
   * The chosen universe, and the wire value it spells. `SWEPT` is the concrete
   * floor rather than `UNIVERSES.at(-1)`: a fallback that can be `undefined`
   * needs a guard at every reader, and a missed guard on the row that decides
   * what the request asks for is a request with no target at all.
   */
  const universeSpec = $derived(UNIVERSE_BY_ID.get(universe) ?? SWEPT);
  const target = $derived(universeSpec.target ?? '');

  let nowMs = $state(Date.now());
  const istToday = $derived(istDay(nowMs));
  const istMin = $derived(istMinute(nowMs));

  /**
   * ═════════════ THE CEILING: THE NEWEST SESSION THAT HAS CLOSED ═════════════
   *
   * COMPUTED FROM THE IST CLOCK ON EVERY TICK, NEVER A LITERAL. A date written
   * down here is correct on the day it is typed and one day staler every day
   * afterwards, and nothing on the page can see that it has gone wrong —
   * `nowMs` is re-read twice a second, so this moves on its own at midnight
   * IST and again at 15:30.
   *
   * WHY THE CLOSE MATTERS, and why "yesterday" is not an off-by-one: while a
   * session is still running no feed has a final bar for it. Asking for it
   * yields a PARTIAL day, and an append-only store cannot correct a partial day
   * later. So the newest askable day is the newest session that has actually
   * finished — which is today after 15:30 IST, and the previous session before
   * it. `ceilReason` says which of those it is, in words, wherever the ceiling
   * bites.
   *
   * The walk backwards is bounded by construction: at most three non-sessions
   * can sit in a row here — a long weekend — and the loop is capped anyway.
   */
  const maxDay = $derived.by(() => {
    let d = istToday;
    if (isSession(d) && istMin >= CLOSE_MIN) return d;
    for (let i = 0; i < 10; i += 1) {
      d = addDays(d, -1);
      if (isSession(d)) return d;
    }
    return d;
  });
  /**
   * WHY THE CEILING IS WHERE IT IS. One sentence, one definition, read by the
   * calendar footer, the ceiling day's own tooltip and the refusal for any day
   * after it — three places that must not be able to say different things.
   */
  const ceilReason = $derived.by(() => {
    const today = istToday;
    const guessing = !holidaysKnownFor(today)
      ? ` This page's NSE holiday table covers ${dayLabel(HOL_FROM)} – ${dayLabel(HOL_THRU)} and today is outside it, so a weekday here is ASSUMED to be a session — an NSE holiday would be offered and would come back empty.`
      : '';
    if (isSession(today) && istMin < CLOSE_MIN) {
      return `Today, ${dayLabel(today)}, is a trading session and it has not closed yet — NSE closes 15:30 IST and it is ${pad(Math.floor(istMin / 60))}:${pad(istMin % 60)} — so no feed has a final bar for it. The newest day that can be asked for is ${dayLabel(maxDay)}, the last session that closed.${guessing}`;
    }
    if (isSession(today)) {
      return `Today, ${dayLabel(today)}, is a trading session and it closed at 15:30 IST, so today IS the newest day that can be asked for.${guessing}`;
    }
    return `${dayLabel(today)} — ${noSessionWhy(today)} — so the newest day that can be asked for is ${dayLabel(maxDay)}, the last NSE session before it.${guessing}`;
  });

  /**
   * THE FLOOR OF THIS PAGE'S OWN CALENDAR, and it is the ONE thing here that is
   * allowed to be a constant: `GET /pull`'s own picker offers 2015 onward
   * (`calendar::YEARS_OFFERED`), so nothing older can be asked for through this
   * server at all.
   *
   * IT IS NOT A DATA FLOOR. An earlier revision refused every day before the
   * page's own session table began, which was this page's THINNESS wearing a
   * rule's clothes: a day nobody had loaded a holiday list for is still a day a
   * vendor answers for. The only floor that may take a day away from the
   * operator is the FEED's, below, because that is the one his own choice moves.
   */
  const minDay = $derived(iso(FLOOR_YEAR, 1, 1));

  /**
   * THE SELECTED FEED'S OWN FLOOR, at the timeframe selected, resolved against
   * today so a rolling one moves with the clock. `null` when the feed states
   * none and when nothing has been read for it — `known` tells those apart.
   */
  /**
   * THE STRICTEST FLOOR ACROSS EVERY TICKED RUNG, because the window is one
   * window and it is asked for at each of them. Taking the first rung's floor
   * would offer a day that the second rung's feed refuses, and the refusal
   * would arrive from the vendor rather than from the control.
   */
  const feedFloor = $derived.by(() => {
    const wire = feeds.active ?? '';
    if (rungsChosen.length === 0) return pairFloor(wire, rung);
    // SEEDED FROM THE FIRST RUNG RATHER THAN FROM null.
    //
    // The old shape started at `null` and relied on `!worst` being true on the
    // first pass to fill it. That is true — the early return above guarantees at
    // least one iteration — but it is true by an argument a reader has to
    // reconstruct, and the checker cannot reconstruct it at all: it flagged
    // `worst.f` and `worst.r` on the return as possibly null. Seeding removes
    // the null from the type instead of asserting it away, so the guarantee is
    // in the code rather than in a comment.
    let worst = { r: rungsChosen[0], f: pairFloor(wire, rungsChosen[0].dir) };
    for (const r of rungsChosen.slice(1)) {
      const f = pairFloor(wire, r.dir);
      if (f.at && (!worst.f.at || f.at > worst.f.at)) worst = { r, f };
    }
    return { ...worst.f, rung: worst.r };
  });
  /**
   * The refusal, naming the FEED, the TIMEFRAME and the day it starts at.
   *
   * `label` HERE, NOT `phrase`, AND IT IS THE ONE SENTENCE THAT GOES THAT WAY.
   * The `·` makes this a breadcrumb rather than a clause — FEED · TIMEFRAME —
   * and a refusal must name the control the operator TOUCHED, with the word
   * that is printed on it. Telling him "one second is out of reach" when he
   * ticked a box marked Ticks sends him looking for a control he does not have.
   * Every other rung-in-a-sentence takes `phrase`; see `RUNGS`.
   */
  const floorSentence = $derived.by(() => {
    if (!feedFloor.at) return null;
    const r = feedFloor.rung ?? RUNGS.find((x) => x.dir === rung);
    const others =
      rungsChosen.length > 1
        ? ` It is the strictest of the ${n(rungsChosen.length)} ticked timeframes, and the window is asked for at every one of them.`
        : '';
    return `${feedName(feeds.active)} · ${r?.label ?? rung} answers from ${dayLabel(feedFloor.at)}${feedFloor.rolling ? ' — a rolling floor, so it moves every day' : ''}.${others}`;
  });

  const minMonth = $derived(minDay.slice(0, 7));
  const maxMonth = $derived(maxDay.slice(0, 7));

  /**
   * THE WINDOW ON THE WIRE, AND THERE IS NO CROSSING LEFT TO MAKE. The two
   * fields hold the two days the request carries — no widening, no clamping and
   * no unit change between what was picked and what is sent.
   */
  const from = $derived(isValidIso(fromDay) ? fromDay : '');
  const to = $derived(isValidIso(toDay) ? toDay : '');

  const windowDays = $derived(
    isValidIso(from) && isValidIso(to) ? dayNum(to) - dayNum(from) + 1 : 0
  );
  /**
   * A BACKWARDS WINDOW HAS NO SPAN, and the counters say so instead of printing
   * `-3 days` and `2 instrument-months` as if the request meant something. The
   * refusal below names the fault; the tiles must not quietly contradict it.
   */
  const windowOk = $derived(windowDays > 0);

  /**
   * THE WINDOW ASKED FOR, AGAINST THE DAYS THE FOLDER ACTUALLY HOLDS.
   *
   * `null` unless there is something to say — a REST feed, an unread folder, a
   * folder with no span, or a window sitting wholly inside the measured one all
   * answer nothing.
   *
   * # Why this is a caution and not a refusal
   *
   * The calendar deliberately does NOT strike days outside the folder's span,
   * and its own comment says why: those days are askable and come back empty
   * rather than refused, so striking them would claim a refusal nobody made.
   * That reasoning is right and it is not the whole job. A read of a folder for
   * days that hold no file was always going to land nothing, and `cautions`
   * exists precisely so an operator sees a known blocker BEFORE the clock
   * starts rather than on the receipt.
   *
   * So the day stays pickable and the outcome stops being a surprise.
   *
   * # Why a folder may be compared this way when a vendor may not
   *
   * A REST feed's history floor is a CLAIM — the vendor's own words about how
   * far back it will answer, which this repository does not get to check. A
   * folder's span is a MEASUREMENT taken by walking it. Comparing a window
   * against a measurement is arithmetic; comparing it against a claim is what
   * `pairFloor` already does, separately, with the source printed beside it.
   *
   * Two shapes, because they are two different mistakes:
   *   past      the whole window is outside the span — nothing can land at all
   *   partial   one end is outside — the run is shorter than it looks
   *
   * Placed here rather than beside `archiveCensus` because it reads `from`,
   * `to` and `windowOk`, all declared above this line. A `$derived` is lazy and
   * would have resolved either way; standing above its own inputs is a thing
   * the next reader has to verify, and it does not have to be.
   */
  const windowVsFolder = $derived.by(() => {
    const c = archiveCensus;
    if (!c || !from || !to || !windowOk) return null;
    // AN EMPTY OR BLANK FOLDER IS ITS OWN SENTENCE AND NOT THIS ONE. There is
    // no span to compare a window against, and "outside 0 days" reads as a
    // range rather than as an absence.
    if (c.state !== 'days' || !c.earliest || !c.latest) return null;
    if (to < c.earliest || from > c.latest) return { shape: 'past', ...c };
    if (from < c.earliest || to > c.latest) return { shape: 'partial', ...c };
    return null;
  });
  const windowMonths = $derived(windowOk ? monthsBetween(from, to) : []);

  /**
   * The exact bytes of ONE request. Shown, not described.
   *
   * `granularity` and the two days are parameters rather than reads of the
   * form, because the same encoder builds the row-level pull below: a button on
   * a census row sends this shape with a narrower window and the row's own
   * rung, and there is exactly one place in this file that knows what a request
   * looks like.
   */
  function wireBodyFor(dir, a, b) {
    const p = new URLSearchParams();
    p.set('target', target);
    p.set('vendor', feeds.active ?? '');
    p.set('from', a);
    p.set('to', b);
    p.set('granularity', dir);
    if (isFolderFeed) p.set('folder', folder);
    return p.toString();
  }
  /** One body per ticked rung, in the ladder's order — what Start would send. */
  const wireBodies = $derived(rungsChosen.map((r) => ({ dir: r.dir, label: r.label, body: wireBodyFor(r.dir, from, to) })));

  // -------------------------------------------------------------- validation
  //
  // EACH ONE NAMES THE FIELD, THE VALUE AND THE RULE. "Invalid date" is a
  // refusal that tells the operator nothing; every message below carries the
  // number that failed and the number it failed against.

  const problems = $derived.by(() => {
    // THE BLOCKER NAMES THE CONTROL THAT IS ACTUALLY UNANSWERED.
    //
    // Below the cut the fields are not on the page at all, so every message
    // this function would otherwise produce points at a control the reader
    // cannot see: "From has no date" is true, useless, and sends them looking
    // for a date box that the segment above it retracted. One refusal, naming
    // the one rung that has to be answered next.
    if (gate) return [{ field: gate.id, why: gate.why }];
    const out = [];
    // THE TWO "not a real day" MESSAGES QUOTE THE RAW STRING ON PURPOSE. Every
    // other date below is rendered for a human, because a human reads it; these
    // two are naming the exact value that failed to parse, and a formatter
    // handed an unparseable value would either echo it anyway or replace it
    // with a dash. The reader has to see the characters that are actually in
    // the field, not a tidied version of them.
    for (const [field, text, day] of [
      ['from', fromText, fromDay],
      ['to', toText, toDay]
    ]) {
      const Name = field === 'from' ? 'From' : 'To';
      // A STRING THE PARSER ALREADY REFUSED IS QUOTED BACK IN ITS OWN WORDS.
      // `parseDay` names which of six ways it failed — an ambiguous 02/09/2024,
      // a month that is not a month, a 31st of a 30-day month — and repeating
      // "not a date" over the top of that would replace a specific answer with
      // a vague one.
      if (typedErr[field]) {
        out.push({ field, why: `${Name} reads "${text}". ${typedErr[field]}` });
      } else if (!text.trim() && !day) {
        out.push({
          field,
          why: `${Name} has no date. Type one — ${dayLabel(maxDay)} — or open its calendar with the ▦ button.`
        });
      } else if (!isValidIso(day)) {
        out.push({
          field,
          why: `${Name} reads "${text}", which is not a date this page can read. Write it as "${dayLabel(maxDay)}" or as "${maxDay}".`
        });
      }
    }
    if (isValidIso(fromDay) && isValidIso(toDay) && dayNum(toDay) < dayNum(fromDay)) {
      out.push({
        field: 'to',
        why: `To is ${dayLabel(toDay)}, which is ${n(dayNum(fromDay) - dayNum(toDay))} day(s) BEFORE From (${dayLabel(fromDay)}). A window runs forward — the two are the wrong way round.`
      });
    }
    for (const [field, value] of [['from', fromDay], ['to', toDay]]) {
      if (!isValidIso(value)) continue;
      const Name = field === 'from' ? 'From' : 'To';
      if (dayNum(value) > dayNum(maxDay)) {
        out.push({ field, why: `${Name} is ${dayLabel(value)}, after ${dayLabel(maxDay)}. ${ceilReason}` });
      }
      // THE FEED'S FLOOR IS THE ONLY THING THAT MAY REFUSE AN OLD DAY, and the
      // refusal NAMES IT: which feed, at which timeframe, and from when. "Out
      // of range" tells an operator nothing he can act on; this tells him to
      // move the date, the feed or the rung, and which one would be enough.
      if (feedFloor.at && value < feedFloor.at) {
        out.push({
          field,
          why: `${Name} is ${dayLabel(value)}, and ${floorSentence} Move it to ${dayLabel(feedFloor.at)} or later, or pick a feed or a timeframe that reaches further back. ${feedFloor.src.join(' · ')}`
        });
      } else if (dayNum(value) < dayNum(minDay)) {
        out.push({
          field,
          why: `${Name} is ${dayLabel(value)}, earlier than ${dayLabel(minDay)} — the floor the server's own picker offers (crates/api/src/calendar.rs).`
        });
      }
    }
    if (windowDays > MAX_WINDOW_DAYS) {
      out.push({
        field: 'to',
        why: `${dayLabel(fromDay)} – ${dayLabel(toDay)} is ${n(windowDays)} days. api::ingest::MAX_WINDOW_DAYS caps one request at ${n(MAX_WINDOW_DAYS)} days, so the parser refuses it before any vendor is contacted.`
      });
    }
    if (isFolderFeed && !folder.trim()) {
      out.push({ field: 'folder', why: 'Folder is empty. An archive run reads CSV files from a directory you name.' });
    }
    return out;
  });

  const problemFor = $derived.by(() => {
    const m = new Map();
    for (const p of problems) if (!m.has(p.field)) m.set(p.field, p.why);
    return m;
  });

  /**
   * A DAY THAT WAS LEGAL WHEN IT WAS CHOSEN AND IS NOT ANY MORE.
   *
   * The floor belongs to the feed and the timeframe, and both are controls
   * ABOVE this one: ticking Groww and 1 minute puts a three-month floor in
   * front of a `from` date chosen half an hour ago, and a rolling floor does
   * the same thing overnight with nobody touching anything. The ceiling moves
   * on its own too, at midnight and at 15:30.
   *
   * NOTHING IS SNAPPED. Silently rewriting a window the operator chose is the
   * fallback §4 bans — the date stands, and this is what makes the refusal
   * VISIBLE without waiting for a submit. Field-level messages wait for one
   * because telling someone their date is missing before they opened the
   * calendar is nagging; a bound moving underneath a date they already set is
   * not nagging, it is news.
   */
  const staleWindow = $derived(
    (isValidIso(fromDay) && dayBlock(fromDay) !== null) ||
      (isValidIso(toDay) && dayBlock(toDay) !== null)
  );

  /**
   * Not refusals — cautions. The SERVER has the last word on every one of
   * these, and the request is still sent. They are here because an operator
   * who is about to spend thirty-two minutes should see the known blocker
   * before the clock starts, not on the receipt.
   */
  const cautions = $derived.by(() => {
    const out = [];
    // EVERY TICKED RUNG, NOT THE FIRST ONE. This read `rungsChosen[0]` while
    // the control below it takes N ticks, so a run with `1min` and `1day`
    // ticked described the minute rung and said nothing at all about the other
    // request it was about to send.
    //
    // THREE SENTENCES, BECAUSE THERE ARE THREE REFUSALS AND THEY ARE FIXED BY
    // THREE DIFFERENT THINGS. See `finestByWire` above for why one word
    // standing for all of them is the failure §4 names.
    const unstored = rungsChosen.filter((x) => !x.stored);
    if (unstored.length > 0) {
      const many = unstored.length > 1;
      out.push(
        // `phrase`, NOT `label` — see `RUNGS`. "The store has no directory for
        // ticks" is a sentence about the STORE, and the store has never heard
        // the word: what it has no directory for is one second.
        //
        // AND THE RUNG LIST IS THE STORE'S OWN, NOT TWO OF SEVEN. This read
        // "answers for 1 minute and 1 day only", which D-0132 made false —
        // `Timeframe::KNOWN` has held seven entries since D-0054 and
        // `store_timeframe` answers for five of them. The true fact for the one
        // rung that can reach this line is that nothing below a minute is filed.
        `The store has no directory for ${unstored.map((x) => x.phrase).join(', ')}. store::path::Timeframe::KNOWN ships 1min, 3min, 5min, 15min, 30min, 60min and 1day, and nothing below a minute, so ${many ? 'these rungs are' : 'this rung is'} refused at the WRITE boundary by pull::ingest::Plan::timeframe — expect a refusal rather than bars. That is a store-format limit and not a vendor one: the feed answers, and nothing here can file what it answers with. The fold that makes these same one-second records storable already exists (pull::fold, one-second buckets); it runs when this feed is asked for 1 minute.`
      );
    }
    const unfetched = rungsChosen.filter((x) => feedFetches(x.dir) === false);
    if (unfetched.length > 0 && active) {
      const many = unfetched.length > 1;
      out.push(
        // `phrase` again: what a VENDOR declares is a rung, never a button word.
        `${active.display} does not declare ${unfetched.map((x) => x.phrase).join(', ')}. crates/api's served() refuses a POST for a rung pull::vendor::Descriptor::granularities does not carry — by name, and before it reads a credential — because this feed's bars path is pinned to one rung and a request for another would come back with a DIFFERENT bar length filed under the one you asked for. ${many ? 'Those requests' : 'That request'} will be refused whole. Fixable, and not from here: record the endpoint in pull::vendor.`
      );
    }
    // ══ THE WINDOW AGAINST THE FILES THAT ARE ACTUALLY THERE ══
    //
    // A folder feed has no history floor to refuse an old day with — TrueData
    // and GDFL both declare `history: &[]`, because no vendor page states a
    // floor for a directory on this machine — so nothing above this line stops
    // a window that names days the operator never bought. The calendar says as
    // much and leaves the day pickable, correctly: an unbought day comes back
    // EMPTY, which is not a refusal and must not be drawn as one.
    //
    // What was missing is that it said so only in a popover, after the fact.
    // This is the same fact where a run is about to be started. It is a
    // MEASUREMENT, not a vendor claim: the span was read by walking the folder,
    // so comparing a window against it is arithmetic rather than an opinion.
    if (windowVsFolder && active) {
      const w = windowVsFolder;
      const span = `${dayLabel(w.earliest)} – ${dayLabel(w.latest)}`;
      const at = w.path ? ` The folder is ${w.path}.` : '';
      out.push(
        w.shape === 'past'
          ? `Not one day of this window is in ${active.display}'s folder, which holds ${span} across ${n(w.files)} file(s). The read will not be refused — an archive answers with whatever files are present, and for these days that is none — so expect a clean run that lands ZERO bars. Move the window inside that span, or buy the months first.${at}`
          : `Part of this window is outside ${active.display}'s folder. It holds ${span} across ${n(w.files)} file(s), and the days either side of that hold no file at all: they come back empty rather than refused, so the run will be shorter than the window says and the census will read short without anything having failed.${at}`
      );
    }
    // WHAT A RECORD AT THE FINEST RUNG ACTUALLY IS, said where an operator is
    // about to ask for it. A one-second archive row is a CONFLATED SNAPSHOT and
    // the word "tick" never applies to it — the invoice says otherwise and the
    // invoice is not evidence.
    const snap = rungsChosen.filter((x) => {
      const v = floorVerdict(feeds.active ?? '', x.dir);
      return v.state === 'finest' && v.floor?.conflated === true;
    });
    const snapFloor = finestByWire.get(feeds.active ?? '');
    if (snap.length > 0 && active && snapFloor) {
      out.push(
        // THE VENDOR'S HALF IS THE VENDOR'S WORDS, AND THIS FILE HOLDS NONE OF
        // THEM. This sentence used to carry the measurement itself — "22,426
        // rows across 22,500 seconds", "three of them at TrueData, four at
        // GDFL" — which is a THIRD copy of the fact and the worst-placed one:
        // it was written for whichever archive was active and printed one
        // vendor's numbers under the other's name. `because` and `source` come
        // off the wire and say it per feed.
        //
        // What stays local is the FOLD, and it is not a vendor fact: it is what
        // this repository's reader does with a shared second — SK-08 in
        // `docs/04-invariants.md`. Nobody published it, so nothing on
        // `/feeds.json` could carry it.
        // `phrase`, AND THIS IS THE SENTENCE THE SPLIT WAS BUILT FOR. The
        // control above says TICKS, because that is what the operator bought and
        // what the vendor marked the files. This says what is inside them, and
        // it must not borrow the button word to do it — "ticks is TrueData's
        // finest rung and one record there is a conflated snapshot. It is not a
        // tick" is a sentence that argues with itself.
        `${snap.map((x) => x.phrase).join(', ')} is ${active.display}'s finest rung and one record there is a ${snapFloor.label ?? 'conflated snapshot'}. The archive is MARKED as ticks — the file name says so and the invoice says so — and it does not hold them; it is not a print stream and must never be read as one. ${snapFloor.because} — ${snapFloor.source} THE TIMESTAMP IS KEYED TO THE SECOND, so two rows sharing one second is EXPECTED INPUT and not corruption. They are folded into one record at that second: the first in file order is its open, the extremes are its high and low, the volumes sum, and the LAST in file order is its close — last wins for the price, and nothing is dropped or refused. File order is the only order there is, so nothing is sorted: a sort would invent a tiebreaker and quietly change which price became the open.`
      );
    }
    // THE FORM'S SHAPE IS UNDECIDED, AND THAT IS SAID BEFORE THE CLOCK STARTS.
    // Everything below this line — whether a credential is read, whether a
    // quota is charged, whether `folder` means anything — turns on a field this
    // server did not send. Named here as well as at the control, because this
    // is the list an operator reads last.
    if (sourceKindUnstated) {
      out.push(SOURCE_KIND_UNSTATED);
    }
    // THE ONE-INSTRUMENT CAUTION IS GONE, AND IT WAS A FALSE STATEMENT ABOUT
    // THIS REPOSITORY'S OWN CODE, RENDERED IN A YELLOW BOX.
    //
    // It read: "The broker path in this build addresses ONE instrument.
    // pull::vendor::HttpSpec carries no request-parameter map, so a target
    // naming a set has been refused..." Both clauses were untrue:
    // `HttpSpec::params` exists and both broker descriptors populate it, and
    // `api::server::broker_run` builds the chosen target's instrument list and
    // calls `broker_window` once per member. The guard it described was removed
    // in D-0136 for exactly that reason.
    //
    // Nothing replaces it. A caution has to name a thing that will happen, and
    // this one named a refusal that no longer exists. The counts beside the
    // universe control already say how many instruments a run will attempt, and
    // they are measured rather than asserted.
    if (rung !== '1min') {
      out.push(
        `Live progress below is measured from /store.json, which stamps every row "1m" regardless of rung. Growth shown while this run is going may belong to another rung — it cannot be separated from the wire.`
      );
    }
    // A FEED NOTHING HAS BEEN READ FOR CLAIMS NOTHING, AND THAT IS SAID OUT
    // LOUD — ONCE, AND ONLY WHERE IT CHANGES WHAT HAPPENS. An unstated floor is
    // not an unlimited one: this page refuses no day for such a feed, so an old
    // window WILL be sent and the vendor is the one that will refuse it.
    // Silence here would read as "reaches everywhere", which is the one thing
    // the page must not imply. Feeds whose floor IS stated say so at the day
    // they refuse and nowhere else — see the window rung.
    //
    // AND IT IS A REST SENTENCE. Every clause of it — a depth the VENDOR
    // states, an absence of evidence, "the vendor will answer for how far back
    // it actually goes" — is a fact about an endpoint somebody else operates.
    // ALL OF IT IS FALSE FOR A FOLDER FEED: there is no vendor to answer, no
    // claim to be absent, and the reach is not unknown at all. It is the files
    // present, it was read, and the arm below says what was found.
    if (
      active &&
      !isFolderFeed &&
      !feedFloor.known &&
      windowOk &&
      from < addDays(istToday, -365)
    ) {
      out.push(
        `No history depth is stated for ${active.display} at ${RUNGS.find((r) => r.dir === rung)?.phrase ?? rung}, so nothing here refuses an old day for it and this window reaches back past a year. That is an absence of evidence, not a reach: the vendor will answer for how far back it actually goes. A floor belongs on pull::vendor::Descriptor beside granularities, not in a browser.`
      );
    }
    // A FOLDER FEED'S REACH IS THE FILES, AND THE WINDOW IS CHECKED AGAINST
    // WHAT WAS READ — never against a floor, because there is none.
    //
    // The three answers stay three. An empty folder is not "no data yet" and
    // not a failure: it is a folder that is there and holds nothing, and the
    // thing to do about it is buy the month. A folder that could not be read is
    // a HALT and names the path, because the thing to do about that is fix the
    // path — a completely different hour of an operator's life.
    if (active && isFolderFeed && folderReach.state === 'halted') {
      out.push(
        `${active.display} is a folder feed, so its answerable range is the files present — and this folder could not be read. ${folderReach.body?.path ? `The path is ${folderReach.body.path}. ` : ''}${folderReach.why} Nothing here can say what it reaches until that is resolved, and this is a halt rather than an empty range: the two look identical in a bar count and mean opposite things.`
      );
    }
    if (active && isFolderFeed && folderReach.state === 'read') {
      const r = folderReach.body?.reach;
      const where = folderReach.body?.path ? ` The folder is ${folderReach.body.path}.` : '';
      if (r?.state === 'empty') {
        out.push(
          `${active.display}'s folder is there and holds no file this feed reads, so it answers for NO day at all.${where} That is a read answer and not a missing one — nothing is wrong with the path, and the files for the window you want have not been bought and put there yet. There is no history floor to consult: for a folder feed the reach IS the files.`
        );
      } else if (r?.state === 'blank') {
        out.push(
          `${n(r.files)} file(s) are in ${active.display}'s folder and not one of them carries a row, so it answers for no day.${where} They were delivered and they are blank — which is a different thing to do about than an empty folder, and the reason the two are not one sentence here.`
        );
      } else if (r?.state === 'days' && windowOk && (from < r.earliest || to > r.latest)) {
        out.push(
          `${active.display} answers for ${dayLabel(r.earliest)} – ${dayLabel(r.latest)}, read from the ${n(r.files)} file(s) present, and this window reaches outside that. The part outside will come back with nothing — not because a vendor refused it, but because those files are not in the folder.${where} Buy and drop the months you want; there is no request that can widen this.`
        );
      }
    }
    return out;
  });

  // ------------------------------------------------------------ the universe
  //
  // THE FEED IS THE ROOT OF THIS PAGE and the form says so in its first line:
  // everything below is the selected feed's answer. `/instruments.json` is
  // asked per feed and answers per feed — the two brokers do not list the same
  // instruments — so a count under a feed's name is only a count if the list it
  // was taken from is that feed's.

  /**
   * Whether the catalogue on hand answers for the feed on screen.
   *
   * `$lib/index.svelte.js` stamps every answer with the feed it was asked for.
   * Between a feed change and its reload, and forever after a reload that
   * failed, the rows belong to a DIFFERENT feed — and printing their count
   * beneath this feed's header is exactly the stale value `CLAUDE.md` forbids.
   */
  const catalogueIsThisFeed = $derived(catalogue.ready && catalogue.feed === feeds.active);

  /** A feed's own name for itself, out of `/feeds.json`, never a wire string. */
  function feedName(wire) {
    return feeds.all.find((f) => f.wire === wire)?.display ?? wire ?? 'no feed';
  }

  /**
   * THE TOKENS ONE CATALOGUE ROW CARRIES, OUT OF THE FIELD THAT CARRIES THEM.
   *
   * `/instruments.json` sends TWO membership fields and they are not the same
   * fact. `crates/api/src/server.rs:813` emits both on every row:
   *
   *   · `universe`  — a `+`-joined STRING, deliberately frozen at the first
   *                   `LEGACY_UNIVERSE_TOKENS` = 3 entries of `UNIVERSE_TOKENS`
   *                   (`index`, `fno`, `ntm`). It is the COMPATIBILITY field,
   *                   never widened so an old reader keeps working, and it will
   *                   therefore never name a NIFTY tier however good the data.
   *   · `universes` — the FULL ARRAY, `["index","fno","ntm","n500","n200",
   *                   "n100","n50"]` in that declared order. D-0089 / D-0090.
   *
   * So the ARRAY is read when the row has one and the legacy STRING is split
   * when it does not. That is not a fallback that hides a failure: the string
   * is a complete, correct answer for the three tokens it names, and the four
   * it cannot name are refused BY NAME below rather than guessed at. Nothing
   * here ever synthesises a tier.
   *
   * A FUNCTION DECLARATION so it hoists above `namedByUniverse`, which is the
   * only other place a row's membership is read. One definition, because two
   * answers to "is this name in this set" is how the second one goes stale.
   *
   * The same pair of fields is read the same way on `/db`. This page asks the
   * question about a REQUEST and that page asks it about the DISK, so the two
   * cannot share a module without one of them importing the other's state, but
   * the field precedence is deliberately identical.
   */
  function membershipTokens(row) {
    return Array.isArray(row?.universes)
      ? row.universes.map(String)
      : String(row?.universe ?? '')
          .split('+')
          .filter(Boolean);
  }

  /** The token a universe row is COUNTED from, whichever field names it. */
  function membershipToken(u) {
    return u?.field ?? u?.token ?? null;
  }

  /**
   * The four tokens the LEGACY `universe` string is structurally unable to
   * name, whatever the data holds. `LEGACY_UNIVERSE_TOKENS = 3` in
   * `crates/api/src/server.rs` cuts `UNIVERSE_TOKENS` after `index`, `fno` and
   * `ntm`, so a row without a `universes` array cannot report these four even
   * when the instrument is in all of them.
   */
  const LEGACY_BLIND = new Set(['n50', 'n100', 'n200', 'n500']);

  /**
   * HOW MANY NAMES THIS FEED'S MASTER PUTS BEHIND EACH TOKEN — counted from the
   * rows on hand, never asserted about a binary this page cannot see.
   *
   * This replaced a one-row probe that asked only whether `rows[0].universes`
   * was an array. That answered "does this build emit the field" and was then
   * read as if it had answered "can this tier be pulled", which are different
   * questions with different fixes. A build can emit the field and a feed can
   * still list nothing behind a tier, and an operator told to "rebuild the API"
   * for an empty set is sent to fix the wrong system.
   *
   * EMPTY WHEN THE CATALOGUE IS NOT THIS FEED'S, and every reader treats that
   * as NOT MEASURED rather than as absent — `reachWhy` is the sentence for that
   * state. A tier is called empty only when a master that DOES answer for this
   * feed has been read and does not carry the token.
   *
   * ONE PASS over a list bounded at ~800 rows, recomputed only when the
   * catalogue changes, never per row and never inside a render loop.
   */
  const tokenCount = $derived.by(() => {
    const m = new Map();
    if (!catalogueIsThisFeed) return m;
    for (const row of catalogue.rows ?? []) {
      for (const t of membershipTokens(row)) m.set(t, (m.get(t) ?? 0) + 1);
    }
    return m;
  });

  /**
   * Whether the master on hand carries the ARRAY field at all — one probe, not
   * an assumption about a version.
   *
   * This is the fact that tells a MISSING FIELD from an EMPTY SET, and the two
   * are different findings with different fixes.
   */
  const carriesArray = $derived.by(() => {
    if (!catalogueIsThisFeed) return false;
    for (const r of catalogue.rows ?? []) if (Array.isArray(r?.universes)) return true;
    return false;
  });

  /**
   * How many names this feed's master puts in a universe, or `null` when that
   * cannot be measured yet. `null` is NOT zero and no caller may treat it as
   * zero: one means "nobody counted" and the other is a finding.
   */
  function masterCount(u) {
    if (!catalogueIsThisFeed) return null;
    const rows = catalogue.rows ?? [];
    if (u.token === '*') return rows.length;
    if (u.id === 'swept') return rows.filter((r) => namedByUniverse(r, u)).length;
    const t = membershipToken(u);
    return t === null ? null : (tokenCount.get(t) ?? 0);
  }

  /**
   * Whether one catalogue row is named by a universe.
   *
   * ONE definition, read by the member list and by the not-on-this-feed list
   * alike. Two copies would be two answers to "is this name in this universe",
   * and the second one is always the one that goes stale.
   *
   * MEMBERSHIP IS READ THROUGH `membershipTokens` AND NOWHERE ELSE, so which
   * field answers is decided in exactly one place: the `universes` ARRAY when
   * the row carries it, the frozen `universe` STRING when it does not.
   *
   * The consequence worth stating: a ranked tier answers FALSE against an old
   * binary — the legacy string cannot spell `n50`, so nothing here can — and
   * the row that would have used the answer is refused BY NAME rather than
   * quietly returning an empty set. A tier is never synthesised: NIFTY Total
   * Market arrives in alphabetical order, and slicing its first fifty names
   * would print 360ONE as a NIFTY 50 constituent and call it a fact.
   */
  function namedByUniverse(row, u) {
    if (!u) return false;
    if (u.id === 'swept') {
      // `CLAUDE.md` §1 names these two and no others.
      return row.key === 'NSE-NIFTY' || row.key === 'NSE-BANKNIFTY';
    }
    if (u.token === '*') return true;
    const want = membershipToken(u);
    return want !== null && membershipTokens(row).includes(want);
  }

  /** Which instruments the chosen universe names, out of THIS feed's catalogue. */
  const members = $derived.by(() => {
    if (!catalogueIsThisFeed) return [];
    return (catalogue.rows ?? []).filter((r) => namedByUniverse(r, universeSpec));
  });

  /**
   * WHY A UNIVERSE CANNOT BE ASKED FOR, or `null` when it can. Every branch
   * names the Rust item that would have to change, because "not supported" is
   * a sentence that tells an operator nothing about who can fix it.
   *
   * THREE GAPS, MEASURED IN THE ORDER AN OPERATOR CAN ACT ON THEM.
   *
   *   1 THE SET IS EMPTY HERE. A master that answers for this feed was read and
   *     puts nothing behind the token. No wire can fix that and no rebuild can:
   *     a pull would ask a vendor for nothing. Stated FIRST, and it disables a
   *     row that the wire could otherwise spell — offering a target whose
   *     selection is empty is the fallback `CLAUDE.md` §4 bans.
   *   2 THE FIELD IS ABSENT. The rows carry no `universes` array, so a ranked
   *     tier cannot be counted at all. That is a SERVER BUILD, and the sentence
   *     says which file already has the code.
   *   3 THE WIRE CANNOT SPELL IT. `api::ingest::SpotTarget` has three variants.
   *     This is the gap the four NIFTY tiers are actually stuck behind on a
   *     current binary, and it is the one no browser change can close.
   *
   * A MEASURED ZERO IS A FINDING; `null` IS NOT. `masterCount` returns `null`
   * while no master answers for this feed, and that state falls through to the
   * wire sentence rather than accusing a feed of listing nothing — the ladder
   * above already refuses everything for the uncounted reason.
   */
  function universeRefusal(u) {
    const t = membershipToken(u);
    const have = masterCount(u);
    const rows = (catalogue.rows ?? []).length;
    const feed = feedName(feeds.active);

    // ---- 1 and 2: is this set answerable from THIS feed's master at all?
    if (t !== null && t !== '*' && have === 0) {
      return !carriesArray && LEGACY_BLIND.has(t)
        ? `${u.label} cannot be expressed by /instruments.json?feed=${feeds.active ?? ''} on the API this page is talking to. Its rows answer with the legacy "universe" string only, and crates/api/src/server.rs freezes that string at three tokens — index, fno and ntm — through LEGACY_UNIVERSE_TOKENS. The full "universes" array, which names n500, n200, n100 and n50, is emitted beside it by that same file (D-0089/D-0090): the source has this tier and the running binary predates it. Rebuild and restart the API. What will not happen is a tier synthesised here.`
        : `0 in this feed's master. ${feed} lists no instrument in ${u.label}, counted over all ${n(rows)} row(s) /instruments.json?feed=${feeds.active ?? ''} returned for it. The field that would name this set is present on those rows and not one carries it — an EMPTY SET, not a missing endpoint. Nothing needs rebuilding and there is nothing here to ask a vendor for.`;
    }

    // ---- 3: the set is answerable. Can a request name it?
    if (u.target !== null) return null;
    // THE SENTENCE NO LONGER COUNTS THE ENUM, AND THAT IS THE FIX.
    //
    // It read "api::ingest::SpotTarget spells swept, indices and equities
    // only" — a hand-maintained count that had been wrong since D-0105 took
    // the enum to seven, and wrong again at nine. This page cannot see the
    // enum, so any number it states is a number that goes stale silently.
    // `u.target === null` is the WHOLE condition, and it is the only thing
    // said here.
    const wire =
      'No target names this set, so a pull for it cannot be put on the wire. The set is counted here from the universes array this server sends; what is missing is a request that can ask for it.';
    if (t === null || !LEGACY_BLIND.has(t)) return wire;
    return `${wire} MEMBERSHIP IS NOT THE GAP: ${n(have ?? 0)} name(s) in ${u.label} are counted here for ${feed}, off the universes array this server does send (D-0089/D-0090). This row is waiting on a fourth api::ingest::SpotTarget variant, which is a docs/05-decisions.md entry and a crates/api change — nothing in the browser closes it.`;
  }

  /**
   * THE SETS NO REQUEST CAN NAME, with the reason each one carries.
   *
   * Derived rather than an `{@const}` in the markup, which Svelte 5 permits
   * only as the immediate child of a block — a `<div>` is not one, and the
   * build says so rather than rendering something half-right.
   *
   * It is REACTIVE and must stay so: `universeRefusal` reads the catalogue and
   * the master counts, so a set that is unreachable before `/instruments.json`
   * answers becomes reachable after it, and the drawer has to empty itself
   * when that happens.
   */
  const refusedUniverses = $derived(
    UNIVERSES.map((u) => ({ u, why: universeRefusal(u) })).filter((x) => x.why !== null)
  );

  /**
   * REACH, not intent. `target` is a choice and it survives everything; this is
   * how many names that choice can actually be asked for right now, and it is
   * the only figure any count on this page is allowed to be built from.
   */
  const reach = $derived(members.length);
  const reachKnown = $derived(catalogueIsThisFeed);
  /** Why nothing is counted, when nothing is counted. Never blank. */
  const reachWhy = $derived.by(() => {
    if (reachKnown) return null;
    if (catalogue.error) {
      return `/instruments.json could not be read, so no name is counted here: ${catalogue.error}`;
    }
    if (!catalogue.ready) return 'reading /instruments.json…';
    return `the instrument list on hand answers for ${feedName(catalogue.feed)}, not for ${feedName(feeds.active)} — nothing on this page is counted from another feed's master`;
  });

  /**
   * THE REFUSED ROWS, SAID ON THE PAGE RATHER THAN ONLY ON HOVER.
   *
   * `universeRefusal` above is the complete answer and it is a paragraph, which
   * is the right length for a tooltip and the wrong place for the only question
   * this rung actually gets asked: *why is NIFTY 50 grey?* A greyed button
   * whose reason is reachable only by pointing at it has not answered that, and
   * a keyboard reader never reaches it at all.
   *
   * TWO CAUSES, NEVER MERGED, because they have different owners and different
   * fixes:
   *
   *   · `unspellable` — the set IS counted here, off the `universes` ARRAY this
   *     server sends. Nothing is missing from the wire and no rebuild changes
   *     it: `api::ingest::SpotTarget` has three variants and none names this
   *     set. That is a `docs/05-decisions.md` entry and a `crates/api` change.
   *   · `empty` — a master that answers for THIS feed was read and puts nothing
   *     behind the token. No target and no rebuild helps, because there is
   *     nothing to ask a vendor for. `CLAUDE.md` §4: name the reason out loud,
   *     never offer an empty selection quietly.
   *
   * `null` WHILE NOTHING IS COUNTED. `masterCount` answers `null` then, and a
   * row with no count is not a row with a count of zero — printing it under
   * "0 in this feed's master" would invent a finding out of a loading state.
   * The counted line above already carries `reachWhy` for that state, and a
   * second copy of that sentence here is exactly the drift this file has paid
   * for before.
   *
   * EIGHT ROWS, ONE PASS, recomputed only when the catalogue or the feed
   * changes — `masterCount` reads the `tokenCount` map and never scans.
   */
  const refusalSummary = $derived.by(() => {
    if (!reachKnown) return null;
    const empty = [];
    const unspellable = [];
    for (const u of UNIVERSES) {
      if (universeRefusal(u) === null) continue;
      const have = masterCount(u);
      if (have === null) continue;
      if (have === 0) empty.push(u.label);
      else unspellable.push(`${u.label} (${n(have)})`);
    }
    if (empty.length === 0 && unspellable.length === 0) return null;
    return {
      empty: empty.join(', '),
      unspellable: unspellable.join(', '),
      anyEmpty: empty.length > 0,
      anyUnspellable: unspellable.length > 0
    };
  });

  /**
   * WHAT THE UNIVERSE CONTROL IS, AND WHY FOUR OF ITS ROWS ARE GREY, ON THE
   * CONTROL ITSELF.
   *
   * This was two paragraphs standing open under the rung. They are not
   * dropped: `universeRefusal(u)` is on each refused ROW in full, and the two
   * CAUSES — counted-but-unspellable, and empty-in-this-feed's-master — are
   * summarised here, on the thing they are about. They have different owners
   * and different fixes and are never merged: one is a
   * `docs/05-decisions.md` entry plus a `crates/api` change, the other is a
   * master with nothing behind the token and nothing to ask a vendor for.
   */
  const universeTitle = $derived.by(() => {
    const head = reachKnown
      ? `The set this feed is asked for. ${universeSpec.label} — ${n(reach)} name(s) reachable on ${feedName(feeds.active)}, pulls with target=${target}.`
      : `The set this feed is asked for. ${universeSpec.label} — not counted: ${reachWhy}`;
    if (!refusalSummary) return head;
    const un = refusalSummary.anyUnspellable
      ? ` Counted here, not requestable: ${refusalSummary.unspellable} — membership is measured off the universes array this server sends, and what is missing is a target that can ask for it. A rebuild does not close it: a docs/05-decisions.md entry and a crates/api change do.`
      : '';
    const em = refusalSummary.anyEmpty
      ? ` 0 in this feed's master: ${refusalSummary.empty} — ${feedName(feeds.active)} lists no instrument in these, counted over all ${n((catalogue.rows ?? []).length)} row(s) /instruments.json?feed=${feeds.active ?? ''} returned. An empty set, not a missing endpoint.`
      : '';
    return head + un + em;
  });

  /** The census spells an instrument `EXCHANGE-SEGMENT-SYMBOL`. */
  function censusKey(row) {
    return `${row.exchange}-${row.segment}-${row.symbol}`;
  }

  // ------------------------------------------------- what this feed is MISSING
  //
  // A NAME THIS FEED CANNOT SERVE IS SHOWN AND MARKED, NEVER DROPPED. Hiding it
  // removes the only evidence the gap exists: `/instruments.json?feed=X` returns
  // the names X's master gave an id to, so a name X does not carry is simply
  // absent and the list looks complete.
  //
  // THE COMPLETE MEMBERSHIP IS NOT ON ANY ENDPOINT THIS BUILD SERVES. There is
  // no `GET /universe.json`, and `/instruments.json` has no all-feeds mode — it
  // reads `feed=` and defaults to one. So the widest membership the browser can
  // MEASURE is the union of every feed's master, one request each, and the page
  // says that rather than claiming an index's true constituent list.
  //
  // It is measured on request rather than on load because each of those
  // requests rebuilds the census server-side. A page that quietly fires one per
  // feed every time it opens is paying that cost for a panel nobody asked for.

  let roster = $state({ at: 0, key: '', byKey: new Map(), error: null, busy: false });

  /** Which feeds a roster reading would be ABOUT. A change makes it stale. */
  const rosterKey = $derived((feeds.all ?? []).map((f) => f.wire).join('|'));
  const rosterFresh = $derived(roster.at > 0 && roster.key === rosterKey);

  /**
   * WHY THE MEASUREMENT CANNOT BE TAKEN RIGHT NOW, or `null` when it can.
   *
   * Three separate conditions used to grey this one button and the label said
   * none of them, so a press that did nothing was indistinguishable from a
   * button that was simply decorative. The ladder above states its blocker by
   * name; a disabled control owes the reader the same sentence.
   *
   * Each branch names the condition AND what would settle it, because "not
   * counted yet" is only half an answer — the other half is which request has
   * to come back before the press is worth making.
   */
  const rosterBlock = $derived.by(() => {
    if (roster.busy) {
      return `a reading is already in flight — ${n((feeds.all ?? []).length)} request(s) out, one per feed. A second press would duplicate every one of them, and each rebuilds the census server-side.`;
    }
    if ((feeds.all ?? []).length === 0) {
      return feeds.error
        ? `/feeds.json could not be read, so there is no list of masters to take a union of: ${feeds.error}`
        : '/feeds.json named no feed at all. The union is one request per feed, and with none listed there is nothing to request and nothing to compare this feed against.';
    }
    if (!reachKnown) {
      // A difference needs BOTH sides. The union is the far side; this feed's
      // own names are the near one, and without them a subtraction would be
      // taken against nothing and print a total that looks like a measurement.
      return `${feedName(feeds.active)}'s own names are not counted yet, and this measurement is a difference that needs both sides — ${reachWhy} — so it settles when /instruments.json?feed=${feeds.active ?? ''} answers for the feed named in the first control of the strip. Until it does, a union measured here would be subtracted from nothing and the total would read like a measurement.`;
    }
    return null;
  });

  async function measureRoster() {
    if (roster.busy) return;
    const list = feeds.all ?? [];
    const key = rosterKey;
    roster = { ...roster, busy: true, error: null };
    try {
      const answers = await Promise.all(
        list.map((f) =>
          fetch(`/instruments.json?feed=${encodeURIComponent(f.wire)}`)
            .then((r) => {
              if (!r.ok) throw new Error(`${f.display}: /instruments.json answered HTTP ${r.status}`);
              return r.json();
            })
            .then((rows) => ({ feed: f, rows }))
        )
      );
      const byKey = new Map();
      for (const a of answers) {
        for (const row of a.rows) {
          let held = byKey.get(row.key);
          if (!held) byKey.set(row.key, (held = { ...row, feeds: [] }));
          held.feeds.push(a.feed.display);
        }
      }
      roster = { at: Date.now(), key, byKey, error: null, busy: false };
    } catch (why) {
      // NAMED, NOT SWALLOWED. A measurement that fails silently leaves a dash
      // that reads like "none missing", which is the opposite of the truth.
      roster = { ...roster, busy: false, error: String(why) };
    }
  }

  /**
   * Names in this universe that some feed lists and THIS one does not. `null`
   * until it has been measured — the difference between "none missing" and
   * "nobody counted" is the whole point, and one dash cannot say both.
   */
  const missingHere = $derived.by(() => {
    if (!rosterFresh || !reachKnown) return null;
    const here = new Set(members.map((m) => m.key));
    const out = [];
    for (const [key, row] of roster.byKey) {
      if (here.has(key)) continue;
      if (!namedByUniverse(row, universeSpec)) continue;
      out.push(row);
    }
    return out.sort((a, b) => a.symbol.localeCompare(b.symbol));
  });

  /** How wide the universe is across every feed — reach plus what reach misses. */
  const universeWidth = $derived(missingHere ? reach + missingHere.length : null);

  // ══════════════════════════ WHICH NAMES ARE TICKED ══════════════════════
  //
  // A COUNT IS NOT A CHOICE, and this rung used to be a count. It rendered two
  // summary lines — how many this feed reaches, how many another feed lists —
  // and nothing on it could be ticked. Its own label conceded it: "read, not
  // chosen".
  //
  // WHAT THE WIRE CARRIES AND WHAT THE PAGE MEASURES ARE TWO DIFFERENT FACTS,
  // and both are on screen. `api::ingest::SpotRequest` has no member field, so
  // the REQUEST names a target and the server resolves it — a tick cannot
  // narrow a pull, and `askGap` says so in words the moment the two numbers
  // differ. What a tick DOES decide is what this page counts: the ask
  // arithmetic, every row of the census below, and the outcome list after a
  // run. Those are measurements of the operator's own selection and they were
  // being taken over a set nobody chose.
  //
  // A NAME THIS FEED CANNOT SERVE IS DRAWN, REFUSED AND TOLD WHY — never
  // dropped. Dropping it teaches the reader the name does not exist, which is
  // the opposite of the finding.

  /**
   * WHAT IS **UN**TICKED, not what is ticked — and the inversion is the whole
   * design of this control.
   *
   * A NEW POOL OPENS FULLY TICKED, because that is what a pull actually asks
   * for: `target=` resolves to the whole set on the server. Holding the ticks
   * instead would need a seeding step, that step would need a "has the pool
   * changed" guard, and the first render before the guard ran would print an
   * ask of zero beside a request for everything. Holding the EXCLUSIONS has no
   * such moment: the default is the truth with no work done, a name added to
   * the master later is ticked like its neighbours, and an exclusion is keyed
   * by instrument so it survives a universe change and comes back with it.
   *
   * Keys are `row.key`, the master's own key, everywhere — in this Set, in the
   * `{#each}` key and in every comparison. The symbol is display text.
   */
  let insOff = $state(new Set());
  /** What is in the search box. Text, and it never becomes a selection. */
  let insQ = $state('');

  /** The names this feed carries, sorted. One order, and it is the symbol's. */
  const insPool = $derived(
    [...members].sort((a, b) => cmpStr(a.symbol, b.symbol) || cmpStr(a.key, b.key))
  );

  /** The ticked rows themselves, in pool order. Every count reads this. */
  const ticked = $derived(insPool.filter((m) => !insOff.has(m.key)));
  const insCount = $derived(ticked.length);

  /**
   * EVERY ROW THE MENU DRAWS — what this feed carries, and what it does not.
   *
   * The second group only exists once the union has been measured (it is one
   * request per feed and it is a press, not a page load). Until then the menu
   * says so rather than presenting this feed's list as the whole universe.
   */
  const insRows = $derived.by(() => {
    const rows = insPool.map((m) => ({
      key: m.key,
      sym: m.symbol,
      detail: m.kind ? String(m.kind).toLowerCase() : 'in this master',
      off: null
    }));
    for (const r of missingHere ?? []) {
      rows.push({
        key: r.key,
        sym: r.symbol,
        detail: `only on ${r.feeds.join(', ')}`,
        off:
          `${feedName(feeds.active)}'s master has no id for ${r.symbol}, so there is nothing to ask this feed for. ` +
          `/instruments.json?feed=${feeds.active ?? ''} did not return it; ${r.feeds.join(', ')} did. ` +
          `It is drawn rather than dropped because a name deleted from this list is a gap with no evidence left.`
      });
    }
    rows.sort((a, b) => cmpStr(a.sym, b.sym) || cmpStr(a.key, b.key));
    return rows;
  });

  /** What the filter currently shows. One definition, read by the list AND the
      bulk actions, so the button can never act on a set the eye cannot see. */
  const insShown = $derived.by(() => {
    const needle = insQ.trim().toUpperCase();
    if (!needle) return insRows;
    return insRows.filter(
      (r) =>
        r.key.toUpperCase().includes(needle) ||
        r.sym.toUpperCase().includes(needle) ||
        r.detail.toUpperCase().includes(needle)
    );
  });
  /** Of those, the ones a tick would mean something for. */
  const insSelectable = $derived(insShown.filter((r) => r.off === null));

  /** The control's own face: how many of how many, and it agrees with the ask. */
  const insSummary = $derived.by(() => {
    if (!reachKnown) return '—';
    if (insPool.length === 0) return 'Nothing in this universe';
    if (insCount === 0) return `None of ${n(insPool.length)} ticked`;
    if (insCount === insPool.length) return `All ${n(insPool.length)} ticked`;
    return `${n(insCount)} of ${n(insPool.length)} ticked`;
  });

  /**
   * WHY THE LIST IS EMPTY, WHEN IT IS EMPTY. Never a blank panel: §4 says an
   * empty list states its own reason, and the reason is always a control the
   * reader can reach.
   */
  const insEmptyWhy = $derived.by(() => {
    if (!reachKnown) return reachWhy;
    if (insPool.length === 0) {
      return `${feedName(feeds.active)} lists no instrument in ${universeSpec.label}. Choose another universe above, or a feed whose master carries these names — nothing here is synthesised.`;
    }
    if (insShown.length === 0) {
      return `Nothing in ${universeSpec.label} matches “${insQ.trim()}”. Clear the box to see all ${n(insRows.length)} row(s).`;
    }
    return null;
  });

  /** Sets are replaced, never mutated: an in-place add is not a state change. */
  function toggleIns(key) {
    const next = new Set(insOff);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    insOff = next;
    cPage = 1;
  }
  /** Bulk actions act on WHAT IS SHOWN, so the filter narrows them too. */
  function insSelectShown() {
    const next = new Set(insOff);
    for (const r of insSelectable) next.delete(r.key);
    insOff = next;
    cPage = 1;
  }
  function insClearShown() {
    const next = new Set(insOff);
    for (const r of insSelectable) next.add(r.key);
    insOff = next;
    cPage = 1;
  }
  /** Every name in THIS pool, unticked. A name outside it is not touched. */
  function insClearAll() {
    const next = new Set(insOff);
    for (const m of insPool) next.add(m.key);
    insOff = next;
    cPage = 1;
  }

  // --------------------------------------------------------------- the ladder
  //
  // WHICH CONTROL CAN BE ANSWERED RIGHT NOW.
  //
  // The form's first line states the cascade and the cascade is real: the feed
  // is the root, and each control below answers a question the one above it
  // asked. A control whose parent is unanswered has no question to answer, so
  // it IS NOT DRAWN.
  //
  // Disabling was the wrong shape. A greyed control still occupies the page,
  // still has to be read, and still invites a click that does nothing — five
  // controls at once say "here are five decisions" when only one can be made.
  // Hiding says "here is the next decision", which is what the cascade means.
  //
  // `cut` is what makes the chain CUMULATIVE. Every control here has a default —
  // `swept`, `spot`, `1min` — so without it a rung would appear merely because
  // its own value happened to be set, three rungs below an unanswered feed.

  /** What a segment tick can actually REACH: ticked AND served by this form. */
  /**
   * IS THIS SEGMENT SERVED **BY THE SELECTED FEED**, which is not a property of
   * the segment.
   *
   * `SEGMENTS[].served` is a static field and it was written for a BROKER: both
   * vendors purge their master on expiry, `core::vendor::decode_master_row`
   * declines every FUT row as `Skip::LiveContract`, and `POST /pull/fno`
   * answers 503 because expired F&O has no HTTP transport. All true, and all
   * true of a BROKER only.
   *
   * AN ARCHIVE IS THE OPPOSITE CASE. TrueData and GDFL are folders of CSVs the
   * operator BOUGHT, and expired futures and expired options are precisely what
   * those files contain — 642 futures CSVs and 11,490 option members measured
   * in one GDFL day. Refusing the segments an archive exists to hold was the
   * static field applied to the wrong kind of feed.
   *
   * `kind === 'folder'` is `SourceKind`'s own word off `/feeds.json`, the same
   * value `kind_label` and `verb` are read from — not a second guess at the
   * transport.
   */
  const isArchiveFeed = $derived(active?.kind === 'folder');
  /**
   * EVERY SEGMENT IS OFFERED ON EVERY FEED — the operator's decision, stated
   * three times and overruling the per-feed rule that stood here.
   *
   * WHAT THAT COSTS, RECORDED RATHER THAN DISCOVERED. On an ARCHIVE this is
   * simply correct: expired futures and options are what the bought CSVs hold.
   * On a BROKER it offers something the vendor cannot answer, and all three
   * reasons remain true — the master purges on expiry so every FUT row decodes
   * as `Skip::LiveContract`, `POST /pull/fno` answers 503 for want of an HTTP
   * transport, and `FnoRequest` names one contract rather than a set.
   *
   * So a broker request for an expired segment WILL be refused. It is refused
   * LOUDLY and by name — §4's requirement is that a failure is named, not that
   * it is made unreachable — and the reason still sits on the row.
   */
  const segServed = (_s) => true;
  const segmentsReached = $derived(SEGMENTS.filter((s) => segSet.has(s.key) && segServed(s)));
  const segmentsUnserved = $derived(SEGMENTS.filter((s) => !segServed(s)));
  /**
   * HOW MANY RUNGS ARE TICKED — and therefore how many requests this form
   * sends. `/pull/spot` carries a single `granularity` field, so the factor
   * here is also the number of POSTs, never a widening of one.
   */
  const rungCount = $derived(rungsChosen.length);

  const ladder = $derived.by(() => {
    const steps = [
      {
        id: 'universe',
        parentReady: Boolean(active) && !archiveHidden,
        why: 'Select a broker feed in the first control of this strip — the universe below is that feed’s answer.'
      },
      {
        id: 'instruments',
        parentReady: Boolean(target),
        why: 'Choose a universe first — the instruments are whichever names it covers.'
      },
      {
        id: 'segment',
        parentReady: reachKnown && reach > 0,
        why: reachKnown
          ? `${feedName(feeds.active)} lists no instrument in this universe, so there is nothing to segment. Choose another universe, or a feed whose master carries these names.`
          : `The instruments in this universe are not counted yet, so nothing below them can be answered: ${reachWhy}`
      },
      {
        id: 'rung',
        parentReady: segmentsReached.length > 0,
        why: 'Tick a segment this form can fill. With none ticked there is no series to choose a bar length for.'
      },
      {
        id: 'window',
        parentReady: rungCount > 0,
        why: 'Choose a bar length first — a window with no rung names no series.'
      }
    ];
    let cut = false;
    return steps.map((s) => {
      const show = !cut && s.parentReady;
      if (!show) cut = true;
      return { id: s.id, why: s.why, show };
    });
  });

  const shows = $derived(new Map(ladder.map((s) => [s.id, s.show])));
  /**
   * THE FIRST UNANSWERED RUNG, and the only control a refusal may name.
   *
   * A refusal that misnames its cause sends the reader to fix something that is
   * not broken: "From has no date" is true and useless when the date field is
   * not on screen because the segment above it was unticked.
   */
  const gate = $derived(ladder.find((s) => !s.show) ?? null);

  /**
   * Instrument-months the request would fill if every name answered in every
   * month file it touches. An ESTIMATE, and the only denominator available: the
   * server states no total.
   *
   * THE MONTH FILE IS THE UNIT `/store.json` COUNTS IN, so this is counted in
   * month files even though the window is chosen in days — a day window that
   * covers three sessions of November still touches November's file, and the
   * store reports that file whole. It over-states a partial month on purpose
   * rather than inventing a per-day denominator no endpoint states, and the
   * line beside it says it is an estimate.
   *
   * EVERY FACTOR IS A REACHABLE COUNT. Reading the universe's nominal size here —
   * or a member list belonging to another feed — is what prints "785
   * instruments" beneath a header that has just said nothing is counted.
   */
  const expectedUnits = $derived(
    reachKnown ? insCount * segmentsReached.length * rungCount * windowMonths.length : 0
  );

  /** The product, with the factors that make it. They multiply, in every state. */
  const productLine = $derived(
    `${reachKnown ? n(insCount) : '—'} instrument(s)` +
      ` × ${n(segmentsReached.length)} segment(s)` +
      ` × ${n(rungCount)} rung(s)` +
      ` × ${n(windowMonths.length)} month file(s)` +
      ` = ${reachKnown ? n(expectedUnits) : '—'}`
  );

  /**
   * THE DIFFERENCE BETWEEN WHAT IS TICKED AND WHAT THE WIRE ASKS FOR, in
   * words, whenever the two are not the same number.
   *
   * The tick list decides what this page counts, measures and reports on. The
   * REQUEST names a target and nothing else — `api::ingest::SpotRequest` has no
   * member field — so a narrowed selection does not narrow the pull. Silence
   * here would be the page implying it sends a list it cannot send.
   */
  const askGap = $derived.by(() => {
    if (!reachKnown || insCount === reach) return null;
    return (
      `${n(insCount)} of ${n(reach)} instrument(s) are ticked, and the count above is theirs. ` +
      `The REQUEST cannot be narrowed to them: api::ingest::SpotRequest carries target, window, feed and ` +
      `granularity and no member field, so target=${target} asks the server for all ${n(reach)} either way. ` +
      `What the ticks change is what this page counts, measures and reports on — the census below, the ` +
      `outcome rows, and this line.`
    );
  });

  /** What the feed reaches, said in one line beside the feed's own name. */
  const scopeNote = $derived.by(() => {
    if (!active) return 'no feed selected — nothing below can be answered';
    if (!reachKnown) return reachWhy;
    if (!rosterFresh) {
      return `carries ${n(reach)} name(s) in this universe · what it does not carry is not measured`;
    }
    const miss = missingHere?.length ?? 0;
    return miss === 0
      ? `carries all ${n(universeWidth ?? reach)} name(s) any feed lists for this universe`
      : `carries ${n(reach)} of ${n(universeWidth ?? reach)} — ${n(miss)} listed only by another feed`;
  });

  // ══════════════════════════════ THE CENSUS ═══════════════════════════════
  //
  // ONE ROW PER SERIES IN THE WINDOW — instrument × segment × timeframe — and
  // every number in it is MEASURED. There was no such surface on this page at
  // all: the form asked for a window and nothing on screen said what the store
  // already held for it, so "is this window done" could only be answered by
  // starting a pull and watching.
  //
  // WHERE EACH NUMBER COMES FROM, and none of them is a guess:
  //
  //   stored     `/store.json?feed=<wire>` — one row per instrument-month-rung,
  //              read once per feed and re-read after every run. A month with
  //              no row is NEVER PULLED, which is not the same as zero bars and
  //              is never printed as one.
  //   expected   the NSE calendar this page already carries, counted over the
  //              days INSIDE the window, times the rung's bars per session.
  //              A rung with no stated bars-per-session gets no expectation at
  //              all — see RUNGS.per.
  //   in flight  `/ingest/status.json` — the sweep's own ladder. It is the only
  //              evidence on this machine that an attempt is being retried, so
  //              `retrying` is drawn only where that route names the row.
  //   halted     the same route's `waiting_on[].halted`, per feed.
  //   reach      the feed's floor at the ticked rungs, which is already what
  //              refuses a day in the date control.

  /**
   * The store, keyed `instrument|timeframe|month` — O(1).
   *
   * # THIS PAGE HELD TWO INDEPENDENTLY-CLOCKED COPIES OF ONE CENSUS
   *
   * `/store.json` was fetched from SIX places across this product and TWO of
   * them were on this page: this one, once per feed, and the run watcher below,
   * every five seconds while a pull was running. Nothing reconciled them, so
   * the census table and the progress bay could state different totals for the
   * same disk at the same moment and both were right about their own snapshot.
   * `/db` was a third opinion on a seventh clock.
   *
   * `$lib/store.svelte.js` reads it ONCE per (feed, generation), builds
   * `byCell` in the same pass that builds every other page's shape, and stamps
   * the answer with the feed it answered for. This object is the four names
   * this page already used, pointed at that one reading — and the run watcher
   * below now reads the SAME one, so the two can no longer disagree.
   *
   * `bars` and `rows` are gated on the feed stamp rather than assumed: a
   * reading that answered for another feed would put one broker's held counts
   * against this broker's window, which is the §4 failure this page's whole
   * census exists to prevent.
   */
  const storeMine = $derived(store.feed !== null && store.feed === (feeds.active ?? ''));
  const storeRead = $derived({
    at: storeMine && store.state === 'ready' ? (store.at ?? 0) : 0,
    feed: store.feed ?? '',
    bars: storeMine ? store.byCell : new Map(),
    rows: storeMine ? store.rows.length : 0,
    // NAMED, NOT SWALLOWED. A census built on a failed read would print
    // "never pulled" on every row, which is a finding this page would have
    // invented out of a network error.
    error: store.error,
    busy: store.state === 'reading'
  });

  /** The sweep's ladder — what is in flight, and whether a feed has halted. */
  let pilot = $state({ at: 0, inFlight: null, feeds: [], state: '', error: null, busy: false });
  let pilotAsked = false;

  async function readPilot() {
    if (pilot.busy) return;
    pilot = { ...pilot, busy: true, error: null };
    try {
      const r = await fetch('/ingest/status.json');
      if (!r.ok) throw new Error(`/ingest/status.json answered HTTP ${r.status}`);
      const j = await r.json();
      pilot = {
        at: Date.now(),
        inFlight: j.in_flight ?? null,
        feeds: Array.isArray(j.waiting_on) ? j.waiting_on : [],
        state: String(j.state ?? ''),
        error: null,
        busy: false
      };
    } catch (why) {
      pilot = { ...pilot, busy: false, error: String(why) };
    }
  }

  /* THE SUBSCRIPTION. One line, and it replaces the fetch, the token, the "have
     I asked for this feed" latch and the retry rule — the shared reading owns
     all four, and a Refresh pressed anywhere (or the poll below) arrives here. */
  $effect(() => syncStore(feeds.active));
  $effect(() => {
    if (pilotAsked) return;
    pilotAsked = true;
    readPilot();
  });

  /** This feed's rung of the sweep ladder, or `null` when it names none. */
  const feedLadder = $derived(
    pilot.feeds.find((f) => f.vendor === feeds.active || f.feed === feeds.active) ?? null
  );
  /** The halt, or `null`. A halt is terminal: attempts have stopped. */
  const feedHalt = $derived(feedLadder && String(feedLadder.halted ?? '').trim() ? String(feedLadder.halted).trim() : null);

  /**
   * SESSIONS PER MONTH INSIDE THE WINDOW — counted once, over the window's own
   * days, and read per series afterwards. Walking the days per row would be
   * 750 × 3,653; this is 3,653 once and a Map probe per month after it.
   */
  const sessionsByMonth = $derived.by(() => {
    const m = new Map();
    if (!windowOk) return m;
    let d = from;
    while (d <= to) {
      if (isSession(d)) {
        const ym = d.slice(0, 7);
        m.set(ym, (m.get(ym) ?? 0) + 1);
      }
      d = addDays(d, 1);
    }
    return m;
  });

  /**
   * THE SEVEN VERDICTS, spelled once: [tone, label, what it means].
   *
   * Read for the chip, for the pill and for the sort ordinal, so a label edited
   * here moves its own rank with it rather than dropping silently to the bottom
   * of a table that still carries the old spelling.
   */
  const VERDICT = {
    fail: [
      'down',
      'failed',
      'The sweep ladder for this feed has HALTED and this series still has month files that are empty or short. Attempts have stopped — the halt reason is on the strip above, and a retry on its own will not help.'
    ],
    never: [
      'warn',
      'never pulled',
      'At least one month file in this window has no entry in /store.json at all. Nothing has asked the vendor for it yet — a pull IS warranted and will very likely settle it.'
    ],
    short: [
      'warn',
      'short',
      'Every month file in this window has been written, and at least one holds fewer bars than the NSE calendar expects for the days inside the window.'
    ],
    retry: [
      'info',
      'retrying',
      '/ingest/status.json names this instrument in flight right now. Nothing is waiting on you; it settles into verified, short or failed on its own.'
    ],
    unknown: [
      'info',
      'no yardstick',
      'This timeframe has no bars-per-session this page can state, so what is stored cannot be compared with the calendar. The bars are counted; the verdict is withheld rather than guessed.'
    ],
    beyond: [
      'info',
      'out of reach',
      'Every month file in this window is older than this feed answers for at this timeframe. No attempt was made and none would help — move the from date, the feed or the timeframe.'
    ],
    ok: [
      'up',
      'verified',
      'Every month file in the window holds at least the bars the NSE calendar expects for the days inside it.'
    ]
  };
  /** Worst first, and the order is a claim about what the reader has to DO. */
  const VORDER = ['fail', 'never', 'short', 'retry', 'unknown', 'beyond', 'ok'];
  const VRANK = new Map(VORDER.map((k, i) => [k, i]));

  /**
   * EVERY SERIES IN THE WINDOW, MEASURED. Ticked instruments × reached
   * segments × ticked rungs, each with its month files resolved against the
   * store, the calendar and the feed's floor.
   */
  const censusRows = $derived.by(() => {
    const out = [];
    if (!reachKnown || !windowOk || windowMonths.length === 0) return out;
    const flight = pilot.inFlight;
    for (const m of ticked) {
      const key = censusKey(m);
      const named =
        flight != null && (flight.instrument === key || flight.instrument === m.key || flight.instrument === m.symbol);
      for (const s of segmentsReached) {
        for (const r of rungsChosen) {
          const months = [];
          let got = 0;
          let exp = 0;
          let unproved = 0;
          let missing = 0;
          const seen = { fail: 0, never: 0, short: 0, retry: 0, unknown: 0, beyond: 0, ok: 0 };
          for (const ym of windowMonths) {
            const sessions = sessionsByMonth.get(ym) ?? 0;
            const expect = r.per === null ? null : sessions * r.per;
            const held = storeRead.bars.get(`${key}|${r.dir}|${ym}`);
            const bars = held === undefined ? null : held;
            // A MONTH THE FEED CANNOT REACH IS NOT A GAP. The floor is the
            // same one the date control refuses a day with.
            const beyond = Boolean(feedFloor.at) && monthLastDay(ym) < feedFloor.at;
            let k;
            if (beyond) k = 'beyond';
            else if (bars === null) k = 'never';
            else if (expect === null) k = 'unknown';
            else if (bars >= expect) k = 'ok';
            else k = 'short';
            got += bars ?? 0;
            exp += expect ?? 0;
            if (k !== 'ok' && k !== 'beyond') unproved += 1;
            if (k === 'never' || k === 'short') missing += 1;
            seen[k] += 1;
            months.push({ month: ym, sessions, exp: expect, got: bars, k });
          }
          // The series verdict, worst first, and every branch has evidence
          // behind it: a halt from the ladder route, a flight from the same
          // route, a missing row from the store, a floor from the feed.
          let state;
          if (seen.beyond === months.length) state = 'beyond';
          else if (named) state = 'retry';
          else if (feedHalt && missing > 0) state = 'fail';
          else if (seen.never > 0) state = 'never';
          else if (seen.short > 0) state = 'short';
          else if (seen.unknown > 0) state = 'unknown';
          else state = 'ok';
          out.push({
            id: `${key}|${s.key}|${r.dir}`,
            // TWO KEYS, AND THEY ARE NOT THE SAME KEY. `key` is the STORE's
            // spelling — EXCHANGE-SEGMENT-SYMBOL, what /store.json files a row
            // under. `mkey` is the MASTER's — what /instruments.json returns and
            // what the tick list, the outcome list and `askedKeys` compare on.
            // Passing one where the other is expected matches nothing and
            // produces an empty answer that looks like a measurement.
            key,
            mkey: m.key,
            sym: m.symbol,
            kind: m.kind ?? '',
            seg: s.key,
            segLabel: s.label,
            tf: r.dir,
            tfLabel: r.label,
            per: r.per,
            months,
            got,
            exp,
            unproved,
            missing,
            state
          });
        }
      }
    }
    return out;
  });

  // ---- the strip: one verdict, the pills that partition it, the one action

  /** Counts by verdict, one pass. `0` for a verdict nothing is in. */
  const censusCounts = $derived.by(() => {
    const t = { fail: 0, never: 0, short: 0, retry: 0, unknown: 0, beyond: 0, ok: 0 };
    for (const r of censusRows) t[r.state] += 1;
    return t;
  });
  /** Non-zero only, worst first. A pill reading "0 failed" is a non-event. */
  const censusPills = $derived(
    VORDER.filter((k) => censusCounts[k] > 0).map((k) => ({ k, count: censusCounts[k] }))
  );
  /** The partition's own sum, shown on screen beside the total it must reach. */
  const censusDrawn = $derived(censusPills.reduce((a, p) => a + p.count, 0));

  /** Month files inside the window that are neither settled nor out of reach. */
  const censusShort = $derived(censusRows.reduce((a, r) => a + r.unproved, 0));
  /** What the store already holds for exactly these series — measured, not asked. */
  const heldInWindow = $derived(
    censusRows.reduce((a, r) => a + r.months.filter((m) => m.got !== null).length, 0)
  );
  const heldBarsInWindow = $derived(censusRows.reduce((a, r) => a + r.got, 0));
  /** Sessions inside the window, from the same calendar every row is judged by. */
  const windowSessions = $derived([...sessionsByMonth.values()].reduce((a, b) => a + b, 0));
  /** The subtitle: the size of the ask, beside the thing it describes. */
  const censusSub = $derived(
    windowOk
      ? `${dayLabel(from)} – ${dayLabel(to)} · ${n(windowDays)} day(s) · ${n(windowSessions)} NSE session(s) · ${n(windowMonths.length)} month file(s) · ${n(censusRows.length)} series`
      : 'no window picked — both ends need a date before a series can be counted'
  );

  /**
   * WHY THE CENSUS IS EMPTY, when it is empty — always a control the reader can
   * reach, in the order the cascade asks for them.
   */
  function censusBlocker() {
    if (gate) return [gate.id === 'universe' ? 'No feed selected' : 'Nothing to census yet', gate.why];
    if (storeRead.error) return ['The store could not be read', `Nothing below is measured, so nothing is drawn rather than drawing zeros: ${storeRead.error}`];
    if (storeRead.at === 0) return ['The store has not been read yet', `Reading /store.json?feed=${feeds.active ?? ''} — every count below is one row of that answer.`];
    if (insCount === 0) return ['No instrument is ticked', `Open the instruments control above and tick at least one of the ${n(insPool.length)} name(s) in ${universeSpec.label}.`];
    if (segmentsReached.length === 0) return ['No segment is ticked', 'Tick Spot — it is the one segment POST /pull/spot fills.'];
    if (rungsChosen.length === 0) return ['No timeframe is ticked', 'Tick at least one bar length. A window with no rung names no series.'];
    if (!windowOk) return ['No window', 'Both ends of the window need a date before anything can be counted against it.'];
    if (windowMonths.length === 0) return ['No month file in that window', `${dayLabel(from)} – ${dayLabel(to)} touches no month file at all.`];
    return ['Nothing matches', 'Clear the verdict filter above to see every row.'];
  }

  /** The one sentence at the head of the table. Five states, five questions. */
  const censusVerdict = $derived.by(() => {
    const t = censusCounts;
    const total = censusRows.length;
    if (total === 0) {
      const b = censusBlocker();
      return { tone: 'warn', title: b[0], why: `${b[1]} Nothing has been asked for, so nothing can be said to be complete.` };
    }
    const open = t.fail + t.never + t.short + t.retry;
    if (t.ok === total) {
      return {
        tone: 'good',
        title: total === 1 ? 'Complete — the one series in this window is verified' : `Complete — all ${n(total)} series in this window are verified`,
        why: `Every month file from ${dayLabel(from)} to ${dayLabel(to)} holds at least the bars the NSE calendar expects. Nothing is outstanding and no pull is warranted.`
      };
    }
    if (!open && t.beyond === total) {
      return {
        tone: 'warn',
        title: 'None of this window is on this feed',
        why: `Not one of the ${n(total)} series is served this far back — ${floorSentence ?? 'the feed states no floor'} Nothing failed and nothing is outstanding, because there was never anything here to ask for.`
      };
    }
    if (!open) {
      return {
        tone: 'good',
        title: 'As complete as it will ever get',
        why: `${n(t.ok)} of the ${n(total)} series are verified and the rest are out of reach or have no yardstick at this rung. Nothing is short, nothing is empty, nothing failed.`
      };
    }
    if (t.fail > 0) {
      return {
        tone: 'bad',
        title: `Not complete — ${n(t.fail)} of ${n(total)} series failed and need you`,
        why: `The sweep ladder for ${feedName(feeds.active)} has halted: ${feedHalt}. A retry on its own will NOT help — fix that cause, then pull.`
      };
    }
    if (t.never > 0) {
      return {
        tone: 'warn',
        title: `Not complete — ${n(t.never)} of ${n(total)} series ${t.never === 1 ? 'has' : 'have'} a month nothing has ever pulled`,
        why: `/store.json has no entry at all for at least one month file of each. A pull IS warranted and will very likely settle it — nothing has tried once.`
      };
    }
    if (t.short > 0) {
      return {
        tone: 'warn',
        title: `Not complete — ${n(t.short)} of ${n(total)} series ${t.short === 1 ? 'is' : 'are'} short of the calendar`,
        why: `Every month file was written and at least one holds fewer bars than the sessions inside ${dayLabel(from)} – ${dayLabel(to)} call for. A pull re-asks for those month files, oldest first.`
      };
    }
    return {
      tone: 'warn',
      title: `${n(t.retry)} of ${n(total)} series ${t.retry === 1 ? 'is' : 'are'} in flight`,
      why: 'The sweep names them right now. Nothing is waiting on you; each settles into verified, short or failed on its own.'
    };
  });

  // ---- sort, filter, page

  let cSort = $state({ key: 'sym', dir: 1 });
  let cBucket = $state(null);
  let cPage = $state(1);

  function sortCensus(key) {
    if (cSort.key === key) cSort = { key, dir: -cSort.dir };
    // A count sorts biggest-first on the first press; a name sorts A→Z.
    else cSort = { key, dir: key === 'sym' ? 1 : -1 };
    cPage = 1;
  }

  const censusSorted = $derived.by(() => {
    const rows = cBucket ? censusRows.filter((r) => r.state === cBucket) : [...censusRows];
    const d = cSort.dir;
    const k = cSort.key;
    rows.sort((a, b) => {
      // EVERY COMPARATOR IS THREE-WAY and every one falls through to the same
      // tie-break, so the order is total and a redraw cannot reshuffle equals.
      let first = 0;
      if (k === 'sym') first = cmpStr(a.sym, b.sym);
      else if (k === 'seg') first = cmpStr(a.segLabel, b.segLabel);
      else if (k === 'tf') first = cmpStr(a.tfLabel, b.tfLabel);
      else if (k === 'got') first = cmpNum(a.got, b.got);
      else if (k === 'unproved') first = cmpNum(a.unproved, b.unproved);
      else if (k === 'state') first = cmpNum(VRANK.get(a.state) ?? VORDER.length, VRANK.get(b.state) ?? VORDER.length);
      // 'Next step' ranks by the same number the button prints, which is the
      // count of unsettled month files — not by the button's own label.
      else if (k === 'act') first = cmpNum(a.unproved, b.unproved);
      return first * d || cmpStr(a.sym, b.sym) || cmpStr(a.tf, b.tf) || cmpStr(a.seg, b.seg);
    });
    return rows;
  });

  /**
   * HOW MANY COLUMNS ARE DRAWN. Segment and Timeframe appear only when more
   * than one is ticked — a column of one repeated value is a sort control over
   * nothing. The empty row's `colspan` is counted from the same rule, so it can
   * never span the wrong number.
   */
  const censusColCount = $derived(
    5 + (segmentsReached.length > 1 ? 1 : 0) + (rungsChosen.length > 1 ? 1 : 0)
  );

  const censusPages = $derived(Math.max(1, Math.ceil(censusSorted.length / PAGE_SIZE)));
  /** The page never survives past the end of a list that just got shorter. */
  const censusPage = $derived(Math.min(Math.max(1, cPage), censusPages));
  const censusSlice = $derived(censusSorted.slice((censusPage - 1) * PAGE_SIZE, censusPage * PAGE_SIZE));

  /** 1 … 7 pages in full; beyond that the ends, the middle and a gap. */
  function pageList(cur, total) {
    if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);
    const s = new Set([1, total, cur, cur - 1, cur + 1]);
    if (cur <= 3) [2, 3, 4].forEach((x) => s.add(x));
    if (cur >= total - 2) [total - 1, total - 2, total - 3].forEach((x) => s.add(x));
    const l = [...s].filter((p) => p >= 1 && p <= total).sort((a, b) => cmpNum(a, b));
    const out = [];
    l.forEach((p, i) => {
      if (i && p - l[i - 1] > 1) out.push('gap' + p);
      out.push(p);
    });
    return out;
  }

  /**
   * THE DAYS ONE ROW IS SHORT, clipped to the window the operator chose.
   *
   * MONTHS ARE WHAT THE STORE FILES; DAYS ARE WHAT THE WIRE CARRIES. The row
   * knows which month files are unsettled, and this turns that into the day
   * range `parse_spot` reads — never wider than the chosen window, and never
   * older than the first unsettled month.
   */
  function shortSpan(row) {
    const open = row.months.filter((x) => x.k !== 'ok' && x.k !== 'beyond');
    if (open.length === 0) return null;
    const firstDay = `${open[0].month}-01`;
    const lastDay = monthLastDay(open[open.length - 1].month);
    return { from: firstDay > from ? firstDay : from, to: lastDay < to ? lastDay : to };
  }

  /**
   * PULL ONE ROW. The same wire shape the main submit sends — `target`,
   * `vendor`, `from`, `to`, `granularity`, the two days as ISO — narrowed to
   * the DAYS this row is short and to the row's own rung.
   *
   * IT CANNOT BE NARROWED TO THE INSTRUMENT AND THE BUTTON SAYS SO. There is no
   * member field on `api::ingest::SpotRequest`, so the server resolves
   * `target=` and answers for the whole set. What this button genuinely
   * narrows is the WINDOW and the RUNG, which is real work saved, and the row
   * is what the outcome list is then built for.
   */
  async function pullRow(row) {
    if (phase === 'running') return;
    const span = shortSpan(row);
    if (!span) return;
    await runPull(
      [{ dir: row.tf, label: row.tfLabel, body: wireBodyFor(row.tf, span.from, span.to) }],
      // THE MASTER'S KEY, because that is what `classifyBuild` compares on.
      new Set([row.mkey])
    );
  }

  // -------------------------------------------------------------- the run
  //
  // `phase` is the whole state machine: idle → running → done. Nothing else
  // decides what the right-hand column shows.

  let phase = $state('idle');
  let startedAt = $state(0);
  let finishedAt = $state(0);
  let receipt = $state(null);
  /**
   * ONE RECEIPT PER REQUEST, because a run is now one request PER TICKED RUNG.
   * `receipt` stays the one the card renders — the first that came back bad, or
   * the last — and this is the list, so a run that answered three times is not
   * reported as if it answered once.
   */
  let receipts = $state([]);
  /** How far through the requests this run is. Counted, never estimated. */
  let sent = $state({ done: 0, of: 0, label: '' });
  /**
   * WHICH KEYS THIS RUN WAS ASKED FOR. The outcome list is built for these and
   * no others: a row-level pull that reported on all 750 names would be
   * reporting a result about instruments nobody asked for in that press.
   */
  let askedKeys = $state(new Set());
  let netError = $state(null);
  let aborted = $state(false);
  let controller = null;

  /** Where the store stood the moment the request left, and where it stands now. */
  let baseline = $state(null);
  let live = $state(null);
  let pollError = $state(null);
  let lastGrowthAt = $state(0);
  /** {t, units, rows} samples, newest last. Bounded — only the tail is kept. */
  let samples = $state([]);

  const elapsedMs = $derived(
    phase === 'running' ? nowMs - startedAt : finishedAt > startedAt ? finishedAt - startedAt : 0
  );

  const unitsDone = $derived(
    baseline && live ? Math.max(0, live.units - baseline.units) : 0
  );
  const rowsGained = $derived(
    baseline && live ? Math.max(0, live.rows - baseline.rows) : 0
  );
  const unitsLeft = $derived(Math.max(0, expectedUnits - (baseline?.units ?? 0) - unitsDone));
  const share = $derived(
    expectedUnits > 0 ? Math.min(1, (baseline ? baseline.units + unitsDone : 0) / expectedUnits) : 0
  );

  /**
   * Instrument-months per second, from the OBSERVED samples only. `null` until
   * two samples exist that actually differ — a rate computed from one point is
   * a number with no measurement behind it.
   */
  const rate = $derived.by(() => {
    if (samples.length < 2) return null;
    const first = samples[0];
    const last = samples[samples.length - 1];
    const secs = (last.t - first.t) / 1000;
    const gained = last.units - first.units;
    if (secs < 5 || gained <= 0) return null;
    return gained / secs;
  });

  const etaSecs = $derived(rate && unitsLeft > 0 ? Math.round(unitsLeft / rate) : null);

  /**
   * THE GOVERNOR IS NOT VISIBLE FROM HERE, and this is the closest honest thing
   * to seeing it. `crates/pull/src/rate.rs` runs an AIMD governor inside the
   * synchronous POST; nothing reports its state. What CAN be observed is that
   * the store has stopped growing while the request is still open — which is
   * what a waiting governor looks like from outside, and also what a slow
   * vendor looks like. It is named as an inference, not as a reading.
   */
  const stalledSecs = $derived(
    phase === 'running' && lastGrowthAt > 0 ? Math.round((nowMs - lastGrowthAt) / 1000) : 0
  );
  const stalled = $derived(phase === 'running' && stalledSecs >= 20);
  /**
   * WHETHER ANYTHING HAS ACTUALLY LANDED. Without this the panel read "Bars are
   * landing" three seconds into a run that had written nothing — a claim with
   * no measurement under it, which is the exact failure the rest of this page
   * is built to avoid.
   */
  const everGrew = $derived(rowsGained > 0 || unitsDone > 0);

  // ------------------------------------------------------------ store census

  /**
   * One reading of the store, narrowed to the window and reduced to two numbers
   * and a map. The array is ~266 KB at 3,353 rows today and grows toward the
   * ~93,776 the store is heading for, so nothing here holds on to it.
   *
   * IT IS NO LONGER A SEVENTH FETCH. `refreshStore` bumps the shared generation
   * and returns the ONE request that bump causes — the same answer the census
   * table thirty lines up is reading, so a run's baseline and the "held" column
   * beside it can no longer come from two different snapshots of one disk.
   * `foldMonths` walks only the months the window names, off the shared
   * `byMonth` index, and memoises on (read, months) so a re-render never
   * re-folds.
   */
  /**
   * THE ASK'S OWN POPULATION — the census keys and the rungs this request
   * reaches, and nothing else.
   *
   * # The 100% meter this removes
   *
   * `snapshot()` counted EVERY row in the window months — every instrument at
   * every rung — while `expectedUnits` counted only this request's reach
   * (instruments × segments × rungs × months). With anything else already held
   * in those months (the 750-name NIFTY Total Market backfill is the ordinary
   * case) the numerator was in the hundreds against a denominator of 2,
   * `Math.min(1, …)` pinned the bar at 100.0%, and `unitsLeft` clamped to 0 —
   * which silently removed the ETA line as well — with half the request still
   * on the wire. It took two pre-existing rows, not 750.
   *
   * Both readings are taken through this, so the numerator and the denominator
   * are over one population. What lands outside it is not discarded: the fold
   * counts it as `outside` and the card names it.
   */
  const askScope = $derived({
    instruments: new Set(ticked.map(censusKey)),
    rungs: new Set(rungsChosen.map((r) => r.dir))
  });

  async function snapshot() {
    await refreshStore();
    // REFUSED, NOT ZEROED. Every outcome on this page is a difference against
    // this reading; measuring against a failed one would credit the run with
    // whatever the store already held.
    if (store.state !== 'ready')
      throw new Error(store.error ?? '/store.json could not be read, and it named no reason');
    return foldMonths(windowMonths, askScope);
  }

  /**
   * The watcher. It is no longer a loop this page owns: `watchStore` is ONE
   * timer for the whole product, held at the finest period any page asked for
   * and released on cleanup, and it bumps the shared generation rather than
   * fetching — so the five-second reading a run is measured against IS the
   * reading every other surface is showing.
   *
   * The awaited poll still cannot stack requests on top of each other; that
   * rule moved into the shared clock with the timer.
   */
  let releaseWatch = null;
  /** The last landed read this page has already folded into `live`. */
  let seenRead = 0;

  $effect(() => {
    const reads = store.reads;
    const state = store.state;
    const why = store.error;
    untrack(() => {
      if (phase !== 'running') return;
      if (state === 'error') {
        // NAMED, NOT SWALLOWED. A watcher that quietly stops is a progress
        // display that silently freezes, which is the failure this page exists
        // to remove.
        pollError = why ?? '/store.json could not be read, and it named no reason';
        return;
      }
      if (state !== 'ready' || reads === seenRead) return;
      seenRead = reads;
      const shot = foldMonths(windowMonths, askScope);
      pollError = null;
      if (!live || shot.units > live.units || shot.rows > live.rows) lastGrowthAt = shot.at;
      live = shot;
      samples = [...samples, { t: shot.at, units: shot.units, rows: shot.rows }].slice(-12);
    });
  });

  // ------------------------------------------------------------- the receipt

  /**
   * The server's own answer, read rather than reconstructed.
   *
   * `DOMParser` on `text/html` builds an INERT document: no browsing context,
   * so no script runs and no resource is fetched. Every value below comes out
   * as `textContent`, so a symbol carrying `&` — M&M, M&MFIN are real
   * instruments — is a string and never markup. Nothing on this page is ever
   * assigned through `innerHTML`.
   */
  function readReceipt(html, ok, status, ctype, marker) {
    // A PAGE IS NOT A RECEIPT. See `$lib/receipt.js`: the content-type is
    // tested before anything is believed, and the parsed document must carry
    // the verdict element every answer this route gives carries.
    const wrong = notAReceipt({ html, status, ctype, marker, hasVerdict: true });
    if (wrong) return wrong;
    const doc = new DOMParser().parseFromString(html, 'text/html');
    const facts = [];
    for (const tr of doc.querySelectorAll('table.kv tr')) {
      const k = tr.querySelector('th')?.textContent?.trim() ?? '';
      const v = tr.querySelector('td')?.textContent?.trim() ?? '';
      if (k) facts.push({ k, v });
    }
    const badge = doc.querySelector('.badge');
    // THE SECOND TEST. `wrong` above cleared the content-type; this one is the
    // document's own verdict element, and its absence is the answer that used
    // to render as a green OK.
    const noVerdict = notAReceipt({ html, status, ctype, marker, hasVerdict: Boolean(badge) });
    if (noVerdict) return noVerdict;
    const halt = doc.querySelector('.halt');
    const scope = halt?.querySelector('b')?.textContent?.trim() ?? '';
    let reason = halt?.textContent?.trim() ?? '';
    if (scope && reason.startsWith(scope)) reason = reason.slice(scope.length).trim();
    return {
      ok,
      status,
      verdict: badge?.textContent?.trim() || (ok ? 'OK' : `HTTP ${status}`),
      good: badge?.classList.contains('good') === true,
      scope,
      reason,
      facts,
      raw: html
    };
  }

  const factValue = $derived.by(() => {
    const m = new Map();
    for (const f of receipt?.facts ?? []) if (!m.has(f.k)) m.set(f.k, f.v);
    return m;
  });

  /** Every `Failed` row the receipt carried — the server sends at most five. */
  const namedFailures = $derived((receipt?.facts ?? []).filter((f) => f.k === 'Failed'));
  const failedCount = $derived.by(() => {
    const raw = factValue.get('Members failed');
    const parsed = Number(String(raw ?? '').replace(/[^0-9]/g, ''));
    return Number.isFinite(parsed) ? parsed : 0;
  });
  /**
   * THE TRUNCATION, STATED. `landed_answer` writes
   * `for f in done.failures.iter().take(5)`, so a run with 407 failures puts
   * five on the wire and the count beside them. The page can see the gap and
   * says so rather than presenting five as the whole story.
   */
  const hiddenFailures = $derived(Math.max(0, failedCount - namedFailures.length));

  // -------------------------------------------------------------- outcomes
  //
  // ONE ROW PER INSTRUMENT THE REQUEST NAMED, whatever happened to it. The
  // reasons come from the server's own words where it gave them and from the
  // measured census where it did not; nothing here is a guess dressed as a
  // verdict.

  /**
   * The six outcomes, spelled ONCE. Read for the label and read for the order,
   * so the two cannot disagree — a label edited here moves its own rank with it
   * rather than dropping silently to the bottom of a table that still carries
   * the old spelling.
   */
  const GROUP = {
    failed: 'Failed',
    refused: 'Refused',
    silent: 'No bars landed',
    outside: 'Outside the universe',
    held: 'Already held',
    stored: 'Stored'
  };

  /**
   * THE OUTCOME ORDER IS AN ORDINAL, NOT THE ALPHABET.
   *
   * Sorting by `group.localeCompare` ranked these six by the first letter of a
   * sentence written for a reader: `Already held` first, `Stored` before them
   * all if it had been spelled `Ok`, and `Failed` buried in the middle. That is
   * not an order any outcome has — it is an accident of English, and it changes
   * the moment a label is reworded, which is a silent reordering nobody would
   * connect to a copy edit.
   *
   * Most actionable first, and each rank is a claim about what the reader has
   * to DO:
   *
   *   0 Failed            the run named a reason, one per instrument — each is
   *                       its own fix, and they are the only rows carrying the
   *                       server's own words.
   *   1 Refused           one cause standing in front of the whole set. Fewer
   *                       rows to read than `Failed`, but fixing it unblocks
   *                       every one of them at once.
   *   2 No bars landed    nothing landed and NOBODY SAID WHY. Nothing to fix
   *                       yet, so it ranks under the two that name a cause —
   *                       but it has to be investigated, which puts it above
   *                       everything that needs no attention at all.
   *   3 Outside the universe  something wrote what this request did not ask for.
   *                       Not this run's failure; still evidence of a second
   *                       writer, and worth seeing before the rows that worked.
   *   4 Already held      the store had it and gained nothing. Correct, but not
   *                       a thing THIS run achieved, so it sits above the rows
   *                       that did.
   *   5 Stored            it worked. Nothing to do, so it sorts last.
   */
  const GROUP_ORDER = [
    GROUP.failed,
    GROUP.refused,
    GROUP.silent,
    GROUP.outside,
    GROUP.held,
    GROUP.stored
  ];
  const GROUP_RANK = new Map(GROUP_ORDER.map((label, i) => [label, i]));
  /**
   * An unranked label sorts AFTER every ranked one, never silently first. A
   * group this table has never heard of is a group that was added without a
   * decision about where it belongs, and burying it at the top would be the
   * fallback that hides the omission.
   */
  function groupRank(label) {
    return GROUP_RANK.get(label) ?? GROUP_ORDER.length;
  }

  let outcomes = $state([]);
  let outcomeIndex = new Map();

  function classifyBuild() {
    if (!baseline) {
      outcomes = [];
      outcomeIndex = new Map();
      return;
    }
    const after = live ?? baseline;
    const failedBySymbol = new Map();
    for (const f of namedFailures) {
      // `landed_answer` writes `{instrument} — {why}`.
      const split = f.v.split(' — ');
      const who = (split.shift() ?? '').trim();
      const why = split.join(' — ').trim() || f.v;
      if (who) failedBySymbol.set(who.toUpperCase(), why);
    }
    /**
     * A WHOLESALE REFUSAL AND A RUN THAT FAILED PART-WAY ARE NOT THE SAME
     * THING, and stamping the run's verdict onto every member conflated them:
     * a run that read 785 members and failed 407 came back with 745 rows all
     * labelled "Refused", quoting a sentence that was never about them.
     *
     * `Members read` is the discriminator. A refusal that never reached the
     * members — the parser's, or the broker path's before the socket — emits no
     * such fact, and only then does the run's own reason belong on every row.
     */
    const refused = factValue.get('Refused') ?? null;
    const reachedMembers = factValue.has('Members read');
    const runReason = reachedMembers
      ? refused
      : (refused ?? (receipt && !receipt.good ? receipt.reason : null));

    const rows = [];
    const seen = new Set();
    for (const m of members) {
      // ONLY WHAT THIS PRESS ASKED FOR. A row-level pull names one instrument
      // and the form names the ticked ones; reporting on the rest would print
      // "not accounted for" against names nobody asked about in this run.
      if (askedKeys.size > 0 && !askedKeys.has(m.key)) continue;
      const key = censusKey(m);
      seen.add(key);
      const before = baseline.byInstrument.get(key) ?? 0;
      const now = after.byInstrument.get(key) ?? 0;
      const gained = now - before;
      const named =
        failedBySymbol.get(m.symbol.toUpperCase()) ??
        failedBySymbol.get(m.key.toUpperCase()) ??
        failedBySymbol.get(key.toUpperCase());
      let group;
      let reason;
      let tone;
      if (named) {
        group = GROUP.failed;
        reason = named;
        tone = 'down';
      } else if (gained > 0) {
        group = GROUP.stored;
        reason = `${n(gained)} bar(s) landed across ${windowMonths.length} month(s) of the window`;
        tone = 'up';
      } else if (runReason) {
        group = GROUP.refused;
        reason = runReason;
        tone = 'down';
      } else if (before > 0) {
        group = GROUP.held;
        reason = `The store already carried ${n(before)} bar(s) for this window and gained none — nothing new was written and the run named no failure for it`;
        tone = 'info';
      } else {
        group = GROUP.silent;
        reason =
          hiddenFailures > 0
            ? `The store gained nothing for this instrument and no reason was sent for it. The run recorded ${n(failedCount)} failed member(s) and put only ${n(namedFailures.length)} reason(s) on the wire, so this may be one of the ${n(hiddenFailures)} whose reason the server truncated.`
            : 'The run reported no failure for this instrument and the store gained nothing for it. It was named by the universe and is not accounted for.';
        tone = 'warn';
      }
      rows.push({ key, symbol: m.symbol, kind: m.kind, gained, before, group, reason, tone });
    }

    // Anything that GREW but was not in the target set. Surprising, and worth
    // seeing: it means something wrote outside the request.
    for (const [key, now] of after.byInstrument) {
      if (seen.has(key)) continue;
      const before = baseline.byInstrument.get(key) ?? 0;
      if (now - before <= 0) continue;
      rows.push({
        key,
        symbol: key.split('-').slice(2).join('-') || key,
        kind: key.split('-')[1] ?? '',
        gained: now - before,
        before,
        group: GROUP.outside,
        reason: `${n(now - before)} bar(s) landed for an instrument this request did not name. Another run may be writing at the same time.`,
        tone: 'info'
      });
    }

    outcomes = rows;
    // O(1) PER KEYSTROKE, the same prefix index `$lib/index.svelte.js` uses.
    // Built once per run — O(n) and paid a single time — so the search box is a
    // Map probe rather than a scan of 750.
    const next = new Map();
    for (const row of rows) {
      const s = row.symbol.toUpperCase();
      for (let i = 1; i <= Math.min(MAX_PREFIX, s.length); i += 1) {
        const k = s.slice(0, i);
        let bucket = next.get(k);
        if (!bucket) next.set(k, (bucket = []));
        bucket.push(row);
      }
    }
    outcomeIndex = next;
  }

  /** Reasons, grouped and counted, most common first. */
  const groups = $derived.by(() => {
    const m = new Map();
    for (const row of outcomes) {
      let g = m.get(row.group);
      if (!g) m.set(row.group, (g = { group: row.group, tone: row.tone, count: 0, reasons: new Map() }));
      g.count += 1;
      g.reasons.set(row.reason, (g.reasons.get(row.reason) ?? 0) + 1);
    }
    return [...m.values()].sort((a, b) => b.count - a.count);
  });

  // ------------------------------------------------------- the verdict strip
  //
  // ═════════════ DID THE WINDOW HE ASKED FOR LAND? ═════════════
  //
  // One sentence at the head of the table, the non-zero counts beside it as
  // filters, and the one bulk action. A reader who stops at the strip has the
  // answer he came for; the pills are the exceptions to it and the rows are the
  // evidence.
  //
  // IT RECOMPUTES NOTHING. Every number here is read off `groups`, which is one
  // pass over `outcomes` — the same rows the table draws and the same
  // `row.group` its Verdict column renders. There is no second classifier and
  // no second definition of "landed", so the strip and the column cannot
  // disagree about a single instrument. The ONLY thing added is arithmetic on
  // counts that already exist.
  //
  // WORST FIRST, because the row reads left to right and the bucket that needs
  // him must not be last. `GROUP_ORDER` already states that order for the
  // table's sort; the pills read the same list rather than a second one.
  const pills = $derived(
    [...groups].sort((a, b) => groupRank(a.group) - groupRank(b.group))
  );
  /** Counts by group, off the same pass. `0` for a group no row is in. */
  const countOf = $derived(new Map(groups.map((g) => [g.group, g.count])));
  const countIn = (label) => countOf.get(label) ?? 0;
  /**
   * The instruments this window did not land for: named failures, wholesale
   * refusals, and the ones nothing accounted for. `Already held` is not short —
   * the store has it — and `Outside the universe` was never asked for.
   */
  const shortCount = $derived(
    countIn(GROUP.failed) + countIn(GROUP.refused) + countIn(GROUP.silent)
  );
  /**
   * THE ONE VERDICT, in the four states that answer four different questions.
   * Collapsing any two would answer a question nobody asked: "as complete as it
   * gets" is a SUCCESS and looks identical to a failure in any count that does
   * not separate it.
   */
  const landed = $derived.by(() => {
    const total = outcomes.length;
    if (total === 0) return null;
    const failed = countIn(GROUP.failed);
    const refused = countIn(GROUP.refused);
    const silent = countIn(GROUP.silent);
    const held = countIn(GROUP.held);
    const stored = countIn(GROUP.stored);
    const outside = countIn(GROUP.outside);
    const span = `${dayLabel(from)} – ${dayLabel(to)}`;
    const aside =
      outside > 0
        ? ` ${n(outside)} instrument(s) outside this request also gained bars — something else is writing.`
        : '';
    if (failed > 0) {
      return {
        tone: 'bad',
        title: `Not complete — ${n(failed)} of ${n(total)} instrument(s) failed and need you`,
        why: `The run named a reason for each of them and they are the only rows carrying the server's own words. A second pull on its own will not help: fix the cause on the row, then ask for ${span} again.${aside}`
      };
    }
    if (refused > 0) {
      return {
        tone: 'bad',
        title: `Not complete — the run refused ${n(refused)} of ${n(total)} instrument(s)`,
        why: `One cause is standing in front of the whole set, so there are fewer rows to read than there are instruments — and fixing it unblocks every one of them at once.${aside}`
      };
    }
    if (silent > 0) {
      return {
        tone: 'warn',
        title: `Not accounted for — ${n(silent)} of ${n(total)} instrument(s) gained nothing and named no reason`,
        why: `Nothing landed for them and nobody said why, so there is nothing to fix yet — there is something to investigate. ${hiddenFailures > 0 ? `The server truncated ${n(hiddenFailures)} reason(s), so some of these may have had one.` : 'The run reported no failure for any of them.'}${aside}`
      };
    }
    if (stored === 0) {
      return {
        tone: 'warn',
        title: `Nothing was written — the store already carried all ${n(total)}`,
        why: `Every instrument this request named already held bars for ${span} and gained none. Nothing failed and nothing is outstanding: asking again returns the same nothing.${aside}`
      };
    }
    return {
      tone: 'good',
      title:
        total === 1
          ? 'The window landed — the one instrument this request named is accounted for'
          : `The window landed — all ${n(total)} instruments this request named are accounted for`,
      why: `${n(stored)} gained bars across ${span}${held > 0 ? ` and ${n(held)} already held them` : ''}. Nothing failed, nothing was refused and nothing is unexplained.${aside}`
    };
  });
  /**
   * WHY THE ONE BULK ACTION IS DISABLED, or `null` when it can be pressed. One
   * source for the `disabled` attribute AND the sentence beside it, so a greyed
   * button whose stated reason has gone stale is not possible.
   */
  const rerunBlock = $derived.by(() => {
    if (phase === 'running') return 'a pull is already on the wire — this is one synchronous POST and no route amends it.';
    if (problems.length > 0) {
      return `the request above has ${n(problems.length)} thing(s) to fix first: ${problems[0].why}`;
    }
    return null;
  });

  let q = $state('');
  let pickedGroup = $state(null);
  let sortKey = $state('group');
  let sortDir = $state(1);

  const filtered = $derived.by(() => {
    const typed = q.trim().toUpperCase();
    // ONE PROBE for 1..4 characters; a filter over one bucket beyond that. The
    // group narrowing is applied to whichever set is already smaller, so the
    // cost is bounded by the bucket and never by the universe.
    let base;
    if (!typed) base = outcomes;
    else if (typed.length <= MAX_PREFIX) base = outcomeIndex.get(typed) ?? [];
    else base = (outcomeIndex.get(typed.slice(0, MAX_PREFIX)) ?? []).filter((r) => r.symbol.toUpperCase().startsWith(typed));
    const narrowed = pickedGroup ? base.filter((r) => r.group === pickedGroup) : base;
    const dir = sortDir;
    const sorted = [...narrowed];
    sorted.sort((a, b) => {
      if (sortKey === 'gained') return (a.gained - b.gained) * dir;
      if (sortKey === 'symbol') return a.symbol.localeCompare(b.symbol) * dir;
      // THE ORDINAL, NOT THE PROSE. `GROUP_ORDER` states what "first" means
      // here; the label is what the row shows and nothing else. Symbol breaks
      // ties inside a rank so the list is stable between renders.
      return (groupRank(a.group) - groupRank(b.group) || a.symbol.localeCompare(b.symbol)) * dir;
    });
    return sorted;
  });

  function sortBy(key) {
    if (sortKey === key) sortDir = -sortDir;
    else {
      sortKey = key;
      sortDir = key === 'gained' ? -1 : 1;
    }
  }

  // --------------------------------------------------------- virtualisation
  //
  // ONLY THE VISIBLE ROWS ARE IN THE DOM. 750 today; the store is heading for
  // ~93,776 census rows and a target list that grows with it. A spacer carries
  // the full height so the scrollbar is honest.

  let vEl = $state(null);
  let vTop = $state(0);
  let vH = $state(420);
  const vFirst = $derived(Math.max(0, Math.floor(vTop / ROW_H) - OVERSCAN));
  const vCount = $derived(Math.min(filtered.length - vFirst, Math.ceil(vH / ROW_H) + OVERSCAN * 2));
  const vSlice = $derived(filtered.slice(vFirst, vFirst + Math.max(0, vCount)));

  function onScroll() {
    if (vEl) vTop = vEl.scrollTop;
  }

  $effect(() => {
    if (!vEl) return;
    // CAPTURED, NOT RE-READ. `vEl` is `$state` and the observer callback fires
    // LATER — after a route change or a conditional block closing, the binding
    // can be null by the time it runs, and `vEl.clientHeight` inside the
    // callback would throw on a page the operator has already left. The early
    // return above proves nothing about a callback that outlives it.
    //
    // Holding the element the observer was attached to also makes the two agree
    // by construction: it can only ever measure the node it observes.
    const el = vEl;
    const ro = new ResizeObserver(() => {
      vH = el.clientHeight;
    });
    ro.observe(el);
    vH = el.clientHeight;
    return () => ro.disconnect();
  });

  // Re-filtering must not leave the viewport scrolled past the end.
  $effect(() => {
    const max = Math.max(0, filtered.length * ROW_H - vH);
    if (vTop > max && vEl) {
      vEl.scrollTop = max;
      vTop = max;
    }
  });

  // ------------------------------------------------------------ the calendar
  //
  // ONE PANEL AT A TIME. `cal.field` is the latch — a single value, so two
  // panels cannot coexist to overlap each other.
  //
  // IT PICKS DAYS, BECAUSE THE WINDOW IS DAYS. A day grid is the honest shape
  // for a control whose value is a day: every day of the month visible, one
  // click each, the ones no bar can exist for drawn faint, and the ones a bound
  // refuses struck through with the reason on them — never a click that
  // silently does nothing.
  //
  // TWO SELECTS IN THE HEADER, NOT JUST ARROWS. Paging one month at a time is
  // how a reader ends up thirty clicks from where they meant to be; a month
  // select and a year select reach any day in the eleven-year span in two
  // picks, and the span is a data question rather than a layout one.
  //
  // Every mechanism the month grid carried is still here: roving focus, the
  // orphan reclaim, clamping the cursor to what can be committed, opening
  // upward when there is no room below, retracting with its rung, and the whole
  // keyboard.

  let cal = $state({ field: null, cursor: '', view: '', up: false });
  let calEl = $state(null);

  /** Tall enough for the header, six week rows and the footer. */
  const CAL_H = 340;
  /** The day grid is seven across, so Up/Down is a week and Left/Right a day. */
  const CAL_COLS = 7;
  /** NSE weeks read Monday-first. */
  const WEEK = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

  /** Which month the open panel is looking at — `YYYY-MM`, the view, not the value. */
  const calView = $derived(cal.view || maxMonth);
  const calYear = $derived(Number(calView.slice(0, 4)));
  const calMonthNo = $derived(Number(calView.slice(5)));
  const calYears = $derived.by(() => {
    const out = [];
    for (let y = FLOOR_YEAR; y <= parts(maxDay).y; y += 1) out.push(y);
    return out;
  });

  /**
   * The forty-two cells of the shown month, Monday-first, as ISO days. Six full
   * weeks always, so the panel does not change height as the reader pages
   * through it and the grid never reflows under the pointer.
   */
  const calDays = $derived.by(() => {
    const first = `${calView}-01`;
    // 0 = Sunday from the platform; the grid reads Monday-first.
    const lead = (weekdayOf(first) + 6) % 7;
    const out = [];
    for (let i = 0; i < 42; i += 1) out.push(addDays(first, i - lead));
    return out;
  });
  /**
   * Whether the month on screen holds ANY day this field can take. A month the
   * reader can reach but cannot pick from says so ONCE, in the footer, rather
   * than in forty-two identical tooltips he would have to hover to discover
   * that his click was wasted.
   */
  const nothingInView = $derived(
    calDays.every((d) => d.slice(0, 7) !== calView || dayBlock(d) !== null)
  );

  function openCal(field) {
    if (cal.field === field) {
      cal = { ...cal, field: null };
      return;
    }
    const current = field === 'from' ? fromDay : toDay;
    const seed = isValidIso(current) ? current : maxDay;
    // OPENS UPWARD WHEN THERE IS NO ROOM BELOW. A picker that drops off the
    // bottom of a scroll container is a picker the operator has to hunt for,
    // and this form sits in one.
    const box = document.getElementById(`cal-${field}`)?.getBoundingClientRect();
    const up = box ? box.bottom + CAL_H > window.innerHeight && box.top > CAL_H : false;
    // IT OPENS ON THE DAY THE FIELD HOLDS, EVEN WHEN THAT DAY IS NOW REFUSED.
    // The view follows the value and the cursor follows what can be committed,
    // which are two different questions: a `from` that fell below a floor when
    // the feed changed is exactly the day the reader is looking for, and
    // jumping him to the floor's month would hide the struck-through cell that
    // explains what happened.
    cal = { field, cursor: clampDay(seed), view: clampView(seed.slice(0, 7)), up };
    // FOCUS HAS TO ENTER THE PANEL, ONCE. The keydown handler is on the panel,
    // so with focus left on the trigger every arrow key went to the page
    // instead: the picker opened and then ignored the keyboard entirely.
    enterOnOpen = true;
  }
  function shutCal(refocus = true) {
    const field = cal.field;
    cal = { ...cal, field: null };
    if (refocus && field) document.getElementById(`cal-${field}`)?.focus();
  }
  /**
   * THE ONE WRITER OF A DAY, and it writes BOTH halves.
   *
   * The text field and the ISO value are two views of one day, so nothing may
   * set one without the other — a label left behind by a commit is a field
   * reading `02 Sep 2024` over a window that starts in March, and the screen
   * would look right the whole time.
   */
  function setDay(field, isoDay) {
    if (field === 'from') {
      fromDay = isoDay;
      fromText = isoDay ? dayLabel(isoDay) : '';
    } else {
      toDay = isoDay;
      toText = isoDay ? dayLabel(isoDay) : '';
    }
    // A VALUE THAT TOOK CLEARS THE COMPLAINT ABOUT THE ONE THAT DID NOT.
    typedErr = { ...typedErr, [field]: null };
  }
  function commit(isoDay) {
    if (dayBlock(isoDay)) return;
    setDay(cal.field, isoDay);
    showProblems = true;
    shutCal();
  }
  /**
   * WHY A DAY CANNOT BE PICKED HERE, or `null` when it can.
   *
   * Forty-two buttons go grey with no word between them, and the bounds that
   * produced them are stated nowhere the reader is looking — the panel covers
   * the paragraph below it. This is the sentence, attached to the day it is
   * about, and it is the SAME rule `problems` states: both read `maxDay`,
   * `feedFloor` and `minDay` rather than restating a number that could drift.
   *
   * THE CEILING IS NAMED FIRST because it is the one bound that is the same for
   * every feed, and naming a vendor for a day that has not happened yet would
   * be absurd. The feed's floor comes before the page's own, because it is the
   * fact the reader can act on.
   */
  function dayBlock(isoDay) {
    if (dayNum(isoDay) > dayNum(maxDay)) {
      return `${dayLabel(isoDay)} is after ${dayLabel(maxDay)}. ${ceilReason}`;
    }
    if (feedFloor.at && isoDay < feedFloor.at) {
      return `${dayLabel(isoDay)} cannot be asked for: ${floorSentence} Pick a feed or a timeframe that reaches further back, and this day opens up.`;
    }
    if (dayNum(isoDay) < dayNum(minDay)) {
      return `${dayLabel(isoDay)} is before ${dayLabel(minDay)}, the oldest day the server's own picker offers (crates/api/src/calendar.rs).`;
    }
    return null;
  }
  /**
   * WHAT A TAKEABLE DAY IS, IN WORDS. Not a refusal — the tooltip of a day that
   * CAN be picked, saying whether it is a session at all and whether the
   * holiday table actually covers it. A weekend inside the window is not an
   * error and must not be drawn as one; it is a day that will return no bars,
   * and the reader is owed that before he counts a shortfall.
   */
  function dayNote(isoDay) {
    const ns = noSessionWhy(isoDay);
    const approx = holidaysKnownFor(isoDay)
      ? ''
      : ` · this page's NSE holiday table covers ${dayLabel(HOL_FROM)} – ${dayLabel(HOL_THRU)} only, so outside it a weekday is assumed to be a session`;
    const ceiling = isoDay === maxDay ? ` · the newest day that can be asked for. ${ceilReason}` : '';
    return `${dayLabel(isoDay)} · ${ns ?? 'trading session'}${approx}${ceiling}`;
  }
  /**
   * A PANEL LEFT OPEN UNDER A RETRACTED RUNG HAS TO CLOSE.
   *
   * Unticking the last segment takes the date fields off the page while their
   * calendar is still latched open. The panel unmounts with it, so nothing is
   * visibly wrong — until the rung comes back and the calendar springs open
   * over a control nobody clicked. `cal.field` is the latch, so releasing it is
   * the whole fix.
   */
  $effect(() => {
    if (!shows.get('window') && cal.field) cal = { ...cal, field: null };
  });
  /**
   * THE CURSOR NEVER LANDS ON A DAY THAT CANNOT BE COMMITTED.
   *
   * Not tidiness — a correctness fix. A day outside the bounds renders as a
   * `disabled` button, and a disabled button cannot take focus: one PageDown
   * past the ceiling dropped focus to `<body>`, after which the panel's keydown
   * handler never fired again and every subsequent key, Enter included, went
   * nowhere. Clamping keeps the cursor on a real target, so focus always has
   * somewhere to be.
   *
   * The FEED's floor clamps it too, and that is deliberate: a cursor sitting on
   * a day this feed refuses is a cursor Enter cannot commit.
   */
  function clampDay(isoDay) {
    if (!isValidIso(isoDay)) return maxDay;
    const lo = feedFloor.at && feedFloor.at > minDay ? feedFloor.at : minDay;
    if (dayNum(isoDay) < dayNum(lo)) return lo;
    if (dayNum(isoDay) > dayNum(maxDay)) return maxDay;
    return isoDay;
  }
  function placeCursor(isoDay) {
    const at = clampDay(isoDay);
    cal = { ...cal, cursor: at, view: at.slice(0, 7) };
  }
  function moveCursor(days) {
    placeCursor(addDays(cal.cursor || maxDay, days));
  }
  /**
   * ═══════════ LOOKING IS UNBOUNDED; THE CURSOR IS NOT ═══════════
   *
   * The two header selects and the ‹ › steps move the VIEW, and the view is
   * clamped only to what this page can DRAW — 2015 to the ceiling's month. The
   * CURSOR is a different value with a different bound: it is the day Enter
   * would commit, so `clampDay` keeps it inside what can actually be taken.
   *
   * They were one value and that was a bug with a rule's voice: navigating to
   * 2015 under a feed whose floor is 2021 snapped the view straight back to
   * 2021, so a month the reader is entitled to LOOK at was unreachable and the
   * page never got to say why its days are struck through. He now gets the
   * month he asked for, forty-two refusals each naming the feed, and the
   * footer's one line saying the same thing once.
   */
  function clampView(ym) {
    if (ym < minMonth) return minMonth;
    if (ym > maxMonth) return maxMonth;
    return ym;
  }
  function pickView(ym) {
    cal = { ...cal, view: clampView(ym) };
  }
  function onCalKey(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      shutCal();
      return;
    }
    // THE HEADER SELECTS KEEP THEIR OWN ARROW KEYS. This handler sits on the
    // panel, so every keydown inside it arrives here — including the ones aimed
    // at the month and year <select>, where the block below would
    // preventDefault them and leave a select that cannot be walked.
    if (e.target !== e.currentTarget && e.target?.tagName === 'SELECT') return;
    const map = {
      ArrowLeft: -1,
      ArrowRight: 1,
      ArrowUp: -CAL_COLS,
      ArrowDown: CAL_COLS
    };
    if (e.key in map) {
      e.preventDefault();
      moveCursor(map[e.key]);
    } else if (e.key === 'PageUp' || e.key === 'PageDown') {
      e.preventDefault();
      const step = e.key === 'PageUp' ? -1 : 1;
      const cur = cal.cursor || maxDay;
      const ym = addMonths(cur.slice(0, 7), step);
      const [y, m] = ym.split('-').map(Number);
      placeCursor(iso(y, m, Math.min(Number(cur.slice(8)), daysInMonth(y, m))));
    } else if (e.key === 'Home') {
      e.preventDefault();
      placeCursor(`${calView}-01`);
    } else if (e.key === 'End') {
      e.preventDefault();
      placeCursor(monthLastDay(calView));
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      commit(cal.cursor);
    }
  }
  /**
   * Roving focus: the cursor day is the only day button in the tab order.
   *
   * It moves focus on OPEN — once, which is what makes the arrow keys reachable
   * at all — and after that only while focus is already on a day. Without that
   * second condition the nav arrows and the two selects would lose focus to the
   * grid the instant they were used.
   */
  let enterOnOpen = false;
  $effect(() => {
    if (!cal.field || !calEl) return;
    const el = calEl.querySelector(`[data-day="${cal.cursor}"]`);
    // THE CURSOR IS NOT IN THE MONTH ON SCREEN — which happens the moment the
    // panel opens on a day a bound has since refused. Focus still has to enter
    // the panel or its keydown handler never fires and the picker is dead to
    // the keyboard; the panel itself carries `tabindex="-1"` for exactly this.
    if (!el && enterOnOpen) {
      enterOnOpen = false;
      calEl.focus();
      return;
    }
    if (!el || document.activeElement === el) return;
    const now = document.activeElement;
    const onADay = now?.classList?.contains('cday') ?? false;
    // ARROWING ACROSS A MONTH BOUNDARY DESTROYS THE FOCUSED BUTTON. The grid is
    // re-rendered for the new month, the node focus was on is gone, and the
    // browser drops focus to `body` — after which the panel's keydown handler
    // never fires again and the picker is dead to the keyboard from the second
    // Down arrow onward. An orphaned focus is therefore reclaimed, while a nav
    // button or a select — which survive the re-render — keep theirs.
    const orphaned = !now || now === document.body || now === calEl;
    if (enterOnOpen || onADay || orphaned) {
      enterOnOpen = false;
      el.focus();
    }
  });

  /**
   * TYPING IS THE OTHER WAY IN, and it never leaves a stale day behind.
   *
   * An unreadable string sets the parsed value to empty rather than keeping the
   * last good one: a field reading `02 Spe 2024` over a window still starting
   * in September would send a pull the operator can see they did not ask for,
   * and `problems` would have nothing to complain about. The refusal
   * `parseDay` wrote is kept beside the characters that earned it.
   */
  function typeDay(field, text) {
    const r = parseDay(text);
    if (field === 'from') {
      fromText = text;
      fromDay = r.iso ?? '';
    } else {
      toText = text;
      toDay = r.iso ?? '';
    }
    typedErr = { ...typedErr, [field]: r.err ?? null };
  }

  // ------------------------------------------------------------------ submit

  /**
   * `e` is optional because there are two ways in and only one of them is a
   * form submit: the button at the foot of the request, and the strip's bulk
   * action at the head of the table. Both send the SAME request — there is only
   * one — so they call the same function rather than growing a second path that
   * could drift from it.
   */
  /**
   * THE ONE PLACE A PULL IS STARTED, and both callers reach it: the form's own
   * Start button with one body per ticked rung, and a census row's Pull with a
   * single body over the days that row is short.
   *
   * ONE REQUEST AT A TIME, IN ORDER. `/pull/spot` is synchronous and the store
   * appends — two in flight would interleave two vendors' writes into one
   * month file — so the rungs are sent one after another and the card counts
   * them.
   */
  async function start(e) {
    e?.preventDefault?.();
    showProblems = true;
    if (problems.length > 0 || phase === 'running') return;
    await runPull(wireBodies, new Set(ticked.map((m) => m.key)));
  }

  async function runPull(bodies, asked) {
    if (bodies.length === 0 || phase === 'running') return;

    receipt = null;
    receipts = [];
    askedKeys = asked;
    sent = { done: 0, of: bodies.length, label: bodies[0].label };
    netError = null;
    pollError = null;
    aborted = false;
    outcomes = [];
    outcomeIndex = new Map();
    samples = [];
    q = '';
    pickedGroup = null;

    // THE BEFORE READING IS TAKEN FIRST AND IS NOT OPTIONAL. Every outcome
    // below is a difference against it; without one there is nothing to
    // subtract and the page would have to guess.
    try {
      baseline = await snapshot();
      live = baseline;
    } catch (why) {
      baseline = null;
      live = null;
      netError = `The store could not be read before starting, so nothing this run does could be measured against it: ${why}`;
      return;
    }

    startedAt = Date.now();
    lastGrowthAt = startedAt;
    finishedAt = 0;
    phase = 'running';
    // HELD, NOT OWNED. One five-second clock for the whole product, released
    // the moment this run stops needing it.
    releaseWatch?.();
    seenRead = store.reads;
    releaseWatch = watchStore(5000);

    controller = new AbortController();
    for (const b of bodies) {
      sent = { done: sent.done, of: bodies.length, label: b.label };
      try {
        const r = await fetch('/pull/spot', {
          method: 'POST',
          headers: { 'content-type': 'application/x-www-form-urlencoded' },
          body: b.body,
          signal: controller.signal
        });
        const one = {
          ...readReceipt(
            await r.text(),
            r.ok,
            r.status,
            r.headers.get('content-type'),
            r.headers.get(RECEIPT_HEADER)
          ),
          rung: b.label
        };
        receipts = [...receipts, one];
        // THE ONE THE CARD SHOWS IS THE FIRST BAD ONE. A later success must not
        // paint over a rung that refused — the list below it carries all of them.
        if (!receipt || (receipt.good && !one.good)) receipt = one;
      } catch (why) {
        if (controller?.signal.aborted) {
          aborted = true;
          break;
        }
        netError = String(why);
        break;
      }
      sent = { done: sent.done + 1, of: bodies.length, label: b.label };
    }
    controller = null;
    finishedAt = Date.now();
    phase = 'done';
    releaseWatch?.();
    releaseWatch = null;

    // THE AFTER READING. Taken even when the request failed — a run that
    // refused halfway still wrote whatever landed before it, and pretending
    // otherwise is the fallback that hides a failure.
    try {
      live = await snapshot();
      pollError = null;
    } catch (why) {
      pollError = String(why);
    }
    classifyBuild();
    // THE CENSUS IS A READING OF THE STORE AND THE STORE JUST MOVED. Leaving it
    // stale would show "never pulled" beside an outcome row saying bars landed.
    // The `snapshot()` above already bumped the shared generation, so the
    // census table is reading that same answer and no second fetch is needed.
    readPilot();
  }

  function stopWatching() {
    controller?.abort();
  }

  function reset() {
    phase = 'idle';
    receipt = null;
    receipts = [];
    sent = { done: 0, of: 0, label: '' };
    askedKeys = new Set();
    netError = null;
    outcomes = [];
    outcomeIndex = new Map();
    baseline = null;
    live = null;
    samples = [];
    aborted = false;
  }

  let showRaw = $state(false);

  onMount(() => {
    const tick = setInterval(() => (nowMs = Date.now()), 500);
    const away = (e) => {
      if (cal.field && calEl && !calEl.contains(e.target) && !e.target.closest?.('.cal-open')) shutCal(false);
      // THE SAME PRESS CLOSES THE STRIP’S OWN MENU. `.dd` is the whole
      // control — the button and the panel it opens — so a press inside the
      // open one is not away, and a press on any other control, a Picker
      // included, is. One value, so two can never be open at once.
      if (drop !== null && !e.target.closest?.('.dd')) drop = null;
    };
    document.addEventListener('pointerdown', away, true);
    return () => {
      clearInterval(tick);
      document.removeEventListener('pointerdown', away, true);
      releaseWatch?.();
      releaseWatch = null;
      controller?.abort();
    };
  });
</script>

<div class="pane">
  <div class="pane-head">
    <span class="pane-title">Ingest</span>
    {#if active}
      <span class="tag acc">{active.display}</span>
      <!-- `pull::vendor::SourceKind::label`, off the wire. The word here used
           to be `transport`, whose vocabulary (`broker`, `archive`) was
           invented in the API handler and existed nowhere else — a second
           spelling of the split `kind` already carried. -->
      {#if active.kind_label}
        <span class="tag">{active.kind_label}</span>
      {:else}
        <span class="tag warn" title={SOURCE_KIND_UNSTATED}>source kind not stated</span>
      {/if}
      <span class="tag info">{RUNGS.find((r) => r.dir === rung)?.label ?? rung}</span>
    {/if}
    <span class="spacer"></span>
    {#if phase === 'running'}
      <span class="status busy" aria-live="polite">
        <span class="dot acc live"></span>
        <span class="txt">{Verb} running</span>
        <span class="ms">{clockOf(elapsedMs)}</span>
      </span>
    {:else if phase === 'done' && receipt}
      <span class="status" class:bad={!receipt.good}>
        <span class="dot" class:up={receipt.good} class:down={!receipt.good}></span>
        <span class="txt">{receipt.verdict}</span>
        <span class="ms">{clockOf(elapsedMs)}</span>
      </span>
    {/if}
    <a class="link" href="/audit">The record of every run</a>
  </div>

  <!-- THE CONTROL COMES WITH THE REFUSAL. Each of these three states is the
       page saying "not with this feed", and the one action every one of them
       asks for is a change of feed — so the control is drawn inside the
       refusal rather than named in it. It used to point at the top bar; the
       top bar no longer carries a picker on this route, and a page that both
       refuses and offers no way out is a dead end. `.blankpick` is the one
       rule that keeps the dropdown from stretching to the width of a page. -->
  {#if feeds.error}
    <div class="blank">
      <h2>The feed list could not be read</h2>
      <p>Nothing can be ingested until the server names a feed to ingest into.</p>
      <p class="mono down">{feeds.error}</p>
      <div class="blankpick">{@render feedRung()}</div>
    </div>
  {:else if !active}
    <div class="blank">
      <h2>No feed is selected</h2>
      <p>
        Ingest writes into exactly one feed's store, so one has to be chosen before a window means
        anything. Pick one here — it is the same control that stands first in the strip once a feed
        is chosen, and it is the only one on this page.
      </p>
      <div class="blankpick">{@render feedRung()}</div>
    </div>
  {:else if archiveHidden}
    <div class="blank">
      <h2>{active.display} has nothing to ingest yet</h2>
      <p>
        This is an archive feed: it reads CSV files that have to be bought first. The server refuses
        it for a reason of its own, quoted here rather than paraphrased.
      </p>
      <p class="warn mono">{active.why}</p>
      <p class="alt">
        <span class="lbl">What to do</span>
        Buy the archive and give the feed a store prefix, or select a broker feed below. The folder
        form appears here the day this feed reports itself ready — nothing on this page names a
        vendor.
      </p>
      <div class="blankpick">{@render feedRung()}</div>
    </div>
  {:else}
    <div class="grid body">
      <!-- TWO COLUMNS WHILE THERE ARE TWO CARDS, ONE WHEN THE RUN CARD IS NOT
           DRAWN. Without this the form kept its 5fr and the vacated 4fr stayed
           on screen as a column of nothing — the panel removed, its hole left
           behind. -->
      <div class="cols" class:solo={phase === 'idle'}>
        <!-- ====================================== THE CONTROL STRIP =======
             ONE PANEL, and every control in it is a labelled dropdown standing
             in the same row as its peers: the feed across the top, then
             universe · instruments · segments · timeframe as equal columns,
             then the two days.

             WHAT LEFT THE RENDERED PAGE, AND WHERE EACH FACT WENT. The four
             tiles, the "The request" heading and its chip, the "not an NSE
             universe" block, the "counted here, not requestable" paragraph,
             the ceiling paragraph and the "Dhan is a broker" sentence are gone
             from the page. Nothing they said is denied — each one is now the
             `title` of the control it is about, or one clause under it:

               names this feed reaches  -> the Instruments control's own face,
                                           and the clause under it
               window · month files     -> the clause under Days to pull
               instrument-months asked  -> the ask line at the foot of the
                                           strip; it is `productLine` verbatim
               held in this window      -> the same line, beside it
               refused universes        -> each row's own `title`, which is
                                           `universeRefusal(u)` in full, plus
                                           `refusalSummary` on the control
               the swept pair           -> the last row of the same menu, said
                                           on the row rather than beside it
               the ceiling              -> `ceilReason` on the To date control
                                           and in the grid it opens
               broker window splitting  -> the Days to pull label's `title`
             ============================================================== -->
        <section class="sel">
          <form onsubmit={start} class="form">
            <!-- THE GRID IS THE FIELDSET. A wrapper between the two would take
                 the grid's tracks for itself and leave the controls stacked
                 inside one column, which is the shape this page is being
                 rewritten out of. -->
            <fieldset class="pickers" disabled={phase === 'running'}>
              <!-- THE FEED IS THE FIRST RUNG OF THE CASCADE, AND THEREFORE THE
                   FIRST CONTROL IN THE ROW. It stood across the top of the
                   strip as a full-width SCOPE LINE — a label, the feed's name
                   wearing a control's face with no affordance, and a clause —
                   which is a paragraph pretending to be a control. It is a
                   dropdown again, and it is a PEER: same track, same label
                   style, same button height and the same one baseline as
                   Universe · Instruments · Segments · Timeframe.

                   ONE CONTROL FOR ONE VALUE, STILL. The top bar's picker is not
                   drawn on the routes that carry their own — see FEED_OWNED in
                   `+layout.svelte`. Two controls writing one `feeds.active`
                   teach the reader they are independent when they are not, and
                   that is the defect the scope line was introduced to fix; the
                   fix is which control exists, not whether the page has one.

                   THE SCOPE SENTENCE SURVIVES AS THE CONTROL'S OWN CLAUSE,
                   which is where it was always headed: "everything below is
                   this feed's answer · carries N name(s) · what it does not
                   carry is not measured" is what tells a reader the whole page
                   is scoped to one vendor, and it is load-bearing. The long
                   sentence is on the button's `title`.

                   A REFUSED FEED IS DRAWN AND REFUSED, never omitted:
                   `/feeds.json` marks a feed not ready and states why, and the
                   row quotes that reason rather than paraphrasing it. -->
              {@render feedRung()}

              {#if shows.get('universe')}
                <!-- THE UNIVERSE RUNG, AND IT IS A DROPDOWN. The eight in NSE's
                     order, then the swept pair, which is not one of them and is
                     labelled as what it is on its own row. A refused row is
                     drawn, disabled and carries `universeRefusal(u)` whole —
                     the same sentence the deleted paragraph printed, on the
                     row it is about. -->
                <div class="pk">
                  <!-- THE SUB-CLAUSE IS ON THE CONTROL, NOT IN THE LABEL. "The
                       set this feed is asked for" is a true and useful sentence
                       and it is the FIRST thing `universeTitle` now says. In the
                       label it was a second line in one cell and no line in the
                       next, which is half of why this row had no baseline. -->
                  <span class="plbl">Universe</span>
                  <div class="dd">
                    <button
                      class="ddb"
                      type="button"
                      aria-expanded={drop === 'uni'}
                      title={universeTitle}
                      onclick={(e) => {
                        e.stopPropagation();
                        drop = drop === 'uni' ? null : 'uni';
                      }}>{universeSpec.label}</button
                    >
                    {#if drop === 'uni'}
                      <div class="ddm" role="group" aria-label="Universe — the set to pull">
                        <!-- SAME RULE AS THE PICKERS: a set no request can
                             name is folded behind one line rather than listed
                             dead.
                             This comment used to name `F&O underlyings` and
                             `Everything` as the two rows that could never be
                             clicked, "because api::ingest::SpotTarget spells
                             swept, indices and equities only". Both are
                             requestable as of D-0136, and the enum has nine
                             variants rather than three. The rule stays because
                             a future row may still arrive without a target;
                             the two examples are gone, and no count of the
                             enum is stated here — this page cannot see it, so
                             any number it gives goes stale in silence. -->
                        {#each UNIVERSES.filter((u) => universeRefusal(u) === null) as u (u.id)}
                          {@const why = null}
                          <button
                            class="ddr"
                            type="button"
                            class:off={why !== null}
                            disabled={why !== null}
                            aria-pressed={universe === u.id}
                            title={why ??
                              `Pulls with target=${u.target}. ${feedName(feeds.active)} reaches ${reachKnown ? n(reach) : 'an uncounted number of'} name(s) in it, counted from /instruments.json?feed=${feeds.active ?? ''}.${u.note ? ` ${u.note}.` : ''}`}
                            onclick={() => {
                              universe = u.id;
                              drop = null;
                            }}
                          >
                            <span class="tk">{universe === u.id ? '✓' : ''}</span>
                            <span class="nm">{u.label}</span>
                            <!-- THE WIRE SLUG IS NOT DRAWN BESIDE THE NAME.
                                 It read `target=n50` in a second column on
                                 every row — the value the form POSTs, which is
                                 an implementation detail of the request and
                                 not a property of the SET the reader is
                                 choosing between. Removed at the operator's
                                 instruction, 14 Aug 2026. It is still on the
                                 wire, still in the request body the "What goes
                                 on the wire" fold prints, and still the row's
                                 `u.target`; what is gone is restating it in
                                 the picker, where the only question is which
                                 set. A row that CANNOT be requested keeps its
                                 word, because there the absence is the fact. -->
                            {#if why !== null}
                              <span class="ct warn">no target</span>
                            {/if}
                          </button>
                        {/each}

                        <!-- THE DRAWER. It holds the SWEEP PAIR and the
                             refusals, in that order, and it deletes neither.
                             The operator asked for the NSE families to lead the
                             menu; §1 still names NSE-NIFTY and NSE-BANKNIFTY as
                             the whole engine surface, and it is still the only
                             set a broker path can serve, so it moves DOWN
                             rather than out. Removing it would leave this form
                             unable to build a legal request at all.
                             The refusals keep their reasons; each names whether
                             the gap is membership or the wire. -->
                        <!-- THE SWEEP PAIR IS NO LONGER OFFERED. The operator
                             asked for it gone three times; §1 still names it as
                             the engine surface and `api::ingest::SpotTarget`
                             still spells only swept, indices and equities, so
                             this menu can now name NO set a broker pull can
                             serve until that enum grows. Recorded in the commit
                             rather than argued here. -->
                        {#if false}
                          <button
                            class="ddr"
                            type="button"
                            aria-pressed={true}
                            title="CLAUDE.md §1 names NSE-NIFTY and NSE-BANKNIFTY as the whole engine surface. It is the only set a broker path can serve: pull::vendor::HttpSpec carries no request-parameter map, so a wider target is refused rather than fetching one series under a name nobody asked for."
                            onclick={() => (drop = null)}
                          >
                            <span class="tk">✓</span>
                            <span class="nm">{SWEPT.label}</span>
                            <span class="ct">the pair this engine sweeps</span>
                          </button>
                        {/if}
                        {#if uniTucked.length > 0}
                          <button
                            class="ddr tuck"
                            type="button"
                            aria-expanded={uniOpen}
                            onclick={() => (uniTuck = !uniOpen)}
                          >
                            <span class="tk">{uniOpen ? '▾' : '▸'}</span>
                            <span class="nm">{uniTuckLabel}</span>
                            <span class="ct">{uniOpen ? 'hide' : 'show'}</span>
                          </button>
                          {#if uniOpen}
                            {#if false}
                              <!-- LIVE, and only here while it is NOT the
                                   selection. When it IS, it is promoted above
                                   the drawer instead — see `uniTucked`. -->
                              <button
                                class="ddr"
                                type="button"
                                aria-pressed={false}
                                title="CLAUDE.md §1 names NSE-NIFTY and NSE-BANKNIFTY as the whole engine surface — this repository's own sweep set, not an NSE index family. It pulls with target=swept, and it is the only set a broker path serves: pull::vendor::HttpSpec carries no request-parameter map, so a wider target is refused rather than fetching one series under a name nobody asked for."
                                onclick={() => {
                                  universe = SWEPT.id;
                                  drop = null;
                                }}
                              >
                                <span class="tk"></span>
                                <span class="nm">{SWEPT.label}</span>
                                <span class="ct">the pair this engine sweeps</span>
                              </button>
                            {/if}
                            {#each refusedUniverses as x (x.u.id)}
                              <button class="ddr off" type="button" disabled title={x.why}>
                                <span class="tk"></span>
                                <span class="nm">{x.u.label}</span>
                                <span class="ct warn">no target</span>
                              </button>
                            {/each}
                          {/if}
                        {/if}
                      </div>
                    {/if}
                  </div>
                  <span
                    class="pknote"
                    class:warn={!reachKnown}
                    title={reachKnown ? `${n(reach)} reachable · target=${target}` : reachWhy}
                  >
                    {reachKnown ? `${n(reach)} reachable · target=${target}` : reachWhy}
                  </span>

                  <!-- ═══════ WHAT AN ARCHIVE HAS INSTEAD OF A UNIVERSE ═══════

                       The line above counts what a MASTER reaches, and the two
                       archive feeds publish none — `Vendor::MASTERED` is the
                       three REST feeds and nothing else, so every tier resolves
                       to nothing for TrueData and GDFL and the count above is
                       an honest zero about the wrong question.

                       The right question for a folder is what is IN it, and the
                       only way to answer it is to walk it. `/folder.json` did
                       that walk the moment this feed was chosen, and its six
                       facts were being spent on one line at the foot of a
                       calendar popover. They are the closest thing this feed
                       HAS to a universe, so they belong on the universe control.

                       NOTHING HERE IS DECLARED. Every number is off the wire,
                       and the state a folder is in decides which sentence gets
                       drawn — empty, blank and days are three different answers
                       and a halt is not an answer at all. D-0141. -->
                  {#if archiveCensus}
                    {@const c = archiveCensus}
                    {#if c.state === 'reading'}
                      <span class="pknote">reading {active.display}'s folder…</span>
                    {:else if c.state === 'halted'}
                      <span class="pknote wrap warn" title={c.why}>
                        the folder could not be read{c.path ? ` — ${c.path}` : ''}. This is a HALT
                        and not an empty folder: nothing was counted, so nothing below is a census
                        of anything.
                      </span>
                    {:else if c.state === 'days'}
                      <span
                        class="pknote wrap"
                        title={`Walked by GET /folder.json?feed=${feeds.active ?? ''} when this feed was selected — pull::folder::read_reach, O(members). ${c.path ?? ''}`}
                      >
                        {n(c.files)} file(s) · {n(c.rows)} row(s) · {dayLabel(c.earliest)} – {dayLabel(
                          c.latest
                        )} — this feed's universe is the folder, read from it
                      </span>
                    {:else if c.state === 'blank'}
                      <span class="pknote wrap warn">
                        {n(c.files)} file(s) are there and not one carries a row. They were bought
                        and they are blank — which is a different thing from not having bought them,
                        and this is the only place the two are told apart.
                      </span>
                    {:else}
                      <span class="pknote wrap warn">
                        the folder is there and holds nothing{c.path ? ` — ${c.path}` : ''}. That is
                        an ANSWER, not a failure: the months have not been bought and put there yet.
                      </span>
                    {/if}
                    <!-- THE NAMES. `/folder.json` carries `instruments` since
                         D-0141: `pull::folder::read_census` collects what
                         `pull::archive::Member` already took off each file
                         name, sorted, whole, never truncated.

                         THREE STATES AND THEY ARE NOT TWO. `null` is a server
                         that sent no such field — a running binary that
                         predates it, which names its own fix. `[]` is a folder
                         that names nothing, which is an ANSWER. A list is a
                         list. Collapsing the first two would print "0
                         instruments" at a server that was never asked. -->
                    {#if c.instruments === null}
                      <span
                        class="pknote wrap warn"
                        title="GET /folder.json emits `instruments` — every distinct name pull::archive::Member took off a file name in that folder, sorted. This server sent no such field, so this page cannot name what is in the folder and does not guess: the file names are the ONLY identity an archive has, and there is no ISIN, no security id and no master to fall back on. Restart the API on a build that emits it."
                      >
                        this server sends no instrument list for the folder — what is in it is not
                        known here, and this page will not synthesise names from a file count
                      </span>
                    {:else if c.instruments.length > 0}
                      <span
                        class="pknote wrap"
                        title={`${c.instruments.length} distinct name(s), read off the file names in ${c.path ?? 'the folder'} and sorted. This is the whole identity an archive has — there is no ISIN, no security id and no master — so it is also the whole of what a universe could mean for this feed. First twenty: ${c.instruments.slice(0, 20).join(', ')}`}
                      >
                        {n(c.instruments.length)} instrument(s) named — {c.instruments
                          .slice(0, 6)
                          .join(', ')}{c.instruments.length > 6
                          ? ` and ${n(c.instruments.length - 6)} more`
                          : ''}
                      </span>
                      <!-- TWO FILES CLAIMING ONE NAME, SAID RATHER THAN
                           DEDUPLICATED IN SILENCE. It is the `ambiguous` bucket
                           of D-0141 on the folder side, and a reader who saw
                           only the distinct list could not tell a clean folder
                           from a colliding one. -->
                      {#if c.collisions > 0}
                        <span
                          class="pknote wrap warn"
                          title="pull::folder::census_of counts the members whose instrument name a previous member had already claimed, rather than deduplicating in silence. GDFL nests Options/ and Futures/, so one stem can appear under both. Two files claiming one instrument is the `ambiguous` case D-0141 names on the folder side: it is not resolved by whichever was walked first."
                        >
                          {n(c.collisions)} file(s) name an instrument another file already named —
                          two members claiming one name are not resolved by walk order
                        </span>
                      {/if}
                    {:else}
                      <span class="pknote wrap warn">
                        the folder names no instrument at all — this is the list being EMPTY, which
                        the server answered, and not a list it failed to send
                      </span>
                    {/if}
                  {/if}
                </div>
              {/if}

              {#if shows.get('instruments')}
                <!-- TICKED, ONE BY ONE, AND THE LIST IS THE CONTROL. This was a
                     roster: two summary lines and nothing selectable, with its
                     own label conceding "read, not chosen". Every name in the
                     universe is a checkbox now, searchable and sorted, with the
                     two bulk actions over WHAT IS SHOWN and a face that counts
                     what is ticked.

                     THE ONE THING THE TICKS CANNOT DO IS NARROW THE REQUEST,
                     and that is said on the control and again under the ask
                     line rather than being worked around: SpotRequest has no
                     member field. What they narrow is what this page counts,
                     measures and reports on — the ask, the census, the outcome
                     rows. -->
                <div class="pk">
                  <span class="plbl">Instruments in that universe</span>
                  <div class="dd">
                    <button
                      class="ddb"
                      type="button"
                      aria-expanded={drop === 'ins'}
                      title={`Ticked — this is what is counted. ${insSummary}. A tick decides what this page counts — the ask line, the census below and the outcome list after a run. It cannot narrow the REQUEST: api::ingest::SpotRequest carries target, window, feed and granularity and no member field, so target=${target} asks for all ${reachKnown ? n(reach) : '—'} either way.`}
                      onclick={(e) => {
                        e.stopPropagation();
                        drop = drop === 'ins' ? null : 'ins';
                      }}>{insSummary}</button
                    >
                    {#if drop === 'ins'}
                      <div class="ddm wide" role="group" aria-label="Instruments in that universe">
                        <!-- THE HEADER IS OUTSIDE EVERY KEYED BLOCK, so Svelte
                             keeps this exact input node across updates: a
                             detached element is blurred, the caret vanishes and
                             the next character is dropped. -->
                        <div class="srch">
                          <input
                            class="search"
                            type="search"
                            bind:value={insQ}
                            placeholder="Search {n(insRows.length)} name(s) — ticker, key or kind"
                            aria-label="Search the instruments in this universe"
                            onclick={(e) => e.stopPropagation()}
                          />
                          <p class="shint">
                            Matches the trading symbol, the store key or the kind. {n(
                              insShown.length
                            )} shown of {n(insRows.length)}.
                          </p>
                          <div class="sact">
                            <button
                              type="button"
                              disabled={insSelectable.length === 0}
                              title={insSelectable.length === 0
                                ? 'Nothing here can be ticked: every row shown is a name this feed does not list.'
                                : `Ticks the ${n(insSelectable.length)} name(s) currently shown. A search narrows this button with the list.`}
                              onclick={insSelectShown}
                            >
                              {insQ.trim()
                                ? `Select all ${n(insSelectable.length)} shown`
                                : `Select all ${n(insSelectable.length)}`}
                            </button>
                            <button
                              type="button"
                              disabled={insCount === 0}
                              title={insCount === 0
                                ? 'Nothing is ticked, so there is nothing to clear.'
                                : 'Unticks the names currently shown. A search narrows this button with the list.'}
                              onclick={insClearShown}
                            >
                              {insQ.trim() ? 'Clear all shown' : 'Clear all'}
                            </button>
                          </div>
                        </div>

                        <div class="inslist">
                          {#if insEmptyWhy}
                            <p class="insnone">{insEmptyWhy}</p>
                          {:else}
                            <!-- A ROW THIS FEED CANNOT SERVE IS DRAWN, REFUSED
                                 AND CARRIES THE REASON. Deleting it would teach
                                 the reader the name does not exist, which is
                                 the opposite of the finding. -->
                            {#each insShown.slice(0, 400) as r (r.key)}
                              <label class:off={r.off !== null} title={r.off ?? ''}>
                                <input
                                  type="checkbox"
                                  checked={r.off === null && !insOff.has(r.key)}
                                  disabled={r.off !== null}
                                  onchange={() => toggleIns(r.key)}
                                  onclick={(e) => e.stopPropagation()}
                                />
                                <span class="nm mono">{r.sym}</span>
                                <span class="ct" class:warn={r.off !== null}>
                                  {r.off === null ? r.detail : 'not on this feed'}
                                </span>
                              </label>
                            {/each}
                            {#if insShown.length > 400}
                              <p class="insnone">
                                {n(insShown.length - 400)} more are not drawn. Narrow the search to
                                reach them — the count on the button is the whole of it, and Select
                                all acts on all {n(insSelectable.length)} shown, drawn or not.
                              </p>
                            {/if}
                          {/if}
                        </div>

                        <!-- THE MEASUREMENT THAT FILLS THE REFUSED ROWS. It is
                             one request per feed and it rebuilds a census
                             server-side each time, which is why it is a press
                             and not a page load. -->
                        {@render instrumentRung()}
                      </div>
                    {/if}
                  </div>
                  <span
                    class="pknote"
                    class:warn={!reachKnown}
                    title={reachKnown
                      ? `${n(reach)} of ${n(catalogue.rows.length)} row(s) in ${active.display}'s master carry this universe.`
                      : reachWhy}
                  >
                    {reachKnown
                      ? `of ${n(catalogue.rows.length)} in ${active.display}'s master`
                      : reachWhy}
                  </span>
                </div>
              {/if}

              {#if shows.get('segment')}
                {@render segmentRung()}
              {/if}

              {#if shows.get('rung')}
                <!-- "Timeframe" is the cascade's word for this rung and the word
                     /db and /markets use for the same axis; "bar length" is what
                     it means, and `granularity` is what it is called on the
                     wire. All three, once, on the one control that carries
                     them.

                     TWO ROWS ON A BROKER, THREE ON AN ARCHIVE, AND NEVER
                     ELEVEN. The owner's rule of 14 Aug 2026: one minute and one
                     day alone, plus the tick rung for TrueData and GDFL alone.
                     The eight that left were rungs NO feed in this build
                     declares — the union of all four descriptors' declared
                     granularities is exactly the three that remain — so they
                     were drawn live and annotated "not fetched by this build"
                     on every feed, forever. Everything coarser than a minute is
                     a fold of bars already on this disk, which is why there is
                     no "add a timeframe" here and nothing missing that one
                     would add. See `RUNGS`.

                     WHICH TWO FEEDS SHOW THE THIRD ROW IS NOT DECIDED HERE. It
                     is `/feeds.json`'s granularity floor: a feed that bottoms
                     out at a minute has the second rung permanently refused and
                     `rungRows` drops it. No vendor is named in this file. -->
                <div class="pk">
                  <span
                    class="plbl"
                    title="Bar lengths — the granularity field on the wire. Timeframe is the cascade's word for this rung and the word /db and /markets use for the same axis; bar length is what it means; granularity is what it is called on the wire. All three name one thing."
                    >Timeframe</span
                  >
                  <Picker
                    label="bar lengths"
                    summary={rungsChosen.length === 0
                      ? 'No timeframe ticked'
                      : rungsChosen.length === rungTally.live
                        ? `All ${n(rungTally.live)} offered`
                        : rungsChosen.map((r) => r.label).join(', ')}
                    rows={rungRows}
                    selected={rungSet}
                    onchange={(sel) => {
                      rungSet = sel;
                      // The drop notice is the operator's to dismiss, and
                      // touching this control is how he does it: he has just
                      // answered it.
                      droppedRungs = null;
                      cPage = 1;
                    }}
                  />

                  <!-- ══════════ WHAT THE FEED CHANGE TOOK, AND WHY ══════════
                       Never a silent keep and never a silent clear. It stands
                       until the operator moves the timeframe himself or changes
                       the feed again — the change that caused it is over in a
                       frame, and a notice that dies with it is one nobody
                       reads. -->
                  {#if droppedRungs}
                    <p class="caution loud drop" aria-live="polite">
                      <span class="tag down">dropped</span>
                      <span class="msg">
                        {n(droppedRungs.rows.length)} ticked timeframe(s) were removed when the feed
                        became <b>{droppedRungs.feed}</b>, which can never serve them:
                        {#each droppedRungs.rows as d (d.dir)}
                          <br /><b class="mono">{d.label}</b> — {d.why}
                        {/each}
                        <br />{droppedRungs.kept === 0
                          ? 'Nothing is ticked now, so nothing below can be counted until you pick a bar length this feed serves.'
                          : `${n(droppedRungs.kept)} timeframe(s) survived the change and are still ticked.`}
                      </span>
                    </p>
                  {/if}

                  <!-- THE FEED'S OWN FLOOR, STATED ONCE, WHERE IT BINDS. The
                       row-level sentences say what each rung is refused for;
                       this says what the FEED is, which is the fact that
                       decides whether the second rung is one of the rows
                       above. -->
                  {#if feedFinest}
                    <span class="pknote wrap" title={`${feedFinest.because} — ${feedFinest.source}`}>
                      {feedFinest.short} One record at {feedFinest.rungPhrase} is a
                      {feedFinest.label}.
                    </span>
                  {:else if feeds.active}
                    <!-- A MISSING FIELD, NAMED AS ONE. This used to read "no
                         granularity floor stated", which was true of a browser
                         holding a transcription that did not cover the feed.
                         There is no transcription now, so the only way to be
                         here is a server that did not send `finest` — and that
                         is a build to restart, not a vendor to wonder about. -->
                    <span
                      class="pknote wrap warn"
                      title="crates/pull carries a granularity floor for every feed it can name, and /feeds.json emits it as `finest` on every row — the finest rung, whether one record there is a bar, a conflated snapshot or a real tick stream, why nothing finer exists in the vendor's own terms, and where those words were read. This page holds no copy of that table: a hardcoded fallback would look exactly as authoritative as a reading, and the operator could not tell them apart. So nothing is refused here, and an unstated floor is not an unlimited one."
                    >
                      this server sent no granularity floor for {feedName(feeds.active)} — nothing
                      here refuses any rung for it
                    </span>
                  {/if}
                  <!-- MULTI-SELECT, AND EVERY TICK IS A REQUEST. The wire
                       carries one `granularity` per POST, so N ticks are N
                       POSTs sent in the ladder's order — the wire fold prints
                       every one and the run card counts them.

                       THERE IS NO "+ ADD A TIMEFRAME" HERE, AND THE REASON IS
                       NOT THAT ONE IS HARD TO BUILD. Everything coarser than a
                       minute is a fold of one-minute bars already on this disk,
                       so asking a vendor for it spends a request, a quota and a
                       window on arithmetic `crates/pull` does for nothing. The
                       owner's rule, 14 Aug 2026. And a rung invented in the
                       browser would be refused anyway:
                       `api::ingest::parse_granularity` matches the directory
                       name exactly.

                       WHAT IS DRAWN IS PER FEED. A rung the active vendor does
                       not publish is filtered out — `rungRows`, D-0137 —
                       because no work closes it, and that filter is what puts
                       the second rung on the two archive feeds and nowhere
                       else. The rungs drawn WHILE refused are the ones a person
                       could close: unfetched, or unstored. -->
                  <span
                    class="pknote"
                    class:warn={rungsChosen.length === 0}
                    title="Three rungs are offered — one minute, one day, and the archives' one second — because they are the whole of what the four descriptors in pull::vendor declare between them, and because everything coarser is folded from bars this repository already holds. A rung the active vendor cannot publish at all is not listed: /feeds.json's granularity floor says no pull, entitlement, purchase or code change makes it exist, so there is no work behind the row. Rungs this build does not fetch yet, and rungs the store has no directory for, ARE drawn — each names work a person could do. A rung cannot be ADDED from here: parse_granularity matches the directory name exactly and refuses anything else."
                  >
                    {#if rungsChosen.length === 0}
                      no timeframe ticked — nothing below can be counted
                    {:else}
                      <!-- "offered", NOT "this feed serves". `rungTally.live` is
                           what is DRAWN, and a drawn row may still be one this
                           build does not fetch — TrueData is offered 1 day and
                           declares none. The old wording read the drawn count
                           as a served count and overstated every archive feed
                           by one. The per-row `not fetched by this build` is
                           where that is answered. -->
                      {n(rungsChosen.length)} ticked of {n(rungTally.live)} offered ·
                      {n(rungsChosen.filter((r) => r.stored).length)} with a store directory ·
                      {n(rungsChosen.length)} request(s) per {verb}
                    {/if}
                  </span>
                  <!-- WHAT THE SERVER DID NOT SAY, SAID. `history[].served` is
                       the bit `crates/api`'s `served()` gates the POST on; a
                       binary that predates the field answers nothing, and the
                       page reports that rather than assuming every rung is
                       fetched. An absent answer is a state, not a default. -->
                  {#if rungServed === null && active}
                    <span
                      class="pknote wrap warn"
                      title="GET /feeds.json emits a history array carrying, per rung, whether Descriptor::granularities declares it. This server sent no such array, so this control cannot say which rungs the build actually fetches and does not guess. The server still refuses an undeclared rung by name — it will just be the run that tells you rather than this line."
                    >
                      this server states no per-rung `served` flag — which rungs the build fetches is
                      not known here
                    </span>
                  {/if}
                </div>
              {/if}

              {#if shows.get('window')}
                <!-- DAYS TO PULL — and DAYS is the honest unit. The vendor
                     answers a day range and the wire carries one:
                     `api::ingest::parse_spot` reads `from` and `to` as
                     `YYYY-MM-DD`. A month picker stood here and named a unit of
                     the STORE as if it were the unit of the REQUEST.

                     NOT A NATIVE `<input type="date">`: it renders `02/09/2024`,
                     which is 2 September to one reader and 9 February to
                     another. These are text fields over an ISO value that never
                     leaves ISO, and the ▦ opens the grid. -->
                <div class="pk wide2">
                  <!-- "THE RANGE THE VENDOR ANSWERS" IS A REST FACT AND IS
                       FALSE FOR A FOLDER FEED. There is no vendor in that
                       sentence: the range is the files somebody bought and put
                       in a directory on this machine, and nothing about it is
                       answered by anybody. -->
                  <span
                    class="plbl"
                    title={(isFolderFeed
                      ? 'The range is whatever files are in the folder. '
                      : 'The range the vendor answers. ') +
                      (isBroker
                        ? `${active.display} is a broker. The window is split to its per-request cap by the server and the rate budget is charged per request. Both ends are inclusive here; on the wire crates/pull must send the day after "to", which Dhan documents as non-inclusive.`
                        : 'Both ends are inclusive here and in every count on this page.')}
                    >Days to {verb}</span
                  >
                  <div class="dates">
                    <div
                      class="dcell field cal-open"
                      class:bad={(showProblems || staleWindow) && problemFor.has('from')}
                    >
                      <span class="dlbl">From date</span>
                      <div class="dwrap">
                        <input
                          class="din mono"
                          type="text"
                          autocomplete="off"
                          spellcheck="false"
                          placeholder="dd Mon yyyy"
                          aria-label={`First day to ${verb}`}
                          aria-invalid={Boolean(fromText.trim()) && !isValidIso(fromDay)}
                          value={fromText}
                          oninput={(e) => typeDay('from', e.currentTarget.value)}
                        />
                        <button
                          type="button"
                          id="cal-from"
                          class="dbtn"
                          aria-haspopup="dialog"
                          aria-expanded={cal.field === 'from'}
                          aria-label="Choose the first day"
                          onclick={() => openCal('from')}>▦</button
                        >
                      </div>
                      {#if cal.field === 'from'}{@render dayGrid()}{/if}
                    </div>
                    <div
                      class="dcell field cal-open right"
                      class:bad={(showProblems || staleWindow) && problemFor.has('to')}
                    >
                      <span class="dlbl">To date</span>
                      <div class="dwrap">
                        <input
                          class="din mono"
                          type="text"
                          autocomplete="off"
                          spellcheck="false"
                          placeholder="dd Mon yyyy"
                          aria-label={`Last day to ${verb}`}
                          aria-invalid={Boolean(toText.trim()) && !isValidIso(toDay)}
                          value={toText}
                          oninput={(e) => typeDay('to', e.currentTarget.value)}
                        />
                        <!-- THE CEILING RIDES THE CONTROL IT BOUNDS. It used to
                             be a paragraph under the form that spoke only when
                             the newest askable day was behind the clock; it now
                             says the same thing on the button that opens the
                             grid, where the reader meets it at the moment it
                             costs him a day, and on every struck cell inside. -->
                        <button
                          type="button"
                          id="cal-to"
                          class="dbtn"
                          aria-haspopup="dialog"
                          aria-expanded={cal.field === 'to'}
                          aria-label="Choose the last day"
                          title={`Newest day that can be asked for: ${dayLabel(maxDay)}. ${ceilReason}`}
                          onclick={() => openCal('to')}>▦</button
                        >
                      </div>
                      {#if cal.field === 'to'}{@render dayGrid()}{/if}
                    </div>
                  </div>
                  <span
                    class="pknote"
                    class:warn={Boolean(fromDay) && Boolean(toDay) && !windowOk}
                  >
                    {#if windowOk}
                      {dayLabel(from)} – {dayLabel(to)} · {n(windowDays)} day(s) · {n(
                        windowMonths.length
                      )} month file(s) {monthLabel(windowMonths[0])}{windowMonths.length > 1
                        ? ` – ${monthLabel(windowMonths[windowMonths.length - 1])}`
                        : ''}
                    {:else if fromDay && toDay}
                      the two dates are the wrong way round
                    {:else}
                      no window picked
                    {/if}
                  </span>
                </div>

                {#if isFolderFeed}
                  <div class="pk" class:bad={showProblems && problemFor.has('folder')}>
                    <span class="plbl">Folder</span>
                    <input
                      class="din mono"
                      bind:value={folder}
                      placeholder={folderReach.body?.path ?? '/path/to/vendor/csvs'}
                      title="CSV files already bought. No token, no rate budget and no 'not today' — a file's last day is not a question about the clock."
                    />
                    <!-- WHERE THE REACH WAS READ, WHICH IS THE FOLDER THIS
                         BUILD RESOLVES ON ITS OWN.
                         `pull::folder::root` reads BRUTEX_ARCHIVES, else
                         $HOME/.brutex/vendor-data — the same two-step the store
                         and the masters roots use, with no literal path in any
                         tracked file. The box above is still what the REQUEST
                         carries, so the two are shown together rather than
                         assumed equal: a reach read from one folder beside a
                         run against another is exactly the silent disagreement
                         this page exists to surface. -->
                    <span class="pknote">no token · no rate budget</span>
                    <!-- THE PATH WRAPS. A `.pknote` clips and ellipses by
                         default, and a PATH with its end cut off is worse than
                         no path at all — it looks like an answer and cannot be
                         acted on. Same defect the `.wrap` modifier exists for
                         one comment above. -->
                    {#if folderReach.body?.path}
                      <span class="pknote wrap">
                        resolved: <button
                          class="pathbtn mono"
                          type="button"
                          onclick={() => {
                            // GUARDED, like the placeholder two lines above
                            // already is. Without it a resolved-path button
                            // rendered before the probe answered would assign
                            // `undefined` into the folder box and the archive
                            // path would silently become empty.
                            if (folderReach.body?.path) folder = folderReach.body.path;
                          }}
                          title="Use the folder this build resolves on its own — BRUTEX_ARCHIVES, else $HOME/.brutex/vendor-data, one subfolder per feed. Resolved exactly the way the store and masters roots are."
                          >{folderReach.body.path}</button
                        >
                      </span>
                    {/if}
                  </div>
                {:else if sourceKindUnstated}
                  <!-- THE FORM'S SHAPE IS THE ANSWER TO A QUESTION THIS SERVER
                       DID NOT ANSWER, so the shape is refused rather than
                       guessed. A folder field drawn here for a broker asks for
                       a path nothing will read; one omitted for an archive
                       hides the only input that feed has. §4: name the
                       failure. -->
                  <p class="caution loud" role="alert">
                    <span class="tag warn">source kind not stated</span>
                    <span class="msg">{SOURCE_KIND_UNSTATED}</span>
                  </p>
                {/if}
              {/if}

              <!-- THE SIZE OF THE ASK, IN ONE LINE, AT THE FOOT OF THE STRIP.
                   This is the whole of what the four tiles carried that is not
                   already on a control: the product and its factors, which is
                   `productLine` unchanged, and the measured store reading
                   beside it. An estimate is labelled an estimate — the server
                   states no total. -->
              <p class="ask">
                <span class="k">Asked</span>
                <b class="mono">{productLine}</b>
                <span class="u">instrument-month(s), estimated</span>
                <span class="k">Held</span>
                <b class="mono">{live ? n(live.units) : storeRead.at ? n(heldInWindow) : '—'}</b>
                <span class="u"
                  >{live
                    ? `· ${n(live.rows)} bar(s), measured from /store.json`
                    : storeRead.at
                      ? `· ${n(heldBarsInWindow)} bar(s), measured from /store.json`
                      : '· measured when the store answers'}</span
                >
              </p>

              <!-- ================================ THE TWO BULK ACTIONS =====
                   One clears the selection; the other would clear the store,
                   and it cannot. Both are drawn, because a control that is
                   missing says nothing and a control that is refused says
                   exactly what stands in the way. -->
              <div class="acts">
                <button
                  class="qb"
                  type="button"
                  disabled={insCount === 0}
                  title={insCount === 0
                    ? 'Nothing is ticked, so there is nothing to clear.'
                    : `Unticks all ${n(insCount)} instrument(s). The chosen universe stays, so the list you are picking from does not disappear — and nothing is deleted from the store.`}
                  onclick={insClearAll}
                >
                  Clear all instruments
                </button>
                <!-- THE LONG GREY SENTENCE BESIDE THIS BUTTON IS ON THE BUTTON.
                     It read "Deleting is refused, not missing: the store is
                     append-only (§3 rule 8) and the server registers no delete
                     route. The button stays so the absence is visible." — every
                     word of which is now the first two sentences of the `title`
                     below. It was a paragraph of grey prose in a control panel
                     that the mockup draws as two quiet buttons and nothing else;
                     it is not deleted, it is on the control it is about.

                     THE LABEL IS THE LABEL. "— no route deletes" was the refusal
                     wedged into the button's face, which is what made this
                     control read as an error message rather than as a button. It
                     is `disabled`, which is the visible refusal; the reason is
                     one hover away and the sentence is longer than any label. -->
                <button
                  class="qb danger"
                  type="button"
                  disabled
                  title="Deleting is refused, not missing, and the button stays so the absence is visible. There is no route that deletes a bar: crates/api/src/server.rs registers no DELETE at all, and CLAUDE.md §3 rule 8 makes the store append-only — a month file is never mutated in place and never removed. Deleting bars is an operator action on the store directory itself, outside this application."
                >
                  Delete all stored bars
                </button>
              </div>
            </fieldset>

            <!-- THE LOCK NAMES ITSELF. `<fieldset disabled>` greys every control
                 above at once and says nothing about why, so a form that has
                 gone flat looks broken rather than busy — the spinner is on a
                 button far below the controls it explains. It is not a refusal
                 to fix, so it is stated once and calmly. -->
            {#if phase === 'running'}
              <!-- ONE CLAUSE, NOT THE PARAGRAPH IT WAS. `<fieldset disabled>`
                   greys every control at once and says nothing about why, so a
                   form gone flat looks broken rather than busy — but the reason
                   is one line and the rest of it is a note about how the route
                   works, which belongs on the line rather than under it. -->
              <p
                class="hint"
                title="The request is one synchronous POST already on the wire and no route amends it, so an edit now could only change what a NEXT run asks for. The outcome list below is a difference measured against the store as it stood when this one left."
              >
                <span class="tag acc">locked</span>
                Busy, not broken — every control above answers again when this run does.
              </p>
            {/if}

<!-- A GATED FORM STATES ITS BLOCKER WITHOUT BEING ASKED. The field-level
                 messages wait for a submit, because telling someone their date is
                 missing before they have opened the calendar is nagging. A retracted
                 rung is not nagging: the controls below it have just left the page,
                 and the reason has to be on screen the moment they do. -->
            {#if (showProblems || gate || staleWindow) && problems.length > 0}
              <ul class="probs panel-in" aria-live="polite">
                {#each problems as p (p.field + p.why)}
                  <li><span class="tag down">{p.field}</span><span class="msg">{p.why}</span></li>
                {/each}
              </ul>
            {/if}

            <div class="actions">
              <button class="btn primary" type="submit" disabled={phase === 'running'}>
                {#if phase === 'running'}
                  <span class="spin ring" aria-hidden="true"></span> Running…
                {:else}
                  <!-- THE VERB IS THE SOURCE KIND'S, NOT THIS BUTTON'S. A
                       broker is PULLED over the network; a folder of bought
                       files is READ off a disk. This read "archive pull" for
                       both, which is the wrong diagnostic frame printed on the
                       control that starts it. `verb` comes from /feeds.json,
                       which takes it from pull::vendor::SourceKind::verb.

                       THE NOUN IS THE SAME FIELD'S. It read
                       `isBroker ? 'broker' : 'folder'`, which called an
                       unstated kind a folder — the else branch of a two-way
                       question with three answers. -->
                  Start {isBroker ? 'broker' : isFolderFeed ? 'folder' : 'feed'}
                  {verb}
                {/if}
              </button>
              {#if phase === 'done'}
                <button class="btn ghost" type="button" onclick={reset}>Clear the result</button>
              {/if}
            </div>
          </form>
        </section>

        <!-- ================================================ THE RUN ======= -->
        <!-- IT IS NOT ON THE PAGE UNTIL THERE IS A RUN.
             Idle, the whole card said one thing: that nothing was running and
             where the progress WOULD be measured from. A panel whose only
             content is why it is empty is a panel that costs half the width of
             the form and answers nothing he asked. It comes back the instant
             `phase` leaves `idle` and stays for the whole of `done`, so the
             receipt, the facts table and the elapsed clock are never hidden —
             `reset` is the only thing that takes it off again, and that is a
             press. -->
        {#if phase !== 'idle'}
        <section class="card">
          <header class="card-h">
            <span class="pane-title">The run</span>
            <span class="spacer"></span>
            {#if phase === 'running'}
              <span class="tag acc">in flight</span>
            {:else if phase === 'done'}
              <span class="tag" class:up={receipt?.good} class:down={receipt && !receipt.good}>
                {receipt?.verdict ?? (netError ? 'no answer' : 'finished')}
              </span>
            {:else}
              <span class="tag">idle</span>
            {/if}
          </header>

            <div class="run">
              <div class="runrow">
                <span class="lbl">Elapsed</span>
                <span class="mono big">{clockOf(elapsedMs)}</span>
                <span class="hint">measured from the moment the request left this browser</span>
              </div>

              <div class="runrow">
                <span class="lbl">Store growth <i>measured</i></span>
                <span class="mono big" class:up={rowsGained > 0}>{n(rowsGained)}</span>
                <span class="hint">
                  bar(s) across {n(unitsDone)} new instrument-month(s), read from
                  <span class="mono">/store.json</span> every 5 s
                </span>
              </div>

              <!-- THE BAR IS DETERMINATE ONLY WHERE THERE IS SOMETHING TO
                   DETERMINE IT. Before the store has grown there is no
                   measurement, so the bar says "unknown" by being
                   indeterminate rather than by inching along on a timer. -->
              {#if expectedUnits > 0 && unitsDone > 0}
                <div class="meter" role="progressbar" aria-valuenow={Math.round(share * 100)} aria-valuemin="0" aria-valuemax="100" aria-label="Estimated share of the request filled">
                  <i style="width:{(share * 100).toFixed(2)}%"></i>
                </div>
                <p class="hint">
                  <b class="mono">{(share * 100).toFixed(1)}%</b> of
                  <span class="mono">{n(expectedUnits)}</span> instrument-months are now held.
                  <b class="warn">This share is an estimate.</b> The denominator is names × months
                  from your own request — the server states no total, and a name the vendor has no
                  history for will never fill.
                  <!-- BOTH SIDES OVER ONE POPULATION, AND WHAT FELL OUTSIDE IT
                       IS STILL NAMED. The numerator counted every row in these
                       months — every instrument, every rung — against a
                       denominator that counted only this request, so one
                       unrelated backfill in the window pinned this bar at
                       100.0% with half the request outstanding. `askScope` is
                       what both readings are taken through now. -->
                  {#if live && live.outside > 0}
                    <span class="mono">{n(live.outside)}</span> instrument-month(s) in these months
                    are outside this request — other instruments, or other rungs — and are counted
                    in neither half of this ratio.
                  {/if}
                </p>
              {:else if phase === 'running'}
                <div class="meter indet"><i></i></div>
                <p class="hint">
                  No growth measured yet, so there is no share to show. The bar is indeterminate on
                  purpose: a number here would be invented.
                </p>
              {/if}

              <!-- GUARDED ON `rate` ITSELF, not only on `etaSecs`. The two are
                   separate `$derived`s and `etaSecs` is `rate && …`, so today a
                   non-null eta does imply a non-null rate — but that is a
                   correlation a reader has to chase across two definitions and
                   an edit to `etaSecs` would silently break. The dereference
                   guards on the value it dereferences. -->
              {#if etaSecs !== null && rate !== null}
                <p class="hint">
                  <span class="tag info">extrapolation</span>
                  About <b class="mono">{clockOf(etaSecs * 1000)}</b> left, from a measured
                  <span class="mono">{rate.toFixed(2)}</span> instrument-months/s over the last
                  {samples.length} readings and {n(unitsLeft)} still to fill. It is a straight-line
                  projection of the recent past and nothing more.
                </p>
              {/if}

              <!-- THE GOVERNOR, AS FAR AS IT CAN HONESTLY BE SEEN — AND ONLY
                   WHILE THERE IS SOMETHING TO SEE.
                   Its fourth branch said "not observable" and then explained, on
                   the finished receipt, that no route reports the governor's
                   state. True, and about an endpoint that does not exist rather
                   than about the run he had just read the verdict of. Once the
                   POST has answered nothing here is measured, so the row is not
                   drawn. It returns for the whole of the next run, carrying the
                   three readings that ARE measured: stalled, landing, or nothing
                   yet. -->
              {#if phase === 'running'}
              <div class="gov" class:hot={stalled}>
                <span
                  class="dot live"
                  class:warn={stalled}
                  class:up={!stalled && everGrew}
                  class:acc={!stalled && !everGrew}
                ></span>
                <div>
                  {#if stalled}
                    <b class="warn">
                      {everGrew
                        ? `Nothing has landed for ${n(stalledSecs)}s`
                        : `Nothing has landed at all, ${n(stalledSecs)}s in`}
                    </b>
                    <span class="hint">
                      That is what a governor waiting for its next permit looks like from outside —
                      and also what a slow vendor, a long window split into chunks, or a credential
                      round-trip looks like. They cannot be told apart from the browser: there is no
                      status route. The request is still open, so this is not yet a hang.
                    </span>
                  {:else if everGrew}
                    <b class="up">Bars are landing</b>
                    <span class="hint">
                      Last growth {n(stalledSecs)}s ago. The governor is inside the request and
                      cannot be read directly; a store that keeps growing is the evidence that it is
                      admitting requests.
                    </span>
                  {:else}
                    <b>Nothing has landed yet</b>
                    <span class="hint">
                      {n(stalledSecs)}s in and the store has not grown. Early in a run that is
                      normal — the credential, the identity and the first chunk all come before the
                      first bar. Nothing is claimed either way until the store moves.
                    </span>
                  {/if}
                </div>
              </div>
              {/if}

              {#if pollError}
                <p class="caution">
                  <span class="tag down">watcher</span>
                  <span class="msg">
                    The progress reading failed and is not being silently retried into a frozen
                    display: {pollError}
                  </span>
                </p>
              {/if}

              {#if phase === 'running'}
                <div class="actions">
                  <button class="btn danger" type="button" onclick={stopWatching}>Stop watching</button>
                  <span class="hint">
                    This aborts the browser's request only. There is no cancel route — a pull is one
                    synchronous POST, and the server keeps going until it is done.
                  </span>
                </div>
              {/if}

              {#if aborted}
                <p class="caution">
                  <span class="tag warn">abandoned</span>
                  <span class="msg">
                    You stopped watching. The server was not told, so the run is most likely still
                    going; the outcome list below is a difference against whatever had landed when
                    you stopped. <a class="link" href="/audit">The audit record</a> is where the
                    finished run will appear.
                  </span>
                </p>
              {/if}

              {#if netError}
                <p class="caution">
                  <span class="tag down">no answer</span><span class="msg">{netError}</span>
                </p>
              {/if}

              <!-- ONE RUN, N REQUESTS, AND THE COUNT IS ON SCREEN. A run over
                   two ticked timeframes is two POSTs; showing one receipt with
                   nothing beside it would report half of what happened as if it
                   were all of it. The verdict card below shows the FIRST bad
                   one — a later success must not paint over a rung that
                   refused — and this strip carries every one of them. -->
              {#if sent.of > 1}
                <p class="hint">
                  <span class="tag acc">{n(sent.done)} of {n(sent.of)}</span>
                  One request per ticked timeframe, sent in the ladder's order.
                  {#each receipts as r, i (r.rung + i)}
                    <span class="tag" class:up={r.good} class:down={!r.good}>{r.rung}: {r.verdict}</span>
                  {/each}
                  {#if phase === 'running'}
                    <span class="mono">{sent.label}</span> is on the wire now.
                  {/if}
                </p>
              {/if}

              {#if receipt}
                <div class="verdict" class:bad={!receipt.good}>
                  <b>{receipt.verdict}{receipts.length > 1 ? ` — ${receipt.rung}` : ''}</b>
                  <span>{receipt.reason}</span>
                </div>
                <table class="kv">
                  <tbody>
                    {#each receipt.facts as f, i (f.k + i)}
                      <tr class="row-in" style="animation-delay:{Math.min(i, 14) * 12}ms">
                        <th>{f.k}</th>
                        <td class:mono={/^[\d.,\s]+$/.test(f.v)}>{f.v}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
                <details class="wire" bind:open={showRaw}>
                  <summary>The server's own receipt page</summary>
                  {#if showRaw}
                    <!-- SANDBOXED, AND WITHOUT allow-scripts. The page is read
                         above as text; this frame is only for comparing what
                         the server drew against what was read out of it. -->
                    <iframe title="Server receipt, sandboxed" sandbox="" srcdoc={receipt.raw}></iframe>
                  {/if}
                </details>
              {/if}
            </div>
        </section>
        {/if}

        <!-- ══════════════════ BELOW THE PANEL, NOT INSIDE IT ══════════════════
             THE CONTROL PANEL CARRIES CONTROLS. A caution box, a fold of wire
             bodies and a paragraph of grey prose are all TRUE and all belong to
             this form, and every one of them was standing between the operator
             and the five dropdowns he came here to set. The panel above is the
             mockup's panel now — a row of controls, a day window, two quiet
             buttons — and everything that was crowding it is here, one row
             below, at full width where a sentence can be a sentence.

             NOTHING IS DELETED. The three cautions are the same `cautions`
             array rendered by the same loop; `askGap` is the same derived
             sentence; the wire fold is byte for byte the fold that was inside
             the form, still closed by default, still one `<p>` per ticked rung.
             What changed is which side of the panel edge they sit on.

             STILL INSIDE `.cols`, spanning both tracks, so it sits under the
             form and the run card rather than becoming a third column beside
             them. -->
        <div class="belowsel">
          <!-- THE FOUR FACTORS ARE FOUR CONTROLS AND THE PRODUCT IS THEIRS.
               Where the ticked count and the reachable count differ, the
               difference is stated in words: the request cannot be narrowed and
               this is the only line that can say so. It sat in a 248px grid
               track and wrapped into a ribbon; at full width it is a sentence. -->
          {#if askGap}
            <p class="caution">
              <span class="tag warn">ticked ≠ requested</span><span class="msg">{askGap}</span>
            </p>
          {/if}

          {#each cautions as c (c)}
            <p class="caution">
              <span class="tag warn">caution</span><span class="msg">{c}</span>
            </p>
          {/each}

          <details class="wire">
            <summary>What goes on the wire</summary>
            <!-- ONE LINE PER TICKED RUNG, because one POST carries one
                 `granularity`. A single line here while two rungs were ticked
                 would be the page describing a request it does not send. -->
            {#each wireBodies as w (w.dir)}
              <p class="mono wirebody">POST /pull/spot<br />{w.body}</p>
            {/each}
            {#if wireBodies.length === 0}
              <p class="mono wirebody">
                Nothing would be sent: no timeframe is ticked, and `granularity` is not an
                optional field on a request this form builds.
              </p>
            {:else if wireBodies.length > 1}
              <p class="hint">
                {n(wireBodies.length)} requests, sent one after another in the ladder's order —
                <span class="mono">{wireBodies.map((w) => w.dir).join(', ')}</span>. They are not
                sent in parallel: <span class="mono">/pull/spot</span> is synchronous and the store
                appends, so two in flight would interleave two answers into one month file.
              </p>
            {/if}
            <!-- FIVE PROSE PARAGRAPHS LEFT THIS FOLD ON 14 AUG 2026, at the
                 operator's instruction, and what stays is the part that is
                 DATA.

                 The request bodies above are a value nothing else on this page
                 shows: the exact line this form will POST, one per ticked
                 rung, with a count beside them saying how many go out and in
                 what order.

                 What went was explanation — why `granularity` defaults to
                 1min by omission, which controls are readings rather than
                 fields, that the two dates are forwarded verbatim, and a
                 paragraph on Dhan's non-inclusive `toDate`. Each is a fact
                 about `crates/api` or `crates/pull` and belongs in those
                 crates, where it is versioned with the code it describes.
                 Restated here it was a second copy free to go stale on its
                 own — and one already had: the `toDate` paragraph gave one
                 vendor's rule as though it were the wire's, when
                 `HttpSpec::range_end` has been per-vendor since D-0113 and
                 Kite's is INCLUSIVE (docs/00-charter.md §4z). -->
          </details>
        </div>
      </div>

      <!-- ============================================== THE OUTCOMES ====== -->
      <!-- IT ARRIVES WITH THE RUN AND STAYS UNTIL THE RESULT IS CLEARED.
           Idle, this whole card was a promise: a header, a border and a
           sentence saying rows would appear here once a run had happened. The
           table itself is the point of the page and is never touched — what
           left is the placeholder standing in for it. The two branches that
           SAY something still do: "not built yet" while the second store
           reading is outstanding, and the diagnostic when a finished run
           produced no rows at all. -->
      {#if phase !== 'idle'}
      <section class="card wide">
        <header class="card-h">
          <span class="pane-title">Every instrument this request named</span>
          <span class="spacer"></span>
          {#if outcomes.length > 0}
            <span class="tag">{n(filtered.length)} shown of {n(outcomes.length)}</span>
          {/if}
        </header>

        {#if outcomes.length === 0}
          <div class="empty">
            {#if phase === 'running'}
              The outcome list is built when the run answers — it is a difference between the store
              before and the store after, and the second reading does not exist yet.
            {:else}
              No outcome rows. The universe named
              {n(members.length)} instrument(s) and nothing could be compared — most likely the
              catalogue is unavailable, which the header says.
            {/if}
          </div>
        {:else}
          <!-- ======================================= THE VERDICT STRIP =====
               AT THE HEAD OF THE TABLE, AND IT ANSWERS THE ONE QUESTION THE
               PAGE EXISTS FOR: did the window land? One sentence, the judgement
               under it, the non-zero counts as filters, the total they have to
               reach, and the one bulk action.

               EVERY NUMBER IS READ, NOT RECOMPUTED. `pills`, `countIn` and
               `landed` are arithmetic over `groups`, which is one pass over the
               same `outcomes` rows the table below draws — so a pill, the
               sentence and the Verdict column are three views of one
               classification and cannot disagree.
               ============================================================ -->
          <div class="lstrip {landed?.tone ?? ''}">
            {#if landed}
              <div class="lq">
                <b>{landed.title}</b>
                <span>{landed.why}</span>
              </div>
            {/if}

            <!-- NON-ZERO ONLY. `groups` is built from rows that exist, so a
                 bucket nobody is in has no pill — a chip reading "0 failed" is
                 a non-event given the same weight as a result, and five of them
                 bury the one that matters. Clicking the pill that is already on
                 clears the filter, which is why there is no "All" chip. -->
            <div class="lpills">
              {#each pills as g (g.group)}
                <button
                  class="lp"
                  type="button"
                  aria-pressed={pickedGroup === g.group}
                  class:on={pickedGroup === g.group}
                  title={pickedGroup === g.group
                    ? `Showing only these ${n(g.count)}. Click again to show every row.`
                    : `Click to show only these ${n(g.count)} — the same rows the Verdict column marks "${g.group}".`}
                  onclick={() => (pickedGroup = pickedGroup === g.group ? null : g.group)}
                >
                  <span class="dot" class:up={g.tone === 'up'} class:down={g.tone === 'down'} class:warn={g.tone === 'warn'} class:acc={g.tone === 'info'}></span>
                  <span class="n">{n(g.count)}</span>
                  {g.group}
                </button>
              {/each}
            </div>

            <!-- THE TOTAL THE PILLS HAVE TO REACH, and it SHOUTS when they do
                 not: a partition that does not add up is a tidy number hiding a
                 row nobody accounted for. It is counted here rather than
                 asserted, from the same two lists. -->
            {#if pills.reduce((a, g) => a + g.count, 0) === outcomes.length}
              <span class="ltot">of {n(outcomes.length)} instrument(s)</span>
            {:else}
              <span class="ltot bad">
                MISMATCH — {n(outcomes.length)} row(s), {n(pills.reduce((a, g) => a + g.count, 0))}
                accounted for
              </span>
            {/if}

            <!-- THE ONE BULK ACTION, AND IT ASKS FOR EXACTLY WHAT THE FORM
                 ABOVE ASKS FOR.
                 IT CANNOT ASK FOR LESS, AND THE LABEL SAYS SO RATHER THAN
                 IMPLYING OTHERWISE. `api::ingest::SpotRequest` carries a
                 target, a window, a feed and a granularity — there is no member
                 list on it — so a pull restricted to the N short instruments
                 has nothing to travel on. A button labelled "pull these N"
                 would be a control that quietly did something else, which is
                 the fallback CLAUDE.md §4 bans. -->
            {#if shortCount > 0}
              <div class="lact">
                <button
                  class="btn primary sm"
                  type="button"
                  disabled={rerunBlock !== null}
                  title={rerunBlock
                    ? `Cannot be pressed: ${rerunBlock}`
                    : `Sends the same POST /pull/spot again for ${dayLabel(from)} – ${dayLabel(to)}. It cannot be narrowed to the ${n(shortCount)} short instruments: SpotRequest carries no member field, so the whole target is asked for or nothing is.`}
                  onclick={() => start()}
                >
                  Pull the window again — {n(shortCount)} short
                </button>
                {#if rerunBlock}
                  <span class="hint warn">Cannot be pressed: {rerunBlock}</span>
                {/if}
              </div>
            {/if}
          </div>

          {#if hiddenFailures > 0}
            <p class="caution loud">
              <span class="tag down">truncated by the server</span>
              <span class="msg">
                This run recorded <b>{n(failedCount)}</b> failed member(s) and put
                <b>{n(namedFailures.length)}</b> reason(s) on the wire.
                <span class="mono">crates/api/src/server.rs</span> renders
                <span class="mono">done.failures.iter().take(5)</span>, so
                <b>{n(hiddenFailures)}</b> reason(s) were never sent and cannot be shown here. The
                rows below fall back to the measured store difference for those members, which says
                WHAT happened but not WHY.
              </span>
            </p>
          {/if}

          {#if pickedGroup}
            {#each groups.filter((g) => g.group === pickedGroup) as g (g.group)}
              <ul class="reasons">
                {#each [...g.reasons.entries()].sort((a, b) => b[1] - a[1]) as [why, count] (why)}
                  <li><span class="tag" class:down={g.tone === 'down'} class:warn={g.tone === 'warn'} class:up={g.tone === 'up'}>{n(count)}</span>{why}</li>
                {/each}
              </ul>
            {/each}
          {/if}

          <div class="obar">
            <input
              class="search"
              type="search"
              bind:value={q}
              placeholder="Filter by symbol — one Map probe per keystroke, never a scan"
              aria-label="Filter outcomes by symbol"
            />
          </div>

          <div class="ohead">
            <button class="sort" onclick={() => sortBy('symbol')}>
              Instrument{sortKey === 'symbol' ? (sortDir > 0 ? ' ▲' : ' ▼') : ''}
            </button>
            <!-- The order this column sorts in is a decision, not the alphabet,
                 so it is written down where the reader can find it. -->
            <button class="sort" onclick={() => sortBy('group')} title={`Most actionable first: ${GROUP_ORDER.join(' → ')}. Not alphabetical — the label is prose and prose has no order.`}>
              Outcome{sortKey === 'group' ? (sortDir > 0 ? ' ▲' : ' ▼') : ''}
            </button>
            <button class="sort num" onclick={() => sortBy('gained')}>
              Bars gained{sortKey === 'gained' ? (sortDir > 0 ? ' ▲' : ' ▼') : ''}
            </button>
            <span>Why</span>
          </div>

          <div class="vlist olist" bind:this={vEl} onscroll={onScroll}>
            {#if filtered.length === 0}
              <div class="empty">
                Nothing matches <span class="mono">{q}</span>{pickedGroup ? ` in "${pickedGroup}"` : ''}.
                {n(outcomes.length)} row(s) exist — clear the filter to see them.
              </div>
            {:else}
              <div class="vspace" style="height:{filtered.length * ROW_H}px">
                {#each vSlice as row, i (row.key)}
                  <div class="orow" style="top:{(vFirst + i) * ROW_H}px" title={row.reason}>
                    <span class="sym">{row.symbol}</span>
                    <span class="tag" class:up={row.tone === 'up'} class:down={row.tone === 'down'} class:warn={row.tone === 'warn'} class:info={row.tone === 'info'}>{row.group}</span>
                    <span class="num" class:up={row.gained > 0}>{row.gained > 0 ? `+${n(row.gained)}` : '0'}</span>
                    <span class="why">{row.reason}</span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <p class="hint foot">
            "Bars gained" is the difference between two readings of
            <span class="mono">/store.json</span> across the {n(windowMonths.length)} month file(s)
            {dayLabel(from)} – {dayLabel(to)} touches — measured, not reported by the run. The store
            reports a month file whole, so a window that asks for part of one is compared against
            all of it. A reason in the run's own words appears wherever the server sent one.
          </p>
        {/if}
      </section>
      {/if}

      <!-- ══════════════════════════ THE CENSUS ══════════════════════════
           EVERY SERIES IN THE WINDOW, AND WHAT THE STORE ACTUALLY HOLDS FOR
           IT. There was no such surface here at all: the form asked for a
           window and nothing on screen said whether that window was already
           filled, so the only way to find out was to start a pull.

           EVERY NUMBER IS MEASURED and the header of each column says its
           unit. Stored comes from /store.json, expected from the NSE calendar
           over the days INSIDE the window, in flight from /ingest/status.json,
           and out-of-reach from the same feed floor that refuses a day in the
           date control above. Nothing here is generated and nothing is
           extrapolated. -->
      <section class="census">
        <h2 class="sec">
          Every series in that window
          <span class="cnt mono">{n(censusRows.length)}</span>
          <em>{censusSub}</em>
        </h2>

        <div class="cgrid">
          <!-- THE STRIP: one verdict in a sentence, the retry judgement under
               it, the non-zero counts as filters, the total they must reach,
               and the one bulk action. -->
          <div class="lstrip {censusVerdict.tone}">
            <div class="lq">
              <b>{censusVerdict.title}</b>
              <span>{censusVerdict.why}</span>
            </div>

            {#if censusPills.length > 0}
              <div class="lpills">
                {#each censusPills as p (p.k)}
                  <button
                    class="lp"
                    type="button"
                    aria-pressed={cBucket === p.k}
                    class:on={cBucket === p.k}
                    title={(cBucket === p.k
                      ? 'Showing only these. Click again to show every series. — '
                      : `Click to show only these ${n(p.count)} — `) + VERDICT[p.k][2]}
                    onclick={() => {
                      cBucket = cBucket === p.k ? null : p.k;
                      cPage = 1;
                    }}
                  >
                    <span
                      class="dot"
                      class:up={VERDICT[p.k][0] === 'up'}
                      class:down={VERDICT[p.k][0] === 'down'}
                      class:warn={VERDICT[p.k][0] === 'warn'}
                      class:acc={VERDICT[p.k][0] === 'info'}
                    ></span>
                    <span class="n">{n(p.count)}</span>
                    {VERDICT[p.k][1]}
                  </button>
                {/each}
              </div>
            {/if}

            <!-- THE PARTITION HAS TO ADD UP AND THE SUM IS ON SCREEN. A tidy
                 number that hides a series nobody accounted for is the failure
                 §4 bans, so the mismatch shouts rather than rounding. -->
            {#if censusRows.length > 0}
              {#if censusDrawn === censusRows.length}
                <span class="ltot">of {n(censusRows.length)} series</span>
              {:else}
                <span class="ltot bad">
                  MISMATCH — {n(censusRows.length)} series, {n(censusDrawn)} accounted for
                </span>
              {/if}
            {/if}

            {#if censusShort > 0}
              <div class="lact">
                <button
                  class="btn primary sm"
                  type="button"
                  disabled={rerunBlock !== null}
                  title={rerunBlock
                    ? `Cannot be pressed: ${rerunBlock}`
                    : `Sends the ${n(wireBodies.length)} request(s) the form above describes — ${dayLabel(from)} – ${dayLabel(to)}, one per ticked timeframe — and re-reads the store after. It asks for the whole target: SpotRequest carries no member field, so it cannot be narrowed to the short series. The month files inside the window are what it fills.`}
                  onclick={() => start()}
                >
                  Pull the {n(censusShort)} month file(s) this window is short
                </button>
                {#if rerunBlock}
                  <span class="hint warn">Cannot be pressed: {rerunBlock}</span>
                {/if}
              </div>
            {/if}
          </div>

          <!-- WHEN THE READING WAS TAKEN, AND A PRESS TO TAKE IT AGAIN. A
               census over a stale store is a census that lies quietly. -->
          <div class="cbar">
            <span class="hint" class:warn={storeRead.error !== null}>
              {#if storeRead.error}
                The store could not be read, so no row below is drawn rather than every row being
                drawn as empty: {storeRead.error}
              {:else if storeRead.busy}
                Reading <span class="mono">/store.json?feed={feeds.active}</span>…
              {:else if storeRead.at}
                {n(storeRead.rows)} instrument-month row(s) read from
                <span class="mono">/store.json?feed={feeds.active}</span> at {stampLabel(
                  storeRead.at
                )}
                · {n(heldInWindow)} of them are inside this window
              {:else}
                The store has not been read yet.
              {/if}
            </span>
            <span class="hint" class:warn={pilot.error !== null}>
              {#if pilot.error}
                The sweep ladder could not be read, so no row is marked retrying and none is marked
                failed — both of those are that route's evidence, not this page's guess:
                {pilot.error}
              {:else if feedHalt}
                The sweep has HALTED for {feedName(feeds.active)}: {feedHalt}
              {:else if pilot.inFlight}
                In flight now: <span class="mono">{pilot.inFlight.instrument}</span>
                {pilot.inFlight.month} ({n(pilot.inFlight.index)} of {n(pilot.inFlight.of)})
              {:else if pilot.at}
                The sweep names nothing in flight, so no row is marked retrying.
              {:else}
                The sweep ladder has not been read.
              {/if}
            </span>
            <span class="spacer"></span>
            <button
              class="btn ghost sm"
              type="button"
              disabled={storeRead.busy || pilot.busy}
              title={storeRead.busy || pilot.busy
                ? 'A reading is already in flight. A second press would duplicate it, and /store.json rebuilds the census server-side.'
                : 'Reads /store.json and /ingest/status.json again. Every number below is one of those two answers — nothing here is cached beyond this press.'}
              onclick={() => {
                refreshStore();
                readPilot();
              }}
            >
              {storeRead.busy || pilot.busy ? 'Reading…' : 'Re-read the store'}
            </button>
          </div>

          <div class="cscroll">
            <table>
              <thead>
                <tr>
                  <th>
                    <button class="sort" type="button" onclick={() => sortCensus('sym')}>
                      <span class="hrow"
                        >Instrument{cSort.key === 'sym' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                      >
                      <span class="hsub">NSE trading symbol</span>
                    </button>
                  </th>
                  {#if segmentsReached.length > 1}
                    <th>
                      <button class="sort" type="button" onclick={() => sortCensus('seg')}>
                        <span class="hrow"
                          >Segment{cSort.key === 'seg' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                        >
                        <span class="hsub">what the route fills</span>
                      </button>
                    </th>
                  {/if}
                  {#if rungsChosen.length > 1}
                    <th>
                      <button class="sort" type="button" onclick={() => sortCensus('tf')}>
                        <span class="hrow"
                          >Timeframe{cSort.key === 'tf' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                        >
                        <span class="hsub">one bar covers</span>
                      </button>
                    </th>
                  {/if}
                  <th class="num">
                    <button
                      class="sort"
                      type="button"
                      title="Stored is counted from /store.json. Expected is the NSE sessions inside the chosen window times the rung's bars per session — weekends and the holidays this page carries excluded. A rung with no stated bars-per-session has no expectation and prints a dash."
                      onclick={() => sortCensus('got')}
                    >
                      <span class="hrow"
                        >Bars stored / expected{cSort.key === 'got'
                          ? cSort.dir > 0
                            ? ' ▲'
                            : ' ▼'
                          : ''}</span
                      >
                      <span class="hsub">bars, from the NSE calendar</span>
                    </button>
                  </th>
                  <th class="num">
                    <button class="sort" type="button" onclick={() => sortCensus('unproved')}>
                      <span class="hrow"
                        >Months unproved{cSort.key === 'unproved'
                          ? cSort.dir > 0
                            ? ' ▲'
                            : ' ▼'
                          : ''}</span
                      >
                      <span class="hsub">of {n(windowMonths.length)} in the window</span>
                    </button>
                  </th>
                  <th>
                    <button
                      class="sort"
                      type="button"
                      title={`Worst first, and the order is a claim about what you have to do: ${VORDER.map((k) => VERDICT[k][1]).join(' → ')}. Not alphabetical — the label is prose and prose has no order.`}
                      onclick={() => sortCensus('state')}
                    >
                      <span class="hrow"
                        >Verdict{cSort.key === 'state' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                      >
                      <span class="hsub">measured, not reported</span>
                    </button>
                  </th>
                  <th>
                    <button
                      class="sort"
                      type="button"
                      title="Ranks by the number the button prints: how many month files inside the window are neither settled nor out of reach."
                      onclick={() => sortCensus('act')}
                    >
                      <span class="hrow"
                        >Next step{cSort.key === 'act' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                      >
                      <span class="hsub">pull, or nothing</span>
                    </button>
                  </th>
                </tr>
              </thead>
              <tbody>
                {#if censusSlice.length === 0}
                  {@const why = censusBlocker()}
                  <tr>
                    <td colspan={censusColCount}>
                      <div class="empty">
                        <b>{why[0]}</b>
                        {why[1]}
                      </div>
                    </td>
                  </tr>
                {:else}
                  {#each censusSlice as r (r.id)}
                    <tr>
                      <td>
                        <b class="mono">{r.sym}</b>
                        <span class="kindtag">{r.kind}</span>
                      </td>
                      {#if segmentsReached.length > 1}
                        <td class="mono">{r.segLabel}</td>
                      {/if}
                      {#if rungsChosen.length > 1}
                        <td class="mono">{r.tfLabel}</td>
                      {/if}
                      <td
                        class="num mono"
                        title={r.per === null
                          ? `${r.tfLabel} has no bars-per-session this page can state, so ${n(r.got)} stored bar(s) are counted and nothing is claimed about what they should be.`
                          : `${n(r.got)} stored against ${n(r.exp)} expected — ${n(windowSessions)} NSE session(s) inside the window × ${n(r.per)} bar(s) per session, over ${n(r.months.length)} month file(s).`}
                      >
                        <span class:up={r.per !== null && r.got >= r.exp} class:warn={r.per !== null && r.got < r.exp}
                          >{n(r.got)}</span
                        >
                        / {r.per === null ? '—' : n(r.exp)}
                      </td>
                      <td
                        class="num mono"
                        class:warn={r.unproved > 0}
                        class:up={r.unproved === 0}
                        title={r.unproved > 0
                          ? `${n(r.unproved)} of the ${n(r.months.length)} month file(s) in the window are not settled: ${r.months.filter((m) => m.k !== 'ok' && m.k !== 'beyond').map((m) => `${monthLabel(m.month)} ${VERDICT[m.k][1]}`).join(', ')}.`
                          : `All ${n(r.months.length)} month file(s) in the window are settled — matched the calendar, or out of this feed's reach.`}
                      >
                        {n(r.unproved)}/{n(r.months.length)}
                      </td>
                      <td>
                        <span
                          class="tag"
                          class:up={VERDICT[r.state][0] === 'up'}
                          class:down={VERDICT[r.state][0] === 'down'}
                          class:warn={VERDICT[r.state][0] === 'warn'}
                          class:info={VERDICT[r.state][0] === 'info'}
                          title={VERDICT[r.state][2]}>{VERDICT[r.state][1]}</span
                        >
                      </td>
                      <td>
                        {#if r.unproved === 0}
                          <span
                            class="dash"
                            title={r.state === 'beyond'
                              ? VERDICT.beyond[2]
                              : 'Nothing to ask for: every month file in this window is settled.'}
                            >—</span
                          >
                        {:else}
                          {@const sp = shortSpan(r)}
                          <button
                            class="btn sm"
                            type="button"
                            disabled={phase === 'running' || problems.length > 0 || sp === null}
                            title={phase === 'running'
                              ? 'A pull is already on the wire — /pull/spot is synchronous and this page sends one at a time.'
                              : problems.length > 0
                                ? `The request above has ${n(problems.length)} thing(s) to fix first: ${problems[0].why}`
                                : `POST /pull/spot — target=${target}, vendor=${feeds.active}, granularity=${r.tf}, from=${sp?.from}, to=${sp?.to}. That is ${dayLabel(sp?.from ?? '')} – ${dayLabel(sp?.to ?? '')}: the days covering the ${n(r.unproved)} unsettled month file(s) of this series, clipped to the window you chose. It CANNOT be narrowed to ${r.sym}: api::ingest::SpotRequest carries no member field, so the server answers for the whole target — what this narrows is the window and the rung, and the outcome list below is built for this row.`}
                            onclick={() => pullRow(r)}
                          >
                            Pull {n(r.unproved)}
                          </button>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          </div>

          <!-- THE PAGER. Six controls and the count they move through. Every
               disabled one says on its face why it is disabled. -->
          <div class="pager">
            <span class="of">
              <b class="mono"
                >{censusSorted.length === 0
                  ? '0'
                  : `${n((censusPage - 1) * PAGE_SIZE + 1)}–${n(Math.min(censusPage * PAGE_SIZE, censusSorted.length))}`}</b
              >
              of {n(censusSorted.length)} series{cBucket
                ? ` marked "${VERDICT[cBucket][1]}" · ${n(censusRows.length)} in all`
                : ''}
            </span>
            <span class="pages">
              <button
                class="pg"
                type="button"
                disabled={censusPage === 1}
                title={censusPage === 1 ? 'First page — already on it' : 'First page'}
                onclick={() => (cPage = 1)}>«</button
              >
              <button
                class="pg"
                type="button"
                disabled={censusPage === 1}
                title={censusPage === 1 ? 'Previous page — this is the first' : 'Previous page'}
                onclick={() => (cPage = censusPage - 1)}>‹</button
              >
              {#each pageList(censusPage, censusPages) as it (it)}
                {#if typeof it === 'string'}
                  <span class="pg gap" title="Pages between the two shown">…</span>
                {:else}
                  <button
                    class="pg"
                    type="button"
                    class:on={it === censusPage}
                    aria-current={it === censusPage ? 'page' : undefined}
                    title={it === censusPage ? `Page ${n(it)} — you are on it` : `Page ${n(it)}`}
                    onclick={() => (cPage = it)}>{n(it)}</button
                  >
                {/if}
              {/each}
              <button
                class="pg"
                type="button"
                disabled={censusPage === censusPages}
                title={censusPage === censusPages ? 'Next page — this is the last' : 'Next page'}
                onclick={() => (cPage = censusPage + 1)}>›</button
              >
              <button
                class="pg"
                type="button"
                disabled={censusPage === censusPages}
                title={censusPage === censusPages ? 'Last page — already on it' : 'Last page'}
                onclick={() => (cPage = censusPages)}>»</button
              >
            </span>
          </div>
        </div>

        <p class="hint foot">
          Stored is <span class="mono">/store.json?feed={feeds.active}</span> — one row per
          instrument-month-timeframe, read once per feed and again after every pull. Expected is the
          NSE sessions INSIDE {windowOk ? `${dayLabel(from)} – ${dayLabel(to)}` : 'the window'},
          not the whole month file, times the rung's bars per session. A month with no row at all is
          <b>never pulled</b>, which is not a zero and is never drawn as one. <b>retrying</b> and
          <b>failed</b> are <span class="mono">/ingest/status.json</span>'s evidence — the sweep's
          own ladder — and appear only where that route names them.
        </p>
        <!-- THE CALENDAR'S OWN LIMIT, BESIDE THE NUMBER IT BOUNDS. Every
             "expected" above is only as good as the holiday table behind it,
             and that table has a first and a last day. -->
        <p class="hint foot" class:warn={!windowOk ? false : !holidaysKnownFor(from) || !holidaysKnownFor(to)}>
          The expected counts exclude weekends and the {n(HOLIDAY_COUNT)} NSE holidays this page carries,
          which cover {dayLabel(HOL_FROM)} – {dayLabel(HOL_THRU)}. Outside those dates a weekday is
          ASSUMED to be a session, so an expectation there can be one session too high and a month
          that is actually complete can read as short. That is a limit of this page's table, not a
          finding about the store.
        </p>
      </section>
    </div>
  {/if}
</div>

<!-- ===================================================================== -->
<!-- THE INSTRUMENTS RUNG.                                                  -->
<!--                                                                        -->
<!-- Present, visible, and NOT SELECTABLE — and the reason is a missing      -->
<!-- FIELD, not a missing widget. `api::ingest::SpotRequest` carries target, -->
<!-- window, feed and granularity; there is no member list on it, so a       -->
<!-- subset of these names has nothing to travel on. A picker here would     -->
<!-- take ticks and send a request that ignored them.                        -->
<!--                                                                        -->
<!-- What it DOES answer is the question the count alone cannot: which names -->
<!-- this feed cannot serve. Those are shown and marked rather than absent,  -->
<!-- because absence is indistinguishable from completeness.                 -->
<!-- ===================================================================== -->
{#snippet instrumentRung()}
  <div class="roster">
    <!-- WHAT THIS PANEL IS FOR, NOW THAT THE LIST ABOVE IS THE CONTROL.
         It used to be the whole rung: two counts and nothing selectable. The
         count of reachable names is on the control's own face, and the names
         another feed lists are DRAWN IN THE LIST ABOVE as refused rows — which
         is where a reader meets them. What is left here is the one thing
         neither of those can be: the MEASUREMENT that fills them, which costs
         one request per feed and is therefore a press. -->
    <div class="rrow">
      <span class="rk">Listed by another feed, not by this one</span>
      <span class="rv mono" class:warn={(missingHere?.length ?? 0) > 0}>
        {missingHere ? n(missingHere.length) : '—'}
      </span>
      <span class="rn">
        {#if missingHere}
          measured across {n(feeds.all.length)} feed(s) · the union of every master, which is the
          widest membership any endpoint here can state
        {:else if roster.busy}
          reading every feed's master…
        {:else}
          not measured. <span class="mono">/instruments.json</span> answers for ONE feed and there
          is no <span class="mono">/universe.json</span>, so the complete membership costs one
          request per feed and is not taken unless asked for.
        {/if}
      </span>
    </div>

    {#if !missingHere}
      <div class="ract">
        <!-- ONE CONDITION DISABLES THIS BUTTON AND THE LINE BESIDE IT NAMES
             WHICH. `rosterBlock` is the single source of both the `disabled`
             attribute and the sentence, so the two can never disagree — a
             greyed control whose stated reason has gone stale is worse than
             one that says nothing. -->
        <button
          class="btn ghost sm"
          type="button"
          onclick={measureRoster}
          disabled={rosterBlock !== null}
        >
          {roster.busy
            ? 'Reading…'
            : `Measure what ${active?.display ?? 'this feed'} is missing — ${n(feeds.all.length)} request(s)`}
        </button>
        {#if rosterBlock}
          <span class="hint warn">Cannot be measured yet: {rosterBlock}</span>
        {:else}
          <span class="hint">
            Each request rebuilds the census server-side, which is why it is a press rather than a
            page load.
          </span>
        {/if}
      </div>
    {/if}

    {#if roster.error}
      <p class="caution">
        <span class="tag down">not measured</span>
        <span class="msg">
          The union could not be read, so the count above stays a dash rather than becoming a zero
          that would read as "nothing missing": {roster.error}
        </span>
      </p>
    {/if}

    {#if missingHere && missingHere.length > 0}
      <!-- THE NAMES THEMSELVES ARE NOT LISTED TWICE. Every one of them is a
           row in the tick list above — drawn, refused, and carrying which feed
           does list it — which is where the reader is already looking. -->
      <span class="hint">
        All {n(missingHere.length)} of them are in the list above, drawn and refused, each with the
        feed that does list it on its own row. Choose that feed in the first control of the strip and they become
        selectable; nothing here is hidden and nothing is deleted from the store.
      </span>
    {/if}
  </div>
{/snippet}

<!-- ===================================================================== -->
<!-- THE FEED RUNG — FIRST IN THE ROW, AND A PEER OF THE FOUR IT GOVERNS.    -->
<!--                                                                        -->
<!-- A snippet rather than markup in place, because the strip is not the     -->
<!-- only place it has to appear: the three blank states above render it too -->
<!-- (no feed selected, an archive with nothing to ingest, and the feed list -->
<!-- failing), and each of those is exactly the state in which the operator  -->
<!-- most needs to change the feed. A control that vanishes precisely when   -->
<!-- it is needed is the dead end this snippet exists to make impossible.    -->
<!--                                                                        -->
<!-- It is `.dd`/`.ddb`/`.ddr` and NOT a `Picker`, for the reason the        -->
<!-- Universe and Instruments menus are not: a Picker row is a checkbox and  -->
<!-- the feed is a SELECTION — exactly one, never a set — and a refused feed -->
<!-- needs a row that is drawn, dead, and carrying `/feeds.json`'s own why.  -->
<!-- ===================================================================== -->
{#snippet feedRung()}
  <div class="pk">
    <!-- THE LABEL IS THE LABEL. Everything the sub-clause used to say — that
         this is the page's whole scope, what a change costs, and where the
         counts are read from — is on the button's `title`, one hover from the
         control it is about. A two-line label in one cell and a one-line label
         in the next is what broke this row's baseline. -->
    <span class="plbl">Broker feed</span>
    <div class="dd">
      <button
        class="ddb"
        type="button"
        aria-expanded={drop === 'feed'}
        title={`${active?.display ?? 'No feed selected'} · ${active?.kind_label ?? 'source kind not stated by this server'}. THE PAGE'S WHOLE SCOPE: everything below this control is this feed's answer. Every count is read from /instruments.json?feed=${feeds.active ?? ''} and every bar is pulled from this feed alone — no page in this product puts one feed's numbers beside another's, because the two are not the same instrument universe, the same session handling or the same price scale. A bar belongs to the vendor that supplied it, so changing this changes what is pulled and what is counted; it is not a different view of one thing. What this feed does not carry is not measured here.`}
        onclick={(e) => {
          e.stopPropagation();
          drop = drop === 'feed' ? null : 'feed';
        }}>{active?.display ?? (feeds.all.length ? 'Select a feed' : 'No feeds')}</button
      >
      {#if drop === 'feed'}
        <div class="ddm" role="group" aria-label="Broker feed — the page's whole scope">
          <!-- SAME DRAWER RULE AS THE UNIVERSE AND THE PICKERS. TrueData and
               GDFL are `ready: false` -- samples were supplied and the real
               archives were never bought -- so they sat dead at the TOP of this
               menu, above the feeds that work. They are folded below now, with
               /feeds.json's own reason intact on each. -->
          {#each feeds.all.filter((f) => f.ready === true) as f (f.wire)}
            {@const ready = true}
            <button
              class="ddr"
              type="button"
              class:off={!ready}
              disabled={!ready}
              aria-pressed={feeds.active === f.wire}
              title={ready
                ? `Selects ${f.display}. Every count and every bar below becomes this feed's, measured again from /instruments.json?feed=${f.wire} and /store.json — nothing is deleted and nothing is merged with the feed you are leaving.`
                : `${f.display} is refused by the server, and this is /feeds.json's own reason rather than a paraphrase of it: ${f.why ?? 'the server marked this feed not ready and stated no reason, which is itself the thing to fix.'}`}
              onclick={() => {
                feeds.active = f.wire;
                drop = null;
              }}
            >
              <span class="tk">{feeds.active === f.wire ? '✓' : ''}</span>
              <span class="nm">{f.display}</span>
              <span class="ct" class:warn={!ready}
                >{f.kind_label ?? 'kind not stated'}{ready ? '' : ' · unavailable'}</span
              >
            </button>
          {/each}
          <!-- THE PROMOTED SELECTION. An operator CAN select a not-ready feed —
               the page carries blank states for exactly that — and when he has,
               the row rides above the drawer so the menu still shows a tick
               without opening anything. -->
          {#if activeNotReady}
            <button
              class="ddr off"
              type="button"
              disabled
              aria-pressed={true}
              title={`${activeNotReady.display} is refused by the server, and this is /feeds.json's own reason rather than a paraphrase of it: ${activeNotReady.why ?? 'the server marked this feed not ready and stated no reason, which is itself the thing to fix.'}`}
            >
              <span class="tk">✓</span>
              <span class="nm">{activeNotReady.display}</span>
              <span class="ct warn"
                >{activeNotReady.kind_label ?? 'kind not stated'} · unavailable</span
              >
            </button>
          {/if}
          {#if tuckedFeeds.length > 0}
            <button
              class="ddr tuck"
              type="button"
              aria-expanded={feedOpen}
              onclick={() => (feedTuck = !feedOpen)}
            >
              <span class="tk">{feedOpen ? '▾' : '▸'}</span>
              <span class="nm">{n(tuckedFeeds.length)} feed(s) this build cannot pull</span>
              <span class="ct">{feedOpen ? 'hide' : 'show why'}</span>
            </button>
            {#if feedOpen}
              {#each tuckedFeeds as f (f.wire)}
                <button
                  class="ddr off"
                  type="button"
                  disabled
                  title={`${f.display} is refused by the server, and this is /feeds.json's own reason rather than a paraphrase of it: ${f.why ?? 'the server marked this feed not ready and stated no reason, which is itself the thing to fix.'}`}
                >
                  <span class="tk"></span>
                  <span class="nm">{f.display}</span>
                  <span class="ct warn"
                    >{f.kind_label ?? 'kind not stated'} · unavailable</span
                  >
                </button>
              {/each}
            {/if}
          {/if}
          {#if feeds.all.length === 0}
            <p class="insnone">
              The server answered <span class="mono">/feeds.json</span> with an empty list, so there
              is no vendor to select and nothing to pull. A feed appears here the day its descriptor
              row exists on the Rust side — nothing in this file names one.
            </p>
          {/if}
        </div>
      {/if}
    </div>
    <!-- THE SCOPE SENTENCE, KEPT WORD FOR WORD. It was a paragraph in a
         full-width block; it is this control's clause now, which is where every
         other fact on this strip lives. A `.pknote` CLIPS, so the whole of it is
         on the clause's own `title` as well — the page's standing rule. -->
    <span
      class="pknote"
      class:warn={!reachKnown}
      title={`everything below is this feed's answer · ${scopeNote}`}
      >everything below is this feed's answer · {scopeNote}</span
    >
  </div>
{/snippet}

<!-- ===================================================================== -->
<!-- THE SEGMENTS RUNG. Every segment the store keys on is DRAWN. Spot is    -->
<!-- offered because `/pull/spot` fills it; the two expired ones are drawn   -->
<!-- dead, in place, each carrying the three measured reasons no build can   -->
<!-- fetch it. See `SEGMENTS` for all three and where each was read.         -->
<!-- ===================================================================== -->
{#snippet segmentRung()}
  <!-- A ONE-ITEM DROPDOWN IS AN UNANSWERED QUESTION. The two expired segments
       were FILTERED OUT of this menu and summarised in a clause beside it, and
       the operator asked the only question that shape provokes: why are expired
       futures and options not here. A hidden row cannot answer it. Both are
       rows now — disabled, in the store's own order, with the short reason
       VISIBLE on the row and the long one on its `title` — which is the shape
       `$lib/Picker.svelte` documents `disabled`/`why` for and the shape the
       universe menu beside it has always used.

       `segmentsReached` is unchanged and still gates everything downstream:
       ticked AND served. A disabled row cannot be ticked, so nothing below this
       control can be reached by a segment this build cannot fill. -->
  <div class="pk">
    <span class="plbl">Segments</span>
    <!-- `tuck`: expired futures and expired options are `served: false` because
         only POST /pull/spot exists — there is no expired-contract route in
         this build at all. Two of the three rows were dead on every feed. -->
    <Picker
      tuck
      label="segments"
      summary={segmentsReached.length > 0
        ? segmentsReached.map((s) => s.label).join(', ')
        : 'No segment selected'}
      rows={SEGMENTS.map((s) => ({
        key: s.key,
        name: s.label,
        detail: s.note,
        disabled: !segServed(s),
        why: s.short,
        title: s.why
      }))}
      selected={segSet}
      onchange={(sel) => (segSet = sel)}
    />
    <span
      class="pknote"
      title={`The store keys on three segments and all three are drawn. ${segmentsUnserved
        .map((s) => `${s.label} — ${s.why}`)
        .join('\n\n')}`}
    >
      {n(segmentsReached.length)} of {n(SEGMENTS.length)} active{segmentsUnserved.length > 0
        ? ` · ${n(segmentsUnserved.length)} refused, drawn with the reason`
        : ''}
    </span>
  </div>
{/snippet}

<!-- ===================================================================== -->
<!-- THE DAY GRID. One pane: a month and a year in the header, that month's  -->
<!-- forty-two cells in the body. Six full weeks always, so the panel never  -->
<!-- changes height under the pointer as it is paged.                        -->
<!--                                                                         -->
<!-- A day that CANNOT be taken is drawn unable to take it and says which     -->
<!-- bound refused; a day that holds no session is drawn faint and says why.  -->
<!-- Those are two different facts and they get two different marks — a       -->
<!-- Saturday is not a refusal, it is a day with nothing in it.               -->
<!--                                                                         -->
<!-- One panel exists at a time because `cal.field` is a single value, so two -->
<!-- can never overlap each other.                                           -->
<!-- ===================================================================== -->
{#snippet dayGrid()}
  <div
    class="cal panel-in"
    class:up={cal.up}
    role="dialog"
    aria-label="Choose a day"
    bind:this={calEl}
    onkeydown={onCalKey}
    tabindex="-1"
  >
    <div class="cal-h">
      <button
        type="button"
        class="nav"
        aria-label="Previous month"
        disabled={calView <= minMonth}
        onclick={() => pickView(addMonths(calView, -1))}>‹</button
      >
      <!-- TWO SELECTS, NOT A SECOND PANE AND NOT ARROWS ALONE. Paging a month
           at a time is how a reader ends up thirty clicks from where they meant
           to be; these two reach any day in the span in two picks, and they
           carry the same bounds the grid does. -->
      <select
        class="calsel"
        aria-label="Month"
        value={String(calMonthNo)}
        onchange={(e) => pickView(`${pad(calYear, 4)}-${pad(Number(e.currentTarget.value))}`)}
      >
        {#each MON as label, i (label)}
          <option value={String(i + 1)} disabled={`${pad(calYear, 4)}-${pad(i + 1)}` < minMonth || `${pad(calYear, 4)}-${pad(i + 1)}` > maxMonth}>
            {label}
          </option>
        {/each}
      </select>
      <select
        class="calsel"
        aria-label="Year"
        value={String(calYear)}
        onchange={(e) => pickView(`${pad(Number(e.currentTarget.value), 4)}-${pad(calMonthNo)}`)}
      >
        {#each calYears as y (y)}
          <option value={String(y)}>{y}</option>
        {/each}
      </select>
      <button
        type="button"
        class="nav"
        aria-label="Next month"
        disabled={calView >= maxMonth}
        onclick={() => pickView(addMonths(calView, 1))}>›</button
      >
    </div>

    <div class="calwk" aria-hidden="true">
      {#each WEEK as w (w)}<span>{w}</span>{/each}
    </div>

    <div class="dgrid" role="group" aria-label="Days of {monthLabel(calView)}">
      {#each calDays as d (d)}
        {@const why = dayBlock(d)}
        <!-- A REFUSED DAY CARRIES ITS OWN REASON, and a takeable one carries
             what it is. `title` is never empty: Svelte drops the attribute when
             the expression is null, and a tooltip that says nothing is worse
             than none. -->
        <button
          type="button"
          class="cday"
          data-day={d}
          disabled={why !== null}
          title={why ?? dayNote(d)}
          tabindex={d === cal.cursor ? 0 : -1}
          aria-pressed={d === (cal.field === 'from' ? fromDay : toDay)}
          class:picked={d === (cal.field === 'from' ? fromDay : toDay)}
          class:cursor={d === cal.cursor}
          class:oth={d.slice(0, 7) !== calView}
          class:nos={noSessionWhy(d) !== null}
          class:now={d === maxDay}
          onclick={() => commit(d)}
        >
          {Number(d.slice(8))}
        </button>
      {/each}
    </div>

    <!-- THE BOUNDS THE STRUCK-THROUGH DAYS ARE ENFORCING, WRITTEN OUT.
         `title` on a disabled control is unreliable — Chrome suppresses
         tooltips on them — so the rule cannot live only there. It is one line,
         always on screen while the grid is, and it reads the same `minDay`,
         `feedFloor` and `maxDay` the grid and the refusals do rather than
         restating a number that could drift from them. -->
    <p class="calnote">
      {dayLabel(feedFloor.at && feedFloor.at > minDay ? feedFloor.at : minDay)} – {dayLabel(maxDay)}
      can be picked. Faint days hold no session; struck-through days are outside what this field may
      take.
      {#if feedFloor.at && feedFloor.at > minDay}
        <br />The floor is the feed's, not this page's: {floorSentence} Pick a feed or a timeframe
        that reaches further back and the earlier days open up.
      {/if}
      <!-- A FOLDER FEED HAS NO FLOOR TO PRINT, AND THIS IS WHAT IT HAS INSTEAD.
           Not a rule about how far back a vendor will answer — there is no
           vendor in this sentence — but the span of the files actually in the
           folder, read from it, with the path beside it. The grid does NOT
           strike days outside it: those days are askable and will simply come
           back with nothing, and striking them would claim a refusal nobody
           made. -->
      {#if folderReachSentence}
        <br />No floor applies — {active?.display} is a folder of bought files, so its range is
        whatever is in it: {folderReachSentence}. Days outside it can still be picked; they hold no file,
        so they come back empty rather than refused.
      {/if}
      {#if nothingInView}
        <span class="far">
          No day in {monthLabel(calView)} can be picked here.
          {#if feedFloor.at && `${calView}-01` < feedFloor.at}
            {floorSentence} Looking is allowed — this month is drawn so the refusal can be read on it.
          {:else}
            Every day in it is outside {dayLabel(minDay)} – {dayLabel(maxDay)}.
          {/if}
        </span>
      {/if}
    </p>
    <div class="cal-f">
      <button type="button" class="btn ghost sm" onclick={() => commit(maxDay)}>
        {dayLabel(maxDay)}
      </button>
      <span class="spacer"></span>
      <button type="button" class="btn ghost sm" onclick={() => shutCal()}>Close</button>
    </div>
  </div>
{/snippet}

<style>
  /* Everything below is built from the tokens in $lib/theme.css. No colour,
     radius, duration or step is spelled twice — a hex code here would be a
     second theme that the toggle does not reach. */

  /* ---- density ----
     DENSE ROWS, GENEROUS SPACING BETWEEN GROUPS. The page is read at a glance
     by someone deciding whether to spend thirty-two minutes, so the gaps that
     separate one QUESTION from the next stay wide (`--s6`) while the gaps
     inside one answer tighten (`--s5`, `--s3`). Every step is a token; none is
     a number typed here. */
  .body {
    padding: var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s5);
  }
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 5fr) minmax(0, 4fr);
    gap: var(--s5);
    align-items: start;
  }
  /* The run card is only drawn once there is a run, and a two-column track with
     one card in it is the removed panel's hole still holding its width. */
  .cols.solo {
    grid-template-columns: minmax(0, 1fr);
  }
  @media (max-width: 1100px) {
    .cols {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* NO `overflow: hidden` HERE. It squared off the header's top corners neatly
     and CLIPPED THE CALENDAR: the panel is taller than the space left under the
     date row, so the last week of the month and the whole footer were cut off
     by the card edge. The header rounds its own two corners instead, which
     costs one line and does not eat a popover. */
  .card {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    box-shadow: var(--e1);
    min-width: 0;
  }
  .card-h {
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: var(--s4) var(--s5);
    background: var(--bg-2);
    border-bottom: 1px solid var(--line);
    border-radius: var(--r4) var(--r4) 0 0;
  }
  .spacer {
    flex: 1;
  }

  /* ---- the form ---- */
  /* GENEROUS BETWEEN GROUPS, DENSE INSIDE ONE. The strip, the wire fold and
     the press are three different questions and get the wide gap; the rows
     inside the strip get the tight one. */
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--s6);
    padding: var(--s5);
  }
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    /* THE RUNGS ARE THE GROUPS. One question per rung, so this is the one gap
       on the page that stays wide — tightening it runs the universe row into
       the instruments panel and the cascade stops reading as a cascade. */
    gap: var(--s6);
    min-width: 0;
  }
  fieldset:disabled {
    opacity: 0.55;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    min-width: 0;
  }
  .lbl {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .lbl i {
    font-style: normal;
    /* --faint, not --n7. `--n7` is theme.css's "hairline, strong / disabled
       edge" -- a BORDER step, 1.57:1 as text. The seven --n0..--n7 steps are
       surfaces and rules; the four text tokens are --ink, --ink-2, --dim and
       --faint. */
    color: var(--faint);
    letter-spacing: 0;
    text-transform: none;
  }
  .hint {
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--dim);
  }
  .hint.foot {
    padding: var(--s4) var(--s5) var(--s5);
    border-top: 1px solid var(--line-soft);
  }
  /* A `.hint` CARRYING A TONE CLASS HAS TO TAKE THE TONE. The scoped `.hint`
     rule above outranks the global `.warn` colour on specificity, so without
     this the sentence naming why a control is refused rendered in exactly the
     same grey as the neutral hint it replaces — the tone said nothing, and the
     only cue that the line was a refusal was reading it. Amber and not red on
     purpose: every branch of `rosterBlock` is a "not yet", not a failure. */
  .hint.warn {
    color: var(--warn);
  }


  /* ======================= THE CONTROL STRIP =========================
     ONE PANEL, AND THE CONTROLS STAND SIDE BY SIDE IN IT.

     What was here before was a column: a row of text-link universe tabs, a
     bordered box of instrument rows, then two full-width stacked dropdowns,
     with four tiles above the lot and three explanatory paragraphs between
     them. Five decisions read as five documents.

     The shape below is one strip. Every rung is the same object — a caps
     label, one dropdown button, one clause under it — laid out on an
     auto-fitting track so four of them sit across at console width and fold to
     two and then one as the panel narrows. Dense rows, wide gaps BETWEEN
     groups, monospace wherever a value lives, and no paragraph anywhere: a
     fact that matters is the control's `title` or its one clause.
     ================================================================== */

  /* THE THREE ALIASES `$lib/Picker.svelte` ASKS FOR AND `theme.css` DOES NOT
     DEFINE. Picker reads `--raise`, `--panel2` and `--accdeep` with hardcoded
     dark fallbacks, so its menu ground is a literal that the light theme never
     reaches and that nothing in this strip could match. Custom properties
     inherit, so naming them here points Picker's own menus at the real ramp
     and makes them the same object as the menus below — without touching
     another file and without a hex code on this page. */
  .sel {
    --raise: var(--bg-2);
    --panel2: var(--panel-2);
    --accdeep: var(--acc-soft);

    background: linear-gradient(180deg, var(--panel-2), var(--panel));
    border: 1px solid var(--line);
    border-radius: var(--r4);
    box-shadow: var(--e1);
    min-width: 0;
  }

  /* auto-fit, not a fixed four: the panel is half the width while a run card
     is beside it, and four 248px tracks cannot fit there. The track floor is
     the width at which a monospace instrument name and its caret stop
     colliding.

     ══════════ WHY THIS ROW WAS NOT A ROW, AND WHAT ACTUALLY BROKE IT ══════════

     `align-items: end` was the cause, and it is worth naming exactly, because
     the symptom pointed at the wrong control. Bottom-aligning grid items makes
     every cell's LAST pixel share a line. The cells do not have the same
     content below their control: Universe carries one `.pknote`; Timeframe
     carries up to four (the feed's granularity floor, the dropped-rung notice,
     the ticked/refused tally, and the missing-`served` warning); Days to pull
     carries a two-field `.dates` block and its own note. Align the bottoms of
     cells with two, five and seven lines of tail and their TOPS land three
     different places — so Timeframe's label floated far above Universe's, and
     Days to pull's did the same. Nothing was wrong with Timeframe. It simply
     had the most to say.

     Two changes, and they are the cause rather than a nudge:

       1. `align-items: start`. The tops share a line, so every label starts at
          the same y and every control sits directly under its own label. A
          longer tail now hangs BELOW its cell, which is where a tail belongs
          and where it cannot move anything above it.
       2. One-line labels (see `.plbl`). Top-alignment alone is not enough: a
          label that wraps to two lines in one cell and one line in the next
          pushes that cell's control down by a line, and the row breaks again
          one level lower. Every sub-clause moved to the control's `title` —
          §4's own remedy — and what is left is a single short noun phrase that
          is held to one line by rule, not by hope.

     A NUDGE WOULD NOT HAVE HELD. A margin tuned against today's four notes is
     wrong the moment a fifth appears, and the fifth is a `{#if}` away in half
     these cells. */
  .pickers {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(248px, 1fr));
    gap: var(--s6) var(--s5);
    align-items: start;
    border: 0;
    margin: 0;
    padding: 0;
    min-width: 0;
  }
  .pk {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-width: 0;
  }
  /* THE DAY WINDOW STARTS ITS OWN ROW, WHICH IS WHAT THE MOCKUP DRAWS: the
     dropdowns across the top, the two dates below them. `1 / -1` rather than
     `span 2` does that at EVERY track count — a spanning item is placed on the
     first line that fits it, so `span 2` let it finish a row of dropdowns
     whenever an even number of tracks left a gap, and the row it finished was
     the row it then broke. Full width also removes the old hazard the media
     query existed for: `span 2` in a one-track grid grew an implicit second
     column and pushed the panel past its own box, and `1 / -1` cannot, because
     `-1` is whatever the last line happens to be.

     `.dates` is capped rather than stretched. Two date fields spread across
     1,200px would read as the panel's most important control, and they are its
     last one. */
  .pk.wide2 {
    grid-column: 1 / -1;
  }
  .pk.wide2 .dates {
    max-width: 560px;
  }
  /* ONE LINE, ALWAYS, AND THAT IS THE SECOND HALF OF THE ROW FIX. A label is a
     short noun phrase — Broker feed, Universe, Instruments in that universe,
     Segments, Timeframe, Days to pull — and each one used to carry an italic
     sub-clause after it ("bar lengths — the granularity field", "ticked — this
     is what is counted"). At a 248px track those wrapped, and they wrapped to
     DIFFERENT heights per cell, which moves that cell's control down a line and
     breaks the baseline the row is supposed to have. Every one of those clauses
     is now the first sentence of its control's `title`, which is where §4 sends
     a fact that does not fit; the label is held to one line here so the rule
     cannot be broken by a longer word later. */
  .plbl {
    font-size: var(--fs-xs);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  /* THE FEED PICKER INSIDE A BLANK STATE. The three refusals above the strip
     render the same `feedRung` snippet, and a `.pk` is a flex column that would
     take the whole width of an empty page. Clamped to one track's worth so it
     reads as the control it is in the strip. */
  .blankpick {
    max-width: 320px;
    margin-top: var(--s5);
  }
  /* EVERYTHING THAT WAS CROWDING THE PANEL, ONE ROW UNDER IT. Both tracks, so
     it is a band beneath the form and the run card and never a third column. */
  .belowsel {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    min-width: 0;
  }
  /* ONE LINE, AND IT CLIPS RATHER THAN WRAPS. This is where a fact that used
     to be a paragraph now lives, and a clause that can grow to three lines
     would reflow the whole strip the moment a feed answered slowly. The full
     sentence is on the control's `title`. */
  .pknote {
    font-size: var(--fs-xs);
    color: var(--faint);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pknote.warn {
    color: var(--warn);
  }
  /* A `pknote` is a one-line tail by default — nowrap, clipped, ellipsed. A
     SENTENCE put in one is a sentence with its end cut off, which is the same
     defect as an ellipsed refusal. The vendor floor and the missing-`served`
     notice are sentences, so they wrap. */
  .pknote.wrap {
    white-space: normal;
    overflow: visible;
    text-overflow: clip;
    line-height: var(--lh-base);
  }
  /* THE RESOLVED FOLDER, WHICH IS A PATH AND THEREFORE BREAKS ANYWHERE.
     `overflow-wrap: anywhere` rather than `break-word`: a path has no spaces to
     break at, so `break-word` would leave it overflowing its column and push
     the strip wider than the viewport. It is a button because it DOES
     something — it fills the field beside it — and it is styled flat so it
     reads as the path it is rather than as a second submit control. */
  .pathbtn {
    font: inherit;
    font-size: inherit;
    padding: 0;
    border: 0;
    background: none;
    color: var(--acc);
    cursor: pointer;
    text-align: left;
    overflow-wrap: anywhere;
    text-decoration: underline dotted;
    text-underline-offset: 2px;
  }
  .pathbtn:hover {
    text-decoration: underline solid;
  }

  /* ---- the dropdown, and it is the SAME OBJECT `$lib/Picker.svelte` draws ----
     Segments and Timeframe are Pickers; Feed, Universe and Instruments cannot
     be, because a Picker row is a checkbox and these three need a row that is
     drawn, disabled and carries its own refusal. Every measurement below is
     Picker's, so the five read as one control. */
  .dd {
    position: relative;
    min-width: 0;
  }
  /* ONE HEIGHT FOR EVERY BUTTON IN THE STRIP, INCLUDING THE TWO THAT ARE NOT
     OURS. `$lib/Picker.svelte`'s `.pbtn` already agrees with `.ddb` on every
     metric — 15px semibold mono, 11px/14px padding, 9px radius — but it does
     not clip, so a long summary wraps to a second line and that ONE control
     becomes taller than its four neighbours. That is the row broken again, by
     a string rather than by a rule. It was measured on a summary the timeframe
     control can no longer produce — the ladder is three rungs now — and the
     clip stays because it is a RULE about this strip, not a reaction to one
     string: Instruments and Universe reach the same width on a long name.
     Picker is shared and is not edited from here; the clip is applied from
     this page, to Pickers inside this strip only. */
  .pk :global(.pbtn) {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ddb {
    appearance: none;
    width: 100%;
    text-align: left;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 9px;
    color: var(--ink);
    font: inherit;
    font-size: 15px;
    font-weight: var(--w-semi);
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    padding: 11px 36px 11px 14px;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background-image: linear-gradient(45deg, transparent 50%, var(--acc) 50%),
      linear-gradient(135deg, var(--acc) 50%, transparent 50%);
    background-position: calc(100% - 17px) 55%, calc(100% - 12px) 55%;
    background-size: 5px 5px, 5px 5px;
    background-repeat: no-repeat;
  }
  .ddb:hover:not(:disabled) {
    border-color: var(--dim);
  }
  .ddb:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: 2px;
  }
  .ddb:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .ddm {
    position: absolute;
    left: 0;
    top: calc(100% + 7px);
    z-index: 40;
    width: max-content;
    min-width: 100%;
    max-width: min(92vw, 560px);
    max-height: 380px;
    overflow-y: auto;
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 5px;
    box-shadow: var(--e3);
  }
  /* The census is a panel of rows and a press, not a list of names, so it
     needs the room a two-column row costs. */
  .ddm.wide {
    min-width: 420px;
    padding: var(--s5);
  }
  .ddr {
    display: flex;
    align-items: center;
    gap: var(--s5);
    width: 100%;
    appearance: none;
    border: 0;
    background: none;
    text-align: left;
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-md);
    padding: 9px 12px;
    min-height: 38px;
    border-radius: 7px;
    cursor: pointer;
  }
  .ddr:hover:not(:disabled) {
    background: var(--panel2);
  }
  .ddr:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: -2px;
  }
  /* Drawn and refused, never absent: a row missing from the list reads as a
     set that does not exist rather than one this route cannot spell. The whole
     reason is on the row's own `title`. */
  .ddr.off {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .ddr .tk {
    flex: 0 0 12px;
    color: var(--acc);
    font-weight: var(--w-bold);
  }
  .ddr .nm {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    font-weight: var(--w-semi);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ddr .ct {
    flex: 0 0 auto;
    max-width: 220px;
    text-align: right;
    color: var(--dim);
    font-size: var(--fs-base);
    font-family: var(--mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ddr .ct.warn {
    color: var(--warn);
  }
  .ddm hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 5px 8px;
  }

  /* ---- the two days ----
     A flex pair rather than a 1fr/1fr grid, so that when the rung loses its
     second track the two cells wrap instead of being squeezed to half a
     monospace date each. */
  /* `flex-start`, FOR THE REASON `.pickers` IS. Bottom-aligning two cells makes
     their labels move apart the instant one of them grows — the `.bad` state
     and the calendar are both per-cell — and FROM DATE / TO DATE sitting at two
     heights is the same broken row one level down. Their tops share a line now,
     so the two labels share a baseline and the two fields share a top edge
     whatever either cell is doing.

     THE FLOOR IS WHAT MAKES THE WRAP CLEAN. `flex: 1 1 0` with `min-width: 0`
     let both cells shrink without limit, so at a narrow width they went on
     shrinking side by side until "02 Sep 2024" no longer fit its own box
     instead of dropping to a second line. A basis wide enough for the text and
     the ▦ makes the second cell wrap whole. */
  .dates {
    display: flex;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: var(--s5);
  }
  .dcell {
    flex: 1 1 190px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .dlbl {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    white-space: nowrap;
  }
  /* THE FIELD IS THE SAME HEIGHT AND THE SAME FACE AS THE DROPDOWN BUTTONS
     BESIDE IT. At the old 13px/7px it sat two pixels short of them and the
     strip read as two rows of different controls. Two class names deep so it
     beats the base `.din` wherever that rule sits in the file. */
  .dates .din {
    font-size: 15px;
    padding: 11px 13px;
    border-radius: 9px;
  }
  .dates .dbtn {
    font-size: 15px;
    padding: 11px 10px;
    border-radius: 9px;
  }

  /* ---- the size of the ask, in one line ----
     Everything the four deleted tiles carried that is not already on a
     control: the product with its factors, and the measured store reading
     beside it. `productLine` verbatim, so the factors still multiply to the
     total printed next to them in every state. */
  .ask {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--s2) var(--s4);
    margin: 0;
    padding-top: var(--s5);
    border-top: 1px solid var(--line);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .ask .k {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--acc);
  }
  .ask b {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
    color: var(--ink);
  }

  /* The submit rung closes the panel the way the strip's own rungs are
     separated — a rule above it and the press on the right. */
  .form > .actions {
    justify-content: flex-end;
    padding-top: var(--s5);
    border-top: 1px solid var(--line);
  }

  /* The instruments rung: three facts and their provenance, never a control.
     `.rv` is the number, and it is a dash whenever nothing counted it. */
  /* The census. It is the panel the Instruments control opens, so it draws no
     border and no ground of its own — `.ddm.wide` is already both, and a box
     inside a box reads as two panels arguing about which one is the answer. */
  .roster {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    min-width: 0;
  }
  .rrow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: baseline;
    gap: var(--s2) var(--s4);
  }
  .rk {
    font-size: var(--fs-sm);
    color: var(--ink);
  }
  .rv {
    font-size: var(--fs-base);
    font-weight: var(--w-bold);
    color: var(--dim);
  }
  .rn {
    grid-column: 1 / -1;
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--dim);
  }
  .ract {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s3) var(--s4);
  }
  /* `.miss` and its three children are gone with the second list they styled:
     the names another feed lists are drawn in the tick list itself now, as
     refused rows, which is where the reader is already looking. */

  /* ---- the month fields and their panel ----
     The old day button and its three-pane drill-down went with the unit. What
     replaces it is the shape the design settled on: a monospace text field that
     reads and writes `Sep 2024`, a ▦ that opens the grid, and the grid itself —
     one year of months, the year a <select> in its header. */
  .field {
    position: relative;
  }
  .dwrap {
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-width: 0;
  }
  /* TYPEABLE, AND MONOSPACE SO THE TWO FIELDS ALIGN. `Sep 2024` and `Dec 2024`
     are the same width in the mono face and are not in the sans one, which is
     what made a From/To pair look like two different controls. */
  .din {
    flex: 1 1 auto;
    min-width: 0;
    appearance: none;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r);
    color: var(--ink);
    font: inherit;
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-base);
    font-weight: var(--w-semi);
    padding: 7px var(--s5);
    outline: none;
  }
  .din:focus {
    border-color: var(--acc);
    box-shadow: 0 0 0 3px var(--acc-soft);
  }
  .din::placeholder {
    color: var(--faint);
    font-weight: var(--w-mid);
  }
  /* A STRING THIS PAGE COULD NOT READ LOOKS UNREAD. The refusal quoting it is
     in the block below the form; this is the field saying which one it is
     about, so the two are never hunted for separately. */
  .din[aria-invalid='true'] {
    color: var(--down);
    border-color: var(--down);
  }
  .dbtn {
    flex: 0 0 auto;
    appearance: none;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--dim);
    font: inherit;
    font-size: var(--fs-base);
    line-height: 1;
    padding: 8px var(--s4);
    border-radius: var(--r);
    cursor: pointer;
  }
  .dbtn:hover {
    color: var(--acc);
    border-color: var(--acc);
  }
  .dbtn[aria-expanded='true'] {
    color: var(--acc);
    border-color: var(--acc);
    box-shadow: 0 0 0 3px var(--acc-soft);
  }
  /* A REFUSED FIELD LOOKS REFUSED WHEREVER IT SITS. `.field` is the day cell
     inside the strip; `.pk` is the archive folder rung, which is a whole
     column of its own. Both carry `bad` from the same `problemFor` set. */
  .field.bad .din,
  .pk.bad .din {
    border-color: var(--down);
    background: var(--down-soft);
  }

  .cal {
    position: absolute;
    z-index: 40;
    top: calc(100% + var(--s2));
    left: 0;
    /* Seven columns of tabular digits plus the panel's own padding. Narrower
       and the 30th of a month wraps its own column. */
    width: 272px;
    padding: var(--s4);
    background: var(--panel);
    border: 1px solid var(--line-hard);
    border-radius: var(--r4);
    box-shadow: var(--e3);
  }
  .cal.up {
    top: auto;
    bottom: calc(100% + var(--s2));
  }
  /* THE RIGHT-HAND CELL HANGS ITS PANEL FROM THE RIGHT EDGE. Left-anchored, the
     To panel ran past the card and clipped its own footer. */
  .field.right .cal {
    left: auto;
    right: 0;
  }
  .cal-h {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-bottom: var(--s4);
  }
  .cal-h .nav {
    font: inherit;
    font-size: var(--fs-md);
    line-height: 1;
    color: var(--dim);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    background: var(--panel);
    cursor: pointer;
    width: 26px;
    height: 26px;
    flex: 0 0 auto;
  }
  .cal-h .nav:hover:not(:disabled) {
    color: var(--acc);
    border-color: var(--acc);
  }
  .cal-h .nav:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  .calsel {
    flex: 1 1 auto;
    min-width: 0;
    appearance: none;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    color: var(--ink);
    font: inherit;
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-sm);
    font-weight: var(--w-semi);
    text-align: center;
    height: 26px;
    padding: 0 var(--s4);
    cursor: pointer;
    outline: none;
  }
  .calsel:focus {
    border-color: var(--acc);
    box-shadow: 0 0 0 3px var(--acc-soft);
  }
  /* SEVEN COLUMNS, MONDAY-FIRST, and the weekday strip shares the grid so the
     two can never fall out of step. */
  .calwk,
  .dgrid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: var(--s1);
  }
  .calwk span {
    text-align: center;
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    /* --dim. This row was 1.57:1 AND its container carries aria-hidden="true",
       so the weekday names reached neither the eye nor a screen reader. */
    color: var(--dim);
    padding: var(--s1) 0 var(--s2);
  }
  .cday {
    font: inherit;
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    height: 28px;
    border: 1px solid transparent;
    border-radius: var(--r2);
    background: none;
    color: var(--ink);
    cursor: pointer;
  }
  .cday:hover:not(:disabled) {
    background: var(--panel-2);
    color: var(--acc);
  }
  /* A NEIGHBOURING MONTH'S DAY IS STILL TAKEABLE — it is drawn quiet rather
     than hidden, because a grid with holes in it reads as broken. */
  .cday.oth {
    color: var(--faint);
  }
  /* NO SESSION IS NOT A REFUSAL. Weekends and NSE holidays hold no bars, so
     they are drawn light — a different mark from the line-through below, which
     means a bound refused the day. */
  .cday.nos {
    /* --dim, and the distinction is now visible instead of promised.
       `dayBlock()` never tests sessions, so these days are ENABLED and
       clickable -- the WCAG exemption for inactive controls does not apply, and
       at 1.57:1 the caption below ("Faint days hold no session") told the
       operator to read something he could not see. Three ranks now: a session
       day is --ink, a no-session day is --dim and clickable, an out-of-bounds
       day is --faint with a line-through. */
    color: var(--dim);
    font-weight: var(--w-mid);
  }
  .cday:disabled {
    /* --faint, which is now AA in its own right. Genuinely inactive, and the
       line-through remains the primary mark rather than the colour. */
    color: var(--faint);
    cursor: not-allowed;
    text-decoration: line-through;
    text-decoration-color: var(--down);
  }
  .cday.now {
    box-shadow: inset 0 0 0 1px var(--line-hard);
  }
  .cday.cursor {
    border-color: var(--acc);
  }
  .cday.picked:not(:disabled) {
    background: var(--acc);
    border-color: var(--acc);
    color: var(--on-acc);
    font-weight: var(--w-bold);
  }
  .calnote {
    margin: var(--s4) 0 0;
    font-size: var(--fs-micro);
    line-height: var(--lh-base);
    color: var(--faint);
  }
  /* The reader navigated somewhere this field cannot take a day from. That is a
     legitimate place to be and it gets an answer in words, on its own line — not
     forty-two identical tooltips. */
  .calnote .far {
    display: block;
    margin-top: var(--s2);
    color: var(--warn);
  }
  .cal-f {
    display: flex;
    align-items: center;
    gap: var(--s3);
    margin-top: var(--s4);
    padding-top: var(--s4);
    border-top: 1px solid var(--line-soft);
  }
  .btn.sm {
    padding: var(--s2) var(--s4);
    font-size: var(--fs-xs);
  }

  /* ---- validation and cautions ---- */
  .probs {
    list-style: none;
    margin: 0;
    padding: var(--s4) var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    background: var(--down-soft);
    border: 1px solid var(--down);
    border-radius: var(--r3);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
  }
  .probs li {
    display: flex;
    gap: var(--s4);
    align-items: baseline;
  }
  .caution {
    display: flex;
    gap: var(--s4);
    align-items: baseline;
    margin: 0;
    padding: var(--s4) var(--s5);
    background: var(--warn-soft);
    border-left: 2px solid var(--warn);
    border-radius: var(--r2);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--ink-2);
  }
  .caution.loud {
    background: var(--down-soft);
    border-left-color: var(--down);
    margin: var(--s5) var(--s5) 0;
  }
  /* Inside a picker column the notice has to line up with the control it is
     about. `.loud`'s inset is for a caution sitting in a card that already has
     its own padding; here it would leave the message indented from the button
     that caused it. */
  .caution.drop {
    margin: 0;
    white-space: normal;
  }
  /* THE MESSAGE IS ONE FLEX ITEM, NOT SEVEN. Without this wrapper every inline
     `<b>` and `<span class="mono">` inside a `.caution` became a flex item of
     its own, and the truncation banner laid itself out as a row of words with
     ragged gaps between them instead of a sentence. */
  .probs .msg,
  .caution .msg {
    flex: 1;
    min-width: 0;
  }

  .wire {
    border: 1px solid var(--line);
    border-radius: var(--r3);
    padding: var(--s4) var(--s5);
    background: var(--well);
  }
  .wire summary {
    cursor: pointer;
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    color: var(--dim);
  }
  .wirebody {
    margin: var(--s4) 0 var(--s3);
    font-size: var(--fs-xs);
    line-height: 1.6;
    word-break: break-all;
    color: var(--ink);
  }
  .wire iframe {
    width: 100%;
    height: 55vh;
    margin-top: var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    background: var(--n2);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex-wrap: wrap;
  }
  .ring {
    width: 12px;
    height: 12px;
    border: 2px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    display: inline-block;
  }

  /* ---- the run ---- */
  .run {
    display: flex;
    flex-direction: column;
    gap: var(--s5);
    padding: var(--s6) var(--s5);
  }
  .runrow {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .big {
    font-size: var(--fs-xl);
    font-weight: var(--w-bold);
    letter-spacing: -0.5px;
    line-height: var(--lh-tight);
  }
  .meter {
    position: relative;
    height: 6px;
    border-radius: var(--r-full);
    background: var(--well);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    background: var(--acc);
    border-radius: var(--r-full);
  }
  .meter.indet i {
    position: absolute;
    inset: 0;
    width: 34%;
    background: linear-gradient(90deg, transparent, var(--acc), transparent);
  }
  .gov {
    display: flex;
    gap: var(--s4);
    align-items: flex-start;
    padding: var(--s4) var(--s5);
    border: 1px dashed var(--line-hard);
    border-radius: var(--r3);
    background: var(--bg-2);
  }
  .gov.hot {
    border-color: var(--warn);
    background: var(--warn-soft);
  }
  .gov .dot {
    margin-top: 5px;
  }
  .gov b {
    display: block;
    font-size: var(--fs-sm);
  }

  .verdict {
    padding: var(--s5);
    border-radius: var(--r3);
    background: var(--up-soft);
    border-left: 3px solid var(--up);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
  }
  .verdict.bad {
    background: var(--down-soft);
    border-left-color: var(--down);
  }
  .verdict b {
    display: block;
    font-size: var(--fs-sm);
    letter-spacing: var(--track-caps);
    margin-bottom: var(--s2);
  }
  table.kv {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-xs);
  }
  table.kv th {
    text-align: left;
    vertical-align: top;
    white-space: nowrap;
    padding: var(--s3) var(--s5) var(--s3) 0;
    color: var(--faint);
    font-weight: var(--w-semi);
    border-bottom: 1px solid var(--line-soft);
    width: 1%;
  }
  /* `white-space: normal` UNDOES A GLOBAL. theme.css sets
     `tbody td { white-space: nowrap }` for the market tables, which is right
     there and wrong here: a receipt's `Balances` line is a sentence, and
     nowrap turned it into one 900-pixel row that pushed the card's own width
     past the panel edge. */
  table.kv td {
    padding: var(--s3) 0;
    border-bottom: 1px solid var(--line-soft);
    color: var(--ink);
    line-height: var(--lh-base);
    white-space: normal;
    overflow-wrap: anywhere;
  }

  /* ---- the verdict strip ----
     ONE ROW AT THE HEAD OF THE TABLE. The sentence takes the space it needs and
     the rest hold their own width, so a long verdict wraps inside its own
     column instead of pushing the pills off the card. */
  .lstrip {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex-wrap: wrap;
    padding: var(--s4) var(--s5);
    border-bottom: 1px solid var(--line);
    background: var(--bg-2);
  }
  .lstrip .lq {
    flex: 1 1 22rem;
    min-width: 0;
  }
  .lstrip .lq b {
    display: block;
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
    letter-spacing: -0.01em;
  }
  .lstrip.good .lq b {
    color: var(--up);
  }
  .lstrip.warn .lq b {
    color: var(--warn);
  }
  .lstrip.bad .lq b {
    color: var(--down);
  }
  .lstrip .lq span {
    display: block;
    margin-top: var(--s1);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--dim);
  }
  .lpills {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    align-items: center;
  }
  .lp {
    appearance: none;
    border: 1px solid transparent;
    background: var(--panel-2);
    color: var(--ink-2);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    padding: var(--s2) var(--s4);
    border-radius: var(--r-full);
    cursor: pointer;
    white-space: nowrap;
  }
  .lp:hover {
    color: var(--ink);
  }
  .lp.on {
    border-color: var(--acc);
    color: var(--acc);
  }
  .lp .n {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-weight: var(--w-bold);
    margin-right: var(--s2);
  }
  .lp .dot {
    margin-right: var(--s3);
  }
  .ltot {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-mini);
    color: var(--faint);
  }
  .ltot.bad {
    color: var(--down);
    font-weight: var(--w-bold);
  }
  .lstrip .lact {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: var(--s3);
  }

  /* ---- the outcome list ---- */
  .reasons {
    list-style: none;
    margin: 0 var(--s5);
    padding: var(--s4) var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    background: var(--well);
    border-radius: var(--r3);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
  }
  .reasons li {
    display: flex;
    gap: var(--s4);
    align-items: baseline;
  }
  .obar {
    padding: var(--s4) var(--s5);
  }
  .ohead,
  .orow {
    display: grid;
    grid-template-columns: 9rem 9.5rem 6.5rem minmax(0, 1fr);
    gap: var(--s5);
    align-items: center;
    padding: 0 var(--s5);
  }
  .ohead {
    padding-bottom: var(--s3);
    border-bottom: 1px solid var(--line);
    background: var(--bg-2);
    padding-top: var(--s3);
  }
  .ohead > * {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    text-align: left;
  }
  .ohead .sort {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    cursor: pointer;
  }
  .ohead .sort:hover {
    color: var(--ink);
  }
  .ohead .sort.num {
    text-align: right;
  }
  .olist {
    height: min(46vh, 520px);
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .vspace {
    position: relative;
    width: 100%;
  }
  .orow {
    position: absolute;
    left: 0;
    right: 0;
    height: var(--row-h);
    border-bottom: 1px solid var(--line-soft);
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
  }
  .orow:hover {
    background: var(--panel-2);
  }
  .orow .sym {
    font-family: var(--mono);
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .orow .num {
    font-family: var(--mono);
    text-align: right;
  }
  .orow .why {
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .orow .tag {
    justify-self: start;
  }

  /* ---- the instrument tick list ----
     A SEARCH BOX BIG ENOUGH TO TYPE IN. It is the full width of the menu and
     it sits above the list it filters, not beside it. */
  .srch {
    position: sticky;
    top: 0;
    z-index: 2;
    padding: var(--s4);
    background: var(--bg-2);
    border-bottom: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  .shint {
    margin: 0;
    font-size: var(--fs-mini);
    color: var(--faint);
  }
  .sact {
    display: flex;
    gap: var(--s3);
  }
  .sact button {
    flex: 1;
    padding: var(--s3) var(--s4);
    border-radius: var(--r2);
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    cursor: pointer;
  }
  .sact button:hover:not(:disabled) {
    border-color: var(--line-hard);
    background: var(--panel-2);
  }
  .sact button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .inslist {
    max-height: 46vh;
    overflow-y: auto;
    overscroll-behavior: contain;
    border-bottom: 1px solid var(--line);
  }
  .inslist label {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--s4);
    padding: var(--s3) var(--s5);
    cursor: pointer;
    border-bottom: 1px solid var(--line-soft);
  }
  .inslist label:hover {
    background: var(--panel-2);
  }
  /* PRESENT, VISIBLE, AND REFUSED. The row is dimmed, never removed — its
     title carries which feed does list it. */
  .inslist label.off {
    cursor: not-allowed;
    opacity: 0.62;
  }
  .inslist input {
    accent-color: var(--acc);
    width: 14px;
    height: 14px;
  }
  .inslist .nm {
    font-weight: var(--w-semi);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .inslist .ct {
    font-size: var(--fs-mini);
    color: var(--faint);
    white-space: nowrap;
  }
  .inslist .ct.warn {
    color: var(--warn);
  }
  .insnone {
    margin: 0;
    padding: var(--s5);
    font-size: var(--fs-xs);
    color: var(--faint);
    line-height: var(--lh-base);
  }

  /* ---- the two bulk actions ---- */
  /* A CAUTION UNDER THE ASK LINE IS A SENTENCE, NOT A COLUMN. Left in a
     248px track it wraps into a ribbon nobody reads. */
  .pickers > .caution {
    grid-column: 1 / -1;
  }
  /* BOTTOM RIGHT, WHICH IS WHERE THE MOCKUP PUTS THEM AND WHERE A DESTRUCTIVE
     CONTROL BELONGS: past everything the reader came here to set, on the far
     side of a hairline, so neither is on the path to the form's own controls.
     They were left-aligned in the flow of the strip with a paragraph of grey
     prose beside them, which read as three more things to fill in. */
  .acts {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--s4);
    flex-wrap: wrap;
    /* NO SECOND HAIRLINE. `.ask` sits directly above these two and already
       draws the rule that ends the controls; a border here would put two lines
       three text-rows apart and read as a mistake. Space alone separates the
       foot's two members. */
    margin-top: calc(-1 * var(--s3));
  }
  /* QUIET BY DEFAULT. `--dim` rather than `--ink`: these two are the panel's
     last controls, not its verb, and the submit button below is the only thing
     on this form that should read as loud. Colour arrives on hover, which is
     where intent is. */
  .qb {
    padding: var(--s3) var(--s5);
    border-radius: var(--r2);
    border: 1px solid var(--line);
    background: transparent;
    color: var(--dim);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    cursor: pointer;
  }
  .qb:hover:not(:disabled) {
    border-color: var(--line-hard);
    color: var(--ink);
    background: var(--panel-2);
  }
  .qb:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  /* AN OUTLINE, NOT A FILL. A filled red button is a button that has already
     decided; this one is permanently refused and its whole job is to be VISIBLE
     as an absence. The border carries the warning and the face stays quiet. */
  .qb.danger {
    border-color: var(--down);
    color: var(--dim);
    background: transparent;
  }
  .qb.danger:hover:not(:disabled) {
    color: var(--down);
    background: var(--down-soft);
  }

  /* ---- the census ---- */
  .census {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    min-width: 0;
  }
  h2.sec {
    display: flex;
    align-items: center;
    gap: var(--s4);
    flex-wrap: wrap;
    margin: var(--s4) 0 0;
    font-size: var(--fs-mini);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
  }
  h2.sec .cnt {
    font-size: var(--fs-mini);
    color: var(--acc);
    background: var(--acc-soft);
    padding: 1px 7px;
    border-radius: 9px;
    letter-spacing: 0;
  }
  h2.sec em {
    font-style: normal;
    font-size: var(--fs-mini);
    letter-spacing: 0;
    text-transform: none;
    color: var(--dim);
    font-weight: var(--w-reg);
  }
  .cgrid {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    box-shadow: var(--e1);
    overflow: hidden;
    min-width: 0;
  }
  .cbar {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex-wrap: wrap;
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line);
  }
  /* THE TABLE SCROLLS INSIDE ITS OWN BOX. A wide table that widens the page
     puts the form's own controls off screen. */
  .cscroll {
    max-height: min(58vh, 640px);
    overflow: auto;
    overscroll-behavior: contain;
  }
  .cscroll th {
    vertical-align: bottom;
  }
  .cscroll .sort {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    cursor: pointer;
  }
  .cscroll th.num .sort {
    align-items: flex-end;
    width: 100%;
  }
  .cscroll .sort:hover {
    color: var(--ink);
  }
  .cscroll .hrow {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
  }
  /* A COLUMN THAT CANNOT STATE ITS OWN UNIT IN THE HEADER is a column whose
     meaning lives in a tooltip. The second line is not decoration. */
  .cscroll .hsub {
    font-size: var(--fs-micro);
    font-weight: var(--w-reg);
    letter-spacing: 0;
    text-transform: none;
    color: var(--ghost, var(--faint));
  }
  .cscroll td.num {
    text-align: right;
  }
  .cscroll td.num.warn {
    color: var(--warn);
  }
  .cscroll td.num.up {
    color: var(--up);
  }
  .cscroll .kindtag {
    margin-left: var(--s3);
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--faint);
  }
  .cscroll .dash {
    color: var(--faint);
  }
  .cscroll .empty b {
    display: block;
    color: var(--ink);
    margin-bottom: var(--s2);
  }

  /* ---- the pager ---- */
  .pager {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex-wrap: wrap;
    padding: var(--s3) var(--s5);
    border-top: 1px solid var(--line);
    background: var(--bg-2);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .pager .of b {
    color: var(--ink);
    font-weight: var(--w-semi);
  }
  .pages {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
  .pg {
    min-width: 26px;
    padding: var(--s2) var(--s3);
    border-radius: var(--r2);
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .pg:hover:not(:disabled) {
    border-color: var(--line-hard);
    background: var(--panel-2);
  }
  .pg:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .pg.on {
    border-color: var(--acc);
    color: var(--acc);
    background: var(--acc-soft);
    font-weight: var(--w-bold);
  }
  .pg.gap {
    border-color: transparent;
    background: none;
    cursor: default;
    color: var(--faint);
  }

  .blank .alt {
    display: block;
    margin-top: var(--s6);
    text-align: left;
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--dim);
  }

  /* EVERY transition and animation this file adds is inside the guard, the
     same rule $lib/theme.css holds itself to. A reduced-motion operator gets
     the whole console with nothing moving. */
  @media (prefers-reduced-motion: no-preference) {
    .din,
    .dbtn,
    .cday,
    .calsel,
    .cal-h .nav,
    .orow {
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        color var(--d-hover) var(--ease-out),
        box-shadow var(--d-state) var(--ease-out);
    }
    .meter i {
      transition: width var(--d-panel) var(--ease-out);
    }
    .meter.indet i {
      animation: bx-indeterminate var(--d-sweep) var(--ease-in-out) infinite;
      transition: none;
    }
  }
</style>
